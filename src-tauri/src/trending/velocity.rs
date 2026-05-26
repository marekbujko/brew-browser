//! v0.4.0 — implied install velocity from rolling-window counts.
//!
//! Homebrew's published analytics expose three windows: 30d, 90d, 365d.
//! These are cumulative install counts over the period. Subtracting
//! gives per-window install deltas, which we can compare to a baseline
//! to produce a single scalar "velocity index" expressing whether a
//! package's recent install rate is above or below its annual average.
//!
//! The whole point: replace the naive sort-by-30d-installs (which puts
//! `ca-certificates`, `openssl@3`, `git` at the top forever because
//! every install pulls them as deps) with a sort that surfaces packages
//! whose adoption is actually accelerating.
//!
//! The collector in `tools/trending-collector/` records the same three
//! windows nightly so the historical view in v0.4.0+ can plot a real
//! daily trajectory. This module covers the always-on baseline that
//! works with only one fetch.

/// Compute the implied 30-day velocity index for a package given its
/// counts across the three published rolling windows.
///
/// Returns `Some(ratio)` where:
/// - `1.0` ≈ steady (recent monthly rate matches annual average)
/// - `> 1.5` → surging (recent rate is 50%+ above annual average)
/// - `< 0.7` → cooling (recent rate is 30%+ below annual average)
///
/// Returns `None` when the inputs don't support a stable estimate:
/// - `c365 == 0` (no historical data — nothing to compare against)
/// - `c365 < c90` or `c90 < c30` (rolling windows must be monotonic
///   non-decreasing; a violation indicates the input is corrupt or
///   the package was just renamed/added)
/// - Annual monthly average is < 1.0 (package is too rarely installed
///   for a stable ratio — small absolute numbers swing the ratio
///   wildly, which is misinformation)
pub fn velocity_index(c30: u64, c90: u64, c365: u64) -> Option<f64> {
    if c365 == 0 || c365 < c90 || c90 < c30 {
        return None;
    }
    // The 365-day window is ~12.17 months; normalize to per-month so
    // the ratio is dimensionless and intuitive ("this month vs. avg
    // month over the year").
    let monthly_avg_annual = (c365 as f64) / (365.0 / 30.0);
    if monthly_avg_annual < 1.0 {
        return None;
    }
    Some((c30 as f64) / monthly_avg_annual)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A package surging in popularity: most of its annual installs
    /// happened in the last 30 days.
    #[test]
    fn heating_returns_above_one_point_five() {
        // 30d=400, 90d=500, 365d=600 — most of annual happened recently.
        let v = velocity_index(400, 500, 600).expect("stable");
        assert!(v > 1.5, "expected surging, got {v}");
    }

    /// A package fading: most of its annual installs were in the older
    /// 275-day chunk.
    #[test]
    fn cooling_returns_below_one() {
        // 30d=10, 90d=100, 365d=1000 — 30d portion is only 1% of annual.
        let v = velocity_index(10, 100, 1000).expect("stable");
        assert!(v < 0.5, "expected cooling well below 1.0, got {v}");
    }

    /// A package steady over the year: the 30d window should be ~1/12
    /// of the 365d window. Velocity index should round near 1.0.
    #[test]
    fn steady_returns_near_one() {
        // 30d=100, 90d=300, 365d=1200 — perfectly steady rate.
        let v = velocity_index(100, 300, 1200).expect("stable");
        // 1200 / (365/30) ≈ 98.63 → 100 / 98.63 ≈ 1.0139
        assert!(
            (v - 1.0).abs() < 0.1,
            "expected ~1.0 for steady rate, got {v}"
        );
    }

    /// Degenerate inputs return None — defense against corrupt or
    /// just-listed packages where the rolling windows don't satisfy
    /// the monotonic non-decreasing invariant.
    #[test]
    fn returns_none_on_zero_annual_count() {
        assert!(velocity_index(0, 0, 0).is_none());
    }

    #[test]
    fn returns_none_on_non_monotonic_windows() {
        // 90d < 30d — impossible if windows are real cumulative counts.
        assert!(velocity_index(100, 50, 200).is_none());
        // 365d < 90d — same problem.
        assert!(velocity_index(50, 100, 80).is_none());
    }

    /// Very small absolute numbers → None. A package installed 5 times
    /// total over the year shouldn't surface as "surging" just because
    /// 4 of those happened recently (sample size is too small for the
    /// ratio to be meaningful).
    #[test]
    fn returns_none_on_too_few_installs() {
        // 365d count of 10 → monthly avg of 10/12.17 ≈ 0.82 < 1.0 → None.
        assert!(velocity_index(4, 7, 10).is_none());
    }

    /// Reasonable mid-tier package — exercises the normal path. Pinned
    /// to keep the formula honest if anyone refactors.
    #[test]
    fn typical_package_with_modest_growth() {
        // 30d=120, 90d=300, 365d=1000.
        // Monthly avg annual = 1000 / 12.17 ≈ 82.19.
        // Velocity = 120 / 82.19 ≈ 1.46. Slightly surging, not quite hot.
        let v = velocity_index(120, 300, 1000).expect("stable");
        assert!(
            (1.4..1.5).contains(&v),
            "expected ~1.46 for the documented inputs, got {v}"
        );
    }
}
