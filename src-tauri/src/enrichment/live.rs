//! HTTP client for the opt-in *live* enrichment endpoint.
//!
//! Mirrors `trending/history/client.rs`: a small soft-fail GET client against a
//! first-party static host. The `Settings::live_enrichment_enabled` toggle is
//! the only thing that authorizes a call here, and the master `paranoid_mode`
//! switch hard-blocks it regardless (both enforced by
//! `AppState::require_live_enrichment`).
//!
//! Endpoints (rendered nightly by `tools/pipeline/render_served.py`):
//!   GET /enrichment/version.json          → freshness probe
//!   GET /enrichment/categories.json       → full categories file (version-gated)
//!   GET /enrichment/entry/<token>.json    → per-token enrichment, on demand

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::commands::categories::CategoriesData;
use crate::enrichment::EnrichmentEntry;
use crate::error::BrewError;

/// Subpath on `brew-browser.zerologic.com` — same first-party host as Enhanced
/// Trending, distinct `/enrichment/*` path. Documented in `memory-bank/security.md`
/// and disclosed in `README.md`.
const BASE: &str = "https://brew-browser.zerologic.com/enrichment";

/// 10s, matching the trending-history client. Static JSON from Caddy — a slow
/// response means the upstream is wedged, not that more time helps.
const TIMEOUT: Duration = Duration::from_secs(10);

/// The tiny freshness probe the app polls on catalog refresh. When `version`
/// or `categories_version` is newer than what the app holds, it pulls the
/// corresponding payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveEnrichmentVersion {
    pub version: String,
    pub generated_at: String,
    pub categories_version: String,
}

/// `GET /enrichment/version.json`.
pub async fn fetch_version() -> Result<LiveEnrichmentVersion, BrewError> {
    get_json(&format!("{}/version.json", BASE)).await
}

/// `GET /enrichment/categories.json` — the full categories file. Pulled only
/// when the served `categoriesVersion` is newer than the app's.
pub async fn fetch_categories() -> Result<CategoriesData, BrewError> {
    get_json(&format!("{}/categories.json", BASE)).await
}

/// `GET /enrichment/entry/<token>.json` — per-token enrichment, on demand.
/// Token is validated against brew's token charset before any round-trip
/// (URL-injection + wasted-fetch guard), same as the trending client.
pub async fn fetch_entry(name: &str) -> Result<EnrichmentEntry, BrewError> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '+' | '.' | '@'))
    {
        return Err(BrewError::InvalidArgument {
            message: format!("invalid package token for enrichment fetch: {name:?}"),
        });
    }
    get_json(&format!("{}/entry/{}.json", BASE, name)).await
}

/// Shared GET-and-decode with uniform error mapping (mirrors the trending
/// client's per-call body).
async fn get_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, BrewError> {
    let client = build_client()?;
    let resp = client.get(url).send().await.map_err(|e| {
        if let Some(status) = e.status() {
            BrewError::HttpStatus {
                url: url.to_string(),
                status: status.as_u16(),
            }
        } else {
            BrewError::Network {
                url: url.to_string(),
                message: e.to_string(),
            }
        }
    })?;

    let status = resp.status();
    if !status.is_success() {
        return Err(BrewError::HttpStatus {
            url: url.to_string(),
            status: status.as_u16(),
        });
    }

    resp.json::<T>().await.map_err(|e| BrewError::Network {
        url: url.to_string(),
        message: format!("decoding json failed: {}", e),
    })
}

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
            url: BASE.into(),
            message: e.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fetch_entry_rejects_bad_tokens() {
        for bad in ["", "../etc/passwd", "wget/../x", "foo bar", "foo;ls"] {
            let r = fetch_entry(bad).await;
            assert!(
                matches!(r, Err(BrewError::InvalidArgument { .. })),
                "must reject {bad:?}"
            );
        }
    }

    #[tokio::test]
    async fn fetch_entry_accepts_real_token_shapes() {
        for ok in ["wget", "openssl@3", "x86_64-elf-gcc", "openssh.test+plus"] {
            let r = fetch_entry(ok).await;
            assert!(
                !matches!(r, Err(BrewError::InvalidArgument { .. })),
                "must NOT reject legitimate token {ok:?}"
            );
        }
    }
}
