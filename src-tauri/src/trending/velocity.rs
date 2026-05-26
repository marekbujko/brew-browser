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
/// Returns `Some(ratio)` where the recent 30-day install rate is
/// compared against the **prior 11 months** (not the whole year —
/// otherwise the recent month double-counts as part of the baseline,
/// which makes brand-new packages always look 12× as fast as average):
/// - `1.0` ≈ steady (recent month matches the prior 11 months' average)
/// - `> 1.5` → surging
/// - `< 0.7` → cooling
///
/// Returns `None` when the inputs don't support a stable estimate:
/// - `c365 == 0` (no historical data)
/// - `c365 < c90` or `c90 < c30` (rolling windows must be monotonic
///   non-decreasing; violation indicates corrupt input or a just-renamed
///   package)
/// - `c365 == c30` (package has zero history before the recent month —
///   nothing to compare against). This is the load-bearing change vs
///   the v0.4.0-beta formula: brand-new packages no longer dominate
///   the leaderboard with the maximum 12.17 ratio.
/// - Prior-11-month monthly average < 1.0 (too few absolute installs
///   for a stable ratio)
pub fn velocity_index(c30: u64, c90: u64, c365: u64) -> Option<f64> {
    if c365 == 0 || c365 < c90 || c90 < c30 {
        return None;
    }
    // Installs in days 31..365 (the prior 11 months).
    let older_installs = c365 - c30;
    if older_installs == 0 {
        return None;
    }
    // 335 days = 365 - 30. Normalize to per-30-day so the ratio is
    // dimensionless and reads as "this month vs prior month-average."
    let older_monthly_avg = (older_installs as f64) / (335.0 / 30.0);
    if older_monthly_avg < 1.0 {
        return None;
    }
    Some((c30 as f64) / older_monthly_avg)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A package surging in popularity: most of its annual installs
    /// happened in the last 30 days.
    #[test]
    fn heating_returns_above_one_point_five() {
        // 30d=400, 90d=500, 365d=600. Older = 200, older_monthly = 200/11.17 ≈ 17.9.
        // Velocity = 400 / 17.9 ≈ 22.3 — strongly surging.
        let v = velocity_index(400, 500, 600).expect("stable");
        assert!(v > 1.5, "expected surging, got {v}");
    }

    /// A package fading: most of its annual installs were in the older
    /// 11-month chunk.
    #[test]
    fn cooling_returns_below_one() {
        // 30d=10, 90d=100, 365d=1000. Older = 990, older_monthly ≈ 88.6.
        // Velocity = 10 / 88.6 ≈ 0.11 — strongly cooling.
        let v = velocity_index(10, 100, 1000).expect("stable");
        assert!(v < 0.5, "expected cooling well below 1.0, got {v}");
    }

    /// A package with a steady install rate over the year: the recent
    /// 30 days should match the prior 11-month per-30-day average.
    #[test]
    fn steady_returns_near_one() {
        // 30d=100, 90d=300, 365d=1200. Older = 1100, older_monthly = 1100/11.17 ≈ 98.5.
        // Velocity = 100 / 98.5 ≈ 1.015.
        let v = velocity_index(100, 300, 1200).expect("stable");
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
        // 30d=4, 90d=7, 365d=10. Older = 6, older_monthly = 6/11.17 ≈ 0.54 < 1.0 → None.
        assert!(velocity_index(4, 7, 10).is_none());
    }

    /// **Load-bearing fix:** brand-new packages where c30 == c365 used
    /// to return the maximum-possible 12.17 velocity (because the
    /// "annual baseline" was identical to the recent month). The new
    /// formula requires non-zero installs in days 31..365 — a
    /// brand-new package has zero, so it correctly returns None and
    /// stays out of the velocity leaderboard.
    #[test]
    fn returns_none_when_brand_new_package_has_no_prior_history() {
        // c30 == c90 == c365 → no prior history. Used to be a 12.17.
        assert!(velocity_index(59, 59, 59).is_none());
        assert!(velocity_index(195, 195, 195).is_none());
        assert!(velocity_index(2535, 2535, 2535).is_none());
    }

    /// Reasonable mid-tier package — exercises the normal path. Pinned
    /// to keep the formula honest if anyone refactors.
    #[test]
    fn typical_package_with_modest_growth() {
        // 30d=120, 90d=300, 365d=1000. Older = 880, older_monthly = 880/11.17 ≈ 78.8.
        // Velocity = 120 / 78.8 ≈ 1.523.
        let v = velocity_index(120, 300, 1000).expect("stable");
        assert!(
            (1.4..1.6).contains(&v),
            "expected ~1.52 for the documented inputs, got {v}"
        );
    }
}
