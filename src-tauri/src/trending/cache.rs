//! In-memory per-window cache for trending analytics.
//!
//! Per `backendApi.md` §6.3: TTL is 1 hour. Stale entries are kept
//! around so the client can fall back to them on a fetch failure.

use std::time::{Duration, Instant};

use crate::types::{TrendingReport, TrendingWindow};

pub const TRENDING_TTL: Duration = Duration::from_secs(60 * 60);

#[derive(Default)]
pub struct TrendingCache {
    pub d30: Option<CachedTrending>,
    pub d90: Option<CachedTrending>,
    pub d365: Option<CachedTrending>,
}

pub struct CachedTrending {
    pub fetched_at: Instant,
    pub report: TrendingReport,
    /// Full, uncapped name→install_count for this window (all formulae, not
    /// just the top-100 display set). Powers cross-window velocity so rising
    /// packages aren't dropped. See `commands::trending::build_velocity_map`.
    pub full_counts: std::collections::HashMap<String, u64>,
}

impl TrendingCache {
    pub fn get(&self, w: TrendingWindow) -> Option<&CachedTrending> {
        match w {
            TrendingWindow::D30 => self.d30.as_ref(),
            TrendingWindow::D90 => self.d90.as_ref(),
            TrendingWindow::D365 => self.d365.as_ref(),
        }
    }

    pub fn put(&mut self, w: TrendingWindow, cached: CachedTrending) {
        match w {
            TrendingWindow::D30 => self.d30 = Some(cached),
            TrendingWindow::D90 => self.d90 = Some(cached),
            TrendingWindow::D365 => self.d365 = Some(cached),
        }
    }

    pub fn clear(&mut self) {
        self.d30 = None;
        self.d90 = None;
        self.d365 = None;
    }

    #[allow(dead_code)]
    pub fn is_fresh(&self, w: TrendingWindow) -> bool {
        self.get(w)
            .map(|c| c.fetched_at.elapsed() < TRENDING_TTL)
            .unwrap_or(false)
    }
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TrendingReport;

    fn make_report(window: TrendingWindow) -> TrendingReport {
        TrendingReport {
            window,
            fetched_at: "2026-05-23T00:00:00Z".to_string(),
            cache_age_seconds: 0,
            total_count: 1000,
            entries: Vec::new(),
        }
    }

    fn make_entry(window: TrendingWindow, age: Duration) -> CachedTrending {
        CachedTrending {
            fetched_at: Instant::now() - age,
            report: make_report(window),
            full_counts: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn ttl_is_one_hour() {
        assert_eq!(TRENDING_TTL, Duration::from_secs(60 * 60));
    }

    #[test]
    fn empty_cache_returns_none_for_all_windows() {
        let c = TrendingCache::default();
        assert!(c.get(TrendingWindow::D30).is_none());
        assert!(c.get(TrendingWindow::D90).is_none());
        assert!(c.get(TrendingWindow::D365).is_none());
    }

    #[test]
    fn put_then_get_returns_inserted_entry() {
        let mut c = TrendingCache::default();
        c.put(TrendingWindow::D30, make_entry(TrendingWindow::D30, Duration::from_secs(0)));
        assert!(c.get(TrendingWindow::D30).is_some());
        // Other windows untouched.
        assert!(c.get(TrendingWindow::D90).is_none());
        assert!(c.get(TrendingWindow::D365).is_none());
    }

    #[test]
    fn per_window_isolation() {
        let mut c = TrendingCache::default();
        c.put(TrendingWindow::D30, make_entry(TrendingWindow::D30, Duration::ZERO));
        c.put(TrendingWindow::D90, make_entry(TrendingWindow::D90, Duration::ZERO));
        c.put(TrendingWindow::D365, make_entry(TrendingWindow::D365, Duration::ZERO));

        assert_eq!(c.get(TrendingWindow::D30).unwrap().report.window, TrendingWindow::D30);
        assert_eq!(c.get(TrendingWindow::D90).unwrap().report.window, TrendingWindow::D90);
        assert_eq!(c.get(TrendingWindow::D365).unwrap().report.window, TrendingWindow::D365);
    }

    #[test]
    fn is_fresh_true_for_recently_inserted() {
        let mut c = TrendingCache::default();
        c.put(TrendingWindow::D30, make_entry(TrendingWindow::D30, Duration::from_secs(60)));
        assert!(c.is_fresh(TrendingWindow::D30), "1-min-old entry should be fresh");
    }

    #[test]
    fn is_fresh_false_for_stale_entry() {
        let mut c = TrendingCache::default();
        // 2 hours old — well past 1h TTL.
        c.put(
            TrendingWindow::D30,
            make_entry(TrendingWindow::D30, Duration::from_secs(2 * 60 * 60)),
        );
        assert!(!c.is_fresh(TrendingWindow::D30), "2h-old entry should be stale");
    }

    #[test]
    fn is_fresh_false_for_missing_entry() {
        let c = TrendingCache::default();
        assert!(!c.is_fresh(TrendingWindow::D30));
        assert!(!c.is_fresh(TrendingWindow::D90));
        assert!(!c.is_fresh(TrendingWindow::D365));
    }

    #[test]
    fn stale_entry_still_retrievable_for_fallback() {
        // Per backendApi.md §6.3 — stale entries are kept around so the
        // trending_fetch flow can fall back on them when the live request fails.
        let mut c = TrendingCache::default();
        c.put(
            TrendingWindow::D30,
            make_entry(TrendingWindow::D30, Duration::from_secs(3 * 60 * 60)),
        );
        let entry = c.get(TrendingWindow::D30).expect("stale entry must remain in cache");
        assert!(
            entry.fetched_at.elapsed() > TRENDING_TTL,
            "entry must in fact be past TTL for this test to be meaningful"
        );
    }

    #[test]
    fn clear_evicts_all_windows() {
        let mut c = TrendingCache::default();
        c.put(TrendingWindow::D30, make_entry(TrendingWindow::D30, Duration::ZERO));
        c.put(TrendingWindow::D90, make_entry(TrendingWindow::D90, Duration::ZERO));
        c.put(TrendingWindow::D365, make_entry(TrendingWindow::D365, Duration::ZERO));

        c.clear();
        assert!(c.get(TrendingWindow::D30).is_none());
        assert!(c.get(TrendingWindow::D90).is_none());
        assert!(c.get(TrendingWindow::D365).is_none());
    }

    #[test]
    fn put_replaces_existing_entry() {
        let mut c = TrendingCache::default();
        c.put(TrendingWindow::D30, make_entry(TrendingWindow::D30, Duration::from_secs(100)));
        let old_time = c.get(TrendingWindow::D30).unwrap().fetched_at;
        // Sleep is not required because Instant is monotonic per-process; new value differs.
        c.put(TrendingWindow::D30, make_entry(TrendingWindow::D30, Duration::ZERO));
        let new_time = c.get(TrendingWindow::D30).unwrap().fetched_at;
        assert!(new_time > old_time, "put must replace the previous entry");
    }
}
