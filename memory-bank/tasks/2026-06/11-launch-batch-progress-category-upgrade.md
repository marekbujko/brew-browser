# 11 — Launch batch: upgrade-all fix + #58 + #57, native test target, security pass

**Date:** 2026-06-07
**Branch:** `feat/launch-batch-progress-category-upgrade` (off `main`, post #56/#51 merge)
**Both builds.** Ships together as one patch.

Cross-cutting batch driven by the v0.5.0 Reddit launch feedback + an issue
firehose. Three features, a new native test target, and a pre-release security
pass.

## ① Upgrade-all misclassification (the firehose — ~20 issues)
~20 open "[brew-browser] Upgrade-all failed" reports were one bug: `brew
upgrade` exits 1 on **non-fatal** warnings (post-install warnings, link
conflicts, already-linked kegs, "already a Binary … skipping link") even though
the work completed, and the app treated any non-zero exit as failure → scary
toast + file-an-issue CTA.
- Rust: `error_patterns::upgrade_warnings_only()` classifier + Pattern 4
  (concurrent-lock hint). `exec.rs` now resolves warnings-only upgrade exits as
  **success** (no failure toast, no report CTA); warnings still stream to
  Activity. Hard-fatal signatures (download/checksum/lock/etc.) still fail.
- Native: `BrewOutputParsing.upgradeWarningsOnly` + `startJob` treats them as
  `.succeeded`.
- Closes #55,#53,#52,#50,#49,#40,#38,#35,#34,#33,#32,#30,#29,#28,#24,#23,#22,#21,#18,#12.

## ② #58 — category click filters the Library (was a bug)
"Top categories in your library" jumped to the full Discover catalog. Fix:
- Tauri `Dashboard.jumpToCategory` → `setSection("library")` (Library already
  filters installed by `discover.selectedCategories`).
- Native: new `libraryCategory` filter + removable chip (`ContentView`),
  Dashboard legend tap → `jumpToLibraryCategory()`.

## ③ #57 — operation progress counts (top Reddit ask, 55▲)
Determinate "Pouring foo (10 of 32)" from brew's `==>` markers.
- Rust: real `BrewStreamEvent::Progress`, `ProgressParser` in `exec.rs`,
  progress bar in `ActivityDrawer.svelte`.
- Native: `BrewProgressParser` + `JobProgress` + linear bar in `ActivityView`.

## Native test target (0 → 36 tests)
First-ever native tests. `Package.swift` gains `BrewBrowserKitTests`
(Swift Testing, `@testable`). Covers the parity-critical pure logic, fixtures
mirroring the Rust tests: `friendlify`, `upgradeWarningsOnly`, `BrewProgressParser`,
`VulnSeverity`, `VulnsService.parseScanOutputKeyed` (the no-smear keying — the
1500-CVE bug), `CategoryCatalog`, `SettingsDTO` forward-compat, + adversarial
fuzz (no panics). Two small production tweaks to enable tests: extracted
`parseScanOutputKeyed`; `SettingsDTO` `private`→`internal`.
Rust side gained matching fuzz/robustness tests (607 → 609+).

## Pre-release security pass
Tools: cargo audit, npm audit, osv-scanner, gitleaks, semgrep + manual review.
- **0 vulnerabilities** (cargo audit); **semgrep 0 findings**; gitleaks 2 false
  positives (suppressed via `.gitleaksignore`).
- Manual: command-injection (typed-arg `Command`/`Process`, no shell;
  `validate_package_name` at every entry), path traversal (`validate_brewfile_id`
  / `SnapshotStore.validateID`), Tauri CSP host-allowlist, update signatures
  (minisign + ed25519), token only in `Authorization` header. All pass.
- Hygiene: `src-tauri/.cargo/audit.toml` documents 17 unmaintained **Linux-only**
  GTK3/glib transitive deps (no CVEs, not built on macOS). npm `cookie <0.7.0`
  (3× low) **accepted, not fixed** — no real surface in a desktop app, and the
  fix would force `cookie@0.7` under a SvelteKit that declares `0.6` (a worse
  trade than the low advisory). Full audit recorded in `security.md` §19.
- **Not run:** interactive runtime/MITM (Offline-Mode-zero-connections, live TLS
  validation, updater-tamper rejection) — statically verified, deferred to a
  hands-on session.

## Docs + landing (dual-build)
- **README / SECURITY / CONTRIBUTING** rewritten to cover **both builds** (Tauri +
  native Swift/SwiftUI): title → "Brew Browser", a SwiftUI badge, both new
  dashboard screenshots, a "Two builds" comparison + parity rule, native
  build-from-source, native architecture, refreshed dependency/security posture.
- **SECURITY.md Hall of fame** — credited **@neodave** for the Brewfile/snapshot
  path-traversal fix (#46), noted as defended in both builds.
- **New screenshots** `docs/screenshots/dashboard-tauri.png` (Tauri) +
  `dashboard-native.png` (native). (First copied swapped, then corrected.)
- **Landing page** (`landing/index.html`) updated for dual-build + both
  screenshots and **published live** to `brew-browser.zerologic.com` via
  `rsync` (NO `--delete`) — verified `updater.json` survived.
- **Footgun fixed:** `landing/README.md` no longer documents a bare
  `rsync --delete` to the web root (that root also serves `updater.json`; a
  `--delete` from `landing/` would wipe the Tauri updater). Host genericized out
  of the committed file. Build-host details live in auto-memory only.

## Outcome
`cargo check` clean, `cargo test` green, `npm run check` 0 errors, `swift build`
clean, `swift test` 36 pass. Filed #57 + #58 (crediting Reddit requesters).
Six more Reddit feature requests triaged but NOT filed (awaiting go-ahead).
Landing page live with both builds.
