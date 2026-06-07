# 02 — Tauri trending tap-token resolution (native parity)

**Date:** 2026-06-04
**Branch:** `fix/tauri-trending-tap-tokens` (off `main`)

## Objective

Port the native build's tap-token fix (`experiment` task 02) to the shipped
Tauri app. Third-party **tap** packages in Trending (e.g.
`anomalyco/tap/opencode`) rendered with no friendly name / summary / version and
errored on detail open.

## Root cause (identical to native)

Homebrew install-analytics report tap formulae fully-qualified as
`user/tap/name`, but the bundled catalog + enrichment are keyed by the **bare**
name. `Trending.svelte` feeds the qualified `e.name` into:

- `enrichment.lookup()` → `entries[token]` (bare key) → null → no friendly
  name / summary.
- `catalog.descOf` / `versionOf` → `map.get(name)` (bare key) → null → blank
  desc + version.
- `PackageDetail.loadDetail` → `brewInfo(name)` → fails when the tap isn't
  tapped locally ("requires the tap …").

## Outcome

- ✅ `npm run check` clean (0 errors; 3 pre-existing unrelated warnings).

## Changes

- **`src/lib/util/token.ts`** (new) — `bareToken()`; mirrors native
  `AppModel.bareToken(_:)`.
- **`src/lib/stores/enrichment.svelte.ts`** — `lookup()` bare-name fallback
  (also fixes `friendlyName`, `summaryOf`, and `PackageDetail`'s `enriched`),
  composed with PR #43's live overlay (live → bundled → bare live → bare
  bundled). `ensureLive()` also fetches/dedupes/keys by the bare name so tap
  rows get live refresh too (mirrors native `ensureLiveEnrichment`).
- **`src/lib/stores/catalog.svelte.ts`** — `descOf` / `versionOf` bare-name
  fallback.
- **`src/lib/components/PackageDetail.svelte`** — `loadDetail` retries
  `brewInfo(bareToken(name))` when the qualified name fails.

All fallback-only (exact match wins first), so normal packages are unaffected.

## Velocity leaderboard parity (the bigger fix)

Comparing the two Trending lists revealed a material data divergence: native
surfaced high-velocity rising packages (nbytes 9.60, simdutf 4.33, opencode
4.04, llhttp 3.38, …) that Tauri omitted entirely.

**Root cause** — `commands/trending.rs:build_velocity_map` built `c30/c90/c365`
from each window's **cached report entries**, which `client::merge_entries`
already truncated to **top-100 by installs**. So velocity was only computable
for packages in the top-100 of *all three* windows. A rising package (top-100
in 30d but ranked >100 over 365d) had no `c365` → no velocity → dropped from the
leaderboard. Native computes velocity from the **full, uncapped** per-window
maps and caps only the display.

**Fix** — carry the full uncapped `name→install_count` map per window:
- `trending/client.rs` — `fetch()` now returns `(TrendingReport, HashMap<String,u64>)`;
  the map is built from `install.items` before `merge_entries` truncates.
- `trending/cache.rs` — `CachedTrending` gains `full_counts`.
- `commands/trending.rs` — `ensure_all_windows_cached` stores `full_counts`;
  `build_velocity_map` joins over them instead of the capped report entries.

`cargo build` clean; `cargo test trending` → 58 passed. Requires a backend
rebuild (`npm run tauri dev` recompiles Rust; HMR alone won't pick it up).

## Refresh feedback

`Trending.svelte` Refresh button now passes `loading={trending.loading}` to the
shared `Button` (spins + disables while refetching) — it previously gave no
visual feedback.

## Parity

Brings Tauri to parity with native commit `efd45d1`
(`experiment` task `02-native-trending-tap-tokens-and-refresh-feedback`):
tap-token resolution, Refresh feedback, and — beyond the native commit — the
velocity leaderboard now matches native's full-map computation.
