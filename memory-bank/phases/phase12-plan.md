# Phase 12 Plan — Catalog + Settings + GitHub

**Created:** 2026-05-24 (post-Phase-11 session)
**Driver:** Two user prompts: (1) "what can we do with formulae.brew.sh API?" → catalog unlocks; (2) "what should live in Settings? GitHub auth?"
**Design decisions:** See `decisions.md` (2026-05-24-night entries) — bundled-catalog-with-manual-refresh + GitHub Device Flow.

## Phase tree

| Sub-phase | Title | Depends on | Effort | Independence |
|-----------|-------|------------|--------|--------------|
| 12a | Bundled catalog + manual refresh | nothing | half day | INDEPENDENT |
| 12b | Settings shell | nothing | half day | INDEPENDENT (parallel with 12a) |
| 12c | GitHub anonymous tier | 12a's network patterns; 12b for the toggle | half day | depends on 12a+12b |
| 12d | Settings — network controls + paranoid mode | 12a, 12b | ~2 hours | depends on 12a+12b |
| 12e | GitHub Device Flow OAuth + Keychain token | 12b for the sign-in UI | half day | depends on 12b |
| 12f | GitHub authed actions (star/issue/watch) | 12c, 12e | half day | depends on 12c+12e |

## Parallel agent waves

12a and 12b are fully independent and can run in parallel as **Wave 1**. 12c, 12d, 12e gate on Wave 1's outputs and run as **Wave 2** (also parallel). 12f waits for Wave 2 and runs solo as **Wave 3**.

```
Wave 1 (parallel):   [12a]  [12b]
                       │      │
                       ├──────┤
                       ▼      ▼
Wave 2 (parallel):   [12c]  [12d]  [12e]
                       │             │
                       └──────┬──────┘
                              ▼
Wave 3:                    [12f]
```

---

## 12a — Bundled catalog + manual refresh

**Goal:** Pull the full Homebrew catalog (`formula.json` + `cask.json`) into the app, bundled at build time + refreshable on user demand.

### Backend

- **New build script** `tools/catalog/fetch.py` (or embed in existing `tools/categorize/`):
  - Fetches `https://formulae.brew.sh/api/formula.json` and `https://formulae.brew.sh/api/cask.json`
  - Gzip-compresses
  - Writes to `src-tauri/data/catalog/formula.json.gz` and `cask.json.gz`
  - Records `as_of` ISO 8601 timestamp in `src-tauri/data/catalog/manifest.json`
  - Run during release prep (manual `python fetch.py`); document in BUILD.md

- **New module `src-tauri/src/catalog/mod.rs`:**
  - `Catalog` struct: `formulae: HashMap<String, Formula>`, `casks: HashMap<String, Cask>`, `as_of: DateTime<Utc>`, `source: "bundled" | "user-refreshed"`
  - `Formula` / `Cask` Rust types matching the API shape (subset of fields actually used: `name`, `desc`, `homepage`, `license`, `deprecated`, `deprecation_date`, `deprecation_reason`, `disabled`, `disable_date`, `disable_reason`, `dependencies`, `recommended_dependencies`, `optional_dependencies`, `conflicts_with`, `versions.stable`, `tap`, `aliases`)
  - `Catalog::load_bundled()` decompresses `include_bytes!("../data/catalog/formula.json.gz")` + cask, parses, returns
  - `Catalog::load_user_data(path)` reads from `~/Library/Application Support/brew-browser/catalog/`
  - `Catalog::resolve_active(state)` — user-data if exists, bundled fallback
  - `Catalog::refresh_to_user_data(state)` — fetches both endpoints via `reqwest`, writes gzipped, updates timestamp

- **New commands `commands/catalog.rs`:**
  - `catalog_summary() -> CatalogSummary { as_of, source, formula_count, cask_count, days_old }`
  - `catalog_refresh() -> CatalogSummary` (long-running; can be async with single in-flight enforced)
  - `catalog_lookup_formula(name) -> Option<Formula>`
  - `catalog_lookup_cask(name) -> Option<Cask>`
  - `catalog_formulae_summary() -> Vec<{name, desc, deprecated, disabled}>` (light shape for search)
  - `catalog_casks_summary() -> Vec<{name, desc, deprecated, disabled}>`

- **State:** `Arc<RwLock<Arc<Catalog>>>` on AppState; readers clone the Arc, writer is only the refresh command.

- **Tests:**
  - Bundled catalog parses
  - User-data path resolution prefers user-data when present
  - Refresh writes timestamp correctly
  - Lookup hit + miss
  - Deprecation/disabled flags surface

### Frontend

- **Types** in `types.ts`: `Formula`, `Cask`, `CatalogSummary`, mirror the Rust shapes (camelCase).
- **API wrappers** in `api.ts`: `catalogSummary`, `catalogRefresh`, `catalogLookupFormula`, `catalogLookupCask`, etc.
- **Store** `stores/catalog.svelte.ts`: `summary`, `refreshing`, `lastRefreshError`, `refresh()`, `lookupFormula(name)`, `lookupCask(name)`.
- **Dashboard wiring:** new line near the brew version stat: `Catalog: 3 days old (bundled) · Refresh →` with click handler → `catalog.refresh()`.
- **Discover banner:** when `summary.daysOld > 14`, show a dismissable banner at top of Discover: "Catalog is 21 days old. Refresh from brew.sh →".

### README update
- Add catalog refresh to the "Open by default" network paths list as #5: "**`https://formulae.brew.sh/api/{formula,cask}.json`** — user-initiated only. Requires a click on Refresh in the Dashboard or Discover banner."

### Acceptance criteria
- App boots and lists/searches work using bundled catalog
- Refresh button writes user-data copy and the UI reflects it
- Last-refreshed timestamp visible in Dashboard
- Stale banner appears at >14 days

---

## 12b — Settings shell

**Goal:** Build the Settings container that all subsequent features plug into. v1 has the easy switches; later sub-phases add to it.

### Frontend (no backend changes)

- **New modal/page** `components/Settings.svelte`:
  - Left-nav style with sections: Appearance · Network · GitHub · Brew · Activity · About
  - Each section is a `<div>` rendered when its nav item is selected
  - Trigger: existing theme buttons in sidebar footer get a small "Settings" gear icon next to them, OR Cmd+, opens it as a modal
  - Implement as a modal (overlay), not a full sidebar route — settings are contextual

- **Sections (v1 scope):**

  **Appearance**
  - Theme: radio group Light / Dark / System (binds to `ui.setTheme`)
  - Default landing: dropdown Dashboard / Library (writes to localStorage key `brew-browser:default-section`)
  - Vibrancy material: dropdown HudWindow / Sidebar / FullScreenUI / Off (writes to localStorage; restart-required note)

  **Network** (placeholder section, populated in 12d)
  - "Network features will appear here once Phase 12d lands"
  - For now: a static note + the "outbound paths" disclosure list from README

  **GitHub** (placeholder, populated in 12c+12e)
  - "Off — no GitHub data will be fetched"

  **Brew**
  - `HOMEBREW_NO_ANALYTICS` toggle (reads/writes via new `brew_env_get`/`brew_env_set` commands — small backend addition acceptable here, or via `brew analytics off` shell-out which is the right primitive)
  - Confirm before destructive: toggle (currently always on; add the toggle now even if it defaults true)

  **Activity**
  - Retention: number input "Keep last N completed jobs" (default 50; binds to a new `activity` store setting persisted to localStorage)
  - Lines per job: number input (default 500)

  **About**
  - App version (from package.json / Cargo.toml — pull via existing `brewDoctor` or new `app_info` command)
  - Brew version (from env store)
  - Repo link + license + memory-bank link
  - "Zero telemetry, zero accounts" affirmation paragraph

- **Wire `ui.svelte.ts`:** add `settingsOpen: boolean` + `openSettings()` / `closeSettings()`
- **Keyboard:** Cmd+, opens Settings (macOS convention)
- **Default landing wiring:** on app mount, read localStorage key and `ui.setSection(...)` accordingly (only if not already set this session)

### Backend
- `brew_set_analytics(enabled: bool)` — shells `brew analytics on|off`, returns void
- `brew_get_analytics() -> bool` — shells `brew analytics state`, parses output

### Tests
- Settings opens on Cmd+,
- Each section renders without errors
- Brew analytics toggle round-trips (skip-able if brew not installed)

### Acceptance criteria
- Cmd+, opens Settings modal
- All 6 sections render
- Theme and Default landing settings persist across restart
- Brew analytics toggle reflects real brew state

---

## 12c — GitHub anonymous tier

**Goal:** Show GitHub repo stats in PackageDetail for any package whose homepage is a GitHub URL. No sign-in required.

**Depends on:** 12b (Settings GitHub section needs a switch), 12a's network patterns

### Backend

- **New module `src-tauri/src/github/mod.rs`:**
  - `parse_github_url(homepage: &str) -> Option<(owner: String, repo: String)>`
  - `RepoStats { owner, repo, stars, forks, open_issues, last_release_tag, last_release_date, archived, archived_at, license_spdx, default_branch, primary_language }`
  - `fetch_repo_stats(client, owner, repo, auth: Option<&str>) -> Result<RepoStats>`
  - Two GitHub API hits per package: `GET /repos/{owner}/{repo}` and `GET /repos/{owner}/{repo}/releases/latest` (or fall back to `/tags` if no releases)

- **Disk cache** at `~/Library/Application Support/brew-browser/github-cache/<owner>__<repo>.json` with TTL 24h. Cache hits: zero network. Cache stale: refetch.

- **Rate limit handling:** parse `X-RateLimit-Remaining` header; when 0, set a session-level cooldown and surface a typed error so UI can show "rate limited — sign in to enable" message.

- **Commands `commands/github.rs`:**
  - `github_repo_stats(homepage: String) -> Option<RepoStats>` — returns None if not a github URL; Err on rate-limit / network errors

- **Capability addition:** Add new disclosed outbound path. CSP `connect-src` extended to include `https://api.github.com` IF Settings has GitHub enabled (this is a runtime check, can't gate CSP per-toggle — so CSP allows it always; the gate is in the Rust command which refuses to call if Settings says "GitHub off"). Document this in security.md.

### Frontend

- **Types:** `RepoStats` in types.ts (camelCase)
- **API wrapper:** `githubRepoStats(homepage)` in api.ts
- **Store** `stores/github.svelte.ts`:
  - `enabled: boolean` (from Settings)
  - `cache: Map<homepage, RepoStats | "miss" | "rate-limited">`
  - `getRepoStats(homepage)` async helper
- **PackageDetail integration:** below the homepage button, if homepage parses as GitHub:
  - Loading: spinner inline
  - Loaded: `⭐ 12.3k · 🍴 487 · last release 3 weeks ago` (formatted)
  - Archived: warning pill "⚠ archived 2 years ago — likely unmaintained"
  - License mismatch with brew's reported license: small "?" hover
- **Settings → GitHub section update:**
  - Toggle: "Show GitHub stats on package pages" (off by default; respects user posture)
  - When on: text "Uses the public GitHub API anonymously. Limit is 60 requests/hour per IP."
  - When off: corresponding banner in PackageDetail GitHub section: "Enable in Settings"

### Tests
- `parse_github_url` accepts the common URL shapes (github.com/owner/repo, with/without trailing slash, with /tree/main, etc.)
- Cache hit returns without HTTP call (test with mocked client)
- Rate-limit response surfaces typed error
- Disabled-in-settings short-circuits

### Acceptance criteria
- Toggling Settings → GitHub on, then opening a github-hosted package, shows stars+forks within 1s
- Cache hits are instant on second open
- Rate-limit triggers a clear error state with "Sign in to lift the limit" CTA (which 12e will fulfill)

---

## 12d — Settings: network controls + paranoid mode

**Goal:** Populate the Network section of Settings with real controls now that catalog (12a) exists.

**Depends on:** 12a (catalog), 12b (Settings shell)

### Frontend (mostly)

- **Network section content:**
  - Catalog refresh policy: radio Off (manual only) / Weekly / Daily (default: Off)
  - Catalog stale-banner threshold: number input "Show banner when catalog is N days old" (default 14)
  - Cask icon fetching: radio Off / Installed only / All (default: All — current behavior)
  - Trending TTL: number input minutes (default 60; matches backend cache)
  - Master switch: "Paranoid mode — block all outbound network calls" (writes a setting that gates every IPC; backend reads the gate before any external fetch)
  - Disclosure list (read-only): all 5 outbound paths, with status next to each (allowed/blocked)

- **Persist** to localStorage under `brew-browser:settings:network:v1`

### Backend

- **Paranoid-mode gate:** new helper `state.paranoid_mode_enabled() -> bool` reads from a small persisted file in app-data (since the gate must survive restart). Every command that does outbound I/O consults this first:
  - `trending_fetch`
  - `cask_icon_from_homepage`
  - `catalog_refresh`
  - `github_repo_stats`
  - Returns a typed `BrewError::ParanoidModeBlocked { feature }` error so UI can show "Disable Paranoid Mode in Settings to use this"

- **Settings persistence command:**
  - `settings_get() -> Settings` — reads from `~/Library/Application Support/brew-browser/settings.json`
  - `settings_set(settings: Settings) -> ()` — writes

### Tests
- Paranoid mode blocks trending_fetch / catalog_refresh / cask_icon_from_homepage / github_repo_stats with the right error code
- Settings round-trip persists across process restart
- Network section reflects backend state

### Acceptance criteria
- Toggling Paranoid Mode immediately blocks the next network call from all affected features with a clear error
- Catalog auto-refresh schedule is honoured (test by setting "Daily" + verifying a refresh fires on next launch)
- Settings survive restart

---

## 12e — GitHub Device Flow OAuth + Keychain

**Goal:** Sign in to GitHub from Settings without an embedded WebView or client secret. Token stored in macOS Keychain.

**Depends on:** 12b (Settings has the GitHub section)

### Backend

- **New dependency:** `tauri-plugin-keyring` or `keyring = "2"` (system Keychain wrapper)
- **GitHub Device Flow (RFC 8628):**
  - Register an OAuth App on github.com beforehand (one-time): client_id only, no client_secret, "Device Flow enabled" checkbox on
  - Commands:
    - `github_device_code_start() -> { device_code, user_code, verification_uri, expires_in, interval }`
      - POST to `https://github.com/login/device/code` with `client_id` + `scope` (e.g. `read:user public_repo` for basic + star)
    - `github_device_code_poll(device_code) -> { access_token } | { pending } | { error }`
      - POST to `https://github.com/login/oauth/access_token` with `device_code` and `client_id`, grant_type `urn:ietf:params:oauth:grant-type:device_code`
      - Returns 200 with token when user has approved; 428 / `authorization_pending` while waiting
    - `github_signout()` — deletes keychain entry
    - `github_status() -> { signed_in: bool, username: Option<String>, scopes: Vec<String> }`
- **Token storage:** `keyring::Entry::new("dev.openbrew.browser", "github_access_token")` — service + account. Read on every command that needs auth.
- **Update `github_repo_stats`:** if keychain has a token, send `Authorization: Bearer <token>` header — rate limit jumps from 60 to 5000/hr.

### Frontend

- **Settings → GitHub section update:**
  - Signed-out state: "Sign in with GitHub" button
  - On click: calls `github_device_code_start`, shows a modal:
    - "Open github.com/login/device in your browser"
    - Big code: `ABCD-EFGH` (copyable)
    - "Open in browser" button (uses tauri-plugin-opener)
    - "Waiting for authorization…" with spinner
    - Polls `github_device_code_poll` every `interval` seconds
    - On success: closes modal, shows "Signed in as @username", "Sign out" button
  - Signed-in state: username, scopes granted, "Sign out" button

- **Capability:** `core:window:default` already covers opener; no new caps needed beyond CSP allowing `github.com` for the auth endpoints.

### Tests
- Keychain round-trip: store + retrieve + delete
- Device code start returns expected shape (skip-able if no client_id configured)
- Auth header is added when token present

### Acceptance criteria
- Sign-in flow works end-to-end in dev build
- Token persists across restart (keychain)
- `github_repo_stats` calls show in network log with Auth header after sign-in
- Rate limit on response headers shows 5000-bucket

### Configuration
- The OAuth App needs to be created by the project maintainer. `client_id` is committed in source (it's not a secret in Device Flow). Document creation in BUILD.md.

---

## 12f — GitHub authed actions

**Goal:** Use the signed-in token to take actions on GitHub: star, file issue, watch.

**Depends on:** 12c (basic GitHub fetches), 12e (token)

### Backend

- **New commands `commands/github.rs` additions:**
  - `github_star(owner: String, repo: String) -> ()` — PUT `/user/starred/{owner}/{repo}`
  - `github_unstar(owner: String, repo: String) -> ()` — DELETE same
  - `github_is_starred(owner: String, repo: String) -> bool` — GET same; 204 = yes, 404 = no
  - `github_watch(owner: String, repo: String, subscribed: bool, ignored: bool) -> ()` — PUT `/repos/{owner}/{repo}/subscription`
  - `github_unwatch(owner: String, repo: String) -> ()` — DELETE same
  - `github_create_issue(owner: String, repo: String, title: String, body: String, labels: Vec<String>) -> { html_url }` — POST `/repos/{owner}/{repo}/issues`
  - All require an authed token; return `BrewError::AuthRequired` if not signed in
  - All require `public_repo` scope (we requested it in 12e); if not granted, return `BrewError::ScopeRequired { scope: "public_repo" }`

### Frontend

- **PackageDetail GitHub stats block (extends 12c):**
  - Star button: shows current starred state (from `github_is_starred`); click to toggle
  - "File issue" button: opens a modal with prefilled title (e.g. "[brew-browser] {pkg.name} ...") and body (system info: brew version, OS version, app version)
  - "Watch" button (less prominent): toggle, shows current state

- **"Wrong?" categorization link on PackageDetail Categories row:**
  - Small "Wrong?" button next to the category pills
  - On click (signed in): opens "File issue" modal pre-filled to file against `msitarzewski/brew-browser` with title `Wrong category for {pkg.name}: currently [a, b]` and body `Suggest: [user types]`
  - On click (signed out): opens GitHub in browser to the new-issue URL (deeplink fallback)

- **Personal stats card on Dashboard:**
  - Small line: "You've starred 47 of your 325 installed packages" (uses `github_is_starred` in batch — call for each installed package, cached)
  - Only shown when signed in

### Tests
- Star round-trip (use a known test repo, or mock)
- Issue creation returns URL
- Scope check fails gracefully

### Acceptance criteria
- Star button reflects real state on first load, toggles on click
- Issue creation opens the created issue's URL in browser after success
- "Wrong?" link works both signed-in (in-app modal) and signed-out (deeplink)

---

## Agent assignments

Run agents in parallel waves. Each agent gets the relevant memory-bank context plus its scoped spec from this doc.

### Wave 1 (parallel — both fully independent)

1. **Backend Architect** → Phase 12a (Bundled catalog backend)
2. **Frontend Developer** → Phase 12b (Settings shell + brew analytics command)

### Wave 2 (parallel — gates on Wave 1)

3. **Backend Architect** → Phase 12c backend (github_repo_stats)
4. **Frontend Developer** → Phase 12c frontend + Phase 12d (Settings network + paranoid backend)
5. **Backend Architect** → Phase 12e backend (Device Flow + Keychain) — note: requires a one-time OAuth App creation by the user; agent should produce the BUILD.md addendum but cannot test the live flow

### Wave 3

6. **Frontend Developer** → Phase 12f (authed actions + Wrong? link + Dashboard stats card)

### Concurrent across all waves

- **Security Engineer**: review each PR for the network-call surface; confirm Paranoid mode gates all paths; verify Keychain entry permissions
- **Technical Writer**: update README + memory bank `decisions.md` + `security.md` + `frontendComponents.md` + `backendApi.md` as each wave lands
- **Code Reviewer**: final pass before each commit cluster

### Tests are non-negotiable

Every backend command gets at least one unit test. Every component gets typed props validated by `npm run check`. Final lint pass: `cargo clippy --all-targets -- -D warnings` and `npm run check` must both pass before any commit.

## Out of scope for Phase 12

- Recipes (Phase 10) — still deferred; benefits from catalog so naturally follows Phase 12
- Build-error rates / reverse deps / dep-tree viz — all become trivial AFTER 12a, but they're Phase 13 polish, not Phase 12 core
- Brewfile validation pre-import — same
- "What's new this week" feed — same
- Tier B Tahoe Liquid Glass (Swift bridge) — v0.2
- `installedAt` field on Package + Last-Updated sort — small standalone task, can slot anywhere
