# 2026-05-24 — Phase 9: Discover category UI + chip linking + sortable columns

**Phase:** 9 (sub-phases 9a, 9b)
**Status:** ✅ Shipped
**Commit:** `84ad010` (combined with Phase 11; 41 files total, +3,845 / -305)
**Date:** 2026-05-24 02:47

## Scope

Light up the categorization data shipped in `c72e31d`. Tile grid + multi-select chip filter + per-package category pills + sortable columns across Library and Trending. Phase 9c ("Wrong?" GitHub-issue link) deferred to fold into Phase 12f.

## What landed

### Phase 9a — Discover category tile UI
- 19-tile grid using Lucide icons, count badges
- Click a tile → adds that single category as a chip filter
- New shared store: `src/lib/stores/discover.svelte.ts` with `selectedCategories: Set<string>`
- Filtered view mode (chip(s) active, no search) lists union sorted alphabetically

### Phase 9b — Category linking pass
- **Multi-select chip filter** shared across Discover + Library (single source of truth in the discover store)
- **Category pills on PackageDetail** — clickable, jumps to Discover with that single category selected; closes the detail panel so user lands on the filtered list
- **Sortable columns** added via new `src/lib/components/SortableHeader.svelte`:
  - Library: Name / Version / Type / Outdated
  - Trending: # / Name / Type / Installs (Installs defaults to descending on first click)
- **Fixed dangling `installed` pill** in Discover — two row layouts now (`row--with-desc` for search, `row--no-desc` for chip-filtered browse); `installed` column at fixed `90px` so it doesn't shift the kind cell
- **Updated empty-state messaging** in Library to distinguish chip vs text filters

## Files (subset)

- New: `src/lib/stores/discover.svelte.ts`, `src/lib/components/SortableHeader.svelte`
- Modified: `Discover.svelte`, `Library.svelte`, `PackageDetail.svelte`, `Trending.svelte`, `PackageRow.svelte`

## Tests / verification

- `npm run check`: 0 errors
- `npm run build`: clean in 1.64s
- Backend untouched — no Rust regression risk

## Notes / decisions

- Categories store is **shared, not duplicated** between Discover and Library. Single source of truth via the discover store.
- `1fr` → `minmax(0, 1fr)` everywhere a flex column had text — without it, long names like `claude-code-templates` would expand the cell beyond its share and push downstream columns rightward.
- `aria-label` (not `aria-sort`) on sortable headers — `aria-sort` requires `role="columnheader"` and our list-grids aren't true tables.
- Phase 9c ("Wrong?" GitHub-issue link) deliberately deferred — folds naturally into Phase 12f (GitHub authed actions), so we ship them together.
