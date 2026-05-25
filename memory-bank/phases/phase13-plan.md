# Phase 13 Plan — Catalog Enrichment via Haiku

**Created:** 2026-05-24 (queued during Phase 12 execution)
**Driver:** User prompt — "add the build automation for fetching meta data with haiku for those that need it… does anything like that already exist in the user's brew install? Will we get pushback?"
**Design decision:** `decisions.md` § "2026-05-24-night: LLM-enriched catalog metadata (offline-generated, runtime-rendered, single AI Features toggle)" — to be added when Phase 13 starts.

## Goal

The bundled catalog (Phase 12a) gives us all 16k+ Homebrew tokens with structured metadata (deps, license, homepage, version) but nothing human-friendly beyond a terse `desc`. Phase 13 generates LLM-derived enrichment **at build time** against the public catalog, ships it as a separate gzipped data file alongside the catalog, and renders it in the UI behind a single Settings toggle.

**Critical posture:** zero runtime LLM calls. The LLM only ever sees the *public catalog*. User data never leaves the app. The Settings toggle controls *rendering*, not *fetching*.

## Pushback mitigations (built in from day one)

- Every enriched field carries an "AI-enriched" badge + tooltip showing provenance (model + date + script source)
- The original brew `desc` always renders alongside the AI summary, never replaced
- A "Wrong?" link on every AI-derived field deeplinks to a prefilled GitHub issue on `msitarzewski/brew-browser` (Phase 12f extends this when signed in)
- Master Settings toggle "Show AI-enriched data" — single bool, default ON, predictable
- README dedicated section: "AI-enriched data — what, why, where it lives, how to disable"
- BUILD.md documents the enrichment pipeline so anyone can rebuild from source

## Tier structure (ship in order)

### Tier A — Friendly names + expanded summaries (~$3-5 against Haiku 4.5)

For each package where `desc` is missing or under 50 chars, generate:
- **`friendly_name`**: human-readable name (`postgresql@14` → `"PostgreSQL 14"`, `ffmpeg` → `"FFmpeg"`)
- **`summary`**: 1-2 sentence "what it is + when you'd want it"

Ship as additional fields in a new `enrichment.json.gz` keyed by token. ~5,000 packages need enrichment (rest have decent `desc` already).

### Tier B — Use cases + similar packages + tags (~$10-15)

For all 15,974 packages:
- **`use_cases`**: 1-3 short bullets — "Why would I install this?"
- **`similar`**: 3-5 related package names (LLM-clustered by purpose, not just same category)
- **`tags`**: tech-stack tags beyond the 19 categories (`react`, `rust`, `kubernetes`, `audio-production`, etc.)

### Tier C — Defer for v2 / community contributions

- **`difficulty`**: beginner/intermediate/advanced setup
- **`gotchas`**: common pitfalls ("conflicts with system X", "requires …")
- **`follow_up`**: post-install steps ("then run `brew services start …`")
- **`estimated_install_time`**: derived from analytics + bottle availability

## Architecture

### Build-time

**New tool `tools/enrich/enrich.py`:**
- Mirrors `tools/categorize/` pattern
- Reads from `src-tauri/data/catalog/{formula,cask}.json.gz` (Phase 12a's bundled catalog)
- Diff-based: state file `tools/enrich/state/last-snapshot.json` records token + hash(name+desc); re-enrich only when hash changes
- Batch-of-50 Haiku calls; one prompt per tier
- Writes `src-tauri/data/enrichment.json.gz` with shape:
  ```json
  {
    "version": "2026-05-24",
    "generated_at": "ISO8601",
    "model": "claude-haiku-4-5",
    "tiers": ["A", "B"],
    "entries": {
      "<token>": {
        "friendly_name": "PostgreSQL 14",
        "summary": "...",
        "use_cases": ["...", "..."],
        "similar": ["redis", "mongodb", "..."],
        "tags": ["database", "sql", "..."]
      }
    }
  }
  ```
- Same `.env` pattern (`ANTHROPIC_API_KEY`), `.gitignored`
- Same diff state pattern so re-runs cost ~$0.01 not $15

**Build chain** (documented in BUILD.md):
```
tools/catalog/fetch.py        # refresh formula.json + cask.json
  → tools/categorize/categorize.py  # update categories.json (delta only)
  → tools/enrich/enrich.py          # update enrichment.json (delta only)
  → cargo tauri build               # bake into binary
```

### Runtime

**Backend** `src-tauri/src/commands/enrichment.rs`:
- `enrichment_data() -> Arc<EnrichmentData>` — bundled JSON via `include_bytes!` + flate2 + serde_json, memoised
- `enrichment_lookup(token: String) -> Option<EnrichmentEntry>` — validate token first, then HashMap lookup
- Same security pattern as catalog: include_bytes for bundle, no runtime fetch

**Frontend** `src/lib/stores/enrichment.svelte.ts`:
- Lazy-load on first access
- `lookup(token)` helper
- `enabled` derived from `ui.aiFeaturesEnabled` — short-circuits to `null` when disabled

**Frontend rendering** (all gated on `ui.aiFeaturesEnabled`):
- **PackageDetail.svelte:**
  - `friendly_name` shown as h1 (if present), original `name` shown smaller below — falls back to `name` only when off
  - `summary` shown above brew's `desc` — both render when on; only `desc` when off
  - "Use cases" expandable card
  - "Similar packages" pills row (clickable → PackageDetail for that package)
  - Tags row below categories
  - Every AI-derived field carries the "AI-enriched" badge
- **Discover, Library, Trending** rows:
  - Show `friendly_name` in addition to `name` if available
  - Filter chips: tags become an additional filter dimension alongside categories
- **Dashboard:**
  - "Top categories" donut stays (categories are tier 0 — generated by the existing categorize tool)
  - If we want a Phase 13 dashboard widget: "Recommended for you" based on tags ∩ installed mix

### Settings UI (extends Phase 12b's Appearance section)

```
Show AI-enriched data        [ ON | OFF ]   (default ON)

When on, brew-browser shows extra information generated by AI from
the public Homebrew catalog: friendly names, expanded descriptions,
use cases, similar package suggestions, and category tags. All AI
processing happens at build time on our infrastructure — no LLM
calls are made from your machine.

When off, only brew's native metadata is shown.
```

Granular sub-toggles in an "Advanced…" disclosure (optional v2):
- Show friendly names
- Show expanded summaries
- Show use cases
- Show similar packages
- Show tags
- Show categories (turning this off disables the entire Phase 9 system)

## Cost & cadence

- **Initial Tier A run:** ~$3-5 against Haiku 4.5
- **Initial Tier B run:** ~$10-15 — total Phase 13 budget ~$20
- **Delta runs:** ~$0.01-0.05 per refresh (~30-50 packages change per week)
- **Cron:** daily 4am `python enrich.py` on Beast or umbp, commit + push if delta non-empty
- **Bundled file size:** Tier A ~500 KB gzipped, Tier B ~2 MB gzipped, Tier C TBD. Total bundle grows from 6 MiB (catalog) to ~8.5 MiB

## Tasks for the Phase 13 agent waves

**Wave 1 (parallel):**
1. **Backend Architect** → `tools/enrich/enrich.py` script + `src-tauri/data/enrichment.json.gz` bundled fixture + `src-tauri/src/enrichment/mod.rs` + `commands/enrichment.rs` + 6-8 tests
2. **Frontend Developer** → `enrichment.svelte.ts` store + `ui.aiFeaturesEnabled` toggle + Settings Appearance section addition + master-toggle wiring (every existing render site gated)

**Wave 2 (after Wave 1):**
3. **Frontend Developer** → PackageDetail Tier A rendering (friendly name + summary + AI-enriched badge + Wrong? link)
4. **Frontend Developer** → Tier B rendering (use cases, similar, tags) — possibly split into two PRs

**Wave 3 (after Wave 2):**
5. **Backend Architect** → categorize-style cron deployment to umbp (daily 4am)
6. **Technical Writer** → README "AI-enriched data" section, BUILD.md enrichment pipeline doc, security.md addition

## Security gates (mirror Phase 12a patterns)

- `enrichment.json.gz`: size-capped on parse (32 MiB raw / 64 MiB decompressed)
- Field-length caps (`friendly_name ≤ 100`, `summary ≤ 1024`, etc.) — defense-in-depth even though the LLM produces bounded output
- IPC commands validate token names via existing validators
- Bundled-only at first; **no user-refresh path** for v1 (the build artifact IS the canonical enrichment — runtime stays purely local)
- If a future v2 wants user-refresh, follows the same "user-initiated only" + "atomic write + corrupt fallback" pattern as the catalog refresh

## Acceptance criteria for Phase 13 v1 (Tier A + B)

- `python tools/enrich/enrich.py` runs end-to-end against an `ANTHROPIC_API_KEY` from `.env`, produces ~2.5 MB gzipped output + delta state file
- `cargo test` adds 6+ enrichment tests (bundle parses, lookup hit/miss, token validator, deprecated-flag preservation)
- `npm run check` clean
- AI Features toggle works: ON renders enriched data with badges; OFF reverts to brew-native everywhere
- Friendly names shown in Discover/Library row headers
- PackageDetail shows summary, use cases, similar, tags (each with AI badge + Wrong? link)
- Sub-toggle plumbing is optional v2 — single master toggle is required v1

## Out of scope for Phase 13

- Granular sub-toggles (queue for v2 if anyone asks)
- Tier C fields (difficulty, gotchas, follow-up, install time)
- Recipes (Phase 10) — pairs naturally with enrichment but is its own phase
- "Recommended for you" dashboard widget — could ship after Tier B as a small follow-up

## Dependencies

- **Phase 12a** (bundled catalog) — required, gives us the data source. **Done.**
- **Phase 12b** (Settings shell) — required, gives us the toggle home. **Done.**
- **Phase 12d** (paranoid + settings persistence) — recommended, gives the AI toggle a persistent settings.json home instead of localStorage
- Otherwise independent of remaining Phase 12 work
