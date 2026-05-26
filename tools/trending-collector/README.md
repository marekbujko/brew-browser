# brew-browser trending-collector

Nightly collector for the brew-browser trending-history endpoint. Deploys to **`brew-browser.zerologic.com`**, runs once a night, and emits the static JSON tree the brew-browser app fetches when a user has opted into "Enhanced Trending History" in Settings → Network.

## What it does

1. Fetches the four published categories × three time windows from `formulae.brew.sh`:
   - `install`, `install-on-request`, `cask-install`, `build-error`
   - 30d, 90d, 365d each
   - 12 HTTP GETs total, all concurrent
2. Appends today's snapshot rows to a SQLite DB at `state/db.sqlite`.
3. Regenerates the static JSON output:
   - `out/index.json` — top-500 packages by velocity index, each with a compact ~30-point sparkline
   - `out/formula/<name>.json` — per-formula full history series
   - `out/cask/<name>.json` — per-cask full history series

Caddy serves the contents of `out/` at `https://brew-browser.zerologic.com/trending-history/*`.

## Why "seed" data?

The day the collector goes live, the SQLite DB is empty — but we don't want users who opt in to see empty charts for two weeks. So the bootstrap run (`seed.js`) derives three historical "buckets" per package by subtracting the rolling windows:

- **Recent bucket** (days 0–30): `count_30d`, midpoint = today − 15
- **Mid bucket** (days 31–90): `count_90d − count_30d`, midpoint = today − 60
- **Older bucket** (days 91–365): `count_365d − count_90d`, midpoint = today − 228

Stored with `source='seed'` so the renderer can distinguish them from real daily snapshots. From day 1 onward, the nightly `collect.js` builds up real daily granularity; after ~30 days of nightly snapshots, adjacent-day subtraction produces clean per-day install counts.

## Requirements

- **Node 20+** (built-in `fetch`)
- **SQLite** (via the bundled `better-sqlite3` native module)

That's it. No Rust, no Python, no transpilation.

## Deploy

The canonical deploy target is `brew-browser.zerologic.com`. Layout:

```
/home/michael/Sites/brew-trending-collector/    ← this code (clone of tools/trending-collector)
/home/michael/data/brew-trending/db.sqlite      ← SQLite state, persists across deploys
/home/michael/Sites/brew-trending/              ← static JSON output (served by Caddy)
```

### 1. First deploy (one-time)

```sh
# On brew-browser.zerologic.com:
mkdir -p /home/michael/Sites/brew-trending-collector
mkdir -p /home/michael/data/brew-trending
mkdir -p /home/michael/Sites/brew-trending

# Sync the code (from your local checkout):
rsync -av --delete \
  --exclude=node_modules --exclude=state --exclude=out \
  tools/trending-collector/ \
  brew-browser.zerologic.com:/home/michael/Sites/brew-trending-collector/

# Install deps on the box:
ssh brew-browser.zerologic.com \
  'cd /home/michael/Sites/brew-trending-collector && npm ci --omit=dev'

# Bootstrap the DB with today's seed buckets:
ssh brew-browser.zerologic.com \
  'cd /home/michael/Sites/brew-trending-collector && \
   DB_PATH=/home/michael/data/brew-trending/db.sqlite \
   OUT_DIR=/home/michael/Sites/brew-trending \
   node seed.js'

# Trigger an initial collect so the JSON tree exists immediately
# (the cron runs once a night, but day-0 needs a kick):
ssh brew-browser.zerologic.com \
  'cd /home/michael/Sites/brew-trending-collector && \
   DB_PATH=/home/michael/data/brew-trending/db.sqlite \
   OUT_DIR=/home/michael/Sites/brew-trending \
   node collect.js'
```

### 2. Cron schedule

Add to `crontab -e` on `brew-browser.zerologic.com`:

```cron
# Nightly trending-history collection — 03:00 server time.
0 3 * * * cd /home/michael/Sites/brew-trending-collector && DB_PATH=/home/michael/data/brew-trending/db.sqlite OUT_DIR=/home/michael/Sites/brew-trending /usr/bin/node collect.js >> /var/log/brew-trending-collector.log 2>&1
```

Adjust `/usr/bin/node` to match the actual Node binary path (`which node`).

### 3. Caddy vhost

`brew-browser.zerologic.com` already serves the updater manifest. Add a `handle_path` block for `/trending-history/*` to the existing vhost — see `memory-bank/security.md` for the exact snippet, which includes the IP-redaction logging directive that makes the privacy claim auditable.

## Subsequent updates

```sh
# Sync code changes:
rsync -av --delete \
  --exclude=node_modules --exclude=state --exclude=out \
  tools/trending-collector/ \
  brew-browser.zerologic.com:/home/michael/Sites/brew-trending-collector/

# Re-install if package.json changed:
ssh brew-browser.zerologic.com \
  'cd /home/michael/Sites/brew-trending-collector && npm ci --omit=dev'
```

The DB and JSON output dirs are outside this directory tree so they survive deploys cleanly.

## Local development

```sh
cd tools/trending-collector
npm install
node seed.js     # one-shot bootstrap; writes state/db.sqlite + out/*.json
node collect.js  # add a "today" snapshot + re-render
```

`state/` and `out/` are gitignored. Inspect the DB with `sqlite3 state/db.sqlite`.

## Re-seeding

`seed.js` refuses to run if any `source='seed'` rows already exist. To force a re-seed:

```sh
sqlite3 state/db.sqlite "DROP TABLE snapshots;"
node seed.js
```

Note: this also wipes the real daily history. Only do this if the bootstrap was genuinely bogus.

## What's NOT here

- **No web server** — the collector writes static files. Caddy serves them.
- **No retention policy** — at ~15MB/year for the full top-10K, SQLite growth is a non-issue for the foreseeable future. Revisit when the DB passes 1 GB.
- **No alerting** — failures log to stderr; pipe the cron output to a log file and grep for `FAILED` or `fatal`. Add proper monitoring if/when this becomes load-bearing.
