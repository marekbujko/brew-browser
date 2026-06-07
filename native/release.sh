#!/bin/bash
# native/release.sh — produce a SIGNED, NOTARIZED, Sparkle-ready release of the
# native app, then generate the appcast. Run on the maintainer's Mac — the one
# holding the Apple "Developer ID Application" cert AND the Sparkle private key
# in its login Keychain (created once by `generate_keys`; its public half is the
# SUPublicEDKey baked into build-app.sh's Info.plist).
#
# Flow: build(release) → Developer ID sign (hardened runtime) → zip → notarize →
#       staple → re-zip → generate_appcast (signs each update with the Sparkle
#       private key) → appcast.xml. Then you upload the zip + appcast.xml to the
#       path behind the public SUFeedURL (brew-browser.zerologic.com/...).
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

echo "==> build (release) + assemble"
./build-app.sh release

VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP/Contents/Info.plist")"
echo "==> version $VERSION"

# Developer ID sign, inside-out, with the hardened runtime (notarization needs
# it). Nested Sparkle code (XPC services, Updater.app, Autoupdate) + the bundled
# SwiftPM resource bundles must be signed before the framework / app that contain
# them. The ad-hoc signature build-app.sh applied is replaced here.
echo "==> Developer ID sign"
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

mkdir -p "$OUT_DIR"
ZIP="$OUT_DIR/BrewBrowser-$VERSION.zip"
echo "==> zip $ZIP"
rm -f "$ZIP"
ditto -c -k --keepParent "$APP" "$ZIP"

echo "==> notarize (waits for Apple)"
xcrun notarytool submit "$ZIP" --keychain-profile "$NOTARY_PROFILE" --wait
echo "==> staple"
xcrun stapler staple "$APP"
rm -f "$ZIP"                       # re-zip so the download carries the ticket
ditto -c -k --keepParent "$APP" "$ZIP"

echo "==> generate appcast (signs with the Sparkle private key in your Keychain)"
"$SPARKLE_BIN/generate_appcast" --download-url-prefix "$DOWNLOAD_URL_PREFIX" "$OUT_DIR"

echo
echo "==> done. Upload these to the path behind $DOWNLOAD_URL_PREFIX:"
ls -1 "$OUT_DIR"/BrewBrowser-*.zip "$OUT_DIR/appcast.xml"
echo "   (appcast.xml must be served at the SUFeedURL in build-app.sh's Info.plist)"
