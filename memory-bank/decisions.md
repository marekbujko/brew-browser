# Architectural Decisions

## 2026-05-23: MIT License

**Context:** Need an OSI-approved license for an open-source macOS GUI utility. Considered GPL (copyleft, prevents closed forks, dual-license option), AGPL (network-services clause, irrelevant for a desktop app), and source-available licenses (FSL/BSL — not OSI-approved).

**Decision:** MIT.

**Rationale:**
- Most permissive and most recognizable OSI license — clearest "use this however you want" signal
- Lowest friction for a contributor-friendly small utility: no CLA needed, no copyleft compliance overhead for downstream users
- Contributor retains copyright on own contributions, so monetization options (paid binaries, App Store, support, dual-license) remain open
- Matches the dependency stack (Tauri MIT/Apache, Svelte MIT, reqwest MIT/Apache) so there are no license-compatibility seams

**Trade-off accepted:** Anyone can fork and ship a closed derivative. For a small utility this is fine; the value is in the live project, not the license clause.

---

## 2026-05-23: Tauri 2 over Electron / Flutter / GPUI

**Context:** Need cross-platform desktop framework. Electron is the historical default but heavy. Flutter renders everything custom. GPUI (Zed's) is pre-1.0 and Zed-coupled. Tauri 2 ships a native webview, ~8 MB bundles, supports mobile.

**Decision:** Tauri 2 + SvelteKit + Svelte 5 + TypeScript.

**Rationale:**
- Smallest binary footprint, fastest cold start
- Web-developer ergonomics for the UI (Svelte 5 = minimal ceremony, fast HMR)
- Rust backend is appropriate for shelling out to `brew` safely
- Tauri 2's iOS/Android support keeps a mobile path open without re-platforming

---

## 2026-05-23: Shell out to `brew`, don't reimplement

**Context:** Could reimplement Homebrew operations directly (parse formula files, manage downloads, etc.) or shell out to the `brew` CLI.

**Decision:** Shell out exclusively. Use `--json=v2` output formats wherever available.

**Rationale:**
- `brew` is the source of truth; reimplementing duplicates state and creates drift
- `--json=v2` outputs are stable contracts
- A respectful UI on top of `brew` is the right scope for this project

---

## 2026-05-23: Trending data from `formulae.brew.sh`

**Context:** Need data source for "trending packages" tab. Options: scraping web pages, building our own analytics, using Homebrew's published analytics.

**Decision:** Use `https://formulae.brew.sh/api/analytics/install/<window>.json` — Homebrew's own published analytics, no auth required, no scraping.

**Rationale:**
- Authoritative source; no reverse-engineering or scraping
- No keys, no rate-limit-as-product
- Cache in memory ~1 hour to be a polite client
- Keeps brew-browser a respectful frontend on top of Homebrew-owned data

---

## 2026-05-23: Serialize brew invocations with a Mutex

**Context:** `brew` does not tolerate concurrent operations against its own state (lockfile collisions, partial installs). UI could trigger overlapping commands.

**Decision:** Wrap all `brew` invocations in a single `tokio::sync::Mutex<()>` held in Tauri managed state.

**Rationale:**
- Prevents data corruption with zero user-visible cost (queue and show queue state)
- Implementation is ~10 LOC
- Future: per-command-class mutex if read-only ops (`list`, `info`, `search`) should run in parallel with writes

---

## 2026-05-24-night: Dashboard is the default landing, brand area is the home button

**Context:** First-launch UX dropped users into a 325-row Library list — overwhelming. Need a friendlier "state of your setup" first impression. Open question: separate sidebar item or repurpose the brand area as the home affordance?

**Decision:** Dashboard becomes the default `ui.section` (over Library). The sidebar brand (`🍺 brew-browser`) is the home button — clicking it returns to Dashboard. No separate "Dashboard" nav item in the sidebar list; brand = logo = home, in line with how web apps work.

**Rationale:**
- First impression frames the relationship: "this is your setup" rather than "here's a list"
- Brand-as-home mirrors universal web-app convention; users already try clicking the logo
- Keeps the sidebar nav list tight (5 items: Library/Discover/Trending/Snapshots/Services/Activity)
- Cmd+0 reserves a stable keyboard shortcut for home, parallel to Cmd+1..6 for sections

**Trade-off accepted:** Discoverability of Cmd+0 is weaker than a visible nav item, but the brand's active state (background highlight when on Dashboard) provides visual feedback. Power users learn shortcuts; everyone else clicks the brand.

---

## 2026-05-24-night: Vibrancy via `window-vibrancy` (Tier A), Tahoe Liquid Glass deferred

**Context:** Requested "native feel" / Liquid Glass treatment. Tier A = NSVisualEffectView via `tauri-plugin-window-vibrancy` (works since macOS 13, ~30 min to wire). Tier B = true Tahoe Liquid Glass via Swift bridge (Tahoe-only, half day).

**Decision:** Ship Tier A this session. Defer Tier B to v0.2.

**Rationale:**
- Tier A delivers 80% of the visual win for 20% of the work
- Works across all supported macOS versions (13+) — Tier B would gate the feel on macOS 26 only
- Tier B's Swift bridge is reversible — we can layer it on top of Tier A later without breaking anything
- Surfaces a real `core:window:allow-start-dragging` capability requirement which had to be added regardless

**Implementation notes:**
- `tauri.conf.json`: `transparent: true`, `titleBarStyle: "Overlay"`, `hiddenTitle: true`
- Apply `NSVisualEffectMaterial::HudWindow` in `lib.rs` setup hook
- Body background must be transparent in CSS for the vibrancy to show through
- Drag regions use `data-tauri-drag-region` attribute on real DOM elements (NOT a fixed overlay, which intercepts scroll wheel events) — added via the new capability

---

## 2026-05-24-night: Categories donut chart (top 8 + Other), not bar list

**Context:** First implementation was a horizontal bar list per category. Two bugs surfaced: (a) bar fill color matched track in dark mode so bars looked empty, (b) when one category dwarfs the rest (Developer Tools at 256 vs next at 29), small bars are visually meaningless.

**Decision:** Donut chart with top 8 individually segmented + "Other" slice. Clickable legend on the right.

**Rationale:**
- Donut conveys *proportion at a glance* better than tiny bars
- Color encodes category (palette-rotated) — no fill/track collision possible
- Top 8 + Other is a standard pattern for long-tail distributions
- Legend rows double as nav: click → Discover with category chip selected
- 180px donut + SVG arcs = ~30 lines of CSS, no chart lib needed

**Math:** Each segment is a `<circle>` with the donut's full circumference as `stroke-dasharray` second value, with first value = `(pct/100) * C`. `stroke-dashoffset = -(startPct/100) * C` shifts each segment to start at the right angle. `transform="rotate(-90)"` puts segment 0 at 12 o'clock.

---

## 2026-05-24-night: `du -sk` over native walk for disk usage

**Context:** Need to size 4 Homebrew sub-trees (Cellar, Caskroom, var/log, cache). Could walk filesystem in Rust or shell out to `du`.

**Decision:** Shell out to `du -sk <path>` in parallel via `tokio::join!`.

**Rationale:**
- BSD `du` is highly optimised (cached inode stats, sparse-file aware)
- Single syscall per path vs O(n) recursion in Rust + serde overhead
- Parallel via `tokio::join!` keeps wall time = max(4 paths) not sum
- 60s cache on AppState means subsequent reads cost nothing
- Output parsing is trivial: `<kb><tab><path>`

**Trade-off accepted:** Shells out to an external binary. But we already shell out to `brew` everywhere; `du` is a base-system tool that's been on every Unix for 40+ years and can't reasonably be missing.

---

## 2026-05-24-night: Bundled catalog + user-initiated refresh (Phase 12a — planned)

**Context:** formulae.brew.sh exposes the entire Homebrew catalog as JSON (~10 MB raw). Caching it locally unlocks fast search, deprecation warnings, build-error stats, reverse deps, and more. But network calls without consent break the project posture; auto-refresh without explicit user action would add a quiet 5th outbound network path.

**Decision:**
- **Bundle a baseline catalog at build time** via `include_bytes!` + gzip (~3 MB compressed)
- **User-initiated refresh** only: a button writes a fresh fetch to `~/Library/Application Support/brew-browser/catalog/`
- **Resolution order at runtime:** user-data catalog (if present) → bundled fallback
- **Soft nudge** (banner) when active catalog is older than 14 days; dismissable
- **No auto-refresh** by default; deferred opt-in setting later

**Rationale:**
- Matches the project posture: every network path is disclosed in README and user-consented
- Baseline-in-binary keeps the app fully functional offline / first-launch
- Manual refresh respects user agency; freshness is visible (timestamp shown)
- Unblocks deprecation warnings, build-error rates, reverse deps, dependency tree, Brewfile validation, "what's new this week" feed — all become catalog reads instead of fresh network fetches

**Network disclosure:** Adds a 5th outbound path, explicitly labeled "user-initiated only".

---

## 2026-05-24-night: GitHub integration via Device Flow (Phase 12c–f — planned)

**Context:** Many packages have GitHub homepages — stars/forks/last-release would enrich PackageDetail. User-authenticated actions (star, file issue, watch) are bigger wins but require OAuth. Native apps can't safely embed client secrets, so PKCE/Implicit are out.

**Decision:** Two-tier GitHub integration.

**Tier 1 — anonymous (no sign-in):**
- Detect GitHub homepage URLs (`/^https?:\/\/github\.com\/(\w+)\/(\w+)$/`)
- Hit public api.github.com (60 reqs/hr anon limit)
- 24h disk cache per repo
- Show: stars, forks, last release date, archived flag, license match-check
- Graceful degradation when rate-limited

**Tier 2 — signed in (OAuth Device Flow):**
- Setting: "Sign in with GitHub"
- Device Flow (RFC 8628): no client secret needed for native apps, designed for them
- Token stored in macOS Keychain via `tauri-plugin-keyring` (or similar)
- Rate limit goes to 5000/hr
- Unlocks: star/unstar, file issue (with pre-filled context), watch, "Wrong?" reporting

**Rationale:**
- Anonymous tier ships value immediately, no consent friction beyond toggling on
- Device Flow is the correct OAuth profile for native apps — no localhost redirect, no embedded WebView, no client secret in shipped binary
- Keychain storage matches platform expectations; no plaintext tokens on disk
- Two-tier respects users who want zero-account posture (default = off entirely) and power users who want full integration

**Trade-off accepted:** Anonymous tier consumes the 60 reqs/hr per IP — heavy browsing may rate-limit. Pitch sign-in when this happens.

---

## 2026-05-24-night: Phase 14 (bundled cask icons) DROPPED

**Context:** User proposed pre-fetching cask icons at build time and bundling them in the binary to eliminate the runtime cask-homepage-probe network path. Tempting because it would reduce the documented outbound paths from 5 to 4 (or 7 to 6 after Phase 12c+e) and make Discover/Trending render instantly without probe latency.

**Decision:** NOT going to do this. Leave the runtime probe + paranoid-mode gate as the cask-icon story.

**Rationale (the redistribution problem):**
- The current probe fetches and DISPLAYS vendor icons (Slack, 1Password, Adobe, ~7,600 casks) on the user's machine — closest analog is "rendering a page that shows the vendor's logo" → generally OK as nominative fair use
- Bundling those icons into our binary is REDISTRIBUTION at scale — many vendors have explicit "no third-party redistribution of brand assets" terms; even where not explicit, the surface area for trademark complaints is real
- "Brew did it" doesn't apply — Homebrew doesn't bundle icons either, they point at homepages
- The current probe is already very respectful: 7d cache, sandboxed against SSRF (rejects RFC1918 / link-local / loopback / cloud-metadata, redirect re-check), per-cask-per-week max, concurrency-limited semaphore, paranoid mode kills it in one click

**Considered middle ground (rejected):** bundle only FOSS-licensed casks' icons via license filter from Phase 12a catalog + top-N popularity from analytics. Would gain ~90% of the network reduction without ~95% of the redistribution risk. **Rejected** as additional complexity for a non-essential win — the runtime probe is already good enough for paranoid-mode-OFF users, and paranoid-mode-ON users already get zero icon probes.

---

## 2026-05-24-night: Phase 12 Wave ordering (Foundation-first, Option A)

**Context:** Phase 12 had 6 sub-phases (a-f) with various dependency edges. Two wave orderings considered: (A) 12d first → combined 12c+12e → 12f, (B) 12c + 12d + 12e in parallel with merge resolution → 12f.

**Decision:** Option A — Foundation-first.

**Rationale:**
- 12d delivers `require_network(feature)` helper + settings persistence — 12c and 12e then consume the helper directly instead of TODO-commenting it
- 12c and 12e both touch `src-tauri/src/github/` module + commands/mod.rs + lib.rs handler list — combining them in one Backend Architect pass avoids two agents fighting over the same files
- Parallel option B saved one wave but required hand-merging mod.rs/lib.rs/tauri.conf.json conflicts — net wall time similar
- A is cleaner; each step starts from a known-good staged state

**Outcome:** delivered cleanly. 274 → 334 tests across Wave 2; zero merge conflicts.

---

## 2026-05-24-night: Combined GitHub backend (12c + 12e in one Backend Architect)

**Context:** Original plan split GitHub work into 12c (anonymous repo stats) and 12e (Device Flow + Keychain). Both touch `src-tauri/src/github/` module. Separate agents would conflict.

**Decision:** One Backend Architect pass implements the entire GitHub module end-to-end: parse_github_url validator (12c) + RepoStats fetch + cache (12c) + Token newtype + Device Flow + Keychain (12e) + 5 IPC commands.

**Rationale:** github module is a single coherent unit. Splitting it forces shared-file coordination. One agent owns it cleanly.

**Constraint observed:** the combined agent ships placeholder `GITHUB_OAUTH_CLIENT_ID` const. Real client_id must be added before any release (documented in BUILD.md, 7-step OAuth App creation guide). 12c functionality works without a client_id — anonymous tier is independent. Only 12e sign-in needs the real client_id.

---

## 2026-05-26: Opt-in trust boundary for enhanced trending history (v0.4.0)

**Context:** v0.4.0 introduces velocity scoring on the Trending tab. The velocity index itself is computable from the always-on `formulae.brew.sh/api/analytics/install` + `install-on-request` endpoints — three windows joined server-side gives us "monthly rate vs annual average" with zero net-new trust boundary. But the user-requested follow-up (Star-History-for-Homebrew style inline sparklines + per-package charts) needs *time-series* data that Homebrew doesn't publish. We have to capture it ourselves on `brew-browser.zerologic.com`.

That means a net-new outbound path to infrastructure **we** operate (path j in the projectbrief enumeration; tenth in the README disclosure). The previous nine paths target Homebrew, GitHub, or generic third parties — trust posture there is "we trust the upstream." Path j is "trust us." That's a meaningful posture shift that needs explicit consent.

**Decision:** ship velocity (free, no new boundary) ALWAYS-ON in v0.4.0. Ship history sparklines + detail-panel charts as an **opt-in per-feature toggle** behind `Settings.enhanced_trending_enabled`. Default `false`. Master Offline Mode hard-locks it regardless.

**Architecture:**

- New `BrewError::FeatureDisabled { feature }` variant, distinct from `ParanoidModeBlocked` so the frontend toast routes to the per-feature toggle (not the master switch).
- New `AppState::require_enhanced_trending()` helper composes the master `require_network` gate with the per-feature toggle. Five rejection paths pinned by tests: toggle off, paranoid on, paranoid-wins-over-toggle, FirstLaunch (opt-in posture preserved), Corrupt (paranoid gate fires first).
- The Caddy block serving the endpoint sets `request>remote_ip "0.0.0.0"` at the log layer so the privacy claim ("no IP retention") is auditable — anyone can ssh in and `cat Caddyfile`. Documented in `security.md` §16.
- Frontend `trendingHistory` store consults `settings.effective.enhancedTrendingEnabled && !paranoidMode` before calling the IPC. Soft-fails silently if the gate denies — feature is enrichment, not load-bearing.

**Alternatives rejected:**

- **Always-on for B too.** Simpler, but breaks the "no telemetry, no surprise calls" posture stated in the projectbrief. The endpoint is our infra, not upstream's. Even though it logs no IP, the existence of a non-Homebrew outbound call deserves explicit consent.
- **Subdomain (`trending-history.zerologic.com`).** Cleaner conceptual split. But adds Caddy cert mgmt + DNS coordination + a second name to disclose. The subpath shares the existing `brew-browser.zerologic.com` cert and is documented as the same trust boundary (project-operated infrastructure).
- **Defer the trust ask until v0.5+.** Would mean shipping velocity without sparklines — half the visual story. The seed-from-rolling-windows trick (derive 3 historical buckets from c30, c90-c30, c365-c90) makes day-0 sparklines viable, so the feature is worth shipping together.
- **No per-feature gate; just rely on Offline Mode.** Master switch is too blunt — turning it off blocks the entire app. A per-feature toggle lets users keep brew search + GitHub auth working while declining the new endpoint specifically.

**Outcome:** documented in `security.md` §16 (server-side audit), `projectbrief.md` "ten paths" (architectural enumeration), `README.md` "Open-source posture" (user-facing), and a new disclosure-list entry in `SettingsSectionNetwork.svelte` (in-app). Endpoint deployed via `tools/trending-collector/` running nightly cron on `brew-browser.zerologic.com`. Per-feature toggle lives in a new `SettingsSectionTrendingHistory.svelte` mounted at the bottom of Network alongside the existing Updates subsection.

---

## 2026-05-27: Opt-in vulnerability scanning via `brew vulns` (v0.5.0)

**Context:** users want to know which of their installed formulae have known CVEs. Two integration shapes were on the table:

1. **Native Rust client** — implement OSV.dev queries directly. We'd extract source URLs from `brew info`, build the OSV GIT-ecosystem query payload, post to `api.osv.dev/v1/query`, parse the response, render. Full control; full ownership of correctness; full ownership of regression risk every time OSV's schema or brew's formula format shifts.
2. **Shell out to `brew vulns`** — Homebrew's own subcommand (`Homebrew/homebrew-brew-vulns`, by Andrew Nesbitt, published January 2026) already does the source-URL extraction, version-tag matching, and GIT-ecosystem query. It's published by Homebrew, designed for this exact use case, and inherits upstream fixes automatically.

The v0.4.0 outbound enumeration is at ten paths (path j is the most recent — opt-in `brew-browser.zerologic.com/trending-history`). v0.5.0 adds an eleventh: the OSV traffic that `brew vulns` performs, plus an optional GHSA enrichment to `api.github.com/advisories/{GHSA_ID}` from our own Rust code when both feature toggles align.

**Decision:** shell out to `brew vulns` for the OSV query; add **best-effort** GHSA enrichment from our Rust code; ship the whole feature as **opt-in** behind `Settings.vulnerability_scanning_enabled` (default `false`). Persist a SHA-256 fingerprint of the install set so opening the app daily doesn't re-shell `brew vulns` (60+ seconds with 200 packages) when nothing has changed.

**Architecture:**

- **Subprocess boundary:** the brew-browser binary itself never opens a socket to `api.osv.dev` or the source forges. `brew vulns` does, as the subprocess we invoke. This keeps the trust boundary honest in three places (user-facing copy, security.md §17 audit, and the in-app disclosure list): "we shell out to the official Homebrew subcommand; here is what it talks to."
- **Powered by brew vulns:** the Settings card credits `Homebrew/homebrew-brew-vulns` and links to the upstream repo. This is both correct attribution and an escape valve — if brew-vulns stagnates we swap in a native Rust client behind the same internal interface (`vulns::client::{check_brew_vulns_installed, scan_all, scan_one}`) without churning the IPC surface or the cache shape.
- **One-click installer:** a new `vulns_install_helper` IPC runs `brew install homebrew/brew-vulns/brew-vulns`. Gated only by `require_network` (NOT `require_vulnerability_scanning`) because the typical first-run flow is "user wants to enable scanning → tap install affordance → flip the toggle on → scan." Refusing to install the prerequisite until the feature is already on would be backwards.
- **GHSA enrichment is best-effort:** OSV returns a vulnerability ID, a brief summary, and an affected-versions range. GHSA returns the same plus richer prose, patched-version ranges, and reference links. We fetch GHSA when the OSV record carries a `GHSA-…` ID **and** `settings.github_enabled` is on. A 403 / 429 / network error leaves the OSV record unchanged and logs (no toast). The scan never fails because the enrichment cherry-on-top failed. This is triple-defense: paranoid gate, then per-feature vuln-scanning gate, then per-feature GitHub gate inside `vulns::enrich::enrich()`.
- **Install-set fingerprint:** SHA-256 (via the new `sha2` + `hex` deps) over sorted `kind:name:version` lines, persisted into the cache file alongside the per-package entries. Daily app opens with no install changes serve the cached report instantly with `source: "cache"`. The `force=true` parameter on `vulns_scan_all` bypasses the skip predicate for the Refresh button. **Why not `DefaultHasher`?** Rust's `DefaultHasher` is intentionally non-deterministic across process runs — its salt is randomized to defeat HashDoS — so a hash recorded in v0.5.0 disk cache would mismatch every subsequent launch, silently invalidating the skip predicate. SHA-256 is deterministic across runs, across machines, and across Rust versions.
- **Persistent on-disk cache:** `~/Library/Application Support/brew-browser/vulns_cache.json` for the scan records (1 MiB cap, atomic write, 6h TTL per record, fail-soft on corrupt + future-schema). Parallel `ghsa_cache.json` for enrichment (2 MiB cap, same atomic-write + read-capped pattern). Both load lazily on the first scan to avoid paying the file-read cost when the user never opts in.
- **Refresh integration:** the post-`brew update` refresh fan-out (Dashboard Refresh, Library Refresh) fires `vulnerabilities.scanAll(force=false)` after the catalog reload so freshly learned upstream versions get scanned. Post-mutation hooks (install / upgrade / uninstall) call `vulns_invalidate(kind, name, version)` to drop the affected cache entry and trigger a per-package re-scan.
- **Casks are out of scope:** `brew vulns` is formula-only (casks ship pre-compiled vendor binaries with their own update channels; OSV's source-URL approach doesn't map). Cask packages render the same UI rows but the Security card label honestly says "Cask coverage isn't supported — `brew vulns` is formula-only." This is the same posture we take with cask icons (Discover renders the same rows; the icon source label tells you when there's nothing to show).

**Alternatives rejected:**

- **Native Rust OSV client.** Tempting for tight integration, but we'd own every change to OSV's schema, every change to brew's source-URL format, every release that adds a new source-forge host. brew-vulns owns those already and is published by the project we wrap. Shelling out is the same architectural posture as every other brew interaction in this app.
- **Always-on (no per-feature toggle).** Would break the "no surprise calls" posture — `brew vulns` reaches `api.osv.dev` and the source forges (which is fine, but it's a non-Homebrew network surface the user didn't ask for). Even though Offline Mode would still kill it via `require_network`, the per-feature toggle gives users a way to keep brew + catalog + GitHub on while declining OSV traffic specifically.
- **Toast on GHSA enrichment failure.** Rejected. The enrichment is a UX nicety, not a correctness requirement. Surfacing transient GitHub rate-limit toasts would train users to dismiss notifications about the more serious gates (paranoid mode, settings corruption) — better to log and move on.
- **In-memory-only cache (no disk persistence).** Re-scanning 200 packages takes 60+ seconds. Without disk persistence, that cost is paid on every app launch, every time. The 1 MiB cap + atomic-write + fail-soft load make on-disk caching strictly better here.
- **Single cache file for both scan records and GHSA enrichment.** Schema-coupling them means one file's corrupt-recovery path resets the other. Keeping them separate (`vulns_cache.json`, `ghsa_cache.json`) lets each fail soft independently and lets us tune the caps separately (1 MiB vs 2 MiB).

**Outcome:** documented in `security.md` §17 (endpoint audit), `projectbrief.md` "eleven paths" (architectural enumeration), `README.md` "Open-source posture" (user-facing), `techContext.md` (subprocess + dep additions), `backendApi.md` §13.15 (IPC surface), `frontendComponents.md` v0.5.0 additions (store + components), and `docs/release-notes/0.5.0.md`. Feature ships behind a new `SettingsSectionVulnerabilities.svelte` mounted in `SettingsSectionNetwork.svelte` alongside the Updates and Enhanced Trending History subsections. UI surface lands in the Dashboard "Exposure" card, the Sidebar count badge, the PackageRow severity dot, and the PackageDetail "Security" card.

## 2026-05-27: Smoke-test discipline for subprocess-integration features

**Status:** durable rule, added after the v0.5.0 smoke-test cycle surfaced five integration bugs no unit test could have caught.

**Context:** v0.5.0 (above) shelled out to `brew vulns` — a third-party subprocess we don't control. Step 2 declared a "defensive" wire shape with `#[serde(default)]` everywhere and shipped 40+ unit tests against synthetic fixtures. The build passed every gate (cargo test, npm check, vite build). The first time the user actually clicked "Scan now" on their real install, **five separate bugs** surfaced in rapid succession — each invisible to the test suite by construction:

1. CLI flag combination requirement (`--include-aliases` needs `--quiet` in brew 5.x)
2. Wrong install-detection probe (`brew commands` doesn't list external `brew-FOO` formula shims)
3. Wire severity is UPPERCASE; `#[serde(rename_all = "lowercase")]` doesn't case-fold on deserialize
4. Wire field is `fixed_versions: [String]`, not `fixed_in: String` — the speculative shape was wrong
5. Subprocess uses non-zero exit as a *signal channel* (exit 1 = "findings present"), not a failure indicator

**Decision:** for any feature that depends on a third-party subprocess or external API we don't own:

1. **Capture a real fixture before declaring done.** Run the actual tool once, save its output verbatim, write a parse test that consumes that fixture. The fixture is the canonical regression pin — keep it in tree even when the shape "looks stable."
2. **Document process-semantics assumptions at the trap site.** If a subprocess uses non-zero exit codes for signal (CI scanners, diff tools, grep), comment the exit-code table in the wrapper so a future maintainer doesn't "fix" it back to a stricter helper. Same for any non-obvious CLI flag requirements.
3. **Treat "defensive parsing" as a starting hypothesis, not a finished posture.** `#[serde(default)]` only helps when the field names are right. Wrong field names + defaults = silent data loss.
4. **Smoke-test on the real binary before declaring the build complete.** Unit tests in a sandbox validate *internal* correctness; only live smoke tests catch the *integration* assumptions. If the feature touches a subprocess, the build isn't done until someone has clicked the button and watched the data flow end-to-end.
5. **Distinguish `#[serde(rename_all = "lowercase")]` (serialization control) from case-folding deserialization (custom impl).** They are NOT the same. The default derived `Deserialize` is case-sensitive.

**Rejected alternatives:**

- "We'll catch it in code review." Reviewers don't run the binary either. The Step 2 review approved the speculative shape because it matched the declared brief — nobody had real output to compare against.
- "We'll catch it in CI integration tests." No CI runner installs every third-party subprocess our app might shell out to. brew-vulns specifically is a brew formula install — unrealistic to provision in GitHub Actions just to exercise our wrapper.
- "Mock the subprocess with a recorded fixture." Useful, but only after the fixture has been captured live at least once. The mock-first failure mode is "mock matches expectations; reality differs."

**Outcome:** added the §"Smoke test cycle" block to `tasks/2026-05/20-v0.5.0-vulnerability-scanning.md` documenting the five bugs + their fixes + the captured-fixture regression pin. Future subprocess-integration tasks (e.g. if we ever wrap `brew livecheck`, `brew bundle-doctor`, or any other third-party brew subcommand) should reference this ADR before committing to a "defensive parse" without a captured fixture.

## 2026-05-30: Distribute via own Homebrew tap (cask), official cask deferred

**Context:** Post-launch on r/MacOS, the most common question was "is there a `brew install`?" The only distribution was a manual `.dmg`. Two paths: (a) our own tap repo, or (b) submit the cask to the official `Homebrew/homebrew-cask`.

**Decision:** Ship our own tap now (`github.com/msitarzewski/homebrew-brew-browser`), defer the official-cask submission until the project clears the notability bar.

**Rationale:**
- Official `homebrew-cask` has a notability gate: under 75 stars / 30 forks / 30 watchers (standard) is auto-rejected, measured on *the packaged repo only* — the author's other repos (incl. agency-agents) don't count. brew-browser was at 45★ / 3 forks at decision time, below the bar.
- The own-tap path ships today with zero review latency and full control; it's the same model auto-brew and Applite use. Users run `brew tap msitarzewski/brew-browser && brew install --cask brew-browser`.
- Revisit official submission at ~75★. Self-submitted PRs (author == repo owner) face an even higher bar (225★ / 90 forks / 90 watchers), so a non-author submitter or crossing the standard bar with social-media-fanfare exception is the likely route.

**Implementation notes:**
- Cask installs the prebuilt, signed, notarized `.dmg` — preserves the Gatekeeper-clean guarantee. `app "brew-browser.app"`, `zap trash:` covers app-support/caches/prefs/keychain for clean uninstall.
- `depends_on macos: ">= :ventura"` (Ventura-or-newer). **`brew style --fix` rewrites this to the bare `:ventura` symbol, which pins to Ventura *exactly* and would block Sonoma/Sequoia/Tahoe.** A comment block in the cask warns against accepting that autocorrect. The cosmetic style offense is accepted in exchange for correct version semantics.
- Verified end-to-end before publishing: `brew audit --cask brew-browser` exit 0, `brew fetch --cask brew-browser` downloads + verifies sha256.
- Each release bumps the cask `version` + `sha256`. Auto-bump in `tools/release/` is a documented TODO; manual `shasum` + edit until then.
- **`auto_updates true` is set on the cask.** brew-browser has an in-app updater, so this tells brew the app self-updates and `brew upgrade` won't fight version drift. We still bump the cask each release (Homebrew 5.2.0+ auto-upgrades `auto_updates` casks when the tap version is newer), so both paths stay current.
- **dmg is the recommended path; cask is kept as a full second option.** README + landing lead with the direct `.dmg` download — it's the build signed/tested directly each release and keeps the user on the app's own verified (minisign) updater. The cask installs the *same* notarized dmg, so there's no separate trust surface; the preference is about update-path control, not artifact difference. Both are fully supported.
- **No install-source detection in the app.** The in-app updater behaviour is driven solely by the user's `update_auto_check` setting (off by default), regardless of whether the app was installed via dmg or cask. The app does not — and should not — detect its install source; the user's setting is the single source of truth. A dmg user keeps whatever they set; a cask user behaves identically in-app while brew handles cask upgrades separately.

**Rejected / deferred:**
- **From-source formula (`brew install --HEAD`)** — deferred experiment. Compiling Tauri inside a formula sandbox is unproven and the result would be unsigned/un-notarized (loses the core security guarantee), so it's not the default path. Only ever runs on explicit user `--HEAD` request, never automatically.
- **Nightly prebuilt cask-HEAD via CI** — deferred; needs Apple signing secrets in GitHub Actions + notarize-per-commit.

**Outcome:** tap live at `msitarzewski/homebrew-brew-browser`. README + landing page updated to lead with the `brew` install. projectbrief Distribution section added.

## 2026-05-30: Native Swift / SwiftUI / Liquid Glass rebuild (experiment)

**Context:** The v0.5.0 launch drew recurring "Tauri isn't native" criticism. The 2026-05-23 "Tauri 2 over Electron/Flutter/GPUI" decision still holds for the shipped product, but it's worth empirically testing how close a fully native rebuild gets — both to answer the criticism and to evaluate macOS 26's Liquid Glass as a possible future direction.

**Decision:** Spin up an experiment branch (`experiment/native-swift-liquid-glass`) that **ports** the existing interface to Swift 6 + SwiftUI + Liquid Glass (macOS 26 Tahoe), living in `native/` as a Swift Package. This is explicitly a port, **not** a redesign: data sources and functionality stay identical to the Tauri app (trending, AI-enhanced categories, GitHub integration, vulnerability scanning, auto-update). Does **not** supersede the 2026-05-23 Tauri decision — the production app stays Tauri on `main` unless/until this experiment proves out.

**Sub-decisions:**
- **SPM (`swift build`), not an Xcode project.** Full Xcode is installed but `xcode-select` points at Command Line Tools, so `xcodebuild` is unavailable from the CLI. `swift build` works under CLT and links every Liquid Glass API. `native/build-app.sh` wraps the SPM binary into a launchable `.app` (SPM alone produces a bare binary with no `Info.plist`, which macOS treats as a background process). Switching the toolchain (`sudo xcode-select -s`) was declined to keep the CLI build path.
- **Stock Apple scaffolding only — no overrides.** No custom window chrome, no `NSVisualEffectView`, no faked backgrounds. Established after several failed attempts to hand-build Xcode-like chrome: `NavigationSplitView`, `.inspector`, `Settings {}` + `SettingsLink`, `TabView`, `Form` carry the whole UI. When the stock default and a pixel-perfect custom look disagree, the stock default wins. (Contrast the 2026-05-24 Tauri "Vibrancy via `window-vibrancy`" decision — the native build deliberately avoids that whole class of window-material hacking.)
- **Reuse the Tauri data contracts verbatim.** `settings.json` keeps the same path + schema (Swift `AppSettings` ⇄ Rust `Settings`); bundled `categories.json` + `enrichment.json` are copied from `src-tauri/data/` (uncompressed in the native build for now). The brew/vulns/github/trending behaviors are reimplemented as Swift `actor`s mirroring the Rust modules, not redesigned.

**Rationale:**
- Settles the "native" question with a real artifact instead of a debate.
- Keeping it a strict port means the comparison is apples-to-apples and the memory bank remains the single spec for both shells.
- Stock-only keeps the experiment honest about what the platform gives for free vs. what required fighting it.

**Trade-off accepted:** Two codebases for the same product while the experiment runs. Mitigated by it being clearly branch-scoped and uncommitted; if it doesn't prove out, the branch is abandoned with zero impact on `main`. Sparkle-based in-app updates are the one subsystem not yet ported (deferred).

**References:** `native/README.md`, `techContext.md` ("Native rebuild" section), `tasks/2026-05/22-native-swift-liquid-glass-rebuild.md`, `progress.md` (2026-05-31), cross-session memory `project-native-swift-rebuild.md`.

## 2026-06-01: Keep BOTH codebases (Tauri + Swift) — parity charter (Option A)

**Decision:** Maintain the Tauri app AND the native Swift app long-term, in
**feature + data-contract parity** (NOT code parity). This is "Option A —
disciplined double-implementation." Option B (a shared `brew-core` Rust crate
behind both UIs, via sidecar-JSON or FFI) was considered and **deferred** — only
revisit when the same brew/parse bug has been fixed in both languages a third
time (i.e. when double-maintenance demonstrably hurts). If revisited, prefer the
**sidecar-JSON** approach over FFI (FFI's Swift↔Rust marshaling complexity
rarely pays off for a solo project).

**Why both (the hard constraint):** SwiftUI does NOT run on Linux and never will
(Swift-the-language does; SwiftUI is Apple-only; Liquid Glass is macOS-26-only).
So the two apps have **distinct, non-overlapping jobs** — neither can replace the
other:
- **Tauri** = the Linux app (`feat/linux-support`, builds on Ubuntu arm64) AND
  the pre-Tahoe macOS app (runs macOS 13+). The shipping product on `main`.
- **Swift** = the macOS-26 flagship (the "genuinely native" answer to the
  "Tauri isn't native" chatter). Requires macOS 26.

**Linux release is ORTHOGONAL to the Swift work** — it's pure Tauri (merge
`feat/linux-support`, CI for `.deb`/`.AppImage`, package, distribute). The Swift
branch neither helps nor blocks it. Don't conflate the two tracks.

### Parity rules (durable — these are instructions for future sessions)

The agent (Claude Code), not a human, maintains parity. The memory-bank is the
single canonical spec for BOTH apps. Concretely, every session:

1. **Any change to a shared DATA CONTRACT must land in both apps in the same
   work, or be explicitly logged as a parity gap.** Shared contracts:
   `settings.json` (path + schema — Rust `Settings` ⇄ Swift `AppSettings`),
   bundled `categories.json` + `enrichment.json` (Swift copies live at
   `native/Sources/BrewBrowserKit/Resources/`, sourced from `src-tauri/data/`),
   the trending endpoint (`brew-browser.zerologic.com`), GitHub OAuth client_id
   (`Ov23liJZKbvrSBuiOPkT`), and the `brew`/`brew vulns` CLI invocations + their
   parse gotchas.
2. **Cross-reference comments ARE the parity map.** Swift services already cite
   their Rust counterparts (e.g. `GitHubService.swift` → `auth.rs:NNN`,
   `VulnsService.swift` → the 5 brew-vulns smoke-test gotchas). Keep these
   citations current; when porting or fixing, read the cited Rust site first.
3. **When fixing a brew-integration bug in one app, check + fix the other.**
   The brew/vulns parse traps are identical across languages (same `brew` output).
   A fix in Rust likely applies to Swift and vice-versa.
4. **Bundled data refresh updates BOTH.** Re-running `tools/enrich` or
   `tools/catalog` regenerates `src-tauri/data/*`; the Swift `Resources/` copies
   must be re-copied in the same change (they're uncompressed copies).
5. **Log parity gaps honestly.** If a feature ships in one app only (e.g. Swift
   still lacks Sparkle updates + library-wide vuln scan-all), record it as a known
   gap in the task record — don't let the two silently drift undocumented.

**Trade-off accepted:** brew/vulns/github/trending logic is written twice (Rust +
Swift). This taxes every new feature. Accepted because the data contracts already
capture ~80% of parity value, the brew-parsing surface is stable, and a solo
project doesn't justify the bridge engineering of Option B yet.

**References:** the "keep both" question + analysis, 2026-06-01; supersedes
nothing (the 2026-05-30 native-rebuild ADR framed the Swift app as an experiment
"unless/until it proves out" — this commits to keeping both regardless, with
defined roles). See [[project-native-swift-rebuild]] cross-session memory.

## 2026-06-02: Tauri←native parity on a main-rooted branch; canonical velocity threshold; one-prompt Keychain

**Context:** The native macOS rebuild (`experiment/native-swift-liquid-glass`) pulled ahead in a few user-visible places. Per the parity charter (2026-06-01), the two builds are kept in feature/data-contract parity, so this work flows back to the shipped Tauri app. Full task record: `tasks/2026-06/01-tauri-native-parity.md`.

**Decisions:**

1. **Branch `tauri-parity` is rooted on `main`, not the experiment branch.** Verified every touched file (Svelte components, `auth.rs`, `Cargo.*`) was byte-identical between `main` and the experiment HEAD, so `git checkout main && git checkout -b tauri-parity` carried the uncommitted work over with zero conflicts. Keeps the eventual PR into `main` clean (no native commits in history). `native/` is untracked on this branch and excluded from commits.

2. **Canonical Trending velocity badge = formula-faithful banded.** `velocity_index` (`velocity.rs`) is documented as "1.0≈steady, >1.5 surging, <0.7 cooling." Native shipped a BINARY `v >= 1 ? flame : snowflake`, which mislabels near-steady packages (e.g. 1.05) as surging. Canonical rule for BOTH builds: `>=1.5` 🔥 / `<=0.7` ❄️ / otherwise neutral (no icon). Tauri's `velocityTier` updated (cool bound 0.5→0.7); native's binary→banded change is the reverse-parity item (memory `project-native-reverse-parity`).

3. **GitHub Keychain status check = one batched read.** Native collapsed three `SecItemCopyMatching` calls (token/username/scopes) into one `kSecMatchLimitAll` query → one auth prompt. Ported to Tauri via a `KeychainSlot::read_many` default (per-account) + a macOS `SystemKeychain` override using the `security-framework` crate's `ItemSearchOptions(...).limit(Limit::All)`. The `keyring` crate has no batch API, hence the direct Security-framework use. Three-account storage schema unchanged (keeps parity + existing users' tokens).

**Rejected alternatives:**
- *Single JSON-blob Keychain item* (one read, no new dep) — rejected: orphans existing tokens (forced re-sign-in) and diverges from native's three-account layout, breaking the keychain parity contract.
- *Make Tauri's velocity match native's binary* — rejected: native is the less-correct side here; aligning to the formula's documented bands is the right canonical.

**Outcome:** Tauri parity work complete and verified (`npm run check` clean, Rust compiles, screenshots confirmed). Native reverse-parity backlog captured in memory `project-native-reverse-parity`.

### 2026-06-06: Native deploy-prep decisions (parity push, Sparkle, rename)
**Status:** Approved + implemented (task `tasks/2026-06/10-*`).
- **Vuln scan = one `brew vulns --json` over the install set, parsed per-record.**
  `brew vulns --formula X` ignores the filter + returns everything; per-formula
  iteration over-reported (every package flagged) and was ~331× slow. Both apps
  now do a single call and key findings by `record.formula`. `VulnsService`
  (Swift) is a `Sendable struct`, not an actor (serialization trap).
- **Never show a green "no vulnerabilities" all-clear from a cached/stale scan.**
  Green only when scanned THIS session; cache-hydrated → amber caution; never
  scanned → hazard. A security tool must not imply safety it hasn't verified.
- **Self-updater = Sparkle 2** (decided 2026-06-06). Public ed25519 key committed
  in `build-app.sh`; private key in login Keychain; `native/release.sh` does the
  signed+notarized release + appcast. Two update feeds coexist on the host
  (`/appcast.xml` native, `/updater.json` Tauri) — never clobber one with the
  other. Updater stays inert when run unbundled (Xcode/`swift run`).
- **App display name = "Brew Browser"** (both builds). Native done freely (no
  users). Tauri `productName` renamed too (renames the bundle file → flag the
  duplicate-on-update migration for shipped 0.5.0 users in release notes).
- **Versions stay independent** (native 0.1.0, Tauri 0.5.0) — separate apps,
  separate bundle ids + update feeds; no edition marker / no "n" suffix.

### 2026-06-07: Launch-batch decisions (upgrade-all classification, native tests, dep posture)
**Status:** Approved + implemented (`tasks/2026-06/11-*`).
- **Non-fatal `brew upgrade` exits count as SUCCESS.** `brew upgrade` returns
  exit 1 on post-install warnings, link conflicts, and already-linked/already-
  present kegs even though it did the work — the cause of ~20 bogus "Upgrade-all
  failed" reports. Both builds now classify those (allowlist of non-fatal
  markers + a hard-fatal denylist) and present success, suppressing the
  file-an-issue CTA. Conservative: any hard-fatal signature (download/checksum/
  lock/report-this) still fails. Logic is `error_patterns::upgrade_warnings_only`
  (Rust) / `BrewOutputParsing.upgradeWarningsOnly` (Swift), unit-tested both sides.
- **Progress is best-effort + heuristic.** Parsed from brew's `==>` markers; no
  total ⇒ indeterminate bar; never blocks the stream. Unknown output = no tick.
- **Native gets a real test target.** Swift Testing + `@testable`, fixtures
  mirroring the Rust tests (the parity guarantee). Two prod tweaks to enable it:
  extracted `VulnsService.parseScanOutputKeyed`; `SettingsDTO` `private`→`internal`.
  The shared-catalog refactor (one JSON of brew-output rules consumed by both
  languages) remains the deferred post-launch cleanup.
- **Dependency posture:** the npm `cookie <0.7.0` low advisory is **accepted, not
  forced.** It's transitive via SvelteKit with no real surface in a desktop app,
  and forcing `cookie@0.7` under a SvelteKit that declares `0.6` risks a real
  runtime bug — a worse trade than a low advisory. The 17 unmaintained Rust
  warnings are Linux-only GTK3/glib deps (no CVEs, not built on macOS),
  documented + ignored in `src-tauri/.cargo/audit.toml`. Full audit: `security.md` §19.

### 2026-06-07: GitHub keychain (combined item), launch hydration, Tauri name revert, catalog source
**Status:** Approved + implemented (`tasks/2026-06/11-*`, commits `57f6f5f`…).
- **GitHub credential = ONE combined Keychain item (`github_credential_v1`),
  ported FROM the native build.** The Tauri 3-item layout + `kSecMatchLimitAll`
  batch (#37) was wrong on two counts: the batch *silently skips* consent-
  required items (status read returned empty after a successful sign-in →
  "signed in but Settings don't update"), and 3 items = 3 prompts under a
  churning identity. Native had already solved this (`GitHubService.swift`
  `github_credential_v1`); we ported it back (reverse-parity gap, native→Tauri).
  One item = one access = one prompt. Reads migrate the legacy 3 items in; writes
  delete-then-recreate so the writing binary owns the ACL.
- **Keychain churn is a `tauri dev` artifact, NOT a code bug.** Unsigned dev
  builds get a new code identity each rebuild → macOS re-prompts and login won't
  "stick." A signed/stable build (Developer-ID DR-based ACL) signs in once and
  stays. Don't chase keychain persistence in `tauri dev`; verify on a signed build.
- **Hydrate persisted state on launch.** The Dashboard GitHub card + vuln
  Exposure card + sidebar badges were empty every launch because the frontend
  never loaded the backend cache/keychain at startup (deliberately lazy). Now
  `+layout` hydrates GitHub status + the vuln cache after settings load (gated on
  the opt-in toggles, fire-and-forget). The old "don't probe Keychain on launch"
  worry only applies to unsigned dev builds.
- **Revert the Tauri app display name to `brew-browser`.** "Brew Browser"
  changed the bundle on disk (`brew-browser.app` → `Brew Browser.app`), which to
  macOS is a different app → shipped users would lose Keychain + window state on
  auto-update. Not worth it. Native keeps "Brew Browser" (no shipped users).
- **Bundled catalog stays the model for now (refresh per release).** The app
  ships its own `catalog/*.json.gz` snapshot AND the user's machine already has
  Homebrew's always-fresh `~/Library/Caches/Homebrew/api/internal/packages.<plat>.jws.json`
  (14 MB, JWS-signed). Reading brew's cache would kill the staleness + the
  duplication, but it's JWS-signed, the filename/format is a brew INTERNAL that
  changes across versions (`formula.jws.json` → `packages.<plat>.jws.json`), and
  it's absent on a never-run-brew machine. So the stable public-API + bundled-
  snapshot path stays; a hybrid (prefer brew's cache, fall back to bundled) is a
  deferred roadmap item. Interim: regenerate the bundle each release via
  `tools/catalog/fetch.py` (done 2026-06-07: as_of now, 8404 formulae / 7703 casks).

## 2026-06-11: Intel = separate per-arch builds (not universal); casks gated off Linux; installer never runs in-app

- **Separate arm64 + x86_64 artifacts for BOTH builds, no universal binaries**
  (user decision: "universal adds unnecessary weight for arm users"). Tauri:
  two dmgs, updater manifest gains `darwin-x86_64`, cask goes per-arch
  (`sha256 arm:/intel:`). Native: `build-app.sh [config] [arch]` +
  `release.sh` loops arches into arch-suffixed zips/dmgs; single Sparkle feed
  first (generate_appcast emits `sparkle:hardwareRequirements`), dual-feed
  fallback documented in the script if it can't disambiguate. Native x86_64
  audience = the four Intel Macs that run macOS 26 (MBP 16" 2019, MBP 13"
  2020 4-port, iMac 27" 2020, Mac Pro 2019 — Apple 122867); Intel Mac minis
  cap at Sequoia → served by the Tauri x64 dmg (min macOS 13).
- **Casks are a macOS-only concept — gate them out of Linux entirely**, data
  layer first (`catalog_casks_summary` empty, `brew search --cask` never
  spawned) plus a UI terminology sweep behind `isLinux`. Homebrew has no
  Linux cask mechanism even for apps with native Linux builds (Flatpak/Snap
  own that role). Hiding beats erroring: "macOS is required" mid-install is
  a trust-destroying dead end. The Library/PackageRow kind cell survives as a
  12px slot because it also hosts the vulnerability dot (functional on Linux).
- **The Homebrew installer never runs inside the app** (onboarding): it needs
  sudo + an interactive TTY, and an app silently wielding admin rights is the
  exact trust failure brew-browser avoids. Open Terminal pre-types a FIXED
  constant command (macOS); Linux is copy-only. The app polls the known
  prefixes every 2s and recovers in-process — PATH setup isn't needed for
  detection because we watch prefixes directly, not the shell environment.

**References**: `tasks/2026-06/13-intel-builds-onboarding-linux.md`
