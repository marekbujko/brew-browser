#!/usr/bin/env python3
"""Render the bundled categories + enrichment data into the static JSON tree
served for the app's opt-in *live* category/description updates (Phase 2).

Mirrors `tools/trending-collector/lib/render.js`: pure, stateless, atomic
(`.tmp` -> rename) writes. Reads the same data the apps bundle; emits a small
tree a static file server (Caddy `file_server`) exposes at `…/enrichment/*`.

Reads:
    src-tauri/data/categories.json
    src-tauri/data/enrichment.json.gz

Writes (under $OUT_DIR, default tools/pipeline/out):
    version.json            {version, generatedAt, categoriesVersion}
    categories.json         full categories file (app pulls when version newer)
    entry/<token>.json      per-token enrichment, camelCase wire shape
                            {friendlyName, summary, useCases, similar, tags, version}

No private host/paths here — the serve location is $OUT_DIR.
"""
import gzip
import json
import os
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
CATEGORIES_PATH = REPO_ROOT / "src-tauri" / "data" / "categories.json"
ENRICHMENT_GZ = REPO_ROOT / "src-tauri" / "data" / "enrichment.json.gz"
OUT_DIR = Path(os.environ.get("OUT_DIR", REPO_ROOT / "tools" / "pipeline" / "out"))

# Token allowlist — mirrors the app's fetch validators (native
# TrendingHistoryService / Tauri client). Anything outside this set is unsafe
# as a URL path segment + filename, so we skip it (the app couldn't request it
# anyway).
TOKEN_RE = re.compile(r"^[A-Za-z0-9._+@-]+$")


def write_atomic(path: Path, data: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(data)
    tmp.replace(path)


def main() -> int:
    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    categories = json.loads(CATEGORIES_PATH.read_text())
    with gzip.open(ENRICHMENT_GZ, "rb") as f:
        enrichment = json.loads(f.read())

    cats_version = categories.get("version", now)
    enr_version = enrichment.get("version", now)

    # version.json — tiny freshness probe the app polls on catalog refresh.
    write_atomic(OUT_DIR / "version.json", json.dumps({
        "version": enr_version,
        "generatedAt": now,
        "categoriesVersion": cats_version,
    }, separators=(",", ":")))

    # categories.json — full file; the app downloads it only when its
    # categoriesVersion is newer than the bundled/cached one.
    write_atomic(OUT_DIR / "categories.json",
                 json.dumps(categories, separators=(",", ":")))

    # entry/<token>.json — per-token enrichment, fetched on demand by the app
    # for packages it shows. camelCase to match the app's wire DTO.
    entries = enrichment.get("entries", {})
    written = skipped = 0
    for token, e in entries.items():
        if not TOKEN_RE.match(token):
            skipped += 1
            continue
        out = {
            "friendlyName": e.get("friendly_name"),
            "summary": e.get("summary"),
            "useCases": e.get("use_cases", []),
            "similar": e.get("similar", []),
            "tags": e.get("tags", []),
            "version": enr_version,
        }
        write_atomic(OUT_DIR / "entry" / f"{token}.json",
                     json.dumps(out, separators=(",", ":")))
        written += 1

    print(f"[render_served] version={enr_version} categoriesVersion={cats_version}")
    print(f"[render_served] entries: {written} written, {skipped} skipped (unsafe token)")
    print(f"[render_served] OUT_DIR={OUT_DIR}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
