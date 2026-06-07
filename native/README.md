# brew-browser — native Swift / Liquid Glass rebuild (experimental)

> **Status: experimental spike on branch `experiment/native-swift-liquid-glass`.**
> This is a faithful **port** of the shipped Tauri app's interface and
> functionality to a fully native macOS app — Swift 6 + SwiftUI + Liquid Glass
> (macOS 26 "Tahoe"). It is **not** released and **not** committed; the entire
> `native/` tree currently lives uncommitted on the experiment branch. The
> production app remains the Tauri build on `main`.

## Why this exists

The launch surfaced recurring "Tauri isn't native" chatter. Rather than argue it,
this branch rebuilds the same product natively to see how close stock Apple
components get us. The Tauri app + the memory bank are the complete spec: the
data sources and functionality stay **identical** (trending, AI-enhanced
categories, GitHub integration, vulnerability scanning, auto-update) — only the
shell changes from WebView/Svelte/Rust to SwiftUI.

Guiding constraint (learned the hard way during the spike): **stock Apple
scaffolding only, no overrides.** No custom window chrome, no
`NSVisualEffectView` hacks, no faked backgrounds. When the stock default and a
pixel-perfect custom look disagree, the stock default wins.

## Toolchain

| Thing | Value |
|-------|-------|
| Language | Swift 6.3.x |
| UI | SwiftUI + Liquid Glass (`.glassEffect`, `GlassEffectContainer`, `.buttonStyle(.glass)`) |
| Min OS | macOS 26.0 (Tahoe) |
| Build system | **Swift Package Manager** (`swift build`) — *not* an Xcode project |
| Charts | Swift Charts (`SectorMark`, `LineMark`) |

**Why SPM and not `xcodebuild`?** Full Xcode *is* installed at
`/Applications/Xcode.app`, but `xcode-select` on this machine points at the
Command Line Tools, so `xcodebuild` is unavailable from the CLI. `swift build`
works fine under CLT and links every Liquid Glass API. You can still open
`native/Package.swift` directly in Xcode for Previews / Run. Switching the active
toolchain would need `sudo xcode-select -s` (intentionally not done — kept on CLI
`swift build`).

## Build & run

```sh
cd native
./build-app.sh debug        # swift build + wrap into BrewBrowser.app
killall BrewBrowser 2>/dev/null; open BrewBrowser.app
```

`swift build` alone produces a bare executable with no `Info.plist`, which macOS
treats as a background process (no Dock icon, won't activate).
`build-app.sh` wraps the SPM binary into a launchable `.app` bundle and copies
the SPM resource bundle (so `categories.json` / `enrichment.json` resolve via
`Bundle.module` at runtime). Pass `release` for an optimized build.

## Architecture

`@Observable @MainActor` view models + `static let shared` singletons for app
state; `actor` types for anything doing I/O (subprocess, network, Keychain).
Stock SwiftUI containers throughout: `NavigationSplitView` (sidebar + detail),
`.inspector(isPresented:)` (package detail), `Settings {}` scene + `SettingsLink`
(preferences), `TabView` (settings panes), `Form`/`.formStyle(.grouped)`.

### Source map — `Sources/BrewBrowser/`

| File | Role |
|------|------|
| `BrewBrowserApp.swift` | `@main` App — `WindowGroup { ContentView() }` + `Settings { SettingsView() }` |
| `ContentView.swift` | `NavigationSplitView` shell — sidebar list w/ count badges, toolbar, `.searchable`, `.inspector` |
| `AppModel.swift` | Root `@Observable` model — sidebar `Section` enum, detail state, open/close/load detail |
| `DashboardView.swift` | Dashboard panel — hero strip, Updates, Composition (bar/pie), Top-categories donut, Storage |
| `PackageDetailView.swift` | Inspector content — all 14 detail sections, gated by settings; `FlowRow` pill layout |
| `SettingsView.swift` | Stock `TabView` with 9 panes (Appearance/Network/GitHub/Brew/Updates/Security/Trending/Activity/About) |
| `BrewService.swift` | `actor` over `brew` subprocess — list/info/outdated/storage/upgrade/uninstall/install/analytics |
| `AppSettings.swift` | `settings.json` (same path + schema as the Tauri app) + feature gating (paranoid/vuln/github/trending/AI) |
| `LocalPrefs.swift` | `UserDefaults` prefs (theme, default landing section, confirm-destructive, activity caps) |
| `Enrichment.swift` | Bundled `enrichment.json` loader — friendly names, summaries, use-cases, similar, tags |
| `VulnsService.swift` | `actor` over `brew vulns --json` — replicates all 5 smoke-test gotchas from the Rust impl |
| `GitHubService.swift` | `actor` — OAuth device flow, Keychain via Security.framework, repo stats |
| `TrendingHistoryService.swift` | `actor` — opt-in per-package sparklines from `brew-browser.zerologic.com` |
| `Categories.swift` | Bundled `categories.json` loader for the Dashboard donut |
| `Resources/` | `categories.json` (838 KB) + `enrichment.json` (2.8 MB, uncompressed) |

Data layers mirror the Tauri app exactly:
- **`settings.json`** — shared file, same path/schema as Rust (`AppSettings`), atomic write, gating logic ported.
- **`UserDefaults`** — `LocalPrefs` for view-only prefs (theme, landing section).
- **Bundled JSON** — `categories.json` + `enrichment.json` (the Tauri app ships these gzipped; the native build ships them uncompressed for now).
- **`brew` subprocess** — `BrewService` / `VulnsService` actors, cwd pinned to `/`.

## Parity status

| Surface | State |
|---------|-------|
| Dashboard | ✅ feature parity (verified side-by-side vs Tauri) |
| Package detail inspector | ✅ all 14 sections render live (Security / Trend / GitHub stars verified) |
| Settings | ✅ 9-tab stock `TabView`, all toggles wired to real systems |
| Data layer | ✅ 6 services + `LocalPrefs` ported |
| Library panel | 🚧 row→detail wired; needs kind pills, sort, filters |
| Discover / Trending / Snapshots / Services / Activity | 🚧 placeholders |
| Dashboard GitHub "starred N of M" card | 🚧 reachable now Settings sign-in exists; needs batch resolver |
| In-app updates (Sparkle) | ⏸ deferred — the only genuinely-unported subsystem; Updates tab auto-check toggle persists, install is a stub |
| Vulns "scan all" from Settings | ⏸ `VulnsService` currently has `scanOne` only |

## Build loop

```
edit → cd native && ./build-app.sh debug → killall BrewBrowser; open BrewBrowser.app → screenshot
```

## Lessons baked in (don't relearn these)

- **`.backgroundExtensionEffect()`** mirrors+blurs *window content* under sidebars — that's what put a "line through the toolbar." It's intrinsic to the effect, not a bug.
- **`.inspector`** draws a divider at its boundary that is not removable while using `.inspector`. Accepted.
- **macOS owns `.searchable` placement** — it renders trailing and collapses to a `»` overflow when narrow. True Safari-style centering needs `.principal`, which broke layout.
- **The Xcode full-height-sidebar look** (traffic lights inside the sidebar) needs AppKit `NSWindow.fullSizeContentView` + `sidebarTrackingSeparator` — not achievable in a pure-SwiftUI `Settings` scene. Don't chase it.
- **Never name an app type `Section` / `Form`** etc. — it shadows the SwiftUI symbol and cascades into dozens of bogus errors. (The app's sidebar enum is named `Section` and works, but qualify `SwiftUI.Section` at use sites.)
- `AppSettings.save()` throws → `try? settings.save()` at every set closure. Actor methods (`github.status()` etc.) need `await` inside a `Task`.

See `memory-bank/tasks/2026-05/22-native-swift-liquid-glass-rebuild.md` for the
full task record and `memory-bank/decisions.md` (2026-05-30 ADR) for the rebuild
rationale.
