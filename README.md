# brew-browser

> A native macOS GUI for Homebrew.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)
[![Built with Tauri 2](https://img.shields.io/badge/Built%20with-Tauri%202-orange)](https://tauri.app)
[![macOS 13+](https://img.shields.io/badge/macOS-13%2B-lightgrey)](https://www.apple.com/macos)
[![Sponsor](https://img.shields.io/badge/♥-Sponsor-EC4899?logo=githubsponsors&logoColor=white)](https://github.com/sponsors/msitarzewski)

A small, fast desktop app for browsing, searching, installing, and snapshotting Homebrew packages. Full source, MIT-licensed, no telemetry, no accounts.

![brew-browser — Dashboard (dark)](docs/screenshots/dashboard-dark.png)

## Why this exists

Homebrew is the standard package manager on macOS. brew-browser gives it a real native GUI: browse what you have installed, search the full catalog, install / uninstall / upgrade with live output, snapshot your setup to a Brewfile and restore it on a new Mac. Trending packages come from Homebrew's published analytics. The whole thing is a thin, respectful frontend over the `brew` CLI itself.

## Features

- **Dashboard** — your Homebrew setup at a glance: installed count, updates available, brew version, formula/cask split, top-categories donut chart, storage usage (Cellar / Caskroom / var/log / cache) with one-click "Reveal in Finder"
- **Library** — every installed formula and cask in one dense, filterable list, with outdated badges, sortable columns, category chip filters, and a slide-over detail panel
- **Discover** — search the full Homebrew catalog (15,974 packages, bundled at build time + user-refreshable) by name or browse via the 19-category tile grid; multi-select chip filter
- **Trending** — top packages from Homebrew's published `formulae.brew.sh` analytics, with 30 / 90 / 365-day windows and sortable columns
- **Snapshots** — save and restore Brewfiles using Homebrew's own `brew bundle` mechanism; "set up a new Mac" in one click
- **Services** — list, start, stop, and restart background services managed by launchd through `brew services`
- **Activity** — every `brew` invocation streams live into a bottom drawer with full stdout/stderr; session history persists across launches (last 50 jobs, capped lines)

A global Cmd+K command palette covers the verbs. Cmd+0 returns to the Dashboard; Cmd+1…6 jumps between sections. Cmd+, opens Settings. Click the 🍺 brand to return home. Window dragging works from any panel header (native macOS overlay title bar + NSVisualEffectView vibrancy).

## What this isn't

- Not a Homebrew replacement — every action shells out to the real `brew` CLI
- Not telemetry-funded — no analytics, no accounts, no phone-home
- Not freemium — there is no paid tier, because there is no tier

## Install (end users)

Download the latest signed + notarized `.dmg` from the [releases page](https://github.com/msitarzewski/brew-browser/releases/latest), open it, and drag **brew-browser** to your Applications folder. No Gatekeeper warning — the build is signed with a Developer ID Application certificate and notarized by Apple.

Apple Silicon only for now. macOS 13 (Ventura) or newer.

A `brew tap` for one-line install is on the roadmap.

## Build from source

Prereqs:

- [Rust](https://rustup.rs/) (stable, edition 2021+)
- [Node.js 22+](https://nodejs.org/) and npm
- [Homebrew](https://brew.sh/) itself
- Xcode Command Line Tools: `xcode-select --install`

Then:

```sh
git clone https://github.com/msitarzewski/brew-browser
cd brew-browser
npm install
npm run tauri dev      # development with HMR
npm run tauri build    # produces a .dmg in src-tauri/target/release/bundle/
```

## Architecture

A Tauri 2 shell hosts a SvelteKit + Svelte 5 frontend in the system WebView. A Rust backend exposes ~55 typed Tauri commands that shell out to `brew` via `tokio::process` and stream stdout/stderr back over typed IPC channels. The full Homebrew catalog is bundled at build time (~6 MiB gzipped) and refreshable on demand. Trending data comes straight from `formulae.brew.sh`'s public analytics JSON, cached in memory for an hour. Optional GitHub integration uses OAuth Device Flow with the token stored only in the macOS Keychain. No shell plugin, no arbitrary command execution — every `brew` invocation is built in Rust from a small set of enumerated inputs. See [PLAN.md](./PLAN.md) for the full design and [memory-bank/backendApi.md](./memory-bank/backendApi.md) for the complete IPC surface.

## Open-source posture

**MIT licensed.** **No CLA.** **No EULA.** **No telemetry.** **No account.** **No dark patterns.**

brew-browser makes outbound network calls in exactly seven documented circumstances. Every one is initiated by something you did and gated by Settings → Network:

- **`https://formulae.brew.sh/api/analytics`** — fetched when you open the Trending tab. Cached in process memory (TTL configurable in Settings → Network; default 60 minutes). Uses Homebrew's own published install-analytics JSON; no API key, no account.
- **`https://formulae.brew.sh/api/{formula,cask}.json`** — the full Homebrew catalog. Bundled at build time so the app works offline. A user-initiated **Refresh** button on the Dashboard (or the Discover stale-catalog banner) writes a fresh copy to `~/Library/Application Support/brew-browser/catalog/`. Auto-refresh is **off** by default; Settings → Network offers weekly / daily opt-in.
- **Cask homepage probes** — when the Discover or Trending tab renders an uninstalled cask that has a `homepage` field, the Rust backend probes that homepage for an icon (in order: `/apple-touch-icon.png`, `<meta og:image>` parsed from the homepage HTML, `/favicon.ico`). One probe per cask per week max — the result, including misses, is cached for 7 days. These probes are sandboxed: link-local, loopback, RFC1918, and cloud-metadata IPs are rejected before the request, and the same check runs again on every redirect hop to prevent SSRF. Settings → Network can scope this to **installed only** or disable it entirely.
- **`https://api.github.com/repos/{owner}/{repo}`** — optional, **off by default**. When **Settings → GitHub → "Show GitHub stats on package pages"** is on, the PackageDetail panel fetches public repo metadata (stars, forks, last release date, archived state) for packages whose homepage parses as a GitHub URL. The URL parser strictly allowlists `github.com` (rejects `gist.`, `raw.githubusercontent.`, suffix-attack domains, path traversal). Results cached to `~/Library/Application Support/brew-browser/github-cache/` for 24 hours. Anonymous rate limit is 60 reqs/hr per IP; sign-in lifts it to 5,000/hr.
- **`https://github.com/login/{device,oauth}/*`** — optional, only when you click **Sign in with GitHub** in Settings. Uses OAuth Device Flow (RFC 8628): you see a user code, open `github.com/login/device` in your browser, paste it, done. No embedded webview, no client secret, no callback URL. Scopes requested: `read:user` + `public_repo` (the minimum for username + star/issue/watch). Access token stored exclusively in **macOS Keychain** under `dev.openbrew.browser/github_access_token`. **The token is never returned to the frontend, never written to disk, and never logged** — verified by unit tests.
- **`brew` itself** — every install, uninstall, upgrade, search, and snapshot shells out to the real `brew` CLI. Whatever network calls `brew` makes (GitHub, OCI registries, bottle mirrors) happen exactly as they would if you ran the command yourself in a terminal. The full stdout/stderr stream is visible in the Activity drawer.
- **Your default browser** — when you click the homepage button on a package, the URL is opened in your default browser via macOS `open(1)`. The app rejects any non-`http(s)` scheme before opening.

Every outbound call respects the Network settings — flip on **Offline Mode** in Settings to block all outbound traffic in one click. Settings persist to `~/Library/Application Support/brew-browser/settings.json`; a corrupt or missing file fails closed (Offline Mode effectively on) until you hit Reset to defaults.

No analytics. No crash reporting. No third-party fonts or pixels. No `fetch()` from the frontend — every backend call goes through typed Tauri IPC.

The full network posture is verified line-by-line in [`memory-bank/security.md`](./memory-bank/security.md) §5. Re-audits are welcome; the source is right there.

## Security

A full security audit lives at [`memory-bank/security.md`](./memory-bank/security.md). Current verdict: **READY-FOR-SCRUTINY** (0 critical / 0 high / 0 medium / 0 low / 0 nit open). All 16 findings from the initial audit are verified-fixed with passing tests. Independent tool battery passes: `cargo audit` 0 vulns, `cargo deny check` advisories+bans+licenses+sources ok, `npm audit --omit=dev` 0 vulns, `semgrep` with security-audit + OWASP-top-10 + Rust + TypeScript rulesets 0 findings, `cargo clippy -D warnings` clean. Zero `unsafe` Rust, zero `@html`/`innerHTML`/`eval` in the frontend, no `tauri-plugin-shell` (every brew invocation is built from typed Rust enums). SSRF defense includes a redirect-policy re-check on every hop.

Dependency posture:

- **Rust:** `cargo audit` reports 0 vulnerabilities across 540 crates. The 17 unmaintained warnings and 1 unsoundness all sit in GTK/glib transitive deps that compile out on macOS.
- **npm (production):** `npm audit --omit=dev` reports 0 vulnerabilities across 25 production packages.
- **Zero `unsafe` Rust** in the entire backend.

Defense-in-depth choices:

- No `tauri-plugin-shell` — the frontend cannot construct arbitrary shell commands. Every `brew` invocation is built in Rust from typed enums.
- Scheme allowlist on the homepage opener — only `http(s)` URLs reach `tauri-plugin-opener`.
- SSRF filter on the cask icon cascade — private, link-local, loopback, and cloud-metadata IPs are rejected pre-flight and on every redirect.
- Path sandboxing on Brewfile import/export — IPC paths are validated against a forbidden-prefix list and a 1 MiB size cap.
- `rustls-tls` + `webpki-roots` for all outbound HTTPS — no system trust store dependency.
- Capability allowlist is minimal: `core:default`, `opener:default`, `core:event:default`, `dialog:allow-open`, `dialog:allow-save`. No `fs:*`, no `http:*`, no `shell:*`.

Issues and PRs on security topics are welcome. See [SECURITY.md](./SECURITY.md) for the responsible disclosure process.

## Contributing

Contributions welcome. See [CONTRIBUTING.md](./CONTRIBUTING.md) for the dev loop, project map, and the short list of things worth opening an issue about first. No CLA. Your contributions stay yours, licensed under MIT to match the project.

## Status

**v0.2.1** shipped (signed + notarized). All seven core panes live: Dashboard, Library, Discover (with bundled catalog + 15,725 AI-curated friendly names and summaries), Trending, Snapshots, Services, and Activity. Optional GitHub integration via OAuth Device Flow is intent-discovered — sign-in only prompts when you actually try to star / watch / file an issue, never as static UI clutter. The Keychain is touched lazily so a fresh install never triggers a macOS auth prompt unless you actually use a GitHub feature. Settings ships with Offline Mode and a corrupt-recovery default. Native macOS title bar with traffic-light alignment, collapsible sidebar with persistent type-ahead search, (i) info popovers in place of AI badges for every enriched field. Expect rough edges in some app-icon edge cases (pkg-installer casks without `.app` bundles) and first-run niceties.

**v0.2.1** (hotfix on top of v0.2.0):
- Lazy Keychain probe — a fresh install no longer fires the macOS "wants to use your confidential information stored in dev.openbrew.browser" prompt on launch. `github.loadStatus()` only runs when you actually click Star / Watch / File-issue, or open Settings → GitHub.
- Fixed the post-sign-in success toast (was showing "@github user" placeholder before the real username loaded).
- Fixed the duplicate "Signed in to GitHub" toast stack (the toast effect was re-firing on every status hydration; the `untrack` wrapping pins it to one toast per real state transition).
- Updated the build credit to the accurate runtime: Claude Code in the terminal, running Opus 4.7 [1m].
- New `.gitleaks.toml` allowlist for the documented public OAuth client_id (RFC 8628 §3.1 — Device Flow IDs aren't credentials).

## License

[MIT](./LICENSE). Do whatever you want with this.

## Acknowledgments

- [Homebrew](https://brew.sh) — does all the actual work. This app is a respectful UI on top.
- [Tauri](https://tauri.app) — native shell without the Electron tax.
- [Svelte](https://svelte.dev) — the runes-based reactivity that made the frontend small.

## Built with

Built with **[Agency Agents](https://github.com/msitarzewski/agency-agents)**, by the creator of Agency Agents — the multi-agent toolkit (Backend Architect, Frontend Developer, Security Engineer, Code Reviewer, Technical Writer, and friends) that orchestrated brew-browser's design and implementation. Powered by Claude Code in the terminal, running Opus 4.7 [1m].

## Support the project

If brew-browser saves you time, consider [sponsoring on GitHub](https://github.com/sponsors/msitarzewski) ♥. No paid tier — sponsorship is purely a thank-you, and it helps fund the Anthropic API spend that keeps the AI-curated catalog metadata fresh.
