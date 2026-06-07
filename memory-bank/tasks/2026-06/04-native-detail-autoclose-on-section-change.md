# 04 — Native: close inspector when the section changes

**Date:** 2026-06-04
**Branch:** `experiment/native-swift-liquid-glass`

## Objective

The package detail inspector stayed open when navigating between sidebar
sections. It should belong to the section that opened it — moving away (e.g.
Library → open a package → switch to Discover) should dismiss it.

## Outcome

- ✅ `swift build` clean; `.app` rebuilt.
- Library/Discover/Trending/Services/Dashboard → open a package → switch section
  → inspector closes. Staying in-section (or clicking another package) keeps it.

## Changes (`native/Sources/BrewBrowserKit/`)

- **`AppModel.swift`** — `detailSection: Section?` tags which section opened the
  inspector (set in `openDetail`, cleared in `closeDetail`). New
  `closeDetailIfSectionChanged()` closes only when `detailSection != selection`.
- **`ContentView.swift`** — `.onChange(of: model.selection)` on the sidebar list
  calls `closeDetailIfSectionChanged()`.

## Notes

Safe against the "jump to Library" helpers (`openInLibrary` / `openLibrary` /
`openOutdatedInLibrary`) — those only change `selection`; they don't open detail,
so there's no switch-then-open flow to fight the auto-close.
