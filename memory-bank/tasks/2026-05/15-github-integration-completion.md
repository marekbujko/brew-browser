# 2026-05-25 — GitHub integration completion (toast architectural fix + scope parsing + watch scope + per-action gate + Octocat chip + re-auth UX)

**Phase:** v0.3.0 prep — GitHub integration polish
**Status:** ✅ Complete (uncommitted, lands in v0.3.0)
**Date:** 2026-05-25 ~02:30–06:00 UTC

## Scope

After the [cache-loop fix](./14-issue-1-hunt-cache-loop.md) landed, the user could sign in cleanly. New bugs surfaced as they exercised the auth path:

1. Toast cascade STILL hit on disconnect→reconnect (cache loop was one source but not the only one — Svelte's `$effect` wrapping the toast was the bigger structural problem)
2. Star action rejected with `ScopeRequired` even when scopes display showed `public_repo`
3. Watch action failed with HTTP 404 from `api.github.com/.../subscription`
4. Connection state invisible without opening Settings → GitHub
5. Recovering from scope-incomplete required a manual sign-out + sign-in

This task fixed all five.

## Fix 1 — Toast architectural pattern (`$effect` → imperative)

**Symptom:** 300+ "Signed in as @msitarzewski" toasts stacking, modal stuck open. Svelte's `effect_update_depth_exceeded` runtime error.

**Root cause:** the success-toast was wired via a `$effect` watching `signinState`. Per Svelte 5 official docs, `$effect` is "an escape hatch" — NOT for one-shot side effects of state transitions. Effects re-run on dependency invalidation; for class-instance `$state` fields with property-level reactivity + multiple components reading the same store, this produces re-entry cycles that the Svelte scheduler catches and aborts (the `effect_update_depth_exceeded` error). Combined with the cache-loop hammering the scheduler, the toast effect re-fired hundreds of times per real transition.

**Fix:** moved `toast.success/error` + the `setTimeout(cancelSignin, 1500)` from the `$effect` in `DeviceFlowModal.svelte` INTO `signIn()` poll loop in `github.svelte.ts` — fired imperatively at the moment of the transition. The `$effect` block in DeviceFlowModal was deleted entirely. Modal template still reads signinState (which is template reactivity, the correct pattern), but no observation-via-effect.

**Web research confirmed:** the official Svelte 5 [`$effect` docs](https://svelte.dev/docs/svelte/$effect) explicitly recommend this pattern; the [`effect_update_depth_exceeded` runtime-error docs](https://svelte.dev/docs/svelte/runtime-errors) flag "multiple components binding to properties of the same object" as a common cause; [issue #14697](https://github.com/sveltejs/svelte/issues/14697) is a community discussion endorsing imperative side effects at the call site.

**Captured username on signinState:** the `SigninState.approved` variant now carries `username: string | null`, populated by the poll loop right after `loadStatus()` so the toast text gets personalization without any second reactive read.

## Fix 2 — Scope parser (comma-separated, not whitespace-separated)

**Symptom:** Star rejected with `ScopeRequired` despite Settings panel showing `Scopes: public_repo,read:user`.

**Root cause:** GitHub's `/login/oauth/access_token` returns the `scope` field comma-separated (`"public_repo,read:user"`), NOT space-separated per OAuth 2.0 RFC 6749 §3.3. Our parser used `s.split_whitespace()` which produced a one-element array `["public_repo,read:user"]`. The action-gate check `scopes.iter().any(|s| s == "public_repo")` then failed (no element matched `"public_repo"` exactly).

**Fix:** `s.split(|c: char| c == ',' || c.is_whitespace()).filter(|x| !x.is_empty())`. Handles both formats defensively. File: `src-tauri/src/github/auth.rs:547-558`.

## Fix 3 — Watch scope (`notifications`, not `public_repo`)

**Symptom:** Watch returned HTTP 404 from `PUT https://api.github.com/repos/{owner}/{repo}/subscription`. Repo was real, auth was present (Star worked).

**Root cause:** GitHub's [OAuth scopes documentation](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps) explicitly says: *"The `notifications` scope grants watch and unwatch access to a repository."* `public_repo` doesn't cover watch. GitHub returns 404 (privacy-preserving mask for "you don't have the scope") instead of a 403.

**Fix:** added `"notifications"` to `GITHUB_OAUTH_SCOPES` in `src-tauri/src/github/auth.rs:96`. Existing users with old 2-scope tokens get an actionable Re-authorize toast (see Fix 5).

## Fix 4 — Per-action scope gate

**Refactor:** `authed_gate()` in `src-tauri/src/commands/github.rs` now takes a `required_scope: &str` parameter. Each command's call site passes the scope it actually needs:

- `github_star` / `github_unstar` / `github_is_starred` → `SCOPE_PUBLIC_REPO`
- `github_watch` / `github_unwatch` → `SCOPE_NOTIFICATIONS`
- `github_create_issue` → `SCOPE_PUBLIC_REPO`

The typed `ScopeRequired { scope }` error now carries the SPECIFIC scope name (not just "public_repo"). +2 new backend tests pin the per-action behavior (411 → 445 → **447 passed**).

## Fix 5 — Actionable re-auth toast

**Pattern:** when an authed action fails with `scope_required`, instead of a generic error toast, show an actionable toast with a `Re-authorize` button. Click → `github.signIn()` re-runs Device Flow requesting the full `GITHUB_OAUTH_SCOPES` list. GitHub's consent screen surfaces only the missing scope (existing grants persist); user clicks Authorize, the new token transparently replaces the old in Keychain. **No sign-out needed.**

**Implementation:**
- `src/lib/stores/toast.svelte.ts` — `Toast` type gains optional `action: { label, onClick }`. New `invokeAction(id)` method runs the callback then dismisses.
- `src/lib/components/Toast.svelte` — renders the action button below the body, tone-colored to match the toast's left border.
- `src/lib/components/PackageDetail.svelte` — new `showActionFailureToast(label, e)` helper. On `scope_required` it fires: title "Couldn't <action>", body "Needs the '<scope>' GitHub permission. Click to grant it without signing out.", button "Re-authorize" → `github.signIn()`. All three catch blocks (star/watch/file-issue) routed through it.

## Fix 6 — GitHub connection-status chip (Octocat icon)

**Visual:** title-bar cluster gains an Octocat icon. State colors:
- Hidden when signed-out (no clutter for non-GitHub users)
- 🟢 Green when signed-in with required scopes
- 🟠 Amber when signed-in but scope-incomplete (click → opens Settings → GitHub for re-auth)

Click anywhere on the chip → `ui.openSettings("github")` (uses the existing deep-link plumbing).

**Real Octocat:** Lucide strips brand icons (trademark policy), so we ship a new `src/lib/components/GithubMarkIcon.svelte` with the official path data lifted from Primer/Octicons (MIT-licensed). API matches Lucide icons (`size`, `class`).

**Trade-off:** the chip needs `github.status` populated on first paint, which requires an eager `github.loadStatus()` from `TitlebarControls.onMount`. That re-introduces a Keychain ACL prompt on every fresh-signature launch (the issue v0.2.1 originally fixed). For v0.3.0 we accept the friction; **v0.3.0+ follow-up: gate the eager call on a `localStorage["brew-browser:has-signed-in"]` flag** so users who never sign in see zero Keychain prompts.

## Files

**Backend (Rust):**
- `src-tauri/src/github/auth.rs` — scope parser fix + `notifications` added to const + test fixture updated to 3-scope + new test pinning the scope minimum at v0.2.2 level
- `src-tauri/src/commands/github.rs` — `authed_gate` takes `required_scope`; per-action constants `SCOPE_PUBLIC_REPO` + `SCOPE_NOTIFICATIONS`; +2 new tests (`authed_gate_returns_scope_required_when_notifications_missing`, `authed_gate_passes_for_watch_with_notifications_scope`); test helper signature updated

**Frontend (Svelte / TypeScript):**
- `src/lib/stores/github.svelte.ts` — toast imperative call in `signIn()` approved/denied/expired branches; `_setSignin` helper centralizes writes; `SigninState.approved` carries `username`; cache-loop fix (issue #1 root cause) — `"error"` variant in `StarredOutcome`
- `src/lib/components/DeviceFlowModal.svelte` — deleted the toast `$effect` entirely; removed `untrack` import (no longer needed)
- `src/lib/stores/toast.svelte.ts` — `Toast.action` type + `invokeAction(id)` method
- `src/lib/components/Toast.svelte` — renders action button with tone-matched styling
- `src/lib/components/PackageDetail.svelte` — `showActionFailureToast` helper; refetch test fix (cache loop); 3 catch blocks routed through new helper
- `src/lib/components/TitlebarControls.svelte` — new chip render; `githubChipState` derived; eager `loadStatus()` in onMount
- `src/lib/components/GithubMarkIcon.svelte` (NEW) — Octocat from Primer/Octicons

## Tests / verification

- `cargo test`: **447 passed**, 0 failed, 6 ignored (445 → 447, +2 new for per-action scopes)
- `cargo clippy --all-targets -- -D warnings`: clean
- `cargo check`: clean
- `npm run check`: 0 errors, 3 pre-existing warnings
- `npm run build`: clean
- End-to-end verified by user: sign-in flow shows ONE clean toast, Star + Watch + File-issue all work with `public_repo + read:user + notifications` token, Octocat chip flips green/amber on scope state

## Web research consulted

- [Svelte 5 `$effect` docs](https://svelte.dev/docs/svelte/$effect) — "$effect is an escape hatch"; not for state synchronization or one-shot side effects
- [`effect_update_depth_exceeded` runtime errors](https://svelte.dev/docs/svelte/runtime-errors) — common cause: multiple components binding to properties of the same object
- [Svelte issue #14697](https://github.com/sveltejs/svelte/issues/14697) — community discussion: don't update state inside effects
- [Joy of Code — Avoid Async Effects in Svelte](https://joyofcode.xyz/avoid-async-effects-in-svelte) — one-shot side effects belong at the call site
- [GitHub REST API watching endpoint docs](https://docs.github.com/en/rest/activity/watching) — endpoint contract for `PUT /repos/{o}/{r}/subscription`
- [GitHub OAuth scopes docs](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps) — `notifications` scope grants watch/unwatch
- [GitHub community discussion: 404 = insufficient permissions](https://github.com/orgs/community/discussions/52522)

## Notes

- The user (Michael) was patient and steered well: pushed back when I went defensive instead of diagnostic, asked me to web-search for best practices when I was guessing, and made the right architectural decision to skip a v0.2.2 hotfix and roll everything into v0.3.0.
- **Six hours of debug time** across the session. The split between "what looked like the bug" (toast effect re-firing) and "the actual root cause" (a cache-loop in PackageDetail hammering Svelte's scheduler, AND a structural misuse of `$effect` for one-shot side effects) is a great teaching example of the rabbit-hole shape of complex reactivity bugs.
- The Octocat chip is genuinely useful — it surfaces auth state at a glance and would have caught the scope-parse + watch-scope bugs days ago if it had existed.
- v0.3.0 still needs: Phase 15 fix-up (5 CRITICALs from the in-app updater review), version bump, build, ship.
