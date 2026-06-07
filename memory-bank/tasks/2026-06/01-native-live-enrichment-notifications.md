# 01 — Native live enrichment + task-completion notifications

**Date:** 2026-06-03
**Branch:** `experiment/native-swift-liquid-glass` (commits `bd5ef9c` + the notifications commit)

## Objective

Bring the native build to parity with the new Tauri **live category/description updates** feature (PR #43), and add native-only **macOS task-completion notifications** (a native advantage the web build can't match).

## Outcome

- ✅ `swift build` clean; app builds + launches via `native/build-app.sh`.
- ↩️ Runtime verification (toggle on → open package → live overlay; background job → notification) pending a manual GUI pass.

## Live enrichment (native, opt-in)

Mirrors the Tauri side (PR #43) and the existing `TrendingHistoryService` pattern.

- `AppSettings.swift` — `liveEnrichmentEnabled` (default false) through the DTO/save round-trip + `liveEnrichmentAllowed` gate (paranoid + toggle + AI features).
- `EnrichmentLiveService.swift` (new) — actor; soft-fail GET of `version`/`categories`/`entry/<token>` from `brew-browser.zerologic.com/enrichment/*`; token allowlist.
- `Enrichment.swift` — `parseLiveEntry` (camelCase served shape). `Categories.swift` — `parse(data:)` so a live `categories.json` can replace the bundle.
- `AppModel.swift` — `liveEnrichment` overlay (`enrichmentEntry` preferred over bundled at all 4 read sites); `ensureLiveEnrichment` on detail open; `refreshLiveCategoriesIfNewer` + `resetLiveEnrichment` on refresh.
- `SettingsView.swift` — toggle + disclosure (mirrors the trending section).

Full feature context (Phase 1 pipeline + Tauri side + deployment): memory `project-live-enrichment`.

## Task-completion notifications (native-only)

- `NotificationService.swift` (new) — `UNUserNotificationCenter` wrapper. Posts only when (a) opted in, (b) the app is **not** frontmost (foreground keeps the Activity drawer), (c) authorized, and (d) the job wasn't user-canceled. Guards `Bundle.main.bundleIdentifier != nil` (works in the `.app`, not a bare `swift run`).
- `LocalPrefs.swift` — `notifyOnTaskCompletion` (native-only UserDefaults pref, default false).
- `AppModel.startJob` — posts on terminal status.
- `SettingsView` (Activity tab) — toggle; flipping on requests notification auth (no surprise launch prompt).

**Design decision:** native keeps the Activity drawer for foreground feedback (no toasts) and adds *system notifications for background completion* — not toasts, which are a web idiom. Signed builds notify reliably; the unsigned dev `.app` may be flaky.

## Reverse-parity remaining (native)

Per memory `project-native-reverse-parity`: only the **Services panel** (+ command palette / keyboard shortcuts / "Wrong?" corrections) remain.
