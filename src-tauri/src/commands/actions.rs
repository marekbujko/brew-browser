//! Streaming write-side commands: install, uninstall, upgrade, update, cancel.
//!
//! Every write command in this module:
//! 1. Acquires the global `brew_write_lock` for the duration of the spawn.
//! 2. Runs through `run_brew_streaming`, which emits the full
//!    `BrewStreamEvent` lifecycle to the provided `Channel`.
//! 3. Invalidates `installed_cache` on success.

use tauri::ipc::Channel;
use tauri::State;
use uuid::Uuid;

use crate::brew::exec::run_brew_streaming;
use crate::commands::info::validate_package_name;
use crate::error::BrewError;
use crate::state::AppState;
use crate::types::{BrewStreamEvent, JobResult, PackageKind};

#[tauri::command]
pub async fn brew_install(
    name: String,
    kind: PackageKind,
    force: bool,
    on_event: Channel<BrewStreamEvent>,
    state: State<'_, AppState>,
) -> Result<JobResult, BrewError> {
    validate_package_name(&name)?;
    let path = state.require_brew_path().await?;

    let kind_flag = match kind {
        PackageKind::Formula => "--formula",
        PackageKind::Cask => "--cask",
    };
    let mut args = vec!["install".to_string(), kind_flag.to_string(), name.clone()];
    if force {
        args.push("--force".to_string());
    }
    let display = format!(
        "brew install {} {}{}",
        kind_flag,
        name,
        if force { " --force" } else { "" }
    );
    let jobs = state.jobs.clone();
    let lock = state.brew_write_lock.clone();

    let _guard = lock.lock_owned().await;
    let result = run_brew_streaming(&path, args, display, on_event, jobs).await;

    if result.is_ok() {
        state.invalidate_caches().await;
    }
    result
}

#[tauri::command]
pub async fn brew_uninstall(
    name: String,
    kind: PackageKind,
    zap: bool,
    on_event: Channel<BrewStreamEvent>,
    state: State<'_, AppState>,
) -> Result<JobResult, BrewError> {
    validate_package_name(&name)?;
    let path = state.require_brew_path().await?;

    let kind_flag = match kind {
        PackageKind::Formula => "--formula",
        PackageKind::Cask => "--cask",
    };
    let mut args = vec!["uninstall".to_string(), kind_flag.to_string(), name.clone()];
    if zap && matches!(kind, PackageKind::Cask) {
        args.push("--zap".to_string());
    }
    let display = format!(
        "brew uninstall {} {}{}",
        kind_flag,
        name,
        if zap { " --zap" } else { "" }
    );
    let jobs = state.jobs.clone();
    let lock = state.brew_write_lock.clone();

    let _guard = lock.lock_owned().await;
    let result = run_brew_streaming(&path, args, display, on_event, jobs).await;

    if result.is_ok() {
        state.invalidate_caches().await;
    }
    result
}

#[tauri::command]
pub async fn brew_upgrade(
    name: Option<String>,
    on_event: Channel<BrewStreamEvent>,
    state: State<'_, AppState>,
) -> Result<JobResult, BrewError> {
    if let Some(n) = name.as_ref() {
        validate_package_name(n)?;
    }
    let path = state.require_brew_path().await?;

    let mut args = vec!["upgrade".to_string()];
    if let Some(n) = name.as_ref() {
        args.push(n.clone());
    }
    let display = match name.as_ref() {
        Some(n) => format!("brew upgrade {}", n),
        None => "brew upgrade".to_string(),
    };
    let jobs = state.jobs.clone();
    let lock = state.brew_write_lock.clone();

    let _guard = lock.lock_owned().await;
    let result = run_brew_streaming(&path, args, display, on_event, jobs).await;

    if result.is_ok() {
        state.invalidate_caches().await;
    }
    result
}

/// Upgrade a specific *set* of packages in one `brew upgrade <a> <b> ...`
/// invocation. Used by the Dashboard's curated "Upgrade…" modal where
/// the user has checked the subset they want upgraded (vs the
/// `brew_upgrade(None)` "upgrade everything" path).
///
/// Empty list → InvalidArgument (the caller should use `brew_upgrade(None)`
/// instead). Validates every name through the same allowlist regex as
/// `brew_install` to prevent shell-metacharacter injection.
#[tauri::command]
pub async fn brew_upgrade_many(
    names: Vec<String>,
    on_event: Channel<BrewStreamEvent>,
    state: State<'_, AppState>,
) -> Result<JobResult, BrewError> {
    if names.is_empty() {
        return Err(BrewError::InvalidArgument {
            message: "brew_upgrade_many requires at least one package name; \
                      use brew_upgrade(None) to upgrade everything"
                .to_string(),
        });
    }
    for n in &names {
        validate_package_name(n)?;
    }
    let path = state.require_brew_path().await?;

    let mut args = vec!["upgrade".to_string()];
    args.extend(names.iter().cloned());
    let display = format!("brew upgrade {}", names.join(" "));
    let jobs = state.jobs.clone();
    let lock = state.brew_write_lock.clone();

    let _guard = lock.lock_owned().await;
    let result = run_brew_streaming(&path, args, display, on_event, jobs).await;

    if result.is_ok() {
        state.invalidate_caches().await;
    }
    result
}

#[tauri::command]
pub async fn brew_update(
    on_event: Channel<BrewStreamEvent>,
    state: State<'_, AppState>,
) -> Result<JobResult, BrewError> {
    let path = state.require_brew_path().await?;

    let args = vec!["update".to_string()];
    let display = "brew update".to_string();
    let jobs = state.jobs.clone();
    let lock = state.brew_write_lock.clone();

    let _guard = lock.lock_owned().await;
    let result = run_brew_streaming(&path, args, display, on_event, jobs).await;

    if result.is_ok() {
        state.invalidate_caches().await;
    }
    result
}

/// `brew doctor` — streamed diagnostics (issue #80). Read-only: surfaces
/// advisories in the Activity drawer, touches nothing. brew doctor exits 1
/// when it finds advisories; `run_brew_streaming` (via `doctor_advisory_exit`)
/// treats that non-zero exit as effective-success, so a doctor run that found
/// warnings completes cleanly with the advisory text in the log rather than
/// throwing a "doctor failed" error. Takes the write lock only to serialize
/// streamed brew jobs (no two brew processes stomping each other's output).
#[tauri::command]
pub async fn brew_doctor_stream(
    on_event: Channel<BrewStreamEvent>,
    state: State<'_, AppState>,
) -> Result<JobResult, BrewError> {
    let path = state.require_brew_path().await?;

    let args = vec!["doctor".to_string()];
    let display = "brew doctor".to_string();
    let jobs = state.jobs.clone();
    let lock = state.brew_write_lock.clone();

    let _guard = lock.lock_owned().await;
    // No cache invalidation — doctor is read-only.
    run_brew_streaming(&path, args, display, on_event, jobs).await
}

/// `brew cleanup --prune=all [--scrub] [--verbose]` — reclaim cache space (issue
/// #80). Destructive of CACHED DOWNLOADS only; installed packages are untouched.
/// `scrub` (opt-in, default off in the UI) also removes the LATEST versions'
/// downloads — more aggressive, so it's a deliberate toggle rather than the
/// default. `verbose` lists every file removed. The UI confirm-gates this with
/// the reclaimable estimate (see `brew_cleanup_preview`). On success the
/// disk-usage cache is dropped so the Storage card re-measures.
#[tauri::command]
pub async fn brew_cleanup(
    scrub: bool,
    verbose: bool,
    on_event: Channel<BrewStreamEvent>,
    state: State<'_, AppState>,
) -> Result<JobResult, BrewError> {
    let path = state.require_brew_path().await?;

    let mut args = vec!["cleanup".to_string(), "--prune=all".to_string()];
    if scrub {
        args.push("--scrub".to_string());
    }
    if verbose {
        args.push("--verbose".to_string());
    }
    let display = format!(
        "brew cleanup --prune=all{}{}",
        if scrub { " --scrub" } else { "" },
        if verbose { " --verbose" } else { "" }
    );
    let jobs = state.jobs.clone();
    let lock = state.brew_write_lock.clone();

    let _guard = lock.lock_owned().await;
    let result = run_brew_streaming(&path, args, display, on_event, jobs).await;

    if result.is_ok() {
        // The on-disk cache shrank; drop the cached disk-usage report so the
        // Storage card re-measures. Installed packages are unchanged, so the
        // installed_cache is left intact.
        *state.disk_usage_cache.lock().await = None;
    }
    result
}

#[tauri::command]
pub async fn cancel_job(job_id: Uuid, state: State<'_, AppState>) -> Result<(), BrewError> {
    let mut map = state.jobs.lock().await;
    let handle = map.get_mut(&job_id).ok_or_else(|| BrewError::JobNotFound {
        job_id: job_id.to_string(),
    })?;

    if let Some(tx) = handle.cancel_tx.take() {
        let _ = tx.send(());
    }
    Ok(())
}
