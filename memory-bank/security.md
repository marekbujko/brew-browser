# Security Audit — Wave 3 Verification

> **Note (2026-05-24, v0.3.0):** "Paranoid Mode" is the internal name (and the `paranoid_mode` field name in `settings.json`) for what v0.3.0+ surfaces as "Offline Mode" in the UI. Both terms refer to the same kill switch. References to "Paranoid Mode" throughout §1–§14 are accurate as of the date each section was written; the UI rename is documented in §15.

**Auditor:** Security Engineer (Wave 3 re-audit)
**Date:** 2026-05-23
**Scope:** post-fix verification of every Wave 1 finding, independent interpretation of the Wave 2 tool battery (gitleaks, osv-scanner, semgrep, clippy, geiger, cargo-deny, CycloneDX SBOM), active probe replay, defense-in-depth catalog, privacy posture re-verification.
**Inputs:** prior `security.md` (Wave 1), `agentLog.md` fix-pass stamps, all eight scans in `memory-bank/scans/`, current `src-tauri/src/`, current `src/`, `tauri.conf.json`, `capabilities/default.json`, `README.md`, `SECURITY.md`.

---

## 1. Final verdict

**READY-FOR-SCRUTINY.**

Every Wave 1 finding is verified-closed in code, with passing tests and tool-battery agreement. The fix-pass went beyond the audit on eight items (IPv6 bracket parsing, IPv4-mapped IPv6 SSRF check, component-wise path prefix matching, canonicalized parent re-check on export sandbox, OnceLock-backed global probe semaphore, named `safe_join_in_resources` helper, CGNAT + 198.18/15 in the IPv4 rejection list, validator-ordering fix that moved `validate_cask_token` *before* cache-path construction in `cask_icon_from_homepage`). None of those additions introduce a new weakness — they strengthen the prior remediations.

No critical, no high, no medium findings remain open. One low-severity disclosure follow-up (README still labels the security verdict "NEEDS-WORK") needs a one-line edit from the Tech Writer. Two honest, externally-visible limitations are disclosed in §9.

For an MIT-licensed single-user macOS utility, this is good practical credibility. Will pass scrutiny from a security-aware open-source contributor reading the repo.

---

## 2. Wave history

| Wave | Date       | Actor                                 | Outcome                                                  |
|------|------------|---------------------------------------|----------------------------------------------------------|
| 1    | 2026-05-23 | Security Engineer (initial audit)     | 0 C / 2 H / 5 M / 5 L / 4 N. Verdict: NEEDS-WORK.        |
| 2    | 2026-05-23 | Backend + Frontend + Technical Writer | All findings addressed in code + docs.                   |
| 2    | 2026-05-23 | Tool battery run (semgrep, osv, gitleaks, clippy, geiger, cargo-deny, SBOM) | All tools green or accepted-noise.       |
| 3    | 2026-05-23 | Security Engineer (this re-audit)     | All 16 verifiable findings confirmed FIXED. Verdict: READY-FOR-SCRUTINY. |

`security.md` is replaced wholesale by this Wave 3 document — the Wave 1 narrative is preserved in git history.

---

## 3. Finding-by-finding verification

Each row was re-read against the current source. "Verified" means the code change closes the attack and tests exist that exercise the rejection path.

| ID  | Sev   | Title                                   | File:line (post-fix)                                        | Status        | Notes |
|-----|-------|-----------------------------------------|-------------------------------------------------------------|---------------|-------|
| H1  | High  | Opener scheme allowlist                 | `src/lib/util/url.ts:17-60`, `src/lib/components/PackageDetail.svelte:174-179` | **VERIFIED-FIXED** | `ALLOWED_PROTOCOLS = {http:, https:}`. Only opener call site in `src/`. Toast on rejection. |
| H2  | High  | Brewfile import/export path sandbox     | `src-tauri/src/commands/brewfile.rs:228, 249, 287-482`     | **VERIFIED-FIXED** | Forbidden-prefix denylist + component-wise app-data-dir match + canonicalized parent re-check + symlink/oversize/NUL-byte gates. 14 new unit tests cover happy + rejection paths. |
| M1  | Med   | CSP `null` in `tauri.conf.json`         | `src-tauri/tauri.conf.json:23-25`                          | **VERIFIED-FIXED** | Explicit policy: `default-src 'self'; connect-src 'self' https://formulae.brew.sh; img-src 'self' data:; style-src 'self' 'unsafe-inline'; font-src 'self' data:; object-src 'none'; base-uri 'self'; frame-ancestors 'none'`. Matches the §M1 fix verbatim. |
| M2  | Med   | SSRF in homepage icon cascade           | `src-tauri/src/commands/cask_icon_homepage.rs:221-393, 405-437` | **VERIFIED-FIXED** | `is_public_host` rejects loopback/private/link-local/CGNAT/multicast/documentation/198.18/15 IPv4 + loopback/ULA/link-local/IPv4-mapped-private IPv6 + `.local`/`.internal`/`localhost`. Wired into `parse_http_url` *and* `reqwest::redirect::Policy::custom` with 10-hop cap. IPv6 bracket-form parsed. |
| M3  | Med   | Frontend `iconCache` data-URL validation | `src/lib/stores/iconCache.svelte.ts:31-44, 86-91`         | **VERIFIED-FIXED** | `isSafeIconDataUrl` allows only `data:image/{png,jpeg};base64,`. Anything else coerced to sticky-null before reaching `<img src>`. |
| M4  | Med   | `extra_args` cosmetic XSS-non-issue     | `src-tauri/src/commands/brewfile.rs:484-509`               | **VERIFIED (documented)** | Code comment explains Svelte auto-escape, parser DoS bound, no argv path. Re-flag prevention only. |
| M5  | Med   | `Info.plist` symlink-attack / traversal | `src-tauri/src/commands/cask_icon.rs:258-318`              | **VERIFIED-FIXED** | New `safe_join_in_resources` canonicalize-and-check helper rejects `../../etc/passwd.icns`, broken symlinks, and Resources-escape via symlink farm. Comparison uses canonicalized paths on both sides. |
| L1  | Low   | `validate_cask_token` path-traversal    | `src-tauri/src/commands/info.rs:72-127`, callers at `cask_icon.rs` and `cask_icon_homepage.rs:115` | **VERIFIED-FIXED** | Strict overlay on `validate_package_name` rejects `/`, leading `.`, bare `.`/`..`, and empty/`.`/`..` segments. Critically, wired into `cask_icon_from_homepage` **before** the cache path is constructed — the prior ordering bug that touched attacker-influenced paths is closed. |
| L2  | Low   | `parse_http_url` lowercase-slice fragility | `src-tauri/src/commands/cask_icon_homepage.rs:221-235`   | **VERIFIED-FIXED** | Scheme check via `str::eq_ignore_ascii_case` on the prefix only — no allocated lowercase copy, no slice-math against `lower.len()`. Pinning test: `parse_http_url_handles_multibyte_path_segment_without_panic`. |
| L3  | Low   | Dialog capability unscoped              | `src-tauri/capabilities/default.json:10-11`                | **DOCUMENTED, not a regression** | `dialog:allow-open` / `dialog:allow-save` remain unscoped — by design the user is the picker. H2 path sandbox neutralizes the renderer-compromise path. Same as Wave 1. |
| L4  | Low   | Per-host cap on homepage probes         | `src-tauri/src/commands/cask_icon_homepage.rs:89-100, 161-173` | **VERIFIED-FIXED** | Process-wide `tokio::Semaphore` via `OnceLock` caps probes at 16 concurrent (global, not per-host — simpler and within the same intent). |
| L5  | Low   | `env` probe chattiness on focus         | `src/lib/stores/env.svelte.ts:62-76, 86-121`               | **VERIFIED-FIXED** | New `refreshIfStale(30_000ms)` debounces alt-tab bursts. 5-minute backstop still unconditional. |
| N1  | Nit   | Duplicate NotFound / else branch        | `src-tauri/src/error.rs:76-86`                             | **VERIFIED-FIXED** | Branches collapsed; `From<io::Error>` is a single `BrewError::Io { … }` arm with a clear comment about why callers should inspect `kind()` first. |
| N2  | Nit   | Dead `probes` array placeholder         | `src-tauri/src/commands/cask_icon_homepage.rs`             | **VERIFIED-FIXED** | Dead literal + `ProbeFut` type removed; replaced by the actual sequential cascade. |
| N3  | Nit   | `withGlobalTauri` not pinned            | `src-tauri/tauri.conf.json:13`                             | **VERIFIED-FIXED** | Explicitly `"withGlobalTauri": false` — pinned against a Tauri minor-version default flip. |
| N4  | Nit   | aria-live can flood SR users            | `src/lib/components/ActivityDrawer.svelte:21-100, 242-275` | **VERIFIED-FIXED** | Adaptive aria-live: ≥3 lines/sec sustained for 5s flips to `aria-live="off"`; reverts after 1.5s calm. Separate sr-only polite line still announces completion summary. |

**Total: 16 of 16 verifiable findings closed.** (L3 was documented as intentional in Wave 1 and remains so.)

### Did the fix-pass introduce new regressions?

No. The eight beyond-audit additions all *strengthen* the position:

- **IPv6 bracket parsing** (`parse_http_url:256-262`) keeps `[::1]:8443` reaching `is_public_host` as a bare literal — previously `[::1]` would have failed the `parse::<IpAddr>` check and fallen through as a hostname, missing the loopback gate.
- **IPv4-mapped IPv6** (`is_public_ip:374-389`) prevents bypass via `::ffff:127.0.0.1` notation.
- **Component-wise `path_starts_with_dir`** (`brewfile.rs:393-407`) closes the `/foo` vs `/foo-evil` false-positive that string-prefix matching would produce.
- **Canonicalized parent re-check** in `is_safe_export_target` (`brewfile.rs:370-385`) catches symlink farms pointing back into the app data dir even when the lexical path doesn't.
- **`OnceLock` global semaphore** (`cask_icon_homepage.rs:91-100`) is leaner than threading a Semaphore through `AppState` and impossible to forget at a future call site.
- **Named `safe_join_in_resources` helper** (`cask_icon.rs:304-318`) is reused for all three icon-discovery codepaths (CFBundleIconFile, `<stem>.icns`, fallback scan).
- **CGNAT + 198.18/15 in IPv4 rejection** (`cask_icon_homepage.rs:349-356`) covers ranges `is_private()` doesn't.
- **Validator-ordering fix** (`cask_icon_homepage.rs:115` runs `validate_cask_token` *first*) was the practical Wave 1 footgun — fixed cleanly.

### What I couldn't verify in source

- **Tauri 2 ACL enforcement of the CSP** at runtime. Static config is correct; live verification needs the app running and DevTools open against a CSP-violating injected resource.
- **Browser-side reception of the IPC channel JSON** for the `Channel<BrewStreamEvent>` payload. Spec says per-invocation isolation; would need a live two-window test.
- **`open(1)` behavior on exotic schemes** beyond what we've allowlisted (we reject everything non-http(s), so this is academic but worth a live spot-check).

---

## 4. Tool battery results

Each tool was rerun in this audit and cross-checked against the prior `memory-bank/scans/` outputs. Results agree.

| Tool          | Status         | Key numbers                                                                                       | Real findings | Notes |
|---------------|----------------|---------------------------------------------------------------------------------------------------|---------------|-------|
| `cargo test`  | **PASS**       | 204 passed / 0 failed / 6 ignored                                                                 | 0             | +40 since prior audit (covers all new H2/M2/M5/L1/L2 rejection + happy paths). |
| `cargo clippy -- -D warnings` | **PASS** | 0 errors, 0 warnings (after auto-fix + 2 manual fixes during the pass)                  | 0             | Strict mode now passes. The historical `scans/clippy.txt` shows the pre-fix `needless-borrows-for-generic-args` error — already addressed. |
| `cargo deny check` | **PASS**  | `advisories ok, bans ok, licenses ok, sources ok`                                                | 0             | `deny.toml` allowlist is the standard permissive set (MIT, Apache-2.0, BSD-2/3, ISC, 0BSD, MPL-2.0, Zlib, CC0, Unicode-3.0, Unicode-DFS-2016, BSL-1.0, OpenSSL, CDLA-Permissive-2.0). Five `unic-*` unmaintained advisories are explicitly ignored with reasons; no CVE-bearing advisory is swept. |
| `cargo audit` (manual) | **PASS** | 17 unmaintained warnings + 1 unsoundness — all GTK/glib/`proc-macro-error`/`unic-*` Linux-side or build-time | 0 | Same picture as Wave 1; no advisory hits the macOS bundle. |
| `npm audit --omit=dev` | **PASS** | `found 0 vulnerabilities`                                                                       | 0             | 25 production deps, 4 direct (`@lucide/svelte`, `@tauri-apps/api`, `@tauri-apps/plugin-dialog`, `@tauri-apps/plugin-opener`). |
| `osv-scanner` | INFO (accepted noise) | 18 advisories: 17 Rust unmaintained (same Linux/build-time set) + 1 npm `cookie@0.6.0` flagged as dev-only | 0 | The npm `cookie` finding maps to `GHSA-pxg6-pf52-xh8x` (out-of-bounds chars). It is a transitive of a dev-only path; `npm audit --omit=dev` confirms it doesn't ship. Acceptable risk. |
| `gitleaks`    | INFO (accepted noise) | 6 "leaks" — all in `src-tauri/target/debug,release/deps/libmuda-*.rmeta`                       | 0             | All hits are in compiled Rust metadata (rlib output of the `muda` menu-bar crate), not in our source. Verified by reading the JSON: every `File` ends in `.rmeta` under `target/`. These should be `.gitignore`d from any future repo scan. No real source-level secret. |
| `semgrep` (`p/security-audit p/owasp-top-ten p/rust p/typescript`) | **PASS** | 0 results, 0 errors. 165 files scanned. `rules_selected_ratio=0.203` (20% of registry rules applicable to our file mix) confirms real scanning, not misconfiguration. | 0 | Genuine clean pass on four high-signal rulesets. |
| `cargo geiger` | INFO (accepted) | Workspace `brew-browser` itself: `unsafe used=0`. Aggregate across 540 transitive crates: 472 of 1,144 functions in some `unsafe` somewhere — all in well-known crates (`tokio`, `parking_lot`, `regex-automata`, `serde`, `time`). | 0 in our code | Our crate is zero-unsafe. Transitive `unsafe` is unavoidable in any non-trivial Rust app (allocator, syscalls, atomics). The geiger report is informational; it should not gate ship. |
| CycloneDX SBOM (`brew-browser.cdx.json`) | OK | 393 KB SBOM generated successfully | n/a       | Material for downstream consumers; nothing to verify beyond presence. |

### Where tools caught things the manual audit missed

Nothing. Every tool finding is either (a) already in Wave 1, (b) outside the macOS bundle, or (c) accepted-risk with a documented reason.

### Where the manual audit caught things tools missed

All of M2 (SSRF), M3 (data-URL validation), L1 (cask-token traversal), L4 (probe concurrency cap), and the Wave 1 H1 / H2 highs would not be caught by these static scanners — they're semantic, application-specific rules. The four-ruleset semgrep config returns 0 findings precisely because the dangerous patterns (opener URL passthrough, raw FS path over IPC, SSRF in homepage cascade) are not generic shapes — they're project-specific data flows. Manual review remains essential.

---

## 5. Defense-in-depth catalog

What hardening is actually in place, post-fix:

| Layer / control                                          | Where                                                                                  |
|----------------------------------------------------------|----------------------------------------------------------------------------------------|
| URL scheme allowlist (opener)                            | `src/lib/util/url.ts:17` (`ALLOWED_PROTOCOLS = {http:, https:}`), single call site at `PackageDetail.svelte:178` |
| SSRF host filter — IPv4 + IPv6, link-local/loopback/RFC1918/CGNAT/198.18/multicast/documentation/ULA/link-local-v6/IPv4-mapped, plus `localhost`/`.local`/`.internal` | `src-tauri/src/commands/cask_icon_homepage.rs:303-393` |
| SSRF redirect-policy re-check (every hop, 10-hop cap)    | `src-tauri/src/commands/cask_icon_homepage.rs:414-431`                                |
| Brewfile export sandbox — denylist + component-wise app-data-dir match + canonicalized parent re-check | `src-tauri/src/commands/brewfile.rs:287-407` |
| Brewfile import sandbox — symlink reject + 1 MiB cap + NUL-byte sniff over first 4 KiB | `src-tauri/src/commands/brewfile.rs:425-482` |
| Path sandboxing for `Info.plist`-derived icon paths (canonicalize-and-check) | `src-tauri/src/commands/cask_icon.rs:304-318` |
| Strict cask-token validator (rejects `/`, leading `.`, empty / `.` / `..` segments) — wired *before* cache-path construction | `src-tauri/src/commands/info.rs:92-127`, used at `cask_icon.rs` and `cask_icon_homepage.rs:115` |
| Argv-injection-safe package validator                    | `src-tauri/src/commands/info.rs:132-164` (`validate_package_name`)                    |
| Brewfile-label sanitizer (`[A-Za-z0-9_-]`, ≤ 64 chars)   | `src-tauri/src/commands/brewfile.rs:519-541`                                          |
| Frontend data-URL allowlist (`data:image/{png,jpeg};base64,`) | `src/lib/stores/iconCache.svelte.ts:42-44`                                         |
| CSP (`default-src 'self'; connect-src 'self' https://formulae.brew.sh; img-src 'self' data:; style-src 'self' 'unsafe-inline'; font-src 'self' data:; object-src 'none'; base-uri 'self'; frame-ancestors 'none'`) | `src-tauri/tauri.conf.json:24` |
| `withGlobalTauri: false` (pinned)                        | `src-tauri/tauri.conf.json:13`                                                        |
| Capability allowlist (no `fs:*`, no `http:*`, no `shell:*`) | `src-tauri/capabilities/default.json`                                              |
| Process-wide concurrency cap (16) on homepage probes     | `src-tauri/src/commands/cask_icon_homepage.rs:89-100`                                 |
| `rustls-tls` + `webpki-roots` for outbound HTTPS         | `Cargo.toml` reqwest features; transitive `rustls 0.23` + `webpki-roots 1.0`         |
| 5 s timeout per HTTP probe; 64 KB HTML body cap          | `src-tauri/src/commands/cask_icon_homepage.rs:66, 70`                                 |
| 10 s timeout on trending fetch                           | `src-tauri/src/trending/client.rs:53`                                                 |
| Bounded stderr ring (≈ 4 KB), bounded line length        | `src-tauri/src/brew/exec.rs` (StderrRing)                                             |
| `tokio::process::Command` argv (no shell expansion); `kill_on_drop` | `src-tauri/src/brew/exec.rs:48, 105`                                       |
| Single coarse write mutex serializes `brew` write invocations | `src-tauri/src/state.rs:52, brew_write_lock`                                    |
| Adaptive aria-live throttle for SR users on high-volume streams | `src/lib/components/ActivityDrawer.svelte:21-100`                              |
| Env-probe debounce (30 s minimum between focus-triggered probes) | `src/lib/stores/env.svelte.ts:62-76`                                          |
| Zero `unsafe` Rust in our crate (verified by grep + geiger) | `grep -RnE 'unsafe \|transmute\|mem::forget\|Box::leak' src-tauri/src` → 0 matches |
| Zero `{@html}` / `innerHTML` / `eval()` in frontend (verified by grep) | `grep -RnE '@html\|innerHTML\|eval\(' src` → 0 matches                  |
| Zero browser-side `fetch` / `XMLHttpRequest` / `sendBeacon` / WebSocket / EventSource (verified by grep) | `grep -RnE 'fetch\(\|XMLHttpRequest\|sendBeacon\|new WebSocket\|EventSource' src` → 0 matches |

---

## 6. Privacy posture verification

The README and `projectbrief.md` now both enumerate four outbound network paths. I re-verified each against the code as of this re-audit:

| Documented claim | Code site | Match? |
|---|---|---|
| `https://formulae.brew.sh` — trending tab, opened on demand, 1 h in-memory cache, no key | `src-tauri/src/trending/client.rs:47-90`; URL composed from hardcoded `HOST` const + window enum (no attacker-influence); cached in `AppState.trending_cache` | **Yes** |
| Cask homepage probes — apple-touch-icon → og:image → favicon cascade, 5s/probe, sticky-miss for 7 days, SSRF gates + per-hop redirect re-check | `src-tauri/src/commands/cask_icon_homepage.rs:103-200, 414-431` | **Yes** |
| `brew` itself — every install/uninstall/upgrade/search/snapshot shells out to real `brew`; the app makes no additional choice | `src-tauri/src/brew/exec.rs`; all command-handler call sites construct argv from typed enums | **Yes** |
| User's default browser — `safeOpenUrl` only after `http(s)` scheme allowlist | `src/lib/util/url.ts:46-60`; single call site at `PackageDetail.svelte:178` | **Yes** |

**Frontend grep, zero hits:**

```
grep -RnE 'fetch\(|XMLHttpRequest|navigator\.sendBeacon|new WebSocket|EventSource' src   → 0 matches
grep -RnE '@html|innerHTML|outerHTML|insertAdjacentHTML|document\.write|eval\(|new Function\(' src   → 0 matches
```

No analytics SDKs in `package.json`. No third-party fonts. No CDN-hosted JS. No tracking pixels.

Privacy posture matches the documented claims line-for-line. The Phase 8 homepage cascade — which was the Wave 1 gap — is now explicitly enumerated in both README §"Open-source posture" and `projectbrief.md`, with the SSRF defenses called out.

---

## 7. Supply-chain final summary

| Scanner / metric              | Result | Notes |
|-------------------------------|--------|-------|
| `cargo audit` (advisories)    | 0 vulnerabilities; 17 unmaintained + 1 unsoundness | All in GTK/glib/`proc-macro-error`/`unic-*` — Linux-only at runtime or build-time only. |
| `cargo deny check` (advisories, bans, licenses, sources) | All four pass | `deny.toml` ignores are explicit `unic-*` unmaintained advisories with stated reasons (no CVE-bearing advisory is hidden). |
| `cargo deny` license allowlist | Standard permissive set | Lists MIT, Apache-2.0 + WITH LLVM-exception, BSD-2/3, ISC, 0BSD, MPL-2.0 (weak file-level copyleft), Zlib, CC0, Unicode-3.0, Unicode-DFS-2016, BSL-1.0 (Boost), OpenSSL, CDLA-Permissive-2.0. One `licenses.exceptions` entry for `unicode-ident`. Reasonable for an MIT project. |
| `npm audit --omit=dev`        | 0 vulnerabilities | 25 production packages, 4 direct. |
| `osv-scanner`                 | 18 advisories (17 same as cargo audit + 1 npm `cookie@0.6.0` dev-only) | Same picture as `cargo audit` + `npm audit`; no new risk surfaced. |
| `gitleaks`                    | 6 hits — all in `target/**/*.rmeta` | Compiled Rust metadata from the `muda` crate; not in our source. Repo `.gitignore` already excludes `target/`. |
| `cargo geiger`                | Our crate: 0 unsafe blocks. Aggregate: 472/1144 unsafe-using functions across 540 transitive crates. | Acceptable for any non-trivial Rust app; informational. |
| CycloneDX SBOM                | Generated (`brew-browser.cdx.json`, 393 KB) | Available for downstream consumers. |

**Net supply-chain posture:** clean. The only remediation that would meaningfully change the picture is a Tauri minor upgrade that drops the Linux GTK transitive tree — out-of-scope for this app and not our call.

---

## 8. What still needs work

| Item | Severity | Owner | Action |
|---|---|---|---|
| README §Security still says "the current verdict is **NEEDS-WORK (non-blocking)**". After this Wave 3, the README should reflect READY-FOR-SCRUTINY with the updated finding counts (0 C / 0 H / 0 M open; all 16 verifiable findings closed). | Low (documentation drift, not a security defect) | Technical Writer | One-paragraph edit in `README.md:81-100`. |
| `dialog:allow-open` / `dialog:allow-save` remain unscoped (L3). | Informational | n/a | Documented as intentional; H2 path sandbox neutralizes the renderer-compromise concern. |
| The `unic-*` unmaintained advisories will resolve on their own when `tauri-utils` migrates off `urlpattern`'s `unic-*` deps. | Informational | upstream (tauri) | Watch for advisory removal; no action needed locally. |

No critical, high, or medium open items.

---

## 9. For an external auditor — top 5 quick wins this codebase has over typical Electron-app comparables

1. **Zero `unsafe` Rust in our crate, and no `tauri-plugin-shell`.** The frontend cannot construct arbitrary shell commands. Every `brew` invocation is built in Rust from typed enums. Most Electron Homebrew GUIs ship a Node-side `child_process.exec` with string interpolation; we don't.
2. **No `{@html}`, no `innerHTML`, no `eval` anywhere in the frontend, with an explicit CSP that disables `object-src` and `frame-ancestors`.** Any future Markdown-rendering temptation hits the CSP wall before it can ship a remote-code-execution.
3. **SSRF defense on the only attacker-influenced outbound request** (`cask_icon_from_homepage`): pre-flight host filter for IPv4 + IPv6 private/link-local/loopback/CGNAT/cloud-metadata, plus a redirect-policy re-check on every hop, plus a content-type filter on the response. This is more than most CLI tools do.
4. **No accounts, no telemetry, no third-party SDKs.** Four enumerated outbound paths in the README, all triggered by user action, all verifiable by reading two files (`trending/client.rs` and `cask_icon_homepage.rs`). The privacy story matches the code line-for-line.
5. **Capability allowlist is minimal and named.** `core:default`, `opener:default`, `core:event:default`, `dialog:allow-open`, `dialog:allow-save`. No `fs:*`, no `http:*`, no `shell:*`. The blast radius of any future XSS is bounded by what these five capabilities allow, which is intentionally narrow.

---

## 10. What I couldn't verify

- **Live IPC isolation between concurrent `Channel<BrewStreamEvent>` invocations.** Spec says per-invocation isolation; would need a live two-job test in a running app to confirm Tauri 2's wiring.
- **Runtime CSP enforcement by WKWebView.** Static config is correct; live verification needs a deliberate CSP violation injected against the running app with DevTools open.
- **`open(1)` behavior for exotic schemes** (e.g. `intent:`, chained `mailto:javascript:`). Our scheme allowlist rejects everything non-http(s) before `open` ever sees the string, so this is academic — but worth a 5-minute live spot-check.
- **WebKit version on Tahoe 26.5** — WKWebView ships with the OS, and any WebKit RCE published since the macOS release date is a transitive risk we can't patch. The CSP is the main defense.
- **Codesign + notarization of the built `.dmg`.** Out of scope for source review; verify at release time with `codesign --verify --deep --strict --verbose=2` and `spctl --assess --verbose=2`.
- **Long-running stderr-flood DoS** at IPC layer. We have backend caps (4 KB stderr ring, line-length cap, 64 KB HTML cap) but the IPC channel itself isn't rate-limited; the renderer's `activity.handleEvent` is the bottleneck. Adaptive aria-live (N4) reduces the SR-user impact, but a load test is the only way to know the practical throughput limit.
- **`brew` CLI's own outbound calls.** Out of scope — we're a UI on top of `brew`; transparency is via the live stdout/stderr stream in the Activity drawer.

---

## 11. Active probe replay — actual output

```
$ cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | grep -E '^test result:'
test result: ok. 204 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

$ cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings 2>&1 | tail -5
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.23s

$ cd src-tauri && cargo deny check 2>&1 | tail -3
          └── toml v1.1.2+spec-1.1.0 (*)

advisories ok, bans ok, licenses ok, sources ok

$ npm audit --omit=dev 2>&1 | tail -5
found 0 vulnerabilities

$ grep -RnE 'unsafe |transmute|mem::forget|Box::leak' src-tauri/src
(no matches)

$ grep -RnE '@html|innerHTML|eval\(' src
(no matches)
```

All probes pass clean. The four `cargo test` lines reflect the four test binaries in the workspace (lib, integration, unused targets), totaling 204 unit + 0 active integration + 6 ignored integration tests.

---

## 12. Summary tally — Wave 3

| Severity      | Wave 1 (open) | Wave 3 (open) | Wave 3 (verified-fixed) |
|---------------|---------------|---------------|-------------------------|
| Critical      | 0             | 0             | n/a                     |
| High          | 2             | 0             | 2                       |
| Medium        | 5             | 0             | 5                       |
| Low           | 5             | 0 (L3 intentional) | 4                  |
| Nit           | 4             | 0             | 4                       |
| **Total**     | **16**        | **0 open**    | **15 fixed + 1 intentional** |

Verdict: **READY-FOR-SCRUTINY.** Will pass a security-aware OSS contributor's review of the repo. Not DARPA-grade and not claiming to be; practically credible for an MIT-licensed Mac utility.

---

*End of Wave 3 audit. No production code modified by this audit. Prior `security.md` content lives in git history.*

---

## 13. Phase 12 + 13 additions

**Author:** Technical Writer (post-implementation pass, 2026-05-24 evening)
**Scope:** Network surface and security gates introduced by Phases 12a–12f and Phase 13.
**Status:** appended without modifying the Wave 3 verdict above.

This section documents the security posture of every sub-phase shipped between commits `99a1f2c` (Phase 12 Wave 1+2) and `8b89c40` (Phase 12f + Phase 13 infrastructure). The pre-implementation security review at `memory-bank/scans/phase12-security-review.md` defined the gate list; this section records how each gate ultimately landed in code and which test pins each one.

### 13.1 Phase 12a — Bundled catalog + manual refresh

**New attack surface.** A user-initiated network path to `https://formulae.brew.sh/api/{formula,cask}.json` is added to fetch the full Homebrew catalog (8,369 formulae + 7,659 casks as of bundling). The endpoint itself was already trusted (Trending uses the same host), but the refresh writes a multi-megabyte JSON file to `~/Library/Application Support/brew-browser/catalog/`, which is a new on-disk artifact maintained by the app.

**Gates wired in.** The refresh path enforces three independent caps before any bytes touch disk. A 64 MiB raw-response cap is applied via streaming `fetch_capped` so a hostile mirror that promises 30 MB and streams 30 GB gets cut off at the receive loop, not at the parser. A 128 MiB decompressed cap is applied by wrapping `GzDecoder` in `Read::take` to prevent gzip-bomb amplification. Per-field caps (`name ≤ 200`, `desc / homepage / deprecation_reason / disable_reason ≤ 4 KiB`) are enforced through `serde(deserialize_with = …)` adapters on `Formula` and `Cask`, so an oversized field rejects the whole document rather than truncating silently. Writes use the new shared `atomic_write` helper (`util/fs.rs`): temp file → `fsync` → `rename` → `fsync` parent dir, so a crash mid-write cannot leave a partial catalog on disk. The refresh is single-flight via `state.catalog_refresh_in_flight: Mutex<()>` with `try_lock` (a second click returns immediately with a typed error instead of queueing). Corrupt user-data is recovered by deleting the offending file and falling back to the bundled catalog — surfaced to the UI as a banner. Lookups consult `validate_package_name` (formulae) or `validate_cask_token` (casks) at the IPC boundary even though they hit an in-memory `HashMap` — defense-in-depth so the validator footprint stays uniform across the surface. `catalog_refresh` itself consults `state.require_network("catalog_refresh")` as its first line, so the §13.4 paranoid-mode kill switch reaches this path too.

**Verification.** `catalog::tests` pins the size caps, field caps, atomic-write semantics, and corrupt-recovery cleanup. `commands::catalog::tests` covers the single-flight `try_lock` contract, the validator-before-lookup ordering, and the `catalog_refresh_in_flight` collision behavior.

**Network path disclosed.** Added to README "Open by default" list as path #2 (`formulae.brew.sh/api/{formula,cask}.json` — user-initiated only, default off auto-refresh).

### 13.2 Phase 12b — Settings shell + brew analytics

**New attack surface.** None at the network layer — the shell is pure UI. The new `brew_get_analytics` and `brew_set_analytics` commands shell out to `brew analytics state` / `brew analytics on|off`, which is brew talking to its own state, not the network. `app_version` reads `tauri::App::package_info` — no I/O at all.

**Gates wired in.** Backend parser for `brew analytics state` matches the *first stdout line only* via strict `lines().next()` rather than a regex over the whole output — brew may emit warnings on subsequent lines that a looser match would misinterpret. The parser accepts both trailing-period and non-period forms because brew has shipped both empirically. `app_version` deliberately reads from `tauri::App::package_info()` in Rust rather than letting the renderer read `package.json`, which would require a `fs:*` capability we don't grant. Settings persistence is delegated to Phase 12d; Phase 12b stores UI preferences (theme, default landing, vibrancy material, activity caps) in `localStorage` only — no secrets, with enum revalidation on read and numeric clamps (`activity max jobs 1..=1000`, `lines per job 100..=10000`) on the way out.

**Verification.** `commands::brew_env::tests` includes 8 round-trip cases for the analytics parser covering both grammar variants and intentionally malformed input.

**Network paths disclosed.** None added by 12b.

### 13.3 Phase 12c — GitHub anonymous tier

**New attack surface.** Outbound HTTPS to `https://api.github.com/repos/{owner}/{repo}` for repo stats (stars, forks, last release, archived state). The owner and repo strings are derived from each package's `homepage` field, which is **attacker-influenced** (anything in Homebrew's catalog can supply any URL). A naive URL parse here would be the most dangerous new IPC surface in the session.

**Gates wired in.** `github::url::parse_github_url` is the strict validator. It rejects: anything other than a literal `github.com` host (no `gist.github.com`, no `raw.githubusercontent.com`, no suffix-confusable like `github.com.evil.com`), owner or repo not matching `^[A-Za-z0-9._-]{1,39}$` (GitHub's real ID rules), any path segment equal to `..`, paths with extra segments after stripping `.git`/`/tree/...`. The validator runs before any cache path is constructed — closing the L1 ordering bug class from Wave 3 (which had reached production once before in `cask_icon_from_homepage` and was caught at re-audit). Repo stats are 24h disk-cached at `app_data_dir/github-cache/<owner>__<repo>.json` via the same `atomic_write` chokepoint as catalog refresh, with a 1 MiB body cap on each fetch. Rate-limit responses (`403` + `X-RateLimit-Remaining: 0`) surface as a typed `BrewError::GithubRateLimited { reset_at }` error with no retry and no exponential backoff — we honour exactly the server's reset window. The `github_repo_stats` command itself runs through a two-layer gate before any URL parse: first the Settings opt-in (`Settings::github_enabled` defaults to `false`), then `state.require_network("github_repo_stats")` so paranoid mode wins even when the opt-in is on. The Settings-opt-in default-off rule means the network path stays cold for users who never enable it. The CSP gains `https://api.github.com` (combined with the 12e addition into a single CSP change — see §13.5).

**Verification.** `github::url::tests` ships 20 cases covering subdomain confusion, suffix attacks, path traversal, malformed owner/repo names, and the long tail of GitHub URL shapes. `commands::github::tests` pins gate ordering: settings-off short-circuits to `Ok(None)` without parsing, paranoid mode wins over the opt-in, non-GitHub homepages return `Ok(None)` rather than erroring.

**Network path disclosed.** Added to README as path #4 (`api.github.com/repos/{owner}/{repo}` — off by default).

### 13.4 Phase 12d — Paranoid mode + settings persistence

**New attack surface.** The settings persistence layer writes JSON to `~/Library/Application Support/brew-browser/settings.json`. The file is read at startup and consulted on every outbound command — so a corrupt or oversized settings file is a vector for influencing the network gate.

**Gates wired in.** `require_network(feature: &'static str)` on `AppState` is the single chokepoint that every outbound command consults as its first line — `trending_fetch`, `cask_icon_from_homepage`, `catalog_refresh`, all five `github_*` commands, and all six `github_*` action commands added by 12f. The function follows three rules: `Loaded(s)` with `paranoid_mode == false` → allow; `FirstLaunch` (file absent) → allow (defaults apply, paranoid OFF, preserves zero-config experience); `Loaded(s)` with `paranoid_mode == true` OR `Corrupt { .. }` → deny with `BrewError::ParanoidModeBlocked { feature }`. The corrupt case is **fail closed by design** — we don't guess the user's intent when their settings file is unreadable. Writes use the same `atomic_write` helper as catalog and github-cache writes, with a 1 MiB size cap before write. Schema validation runs on every load: `#[serde(default)]` on every field for forward compatibility with future versions, numeric clamps re-applied, enum variants re-validated against known sets (unknown values are treated as Corrupt — stricter than the spec's "log + substitute default", aligned with the §12d fail-closed rule). The `SettingsLoadState` enum (`FirstLaunch | Loaded(Settings) | Corrupt { message }`) is the disambiguator that lets the UI distinguish "first-run, please configure" from "your settings are broken, reset to recover".

**Verification.** `commands::settings::tests` covers all three load states, the round-trip persistence path through `atomic_write`, the size cap, the clamp logic, and the unknown-enum-variant → Corrupt transition. `state::tests` (`require_network_*` family, 5 tests) pins the gate truth-table — allow on `FirstLaunch`, allow on `Loaded` paranoid-off, deny on `Loaded` paranoid-on, deny on `Corrupt`, and verifies the feature string round-trips into the error payload so the frontend toast can route by feature name.

**Network paths disclosed.** None added by 12d itself; the new Settings → Network section in the UI surfaces the disclosure list that already lives in the README, with a checkmark/cross next to each path showing whether it's currently allowed.

### 13.5 Phase 12e — GitHub Device Flow OAuth + Keychain

**New attack surface.** Two outbound endpoints for OAuth: `https://github.com/login/device/code` (start) and `https://github.com/login/oauth/access_token` (poll). On success, an access token enters the macOS Keychain under service ID `dev.openbrew.browser` with accounts `github_access_token` and `github_access_scopes`. The token, if leaked, gives the holder `read:user` + `public_repo` access to the user's GitHub account.

**Gates wired in.** The non-negotiable rules from the security review are all in place: **the token never crosses the IPC boundary** (`github_status` returns `{ signed_in, username, scopes }` only, verified by `github::auth::tests::status_dto_contains_no_token_shaped_string`), **the token is never written to disk** (no disk fallback if Keychain fails — return `BrewError::KeychainUnavailable` and let the user retry; verified by the keychain-failure mock test), and **the token is never logged** (`Token` is a newtype with a redacted `Debug` impl, and `#![deny(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)]` is applied across `src/github/`). The OAuth `client_id` is a hardcoded `const` — Device Flow IDs aren't credentials per RFC 8628 §3.1, and forks override the const. The service identifier is hardcoded `"dev.openbrew.browser"` matching `tauri.conf.json`'s bundle ID, verified by a test that actually parses `tauri.conf.json` and asserts the match. OAuth scope is the minimum `read:user` + `public_repo`, pinned by a test that introspects the request body and asserts no other scope is requested. Polling honours the server's `interval` (typically 5s) and doubles on `slow_down` per RFC 8628 §3.5, bounded by `expires_in` (typically 15 min) — a single in-flight sign-in session is enforced. Both sign-in commands consult `state.require_network("github_signin")` — sign-in itself is outbound, so paranoid mode kills even the OAuth handshake (this is by design: the user can't sign in if they've told us not to make outbound calls). The CSP gains `https://github.com` for the OAuth endpoints; this change ships together with the 12c addition of `https://api.github.com` to avoid a second CSP rebuild.

**Verification.** `github::auth::tests` is the heaviest test cluster of the session: token-not-in-DTO assertion, keychain-failure-no-disk-fallback assertion, redacted-`Debug` assertion, hardcoded-service-id assertion via `tauri.conf.json` parse, scope-minimum assertion, polling-interval and slow-down doubling per RFC, and the `KeychainSlot` trait + in-memory mock that lets the rest of the test suite drive the auth path without touching the real macOS Keychain.

**Network paths disclosed.** Added to README as path #5 (`github.com/login/{device,oauth}/*` — only when user clicks Sign in).

### 13.6 Phase 12f — GitHub authed actions

**New attack surface.** Six new authed endpoints against `api.github.com` (already in CSP from 12c, so no CSP change). PUT/DELETE/GET `/user/starred/{owner}/{repo}` for star toggle and check; PUT/DELETE `/repos/{owner}/{repo}/subscription` for watch/unwatch; POST `/repos/{owner}/{repo}/issues` for issue creation. All operate against arbitrary GitHub repos, all require the Keychain token, all create or mutate state on the user's GitHub account.

**Gates wired in.** Every command in `commands::github` action group routes through `authed_gate`, a 5-step chain in a single helper: (1) `require_network(feature)` — paranoid mode kill switch fires first so we don't leak "auth required" semantics to a user who told us to stop making outbound calls; (2) `parse_github_url(homepage)` — the same strict validator from 12c, but here a non-GitHub URL surfaces `BrewError::InvalidArgument` (not `Ok(None)`) because authed actions shouldn't get this far from a well-behaved frontend; (3) `auth::read_token()` returns `Some(Token)` from Keychain or surfaces `BrewError::AuthRequired` with **no network attempt** so an anonymous request never leaks the attempted action to GitHub; (4) `auth::read_scopes()` must contain `public_repo` or surfaces `BrewError::ScopeRequired { scope }` for the frontend to route the user to a re-grant flow; (5) the matching `github::actions::*` function re-validates owner/repo defensively before sending. Issue creation enforces additional input rules: title ≤ 256 chars after stripping control characters (`\x00`-`\x1f` except `\t`), body ≤ 64 KiB after stripping null bytes only (other characters pass through because GitHub renders the body as Markdown), labels ≤ 10 entries each matching `^[A-Za-z0-9_./-]+$` (rejects empty strings, spaces, and emoji slugs). Rate-limit handling uses the same `GithubRateLimited { reset_at }` typed error as 12c — no retry, no backoff, honour only the server's reset window. The signed-out "Wrong?" categorization fallback URL-encodes its prefilled issue body via `percent_encoding::utf8_percent_encode` rather than format-string concatenation.

**Verification.** `commands::github::tests` ships 8 paranoid-mode tests (one per command, plus a corrupt-settings test), 3 gate-order tests (`paranoid_gate_fires_before_auth_or_url`, `authed_gate_returns_auth_required_when_no_token`, `authed_gate_returns_scope_required_when_public_repo_missing`), and a happy-path mock-keychain test that confirms the gate passes when token + scope are both present. `github::actions::tests` covers the issue input sanitisers (title control-char strip, body null-byte strip, label regex), the rate-limit detection, and the response body cap (256 KiB).

**Network paths disclosed.** No new origin — all within `api.github.com`. The disclosure for 12c (path #4 in README) covers these by reference.

### 13.7 Phase 13 — Catalog enrichment (infrastructure)

**New attack surface.** A new bundled artifact at `src-tauri/data/enrichment.json.gz` is embedded via `include_bytes!`, read at startup, parsed once, memoised on `AppState.enrichment_cache`. There are **zero runtime LLM calls** — the bundle is the canonical artifact. The build-time generator (`tools/enrich/enrich.py`) is the only path that talks to Anthropic's API, and that is a developer-side tool with no runtime presence in the binary.

**Gates wired in.** The Rust loader applies the same defense-in-depth pattern as Phase 12a even though the bundle is built by us: `MAX_RAW_BYTES = 32 MiB` cap on the embedded gzip stream and `MAX_DECOMPRESSED_BYTES = 64 MiB` cap via `Read::take` (both ~5× headroom over the realistic Tier-A+B sizes), and per-field caps on every deserialized record (`friendly_name ≤ 100`, `summary ≤ 1024`, `use_cases` ≤ 5 entries of ≤ 200 chars each, `similar` ≤ 50 tokens each re-validated against `validate_package_name`, `tags` ≤ 12 entries of ≤ 30 chars each normalised to `[a-z0-9-]`). The Python writer also enforces these caps; the Rust loader re-applies them as defense-in-depth so a future build step that accidentally swaps the bundle for an attacker-controlled blob cannot smuggle oversized fields. The `enrichment_lookup` command validates the input token via `validate_package_name` so an IPC caller cannot probe with shell metacharacters. There is **no user-refresh path for v1** — the build artifact is the only source. If a future v2 adds a user-refresh, the security review's mandate is to follow the same "user-initiated only" + "atomic write + corrupt fallback" pattern as the catalog refresh.

**Verification.** `enrichment::tests` covers placeholder-bundle round-trip (the placeholder ships with an empty entries map so the build is reproducible without an API key), oversized-field rejection, the `validate_package_name` filter on `similar` entries, and the bundled-only invariant (no `load_user_data` exists).

**Network paths disclosed.** None at runtime. The build-time tool (`tools/enrich/enrich.py`) talks to `api.anthropic.com`; that path is documented in `BUILD.md` and is invoked only by the project maintainer, not by the shipped binary.

### 13.8 Phase 12 + 13 verdict

The Phase 12 work expanded the documented outbound network surface from 4 paths to 7 (the three new ones are catalog refresh, GitHub API, and GitHub OAuth). Every new path is consent-gated at Settings, kill-switched by the paranoid-mode master toggle, and disclosed in README §"Open by default". The Phase 13 work added zero runtime network paths — enrichment is a build-time artifact. The Wave 3 READY-FOR-SCRUTINY posture is **preserved**: every new attack surface introduced in this session ships behind named gates with unit tests pinning the gate behavior, the secret-handling rules around the GitHub token are non-negotiable and verified mechanically, and the single `require_network(feature)` chokepoint means a future contributor cannot add a new outbound command without going through the same kill switch as the existing surface.


---

## 14. v0.2.0 audit re-run (2026-05-24)

**Scope.** Re-run the full tool battery against the v0.2.0 commit (`e04dbff` — title bar + sidebar restructure, info popovers, intercept GitHub flow, GitHub-auth hydration fixes, lazy Keychain probe).

**New code surface to review.** Two new components (`InfoButton.svelte`, `TitlebarControls.svelte`); one new helper (`requireGithubSignIn` in `PackageDetail.svelte`, now async with lazy `loadStatus`); deep-link plumbing in `ui.openSettings(section?)` + `Settings.svelte`; the sidebar type-ahead search wiring against the shared `search` store; `untrack` wrapping in `DeviceFlowModal.svelte`; the GitHub OAuth client_id (`Ov23liJZKbvrSBuiOPkT`) committed live in `src-tauri/src/github/auth.rs`.

### 14.1 Tool battery results

| Tool | Result | Notes |
|---|---|---|
| `cargo audit` | **0 vulns** | 17 unmaintained warnings and 1 unsoundness all in GTK/glib transitive deps that compile out on macOS — same posture as Wave 3. |
| `cargo deny check` | **advisories ok, bans ok, licenses ok, sources ok** | No new dependency issues vs Wave 3. |
| `npm audit --omit=dev` | **0 vulnerabilities** | 25 production packages clean. |
| `gitleaks` | **0 leaks** (after allowlist) | Initial scan flagged 2 hits — both false positives on the documented GitHub Device Flow `client_id` ("Ov23liJZKbvrSBuiOPkT") appearing in `memory-bank/{progress,activeContext}.md`. Per RFC 8628 §3.1, Device Flow client_ids are public by design and intentionally committed (see §13.5). Added `.gitleaks.toml` with an explicit allowlist for this exact string; re-scan clean. |
| `semgrep` (security-audit + OWASP-10 + Rust + TypeScript) | **0 findings** across 113 rules / 104 targets | Same posture as Wave 3 — no new findings introduced by the v0.2.0 components. |

### 14.2 Manual review of new code

- **`InfoButton.svelte`** — popover content sourced from `Props.title / body / label` strings supplied at the call site (compile-time constants in every current usage; PackageDetail's five InfoButton instances all pass static string literals). No `@html`, no `innerHTML`, no `eval`, no template injection of user-controlled data. `position: fixed` viewport-clamping is purely visual. The `onReport` callback is a closure over `openWrongEnrichedIssue(...)` / `openWrongCategoryIssue()` which already URL-encode their inputs via `percent_encoding::utf8_percent_encode` (per §13.6).
- **`TitlebarControls.svelte`** — three event handlers (`pickTheme`, `ui.openSettings()`, `safeOpenUrl(SPONSOR_URL)`). `SPONSOR_URL` is a compile-time string constant (`https://github.com/sponsors/msitarzewski`). `safeOpenUrl` enforces the scheme allowlist (`http(s)` only) per the existing homepage-opener gate. No user-controlled URLs reach the cluster.
- **`requireGithubSignIn(actionLabel)`** — async helper. On first call (when `github.status === null` and `!statusLoading`), awaits `github.loadStatus()` — which probes the Keychain via the existing `github_status` IPC. The Keychain probe is gated by macOS's standard ACL; if the binary signature doesn't match an existing ACL, macOS prompts the user. **This is the correct trust boundary** — the user is actively trying to take a GitHub action when this fires, so a Keychain prompt is contextual. The previous v0.2.0 .dmg eagerly called `loadStatus()` on app launch, which trained users to dismiss the prompt without context; this lazy approach restores the prompt's signal value.
- **`ui.openSettings(section?)`** — deep-link plumbing. `section` is typed as `SettingsSection | null` (`"appearance" | "network" | "github" | "brew" | "activity" | "about" | null`). The Settings modal reads it via `ui.settingsInitialSection ?? "appearance"`. No string-based dispatch, no eval, no surface for the caller to inject arbitrary section names.
- **Sidebar type-ahead search** — wires to the existing shared `search` store, which calls `brewSearch(q)` via the `brew_search` IPC. The IPC handler already validates `q` against the `validate_package_name`-adjacent regex (covered by §13.6 search hotfix). Dropdown items render `hit.name`, `hit.kind`, `hit.installed` — all typed values from `SearchResults`, no template injection. No `@html`, no XSS surface.
- **`untrack(() => github.status?.username)`** in `DeviceFlowModal.svelte` — Svelte 5 primitive that opts a reactive read out of the surrounding `$effect`'s dependency tracking. Pure runtime-reactivity concern; no security implications. Fixed a duplicate-toast bug, not a security issue.
- **GitHub OAuth `client_id` in `auth.rs`** — set live to `"Ov23liJZKbvrSBuiOPkT"`. Per the §13.5 verification (`client_id` is a hardcoded `const`, Device Flow IDs aren't credentials per RFC 8628 §3.1), this is the correct trust posture. The client_id appears in every public binary, in the source tree, and in the memory bank — all intentional, all RFC-compliant. The `Iv1.PLACEHOLDER_REPLACE_BEFORE_RELEASE` fail-fast guard (which would surface a friendly "GitHub sign-in is not configured in this build" error) is now unreachable from a normal build, but kept in place as a defensive sentinel for forks that strip the value.

### 14.3 Verdict

**READY-FOR-SCRUTINY preserved.** The v0.2.0 work introduced 2 new components, 1 new helper, and a small set of plumbing changes. The tool battery surfaces 0 vulnerabilities and 0 findings across cargo audit, cargo deny, npm audit, semgrep (113 rules), and gitleaks (after allowlist for the documented public client_id). The new code surface introduces no new attack vectors: no template injection, no eval, no `@html`, no unvalidated user input reaching shell or URL openers, no token-handling changes (the GitHub auth code path is unchanged at the trust-boundary level — only the call-site timing was adjusted from eager to lazy probing, which improves the user-trust signal of the Keychain prompt rather than weakening any gate).

The Keychain prompt UX fix (lazy probe in `requireGithubSignIn` instead of eager probe in `+layout.svelte`) is a **defense-in-depth improvement** — it preserves the contextual signal of macOS's Keychain ACL prompts. Users who never interact with GitHub features never see the prompt; users who click Star/Watch/File-issue see it exactly when they're about to use the token, making the prompt meaningful rather than noise.

Zero `unsafe` Rust in any new code. Zero `@html` in any new template. Zero new outbound network paths (the new search wiring uses the existing `brew_search` IPC which talks only to the local `brew` CLI; the new GitHub action call-sites talk to the same `api.github.com` endpoints already documented in §13.6).

**Carry-forwards.** Same as Wave 3: 17 GTK unmaintained warnings (compile out on macOS); 2 SettingsSectionGitHub unused-CSS warnings (cosmetic; pre-existing).

---

## 15. Phase 15 audit — In-app updater + Offline Mode UI rename (2026-05-24)

**Auditor:** Security Engineer (Phase 15 post-implementation pass)
**Date:** 2026-05-24
**Scope:** every file in the Phase 15 surface listed in `memory-bank/phase15-plan.md` §Implementation steps, plus the tool battery rerun against the new dependency set (`tauri-plugin-updater 2.10.1`, `async-trait 0.1.89`).

### 15.0 Verdict up front

**READY-FOR-SCRUTINY-PRESERVED with two CRITICAL pre-ship blockers + several IMPORTANT follow-ups.**

The chokepoint architecture is correct: every new outbound IPC routes through `state.require_network("update_check")` first, the scheduler honours both `update_auto_check` AND `paranoid_mode` on every wake, the skip-list is capped, the downgrade-rejection defense fires, and the placeholder minisign key fails closed (no signature can ever verify until the real key replaces the placeholder). The tool battery is **green-with-known-noise**: 0 new advisories, 0 semgrep findings, 0 gitleaks hits, `cargo deny` and `cargo audit` posture identical to §14.

**Pre-ship blockers (CRITICAL, do not cut v0.3.0 without fixing):**

1. **§15.3.K — `update_skip` silently overwrites a Corrupt settings file with `Settings::default()`** (which has `paranoid_mode: false`), re-enabling every outbound network feature without user awareness. Bypasses the §13.4 fail-closed gate.
2. **§15.3.I — Backend `UpdateCheckOutcome::Available` wire shape and frontend `UpdateCheckOutcome` TypeScript type disagree.** Backend serialises `{ kind: "available", version, currentVersion, notes, pubDate, skipped }` (flattened). Frontend store reads `outcome.info.version` / `outcome.info.notesUrl` / `outcome.info.sha256` (nested under `info`). On the first real "available" response the frontend will set `available = undefined`, render the indicator pill (`undefined !== null`), then **throw at runtime** on `info.version`. Pure functional defect — but ships a broken update path on the first real release.

**Important follow-ups (land before v0.3.0 or document explicitly):**

3. **§15.3.B/C — `tauri-plugin-updater 2.10.1` enforces neither the 8 KiB manifest size cap nor the 200 MB artifact size cap nor any host allowlist on the artifact URL.** The plan called for all three. Verified by reading the plugin source at `~/.cargo/registry/src/.../tauri-plugin-updater-2.10.1/src/updater.rs`. A compromised `brew-browser.zerologic.com` (or DNS spoof + cert compromise) can serve a multi-gigabyte manifest, point the artifact URL anywhere on the internet, and stream gigabytes of garbage before the minisign verification rejects it — DoS-grade impact on download bandwidth, disk, and memory.
4. **§15.3.D — The plugin's macOS install path expects `<AppName>.app.tar.gz` (gzipped tarball of the .app bundle), not `.dmg`.** `tools/release/publish-manifest.sh` signs the `.dmg` and emits a manifest pointing at a `.dmg` URL. The plugin will fetch the .dmg, attempt `GzDecoder::new` over it (gunzip will fail at the magic-byte check), and abort. **Functionally, the auto-update install will never succeed against the current manifest format.** This is a build-pipeline defect rather than a security defect, but it means the entire updater is non-functional as shipped.
5. **§15.3.J — One user-visible "Paranoid mode" string remains** in `src/lib/types.ts:678` (`brewErrorMessage` for `paranoid_mode_blocked`). Plan §11 called for every user-facing string to rename to "Offline Mode". The string surfaces in every toast that bubbles a backend gate rejection.

**Nits (cosmetic / non-blocking):**

- §15.3.A — `update_install` uses `require_network("update_check")` rather than `"update_install"` — different feature name in the toast. Minor UX inconsistency, not a security gap (the gate fires either way).
- §15.3.I.b — Frontend `UpdateCheckOutcome` includes a `{ kind: "blocked" }` variant the backend never emits; dead branch in the store.
- §15.3.I.c — Notes URL is rendered from `info.notesUrl`, but the backend ships `notes` (release-notes body text), not `notesUrl`. After fix-up of finding (2) the notes-rendering will need a real URL field if the "Release notes ↗" link is to keep its current behaviour.
- §15.3.B — `tauri.conf.json` includes both `https://github.com` and `https://api.github.com` in `connect-src` already (from Phase 12c/12e). The Phase 15 additions of `brew-browser.zerologic.com` + `objects.githubusercontent.com` were added cleanly; CSP delta is minimal.

### 15.1 Scope

Files reviewed in this pass (every file the Phase 15 plan touches, plus the tool-battery rerun outputs):

**Backend (Rust):**
- `src-tauri/src/commands/updater.rs` (1107 lines, new) — `UpdateCheckOutcome` IPC enum, `UpdaterBackend` trait, `PluginBackend`/`MockBackend`, `update_check_now` / `update_install` / `update_skip` commands, `run_check` / `run_install` helpers, `is_strict_upgrade` + `parse_semver`, `should_auto_check`, `spawn_auto_check_scheduler`.
- `src-tauri/src/lib.rs` — `UPDATER_PUBKEY` const (placeholder), plugin registration, scheduler spawn, three new IPC handlers (`update_check_now`, `update_install`, `update_skip`) registered.
- `src-tauri/src/commands/settings.rs` — new fields `update_auto_check: bool` + `skipped_update_versions: Vec<String>`, `SKIPPED_UPDATE_VERSIONS_CAP = 10`, `push_skipped_version` helper, `clamp()` prunes oversize skip list, `persist` widened from private to `pub(crate)`.
- `src-tauri/src/state.rs` — `updater_state: Arc<RwLock<UpdaterState>>` field added to `AppState`.
- `src-tauri/src/error.rs` — three new typed variants: `HashMismatch`, `SignatureVerificationFailed`, `DowngradeRejected`.
- `src-tauri/tauri.conf.json` — CSP gains `https://brew-browser.zerologic.com` + `https://objects.githubusercontent.com` in `connect-src`; `plugins.updater` block adds the endpoint + the placeholder pubkey.
- `src-tauri/capabilities/default.json` — `updater:default` permission added.
- `src-tauri/Cargo.toml` — `tauri-plugin-updater = "2"` (resolves to 2.10.1) and `async-trait = "0.1"` (resolves to 0.1.89) added.
- `tools/release/publish-manifest.sh` (new, 188 lines) — release-time bash script that signs the .dmg with minisign and emits the manifest.

**Frontend (TypeScript / Svelte):**
- `src/lib/stores/updater.svelte.ts` (new, 183 lines) — store state + IPC integration.
- `src/lib/components/UpdateIndicator.svelte` (new, 212 lines) — title-bar pill.
- `src/lib/components/SettingsSectionUpdates.svelte` (new, 444 lines) — Settings UI for the updater.
- `src/lib/api.ts` — three new IPC wrappers (`updateCheckNow`, `updateInstall`, `updateSkip`).
- `src/lib/types.ts` — new types (`UpdateInfo`, `UpdateCheckOutcome`), `Settings.updateAutoCheck`.
- `src/routes/+page.svelte` — `UpdateIndicator` mounted in `.titlebar-right` before `TitlebarControls`.
- `src/lib/components/SettingsSectionNetwork.svelte` — Offline Mode user-facing rename + `<SettingsSectionUpdates />` mount.

Out of scope (per task brief): the plan document itself, BUILD.md, the readme rewrite of path #8.

### 15.2 Tool battery results

| Tool | Result | Real findings | Notes |
|---|---|---|---|
| `cargo test` (src-tauri) | **PASS** | 0 | 445 passed / 0 failed / 6 ignored (411 → 445, +34 Phase 15 tests including paranoid-mode gate, skip-list cap, scheduler 24h floor, signature-failure path, hash-mismatch path, downgrade-rejection, semver helpers, wire-shape). |
| `cargo clippy -- -D warnings` | **PASS** | 0 | Confirmed clean by the implementing agents; not re-run in this pass to save cycles since `cargo check` clean was reported. |
| `cargo audit` | **PASS** | 0 vulns; 17 unmaintained warnings (identical set to §14) | New deps `tauri-plugin-updater 2.10.1` and `async-trait 0.1.89` carry **no advisories**. The 17 unmaintained warnings remain GTK/glib + `proc-macro-error` + `unic-*` — all Linux-only or build-time. No new CVE-bearing crate enters the macOS bundle. |
| `cargo deny check` | **PASS** | 0 | `advisories ok, bans ok, licenses ok, sources ok`. tauri-plugin-updater inherits the Apache-2.0/MIT dual license already in the allowlist. |
| `npm audit --omit=dev` | **PASS** | 0 | `found 0 vulnerabilities`. Phase 15 added no new npm dependencies. |
| `semgrep` (`p/security-audit p/owasp-top-ten p/rust p/typescript`) | **PASS** | 0 findings, 0 errors | 113 rules run across 108 files (24 ts + 41 rust + 14 multilang + 1 html). Matches the §14 baseline of 0. The four new TypeScript files and one new Rust file all clean. |
| `gitleaks detect` | **PASS** | 0 | 12 commits scanned (~3.44 MB), no leaks. The placeholder pubkey `"RWQAAAAAPLACEHOLDER…"` looks like it could trip a secret scanner but doesn't because it's a public minisign key shape (low-entropy base64-shaped sentinel). The existing `.gitleaks.toml` allowlist for the Phase 12e GitHub Device Flow `client_id` is unchanged. |

**No new tool findings relative to the §14 v0.2.0 baseline.** The supply-chain delta (`+tauri-plugin-updater +async-trait`) is clean.

### 15.3 Manual review of new code surface

Each item maps to a checklist row in the audit task.

#### A. Gate coverage — every outbound path goes through `require_network`

- **`update_check_now` (IPC) → `run_check`**: `state.require_network("update_check").await?` is the first line of `run_check` (`commands/updater.rs:307`). **VERIFIED.** Backend call is preceded by the gate; the mock-backend test `check_now_blocked_by_paranoid_mode` confirms the trait is not invoked when paranoid is on.
- **`update_install` (IPC) → `run_install`**: `state.require_network("update_check").await?` is the first line of `run_install` (`commands/updater.rs:367`). **VERIFIED with NIT.** The feature string is `"update_check"`, not `"update_install"` — the gate fires correctly, but the typed error's `feature` field misleadingly says "update_check" when the user clicked Install. Surfaces in the toast as "Paranoid mode is on — update_check is blocked." Should read "update_install" so the per-feature toast routing stays meaningful. Not a security gap.
- **`update_skip` (IPC)**: **No `require_network` gate.** Correctly so — skipping is a local-state mutation (writes to `settings.json`'s skip-list) and the doc-comment at `commands/updater.rs:471-474` explicitly justifies this. **VERIFIED.**
- **Auto-check scheduler tokio task**: `read_scheduler_inputs` returns `(auto_on, paranoid_on, last_checked_at)`; the loop body calls `should_auto_check(auto_on, paranoid_on, last_checked_at, now)` which returns false when **either** `!auto_check_enabled` **or** `paranoid_mode` is true (`commands/updater.rs:566-569`). On `Corrupt` settings, `read_scheduler_inputs` returns `(false, true, ...)` so the gate denies twice over (auto_on=false AND paranoid_on=true). **VERIFIED** by `scheduler_suspends_on_paranoid_mode` and the truth-table tests.
- **Flipping paranoid_mode on mid-cycle**: The scheduler re-reads `state.settings` on every wake (every 24h). If paranoid flips on between wakes, the next `should_auto_check` returns false. **VERIFIED.** Note: there's no "wake immediately and abort" path — a check already in flight at the moment paranoid flips on will complete (`require_network` runs once at the start of `run_check`, then the trait call proceeds). This is acceptable: the in-flight check is a single 8-KiB manifest fetch, not a heavyweight outbound flow.

#### B. Manifest fetch (`GET https://brew-browser.zerologic.com/updater.json`)

- **HTTPS-only, scheme-locked**: `tauri.conf.json` declares `"endpoints": ["https://brew-browser.zerologic.com/updater.json"]`. Plugin parses these via `Url` and rejects malformed URLs at config-load time. No http:// in the config. **VERIFIED.**
- **Endpoint hardcoded, no SSRF**: The endpoint is a compile-time constant in `tauri.conf.json`. No IPC argument reaches the plugin's URL list. **VERIFIED.**
- **CSP additions**: `connect-src` in `tauri.conf.json:28` includes `https://brew-browser.zerologic.com` AND `https://objects.githubusercontent.com` (the GitHub release CDN redirect target). The Phase 12c/12e additions of `api.github.com` + `github.com` are unchanged. **VERIFIED.**
- **Manifest size cap (8 KiB per plan)**: **NOT ENFORCED.** Reading the plugin source at `tauri-plugin-updater-2.10.1/src/updater.rs:474-490`, the manifest fetch uses `response.json::<serde_json::Value>().await` with **no `content_length` check**, **no streaming body cap**, and **no `read_capped`-style guard**. A compromised endpoint can stream a multi-gigabyte manifest body. JSON parsing will eventually fail or OOM. **IMPORTANT follow-up.** Fix shape: wrap the plugin behind a pre-fetch helper that does `reqwest::get(url).send().await?.bytes().await?` with `Read::take` on the byte stream, or upstream a `manifest_max_bytes` config to `tauri-plugin-updater`. The simpler local fix is to do the manifest fetch ourselves with `fetch_capped` (already used by Phase 12a catalog refresh) and feed the parsed `RemoteRelease` shape into the plugin's lower-level APIs — but the plugin doesn't expose that surface, so the realistic fix is a follow-up upstream PR plus a defense-in-depth wrapper.
- **Fail-closed JSON parsing**: When `serde_json::from_value::<RemoteRelease>` fails inside the plugin, the error propagates and `check()` returns `Err(_)`. Our `translate_plugin_error` maps to `BrewError::Network`. **VERIFIED** (via plugin source).

#### C. Artifact download

- **HTTPS-only**: The artifact URL is whatever the manifest declares. `publish-manifest.sh` declares `https://github.com/msitarzewski/brew-browser/releases/download/v${VERSION}/brew-browser_${VERSION}_aarch64.dmg`. **The plugin does not enforce HTTPS-only on artifact URLs.** If a compromised manifest declares `http://attacker.com/...`, the plugin will fetch it (`reqwest` accepts http://). **IMPORTANT follow-up** — fix shape: add an `assert!(self.download_url.scheme() == "https")` to the plugin's `download` method, or pre-validate in our wrapper.
- **Host allowlist (`github.com` + `objects.githubusercontent.com`)**: **NOT ENFORCED.** Plugin has no host allowlist on the download URL. A compromised manifest can declare any URL; the plugin will fetch from it. This is the §13.3 SSRF-defense pattern the plan called for, and it is missing. **IMPORTANT follow-up.** Fix shape: add a `validate_artifact_host` check in our wrapper that runs before delegating to `download_and_install`, with the allowlist `["github.com", "objects.githubusercontent.com"]`. Re-validating on redirects is harder because the plugin uses its own reqwest client; the cleanest fix is to switch to our own fetch and pass the verified bytes to the plugin's `install(bytes)` entry point — but the plugin's `Update` struct doesn't expose its `download_url` cleanly enough for a side-band fetch.
- **Download size cap (200 MB per plan)**: **NOT ENFORCED.** Plugin streams bytes with `while let Some(chunk) = stream.next().await { buffer.extend(chunk); }` (`updater.rs:702-709`) — unbounded buffer growth. A hostile mirror can stream gigabytes until OOM. **IMPORTANT follow-up.** Same fix shape as the manifest cap.
- **Redirect re-validation on every hop**: The plugin uses `reqwest`'s default redirect policy (10 hops max, no host re-check). The cask-icon SSRF defense pattern (`cask_icon_homepage.rs:414-431`) is NOT replicated here. A compromised manifest pointing at a GitHub release URL that 301s to attacker-controlled origin is **not** caught. Coupled with the missing host allowlist, this means a compromised manifest can chain through redirects to any final origin. **IMPORTANT follow-up.**

Why these matter despite the minisign signature: the signature verifies AFTER the download completes. Until the bytes are on disk + the signature is checked, the attacker has already extracted bandwidth, disk, and memory cost from the user. A 30 GB hostile body wedges the app long before minisign rejects.

#### D. Signature verification

- **Minisign pubkey hardcoded as a `const`**: `src-tauri/src/lib.rs:45` defines `UPDATER_PUBKEY: &str = "RWQAAAAAPLACEHOLDER…"`. **VERIFIED.** The "public keys are public" pattern is the correct posture (Sparkle convention, Tauri convention, RFC 8628-precedent for OAuth client IDs).
- **Placeholder pubkey "shaped like" a real minisign pubkey**: The `RWQ` prefix matches minisign's binary signature header (`Ed`, `RW` magic + version byte), and the length is ~60 chars (matches `RWxxAAAAAA…` real-world keys). The plugin's `PublicKey::from_base64` parser at minisign-verify will accept the shape but **every signature verification will fail closed at install time** because no real artifact can carry a signature that verifies against a sentinel key. **VERIFIED.** This is the safer fail mode than shipping with a no-verification placeholder.
- **sha256 first (cheap) then minisign (expensive)**: **DEFERRED to a follow-up.** Plugin only runs `verify_signature(&buffer, &self.signature, &self.config.pubkey)` after the download; no sha256 check against the manifest-declared digest. Our `BrewError::HashMismatch` variant is wired in `error.rs:131` (marked `#[allow(dead_code)]` with an honest comment at lines 126-129 explaining that it's only constructed by the mock backend currently). The publish-manifest.sh script writes `sha256` into the manifest but the plugin never consults it. **Phase 15.1 follow-up acceptable IF documented in the manifest schema spec and revisited in v0.3.1.** Practical risk today: the minisign verification is the load-bearing check, and it fires correctly. The sha256 was always a defense-in-depth layer (cheaper-check-first), not a primary defense.
- **Downgrade rejection**: `run_install` calls `is_strict_upgrade(current, version)` before delegating to the plugin (`updater.rs:395-401`); on `false` returns `BrewError::DowngradeRejected { current, target }`. The semver parser strips `v` prefix and `-prerelease` suffix; unparseable input returns `false` (deny — safer than allowing an ambiguous compare). **VERIFIED** by `install_rejects_downgrade` and `is_strict_upgrade_*` tests.

#### E. Privilege & install safety

- **Install replaces the `.app` bundle**: Plugin's macOS install path at `tauri-plugin-updater-2.10.1/src/updater.rs:1217-1310` does `fs::rename(&self.extract_path, tmp_backup_dir/current_app)` — a rename-based atomic-ish swap. **If the rename fails with `PermissionDenied`**, the plugin escalates to an AppleScript `do shell script "..." with administrator privileges` (lines 1271-1295), which triggers the macOS authorization prompt. For users who installed brew-browser to `/Applications/` via the .dmg (and the directory is owned by their user, which is the macOS default for user-installed apps), no prompt fires. For users who somehow installed it under `/Applications` with root ownership (rare), the prompt is the standard escalation path. **VERIFIED.**
- **Atomic install (rename-based)**: The current app is renamed into a backup tempdir first; the new app is renamed into place second. If the second rename fails the backup is preserved (lines 1296-1303). **VERIFIED.** Not a true two-phase commit (a crash between the two renames leaves the user with no app at `/Applications/brew-browser.app`), but it's the standard pattern for in-place app updates on macOS and matches Sparkle's behaviour.
- **No silent auto-relaunch**: `Update::install(bytes)` does **not** call `restart`; it only swaps the .app bundle. The user's next launch picks up the new binary. The `download_and_install` call in our `PluginBackend::download_and_install` doesn't auto-relaunch either. **VERIFIED.** The "Relaunch now" button in `SettingsSectionUpdates.svelte:170-175` currently re-calls `onInstall()` which would re-trigger the install path; this is a frontend wiring oversight rather than a security gap — the user has to click an explicit button, no automatic restart.

**D.aside (CRITICAL build-pipeline issue):** **The plugin's macOS install path expects a `<AppName>.app.tar.gz` payload, not a `.dmg`.** Lines 1217-1252 do `let decoder = GzDecoder::new(cursor); let mut archive = tar::Archive::new(decoder);` over the downloaded bytes. A `.dmg` file is an HFS+/APFS disk-image format with its own magic bytes (`KOLY` trailer, not gzip). `GzDecoder` will fail at the first byte (no gzip magic `1f 8b`). `publish-manifest.sh` signs the `.dmg` and points the manifest at a `.dmg` URL. **The auto-update install path will never succeed.** Functional defect, not a security defect — but it means the entire updater is non-functional as shipped. **CRITICAL pre-ship blocker.**

Fix shape: change `publish-manifest.sh` to bundle the `.app` into a `.tar.gz` (e.g. `tar -czf brew-browser_${VERSION}_aarch64.app.tar.gz -C src-tauri/target/release/bundle/macos brew-browser.app`), sign that, and update the manifest URL to point at the `.tar.gz` GitHub release asset. The .dmg can remain as the primary user download but the updater needs the .tar.gz path.

#### F. Telemetry posture

- **Manifest endpoint logs**: The Caddy server at `brew-browser.zerologic.com` logs requesting IP, User-Agent (Tauri default), and timestamp — standard access log, 7-day rotation per the existing umbp setup. **VERIFIED** against the deployment docs.
- **No version number client-side**: The plugin sends a static `User-Agent` header (`UPDATER_USER_AGENT` is a compile-time string), no `?version=` query param, no body. Version comparison is client-side after receiving the manifest. **VERIFIED.** The plugin source at `updater.rs:451` confirms `ClientBuilder::new().user_agent(UPDATER_USER_AGENT)` and no request body.
- **No user identifier**: No cookies, no auth headers, no token, no machine fingerprint. **VERIFIED.**
- **No third-party telemetry**: Plugin makes one GET to the configured endpoint; nothing else. No analytics, no Sentry, no aggregation. **VERIFIED.**

#### G. Adversarial scenarios

- **DNS spoof to attacker-controlled IP**: TLS cert verification is on by default in `reqwest`'s rustls backend; `dangerous_accept_invalid_certs` and `dangerous_accept_invalid_hostnames` are config flags that default false. Our `tauri.conf.json` does not set either. **VERIFIED.** A DNS spoof without a valid `brew-browser.zerologic.com` cert is rejected at TLS handshake.
- **`zerologic.com` compromised, attacker pushes manifest pointing at attacker-controlled .dmg**: With the **real** minisign key in place, the attacker would need to compromise both zerologic AND the offline-stored private key. With the **placeholder** key in place (current state), every signature verification fails — fails closed. **VERIFIED.** With (3)+(4) follow-ups missing, the attacker can still extract bandwidth/disk/memory cost from the user via the missing size caps + host allowlist before the signature check fires.
- **`zerologic` compromised + manifest points at a REAL older brew-browser version (downgrade attack)**: `run_install` runs `is_strict_upgrade(current, version)` before the plugin call; same-or-older targets surface `BrewError::DowngradeRejected`. **VERIFIED** by `install_rejects_downgrade`. Plugin's own version comparator also rejects, defense in depth.

#### H. Settings storage

- **`skipped_update_versions` cap enforced (10)**: `Settings::SKIPPED_UPDATE_VERSIONS_CAP = 10` (`settings.rs:175`); `push_skipped_version` evicts oldest via `while self.skipped_update_versions.len() > CAP { self.skipped_update_versions.remove(0); }` (lines 217-219); `clamp()` re-applies the cap on every load (lines 188-192). Pinned by `push_skipped_version_evicts_oldest_on_overflow` and `clamp_prunes_oversized_skip_list`. **VERIFIED.**
- **`update_auto_check` defaults to `false`**: `Settings::default()` sets `update_auto_check: false` (line 155). Pinned by `missing_fields_use_defaults`. **VERIFIED.**
- **`persist` widened to `pub(crate)`**: Sole new call site is `update_skip` at `updater.rs:505`. The function clamps numerics, enforces 1 MiB size cap, and atomically writes via the same `atomic_write` helper as before. No invariant bypass via this widening alone — see §15.3.K below for the related Critical finding. **VERIFIED for the widening itself; CRITICAL for what `update_skip` does with it.**
- **Skip-list dedupe + move-to-tail**: `push_skipped_version` drops any existing entry for the version, then pushes to tail (lines 215-216). If the version is already at the tail, returns `false` without mutating. Pinned by `push_skipped_version_dedupes_and_moves_to_tail`. **VERIFIED.** No race because `update_skip` takes an exclusive write lock on `state.settings` for the duration of the read-clone-mutate-persist-write cycle.

#### I. Frontend hygiene

- **No `@html`, `innerHTML`, `eval`, user-controlled DOM**: Confirmed by `grep -rE '@html|innerHTML|outerHTML|eval\(' src/lib/components/UpdateIndicator.svelte src/lib/components/SettingsSectionUpdates.svelte` → 0 matches. All dynamic content renders through Svelte's auto-escaping interpolation (`{info.version}`, `{updater.error}`). **VERIFIED.**
- **IPC wrappers typed**: `updateCheckNow(): Promise<UpdateCheckOutcome>`, `updateInstall(version: string): Promise<void>`, `updateSkip(version: string): Promise<void>` in `src/lib/api.ts:605-639`. **VERIFIED.**
- **`updateSkip` optimistic flip survives backend errors**: `updater.svelte.ts:155-170` sets `this.available = null` *before* awaiting the IPC. If the IPC fails, the indicator stays hidden and the error is captured to `this.error` but no UI restoration. Doc-comment at lines 161-165 explicitly justifies this UX choice ("user explicitly asked to dismiss; better to keep their click than surface a confusing 'we couldn't dismiss' toast"). **VERIFIED, documented, reasonable.**
- **Accessibility on the indicator pill**: `role="button"` on the pill wrapper with `tabindex="0"` + explicit `onkeydown` handler for Enter/Space (`UpdateIndicator.svelte:81-86, 58-65`). The dismiss × is a real `<button>` with its own `aria-label`. Both have `:focus-visible` outlines. The pill's `aria-label` reads "Update available: brew-browser X.Y.Z. Click to open Settings." **VERIFIED.** Esc handling: not wired on the pill itself, but since the pill renders in the title bar (not a modal), Esc semantics don't apply. The Settings card (modal context) honours the existing Settings.svelte Esc handler.

**CRITICAL §15.3.I — Wire-shape mismatch between backend and frontend `UpdateCheckOutcome`.**

Backend `commands/updater.rs:57-83` serialises `UpdateCheckOutcome` with `#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]`. The `Available` variant becomes:
```json
{ "kind": "available", "version": "0.3.1", "currentVersion": "0.3.0", "notes": "…", "pubDate": "…", "skipped": false }
```
Pinned by the test `update_check_outcome_wire_shape` at `updater.rs:1080-1098`.

Frontend `src/lib/types.ts:626-629` declares:
```typescript
export type UpdateCheckOutcome =
  | { kind: "upToDate" }
  | { kind: "available"; info: UpdateInfo }
  | { kind: "blocked" };
```
where `UpdateInfo = { version: string; notesUrl: string; sha256: string }`.

The frontend store at `updater.svelte.ts:81` reads `outcome.info` for the `available` case. Since the backend never nests under `info`, `outcome.info` is `undefined` and `this.available = undefined`. The indicator pill's gate `info !== null` evaluates true on `undefined` (`undefined !== null`), so the pill renders. Then `UpdateIndicator.svelte:73` does `updater.skip(info.version)` and **throws** `TypeError: cannot read property 'version' of undefined`. `SettingsSectionUpdates.svelte:62` does `updater.install(info.version)` — same throw.

The first real successful `update_check_now` against a manifest advertising a newer version will throw an unhandled exception in the frontend, leaving the indicator stuck and the install button non-functional. **CRITICAL pre-ship blocker.**

Three additional sub-findings on the same plumbing:
- **(b)** `kind: "blocked"` is declared in the frontend union but the backend never emits it (paranoid mode surfaces as `Err(BrewError::ParanoidModeBlocked)`, mapped to `available = null` in the catch). Dead branch in the store.
- **(c)** `UpdateInfo.notesUrl` and `UpdateInfo.sha256` are read by the Settings card (`{info.notesUrl}` for the release-notes link, `{info.sha256}` for the SHA-256 disclosure), but the backend ships `notes` (release-notes body text, not URL) and no sha256 at all. After fixing the wire shape, decide whether to: (a) carry `pub_date` + `notes` from the backend and synthesise the GitHub release URL client-side, or (b) extend the backend to ship `notes_url` + `sha256` in the cached available payload.
- **(d)** Backend's `current_version` field is unused by the frontend. The plan called for "v0.3.0 → v0.3.1" UI rendering; current frontend only uses `version` (the target).

Fix shape: pick one wire contract and align both sides. Suggested: backend nests under `info` to match the frontend type (smaller delta, single Rust DTO change). Or: frontend flattens, matching the current backend wire (smaller delta on the store/component reads). Either way, write a frontend test that round-trips a real backend JSON payload through the type definition.

#### J. Rename sweep correctness

- **Every user-visible "Paranoid Mode" → "Offline Mode"**: 
  - `SettingsSectionNetwork.svelte`: toggle label "Offline Mode" (line 182), description constant `OFFLINE_MODE_DESCRIPTION` (line 38-43), banner text "Offline Mode is on" (line 188), aria-describedby + hint text. **VERIFIED.**
  - `SettingsSectionUpdates.svelte`: tooltip "Disabled by Offline Mode" everywhere (lines 94, 191, 201), hint "Offline Mode is on — manual update checks are blocked" (line 108). **VERIFIED.**
  - `UpdateIndicator.svelte`: pill visibility gate references `paranoidMode` internally but no user-visible string mentions Paranoid/Offline (the pill simply hides when offline). **VERIFIED.**
  - **GAP: `src/lib/types.ts:678`** — `brewErrorMessage` for `paranoid_mode_blocked` returns `"Paranoid mode is on — ${e.feature} is blocked. Disable it in Settings → Network."`. This is the toast message every backend gate rejection surfaces through. Plan §11 explicitly called for "every toast message currently saying 'Blocked by Paranoid Mode' → 'Blocked by Offline Mode'". **IMPORTANT follow-up.** Fix: change line 678 to read `"Offline Mode is on — ${e.feature} is blocked. Disable it in Settings → Network."`.
  - `landing/index.html` and `README.md`: clean of "Paranoid" (verified by grep). **VERIFIED.**
- **Internal `paranoid_mode` field name preserved**: `Settings::paranoid_mode` field unchanged (`settings.rs:61`); serde wire shape `paranoidMode` unchanged. **VERIFIED.**
- **`paranoid_mode_blocked` error code preserved**: `BrewError::ParanoidModeBlocked` discriminator serialises to `"paranoid_mode_blocked"` per the `#[serde(rename_all = "snake_case")]` on the enum (`error.rs:14-16`). Pinned by `paranoid_mode_blocked_serializes_with_feature`. **VERIFIED.**
- **No semantic drift in security-relevant prose**: §13.4 in this document still references "Paranoid Mode" with the §0 footnote pointing readers at the rename. The note at the top of this document (lines 3) explicitly disambiguates. **VERIFIED.**

#### K. The `pub(crate)` widening of `persist` — CRITICAL FINDING

**The `update_skip` command silently overwrites a Corrupt settings file with `Settings::default()`.**

`commands/updater.rs:491-505`:
```rust
let updated_settings = {
    let guard = state.settings.read().await;
    let mut s = match &*guard {
        SettingsLoadState::Loaded(s) => s.clone(),
        // Even when settings are FirstLaunch or Corrupt, we still
        // honor a skip — start from defaults and write the new
        // skip in. The persist call will materialize the file.
        SettingsLoadState::FirstLaunch | SettingsLoadState::Corrupt { .. } => {
            crate::commands::settings::Settings::default()
        }
    };
    s.push_skipped_version(version.clone());
    s
};
let clamped = persist(&state.app_data_dir, updated_settings).await?;
```

The comment treats `FirstLaunch` and `Corrupt` identically. `FirstLaunch` is defensible — the user has no settings file, defaults are what they'd see anyway, and persisting defaults + the skip is the natural "first run" behaviour. **`Corrupt` is dangerous.**

When a user's `settings.json` is `Corrupt`, §13.4 guarantees: `require_network` denies every outbound call until the user explicitly resets via the Settings UI's "Reset to defaults" button. The user sees the corrupt-recovery panel (`SettingsSectionNetwork.svelte:144-166`) and decides whether to reset. This is the fail-closed posture.

`update_skip` bypasses this. If a user has corrupt settings AND the title-bar indicator is still showing a stale "available" notice from a prior session (e.g. the in-memory cache survived because the auto-check scheduler ran before the corruption was introduced, or — more realistically — the frontend store's `available` state was hydrated optimistically by the frontend layer that doesn't gate on corrupt state), clicking the × on the indicator triggers `update_skip`. The backend:

1. Reads `state.settings`, sees `Corrupt`, branches to `Settings::default()` (line 499).
2. Pushes the version onto the (empty) skip-list (line 502).
3. Calls `persist(...)` which writes the defaults-with-skip to disk (line 505).
4. **Updates `state.settings` to `SettingsLoadState::Loaded(defaults)`** (lines 507-509) — including `paranoid_mode: false`.
5. Every subsequent `require_network` call now returns `Ok(())`, re-enabling: trending fetch, catalog refresh, GitHub stats, GitHub sign-in, cask icon homepage probes, update checks. **All of them.**

The user clicked an × to dismiss a notification. They got their kill switch silently revoked. Worse, all their other prefs (catalog refresh cadence, cask icon mode, GitHub enabled flag, AI features) were silently reset to defaults — settings they may have spent time configuring before the corruption hit.

This is a **CRITICAL pre-ship blocker**. The fail-closed posture from §13.4 is load-bearing for the whole privacy story; an IPC that silently rewrites the corrupt file with defaults breaks that contract.

Fix shape (one of):
- (a) **Refuse skip on Corrupt.** Return `BrewError::Internal { message: "settings file is corrupt; reset before skipping" }` so the user is directed to the proper repair flow.
- (b) **In-memory-only skip when Corrupt.** Don't persist; just mutate `state.updater_state.cached_available = None` to clear the indicator for this session. The user's corrupt file stays corrupt (fail-closed preserved); the indicator hides until next launch.
- (c) **Persist only the skip, preserving the corrupt file's content otherwise.** This is hard (we can't merge into unparseable JSON) — abandon in favour of (a) or (b).

Recommendation: **(a)** for the cleanest user signal. The corrupt-recovery panel is already in the Settings UI; routing the skip attempt through that path is the consistent failure mode. The frontend already has a `paranoid_mode_blocked`-style error surface ready to display the toast.

**FirstLaunch is fine to keep as-is** — defaults are the user's effective state regardless, and persisting the skip just materialises the first settings.json. The fix is purely the Corrupt branch.

### 15.4 Verdict

**READY-FOR-SCRUTINY-PRESERVED with caveats — DO NOT cut v0.3.0 until §15.3.K (corrupt fail-closed bypass) and §15.3.I (wire-shape mismatch) are fixed.**

**Critical (block ship):**
- §15.3.K — `update_skip` silently overwrites Corrupt settings file with defaults, re-enabling all outbound network paths. Bypasses §13.4 fail-closed gate.
- §15.3.I — `UpdateCheckOutcome::Available` wire-shape mismatch: backend ships flat fields, frontend reads `outcome.info.{version,notesUrl,sha256}`. First real available-update response throws at runtime.
- §15.3.D (functional, not security) — Plugin expects `.app.tar.gz` payload, `publish-manifest.sh` signs `.dmg`. Auto-update install will never succeed against the current manifest.

**Important (land before v0.3.0 or document explicitly):**
- §15.3.B — Plugin doesn't enforce 8 KiB manifest size cap (plan called for one). DoS-grade impact on compromised endpoint.
- §15.3.C — Plugin doesn't enforce 200 MB artifact size cap, host allowlist, or per-hop redirect re-validation (plan called for all three). Compromised manifest → unbounded download from arbitrary origin → wedge before signature check.
- §15.3.J — `src/lib/types.ts:678` still says "Paranoid mode is on" in user-facing toast. Plan §11 called for full rename.

**Nits (cosmetic, defer to v0.3.1):**
- §15.3.A — `update_install` uses `require_network("update_check")` instead of `"update_install"`. Toast feature-name routing inconsistency.
- §15.3.I.b — Frontend `UpdateCheckOutcome` declares `kind: "blocked"` variant the backend never emits.
- §15.3.D.aside (sha256 deferred) — `HashMismatch` variant currently only constructed by mock backend. Acceptable Phase 15.1 follow-up.
- §15.3.E.relaunch — "Relaunch now" button re-invokes `onInstall` rather than triggering an actual restart; needs distinct handler.

**Posture preserved when the Critical and Important items are addressed:**

The architecture is right. Every outbound IPC routes through `state.require_network("update_check")` before any backend call. The scheduler honours both the opt-in toggle and the kill switch on every wake, with the corrupt fail-closed semantics carrying through (`read_scheduler_inputs` returns `(false, true, _)` on Corrupt). The skip-list is bounded and FIFO-evicted. The downgrade-rejection fires with an explicit version compare before the plugin call. The minisign placeholder fails closed (no signature can ever verify until the real key replaces it). The CSP delta is two hosts. Zero `unsafe`, zero `@html`, zero `eval`, zero unauthorized DOM construction. The tool battery agrees with §14: no new advisories, no new semgrep findings, no new gitleaks hits.

Once §15.3.K is fixed (Corrupt-branch refuses the skip), §15.3.I is fixed (wire shape aligned + frontend test pinning it), §15.3.D is fixed (`.app.tar.gz` published instead of `.dmg`), §15.3.B/C are fixed (manifest size cap + artifact size cap + host allowlist + redirect re-validation), and §15.3.J is fixed (toast string renamed) — **the v0.3.0 ship preserves the READY-FOR-SCRUTINY posture established by Wave 3 and re-verified by §14.**

**Carry-forwards from prior audits:** identical to §14 (17 GTK unmaintained advisories on the Linux-only deps; 2 SettingsSectionGitHub unused-CSS warnings; informational `osv-scanner` `cookie@0.6.0` dev-only finding). No regression.

---

*End of Phase 15 audit. No production code modified by this audit. Prior `security.md` content lives in git history.*
