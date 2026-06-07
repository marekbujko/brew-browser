# Contributing to brew-browser

Thanks for considering a contribution. This project is small, opinionated, and deliberately open. The bar for landing changes is "does it match the patterns already here and not break anything," not "have you signed paperwork."

Brew Browser ships in **two builds** that share one design + data contract: the cross-platform **Tauri** app (`src/` + `src-tauri/`) and the native **Swift/SwiftUI** app (`native/`). See [README → Two builds](./README.md#two-builds) and the `memory-bank/decisions.md` parity ADR (2026-06-01). **A change to a shared data contract — `settings.json` schema, bundled `categories.json`/`enrichment.json`, the `brew`/`brew vulns` invocations, or the trending/enrichment endpoints — should land in (or be logged as a gap against) both builds.**

## TL;DR

1. Fork the repo, create a topic branch off `main`.
2. Make your change. Keep it small and focused.
3. Run the checks for the build(s) you touched: Tauri → `cargo test` + `npm run check`; native → `swift build` + `swift test` (in `native/`).
4. Open a PR with a short description of what changed and why.

**No CLA. No rights assignment.** Your contributions remain yours, licensed under [MIT](./LICENSE) to match the project. By opening a PR you confirm you wrote the change or have the right to contribute it under that license.

## Dev setup

Prereqs:

- [Rust](https://rustup.rs/) (stable, edition 2021+)
- [Node.js 22+](https://nodejs.org/) and npm
- [Homebrew](https://brew.sh/) itself (the app shells out to `brew`)
- Xcode Command Line Tools: `xcode-select --install`

Loop:

```sh
git clone https://github.com/<your-fork>/brew-browser
cd brew-browser
npm install
npm run tauri dev      # full app with HMR
npm run check          # svelte-check + tsc
cargo test --manifest-path src-tauri/Cargo.toml
```

`npm run tauri build` produces a `.dmg` under `src-tauri/target/release/bundle/` if you want to test a real artifact.

### Native build (macOS 26)

The native app needs **macOS 26 + a recent Xcode toolchain** (`xcode-select -p` should point at `Xcode.app`, not Command Line Tools). It's a pure Swift Package — no `.xcodeproj`.

```sh
cd native
swift build                 # compile
swift test                  # unit tests (Swift Testing)
./build-app.sh              # wrap into Brew Browser.app
open BrewBrowser.app        # run (use the .app, not `swift run` — Sparkle needs the bundle)
```

## Project structure

A quick map. The canonical, always-up-to-date version lives in [`memory-bank/toc.md`](./memory-bank/toc.md).

```
brew-browser/
├── src/                            Svelte 5 + TS frontend
│   ├── lib/
│   │   ├── components/             18 Svelte components (Library, Sidebar, Modal, ActionDrawer, …)
│   │   ├── stores/                 7 .svelte.ts stores (packages, search, activity, env, …)
│   │   ├── styles/                 OKLCH tokens, reset, typography
│   │   ├── api.ts                  typed invoke() wrappers for all backend commands
│   │   └── types.ts                TS mirrors of the Rust DTOs
│   └── routes/                     SvelteKit SPA entry
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs                  Tauri Builder + invoke_handler wiring
│   │   ├── error.rs                BrewError + From impls
│   │   ├── state.rs                AppState, job table, write mutex
│   │   ├── types.rs                shared DTOs (Package, PackageList, …)
│   │   ├── brew/                   exec / parse / paths helpers
│   │   ├── commands/               one file per command group
│   │   └── trending/               reqwest client + 1h cache
│   ├── tests/                      integration tests (ignored by default)
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── capabilities/default.json
├── native/                         native Swift 6 + SwiftUI build (Swift Package)
│   ├── Sources/
│   │   ├── BrewBrowser/            thin @main App entry
│   │   └── BrewBrowserKit/         views, AppModel, services (Brew/GitHub/Vulns/…),
│   │       │                       BrewOutputParsing.swift, bundled JSON Resources/
│   │       └── …
│   ├── Tests/BrewBrowserKitTests/  swift test (Swift Testing) — parity fixtures
│   ├── Package.swift
│   ├── build-app.sh                wrap the SPM binary into Brew Browser.app
│   └── release.sh                  signed + notarized release + Sparkle appcast
├── memory-bank/                    living design docs (read these before any non-trivial change)
├── docs/                           BUILD instructions, PLAN.md, PHILOSOPHY.md, release-notes/, icon/, screenshots/
├── LICENSE                         MIT
└── README.md
```

## How to add a Tauri command

The pattern is documented in [`memory-bank/systemPatterns.md`](./memory-bank/systemPatterns.md) (§1–10) and the full command surface lives in [`memory-bank/backendApi.md`](./memory-bank/backendApi.md). Short version:

1. Add the typed DTO to `src-tauri/src/types.rs` with `#[derive(Serialize, Deserialize)]` and `#[serde(rename_all = "camelCase")]`.
2. Add the command in the appropriate file under `src-tauri/src/commands/` as `async fn` returning `Result<T, BrewError>`.
3. Register it in the `tauri::generate_handler!` list in `src-tauri/src/lib.rs`.
4. Mirror the TS type in `src/lib/types.ts` and add a typed wrapper in `src/lib/api.ts`.
5. Add a unit test next to the parser if you introduced one; add an integration test under `src-tauri/tests/` (gated behind `#[ignore]`) if it shells out.

If your command mutates brew state, acquire `state.brew_write_lock` for the duration of the child process. Reads bypass the lock.

## How to add a Svelte component

The component conventions are in [`memory-bank/designSystem.md`](./memory-bank/designSystem.md) and the runes / store patterns are in [`memory-bank/systemPatterns.md`](./memory-bank/systemPatterns.md).

1. Use Svelte 5 runes (`$state`, `$derived`, `$effect`) — no legacy `let` reactivity.
2. Pull data from the relevant store in `src/lib/stores/` rather than re-invoking the backend.
3. Use the OKLCH design tokens in `src/lib/styles/tokens.css` — no hardcoded colors.
4. Keep components small. If a component grows past ~200 lines it almost certainly wants to be split.
5. Theme via the `[data-theme="light"|"dark"]` attribute on `<html>` — light and dark must both render.

## Tests

- **Rust:** `cargo test --manifest-path src-tauri/Cargo.toml`. Unit tests live inline under `#[cfg(test)] mod tests`; fixture-driven parser tests use the JSON under `src-tauri/tests/fixtures/`.
- **Integration (real brew):** `cargo test --manifest-path src-tauri/Cargo.toml -- --ignored` — these spawn real `brew` and require Homebrew on the host.
- **Frontend:** `npm run check` (svelte-check + tsc). There is no Vitest suite yet; adding one is welcome.
- **Native:** `cd native && swift test` (Swift Testing). The native suite mirrors the Rust fixtures for the shared parsing/classification logic (brew output, vuln keying, settings contract) — when you change shared behavior, update both sides' fixtures so they stay pinned together.

A PR that introduces new logic without tests will get a request for tests, not a rejection. A PR that breaks an existing test will get a request to fix it.

## Code style

Minimal. Match what's already there:

- Rust: `cargo fmt` defaults. No custom rustfmt.toml.
- TypeScript / Svelte: project defaults. No Prettier config fight, no autoformat-on-save mandate.
- Prefer the patterns in [`memory-bank/systemPatterns.md`](./memory-bank/systemPatterns.md) over inventing new ones. If you think a pattern needs to change, open an issue first.

## Submitting changes

Open a PR with:

- **What changed** — one or two sentences.
- **Why** — the motivation. "It bugged me" is a fine reason for a small fix.
- **Screenshots** if the change touches the UI (before/after when reasonable).
- **Test notes** — what you ran locally, what you didn't.

Smaller PRs land faster. A 30-line bug fix will get merged before a 3,000-line refactor.

## What kind of PRs land easily

- Bug fixes with a clear reproduction.
- Small accessibility improvements (focus order, ARIA labels, keyboard handling).
- Documentation fixes — typos, broken links, clearer wording.
- Test coverage for existing code paths.
- Performance tweaks with a measurement.
- Small UX polish that matches the existing tone (quiet, dense, Mac-native).

## What needs discussion first

Open an issue before sending a PR for:

- **New features** — especially anything that adds a sidebar section or a new top-level surface.
- **New dependencies** — both Rust crates and npm packages. The bar is "earns its weight."
- **Architectural changes** — anything that touches the IPC contract, the write-mutex model, or the streaming pattern.
- **Network calls** — the only outbound traffic today is to `formulae.brew.sh`. Adding a new host is a discussion.
- **Telemetry, analytics, accounts, or anything user-identifying** — these are non-goals. A PR adding any of them will be closed.

## Code of conduct

Be kind. Assume good faith. Disagree about the work, not the person. Don't be a jerk in issues, PRs, commit messages, or anywhere else this project shows up.

That's the whole policy. If something serious comes up that isn't covered by "don't be a jerk," email the maintainer listed in the repo and we'll deal with it directly.
