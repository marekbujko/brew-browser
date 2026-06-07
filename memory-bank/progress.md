# Progress

## 2026-06-07 — launch batch (both builds): firehose fix + #58 + #57 + native tests + security pass

- ✅ **Upgrade-all firehose fixed** — `brew upgrade` exits 1 on non-fatal warnings; both builds now classify those as success (no "Upgrade-all failed" toast / report CTA). Closes ~20 issues. `upgrade_warnings_only` (Rust) / `BrewOutputParsing` (Swift), tested.
- ✅ **#58** category click filters the Library (was jumping to Discover) — both builds.
- ✅ **#57** operation progress counts ("Pouring N of M") from brew `==>` markers — both builds.
- ✅ **Native test target** (first ever): 0 → 36 tests (Swift Testing) mirroring the Rust fixtures + fuzz. Rust gained matching fuzz tests.
- ✅ **Pre-release security pass** — cargo audit 0 vulns, semgrep 0, gitleaks clean (false positives suppressed). Manual injection/path/CSP/signature/token review all pass. `audit.toml` documents Linux-only unmaintained deps. See `security.md` §19 + `tasks/2026-06/11-*`.
- ✅ **Docs + landing for dual-build** — README/SECURITY/CONTRIBUTING now cover both Tauri + native builds (+ Hall of fame for @neodave / #46); new dashboard screenshots; **landing page published live** (rsync, no `--delete`, updater.json verified intact); `landing/README.md` `--delete` footgun fixed.
- On branch `feat/launch-batch-progress-category-upgrade` (off main). Runtime/MITM tests deferred (hands-on).

## 2026-06-03 — native: live enrichment + task notifications (experiment)

- ✅ Native **live category/description updates** (opt-in) — `EnrichmentLiveService` mirrors `TrendingHistoryService`; overlay on bundled; toggle + disclosure. Parity with Tauri PR #43.
- ✅ Native **macOS task-completion notifications** (opt-in, background-only) — `NotificationService` + `LocalPrefs.notifyOnTaskCompletion` + Settings toggle; foreground still uses the Activity drawer.
- ✅ `swift build` clean. See `tasks/2026-06/01-native-live-enrichment-notifications.md` + memory `project-live-enrichment`.
- Native reverse-parity remaining: **Services panel** (last big one).

## 2026-06-02 — Tauri←native parity (branch `tauri-parity`)

- ✅ Shared `PackageRowIcon.svelte` (Tauri analog of native `PackageIcon`); list icons added to Discover + Trending (had none), Library routed through it.
- ✅ Detail panel: centered 64px app icon.
- ✅ Dashboard: Composition pie + Top-categories side-by-side on wide panes; chips in card header; equal-height cards; charts matched to native specs (outer radius 60, donut inner ratio 0.6, filled `<path>` pie, vertical centering).
- ✅ Keychain: one batched `SecItemCopyMatching` (one prompt) via `security-framework`, mirroring native `keychainReadAll`.
- ✅ Trending velocity badge: canonical banded threshold (≥1.5 🔥 / ≤0.7 ❄️ / neutral).
- ✅ `npm run check` 0 errors; Rust compiles; verified via screenshots. Committed + pushed.
- ↩️ Reverse-parity (native←Tauri) tracked in memory `project-native-reverse-parity`: legend icons, banded velocity, Snapshots/Services panels.

See `tasks/2026-06/01-tauri-native-parity.md` + `decisions.md#2026-06-02`.

## 2026-05-24 (overnight)

### Done since last sync

- ✅ `git init` + first commit (`653e26f`) — initial release, 186 files
- ✅ `gh repo create msitarzewski/brew-browser --public --push` — repo live on GitHub
- ✅ Bulk categorize run completed against Claude Haiku 4.5 — 15,974 items, $1.50, 19 min
- ✅ Second commit (`c72e31d`) — categories.json (838 KB) + landing page in-repo
- ✅ Third commit (`2dad9be`) — Caddyfile snippet removed (user handles Caddy config manually)
- ✅ Landing page rsync'd to `michael@100.98.187.7:Sites/brew-browser/` on umbp
- ✅ Full SEO/social treatment added to landing: OG, Twitter/X cards, JSON-LD SoftwareApplication, PWA manifest, robots.txt, sitemap.xml, 1200×630 social card
- ✅ Social card iterated through multiple designs based on user feedback
- ✅ `ideas.md` captures: Recipes, optional GitHub OAuth, Liquid Glass / NSVisualEffectView discussion, Discover-UI surface ideas

### Phases

| Phase | Status |
|-------|--------|
| 0 — Scaffold | ✅ |
| 1 — Read-only Homebrew browser | ✅ |
| 2 — Search Homebrew index | ✅ (categories UI pending) |
| 3 — Install/uninstall/upgrade w/ streaming | ✅ |
| 4 — Brewfile snapshot/restore | ✅ (NB: known upstream brew bundle bug surfaced via friendly error mapping) |
| 5 — Polish + build artifact | ✅ (unsigned .dmg; signing pending cert install) |
| 6 — Trending tab | ✅ |
| 7 — Cask icons installed | ✅ |
| 8 — Cask icons homepage cascade | ✅ |
| Security — audit + fixes + tool battery + re-audit | ✅ READY-FOR-SCRUTINY |
| Reframe pass | ✅ counter-narrative dropped from all docs |
| Categorize tool + bulk run | ✅ 15,974 items via Claude Haiku 4.5 |
| Landing page + SEO/social | ✅ deployed to umbp |
| **v0.1.0 GitHub release** | ✅ SHIPPED — signed/notarized .dmg attached at <https://github.com/msitarzewski/brew-browser/releases/tag/v0.1.0> |
| **Phase 9a — Discover category tile UI** | ✅ tile grid + filtered view + Lucide icons, uncommitted |
| **Phase 9b — Category linking pass** | ✅ multi-select chip filter (Discover + Library), category pills on PackageDetail, sortable columns (Library + Trending), fixed dangling `installed` pill, uncommitted |
| **Phase 11 — Dashboard** | ✅ hero/updates/composition/donut/storage cards; brand → home; updates card → outdated library; uncommitted |
| **Phase 11b — Services** | ✅ sidebar item ⌘5, page with start/stop/restart, per-package detail card, sidebar badge for running count, uncommitted |
| **Phase 11c — Native macOS feel** | ✅ vibrancy + drag regions (data-tauri-drag-region + capability), traffic-light-aware sidebar, uncommitted |
| **Phase 11d — Activity persistence** | ✅ localStorage mirror, cap 50 jobs / 500 lines, hydrate on bootstrap, uncommitted |
| **Phase 12a — Bundled catalog + manual refresh** | ✅ |
| **Phase 12b — Settings shell** | ✅ |
| **Phase 12c — GitHub anonymous tier** | ✅ (combined with 12e in one Backend Architect pass) |
| **Phase 12d — Settings: network + paranoid + settings persistence** | ✅ |
| **Phase 12e — GitHub Device Flow OAuth + Keychain** | ✅ (combined with 12c) |
| **Phase 12f — GitHub authed actions** | next (after Wave 1+2 commit) |
| **Phase 13 — Catalog enrichment (Haiku)** | queued, parallel-OK with 12f |
| **Phase 14 — bundled cask icons** | DROPPED (trademark/redistribution risk) |
| **Phase 9c — "Wrong?" GitHub-issue link** | folds into 12f |
| **Phase 9d — `installedAt` on Package + Last-Updated sort** | small standalone, not in any phase |
| **Phase 10 — Recipes** | deferred — catalog now available so unblocked |

### Phase 9b notes

- New store: `src/lib/stores/discover.svelte.ts` — multi-select `selectedCategories: Set<string>`, shared by Discover + Library + PackageDetail. `selectOnly(slug)` for tile-click semantics, `toggle(slug)` for chip add/remove.
- Discover.svelte: replaces local single-`activeCategory` with the shared store; tile click → adds single chip; chip bar above results with per-chip X + Clear button; search results filter to OR-match selected chips; chip-only browse mode (no query, chips set) lists union sorted alphabetically.
- Fixed UX bug from the user's screenshot: `installed` pill no longer floats. Two row layouts: `.row--with-desc` (1fr 80px 2fr auto) for search; `.row--no-desc` (1fr 80px auto) for chip-filtered browse.
- PackageDetail: new "Categories" meta row with clickable pills. Click jumps to Discover with that single category selected (closes detail panel so user lands on the filtered list, not an obscured view).
- New component: `src/lib/components/SortableHeader.svelte` — small reusable header button with up/down arrow indicator, click toggles direction or switches column. Uses `aria-label` (not `aria-sort`, since that requires `role="columnheader"` and our list-grids aren't true tables).
- Library: sortable Name / Version / Type / Outdated; shares the Discover category chips so the user can keep context across tabs; updated empty-state messaging to reflect chip vs. text filters separately.
- Trending: sortable # / Name / Type / Installs. Installs defaults to descending on first click.
- Lint/test: `npm run check` 0 errors / 1 pre-existing warning. `npm run build` clean in 1.64s. Backend untouched this pass — no Rust regression risk.
- Status: code is in working tree, NOT committed. Awaiting user UX confirmation.

### Phase 11 notes (Dashboard + Services + native feel + persistence)

Single big session 2026-05-24-night. Highlights:

- **Dashboard.svelte** is the new default landing. Hero row (installed / outdated / brew version), Updates panel with one-click upgrade-all (and the title is a clickable link → Library outdated filter), Composition split bar with on-request/dep/pinned meta, Top-Categories donut (180px SVG, 9-color palette, top 8 + Other, click legend → Discover with chip pre-selected), Storage card with 4 paths and Open-in-Finder per row.
- **Donut math:** `stroke-dasharray="(pct/100)*C C"` + `stroke-dashoffset="-(startPct/100)*C"` + `rotate(-90)` for top start. Center text shows total installed.
- **Services backend** (`commands/services.rs`): 5 commands (list, clear-cache, start, stop, restart), 5s list cache, write-lock around state mutations, alphanumeric+symbol name validation.
- **Services frontend:** sidebar item ⌘5, sortable Name/Status/User columns, per-row action buttons (smart-disabled by current state), badge = count of running services. PackageDetail shows a Service card with pill + 3 buttons when the formula has a brew services entry.
- **Disk usage backend** (`commands/disk_usage.rs`): `disk_usage` + `open_in_finder`, 4 paths surveyed in parallel via `tokio::join!`, 60s cache, security gate on Finder reveal (must be inside Homebrew prefix/cache).
- **Native macOS feel:** vibrancy via `window-vibrancy = "0.6"` + `apply_vibrancy(NSVisualEffectMaterial::HudWindow, …)`; tauri.conf.json `transparent: true` + `titleBarStyle: "Overlay"` + `hiddenTitle: true`; sidebar brand padded to clear traffic lights; `data-tauri-drag-region` on brand-wrap + every panel-head with the new `core:window:allow-start-dragging` capability.
- **Activity persistence:** localStorage mirror `brew-browser:activity:v1`, cap 50 jobs / 500 lines per job, debounced 400ms writes + immediate flush on terminal events, hydrate from +layout mount.
- **Sortable lists hardening:** `1fr` → `minmax(0, 1fr)` everywhere a flex column had text (Discover, Library header + PackageRow, Trending). Fixed cross-row pill alignment that depended on name length. Also `auto` → `90px` for the installed column so it doesn't collapse-and-shift the kind cell.
- **Trending Refresh fix:** the force flag now busts the backend cache before calling `trending_fetch` — was silently ignored before.
- **Dashboard scroll + drag bug fix:** removed the fixed-position drag-overlay (was eating scroll wheel + not actually triggering drag); fixed flex children getting shrunken to fit by adding `.body > * { flex-shrink: 0 }`.
- **Test count:** 207 → 210 (3 new for `services` name validation, 2 for `disk_usage` du_bytes, 1 for `categories` already counted last session = pre-session 204 + 6 = 210).

### Phase 9a notes

- Backend: `commands/categories.rs` — `categories_data` Tauri command, embeds JSON via `include_str!` (zero runtime file dep), parsed once + memoised on `AppState.categories_cache`. 1 new unit test (205 total, was 204).
- Frontend types: `CategoryMeta`, `CategoriesData` in `types.ts`.
- API wrapper: `categoriesData()` in `api.ts`.
- Store: `src/lib/stores/categories.svelte.ts` — lazy-load, derived `tiles` (sorted by count, uncategorized last), `tokensInCategory(slug)` for the filtered view, `categoriesOf(name, kind)` for future chip rendering.
- Icon resolver: `src/lib/util/categoryIcon.ts` — static map of 19 Lucide icons, falls back to `HelpCircle`.
- Discover.svelte: new tile grid (`auto-fill, minmax(180px, 1fr)`), clicking a tile drills into a filtered list, back button returns to grid. Search still wins when there's a query.
- Lint/test: cargo clippy `-D warnings` clean, cargo test 205 pass, `npm run check` 0 errors, `npm run build` clean.
- Status: code is in working tree, NOT committed. Awaiting user sign-off on UX before commit.

### Test + build status (current)

- `cargo test --manifest-path src-tauri/Cargo.toml`: **210 passed / 0 failed / 6 ignored** (up from 204)
- `cargo check`: clean
- `cargo clippy --all-targets -- -D warnings`: clean
- `npm run build`: clean
- `npm run check`: 0 errors (1 pre-existing tsconfig-node warning)
- `cargo deny check`: advisories ok, bans ok, licenses ok, sources ok (pre-session)
- `cargo tauri build`: produces signed/notarized 5.7 MB `.dmg` (v0.1.0 already shipped)

### Security posture

| Tool | Result |
|------|--------|
| Wave 1 audit findings | **16/16 verified fixed** (0C / 0H / 0M / 0L / 0N open) |
| `cargo audit` | 0 vulns |
| `cargo deny check` | advisories+bans+licenses+sources ok |
| `npm audit --omit=dev` | 0 vulns |
| `osv-scanner` | 19 advisories (all Linux-only or acknowledged) |
| `gitleaks` | 0 leaks in source |
| `semgrep` (security-audit + OWASP-10 + rust + typescript) | 0 findings |
| `unsafe` Rust in brew-browser | 0 |
| `@html` / `innerHTML` / `eval` in frontend | 0 |
| Tauri shell plugin | not used (IPC is the security boundary) |

### Open items

| Item | Blocker |
|------|---------|
| Apple Developer ID Application cert | User must install via developer.apple.com |
| Signed + notarized `.dmg` | Above |
| v0.1.0 GitHub release with `.dmg` attached | Above |
| Updated social card PNG saved to persistent path | User must drop file somewhere I can grab |
| Master icon swap to beer-mug variant (optional) | Decision pending |
| Phase 9 — Discover category UI build | Ready to start when user signals |

### Repo state

```
/Users/michael/Clean/brew-browser/  (15.9k+ packages categorized, 2 production commits + this sync pending)
├── LICENSE                           MIT
├── README.md                         polished, security section, 4-path network disclosure
├── CONTRIBUTING.md                   141 lines
├── SECURITY.md                       responsible disclosure
├── PLAN.md                           phase tracker
├── .gitignore                        comprehensive (target/, node_modules/, .env, etc.)
├── package.json                      brew-browser, MIT
├── src/                              36+ files
├── src-tauri/
│   ├── src/                          22 Rust files (modular)
│   ├── Cargo.toml                    8 deps
│   ├── deny.toml                     permissive-license allowlist
│   ├── data/categories.json          838 KB — 7,607 casks + 8,367 formulae from Haiku 4.5
│   ├── icons/                        38 minted platform icons
│   ├── tests/                        integration + 10 real-brew fixtures
│   └── target/release/bundle/dmg/    brew-browser_0.1.0_aarch64.dmg (6.1 MB, unsigned)
├── tools/categorize/                 offline LLM-driven category tool
│   ├── categorize.py                 main script
│   ├── prompts/system.txt            calibration prompt
│   ├── .env.example                  template (real .env gitignored)
│   ├── state/last-tokens.json        diff state (15,974 tokens recorded)
│   └── README.md                     setup + cron docs
├── landing/                          static landing page
│   ├── index.html                    full OG/Twitter/JSON-LD/PWA treatment
│   ├── style.css                     OKLCH tokens, dark-first
│   ├── brew-browser.svg              icon copy
│   ├── manifest.json                 PWA
│   ├── robots.txt + sitemap.xml      SEO basics
│   ├── social-card.png / .svg        1200×630
│   └── README.md                     deploy via rsync to umbp
├── docs/icon/                        master SVG + size previews
└── memory-bank/                      20 files (this dir)
```

## 2026-05-24 (late session — Phase 12 Wave 1 + Wave 2)

### Done

- ✅ Commit `84ad010` pushed (Phase 9 + 11 + memory bank refresh)
- ✅ Phase 12 plan written: `memory-bank/phase12-plan.md`
- ✅ Pre-implementation Security Engineer review: `memory-bank/scans/phase12-security-review.md` — APPROVED with explicit gates
- ✅ **Phase 12a** (Backend Architect agent) — bundled catalog + manual refresh, +38 tests
- ✅ **Phase 12b** (Frontend Developer agent) — Settings shell + 6 sections + brew analytics, +8 tests
- ✅ **Phase 12d** (Backend Architect agent) — paranoid mode + settings persistence + Network section, +18 tests
- ✅ **Phase 12c + 12e** combined (Backend Architect agent) — GitHub anonymous tier + Device Flow + Keychain, +60 tests
- ✅ Phase 13 plan written: `memory-bank/phase13-plan.md` — Tier A friendly names+summaries, Tier B use cases+similar+tags, AI Features master toggle, ~$20 cost, zero runtime LLM calls
- ✅ Phase 14 (bundled cask icons) **explicitly DROPPED** — trademark/redistribution risk, runtime probe + paranoid gate is enough

### Phases (updated)

| Phase | Status |
|-------|--------|
| **Phase 12a — Bundled catalog + manual refresh** | ✅ |
| **Phase 12b — Settings shell + brew analytics** | ✅ |
| **Phase 12c — GitHub anonymous repo stats** | ✅ (combined with 12e) |
| **Phase 12d — Paranoid + network settings + settings persistence** | ✅ |
| **Phase 12e — Device Flow OAuth + Keychain** | ✅ (combined with 12c) |
| **Phase 12f — GitHub authed actions** | next |
| **Phase 13 — Catalog enrichment** | queued, can run parallel with 12f |
| **Phase 14 — bundled cask icons** | DROPPED (trademark risk) |
| **Phase 10 — Recipes** | deferred — depends on catalog (now available) |
| **Phase 9d — installedAt + Last-Updated sort** | small standalone, not blocking |

### Test + lint status (current)

- `cargo test`: **334 passed / 0 failed / 6 ignored** (was 274 at start of Wave 2; 210 at start of session)
- `cargo clippy --all-targets -- -D warnings`: clean
- `npm run check`: 0 errors, 1 pre-existing tsconfig-node warning
- `npm run build`: clean

### Phase 12 Wave 2 notes

**Wave 2 ordering (Option A — Foundation-first):** 12d → combined 12c+12e → 12f. Locked in by user. Reasoning: 12d delivers `require_network` helper + settings persistence; 12c+12e then consume the helper directly (no TODOs); combined as one Backend Architect pass because 12c and 12e both touch the same `src-tauri/src/github/` module.

**Phase 12a key deviations from spec (accepted):**
- `CatalogRefreshInProgress` returned as generic `InvalidArgument` until 12d added the proper variant — agent left a TODO grep-marker
- Bundled gzipped catalog is 6.1 MiB not "~3 MiB" estimate (catalog grew upstream)
- `fetch.py` doesn't strip unused JSON fields (deferred — would shrink to ~1 MiB at cost of build-time coupling to Rust struct shape)

**Phase 12b key deviations (accepted):**
- `commands/mod.rs` alphabetical position is true-alphabetical (brew_env < brewfile), not literal-spec position
- Lucide has no `github` icon (trademark) → `git-fork` substituted for the GitHub section
- Settings modal sized `220px nav + 1fr content` not `350px + 600px` (looked awkward at macOS density)
- `brew_get_analytics` parser accepts both trailing-period and non-period forms (empirically brew has shipped both)
- Activity caps wired to Settings but not yet consumed by activity store (deferred — value persists, no retroactive trim)

**Phase 12d key deviations (accepted, more conservative than spec):**
- Unknown enum variant → file treated as Corrupt → fail closed (instead of "log + substitute default"). Aligned with §12d "fail closed when corrupt" rule
- `require_network` gates Trending even on cache hits (UX consistency over micro-savings)
- Catalog stale banner threshold + cask icon mode NOT retroactively wired to consume the new settings — store is ready, consumers swap as a 1-line change later

**Phase 12c+12e key deviations (accepted):**
- Cache TTL backdating test rewritten as constant-pin + fresh-read positive test (filetime isn't a dep)
- CSP comment moved to Rust module docs (tauri-build rejects unknown JSON fields like `_comment_csp`)
- Custom `KeychainSlot` trait + in-memory mock instead of `keyring` crate's mock feature (same coverage, no runtime context switch)
- Username resolution failure is non-fatal during sign-in (token still stored; username shows "github user" until next sign-in)
- `AuthRequired` + `ScopeRequired` error variants land but `#[allow(dead_code)]` until 12f consumes them

### Files staged (47 changes, ready to commit)

Backend: `Cargo.toml`, `Cargo.lock`, `tauri.conf.json`, `capabilities/default.json`, `src/{catalog,github,util}/`, `src/commands/{catalog,brew_env,disk_usage,github,services,settings}.rs`, `src/commands/{mod,trending,cask_icon_homepage}.rs` (paranoid gate wiring), `src/{error,lib,state}.rs`

Frontend: `src/{app.css, +layout, +page}.svelte`, `src/lib/{api,types}.ts`, `src/lib/components/{Dashboard, Discover, Library, PackageDetail, PackageRow, Services, Settings, SettingsSection*, DeviceFlowModal, Sidebar, Snapshots, Trending, ActivityHistory, SortableHeader}.svelte`, `src/lib/stores/{activity, categories, discover, github, library, services, settings, trending, ui}.svelte.ts`, `src/lib/util/categoryIcon.ts`

Docs: README.md (Open by default → 7 paths + Paranoid Mode), BUILD.md (GitHub OAuth section), memory bank updates

Data: `src-tauri/data/catalog/{formula,cask}.json.gz` + `manifest.json`

Tools: `tools/catalog/{fetch.py,README.md}`

Misc untracked: `PHILOSOPHY.md` (user-authored, 271 lines)

## 2026-05-24 (evening — Phase 12+13 wrap)

### Done

- ✅ Commit `99a1f2c` pushed — Phase 12 Wave 1+2 (catalog 12a, settings 12b, paranoid 12d, GitHub anonymous + Device Flow 12c+12e). 47 files. Test count 210 → 334.
- ✅ Commit `8b89c40` pushed — Phase 12f (GitHub authed actions: star/unstar/is_starred/watch/unwatch/create_issue + Wrong? link + Dashboard personal-stats card) **plus** Phase 13 infrastructure (enrichment module + commands + store + Settings AI Features master toggle + placeholder bundle + `tools/enrich/enrich.py`). ~30 files. Test count 334 → 385.
- ✅ Search-no-match hotfix in `src-tauri/src/commands/search.rs` — `brew search --formula <q>` and `--cask <q>` each exit 1 with "Error: No formulae or casks found for..." when their own kind has zero matches; for formula-only tokens like `abcl` the cask side legitimately has nothing. Each side now handled independently; "no match" treated as empty, only real errors propagated. +3 unit tests. **Uncommitted** in working tree — narrow scope, awaiting user.
- ✅ Tier A catalog enrichment kicked off via `python tools/enrich/enrich.py --tier-a` — running in background as of 2026-05-24 evening. Will produce ~500 KB gzipped `enrichment.json.gz` with friendly-name + summary for the ~5,000 packages with thin or missing `desc`. Estimated cost $3-5 against Haiku 4.5. Hands off `src-tauri/data/enrichment.json.gz` until the run completes.

### Phases (updated)

| Phase | Status |
|-------|--------|
| **Phase 12a — Bundled catalog + manual refresh** | ✅ shipped (`99a1f2c`) |
| **Phase 12b — Settings shell + brew analytics** | ✅ shipped (`99a1f2c`) |
| **Phase 12c — GitHub anonymous repo stats** | ✅ shipped (`99a1f2c`, combined with 12e) |
| **Phase 12d — Paranoid + network settings + settings persistence** | ✅ shipped (`99a1f2c`) |
| **Phase 12e — Device Flow OAuth + Keychain** | ✅ shipped (`99a1f2c`, combined with 12c) |
| **Phase 12f — GitHub authed actions** | ✅ shipped (`8b89c40`) |
| **Phase 13 — Catalog enrichment infrastructure** | ✅ shipped (`8b89c40`); Tier A bundle baking in background |
| **Phase 9c — "Wrong?" GitHub-issue link** | ✅ shipped (folded into 12f in `8b89c40`) |
| **Phase 14 — bundled cask icons** | DROPPED (trademark risk) |
| **Phase 10 — Recipes** | deferred — catalog + enrichment now available, naturally pairs |
| **Phase 9d — installedAt + Last-Updated sort** | small standalone, not blocking |
| **Tier B Tahoe Liquid Glass (Swift bridge)** | v0.2 |

### Files touched

3 commits across the session totalled **~110 files** modified or added (84ad010 Phase 9+11 was 38 files; 99a1f2c Phase 12 Wave 1+2 was 47 files; 8b89c40 Phase 12f + Phase 13 was ~30 files; some overlap on `lib.rs` / `commands/mod.rs` / shared state). 70+ unique files when deduplicated.

### Test + lint status (current)

- `cargo test`: **385 passed / 0 failed / 6 ignored** (was 334 at start of evening; 210 at start of session)
- `cargo clippy --all-targets -- -D warnings`: clean
- `cargo check`: clean
- `npm run check`: 0 errors, 1 pre-existing tsconfig-node warning
- `npm run build`: clean

### Phase 12f notes

- `authed_gate(state, homepage, feature)` chain — single helper, 5 ordered steps: `require_network` → `parse_github_url` → `read_token` → `read_scopes` → `actions::build_client`. Every action command routes through it. Paranoid fires FIRST so we don't leak "auth required" semantics to a user who told us to stop making outbound calls.
- Issue creation input rules implemented as separate sanitisers in `actions::sanitise_title` / `sanitise_body` / `sanitise_labels`. Title strips control chars except `\t`. Body strips null bytes only (GitHub renders Markdown — don't maul user-intended markup). Labels are `≤ 10` entries matching `^[A-Za-z0-9_./-]+$`.
- Dashboard "Personal stats" card uses a 50-permit `Semaphore` for the batch `github_is_starred` calls (one per installed package whose homepage is a GitHub URL). Backend cache is 24h so repeat opens are free.
- "Wrong?" categorization deeplink uses `percent_encoding::utf8_percent_encode` rather than format-string concatenation for the prefilled body.
- `BrewError::AuthRequired` and `BrewError::ScopeRequired { scope }` (added in 12e behind `#[allow(dead_code)]`) are now consumed — `dead_code` allowance removed.

### Phase 13 notes

- `tools/enrich/enrich.py` accepts `--tier-a`, `--tier-b`, `--all`, `--dry-run`. Running with no flags prints help and exits — **you cannot accidentally spend money on an Anthropic API call**.
- Placeholder bundle ships at 114 bytes (empty entries map) so the build is reproducible without an API key. Real Tier A bundle is ~500 KB gzipped; Tier A + B together ~2 MiB gzipped. Bundle grows the binary from 6 MiB (catalog) to ~8.5 MiB.
- Rust loader applies the same defense-in-depth caps as the Phase 12a catalog even though the bundle is built by us: 32 MiB raw cap, 64 MiB decompressed cap, per-field length caps (`friendly_name ≤ 100`, `summary ≤ 1024`, `use_cases ≤ 5 entries of ≤ 200 chars`, `similar ≤ 50 tokens` each re-validated against `validate_package_name`, `tags ≤ 12 entries of ≤ 30 chars`).
- **Zero runtime LLM calls.** The `anthropic` SDK is a Python build-time dep only; it never enters the Rust binary. The AI Features toggle controls *rendering*, not *fetching*.
- BUILD.md added an "Catalog enrichment (Phase 13 — optional)" section covering the run-order, tier flags + costs, `ANTHROPIC_API_KEY` location, and an operational examples block.

### Search no-match hotfix notes

- **Bug existed since Phase 2** — surfaced by user search behavior this session (likely a formula-only token like `abcl`).
- Root cause: `f_res.map_err(...)??` flattened both the join error and the inner `BrewError`, so a `BrewExitNonZero { exit_code: 1, stderr_excerpt: "Error: No formulae or casks found..." }` from the cask side propagated as a typed error instead of an empty result.
- Fix: split the flattening, pattern-match on the inner result, and treat the "no match" error pattern as `String::new()`. If BOTH sides fail in unrelated ways, surface the formula error (matches the order the user is most likely searching for).
- New `is_brew_search_no_match` helper + 3 unit tests (`detects_no_match_exit_pattern`, `does_not_match_other_brew_errors`, `does_not_match_non_exit_errors`).
- Test count net change: 0 (the existing search tests still pass; the new tests are additive). 385 → 385.

### Documentation pass (this section)

The Technical Writer pass appended sections to:
- `memory-bank/security.md` §13 ("Phase 12 + 13 additions") — 7 sub-sections covering the new attack surface, gates, and verification approach for each sub-phase. Wave 3 READY-FOR-SCRUTINY verdict above is untouched.
- `memory-bank/backendApi.md` §13 — every new Tauri command shipped this session with signature, paranoid-mode-gate status, auth requirements, and source file path.
- `memory-bank/frontendComponents.md` — new "Phase 9+11+12+13 additions" section with components, stores, utilities, and mount points.
- `memory-bank/activeContext.md` — "Post-Phase-12+13 sync" section.
- `memory-bank/progress.md` (this file) — "Phase 12+13 wrap" section.
- Spot-fixes to README.md (Status line, Architecture line — ~55 commands not ~20) and BUILD.md (cross-references to the two phase plans).


## 2026-05-24 (late session — Phase 12g/13b cleanup + UI polish)

### Done since commit `8b89c40`

#### Tier A enrichment baked
- `python tools/enrich/enrich.py --tier-a` — full run against Anthropic Haiku 4.5
- **15,725 entries** written to `src-tauri/data/enrichment.json.gz` (771 KB compressed, 0.74 MiB)
- Total bundled data: 6.1 MiB catalog + 0.74 MiB enrichment ≈ 6.9 MiB
- Cost: ~$3-5 against user's Anthropic API
- Sample quality validated on first 12 entries (a2ps, abcl, ab-av1, etc.) — Haiku correctly identifies niche tools, transforms opaque tokens to readable names, leaves already-readable tokens alone
- `tools/enrich/enrich.py` patched with cascade `.env` lookup (tools/enrich/.env → tools/categorize/.env → process env) so the user doesn't have to duplicate ANTHROPIC_API_KEY

#### Phase 12g/13b cleanup (all 4 IMPORTANT findings from Code Reviewer addressed)
1. **Phase 12a frontend wired** (was dead code from UI's perspective) — `Catalog`/`Formula`/`Cask`/`CatalogSummary` types, 6 IPC wrappers, `src/lib/stores/catalog.svelte.ts` with `summary`/`refreshing`/`isStale`/`daysOldLabel`, Dashboard catalog freshness line, Discover stale-catalog banner
2. **Three persisted settings now actually honored:** `trending_ttl_minutes` in `trending_fetch`, `cask_icon_mode` in `cask_icon_from_homepage` (with pure `cask_icon_gate_decision` helper), `catalog_auto_refresh` via new startup hook `maybe_auto_refresh_catalog` + `should_auto_refresh(schedule, age)` decision helper + extracted `refresh_catalog_inner`. +23 tests
3. **Search hotfix** — `brew search abcl` was crashing on "brew_exit_non_zero" because `brew search --cask abcl` exits 1 (formula-only token). `is_brew_search_no_match` helper now tolerates per-kind no-match exits; `brew_search_desc` verified-not-affected (exits 0 on no match). +4 tests
4. **Phase 13 friendly names in list rows** — Discover (search + chip-filtered), Library (via PackageRow), Trending all render `friendly_name` as a subtitle below the raw token when AI Features toggle is on

#### Native macOS menu
- `tauri::menu::MenuBuilder` in `src-tauri/src/lib.rs` — App menu (About brew-browser, Settings… ⌘,, Hide / Hide Others / Show All, Quit) + Edit (Undo/Redo/Cut/Copy/Paste/Select All) + Window (Minimize/Maximize/Close) submenus
- `MENU_EVENT_ABOUT` and `MENU_EVENT_SETTINGS` constants emit Tauri events; `+layout.svelte` `listen()`s and opens the matching modal
- Requires app restart (menus build at startup)

#### About brew-browser modal
- New `src/lib/components/AboutModal.svelte` — 🍺 hero, version + brew + license + repo meta, big "♥ Donate to the project" CTA, credits paragraph crediting **Agency Agents** (clickable link → https://github.com/msitarzewski/agency-agents) "powered by Claude Code in the terminal, running Opus 4.7 [1m]"
- Mounted in `+page.svelte`, opened via `ui.openAbout()` or the native App menu's "About brew-browser" item

#### GitHub Sponsors setup
- `.github/FUNDING.yml` → `github: [msitarzewski]` (surfaces "Sponsor" button on the repo page)
- Shared `src/lib/util/donate.ts` exports `SPONSOR_URL` — single source for AboutModal CTA and sidebar footer link
- Sidebar footer gets `♥ Donate` link under brew version

#### TopBar (theme + Settings group)
- New `src/lib/components/TopBar.svelte` — theme dropdown (sun/moon/monitor icon reflecting current → opens 3-item popover Light/Dark/System) + Settings gear in a subtle sunken-background button group with hair-line divider
- `position: absolute` inside `.content` (NOT fixed) — anchored to main panel area, never overlaps PackageDetail
- Theme + Settings stripped from sidebar footer
- Multiple style iterations: pill → flat → pill-with-divider → final responsive

#### Unified panel-head ("precision, happy")
- Global `.panel-head` baseline in `src/app.css` — `padding: 18px var(--space-4)`, `min-height: 60px`, `border-bottom: 1px solid var(--color-border)`, h1 `font-size: var(--text-h1); line-height: 1.2;`
- `!important` justified as cross-component coordination (Svelte scopes styles per component)
- `.content .panel-head` scopes the 96px right-padding TopBar-reserve to main panels only — detail header gets symmetric padding so close X sits flush at right edge
- Detail header gets `class="panel-head"` so it inherits the shared baseline — separator y-coordinate matches every main panel-head exactly

#### Responsive headers + columns (avoid the crashing)
- Trending / Library / Services / ActivityHistory all wrap their Refresh or "Clear completed" in `.refresh-wrap` / `.action-wrap`
- `@media (max-width: 1000px)` hides those wraps + auxiliary text ("Updated Ns ago", "N running · M total")
- Trending + Library list rows + headers get tiered responsive column drops:
  - `@media (max-width: 880px)` drops trailing 5th column (installed pill / Outdated badge)
  - `@media (max-width: 720px)` drops middle secondary column (Installs / Version)
  - `# / NAME / TYPE` always visible
- `overflow: hidden` + `min-width: 0` on every header/row cell prevents column-header glyph collision (the NAVME/INVPALLS bug)
- All hidden actions remain accessible via Cmd+R / per-row controls

#### Pillgroup unified
- Trending + Library `.pillgroup` lose the hard border, switch to sunken-background-only matching the new TopBar group pattern

#### PackageDetail rework
- h1: `enriched?.friendlyName ?? ui.selectedPackage.name` — friendly name when AI on + enrichment has one; raw token otherwise. Token always surfaces (in h1 OR in new Token meta dl row)
- Type pill right-aligned (`margin-left: auto`)
- Close X flush at right edge (after `.content`-scoped padding fix)
- AI-enriched badge removed from h1 — provenance still on summary/use_cases/similar/tags lower in body
- Detail-header separator y-coordinate matches main panel-head separator exactly (precision-happy)

#### Brew analytics parser widened
- `parse_analytics_state` now accepts `[<backend>] [a|A]nalytics are [en|dis]abled[.]` — modern brew emits `"InfluxDB analytics are enabled."` which the original strict matcher rejected
- Strict-first-line constraint preserved (no whole-output regex)
- +3 tests pinning InfluxDB + arbitrary-backend variants

#### GitHub sign-in friendlier error
- `start_device_flow` fails fast with "GitHub sign-in is not configured in this build…" when `GITHUB_OAUTH_CLIENT_ID` is still the placeholder
- `github` store uses `brewErrorMessage(e)` instead of `e.code` — human message reaches the frontend
- DeviceFlowModal drops redundant `toast.error` on error state — modal renders inline

#### Detail-panel auto-close on navigation
- `ui.setSection(s)` now also clears `ui.selectedPackage` — clicking any sidebar/Cmd+0-6 closes the detail panel

#### Other polish
- README "Open by default" updated (still 7 paths after all this)
- BUILD.md mentions GitHub OAuth App setup (still needs real client_id before release)

### Test + build status (final)

- `cargo test`: **411 passed** (was 385 at end of Phase 13)
- `cargo clippy --all-targets -- -D warnings`: clean
- `cargo check`: clean
- `npm run check`: 0 errors, 1 pre-existing tsconfig-node warning
- `npm run build`: clean

### Files this session (since `8b89c40`)

**New (5):** `.github/FUNDING.yml`, `src/lib/components/AboutModal.svelte`, `src/lib/components/TopBar.svelte`, `src/lib/stores/catalog.svelte.ts`, `src/lib/util/donate.ts`

**Backend modified (10):** `src/{app.css}` ... wait this is frontend. Backend: `src-tauri/Cargo.toml` (unchanged), `src-tauri/src/commands/{brew_env,cask_icon_homepage,catalog,search,trending,settings}.rs`, `src-tauri/src/github/auth.rs`, `src-tauri/src/lib.rs` (menu), `src-tauri/src/state.rs` (auto-refresh hook), `src-tauri/data/enrichment.json.gz` (15,725 entries)

**Frontend modified (~15):** `src/app.css`, `src/lib/api.ts`, `src/lib/types.ts`, `src/lib/components/{Dashboard,Discover,Library,PackageDetail,PackageRow,Trending,Services,Sidebar,Settings,SettingsSectionBrew,DeviceFlowModal,ActivityHistory}.svelte`, `src/lib/stores/{ui,github}.svelte.ts`, `src/routes/{+layout,+page}.svelte`

**Tools / docs:** `tools/enrich/enrich.py` (cascade .env)


---

## 2026-05-24 (v0.2.0 release)

### Shipped

- **Native macOS title bar** — unified 36 px chrome above the main split, traffic lights and toggle and page title all centered on one horizontal axis (`trafficLightPosition: { x: 14, y: 20 }`). Adaptive toggle position (inside sidebar's right edge when expanded; next to traffic lights when collapsed).
- **Page title moved to title bar** — `ui.pageTitle` derived from `ui.section`. Every pane's `<h1>` was removed from its panel-head; panes with no remaining action content (Dashboard, Discover) lost their entire `<header>`. The others became thin secondary toolbars right-aligning their action clusters.
- **Title-bar right cluster** — new `TitlebarControls.svelte` hosts theme dropdown (single icon → 3-item popover) + Settings gear + pink-filled Donate heart, grouped as one pill. Lighter `--color-surface` background (was the dark sunken). Right-edge aligns with the panel content's right padding.
- **Sidebar refactor** — brand area gone; Dashboard is now the first nav item with `LayoutDashboard` icon. New persistent **type-ahead search input** at the top (no separator below it): debounced 300 ms → `brew_search`; dropdown of top 7 hits with kind pill + installed badge; ArrowUp/Down + Enter + Esc support; "See all results in Discover →" affordance. Hidden in collapsed mode.
- **Collapsible sidebar** — `ui.sidebarCollapsed` state, persisted to localStorage. 200 → 56 px transition. Nav items become icon-only with a small badge overlay; theme/Settings/Donate hidden (they live in the title bar now); status row collapses to dot-only.
- **(i) info popovers replace AI badges + "Wrong?" links** — new reusable `InfoButton.svelte`: hover-activated with open/close delays, focus-supported for keyboard a11y, `position: fixed` so it escapes ancestor `overflow:hidden`. Every "Wrong?" link and every "AI-enriched" sparkle badge gone from `PackageDetail.svelte`; replaced with a single (i) per cluster (Categories, Tags, Summary, Why install this?, Similar packages). Summary's (i) sits inline at the end of the text. Body text: *"Generated offline at build time by Claude Haiku 4.5 — no network or LLM calls happen while you use brew-browser. Open an issue if X looks off and we'll fix it in the next release."*
- **Intercept-on-action GitHub flow** — static "Sign in via Settings → GitHub to star, watch, or file issues." paragraph removed. Star / Watch / File issue buttons always render when stats card is visible. Clicking while signed-out triggers `requireGithubSignIn(actionLabel)`: deep-links to Settings → GitHub via `ui.openSettings("github")` + toasts a hint. Intent-based discovery: the prompt only appears when the user actually wants to act.
- **Settings deep-link plumbing** — `SettingsSection` type promoted to `types.ts`; `ui.openSettings(section?)` accepts an optional target section; `closeSettings()` clears it; `Settings.svelte` honors `ui.settingsInitialSection ?? "appearance"` on open.
- **GitHub OAuth App live** — `GITHUB_OAUTH_CLIENT_ID = "Ov23liJZKbvrSBuiOPkT"` (Device Flow client_ids are RFC 8628-public; safe to commit). Sign-in no longer fails fast with "not configured."
- **License-mismatch row wraps as prose** — whole sentence in one `<span>`, AlertCircle as a single non-shrinking flex child.
- **EmptyState vertically centered** — every empty state across the app (Library, Discover, Snapshots, Trending, etc.) sits in the middle of its pane.
- **Snapshots inline CTAs removed** — empty state is purely informational; the duplicate Import + New Snapshot buttons (already in the panel-head) are gone.
- **Selected-row persistence** — Library, Trending, Discover (both row variants), Services, Dashboard outdated all bind `selected={…}` to `ui.selectedPackage` so the source row stays highlighted while the detail panel is open.
- **Chip-filter clears on cross-pane navigation** — `ui.setSection()` calls `discover.clear()` when section actually changes; deeplink callers reordered to call setSection first then selectOnly.
- **Native-square app icon (Tahoe-clean)** — `docs/icon/brew-browser.svg` re-authored as a full-bleed 181×181 square; all icon sizes regenerated via `npm run tauri icon`. Fixes the macOS Tahoe double-squircle artifact where the OS mask was exposing transparent corners.

### Files this session

**New (2):** `src/lib/components/InfoButton.svelte`, `src/lib/components/TitlebarControls.svelte`

**Deleted (1):** `src/lib/components/TopBar.svelte` (folded into the new title bar + TitlebarControls split)

**Backend modified:** `src-tauri/src/github/auth.rs` (live client_id), `src-tauri/tauri.conf.json` (version bump + trafficLightPosition)

**Frontend modified:** `src/app.css`, `src/lib/types.ts`, `src/lib/stores/ui.svelte.ts`, `src/lib/components/{Dashboard,Discover,Library,Trending,Snapshots,Services,ActivityHistory,Sidebar,Settings,SettingsSectionGitHub,EmptyState,PackageDetail}.svelte`, `src/routes/{+layout,+page}.svelte`

**Icons:** `src-tauri/icons/*` (all sizes regenerated), `docs/icon/brew-browser.{svg,af}`

### Tests & lint

- `cargo test`: **411 passed**, 0 failed, 6 ignored
- `cargo clippy --all-targets -- -D warnings`: clean
- `npm run check`: 0 errors
- `npm run build`: clean

---

## 2026-05-24 (v0.2.1 hotfix)

### What & why

Hotfix on top of v0.2.0 (`e04dbff`) that addresses two distinct issues users hit immediately on the v0.2.0 .dmg:

1. **macOS Keychain prompt on every launch.** The v0.2.0 fix for "Star/Watch bounces to Settings even when signed in" added an eager `void github.loadStatus()` to `+layout.svelte`'s `onMount`. That call probes the Keychain via `keyring::get`, which prompts the user when the binary signature doesn't match an existing item's ACL. Fresh v0.2.0 installs (new signature vs whatever wrote the Keychain entry) saw the "brew-browser wants to use your confidential information stored in dev.openbrew.browser" prompt on every launch — even users who'd never used a GitHub feature. Worse, it trained users to dismiss the prompt without context.

   Fix: removed the eager call. Made `requireGithubSignIn(actionLabel)` async; it now lazy-probes the Keychain on the first GitHub-action click (Star / Watch / File-issue). Settings → GitHub still hydrates status on panel mount (unchanged). Net effect: the macOS prompt only fires when the user is actively trying to use the token, which is the contextual moment it's meant for.

2. **Three GitHub-auth UX bugs** (the cluster fixed in commit `e04dbff` after the initial v0.2.0 push but already shipped in v0.2.0's .dmg):
   - "Signed in as @github user." placeholder in the post-sign-in toast (fixed: `loadStatus()` runs before `signinState = approved` flips)
   - Stack of duplicate "Signed in to GitHub" toasts (fixed: `untrack(() => github.status?.username)` in DeviceFlowModal so the effect's only reactive dep is `signinState`)
   - Star/Watch/File-issue bounced authenticated users to Settings (fixed by #1 above — lazy probe in `requireGithubSignIn`)

### Other small things

- Replaced "Powered by Anthropic's Claude Opus 4.7 and the Claude Agent SDK" with "Powered by Claude Code in the terminal, running Opus 4.7 [1m]" in README.md + AboutModal.svelte. More accurate attribution: Claude Code is the runtime (this CLI), `claude-opus-4-7[1m]` is the model.
- Added `.gitleaks.toml` allowlist for the documented public OAuth Device Flow client_id (`Ov23liJZKbvrSBuiOPkT`). Per RFC 8628 §3.1, Device Flow client_ids are public by design and intentionally committed.
- `memory-bank/security.md` §14 appended: full v0.2.0 audit re-run results (cargo audit 0, cargo deny ok, npm audit 0, semgrep 0, gitleaks 0-after-allowlist) — verdict READY-FOR-SCRUTINY preserved.

### Files this hotfix

**Modified:**
- `src-tauri/Cargo.toml` (version 0.2.0 → 0.2.1)
- `src-tauri/tauri.conf.json` (version 0.2.0 → 0.2.1)
- `src/routes/+layout.svelte` (removed eager `github.loadStatus()` from onMount)
- `src/lib/components/PackageDetail.svelte` (async `requireGithubSignIn` + 3 awaited call sites)
- `src/lib/components/AboutModal.svelte` (Built-with credit updated)
- `src/lib/stores/github.svelte.ts` (poll loop: `loadStatus` before `signinState = approved`)
- `src/lib/components/DeviceFlowModal.svelte` (`untrack` wrap on the toast effect's status read)
- `README.md` (v0.2.1 status block + Built-with credit + Sponsor badge)
- `landing/index.html` (softwareVersion 0.2.0 → 0.2.1)
- `memory-bank/security.md` (appended §14 v0.2.0 audit results)
- `memory-bank/progress.md` (this entry)

**New:**
- `.gitleaks.toml` (allowlist for the public Device Flow client_id)

### Tests & lint

- `cargo test`: 411 passed, 0 failed, 6 ignored
- `cargo clippy --all-targets -- -D warnings`: clean
- `npm run check`: 0 errors
- `npm run build`: clean

### Security audit (re-run for v0.2.0/v0.2.1 surface)

- `cargo audit`: 0 vulns
- `cargo deny check`: advisories ok, bans ok, licenses ok, sources ok
- `npm audit --omit=dev`: 0 vulnerabilities
- `semgrep` (security-audit + OWASP-10 + Rust + TS, 113 rules): 0 findings
- `gitleaks`: 0 leaks (after allowlist; 2 false positives on the public client_id were the only initial hits)

**Verdict: READY-FOR-SCRUTINY preserved.** See `memory-bank/security.md` §14 for the full breakdown.

---

## 2026-05-25 (v0.3.0 prep — Phase 15 + GitHub integration completion + issue #1 fix)

### Big-picture summary

Two coupled tracks landed in this session, both targeting v0.3.0:

**Track A — Phase 15 (in-app updater + Offline Mode UI rename).** Implemented via a 4-agent parallel wave (Backend Architect + Frontend Developer × 2 + Technical Writer) + Lead bridging IPC. **+34 backend tests**, all `cargo check` / `cargo clippy` / `npm run check` clean. Then 2-agent review wave (Code Reviewer + Security Engineer) returned **NEEDS-WORK with 5 CRITICAL findings**: IPC wire-shape mismatch on Available, "Relaunch now" button re-runs install, manifest format (.dmg vs .app.tar.gz), missing error variants in frontend union, and `update_skip` silently revokes paranoid mode on Corrupt settings (Lead's bridging command introduced that one). All five well-scoped, ~2-3h fix-up. Tracked as task #41.

**Track B — Issue #1 root cause + GitHub integration completion.** User reported toast cascade recurring after v0.2.1 ship. Spent ~6h tracing. Real root cause: a starredCache infinite-loop in PackageDetail (`isStarred` catch wrote "unknown" — same as the cache-miss sentinel — causing infinite refetches). Cascade fixed. Then surfaced **3 more bugs in a row**: scope parser was using `split_whitespace` (GitHub returns comma-separated), Star failed with ScopeRequired; Watch needs `notifications` scope (GitHub's docs explicitly require it), got HTTP 404; toast `$effect` pattern itself was structurally wrong per Svelte 5 docs (`$effect` is "an escape hatch", not a side-effect channel). All four fixed. Then added: per-action scope gate (parameterized `authed_gate`), actionable Re-authorize toast (new `Toast.action` type), Octocat status chip in title bar (real Octocat from Primer/Octicons, Lucide strips brand icons). +2 new backend tests (445 → 447). Tracked as tasks #14 and #15.

### Shipped (uncommitted, lands in v0.3.0 commit)

**Phase 15:**
- `tauri-plugin-updater` integrated, scheduler, 8 backend tests
- `UpdateIndicator.svelte` (title-bar pill) + `SettingsSectionUpdates.svelte` (Settings UI) + `updater.svelte.ts` store
- "Paranoid Mode" → "Offline Mode" UI rename (internal `paranoid_mode` field stays for migration compat)
- BUILD.md: minisign setup + manifest publishing flow
- security.md §15 stub (to be filled post-fix-up)
- New `tools/release/publish-manifest.sh`, `.gitleaks.toml` allowlist

**GitHub fixes:**
- `StarredOutcome` adds `"error"` variant (cache-loop killer)
- Scope parser splits on commas + whitespace
- `notifications` added to `GITHUB_OAUTH_SCOPES`
- Per-action scope gate in `commands/github.rs`
- Toast `$effect` removed from DeviceFlowModal; imperative call in `signIn()` poll loop
- `Toast.action` type + `invokeAction(id)` + actionable rendering
- `showActionFailureToast` helper in PackageDetail routes all 3 catch blocks
- `GithubMarkIcon.svelte` (Primer/Octicons MIT)
- `TitlebarControls` chip with green/amber/hidden states + eager `loadStatus()` on mount (v0.3.0+ follow-up to gate on localStorage flag)
- "Powered by Claude Code in the terminal, running Opus 4.7 [1m]" attribution in 3 spots (already shipped in v0.2.1, mentioned here for completeness)

**Memory bank:**
- Two new task records: `tasks/2026-05/{14-issue-1-hunt-cache-loop.md, 15-github-integration-completion.md}`
- `tasks/2026-05/README.md` index updated
- `activeContext.md` rewritten

### Tests & lint (final, this session)

- `cargo test`: **447 passed**, 0 failed, 6 ignored (411 → 445 from Phase 15 → 447 from per-action scope tests)
- `cargo clippy --all-targets -- -D warnings`: clean
- `cargo check`: clean
- `npm run check`: 0 errors, 3 pre-existing warnings (SettingsSectionGitHub unused-CSS, tsconfig-node-types)
- `npm run build`: clean
- All diagnostic instrumentation reverted (no `[diag]` console.log / `console.trace` left in code)

### What's blocking v0.3.0 ship

1. **Phase 15 fix-up pass** (task #41) — 5 CRITICAL findings, ~2-3h
2. Optional: **Expand GitHub package resolution** (task #46) — walks `urls.stable.url` / `head` / cask `url` fields beyond `homepage`, ~1-2h
3. Version bump → commit → tag → build → ship → memory bank refresh

### Web research consulted

- [Svelte 5 `$effect` docs](https://svelte.dev/docs/svelte/$effect) — "$effect is an escape hatch"
- [`effect_update_depth_exceeded` runtime errors](https://svelte.dev/docs/svelte/runtime-errors)
- [Svelte issue #14697](https://github.com/sveltejs/svelte/issues/14697) — community guidance on not updating state in effects
- [GitHub REST API watching](https://docs.github.com/en/rest/activity/watching) — endpoint contract
- [GitHub OAuth scopes docs](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps) — notifications scope grants watch/unwatch
- [GitHub community discussion #52522](https://github.com/orgs/community/discussions/52522) — 404 = often insufficient permissions

## 2026-05-25 (v0.3.0 SHIPPED)

### Done

- ✅ **Phase 15 fix-up** (task #16) — all 5 CRITICAL findings resolved + 1 IMPORTANT (`cached_available` clear post-install) folded in. Commit `1bfe21f`. +3 backend tests.
- ✅ **GitHub coverage expansion** (task #17) — `Package::github_homepage` walks homepage → urls.stable.url → urls.head.url (formula) or homepage → url (cask). Backend pre-resolves canonical github.com/<o>/<r>. Frontend Dashboard + PackageDetail use the pre-resolved field. Commit `820c1f0`. +23 backend tests (450 → 473).
- ✅ **v0.3.0 release** (commit `d7c2bca`, tag `v0.3.0`):
  - Generated real minisign keypair at `~/.config/brew-browser/updater.key` (second attempt — first password mismatch traced to bash `$`/`!` expansion in double-quoted env vars; switch to single quotes resolved it). Pubkey embedded in `lib.rs` + `tauri.conf.json`.
  - Version bumped `0.2.1 → 0.3.0` (`Cargo.toml`, `Cargo.lock`, `tauri.conf.json`, `landing/index.html`).
  - Added `createUpdaterArtifacts: true` to `tauri.conf.json` bundle config — Tauri 2 needs this flag explicitly or the `.app.tar.gz` updater target silently skips.
  - `sign-and-notarize.sh` pre-flights both Apple + Tauri signing env vars, bridges `_PATH` → inline contents (Tauri 2 reads `TAURI_SIGNING_PRIVATE_KEY` only, despite signer-generate output claiming `_PATH` works).
  - `publish-manifest.sh` rewritten to consume `.app.tar.gz.sig` from the bundler (not re-sign via raw minisign — that format wouldn't validate against the embedded Tauri-format pubkey).
  - Build pipeline shipped: signed + notarized `.dmg` + signed `.app.tar.gz` + `.app.tar.gz.sig`. End-to-end verified via `spctl assess` + `stapler validate`.
  - GH release created with both artifacts: <https://github.com/msitarzewski/brew-browser/releases/tag/v0.3.0>. Renamed `.app.tar.gz` asset via API to match the versioned manifest URL (`#newname` syntax on `gh release create` only updated label).
  - Manifest rsync'd to `umacbookpro:Sites/brew-browser/updater.json`. Live at <https://brew-browser.zerologic.com/updater.json>.
  - Issue #1 auto-closed via `Closes #1` keyword in the release commit. Posted a thanks/follow-up comment to @heyjawrsh.
  - README "Status" section updated to lead with v0.3.0.
  - `docs/release-notes/0.3.0.md` (new convention — release notes live under docs/, not repo root).

### Tests & lint at release time

- `cargo test`: **473 passed**, 0 failed, 6 ignored
- `cargo clippy --all-targets -- -D warnings`: clean
- `npm run check`: 0 errors, 3 pre-existing warnings
- `npm run build`: clean

### Release-pipeline lessons learned (for v0.4.0 and beyond)

1. **`TAURI_SIGNING_PRIVATE_KEY` requires inline contents.** The path variant doesn't work despite the signer-generate output claiming it does (Tauri 2 CLI as of 2026-05). Build script now bridges automatically; alternative is `export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.config/brew-browser/updater.key)"` in signing.env.
2. **Password env vars need single quotes** in signing.env when the password contains `$`, `!`, backticks, or anything else bash expands inside double quotes. Use `'...'` literal strings.
3. **`createUpdaterArtifacts: true`** is required in `tauri.conf.json` for Tauri 2 to emit `.app.tar.gz` alongside `.dmg`. Default is off; without it the updater target silently skips.
4. **`.app.tar.gz.sig` is Tauri-format, not raw minisign-format** (`.minisig`). The plugin's verification reads the .sig contents as a single string. Don't re-sign via minisign CLI — the format wouldn't validate.
5. **`gh release create asset.ext#new-name` only sets the label**, not the filename. Use `gh api -X PATCH /repos/.../releases/assets/<id> -f name=<new-name>` to actually rename the downloadable asset.
6. **CDN cache settles within ~10s** after rename. Initial 404 on renamed asset is benign; retry after a brief pause.

### Outstanding (v0.3.x or v0.4.0)

- Localstorage flag gating `TitlebarControls.onMount` eager `loadStatus()` — preserve v0.2.1's "zero Keychain prompt unless you sign in" promise for users who never sign in. ~15min.
- `cancelSignin` timer leak edge case (rare, requires user clicking Cancel within the 1500ms post-approve window). ~10min.
- Startup placeholder-pubkey guard (5-line panic-on-PLACEHOLDER in release builds) — would have caught the v0.3.0 cycle of "key generated but pubkey still placeholder."
- Persist `last_checked_at` to disk so the auto-updater honours the 24h floor across launches (currently in-memory only, effectively never auto-fires for typical morning/evening usage patterns).
- Manifest URL allowlist enforcement — documented in `security.md` §15 but not implementable through `tauri-plugin-updater 2.10.1` (no pre-fetch hook).

## 2026-05-25 (v0.3.1 SHIPPED — same day as v0.3.0)

Cumulative point release rolling up 13 commits since `d7c2bca` (v0.3.0). Theme: polish + magic.

### Done

- ✅ **Report-to-brew-browser flow** (commit `eff67e7`) — every error toast now carries a "Report" action that opens a pre-filled GH new-issue URL with full context (app + brew version, command, exit code, stderr excerpt, friendly message). Same button surfaces in the Activity drawer's failed-job footer. New `src/lib/util/reportIssue.ts` centralizes the URL builder + the `reportableToastError(title, e)` helper that replaced 10 sites with the `toast.error(title, e.code)` anti-pattern (which threw away the friendly message and gave the user no recourse beyond the raw discriminator).
- ✅ **`brew_list(force)` cache-bypass fix** (`6aaf3c1`) — Refresh button can now actually refresh the installed list. Previously `state.installed_cache` was warm-on-first-launch and never invalidated except by in-app actions; `brew upgrade` runs from the user's terminal could mask the list indefinitely.
- ✅ **Curated Upgrade modal** (`b11c7ac`) — "Choose…" button on Dashboard Updates card opens a checkbox list of every outdated package, pinned-badged + auto-checked, with batched `brew_upgrade_many(names)` IPC streaming into Activity.
- ✅ **Refresh now does `brew update` too** (`2983a02`) — full three-step sync: brew update → catalog refresh → installed reload. Streaming brew-update output to Activity so the user sees what brew is doing. Closes the loop on "why are my outdated flags stale?"
- ✅ **Activity persistence hardening** (`f4ddd2e`) — `startJob` persists immediately (not via the 400ms debounce); cap raised from 50 to 200; persist + hydrate now log to console on failure instead of silently swallowing.
- ✅ **Root → docs/ reorganization** (`83bc371` + `1971bbe`) — `BUILD.md` / `PHILOSOPHY.md` / `PLAN.md` moved to `docs/`. Root now holds only GitHub conventions (README, SECURITY, CONTRIBUTING) + AI workflow files (AGENTS.md + CLAUDE.md symlink). New `Project root vs memory-bank` section codified in `toc.md`.
- ✅ **Memory-bank refresh** (`95e9e6a` + `51ae088`) — toc.md gains "Live vs historical entries" convention; six live docs refreshed against v0.3.0 reality (backendApi, frontendComponents, ideas, projectbrief, techContext, realityCheck).
- ✅ **Bundle id rename** (`1b1287d`) — `dev.openbrew.browser` → `com.zerologic.brew-browser`. KEYCHAIN_SERVICE coordinated. `service_id_matches_tauri_conf` test pinned to new value. v0.3.0 users will re-sign-in to GitHub once via the existing Re-authorize toast button.
- ✅ **Donut hover-with-counts + AI subtitle fallback** (`2805e8c`) — hover any slice (or matching legend row) → fatten + dim siblings + center-text takeover ("325 / installed" → "{count} / {label}"). `prefers-reduced-motion` respected. Discover browse rows fall back to catalog `desc` when AI doesn't have a friendly name.
- ✅ **Unified Description + Version columns across list views** (`e7b7093`) — Library, Discover, Trending all share the canonical row layout: `(icon/rank) | NAME | DESCRIPTION | VERSION | TYPE | TRAIL`. Description column prefers AI summary > upstream desc. Version reads from catalog for uninstalled, installed-version for Library. `CatalogEntrySummary.version` field added to backend. Responsive breakpoints drop Trail → Description → Version in priority order.
- ✅ **Magic `local_search`** (`2f79559`) — new in-process IPC scans catalog + enrichment + categories in unified union-search. Weighted scoring: name (1000/700/500) > friendlyName (350) > category label (280) > summary (180) > desc (120) > tag (100). Multi-term AND semantics. Sub-20ms even on 3-term queries. Replaces `brew_search` as the search store's default. "video player" finds VLC. "AI" finds ollama + llm. "Video & Audio" returns the whole category.

### Tests & lint at release time

- `cargo test`: **473 passed**, 0 failed, 6 ignored
- `cargo clippy --all-targets -- -D warnings`: clean
- `npm run check`: 0 errors, 3 pre-existing warnings (unchanged)
- `npm run build`: clean
- `bash -n` on both release scripts: clean

### Notes

- The bundle id rename is the only user-visible cost: v0.3.0 users have to re-sign-in to GitHub once. Worth doing now at 18 stars rather than later at 1800.
- `local_search` is the marquee feature. The scoring rubric was tuned by hand against a few representative queries (`vlc`, `password manager`, `AI`, `Video & Audio`). Future polish: surface category-label exact-match as a pinned "Browse → X" suggestion above brew's hits.
- One stable v0.3.x follow-up: persist `last_checked_at` to disk so the auto-updater 24h floor honours typical morning/evening usage. Currently in-memory only.

## 2026-05-26 (v0.4.0 backend on branch)

Same-day continuation. Branch `feat/v0.4.0-velocity-and-history` off `main` at `d6d28a0`. Full file:line detail in `tasks/2026-05/19-v0.4.0-backend.md`.

### Done (Steps 1–3 of 9 — full backend)

- ✅ **Step 1** — `Settings.enhanced_trending_enabled: bool` (default `false`, forward-compat tested), `state::AppState::require_enhanced_trending()` gate composing master paranoid with per-feature toggle, new `BrewError::FeatureDisabled { feature }` variant so frontend can route toast to the right setting. +9 tests.
- ✅ **Step 2** — `trending::client::fetch` now hits `install` + `install-on-request` in parallel via `tokio::join!` and merges on package name. New `trending::velocity::velocity_index(c30, c90, c365) → Option<f64>` pure-math helper (returns `None` on degenerate or too-small inputs). `commands::trending::trending_fetch` eager-warms all three windows via `tokio::task::JoinSet` and back-fills `velocity_index` from the cross-window join. +14 tests.
- ✅ **Step 3** — New `trending::history::{mod, client, cache}` module. Two new IPCs: `trending_history_index()` (summary blob — top-N with velocity + compact sparkline; single fetch on tab mount) and `trending_history_fetch(name, kind)` (per-package full series; on-demand from PackageDetail). Both gated by `require_enhanced_trending`. URL builder rejects path traversal. LRU cache (cap 500, TTL 6h). 5 new types in `types.rs` for the history wire shape. +10 tests.

### Decisions locked (per-decision rationale in task #19)

- **D1** subpath `brew-browser.zerologic.com/trending-history/*` (not a new vhost) — reuses Caddy + cert
- **D2** GitHub mirror of nightly JSON deferred to v0.5+
- **D3** default sort by velocity desc + inline sparklines per row (star-history.com aesthetic); index blob carries compact sparkline arrays so the list renders from one fetch
- **D4** sparkline empty state when toggle is off = passive (only in Settings → Network)
- **D5** velocity computed server-side; frontend doesn't know the formula

### Tests & lint at backend checkpoint

- `cargo test`: **506 passed**, 0 failed, 6 ignored (473 → 506, +33 new)
- `cargo build`: clean — zero dead-code warnings (every new symbol is wired and exercised)
- Frontend untouched in this checkpoint

### Workflow change (durable)

From this branch onward, merges to `main` go through pull requests — push branch, `gh pr create`, review/CI, merge. No more direct pushes to `main`.

### Still ahead

- **Step 4** Settings UI (new `SettingsSectionTrendingHistory.svelte`, disclosure-list entry)
- **Step 5** Trending tab UI (velocity column + sort-by-velocity default + inline sparklines)
- **Step 6** PackageDetail sparkline (new `TrendingSparkline.svelte` + new `trendingHistory.svelte.ts` store)
- **Step 7** umbp `tools/trending-collector/` (Bun TS daemon + seed.ts + cron + SQLite + static JSON output)
- **Step 8** Memory bank + docs polish (decisions.md ADR, projectbrief.md nine→ten paths, security.md endpoint audit, backendApi.md / frontendComponents.md / techContext.md, `docs/release-notes/0.4.0.md`, README disclosure)
- **Step 9** Caddy privacy hardening (IP-strip, no cookies, GET-only, cache-control; document snippet in security.md so it's auditable)

## 2026-05-26 (later — v0.4.0 Steps 4–8 done on branch)

Same day, second commit + third commit on top of the backend. Full detail in `tasks/2026-05/19-v0.4.0-backend.md` (record now spans Steps 1–8). **Only Step 9 — Caddy deploy + verification — remains** before this branch PRs into main and v0.4.0 ships.

### Done (Steps 4–6 — commit `6711133`)

- ✅ **Step 4** — Settings → Network UI: new `SettingsSectionTrendingHistory.svelte` opt-in subsection mounted alongside the Updates subsection at the bottom of Network. New 6th `pathStatuses` entry in `SettingsSectionNetwork.svelte`. `Settings.enhancedTrendingEnabled` + `feature_disabled` variant in `BrewErrorPayload` union in `types.ts`. `api.ts` IPC bindings (`trendingHistoryIndex`, `trendingHistoryFetch`).
- ✅ **Step 5** — Trending tab restructure: default sort velocity desc (was rank asc), new Velocity column with Flame/Snowflake/dash badges + numeric value, count cell becomes vertical-flex with inline `TrendingSparkline` beneath when enhanced trending is on. 8-col responsive grid, breakpoint-priority drops. New shared `TrendingSparkline.svelte` with `inline` + `detail` variants. New `trendingHistory.svelte.ts` store with sync lookup helpers.
- ✅ **Step 6** — PackageDetail integration: `loadDetail` fires `trendingHistory.ensureSeriesLoaded(name, kind)`; new `trend-card` section between description and AI blocks renders the `detail`-variant sparkline. Strictly passive per D4 — no placeholder when toggle is off, the section simply doesn't exist.

`npm run check`: 0 errors, 3 pre-existing warnings (v0.3.1 baseline).

### Done (Step 7 — commit `6901b64`)

- ✅ **trending-collector** for `brew-browser.zerologic.com`. Plain Node 20+ ESM, single dependency `better-sqlite3`. `tools/trending-collector/` directory with:
  - `lib/common.js` (SQLite schema, HTTP helpers, velocity math mirroring Rust, atomic JSON writes)
  - `lib/render.js` (regenerates `index.json` top-500 + per-package files with adjacent-day-subtraction-derived daily install estimates)
  - `seed.js` (one-shot bootstrap deriving 3 historical buckets per package from rolling-window subtraction + writes today's c30/c90/c365 as daily so next nightly run has a predecessor)
  - `collect.js` (nightly cron entrypoint, 12 concurrent HTTP GETs, INSERT OR IGNORE for idempotent same-day re-runs)
  - `README.md` (full deploy walkthrough)

`node --check` on every JS file: clean.

### Done (Step 8 — this commit)

- ✅ Memory bank + docs polish:
  - `projectbrief.md` nine → ten outbound paths; new item (j) for the opt-in endpoint
  - `decisions.md` new ADR `2026-05-26: Opt-in trust boundary for enhanced trending history (v0.4.0)`
  - `security.md` §16 full endpoint audit including the actual Caddyfile snippet + threat-model table + pre-launch checklist (the snippet IS the auditable artifact for the privacy claim)
  - `techContext.md` Trending data sources section rewritten
  - `backendApi.md` §13.14 v0.4.0 backend surface documented
  - `frontendComponents.md` v0.4.0 additions block
  - `docs/release-notes/0.4.0.md` (NEW) user-facing release notes
  - `README.md` outbound disclosure updated
- ✅ Task record `tasks/2026-05/19-v0.4.0-backend.md` expanded to cover Steps 1–8 (renamed in scope from "backend" to "ship").

### What's left

**Step 9 — Caddy deploy** on `brew-browser.zerologic.com`. Verbatim config in `security.md` §16.2; verification curl checklist in §16.6. Then bootstrap run (`node seed.js` on the box) → PR into main → v0.4.0 release.

## 2026-05-26 (Step 9 deployed, v0.4.0 PR'd)

Same day, three Caddyfile syntax iterations + three deploy-day bug fixes surfaced and resolved. Full file:line detail in `tasks/2026-05/19-v0.4.0-backend.md` (now spans Steps 1–9 with the verification narrative).

### Done (Step 9 — deploy verification)

- ✅ Collector deployed to `/home/michael/Sites/brew-trending-collector/`. `npm install --omit=dev` succeeded; `better-sqlite3` prebuilt binary loaded in ~3s.
- ✅ DB bootstrapped — `seed.js` inserted **383,581 rows** across 4 categories × 3 windows × 3 historical buckets + today's daily snapshot.
- ✅ Initial collect rendered 500 index entries + 18,028 per-package files in ~33s.
- ✅ Caddyfile updated: `handle_path /trending-history/*` handler + site-wide IP-redacted `log` block (`format filter { wrap json; fields { ... delete } }` worked on the third syntax attempt — earlier versions tried `format json { fields { ... } }` and then `log` nested inside `handle_path`, both wrong; pinned in `security.md` §16.2 with iteration history).
- ✅ Caddy reload wedged on log-file permission denied — fixed with `sudo chown caddy:caddy` + `sudo systemctl restart caddy` (reset-failed didn't unwedge the reload-notify job; restart bypassed it).
- ✅ Cron installed: `0 3 * * *`. Dry-run took 43s, pulled in 101 new rows beyond the seed.
- ✅ Pre-launch checklist from `security.md` §16.6: every curl returns expected (200 on index, 405 on POST, 200 on per-package, 404 on nonexistent). **`grep -cE 'remote_ip|client_ip|X-Forwarded-For|X-Real-Ip' /var/log/caddy/brew-browser.log` returns 0** — the auditable privacy artifact.
- ✅ Real leaderboard top after the velocity-formula fix: `hermes-agent` (v=1372), `raullenchai/rapid-mlx` (v=159), `grafana/gcx` (v=140), `openssl@4` (v=129) — genuine adoption signal.

### Deploy-day bug fixes that landed on the branch

1. **Velocity formula bias toward brand-new packages.** Old formula compared recent month vs whole-year average (recent month double-counted as baseline), so brand-new packages with c30 == c365 always returned the maximum 12.17 ratio. New formula compares recent month vs **prior 11 months** (c365 - c30). Updated in both Rust and JS byte-for-byte. +1 test pinning brand-new-package-returns-None. Tests 506 → 507.
2. **Cask URL needed `homebrew-cask` repo segment.** Plus cask items use `cask:` instead of `formula:` field — extractItems now normalizes both shapes.
3. **Path normalization `Sites/` vs `sites/`.** Server convention is capitalized (Mac convention preserved on the Ubuntu box). Sed-replaced across collector + memory-bank docs.

### Workflow learning

Caddy 2.x's log-filter syntax bit us three times. The deployed-as-is block is now pinned in `security.md` §16.2 with iteration history called out inline so a future reader doesn't accidentally revive an earlier broken draft from git history.

### Status

**Step 9 complete.** PR open against `main`. After merge: cut the v0.4.0 release via the standard pipeline (`sign-and-notarize.sh` → `publish-manifest.sh 0.4.0` → `gh release create` → `gh api PATCH` for asset rename → manifest rsync). Tauri-release gotchas reference: `~/.claude/projects/-Users-michael-Clean/memory/tauri_release_pipeline_gotchas.md`.

## 2026-05-27 (v0.5.0 ready for PR — opt-in vulnerability scanning)

Branch `feat/v0.5.0-vulnerability-scanning` off `main`. Full file:line detail in `tasks/2026-05/20-v0.5.0-vulnerability-scanning.md`. All 8 steps complete; PR follows the docs commit.

### Done (all 8 steps)

- ✅ **Step 1** — `Settings.vulnerability_scanning_enabled: bool` (default `false`, forward-compat tested). `state::AppState::require_vulnerability_scanning()` gate composing master paranoid with per-feature toggle. New `BrewError::VulnsNotInstalled { install_command }` variant so the frontend can route the user to the one-click installer affordance instead of a generic exit-non-zero toast. Five rejection paths pinned by tests.
- ✅ **Step 2** — `src-tauri/src/vulns/{client,cache,fingerprint,enrich}.rs` module (~2,100 lines). `client` shells out to `brew vulns --json`; `cache` is the persistent `vulns_cache.json` layer (1 MiB cap, atomic-write, 6h per-record TTL); `fingerprint` produces a deterministic SHA-256 over sorted `kind:name:version` lines via new `sha2 = "0.10"` + `hex = "0.4"` deps. Fail-soft on corrupt + future-schema.
- ✅ **Step 3** — Four IPCs (`vulns_scan_all(force)`, `vulns_scan_one(name)`, `vulns_install_helper`, `vulns_invalidate(kind, name, version)`). Gate composition pinned. `vulns_install_helper` intentionally bypasses the per-feature toggle (first-run flow is "install → toggle on → scan") but still respects the master paranoid gate via `require_network`.
- ✅ **Step 4** — GHSA enrichment via `vulns::enrich::enrich()`. Fetches `api.github.com/advisories/{GHSA_ID}` when (a) OSV record has a `GHSA-…` ID AND (b) `settings.github_enabled` is on AND (c) master paranoid gate is off (triple-defense). Parallel `ghsa_cache.json` (2 MiB cap). Best-effort: 403/429/network error leaves the OSV record unchanged and logs (no toast).
- ✅ **Step 5** — Frontend store `src/lib/stores/vulnerabilities.svelte.ts` (~350 lines). `byPackage` Map keyed by `"{kind}:{name}"`, `severityCounts` derived rollup, IPC wrappers, sync lookups for inline consumers. Error routing: `vulns_not_installed` → Settings card install affordance; everything else → `reportableToastError`. Types in `src/lib/types.ts`, IPC bindings in `src/lib/api.ts`.
- ✅ **Step 6** — UI surface: new `SettingsSectionVulnerabilities.svelte` opt-in subsection; Dashboard `Exposure` card with severity counts + ✓ clean-state framing; Sidebar count badge with max-severity tone; PackageRow inline severity dot; PackageDetail Security card with per-CVE rows + "Upgrade to fix" button wired to existing `brew_upgrade` pipeline. Cask rows render honest "Cask coverage isn't supported — brew vulns is formula-only" message.
- ✅ **Step 7** — Refresh-feed integration: post-`brew update` fan-out (Dashboard Refresh, Library Refresh) fires `vulnerabilities.scanAll(force=false)` so freshly learned upstream versions get scanned, with the install-set fingerprint skip predicate still applying when nothing changed. Post-mutation hooks (install / upgrade / uninstall) call `vulns_invalidate` + `vulnerabilities.scanOne(name)` so affected packages reflect the new state immediately.
- ✅ **Step 8** — Memory bank + docs (this commit): projectbrief.md ten → eleven outbound paths; decisions.md ADR `2026-05-27: Opt-in vulnerability scanning via brew vulns (v0.5.0)`; security.md §17 full endpoint audit + threat-model table + gate-composition table + pre-launch checklist; techContext.md (brew-vulns subprocess + sha2/hex deps + new "Vulnerability scanning" section); backendApi.md §13.15 (4 IPCs + module surface + 5 new wire types); frontendComponents.md v0.5.0 additions block (store + 5 component integrations + refresh-feed pattern); README outbound disclosure updated (path k); `docs/release-notes/0.5.0.md` (NEW); task record `tasks/2026-05/20-v0.5.0-vulnerability-scanning.md` (NEW).

### Tests & lint at PR-open (post-smoke-test cycle)

- `cargo test`: **585 passed**, 0 failed, 6 ignored (507 → 585, +78 new — the +6 over the original +72 is the captured-fixture suite from the smoke-test cycle)
- `cargo build`: clean — zero dead-code warnings
- `npm run check`: 0 errors, 3 pre-existing warnings (v0.4.0 baseline)
- `npm run build`: clean

### Smoke-test cycle (same day, post-docs commit)

Five integration bugs surfaced + fixed during the first live run on the user's 326-package install. Each required either a real `brew` subprocess or the actual `brew vulns` binary — none catchable by unit-test sandbox. Full table + lessons in `tasks/2026-05/20-v0.5.0-vulnerability-scanning.md` § "Smoke test cycle". Summary:

1. **`brew commands --include-aliases` errors without `--quiet`** (modern brew 5.x) — initial fix added the flag, then was superseded by #2.
2. **`brew commands` doesn't list external `brew-FOO` formula shims** — wrong install probe entirely. brew-vulns ships as a formula at `$(brew --prefix)/bin/brew-vulns`; switched to `brew --prefix brew-vulns` (clean exit 0/1, no output parsing).
3. **JSON severity is UPPERCASE** in wire (`"HIGH"`, `"MEDIUM"`, ...) — `#[serde(rename_all = "lowercase")]` does NOT case-fold on deserialize; ALL severities silently became `Unknown`. Custom `Deserialize` impl now case-folds and accepts `"MODERATE"` as a GHSA-flavored alias for `Medium`.
4. **JSON uses `fixed_versions: [String]`** (array, often empty) — not `fixed_in: String`. Custom `first_string_or_none` deserializer maps the array's first element into the existing `fixed_in: Option<String>` field; `summary: null` now explicitly normalized to `""` via `string_or_null`.
5. **`brew vulns --json` exits 1 when findings are present** (standard CI-scanner convention; exit 0 = clean, exit ≥ 2 = real error) — `run_brew_capture` rejected the JSON output. New private `run_vulns_capture` helper accepts exit 0 OR 1 as success, only typed-errors on ≥ 2.

Regression-pinned by `vulns::client::tests::raw_scan_result_parses_real_brew_vulns_output` using a captured fixture from real `brew vulns --json` output (augeas + openjpeg + p11-kit). All five failure modes commented at their trap sites in `vulns/client.rs` so future maintainers see the why.

**Lessons** (for the next subprocess-integration feature):

- `#[serde(rename_all = "lowercase")]` controls serialization, NOT case-folding on deserialize. Custom impl when the wire format is upper-case.
- `brew commands` only enumerates built-in + tap-resident subcommands; NOT external `brew-FOO` formula shims. Use `brew --prefix <formula>` for install detection.
- CI-flavored scanners (brew vulns, trivy, grype, ...) use non-zero exit codes as a *signal channel*, not a failure indicator. Always check for stdout before treating non-zero as fatal.
- A "defensive parse" without a captured fixture is just an aspiration — Step 2 declared `fixed_in?: String` and shipped; only the smoke test revealed the truth. **Capture the fixture early.**
- Subprocess integration cannot be unit-tested into correctness. Live smoke testing with the real binary is the only way to catch process-semantics bugs (#1, #2, #5) AND validate schema assumptions (#3, #4).

### Decisions locked

- **Subprocess over native OSV client** — inherits upstream fixes; correct attribution ("Powered by brew vulns"); internal interface preserves the escape hatch if upstream stagnates.
- **GHSA enrichment is best-effort** — UX nicety, not correctness requirement; soft-fail without toast keeps the scan reliable.
- **SHA-256 over `DefaultHasher`** — deterministic across runs / machines / Rust versions; DefaultHasher's salt randomisation would silently invalidate the cached fingerprint on every launch.
- **Opt-in** — adds an eleventh outbound path; first-launch posture preserved.
- **Casks honestly excluded** — no fake clean state; the UI explicitly tells the user the coverage gap.

### What's left

- Open the PR for `feat/v0.5.0-vulnerability-scanning` → review → merge.
- Cut the v0.5.0 release via the standard pipeline (same as v0.4.0).

## 2026-05-30 (launch day + Homebrew tap)

- ✅ **v0.5.0 launched on r/MacOS** (Saturday = the only day app posts are allowed there). Reposted as image-first after a text-only attempt underperformed. ~4.1K views in 48h, 80%+ upvote ratio (the predecessor "Homebrew Store" post was deleted by its author at 65%), engaged comments. Stars climbing (~45).
- ✅ **Issue #8 fixed** — window unmovable while Settings open. Root cause: the Settings scrim (`inset: 0`) covered the 36px title-bar `data-tauri-drag-region`, swallowing the mousedown macOS needs to start a drag. Fix: `inset: 36px 0 0 0`. PR #10, merged. Reported by @unluckyquote.
- ✅ **Linux support** — `feat/linux-support` branch (committed `282d8ff`, **not merged** by choice). keyring per-target features, Linuxbrew path detection, brew-cwd hardening (`exec.rs` pins cwd to `/`), xdg-open reveal, cask_icon cfg-gating, macOS-only menu, platform-aware labels, CI workflow. Verified building + running on arm64 Ubuntu 26.04 in a Parallels VM.
- ✅ **Homebrew tap** — new repo `msitarzewski/homebrew-brew-browser` with `Casks/brew-browser.rb` (signed/notarized .dmg, v0.5.0, sha256 verified). `brew tap msitarzewski/brew-browser && brew install --cask brew-browser`. `brew audit` exit 0, `brew fetch` sha256-verified. README + landing + projectbrief + decisions ADR (2026-05-30) updated.

### TODOs surfaced
- Cask `version`/`sha256` auto-bump in `tools/release/` (manual each release until then).
- Official `Homebrew/homebrew-cask` submission once past 75★.
- Title-bar/UX redesign (scoped with user, Option A: standard title bar, drop vibrancy) — not built; premature, parked.
- From-source `--HEAD` formula — deferred experiment; only ever on explicit user request.

## 2026-05-31 (native Swift / Liquid Glass rebuild — experiment branch)

Off-`main` experiment on branch `experiment/native-swift-liquid-glass`: a faithful **port** of the Tauri app's interface to Swift 6 + SwiftUI + Liquid Glass (macOS 26 Tahoe), in `native/` as a Swift Package. Motivation: answer the "Tauri isn't native" chatter with a real artifact, and evaluate Liquid Glass. Data sources + functionality stay identical to the Tauri app (port, not redesign). Decision in `decisions.md` (2026-05-30 ADR); full task record at `tasks/2026-05/22-native-swift-liquid-glass-rebuild.md`; build loop + source map in `native/README.md`.

### Done + building clean (`./build-app.sh debug`, 0 errors)

- **Dashboard** — feature parity, verified side-by-side vs Tauri (hero strip, Updates list, Composition bar/pie, Top-categories donut, Storage; responsive via `.onGeometryChange`).
- **Package detail inspector** — stock `.inspector(isPresented:)`, all 14 sections render live (meta, summary, homepage, categories, tags, Security, install-Trend sparkline, use-cases, similar, GitHub stars/forks, caveats, deps).
- **Settings** — `Settings {}` scene + `SettingsLink`, stock 9-tab `TabView` (Appearance / Network / GitHub / Brew / Updates / Security / Trending / Activity / About), every toggle wired to real systems.
- **Data layer** — 6 services ported (`BrewService`, `EnrichmentCatalog`, `AppSettings`, `VulnsService`, `GitHubService`, `TrendingHistoryService`) + `LocalPrefs`. `settings.json` shares the Tauri path/schema; `categories.json` + `enrichment.json` bundled (uncompressed). ~4,800 lines of Swift across 14 files.

### Pending

- **Library panel** — row→detail already wired; needs kind pills, sort, filters.
- **Discover / Trending / Snapshots / Services / Activity** panels (placeholders).
- **Dashboard GitHub "starred N of M" card** — reachable now Settings sign-in exists; needs a batch resolver.
- **Sparkle** for real in-app updates (the only genuinely-deferred subsystem; Updates auto-check toggle persists, install is a stub).
- **Vulns "scan all"** from Settings (`VulnsService` has `scanOne` only today).

### Toolchain + state

- SPM (`swift build`), not an Xcode project. Full Xcode installed but `xcode-select` → CommandLineTools, so `xcodebuild` unavailable; `swift build` links Liquid Glass fine. `native/build-app.sh` wraps the binary into a `.app`.
- **Stock Apple components only, no overrides** — the recurring lesson of the spike. `swift build` → 0 errors at end of session.
- **Not committed.** The entire `native/` tree is uncommitted on the branch; no commits past `main`. `main` (Tauri v0.5.0) untouched. Not yet visually verified beyond the panels noted above.

## 2026-06-01 (native rebuild — Library panel + fixes, committed)

First commit of the `native/` tree landed (`584f64f`, 2026-05-31) with a `.gitignore` keeping `.build/` + `BrewBrowser.app` out. Then the **Library panel** + a run of fixes, all user-verified and committed. Detail in `tasks/2026-05/22-native-swift-liquid-glass-rebuild.md`.

- **Library panel** — native SwiftUI `Table` (sortable columns: Name / Description[AI-gated] / Version / Type / Outdated), **centered** segmented type filter (All / Formulae / Casks / Outdated w/ counts), row→detail inspector. `LibraryFilter` enum + `LibraryRow` + `sortedLibraryRows` on `AppModel`. macOS-default `Table` + segmented control, no overrides.
- **Library crash fixed** — clicking the Library tab crashed (SIGTRAP). Real cause (found via `lldb` on `objc_exception_throw`, since the `.ips` hid it): **two `.searchable` in one toolbar** → duplicate `com.apple.SwiftUI.search` item. Collapsed to one shared toolbar search field.
- **Casks now load** — `loadLibrary` only listed formulae; added `BrewService.listInstalledCasks` + `listInstalledAll`.
- **Keychain 3→1** — `GitHubService.status()` did three separate `SecItemCopyMatching` reads (three prompts); new `keychainReadAll()` batches into one. (Per-rebuild prompt remains — ad-hoc signing; dev-loop-only, deferred.)
- **Lessons:** for SIGTRAP-via-`_crashOnException`, use lldb on `objc_exception_throw` (the `.ips` omits the reason); exactly one `.searchable` per toolbar; conditional `TableColumn` is unstable — branch into static-column `Table` variants.
- **Xcode MCP available** — 20 `mcp__xcode__*` tools connected this session. Caveat: they drive Xcode (`.xcodeproj`/scheme); this is SPM-only, so `BuildProject` may not map without opening `Package.swift` in Xcode. Build loop stays `./build-app.sh` until verified.
- **Still pending:** Discover / Trending / Snapshots / Services / Activity panels; Dashboard GitHub "starred N of M" card; Sparkle in-app updates; Vulns scan-all.
