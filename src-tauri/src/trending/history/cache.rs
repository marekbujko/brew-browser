//! Per-package in-memory cache for trending-history series.
//!
//! TTL is 6 hours — the collector regenerates the static files
//! nightly, so anything within the same day is effectively immutable
//! from the app's perspective. Index blob and per-package series share
//! the same TTL.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::types::{PackageKind, TrendingHistoryIndex, TrendingHistorySeries};

/// 6 hours. Longer than the always-on trending TTL (1h default)
/// because the underlying data only changes nightly.
pub const TRENDING_HISTORY_TTL: Duration = Duration::from_secs(6 * 60 * 60);

/// Hard cap on per-package entries. The series payload is small (~30
/// daily points of u64 = under 1 KB serialized) so 500 packages = under
/// 500 KB RSS, which is fine. Prevents unbounded growth if the user
/// scrolls through many detail panels.
pub const TRENDING_HISTORY_MAX_PACKAGES: usize = 500;

/// Cache key for per-package series.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct HistoryKey {
    pub name: String,
    pub kind: PackageKind,
}

#[derive(Default)]
pub struct TrendingHistoryCache {
    /// Index blob — single slot, refreshed lazily on staleness.
    pub index: Option<CachedIndex>,
    /// Per-package series — LRU-by-insertion. Old entries get evicted
    /// when the map grows past `TRENDING_HISTORY_MAX_PACKAGES`.
    pub series: HashMap<HistoryKey, CachedSeries>,
}

pub struct CachedIndex {
    pub fetched_at: Instant,
    pub index: TrendingHistoryIndex,
}

pub struct CachedSeries {
    pub fetched_at: Instant,
    pub series: TrendingHistorySeries,
}

impl TrendingHistoryCache {
    /// Returns the cached index if present, regardless of freshness.
    /// Freshness is checked at the command layer so a stale entry can
    /// be served on a fetch failure.
    pub fn get_index(&self) -> Option<&CachedIndex> {
        self.index.as_ref()
    }

    pub fn put_index(&mut self, index: TrendingHistoryIndex) {
        self.index = Some(CachedIndex {
            fetched_at: Instant::now(),
            index,
        });
    }

    pub fn get_series(&self, key: &HistoryKey) -> Option<&CachedSeries> {
        self.series.get(key)
    }

    pub fn put_series(&mut self, key: HistoryKey, series: TrendingHistorySeries) {
        // Eviction: if at cap, drop the oldest entry by `fetched_at`.
        if self.series.len() >= TRENDING_HISTORY_MAX_PACKAGES
            && !self.series.contains_key(&key)
        {
            if let Some(oldest_key) = self
                .series
                .iter()
                .min_by_key(|(_, v)| v.fetched_at)
                .map(|(k, _)| k.clone())
            {
                self.series.remove(&oldest_key);
            }
        }
        self.series.insert(
            key,
            CachedSeries {
                fetched_at: Instant::now(),
                series,
            },
        );
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.index = None;
        self.series.clear();
    }

    pub fn is_index_fresh(&self) -> bool {
        self.index
            .as_ref()
            .map(|c| c.fetched_at.elapsed() < TRENDING_HISTORY_TTL)
            .unwrap_or(false)
    }

    pub fn is_series_fresh(&self, key: &HistoryKey) -> bool {
        self.series
            .get(key)
            .map(|c| c.fetched_at.elapsed() < TRENDING_HISTORY_TTL)
            .unwrap_or(false)
    }
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TrendingHistorySource;

    fn make_index() -> TrendingHistoryIndex {
        TrendingHistoryIndex {
            generated_at: "2026-05-26T03:00:00Z".into(),
            packages: Vec::new(),
            cache_age_seconds: 0,
        }
    }

    fn make_series(name: &str) -> TrendingHistorySeries {
        TrendingHistorySeries {
            name: name.into(),
            kind: PackageKind::Formula,
            points: vec![crate::types::TrendingHistoryPoint {
                date: "2026-05-26".into(),
                count_30d: Some(100),
                count_90d: None,
                count_365d: None,
                count_install_on_request_30d: None,
                estimated_daily_installs: None,
                source: TrendingHistorySource::Daily,
            }],
            generated_at: "2026-05-26T03:00:00Z".into(),
            cache_age_seconds: 0,
        }
    }

    fn key(name: &str) -> HistoryKey {
        HistoryKey {
            name: name.into(),
            kind: PackageKind::Formula,
        }
    }

    #[test]
    fn ttl_is_six_hours() {
        assert_eq!(TRENDING_HISTORY_TTL, Duration::from_secs(6 * 60 * 60));
    }

    #[test]
    fn empty_cache_returns_none() {
        let c = TrendingHistoryCache::default();
        assert!(c.get_index().is_none());
        assert!(c.get_series(&key("wget")).is_none());
    }

    #[test]
    fn put_then_get_index() {
        let mut c = TrendingHistoryCache::default();
        c.put_index(make_index());
        assert!(c.get_index().is_some());
        assert!(c.is_index_fresh(), "just-inserted index must be fresh");
    }

    #[test]
    fn put_then_get_series() {
        let mut c = TrendingHistoryCache::default();
        c.put_series(key("wget"), make_series("wget"));
        assert!(c.get_series(&key("wget")).is_some());
        assert!(c.is_series_fresh(&key("wget")));
        // Different package → miss.
        assert!(c.get_series(&key("git")).is_none());
    }

    #[test]
    fn put_series_replaces_existing_entry() {
        let mut c = TrendingHistoryCache::default();
        c.put_series(key("wget"), make_series("wget"));
        let first = c.get_series(&key("wget")).unwrap().fetched_at;
        c.put_series(key("wget"), make_series("wget"));
        let second = c.get_series(&key("wget")).unwrap().fetched_at;
        assert!(second >= first, "replacement must update fetched_at");
        assert_eq!(c.series.len(), 1);
    }

    #[test]
    fn evicts_oldest_at_cap() {
        let mut c = TrendingHistoryCache::default();
        // Fill at cap with synthetic entries.
        for i in 0..TRENDING_HISTORY_MAX_PACKAGES {
            c.put_series(key(&format!("pkg{i}")), make_series(&format!("pkg{i}")));
        }
        assert_eq!(c.series.len(), TRENDING_HISTORY_MAX_PACKAGES);

        // Insert one more → oldest gets evicted.
        c.put_series(key("newcomer"), make_series("newcomer"));
        assert_eq!(c.series.len(), TRENDING_HISTORY_MAX_PACKAGES);
        assert!(
            c.get_series(&key("newcomer")).is_some(),
            "newcomer must be inserted"
        );
        // pkg0 was inserted first → expect it evicted. (Insertion order
        // matches Instant::now ordering since each put_series creates
        // a fresh Instant.)
        assert!(
            c.get_series(&key("pkg0")).is_none(),
            "oldest entry (pkg0) must have been evicted"
        );
    }

    #[test]
    fn clear_evicts_everything() {
        let mut c = TrendingHistoryCache::default();
        c.put_index(make_index());
        c.put_series(key("wget"), make_series("wget"));
        c.clear();
        assert!(c.get_index().is_none());
        assert!(c.get_series(&key("wget")).is_none());
    }
}
