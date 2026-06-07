# Tasks — 2026-06

Per-task records for June 2026 work on brew-browser. Two parallel tracks this
month (per the parity charter — `decisions.md` 2026-06-01, memory
`project-parity-charter`): the shipped **Tauri** app on `main` and the **native**
macOS rebuild on `experiment/native-swift-liquid-glass`. Numbering is per-track;
filenames are prefixed `NN-tauri-*` / `NN-native-*` so they don't collide.

## Native track (`experiment/native-swift-liquid-glass`)

Native macOS build under `native/`.

| # | Task | Date | Commit |
|---|---|---|---|
| 01 | [Native live enrichment + task-completion notifications](./01-native-live-enrichment-notifications.md) | 2026-06-03 | `bd5ef9c` + notifications |
| 02 | [Native trending tap-token resolution + Refresh feedback](./02-native-trending-tap-tokens-and-refresh-feedback.md) | 2026-06-04 | `efd45d1` |
| 03 | [Native Services panel (parity with Tauri)](./03-native-services-panel.md) | 2026-06-04 | `df4ce3d` |
| 04 | [Native: close inspector when the section changes](./04-native-detail-autoclose-on-section-change.md) | 2026-06-04 | `24545ed` |
| 05 | [Native: validate snapshot id before path join (parity with Tauri PR #46)](./05-native-snapshot-id-validation.md) | 2026-06-04 | `28c378e` |
| 06 | [Native dashboard load perf (actor→struct, ungated load)](./06-native-dashboard-load-perf.md) | 2026-06-04 | (this commit) |
| 07 | [Native GitHub toolbar chip + dashboard card + one-prompt keychain](./07-native-github-toolbar-card-oneprompt.md) | 2026-06-05 | `fbf6aad` |
| 08 | [Native Activity parity (+ Tauri mirror)](./08-native-activity-parity.md) | 2026-06-06 | (this commit) |
| 09 | [Native ← Tauri parity roadmap (remaining gap)](./09-native-parity-roadmap.md) | 2026-06-06 | (planning doc) |
| 10 | [Native parity polish, Sparkle end-to-end, "Brew Browser" rename](./10-native-parity-polish-sparkle-rename.md) | 2026-06-06 | `892cbdf`…`e08a377` |

## Tauri track (`main`)

Shipped Tauri app.

| # | Task | Date | Branch | Release |
|---|---|---|---|---|
| 01 | [Tauri←native feature parity (icons, Dashboard charts, keychain one-prompt, velocity threshold)](./01-tauri-native-parity.md) | 2026-06-02 | `tauri-parity` (#37) | — |
| 02 | [Tauri trending tap-token resolution (native parity)](./02-tauri-trending-tap-tokens.md) | 2026-06-04 | `fix/tauri-trending-tap-tokens` (#44) | — |

## Context

The `experiment/native-swift-liquid-glass` rebuild (native macOS 26 app under
`native/`) raced ahead in a few areas, then the two builds were brought back into
feature/data-contract parity in both directions. Per the parity charter
(`decisions.md` 2026-06-01 + memory `project-parity-charter`), feature/data work
is kept in sync across the two shells; the memory-bank is the single canonical
spec for both. The Tauri track this month brought the shipped app up to the
native build's list/detail icons + Dashboard charts, one-prompt Keychain, and the
canonical Trending velocity threshold, plus the live-enrichment pipeline + app
(#41/#42/#43). The native track closed the reverse gap (Services, Activity,
GitHub, vulnerability surfacing) and finished with Sparkle self-update + the
"Brew Browser" rename ahead of deploy.
