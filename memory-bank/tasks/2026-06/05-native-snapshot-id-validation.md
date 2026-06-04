# 05 — Native: validate snapshot id before path join (defense-in-depth)

**Date:** 2026-06-04
**Branch:** `experiment/native-swift-liquid-glass`

## Objective

Mirror the security hardening from **Tauri PR #46** (@neodave —
`fix(brewfile): validate id before joining into filesystem path`) on the native
side, for security-posture parity.

## Threat model (why this is defense-in-depth, not a live fix)

PR #46 fixed a real *latent* path-traversal in Tauri: `BrewfileId` arrives over
the IPC boundary from the renderer and was joined straight into a path, so a
crafted `id` (`../../etc/...`, absolute paths) could escape `brewfiles_dir`.

Native's `SnapshotStore.path(forID:)` had the same code shape — but native has
**no untrusted caller**: every `id` originates from `sanitizeLabel` (create /
import) or a directory scan (`list()` → `lastPathComponent` of real files). A
compiled Swift app has no renderer that can call `delete(id:)` with arbitrary
input. So this is hardening for parity, not a reachable hole.

## Outcome

- ✅ `swift build` clean.

## Changes (`native/Sources/BrewBrowserKit/SnapshotStore.swift`, `AppModel.swift`)

- `SnapshotError.invalidID(String)` added.
- `SnapshotStore.validateID(_:)` — same allowlist as the Tauri fix:
  `[A-Za-z0-9_-]`, 1–64 chars (also exactly what `sanitizeLabel` emits).
- `path(forID:)` is now **throwing** — validation runs at the single point an id
  becomes a path, so the compiler routes `delete` / `export` / `importFile` /
  `dumpTarget` through it (mirrors PR #46 making `brewfile_path` fallible).
- `dumpTarget(forLabel:)` is throwing; `AppModel.dumpSnapshot` guards it
  (`try?` — the sanitized id can't realistically fail).

No native test target exists (SPM lib+exe only), so no unit test added; the
Tauri PR carries the traversal/accept tests.

## Cross-reference

- Tauri fix: PR #46, merge `04bf6c9` (commit `9e7105b`), on `main`.
