#!/bin/bash
# native/release.sh — produce SIGNED, NOTARIZED, Sparkle-ready releases of the
# native app for BOTH arches (separate arm64 and x86_64 builds — deliberately
# NOT a universal binary), then generate the appcast. Run on the maintainer's
# Mac — the one holding the Apple "Developer ID Application" cert AND the
# Sparkle private key in its login Keychain (created once by `generate_keys`;
# its public half is the SUPublicEDKey baked into build-app.sh's Info.plist).
#
# x86_64 native covers ONLY the four Intel Macs that run macOS 26
# (MBP 16" 2019, MBP 13" 2020 4-port, iMac 27" 2020, Mac Pro 2019).
#
# Flow: per arch [ build(release, arch) → Developer ID sign (hardened runtime)
#       → zip BrewBrowser-$VERSION-<arch>.zip → notarize → staple → re-zip ]
#       → generate_appcast ONCE over $OUT_DIR (zips only; signs each update
#       with the Sparkle private key) → appcast.xml → per-arch dmgs.
#       Then you upload the zips + appcast.xml to the path behind the public
#       SUFeedURL (brew-browser.zerologic.com/...).
#
# Ordering is load-bearing: generate_appcast scans $OUT_DIR and treats every
# archive it finds as an update — so the dmgs (first-install download, not a
# Sparkle artifact) are built AFTER the appcast AND live in $OUT_DIR/dmg/, a
# subdir generate_appcast never scans. That also fixes the old bug where dmgs
# left in $OUT_DIR polluted the NEXT release's appcast scan.
#
# Required env:
#   DEVELOPER_ID_APP   e.g. "Developer ID Application: Your Name (TEAMID)"
#   NOTARY_PROFILE     a notarytool keychain profile name; create once with:
#                        xcrun notarytool store-credentials NOTARY_PROFILE \
#                          --apple-id you@example.com --team-id TEAMID --password <app-specific-pw>
# Optional env:
#   DOWNLOAD_URL_PREFIX  base URL the appcast enclosures point at
#                        (default: https://brew-browser.zerologic.com/native/)
#   OUT_DIR              where artifacts land (default: native/dist)
#
# NOTE: bump CFBundleShortVersionString + CFBundleVersion in build-app.sh before
# each release, or the appcast won't advertise a newer version. No private host
# names live here — only the public domain.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
cd "$HERE"

: "${DEVELOPER_ID_APP:?set DEVELOPER_ID_APP to your 'Developer ID Application: …' identity}"
: "${NOTARY_PROFILE:?set NOTARY_PROFILE to your notarytool keychain profile (see header)}"
DOWNLOAD_URL_PREFIX="${DOWNLOAD_URL_PREFIX:-https://brew-browser.zerologic.com/native/}"
OUT_DIR="${OUT_DIR:-$HERE/dist}"

SPARKLE_BIN="$HERE/.build/artifacts/sparkle/Sparkle/bin"
APP="$HERE/BrewBrowser.app"
[ -x "$SPARKLE_BIN/generate_appcast" ] || { echo "Sparkle tools missing — run 'swift build' first."; exit 1; }

ARCHES=(arm64 x86_64)
DMG_DIR="$OUT_DIR/dmg"
mkdir -p "$OUT_DIR" "$DMG_DIR"

# Stapled per-arch .apps are kept here until the dmgs are built — build-app.sh
# overwrites BrewBrowser.app on every run, so the arm64 app must survive the
# x86_64 build (and both must survive until after generate_appcast).
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

VERSION=""
for arch in "${ARCHES[@]}"; do
  echo "==> [$arch] build (release) + assemble"
  ./build-app.sh release "$arch"

  V="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP/Contents/Info.plist")"
  if [ -z "$VERSION" ]; then
    VERSION="$V"
    echo "==> version $VERSION"
  elif [ "$V" != "$VERSION" ]; then
    echo "version mismatch across arches ($VERSION vs $V)"; exit 1
  fi

  # Developer ID sign, inside-out, with the hardened runtime (notarization needs
  # it). Nested Sparkle code (XPC services, Updater.app, Autoupdate) + the bundled
  # SwiftPM resource bundles must be signed before the framework / app that contain
  # them. The ad-hoc signature build-app.sh applied is replaced here.
  echo "==> [$arch] Developer ID sign"
  SIGN=(codesign --force --options runtime --timestamp --sign "$DEVELOPER_ID_APP")
  FW="$APP/Contents/Frameworks/Sparkle.framework"
  if [ -d "$FW" ]; then
    find "$FW" -type d \( -name "*.xpc" -o -name "*.app" \) -print0 \
      | xargs -0 -I{} "${SIGN[@]}" "{}"
    [ -f "$FW/Versions/Current/Autoupdate" ] && "${SIGN[@]}" "$FW/Versions/Current/Autoupdate"
    "${SIGN[@]}" "$FW"
  fi
  for b in "$APP"/Contents/Resources/*.bundle; do [ -e "$b" ] && "${SIGN[@]}" "$b"; done
  "${SIGN[@]}" "$APP"
  codesign --verify --deep --strict --verbose=2 "$APP"

  ZIP="$OUT_DIR/BrewBrowser-$VERSION-$arch.zip"
  echo "==> [$arch] zip $ZIP"
  rm -f "$ZIP"
  ditto -c -k --keepParent "$APP" "$ZIP"

  echo "==> [$arch] notarize (waits for Apple)"
  xcrun notarytool submit "$ZIP" --keychain-profile "$NOTARY_PROFILE" --wait
  echo "==> [$arch] staple"
  xcrun stapler staple "$APP"
  rm -f "$ZIP"                     # re-zip so the download carries the ticket
  ditto -c -k --keepParent "$APP" "$ZIP"

  # Park the stapled .app for the dmg pass (after generate_appcast).
  mkdir -p "$WORK/$arch"
  cp -R "$APP" "$WORK/$arch/BrewBrowser.app"
done

# generate_appcast runs ONCE over $OUT_DIR, which at this point holds ONLY the
# .zips (this release's two arches + prior releases' history) — the dmgs don't
# exist yet and land in $DMG_DIR, which it never scans. Sparkle emits
# sparkle:hardwareRequirements per archive, which is how two same-version items
# (one per arch) coexist in one feed (the 0.1.0 appcast demonstrated this).
echo "==> generate appcast (signs with the Sparkle private key in your Keychain)"
"$SPARKLE_BIN/generate_appcast" --download-url-prefix "$DOWNLOAD_URL_PREFIX" "$OUT_DIR"

# Disk images for FIRST-INSTALL download (humans prefer a .dmg; Sparkle uses the
# .zips above for auto-updates). Built from the stapled per-arch .apps, staged
# with an /Applications symlink for drag-to-install, then Developer-ID signed +
# notarized + stapled — same treatment as the Tauri build's .dmg.
for arch in "${ARCHES[@]}"; do
  DMG="$DMG_DIR/BrewBrowser-$VERSION-$arch.dmg"
  echo "==> [$arch] dmg $DMG"
  STAGE="$WORK/dmg-$arch"
  mkdir -p "$STAGE"
  cp -R "$WORK/$arch/BrewBrowser.app" "$STAGE/"
  ln -s /Applications "$STAGE/Applications"
  rm -f "$DMG"
  hdiutil create -volname "Brew Browser" -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null
  codesign --force --timestamp --sign "$DEVELOPER_ID_APP" "$DMG"
  echo "==> [$arch] notarize dmg (waits for Apple)"
  xcrun notarytool submit "$DMG" --keychain-profile "$NOTARY_PROFILE" --wait
  xcrun stapler staple "$DMG"
done

# Post-run sanity: the appcast must carry ONE item per arch for this version,
# each with sparkle:hardwareRequirements — that's how a single feed serves both
# arches. Counted here so a bad feed never ships silently.
APPCAST="$OUT_DIR/appcast.xml"
ITEM_COUNT="$(grep -c "BrewBrowser-$VERSION-" "$APPCAST" 2>/dev/null || true)"
HW_COUNT="$(grep -c "sparkle:hardwareRequirements" "$APPCAST" 2>/dev/null || true)"

echo
echo "==> done."
echo "   Post-run checklist:"
echo "   [ ] appcast.xml has one <item> per arch for $VERSION (found $ITEM_COUNT enclosure(s) matching BrewBrowser-$VERSION-<arch>.zip)"
echo "   [ ] each item carries sparkle:hardwareRequirements (found $HW_COUNT in the feed)"
echo "   [ ] spot-check: grep -A3 \"BrewBrowser-$VERSION-\" \"$APPCAST\""
if [ "${ITEM_COUNT:-0}" -lt 2 ] || [ "${HW_COUNT:-0}" -lt 2 ]; then
  echo
  echo "   ############################################################################"
  echo "   ## WARNING: appcast.xml does NOT look right for a dual-arch release.      ##"
  echo "   ## generate_appcast may have collapsed/dropped the two same-version zips. ##"
  echo "   ## FALLBACK PLAN: dual feeds — generate appcast-arm64.xml and             ##"
  echo "   ## appcast-x86_64.xml from per-arch zip dirs, and have build-app.sh bake  ##"
  echo "   ## a per-arch SUFeedURL into Info.plist. Do NOT ship this appcast as-is.  ##"
  echo "   ############################################################################"
fi
echo
echo "   Upload to the host (path behind $DOWNLOAD_URL_PREFIX) — Sparkle auto-update feed:"
ls -1 "$OUT_DIR"/BrewBrowser-"$VERSION"-*.zip "$APPCAST"
echo "   (appcast.xml must be served at the SUFeedURL in build-app.sh's Info.plist)"
echo "   Attach to the GitHub release (first-install downloads, per arch):"
ls -1 "$DMG_DIR"/BrewBrowser-"$VERSION"-*.dmg
