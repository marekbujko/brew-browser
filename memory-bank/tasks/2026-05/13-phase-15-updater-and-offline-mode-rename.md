# 2026-05-25 — Phase 15: In-app updater + Offline Mode rename

**Phase:** 15 (planned + implemented + reviewed; **NEEDS-WORK** before ship)
**Status:** ⚠️ In progress — implementation merged, 5 CRITICAL findings outstanding
**Commit:** uncommitted (working tree)
**Release:** target v0.3.0
**Date:** 2026-05-25 (planning started 2026-05-24 late evening, implementation early 2026-05-25)

## Scope

Two coupled changes shipped together:
1. **In-app update mechanism.** Today's v0.2.1 users have no way to learn that v0.2.2 / v0.3.0 / etc. exists without stumbling on the GitHub releases page. Add a properly-gated "check for updates" flow via `tauri-plugin-updater`: manual button + optional daily auto-check (off by default) + title-bar indicator pill + × close-as-skip.
2. **Rename "Paranoid Mode" → "Offline Mode" in the user-facing UI.** "Paranoid" carries unintended stigma. "Offline Mode" is plainspoken and matches the README's "no surprise network" framing. Internal `paranoid_mode` field + `require_network()` helper stay unchanged (no migration churn).

## What landed (implementation phase — NEEDS-WORK)

Plan written to `memory-bank/phase15-plan.md` after pre-flight design discussion covering:
- Tauri plugin vs roll-our-own (plugin wins — battle-tested)
- Manifest hosted at `brew-browser.zerologic.com/updater.json` (vs GitHub Releases — cleaner CSP, tighter control)
- Minisign verification + sha256 (Sparkle convention, defense-in-depth)
- UI placement: no modal anywhere; title-bar indicator pill + close-as-skip + Settings → Network → Updates subsection
- 5 open questions resolved before implementation start (skip-list in settings.json; sha256 included; daily auto-check; stable channel only; security.md footnote rather than refactor)

**Wave 1: 4 parallel implementation agents:**
- **Backend Architect** — Rust + Tauri plugin + scheduler + 8 backend tests. +34 backend tests (411 → 445). cargo check + clippy clean. Files: new `commands/updater.rs`, new `tools/release/publish-manifest.sh`, modified `Cargo.toml`, `tauri.conf.json`, `capabilities/default.json`, `lib.rs`, `error.rs`, `state.rs`, `settings.rs`.
- **Frontend Developer #1** — store + IPC wrappers + UpdateIndicator + SettingsSectionUpdates + Settings section rename. 0 errors. Files: new `updater.svelte.ts`, `UpdateIndicator.svelte`, `SettingsSectionUpdates.svelte`. Modified `api.ts`, `types.ts`, `+page.svelte`, `SettingsSectionNetwork.svelte`.
- **Frontend Developer #2 (Rename Sweep)** — Paranoid → Offline in `PackageDetail.svelte`, `stores/{catalog,github}.svelte.ts`, `README.md`, `landing/index.html`. AboutModal.svelte + landing already clean.
- **Technical Writer** — `BUILD.md` minisign setup section + `memory-bank/security.md` footnote at top + §15 stub at bottom.

**Bridging work (Lead):** Added `update_skip` IPC command to wire the frontend's skip wrapper to backend's `push_skipped_version` settings helper. Made `persist` `pub(crate)` so updater.rs could use it. Fixed one `let-chain` syntax issue (project on Rust 2021 edition).

**Wave 2: 2 parallel review agents:**
- **Code Reviewer** — VERDICT: **NEEDS-WORK**. 4 CRITICAL + 8 IMPORTANT + 7 NIT findings.
- **Security Engineer** — VERDICT: **READY-FOR-SCRUTINY preserved IF criticals fixed.** 2 CRITICAL (one overlapping CR, one new) + 4-5 IMPORTANT. Wrote `memory-bank/security.md` §15 in full.

## Outstanding CRITICAL findings (5)

1. **IPC wire-shape mismatch on `UpdateCheckOutcome::Available`.** Backend ships flattened `{kind, version, currentVersion, notes, pubDate, skipped}`; frontend reads `outcome.info.{version, notesUrl, sha256}`. Happy path silently breaks (indicator renders `undefined`).
2. **"Relaunch now" button re-runs install.** No `update_relaunch` IPC; button calls `updater.install(version)` again — infinite re-install loop. Plugin's macOS install path doesn't auto-restart the binary either.
3. **Manifest artifact format mismatch.** Plugin expects `.app.tar.gz` (gzipped tar); my `publish-manifest.sh` + `BUILD.md` point at `.dmg`. Every install attempt fails with "invalid gzip" error.
4. **Missing error variants in frontend union.** Backend's `HashMismatch`, `SignatureVerificationFailed`, `DowngradeRejected` don't exist in `BrewErrorPayload`. `brewErrorMessage(e)` returns `undefined` → error suppressed entirely.
5. **`update_skip` revokes paranoid mode on Corrupt settings.** ⚠️ Security bug introduced by Lead's bridging command. When settings are Corrupt (fail-closed paranoid-on state), `update_skip` writes `Settings::default()` (paranoid_mode = false) to disk — clicking × to dismiss an update notification silently disables the network kill switch.

## Outstanding IMPORTANT findings

- Generic `brewErrorMessage` default toast still says "Paranoid mode is on" — rename sweep didn't reach the central default
- `cached_available` never cleared after successful install (compounds CRITICAL-2)
- Manifest URL allowlist documented in threat model but NOT implemented (claim in security.md §15 needs revision OR the allowlist needs adding)
- Plugin enforces NEITHER 8 KiB manifest size cap NOR 200 MB artifact size cap NOR per-hop redirect re-validation — all called for in the plan but `tauri-plugin-updater 2.10.1` doesn't expose hooks
- Placeholder pubkey duplicated in `lib.rs` + `tauri.conf.json` with no startup guard against shipping a release build with the placeholder
- `update_skip` snapshot-then-write concurrent-skip race (lost-write under simultaneous clicks)
- Auto-check scheduler timestamp is in-memory only — for typical "open in morning, close at night" usage patterns, the auto-check effectively never fires across launches
- `update_install` uses feature name `"update_check"` instead of `"update_install"` (toast UX inconsistency, minor)

## Files

New (4): `src-tauri/src/commands/updater.rs`, `tools/release/publish-manifest.sh`, `src/lib/components/UpdateIndicator.svelte`, `src/lib/components/SettingsSectionUpdates.svelte`, `src/lib/stores/updater.svelte.ts`, `BUILD.md` (extended), `memory-bank/phase15-plan.md`. Modified: backend (`Cargo.toml`, `tauri.conf.json`, `capabilities/default.json`, `lib.rs`, `error.rs`, `state.rs`, `commands/{mod,settings,updater}.rs`), frontend (`api.ts`, `types.ts`, `+page.svelte`, `SettingsSectionNetwork.svelte`, the rename-sweep targets), memory-bank (`security.md` footnote + §15, `phase15-plan.md` resolved-decisions section).

## Tests / verification

- `cargo test`: **445 passed**, 0 failed, 6 ignored (411 → 445, +34 new for Phase 15)
- `cargo clippy --all-targets -- -D warnings`: clean
- `npm run check`: 0 errors
- `npm run build`: clean
- Tool battery (Security Engineer): cargo audit / deny / npm audit / semgrep / gitleaks all clean

## Notes / decisions

- **First 4-agent parallel wave in the project.** File scopes pre-partitioned to avoid conflict. Worked cleanly; integration cost was 1 small IPC bridge + 1 visibility fix.
- **2-agent review wave** (Code Reviewer + Security Engineer in parallel) caught issues neither would have on their own:
  - Code Reviewer caught the wire-shape mismatch + relaunch button + manifest format + missing error variants
  - Security Engineer caught the `update_skip` paranoid-revoke bug + size-cap / allowlist gaps
- **Architecture is sound.** Every gate routes through `require_network`; downgrade rejection fires before plugin delegation; placeholder pubkey fails closed; CSP delta clean; zero unsafe / @html / eval. The CRITICAL findings are integration-seam bugs, not design flaws.
- **Estimated fix-up:** ~2-3 hours by Lead, not another agent wave. Failures are localized to seams; single-hand integration is more efficient than coordination.
- v0.3.0 won't ship until all 5 CRITICALs land + a Wave 3 re-review.
- See task #41 in the task tracker for the fix-up tracking.
