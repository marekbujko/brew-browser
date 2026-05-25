# 2026-05-25 — Issue #1 hunt: starredCache infinite-loop fix

**Phase:** Bug fix (carries into v0.3.0)
**Status:** ✅ Fixed (uncommitted, lands in v0.3.0)
**Date:** 2026-05-25 ~01:00–02:30 UTC

## Scope

Joshua Butner ([@heyjawrsh](https://github.com/heyjawrsh)) filed [issue #1](https://github.com/msitarzewski/brew-browser/issues/1) reporting "a series of alerts / toasts that I cannot dismiss" after signing in to GitHub. v0.2.1 had a fix attempt (`untrack` wrap in `DeviceFlowModal`'s toast effect) but the bug recurred for the issue reporter AND the project owner during disconnect→reconnect testing.

This task: trace the actual root cause systematically (instrumentation + console traces) and apply the right fix.

## Root cause

**`PackageDetail.svelte`'s `$effect` watching `starredCache` had an infinite-loop trap.** When `isStarred()` IPC failed (transient auth state during reconnect), the catch block wrote `"unknown"` into `starredCache` — the SAME sentinel the PackageDetail effect uses to mean "haven't fetched yet". The effect re-ran, saw `"unknown"`, fired isStarred again. Failed again. Wrote `"unknown"`. **Infinite IPC storm.**

Each IPC write to the cache invalidated every effect that read `signinState`, including the toast effect in DeviceFlowModal — which is why the visible symptom was a stack of "Signed in to GitHub" toasts that looked like the toast effect was looping. **The toast effect was a downstream victim of Svelte's effect scheduler being hammered by the cache storm, not the cause.**

The previous `untrack` fix and the v0.2.1 ship didn't touch this code path; the cache loop pre-existed in Phase 12f (`8b89c40`).

## Fix

1. New `"error"` variant in `StarredOutcome` type, distinct from `"unknown"`. The cache write on failure stores `"error"`; the PackageDetail effect's refetch test changes from `starredState === "unknown"` to `cached === undefined`. Failed attempts no longer retrigger refetches.
2. The `untrack` wrap in DeviceFlowModal's toast effect was insufficient to stop the cascade because the underlying cache storm was invalidating effects via a different path. Architectural fix for the toast (separate task): moved out of `$effect` entirely.

## Files

- `src/lib/stores/github.svelte.ts` — `StarredOutcome` adds `"error"`; `isStarred` catch writes `"error"` instead of `"unknown"`; expanded JSDoc explaining the sentinel split
- `src/lib/components/PackageDetail.svelte` — effect refetch test now checks `cached === undefined`, not display sentinel

## Verification

- Network tab during sign-in showed **0** `github_is_starred` calls (vs thousands pre-fix)
- 411 → 445 backend tests still pass (no test changes needed; fix was frontend-only at the cache layer)
- User confirmed "GH coming back instantly" after fix; no toast cascade

## Notes

- **The instrumentation trail told the story:** added per-effect run counters, `console.trace()` on every `signinState` write, observer effect to log every signinState transition. The `[diag-toast]` lines repeating 1000+ times WITHOUT `[diag-observer]` interleaving was the critical clue — proved the toast effect was being re-fired without signinState actually changing.
- Confirmed root cause by checking Network tab: cache-loop IPC storm was the actual driver. Once the cache fix landed, the network was silent and the toast effect fired exactly once per real signin transition.
- The toast architectural fix (next task) was the actual `effect_update_depth_exceeded` killer for users who STILL hit a cascade after the cache fix.
- See web research thread in NEXT-SESSION.md / [Svelte 5 $effect docs](https://svelte.dev/docs/svelte/$effect) — "$effect is an escape hatch" is the official guidance.
