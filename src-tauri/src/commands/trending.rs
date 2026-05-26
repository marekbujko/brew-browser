//! Trending tab commands: `trending_fetch` and `trending_clear_cache`.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use tauri::State;

use crate::commands::settings::{Settings, SettingsLoadState};
use crate::error::BrewError;
use crate::state::AppState;
use crate::trending::cache::CachedTrending;
use crate::trending::{client, velocity};
use crate::types::{TrendingReport, TrendingWindow};

/// The three windows whose counts feed the velocity computation. Pinned
/// here as a const so the cache-warmup loop and the velocity back-fill
/// stay in lockstep.
const ALL_WINDOWS: [TrendingWindow; 3] = [
    TrendingWindow::D30,
    TrendingWindow::D90,
    TrendingWindow::D365,
];

/// Resolve the effective trending cache TTL from the persisted settings
/// (Phase 12d). The cache itself is TTL-agnostic — freshness is decided
/// here against the user's preference so a `settings_set` is honoured
/// on the very next `trending_fetch` without restarting the process.
///
/// **Fail-closed semantics:** when settings are `Corrupt`, the
/// `require_network` gate has already denied the call before we got
/// here, so this helper only ever runs on `Loaded` or `FirstLaunch`.
/// Both yield the default TTL when the setting is absent.
pub(crate) fn effective_trending_ttl(settings_state: &SettingsLoadState) -> Duration {
    let minutes = match settings_state {
        SettingsLoadState::Loaded(s) => s.trending_ttl_minutes,
        // FirstLaunch / Corrupt both fall through to the default. The
        // Corrupt branch is unreachable in practice (require_network has
        // already returned ParanoidModeBlocked) but we still hand back
        // the default rather than panic — defensive only.
        _ => Settings::default().trending_ttl_minutes,
    };
    Duration::from_secs(u64::from(minutes) * 60)
}

#[tauri::command]
pub async fn trending_fetch(
    window: TrendingWindow,
    state: State<'_, AppState>,
) -> Result<TrendingReport, BrewError> {
    // Paranoid-mode gate (Phase 12d). Even a stale cache hit is
    // disallowed in paranoid mode — the user's expectation is "no
    // outbound calls would happen here", and even though the cached
    // path doesn't hit the network, returning fresh-looking data
    // contradicts the toggle. The cost is zero (the gate is a single
    // RwLock read), the policy is unambiguous.
    state.require_network("trending_fetch").await?;

    // Resolve TTL from settings (Phase 13 — Finding 2 follow-up). The
    // hardcoded `TRENDING_TTL` constant in `trending::cache` is now the
    // *default* baseline only; the live decision uses the user's
    // configured `trending_ttl_minutes` (clamped 5..=1440).
    let ttl = {
        let guard = state.settings.read().await;
        effective_trending_ttl(&guard)
    };

    let installed_set = build_installed_set(&state).await;

    // v0.4.0 — ensure ALL THREE windows are cached so we can compute
    // velocity_index for the returned entries. On a cold cache this
    // triggers 6 parallel HTTP requests (3 windows × 2 endpoints,
    // install + install-on-request). Subsequent calls within TTL hit
    // cache for all three and skip the network entirely.
    //
    // Velocity is the headline feature of the Trending tab in v0.4.0,
    // so eager warm-up is the right trade-off: 6× one-shot cost in
    // exchange for "velocity is always available, never racing in
    // after the user sees the list".
    ensure_all_windows_cached(&state, &installed_set, ttl).await;

    // Build the velocity map from whatever's cached. If any window's
    // refresh failed (and we have no fallback), the map omits those
    // packages and velocity stays None on their entries — degrades
    // gracefully rather than failing the whole tab.
    let velocity_map = build_velocity_map(&state).await;

    // Now return the requested window. Critical: even if its cache
    // entry is stale and the refresh in `ensure_all_windows_cached`
    // failed, we still serve the stale data with a stale age — same
    // fallback behaviour as v0.3.x.
    let cache = state.trending_cache.lock().await;
    match cache.get(window) {
        Some(cached) => {
            let mut report = cached.report.clone();
            report.cache_age_seconds = cached.fetched_at.elapsed().as_secs();
            // Back-fill velocity from the cross-window map.
            for entry in &mut report.entries {
                entry.velocity_index = velocity_map.get(&entry.name).copied().flatten();
            }
            Ok(report)
        }
        None => Err(BrewError::Network {
            url: format!(
                "https://formulae.brew.sh/api/analytics/install/{}.json",
                window.as_path_segment()
            ),
            message: "all three window fetches failed".into(),
        }),
    }
}

/// v0.4.0 — fan out fetches for every window not currently fresh in
/// cache. Soft-fails per-window so a transient 5xx on one doesn't
/// poison the others. Updates the cache with whatever succeeded.
async fn ensure_all_windows_cached(
    state: &AppState,
    installed: &HashSet<String>,
    ttl: Duration,
) {
    // Quick scan under the lock — identify stale/missing windows.
    let stale: Vec<TrendingWindow> = {
        let cache = state.trending_cache.lock().await;
        ALL_WINDOWS
            .iter()
            .copied()
            .filter(|w| {
                cache
                    .get(*w)
                    .is_none_or(|c| c.fetched_at.elapsed() >= ttl)
            })
            .collect()
    };
    if stale.is_empty() {
        return;
    }

    // Fan out via JoinSet — each `client::fetch` already runs install
    // + install-on-request in parallel, so 3 stale windows = 6
    // concurrent requests. tokio's runtime handles the multiplexing.
    // JoinSet (vs. `futures::future::join_all`) keeps us on the
    // tokio-only dependency footprint and lets each task be cancelled
    // independently if a future need arose.
    let mut set: tokio::task::JoinSet<(TrendingWindow, Result<TrendingReport, BrewError>)> =
        tokio::task::JoinSet::new();
    for w in stale {
        let installed_owned: HashSet<String> = installed.clone();
        set.spawn(async move { (w, client::fetch(w, &installed_owned).await) });
    }
    let mut results: Vec<(TrendingWindow, Result<TrendingReport, BrewError>)> = Vec::new();
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok(pair) => results.push(pair),
            Err(e) => eprintln!("[trending] window fetch task panicked: {e}"),
        }
    }

    // Write successful results back to cache. Failed windows leave
    // whatever stale entry was previously there (the fall-back-to-
    // stale-on-error contract from v0.3.x).
    let mut cache = state.trending_cache.lock().await;
    for (w, res) in results {
        match res {
            Ok(report) => cache.put(
                w,
                CachedTrending {
                    fetched_at: Instant::now(),
                    report,
                },
            ),
            Err(e) => {
                eprintln!(
                    "[trending] window {} fetch failed (keeping stale if present): {}",
                    w.as_path_segment(),
                    e
                );
            }
        }
    }
}

/// v0.4.0 — build a `name → Option<velocity_index>` map by joining
/// install counts across whatever windows are in cache. Packages
/// missing from any window get `None`. The `Option<Option<f64>>`
/// flattening at the call site preserves the "missing entirely"
/// vs "present but velocity_index could not be computed" distinction.
async fn build_velocity_map(state: &AppState) -> HashMap<String, Option<f64>> {
    let cache = state.trending_cache.lock().await;

    let extract = |w: TrendingWindow| -> HashMap<String, u64> {
        cache
            .get(w)
            .map(|c| {
                c.report
                    .entries
                    .iter()
                    .map(|e| (e.name.clone(), e.install_count))
                    .collect()
            })
            .unwrap_or_default()
    };

    let c30 = extract(TrendingWindow::D30);
    let c90 = extract(TrendingWindow::D90);
    let c365 = extract(TrendingWindow::D365);
    drop(cache);

    // Union of names present in all three maps — only join where we
    // have data for every window.
    let mut out = HashMap::with_capacity(c30.len());
    for (name, count_30) in &c30 {
        if let (Some(count_90), Some(count_365)) = (c90.get(name), c365.get(name)) {
            out.insert(
                name.clone(),
                velocity::velocity_index(*count_30, *count_90, *count_365),
            );
        }
    }
    out
}

#[tauri::command]
pub async fn trending_clear_cache(state: State<'_, AppState>) -> Result<(), BrewError> {
    let mut cache = state.trending_cache.lock().await;
    cache.clear();
    Ok(())
}

// ---------- v0.4.0: Trending history (opt-in) ----------

/// Fetch the trending-history summary blob (top-N packages with
/// velocity index + compact sparkline). Frontend Trending tab calls
/// this once on mount and uses the data to render inline sparklines
/// per row without per-row HTTP.
///
/// Gated by [`AppState::require_enhanced_trending`] — fails closed
/// with `ParanoidModeBlocked` if paranoid is on, with `FeatureDisabled`
/// if the per-feature toggle is off. Falls back to stale cache on
/// network failure (same contract as `trending_fetch`).
#[tauri::command]
pub async fn trending_history_index(
    state: State<'_, AppState>,
) -> Result<crate::types::TrendingHistoryIndex, BrewError> {
    state.require_enhanced_trending().await?;

    // Short critical section: serve fresh cache if available.
    {
        let cache = state.trending_history_cache.lock().await;
        if cache.is_index_fresh() {
            if let Some(cached) = cache.get_index() {
                let mut index = cached.index.clone();
                index.cache_age_seconds = cached.fetched_at.elapsed().as_secs();
                return Ok(index);
            }
        }
    }

    // Fetch fresh.
    match crate::trending::history::client::fetch_index().await {
        Ok(index) => {
            let mut cache = state.trending_history_cache.lock().await;
            cache.put_index(index.clone());
            Ok(index)
        }
        Err(e) => {
            // Fall back to stale cache if any.
            let cache = state.trending_history_cache.lock().await;
            if let Some(cached) = cache.get_index() {
                let mut index = cached.index.clone();
                index.cache_age_seconds = cached.fetched_at.elapsed().as_secs();
                return Ok(index);
            }
            Err(e)
        }
    }
}

/// Fetch the per-package trending-history series. PackageDetail's
/// sparkline calls this on demand when the panel opens for a given
/// package. Cached for 6h on a per-package LRU.
#[tauri::command]
pub async fn trending_history_fetch(
    name: String,
    kind: crate::types::PackageKind,
    state: State<'_, AppState>,
) -> Result<crate::types::TrendingHistorySeries, BrewError> {
    state.require_enhanced_trending().await?;

    let key = crate::trending::history::cache::HistoryKey {
        name: name.clone(),
        kind,
    };

    // Short critical section: serve fresh cache if available.
    {
        let cache = state.trending_history_cache.lock().await;
        if cache.is_series_fresh(&key) {
            if let Some(cached) = cache.get_series(&key) {
                let mut series = cached.series.clone();
                series.cache_age_seconds = cached.fetched_at.elapsed().as_secs();
                return Ok(series);
            }
        }
    }

    // Fetch fresh.
    match crate::trending::history::client::fetch_package(&name, kind).await {
        Ok(series) => {
            let mut cache = state.trending_history_cache.lock().await;
            cache.put_series(key, series.clone());
            Ok(series)
        }
        Err(e) => {
            // Fall back to stale cache if any.
            let cache = state.trending_history_cache.lock().await;
            if let Some(cached) = cache.get_series(&key) {
                let mut series = cached.series.clone();
                series.cache_age_seconds = cached.fetched_at.elapsed().as_secs();
                return Ok(series);
            }
            Err(e)
        }
    }
}

async fn build_installed_set(state: &AppState) -> HashSet<String> {
    let cache = state.installed_cache.read().await;
    let mut set = HashSet::new();
    if let Some(list) = cache.as_ref() {
        for p in list.formulae.iter().chain(list.casks.iter()) {
            set.insert(p.name.clone());
        }
    }
    set
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::settings::Settings;
    use crate::trending::cache::TRENDING_TTL;

    #[test]
    fn effective_ttl_uses_loaded_setting() {
        // 5-minute setting → 300-second TTL.
        let s = Settings {
            trending_ttl_minutes: 5,
            ..Settings::default()
        };
        let ttl = effective_trending_ttl(&SettingsLoadState::Loaded(s));
        assert_eq!(ttl, Duration::from_secs(5 * 60));
    }

    #[test]
    fn effective_ttl_uses_default_on_first_launch() {
        // No settings file yet → fall back to the struct default
        // (60 minutes, matching the historical TRENDING_TTL constant).
        let ttl = effective_trending_ttl(&SettingsLoadState::FirstLaunch);
        assert_eq!(ttl, TRENDING_TTL);
        assert_eq!(ttl, Duration::from_secs(60 * 60));
    }

    #[test]
    fn effective_ttl_uses_default_on_corrupt() {
        // Corrupt is unreachable in practice (require_network denies
        // first), but the helper is defensive and falls back to the
        // default rather than panicking.
        let ttl = effective_trending_ttl(&SettingsLoadState::Corrupt {
            message: "boom".into(),
        });
        assert_eq!(ttl, TRENDING_TTL);
    }

    #[test]
    fn effective_ttl_max_setting() {
        let s = Settings {
            trending_ttl_minutes: 1440, // 24h, the clamp ceiling
            ..Settings::default()
        };
        let ttl = effective_trending_ttl(&SettingsLoadState::Loaded(s));
        assert_eq!(ttl, Duration::from_secs(24 * 60 * 60));
    }

    /// Core gate test: a cache entry inserted 10 minutes ago must be
    /// considered stale when the user's `trending_ttl_minutes` is 5.
    /// This is the explicit acceptance criterion from the spec — proves
    /// the setting actually affects the cache decision rather than the
    /// hardcoded 60-minute constant.
    #[test]
    fn ttl_setting_makes_old_cache_stale() {
        use crate::trending::cache::{CachedTrending, TrendingCache};
        use crate::types::{TrendingReport, TrendingWindow};

        let s = Settings {
            trending_ttl_minutes: 5, // 5 minute TTL
            ..Settings::default()
        };
        let ttl = effective_trending_ttl(&SettingsLoadState::Loaded(s));
        assert_eq!(ttl, Duration::from_secs(5 * 60));

        // Plant a cache entry that's 10 minutes old.
        let mut cache = TrendingCache::default();
        cache.put(
            TrendingWindow::D30,
            CachedTrending {
                fetched_at: Instant::now() - Duration::from_secs(10 * 60),
                report: TrendingReport {
                    window: TrendingWindow::D30,
                    fetched_at: "2026-05-24T00:00:00Z".into(),
                    cache_age_seconds: 0,
                    total_count: 0,
                    entries: Vec::new(),
                },
            },
        );

        // Mirror the freshness check from `trending_fetch`: with the
        // configured TTL of 5 min, the 10-min-old entry must be stale.
        let entry = cache.get(TrendingWindow::D30).expect("planted");
        let age = entry.fetched_at.elapsed();
        assert!(
            age >= ttl,
            "entry age {age:?} must be >= TTL {ttl:?} for the stale-check to fire"
        );

        // Sanity: under the historical 60-minute TTL, the same entry
        // would have been considered fresh — confirms the setting is
        // what's changing the decision.
        assert!(age < TRENDING_TTL, "10 min < 60 min default → would have been fresh");
    }
}
