# NEXT-SESSION handoff — read this first

**Date written:** 2026-05-25 (just before `/compact`, end of marathon GitHub-debug session)
**Session lead:** Claude Opus 4.7 [1m] (Claude Code in the terminal) with Michael

Read this first, then `activeContext.md`, then the latest entries in `progress.md`, then specific `tasks/2026-05/{14, 15}.md` for full detail on what just happened.

---

## Current state at compact

- **v0.2.1** is the latest GitHub release (signed/notarized .dmg live)
- **v0.3.0 prep is uncommitted** in the working tree — ~30 modified files
- **9 stars on GitHub.** Joshua Butner ([@heyjawrsh](https://github.com/heyjawrsh)) filed [issue #1](https://github.com/msitarzewski/brew-browser/issues/1) — ROOT-CAUSED + FIXED this session; ships in v0.3.0
- **LinkedIn announcement** at <https://www.linkedin.com/in/msitarzewski/recent-activity/all/> — 1,496 impressions in first 10h, 5 new followers, real traction
- **HN post buried** at 1 point; not regenerating

## What's in the working tree

### Phase 15 — In-app updater + Offline Mode UI rename
**Status: implementation complete, NEEDS-WORK from review (5 CRITICALs)**

4-agent parallel implementation wave returned clean (+34 backend tests). 2-agent review wave returned **NEEDS-WORK**. The 5 CRITICAL findings (see task #41 + `tasks/2026-05/13-phase-15-updater-and-offline-mode-rename.md` for details):

1. **IPC wire-shape mismatch on `UpdateCheckOutcome::Available`** — backend sends flat `{kind, version, currentVersion, notes, pubDate, skipped}`; frontend reads `outcome.info.{version, notesUrl, sha256}`. The frontend reads `undefined` for everything. Fix: flatten the frontend type to match the backend serde output, drop the invented `notesUrl`/`sha256` sub-fields.
2. **"Relaunch now" button re-runs install** — there's no `app.restart()` IPC; clicking the button calls `updater.install(version)` again. Infinite re-install loop. Fix: add `update_relaunch()` IPC that calls `tauri::AppHandle::restart()`; rewire button.
3. **Manifest artifact format** — plugin expects `.app.tar.gz` (gzipped tar of `.app`); `tools/release/publish-manifest.sh` + `BUILD.md` say `.dmg`. Every install attempt fails with "invalid gzip" error. Fix: emit `.app.tar.gz` alongside `.dmg`; update manifest URL.
4. **Missing error variants in frontend union** — backend's `HashMismatch`, `SignatureVerificationFailed`, `DowngradeRejected` don't exist in `BrewErrorPayload`. `brewErrorMessage(e)` returns `undefined` → error suppressed entirely. Fix: extend `BrewErrorPayload` + add `brewErrorMessage` cases.
5. **`update_skip` revokes paranoid mode on Corrupt settings** ⚠️ — Lead's bridging command writes `Settings::default()` (paranoid_mode = false) when settings are Corrupt. Clicking × on an update indicator silently disables the network kill switch. Fix: refuse skip on Corrupt OR keep skip in-memory only.

Estimate: **2-3 hours**. All five are well-scoped, mechanical. Code Reviewer + Security Engineer both signed off on the architecture; only the integration seams need work.

### GitHub integration completion (4 cascading bug fixes + 3 new features)

Six hours of debug across issue #1. The story (see task #14 + #15 for full detail):

1. **Cache loop fix** — `PackageDetail.svelte`'s `isStarred` effect overloaded `"unknown"` as both "haven't fetched yet" AND "fetched and failed." On failure → cache write → effect re-run → refetch → failure. Infinite IPC storm. Fixed with `"error"` variant in `StarredOutcome`.
2. **Toast `$effect` → imperative refactor** — even after cache fix, users could hit `effect_update_depth_exceeded`. Per Svelte 5 docs, `$effect` is "an escape hatch" not "a side-effect channel." Moved `toast.success` from `$effect` to imperative call in `signIn()` poll loop. `SigninState.approved` now carries `username` so the toast text reads from signinState (one dep, no hidden second read).
3. **Scope parser** — GitHub returns OAuth `scope` field comma-separated, not space-separated. `split_whitespace()` → `split(comma + whitespace)`.
4. **Watch scope** — `PUT /repos/{o}/{r}/subscription` requires `notifications`, not `public_repo`. Added to `GITHUB_OAUTH_SCOPES`. GitHub returns 404 as the privacy-mask for "missing scope."

Then added:
- **Per-action scope gate** — `authed_gate(required_scope)`. star/issue → `public_repo`, watch/unwatch → `notifications`. +2 backend tests pin behavior.
- **Actionable Re-authorize toast** — `Toast.action: { label, onClick }`. On `scope_required`, toast offers "Re-authorize" button → calls `signIn()` (GitHub shows scope diff only; no sign-out needed).
- **Octocat status chip in title bar** — real Octocat from Primer/Octicons (MIT-licensed; Lucide strips brand icons). Green/amber/hidden. Click → Settings → GitHub. Eager `loadStatus()` in `TitlebarControls.onMount` makes it work; v0.3.0+ follow-up to gate on localStorage flag.

End-to-end verified by user.

## Critical context for any release

- **Apple signing env** at `~/.config/brew-browser/signing.env` (chmod 600, outside repo) — valid + live
- **Anthropic API key** in `tools/categorize/.env` — valid + live
- **GitHub OAuth client_id** (`Ov23liJZKbvrSBuiOPkT`) — public per RFC 8628 §3.1
- **GitHub OAuth scope list** in `src-tauri/src/github/auth.rs:96` — now `["read:user", "public_repo", "notifications"]`. Existing v0.2.1 users with 2-scope tokens will hit the actionable Re-authorize toast and be guided through an incremental scope grant.
- **Updater minisign pubkey** — still PLACEHOLDER in `tauri.conf.json` + `src-tauri/src/lib.rs`. Real key needs `tauri signer generate -w ~/.config/brew-browser/updater.key` per BUILD.md BEFORE v0.3.0 ships. Without it, every install attempt fails signature verification (fails closed, but visible to user).

## What's queued for the next session (priority order)

### 1. Phase 15 fix-up pass (task #41) — BLOCKS v0.3.0 SHIP
~2-3 hours, all 5 CRITICALs listed above. After fix-up: Code Reviewer + Security Engineer Wave 3 re-review.

### 2. v0.3.0 release
- Generate real minisign keypair: `tauri signer generate -w ~/.config/brew-browser/updater.key`
- Replace placeholder pubkey in `src-tauri/src/lib.rs` + `tauri.conf.json`
- Version bump in `src-tauri/Cargo.toml` + `tauri.conf.json` + `landing/index.html` softwareVersion: `0.2.1` → `0.3.0`
- README "Status" section updated to reflect v0.3.0
- Memory bank refresh (activeContext, progress, NEXT-SESSION)
- Landing page deploys via `rsync` to `umacbookpro:Sites/brew-browser/`
- `npm run tauri build` with signing env sourced → signed + notarized .dmg
- `tools/release/publish-manifest.sh 0.3.0` to emit + deploy `updater.json` (after Phase 15 fix-up sorts the .app.tar.gz format)
- `git tag v0.3.0`, push, `gh release create v0.3.0 --notes-file <notes>` with the .dmg attached
- Comment on issue #1 with "fixed in v0.3.0" + close

### 3. Optional / nice-to-have for v0.3.0

- **Expand GitHub package resolution** (task #46) — walk `urls.stable.url` / `head` / cask `url` beyond `homepage`. Roughly doubles the "X of Y with GitHub homepages" coverage on the Dashboard's personal-stats card. ~1-2h.
- **Localstorage flag for eager `loadStatus()`** — gate the eager call in `TitlebarControls.onMount` on `localStorage["brew-browser:has-signed-in"]` so users who never sign in see zero Keychain prompts. Set the flag on `signIn` success; clear on `signOut`. ~15min.
- **`cancelSignin` timer leak edge case** — imperative `setTimeout(cancelSignin, 1500)` in `signIn()` doesn't get cleared if user clicks Cancel before 1500ms. Old timer fires + sets state to idle even during a new signin. Edge case (unusual usage); track the timer id and clear on `cancelSignin()`. ~10min.

## Credentials / paths reference

| What | Where |
|------|-------|
| Repo on disk | `/Users/michael/Clean/brew-browser/` |
| GitHub repo | `github.com/msitarzewski/brew-browser` |
| Anthropic API key | `tools/categorize/.env` (cascade-shared by enrich) |
| Apple signing env | `~/.config/brew-browser/signing.env` (chmod 600) |
| Updater minisign key (target — generate before v0.3.0) | `~/.config/brew-browser/updater.key` |
| Landing source | `landing/` |
| Landing deploy | `michael@umacbookpro:Sites/brew-browser/` |
| umbp Tailnet IP / hostname | `100.98.187.7` / `umacbookpro` |
| Catalog data | `src-tauri/data/catalog/{formula,cask}.json.gz` (~6.1 MiB) + `manifest.json` |
| Enrichment data | `src-tauri/data/enrichment.json.gz` (15,725 Tier A entries) |
| Catalog refresh | `python tools/catalog/fetch.py` |
| Enrichment refresh | `tools/categorize/.venv/bin/python3 tools/enrich/enrich.py --tier-a` |
| Runtime caches | `~/Library/Application Support/brew-browser/` |
| Keychain | service `dev.openbrew.browser`, accounts `github_access_token` + `_scopes` + `github_access_token_scopes` |
| Icon source | `docs/icon/brew-browser.svg` (full-bleed square, Tahoe-clean) |
| Icon regen | `npm run tauri icon docs/icon/brew-browser.svg` |
| Memory bank task records | `memory-bank/tasks/2026-05/*.md` (15 + README + deferred) |
| Memory bank phase plans | `memory-bank/phases/{phase12,phase13}-plan.md` (shipped); `memory-bank/phase15-plan.md` (in-flight, top level) |
| Memory bank scan artifacts | `memory-bank/scans/2026-05-23/*` (initial pre-release tool battery) |

## Notes from this session that matter for v0.3.0

- **6-hour debug into issue #1** ended in 4 fixes + 3 new features. The recovery story is genuinely instructive. The cache-loop was hammering Svelte's scheduler, causing the toast effect to look like the bug. The actual structural problem was using `$effect` for a one-shot side effect — Svelte 5's docs explicitly call out that pattern as wrong.
- **`grep_for_diag_must_be_clean`** — all `[diag]` / `console.trace` instrumentation was reverted at end of session. Verified `grep -rn "diag\|console.trace" src/lib/` returns nothing. Backend `_setSignin` wrapper kept (clean, no logs — useful for future invariants).
- **`AGENTS.md` and `CLAUDE.md`** at repo root are intentional (user's AI-workflow guide). Untracked. Should be committed in the v0.3.0 commit since the user said to leave them. CLAUDE.md is a symlink to AGENTS.md.
- **Tool classifier had a transient outage** near end of session — `npm run tauri dev` couldn't be launched via Bash for ~10 minutes. User ran it themselves. Worked fine via the user's terminal. (Anthropic-side outage on the auto-mode safety check; resolved by itself.)
- **Web research is well-cited** in the task notes — every architectural decision (especially the `$effect` → imperative move) has the official Svelte 5 docs link backing it.
- **No agent stamps in `agentLog.md` this session** — the convention is dormant. Either re-enable in future agent prompts or drop the protocol item from `toc.md`. Not a priority.

## Open questions worth thinking about for v0.3.0

1. Should Phase 15's manifest sha256 verification path (currently deferred to v0.3.1 per Backend Architect's deviation note) move to "do before v0.3.0 ships"? It's a real defense-in-depth gap.
2. Octocat chip needs to differentiate **"signed-in but token is from v0.2.1 era (missing notifications)"** from **"signed-in but somehow lost public_repo too"**. Right now both = amber with same tooltip. v0.3.0 follow-up to show the specific missing scope in the tooltip.
3. README's "Status" section currently says v0.2.1 — make sure to refresh as part of the v0.3.0 commit.

## What is NOT a problem (calling out so next-session-Claude doesn't re-investigate)

- ✅ Toast cascade — FIXED
- ✅ Star action — WORKS
- ✅ Watch action — WORKS for users with `notifications` scope
- ✅ Scope parser — FIXED (comma + whitespace split)
- ✅ GitHub OAuth Device Flow — works as designed
- ✅ Per-action scope gating — wired
- ✅ Re-authorize toast button — wired (not visually verified end-to-end because user's token already has notifications, but architecture is sound)
- ✅ Octocat status chip — works in the dev build
- ✅ Build / lint / test — all clean (447 backend tests pass, npm clean, clippy clean)

## What IS a problem (don't forget)

- ⚠️ Phase 15 has 5 CRITICAL findings still pending (task #41)
- ⚠️ Updater minisign pubkey is still placeholder
- ⚠️ Eager `loadStatus()` re-introduces v0.2.1 Keychain prompt (follow-up: localStorage flag)
