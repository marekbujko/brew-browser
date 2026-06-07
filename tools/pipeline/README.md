# brew-browser — nightly data pipeline

Build-time tooling. Regenerates the AI **categories** + **descriptions** nightly
and renders the static tree the app fetches for **opt-in live updates**.

Deployed as a **copy** (rsync), like `tools/trending-collector` — no git on the
host, no data committed back from the host. Same model as trending: the served
tree is the delivery mechanism; the app fetches it live.

## What it does (`nightly-refresh.sh`)

1. `tools/catalog/fetch.py` — refresh the Homebrew catalog.
2. `tools/categorize/categorize.py` — incremental category pass → `src-tauri/data/categories.json`.
3. `tools/enrich/enrich.py --tier-a` — incremental description pass → `src-tauri/data/enrichment.json.gz`.
4. `tools/pipeline/render_served.py` — render `$OUT_DIR/{version.json, categories.json, entry/<token>.json}`.

Each regen step is diff-aware (its own `state/`), so a typical night hits the
API for only the handful of packages that changed.

The repo's **bundled baseline** (`src-tauri/data/*`) is refreshed *manually at
release time*: run this pipeline locally and `git commit src-tauri/data`. Live
fetch covers day-to-day freshness for opted-in users between releases.

## Configuration (env vars — no private host/paths in this repo)

| Var | Default | Meaning |
|-----|---------|---------|
| `TOOL_DIR` | 2 levels up from the script | the deployed copy holding `tools/` + `src-tauri/data/` |
| `OUT_DIR` | `$TOOL_DIR/tools/pipeline/out` | served render target (point at the web-served dir) |
| `PYTHON` | `$TOOL_DIR/.venv/bin/python` | interpreter |
| `ENRICH_FLAGS` | `--tier-a` | enrich tiers to run |

## Deploy (rsync a minimal copy — substitute your own host/paths)

```sh
# From a repo checkout, rsync only what the pipeline needs to the host:
TOOL_DIR=<host>:/path/to/tools/brew-browser/enrichment
rsync -az --delete tools/ "$TOOL_DIR/tools/"
rsync -az --delete src-tauri/data/ "$TOOL_DIR/src-tauri/data/"   # seed + working data

# On the host, once:
cd "$TOOL_DIR_LOCAL"
python3 -m venv .venv
.venv/bin/pip install -r tools/categorize/requirements.txt -r tools/enrich/requirements.txt
cp tools/enrich/.env.example tools/enrich/.env          # paste ANTHROPIC_API_KEY
cp tools/categorize/.env.example tools/categorize/.env  # same key

# First run (full bulk — minutes, a few $). Subsequent runs are incremental.
TOOL_DIR="$TOOL_DIR_LOCAL" OUT_DIR="$OUT_DIR" tools/pipeline/nightly-refresh.sh
```

Re-rsync `tools/` whenever the pipeline code changes (the host copy is not a
git checkout — it doesn't pull).

### Cron (after the trending collector settles)

```cron
30 3 * * * TOOL_DIR="$TOOL_DIR" OUT_DIR="$OUT_DIR" "$TOOL_DIR/tools/pipeline/nightly-refresh.sh" >> "$LOGFILE" 2>&1
```

### Caddy (serve `$OUT_DIR` at `…/enrichment/*`)

Mirror the trending-history block (6h cache, IP-redacted logs — see
`memory-bank/security.md`):

```caddy
handle_path /enrichment/* {
    root * {$OUT_DIR}
    file_server
    @writes method POST PUT DELETE PATCH
    respond @writes 405
    header {
        Cache-Control "public, max-age=21600"
        -Set-Cookie
        -Server
    }
}
```

The app fetches `https://<public-domain>/enrichment/{version.json,categories.json,entry/<token>.json}`
only when the user opts in (Settings → live category/description updates) and
Offline Mode is off.
