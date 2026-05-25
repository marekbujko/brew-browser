# 2026-05-24 — Phase 12g/13b cleanup + Tier A enrichment baked + extensive UI polish

**Phase:** 12g (cleanup) + 13b (frontend wiring) + Tier A enrichment data bake
**Status:** ✅ Shipped
**Commit:** `e1d6a87` (41 files, +2,777 / -278)
**Date:** 2026-05-24 10:25

## Scope

Address the 4 IMPORTANT findings from the Code Reviewer pass on Wave 1+2, fully wire the Phase 12a frontend (it was dead code from UI's perspective until now), and bake the Tier A enrichment artifact into the repo. Also: native macOS menu bar, About modal, GitHub Sponsors setup, sidebar TopBar prototype, unified panel-head styling, responsive headers + columns.

## What landed

### Phase 12g/13b cleanup (4 IMPORTANT findings addressed)
1. **Phase 12a frontend wired** — `src/lib/stores/catalog.svelte.ts`, `Catalog`/`Formula`/`Cask`/`CatalogSummary` types in types.ts, 6 IPC wrappers in api.ts, Dashboard catalog freshness line, Discover stale-catalog banner (dismissable per-session)
2. **Three persisted settings actually honored:**
   - `trending_ttl_minutes` consumed in `trending_fetch` (was hardcoded 60min)
   - `cask_icon_mode` consumed in `cask_icon_from_homepage` (Off/InstalledOnly/All gate before paranoid check)
   - `catalog_auto_refresh` consumed via new `maybe_auto_refresh_catalog` startup hook + `should_auto_refresh` decision helper
3. **search-no-match hotfix** verified — `is_brew_search_no_match` helper from `8b89c40` confirmed working, +4 tests pinning the pattern (already landed in `8b89c40`; cleanup confirmed)
4. **Phase 13 friendly names rendered in list rows** — Discover (search + chip-filtered), Library (via PackageRow), Trending all show `friendly_name` as subtitle below the raw token when AI Features toggle is on
5. **+23 backend tests** for settings consumers, **+4 search hotfix**, **+34 total → 411 backend tests** (was 385 → 411)

### Tier A enrichment baked
- `python tools/enrich/enrich.py --tier-a` against Anthropic Haiku 4.5
- **15,725 entries** written to `src-tauri/data/enrichment.json.gz` (771 KB compressed)
- Cost: ~$3-5 against user's Anthropic API
- Bundle now: 6.1 MiB catalog + 0.74 MiB enrichment = ~6.9 MiB total bundled data
- `tools/enrich/enrich.py` patched: cascade `.env` lookup — falls back to `tools/categorize/.env` so user doesn't have to duplicate ANTHROPIC_API_KEY

### Extensive UI polish (in the same session)
**Native macOS menu bar**
- `tauri::menu` builder in `src-tauri/src/lib.rs` — App menu (About brew-browser / Settings… ⌘, / Hide / Hide Others / Show All / Quit) + Edit + Window submenus
- `on_menu_event` handler emits `menu:about` and `menu:settings` Tauri events
- Frontend `+layout.svelte` listens and opens the matching modal

**About brew-browser modal** (`src/lib/components/AboutModal.svelte`)
- 🍺 hero + version + brew version + license + repo + Agency Agents credit (link → github.com/msitarzewski/agency-agents)
- "Donate to the project" CTA → GitHub Sponsors
- Built-with credit (later updated to current wording in v0.2.1)

**GitHub Sponsors setup**
- `.github/FUNDING.yml` → `github: [msitarzewski]` (Sponsor button appears on repo)
- Shared `src/lib/util/donate.ts` exports `SPONSOR_URL` (single source for AboutModal + Sidebar)
- Sidebar footer gets a `♥ Donate` link under the brew version (later moved to title-bar cluster in v0.2.0)

**Sidebar right-cluster TopBar prototype** (later restructured in v0.2.0)
- New `src/lib/components/TopBar.svelte` (deleted in v0.2.0 when title bar restructure landed)
- Theme dropdown + Settings gear in subtle sunken-background group with hair-line divider

**Unified panel-head styling** (the "precision, happy" pass)
- Global `.panel-head` baseline in `src/app.css` pins height, padding, border-bottom, h1 typography for every panel-head — Dashboard / Library / Discover / Trending / Snapshots / Services / Activity AND PackageDetail
- `!important` justified as cross-component layout coordination
- Header separators align to the pixel across panels

**Responsive headers + columns** (avoid the crashing at narrow widths)
- All panel-heads with right-cluster controls wrap their Refresh/Clear in `.refresh-wrap` / `.action-wrap`
- `@media (max-width: 1000px)` hides those wraps + auxiliary text
- List rows on Trending + Library get two-tier responsive column drops
- `overflow: hidden` + `min-width: 0` on every header/row cell

**PackageDetail rework**
- h1 renders `enriched?.friendlyName ?? ui.selectedPackage.name`
- AI on + friendly name → friendly is the title; raw token in new "Token" meta row
- Type pill right-aligned via `margin-left: auto`
- AI-enriched badge removed from h1 (still on summary / use_cases / similar / tags below)

**Other small fixes**
- Detail panel closes on any sidebar navigation
- Pillgroup style unified (sunken bg, no border) on Trending + Library
- Brew analytics parser widened to accept InfluxDB and arbitrary backend prefixes (fix for `"InfluxDB analytics are enabled."` rejection)
- GitHub sign-in fail-fast with clear message when `GITHUB_OAUTH_CLIENT_ID` is still placeholder

## Files

41 files. New (5): `.github/FUNDING.yml`, `AboutModal.svelte`, `TopBar.svelte` (later deleted), `catalog.svelte.ts`, `donate.ts`. Backend modified (10) + frontend modified (~15) + tooling.

## Tests / verification

- `cargo test`: **411 passed** (385 → 411, +26)
- `cargo clippy --all-targets -- -D warnings`: clean
- `npm run check`: 0 errors
- `npm run build`: clean

## Notes / decisions

- **Largest single polish session** — landed enrichment bake + 4 cleanup items + 8+ UI polish threads in one commit. Risky in retrospect; should have been split into 2-3 commits.
- **`!important` on `.panel-head` global** is intentional — cross-component layout coordination across Svelte's scoped styles is otherwise impossible without prop-drilling style overrides.
- **TopBar prototype** in this commit gets entirely rebuilt in v0.2.0 (`f556441`) when the user requests a real macOS title bar. The work here informed the v0.2.0 redesign.
- **Sidebar Donate link** also moves in v0.2.0 — first to the controls row, then to the title-bar cluster as a pink heart.
- Enrichment bundle is **deterministic** at bundle time; the Rust loader re-validates every field against the same caps the Python writer enforced, so a future maintenance accident that swaps the bundle for a wider-cap version still fails-closed.
