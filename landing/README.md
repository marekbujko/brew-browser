# landing

Static landing page for Brew Browser, served at `brew-browser.zerologic.com` via Caddy on the build host.

## Files

- `index.html` — the page (covers both builds: Tauri + native Swift/SwiftUI)
- `style.css` — embedded design tokens matching the app (dark-first, warm amber, OKLCH)
- `brew-browser.svg` — the app icon (copy of `../docs/icon/brew-browser.svg`)
- `screenshots/` — `dashboard-tauri.png`, `dashboard-native.png` (+ legacy shots)

## Deploy

Set `DEPLOY_HOST` to your ssh alias for the build host (kept out of this repo).
From this directory:

```sh
rsync -avz --exclude README.md ./ "$DEPLOY_HOST":Sites/brew-browser/
```

> ⚠️ **Do NOT add a bare `--delete`.** This same web root also serves
> `updater.json`, `/appcast.xml`, `/native/`, `/enrichment/*`, and
> `/trending-history/*` — none of which live in this directory. A `--delete`
> sync from here would wipe the Tauri updater, the native Sparkle appcast, and
> the enrichment/trending data for every user. If you must prune stale landing
> files, add `--delete` **with** an `--exclude` for each of those paths.

Caddy config on the host is managed manually.

## Update flow

1. Edit `index.html` / `style.css` locally
2. View locally: `python3 -m http.server -d . 8089` then open `http://localhost:8089`
3. `rsync` to the host when ready (command above)
4. Verify: `curl -s https://brew-browser.zerologic.com/updater.json | jq .version` still resolves (i.e. you didn't clobber the updater)
