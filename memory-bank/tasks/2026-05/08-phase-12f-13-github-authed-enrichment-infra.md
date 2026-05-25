# 2026-05-24 — Phase 12f + Phase 13: GitHub authed actions + catalog enrichment infrastructure

**Phase:** 12f + 13 + 9c (folded in)
**Status:** ✅ Shipped (Tier A enrichment bundle baked in subsequent `e1d6a87`)
**Commit:** `8b89c40` (31 files, +4,784 / -39)
**Date:** 2026-05-24 04:57

## Scope

Two parallel-OK tracks landing together: GitHub *authed* actions (star, watch, file issue — on top of the anonymous repo stats from `99a1f2c`) and the catalog *enrichment* infrastructure (build-time LLM enrichment of every package's friendly name + summary + use cases + similar + tags). Phase 9c ("Wrong?" GitHub-issue link for reporting bad categories/enrichment) folds into 12f as designed.

## What landed

### Phase 12f — GitHub authed actions
Six new authed endpoints against `api.github.com`:
- PUT/DELETE/GET `/user/starred/{owner}/{repo}` — star toggle + check
- PUT/DELETE `/repos/{owner}/{repo}/subscription` — watch / unwatch
- POST `/repos/{owner}/{repo}/issues` — issue creation

**`authed_gate` chain** — 5-step gate in a single helper, applied to every authed command:
1. `require_network(feature)` — paranoid mode fires first so we don't leak "auth required" semantics
2. `parse_github_url(homepage)` — the same strict validator from 12c, but here non-GitHub URL → `BrewError::InvalidArgument` (not `Ok(None)`)
3. `auth::read_token()` from Keychain → `Some(Token)` or `BrewError::AuthRequired` with **no network attempt**
4. `auth::read_scopes()` must contain `public_repo` or `BrewError::ScopeRequired { scope }`
5. matching `github::actions::*` function re-validates owner/repo defensively before sending

**Issue creation input sanitization:**
- Title ≤ 256 chars after stripping control chars (`\x00`-`\x1f` except `\t`)
- Body ≤ 64 KiB after stripping null bytes only (other chars pass through; GitHub renders body as Markdown)
- Labels ≤ 10 entries each matching `^[A-Za-z0-9_./-]+$` (rejects empty strings, spaces, emoji slugs)

**Rate-limit handling** reuses `GithubRateLimited { reset_at }` typed error from 12c. No retry, no backoff, honor server's reset window.

**Frontend:** Star button + Watch button + File issue button on PackageDetail, gated on `githubActionsEligible` (= signed in + github toggle on + GitHub homepage).

### Phase 9c — "Wrong?" GitHub-issue link (folded into 12f)
- `openWrongCategoryIssue()` helper in `PackageDetail.svelte`
- Signed-in users get an in-app `IssueModal` targeting `msitarzewski/brew-browser`
- Signed-out users get a deeplink to `github.com/.../issues/new?title=...&body=...&labels=category-suggestion`
- All URL-encoded via `percent_encoding::utf8_percent_encode` (not format-string concat) per §12f security review

### Phase 13 — Catalog enrichment infrastructure
- New bundled artifact: `src-tauri/data/enrichment.json.gz` (ships as placeholder at this commit; baked with real data in `e1d6a87`)
- `include_bytes!` embed, parsed once at startup, memoized on `AppState.enrichment_cache`
- **Zero LLM calls at runtime** — bundle is the canonical artifact
- New IPC: `enrichment_lookup(token)`, `enrichment_categories_visible()`
- Validation: `MAX_RAW_BYTES = 32 MiB`, `MAX_DECOMPRESSED_BYTES = 64 MiB` via `Read::take`
- Per-field caps: `friendly_name ≤ 100`, `summary ≤ 1024`, `use_cases` ≤ 5 entries of ≤ 200 chars, `similar` ≤ 50 tokens re-validated against `validate_package_name`, `tags` ≤ 12 entries of ≤ 30 chars normalized to `[a-z0-9-]`
- Python writer (`tools/enrich/enrich.py`) enforces same caps; Rust loader re-applies as defense-in-depth
- `enrichment_lookup` validates token via `validate_package_name` so IPC caller can't probe with shell metacharacters
- **No user-refresh path for v1** — bundle is the only source

### `search` hotfix (in the same commit)
- `brew search abcl` was returning "Search failed: brew_exit_non_zero" because `brew search --cask abcl` exits 1 (formula-only token)
- New `is_brew_search_no_match` helper tolerates per-kind no-match exits
- +4 backend tests pinning the pattern

## Files

31 files changed. Highlights:
- New: `commands/github/{actions.rs, ...}`, `commands/enrichment.rs`, `tools/enrich/{enrich.py, prompts/, requirements.txt}`
- Modified: `PackageDetail.svelte` (authed action buttons + Wrong? link + IssueModal mount), `api.ts`, `types.ts`

## Tests / verification

- `cargo test`: ~380 passing (+ ~50 from this commit alone — 8 paranoid-mode tests per command, 3 gate-order tests, happy-path mock-keychain test, issue input sanitizers, rate-limit detection, response body cap; plus 4 search hotfix tests; plus enrichment loader tests)
- `cargo clippy --all-targets -- -D warnings`: clean
- `npm run check`: 0 errors

## Notes / decisions

- **Tier A enrichment bundle ships as placeholder at this commit.** Real data baking takes 90+ minutes against Anthropic Haiku 4.5 (~$3-5) and runs in background later in the session — bundle artifact updated in `e1d6a87`.
- **Tier B enrichment** (use_cases / similar / tags per package, ~$10-15) is queued; not run yet.
- The 5-step authed gate is the **non-negotiable contract** for any future GitHub-authed feature. Future contributors cannot add an authed command that skips a step without explicit test failures.
- Issue body cap of **64 KiB** is generous for GitHub but bounded — without a cap the IPC could be used to spam a malicious enormous issue.
- Network paths disclosed: No new origin (all within `api.github.com`). The 12c disclosure (README path #4) covers these by reference.
- **`AbortError` issue with mock fetch in tests** required `let result = match result { ... }` style for the test mock; documented in `agentLog.md`.
