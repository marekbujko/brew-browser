# 06 — Native dashboard load perf (the "Reading your Homebrew setup…" hang)

**Date:** 2026-06-04/05
**Branch:** `experiment/native-swift-liquid-glass`

## Objective

The dashboard sat on the "Reading your Homebrew setup…" spinner for several
seconds at launch. Make it paint fast.

## Three stacked causes (found by measuring, not guessing)

1. **`brew outdated --json=v2` = ~4.35s** gated the whole dashboard
   (`dashboardLoaded` was set only after the slowest parallel op).
2. **Synchronous bundled-data parse at `AppModel()` init** — `categories.json`
   (~800K) + `enrichment.json` (~2.7M, 15.7k entries) were stored-property
   initializers running on the main actor *before first paint*.
3. **The real killer: `BrewService` was an `actor`.** Every `async let
   brew.countX()` in `loadDashboard` was actor-isolated, so the ~8 "parallel"
   brew calls ran **serially** (~4s total). All the calls measured <1s each.

## Fixes

- **`BrewService` `actor` → `Sendable struct`**, with the live-process registry
  moved to its own tiny `ProcessRegistry` actor (cancellation stays safe). Now
  read-only brew calls run on the cooperative pool and genuinely parallelize.
- **`loadDashboard` ungated:** paints after the fast path (installed list + cheap
  counts + version/prefix); `loadOutdated()` and `loadStorage()` stream into the
  Updates tile + Storage card behind `outdatedLoading` / `storageLoading`
  placeholders. Dropped the redundant `brew list --cask` count (derived from the
  installed list).
- **`loadBundledData()`** parses categories + enrichment OFF the main thread
  (`Task.detached`) and backfills the category breakdown + open detail; the
  former synchronous stored-property inits are gone (now `var ...?` filled later).
  Triggered from `ContentView.task`. Types are `Sendable`.

## Outcome

First paint dropped from ~several seconds to ~1s. `swift build` clean.

## Note

The `BrewService.swift:240`-style "no async operations within await" warning is
gone (the registry call is now a real cross-actor hop). Same struct pattern was
later applied to `GitHubService` (task 07).
