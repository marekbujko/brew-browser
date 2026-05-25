# 2026-05-24 — GitHub auth hydration + sign-in toast fixes (between v0.2.0 and v0.2.1)

**Phase:** Hotfix (rolls into v0.2.1)
**Status:** ✅ Shipped (folded into `v0.2.1` tag)
**Commit:** `e04dbff` (3 files, +27 / -5)
**Date:** 2026-05-24 13:04 (7 minutes after v0.2.0)

## Scope

Three intertwined GitHub-auth bugs that surfaced immediately when user ran the freshly-released v0.2.0 .dmg. Quick mechanical fixes; same-day rollup into v0.2.1.

## What landed

### Bug 1 — `requireGithubSignIn()` bounced authenticated users to Settings
- Root cause: `github.loadStatus()` was only called when Settings → GitHub mounted. First click on Star / Watch / File-issue (before user opened Settings in current session) saw `github.status === null`, making `githubSignedIn = !!github.status?.signedIn` falsy
- Fix: call `github.loadStatus()` from `+layout.svelte` on app start, next to the other hydration calls
- (Later revised in v0.2.1 to lazy probing — eager probe was triggering a macOS Keychain prompt on every launch even for users who'd never used a GitHub feature)

### Bug 2 — Post-sign-in toast read "Signed in as @github user."
- Root cause: in `github.svelte.ts` poll loop, `this.signinState = { kind: "approved" }` was set BEFORE `await this.loadStatus()`. `DeviceFlowModal`'s `$effect` fires the moment signinState becomes "approved" and reads `status.username` — but at that instant, status hadn't refreshed yet, username was null, fallback "@github user" shown
- Fix: reorder — `loadStatus()` runs first, then `signinState = approved`. Toast and Settings panel both see the real username from the first paint.

### Bug 3 — Stack of duplicate "Signed in to GitHub" toasts
- Root cause: same `$effect` read `github.status?.username` reactively. Every post-approve status hydration (and any later status refresh while still "approved") re-ran the effect → new toast each time. The timeout cleanup only cleared the pending `cancelSignin` timer, not the already-fired toast
- Fix: wrap the username read in `untrack(() => …)` in `DeviceFlowModal.svelte` so the effect's only reactive dep is `signinState`. One state transition, one toast.

## Files

- `src/routes/+layout.svelte` — added eager `github.loadStatus()` (later reverted to lazy in v0.2.1)
- `src/lib/stores/github.svelte.ts` — poll-loop reorder: loadStatus before signinState flip
- `src/lib/components/DeviceFlowModal.svelte` — `untrack` wrap on the toast effect's status read

## Tests / verification

- `cargo test`: 411 (unchanged)
- `npm run check`: 0 errors
- `npm run build`: clean

## Notes / decisions

- **All three bugs surfaced together** during user testing: fresh launch + Keychain token + first click on Star → (a) bounce to Settings, then (b) sign-in retry toasted fallback username, then (c) stacked 10+ duplicate success toasts as status hydrated through.
- The eager `loadStatus()` in `+layout.svelte` was the WRONG fix for bug 1 — it triggered a macOS Keychain ACL prompt on every launch for fresh installs (binary sig change → ACL re-confirmation prompt). User flagged the prompt almost immediately; v0.2.1 reverted to lazy probing in `requireGithubSignIn()`.
- **Lesson:** the Keychain ACL prompt is a UX cost; touch the Keychain ONLY when the user is actively trying to use the token. The contextual prompt is meaningful; the eager prompt trains users to dismiss without context.
- This commit was NOT released on its own — it stayed in the working tree for ~30 minutes until the v0.2.1 release commit moved the tag forward to capture both this and the lazy-probe fix.
