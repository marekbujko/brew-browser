//! v0.4.0 — opt-in historical trending data from
//! `brew-browser.zerologic.com/trending-history/*`.
//!
//! Distinct trust boundary from the always-on `formulae.brew.sh`
//! analytics — this endpoint is operated by the project, not upstream
//! Homebrew. Gated by [`crate::state::AppState::require_enhanced_trending`]
//! which composes the master paranoid switch with the per-feature
//! `enhanced_trending_enabled` toggle.
//!
//! Two endpoints, both static JSON served from Caddy on umbp:
//!
//! - `GET /trending-history/index.json` — summary blob (top-N packages
//!   with server-precomputed velocity index + compact sparkline).
//!   Fetched once on Trending tab mount.
//! - `GET /trending-history/{kind}/{name}.json` — per-package full
//!   series. Fetched on demand from PackageDetail.
//!
//! No telemetry, no cookies, no auth. Server logs strip remote IP at
//! the nginx layer (see `memory-bank/security.md` for the audit).

pub mod cache;
pub mod client;
