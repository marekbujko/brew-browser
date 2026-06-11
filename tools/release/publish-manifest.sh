#!/usr/bin/env bash
# Phase 15 — emit the in-app updater manifest for a released .app.tar.gz.
#
# Usage:
#   tools/release/publish-manifest.sh 0.3.0
#
# What it does:
#   1. Validates the version argument shape.
#   2. Locates the .app.tar.gz artifact(s) at the canonical macos bundle
#      paths — arm64 under target/release/bundle, x86_64 under
#      target/x86_64-apple-darwin/release/bundle (Tauri names Intel
#      artifacts `_x64`). The arm64 artifact is mandatory; the x86_64
#      one is optional and its platform key is omitted (with a loud
#      warning) when absent, so arm64-only releases keep working.
#      **The Tauri updater plugin's macOS install path expects a
#      gzipped tar of the .app bundle — NOT the .dmg.** Feeding it a
#      .dmg results in an "invalid gzip" error on every install attempt.
#      The .dmg is still uploaded to GitHub Releases for fresh installs;
#      only the auto-updater path needs the .app.tar.gz.
#   3. Computes the artifact's SHA-256 digest.
#   4. Reads the .sig file the Tauri build already produced beside the
#      artifact (the bundler runs minisign-via-TAURI_SIGNING_* during
#      `npm run tauri build` — see `tools/build/sign-and-notarize.sh`).
#      We do NOT re-sign here: doing so would produce a minisign-native
#      .minisig that the Tauri plugin's verification path doesn't accept.
#      The Tauri-format .sig is what the embedded pubkey was generated
#      to verify against.
#   5. Emits dist/updater.json with the shape the Tauri updater
#      plugin expects:
#        {
#          "version": "0.3.0",
#          "notes": "<release notes — empty placeholder for now>",
#          "pub_date": "2026-05-24T00:00:00Z",
#          "platforms": {
#            "darwin-aarch64": {
#              "signature": "<contents of .app.tar.gz.sig, single-line>",
#              "url": "<github release asset URL of the .app.tar.gz>",
#              "sha256": "<artifact digest>"
#            },
#            "darwin-x86_64": {  // only when the _x64 artifact exists
#              "signature": "...",
#              "url": "...",
#              "sha256": "..."
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
#   - Re-sign the artifact (Tauri's bundler already did via TAURI_SIGNING_*).
#   - Publish to the CDN (the rsync is the user's call).
#   - Build the artifact (npm run tauri build is upstream of this).
#   - Update CHANGELOG.md or push the git tag.
#   - Upload the .app.tar.gz to GitHub Releases (the user attaches it
#     to the `gh release create` invocation, alongside the .dmg).
#
# Exit codes:
#   0  — manifest written successfully
#   1  — usage error
#   2  — artifact or .sig missing
#   3  — sha256 tooling missing

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

# The Tauri bundler emits the updater artifact + signature at these
# paths — one bundle root per arch. Filenames are fixed (no version or
# arch stamp) inside each `bundle/macos/`; we upload to GitHub Releases
# under versioned, arch-stamped names so the manifest URLs are
# unambiguous (Tauri's own artifact naming: arm64 = `_aarch64`,
# Intel = `_x64`).
AARCH64_ARTIFACT_PATH="$REPO_ROOT/src-tauri/target/release/bundle/macos/brew-browser.app.tar.gz"
AARCH64_SIGNATURE_FILE="${AARCH64_ARTIFACT_PATH}.sig"
AARCH64_RELEASE_NAME="brew-browser_${VERSION}_aarch64.app.tar.gz"
X64_ARTIFACT_PATH="$REPO_ROOT/src-tauri/target/x86_64-apple-darwin/release/bundle/macos/brew-browser.app.tar.gz"
X64_SIGNATURE_FILE="${X64_ARTIFACT_PATH}.sig"
X64_RELEASE_NAME="brew-browser_${VERSION}_x64.app.tar.gz"
DIST_DIR="$REPO_ROOT/dist"
MANIFEST_PATH="$DIST_DIR/updater.json"

# Linux updater artifact. Tauri's Linux updater install path uses the
# .AppImage (NOT the .deb/.rpm), and signs it with the same
# TAURI_SIGNING_PRIVATE_KEY (minisign is cross-platform). The bundler
# stamps the AppImage with the version + arch, so glob for it rather
# than hard-coding the name. This block is OPTIONAL: when running on a
# Mac that only built the .app.tar.gz, no AppImage exists and we emit a
# macOS-only manifest exactly as before.
LINUX_ARTIFACT_PATH="$(ls -t "$REPO_ROOT"/src-tauri/target/release/bundle/appimage/*.AppImage 2>/dev/null | head -1 || true)"
LINUX_ARTIFACT_RELEASE_NAME="brew-browser_${VERSION}_amd64.AppImage"

# ---------- Preflight ----------

if [[ ! -f "$AARCH64_ARTIFACT_PATH" ]]; then
    echo "error: updater artifact not found at $AARCH64_ARTIFACT_PATH" >&2
    echo "  did you run 'npm run tauri build' first?" >&2
    echo "  the macOS updater target produces .app.tar.gz alongside the .dmg." >&2
    exit 2
fi

if [[ ! -f "$AARCH64_SIGNATURE_FILE" ]]; then
    echo "error: updater signature not found at $AARCH64_SIGNATURE_FILE" >&2
    echo "  The Tauri bundler produces this when TAURI_SIGNING_PRIVATE_KEY[_PATH]" >&2
    echo "  + TAURI_SIGNING_PRIVATE_KEY_PASSWORD are set during 'npm run tauri build'." >&2
    echo "  Source ~/.config/brew-browser/signing.env, then re-run the build." >&2
    exit 2
fi

# x86_64 is optional — omit its platform key (loudly) when the artifact
# is absent rather than failing, so an arm64-only release still ships.
# An artifact WITHOUT its .sig is a broken build, not an absent arch:
# fail in that case exactly like the aarch64 path.
HAVE_X64=0
if [[ -f "$X64_ARTIFACT_PATH" ]]; then
    if [[ ! -f "$X64_SIGNATURE_FILE" ]]; then
        echo "error: x86_64 updater signature not found at $X64_SIGNATURE_FILE" >&2
        echo "  The artifact exists but its .sig is missing — the TAURI_SIGNING_*" >&2
        echo "  env vars probably weren't set during the x86_64 build. Re-run" >&2
        echo "  'npm run tauri build -- --target x86_64-apple-darwin' with" >&2
        echo "  signing.env sourced." >&2
        exit 2
    fi
    HAVE_X64=1
else
    echo "" >&2
    echo "⚠⚠⚠ WARNING: no x86_64 updater artifact at $X64_ARTIFACT_PATH" >&2
    echo "    The manifest will contain ONLY darwin-aarch64 — Intel users get" >&2
    echo "    no auto-update for v${VERSION}. For a dual-arch release, run" >&2
    echo "    'npm run tauri build -- --target x86_64-apple-darwin' first" >&2
    echo "    (one-time: rustup target add x86_64-apple-darwin), then re-run" >&2
    echo "    this script." >&2
    echo "" >&2
fi

if ! command -v shasum >/dev/null 2>&1; then
    echo "error: shasum not on PATH" >&2
    echo "  on macOS this is built-in; on Linux: apt install perl" >&2
    exit 3
fi

# ---------- Compute hashes ----------

echo "info: computing SHA-256 of aarch64 $(basename "$AARCH64_ARTIFACT_PATH")..." >&2
AARCH64_SHA256=$(shasum -a 256 "$AARCH64_ARTIFACT_PATH" | awk '{print $1}')
echo "info: aarch64 sha256 = $AARCH64_SHA256" >&2
if [[ $HAVE_X64 -eq 1 ]]; then
    echo "info: computing SHA-256 of x86_64 $(basename "$X64_ARTIFACT_PATH")..." >&2
    X64_SHA256=$(shasum -a 256 "$X64_ARTIFACT_PATH" | awk '{print $1}')
    echo "info: x86_64 sha256 = $X64_SHA256" >&2
fi

# ---------- Read signatures ----------

# Tauri's bundler emits a single-line base64 blob in the .sig file
# (Tauri's format, NOT raw minisign's multi-line .minisig format).
# The plugin's verification path reads this string directly and parses
# it against the embedded pubkey; do not re-encode.
read_signature() {
    local raw
    raw=$(cat "$1")
    # Trim trailing newline if present (cat's $() strips it, but be safe).
    raw="${raw%$'\n'}"
    # Defensive: JSON-escape any literal newlines (Tauri's .sig is normally
    # a single line but be belt-and-braces in case bundler version drift
    # changes the shape).
    raw=$(perl -pe 's/\n/\\n/g' <<< "$raw")
    printf '%s' "${raw%\\n}"
}

AARCH64_SIGNATURE_JSON=$(read_signature "$AARCH64_SIGNATURE_FILE")
if [[ $HAVE_X64 -eq 1 ]]; then
    X64_SIGNATURE_JSON=$(read_signature "$X64_SIGNATURE_FILE")
fi

# ---------- Linux platform block (conditional) ----------

# Build the linux-x86_64 platform entry only when the AppImage + its
# .sig are both present. Absent (Mac-only build) → LINUX_BLOCK stays
# empty and the manifest is macOS-only, byte-for-byte as before.
LINUX_BLOCK=""
if [[ -n "$LINUX_ARTIFACT_PATH" && -f "$LINUX_ARTIFACT_PATH" ]]; then
    LINUX_SIGNATURE_FILE="${LINUX_ARTIFACT_PATH}.sig"
    if [[ ! -f "$LINUX_SIGNATURE_FILE" ]]; then
        echo "error: found AppImage but its signature is missing:" >&2
        echo "  $LINUX_SIGNATURE_FILE" >&2
        echo "  Tauri produces this when TAURI_SIGNING_PRIVATE_KEY[_PATH] is set" >&2
        echo "  during the Linux 'npm run tauri build'. Re-run with signing env." >&2
        exit 2
    fi

    echo "info: computing SHA-256 of $(basename "$LINUX_ARTIFACT_PATH")..." >&2
    LINUX_SHA256=$(shasum -a 256 "$LINUX_ARTIFACT_PATH" | awk '{print $1}')
    echo "info: linux sha256 = $LINUX_SHA256" >&2

    # Same single-line Tauri .sig format + defensive newline-escaping as
    # the macOS signature above.
    LINUX_SIGNATURE_RAW=$(cat "$LINUX_SIGNATURE_FILE")
    LINUX_SIGNATURE_JSON="${LINUX_SIGNATURE_RAW%$'\n'}"
    LINUX_SIGNATURE_JSON=$(perl -pe 's/\n/\\n/g' <<< "$LINUX_SIGNATURE_JSON")
    LINUX_SIGNATURE_JSON="${LINUX_SIGNATURE_JSON%\\n}"

    LINUX_URL="https://github.com/msitarzewski/brew-browser/releases/download/v${VERSION}/${LINUX_ARTIFACT_RELEASE_NAME}"

    # Leading comma + newline so it appends cleanly after the
    # darwin-aarch64 entry inside the "platforms" object.
    LINUX_BLOCK=$(cat <<EOF
,
    "linux-x86_64": {
      "signature": "${LINUX_SIGNATURE_JSON}",
      "url": "${LINUX_URL}",
      "sha256": "${LINUX_SHA256}"
    }
EOF
)
else
    echo "info: no Linux AppImage found — emitting macOS-only manifest." >&2
fi

# ---------- Emit manifest ----------

mkdir -p "$DIST_DIR"

# Release notes are intentionally a placeholder. The user can hand-edit
# the manifest before the rsync to inject the CHANGELOG entry — keeping
# the manifest generator and the release notes editorial step separate
# is the simpler shape than wiring CHANGELOG parsing here.
PUB_DATE=$(date -u +%Y-%m-%dT%H:%M:%SZ)
AARCH64_URL="https://github.com/msitarzewski/brew-browser/releases/download/v${VERSION}/${AARCH64_RELEASE_NAME}"
X64_URL="https://github.com/msitarzewski/brew-browser/releases/download/v${VERSION}/${X64_RELEASE_NAME}"

PLATFORMS_JSON="    \"darwin-aarch64\": {
      \"signature\": \"${AARCH64_SIGNATURE_JSON}\",
      \"url\": \"${AARCH64_URL}\",
      \"sha256\": \"${AARCH64_SHA256}\"
    }"
if [[ $HAVE_X64 -eq 1 ]]; then
    PLATFORMS_JSON="${PLATFORMS_JSON},
    \"darwin-x86_64\": {
      \"signature\": \"${X64_SIGNATURE_JSON}\",
      \"url\": \"${X64_URL}\",
      \"sha256\": \"${X64_SHA256}\"
    }"
fi

cat > "$MANIFEST_PATH" <<EOF
{
  "version": "${VERSION}",
  "notes": "See https://github.com/msitarzewski/brew-browser/releases/tag/v${VERSION} for release notes.",
  "pub_date": "${PUB_DATE}",
  "platforms": {
${PLATFORMS_JSON}${LINUX_BLOCK}
  }
}
EOF

echo ""
echo "✓ manifest written to: $MANIFEST_PATH"
echo "  version:           $VERSION"
echo "  pub_date:          $PUB_DATE"
echo "  aarch64 sha256:    $AARCH64_SHA256"
echo "  aarch64 url:       $AARCH64_URL"
echo "  aarch64 signature: $(wc -c < "$AARCH64_SIGNATURE_FILE" | tr -d ' ') bytes from $AARCH64_SIGNATURE_FILE"
if [[ $HAVE_X64 -eq 1 ]]; then
    echo "  x86_64 sha256:     $X64_SHA256"
    echo "  x86_64 url:        $X64_URL"
    echo "  x86_64 signature:  $(wc -c < "$X64_SIGNATURE_FILE" | tr -d ' ') bytes from $X64_SIGNATURE_FILE"
else
    echo "  x86_64:            ABSENT — darwin-x86_64 key omitted (see warning above)"
fi
echo ""
echo "next steps (run manually):"
echo "  1. rsync -av $MANIFEST_PATH umacbookpro:Sites/brew-browser/updater.json"
echo "  2. gh release create v${VERSION} \\"
echo "       src-tauri/target/release/bundle/dmg/brew-browser_${VERSION}_aarch64.dmg \\"
if [[ $HAVE_X64 -eq 1 ]]; then
    echo "       src-tauri/target/x86_64-apple-darwin/release/bundle/dmg/brew-browser_${VERSION}_x64.dmg \\"
fi
echo "       $AARCH64_ARTIFACT_PATH#${AARCH64_RELEASE_NAME} \\"
if [[ $HAVE_X64 -eq 1 ]]; then
    echo "       $X64_ARTIFACT_PATH#${X64_RELEASE_NAME} \\"
fi
echo "       --notes-file <release-notes.md>"
echo ""
echo "verify before publishing:"
echo "  shasum -a 256 $AARCH64_ARTIFACT_PATH"
if [[ $HAVE_X64 -eq 1 ]]; then
    echo "  shasum -a 256 $X64_ARTIFACT_PATH"
fi
echo "  curl -s https://brew-browser.zerologic.com/updater.json | jq"
