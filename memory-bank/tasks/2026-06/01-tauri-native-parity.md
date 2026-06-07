# 01 — Tauri←native feature parity

**Date:** 2026-06-02
**Branch:** `tauri-parity` (rooted on `main`, so it can PR cleanly later — NOT the experiment branch)

## Objective

Bring the shipped Tauri app up to the native rebuild's treatment of icons + Dashboard charts, collapse the GitHub Keychain reads to one prompt (as native already does), and reconcile the Trending velocity badge to one canonical threshold.

## Outcome

- ✅ `npm run check`: 0 errors (3 pre-existing warnings unrelated to these files)
- ✅ Rust compiles clean via `tauri dev` (keychain change)
- ✅ Verified visually via user screenshots (Dashboard wide/narrow, cask/formula detail, lists)

## Files modified

- `src/lib/components/PackageRowIcon.svelte` (**new**) — shared list/detail icon, Tauri analog of native's `PackageIcon`: resolved cask icon → `terminal` glyph (formulae) → `square-dashed` (unresolved casks). Glyph inherits `currentColor` so it tints on row selection. Casks resolve via `iconCache.getIcon`; Discover hits resolve homepage via `catalog.lookupCask`.
- `src/lib/components/PackageRow.svelte` — Library rows route through `PackageRowIcon` (removed the duplicated inline icon state + CSS).
- `src/lib/components/Discover.svelte` — added icon in the name cell of both row modes (search + browse); previously **no icons**.
- `src/lib/components/Trending.svelte` — added icon in the name cell (formulae-only leaderboard → terminal glyph); previously **no icons**. Reconciled `velocityTier` threshold (below).
- `src/lib/components/PackageDetail.svelte` — centered 64px app icon at the top of the detail body (kept the shared `.panel-head` pixel-alignment intact rather than restructuring the header bar).
- `src/lib/components/Dashboard.svelte` — Composition pie + Top-categories paired side-by-side on wide panes (see below).
- `src-tauri/Cargo.toml` + `Cargo.lock` — added `security-framework = "2"` (macOS target).
- `src-tauri/src/github/auth.rs` — batched Keychain read (below).

## Dashboard charts (matched to native specs)

- **Layout:** Composition + Top-categories pair two-across once the pane is wide (`@container dash (min-width: 820px)` on `.body`, which tracks the *content pane* width — sidebar/inspector excluded, more correct than native's manual `onGeometryChange(>980)`). Below the breakpoint they stack full-width and Composition reverts to its horizontal bar.
- **Pie:** filled SVG `<path>` wedges, outer radius **60** (fills the `0 0 120 120` viewBox = native's 180×180 frame). (A thick-stroke circle was ambiguous at the `stroke=2·r` boundary and rendered larger.)
- **Donut:** `DONUT_RADIUS 48` + `24px` stroke → outer **60**, inner **36** = ratio **0.6**, matching native's `innerRadius: .ratio(0.6)`. Hover is now opacity-only (removed the stroke-width bump), matching native's `.opacity()`.
- **Equal height + centering:** paired cards become flex columns; chart bodies flex-fill the (equal) card height; each chart is vertically centered — two same-size charts centered in equal-height cards line up, exactly how native does it.
- **Chips:** `on request` / `as dependency` / `pinned` moved from a bottom row into the Composition card header upper-right (native's `CompositionCard` title row).

## Keychain — one prompt (mirrors native `keychainReadAll`)

- `KeychainSlot` trait gained `read_many(&[&str])` with a **default** impl (one `read` per account → N prompts) so tests + non-macOS backends are unaffected.
- macOS `SystemKeychain` overrides `read_many` with `read_all_batch()`: one `ItemSearchOptions::new().class(generic_password).service(KEYCHAIN_SERVICE).load_attributes(true).load_data(true).limit(Limit::All).search()` → all GitHub items in **one** `SecItemCopyMatching` (one auth prompt). `errSecItemNotFound` → empty map (signed out); other errors → `KeychainUnavailable`. `simplify_dict()` keys: `acct` (account) / `v_Data` (value).
- `status_with()` now does ONE `read_many([token, username, scopes])` instead of three `read()` calls. In dev (unsigned binary re-signed each build, ACL never matches) this is **3 → 1** prompts per launch; in a signed release the user hits "Always Allow" once.

## Trending velocity threshold (canonical rule)

Agreed canonical = **formula-faithful banded**, matching `velocity.rs`'s documented "1.0≈steady, >1.5 surging, <0.7 cooling":
- `>= 1.5` → 🔥 surge
- `<= 0.7` → ❄️ cool
- otherwise → neutral (no icon, just the number)

Tauri's `velocityTier` updated (cool bound `0.5 → 0.7`). **Native still uses a binary `v >= 1` rule** (`TrendingView.swift:velocityCell`) — its change to banded (incl. an iconless neutral state) is tracked in memory `project-native-reverse-parity`.

## Reverse-parity (native←Tauri) — NOT in this task

- Native Dashboard category-legend **icons** (user: keep in Tauri, add to native).
- Native Trending **banded** velocity threshold (above).
- Native **Snapshots** + **Services** panels (still placeholder stubs).

## Notes

- All on `tauri-parity`; `native/` is untracked here (it's tracked on the experiment branch) and is **not** part of this commit.
