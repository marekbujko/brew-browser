# 02 — Native trending tap-token resolution + Refresh feedback

**Date:** 2026-06-04
**Branch:** `experiment/native-swift-liquid-glass`

## Objective

Fix two issues surfaced while verifying live enrichment in the native Trending
panel:

1. Third-party **tap** packages (e.g. `anomalyco/tap/opencode`) rendered with no
   friendly name, `—` version, and a blank description — and clicking the row
   errored on `brew info`.
2. The **Refresh** buttons gave no active feedback — they only `.disabled`d
   while loading, so a quick refresh looked like nothing happened.

## Root cause

Homebrew install-analytics (the Trending data source) report tap formulae
**fully-qualified** as `user/tap/name`, but the bundled catalog + enrichment are
keyed by the **bare** name (`opencode`). So every lookup missed:

- `catalogLookup` → nil → raw token shown as name, `—` version, empty desc.
- `enrichmentEntry` → nil → empty friendly name + AI summary.
- `EnrichmentLiveService.isValidToken` rejects `/`, so even the opt-in live
  fetch refused tap tokens.
- `loadDetail` shelled `brew info <user/tap/name>`, which fails when the tap
  isn't tapped locally ("requires the tap …").

## Outcome

- ✅ `swift build` clean; `.app` rebuilt via `build-app.sh`.
- ✅ Verified in-app: the `anomalyco/tap/opencode` row resolves to **OpenCode**,
  version 1.15.10, *"Terminal-based AI coding agent…"*; detail loads via the
  core formula.

## Changes (`native/Sources/BrewBrowserKit/`)

- **`AppModel.swift`**
  - `bareToken(_:)` — strips `user/tap/` → bare name (last `/`-segment).
  - `catalogLookup` — bare-name **fallback** (exact match wins first).
  - `enrichmentEntry(for:)` — bare-name fallback for friendly name + summary.
  - `ensureLiveEnrichment(_:)` — fetch + key by bare name (also clears the
    `/`-rejecting allowlist).
  - `loadDetail` — on `brew info` failure for a qualified name, retry the bare
    name (the core formula the list already resolved to). Genuine tap-only
    formulae still surface brew's error, with the bundled enrichment header
    intact.
- **`TrendingView.swift`** + **`ContentView.swift`**
  - Refresh buttons swap the arrow → `ProgressView` (small) while loading.
    Stock SwiftUI; no overrides.

All lookups are **fallback-only**, so normal (bare) packages are unaffected and
there is no same-name collision risk.

## Parity TODO (Tauri)

The Tauri Trending view almost certainly has the same qualified-token-vs-bare-key
mismatch (list lookups **and** detail `brew info`). Fix next on the Tauri side.

## Patterns applied

- Bare-token normalization mirrors how analytics vs catalog keys diverge;
  applied as a fallback so it composes with the existing exact-match path.
