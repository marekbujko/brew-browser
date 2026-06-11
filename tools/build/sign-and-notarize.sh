#!/usr/bin/env bash
# brew-browser — full signed + notarized release build
#
# Usage:   source ~/.config/brew-browser/signing.env && ./tools/build/sign-and-notarize.sh
#
# Runs: cargo tauri build → notarize+staple the .dmg(s) → verify with spctl.
# Requires: APPLE_ID, APPLE_PASSWORD, APPLE_TEAM_ID env vars set (see BUILD.md).
#
# Architectures — two build commands, two bundle roots:
#   npm run tauri build
#       → arm64 (host) bundles under src-tauri/target/release/bundle/
#         (artifacts named `_aarch64`)
#   npm run tauri build -- --target x86_64-apple-darwin
#       → Intel bundles under src-tauri/target/x86_64-apple-darwin/release/bundle/
#         (Tauri names Intel artifacts `_x64`; one-time setup:
#          rustup target add x86_64-apple-darwin)
# This script always runs the arm64 build itself, then signs/notarizes/staples
# every bundle it finds for BOTH arches. For a dual-arch release, run the
# x86_64 build (with signing.env sourced) before invoking this script. When
# the x86_64 bundles are absent the arch is skipped with a warning, so
# single-arch (arm64-only) releases keep working unchanged.

set -euo pipefail

# ─── Pre-flight ──────────────────────────────────────────────────────────────

cd "$(dirname "$0")/../.."   # repo root

if [[ -z "${APPLE_ID:-}" || -z "${APPLE_PASSWORD:-}" || -z "${APPLE_TEAM_ID:-}" ]]; then
  echo "✗ Missing Apple env vars. Source your signing env first:" >&2
  echo "    source ~/.config/brew-browser/signing.env" >&2
  echo "See BUILD.md for the env file template." >&2
  exit 1
fi

# Phase 15 — the updater bundle target (`.app.tar.gz`) needs Tauri's
# minisign signing keys in env. The plugin's macOS install path expects
# a signed `.app.tar.gz` alongside the manifest URL; without these env
# vars the build silently skips the signature → install attempts fail
# with "signature verification failed" against the embedded pubkey.
if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" && -z "${TAURI_SIGNING_PRIVATE_KEY_PATH:-}" ]]; then
  echo "✗ Missing TAURI_SIGNING_PRIVATE_KEY (or _PATH). Add to signing.env:" >&2
  echo "    export TAURI_SIGNING_PRIVATE_KEY_PATH=\"\$HOME/.config/brew-browser/updater.key\"" >&2
  echo "    export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=\"<your-key-password>\"" >&2
  echo "See BUILD.md → 'Per-release manifest publishing flow'." >&2
  exit 1
fi
if [[ -z "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}" ]]; then
  echo "✗ Missing TAURI_SIGNING_PRIVATE_KEY_PASSWORD env var." >&2
  echo "  Set in signing.env; required even when the key has no password (use empty string)." >&2
  exit 1
fi

# Tauri's bundler reads `TAURI_SIGNING_PRIVATE_KEY` as the literal key
# contents — it does NOT resolve `_PATH` itself despite the
# `tauri signer generate` output suggesting otherwise (observed on
# tauri-cli 2.x as of 2026-05). Bridge the gap here: when only `_PATH`
# is set, read the file and export the contents so downstream
# `npm run tauri build` sees what it expects.
if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]]; then
  if [[ ! -f "${TAURI_SIGNING_PRIVATE_KEY_PATH}" ]]; then
    echo "✗ TAURI_SIGNING_PRIVATE_KEY_PATH points at a non-existent file:" >&2
    echo "    ${TAURI_SIGNING_PRIVATE_KEY_PATH}" >&2
    exit 1
  fi
  export TAURI_SIGNING_PRIVATE_KEY="$(cat "${TAURI_SIGNING_PRIVATE_KEY_PATH}")"
fi

if ! security find-identity -v -p codesigning | grep -q 'Developer ID Application'; then
  echo "✗ No 'Developer ID Application' identity found in your keychain." >&2
  echo "  See BUILD.md, Prerequisites." >&2
  exit 1
fi

echo "▸ pre-flight ok"
echo "  apple-id:   $APPLE_ID"
echo "  team-id:    $APPLE_TEAM_ID"
echo "  minisign:   ${TAURI_SIGNING_PRIVATE_KEY_PATH:-<inline>}"

# ─── Build (compile + sign + notarize .app inside) ───────────────────────────

echo
echo "▸ npm run tauri build (compile + sign .app + sign .app.tar.gz + bundle .dmg)"
npm run tauri build

# ─── Locate the produced bundles (per arch, version-agnostic) ────────────────

AARCH64_BUNDLE="src-tauri/target/release/bundle"
X64_BUNDLE="src-tauri/target/x86_64-apple-darwin/release/bundle"

DMGS=()

# arm64 — mandatory; this script just built it.
DMG_AARCH64="$(ls -t "$AARCH64_BUNDLE"/dmg/brew-browser_*_aarch64.dmg 2>/dev/null | head -1 || true)"
if [[ -z "$DMG_AARCH64" || ! -f "$DMG_AARCH64" ]]; then
  echo "✗ build completed but no .dmg found under $AARCH64_BUNDLE/dmg/" >&2
  exit 1
fi
DMGS+=("$DMG_AARCH64")
echo
echo "▸ arm64 .dmg produced: $DMG_AARCH64"

# x86_64 — optional; built separately via
# `npm run tauri build -- --target x86_64-apple-darwin`. Skip-if-absent so
# arm64-only releases keep working unchanged.
DMG_X64="$(ls -t "$X64_BUNDLE"/dmg/brew-browser_*_x64.dmg 2>/dev/null | head -1 || true)"
if [[ -n "$DMG_X64" && -f "$DMG_X64" ]]; then
  DMGS+=("$DMG_X64")
  echo "▸ x86_64 .dmg found: $DMG_X64"
else
  echo "⚠ no x86_64 .dmg under $X64_BUNDLE/dmg/ — this will be an arm64-only release." >&2
  echo "  For a dual-arch release, run 'npm run tauri build -- --target x86_64-apple-darwin'" >&2
  echo "  (with signing.env sourced) before this script." >&2
fi

# Phase 15 — confirm the updater .app.tar.gz + .sig also exist for every
# arch we're shipping. These are what `tools/release/publish-manifest.sh`
# hashes and references in the manifest URL; a release without them ships
# a working .dmg fresh-install path but a broken auto-updater path.
require_updater_artifacts() {
  local arch="$1" bundle_root="$2"
  local app_tar_gz="$bundle_root/macos/brew-browser.app.tar.gz"
  local app_tar_gz_sig="${app_tar_gz}.sig"
  if [[ ! -f "$app_tar_gz" ]]; then
    echo "✗ $arch updater artifact missing: $app_tar_gz" >&2
    echo "  Tauri should emit this automatically when the updater plugin is" >&2
    echo "  registered AND TAURI_SIGNING_PRIVATE_KEY[_PATH] is set." >&2
    exit 1
  fi
  if [[ ! -f "$app_tar_gz_sig" ]]; then
    echo "✗ $arch updater signature missing: $app_tar_gz_sig" >&2
    echo "  The TAURI_SIGNING_PRIVATE_KEY env vars probably weren't used by the build." >&2
    exit 1
  fi
  echo "▸ $arch updater artifact: $app_tar_gz"
  echo "▸ $arch updater signature: $app_tar_gz_sig"
}

require_updater_artifacts arm64 "$AARCH64_BUNDLE"
if [[ -n "$DMG_X64" ]]; then
  require_updater_artifacts x86_64 "$X64_BUNDLE"
fi

# ─── Notarize + staple each .dmg wrapper itself ──────────────────────────────

for DMG in "${DMGS[@]}"; do
  echo
  echo "▸ submitting $(basename "$DMG") to Apple notary (waiting for ticket — typically 1-5 min)"
  xcrun notarytool submit "$DMG" \
    --apple-id "$APPLE_ID" \
    --password "$APPLE_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" \
    --wait

  echo
  echo "▸ stapling notarization ticket to $(basename "$DMG")"
  xcrun stapler staple "$DMG"
done

# ─── Verify ──────────────────────────────────────────────────────────────────

echo
echo "▸ verification"
for DMG in "${DMGS[@]}"; do
  spctl --assess --type install --verbose=4 "$DMG"
  xcrun stapler validate "$DMG"
done

echo
echo "✓ done — ${#DMGS[@]} .dmg(s) signed, notarized, stapled, and ready to ship"
ls -lh "${DMGS[@]}"
