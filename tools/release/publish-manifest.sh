#!/usr/bin/env bash
# Phase 15 — emit the in-app updater manifest for a released .dmg.
#
# Usage:
#   tools/release/publish-manifest.sh 0.3.0
#
# What it does:
#   1. Validates the version argument shape.
#   2. Locates the .dmg artifact at the canonical bundle path.
#   3. Computes the artifact's SHA-256 digest.
#   4. Signs the .dmg with `minisign` using the private key at
#      ~/.config/brew-browser/updater.key (the user generates this
#      key once per the BUILD.md instructions; this script does not).
#   5. Emits dist/updater.json with the shape the Tauri updater
#      plugin expects:
#        {
#          "version": "0.3.0",
#          "notes": "<release notes — empty placeholder for now>",
#          "pub_date": "2026-05-24T00:00:00Z",
#          "platforms": {
#            "darwin-aarch64": {
#              "signature": "<minisign output>",
#              "url": "<github release asset URL>",
#              "sha256": "<artifact digest>"
#            }
#          }
#        }
#   6. Echoes (but does NOT execute) the rsync command the user runs
#      to publish the manifest to brew-browser.zerologic.com via
#      umacbookpro:Sites/brew-browser/updater.json. Publishing is a
#      deliberate manual step.
#
# What it does NOT do:
#   - Generate the minisign keypair (one-time setup, see BUILD.md).
#   - Publish to the CDN (the rsync is the user's call).
#   - Build the .dmg (npm run tauri build is upstream of this).
#   - Update CHANGELOG.md or push the git tag.
#
# Exit codes:
#   0  — manifest written successfully
#   1  — usage error
#   2  — artifact missing
#   3  — minisign / sha256 tooling missing
#   4  — signing failed

set -euo pipefail

# ---------- Argument validation ----------

if [[ $# -ne 1 ]]; then
    echo "usage: $0 <version>" >&2
    echo "  e.g. $0 0.3.0" >&2
    exit 1
fi

VERSION="$1"
# Reject anything that isn't strict semver-three-part. Defense against
# accidental "v0.3.0" or "0.3" arguments that would mis-name the artifact.
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
    echo "error: VERSION must be semver (got: $VERSION)" >&2
    echo "  expected: <major>.<minor>.<patch>[-prerelease]" >&2
    exit 1
fi

# ---------- Paths ----------

# Resolve repo root from this script's location so it works regardless
# of the caller's cwd. `realpath` is portable on macOS via coreutils;
# we fall back to BASH_SOURCE-based resolution if it isn't installed.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

DMG_PATH="$REPO_ROOT/src-tauri/target/release/bundle/dmg/brew-browser_${VERSION}_aarch64.dmg"
KEY_PATH="$HOME/.config/brew-browser/updater.key"
DIST_DIR="$REPO_ROOT/dist"
MANIFEST_PATH="$DIST_DIR/updater.json"

# ---------- Preflight ----------

if [[ ! -f "$DMG_PATH" ]]; then
    echo "error: DMG not found at $DMG_PATH" >&2
    echo "  did you run 'npm run tauri build' first?" >&2
    exit 2
fi

if ! command -v minisign >/dev/null 2>&1; then
    echo "error: minisign not on PATH" >&2
    echo "  install via: brew install minisign" >&2
    exit 3
fi

if ! command -v shasum >/dev/null 2>&1; then
    echo "error: shasum not on PATH" >&2
    echo "  on macOS this is built-in; on Linux: apt install perl" >&2
    exit 3
fi

if [[ ! -f "$KEY_PATH" ]]; then
    echo "error: minisign private key not found at $KEY_PATH" >&2
    echo "  generate it once per BUILD.md instructions:" >&2
    echo "    tauri signer generate -w $KEY_PATH" >&2
    exit 3
fi

# ---------- Compute hash ----------

echo "info: computing SHA-256 of $(basename "$DMG_PATH")..." >&2
SHA256=$(shasum -a 256 "$DMG_PATH" | awk '{print $1}')
echo "info: sha256 = $SHA256" >&2

# ---------- Sign ----------

# minisign -S writes <input>.minisig next to the input file. We capture
# the signature output, then read the .minisig file back into the
# manifest JSON. The trusted comment is a freeform field we set to the
# version + build date for audit traceability.
SIGNATURE_FILE="${DMG_PATH}.minisig"
TRUSTED_COMMENT="brew-browser ${VERSION} ($(date -u +%Y-%m-%dT%H:%M:%SZ))"

# Remove any stale .minisig so a repeat run doesn't merge two signatures.
rm -f "$SIGNATURE_FILE"

echo "info: signing $(basename "$DMG_PATH") with minisign..." >&2
if ! minisign -Sm "$DMG_PATH" -s "$KEY_PATH" -t "$TRUSTED_COMMENT" >/dev/null; then
    echo "error: minisign signing failed" >&2
    exit 4
fi

if [[ ! -f "$SIGNATURE_FILE" ]]; then
    echo "error: minisign reported success but $SIGNATURE_FILE is missing" >&2
    exit 4
fi

# Read signature as a single-line string for the JSON payload. The
# Tauri updater plugin expects the full .minisig file contents
# (untrusted + trusted comments + signature lines) as the "signature"
# field — it parses them itself.
SIGNATURE_RAW=$(cat "$SIGNATURE_FILE")

# JSON-escape the signature: convert literal newlines to \n. Use perl
# (always available on macOS) for portable in-place escaping; jq is the
# obvious alternative but adding a hard dep on jq for one string-escape
# step felt heavy.
SIGNATURE_JSON=$(perl -pe 's/\n/\\n/g' <<< "$SIGNATURE_RAW")
# The above leaves a trailing \n from the heredoc; strip it.
SIGNATURE_JSON="${SIGNATURE_JSON%\\n}"

# ---------- Emit manifest ----------

mkdir -p "$DIST_DIR"

# Release notes are intentionally a placeholder. The user can hand-edit
# the manifest before the rsync to inject the CHANGELOG entry — keeping
# the manifest generator and the release notes editorial step separate
# is the simpler shape than wiring CHANGELOG parsing here.
PUB_DATE=$(date -u +%Y-%m-%dT%H:%M:%SZ)
URL="https://github.com/msitarzewski/brew-browser/releases/download/v${VERSION}/brew-browser_${VERSION}_aarch64.dmg"

cat > "$MANIFEST_PATH" <<EOF
{
  "version": "${VERSION}",
  "notes": "See https://github.com/msitarzewski/brew-browser/releases/tag/v${VERSION} for release notes.",
  "pub_date": "${PUB_DATE}",
  "platforms": {
    "darwin-aarch64": {
      "signature": "${SIGNATURE_JSON}",
      "url": "${URL}",
      "sha256": "${SHA256}"
    }
  }
}
EOF

echo ""
echo "✓ manifest written to: $MANIFEST_PATH"
echo "  version:    $VERSION"
echo "  sha256:     $SHA256"
echo "  url:        $URL"
echo "  pub_date:   $PUB_DATE"
echo "  signature:  $(wc -c < "$SIGNATURE_FILE" | tr -d ' ') bytes from $SIGNATURE_FILE"
echo ""
echo "next step (run manually):"
echo "  rsync -av $MANIFEST_PATH umacbookpro:Sites/brew-browser/updater.json"
echo ""
echo "verify before publishing:"
echo "  shasum -a 256 $DMG_PATH"
echo "  minisign -Vm $DMG_PATH -P \"\$(cat ~/.config/brew-browser/updater.pub)\""
