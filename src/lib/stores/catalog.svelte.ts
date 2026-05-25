/**
 * Catalog store (Phase 12a) — wraps the bundled/user-refreshed Homebrew
 * catalog metadata exposed by the 6 `catalog_*` Tauri commands.
 *
 * Owns:
 *   - the cached `CatalogSummary` shown on the Dashboard hero strip
 *   - the in-flight refresh state for the Refresh button
 *   - the derived "is the catalog stale?" gate used by the Discover banner
 *   - per-token lookup wrappers (formula / cask) for any caller that
 *     wants the full record without going through `api.ts` directly
 *
 * The store is intentionally thin: the backend is the source of truth.
 * We never mirror the entire catalog into JS; we cache just the summary
 * + on-demand single-token lookups so the renderer stays light.
 */

import {
  catalogLookupCask,
  catalogLookupFormula,
  catalogRefresh,
  catalogSummary,
} from "$lib/api";
import { settings } from "$lib/stores/settings.svelte";
import {
  brewErrorMessage,
  isBrewError,
  type Cask,
  type CatalogSummary,
  type Formula,
} from "$lib/types";

class CatalogStore {
  /** Last-known summary, or `null` until `ensureLoaded()` resolves. */
  summary: CatalogSummary | null = $state(null);

  /** True while a `catalog_refresh` IPC call is in flight. UI disables
      the Refresh button on this so a double-click can't queue. */
  refreshing: boolean = $state(false);

  /** Human-readable message from the most recent refresh failure, or
      `null` after a successful refresh / when no refresh has happened. */
  refreshError: string | null = $state(null);

  private summaryLoadPromise: Promise<void> | null = null;

  /** Lazy-load the catalog summary on first access. Safe to call from
      multiple mount points (the Dashboard hero + the Discover banner
      both read it). */
  async ensureLoaded(): Promise<void> {
    if (this.summary || this.summaryLoadPromise) {
      return this.summaryLoadPromise ?? Promise.resolve();
    }
    this.summaryLoadPromise = (async () => {
      try {
        this.summary = await catalogSummary();
      } catch (e) {
        // Summary load is a pure in-memory backend read — failures here
        // are surprising but should never break the UI shell. Record on
        // refreshError so the Dashboard can surface "catalog unavailable"
        // without throwing.
        this.refreshError = isBrewError(e) ? brewErrorMessage(e) : String(e);
      } finally {
        this.summaryLoadPromise = null;
      }
    })();
    return this.summaryLoadPromise;
  }

  /** Trigger a backend refresh. Updates `summary` on success; records
      a typed message on `refreshError` on failure. Handles the two
      domain-specific error codes (`paranoid_mode_blocked` and
      `brew_exit_non_zero`) with friendlier copy than the generic
      `brewErrorMessage` so the user knows exactly what to do. */
  async refresh(): Promise<boolean> {
    if (this.refreshing) return false;
    this.refreshing = true;
    this.refreshError = null;
    try {
      this.summary = await catalogRefresh();
      return true;
    } catch (e) {
      if (isBrewError(e)) {
        switch (e.code) {
          case "paranoid_mode_blocked":
            this.refreshError =
              "Offline mode is on — catalog refresh is blocked. Disable it in Settings → Network.";
            break;
          case "brew_exit_non_zero":
            this.refreshError =
              e.friendlyMessage ?? `brew failed during refresh (exit ${e.exitCode}).`;
            break;
          default:
            this.refreshError = brewErrorMessage(e);
        }
      } else {
        this.refreshError = String(e);
      }
      return false;
    } finally {
      this.refreshing = false;
    }
  }

  /** Convenience wrapper around `catalog_lookup_formula`. Returns `null`
      on miss (or on error — callers that need to distinguish should
      invoke `catalogLookupFormula` directly). */
  async lookupFormula(name: string): Promise<Formula | null> {
    try {
      return await catalogLookupFormula(name);
    } catch {
      return null;
    }
  }

  /** Convenience wrapper around `catalog_lookup_cask`. */
  async lookupCask(name: string): Promise<Cask | null> {
    try {
      return await catalogLookupCask(name);
    } catch {
      return null;
    }
  }

  /** Pretty "today" / "1 day old" / "N days old" label for the
      Dashboard hero strip. Returns "—" when the summary hasn't loaded
      yet so the spot doesn't flash empty. */
  get daysOldLabel(): string {
    if (!this.summary) return "—";
    const n = this.summary.daysOld;
    if (n <= 0) return "today";
    if (n === 1) return "1 day old";
    return `${n} days old`;
  }

  /** True when the active catalog is older than the user's stale-banner
      threshold (default 14 days from `settings.catalogStaleBannerDays`).
      Returns `false` until both the summary and the settings are loaded
      so the Discover banner doesn't flash on first paint. */
  get isStale(): boolean {
    if (!this.summary) return false;
    const threshold = settings.effective.catalogStaleBannerDays;
    return this.summary.daysOld > threshold;
  }
}

export const catalog = new CatalogStore();
