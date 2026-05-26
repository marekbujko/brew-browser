#!/usr/bin/env node
// Nightly collector for the brew-browser trending-history endpoint.
//
// Hits formulae.brew.sh for the current 30d/90d/365d counts across all
// four published categories (install, install-on-request, cask-install,
// build-error), inserts them as daily snapshots into the SQLite DB,
// then regenerates the static JSON tree consumed by the brew-browser
// app via the opt-in `enhancedTrendingEnabled` setting.
//
// Idempotent against the same date — PK collision on (name, kind,
// date, category, window) means re-running this on the same day is a
// no-op for the DB, only the JSON render re-runs.

import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  CATEGORIES,
  WINDOWS,
  extractItems,
  fetchAnalyticsPayload,
  openDb,
  parseCount,
  todayISO,
} from "./lib/common.js";
import { renderAll } from "./lib/render.js";

const __dirname = dirname(fileURLToPath(import.meta.url));

const DB_PATH = process.env.DB_PATH ?? resolve(__dirname, "state/db.sqlite");
const OUT_DIR =
  process.env.OUT_DIR ??
  resolve(__dirname, "out");
// On the deploy target (brew-browser.zerologic.com) this should be set
// to `/home/michael/Sites/brew-trending/` via the cron line so the
// JSON tree lands where Caddy can serve it.

async function main() {
  const startedAt = Date.now();
  const today = todayISO();
  console.log(`[collect] ${today}  db=${DB_PATH}  out=${OUT_DIR}`);

  const db = openDb(DB_PATH);

  // Fetch every (category, window) combination concurrently. 4×3 = 12
  // HTTP requests; formulae.brew.sh is a static CDN, this is fine.
  const fetches = [];
  for (const cat of CATEGORIES) {
    for (const win of WINDOWS) {
      fetches.push(
        fetchAnalyticsPayload(cat, win)
          .then((payload) => ({ cat, win, payload, ok: true }))
          .catch((err) => {
            console.error(
              `[collect]   FAILED ${cat} ${win}: ${err.message}`,
            );
            return { cat, win, payload: null, ok: false };
          }),
      );
    }
  }
  const results = await Promise.all(fetches);

  // Insert all successful results in a single transaction. The PK
  // covers (name, kind, date, category, window) so re-runs are safe.
  // We use INSERT OR IGNORE so re-running the same day leaves the
  // original count intact (operator can DELETE-then-rerun if needed).
  const insert = db.prepare(
    `INSERT OR IGNORE INTO snapshots
       (package_name, kind, snapshot_date, category, window, count, source)
     VALUES (@name, @kind, @date, @category, @window, @count, 'daily')`,
  );

  let insertedRows = 0;
  let failedFetches = 0;
  const txn = db.transaction(() => {
    for (const r of results) {
      if (!r.ok) {
        failedFetches += 1;
        continue;
      }
      const items = extractItems(r.payload);
      const kind = r.cat === "cask_install" ? "cask" : "formula";
      for (const item of items) {
        if (!item?.formula) continue;
        const count = parseCount(item.count);
        if (count === 0) continue;
        const result = insert.run({
          name: item.formula,
          kind,
          date: today,
          category: r.cat,
          window: r.win,
          count,
        });
        if (result.changes > 0) insertedRows += 1;
      }
    }
  });

  txn();

  console.log(
    `[collect] fetched ${results.length} endpoints ` +
      `(${failedFetches} failed), inserted ${insertedRows} new rows`,
  );

  // Regenerate JSON output from current DB state.
  const renderStart = Date.now();
  const stats = await renderAll(db, OUT_DIR);
  console.log(
    `[collect] render: ${stats.indexEntries} index entries, ` +
      `${stats.perPackageFiles} per-package files in ${Date.now() - renderStart}ms`,
  );

  db.close();
  console.log(`[collect] done in ${Date.now() - startedAt}ms`);
}

main().catch((e) => {
  console.error("[collect] fatal:", e);
  process.exit(1);
});
