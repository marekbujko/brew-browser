# 2026-05-24 — Release pipeline: signed + notarized .dmg

**Phase:** Build infrastructure
**Status:** ✅ Shipped
**Commit:** `cb60e4a` (12 files, +496 / -125)
**Date:** 2026-05-24 00:48

## Scope

End-to-end release flow producing a Gatekeeper-clean `.dmg`: Developer ID Application signing + Apple notary submission + stapling. Removes the "open anyway" right-click ceremony for end users.

## What landed

- **Signing config** loaded from `~/.config/brew-browser/signing.env` (chmod 600, outside repo). Required env: `APPLE_ID`, `APPLE_PASSWORD` (app-specific), `APPLE_TEAM_ID`, signing identity.
- **`signingIdentity`** wired into `src-tauri/tauri.conf.json` macOS bundle config: `"Developer ID Application: Michael Sitarzewski (7JQGQ7CRH8)"`.
- **`hardenedRuntime: true`** — required for notary.
- **`minimumSystemVersion: "13.0"`** pinned.
- **Build flow:** `set -a && source ~/.config/brew-browser/signing.env && set +a && npm run tauri build` produces signed `.app` → submits to Apple notary → polls → staples → emits `.dmg` in `src-tauri/target/release/bundle/dmg/brew-browser_X.Y.Z_aarch64.dmg`.
- **`BUILD.md`** documenting the keypair setup, the env-file rationale, and the per-release flow.

## Tests / verification

- v0.1.0 .dmg subsequently uploaded to GitHub Releases via `gh release create v0.1.0` — Gatekeeper accepted cleanly, no "open anyway" prompt.
- 2 downloads of v0.1.0 .dmg (per release-day stats).

## Notes / decisions

- **Signing env lives outside the repo** so the credentials never touch git history. Even with `.gitignore`, the principle is "secrets don't live next to code."
- **App-specific Apple password** (not the user's main Apple ID password) — limits blast radius if leaked, can be revoked independently at appleid.apple.com.
- Notary turnaround is typically ~30 seconds for our binary size; flow blocks waiting.
- BUILD.md gains an "Updater keypair + manifest publishing (Phase 15)" section much later when in-app updates are added.
