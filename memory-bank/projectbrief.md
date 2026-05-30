# Project Brief

## Mission

Ship a native macOS GUI for Homebrew: browse installed formulae and casks, search the full catalog, install / uninstall / upgrade with live output, snapshot the setup to a Brewfile and restore it on a new Mac. MIT-licensed, full source, no telemetry, no accounts, no dark patterns.

## Why now

Homebrew is the standard package manager on macOS, and the CLI experience is excellent — but a real native GUI lowers the bar for browsing what's installed, finding what to install next, and moving a setup between machines. brew-browser fills that slot with a small, fast Tauri 2 app that shells out to `brew` for every action and stays out of the way otherwise.

## Audience

- Mac users who want a GUI on top of Homebrew
- Developers looking for a reference implementation of a Tauri 2 + Svelte 5 + Rust app that shells out to system tools
- Anyone who wants to inspect, build, or fork the source

## Success criteria (demo level)

- All 6 MVP features work end-to-end on Beast (M5 Max, macOS Tahoe 26.5)
- Openness is stated clearly: MIT, full source, no EULA, no CLA, no telemetry
- README explains "why this exists" in one paragraph a new reader can immediately get
- `cargo tauri build` produces a working `.dmg` anyone can install

## Non-goals

- Not a Homebrew replacement
- Not a long-running product — focused MVP that can be polished later
- Not optimized for million-package scale (designed for ~50-500 packages per user)
- Not multi-platform for MVP (macOS-first; brew runs on Linux but Linux is out of scope)

## Constraints

- **Fresh implementation.** Not derived from or inspired by any specific other project. Convergent functionality (anything else that wraps `brew`) is fine; copying UI or code is not.
- **License: MIT.** Locked. Most permissive and most recognizable OSI license; no CLA needed; contributor retains copyright on own contributions for future monetization optionality.
- **No telemetry. No accounts. No surprise network calls.** Outbound traffic is limited to eleven documented paths, every one of them gated by Settings → Network (Offline Mode kills them all in one click): (a) `formulae.brew.sh/api/analytics/install` + `install-on-request` for Trending, (b) `formulae.brew.sh/api/{formula,cask}.json` for catalog refresh, (c) per-cask homepage probes for icon discovery on uninstalled casks in Discover/Trending (apple-touch-icon → og:image → favicon cascade, cached 7 days, SSRF-filtered against private/link-local/cloud-metadata IPs), (d) `api.github.com/repos/...` for read-only repo stats when the user opts in to GitHub stats (Phase 12c+), (e) `github.com/login/...` for OAuth Device Flow when the user signs in (Phase 12e), (f) `api.github.com/{user,repos}/...` for star/watch/file-issue actions when the user clicks them (Phase 12f), (g) whatever `brew` itself does during install/upgrade/search, (h) the user's default browser when the homepage button is clicked, (i) `brew-browser.zerologic.com/updater.json` + `github.com/.../releases/download/.../app.tar.gz` for the in-app updater (Phase 15, off-by-default auto-check), (j) `brew-browser.zerologic.com/trending-history/*` for Enhanced Trending History — distinct trust boundary (project infra), opt-in only, off by default (v0.4.0+), (k) `api.osv.dev` + source forges (`github.com`, `gitlab.com`, `codeberg.org`) via the `brew vulns` subprocess + `api.github.com/advisories/*` for GHSA enrichment when both vulnerability scanning AND GitHub auth are on — Vulnerability Scanning, opt-in only, off by default (v0.5.0+). See `README.md`'s Open-source posture section for the user-facing enumeration and `memory-bank/security.md` for the line-by-line verification.
- **No reinventing brew.** brew-browser is a respectful UI over the `brew` CLI. It does not parse formula files, does not manage taps, does not compute dependencies — `brew` does all of that.

## Distribution

- **Manual:** signed + notarized `.dmg` on GitHub Releases (Apple Silicon, macOS Ventura+).
- **Homebrew tap (v0.5.0+):** `brew tap msitarzewski/brew-browser && brew install --cask brew-browser`. The cask lives in the separate repo `github.com/msitarzewski/homebrew-brew-browser` (`Casks/brew-browser.rb`) and points at the same notarized `.dmg`. Each release must bump the cask's `version` + `sha256` (manual today; auto-bump in the release pipeline is a TODO). Submitting to the official `Homebrew/homebrew-cask` is deferred until brew-browser crosses the notability bar (75★ / 30 forks / 30 watchers, measured on this repo only). See `decisions.md` 2026-05-30.
