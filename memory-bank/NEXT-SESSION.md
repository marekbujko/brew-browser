# NEXT-SESSION handoff — read this first

**Date written:** 2026-05-26 (v0.4.0 backend on branch)
**Session lead:** Claude Opus 4.7 [1m] (Claude Code in the terminal) with Michael

Read this first, then `activeContext.md`, then the latest entries in `progress.md`, then specific `tasks/2026-05/*.md` for full detail on what just happened.

---

## v0.4.0 backend on branch (2026-05-26)

**Branch:** `feat/v0.4.0-velocity-and-history` (off `main` at `d6d28a0`). Working tree has the full backend (Steps 1–3 of the 9-step plan); frontend, umbp collector, docs polish, and Caddy hardening still ahead. **Workflow change as of this branch: merges to `main` go through PRs from here on — no more direct pushes.**

### Backend done

- **Step 1**: `Settings.enhanced_trending_enabled` (off by default), `state::require_enhanced_trending()` gate composing master paranoid with per-feature toggle, new `BrewError::FeatureDisabled` variant.
- **Step 2**: parallel `install` + `install-on-request` fetch via `tokio::join!`, server-side velocity from cached 3-window join via `tokio::task::JoinSet`, `TrendingEntry` extended with optional IOR + `velocity_index` fields.
- **Step 3**: new `trending::history::{mod, client, cask}` module, two IPCs (`trending_history_index`, `trending_history_fetch`), per-package LRU cache (cap 500, TTL 6h), path-traversal-safe URL builder.
- Tests: **473 → 506** (+33 new). `cargo build` clean, zero dead-code warnings.
- Full file:line detail in `tasks/2026-05/19-v0.4.0-backend.md`. Architectural decisions D1–D5 locked in that file.

### What to do on next-session pickup

1. **Read `tasks/2026-05/19-v0.4.0-backend.md` first** — it has every file:line, every test name, and every locked decision.
2. **Continue with Step 4** (frontend Settings UI). Reuse pattern from `SettingsSectionUpdates.svelte`. New `SettingsSectionTrendingHistory.svelte`. Add 6th entry to `pathStatuses` in `SettingsSectionNetwork.svelte`.
3. Then Step 5 (Trending tab — velocity column + sort + inline sparklines), Step 6 (PackageDetail sparkline), Step 7 (umbp collector — new `tools/trending-collector/`), Step 8 (memory-bank + docs), Step 9 (Caddy privacy hardening).
4. When ready to merge: `gh pr create` from the branch into `main`. Don't push to `main` directly.

### v0.3.x polish queue (still valid — batch into v0.3.2 or fold into v0.4.0 finishing pass)

(Same list as before — these don't depend on v0.4.0 backend, can be picked up whenever.)

- **Donut hover center-text overflow** on long category labels.
- **Stale "Paranoid mode is on" toast** wording — one-line fix in `src/lib/types.ts`.
- **localStorage flag gating eager `loadStatus()`** in TitlebarControls.
- **`cancelSignin` timer leak** — rare edge case.
- **Startup placeholder-pubkey guard** — 5-line panic-on-PLACEHOLDER in release builds.
- **Persist `last_checked_at` to disk** — auto-updater 24h floor across launches.
- **Activity → data-dir migration** — bigger persistence cleanup.

---

## v0.3.1 shipped (2026-05-26)

Same-day cumulative point release on top of v0.3.0. Highlights: magic `local_search` (catalog + AI summary + friendlyName + category labels + tags in one weighted union scan), curated Upgrade modal, Refresh-actually-runs-brew-update-too, unified Description + Version columns across Library / Discover / Trending, donut hover-with-counts, Report-to-brew-browser button on every error toast + the Activity drawer failed-job footer, Activity persistence hardening, bundle id rename (`dev.openbrew.browser` → `com.zerologic.brew-browser`), root → docs/ reorganization.

Released as `v0.3.1` tag + GH release with both `.dmg` (fresh install) and `.app.tar.gz` (auto-updater) assets. Manifest deployed to `umacbookpro:Sites/brew-browser/updater.json`. Asset rename via `gh api PATCH` (the `gh release create #newname` syntax only sets the label, not the filename). Full release write-up at `memory-bank/tasks/2026-05/18-v0.3.1-release.md` + `docs/release-notes/0.3.1.md`.

### v0.3.x polish queue (small wins, batch into v0.3.2 when there's enough)

- **Donut hover center-text overflow** on long category labels ("Graphics & Design", "System Utilities", "AI & ML"). Wraps or overflows the inner ring on narrow windows. ~30min — auto-shrink / ellipsis / multi-line / abbreviated labels TBD.
- **`weight::*` dead-code warning** in `local_search` — false-positive analyzer quirk on closures inside async fn. Currently silenced with `#[allow(dead_code)]`; revisit when the analyzer improves.
- **Stale "Paranoid mode is on" toast** wording still slips through `brewErrorMessage(e)` for `paranoid_mode_blocked` — central default missed during the Phase 15 rename sweep. One-line fix in `src/lib/types.ts`.
- **localStorage flag gating eager `loadStatus()` in TitlebarControls** — preserves v0.2.1's "zero Keychain prompt for non-signed-in users" promise. Currently re-introduces a prompt on every fresh-signature launch. ~15min.
- **`cancelSignin` timer leak** — `setTimeout(cancelSignin, 1500)` in `signIn()` doesn't get cleared if the user clicks Cancel before 1500ms. Rare edge case; ~10min.
- **Startup placeholder-pubkey guard** — panic-in-release-build if `UPDATER_PUBKEY.contains("PLACEHOLDER")`. Catches the next maintainer skipping the pubkey-replacement step. ~5min.
- **Persist `last_checked_at` to disk** so the auto-updater 24h floor honours typical morning-open/evening-close usage. Currently in-memory only; effectively never auto-fires across launches. ~30min.
- **Activity → data-dir migration** (the bigger persistence cleanup). Move from localStorage (WebKit-scoped, dev/prod split, ~5-10MB quota) to `~/Library/Application Support/brew-browser/activity.json` via two new Tauri commands. ~45min.

### v0.4.0 candidates (bigger features)

- **`installedAt` on Package + Last-Updated sort** (1-2h, on deferred list since Phase 9).
- **Octocat chip tooltip differentiation** — currently `notifications`-missing and `public_repo`-missing both render amber with the same tooltip. ~30min.
- **Tier B enrichment run** (~$15 Anthropic API). Unlocks "Why install this?", "Similar packages", "Tags" sections in PackageDetail.
- **Recipes feature** (Phase 10, needs Tier B). Curated 3-5 package combos by use case.
- **`brew tap msitarzewski/brew-browser`** for one-line `brew install --cask brew-browser`. Post-launch convenience.
- **Tahoe Liquid Glass via Swift FFI** — Tauri 2 doesn't expose this directly. Polish.

---

## Historical state (pre-v0.3.1)

- **v0.3.0** is live. Signed + notarized + stapled `.dmg` AND signed `.app.tar.gz` (the auto-updater artifact) both on the GH release: <https://github.com/msitarzewski/brew-browser/releases/tag/v0.3.0>.
- **Manifest live** at <https://brew-browser.zerologic.com/updater.json>. Served by Caddy on umbp. Auto-updater path validated end-to-end with real minisign keypair.
- **Issue #1** (Joshua Butner / @heyjawrsh) closed. Posted a thanks comment.
- **Stars:** 18 as of 2026-05-25 (doubled since v0.2.1 ship; LinkedIn announcement still circulating, v0.3.0 release likely contributing).
- **Working tree clean.** Three commits on `main` this two-session arc: `1bfe21f` (Phase 15 fix-up), `820c1f0` (GitHub coverage expansion), `d7c2bca` (v0.3.0 release).

## What landed in v0.3.0

See `docs/release-notes/0.3.0.md` for the user-facing version. The technical narrative:

- Phase 15 (in-app updater + Offline Mode rename) — 4-agent parallel implementation + 5-CRITICAL fix-up + signed + shipped.
- GitHub coverage expansion — `Package::github_homepage` walks homepage → urls.stable.url → urls.head.url (formula) or homepage → url (cask). Frontend uses pre-resolved field for all GitHub feature routing.
- Issue #1 fixes — cache loop in `PackageDetail` + structural misuse of `$effect` for one-shot side effects, plus the per-action scope gate + actionable Re-authorize toast + GitHub Octocat status chip. All in the v0.3.0 ship.
- Backend test count: 411 → 473 over the v0.3.0 cycle.

## Release-pipeline lessons (in case you ship v0.4.0)

Six things bit us during the v0.3.0 release that won't on the next one:

1. **`TAURI_SIGNING_PRIVATE_KEY` requires inline contents.** The path variant (`_PATH`) doesn't work in Tauri 2 CLI as of 2026-05 despite the signer-generate output claiming it does. `tools/build/sign-and-notarize.sh` now bridges automatically: if only `_PATH` is set, it reads the file and exports the contents into `TAURI_SIGNING_PRIVATE_KEY`.
2. **Password env vars need single quotes** in `signing.env` when the password contains `$`, `!`, backticks, or anything else bash expands inside double quotes. Use `'...'` literal strings.
3. **`createUpdaterArtifacts: true`** is required in `tauri.conf.json` (`bundle` section) for Tauri 2 to emit `.app.tar.gz` alongside `.dmg`. Default is off; without it the updater target silently skips.
4. **`.app.tar.gz.sig` is Tauri-format, not raw minisign-format** (`.minisig`). The plugin reads the .sig contents as a single string and validates against the embedded pubkey. Don't re-sign via the `minisign` CLI — that produces the wrong format. `publish-manifest.sh` now reads the bundler-produced .sig directly.
5. **`gh release create asset.ext#new-name` only sets the label**, not the filename. To rename the downloadable asset, use the API:
   ```sh
   gh api -X PATCH /repos/<owner>/<repo>/releases/assets/<id> -f name=<new-name>
   ```
6. **CDN cache settles within ~10s** after rename. Initial 404 on renamed asset is benign; retry after a pause.

## Critical context for any future release

- **Apple signing env** at `~/.config/brew-browser/signing.env` (chmod 600, outside repo) — valid + live
- **Minisign private key** at `~/.config/brew-browser/updater.key` (chmod 600, outside repo) — valid, single-password-protected. Public counterpart at `.key.pub`. Embedded pubkey in `lib.rs` + `tauri.conf.json` matches.
  - ⚠️ **If you lose this key**, users on the auto-update path can no longer verify a new release's signature against the v0.3.0+ embedded pubkey. Recovery means cutting a release with a new embedded pubkey and asking existing users to manually re-download from the releases page. Back it up.
- **Anthropic API key** in `tools/categorize/.env` (gitignored) — valid + live
- **GitHub OAuth client_id** (`Ov23liJZKbvrSBuiOPkT`) — public per RFC 8628 §3.1
- **GitHub OAuth scope list** in `src-tauri/src/github/auth.rs:96` — `["read:user", "public_repo", "notifications"]`. v0.2.1 users with 2-scope tokens hit the actionable Re-authorize toast on Watch attempts and are guided through an incremental scope grant.

## What's queued for next session (priority order)

### 1. Optional polish for v0.3.x (small wins)

- **Localstorage flag for eager `loadStatus()`** — gate `TitlebarControls.onMount` on `localStorage["brew-browser:has-signed-in"]` so users who never sign in see zero Keychain prompts. Set on signIn success; clear on signOut. ~15min.
- **`cancelSignin` timer leak** — track the `setTimeout(cancelSignin, 1500)` timer id in `signIn()` and clear on `cancelSignin()`. ~10min.
- **Startup placeholder-pubkey guard** — panic-in-release-build if `UPDATER_PUBKEY.contains("PLACEHOLDER")`. Catches the next maintainer skipping the pubkey-replacement step. ~5min.
- **Persist `last_checked_at` to disk** so the auto-updater 24h floor honours typical "open in morning, close at night" usage patterns. The scheduler currently uses in-memory state only. ~30min.

### 2. v0.4.0 ideas (from the user's running ideas list)

See `memory-bank/ideas.md` for the full backlog. Top candidates:
- Recipes (multi-package install bundles with names)
- Discover-UI surface improvements
- macOS Liquid Glass / further NSVisualEffectView treatments
- Octocat chip tooltip differentiation ("missing notifications scope" vs "missing public_repo" — currently both render amber with same tooltip)

### 3. v0.3.x point release if Phase 15 issues surface in the wild

The in-app updater is fresh. Possible early-feedback issues:
- Slow networks: install progress feedback (currently no Channel-based progress events)
- Multi-user macOS: keychain ACL might re-prompt across users
- Non-standard install paths (custom `/Applications`): unhandled

Watch for issues from v0.3.0 users; have a v0.3.1 hotfix flow ready.

## Credentials / paths reference

| What | Where |
|------|-------|
| Repo on disk | `/Users/michael/Clean/brew-browser/` |
| GitHub repo | `github.com/msitarzewski/brew-browser` |
| Anthropic API key | `tools/categorize/.env` |
| Apple signing env | `~/.config/brew-browser/signing.env` (chmod 600) — now also contains `TAURI_SIGNING_PRIVATE_KEY` + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` |
| Updater minisign key | `~/.config/brew-browser/updater.key` (chmod 600) — keep secret + back up |
| Updater minisign pubkey | `~/.config/brew-browser/updater.key.pub` (matches embedded `UPDATER_PUBKEY` in `lib.rs`) |
| Landing source | `landing/` |
| Landing deploy | `michael@umacbookpro:Sites/brew-browser/` |
| Updater manifest | `dist/updater.json` (gitignored, emitted by `publish-manifest.sh`) → rsynced to `umacbookpro:Sites/brew-browser/updater.json` |
| umbp Tailnet IP / hostname | `100.98.187.7` / `umacbookpro` |
| Catalog data | `src-tauri/data/catalog/{formula,cask}.json.gz` + `manifest.json` |
| Enrichment data | `src-tauri/data/enrichment.json.gz` (15,725 Tier A entries) |
| Catalog refresh | `python tools/catalog/fetch.py` |
| Enrichment refresh | `tools/categorize/.venv/bin/python3 tools/enrich/enrich.py --tier-a` |
| Runtime caches | `~/Library/Application Support/brew-browser/` |
| Keychain | service `com.zerologic.brew-browser` (renamed in v0.3.1; was `dev.openbrew.browser` through v0.3.0), accounts `github_access_token` + `_scopes` + `github_access_token_scopes` |
| Icon source | `docs/icon/brew-browser.svg` (full-bleed square, Tahoe-clean) |
| Icon regen | `npm run tauri icon docs/icon/brew-browser.svg` |
| Release notes | `docs/release-notes/<version>.md` (convention started in v0.3.0) |
| Memory bank task records | `memory-bank/tasks/2026-05/*.md` (17 + README + deferred) |
| Memory bank phase plans | `memory-bank/phases/{phase12,phase13}-plan.md` (shipped); `memory-bank/phase15-plan.md` (top level, shipped) |

## Notes from the v0.3.0 release session

- **First release with a working auto-updater path.** Validated end-to-end with the real minisign keypair. Future releases follow `tools/build/sign-and-notarize.sh` → `tools/release/publish-manifest.sh 0.X.0` → `gh release create v0.X.0 --notes-file docs/release-notes/0.X.0.md` → rsync manifest → `gh api PATCH /releases/assets/.../name`.
- **Release notes live under `docs/release-notes/<version>.md`** going forward, not at repo root. Per user request mid-release.
- **`gh release create #newname` syntax does NOT rename the asset.** Bit us during the v0.3.0 release; the rename was a post-hoc API PATCH. Document this in the maintainer-side BUILD.md for v0.4.0 prep.
- **Memory bank refresh** is the final step of any release — commit + push the v0.3.0 release commit, THEN refresh activeContext + progress + NEXT-SESSION as a separate follow-up commit. Keeps the release commit narrative clean.

## What is NOT a problem (calling out so next-session-Claude doesn't re-investigate)

- ✅ Auto-updater path — fully working v0.3.0 → v0.3.x
- ✅ Manifest serving — Caddy on umbp, 200 with correct cache headers
- ✅ Toast cascade / Star / Watch / File-issue / Sign-in flow — all clean
- ✅ Build pipeline — `tools/build/sign-and-notarize.sh` handles all env-var quirks
- ✅ Tests + clippy + check + build — all green at 473 backend tests
- ✅ Pubkey + key — generated, matched, embedded, backed up (by you, hopefully)
