//! `cask_icon` — extract a cask's `.app` bundle icon, cache it as PNG,
//! return a base64 data URL the frontend can drop into `<img src=...>`.
//!
//! Algorithm (Phase 7):
//! 1. Resolve the installed `.app` path for the cask token. Prefer the
//!    paths brew already enumerates in `brew info --json=v2 --cask <token>`
//!    via `artifacts[].app[]`. Fall back to `/Applications/<Name>.app` and
//!    `~/Applications/<Name>.app` for that filename.
//! 2. Find the icon file inside the bundle:
//!    - Read `Contents/Info.plist`'s `CFBundleIconFile` via
//!      `defaults read <Info.plist> CFBundleIconFile` (no plist crate).
//!    - Append `.icns` if missing.
//!    - If that file doesn't exist, fall back to the first `*.icns` in
//!      `Contents/Resources/`, preferring one whose stem matches the
//!      bundle name.
//! 3. Convert `.icns` → 64x64 PNG with macOS-native `sips`.
//! 4. Cache to `<state.cache_dir>/icons/<token>.png`. On subsequent
//!    calls, reuse if newer than 7 days.
//! 5. Return `Some(data:image/png;base64,...)` on success; `Ok(None)`
//!    when the cask is not installed or has no usable icon (common for
//!    pkg-installer casks and bare-binary casks).
//!
//! Hard requirements:
//! - Token validation reuses `validate_package_name` semantics so a
//!   malicious token can't escape into argv to `defaults`, `sips`, or
//!   the path constructors. Cask tokens are a tighter subset of formula
//!   names; the same allowlist covers them.
//! - All shell-outs go through `tokio::process::Command` with typed args
//!   (no shell, no `tauri-plugin-shell`).
//! - This is a read-only filesystem op; we do NOT take the brew write
//!   mutex. Multiple icon fetches can run concurrently.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use tauri::State;

use crate::commands::info::validate_cask_token;
use crate::error::BrewError;
use crate::state::AppState;

// macOS-only: the `.app`-bundle icon extraction pipeline (defaults/sips
// shell-outs, brew-info resolution, .icns discovery). These symbols are
// unused on non-macOS targets where `cask_icon` short-circuits to
// `Ok(None)`, so gate them to avoid dead-code warnings on the Linux build.
#[cfg(target_os = "macos")]
use tokio::process::Command;
#[cfg(target_os = "macos")]
use crate::brew::exec::run_brew_capture;
#[cfg(target_os = "macos")]
use crate::brew::parse::RawInfoV2;
#[cfg(target_os = "macos")]
use crate::error::truncate_head;

/// Cache TTL — re-extract if the cached PNG is older than this.
const ICON_CACHE_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// Display size for the rendered PNG. Kept small so the data URL payload
/// stays tight in the IPC bridge and the DOM.
const ICON_PIXELS: u32 = 64;

#[tauri::command]
pub async fn cask_icon(
    token: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, BrewError> {
    // Defense in depth at the IPC boundary. `validate_cask_token`
    // applies the package-name rules plus a tighter filesystem-safe
    // overlay (rejects `/`, `..`, leading `.`) — required because the
    // token is composed directly into a cache path on disk
    // (`<cache_dir>/icons/<token>.png`).
    validate_cask_token(&token)?;

    let icons_dir = state.cache_dir.join("icons");
    ensure_dir(&icons_dir)?;
    let cache_path = icons_dir.join(format!("{}.png", token));

    // Fast path: serve from cache when fresh.
    if let Some(data_url) = read_fresh_cache(&cache_path).await? {
        return Ok(Some(data_url));
    }

    // `.app`-bundle icon extraction is macOS-only — it shells out to the
    // macOS-native `/usr/bin/defaults` and `/usr/bin/sips` and walks the
    // `Contents/Resources/*.icns` bundle layout. Linux casks don't
    // produce `.app` bundles, so the `IconSource` routing won't even
    // select this path; we short-circuit to `Ok(None)` here anyway as
    // defense in depth, so we never attempt to spawn binaries that don't
    // exist on Linux.
    #[cfg(target_os = "macos")]
    {
        // Resolve the .app bundle. None → cask not installed or no `app`
        // artifact (common for pkg / binary-only casks). Return Ok(None).
        let app_path = match resolve_app_path(&state, &token).await? {
            Some(p) => p,
            None => return Ok(None),
        };

        // Find the .icns file inside the bundle. None → unbundled app or
        // some app shipping non-standard resources. Return Ok(None).
        let icns_path = match find_icns(&app_path).await {
            Some(p) => p,
            None => return Ok(None),
        };

        // Convert .icns → cached PNG. sips failure is a real error
        // (sips ships with macOS; missing or crashing it is genuinely
        // exceptional).
        sips_convert_to_png(&icns_path, &cache_path).await?;

        // Read back, encode, return.
        encode_png_as_data_url(&cache_path).await.map(Some)
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(None)
    }
}

// ---------- Cache layer ----------

async fn read_fresh_cache(cache_path: &Path) -> Result<Option<String>, BrewError> {
    let meta = match tokio::fs::metadata(cache_path).await {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(BrewError::Io {
                message: format!("stat {}: {}", cache_path.display(), e),
            })
        }
    };
    if !is_fresh(&meta, ICON_CACHE_TTL) {
        return Ok(None);
    }
    Some(encode_png_as_data_url(cache_path).await).transpose()
}

fn is_fresh(meta: &std::fs::Metadata, ttl: Duration) -> bool {
    let modified = match meta.modified() {
        Ok(t) => t,
        Err(_) => return false,
    };
    match SystemTime::now().duration_since(modified) {
        Ok(age) => age < ttl,
        Err(_) => true, // Future mtime → treat as fresh, not stale.
    }
}

async fn encode_png_as_data_url(path: &Path) -> Result<String, BrewError> {
    let bytes = tokio::fs::read(path).await.map_err(|e| BrewError::Io {
        message: format!("read {}: {}", path.display(), e),
    })?;
    Ok(format!("data:image/png;base64,{}", B64.encode(&bytes)))
}

fn ensure_dir(dir: &Path) -> Result<(), BrewError> {
    if dir.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dir).map_err(|e| BrewError::Io {
        message: format!("create dir {}: {}", dir.display(), e),
    })
}

// ---------- App-path resolution ----------

/// Return the absolute path to the cask's `.app` bundle, or `None` if
/// the cask isn't installed / has no `app` artifact.
#[cfg(target_os = "macos")]
async fn resolve_app_path(
    state: &State<'_, AppState>,
    token: &str,
) -> Result<Option<PathBuf>, BrewError> {
    let path = state.require_brew_path().await?;
    let display = format!("brew info --json=v2 --cask {}", token);
    let raw = match run_brew_capture(
        &path,
        &["info", "--json=v2", "--cask", token],
        &display,
    )
    .await
    {
        Ok(s) => s,
        // If brew refused (unknown cask, network blip, etc.), treat as
        // "no icon available" rather than propagating — this command is
        // best-effort UX, not a hard error path.
        Err(BrewError::BrewExitNonZero { .. }) => return Ok(None),
        Err(e) => return Err(e),
    };

    let parsed: RawInfoV2 = serde_json::from_str(&raw).map_err(|e| BrewError::JsonParse {
        command: display,
        message: e.to_string(),
        raw_excerpt: truncate_head(&raw, 2048),
    })?;
    let cask = match parsed.casks.into_iter().next() {
        Some(c) => c,
        None => return Ok(None),
    };

    // brew info `installed` field for casks is the version string or null.
    // We only render icons for installed casks — uninstalled ones have no
    // .app on disk.
    if cask.installed.is_none() {
        return Ok(None);
    }

    let app_filename = match first_app_filename(&cask.artifacts) {
        Some(name) => name,
        None => return Ok(None),
    };

    Ok(resolve_app_bundle(&app_filename))
}

/// Walk `artifacts[].app[]` and return the first string entry. Brew
/// sometimes serializes `app` as `["Firefox.app"]`, sometimes as
/// `[{"target": "Firefox.app", "source": "..."}]`; handle both.
#[cfg(target_os = "macos")]
fn first_app_filename(artifacts: &Option<serde_json::Value>) -> Option<String> {
    let arr = artifacts.as_ref()?.as_array()?;
    for entry in arr {
        let obj = entry.as_object()?;
        if let Some(serde_json::Value::Array(apps)) = obj.get("app") {
            for a in apps {
                if let Some(s) = a.as_str() {
                    return Some(s.to_string());
                }
                if let Some(o) = a.as_object() {
                    if let Some(s) = o.get("target").and_then(|v| v.as_str()) {
                        return Some(s.to_string());
                    }
                    if let Some(s) = o.get("source").and_then(|v| v.as_str()) {
                        // `source` may be a path; basename it.
                        return Some(
                            Path::new(s)
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_else(|| s.to_string()),
                        );
                    }
                }
            }
        }
    }
    None
}

/// Given an `.app` filename like `"Firefox.app"`, return the first of
/// `/Applications/<name>` or `~/Applications/<name>` that exists.
/// Returns `None` if neither does.
#[cfg(target_os = "macos")]
fn resolve_app_bundle(filename: &str) -> Option<PathBuf> {
    // Filename safety — must end with `.app` and not contain path
    // separators (so we don't accidentally resolve into a parent dir).
    if !filename.ends_with(".app") || filename.contains('/') || filename.contains("..") {
        return None;
    }
    let mut candidates: Vec<PathBuf> = Vec::with_capacity(2);
    candidates.push(PathBuf::from("/Applications").join(filename));
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join("Applications").join(filename));
    }
    candidates.into_iter().find(|p| p.is_dir())
}

// ---------- .icns discovery ----------

/// Locate the `.icns` icon file inside an `.app` bundle.
///
/// **M5 traversal defense.** The `CFBundleIconFile` value comes from a
/// plist file that lives inside an attacker-influenced bundle (a user
/// could install a cask whose `Info.plist` contains
/// `CFBundleIconFile = "../../etc/passwd"` to attempt to point at a
/// system file outside `Contents/Resources/`). After constructing the
/// candidate path, we canonicalize it and verify it still lives inside
/// the canonicalized `Resources/` directory before returning it. The
/// downstream `sips` invocation only ever sees paths that have passed
/// this gate.
///
/// `read_bundle_icon_file` is unchanged — `defaults read` happily
/// reads any plist path; the gate is the *use* of its return value.
#[cfg(target_os = "macos")]
async fn find_icns(app_path: &Path) -> Option<PathBuf> {
    let info_plist = app_path.join("Contents").join("Info.plist");
    let resources = app_path.join("Contents").join("Resources");

    // Primary: `defaults read <Info.plist> CFBundleIconFile`.
    if let Some(value) = read_bundle_icon_file(&info_plist).await {
        let value = value.trim();
        if !value.is_empty() {
            // CFBundleIconFile may or may not include the .icns suffix.
            let with_ext = if value.to_lowercase().ends_with(".icns") {
                value.to_string()
            } else {
                format!("{}.icns", value)
            };
            if let Some(safe) = safe_join_in_resources(&resources, &with_ext) {
                if safe.is_file() {
                    return Some(safe);
                }
            }
        }
    }

    // Fallback 1: prefer an `<bundle-stem>.icns` if present.
    if let Some(stem) = app_path.file_stem().and_then(|s| s.to_str()) {
        let with_ext = format!("{}.icns", stem);
        if let Some(safe) = safe_join_in_resources(&resources, &with_ext) {
            if safe.is_file() {
                return Some(safe);
            }
        }
    }

    // Fallback 2: first `*.icns` in Resources/. This walker only returns
    // entries already under `resources` so no traversal check is needed.
    first_icns_in_dir(&resources).await
}

/// Join `candidate` onto `resources` and verify the resulting path is
/// still inside `resources` after canonicalization. Returns `None` for
/// any attempted traversal (e.g. `candidate = "../../etc/passwd.icns"`)
/// or any case where canonicalization fails (file missing, broken
/// symlink, permission denied).
///
/// Both sides are canonicalized so that a symlink farm pointing back
/// into `resources` from an external location is still detected as a
/// traversal — we compare resolved physical paths, not lexical paths.
#[cfg(target_os = "macos")]
fn safe_join_in_resources(resources: &Path, candidate: &str) -> Option<PathBuf> {
    // Reject obvious lexical traversal before touching the disk —
    // canonicalize would either resolve out or fail, but a quick
    // upfront check avoids a syscall on the common attack path.
    if candidate.contains("..") || candidate.contains('/') {
        return None;
    }
    let joined = resources.join(candidate);
    let candidate_canon = joined.canonicalize().ok()?;
    let resources_canon = resources.canonicalize().ok()?;
    if !candidate_canon.starts_with(&resources_canon) {
        return None;
    }
    Some(candidate_canon)
}

/// Shell out to `defaults read <Info.plist> CFBundleIconFile`. Returns
/// `None` when the key is absent or `defaults` exits non-zero (binary
/// plists are still readable by `defaults`).
#[cfg(target_os = "macos")]
async fn read_bundle_icon_file(info_plist: &Path) -> Option<String> {
    // `defaults read` wants the path without the trailing `.plist`.
    let arg = info_plist
        .to_str()
        .map(|s| s.trim_end_matches(".plist").to_string())?;
    let output = Command::new("/usr/bin/defaults")
        .arg("read")
        .arg(&arg)
        .arg("CFBundleIconFile")
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(target_os = "macos")]
async fn first_icns_in_dir(dir: &Path) -> Option<PathBuf> {
    let mut rd = tokio::fs::read_dir(dir).await.ok()?;
    // Read all entries first so we can sort for determinism.
    let mut entries: Vec<PathBuf> = Vec::new();
    while let Ok(Some(entry)) = rd.next_entry().await {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("icns"))
            .unwrap_or(false)
        {
            entries.push(p);
        }
    }
    entries.sort();
    entries.into_iter().next()
}

// ---------- sips conversion ----------

#[cfg(target_os = "macos")]
async fn sips_convert_to_png(input: &Path, output: &Path) -> Result<(), BrewError> {
    // sips: macOS-native, no extra deps. Resize to ICON_PIXELS square.
    let out = Command::new("/usr/bin/sips")
        .args([
            "-s",
            "format",
            "png",
            "-z",
            &ICON_PIXELS.to_string(),
            &ICON_PIXELS.to_string(),
        ])
        .arg(input)
        .arg("--out")
        .arg(output)
        .output()
        .await
        .map_err(|e| BrewError::Io {
            message: format!("spawn sips: {}", e),
        })?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(BrewError::Io {
            message: format!(
                "sips failed (exit {:?}) for {}: {}",
                out.status.code(),
                input.display(),
                stderr.trim()
            ),
        });
    }
    Ok(())
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::info::validate_package_name;
    // `json!` is only used by the macOS-gated `first_app_filename` tests.
    #[cfg(target_os = "macos")]
    use serde_json::json;

    // ---------- token validation reuse ----------

    #[test]
    fn validates_token_with_same_rules_as_package_name() {
        // Sanity-check that validate_package_name (the base validator)
        // accepts realistic cask tokens and rejects shell-injection shapes.
        // cask_icon now uses the stricter `validate_cask_token` which
        // layers on filesystem-safety rules — see `validate_cask_token`
        // tests in `info::tests`.
        validate_package_name("firefox").expect("firefox");
        validate_package_name("visual-studio-code").expect("dashed");
        validate_package_name("docker").expect("plain");
        validate_package_name("1password").expect("digit-first");

        assert!(validate_package_name("-rf").is_err(), "leading dash");
        assert!(validate_package_name("").is_err(), "empty");
        assert!(validate_package_name("foo bar").is_err(), "space");
        assert!(validate_package_name("foo;bar").is_err(), "semicolon");
        assert!(validate_package_name("$(whoami)").is_err(), "subshell");
        assert!(validate_package_name("foo\0bar").is_err(), "null byte");
        assert!(validate_package_name("foo\nbar").is_err(), "newline");
        // `../foo` slips past validate_package_name (allowlist chars).
        // The stricter `validate_cask_token` rejects it — see info::tests.
    }

    #[test]
    fn cask_icon_command_uses_strict_token_validator() {
        // Path-traversal-looking tokens that would compose into the
        // cache path must be rejected by `validate_cask_token` — the
        // gate `cask_icon` actually enforces.
        use crate::commands::info::validate_cask_token;
        assert!(validate_cask_token("../etc/passwd").is_err());
        assert!(validate_cask_token("homebrew/cask/firefox").is_err());
        assert!(validate_cask_token(".").is_err());
    }

    // ---------- first_app_filename (macOS-only: gated with the fn) ----------

    #[cfg(target_os = "macos")]
    #[test]
    fn first_app_filename_handles_string_form() {
        let artifacts = Some(json!([
            { "app": ["Firefox.app"] }
        ]));
        assert_eq!(first_app_filename(&artifacts).as_deref(), Some("Firefox.app"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn first_app_filename_handles_object_target_form() {
        let artifacts = Some(json!([
            { "app": [ { "target": "Visual Studio Code.app", "source": "Code.app" } ] }
        ]));
        assert_eq!(
            first_app_filename(&artifacts).as_deref(),
            Some("Visual Studio Code.app")
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn first_app_filename_falls_back_to_source_basename() {
        let artifacts = Some(json!([
            { "app": [ { "source": "/staged/path/Some.app" } ] }
        ]));
        assert_eq!(first_app_filename(&artifacts).as_deref(), Some("Some.app"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn first_app_filename_skips_non_app_artifacts() {
        let artifacts = Some(json!([
            { "binary": ["foo"] },
            { "pkg": ["installer.pkg"] },
            { "app": ["Real.app"] }
        ]));
        assert_eq!(first_app_filename(&artifacts).as_deref(), Some("Real.app"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn first_app_filename_returns_none_for_no_artifacts() {
        assert_eq!(first_app_filename(&None), None);
        assert_eq!(first_app_filename(&Some(json!([]))), None);
        assert_eq!(
            first_app_filename(&Some(json!([{ "pkg": ["installer.pkg"] }]))),
            None
        );
    }

    // ---------- resolve_app_bundle ----------

    #[cfg(target_os = "macos")]
    #[test]
    fn resolve_app_bundle_rejects_non_app_filenames() {
        assert_eq!(resolve_app_bundle("Firefox"), None);
        assert_eq!(resolve_app_bundle("Firefox.pkg"), None);
        assert_eq!(resolve_app_bundle(""), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn resolve_app_bundle_rejects_path_traversal() {
        assert_eq!(resolve_app_bundle("../etc/passwd.app"), None);
        assert_eq!(resolve_app_bundle("..app"), None);
        assert_eq!(resolve_app_bundle("sub/dir/Foo.app"), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn resolve_app_bundle_returns_none_when_neither_path_exists() {
        // This filename is overwhelmingly unlikely to be installed.
        let r = resolve_app_bundle("__brew_browser_nope_xyzzy__.app");
        assert_eq!(r, None);
    }

    // ---------- is_fresh ----------

    #[test]
    fn is_fresh_true_when_within_ttl() {
        // tempfile crate is a dev-dep already.
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        let meta = std::fs::metadata(tmp.path()).expect("stat");
        assert!(is_fresh(&meta, Duration::from_secs(3600)));
    }

    #[test]
    fn is_fresh_false_when_ttl_is_zero() {
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        let meta = std::fs::metadata(tmp.path()).expect("stat");
        // Even a brand-new file is not "fresh" under a zero TTL — used as
        // a defensive check that the comparison is strict-less-than.
        assert!(!is_fresh(&meta, Duration::from_secs(0)));
    }

    // ---------- encode_png_as_data_url ----------

    #[tokio::test]
    async fn encode_png_as_data_url_produces_correct_prefix() {
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        // Write a tiny "fake png" — encoding doesn't validate the format,
        // it just base64-wraps the bytes.
        std::fs::write(tmp.path(), [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a])
            .expect("write");
        let url = encode_png_as_data_url(tmp.path()).await.expect("encode");
        assert!(url.starts_with("data:image/png;base64,"), "url = {url}");
        // The base64 body decodes back to the original bytes.
        let body = url.trim_start_matches("data:image/png;base64,");
        let decoded = B64.decode(body).expect("decode");
        assert_eq!(decoded, [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
    }

    // ---------- ensure_dir ----------

    #[test]
    fn ensure_dir_creates_missing_directory() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let nested = tmp.path().join("icons").join("nested");
        ensure_dir(&nested).expect("create");
        assert!(nested.is_dir());
    }

    #[test]
    fn ensure_dir_is_idempotent_when_dir_exists() {
        let tmp = tempfile::tempdir().expect("tempdir");
        ensure_dir(tmp.path()).expect("existing");
    }

    // ---------- M5: safe_join_in_resources ----------

    #[cfg(target_os = "macos")]
    #[test]
    fn safe_join_accepts_plain_filename_in_resources() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let resources = tmp.path().join("Resources");
        std::fs::create_dir_all(&resources).expect("mkdir");
        let icon = resources.join("AppIcon.icns");
        std::fs::write(&icon, b"fake icns").expect("write");
        let r = safe_join_in_resources(&resources, "AppIcon.icns");
        let resolved = r.expect("should accept in-bundle filename");
        assert!(resolved.ends_with("AppIcon.icns"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn safe_join_rejects_dotdot_traversal() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let resources = tmp.path().join("Resources");
        std::fs::create_dir_all(&resources).expect("mkdir");
        // Attempt to escape Resources/ and land on a sibling.
        let outside = tmp.path().join("evil.icns");
        std::fs::write(&outside, b"oops").expect("write");
        // Lexical reject — `..` is rejected before canonicalize.
        assert!(safe_join_in_resources(&resources, "../evil.icns").is_none());
        assert!(safe_join_in_resources(&resources, "../../etc/passwd.icns").is_none());
        assert!(safe_join_in_resources(&resources, "../../../../etc/passwd.icns").is_none());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn safe_join_rejects_slash_separated_subpath() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let resources = tmp.path().join("Resources");
        let nested = resources.join("sub");
        std::fs::create_dir_all(&nested).expect("mkdir");
        std::fs::write(nested.join("x.icns"), b"x").expect("write");
        // Even a legitimate-looking subpath is rejected — CFBundleIconFile
        // never contains a `/` (Apple docs: filename only, no directory).
        assert!(safe_join_in_resources(&resources, "sub/x.icns").is_none());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn safe_join_rejects_symlink_pointing_outside_resources() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let resources = tmp.path().join("Resources");
        std::fs::create_dir_all(&resources).expect("mkdir");
        let outside = tmp.path().join("secret.icns");
        std::fs::write(&outside, b"oops").expect("write");
        let link = resources.join("Icon.icns");
        // Skip on systems where symlink creation fails.
        if std::os::unix::fs::symlink(&outside, &link).is_err() {
            return;
        }
        // Even though the candidate filename is benign, the canonicalized
        // target is outside Resources/ — must be rejected.
        let r = safe_join_in_resources(&resources, "Icon.icns");
        assert!(r.is_none(), "symlink escape should be rejected, got {:?}", r);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn safe_join_returns_none_for_nonexistent_target() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let resources = tmp.path().join("Resources");
        std::fs::create_dir_all(&resources).expect("mkdir");
        assert!(safe_join_in_resources(&resources, "missing.icns").is_none());
    }
}
