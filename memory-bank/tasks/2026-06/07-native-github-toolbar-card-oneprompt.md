# 07 — Native GitHub: toolbar chip + dashboard card + one-prompt keychain

**Date:** 2026-06-05
**Branch:** `experiment/native-swift-liquid-glass`

## Objective

Bring native to parity with the Tauri GitHub surfaces: a sponsor **heart** + an
**Octocat** connection chip in the toolbar, and the dashboard **"starred N of M"**
card — and fix the lazy-creds refresh + the dev keychain prompt storm.

## Toolbar

- `+` (no-op stub) → **pink heart** (`heart.fill`, `.foregroundStyle(.pink)` — a
  toolbar ignores `.tint` for symbol color) opening `github.com/sponsors/msitarzewski`.
- **Octocat chip**, shown only when signed in: `GithubMarkIcon` (bundled
  Octicons vector **PDF**, `Resources/github-mark.pdf`, rendered as a template
  image — SwiftUI can't load SVG; converted with `rsvg-convert`). Green when the
  `public_repo` scope is present, amber when incomplete. Click → opens Settings
  **GitHub pane** via a persisted `SettingsTab` AppStorage selection
  (`settings.selectedTab`) the button sets before `openSettings()`. A plain
  `Button` (not `SettingsLink`, which rendered blank with a custom icon).

## Dashboard card

`GitHubCard` (DashboardView), gated on `githubStatsEligible` (signed-in +
`githubAllowed`): header Octocat + `@username`, body "You've starred **N** of
**M** installed packages with GitHub homepages," with a checking… state.

- **M accuracy:** native previously only matched the catalog `homepage` field
  (→ "2 of 69"). Added a resolved `CatalogPackage.githubHomepage` — the Tauri
  cascade: homepage → source URL (`urls.stable.url` formula / `url` cask),
  canonicalized to `github.com/<owner>/<repo>` (`resolveGithubHomepage` in
  CatalogService). De-duped. Now ≈ Tauri's "6 of 193".
- **N speed:** `GitHubService.batchIsStarred` is a bounded-concurrency fan-out
  (12 in flight). Required making `GitHubService` a `Sendable struct` (it was an
  actor that serialized every call; it's stateless bar an immutable URLSession).
  Removed the now-stale `await` on the sync `status()` / `signOut()` at 4 sites.

## Lazy-creds refresh

GitHub sign-in is lazy. `githubStatus` + the card now refresh on every path that
can change it: detail-open (`loadGitHub`), device-flow completion
(`ensureGitHubSignIn`), and **window-becomes-active** (`scenePhase .active` in
ContentView — catches the Settings sign-in, which uses a separate GitHubService).
`loadGithubStats` is idempotent (`githubStatsLoaded`), reset on sign-out.

## One Keychain prompt (dev + prod)

Root cause of "perpetually signed-out, no prompt" in dev: `status()` used a
`kSecMatchLimitAll` batch read, which **silently skips** items whose ACL doesn't
match the binary (dev re-signs ad-hoc each rebuild) instead of prompting. Tauri
reads per-item → prompts.

Fix: store token + username + scopes as **one combined Keychain item**
(`github_credential_v1`, JSON). One item = one read = **one prompt**, dev and
prod. `readCredential()` migrates the legacy three-item layout into it on first
read (that migration prompts 3× once, then 1× forever). `status`, `repoStats`,
and `authedGate` all read the single item; sign-in writes it; `signOut` deletes
it + the legacy three.

## Outcome

`swift build` clean. Octocat green + card "≈6 of ≈193", one prompt per launch in
dev. Files: ContentView, DashboardView, AppModel, GitHubService, CatalogService,
SettingsView, GithubMarkIcon (new), Package.swift, Resources/github-mark.pdf.
