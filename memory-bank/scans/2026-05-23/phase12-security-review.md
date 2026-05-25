# Phase 12 — Security Review

**Reviewer:** Security Engineer (pre-implementation)
**Date:** 2026-05-24
**Inputs:** `memory-bank/phase12-plan.md`, `memory-bank/security.md`, `memory-bank/decisions.md`, `src-tauri/capabilities/default.json`, `src-tauri/tauri.conf.json`, the existing sandbox/SSRF models in `src-tauri/src/commands/{disk_usage,cask_icon_homepage,info}.rs` and `src-tauri/src/trending/client.rs`.

**Net verdict:** Phase 12 plan is **APPROVED with the gates below**. Core architecture (bundled catalog, manual refresh, Device Flow, Keychain, paranoid master switch) is the right shape and preserves the project's posture. Gaps are implementation discipline — atomic writes, size caps, validator ordering, single-helper gates — not design flaws.

---

## Summary

- **Three Critical-before-merge gates:**
  1. **Paranoid mode** must be opt-out at first launch (defaults apply, gate off) BUT the gate file's *absence* and *corruption* must be distinguished — corrupt → fail closed, deny network.
  2. **Settings JSON write** must be atomic + size-capped (1 MiB) + path-bounded to `app_data_dir`.
  3. **GitHub token** must NEVER be returned to the frontend, NEVER logged, NO disk fallback if Keychain fails.

- **CSP needs three additions over Phase 12 lifetime:**
  - `https://api.github.com` (12c, runtime-gated)
  - `https://github.com` (12e, OAuth endpoints)
  - `https://formulae.brew.sh` already in CSP — no change for 12a

- **Two new validators required at IPC boundaries:**
  - Strict GitHub `owner/repo` validator (12c, attacker-influenced via homepage)
  - Issue body length cap + control-char strip (12f)

- **Quiet "default opens an outbound path" trap:** anonymous-GitHub toggle defaults off (good), but gate must enforce at the Rust command layer, not just behind the frontend setting. A future Settings UI bug must not silently re-enable network.

---

## Per-sub-phase findings

### 12a — Bundled catalog + manual refresh

**Network paths:** `https://formulae.brew.sh/api/{formula,cask}.json` — user-initiated only. Already in CSP. No capability changes.

**Required hardening:**
- **Hard cap raw response:** `MAX_CATALOG_BYTES = 64 MiB` before parse. Compressed catalog is ~10 MB; 64 MiB is 6× headroom.
- **Hard cap decompressed:** `MAX_DECOMPRESSED_BYTES = 128 MiB` via `flate2::read::GzDecoder` wrapped in `.take(MAX_DECOMPRESSED)`. Prevents gzip bomb.
- **Field-length caps:** `name ≤ 200`, `desc / homepage / deprecation_reason / disable_reason ≤ 4 KiB`. Post-parse validator or `serde(deserialize_with)`.
- **Atomic write:** `formula.json.gz.tmp` → `tokio::fs::rename` to final path. Crash mid-write must not produce a partial catalog. Add test that truncates temp then attempts rename.
- **Corrupt recovery:** `Catalog::load_user_data` returning `Err` → delete offending file → re-resolve to bundled. Surface to UI as banner: "Refreshed catalog was corrupt; restored bundled. [Refresh again]".
- **Path bounded:** catalog dir = `app_data_dir.join("catalog")` via a single helper; never composed from IPC input.
- **Concurrent refresh:** `is_refreshing: AtomicBool` (or `Mutex<()>`) — button is idempotent; surface `CatalogRefreshInProgress` error.
- **Lookup validation:** `validate_package_name` on `name` even though lookups are in-memory HashMap reads. Defense-in-depth + uniform IPC validation.

### 12b — Settings shell

**Network paths:** None. localStorage + `brew analytics on|off` shell-out only.

**Required hardening:**
- **`brew_get_analytics`:** parse FIRST line only of stdout with strict match (`stdout.lines().next() == Some("Analytics are enabled")`). Don't regex the whole output — `brew` may print warnings.
- **Enum validation on read:** `default-landing` and `vibrancy-material` validated against known sets on read; fall back to default on unknown.
- **Numeric input clamping:** "last N jobs" 1..=1000; "lines per job" 100..=10000. User input even though their own UI — out-of-range can DoS the renderer.
- **App version source:** `tauri::App::package_info().version` in Rust, not runtime `package.json` reads.
- **Plan for 12d migration:** localStorage values from 12b should migrate into the settings.json file (one-time migration; localStorage becomes a frontend cache). Plan now.

### 12c — GitHub anonymous tier

**Network paths:** `https://api.github.com/repos/*` — runtime-gated by Settings + Paranoid.

**CSP change required:**
```
connect-src 'self' https://formulae.brew.sh https://api.github.com
```
Note: don't add `api.github.com` conditionally — CSP is boot-set. Runtime gate is in Rust.

**CRITICAL validator — `parse_github_url`:**
The regex `^https?://github\.com/(\w+)/(\w+)$` from the plan is too loose. `\w` accepts `_` but not URL-encoded confusables. Implement as:
1. Parse with existing `parse_http_url` (rejects non-public hosts).
2. **Exact host match:** `host == "github.com"` case-insensitive. Reject `gist.github.com`, `raw.githubusercontent.com`, `github.com.evil.com`.
3. Split path on `/`. Require exactly `["", owner, repo]` after stripping trailing `/`, `/tree/…`, `.git` suffix.
4. Owner + repo: `^[A-Za-z0-9._-]{1,39}$` (GitHub's real rules). Reject leading `.` or `..`. Reject `..` segments entirely.
5. Path constructed as `format!("https://api.github.com/repos/{}/{}", owner, repo)` — with validator above, can't break out.

Cache filename uses same validated owner/repo (apply validator BEFORE building path, matching the Wave 3 audit's L1 lesson).

**Cache:** size-capped (1 MiB), atomic write, 24h TTL matching `ICON_CACHE_TTL`. Parse errors → cache miss + refetch.

**Rate limit:** `X-RateLimit-Remaining` parsed bounded (`u32::parse`), session cooldown surfaced as typed error.

**Paranoid pre-wire:** `github_repo_stats` consults `state.require_network(feature)` from day one, even though paranoid mode lands in 12d.

### 12d — Paranoid mode + network controls

**Architecture:** Single `state.require_network(feature: &str) -> Result<(), BrewError>` helper called as first line of every outbound command. Returns `BrewError::ParanoidModeBlocked { feature }`.

**ALL outbound commands must consult it:**
- `trending_fetch`
- `cask_icon_from_homepage`
- `catalog_refresh`
- `github_repo_stats`
- `github_device_code_start` and `github_device_code_poll` (sign-in itself is outbound)
- `github_star`, `github_unstar`, `github_is_starred`, `github_watch`, `github_unwatch`, `github_create_issue`
- Future: recipe fetches, tap fetches

**First-launch policy:** Paranoid mode OFF by default. App's current network is already user-consented (Trending requires opening tab, refresh requires click, homepage probes only on uninstalled cask details). Forcing paranoid ON would silently break Trending.

**Persistence loader — fail closed on corrupt:**
- File absent → defaults apply (paranoid OFF, this is first launch).
- File corrupt → **fail closed, paranoid effectively ON**, surface error to UI. Don't guess intent.
- Use `intent: PendingSettings | Loaded(Settings)` state to disambiguate.

**Settings JSON write:**
- Atomic (temp + rename, fsync dir)
- Size-cap before write (≤ 1 MiB)
- Schema validate every field; use `#[serde(default)]` for forward compat; log unknown fields to stderr ring
- Numeric clamp, enum revalidate, string length-cap

### 12e — GitHub Device Flow + Keychain

**Network paths:** `https://github.com/login/device/code` + `https://github.com/login/oauth/access_token`.

**CSP change required:**
```
connect-src 'self' https://formulae.brew.sh https://api.github.com https://github.com
```

**CRITICAL token rules:**
- **Token NEVER returned to frontend.** `GithubStatus` IPC returns `{ signed_in, username, scopes }` only. Add unit test that introspects `serde_json::to_string(&status)` and asserts no token-shaped string.
- **No disk fallback on Keychain failure.** Return `BrewError::KeychainUnavailable` to user. Add test that mocks Keychain failure and verifies no file in `app_data_dir` contains the token.
- **Token NEVER logged.** Newtype `Token(String)` with custom `Debug` impl that redacts. Deny `clippy::print_*` and `clippy::dbg_macro` in the github module.
- **Service identifier:** hardcoded `"dev.openbrew.browser"` matching `tauri.conf.json` bundle ID. Add test that parses tauri.conf.json and asserts match.
- **OAuth `client_id`:** hardcoded `const` in source (not env var). Device Flow client_ids aren't secret. Forks override the const.
- **Scope minimum:** `read:user public_repo` only. Documented per-scope comment in source. Test asserts no other scope in request body.

**Polling:**
- Honor server `interval` (typically 5s).
- On `slow_down` response, double the interval per RFC 8628 §3.5.
- Bounded by server's `expires_in` (typically 15 min); emit `DeviceFlowExpired` when exceeded.
- Single in-flight session enforced.

### 12f — GitHub authed actions

**Network paths:** All within `api.github.com` (already in CSP from 12c). No new origins.

**Required hardening:**
- **Reuse `parse_github_url` validator** for every command's owner/repo input.
- **Issue title:** cap 256 chars. Strip control chars (`\x00`-`\x1f` except `\t`).
- **Issue body:** cap 64 KiB. Strip null bytes. No HTML stripping (GitHub renders markdown).
- **Labels:** hardcoded allowlist (`["bug", "category-suggestion"]`) OR cap 10 labels with per-label rules. Pick one; document.
- **Auth-required gate:** check Keychain BEFORE constructing request URL. Don't let an anonymous request leak attempted action.
- **Scope-required gate:** cache scopes from token-exchange response; check before each call. Don't rely on 403 response.
- **Deeplink fallback URL-encoded** via `percent_encoding::utf8_percent_encode`, not manual `format!`.
- **Dashboard batch `github_is_starred`:** 50-permit semaphore (reuse `cask_icon_homepage.rs` pattern), 24h cache, skip non-GitHub-homepage packages, consult `require_network` once at top.
- **Rate-limit (403 + `X-RateLimit-Remaining: 0`):** surface `GithubRateLimited { reset_at }`. **No retry. No exponential backoff.** Honor only server's reset time.

**Cross-origin: catalog content as XSS vector?**
- PackageDetail's `desc`, `deprecation_reason`, `disable_reason` are server-controlled text. Svelte's `{value}` escapes by default — safe.
- **Re-verify** `grep -RnE '@html|innerHTML|outerHTML|insertAdjacentHTML|document\.write|eval\(|new Function\('` returns 0 at each merge.
- Add "(from Homebrew catalog)" attribution under any deprecation banner so user knows the text is sourced.

---

## Cross-cutting concerns

**Outbound paths grow from 4 → 7 by end of Phase 12.** Update README §"Open by default" with each sub-phase's PR. Don't batch.

**CSP updates batched:** Both new origins (`api.github.com` from 12c, `github.com` from 12e) land in 12c. Ship one CSP change, not two. (CSP changes require restart in production.)

**Single-helper chokepoints:**
- `require_network(feature)` — built in 12d, retroactively wired into 12a's `catalog_refresh` and 12c's `github_repo_stats`.
- `validate_github_repo_path(homepage)` — built in 12c, reused by 12f.
- `atomic_write(path, bytes)` — in `util/fs.rs`. Used by 12a (catalog), 12c (github-cache), 12d (settings.json). tmp + rename + fsync parent.
- `read_capped(path, max_bytes)` — used by 12a (catalog files), 12c (cache reads), 12d (settings.json).

**Audit gates:**
- `grep -rE 'reqwest::Client|reqwest::get' src-tauri/src/` reviewed by hand at 12f merge. Every hit either consults `require_network` or has a `// SECURITY: no-gate justified because ...` comment.
- `grep -RnE '@html|innerHTML|outerHTML|insertAdjacentHTML|document\.write|eval\(|new Function\('` → 0 at each sub-phase merge.

---

## Mandatory before-merge checklist

- [ ] CSP updated in `tauri.conf.json` (12c).
- [ ] `parse_github_url` rejects ≥ 15 attack cases.
- [ ] `require_network(feature)` exists; every outbound command consults it; per-feature unit test.
- [ ] `atomic_write` used for every JSON/binary write to app_data_dir.
- [ ] `read_capped` used for every JSON read with explicit max-byte limits.
- [ ] Settings: file-absent → defaults; file-corrupt → deny; atomic write; bounded to app_data_dir; 1 MiB cap.
- [ ] Catalog refresh: atomic; size-capped (raw + decompressed); single-flight; corrupt-recovery falls back to bundled.
- [ ] Token never returned to frontend: unit test asserts `GithubStatus` serialization has no token.
- [ ] Token never written to disk: unit test mocks Keychain failure and asserts no token-shaped string in `app_data_dir`.
- [ ] Token never logged: `clippy::dbg_macro`, `print_stdout`, `print_stderr` denied in `github` module.
- [ ] OAuth scopes: only `read:user` + `public_repo`. Test asserts no other in request body.
- [ ] Device flow honors `interval` + `slow_down` per RFC 8628 §3.5.
- [ ] All GitHub action commands validate owner/repo before building request.
- [ ] Issue creation: title ≤ 256, body ≤ 64 KiB, labels from allowlist, control chars stripped.
- [ ] Deeplink URLs URL-encoded.
- [ ] Batch `github_is_starred`: semaphore + cache + skip non-GitHub.
- [ ] Rate-limit: typed error, no retry, no backoff.
- [ ] README "Open by default" updated each sub-phase.
- [ ] No `{@html}` introduced.

---

## Recommended security.md additions

Append to `memory-bank/security.md` as each sub-phase lands. Group under new §13 "Phase 12 additions" so the Wave 3 verdict stays intact.

- **12a:** Bundled-catalog parsing trusted (built by us). User-refresh size-capped (64 MiB compressed / 128 MiB decompressed); corrupt → fall back to bundled; writes atomic; path bounded to `app_data_dir/catalog/`.
- **12a:** Disclosed outbound path: `https://formulae.brew.sh/api/{formula,cask}.json` — user-initiated only.
- **12b:** Settings split: localStorage (frontend, no secrets, enum-validated on read) + future `settings.json` (12d).
- **12b:** `brew_get_analytics` parses only first stdout line with strict match.
- **12c:** GitHub `owner/repo` from `homepage` is attacker-influenced. Strict allowlist validator: exact host `github.com`, owner+repo `^[A-Za-z0-9._-]{1,39}$`, no `..` segments. Runs before any cache path or URL construction.
- **12c:** Disclosed outbound path: `https://api.github.com/*`. Added to `connect-src`.
- **12c:** Repo-stats cache: 24h TTL, 1 MiB cap, atomic writes, bounded to `app_data_dir/github-cache/`.
- **12d:** Paranoid mode = single `require_network(feature)` helper. File-absent → defaults (paranoid off). File-corrupt → fail closed (deny network until user repairs Settings).
- **12d:** Settings JSON: 1 MiB cap, atomic writes, schema validated, `#[serde(default)]` for forward compat, numeric clamped, enums revalidated.
- **12e:** OAuth Device Flow (RFC 8628). `client_id` hardcoded const (not secret per Device Flow). Token stored in Keychain under `dev.openbrew.browser` / `github_access_token`. **Token never returned to frontend, never written to disk, never logged.** `GithubStatus` IPC returns `{signed_in, username, scopes}` only — verified by unit test.
- **12e:** Device-flow polling honors server `interval`; doubles on `slow_down`; bounded by `expires_in`; single in-flight session enforced.
- **12e:** Disclosed outbound path: `https://github.com/login/{device,oauth}/*`. Added to `connect-src`.
- **12e:** OAuth scopes: `read:user public_repo` only.
- **12f:** GitHub authed actions validate owner/repo via same allowlist. Title ≤ 256, body ≤ 64 KiB with control-char strip, labels from hardcoded allowlist.
- **12f:** Batch `github_is_starred`: 50-permit semaphore, 24h cache, skip non-GitHub packages, `require_network` once at top.
- **12f:** Rate-limit (403 + `X-RateLimit-Remaining: 0`) → `BrewError::GithubRateLimited { reset_at }`. No retry, no backoff.
- **12f:** Signed-out "Wrong?" deeplink URL-encoded via `percent_encoding`.
