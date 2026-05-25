# 2026-05-24 — Phase 12 Wave 1+2: bundled catalog + Settings + paranoid mode + GitHub anon + Device Flow

**Phase:** 12 (sub-phases 12a, 12b, 12c, 12d, 12e — five sub-phases shipped together)
**Status:** ✅ Shipped
**Commit:** `99a1f2c` (52 files, +9,446 / -183 — largest single commit of the project)
**Date:** 2026-05-24 04:17

## Scope

Big batched commit landing five sub-phases in one go, executed by parallel agents (Backend Architect + Frontend Developer) coordinated by Lead. Establishes Settings infrastructure, the network kill switch (Paranoid Mode), the bundled catalog with manual refresh, and the optional GitHub integration (both anonymous repo stats AND OAuth Device Flow sign-in).

## What landed

### Phase 12a — Bundled catalog + manual refresh
- Full Homebrew package index bundled at build time as `src-tauri/data/catalog/{formula,cask}.json.gz` (~6.1 MiB total gzipped)
- New IPC: `catalog_summary`, `catalog_refresh`, `catalog_lookup_formula`, `catalog_lookup_cask`, `catalog_search`, `catalog_categories`
- Manifest.json with version + generated-at timestamp + counts
- Auto-refresh **off by default**; Settings → Network offers daily/weekly opt-in (added in 12d)
- Manual Refresh button on Dashboard; stale-catalog banner on Discover (added in cleanup pass `e1d6a87`)
- Defense-in-depth size caps on the bundled artifact

### Phase 12b — Settings shell + brew analytics
- New modal: `Settings.svelte` with sidebar + 6 sections (Appearance, Network, GitHub, Brew, Activity, About)
- Each section is its own component: `SettingsSection{Appearance,Network,GitHub,Brew,Activity,About}.svelte`
- Settings persistence to `settings.json` in app data dir
- `SettingsLoadState` three-state: `FirstLaunch`, `Loaded(Settings)`, `Corrupt { ... }` — fails closed (paranoid on) on corrupt
- **Brew analytics** read/toggle (`brew analytics state` + `on`/`off`) wired through

### Phase 12c — GitHub anonymous repo stats
- `github_repo_stats(homepage)` IPC fetches public `api.github.com/repos/{owner}/{repo}` for packages whose homepage is a GitHub URL
- Strict URL parser: allowlists `github.com`, rejects `gist.`, `raw.githubusercontent.`, suffix-attack domains, path traversal
- Disk cache at `~/Library/Application Support/brew-browser/github-cache/`, 24h TTL
- Rate-limit handling: `GithubRateLimited { reset_at }` typed error, no retry
- PackageDetail surfaces stars / forks / last release / archived state when enabled
- Settings → GitHub → "Show GitHub stats on package pages" toggle gates the fetch

### Phase 12d — Paranoid mode + network settings + settings persistence
- **`require_network(feature)` helper** — central chokepoint every outbound IPC must route through
- **`paranoid_mode: bool`** in settings — when on, every outbound network path returns `BrewError::ParanoidModeBlocked { feature }`
- Settings → Network section with the master toggle + per-feature subsettings (catalog auto-refresh, trending TTL, cask icon mode)
- Settings persistence writes are atomic (temp file + rename)
- Corrupt settings file fails closed (paranoid effectively on) with a one-click "Reset to defaults"

### Phase 12c + 12e combined — Device Flow OAuth + Keychain
- `github_start_signin` + `github_poll_signin` IPC implementing RFC 8628 Device Flow
- `DeviceFlowModal.svelte` shows the user code; user opens `github.com/login/device`, pastes, approves
- Token stored exclusively in macOS Keychain (`keyring` crate) under service `dev.openbrew.browser`, account `github_access_token` + `_scopes`
- **Token never returned to the frontend, never written to disk, never logged** — verified by unit tests (`github::auth::tests::status_dto_contains_no_token_shaped_string`, keychain-failure-no-disk-fallback assertion, redacted-Debug assertion)
- `Token` newtype with `#[derive(Debug)]` redacted output
- `#![deny(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)]` across `src/github/`
- OAuth `client_id` const placeholder at this point: `Iv1.PLACEHOLDER_REPLACE_BEFORE_RELEASE` (real value swapped in `f556441`)
- Scopes: `read:user` + `public_repo` minimum, asserted by a test that introspects the request body
- Polling honors server `interval` (typically 5s) and doubles on `slow_down` per RFC 8628 §3.5

## Tests / verification

- `cargo test`: ~330 passing (+ ~124 new across the five sub-phases)
- `cargo clippy --all-targets -- -D warnings`: clean
- `npm run check`: 0 errors
- Test coverage notably strong on the GitHub auth path: 8 paranoid-mode tests (one per command), 3 gate-order tests, hardcoded-service-id assertion (parses tauri.conf.json and asserts match), scope-minimum assertion, polling-interval and slow-down doubling tests, `KeychainSlot` trait + in-memory mock

## Files

52 files changed. Highlights:
- New backend modules: `commands/{catalog,settings,github}/*.rs`, `commands/github/auth.rs`
- New frontend stores: `catalog.svelte.ts`, `settings.svelte.ts`, `github.svelte.ts`
- New components: `Settings.svelte`, 6× `SettingsSection*.svelte`, `DeviceFlowModal.svelte`
- New IPC wrappers in `api.ts`
- New types in `types.ts`
- CSP additions: `connect-src` += `https://api.github.com https://github.com`

## Notes / decisions

- **Five sub-phases batched** because the dependency graph made phase-by-phase commits awkward: 12d's settings persistence is the substrate for 12b's persisted toggles, 12c needs the settings to gate fetches, 12e needs the IPC plumbing from 12c. Parallel agents worked on non-overlapping files, then Lead integrated.
- **`require_network()` is non-negotiable** — every subsequent outbound feature must route through it. This is the single chokepoint that gives Paranoid Mode its meaning.
- **Token-handling rules** documented in security.md §13.5 are non-negotiable: token doesn't cross the IPC boundary, doesn't write to disk, doesn't log. All verified by mechanical tests, not just review.
- **Auto-refresh on the catalog is OFF by default** — the bundled artifact is fresh enough at ship time; users who care about staleness opt in. Honors the "no surprise network" posture.
- Phase 12f (GitHub authed actions) intentionally deferred — next commit (`8b89c40`).
- Phase 13 (catalog enrichment infrastructure) queued for parallel work with 12f.
- Phase 14 (bundled cask icons) **dropped** during planning — trademark/redistribution risk; see decisions.md.
- Network paths added to README §"Open by default": catalog refresh, GitHub stats (api.github.com), GitHub OAuth (github.com/login/device).
