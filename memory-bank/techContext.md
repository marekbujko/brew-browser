# Tech Context

## Stack

| Layer | Choice | Version |
|-------|--------|---------|
| Shell | Tauri | 2.x |
| Frontend framework | SvelteKit + Svelte | 5.x |
| Frontend language | TypeScript | ~5.6 |
| Build tool | Vite | 6.x |
| SvelteKit adapter | `@sveltejs/adapter-static` (SPA fallback to `index.html`) | 3.x |
| Backend | Rust | 1.95 |
| Async runtime | `tokio` (process, io-util, rt-multi-thread, macros, sync) | 1.x |
| Serde | `serde`, `serde_json` | 1.x |
| Tauri opener plugin | `tauri-plugin-opener` | 2.x |
| HTTP (for trending + GitHub + catalog + updater) | `reqwest` (async, with `rustls-tls` + `webpki-roots` features — no system trust store dependency) | 0.12.x |
| Styling | Plain CSS + design tokens (`src/lib/styles/tokens.css`, `typography.css`, `reset.css`). No utility framework; small custom system per `designSystem.md`. | — |
| Test framework — backend | Rust built-in: `#[test]` for unit, `#[tokio::test]` for async, `#[ignore]` for integration tests that shell out to `brew`. 473 unit tests at v0.3.0. | (stdlib) |
| Test framework — frontend | `svelte-check` against the SvelteKit + TS sources (typecheck only — no Vitest yet; backend is the load-bearing test surface). | 4.x |
| Crypto (updater signature verification) | minisign via `tauri-plugin-updater` 2.x; embedded `UPDATER_PUBKEY` const in `src-tauri/src/lib.rs` | 2.10.x |
| Crypto (install-set fingerprint, v0.5.0+) | `sha2` (SHA-256) + `hex` (lowercase hex encoding) — used by `vulns::fingerprint::compute()` to produce a deterministic install-set fingerprint for the vulnerability-scan whole-scan skip predicate | `sha2 = "0.10"`, `hex = "0.4"` |
| Vulnerability scanning (v0.5.0+) | `Homebrew/homebrew-brew-vulns` — official Homebrew subcommand by Andrew Nesbitt (Jan 2026). Shelled out via `tokio::process::Command::new(brew).args(["vulns", ...])` from `src-tauri/src/vulns/client.rs`. Talks to `api.osv.dev` (OSV GIT ecosystem) and the source forges (`github.com`, `gitlab.com`, `codeberg.org`) for source-URL → version-tag resolution. Installed on demand via the one-click `vulns_install_helper` IPC (`brew install homebrew/brew-vulns/brew-vulns`). | n/a (external) |

## Host environment

- Beast (M5 Max, 128 GB unified memory, macOS Tahoe 26.5)
- Homebrew 5.1.13 with `brew bundle` available
- Node 26.0.0, npm 11.12.1, rustc 1.95.0, cargo 1.95.0

## Files of record

```
brew-browser/
├── LICENSE                          MIT
├── README.md                        loud open-source narrative
├── docs/                            BUILD.md, PLAN.md (phase tracker), PHILOSOPHY.md, release-notes/, icon/, screenshots/
├── package.json                     name=brew-browser
├── src/                             Svelte frontend
│   ├── app.html
│   └── routes/
│       ├── +layout.ts               ssr=false (SPA mode)
│       └── +page.svelte             (default scaffold; replace in Phase 1)
├── src-tauri/
│   ├── Cargo.toml                   name=brew-browser, lib=brew_browser_lib
│   ├── src/
│   │   └── lib.rs                   (default greet command; replace in Phase 1)
│   ├── tauri.conf.json              productName=brew-browser, 1100×720 window
│   └── capabilities/default.json    core:default + opener:default
├── memory-bank/                     this directory
└── static/                          favicon + default logos
```

## Tauri capability allowlist

Currently: `core:default`, `opener:default`.

**Phase 1+ will need:** shell-execute capability for `brew`. Plan: `tauri-plugin-shell` with a strict allowlist permitting only `brew` and `brew bundle` invocations. Alternative: stay within Tauri's built-in command execution via Rust (`tokio::process::Command::new("brew")` from inside Tauri commands, no shell plugin needed because the IPC boundary stops the frontend from passing arbitrary arg vectors). **Prefer the Rust-only path** — keeps the attack surface tighter.

## Frontend → Backend IPC

Pattern: `import { invoke } from "@tauri-apps/api/core"` on the frontend, `#[tauri::command]` on the Rust side. Long-running commands stream output via Tauri's event channel (`app.emit_to(<window>, <event-name>, payload)` in Rust; `listen(eventName, callback)` in JS).

## Brew interaction patterns

All `brew` calls go through `tokio::process::Command::new("brew")`. Use `--json=v2` wherever supported (`brew list`, `brew info`, `brew search`). For streaming commands (`install`, `uninstall`, `upgrade`), stream stdout+stderr line-by-line to the frontend via Tauri events.

Serialize concurrent `brew` invocations using a `tokio::sync::Mutex<()>` in Tauri's managed state — `brew` does NOT tolerate concurrent operations against its own state.

## Trending data sources

**Always-on (Homebrew first-party):** `https://formulae.brew.sh/api/analytics/install/{30d,90d,365d}.json` plus the v0.4.0 addition `install-on-request/{30d,90d,365d}.json`. Public Homebrew-maintained JSON. No auth required. The backend fetches both endpoints in parallel for the requested window and eager-warms all three windows on first call (via `tokio::task::JoinSet`) so the server-side velocity index is computable from a single user-facing fetch. In-memory cache TTL configurable in Settings → Network; default 60 minutes.

**Opt-in (project-operated, v0.4.0+):** `https://brew-browser.zerologic.com/trending-history/*` — the Enhanced Trending History endpoint. Distinct trust boundary from the Homebrew first-party paths above. Gated by `Settings.enhanced_trending_enabled` (default false). Served as static JSON by Caddy on the same vhost that serves the updater manifest. Two URL shapes:

- `/trending-history/index.json` — summary blob (top-500 packages with server-precomputed velocity index + ~30-point compact sparkline). Fetched once on Trending tab mount; powers inline row sparklines.
- `/trending-history/{kind}/{name}.json` — per-package full series. Fetched on demand from PackageDetail.

**Collector:** `tools/trending-collector/` — plain Node 20+ ESM cron job that lives on `brew-browser.zerologic.com`. Runs nightly at 03:00 server time, hits the 12 (4 categories × 3 windows) formulae.brew.sh endpoints concurrently, appends rows to `/home/michael/data/brew-trending/db.sqlite` (composite-PK so re-runs are no-ops), then regenerates the static JSON tree at `/home/michael/Sites/brew-trending/` for Caddy to serve.

**Day-zero seed trick:** the bootstrap (`seed.js`) derives three historical "buckets" per package from rolling-window subtraction (c30, c90-c30, c365-c90), tagged `source='seed'`, so charts have data the day the collector turns on. From day 1 onward the nightly collector accumulates real daily snapshots; after ~30 days, adjacent-day `count_30d` subtraction produces clean per-day install estimates that dominate the chart visually.

**Privacy posture for the project-operated endpoint:** IP redacted at the Caddy log layer (`request>remote_ip "0.0.0.0"`), no cookies set/accepted, GET-only (writes 405), 6h Cache-Control. Documented + auditable in `security.md` §16 (the actual Caddy snippet lives there so anyone can `cat Caddyfile` on the server and verify).

## Vulnerability scanning (v0.5.0+)

**Opt-in via `Settings.vulnerability_scanning_enabled` (default `false`).** When the toggle is on AND paranoid mode is off, the backend shells out to the official `brew vulns` subcommand (`Homebrew/homebrew-brew-vulns`, by Andrew Nesbitt, January 2026) to query OSV.dev for known CVEs against installed formulae.

**Why shell out, not query OSV directly?** `brew vulns` already extracts source URLs from formula files, matches installed versions against vulnerable-version ranges, and queries OSV's GIT ecosystem. Inheriting upstream fixes automatically is strictly better than owning that surface ourselves. The backend module at `src-tauri/src/vulns/` is a thin orchestration layer over the subprocess: `client` (invocation + parse), `cache` (persistent on-disk cache + install-set fingerprint), `fingerprint` (SHA-256 over sorted `kind:name:version` lines for the whole-scan skip predicate), `enrich` (optional GHSA enrichment).

**Cache:** `~/Library/Application Support/brew-browser/vulns_cache.json` (1 MiB cap, atomic-write, fail-soft on corrupt + future-schema, 6h TTL per record). Install-set fingerprint persisted alongside entries so opening the app daily on an unchanged install set serves the cached report instantly without re-shelling brew-vulns.

**Optional GHSA enrichment:** when `settings.github_enabled` is also on, the backend fetches `api.github.com/advisories/{GHSA_ID}` for any OSV record carrying a `GHSA-…` ID. Best-effort: a failure leaves the OSV record unchanged and logs without toasting. Parallel cache at `ghsa_cache.json` (2 MiB cap, same atomic-write + read-capped pattern). See `security.md` §17 for the gate-composition audit.

**Casks not supported:** `brew vulns` is formula-only (casks ship vendor binaries with their own update channels; OSV's source-URL approach doesn't map). Cask rows render the same UI shells but the Security card honestly says "Cask coverage isn't supported."

## Known sharp edges

- **Tauri sandbox vs. shell execution** — explicit allowlist in `tauri.conf.json` is required to permit any subprocess
- **Cask installs may prompt for sudo** — `brew install --cask` sometimes invokes macOS installer; we surface stdout verbatim and document the limitation
- **`brew bundle dump` is slow on large libraries** — needs progress feedback
- **`brew search` is slow on cold cache** — show loading state, cache for the session
- **SvelteKit with `adapter-static` requires `ssr=false`** — already configured in `+layout.ts`

## Native rebuild (experimental — branch `experiment/native-swift-liquid-glass`)

A parallel, **fully native** port of the app lives in `native/`. It does **not**
replace the Tauri stack above — the shipped product stays Tauri on `main`. This
section documents the native experiment's stack; see `native/README.md` for the
full source map + build loop, and `decisions.md` (2026-05-30 ADR) for the why.

| Layer | Choice | Version |
|-------|--------|---------|
| Language | Swift | 6.3.x |
| UI | SwiftUI + Liquid Glass (`.glassEffect`, `GlassEffectContainer`, `.buttonStyle(.glass)`, `.backgroundExtensionEffect`) | macOS 26 SDK |
| Charts | Swift Charts (`SectorMark`, `LineMark`) | (SDK) |
| Min OS | macOS 26.0 (Tahoe) | — |
| Build system | Swift Package Manager (`swift build`) — **not** an Xcode project | swift-tools 6.2 |
| App bundling | `native/build-app.sh` wraps the SPM binary into a launchable `.app` (adds `Info.plist`, copies the `Bundle.module` resource bundle) | — |

**Toolchain note:** full Xcode is installed at `/Applications/Xcode.app`, but
`xcode-select` points at Command Line Tools, so `xcodebuild` is unavailable from
the CLI. `swift build` works under CLT and links all Liquid Glass APIs. Build via
`native/build-app.sh [debug|release]`; `Package.swift` opens directly in Xcode for
Previews/Run if needed.

**Concurrency model:** `@Observable @MainActor` view models with `static let shared`
singletons for app state; `actor` types for I/O (`BrewService`, `VulnsService`,
`GitHubService`, `TrendingHistoryService`) — call sites use `await` inside `Task`.

**Data layer (ported verbatim from the Tauri app):**
- `settings.json` — same path + schema as the Rust `Settings`; Swift `AppSettings`
  reads/writes it (atomic write, feature gating ported: paranoid / vuln / github /
  enhanced-trending / AI features).
- `UserDefaults` — `LocalPrefs` for view-only prefs (theme, default landing section,
  confirm-destructive, activity caps).
- Bundled JSON — `categories.json` (838 KB) + `enrichment.json` (2.8 MB) copied from
  `src-tauri/data/` into `native/Sources/BrewBrowser/Resources/`, shipped
  **uncompressed** in the native build (Tauri ships them gzipped).
- `brew` subprocess — `BrewService` / `VulnsService` actors over `Process`, cwd
  pinned to `/`, `--json=v2` where available. `VulnsService` replicates all 5
  `brew vulns` smoke-test gotchas from the Rust impl.

**Not yet ported:** Sparkle-based in-app updates (the Updates tab's auto-check
toggle persists, but install is a stub). Everything else (trending, GitHub OAuth +
Keychain, vulnerability scanning, AI enrichment) is ported.
