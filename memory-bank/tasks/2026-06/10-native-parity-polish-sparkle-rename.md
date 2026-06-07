# 10 — Native parity polish, Sparkle end-to-end, "Brew Browser" rename

**Date:** 2026-06-06
**Branch:** `experiment/native-swift-liquid-glass`
**Status:** all committed + pushed (`892cbdf` … `e08a377`). Working tree clean.

Follow-on to the bundle work (task 09). This captures the post-bundle fixes, the
Sparkle updater going fully live, and the app rename — i.e. the run-up to deploy.

## Vulnerability feature — the long tail (most-edited area)
Root causes found by reading raw `brew vulns` output, not guessing:
- **`VulnsService` was an `actor`** → every `scanOne` serialized (same trap as
  BrewService/GitHubService). Converted to `Sendable struct`. (`ea50d6b`)
- **`brew vulns --formula <name>` IGNORES `--formula`** and returns the WHOLE
  install set. The old per-formula sweep therefore (a) re-scanned everything 331×
  and (b) attributed the entire finding set to EVERY package → "319 of 331
  vulnerable", 1500+ findings. Fix: one `brew vulns --json` call, parse records,
  key findings by `record.formula`. (`276ed62`) Same bug existed in Tauri
  `scan_one` (flat_map over all records) — fixed to filter to the formula too
  (`ed00b1e`).
- **Scope:** `brew vulns --json` scans ~the install set; native now matches
  Tauri's count (~12–13 vulnerable PACKAGES / 17 advisories). 12 = packages,
  17 = findings — the Exposure line spells out "N findings across M of T
  packages" (`8ca1f13`) and the sidebar footer says "N vulnerable packages"
  (`7cbaa85`).
- **No false all-clear:** a green "no vulnerabilities" only shows when scanned
  THIS session. Cache-hydrated/stale → amber "No advisories as of the last scan
  (N ago) — re-scan"; never-scanned → hazard. Both builds. Native uses a
  `vulnScannedThisSession` flag; Tauri uses `scannedThisSession(date)` (scan ts ≥
  app-session start). (`83abfd6` native, `515407e` tauri)
- **Detail card reads the cached full scan** (no per-open re-scan); re-scan is an
  explicit full-system scan, labeled so. Persisted `vulnFindings` survives
  relaunch. (`ddf1094`) Detail showed OTHER packages' CVEs before the filter
  fix (`593401d`).
- Dashboard auto-scan-on-launch removed; results persist; "Scan now" is
  user-initiated. (`8ee39a4`) Spinners animate (`56cb367`).
- Severity dots in EVERY list (Library/Discover/Trending/Services/Updates).

## Other native polish
- "Wrong?" links → Tauri-style **(i) InfoButton** popover ("Generated offline at
  build time by Claude Haiku 4.5 …" + Report-an-issue). (`d424990`)
- Catalog **Refresh** runs `brew update` streamed to the Activity drawer +
  animated spinner. (`3969c4f`) Dropped the redundant Updates-card "Update"
  button (Refresh covers it). (`673c8b4`)
- Dashboard Updates rows stay selected while their detail is open. (`593401d`)

## Sparkle self-updater — LIVE
- `build-app.sh` ad-hoc re-signs the .app AFTER `install_name_tool` (the rpath
  edit invalidated the linker sig → Apple Silicon refused to launch). (`8e71483`,
  earlier)
- Generated the ed25519 keypair; PUBLIC key baked into `build-app.sh`
  `SUPublicEDKey` (committed — public by design). Private key in the login
  Keychain. `native/release.sh`: build→Developer-ID sign→notarize→staple→zip→
  `generate_appcast`. (`821c9a8`)
- **Proven end-to-end 2026-06-06:** hosted a test 0.2.0 appcast+zip on the host;
  the installed 0.1.0 app's "Check for Updates" showed the **"new version 0.2.0
  available"** prompt. Test appcast then REMOVED (would false-advertise);
  `/appcast.xml` is 404 again. `updater.json` (Tauri) untouched.
- **Updater inert when run UNBUNDLED** (Xcode Run / `swift run`): no `SUFeedURL`
  in Bundle.main → skip Sparkle so it can't throw "updater failed to start /
  …Debug". Real updates require the assembled `.app`. (`f971bf3`)

## Rename → "Brew Browser"
- Native: `CFBundleName` + `CFBundleDisplayName` = "Brew Browser" (menu bar +
  Dock); menu items + About title updated. Bundle id / executable / repo slug
  unchanged. (`763ee02`)
- Tauri: `productName` + window title + About title = "Brew Browser". (`514cee0`)
  ⚠️ **Renames the bundle (`brew-browser.app` → `Brew Browser.app`)** — shipped
  0.5.0 users may keep a stale `brew-browser.app` after the next auto-update;
  call it out in release notes.
- About box: custom SwiftUI `AboutView` mirroring the Tauri AboutModal (real app
  icon, meta card, Donate CTA, Built-with credits, zero-telemetry line).
  (`f971bf3`, icon `5ad1601`)

## Outcome
Native at full feature parity with Tauri. `swift build` + `./build-app.sh` clean,
launches. Tauri `npm run check` 0 errors, `cargo check` clean.
