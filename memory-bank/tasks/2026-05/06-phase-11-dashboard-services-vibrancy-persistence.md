# 2026-05-24 — Phase 11: Dashboard + Services + native macOS feel + Activity persistence

**Phase:** 11 (sub-phases 11, 11b, 11c, 11d)
**Status:** ✅ Shipped
**Commit:** `84ad010` (combined with Phase 9; 41 files total, +3,845 / -305)
**Date:** 2026-05-24 02:47

## Scope

Four sub-phases shipped in one session:

- **11** — Dashboard becomes the default landing
- **11b** — Services tab (`brew services` GUI)
- **11c** — Native macOS feel (vibrancy + drag regions + traffic-light-aware sidebar)
- **11d** — Activity persistence (localStorage mirror, hydrate on bootstrap)

## What landed

### 11 — Dashboard
- New `src/lib/components/Dashboard.svelte` as default landing
- **Hero row** — installed count / outdated count / brew version cards
- **Updates panel** — one-click "Upgrade all" + clickable card title → Library outdated filter
- **Composition** — split bar (formulae vs casks) with on-request / dependency / pinned meta
- **Top-Categories donut** — 180px SVG, 9-color palette, top 8 + "Other", click legend → Discover with chip pre-selected
- Donut math: `stroke-dasharray="(pct/100)*C C"` + `stroke-dashoffset="-(startPct/100)*C"` + `rotate(-90)` for top start
- **Storage card** — 4 paths surveyed in parallel (`Cellar`, `Caskroom`, `var/log`, cache) with "Reveal in Finder" per row

### 11b — Services
- New backend: `src-tauri/src/commands/services.rs` — 5 commands (list, clear-cache, start, stop, restart)
- 5s list cache, write-lock around state mutations, alphanumeric + symbol name validation
- New frontend: `src/lib/components/Services.svelte` — sidebar item ⌘5, sortable Name/Status/User columns, per-row action buttons (smart-disabled by current state)
- **Sidebar badge** = count of running services
- **PackageDetail Service card** — pill + 3 buttons when the formula has a `brew services` entry

### 11c — Native macOS feel
- `window-vibrancy = "0.6"` dep + `apply_vibrancy(NSVisualEffectMaterial::HudWindow, …)`
- `tauri.conf.json` — `transparent: true`, `titleBarStyle: "Overlay"`, `hiddenTitle: true`
- Sidebar brand-wrap padded `44px` top to clear traffic-light cluster
- `data-tauri-drag-region` on brand-wrap + every panel-head (later refactored in v0.2.0 when title bar moved to dedicated header)
- New capability: `core:window:allow-start-dragging`

### 11d — Activity persistence
- New: `activity` store gets a localStorage mirror at `brew-browser:activity:v1`
- Cap: 50 jobs / 500 lines per job (configurable later in Phase 12b's Settings → Activity)
- Debounced 400ms writes + immediate flush on terminal events (Finished, Failed)
- Hydration on `+layout.svelte` mount

### Bonus fixes in the same session
- **Sortable lists hardening** — `1fr` → `minmax(0, 1fr)` everywhere a flex column had text; `auto` → `90px` for the installed column
- **Trending Refresh fix** — force flag now busts the backend cache before calling `trending_fetch` (was silently ignored)
- **Dashboard scroll + drag bug fix** — removed the fixed-position drag-overlay (was eating scroll wheel + not actually triggering drag); fixed flex children getting shrunken with `.body > * { flex-shrink: 0 }`

## Files (subset)

- New: `Dashboard.svelte`, `Services.svelte`, `services.rs`, `disk_usage.rs`, `SortableHeader.svelte` (shared with Phase 9b)
- Modified: 41 files total; Cargo.toml + tauri.conf.json + capabilities + many components

## Tests / verification

- `cargo test`: rough running count ~250 (exact delta unknown — pre-commit history not fully tracked)
- `cargo clippy --all-targets -- -D warnings`: clean
- `npm run check`: 0 errors

## Notes / decisions

- Donut chart hand-rolled as SVG (no chart library); ~180 lines, zero new deps, perfect for the 8-category palette we needed.
- Services name validation strict on purpose — service names map directly to launchd plist paths.
- `disk_usage` uses `tokio::join!` to survey all 4 paths in parallel; 60s cache so subsequent renders are instant.
- "Reveal in Finder" gated to require the target path is inside the Homebrew prefix or cache (no arbitrary-path Finder reveals).
- Activity persistence design choice: **debounced** to avoid hammering localStorage on every log line; **immediate flush** on terminal events so a crash doesn't lose the "what was running when it died" trail.
