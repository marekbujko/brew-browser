# 2026-05-24 — Categorize bulk run + initial landing page

**Phase:** Post-v0.1.0 release support
**Status:** ✅ Shipped
**Commit:** `c72e31d` (6 files, +50,421 / -4 lines — the +50K is `categories.json`)
**Release:** v0.1.0 supporting artifacts
**Date:** 2026-05-24 00:11 (twelve minutes after the initial release)

## Scope

Two parallel tracks landing together:

1. **Bulk LLM categorization** of the full Homebrew package index (15,974 items) into a structured `categories.json`, regenerable via `tools/categorize/categorize.py`. Drives the Discover tile grid (Phase 9a) and the Dashboard top-categories donut (Phase 11).
2. **Static landing page** at `landing/` for `brew-browser.zerologic.com`, served from umbp via Caddy (Caddy config maintained manually by user, not in repo).

## What landed

### Categorize bulk run
- `tools/categorize/categorize.py` driver, hits Anthropic API with Claude Haiku 4.5
- 15,974 packages processed in ~19 minutes
- Cost: ~$1.50 against user's Anthropic API
- Output: `src-tauri/data/categories.json` (~838 KB), 19 category slugs
- Cascade `.env` lookup (categorize tool's own `.env`, fallback to repo root)

### Landing page
- `landing/index.html` — hero with icon, tagline, CTAs (Download / Source on GitHub), feature list, "Open by default" posture (MIT, no telemetry, no accounts, etc.), security tool battery badges, install instructions
- `landing/style.css` — embedded design tokens matching the app (dark-first, warm amber, OKLCH)
- `landing/brew-browser.svg` — copy of `docs/icon/brew-browser.svg`
- Full SEO + social treatment: OpenGraph, Twitter/X cards, JSON-LD `SoftwareApplication`, PWA manifest, robots.txt, sitemap.xml
- 1200×630 social card (`social-card.png` + `.svg`) iterated through multiple designs based on user feedback

## Files

- `src-tauri/data/categories.json` (838 KB)
- `tools/categorize/{categorize.py, prompts/, requirements.txt, README.md}`
- `landing/{index.html, style.css, brew-browser.svg, manifest.json, robots.txt, sitemap.xml, social-card.png, social-card.svg, README.md}`

## Deployment

`landing/` rsynced to `michael@umbp:Sites/brew-browser/` (Tailnet IP `100.98.187.7`); Caddy serves from there.

## Notes / decisions

- Categorize is **build-time / maintainer-side only.** Zero LLM calls at runtime; the bundle is the canonical artifact.
- Per-package categories cap, validation in both Python writer and Rust loader (defense-in-depth).
- Landing page deploy is a manual `rsync` step; no CI hook yet.
- 19 category slugs land here; later enrichment work (Phase 13) adds per-package friendly names + summaries on top.
