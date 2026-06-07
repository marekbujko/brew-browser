# 08 — Native Activity parity (+ Tauri mirror)

**Date:** 2026-06-05/06
**Branch:** `experiment/native-swift-liquid-glass`

## Objective

Bring the native Activity feature to full parity with Tauri and fix the
"feels empty" complaint. Root cause: native recorded Activity jobs for only 5
operations; the two highest-frequency ones produced nothing.

## What changed

### Fill the panel (the empty-history fix)
- `AppModel.upgradeAll()` (`brew upgrade`) + `updateHomebrew()` (`brew update`),
  both through the existing `startJob` streaming engine.
- `DashboardView` UpdatesCard: wired the previously-dead "Upgrade all" button and
  added an "Update" button.

### Drawer parity
- Live elapsed `m:ss` timer while running (`TimelineView(.periodic)`); `durationMs`
  now set on completion.
- Completion footer: "Done in N.Ns." / "Failed after N.Ns. Exit N." + a
  **Report to brew-browser** link / "Stopped."
- Content-based line classification coloring (`==>`/error/warning/download…),
  matching Tauri's `classifyLine`; ANSI strip.
- `ReportIssue.swift` (new) — prefilled GitHub new-issue URL (app+brew version,
  command, exit code, stderr excerpt cap 2000, `from-app` label) via NSWorkspace.
  Mirrors `src/lib/util/reportIssue.ts`.

### stderr tagging
- `BrewService.runStreaming` now uses **two pipes** (stdout/stderr), each with its
  own drain handler; `StreamEvent.line` carries `isStderr`. Replaces the old
  merged-pipe "everything is stdout" (so stderr coloring + the report excerpt
  actually work). No deadlock — each handler drains its own pipe.

### History list management
- Per-job delete: **hover-revealed trash** icon (grey→red), plus swipe and
  right-click. Running job is canceled first (`removeJob`).
- Exit code shown on failed rows; duration shown per row.

### Tabs removed (both builds)
- Decided there's no good UX for a per-job tab strip — it degenerates into a row
  of identical labels (see Tauri screenshot of N× "Updating Homebrew taps").
- Native: never shipped the segmented switcher (built then removed).
- Tauri mirror: removed the `.tabs` strip from `ActivityDrawer.svelte` + dead CSS;
  added per-row delete to `ActivityHistory.svelte` (the tab `×` removal behavior,
  moved to the panel where it scales).
- **The Activity panel is now the single source of truth** for selecting what
  shows in the tray and for removing entries.

### Focus behavior
- A new job won't yank the drawer away from a job you're actively watching; it
  still lands in the Activity panel.

## Files
- Native: `ActivityView.swift`, `AppModel.swift`, `BrewService.swift`,
  `DashboardView.swift`, `ReportIssue.swift` (new).
- Tauri: `ActivityDrawer.svelte`, `ActivityHistory.svelte`.

## Outcome
`swift build` clean (no warnings); Tauri `npm run check` 0 errors. App rebuilt.
