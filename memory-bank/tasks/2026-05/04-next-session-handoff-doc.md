# 2026-05-24 — NEXT-SESSION handoff doc

**Phase:** Memory-bank infrastructure
**Status:** ✅ Shipped
**Commit:** `c2ab41f` (1 file, +166 lines)
**Date:** 2026-05-24 00:54

## Scope

Establish a single canonical document that future sessions (or future-me after `/compact`) read first. Captures current state at the moment of writing, what's queued next, and the credentials/paths reference table so handoffs don't require trawling the full memory-bank.

## What landed

- New file: `memory-bank/NEXT-SESSION.md`
- Sections established (template for all subsequent rewrites):
  - **Current state at compact** — what shipped, what's in the working tree, test/build/lint status
  - **What's queued for the post-compact session** — ordered priority list with rationale
  - **Critical context for any release** — placeholder warnings, env-file locations, signing config status
  - **Credentials / paths reference** — table mapping concept → filesystem path
  - **Open items not in the post-compact plan** — deferred/dropped record
  - **Repeated prompt-injection observation** (later additions) — adversarial-pattern notes for the next session

## Notes / decisions

- File is **rewritten** at compact time, not appended-to. The point is "what should the next session know NOW," not the archaeology.
- The "Credentials / paths reference" table is the highest-value section — gets used every session.
- Convention solidified: every release update rewrites this file as part of the release commit.
- File survives across compacts and across all subsequent releases; rewritten ~6 times across this session's first 30 hours.
