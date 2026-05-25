# Deferred / dropped tasks

Running ledger of work that was scoped but explicitly not shipped (yet, or ever). Kept separate from the per-phase task records because the *non-decision* to ship is itself worth tracking.

## Dropped

### Phase 14 — Bundled cask icons
**Status:** Dropped (planning stage)
**Reason:** Trademark / redistribution risk. Bundling third-party app icons (Visual Studio Code, Slack, Figma, etc.) inside our `.dmg` would require per-cask licensing review with no clear safe-harbor. The Phase 8 homepage-cascade probe is the working alternative — it pulls the icon from the cask's own homepage at runtime with SSRF defense + user-controlled rate limiting (Settings → Network → Cask icon mode: Off / InstalledOnly / All).
**See:** `decisions.md` for the full reasoning. Will not revisit without a legal review.

## Deferred (intentionally not shipped, may return)

### Phase 10 — Recipes
**Status:** Deferred
**Why deferred initially:** Originally blocked on having a real catalog. Catalog landed in Phase 12a — now technically unblocked.
**Why still not done:** No clear "v1 recipe" concept that's both useful AND scoped tightly enough to ship in a session. Pairs naturally with Phase 13 enrichment (use-cases) but that data isn't fully baked yet — only Tier A (friendly names + summaries) is in the bundle; Tier B (use-cases + similar + tags) costs ~$15 to run and hasn't been bought.
**Revisit when:** Tier B enrichment lands. Then "Recipes" can mean "curated 3-5 package combos by use case" (e.g., "Web Dev Starter: node + git + watchman + httpie + jq").

### Phase 9d — `installedAt` on Package + Last-Updated sort
**Status:** Deferred (small standalone)
**Scope:** Add `installed_at: chrono::DateTime<Utc>` to the `Package` struct, populated from `brew info`'s install record. Wire a "Last Updated" sortable column in Library.
**Why not yet:** Small enough that no agent wave's been spent on it; big enough (touches Package serde + tests + frontend sort wiring) that it doesn't fit the gaps between bigger phases. Picks up easily whenever.
**Estimated effort:** 1-2 hours.

### Tier B enrichment run
**Status:** Deferred (cost-gated)
**Scope:** `python tools/enrich/enrich.py --tier-b` — adds `use_cases` (array of one-line bullets), `similar` (related-package tokens), `tags` (fine-grained tech-stack labels) to every entry in `enrichment.json.gz`.
**Cost:** ~$10-15 against user's Anthropic API
**Why deferred:** Tier A is sufficient for the dashboard donut + Discover labels. Tier B unlocks deeper PackageDetail sections ("Why install this?", "Similar packages", "Tags") that currently render only the empty-state for most packages. Worth running once there's clear user demand or a v0.3/v0.4 release pegged to it.

### Daily / weekly categorize + enrich cron
**Status:** Deferred (infrastructure stub)
**Scope:** Cron on beast or umbp that nightly: (1) fetches latest catalog from formulae.brew.sh, (2) diffs against bundled catalog to find new/changed packages, (3) categorizes + enriches just the delta (cheap, <$0.10 per night), (4) opens a PR with the updated `categories.json` + `enrichment.json.gz`.
**Why deferred:** Bundle is fresh enough at each release. Manual `python tools/catalog/fetch.py && python tools/categorize/categorize.py && python tools/enrich/enrich.py --tier-a` before each release works for now. Cron becomes worth setting up when release cadence drops below monthly.

### Tier B Tahoe Liquid Glass (Swift bridge)
**Status:** Deferred to v0.3+
**Scope:** Replace the current `window-vibrancy` `NSVisualEffectMaterial::HudWindow` with macOS Tahoe's new "Liquid Glass" material via a Swift FFI bridge. Tauri 2 doesn't expose this directly.
**Why deferred:** Current vibrancy looks good. Liquid Glass is incremental polish, not a missing feature. Worth a session once the v0.3.0 ship dust settles.

### `brew tap msitarzewski/brew-browser` for one-line install
**Status:** Deferred (post-launch)
**Scope:** Publish a Homebrew tap that lets users `brew install --cask brew-browser` instead of downloading the `.dmg` manually.
**Why deferred:** Have to set up a separate `homebrew-brew-browser` repo, write the cask formula, point it at the GitHub Releases asset, version-bump on each release. Worth doing once release cadence stabilizes.

## Removed conventions

### `tasks/YYYY-MM/*.md` per-task record protocol
**Status:** Was specified in `toc.md` but not used through Phase 1 → Phase 15. **Restored** retroactively in this very file's directory by reconstructing task records from `progress.md` + git history (2026-05-25). Going forward: each shipped task or non-trivial cleanup gets a task record before the commit that ships it.

### Per-agent `agentLog.md` stamping
**Status:** Was specified in `toc.md` ("Stamp every run") but lagged: early phases stamped; Phase 12 onward got sparse; Phase 15's 4-agent wave didn't stamp at all (Lead didn't include it in the agent prompts). Convention is currently dormant. To re-enable: include "append one line to agentLog.md" in every Agent prompt going forward, or drop the requirement from toc.md.
