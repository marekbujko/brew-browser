# Active Context

**Date:** 2026-05-26 (v0.4.0 backend complete on branch)
**State:** Backend for v0.4.0 (Trending Velocity + Opt-in History Endpoint) complete and tested on branch `feat/v0.4.0-velocity-and-history`. Frontend (Steps 4–6), umbp trending-collector (Step 7), docs + memory-bank polish (Step 8), and Caddy privacy hardening (Step 9) still ahead. From this branch onward, merges to `main` go through PRs — no more direct pushes to `main`.

## Repo

- **github.com/msitarzewski/brew-browser** — public, MIT
- **Released:** v0.1.0, v0.2.0, v0.2.1, v0.3.0, v0.3.1 (live on GitHub Releases — `gh release list`)
- **Working toward:** v0.4.0 (single coherent release; backend on this branch, frontend + collector + docs ahead)
- **Branch:** `feat/v0.4.0-velocity-and-history` (off `main` at `d6d28a0`)
- **Stars:** 18 (as of v0.3.1 ship)

## What landed this session (uncommitted, on the branch)

### v0.4.0 backend — Steps 1–3 (full file:line detail in `tasks/2026-05/19-v0.4.0-backend.md`)

**Step 1 — Settings + per-feature gate + new error variant**

- `Settings.enhanced_trending_enabled: bool` (default `false`, `#[serde(default)]`)
- `state::AppState::require_enhanced_trending()` — composes paranoid gate + per-feature toggle; returns `ParanoidModeBlocked` or `FeatureDisabled` per the rule "different cure = different error"
- `BrewError::FeatureDisabled { feature: String }` — new variant so frontend toast routes to the right setting

**Step 2 — install-on-request fetch + velocity computation**

- `trending::client::fetch` now hits `install` + `install-on-request` in parallel via `tokio::join!`; merges on package name
- `trending::velocity::velocity_index(c30, c90, c365) → Option<f64>` — pure math, returns `None` on degenerate or too-small inputs
- `commands::trending::trending_fetch` eager-warms all three windows via `tokio::task::JoinSet`; back-fills `velocity_index` on every entry from the cross-window join
- `TrendingEntry` extended with optional `install_on_request_count{,_formatted}` + `velocity_index` (`skip_serializing_if = "Option::is_none"` for wire back-compat)

**Step 3 — Opt-in history endpoint client + cache + IPCs**

- New module `trending::history::{mod, client, cache}`
- `trending_history_index() → TrendingHistoryIndex` — summary blob (top-N with velocity + compact sparkline). Single fetch on Trending tab mount.
- `trending_history_fetch(name, kind) → TrendingHistorySeries` — per-package full series. On-demand from PackageDetail.
- Both gated by `require_enhanced_trending`; both follow the cache-hit / fetch / stale-fallback contract from v0.3.x
- URL builder rejects path traversal (strict whitelist of `[A-Za-z0-9._+@-]`)
- LRU per-package cache (cap 500, TTL 6h matching the nightly collector cadence)
- 5 new types in `types.rs`: `TrendingHistorySource`, `TrendingHistoryPoint`, `TrendingHistorySeries`, `TrendingHistoryIndex`, `TrendingHistoryIndexEntry`

## Tests & lint (current)

- `cargo test`: **506 passed**, 0 failed, 6 ignored (473 → 506, +33 new)
- `cargo build`: clean (no dead-code warnings — every new symbol is wired and exercised)
- Frontend untouched in this checkpoint; `npm run check` posture unchanged from v0.3.1

## Working tree (8 modified + 4 new)

**Modified (backend Steps 1–3):**
- `src-tauri/src/commands/settings.rs` (+97)
- `src-tauri/src/commands/trending.rs` (+283)
- `src-tauri/src/error.rs` (+19)
- `src-tauri/src/lib.rs` (+2)
- `src-tauri/src/state.rs` (+121)
- `src-tauri/src/trending/client.rs` (+300)
- `src-tauri/src/trending/mod.rs` (+6)
- `src-tauri/src/types.rs` (+114)

**New (backend Step 3):**
- `src-tauri/src/trending/velocity.rs` (~115 lines)
- `src-tauri/src/trending/history/mod.rs` (~22 lines)
- `src-tauri/src/trending/history/client.rs` (~150 lines)
- `src-tauri/src/trending/history/cache.rs` (~225 lines)

**New (memory bank):**
- `memory-bank/tasks/2026-05/19-v0.4.0-backend.md`

## Decisions locked in this session (full rationale in task #19)

- **D1**: Subpath `brew-browser.zerologic.com/trending-history/*`, not a new vhost. Reuses Caddy + cert.
- **D2**: GitHub mirror of nightly JSON deferred to v0.5+.
- **D3**: Default sort by velocity desc + inline mini-sparklines per row (star-history.com aesthetic). Index blob carries compact sparkline arrays so the list renders from one fetch.
- **D4**: Sparkline empty state when toggle is off = passive (only present in Settings → Network). No detail-panel placeholder, no banner.
- **D5**: Velocity computed server-side, joined in `trending_fetch` from cached 30d/90d/365d windows. Frontend never knows the formula.

## What's still ahead (PRD-side)

- **Step 4** — Settings UI: new `SettingsSectionTrendingHistory.svelte` mounted at bottom of Network; new `pathStatuses` entry in `SettingsSectionNetwork.svelte`.
- **Step 5** — Trending tab UI: velocity column, sort-by-velocity default, inline sparklines per row when enhanced trending is on.
- **Step 6** — PackageDetail sparkline: new `TrendingSparkline.svelte` + new `trendingHistory.svelte.ts` store.
- **Step 7** — umbp trending-collector: Bun TS daemon (`tools/trending-collector/`), seed.ts bootstrap, nightly cron, SQLite, static JSON output to `/home/michael/sites/brew-trending/`.
- **Step 8** — Memory bank + docs: decisions.md ADR for opt-in trust boundary; projectbrief.md nine→ten paths; security.md endpoint audit; backendApi.md / frontendComponents.md / techContext.md updates; `docs/release-notes/0.4.0.md`; README disclosure update; cross-session memory pointers.
- **Step 9** — Caddy privacy hardening: IP-strip at the proxy layer, no cookies, GET-only, cache-control, document the snippet in security.md.

## Workflow change (durable)

From this branch onward, all merges to `main` go through pull requests. Push the branch, open a PR via `gh pr create`, wait for review/CI, then merge. No more direct pushes to `main`.

## Memory bank inventory

`toc.md`, `projectbrief.md`, `techContext.md`, `decisions.md`, `activeContext.md` (this), `progress.md`, `systemPatterns.md`, `designSystem.md`, `uxArchitecture.md`, `visualStory.md`, `backendApi.md`, `frontendComponents.md`, `codeReview.md`, `apiTests.md`, `accessibility.md`, `realityCheck.md`, `security.md`, `ideas.md`, `agentLog.md` (dormant), `NEXT-SESSION.md`, `tasks/2026-05/` (19 task records + README + deferred), `phases/`, `scans/2026-05-23/`.
