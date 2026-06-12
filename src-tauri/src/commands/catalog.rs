//! Catalog commands (Phase 12a).
//!
//! Surface for the bundled-or-user-refreshed Homebrew catalog living on
//! `AppState`. All commands except `catalog_refresh` are pure reads that
//! return clones / Arc references and do no I/O beyond a single
//! `RwLock::read`.
//!
//! Outbound network: `catalog_refresh` is the ONLY command in this
//! module that talks to formulae.brew.sh. When Phase 12d lands, it
//! must call `state.require_network("catalog_refresh")` first; the
//! security review (§Cross-cutting concerns) makes this an explicit
//! retroactive gate.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::catalog::{Cask, Catalog, Formula, Manifest, MAX_CATALOG_BYTES};
use crate::commands::info::{validate_cask_token, validate_package_name};
use crate::commands::settings::{CatalogAutoRefresh, SettingsLoadState};
use crate::error::BrewError;
use crate::state::AppState;

/// Endpoints the refresh command fetches from. The Python build script
/// uses the same URLs — keep them in sync if either side changes.
const FORMULA_URL: &str = "https://formulae.brew.sh/api/formula.json";
const CASK_URL: &str = "https://formulae.brew.sh/api/cask.json";
const CATALOG_API_BASE: &str = "https://formulae.brew.sh/api/";

/// HTTP timeout per fetch (whole-request). The catalog files are ~30 MB
/// and ~15 MB; on a typical home connection both arrive in well under
/// 30 seconds. 60 seconds leaves margin for slow networks without
/// stalling the UI thread indefinitely.
const REFRESH_TIMEOUT: Duration = Duration::from_secs(60);

/// User-Agent string for outbound catalog fetches.
const USER_AGENT: &str = "brew-browser/0.1 (+https://github.com/msitarzewski/brew-browser)";

// ---------- IPC payloads ----------

/// Summary surface for the Dashboard / Discover banner — small, snappy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSummary {
    pub as_of: String,
    /// "bundled" or "user-refreshed". (Matches `CatalogSource::as_wire`.)
    pub source: String,
    pub formula_count: usize,
    pub cask_count: usize,
    /// Days between `as_of` and now (UTC, server clock). Negative values
    /// are clamped to 0 (clock skew, future `as_of`).
    pub days_old: i64,
    /// True iff even the bundled catalog failed to parse. UI should
    /// show a fatal banner ("Catalog unavailable — please reinstall").
    pub corrupt: bool,
}

/// Light per-entry record for list views — narrower than the full
/// `Formula` / `Cask` so the IPC payload stays cheap.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogEntrySummary {
    pub name: String,
    pub desc: Option<String>,
    /// "Stable" version string — what `brew install <name>` would pull.
    /// For formulae this comes from `Formula.versions_stable`; for casks
    /// it's the top-level `Cask.version`. Optional because some entries
    /// (head-only formulae, vintage casks) genuinely have no value here.
    pub version: Option<String>,
    pub deprecated: bool,
    pub disabled: bool,
}

/// One reverse-dependent of a queried package: a source that declares
/// the queried token in one of its dependency arrays (or, for a cask,
/// in `depends_on.formula`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReverseDependent {
    /// Name (formula) or token (cask) of the dependent.
    pub name: String,
    /// "formula" or "cask".
    pub kind: ReverseDependentKind,
    /// How the dependent declares the edge.
    pub edge: ReverseDependentEdge,
}

/// Whether a reverse-dependent is a formula or a cask. Casks can depend
/// on formulae (via `depends_on.formula`); the reverse never occurs in
/// this dataset, so a cask's own reverse set is always empty.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ReverseDependentKind {
    Formula,
    Cask,
}

/// The dependency relationship that links a dependent to the queried
/// target. A cask edge is always classified `Required` (a cask's
/// `depends_on.formula` is a hard runtime requirement) but is
/// distinguished by `ReverseDependentKind::Cask`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReverseDependentEdge {
    Required,
    Build,
    Recommended,
    Optional,
}

impl ReverseDependentEdge {
    /// Deterministic precedence for dedupe — when a single source
    /// declares the target in multiple arrays we keep the *strongest*
    /// edge. Lower number = stronger. Mirrors the parity contract:
    /// required > recommended > build > optional.
    fn precedence(self) -> u8 {
        match self {
            ReverseDependentEdge::Required => 0,
            ReverseDependentEdge::Recommended => 1,
            ReverseDependentEdge::Build => 2,
            ReverseDependentEdge::Optional => 3,
        }
    }
}

/// Full reverse-dependents payload for one queried token.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReverseDependents {
    /// The token that was queried (echoed back for the UI).
    pub name: String,
    /// Dependents, deduped by (name, kind) keeping the strongest edge,
    /// sorted ascending by name. Empty when nothing depends on the
    /// queried token (honest leaf-node state).
    pub dependents: Vec<ReverseDependent>,
}

// ---------- Helpers ----------

/// Pure catalog-graph inversion. Returns every source that declares
/// `target` in a dependency array, classified by edge type, deduped by
/// (name, kind) keeping the strongest edge, and sorted ascending by
/// name.
///
/// `include_casks` controls whether cask `depends_on.formula` edges are
/// folded in — the caller passes `cfg!(target_os = "macos")` so the
/// cask surface is a macOS-only superset (casks are unavailable on
/// Linux). Formula→formula edges are always included.
///
/// Self-loops (a malformed record listing itself) are excluded. The
/// scan is a single linear pass over the catalog rather than a cached
/// inverted index: at ~8.4k formulae it is sub-millisecond, and an
/// on-demand pass avoids any `AppState` mutation or memoization
/// bookkeeping.
fn invert_dependents(catalog: &Catalog, target: &str, include_casks: bool) -> Vec<ReverseDependent> {
    use std::collections::HashMap;

    // Keyed by (name, kind) so a formula and a (hypothetical) cask of
    // the same name stay distinct. Value is the strongest edge seen.
    let mut best: HashMap<(String, ReverseDependentKind), ReverseDependentEdge> = HashMap::new();

    let mut consider = |source: &str, kind: ReverseDependentKind, edge: ReverseDependentEdge| {
        // Exclude self-loops — a record listing itself is malformed
        // catalog data, not a real dependency.
        if source == target {
            return;
        }
        let key = (source.to_string(), kind);
        best.entry(key)
            .and_modify(|existing| {
                if edge.precedence() < existing.precedence() {
                    *existing = edge;
                }
            })
            .or_insert(edge);
    };

    for f in catalog.formulae.values() {
        if f.dependencies.iter().any(|d| d == target) {
            consider(&f.name, ReverseDependentKind::Formula, ReverseDependentEdge::Required);
        }
        if f.build_dependencies.iter().any(|d| d == target) {
            consider(&f.name, ReverseDependentKind::Formula, ReverseDependentEdge::Build);
        }
        if f.recommended_dependencies.iter().any(|d| d == target) {
            consider(&f.name, ReverseDependentKind::Formula, ReverseDependentEdge::Recommended);
        }
        if f.optional_dependencies.iter().any(|d| d == target) {
            consider(&f.name, ReverseDependentKind::Formula, ReverseDependentEdge::Optional);
        }
    }

    if include_casks {
        for c in catalog.casks.values() {
            if c.depends_on_formula.iter().any(|d| d == target) {
                consider(&c.token, ReverseDependentKind::Cask, ReverseDependentEdge::Required);
            }
        }
    }

    let mut out: Vec<ReverseDependent> = best
        .into_iter()
        .map(|((name, kind), edge)| ReverseDependent { name, kind, edge })
        .collect();
    // Sort ascending by name, then kind, for a stable UI order even when
    // a formula and cask share a name.
    out.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| (a.kind as u8).cmp(&(b.kind as u8)))
    });
    out
}

fn summarize(catalog: &Catalog) -> CatalogSummary {
    let days_old = compute_days_old(&catalog.as_of);
    CatalogSummary {
        as_of: catalog.as_of.clone(),
        source: catalog.source.as_wire().to_string(),
        formula_count: catalog.formula_count,
        cask_count: catalog.cask_count,
        days_old,
        corrupt: catalog.corrupt,
    }
}

fn compute_days_old(as_of: &str) -> i64 {
    use chrono::{DateTime, Utc};
    if as_of.is_empty() {
        return 0;
    }
    let Ok(t) = as_of.parse::<DateTime<Utc>>() else {
        // Manifest produced by tools/catalog/fetch.py uses `%Y-%m-%dT%H:%M:%SZ`
        // which DateTime::from_str accepts via RFC 3339 — but be defensive
        // for hand-edited manifests.
        return 0;
    };
    let delta = Utc::now() - t;
    delta.num_days().max(0)
}

async fn read_active_catalog(state: &AppState) -> Arc<Catalog> {
    let guard = state.catalog.read().await;
    Arc::clone(&*guard)
}

// ---------- Commands ----------

#[tauri::command]
pub async fn catalog_summary(state: State<'_, AppState>) -> Result<CatalogSummary, BrewError> {
    let catalog = read_active_catalog(&state).await;
    Ok(summarize(&catalog))
}

#[tauri::command]
pub async fn catalog_refresh(state: State<'_, AppState>) -> Result<CatalogSummary, BrewError> {
    // The IPC command is a thin wrapper around `refresh_catalog_inner`
    // so the same logic can be triggered from the startup auto-refresh
    // helper (which has `&AppState`, not `State<'_, AppState>`).
    refresh_catalog_inner(&state).await
}

/// Inner refresh routine. Takes `&AppState` so it can be called from
/// both the IPC handler above and the startup auto-refresh helper
/// (`maybe_auto_refresh_catalog`). The function still consults
/// `require_network` and the single-flight mutex; both gates remain
/// authoritative regardless of caller.
pub(crate) async fn refresh_catalog_inner(state: &AppState) -> Result<CatalogSummary, BrewError> {
    // Paranoid-mode gate (Phase 12d). Replaces the prior TODO. With this
    // gate in place, the user's "block all outbound" master switch is
    // honoured by the catalog refresh just like every other network-
    // touching command.
    state.require_network("catalog_refresh").await?;

    // Single-flight enforcement. `try_lock` returns Err immediately if
    // a refresh is already in progress — the user's second click on the
    // Refresh button shouldn't queue, it should fast-fail with a typed
    // error so the UI can show "Already refreshing…".
    let _flight = match state.catalog_refresh_in_flight.try_lock() {
        Ok(guard) => guard,
        Err(_) => {
            return Err(BrewError::InvalidArgument {
                message: "catalog refresh already in progress".into(),
            });
        }
    };

    // Build a polite client.
    let client = reqwest::Client::builder()
        .timeout(REFRESH_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| BrewError::Network {
            url: CATALOG_API_BASE.to_string(),
            message: format!("client build: {e}"),
        })?;

    // Fetch both endpoints. Each goes through `fetch_capped` which
    // enforces the 64 MiB raw cap so a hostile mirror can't OOM us.
    let formula_raw = fetch_capped(&client, FORMULA_URL).await?;
    let cask_raw = fetch_capped(&client, CASK_URL).await?;

    // Quick structural sanity check before we commit anything to disk.
    let formula_count = count_top_level_array(&formula_raw, FORMULA_URL)?;
    let cask_count = count_top_level_array(&cask_raw, CASK_URL)?;

    // gzip both — this is what we'll persist + what `load_user_data`
    // expects on next launch.
    let formula_gz = gzip_compress(&formula_raw)?;
    let cask_gz = gzip_compress(&cask_raw)?;

    let manifest = Manifest {
        as_of: chrono::Utc::now().to_rfc3339(),
        formula_count,
        cask_count,
        formula_compressed_bytes: formula_gz.len() as u64,
        cask_compressed_bytes: cask_gz.len() as u64,
        fetched_from: CATALOG_API_BASE.to_string(),
    };

    Catalog::write_user_data(&state.app_data_dir, &formula_gz, &cask_gz, &manifest).await?;

    // Re-load the newly-written user-data copy through the same parser
    // the next launch will use — this catches any parse drift between
    // raw bytes and on-disk shape immediately rather than at next boot.
    let new_catalog = Catalog::load_user_data(&state.app_data_dir)
        .await?
        .ok_or_else(|| BrewError::Internal {
            message: "wrote user-data catalog but load returned None".into(),
        })?;
    let new_summary = summarize(&new_catalog);

    // Swap the AppState Arc — every subsequent reader sees the fresh
    // catalog. Existing readers holding a clone of the old Arc are fine;
    // we just drop our reference to it.
    {
        let mut guard = state.catalog.write().await;
        *guard = Arc::new(new_catalog);
    }

    Ok(new_summary)
}

#[tauri::command]
pub async fn catalog_lookup_formula(
    name: String,
    state: State<'_, AppState>,
) -> Result<Option<Formula>, BrewError> {
    // Defense in depth — even though the lookup is an in-memory HashMap
    // read with no path composition, validate so the IPC boundary stays
    // uniform with the rest of the surface (security review §12a).
    validate_package_name(&name)?;
    let catalog = read_active_catalog(&state).await;
    Ok(catalog.formulae.get(&name).cloned())
}

#[tauri::command]
pub async fn catalog_lookup_cask(
    name: String,
    state: State<'_, AppState>,
) -> Result<Option<Cask>, BrewError> {
    validate_cask_token(&name)?;
    let catalog = read_active_catalog(&state).await;
    Ok(catalog.casks.get(&name).cloned())
}

#[tauri::command]
pub async fn catalog_formulae_summary(
    state: State<'_, AppState>,
) -> Result<Vec<CatalogEntrySummary>, BrewError> {
    let catalog = read_active_catalog(&state).await;
    let mut out: Vec<CatalogEntrySummary> = catalog
        .formulae
        .values()
        .map(|f| CatalogEntrySummary {
            name: f.name.clone(),
            desc: f.desc.clone(),
            version: f.versions_stable.clone(),
            deprecated: f.deprecated,
            disabled: f.disabled,
        })
        .collect();
    // Stable order so the frontend can rely on it for paging / virtualization.
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

#[tauri::command]
pub async fn catalog_casks_summary(
    state: State<'_, AppState>,
) -> Result<Vec<CatalogEntrySummary>, BrewError> {
    // Casks are macOS-only — Homebrew on Linux has no cask support, and the
    // bundled catalog is built on macOS. Returning an empty summary here keeps
    // every downstream surface (Discover tiles, catalog search, category
    // counts) honestly cask-free on Linux instead of offering installs that
    // can only fail with "macOS is required for this software."
    if cfg!(not(target_os = "macos")) {
        return Ok(Vec::new());
    }
    let catalog = read_active_catalog(&state).await;
    let mut out: Vec<CatalogEntrySummary> = catalog
        .casks
        .values()
        .map(|c| CatalogEntrySummary {
            name: c.token.clone(),
            desc: c.desc.clone(),
            version: c.version.clone(),
            deprecated: c.deprecated,
            disabled: c.disabled,
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Reverse dependencies — every package in the catalog that depends on
/// `name`. Pure catalog-graph inversion: no `brew` subprocess, no
/// network, arch-independent for the formula→formula graph.
///
/// Cask dependents (a cask requiring this formula via
/// `depends_on.formula`) are a macOS-only superset — folded in only
/// under `cfg!(target_os = "macos")`, mirroring `catalog_casks_summary`.
/// On Linux the result is the formula→formula graph alone, which is
/// byte-identical to the macOS formula-graph result.
#[tauri::command]
pub async fn catalog_reverse_dependents(
    name: String,
    state: State<'_, AppState>,
) -> Result<ReverseDependents, BrewError> {
    // Defense in depth — the lookup is an in-memory scan with no path
    // composition, but validate so the IPC boundary stays uniform.
    validate_package_name(&name)?;
    let catalog = read_active_catalog(&state).await;
    let include_casks = cfg!(target_os = "macos");
    let dependents = invert_dependents(&catalog, &name, include_casks);
    Ok(ReverseDependents { name, dependents })
}

// ---------- Auto-refresh (Phase 13 — Finding 2) ----------

/// Decide whether the auto-refresh scheduler should kick off a fetch
/// right now. Pure function — extracted so the schedule logic can be
/// unit-tested without an `AppState`, network, or filesystem.
///
/// Returns `true` when:
/// - `schedule` is [`CatalogAutoRefresh::Daily`] and `age` >= 24 hours,
/// - `schedule` is [`CatalogAutoRefresh::Weekly`] and `age` >= 7 days.
///
/// Returns `false` for [`CatalogAutoRefresh::Off`] regardless of age,
/// and for any positive schedule whose age is below the threshold.
pub(crate) fn should_auto_refresh(schedule: CatalogAutoRefresh, age: Duration) -> bool {
    match schedule {
        CatalogAutoRefresh::Off => false,
        CatalogAutoRefresh::Daily => age >= Duration::from_secs(24 * 60 * 60),
        CatalogAutoRefresh::Weekly => age >= Duration::from_secs(7 * 24 * 60 * 60),
    }
}

/// Compute the age of the active catalog relative to `now`. Returns
/// `None` when `as_of` is empty or unparseable (in which case the
/// scheduler treats the catalog as "unknown age" and skips the refresh
/// rather than re-fetching on every boot — manual refresh remains
/// available as the recovery path).
fn catalog_age(as_of: &str, now: chrono::DateTime<chrono::Utc>) -> Option<Duration> {
    use chrono::DateTime;
    if as_of.is_empty() {
        return None;
    }
    let t = as_of.parse::<DateTime<chrono::Utc>>().ok()?;
    let delta = now.signed_duration_since(t);
    delta.to_std().ok()
}

/// Startup auto-refresh hook. Called once just after `AppState::build`
/// (see `state::initialize`) on a background tokio task so it never
/// blocks the setup hook.
///
/// Behaviour:
/// 1. Reads settings via `state.settings.read()`. `Off` → return.
///    `Corrupt` → also return (the `require_network` gate inside the
///    refresh would have denied anyway; we short-circuit one step
///    earlier so we don't even log an attempt).
/// 2. Reads the active catalog's `as_of`; unparseable → log + return.
/// 3. If `should_auto_refresh` returns true, calls `refresh_catalog_inner`.
///    Network errors are non-fatal — they are logged via `tracing` and
///    the user keeps the existing (stale) catalog. Manual refresh from
///    the Dashboard remains available as the recovery path.
///
/// Paranoid mode is handled transparently by `refresh_catalog_inner`'s
/// existing `require_network` gate.
pub async fn maybe_auto_refresh_catalog(state: &AppState) {
    // 1. Read the schedule from settings.
    let schedule = {
        let guard = state.settings.read().await;
        match &*guard {
            SettingsLoadState::Loaded(s) => s.catalog_auto_refresh,
            // FirstLaunch defaults to Off (current behaviour preserved);
            // Corrupt also short-circuits — the user has to repair
            // settings before any auto-refresh fires.
            _ => CatalogAutoRefresh::Off,
        }
    };
    if matches!(schedule, CatalogAutoRefresh::Off) {
        return;
    }

    // 2. Look up the active catalog's age.
    let as_of = {
        let guard = state.catalog.read().await;
        guard.as_of.clone()
    };
    let now = chrono::Utc::now();
    let Some(age) = catalog_age(&as_of, now) else {
        tracing::info!(
            "catalog auto-refresh: skipping — catalog as_of is empty or unparseable ({as_of:?})"
        );
        return;
    };

    // 3. Apply the schedule.
    if !should_auto_refresh(schedule, age) {
        tracing::debug!(
            "catalog auto-refresh: not due yet (schedule={schedule:?}, age={age:?})"
        );
        return;
    }

    tracing::info!(
        "catalog auto-refresh: scheduling refresh (schedule={schedule:?}, age={age:?})"
    );

    // 4. Run the refresh. Errors are non-fatal — log and move on. The
    // user keeps the stale catalog and can hit the Dashboard refresh
    // button if they want to retry immediately.
    match refresh_catalog_inner(state).await {
        Ok(summary) => {
            tracing::info!(
                "catalog auto-refresh: success ({} formulae, {} casks, as_of={})",
                summary.formula_count,
                summary.cask_count,
                summary.as_of
            );
        }
        Err(e) => {
            tracing::warn!("catalog auto-refresh: failed (non-fatal): {e:?}");
        }
    }
}

// ---------- Refresh internals ----------

/// Stream `url` into a `Vec<u8>`, capping at `MAX_CATALOG_BYTES`. The
/// per-chunk loop lets us reject oversize responses before allocating
/// the full body — a hostile mirror that promises 30 MB and streams
/// 30 GB gets cut off at 64 MiB.
async fn fetch_capped(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, BrewError> {
    let resp = client.get(url).send().await?;
    if !resp.status().is_success() {
        return Err(BrewError::HttpStatus {
            url: url.to_string(),
            status: resp.status().as_u16(),
        });
    }
    let mut bytes: Vec<u8> = Vec::with_capacity(8 * 1024 * 1024);
    let mut stream = resp;
    loop {
        let chunk = stream.chunk().await?;
        let Some(chunk) = chunk else { break };
        if bytes.len() as u64 + chunk.len() as u64 > MAX_CATALOG_BYTES {
            return Err(BrewError::Network {
                url: url.to_string(),
                message: format!(
                    "response exceeded {} byte cap",
                    MAX_CATALOG_BYTES
                ),
            });
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn gzip_compress(bytes: &[u8]) -> Result<Vec<u8>, BrewError> {
    use std::io::Write;
    let mut encoder =
        flate2::write::GzEncoder::new(Vec::with_capacity(bytes.len() / 4), flate2::Compression::best());
    encoder.write_all(bytes).map_err(|e| BrewError::Io {
        message: format!("gzip write: {e}"),
    })?;
    encoder.finish().map_err(|e| BrewError::Io {
        message: format!("gzip finish: {e}"),
    })
}

/// Count the elements in the top-level JSON array without fully
/// re-deserializing the records into typed structs. Used pre-write to
/// (a) catch totally non-JSON responses and (b) seed `Manifest.*_count`.
fn count_top_level_array(bytes: &[u8], url: &str) -> Result<usize, BrewError> {
    let v: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| BrewError::JsonParse {
            command: url.to_string(),
            message: e.to_string(),
            raw_excerpt: String::new(),
        })?;
    let arr = v.as_array().ok_or_else(|| BrewError::JsonParse {
        command: url.to_string(),
        message: "expected top-level JSON array".into(),
        raw_excerpt: String::new(),
    })?;
    Ok(arr.len())
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::CatalogSource;

    // Most tests don't have an AppState; they go through Catalog
    // directly. Command-level tests that need AppState live in the
    // integration tests folder; the unit tests here cover everything
    // testable in isolation.

    #[test]
    fn summary_carries_source_string() {
        // Build a minimal Catalog by hand (skipping the heavy load).
        let cat = Catalog {
            formulae: Default::default(),
            casks: Default::default(),
            as_of: "2026-05-24T00:00:00Z".to_string(),
            source: CatalogSource::Bundled,
            formula_count: 5,
            cask_count: 3,
            corrupt: false,
        };
        let s = summarize(&cat);
        assert_eq!(s.source, "bundled");
        assert_eq!(s.formula_count, 5);
        assert_eq!(s.cask_count, 3);
        assert!(!s.corrupt);
    }

    #[test]
    fn summary_user_refreshed_source_string() {
        let cat = Catalog {
            formulae: Default::default(),
            casks: Default::default(),
            as_of: "2026-05-24T00:00:00Z".to_string(),
            source: CatalogSource::UserRefreshed,
            formula_count: 0,
            cask_count: 0,
            corrupt: false,
        };
        assert_eq!(summarize(&cat).source, "user-refreshed");
    }

    #[test]
    fn compute_days_old_handles_empty() {
        assert_eq!(compute_days_old(""), 0);
    }

    #[test]
    fn compute_days_old_handles_bad_input() {
        assert_eq!(compute_days_old("not a date"), 0);
    }

    #[test]
    fn compute_days_old_clamps_future_dates_to_zero() {
        // Year 9999 should always be in the future for any practical run.
        let days = compute_days_old("9999-01-01T00:00:00Z");
        assert_eq!(days, 0);
    }

    #[test]
    fn compute_days_old_returns_positive_for_old_date() {
        // Year 2000 is far enough in the past that the result is huge
        // and positive.
        let days = compute_days_old("2000-01-01T00:00:00Z");
        assert!(days > 365 * 20);
    }

    #[test]
    fn gzip_round_trips() {
        let payload = b"hello world, this is a small test payload \xff\x00\x01";
        let gz = gzip_compress(payload).expect("compress");
        use std::io::Read;
        let mut d = flate2::read::GzDecoder::new(&gz[..]);
        let mut out = Vec::new();
        d.read_to_end(&mut out).unwrap();
        assert_eq!(out, payload);
    }

    #[test]
    fn count_top_level_array_counts_elements() {
        let bytes = br#"[{"a":1},{"a":2},{"a":3}]"#;
        let n = count_top_level_array(bytes, "test").expect("count");
        assert_eq!(n, 3);
    }

    #[test]
    fn count_top_level_array_rejects_non_array() {
        let bytes = br#"{"a":1}"#;
        let r = count_top_level_array(bytes, "test");
        assert!(matches!(r, Err(BrewError::JsonParse { .. })));
    }

    // ---------- Catalog-level tests ----------
    //
    // These use the global Catalog directly (no AppState) — they cover
    // the wiring catalog_lookup_* and *_summary commands rely on.

    #[tokio::test]
    async fn bundled_catalog_parses() {
        let cat = Catalog::load_bundled().expect("load bundled");
        assert!(cat.formulae.len() > 1000, "expected >1k formulae");
        assert!(cat.casks.len() > 1000, "expected >1k casks");
    }

    #[tokio::test]
    async fn lookup_known_formula() {
        let cat = Catalog::load_bundled().expect("load bundled");
        let f = cat.formulae.get("wget").cloned().expect("wget present");
        assert_eq!(f.name, "wget");
    }

    #[tokio::test]
    async fn lookup_unknown_returns_none() {
        let cat = Catalog::load_bundled().expect("load bundled");
        let f = cat.formulae.get("this-is-not-a-real-formula-xyzzy").cloned();
        assert!(f.is_none());
    }

    #[tokio::test]
    async fn deprecation_flag_surfaces() {
        // Any deprecated entry in the bundled snapshot proves the flag
        // round-trips through serde.
        let cat = Catalog::load_bundled().expect("load bundled");
        let any_dep = cat.formulae.values().any(|f| f.deprecated);
        assert!(any_dep, "expected at least one deprecated formula");
    }

    #[test]
    fn validate_blocks_invalid_name_for_formula_lookup() {
        // Mirrors what catalog_lookup_formula does first. The formula
        // validator accepts `/` and `.` (tap-qualified names like
        // `homebrew/core/wget` need them), so a path-traversal shape
        // like `../../etc/passwd` is silently treated as a non-match
        // (HashMap miss → Ok(None)). The validator IS still the IPC
        // boundary chokepoint; it must reject anything that would let
        // a flag injection or control-char attack through:
        for bad in &["", "-flag", "foo bar", "foo\0", "foo;bar"] {
            let r = validate_package_name(bad);
            assert!(
                matches!(r, Err(BrewError::InvalidArgument { .. })),
                "expected invalid_argument for {:?}, got {:?}",
                bad,
                r
            );
        }
    }

    #[test]
    fn validate_blocks_invalid_name_for_cask_lookup() {
        // The cask-token validator is stricter — it rejects `/` and
        // leading `.` outright, so traversal-shaped tokens never even
        // reach a HashMap lookup.
        let r = validate_cask_token("../../../etc/passwd");
        assert!(
            matches!(r, Err(BrewError::InvalidArgument { .. })),
            "expected invalid_argument for traversal-shaped token"
        );
        // Plus the same flag/control-char rejections.
        for bad in &["", "-flag", "foo bar", "foo\0", ".hidden"] {
            let r = validate_cask_token(bad);
            assert!(
                matches!(r, Err(BrewError::InvalidArgument { .. })),
                "expected invalid_argument for {:?}, got {:?}",
                bad,
                r
            );
        }
    }

    // ---------- Reverse dependents (Feature #1) ----------

    use crate::catalog::{Cask, Formula};

    /// Minimal formula with just the fields the inversion reads. All
    /// dependency arrays default empty; callers fill what they need.
    fn fixture_formula(name: &str) -> Formula {
        Formula {
            name: name.to_string(),
            full_name: name.to_string(),
            desc: None,
            homepage: None,
            license: None,
            deprecated: false,
            deprecation_date: None,
            deprecation_reason: None,
            disabled: false,
            disable_date: None,
            disable_reason: None,
            dependencies: Vec::new(),
            build_dependencies: Vec::new(),
            recommended_dependencies: Vec::new(),
            optional_dependencies: Vec::new(),
            conflicts_with: Vec::new(),
            versions_stable: None,
            tap: "homebrew/core".to_string(),
            aliases: Vec::new(),
        }
    }

    fn fixture_cask(token: &str) -> Cask {
        Cask {
            token: token.to_string(),
            name: Vec::new(),
            desc: None,
            homepage: None,
            deprecated: false,
            deprecation_date: None,
            deprecation_reason: None,
            disabled: false,
            version: None,
            tap: "homebrew/cask".to_string(),
            depends_on_formula: Vec::new(),
        }
    }

    /// Build a small in-memory catalog from the given formulae + casks.
    fn fixture_catalog(formulae: Vec<Formula>, casks: Vec<Cask>) -> Catalog {
        use std::collections::HashMap;
        let mut fmap = HashMap::new();
        for f in formulae {
            fmap.insert(f.name.clone(), f);
        }
        let mut cmap = HashMap::new();
        for c in casks {
            cmap.insert(c.token.clone(), c);
        }
        Catalog {
            formula_count: fmap.len(),
            cask_count: cmap.len(),
            formulae: fmap,
            casks: cmap,
            as_of: "2026-06-12T00:00:00Z".to_string(),
            source: CatalogSource::Bundled,
            corrupt: false,
        }
    }

    #[test]
    fn invert_returns_required_dependents() {
        let mut wget = fixture_formula("wget");
        wget.dependencies = vec!["openssl@3".to_string(), "gettext".to_string()];
        let mut curl = fixture_formula("curl");
        curl.dependencies = vec!["openssl@3".to_string()];
        let cat = fixture_catalog(vec![wget, curl, fixture_formula("openssl@3")], vec![]);

        let deps = invert_dependents(&cat, "openssl@3", true);
        let names: Vec<&str> = deps.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, vec!["curl", "wget"]);
        assert!(deps.iter().all(|d| d.edge == ReverseDependentEdge::Required));
        assert!(deps.iter().all(|d| d.kind == ReverseDependentKind::Formula));
    }

    #[test]
    fn invert_classifies_build_edge() {
        let mut wget = fixture_formula("wget");
        wget.build_dependencies = vec!["pkgconf".to_string()];
        let cat = fixture_catalog(vec![wget, fixture_formula("pkgconf")], vec![]);

        let deps = invert_dependents(&cat, "pkgconf", true);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "wget");
        assert_eq!(deps[0].edge, ReverseDependentEdge::Build);
    }

    #[test]
    fn invert_classifies_recommended_and_optional_distinctly() {
        let mut a = fixture_formula("a");
        a.recommended_dependencies = vec!["libfoo".to_string()];
        let mut b = fixture_formula("b");
        b.optional_dependencies = vec!["libfoo".to_string()];
        let cat = fixture_catalog(vec![a, b, fixture_formula("libfoo")], vec![]);

        let deps = invert_dependents(&cat, "libfoo", true);
        let a_edge = deps.iter().find(|d| d.name == "a").unwrap().edge;
        let b_edge = deps.iter().find(|d| d.name == "b").unwrap().edge;
        assert_eq!(a_edge, ReverseDependentEdge::Recommended);
        assert_eq!(b_edge, ReverseDependentEdge::Optional);
    }

    #[test]
    fn invert_leaf_with_no_dependents_is_empty() {
        let cat = fixture_catalog(vec![fixture_formula("loner")], vec![]);
        let deps = invert_dependents(&cat, "loner", true);
        assert!(deps.is_empty());
    }

    #[test]
    fn invert_excludes_self_loop() {
        // A malformed record listing itself must not appear in its own
        // reverse set.
        let mut weird = fixture_formula("weird");
        weird.dependencies = vec!["weird".to_string()];
        let cat = fixture_catalog(vec![weird], vec![]);
        let deps = invert_dependents(&cat, "weird", true);
        assert!(deps.is_empty(), "self-loop must be excluded");
    }

    #[test]
    fn invert_dedupes_with_required_over_build_precedence() {
        // A source that lists the target in BOTH dependencies and
        // build_dependencies dedupes to a single entry keeping the
        // stronger (required) edge.
        let mut src = fixture_formula("src");
        src.dependencies = vec!["target".to_string()];
        src.build_dependencies = vec!["target".to_string()];
        let cat = fixture_catalog(vec![src, fixture_formula("target")], vec![]);

        let deps = invert_dependents(&cat, "target", true);
        assert_eq!(deps.len(), 1, "duplicate source must dedupe");
        assert_eq!(deps[0].edge, ReverseDependentEdge::Required);
    }

    #[test]
    fn invert_cask_depends_on_formula_produces_cask_dependent() {
        let mut aptible = fixture_cask("aptible");
        aptible.depends_on_formula = vec!["libfido2".to_string()];
        let cat = fixture_catalog(vec![fixture_formula("libfido2")], vec![aptible]);

        let deps = invert_dependents(&cat, "libfido2", true);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "aptible");
        assert_eq!(deps[0].kind, ReverseDependentKind::Cask);
        assert_eq!(deps[0].edge, ReverseDependentEdge::Required);
    }

    #[test]
    fn invert_omits_casks_when_include_casks_false() {
        // Linux path: cask edges are excluded so the result is the
        // formula→formula graph alone.
        let mut aptible = fixture_cask("aptible");
        aptible.depends_on_formula = vec!["libfido2".to_string()];
        let cat = fixture_catalog(vec![fixture_formula("libfido2")], vec![aptible]);

        let deps = invert_dependents(&cat, "libfido2", false);
        assert!(deps.is_empty(), "cask edge must be omitted when include_casks=false");
    }

    #[test]
    fn invert_output_sorted_by_name() {
        let mut zeta = fixture_formula("zeta");
        zeta.dependencies = vec!["t".to_string()];
        let mut alpha = fixture_formula("alpha");
        alpha.dependencies = vec!["t".to_string()];
        let mut mid = fixture_formula("mid");
        mid.dependencies = vec!["t".to_string()];
        let cat = fixture_catalog(vec![zeta, alpha, mid, fixture_formula("t")], vec![]);

        let deps = invert_dependents(&cat, "t", true);
        let names: Vec<&str> = deps.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "mid", "zeta"]);
    }

    #[test]
    fn invert_real_bundled_high_fan_in_nonempty() {
        // openssl@3 is depended on by thousands of formulae in the real
        // bundled snapshot. Synthetic leaf returns empty.
        let cat = Catalog::load_bundled().expect("load bundled");
        let deps = invert_dependents(&cat, "openssl@3", true);
        assert!(
            deps.len() > 100,
            "openssl@3 should have a large reverse-dependent set, got {}",
            deps.len()
        );
        // Sorted ascending — verify the invariant holds on real data.
        for w in deps.windows(2) {
            assert!(w[0].name <= w[1].name, "reverse-dependents must be sorted by name");
        }

        let leaf = invert_dependents(&cat, "this-is-not-a-real-formula-xyzzy", true);
        assert!(leaf.is_empty(), "unknown token yields empty set");
    }

    #[test]
    fn invert_real_bundled_cask_dependent_present() {
        // The real catalog has 28 casks carrying depends_on.formula;
        // libfido2 is one such target (cask `aptible`).
        let cat = Catalog::load_bundled().expect("load bundled");
        let deps = invert_dependents(&cat, "libfido2", true);
        assert!(
            deps.iter().any(|d| d.kind == ReverseDependentKind::Cask),
            "libfido2 should have at least one cask dependent in the bundled snapshot"
        );
    }

    // ---------- Auto-refresh schedule (Phase 13 — Finding 2) ----------

    /// `Off` schedule never triggers regardless of how old the catalog is.
    #[test]
    fn auto_refresh_off_is_always_false() {
        assert!(!should_auto_refresh(
            CatalogAutoRefresh::Off,
            Duration::from_secs(0)
        ));
        assert!(!should_auto_refresh(
            CatalogAutoRefresh::Off,
            Duration::from_secs(365 * 24 * 60 * 60),
        ));
    }

    /// `Daily` + 25h → true (past the 24h threshold). This is the
    /// explicit acceptance criterion from the spec.
    #[test]
    fn auto_refresh_daily_at_25_hours_is_true() {
        let age = Duration::from_secs(25 * 60 * 60);
        assert!(should_auto_refresh(CatalogAutoRefresh::Daily, age));
    }

    /// `Daily` + 23h → false (still within the day window).
    #[test]
    fn auto_refresh_daily_at_23_hours_is_false() {
        let age = Duration::from_secs(23 * 60 * 60);
        assert!(!should_auto_refresh(CatalogAutoRefresh::Daily, age));
    }

    /// `Daily` + exactly 24h → true (inclusive threshold).
    #[test]
    fn auto_refresh_daily_at_24_hours_is_true() {
        let age = Duration::from_secs(24 * 60 * 60);
        assert!(should_auto_refresh(CatalogAutoRefresh::Daily, age));
    }

    /// `Weekly` + 8 days → true.
    #[test]
    fn auto_refresh_weekly_at_8_days_is_true() {
        let age = Duration::from_secs(8 * 24 * 60 * 60);
        assert!(should_auto_refresh(CatalogAutoRefresh::Weekly, age));
    }

    /// `Weekly` + 6 days → false.
    #[test]
    fn auto_refresh_weekly_at_6_days_is_false() {
        let age = Duration::from_secs(6 * 24 * 60 * 60);
        assert!(!should_auto_refresh(CatalogAutoRefresh::Weekly, age));
    }

    /// `Weekly` + exactly 7 days → true (inclusive threshold).
    #[test]
    fn auto_refresh_weekly_at_7_days_is_true() {
        let age = Duration::from_secs(7 * 24 * 60 * 60);
        assert!(should_auto_refresh(CatalogAutoRefresh::Weekly, age));
    }

    /// `Weekly` + zero age (fresh catalog) → false.
    #[test]
    fn auto_refresh_weekly_zero_age_is_false() {
        assert!(!should_auto_refresh(
            CatalogAutoRefresh::Weekly,
            Duration::ZERO
        ));
    }

    /// `Daily` + zero age (fresh catalog) → false.
    #[test]
    fn auto_refresh_daily_zero_age_is_false() {
        assert!(!should_auto_refresh(
            CatalogAutoRefresh::Daily,
            Duration::ZERO
        ));
    }

    // ---------- catalog_age helper ----------

    #[test]
    fn catalog_age_empty_string_is_none() {
        let now = chrono::Utc::now();
        assert!(catalog_age("", now).is_none());
    }

    #[test]
    fn catalog_age_unparseable_is_none() {
        let now = chrono::Utc::now();
        assert!(catalog_age("not a date", now).is_none());
    }

    #[test]
    fn catalog_age_computes_known_delta() {
        let now: chrono::DateTime<chrono::Utc> = "2026-05-24T12:00:00Z".parse().unwrap();
        // 25 hours earlier.
        let age = catalog_age("2026-05-23T11:00:00Z", now).expect("parse");
        assert_eq!(age, Duration::from_secs(25 * 60 * 60));
    }

    #[test]
    fn catalog_age_future_timestamp_is_none() {
        // A future `as_of` (clock skew, hand-edited manifest) yields a
        // negative chrono::Duration which `to_std()` rejects → None.
        // The scheduler treats this as "unknown age" and skips, which
        // is the safe behaviour.
        let now: chrono::DateTime<chrono::Utc> = "2026-05-24T12:00:00Z".parse().unwrap();
        let r = catalog_age("9999-01-01T00:00:00Z", now);
        assert!(r.is_none(), "future timestamps must collapse to None");
    }
}
