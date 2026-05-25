# Phase 15 plan — In-app updates + Offline Mode rename

**Status:** Draft (2026-05-24, post-v0.2.1 release)
**Owner:** Michael
**Target release:** v0.3.0

## Goal

Two coupled changes:

1. **Add an in-app update mechanism.** Today, v0.2.1 users have no way to learn that v0.2.2 / v0.3.0 / etc. exists without stumbling on the GitHub releases page. We add a properly-gated "check for updates" flow — manual button, optional weekly auto-check (off by default), with the same consent posture as every other outbound network call in the app.

2. **Rename "Paranoid Mode" → "Offline Mode" in the user-facing UI.** "Paranoid" carries a stigma — it implies the user is being unreasonable to want network kill-switch behavior. "Offline Mode" is plainspoken, communicates the actual function, and matches the README's "no surprise network" framing. Internal field name (`paranoid_mode` in settings.json + `require_network()` helper) stays unchanged to avoid migration churn.

The two land together because update-check is one of the outbound network paths that Offline Mode kills, and a rename without a fresh user-visible reason to read the Settings → Network panel would be churn for no apparent benefit. Shipping them in the same release tells one coherent story: "we added in-app updates, and here's the renamed kill switch that controls them along with everything else."

## Out of scope (v0.3.0)

- **No auto-install.** Updates always require explicit user confirmation. We download on click, present "ready to install," and only relaunch into the new binary when the user clicks Install.
- **No background auto-download.** The auto-check toggle only checks for the existence of a newer version. Downloading the .dmg is always user-initiated, even when auto-check found one.
- **No telemetry from the update check.** The check fetches a static JSON manifest; we don't log "user X with version Y checked at time Z" anywhere. The manifest endpoint is a CDN-served static file with no server-side analytics.
- **No "skip this version forever" persistence across major versions.** Skip lasts for the specific skipped version only — a later release re-prompts.
- **No update channels (beta/stable/nightly).** One channel. If we want betas later, that's v0.4+.
- **No update for the `brew` CLI itself.** We're updating brew-browser, not Homebrew. The app's role w.r.t. brew remains "respectful UI on top."

## Design decisions

### Updater plugin: Tauri's official `tauri-plugin-updater`

Battle-tested, used by every shipping Tauri app I know of. Handles the manifest fetch, version comparison, .dmg download, signature verification, and the relaunch dance. ~150 lines of Rust + plugin config vs. several hundred for a roll-our-own version that would re-create the same primitives badly.

The alternative considered: phone home to GitHub Releases API, parse the JSON, show a "v0.3.0 available — download here" banner with a click-through to the .dmg URL. User manually drags-and-drops the new .app over the old one. This is zero new infrastructure but unacceptable friction — most users won't bother, and the ones who do are the security-minded crowd who would prefer the verified-signature in-app install anyway.

### Manifest hosting: `https://brew-browser.zerologic.com/updater.json`

Hosted on the existing landing-page server (Caddy on umbp). One file, static, no infrastructure ramp-up. Updated as part of the release workflow.

The alternative considered: publish `updater.json` as a release asset on GitHub Releases, point Tauri at `github.com/msitarzewski/brew-browser/releases/latest/download/updater.json`. Pros: zero new files outside the existing release artifacts, lives next to the .dmg. Cons: GitHub Releases redirects asset downloads to `objects.githubusercontent.com`, which means CSP needs both hosts allowlisted (extra surface for review), and our control over the URL is at the mercy of GitHub's URL stability.

Going with zerologic for: smaller CSP delta (one new host), tighter control over the manifest format and its evolution, and a clean separation between "the artifact" (GitHub Releases) and "the discovery layer" (our domain).

### Signature verification: Tauri's minisign-based scheme

Tauri's updater verifies a minisign signature over the downloaded artifact. Without this, anyone who could spoof DNS to `brew-browser.zerologic.com` or compromise the manifest could push a malicious binary. Apple's notarization signature on the .dmg verifies that Apple's notary service signed it; the minisign signature verifies that the artifact came from us — different threat model, both needed.

One-time setup:
- Generate a minisign keypair via `tauri signer generate -w ~/.config/brew-browser/updater.key`
- Store the private key chmod 600 outside the repo (same dir as `signing.env`)
- Hardcode the public key as a `const` in `src-tauri/src/lib.rs` (Tauri convention — public keys are public)
- Build flow signs the .dmg with the private key as a step in `npm run tauri build`; the signature gets published into the manifest

### UI placement: title-bar indicator pill + Settings → Network → Updates subsection

**No modal, ever.** Update notifications surface as a peripheral indicator in the title bar — matching the Mac-app convention where Mail shows unread counts or Notes shows sync-pending dots in chrome rather than blocking the user with dialogs.

**Title-bar indicator pill** (new `UpdateIndicator.svelte`), positioned in `.titlebar-right` immediately before the `TitlebarControls` cluster (theme + settings + donate), with a small gap between them:

- Renders **only** when `updater.available !== null` AND Offline Mode is off
- Visual: small pill, `--color-surface` background matching the controls cluster, accent-colored "↑ Update available" text
- **Click the pill** → opens Settings on the Network section via `ui.openSettings("network")` (we already plumbed deep-link support for "github"; "network" works through the same path). The Updates subsection is the bottom of that pane, scrolls into view, and the install action is one click away.
- **Click the small × on the right edge of the pill** → calls `updater.skip(version)`, which writes the current available version to the skip-list. The pill disappears. A newer version (when one ships) re-triggers the indicator. Click bubbling is stopped so the × doesn't also trigger the pill's "open settings" handler.
- The × close behaves *as* the skip signal — no separate "skip this version" button is needed in the indicator itself. The user who knows what they want can dismiss with one click; the user who wants to read about it clicks the pill and lands in Settings.

**Settings → Network → Updates subsection** (new `SettingsSectionUpdates.svelte`, mounted inside the existing Network section):

- **Check for updates now** button — manual trigger, disabled with tooltip "Disabled by Offline Mode" when Offline Mode is on
- **Auto-check daily** toggle — opt-in, default **off**
- **When an update is available**, the subsection grows a notice card: "**v0.3.1 available** · [Release notes ↗] · [Install update]". No separate "Skip" button here — clicking the × on the title-bar indicator is the skip path, and the Settings card only shows the positive action.
- **Update channel** display: "Stable" (read-only, placeholder for future v0.4+ beta channel support)

This split puts the *attention-getter* in chrome (peripheral, glanceable, dismissable in one click) and the *action* in Settings (where the user can read release notes, see what's changing, and commit deliberately). No path requires a modal.

### Offline Mode (the rename) gates the entire update mechanism

- The manual button is disabled with tooltip "Disabled by Offline Mode" when Offline Mode is on
- The auto-check timer is suppressed (no fetch happens) when Offline Mode is on
- The auto-check toggle itself stays usable so the user can configure their preference for the next time Offline Mode is off
- If a previous check has already cached an "update available" notice, the notice continues to display (the data is already on disk) but the Install button is disabled with the same tooltip until Offline Mode is off

This matches the existing behavior of every other outbound feature: the toggle remains configurable, the action is the thing that gets blocked.

## Implementation steps

### Backend (Rust)

1. **Add `tauri-plugin-updater` to `src-tauri/Cargo.toml`** + register in `src-tauri/src/lib.rs`. Plugin config: endpoint URL pointing at `brew-browser.zerologic.com/updater.json`, public key embedded as const.

2. **Wrap the plugin behind the `require_network("update_check")` chokepoint.** Two new commands:
   - `update_check_now() -> Result<UpdateCheckOutcome, BrewError>` — calls `require_network("update_check")?`, then invokes the plugin's check. Returns one of: `UpToDate`, `Available { version, notes_url, signature_ok }`, `Blocked` (offline mode).
   - `update_install(version: String) -> Result<(), BrewError>` — same gate, then invokes the plugin's download + verify + install. The `version` arg is a sanity check (the cached "available" entry must match) to defend against UI staleness.

3. **Skip-list persistence.** Add `settings.skipped_update_versions: Vec<String>` to the existing settings schema. The skip button writes to it; the check command consults it to know whether to surface a notice for a given version. Caps: max 10 entries (oldest evicted) to bound storage.

4. **Auto-check scheduler.** Tokio task spawned at startup that wakes every 24h and calls `update_check_now()` when the auto-check toggle is on AND Offline Mode is off. Backoff on failure: 1h, then 6h, then 24h. Last-checked timestamp persisted so cross-launch behavior is predictable. No timer fires faster than once per 24h regardless of launches.

5. **Release-build hook.** Add a step to the build flow that signs the .dmg with the minisign key and emits the manifest JSON. Both go into `dist/` for the publish step.

### Frontend (Svelte)

6. **New IPC wrappers** in `src/lib/api.ts`: `updateCheckNow()`, `updateInstall(version)`.

7. **New `updater` store** at `src/lib/stores/updater.svelte.ts`:
   - State: `lastChecked`, `available: UpdateInfo | null`, `installing: boolean`, `error: string | null`
   - Actions: `checkNow()`, `install()`, `skip(version)`
   - Reactive to `settings.effective.offlineMode` so the UI knows to disable buttons

8. **New `SettingsSectionUpdates.svelte`** mounted at the bottom of the existing Settings → Network section. Contents: "Check for updates now" button, "Auto-check daily" toggle, channel display, conditional update notice card (when `updater.available`). Install action lives here. Honors Offline Mode for button state.

9. **New `UpdateIndicator.svelte`** placed in `.titlebar-right` immediately before `<TitlebarControls />` (with an 8px gap between them). Renders nothing when `updater.available` is null or Offline Mode is on. Otherwise renders a small pill: accent-colored "↑ Update available" text + small × close button. Pill onclick → `ui.openSettings("network")`. × onclick → `updater.skip(version)` + `event.stopPropagation()`. Inline progress state during install (small spinner replaces the × while installing) to avoid spawning a separate progress UI.

10. **Install flow.** "Install update" button in Settings invokes `update_install(version)`. Progress streams back via Tauri event channel and updates a small progress bar in the Settings card (same pattern as the existing brew-job streaming). On completion: success toast + "Relaunch now" button. No modal. Failure: inline error in the Settings card, retry available. The title-bar indicator pill reflects the installing state too so the user sees activity even if they navigated away from Settings.

### Rename sweep (Paranoid Mode → Offline Mode)

11. **UI-only changes.** Search every user-facing string for "Paranoid" / "paranoid mode" and replace with "Offline" / "Offline Mode" in:
    - `src/lib/components/SettingsSectionNetwork.svelte` (section heading, toggle label, description)
    - All toast messages currently saying "Blocked by Paranoid Mode" → "Blocked by Offline Mode"
    - All inline error states in `PackageDetail.svelte` (cask icon blocked, GitHub stats blocked, etc.)
    - `README.md` — every "Paranoid Mode" mention in §"Open by default" gets renamed; rephrase the surrounding sentence if needed for natural prose
    - `landing/index.html` — same treatment
    - `AboutModal.svelte` (if "Paranoid" appears there)

12. **Internal references stay as-is.**
    - `settings.json` key remains `paranoid_mode` (no migration required, settings files in the wild keep working)
    - `require_network(feature)` helper keeps its name (the feature names like `"github_stats"` are internal)
    - `memory-bank/security.md` keeps its existing "Paranoid Mode" references; a one-line footnote at the top of the doc notes the v0.3.0 UI rename. §1-§14 archeology stays intact.

13. **Tooltip on the new "Offline Mode" toggle** explains: "Blocks every outbound network call: catalog refresh, Trending fetch, GitHub stats, GitHub sign-in, cask icon homepage probes, update checks. All UI that depends on the network shows a 'disabled by Offline Mode' notice. brew itself still runs normally — its network access is the user's call at the terminal."

## Tests

- **`update_check_now` happy path** — mock plugin returns "no update," command returns `UpToDate`
- **`update_check_now` available path** — mock returns "v0.3.1 available," command returns `Available { ... }` with the right fields
- **`update_check_now` blocked by Offline Mode** — Offline Mode = on, command returns `BrewError::ParanoidModeBlocked { feature: "update_check" }` (internal name stays paranoid_mode_blocked for serde compatibility)
- **`update_install` rejects a stale version arg** — UI requests install of v0.3.0 when manifest says v0.3.1; command returns `BrewError::InvalidArgument` rather than installing the wrong thing
- **Skip-list cap** — adding the 11th skip evicts the oldest entry; check that the new entry is present and the oldest is gone
- **Auto-check scheduler honors the 24h floor** — repeated start/stop within an hour does not fire multiple checks
- **Auto-check scheduler suspends when Offline Mode flips on** — flipping the toggle mid-cycle causes the next scheduled check to no-op
- **Manifest signature verification failure path** — feed a manifest with a wrong-key signature; install command returns `BrewError::SignatureVerificationFailed`
- **Manifest sha256 mismatch** — manifest declares sha256 X, downloaded artifact hashes to sha256 Y; install command returns `BrewError::HashMismatch` before invoking minisign verification (cheaper check first)
- **Title-bar indicator rendering gate** — store has `available = null` → indicator renders nothing; store has `available = {...}` AND Offline Mode = off → indicator renders pill; store has `available = {...}` AND Offline Mode = on → indicator hides (component test)
- **Title-bar indicator × dismiss is the skip path** — clicking × on the indicator pill calls `updater.skip(version)` and the indicator disappears; the version is now in the skip-list; subsequent `update_check_now` calls that return the same version do NOT re-surface the indicator
- **Title-bar indicator click opens Settings on Network section** — clicking the pill (anywhere except the ×) invokes `ui.openSettings("network")`; `ui.settingsInitialSection === "network"` after the click

## Threat model considerations

This is a new outbound-network feature that downloads and executes code, so it warrants a careful read.

**Manifest fetch (`GET https://brew-browser.zerologic.com/updater.json`):**
- HTTPS-only, scheme-locked in the plugin config. No http:// fallback.
- Endpoint is hardcoded; no user-supplied URL means no SSRF.
- CSP gains one new origin: `connect-src` += `https://brew-browser.zerologic.com`.
- Manifest body has a size cap (8 KiB, vastly oversized for the actual ~500-byte payload) — prevents a compromised endpoint from streaming a multi-gigabyte body to OOM the app.
- Manifest JSON parsing fails closed: any missing/malformed field returns `Err`, no partial state.

**Artifact download (`GET https://github.com/.../brew-browser_X.Y.Z_aarch64.dmg`):**
- The artifact URL is read from the manifest, but validated against an allowlist of trusted hosts (`github.com` and `objects.githubusercontent.com` for the redirect target) before download. A compromised manifest cannot point us at an arbitrary URL.
- HTTPS-only, no http:// fallback, no redirect to a non-allowlisted host (re-validated on every hop, matching the cask-icon SSRF defense pattern).
- Download size cap of 200 MB (vastly oversized; current .dmg is ~13 MB) to bound disk pressure.

**Signature verification:**
- Minisign public key hardcoded as `const PUB_KEY: &str = "..."` in `src-tauri/src/lib.rs`. Public keys are public; embedding them in the binary is the standard pattern.
- Tauri's updater plugin handles the verification call. We don't roll the crypto ourselves.
- Verification failure aborts the install with no on-disk side effects (the downloaded .dmg is deleted before the error returns).

**Privilege escalation:**
- The install replaces the .app bundle inside `/Applications/brew-browser.app/`. This requires write access to that path, which a normal user has for apps they installed themselves. No `sudo` prompt.
- The replacement is atomic (rename-based) so a crash mid-install doesn't leave a half-installed app.
- The new binary is launched only on user click; we never relaunch automatically without confirmation.

**Telemetry concerns:**
- The manifest fetch reveals to our server: the requesting IP, User-Agent (Tauri default), and the request timestamp. No version number is sent (the comparison is client-side). No user identifier of any kind.
- We do not log these requests beyond Caddy's standard access log, which rolls off at 7 days. No analytics, no aggregation, no telemetry shipped to a third party.
- This is documented in the privacy posture as outbound network path #8 ("manifest check from brew-browser.zerologic.com when you click 'Check for updates'").

**Adversarial scenarios:**
- *DNS spoof to attacker-controlled IP:* TLS cert verification catches it; the attacker can't serve a valid cert for brew-browser.zerologic.com without compromising our private key or a CA. If they did, the minisign signature on the artifact provides a second line of defense.
- *zerologic.com compromised, attacker pushes a malicious manifest pointing at a malicious .dmg:* The artifact URL allowlist limits where the .dmg can come from, and the minisign signature must verify against our embedded public key. The attacker would need to compromise both zerologic AND the offline-stored minisign private key to forge an update.
- *zerologic compromised + attacker pushes a manifest pointing at a real (older) brew-browser version:* This is a downgrade attack. Mitigation: Tauri's updater compares the manifest version to the running version and refuses to "update" to a same-or-older version. We add a defense-in-depth check that explicitly rejects downgrades.

## Rollout plan

1. **v0.3.0 ships** the updater plugin + Offline Mode rename, but uses the existing zerologic.com landing-page deploy flow to publish the manifest. First manifest contains v0.3.0 (not v0.2.1) so existing v0.2.1 users won't immediately see "v0.3.0 available" until they update through it.
2. **v0.3.1 (or any subsequent release) is the first test of the auto-update path.** Existing v0.3.0 users with the auto-check toggle on will get the in-app prompt; users who left it off can still hit "Check for updates now" manually.
3. **memory-bank/security.md §15 captures the post-Phase-15 audit** of the new code surface: manifest fetch, signature verification, the artifact-host allowlist, the Offline Mode chokepoint coverage.
4. **README §"Open by default" updated** to list path #8 (manifest check) alongside the existing 7 paths, and to rename "Paranoid Mode" → "Offline Mode" throughout. Landing page mirrored.

## Resolved decisions

These were "open questions" in the draft; all five are now decided. Recorded here as a running ledger of what we landed on and the Mac-app reasoning behind each.

1. **Skip-list lives in `settings.json`** as `skipped_update_versions: Vec<String>` (capped at 10, oldest evicted). Sparkle convention: every shipping Sparkle-based Mac app (VS Code, Discord, Slack, Tower, Things) stores update state alongside user prefs in `NSUserDefaults`. A separate `update-state.json` would be a file users could lose or wonder about. Lower surface, fewer questions.

2. **Manifest carries sha256 in addition to minisign signature.** Sparkle manifests include both EdDSA signatures *and* length/hash metadata; Carbon Copy Cloner, Tower, and every paid Mac app's update manifest carries hashes. Costs ~20 bytes, defense-in-depth, gives the security-pro user a `shasum -a 256` path to verify out-of-band against the published manifest. The verifier checks sha256 first (cheap) before minisign (more expensive).

3. **Auto-check fires daily (when opted-in).** Sparkle's default is daily, macOS App Store checks per-launch (effectively daily), VS Code defaults to daily. Users have been conditioned to expect daily-ish. The case-clincher: when we ship a hotfix like v0.2.0 → v0.2.1, users finding the fix in 24h beats finding it in 6 days. The manifest endpoint is a static file on Caddy — load cost is irrelevant. Users who prefer weekly can configure in Settings.

4. **Stable channel only at v0.3.0.** Beta channels go in when there's real user demand and a stable release cadence to split — Tower added one years after launching, Things did the same. Beta channel requires a separate manifest, separate signing, separate skip-list, and additional UI. Premature at v0.3.0 with a fresh launch audience and no user calling for it. Revisit at v0.4+ if real demand materializes.

5. **`memory-bank/security.md` keeps "Paranoid Mode" in §1–§14, with a one-line footnote at the top noting the v0.3.0 UI rename.** The doc is internal-facing audit narrative; refactoring 50+ paragraphs of cross-referenced prose to swap a term is high-effort, low-value, and risks breaking links between sections. Apple does the same — internal docs note when UX terms change, historical content stays. The footnote: *"Note: 'Paranoid Mode' is the internal name (and the `paranoid_mode` field name in `settings.json`) for what v0.3.0+ surfaces as 'Offline Mode' in the UI. Both terms refer to the same kill switch."*

## Open questions (post-v0.3.0)

None blocking implementation. Future considerations to defer:

- **Beta channel** (decision #4) — revisit if real demand materializes
- **Update size threshold** — at what download size do we warn the user before starting? Today's .dmg is ~13 MB so a non-issue, but if we ever pick up dependencies that bloat the binary, a "this download is ~50 MB, continue?" gate becomes a fair question
- **Background download with explicit install** — currently we hold downloading until user click. If user behavior shows them always installing on click, we could pre-download silently after the auto-check and have install be instant. Adds complexity for marginal UX gain; defer until there's a reason

## Estimated effort

- Backend: 1 working session (~4-6 hours) — plugin integration, two new commands, scheduler, skip-list, tests
- Frontend: 1 working session (~3-5 hours) — store, Settings subsection, banner, install flow, the rename sweep
- Release infra: 0.5 sessions — manifest publishing step, minisign keypair setup, BUILD.md update
- Audit + README updates: 0.5 sessions — security.md §15, README path #8 and rename, landing page mirror

Realistic ship: one focused day, same shape as v0.2.0 — if we go in fresh.
