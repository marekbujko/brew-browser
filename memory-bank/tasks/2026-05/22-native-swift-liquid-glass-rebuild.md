# 2026-05-31 — Native Swift / SwiftUI / Liquid Glass rebuild (experiment)

**Phase:** Experimental native rebuild — port the Tauri interface to Swift 6 + SwiftUI + Liquid Glass (macOS 26 Tahoe)
**Status:** 🚧 In progress — Dashboard + package detail + Settings + **Library** + data layer done and building clean; inspector resize hardened; Library crash + casks + keychain-prompt fixes landed; remaining panels (Discover/Trending/Snapshots/Services/Activity) pending. First commit 2026-05-31; Library + fixes commit 2026-06-01.
**Branch:** `experiment/native-swift-liquid-glass` (off `main`). First commit of `native/` on this branch; still not on the PR-into-main path.
**Workflow:** Experiment branch; not on the PR-into-main release path. `main` (Tauri v0.5.0) untouched.

## Scope

The v0.5.0 launch drew recurring "Tauri isn't native" criticism. This branch answers it empirically: a **faithful port** of the shipped app's interface to a fully native macOS app — Swift 6, SwiftUI, Liquid Glass — living in `native/` as a Swift Package. It is a port, **not** a redesign: data sources and functionality stay identical to the Tauri app (trending, AI-enhanced categories, GitHub integration, vulnerability scanning, auto-update). The Tauri app + the memory bank are the complete spec.

## Decisions

- **D1 — Port, not redesign.** Reuse every data contract verbatim. `settings.json` keeps the same path + schema as the Rust `Settings`; bundled `categories.json` + `enrichment.json` are copied from `src-tauri/data/`. brew/vulns/github/trending behaviors are reimplemented as Swift `actor`s mirroring the Rust modules. Recorded in `decisions.md` (2026-05-30 ADR). Does **not** supersede the 2026-05-23 "Tauri 2" decision — production stays Tauri on `main`.
- **D2 — SPM (`swift build`), not an Xcode project.** Full Xcode is installed but `xcode-select` points at Command Line Tools, so `xcodebuild` is unavailable from the CLI. `swift build` works under CLT and links every Liquid Glass API. `native/build-app.sh` wraps the SPM binary into a launchable `.app`. Switching the toolchain (`sudo xcode-select -s`) declined.
- **D3 — Stock Apple scaffolding only, no overrides.** No custom window chrome, no `NSVisualEffectView`, no faked backgrounds. `NavigationSplitView`, `.inspector`, `Settings {}` + `SettingsLink`, `TabView`, `Form` carry the whole UI. When the stock default and a pixel-perfect custom look disagree, the stock default wins. (Established after several failed attempts to hand-build Xcode-like chrome.)
- **D4 — Concurrency:** `@Observable @MainActor` view models + `static let shared` singletons for app state; `actor` types for all I/O.
- **D5 — Bundle enrichment uncompressed** in the native build for now (Tauri ships it gzipped). 2.8 MB `enrichment.json` + 838 KB `categories.json` in `Resources/`.

## What landed

### Done + building clean (`./build-app.sh debug`, 0 errors)

- **Dashboard** (`DashboardView.swift`) — feature parity vs Tauri, verified side-by-side. Hero strip (3-up StatTiles), Updates card (aligned columns), Composition (stacked bar / pie when paired), Top-categories donut, Storage breakdown. Responsive via `.onGeometryChange` (breakpoint 980).
- **Package detail inspector** (`PackageDetailView.swift`) — stock `.inspector(isPresented:)` bound to `AppModel.showDetail`. All 14 sections render live (meta, summary, homepage, categories, tags, Security, install-Trend sparkline, use-cases, similar, GitHub stars/forks, caveats, deps), gated by settings. Plain-text title in content; `FlowRow: Layout` for pill wrapping; footer Upgrade/Uninstall with confirmation. **Inspector resize hardened** (see Resize/crash fixes below).
- **Settings** (`SettingsView.swift`) — `Settings {}` scene + `SettingsLink` toolbar gear, ⌘, opens it. Stock 9-tab `TabView` (Appearance / Network / GitHub / Brew / Updates / Security / Trending / Activity / About), `Form`/`.formStyle(.grouped)`. Every toggle wired to real systems (AppSettings gates, LocalPrefs, BrewService analytics, GitHubService device-flow sign-in/out, VulnsService install helper).
- **Data layer (6 services + LocalPrefs)** — `BrewService` (actor over `brew`: list/info/outdated/storage/upgrade/uninstall/install/analytics), `AppSettings` (settings.json same path/schema + gating + `reset()`), `Enrichment`/`EnrichmentCatalog` (bundled enrichment.json), `VulnsService` (`brew vulns --json`, all 5 smoke-test gotchas), `GitHubService` (OAuth device flow + Keychain via Security.framework, client_id `Ov23liJZKbvrSBuiOPkT`), `TrendingHistoryService` (opt-in sparklines from `brew-browser.zerologic.com`), `LocalPrefs` (UserDefaults: theme/applyTheme, landing section, confirm-destructive, activity caps).
- **Library panel** (`ContentView.swift` `LibraryView` + `AppModel`, 2026-06-01) — native SwiftUI `Table` (the macOS sortable multi-column control: click-to-sort headers w/ chevron, resizable/draggable columns, native selection). Columns: Name (icon) · **Description** (AI-gated) · Version (mono) · Type (`KindPill`) · Outdated (orange ↑). **Centered segmented type filter** above the table — `All · Formulae · Casks · Outdated`, counts in labels (Finder/Preview view-switcher convention). Row selection drives the shared `.inspector`. New `LibraryFilter` enum + `LibraryRow` row-model + `sortedLibraryRows`/`libraryFilterCount` on `AppModel`; replaced the old `libraryOutdatedOnly` bool. Dashboard entry points (`openInLibrary`/`openLibrary`/`openOutdatedInLibrary`) set the filter. **Out of scope** (as planned): no "Vulnerable" pill (needs library-wide scan-all, deferred), no category chips (Discover not ported). See "Library bug fixes" below.

- **Discover panel** (`DiscoverView.swift` + `CatalogService` + `IconService`, 2026-06-01) — browse the full Homebrew catalog (8,369 formulae + 7,659 casks) via a stock `Table` + centered category `Picker`, reusing Library's pattern (single shared `globalQuery` search, static-column-set variants, selection→inspector). New: `CatalogService` (actor) loads + gunzips the bundled `catalog/{formula,cask}.json.gz` via Apple's `Compression` framework (verified: strip 10-byte gzip header + 8-byte trailer, `COMPRESSION_ZLIB`); `IconService` (actor) does **full app-icon discovery** — Appcasks (`github.com/App-Fair/appcasks/releases/download/cask-<token>/AppIcon.png`) → Google favicon (`google.com/s2/favicons?domain=<host>&sz=64`) → SF Symbol, on-disk cached at `~/Library/Application Support/brew-browser/native-icons/`, gated by Offline Mode + `caskIconMode` (formulae always SF Symbol). **Icon cascade ALSO ported to Tauri** (`504810b`, `cask_icon_homepage.rs`) — parity done.
- **App icons everywhere** (`PackageIcon`, 2026-06-01) — extracted the Discover icon view into a shared `PackageIcon(model:token:kind:homepage:size:)`; used in Library (both column variants), Discover, and the toolbar search-suggestions dropdown. Same Appcasks→favicon→SF-Symbol resolution + `iconCache` everywhere. NOTE: `LibraryRow` has no `homepage`, so installed casks resolve via Appcasks(token-only) or the shared cache; favicon fallback only fires where homepage is known (Discover). Add `homepage` to `LibraryRow` if guaranteed Library fallback is wanted.
- **Detail panel icon + install-aware footer** (2026-06-01) — centered 64pt `DetailIcon` above title; footer adapts Install↔Uninstall(+Upgrade) off the **live installed list** (`model.installed`, not cache-laggy `brew info`). `OutdatedPackage` now carries `kind` (cask pills correct).
- **Activity system** (`Activity.swift` + `ActivityView.swift`, 2026-06-01) — streaming brew **jobs** (install/upgrade/uninstall) via `BrewService.runStreaming(jobId:_:)` (merged stdout+stderr, stdin=/dev/null so sudo/.pkg prompts fail fast + visible, cancel via SIGTERM). `ActivityJob`/`ActivityLine` Codable model, persisted to UserDefaults (cap 50 jobs / 500 lines). Bottom **`ActivityDrawer`** (full-width bar BELOW the split view in a VStack — NOT a safeAreaInset, which stole the inspector footer): live console, status-aware label (Installing→Installed), auto-collapse on success/stay-open on failure, copy + cancel + **close (X)** + chevron. **`ActivityView`** = the Activity panel (job history, click→drawer, Clear Finished). Sidebar badge = running count.
- **Trending panel** (`TrendingService.swift` + `TrendingView.swift`, 2026-06-02) — top-100 **formulae** install leaderboard from `formulae.brew.sh/api/analytics/install/{window}` (casks excluded, matching Tauri). Stock `Table`, centered 30d/90d/365d window control, Refresh button + ⌘R + "Updated Ns ago" label, 60-min in-memory cache (force-bypass on Refresh). **Velocity computed locally** from the 3 analytics windows (verbatim port of Tauri `velocity_index(c30,c90,c365)`), shown as an always-on column; **default sort = Velocity desc** (surging-first, Tauri v0.4.0 default). Sparklines (stacked under install count, leading-zeros trimmed) are the **opt-in** layer from the zerologic history index (`enhancedTrendingAllowed`). **Three hard-won corrections**: (1) velocity must come from analytics windows, NOT the zerologic index (which is a separate velocity-ranked top-500 — wrong population, blank descs); (2) cap at top-100 BEFORE building/sorting rows — handing ~28k rows to the Table made sorting crawl; (3) the zerologic `index.json` is `{packages:[…]}`, not a bare array — decode the wrapper or velocity silently stays empty.
- **GitHub actions full parity** (2026-06-01) — `PackageDetailView` GitHub card now has **Star / Watch / File-issue** (was stats + silent Star only) with **device-flow sign-in interception** (signed-out action → opens GitHub verify page, copies code, sheet, polls; then proceeds). `AppModel` star/watch/issue + `ensureGitHubSignIn`. **GitHub homepage cascade** ported (Tauri task #17): `PackageInfo.githubHomepage` resolves homepage → urls.stable.url → urls.head.url (formula) / homepage → url (cask) via `GitHubService.resolveGithubURL`, so packages with non-GitHub homepages but GitHub source show the card. **GitHub avatar disclosure note:** showing formula icons via `github.com/<owner>.png` would reuse the EXISTING github.com outbound line (per-host disclosure), no new resource — but NOT done (Tauri shows no formula icons; matching it).
- **Friendly-name subtitles in all 3 lists** (2026-06-02) — Library, Discover, Trending all show the AI `friendlyName` (enrichment) as a dimmed subtitle under the token (AI-on + differs from token). `friendlyName` added to `LibraryRow`/`DiscoverRow`/`TrendingRow`; previously Discover/Trending accidentally showed catalog displayName. Matches Tauri `PackageRow`.

### Pending

- **Discover view-switcher** — list (current `Table`) vs icon/grid (`LazyVGrid`, bigger 32/48px icons), Finder-style toolbar toggle. Grid needs lazy per-visible-tile icon loading. Deferred to its own task.
- **Snapshots / Services** panels (placeholders).
- **Dashboard GitHub "starred N of M" card** — reachable now Settings sign-in exists; needs a batch resolver for installed packages with github homepages.
- **Sparkle** for real in-app updates — only genuinely-deferred subsystem (Updates auto-check toggle persists; install is a stub).
- **Vulns "scan all"** from Settings (`VulnsService` has `scanOne` only).
- **Discover icon perf** — first browse fires many concurrent icon fetches (cached after). Watch for jank on large windows; consider lazy-resolve only visible rows if it bites.

## Resize / crash fixes (2026-05-31, inspector divider)

Dragging the package-detail inspector divider had three distinct failures, all diagnosed from macOS crash reports at `~/Library/Logs/DiagnosticReports/BrewBrowser-*.ips` (the `.ips` `lastExceptionBacktrace` — **no Xcode needed**; SPM `.app` builds produce full system crash reports).

1. **SIGABRT crash mid-drag.** `.inspectorColumnWidth` applied to the *conditionally-created* `PackageDetailView` (`if let pkg = …`) let the hosting view re-report its min/max during the drag → re-entrant `NSWindow` constraint update inside `-[NSSplitView mouseDown:]` → `abort()`. **Fix:** put `.inspectorColumnWidth` on an always-present `Group` wrapper and let the content fill the column (`.frame(maxWidth:.infinity, maxHeight:.infinity)`), giving the inspector a stable size contract during the gesture. (`ContentView.swift`.)
2. **Window resized / panel hid instead of the inspector resizing.** Dragging near the window edge got grabbed as a window resize. **Fix:** firm `minWidth: 420` on the detail (main) pane so the divider drag takes content space down to a floor and stops, plus `.defaultSize` + `.windowResizability(.contentMinSize)` on the scene (`BrewBrowserApp.swift`). Inspector floor also raised to `min 360, ideal 400, max 560` so accidental small drags don't reach the collapse zone.
3. **Empty panel after drag-collapse-then-reopen** (the load-bearing one). The `.inspector(isPresented:)` binding was computed off `detailPackage != nil` with a setter that called `closeDetail()` — so a drag-to-edge collapse *destroyed the package data*, and re-expanding within the same gesture showed blank space. **Fix:** decoupled presentation from data. New `AppModel.showDetail` Bool is the presentation flag the inspector binds to (a drag-collapse just sets it false, package survives); `detailPackage` is the data, cleared **only** by `closeDetail()` (the ⊗ close box). `openDetail` sets both; the ⊗ clears both. Verified by the user: open → resize → drag to edge → drag back open restores the same package with content intact.

**Stock-only caveat:** SwiftUI's `.inspector` still self-dismisses when the divider is dragged fully past `min` — that's Apple's built-in collapse, not removable without fighting `NSSplitView` (the override that caused fix #1's crash). The raised `min` makes it deliberate rather than accidental; the ⊗ is the intended dismiss.

## Library bug fixes (2026-06-01)

1. **Crash on clicking the Library sidebar tab** (SIGTRAP via `+[NSApplication _crashOnException:]` during `_NSViewLayout`). Real exception — `NSToolbar already contains an item with the identifier com.apple.SwiftUI.search` — was **not** in the `.ips` report; caught by running the binary under `lldb -o "breakpoint set -n objc_exception_throw" -o run -o "po (id)$arg1"`. Cause: **two `.searchable` modifiers** live at once — the global one on the detail column (`globalQuery`) plus `LibraryView`'s own (`query`) — both claiming the same toolbar search item. **Fix:** one search field only. Removed `LibraryView`'s `.searchable`; the table filters off the shared `globalQuery`. Deleted the now-dead `query` property; pointed `openInLibrary` + empty states at `globalQuery`. *Lesson: for SIGTRAP-via-`_crashOnException`, go straight to lldb on `objc_exception_throw` — the `.ips` hides the reason. And: exactly ONE `.searchable` per toolbar.*
2. **False detour:** before lldb, a conditional `TableColumn` (the AI-gated Description column inside one `Table` builder) was *suspected* from the incomplete `.ips` and refactored into two static-column `Table` variants (`tableWithDescription` / `tableNoDescription`). Kept — conditional columns are genuinely unstable and static sets are the right pattern — but it was **not** the crash. The `.searchable` duplicate was.
3. **Casks missing (Casks (0)).** `loadLibrary` only called `listInstalledFormulae`. **Fix:** added `BrewService.listInstalledCasks` + `listInstalledAll` (loads both concurrently, name-sorted); `loadLibrary` now loads both and derives formula/cask counts from `installed`.
4. **Filter bar floated mid-pane.** The content `VStack` centered. **Fix:** `.frame(maxWidth:.infinity, maxHeight:.infinity, alignment: .top)` on the stack + fill the table.
5. **Filter centered** per macOS view-switcher convention (`.frame(maxWidth:.infinity, alignment:.center)` on the segmented `Picker`).

## Keychain: 3 prompts → 1 (2026-06-01)

`GitHubService.status()` made **three** separate `SecItemCopyMatching` reads (token / username / scopes) — each Keychain access prompts independently, so launch showed three consecutive prompts. **Fix:** new `keychainReadAll()` does a single `SecItemCopyMatching` with `kSecMatchLimitAll` + `kSecReturnAttributes` for the whole `com.zerologic.brew-browser` service, returning all accounts keyed by name; `status()` reads from that one result. No schema change; still shares entries with the Tauri app. *Note: the app still prompts once per **rebuild** because `build-app.sh` ad-hoc-signs (new code identity each build → "Always Allow" ACL doesn't persist). Eliminating that is a stable-codesign-identity tooling change, deferred — it's dev-loop-only; a signed release wouldn't prompt. User is satisfied at one prompt.*

## Files

`native/` (committed 2026-05-31):
- `Package.swift`, `build-app.sh`, `README.md`
- `Sources/BrewBrowser/{BrewBrowserApp,ContentView,AppModel,DashboardView,PackageDetailView,SettingsView}.swift`
- `Sources/BrewBrowser/{BrewService,AppSettings,LocalPrefs,Enrichment,VulnsService,GitHubService,TrendingHistoryService,Categories}.swift`
- `Sources/BrewBrowser/Resources/{categories.json,enrichment.json}`

~4,800 lines of Swift across 14 source files.

## Build / verification

- `cd native && ./build-app.sh debug` → **Build complete, 0 errors.**
- Launch: `killall BrewBrowser; open native/BrewBrowser.app`. Visual verification by the user via screenshots.
- No automated test suite on the native side yet (the Tauri Rust suite remains the load-bearing test surface for the shipped product).

## Lessons (gotchas captured so they aren't relearned)

- **`.backgroundExtensionEffect()`** mirrors+blurs *window content* under sidebars — the source of the "line through the toolbar." Intrinsic to the effect.
- **`.inspector`** draws a non-removable divider at its boundary while in use. Accepted.
- **macOS owns `.searchable` placement** (trailing, collapses to `»` overflow). Safari-style centering needs `.principal`, which broke layout.
- **The Xcode full-height-sidebar look** (traffic lights inside the sidebar) needs AppKit `NSWindow.fullSizeContentView` + `sidebarTrackingSeparator` — not achievable in a pure-SwiftUI `Settings` scene. Don't chase it; the stock top-tab `TabView` was the right answer all along.
- **Never name an app type `Section`/`Form`** — shadows the SwiftUI symbol, cascades into ~60 bogus errors. Qualify `SwiftUI.Section` at use sites.
- `AppSettings.save()` throws → `try? settings.save()` at every set closure; actor methods need `await` in a `Task`; segmented `Picker` over a custom `Binding` needs explicit `.tag()` rows, not `ForEach(allCases)`.
- **`.inspector` presentation must be decoupled from the underlying data** — bind `isPresented` to a dedicated Bool, not to `data != nil` with a destroy-on-false setter, or a drag-collapse wipes the data and re-expand shows an empty pane. See Resize/crash fixes #3.
- **Diagnose macOS crashes from `~/Library/Logs/DiagnosticReports/*.ips`** (`lastExceptionBacktrace`) — SPM-built `.app`s produce full system crash reports; Xcode is only needed for live breakpoints on non-crashing bugs.

## Cross-reference

- `native/README.md` — build loop + full source map.
- `memory-bank/decisions.md` — 2026-05-30 ADR (rebuild rationale + sub-decisions).
- `memory-bank/techContext.md` — "Native rebuild" stack section.
- `memory-bank/progress.md` — 2026-05-31 entry.
- Cross-session memory: `~/.claude/projects/-Users-michael-Software-brew-browser/memory/project-native-swift-rebuild.md`.
