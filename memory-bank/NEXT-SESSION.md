# NEXT-SESSION handoff — read this first

**Date written:** 2026-05-26 (v0.4.0 backend on branch); **top section updated 2026-05-31 (native experiment).**
**Session lead:** Claude Opus 4.7 [1m] (Claude Code in the terminal) with Michael

Read this first, then `activeContext.md`, then the latest entries in `progress.md`, then specific `tasks/2026-05/*.md` for full detail on what just happened.

---

## ⚗️ CURRENT BRANCH (2026-05-31): native Swift / Liquid Glass rebuild

> **You are likely on `experiment/native-swift-liquid-glass`, not `main`.** If so,
> the v0.4.0/v0.5.0 release content below describes `main` (the shipped Tauri app)
> and is background, not the active task.

A faithful **port** of the Tauri interface to native Swift 6 + SwiftUI + Liquid
Glass (macOS 26 Tahoe), in `native/` as a Swift Package. Port, not redesign — same
data sources + functionality as the shipped app. **Uncommitted; no commits past
`main`.**

**Done + building clean** (`cd native && ./build-app.sh debug` → 0 errors):
Dashboard (parity), package detail inspector (all 14 sections live), Settings
(9-tab stock `TabView`), data layer (6 services + `LocalPrefs`).

**Next, in priority order:**
1. **Library panel** — row→detail already wired; add kind pills, sort, filters.
2. Discover / Trending / Snapshots / Services / Activity panels (placeholders).
3. Dashboard GitHub "starred N of M" card (needs a batch resolver).
4. Sparkle for real in-app updates (deferred; auto-check toggle persists, install stub).
5. Vulns "scan all" from Settings (`VulnsService` has `scanOne` only).

**Build loop:** edit → `cd native && ./build-app.sh debug` → `killall BrewBrowser;
open native/BrewBrowser.app` → **user takes screenshots** (don't screencapture).
**Toolchain:** SPM only — full Xcode installed but `xcode-select` → CommandLineTools,
so no `xcodebuild`; `swift build` links Liquid Glass fine. **Constraint:** stock
Apple components only, no overrides.

**Docs:** `native/README.md` (build + source map), `decisions.md` (2026-05-30 ADR),
`techContext.md` ("Native rebuild" section), `tasks/2026-05/22-native-swift-liquid-glass-rebuild.md`,
`progress.md` (2026-05-31), and cross-session memory
`~/.claude/projects/-Users-michael-Software-brew-browser/memory/project-native-swift-rebuild.md`.

---

## v0.4.0 — PR open, awaiting merge + release cut (2026-05-26)

**Branch:** `feat/v0.4.0-velocity-and-history`. **All 9 steps done. PR is open against `main`.** Deploy live on `brew-browser.zerologic.com`. Cron runs first nightly snapshot Wed 03:00. **Workflow rule (durable): merges to main go through PRs, no direct pushes.**

### What's on the branch (committed + pushed)

- **Step 1–3** (backend, commit `3f576b8`): `Settings.enhanced_trending_enabled`, `require_enhanced_trending` gate, `FeatureDisabled` error variant, parallel `install` + `install-on-request` fetch, server-side velocity from 3-window join, history module (`trending_history_index` + `trending_history_fetch` IPCs), per-package LRU cache, path-traversal-safe URL builder.
- **Step 4–6** (frontend, commit `6711133`): `SettingsSectionTrendingHistory.svelte`, 6th `pathStatuses` entry, Trending tab restructure (velocity column + default sort + inline sparklines + 8-col responsive grid), `TrendingSparkline.svelte` shared SVG, `trendingHistory.svelte.ts` store, PackageDetail `trend-card` section.
- **Step 7** (collector, commit `6901b64`): `tools/trending-collector/` plain Node 20+ ESM with `better-sqlite3`.
- **Step 8** (docs, commit `e2598ef`): projectbrief nine → ten paths, decisions.md ADR, security.md §16 endpoint audit, techContext / backendApi §13.14 / frontendComponents updates, docs/release-notes/0.4.0.md, README disclosure.
- **Deploy-day fixes** (commits `bc7c176`, `cfb6115`, `84ad9ae`, `1ef98dc`): velocity formula bias fix (compare vs prior 11 months, not whole year), cask URL needs `homebrew-cask` segment + `cask:` field normalization, `Sites/` capitalization, Caddy log-filter syntax (`format filter { wrap json; fields { ... delete } }`).

Tests: **507 passing** (was 473 at v0.3.1). `npm run check` clean. `cargo build` clean.

### Production verification (already done, captured for audit trail)

- `https://brew-browser.zerologic.com/trending-history/index.json` → 200, expected headers, no Set-Cookie, no Server header
- `POST` → 405, `nonexistent.json` → 404
- `sudo grep -cE 'remote_ip|client_ip|X-Forwarded-For|X-Real-Ip' /var/log/caddy/brew-browser.log` → 0 (the auditable privacy artifact)
- Cron live (`0 3 * * *`), dry-run completed in 43s, pulled 101 new rows beyond seed
- Real leaderboard top: `hermes-agent` (v=1372), `raullenchai/rapid-mlx` (v=159), `grafana/gcx` (v=140), `openssl@4` (v=129) — genuine adoption signal

### What to do on next-session pickup

1. **Check PR status.** `gh pr list` for the open PR against main. Address any review feedback.
2. **Merge.** Squash or merge-commit — both fine.
3. **Cut v0.4.0 release** (same flow as v0.3.1):
   ```sh
   # Version bump first — Cargo.toml + Cargo.lock + tauri.conf.json + landing/index.html
   tools/build/sign-and-notarize.sh
   tools/release/publish-manifest.sh 0.4.0
   gh release create v0.4.0 \
     --title "v0.4.0 — Trending Velocity + Opt-in History Endpoint" \
     --notes-file docs/release-notes/0.4.0.md \
     bundle/macos/brew-browser_0.4.0_aarch64.dmg \
     bundle/macos/brew-browser.app.tar.gz \
     bundle/macos/brew-browser.app.tar.gz.sig
   # rename the asset to versioned name (gh release create #newname only sets label):
   ASSET_ID=$(gh api repos/msitarzewski/brew-browser/releases/tags/v0.4.0 --jq '.assets[] | select(.name=="brew-browser.app.tar.gz") | .id')
   gh api -X PATCH /repos/msitarzewski/brew-browser/releases/assets/$ASSET_ID -f name=brew-browser_0.4.0_aarch64.app.tar.gz
   # rsync manifest:
   rsync -av updater.json umacbookpro:Sites/brew-browser/updater.json
   ```
4. **Verify auto-update path** from a v0.3.1 install: Settings → Network → Updates → "Check for updates now" should surface v0.4.0; Install should succeed with `Cache-Control: public, max-age=...` on the manifest fetch.

Tauri release-pipeline gotchas reference (six things that bit during v0.3.0/v0.3.1): `~/.claude/projects/-Users-michael-Clean/memory/tauri_release_pipeline_gotchas.md`.

### v0.3.x polish queue (still valid — batch into v0.4.1 if there's enough)

These don't depend on v0.4.0 work; pick up whenever.

- **Donut hover center-text overflow** on long category labels.
- **Stale "Paranoid mode is on" toast** — fixed in v0.4.0 Step 4 (`brewErrorMessage` now reads "Offline Mode is on — …"). Can remove from this queue.
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
