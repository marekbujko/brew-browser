//! Disk-usage + Finder-reveal commands for the Dashboard "Storage" card.
//!
//! - `disk_usage()` shells out to `du -sk` in parallel for the canonical
//!   Homebrew sub-trees (Cellar, Caskroom, var/log, download cache) and
//!   returns their sizes in bytes. Results are memoised on `AppState` with
//!   a short TTL so the Dashboard refresh button feels instant after the
//!   first probe.
//! - `open_in_finder(path)` reveals a path in macOS Finder. Refuses any path
//!   that is not inside the resolved Homebrew prefix or download cache.
//!
//! Security gate (revealer): the path is canonicalised and checked against
//! the canonicalised prefix + cache roots, so symlinks-out-of-tree can't
//! escape. The frontend's call site is the Dashboard's Open buttons, which
//! pass back paths the backend itself just produced — but the gate stays
//! anyway because IPC is the trust boundary.
//!
//! Performance: the four `du -sk` invocations run concurrently via tokio::join!.
//! On a 325-package install, total wall time is dominated by the slowest path
//! (typically Cellar at ~1-3 s on SSD).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::process::Command;

use crate::brew::exec::run_brew_capture;
use crate::error::BrewError;
use crate::state::AppState;

const DU_TIMEOUT: Duration = Duration::from_secs(20);
const DU_CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskUsageEntry {
    pub label: String,
    pub path: String,
    pub bytes: u64,
    pub exists: bool,
    /// `None` on success; otherwise a short message (timeout, permissions, etc.).
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskUsageReport {
    pub generated_at: String,
    pub prefix: String,
    pub cache_dir: String,
    pub entries: Vec<DiskUsageEntry>,
    pub total_bytes: u64,
    /// Seconds since this report was produced (server-side). 0 on a fresh
    /// fetch; older when served from cache.
    pub cache_age_seconds: u64,
}

/// Cached entry on `AppState` so repeated Dashboard renders don't re-shell
/// `du` on every nav. TTL set by `DU_CACHE_TTL`.
#[derive(Debug, Clone)]
pub struct CachedDiskUsage {
    pub report: DiskUsageReport,
    pub fetched_at: Instant,
}

/// Run `brew <args>` and return trimmed stdout — used to resolve the prefix
/// + cache dir before sizing them.
async fn brew_subcommand(brew: &Path, args: &[&str], context: &str) -> Result<String, BrewError> {
    let out = run_brew_capture(brew, args, context).await?;
    Ok(out.trim().to_string())
}

/// `du -sk <path>` in bytes. Returns `(bytes, exists, error)`:
///   - exists=false: path doesn't exist on disk (e.g. Caskroom is absent on
///     pure-formula installs); not an error
///   - error=Some: the size couldn't be measured (timeout, permission denied)
async fn du_bytes(path: &Path) -> (u64, bool, Option<String>) {
    if !path.exists() {
        return (0, false, None);
    }
    let output = tokio::time::timeout(
        DU_TIMEOUT,
        Command::new("du").arg("-sk").arg(path).output(),
    )
    .await;
    let result = match output {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => return (0, true, Some(format!("du spawn failed: {e}"))),
        Err(_) => return (0, true, Some("du timed out".into())),
    };
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr).trim().to_string();
        return (
            0,
            true,
            Some(if stderr.is_empty() { "du failed".into() } else { stderr }),
        );
    }
    let text = String::from_utf8_lossy(&result.stdout);
    let kb: u64 = text
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    (kb * 1024, true, None)
}

#[tauri::command]
pub async fn disk_usage(state: State<'_, AppState>) -> Result<DiskUsageReport, BrewError> {
    // Cache hit — return fast with computed age so the UI can render a
    // "Updated Ns ago" label.
    {
        let cache = state.disk_usage_cache.lock().await;
        if let Some(cached) = cache.as_ref() {
            let age = cached.fetched_at.elapsed();
            if age < DU_CACHE_TTL {
                let mut r = cached.report.clone();
                r.cache_age_seconds = age.as_secs();
                return Ok(r);
            }
        }
    }

    let brew = state.require_brew_path().await?;
    let prefix = brew_subcommand(&brew, &["--prefix"], "brew --prefix").await?;
    let cache = brew_subcommand(&brew, &["--cache"], "brew --cache").await?;
    let prefix_path = PathBuf::from(&prefix);

    let cellar_path = prefix_path.join("Cellar");
    let caskroom_path = prefix_path.join("Caskroom");
    let varlog_path = prefix_path.join("var").join("log");
    let cache_path = PathBuf::from(&cache);

    // Parallel du — the slowest path bounds total wall time.
    let (cellar, caskroom, varlog, dl_cache) = tokio::join!(
        du_bytes(&cellar_path),
        du_bytes(&caskroom_path),
        du_bytes(&varlog_path),
        du_bytes(&cache_path),
    );

    let entries = vec![
        DiskUsageEntry {
            label: "Formulae (Cellar)".into(),
            path: cellar_path.to_string_lossy().into_owned(),
            bytes: cellar.0,
            exists: cellar.1,
            error: cellar.2,
        },
        DiskUsageEntry {
            label: "Casks (Caskroom)".into(),
            path: caskroom_path.to_string_lossy().into_owned(),
            bytes: caskroom.0,
            exists: caskroom.1,
            error: caskroom.2,
        },
        DiskUsageEntry {
            label: "Logs (var/log)".into(),
            path: varlog_path.to_string_lossy().into_owned(),
            bytes: varlog.0,
            exists: varlog.1,
            error: varlog.2,
        },
        DiskUsageEntry {
            label: "Download cache".into(),
            path: cache_path.to_string_lossy().into_owned(),
            bytes: dl_cache.0,
            exists: dl_cache.1,
            error: dl_cache.2,
        },
    ];

    let total_bytes = entries.iter().map(|e| e.bytes).sum();

    let report = DiskUsageReport {
        generated_at: chrono::Utc::now().to_rfc3339(),
        prefix,
        cache_dir: cache,
        entries,
        total_bytes,
        cache_age_seconds: 0,
    };

    let mut cache_lock = state.disk_usage_cache.lock().await;
    *cache_lock = Some(CachedDiskUsage {
        report: report.clone(),
        fetched_at: Instant::now(),
    });
    Ok(report)
}

/// Invalidate the disk-usage cache so the next call re-runs `du`. Useful for
/// a manual Refresh button on the Dashboard.
#[tauri::command]
pub async fn disk_usage_clear_cache(state: State<'_, AppState>) -> Result<(), BrewError> {
    let mut cache = state.disk_usage_cache.lock().await;
    *cache = None;
    Ok(())
}

#[tauri::command]
pub async fn open_in_finder(path: String, state: State<'_, AppState>) -> Result<(), BrewError> {
    let brew = state.require_brew_path().await?;
    let prefix = brew_subcommand(&brew, &["--prefix"], "brew --prefix").await?;
    let cache = brew_subcommand(&brew, &["--cache"], "brew --cache").await?;

    // Canonicalise both the target and the allowed roots. Logical-comparison
    // fallback covers the case where the target doesn't exist (e.g. Caskroom
    // on a pure-formula install); we still refuse it for safety.
    let target = PathBuf::from(&path);
    let target_canon = target.canonicalize().unwrap_or_else(|_| target.clone());
    let prefix_canon = PathBuf::from(&prefix).canonicalize().unwrap_or_else(|_| PathBuf::from(&prefix));
    let cache_canon = PathBuf::from(&cache).canonicalize().unwrap_or_else(|_| PathBuf::from(&cache));

    let inside = target_canon.starts_with(&prefix_canon) || target_canon.starts_with(&cache_canon);
    if !inside {
        return Err(BrewError::InvalidArgument {
            message: format!(
                "refusing to open {}: not inside the Homebrew prefix or cache",
                path
            ),
        });
    }

    reveal_in_file_manager(&path).await
}

/// Reveal `path` in the platform file manager. The caller has already
/// passed the path through the Homebrew-prefix/cache security gate; this
/// only spawns the reveal.
///
/// - macOS: `open -R <path>` selects the item in Finder.
/// - Linux: there is no portable "reveal and select" verb. `xdg-open
///   <file>` would *launch* the file in its default app (wrong for a
///   "show me where this is" action), so we open the containing
///   directory instead — `xdg-open <parent-dir>` pops the file manager
///   at the right location. A path with no parent (shouldn't happen for
///   gated Homebrew paths) falls back to the path itself.
#[cfg(target_os = "macos")]
async fn reveal_in_file_manager(path: &str) -> Result<(), BrewError> {
    let status = Command::new("open")
        .arg("-R")
        .arg(path)
        .status()
        .await
        .map_err(|e| BrewError::Io {
            message: format!("failed to spawn open: {e}"),
        })?;
    if !status.success() {
        return Err(BrewError::Internal {
            message: format!("open exited with {:?}", status.code()),
        });
    }
    Ok(())
}

#[cfg(target_os = "linux")]
async fn reveal_in_file_manager(path: &str) -> Result<(), BrewError> {
    let target = Path::new(path);
    let dir = target.parent().unwrap_or(target);
    let status = Command::new("xdg-open")
        .arg(dir)
        .status()
        .await
        .map_err(|e| BrewError::Io {
            message: format!("failed to spawn xdg-open: {e}"),
        })?;
    if !status.success() {
        return Err(BrewError::Internal {
            message: format!("xdg-open exited with {:?}", status.code()),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn du_bytes_returns_zero_for_missing_path() {
        let (bytes, exists, err) = du_bytes(Path::new("/definitely/not/a/real/path/here")).await;
        assert_eq!(bytes, 0);
        assert!(!exists);
        assert!(err.is_none());
    }

    #[tokio::test]
    async fn du_bytes_returns_size_for_real_path() {
        // /etc/hosts has existed on every Mac for the entire history of macOS.
        // (/usr/bin/ls moved into the dyld shared cache in recent releases, so
        // it isn't a reliable test fixture.)
        let (bytes, exists, err) = du_bytes(Path::new("/etc/hosts")).await;
        assert!(exists, "/etc/hosts should exist");
        assert!(err.is_none(), "du should succeed on /etc/hosts: {err:?}");
        assert!(bytes > 0, "/etc/hosts should report nonzero size");
    }
}
