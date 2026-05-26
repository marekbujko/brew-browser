//! `reqwest`-based fetch + parse for Homebrew analytics endpoints.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use chrono::Utc;
use serde::Deserialize;

use crate::error::BrewError;
use crate::types::{PackageKind, TrendingEntry, TrendingReport, TrendingWindow};

/// Primary analytics endpoint — total installs over the window,
/// inclusive of dependency-pulled installs. Used for the canonical
/// `install_count` field on each entry.
const HOST_INSTALL: &str = "https://formulae.brew.sh/api/analytics/install";

/// v0.4.0 — `install-on-request` excludes dependency pulls; reflects
/// only what users explicitly typed `brew install <foo>` for. Fetched
/// in parallel with the primary `install` endpoint and merged into the
/// same `TrendingEntry` so the frontend can choose which signal to
/// display. Dramatically de-noises the leaderboard.
const HOST_INSTALL_ON_REQUEST: &str =
    "https://formulae.brew.sh/api/analytics/install-on-request";

const TIMEOUT: Duration = Duration::from_secs(10);
const MAX_ENTRIES: usize = 100;

/// JSON shape published at e.g. `/api/analytics/install/30d.json`.
///
/// The live endpoint returns a flat `items: [...]` array (see fixture
/// `tests/fixtures/trending_30d.json` for the canonical shape). Older
/// documentation referenced a `formulae: { name: [...] }` object-of-arrays
/// form; we keep deserialization support for that legacy shape as a
/// fallback so a future endpoint revert wouldn't silently break the tab.
#[derive(Debug, Deserialize)]
struct RawAnalytics {
    #[serde(default)]
    pub total_count: u64,
    #[serde(default)]
    pub items: Vec<RawAnalyticsItem>,
    #[serde(default)]
    pub formulae: std::collections::HashMap<String, Vec<RawAnalyticsItem>>,
}

#[derive(Debug, Deserialize)]
struct RawAnalyticsItem {
    #[serde(default)]
    pub number: u32,
    #[serde(default)]
    pub formula: String,
    #[serde(default)]
    pub count: String,
}

/// Fetch + parse a single window's analytics into a `TrendingReport`,
/// hitting **both** the primary `install` and v0.4.0 `install-on-request`
/// endpoints in parallel and merging the results.
///
/// `installed` is the set of formula names the user has locally —
/// used to populate `installed_locally` on each entry.
///
/// **Velocity is not populated here** — it requires data from the other
/// two windows. `commands::trending::trending_fetch` orchestrates the
/// three-window fetch and back-fills `velocity_index` from the cache.
///
/// Degraded path: if the `install-on-request` fetch fails (timeout,
/// 5xx, etc.) the primary `install` data still ships and entries carry
/// `install_on_request_count: None`. The opposite — primary fails —
/// is a hard error since there's no useful report without it.
pub async fn fetch(
    window: TrendingWindow,
    installed: &HashSet<String>,
) -> Result<TrendingReport, BrewError> {
    let client = build_client()?;

    // Fire both fetches concurrently. tokio::join! waits for both;
    // we tolerate the secondary failing but not the primary.
    let (install_res, ior_res) = tokio::join!(
        fetch_raw_window(&client, HOST_INSTALL, window),
        fetch_raw_window(&client, HOST_INSTALL_ON_REQUEST, window),
    );
    let install = install_res?;
    let ior_map: HashMap<String, RawAnalyticsItem> = match ior_res {
        Ok(raw) => raw
            .items
            .into_iter()
            .filter(|i| !i.formula.is_empty())
            .map(|i| (i.formula.clone(), i))
            .collect(),
        Err(e) => {
            // Soft-fail: log and continue with empty install-on-request
            // data. The primary leaderboard still works; only the
            // de-duplicated count is missing for this fetch cycle.
            eprintln!(
                "[trending] install-on-request fetch failed (continuing with install-only): {}",
                e
            );
            HashMap::new()
        }
    };

    let entries = merge_entries(install.items, ior_map, installed);

    Ok(TrendingReport {
        window,
        fetched_at: Utc::now().to_rfc3339(),
        cache_age_seconds: 0,
        total_count: install.total_count,
        entries,
    })
}

/// v0.4.0 — pure merge of the two endpoint payloads into the final
/// entry list. Extracted from `fetch` so the merge can be unit-tested
/// without network. Behaviour:
///
/// - Iterate the primary `install` items in their original order.
/// - Look up the install-on-request count for each by formula name.
/// - Sort entries by descending `install_count` and re-rank starting at 1.
/// - Truncate to [`MAX_ENTRIES`] (top 100 by installs).
/// - `velocity_index` is left `None` here — populated by the
///   orchestrating command after all three windows are joined.
fn merge_entries(
    install_items: Vec<RawAnalyticsItem>,
    ior_map: HashMap<String, RawAnalyticsItem>,
    installed: &HashSet<String>,
) -> Vec<TrendingEntry> {
    let mut entries: Vec<TrendingEntry> = install_items
        .into_iter()
        .filter(|item| !item.formula.is_empty())
        .map(|item| {
            let install_count = parse_count(&item.count);
            let ior_match = ior_map.get(&item.formula);
            TrendingEntry {
                rank: item.number,
                name: item.formula.clone(),
                kind: PackageKind::Formula,
                install_count,
                install_count_formatted: item.count,
                install_on_request_count: ior_match.map(|i| parse_count(&i.count)),
                install_on_request_count_formatted: ior_match.map(|i| i.count.clone()),
                velocity_index: None,
                installed_locally: installed.contains(&item.formula),
            }
        })
        .collect();

    entries.sort_by_key(|e| std::cmp::Reverse(e.install_count));
    for (i, e) in entries.iter_mut().enumerate() {
        e.rank = (i as u32) + 1;
    }
    entries.truncate(MAX_ENTRIES);
    entries
}

/// Build the shared reqwest client. Extracted so the concurrent
/// `fetch_raw_window` calls share connection pooling.
fn build_client() -> Result<reqwest::Client, BrewError> {
    reqwest::Client::builder()
        .timeout(TIMEOUT)
        .user_agent(concat!(
            "brew-browser/",
            env!("CARGO_PKG_VERSION"),
            " (+https://github.com/msitarzewski/brew-browser)"
        ))
        .build()
        .map_err(|e| BrewError::Network {
            url: HOST_INSTALL.into(),
            message: e.to_string(),
        })
}

/// Fetch + parse a single endpoint/window combination into the raw
/// JSON shape. The merging into `TrendingEntry` happens in `fetch()`
/// after both endpoints respond.
async fn fetch_raw_window(
    client: &reqwest::Client,
    host: &str,
    window: TrendingWindow,
) -> Result<RawAnalytics, BrewError> {
    let url = format!("{}/{}.json", host, window.as_path_segment());

    let resp = client.get(&url).send().await.map_err(|e| {
        if let Some(status) = e.status() {
            BrewError::HttpStatus {
                url: url.clone(),
                status: status.as_u16(),
            }
        } else {
            BrewError::Network {
                url: url.clone(),
                message: e.to_string(),
            }
        }
    })?;

    let status = resp.status();
    if !status.is_success() {
        return Err(BrewError::HttpStatus {
            url,
            status: status.as_u16(),
        });
    }

    let raw: RawAnalytics = resp.json().await.map_err(|e| BrewError::Network {
        url: url.clone(),
        message: format!("decoding json failed: {}", e),
    })?;

    // Prefer the flat `items` array (current live endpoint shape).
    // Fall back to flattening the legacy `formulae` object-of-arrays so
    // a future endpoint revert doesn't silently empty the Trending tab.
    let items: Vec<RawAnalyticsItem> = if !raw.items.is_empty() {
        raw.items
    } else {
        raw.formulae
            .into_values()
            .filter_map(|items| items.into_iter().next())
            .collect()
    };

    Ok(RawAnalytics {
        total_count: raw.total_count,
        items,
        formulae: std::collections::HashMap::new(),
    })
}

fn parse_count(s: &str) -> u64 {
    s.chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse::<u64>()
        .unwrap_or(0)
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    fn load_fixture(name: &str) -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e))
    }

    // ---------- parse_count ----------

    #[test]
    fn parse_count_strips_commas_and_returns_number() {
        assert_eq!(parse_count("1,234,567"), 1_234_567);
        assert_eq!(parse_count("100"), 100);
        assert_eq!(parse_count("0"), 0);
    }

    #[test]
    fn parse_count_empty_or_garbage_returns_zero() {
        assert_eq!(parse_count(""), 0);
        assert_eq!(parse_count("---"), 0);
        // Non-digit and non-comma chars are stripped, leaving "" → 0.
        assert_eq!(parse_count("abc"), 0);
    }

    #[test]
    fn parse_count_handles_decimal_like_input_by_concatenation() {
        // "1.5" → "15" (we strip non-digits including the decimal point).
        // brew's count format is always thousands-comma-separated integers,
        // so this is expected behavior.
        assert_eq!(parse_count("1.5"), 15);
    }

    // ---------- RawAnalytics shape: real fixture ----------
    //
    // BUG (documented in apiTests.md): the published formulae.brew.sh
    // analytics payload uses `items: [...]` as the top-level array, not
    // the `formulae: { name: [...] }` object the current parser expects.
    // This test pins the expected wire shape against the real fixture so
    // when the parser is fixed in a future wave, the test confirms the
    // intended shape.

    #[test]
    fn real_trending_fixture_has_top_level_items_array_not_formulae_object() {
        let raw = load_fixture("trending_30d.json");
        let v: serde_json::Value = serde_json::from_str(&raw).expect("valid json");
        assert!(
            v.get("items").and_then(|x| x.as_array()).is_some(),
            "real trending payload must have top-level `items` array"
        );
        // The current parser's `formulae` field is NOT in the real payload.
        // If this assertion ever fails (i.e., `formulae` shows up), the
        // BUG documented in apiTests.md should be revisited.
        assert!(
            v.get("formulae").is_none(),
            "real trending payload should not have top-level `formulae` key (it has `items`)"
        );

        // total_count and category are present on the real payload.
        assert!(v.get("total_count").and_then(|x| x.as_u64()).is_some());
        assert!(v.get("category").and_then(|x| x.as_str()).is_some());
    }

    #[test]
    fn raw_analytics_items_round_trip_individual_item_struct() {
        // The per-item shape DOES match RawAnalyticsItem; only the
        // container shape is wrong. Confirm an individual item parses.
        let raw = load_fixture("trending_30d.json");
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let first = &v["items"][0];
        let item: RawAnalyticsItem =
            serde_json::from_value(first.clone()).expect("item must parse");
        assert!(item.number >= 1);
        assert!(!item.formula.is_empty());
        assert!(!item.count.is_empty());
    }

    #[test]
    fn parser_consumes_items_array_from_real_payload() {
        // After FIX-B (PASS-2): the parser now reads the flat `items`
        // array. `formulae` remains absent on the real payload — that's
        // expected — and the legacy fallback path is exercised by
        // `raw_analytics_parses_documented_legacy_shape` below.
        let raw = load_fixture("trending_30d.json");
        let parsed: RawAnalytics = serde_json::from_str(&raw)
            .expect("RawAnalytics parses real payload");
        assert_eq!(parsed.total_count, 25_713_624);
        assert!(
            parsed.formulae.is_empty(),
            "real payload has no legacy `formulae` object — `items` is the source of truth"
        );
        assert!(
            !parsed.items.is_empty(),
            "real payload must populate `items` for the parser to yield entries"
        );
        // First item from the fixture.
        let first = &parsed.items[0];
        assert_eq!(first.number, 1);
        assert_eq!(first.formula, "ca-certificates");
        assert_eq!(first.count, "481,964");
    }

    // ---------- Compat: synthetic legacy payload still parses ----------
    //
    // If formulae.brew.sh ever publishes the documented (but unused)
    // `formulae: { name: [...] }` shape, the current parser handles it.
    // Locking the behavior here so a fix doesn't regress legacy support.

    #[test]
    fn raw_analytics_parses_documented_legacy_shape() {
        let synthetic = serde_json::json!({
            "total_count": 100u64,
            "formulae": {
                "wget": [
                    { "number": 1, "formula": "wget", "count": "42" }
                ],
                "git": [
                    { "number": 2, "formula": "git", "count": "10" }
                ]
            }
        });
        let raw = serde_json::to_string(&synthetic).unwrap();
        let parsed: RawAnalytics = serde_json::from_str(&raw).expect("legacy shape parses");
        assert_eq!(parsed.total_count, 100);
        assert_eq!(parsed.formulae.len(), 2);
        assert!(parsed.formulae.contains_key("wget"));
        assert!(parsed.formulae.contains_key("git"));
    }

    // ---------- v0.4.0: merge_entries ----------

    fn raw(name: &str, count: &str, number: u32) -> RawAnalyticsItem {
        RawAnalyticsItem {
            number,
            formula: name.into(),
            count: count.into(),
        }
    }

    #[test]
    fn merge_carries_install_on_request_when_both_endpoints_have_the_package() {
        // ca-certificates: dep-pulled monster on install, much smaller
        // on install-on-request (almost nobody types `brew install
        // ca-certificates` directly).
        let install = vec![raw("ca-certificates", "481,964", 1)];
        let ior: HashMap<String, RawAnalyticsItem> = [(
            "ca-certificates".to_string(),
            raw("ca-certificates", "1,234", 1),
        )]
        .into_iter()
        .collect();
        let installed = HashSet::new();

        let entries = merge_entries(install, ior, &installed);
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.install_count, 481_964);
        assert_eq!(e.install_on_request_count, Some(1_234));
        assert_eq!(
            e.install_on_request_count_formatted.as_deref(),
            Some("1,234")
        );
    }

    #[test]
    fn merge_degrades_gracefully_when_install_on_request_missing() {
        // Primary fetch succeeded but install-on-request did not — the
        // entry must still ship, just without the IOR fields. This is
        // load-bearing for the soft-fail contract documented in fetch().
        let install = vec![raw("wget", "1000", 1), raw("git", "5000", 2)];
        let ior = HashMap::new();
        let installed = HashSet::new();

        let entries = merge_entries(install, ior, &installed);
        assert_eq!(entries.len(), 2);
        for e in &entries {
            assert!(
                e.install_on_request_count.is_none(),
                "{} must carry no IOR data on degraded path",
                e.name
            );
            assert!(e.install_on_request_count_formatted.is_none());
        }
        // Primary install_count must be intact.
        let git = entries.iter().find(|e| e.name == "git").unwrap();
        assert_eq!(git.install_count, 5_000);
    }

    #[test]
    fn merge_only_includes_packages_in_install_payload() {
        // If install-on-request returns a package the primary doesn't
        // know about (theoretical edge case), the merge keeps the
        // primary as the source of truth and drops the IOR-only entry.
        let install = vec![raw("wget", "100", 1)];
        let ior: HashMap<String, RawAnalyticsItem> = [
            ("wget".to_string(), raw("wget", "50", 1)),
            ("ghost".to_string(), raw("ghost", "99", 2)),
        ]
        .into_iter()
        .collect();
        let installed = HashSet::new();

        let entries = merge_entries(install, ior, &installed);
        assert_eq!(entries.len(), 1, "ghost (IOR-only) must NOT appear");
        assert_eq!(entries[0].name, "wget");
    }

    #[test]
    fn merge_re_ranks_after_sort() {
        // Input order doesn't matter; final ranks must reflect
        // descending install_count.
        let install = vec![
            raw("c", "10", 1),
            raw("a", "100", 2),
            raw("b", "50", 3),
        ];
        let ior = HashMap::new();
        let installed = HashSet::new();

        let entries = merge_entries(install, ior, &installed);
        assert_eq!(entries[0].name, "a");
        assert_eq!(entries[0].rank, 1);
        assert_eq!(entries[1].name, "b");
        assert_eq!(entries[1].rank, 2);
        assert_eq!(entries[2].name, "c");
        assert_eq!(entries[2].rank, 3);
    }

    #[test]
    fn merge_truncates_to_max_entries() {
        // Synthesize MAX_ENTRIES + 50 entries; result must be capped.
        let install: Vec<RawAnalyticsItem> = (0..(MAX_ENTRIES + 50))
            .map(|i| raw(&format!("pkg{i}"), "1", i as u32 + 1))
            .collect();
        let entries = merge_entries(install, HashMap::new(), &HashSet::new());
        assert_eq!(entries.len(), MAX_ENTRIES);
    }

    #[test]
    fn merge_velocity_index_starts_none() {
        // velocity_index is populated by the orchestrating command
        // after joining all three windows — not by merge_entries.
        let install = vec![raw("wget", "100", 1)];
        let entries = merge_entries(install, HashMap::new(), &HashSet::new());
        assert!(entries[0].velocity_index.is_none());
    }

    #[test]
    fn merge_marks_installed_packages() {
        let install = vec![raw("wget", "100", 1), raw("git", "200", 2)];
        let mut installed = HashSet::new();
        installed.insert("git".to_string());
        let entries = merge_entries(install, HashMap::new(), &installed);
        let git = entries.iter().find(|e| e.name == "git").unwrap();
        let wget = entries.iter().find(|e| e.name == "wget").unwrap();
        assert!(git.installed_locally);
        assert!(!wget.installed_locally);
    }
}
