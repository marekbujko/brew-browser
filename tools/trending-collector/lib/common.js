// Shared helpers for the brew-browser trending-collector.
//
// Plain Node 20+ ESM, no TS step. The collector is small and lives on
// `brew-browser.zerologic.com`; keeping it dependency-light makes ops
// boring (`apt install nodejs && npm i` and it just runs).

import Database from "better-sqlite3";
import { mkdirSync } from "node:fs";
import { mkdir, writeFile } from "node:fs/promises";
import { dirname } from "node:path";

// ---------- Constants ----------

export const FORMULAE_HOST = "https://formulae.brew.sh/api/analytics";

/** The four categories the brew analytics API publishes. We pull all
 *  four every night so the historical record is complete; the app
 *  currently consumes `install` + `install_on_request`. */
export const CATEGORIES = ["install", "install_on_request", "cask_install", "build_error"];

export const WINDOWS = ["30d", "90d", "365d"];

/** Top-N to expose in the index.json summary blob. Per-package files
 *  cover everything; the index is just the leaderboard. */
export const INDEX_TOP_N = 500;

/** Sparkline length in the index entry. ~30 daily points is enough
 *  for the inline chart in the trending list to read trajectory. */
export const SPARKLINE_DAYS = 30;

/** User-agent for the outbound HTTP calls. Keeps brew-browser's
 *  identity visible in formulae.brew.sh's logs so they can reach out
 *  if our cadence ever becomes a problem. */
export const USER_AGENT =
  "brew-browser-trending-collector/0.4.0 (+https://github.com/msitarzewski/brew-browser)";

/** 10s per-request timeout. The endpoints serve static JSON from
 *  Cloudflare; a slow response usually means upstream is wedged. */
export const FETCH_TIMEOUT_MS = 10_000;

// ---------- Database ----------

/** Open (or create) the SQLite db at `dbPath`, applying the schema if
 *  needed. Returns a `better-sqlite3` Database instance. */
export function openDb(dbPath) {
  mkdirSync(dirname(dbPath), { recursive: true });
  const db = new Database(dbPath);
  db.pragma("journal_mode = WAL");
  db.pragma("synchronous = NORMAL");
  db.pragma("foreign_keys = ON");

  // Single normalized table — one row per (package, window, category,
  // snapshot date). `source` distinguishes seed buckets (derived from
  // rolling-window subtraction at bootstrap) from real daily snapshots
  // captured by the nightly cron. Composite PK prevents duplicates.
  db.exec(`
    CREATE TABLE IF NOT EXISTS snapshots (
      package_name  TEXT NOT NULL,
      kind          TEXT NOT NULL CHECK (kind IN ('formula', 'cask')),
      snapshot_date TEXT NOT NULL,
      category      TEXT NOT NULL CHECK (category IN ('install', 'install_on_request', 'cask_install', 'build_error')),
      window        TEXT NOT NULL CHECK (window IN ('30d', '90d', '365d')),
      count         INTEGER NOT NULL,
      source        TEXT NOT NULL DEFAULT 'daily' CHECK (source IN ('seed', 'daily')),
      PRIMARY KEY (package_name, kind, snapshot_date, category, window)
    );

    CREATE INDEX IF NOT EXISTS idx_snapshots_pkg
      ON snapshots (package_name, kind, snapshot_date);
    CREATE INDEX IF NOT EXISTS idx_snapshots_date
      ON snapshots (snapshot_date);
    CREATE INDEX IF NOT EXISTS idx_snapshots_cat_window
      ON snapshots (category, window, snapshot_date);
  `);

  return db;
}

// ---------- Velocity (mirror of src-tauri/src/trending/velocity.rs) ----------

/**
 * Implied 30-day velocity index. 1.0 = steady, >1.5 surging, <0.7 cooling.
 *
 * Compares the recent 30 days against the **prior 11 months** (days
 * 31..365), not the whole 365-day window — otherwise the recent month
 * double-counts as part of the baseline and brand-new packages get
 * the maximum-possible 12.17 ratio regardless of whether they're
 * actually trending.
 *
 * Returns `null` on degenerate inputs:
 *   - c365 == 0 (no historical data)
 *   - non-monotonic windows
 *   - c365 == c30 (brand-new package, no prior history)
 *   - prior-11-month monthly average < 1.0 (too few absolute installs)
 *
 * Matches the Rust implementation byte-for-byte so server-precomputed
 * and client-computed values agree.
 */
export function velocityIndex(c30, c90, c365) {
  if (c365 === 0 || c365 < c90 || c90 < c30) return null;
  const olderInstalls = c365 - c30;
  if (olderInstalls === 0) return null;
  // 335 days = 365 - 30. Normalize to per-30-day.
  const olderMonthlyAvg = olderInstalls / (335.0 / 30.0);
  if (olderMonthlyAvg < 1.0) return null;
  return c30 / olderMonthlyAvg;
}

// ---------- HTTP ----------

/** Fetch one (category, window) endpoint's JSON. Returns the parsed
 *  body. Throws on non-2xx or timeout. */
export async function fetchAnalyticsPayload(category, window) {
  // Map our internal category name to the URL path segment + the
  // optional tap-repo segment. Per the formulae.brew.sh API pattern
  // (`/api/analytics/{category}/{repo}/${DAYS}.json`), `install` /
  // `install-on-request` / `build-error` default to homebrew-core
  // when the repo segment is omitted, but `cask-install` needs the
  // `homebrew-cask` segment explicitly — otherwise the no-segment
  // URL returns an aggregated shape that doesn't expose the
  // per-cask counts we need.
  const seg = category.replaceAll("_", "-");
  const url =
    category === "cask_install"
      ? `${FORMULAE_HOST}/${seg}/homebrew-cask/${window}.json`
      : `${FORMULAE_HOST}/${seg}/${window}.json`;

  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(), FETCH_TIMEOUT_MS);
  try {
    const resp = await fetch(url, {
      signal: ctrl.signal,
      headers: { "User-Agent": USER_AGENT, Accept: "application/json" },
    });
    if (!resp.ok) {
      throw new Error(`HTTP ${resp.status} from ${url}`);
    }
    return await resp.json();
  } finally {
    clearTimeout(timer);
  }
}

/** Parse the `count` string from a brew analytics item (which is
 *  comma-formatted, e.g. `"1,234,567"`) into a clean integer. */
export function parseCount(s) {
  if (typeof s !== "string") return 0;
  const digits = s.replace(/[^0-9]/g, "");
  return digits ? Number.parseInt(digits, 10) : 0;
}

/** Extract the flat items array from the brew analytics JSON shape.
 *  Handles all three observed shapes:
 *
 *  1. `{ items: [{ formula, count, number }, ...] }` — install +
 *     install-on-request endpoints' live shape
 *  2. `{ formulae: { name: [{ formula, count }] } }` — older documented
 *     shape; the Rust client keeps this fallback too
 *  3. `{ formulae: { name: [{ cask, count }] } }` — cask-install
 *     endpoint's shape (note the `cask:` field instead of `formula:`)
 *
 *  Normalizes case (3) into the canonical `{ formula, count, number }`
 *  shape so downstream code only needs one field name. */
export function extractItems(payload) {
  if (Array.isArray(payload?.items) && payload.items.length > 0) {
    return payload.items;
  }
  if (payload?.formulae && typeof payload.formulae === "object") {
    return Object.values(payload.formulae)
      .map((arr) => (Array.isArray(arr) ? arr[0] : null))
      .filter(Boolean)
      .map((item) => {
        // Normalize the cask-install field name. `item.cask` may be
        // present instead of `item.formula`; copy it across so the
        // consumer's `item.formula` check works uniformly.
        if (item && !item.formula && item.cask) {
          return { ...item, formula: item.cask };
        }
        return item;
      });
  }
  return [];
}

// ---------- Date helpers ----------

/** Today's date as ISO `YYYY-MM-DD`. Uses UTC so cron behaviour is
 *  predictable across DST jumps; the cron schedule itself is in PT. */
export function todayISO() {
  return new Date().toISOString().slice(0, 10);
}

/** Subtract `days` from `iso` (`YYYY-MM-DD`) and return the resulting
 *  ISO date. */
export function shiftISO(iso, days) {
  const d = new Date(`${iso}T00:00:00Z`);
  d.setUTCDate(d.getUTCDate() + days);
  return d.toISOString().slice(0, 10);
}

// ---------- JSON output ----------

/** Atomic JSON file write. Writes to `.tmp` first then renames so
 *  Caddy never serves a half-written file. */
export async function writeJsonAtomic(path, obj) {
  const tmp = `${path}.tmp`;
  await mkdir(dirname(path), { recursive: true });
  await writeFile(tmp, JSON.stringify(obj));
  // Use the fs/promises rename, which is atomic within a single
  // filesystem (which is our case — output dir is on the same FS).
  const { rename } = await import("node:fs/promises");
  await rename(tmp, path);
}

// ---------- Path validation (mirrors backend client::fetch_package guard) ----------

const TOKEN_RE = /^[A-Za-z0-9._+@-]+$/;

/** True iff `name` is a safe brew token to use as a path segment.
 *  Refusing anything outside this set means the on-disk JSON tree
 *  never has weird filenames or escape characters. */
export function isSafePackageToken(name) {
  return typeof name === "string" && name.length > 0 && TOKEN_RE.test(name);
}
