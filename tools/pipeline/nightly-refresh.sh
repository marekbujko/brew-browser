#!/usr/bin/env bash
# Nightly regeneration of brew-browser's AI categories + descriptions, and the
# static tree served for the app's opt-in *live* updates.
#
# Deployed as a COPY (rsync), like the trending collector — there is NO git in
# this script. The rendered tree (render_served.py -> $OUT_DIR) IS the delivery
# mechanism; the app fetches it live. The repo's *bundled* baseline is refreshed
# manually at release time (run this locally, then commit src-tauri/data).
#
#   TOOL_DIR     dir holding tools/ + src-tauri/data/  (default: 2 levels up from this script)
#   OUT_DIR      served output dir (render target)     (default: $TOOL_DIR/tools/pipeline/out)
#   PYTHON       interpreter                            (default: $TOOL_DIR/.venv/bin/python)
#   ENRICH_FLAGS enrich.py tier flags                   (default: --tier-a)
#
# Each regen step is incremental (diff-aware via its own tools/*/state). Exits
# non-zero on hard error.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TOOL_DIR="${TOOL_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
PYTHON="${PYTHON:-$TOOL_DIR/.venv/bin/python}"
ENRICH_FLAGS="${ENRICH_FLAGS:---tier-a}"
export OUT_DIR="${OUT_DIR:-$TOOL_DIR/tools/pipeline/out}"

cd "$TOOL_DIR"
log() { printf '%s %s\n' "$(date -u +%FT%TZ)" "$*"; }

log "=== nightly-refresh start (tool=$TOOL_DIR out=$OUT_DIR) ==="
log "catalog fetch…";          "$PYTHON" tools/catalog/fetch.py
log "categorize…";             "$PYTHON" tools/categorize/categorize.py
log "enrich ($ENRICH_FLAGS)…"; "$PYTHON" tools/enrich/enrich.py $ENRICH_FLAGS
log "render served…";          "$PYTHON" tools/pipeline/render_served.py
log "=== nightly-refresh done ==="
