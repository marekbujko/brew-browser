# Active Context

**Date:** 2026-05-30 (v0.5.0 shipped + launched on r/MacOS; Homebrew tap live)
**State:** v0.5.0 is released. Debuted on r/MacOS (Saturday — the day app posts are allowed there): ~4.1K views, 80%+ upvote ratio, engaged comments, +stars. Post-launch work landed: issue #8 fix (window draggable while Settings open, merged), a Linux-support branch (committed, not merged — `feat/linux-support`, verified building+running on arm64 Ubuntu), and a **Homebrew tap** so users can `brew install`.

## Repo

- **github.com/msitarzewski/brew-browser** — public, MIT
- **Released:** v0.1.0 → v0.5.0 (live on GitHub Releases — `gh release list`)
- **Tap:** `github.com/msitarzewski/homebrew-brew-browser` — `brew tap msitarzewski/brew-browser && brew install --cask brew-browser` (live, audited, fetch-verified)
- **Branch:** `main` (open: docs/brew-tap-install PR #11; feat/linux-support committed, unmerged by choice)
- **Stars:** ~45 (climbing post-launch)
- **Open issues of note:** #8 fixed (PR #10 merged). MacPorts backend requested on Reddit (scoped: core ops portable, trending/vulns/snapshots have no MacPorts equivalent).

## v0.5.0 shipped on the branch (Steps 1–8)

Full file:line detail + decisions + verification narrative in `tasks/2026-05/20-v0.5.0-vulnerability-scanning.md`. Bullet summary:

- **Step 1** — `Settings.vulnerability_scanning_enabled` (default `false`, forward-compat tested), `state::require_vulnerability_scanning()` gate composing master paranoid with per-feature toggle, new `BrewError::VulnsNotInstalled { install_command }` variant routing the user to the one-click installer affordance instead of a generic exit-non-zero toast. Five rejection paths pinned by tests (toggle off, paranoid on, paranoid-wins-over-toggle, FirstLaunch, Corrupt).
- **Step 2** — New `src-tauri/src/vulns/{client,cache,fingerprint,enrich}.rs` module (~2,100 lines). `client` invokes `brew vulns --json` via `tokio::process::Command`, parses with serde-default defenses, exposes `check_brew_vulns_installed` + `scan_all` + `scan_one` + `install_brew_vulns`. `cache` is the persistent `vulns_cache.json` layer (1 MiB cap, atomic-write, fail-soft, 6h per-record TTL). `fingerprint` produces a deterministic SHA-256 over sorted `kind:name:version` lines for the whole-scan skip predicate. New `sha2 = "0.10"` + `hex = "0.4"` deps.
- **Step 3** — Four IPC commands: `vulns_scan_all(force)`, `vulns_scan_one(name)`, `vulns_install_helper`, `vulns_invalidate(kind, name, version)`. Gate composition documented inline + pinned by tests. `vulns_install_helper` intentionally bypasses the per-feature toggle (first-run flow is "install → toggle on → scan"); still respects master paranoid gate.
- **Step 4** — GHSA enrichment via `vulns::enrich::enrich()`. Fetches `api.github.com/advisories/{GHSA_ID}` when (a) the OSV record carries a `GHSA-…` ID AND (b) `settings.github_enabled` is on AND (c) the master paranoid gate is off — triple-defense. Parallel cache at `ghsa_cache.json` (2 MiB cap). Best-effort: 403/429/network error leaves the OSV record unchanged and logs (no toast).
- **Step 5** — Frontend store `src/lib/stores/vulnerabilities.svelte.ts` (~350 lines). `byPackage` Map keyed by `"{kind}:{name}"`, `severityCounts` derived rollup, `scanAll` / `scanOne` / `installHelper` / `invalidate` wrappers, sync lookups for inline UI consumers (`maxSeverityFor`, `vulnsFor`). Error routing: `vulns_not_installed` → captured for Settings card install affordance; everything else → `reportableToastError`. Types ported in `src/lib/types.ts`, IPC bindings in `src/lib/api.ts`.
- **Step 6** — UI surface: new `SettingsSectionVulnerabilities.svelte` opt-in subsection mounted in `SettingsSectionNetwork.svelte` alongside Updates + Enhanced Trending History; Dashboard `Exposure` card with severity counts + Scan-now button + ✓ clean-state framing; Sidebar count badge with max-severity tone; PackageRow inline severity dot; PackageDetail Security card with per-CVE rows, severity pills, fixed-in ranges, "Upgrade to fix" button wired to existing `brew_upgrade` pipeline. Cask rows render honest "Cask coverage isn't supported — brew vulns is formula-only" message rather than fake clean state.
- **Step 7** — Refresh-feed integration: post-`brew update` fan-out (Dashboard Refresh, Library Refresh) fires `vulnerabilities.scanAll(force=false)` so freshly learned upstream versions get scanned. Post-mutation hooks (install / upgrade / uninstall in `packages.svelte.ts`) call `vulns_invalidate(kind, name, version)` + `vulnerabilities.scanOne(name)` so the affected package's CVE row reflects the new state immediately. The `force=false` parameter on the post-update scan means the install-set fingerprint skip predicate still applies — a refresh that didn't change install state won't re-shell `brew vulns`.
- **Step 8** — Memory bank + docs (this commit): projectbrief ten → eleven paths, decisions.md ADR, security.md §17 endpoint audit, techContext.md (brew-vulns + sha2/hex deps), backendApi.md §13.15, frontendComponents.md v0.5.0 additions block, `docs/release-notes/0.5.0.md`, README disclosure refresh, task record `tasks/2026-05/20-v0.5.0-vulnerability-scanning.md`.

## Tests & lint at PR-open

- `cargo test`: **585 passed**, 0 failed, 6 ignored (507 → 585, +78 new — the +6 over the original +72 is the captured-fixture suite added during the smoke-test cycle)
- `cargo build`: clean (zero dead-code warnings — every new symbol is wired)
- `npm run check`: 0 errors, 3 pre-existing warnings (v0.4.0 baseline)
- `npm run build`: clean Vite build

## Smoke-test cycle (2026-05-27)

Five integration bugs surfaced + fixed during the first end-to-end smoke test on the user's real install (326 packages, 11 vulnerable). Each required either a real `brew` subprocess or the actual `brew vulns` binary on disk — none were catchable by unit-test sandbox. Full table + lessons in `tasks/2026-05/20-v0.5.0-vulnerability-scanning.md` under "Smoke test cycle". Summary:

1. `brew commands --include-aliases` errors without `--quiet` (modern brew 5.x) — added `--quiet`, then superseded
2. `brew commands` doesn't list external `brew-FOO` formula shims — switched install probe to `brew --prefix brew-vulns`
3. JSON severity is UPPERCASE in wire — custom case-folding `Deserialize` impl (also accepts `MODERATE` → Medium)
4. JSON uses `fixed_versions: [String]` (array), not `fixed_in: String` — `first_string_or_none` deserializer maps array's first element into the existing `fixed_in: Option<String>` field
5. `brew vulns --json` exits 1 when findings present (CI-scanner convention) — new `run_vulns_capture` helper accepts exit 0 OR 1 as success, only typed-errors on ≥ 2

Regression-pinned by the captured-fixture test `vulns::client::tests::raw_scan_result_parses_real_brew_vulns_output` using real `brew vulns --json` output from the user's install. All five failure modes are commented at their trap sites in `vulns/client.rs` so future maintainers see why the fix exists.

## Decisions locked (per-decision rationale in task #20 + decisions.md ADR)

- **Why shell out to brew-vulns instead of native OSV query?** Inherits upstream fixes automatically; correct attribution ("Powered by brew vulns"); escape hatch via the internal interface if upstream stagnates. Cost: requires brew-vulns to be installed (we provide the one-click installer).
- **Why GHSA enrichment is best-effort?** GHSA enrichment is a UX nicety, not a correctness requirement. A 403/429/network error from `api.github.com` should not break the whole scan; we leave the OSV record unchanged and log without toasting.
- **Why install-set SHA-256 fingerprint?** `DefaultHasher` is non-deterministic across runs (HashDoS defense) — a hash recorded in v0.5.0 disk cache would mismatch every subsequent launch, silently invalidating the skip predicate. SHA-256 is deterministic across runs, machines, Rust versions.
- **Why opt-in?** Adds an eleventh outbound path (`api.osv.dev` via subprocess + `api.github.com/advisories` from our code). The first-launch posture stays "zero outbound beyond what the user has explicitly consented to."
- **Casks not supported.** `brew vulns` is formula-only; we render honest "coverage isn't supported" rather than fake clean state.

## Workflow note

Branch ready for PR. Follows the durable v0.4.0+ workflow: push branch → `gh pr create` → review → merge. No direct pushes to `main`.

## What's left

- Open PR for the v0.5.0 branch.
- Cut the v0.5.0 release after merge: `tools/build/sign-and-notarize.sh` → `tools/release/publish-manifest.sh 0.5.0` → `gh release create v0.5.0 ...` → `gh api PATCH` for asset rename → manifest rsync to `brew-browser.zerologic.com:Sites/brew-browser/updater.json`. Same flow as v0.4.0; Tauri-release gotchas in cross-session memory `tauri_release_pipeline_gotchas.md`.

## Memory bank inventory

`toc.md`, `projectbrief.md`, `techContext.md`, `decisions.md`, `activeContext.md` (this), `progress.md`, `systemPatterns.md`, `designSystem.md`, `uxArchitecture.md`, `visualStory.md`, `backendApi.md`, `frontendComponents.md`, `codeReview.md`, `apiTests.md`, `accessibility.md`, `realityCheck.md`, `security.md` (now through §17), `ideas.md`, `agentLog.md` (dormant), `NEXT-SESSION.md`, `tasks/2026-05/` (20 task records + README + deferred), `phases/`, `scans/2026-05-23/`.
