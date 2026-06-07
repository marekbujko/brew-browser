# 03 — Native Services panel (parity with Tauri)

**Date:** 2026-06-04
**Branch:** `experiment/native-swift-liquid-glass`

## Objective

Bring the **Services** feature to native parity with the Tauri build. The
`.services` sidebar section existed (icon + running-count badge) but fell through
to `PlaceholderView` — no view, data, or actions. (Snapshots was found already
complete on native — no work needed; the stale "Snapshots/Services stubs" note
was wrong.)

## Outcome

- ✅ `swift build` clean; `.app` rebuilt via `build-app.sh`.
- ✅ Manually verified: start/stop a real service (mailpit) flips its status pill;
  no Activity drawer on success (hybrid UX — drawer only on failure).

## What was built

Mirrors Tauri's `Services.svelte` + `commands/services.rs` + the PackageDetail
service card.

- **`BrewService.swift`** — top-level `Service` model (`name`, `status`, `user?`,
  `file?`, `exitCode?`); `Service.Status` enum (`started/stopped/error/scheduled/
  unknown` + `notLoaded` for brew's `"none"`, avoiding `Optional.none` clash) with
  `sortRank` + `isRunning`; `ServiceVerb` (start/stop/restart). New actor methods:
  `servicesList()` (`brew services list --json`, JSON parsed) and
  `serviceAction(_:name:)` (quiet `brew services <verb> <name>`).
- **`AppModel.swift`** — `services`/`servicesLoading`/`servicesError`/
  `servicesLoaded`/`servicePending` state; `sortedServices` (running→dead by
  `sortRank`, then name — matches Tauri); `loadServices()`; `service(for:)`;
  `performServiceAction()` (**hybrid UX**: per-row spinner on the happy path,
  failures recorded as a failed `ActivityJob` that opens the drawer via
  `recordFailedServiceJob`); lazy `loadServices()` on formula detail-open.
- **`ServicesView.swift`** (new) — stock SwiftUI `Table`: Name (click → detail) ·
  Status pill · User · per-row Start/Stop/Restart (same disable rules as Tauri).
  Header "N running · M total" + Refresh (spinner). Loading / error / empty
  states. Includes `ServiceStatusPill` (green running / orange scheduled / red
  error / gray rest).
- **`PackageDetailView.swift`** — `serviceCard` for an installed formula with a
  service (status pill + user + Start/Stop/Restart), placed after the security card.
- **`ContentView.swift`** — `.services` now dispatches to `ServicesView`.

## Design decision — hybrid action UX

Service start/stop/restart run **quietly** with a per-row spinner; the status pill
flips on completion. Only **failures** surface (as a failed Activity-drawer job).
Chosen over always-streaming-to-drawer because service ops are quick launchd
calls; the drawer would be noise on the happy path. The existing
`countRunningServices()` (text parse, used for the dashboard badge) was left as-is;
`loadServices()` is the JSON-based source for the panel + detail card.

## Reverse-parity status

Per memory `project-native-reverse-parity`: **Services now done; Snapshots was
already done.** Remaining native←Tauri items: ⌘K command palette, keyboard
shortcuts, "Wrong?" category corrections.
