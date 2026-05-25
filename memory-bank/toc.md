# Memory Bank — brew-browser

Project-scoped memory bank. **All agents working on this project read from and write to this directory.** Source of truth for design decisions, architectural choices, current state, and inter-agent insights.

## Quick-read order (every session)

1. `NEXT-SESSION.md` — current state + outstanding work, refreshed every milestone
2. `activeContext.md` — what's happening *right now* (the current wave / phase)
3. `progress.md` — the chronological log; latest entry covers the most recent release
4. Latest `tasks/YYYY-MM/*.md` records — per-shipped-unit detail

## File map

### Top-level docs

| File | Owner | Read by | Write when |
|------|-------|---------|------------|
| `toc.md` | Lead | all | structure changes |
| `NEXT-SESSION.md` | Lead | all (every session) | every milestone or release |
| `projectbrief.md` | Lead | all | mission shifts |
| `techContext.md` | Lead | all | new tech adopted |
| `decisions.md` | all | all | architectural decision made |
| `activeContext.md` | Lead | all | every wave start/end |
| `progress.md` | Lead | all | phase completes |
| `systemPatterns.md` | Backend Architect + Frontend Developer | all | new pattern emerges |
| `designSystem.md` | UI Designer | Frontend Developer, Whimsy Injector | design decisions |
| `uxArchitecture.md` | UX Architect | Frontend Developer, UI Designer | flow/IA decisions |
| `visualStory.md` | UI Designer | all | screenshot/voice/visual plans |
| `backendApi.md` | Backend Architect | Frontend Developer, API Tester | API surface changes |
| `frontendComponents.md` | Frontend Developer | UX Architect, Code Reviewer | component built |
| `codeReview.md` | Code Reviewer | Lead, Backend Architect, Frontend Developer | review pass done |
| `realityCheck.md` | Reality Checker | Lead | production gate evaluated |
| `apiTests.md` | API Tester | Backend Architect | tests defined/run |
| `accessibility.md` | Accessibility Auditor | UI Designer, Frontend Developer | a11y audit pass |
| `security.md` | Security Engineer | all | release-time + on new outbound surface |
| `agentLog.md` | all | Lead | each agent run (append-only stamp) — *currently dormant; re-enable if desired* |
| `ideas.md` | Lead | all | candidate feature surfaces |

### Subdirectories

| Path | Owner | Contents |
|------|-------|----------|
| `tasks/YYYY-MM/*.md` | Lead | Per-shipped-unit task records. One file per phase or release. See `tasks/2026-05/README.md` for index. |
| `phases/phaseNN-plan.md` | Lead | **Shipped** phase plans (Phase 12, Phase 13, etc.) — design-time intent preserved as historical context. In-flight plans (e.g., `phase15-plan.md`) stay at top level until the phase ships. |
| `scans/YYYY-MM-DD/*` | Security Engineer | Point-in-time outputs from `cargo audit`, `cargo deny`, `semgrep`, `gitleaks`, etc. Date-stamped folders. Latest scans should match `security.md`'s most recent §N audit. |

## Agent collaboration protocol

1. **Read first.** Before writing, every agent reads at minimum: `NEXT-SESSION.md`, `projectbrief.md`, `activeContext.md`, `decisions.md`, the latest `tasks/YYYY-MM/*.md` records, and any files in their "Read by" column above.
2. **Write only your owned files.** Agents do not modify each other's spec files. To request a change, append a note to the file with `// REQUEST FROM <agent>:` and the owning agent integrates.
3. **Decisions go in `decisions.md`.** Any architectural choice (library pick, pattern adopted, tradeoff resolved) gets an ADR entry.
4. **No code generation without spec.** Implementation agents (Backend Architect, Frontend Developer) only write code that traces back to a spec file. If the spec is missing, write the spec first.
5. **Phase plans live at `memory-bank/phase{N}-plan.md` while in-flight, then move to `memory-bank/phases/phase{N}-plan.md` after the phase ships.** Keeps the top-level focused on what's actually active.
6. **Per-task records land in `tasks/YYYY-MM/{NN-slug}.md`** alongside the commit that ships them. See `tasks/2026-05/README.md` for the canonical shape; `tasks/2026-05/99-deferred-and-dropped.md` for the running ledger of work explicitly NOT shipped.
7. **Security scan artifacts go in `scans/YYYY-MM-DD/`** — each tool battery run gets its own dated folder so the historical trail is preserved.

## Conventions currently in use

- ✅ `NEXT-SESSION.md` rewritten at every milestone
- ✅ `tasks/YYYY-MM/` populated per shipped unit (retroactively backfilled 2026-05-25)
- ✅ `phases/` for shipped plans, top-level for in-flight
- ✅ `scans/YYYY-MM-DD/` for tool-battery snapshots
- ✅ `security.md` §N appended per release-time audit

## Conventions currently dormant

- ⏸ `agentLog.md` append-on-every-run stamping — sparse since Phase 12; Wave 1 parallel agents in Phase 15 didn't stamp. Either re-enable by including "append one line to agentLog.md" in every Agent prompt, or drop this protocol item.

## Conventions explicitly dropped

(none yet)
