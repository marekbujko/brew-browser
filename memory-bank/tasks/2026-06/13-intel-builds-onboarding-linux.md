# 13 — Intel builds + missing-Homebrew onboarding + Linux integration (both builds)

**Date:** 2026-06-10/11 | **Branch:** `feat/intel-builds-and-onboarding` (off `main` @ v0.5.1) | **Status:** built + QA green, uncommitted, awaiting commit/PR/release

## Objective

Triggered by post-0.5.1 "corrupt download" reports that turned out to be Intel
Mac users opening arm64-only dmgs (audit record: every shipped 0.1.0/0.5.1
artifact verified bit-perfect, signed, notarized, stapled — nothing was
actually corrupt). Three approved workstreams, built via multi-agent workflow
(4 parallel build agents → Linux integration agent → QA loop, green in 1 cycle):

1. **WS1 — Missing-Homebrew onboarding** (both shells): replace the dead-end
   "Homebrew was not found" error with a guided first-run experience.
2. **WS2 — Intel (x86_64) builds**: Tauri per-arch dmgs + native per-arch
   artifacts (user decision: separate builds, NOT universal — no extra weight
   for arm64 users).
3. **WS3 — Linux**: bring `282d8ff` (feat/linux-support, was 99 behind) onto
   the branch and integrate with the new onboarding.

## Outcome

- ✅ cargo: **618 passed**, 0 failed (585 → 618)
- ✅ svelte-check: 0 errors, 3 warnings (exact pre-existing baseline)
- ✅ swift: **44 passed** (36 → 44, 8 new onboarding/VulnsService tests)
- ✅ `bash -n` clean: all 4 release-tooling scripts
- ✅ x86_64 `swift build` cross-compile verified on the dev host
- ✅ Tauri x86_64-apple-darwin `cargo check` verified (with the RUSTC pin below)
- ✅ **Linux verified end-to-end in the Scratch VM (2026-06-11):** all 3 bundles
  built (deb/rpm/AppImage, arm64); deb installed; app launched in a real GNOME
  Wayland session; Dashboard loaded 3 Linuxbrew formulae with storage paths
  resolved to `/home/linuxbrew/.linuxbrew`. **Onboarding verified live:** hid
  the Linuxbrew prefix → relaunch showed the onboarding view (Linux copy with
  apt build-deps step, Copy primary, Open Terminal correctly absent) → restored
  the prefix while the app ran → the 2s poll detected it and the SAME process
  dissolved into the loaded Dashboard, no relaunch. Screenshots captured via
  `prlctl capture` (gnome-screenshot is blocked under Wayland).
- Diff: 32 files changed, +1,757 / −272, +3 new files

## Files Modified (key)

### WS1 Tauri
- `src-tauri/src/commands/env.rs` — extended the existing brew_doctor probe
  module (reuse over creation): `SystemStatus` DTO, `system_status()`,
  `brew_redetect()`, `open_terminal_install()` with the Homebrew install
  one-liner as a **fixed raw-string constant** (zero interpolation; osascript
  `do script` + `activate`; spawn failure → typed error → frontend copy fallback)
- `src-tauri/src/state.rs:279-298` — `set_brew_path()` + `redetect_brew_path()`
  (re-runs `resolve_brew_path()`, writes the `RwLock` — recovery without relaunch)
- `src-tauri/src/lib.rs` — 3 commands registered
- `src/lib/components/OnboardingView.svelte` (new) — gates `+page.svelte`;
  CLT step → install one-liner + Copy + Open Terminal (copy is primary);
  2s poll via `brew_redetect`; auto-dissolves into `packages.load(true)`
- `src/lib/types.ts`, `src/lib/api.ts`, `src/lib/stores/env.svelte.ts` — bindings/gate

### WS1 native
- `BrewService.swift:113` — `resolveBrewPath()` promoted to internal shared resolver
- `VulnsService.swift` — **bug fix**: hardcoded `/opt/homebrew/bin/brew` fallback
  removed; `brewPath: String?`, nil → throws `.brewNotFound` (no fake paths)
- `AppModel.swift:893-961` — `brewMissing`/`cltInstalled` state set synchronously
  in `init` (no flash of normal UI); `pollForBrew()` 2s loop, off-main probes;
  `commandLineToolsInstalled()` via `xcode-select -p`
- `OnboardingView.swift` (new) — stock SwiftUI only; `ContentView.swift` gate
- `OnboardingStateTests.swift` (new) + VulnsService test updates

### WS2 (release tooling)
- `tools/build/sign-and-notarize.sh` — per-arch dmg discovery (aarch64 mandatory,
  x64 skip-if-absent with warning); notarize/staple/verify loop over both
- `tools/release/publish-manifest.sh` — `darwin-x86_64` platform key
  (`brew-browser_<v>_x64.app.tar.gz`), gated: absent → loud warn + omit;
  present-without-sig → exit 2
- `native/build-app.sh` — `[debug|release] [arm64|x86_64]`; `--arch` flag with
  `--show-bin-path` bindir resolution (handles both SwiftPM layout generations);
  no-arg behavior byte-identical; re-sign-inside-out-last preserved
- `native/release.sh` — per-arch loop → `BrewBrowser-<v>-<arch>.{zip,dmg}`;
  **`generate_appcast` now runs over a zips-only dir BEFORE dmgs are built**
  (fixes the latent bug where dist dmgs would pollute the next appcast scan);
  single feed relying on `sparkle:hardwareRequirements`, dual-feed fallback
  documented in the post-run checklist
- `README.md` + `landing/index.html` — "Which download?" guide per arch/build

### WS3 Linux (from `282d8ff`, 3-way applied, conflicts resolved)
- Per-target keyring features (`Cargo.toml`: macOS apple-native +
  security-framework; Linux sync-secret-service + crypto-rust)
- Linuxbrew prefixes in `brew/paths.rs`; exec cwd pinned to `/`
- cfg-gated Finder reveal / cask icons / native menu (`lib.rs`)
- New: `tauri.linux.conf.json`, `.github/workflows/linux-build.yml`,
  `src/lib/util/platform.ts`
- Onboarding Linux-aware: Open Terminal gated off (frontend + backend), Linuxbrew copy

## Addendum (2026-06-11): casks don't exist on Linux — data + UI gating

User testing on the VM surfaced casks being offered on Linux (Install could only
fail with "macOS is required"). Casks are a macOS-only Homebrew concept (no
Linux variant exists, even for apps with native Linux builds — Flatpak/Snap own
that role there). Fixes, all verified on the VM:

**Data layer:** `catalog.rs` `catalog_casks_summary` returns empty on
non-macOS (kills Discover tiles/search/category counts at the source);
`search.rs` never spawns `brew search --cask` on non-macOS;
`PackageDetail.svelte` `caskOnLinux` guard replaces Install with "Casks require
macOS" for any cask that still surfaces (snapshots/deep links).

**UI terminology sweep (gated on `isLinux` from `src/lib/util/platform.ts`;
macOS rendering byte-identical):** Dashboard Composition card hidden +
Caskroom storage row dropped; Library "Casks" filter chip removed + Type column
hidden (the kind cell stays as a 12px slot — it also hosts the vulnerability
severity dot, which IS functional on Linux); Trending Type column removed
(responsive grid reworked); Discover category browse skips cask tokens from the
platform-agnostic bundled categories.json (counts agree); Settings network copy
reworded (no cask icon control/probe entries, `xdg-open` instead of "macOS
open(1)" — verified against the opener plugin's actual Linux behavior);
Snapshots keeps real cask counts from Mac-origin snapshots but drops "0 casks"
noise, and its empty-state shows the XDG path (`~/.local/share/...`) and
"another machine" on Linux; About credits formula-only.

QA after sweep: cargo test green, svelte-check 0 errors / 3 baseline warnings.
VM-verified screenshots: Dashboard, Library, Trending, Discover all cask-free.

## Decisions

- **Separate arch builds, not universal** (user): arm64 users don't pay Intel
  weight. Native: single Sparkle feed first (generate_appcast emits
  `hardwareRequirements`), dual-feed fallback documented.
- **Native x86_64 audience**: the four Intel Macs on macOS 26 (MBP 16" 2019,
  MBP 13" 2020 4-port, iMac 27" 2020, Mac Pro 2019) — confirmed via Apple
  support page 122867. Intel Mac minis cannot run Tahoe → served by Tauri x64.
- **Installer never runs in-app**: needs sudo + TTY; Open Terminal pre-types
  the fixed command, Copy is primary (osascript Automation prompt is one-time).
- **Reuse**: commands went into existing `env.rs` (the env/probe cluster);
  no new Rust module created.

## Known caveats

- Dev-host `swift test` requires `DEVELOPER_DIR=/Applications/Xcode-beta.app/...`
  (CLT toolchain has a broken swift-package dyld link) + a gitignored
  `.build/` symlink for the Sparkle test bundle. Environment-only.
- Cask per-arch body (on_arm/on_intel) prepared in the build report — applied
  in the tap repo at release time.
- Tauri x64 release requires `rustup target add x86_64-apple-darwin` (done on
  dev host 2026-06-11).
- **Dev-host Tauri release-build gotcha — ROOT-CAUSED 2026-06-11 (E0463
  proc-macro failures):** `tauri build` exports
  `MACOSX_DEPLOYMENT_TARGET=13.0` (from `tauri.conf.json`
  `minimumSystemVersion`). On this host's macOS 27 beta toolchain, proc-macro
  dylibs *built under that env* are unloadable by rustc → `E0463 can't find
  crate` for `ctor_proc_macro`/`phf_macros`/`thiserror_impl`/… (dylib is
  correct-arch, correct-version, signed, passed via `--extern`; rustc just
  can't load it — suspected beta dyld/ld regression, recheck after Xcode
  updates). Reproduced minimally: `MACOSX_DEPLOYMENT_TARGET=13.0 cargo build
  --release` on a fresh crate with `ctor` fails identically; without the env
  it passes. **Amplifier:** cargo does NOT fingerprint that env var, so
  tainted proc-macros are silently reused by every subsequent build (either
  toolchain, both arches) — failures persist until `cargo clean`. This also
  produced red herrings all evening (mixed Homebrew-1.95/rustup-1.96 runs).
  **Working recipe (PROVEN 2026-06-11: produced both dmgs incl. the first
  `_x64.dmg`). Order matters — prime BOTH arches BEFORE any tauri build, and
  feature-match the prime or `tauri_macros` rebuilds tainted:**
  1. one toolchain only — rustup's, which has the x64 std:
     `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"`
  2. if the target dir ever saw a tauri build that failed E0463: `cargo clean`
     (a failed tauri build taints the cache for ALL later builds, primes
     included)
  3. PRIME env-free, feature-matched, both arches first:
     `cargo build --release --features tauri/custom-protocol`
     `cargo build --release --features tauri/custom-protocol --target x86_64-apple-darwin`
  4. only then bundle: `npm run tauri build` and
     `npm run tauri build -- --target x86_64-apple-darwin`
  (Priming is build-time only — shipped binaries still link with deployment
  target 13.0 as configured. Proc-macros never ship.)
- Plain `cargo test` (dev profile, no tauri env) is unaffected: 618 green.

## Artifacts

- Workflow run `wf_a5f4aa56-486` (6 agents, QA green cycle 1/3)
- PR: pending (branch uncommitted at time of writing)
