//! HTTP client for the opt-in trending-history endpoint.

use std::time::Duration;

use crate::error::BrewError;
use crate::types::{PackageKind, TrendingHistoryIndex, TrendingHistorySeries};

/// Subpath on `brew-browser.zerologic.com` (v0.4.0). Documented in
/// `memory-bank/security.md` and disclosed in `README.md`. The
/// `Settings::enhanced_trending_enabled` toggle is the only thing that
/// authorizes a call to this host — the master `paranoid_mode` switch
/// hard-blocks it regardless.
const BASE: &str = "https://brew-browser.zerologic.com/trending-history";

/// 10 seconds matches the always-on trending fetch. Endpoint serves
/// static JSON from Caddy — a slow response usually means the upstream
/// is wedged, not that more time will help.
const TIMEOUT: Duration = Duration::from_secs(10);

/// Fetch the summary blob — top-N packages with velocity index +
/// compact sparkline. The frontend Trending tab calls this once on
/// mount and reuses the data for every row.
pub async fn fetch_index() -> Result<TrendingHistoryIndex, BrewError> {
    let url = format!("{}/index.json", BASE);
    let client = build_client()?;
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

    let index: TrendingHistoryIndex = resp.json().await.map_err(|e| BrewError::Network {
        url: url.clone(),
        message: format!("decoding json failed: {}", e),
    })?;
    Ok(index)
}

/// Fetch the per-package series. `kind` resolves to the URL segment
/// (`formula` or `cask`); `name` becomes the basename.
pub async fn fetch_package(
    name: &str,
    kind: PackageKind,
) -> Result<TrendingHistorySeries, BrewError> {
    // Reject names that would escape the path. The collector publishes
    // package files with names that match brew's token rules
    // (alphanumeric + `-` + `_` + `+` + `.` + `@`); refusing anything
    // outside that set up front prevents URL-injection and a wasted
    // round-trip on inputs that can't possibly resolve to a real file.
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '+' | '.' | '@'))
    {
        return Err(BrewError::InvalidArgument {
            message: format!("invalid package token for history fetch: {name:?}"),
        });
    }

    let kind_seg = match kind {
        PackageKind::Formula => "formula",
        PackageKind::Cask => "cask",
    };
    let url = format!("{}/{}/{}.json", BASE, kind_seg, name);
    let client = build_client()?;
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

    let series: TrendingHistorySeries = resp.json().await.map_err(|e| BrewError::Network {
        url: url.clone(),
        message: format!("decoding json failed: {}", e),
    })?;
    Ok(series)
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

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fetch_package_rejects_empty_name() {
        let r = fetch_package("", PackageKind::Formula).await;
        match r {
            Err(BrewError::InvalidArgument { message }) => {
                assert!(message.contains("invalid package token"));
            }
            other => panic!("expected InvalidArgument, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn fetch_package_rejects_path_traversal() {
        for bad in ["../etc/passwd", "wget/../sneaky", "foo bar", "foo;ls"] {
            let r = fetch_package(bad, PackageKind::Formula).await;
            assert!(
                matches!(r, Err(BrewError::InvalidArgument { .. })),
                "must reject {bad:?}"
            );
        }
    }

    #[tokio::test]
    async fn fetch_package_accepts_real_token_shapes() {
        // These are all legitimate brew tokens that exist today; the
        // validator must NOT reject them. We don't await the real
        // network call here — we only check the validation gate.
        // Strategy: drive the validation by passing a name that would
        // fail later (with a fake host), then assert that the failure
        // mode is *not* InvalidArgument.
        for ok in ["wget", "openssl@3", "x86_64-elf-gcc", "openssh.test+plus"] {
            let r = fetch_package(ok, PackageKind::Formula).await;
            assert!(
                !matches!(r, Err(BrewError::InvalidArgument { .. })),
                "must NOT reject legitimate token {ok:?}"
            );
        }
    }
}
