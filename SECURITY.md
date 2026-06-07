# Security Policy

Thanks for taking the time to look. This project takes security seriously, and reports — large or small — are welcome. Brew Browser ships in **two builds** (a cross-platform **Tauri** app and a native **Swift/SwiftUI** app); both are in scope.

## Supported versions

| Build | Version | Supported |
|-------|---------|-----------|
| Tauri (macOS 13+ / Linux) | latest `0.5.x` | Yes |
| Native (Swift/SwiftUI, macOS 26) | latest `0.x` | Yes |

This is an early-stage project. The two builds version independently. The latest released line of each build receives security fixes; once a new minor ships, the previous minor receives fixes for 90 days.

## Reporting a vulnerability

Email **msitarzewski@gmail.com** with:

- A clear description of the issue and the impact you believe it has
- Steps to reproduce, or a proof-of-concept if you have one
- The version / commit you tested against
- Your name or handle if you'd like credit (optional)

Please do **not** open a public GitHub issue for security reports. Once a fix is shipped, the original report can be cross-linked from the public changelog.

## Response time

This is a side project, so responses are best-effort:

- **Acknowledgement:** within 7 days of receipt
- **Initial assessment:** within 14 days
- **Fix or mitigation plan:** within 30 days for high/critical findings

If a report sits unanswered past these windows, a polite follow-up is welcome.

## Scope

**In scope (both builds unless noted):**

- Remote code execution in the app, any Tauri IPC command, or any native service
- Command/argument injection into a `brew` (or `du`/`security`) subprocess
- Privilege escalation
- Data exfiltration from the local machine; leakage of the GitHub token out of the Keychain
- Cross-site scripting (XSS) in the webview *(Tauri build)*
- SSRF or other outbound-request abuse originating from the app
- Path traversal / arbitrary file read/write through any command or the Brewfile/snapshot id handling
- Cache poisoning of the icon, trending, enrichment, or vulns cache
- Bypass of the URL-scheme allowlist on the homepage opener
- Acceptance of a tampered or unsigned in-app update (minisign for Tauri, Sparkle ed25519 for native)

**Out of scope:**

- Vulnerabilities in `brew` itself — report those to [Homebrew](https://github.com/Homebrew/brew/security/policy)
- Vulnerabilities in third-party tap content, formulae, or casks installed via this app
- Vulnerabilities in macOS, WebKit, or other system components
- Attacks that require an already-compromised local account (same-UID processes can do anything the user can)
- Social-engineering attacks
- Missing security headers on `formulae.brew.sh` (not our service)
- Issues that require physical access to an unlocked machine

## Disclosure policy

Coordinated disclosure, 90-day default. If a fix takes longer than 90 days, the reporter and the maintainer agree on an extended timeline in writing before the embargo expires. If no fix is plausible within 90 days, the reporter is free to publish after that window closes.

A current security audit lives at [`memory-bank/security.md`](./memory-bank/security.md) and may answer your question before you write the email.

## Hall of fame

Reporters who have found and responsibly disclosed security issues:

<!-- Add as: Name (handle) — short description, fix in commit/PR link -->

- **[@neodave](https://github.com/neodave)** — path traversal in Brewfile / snapshot id handling: an unvalidated id could escape the storage directory via `..` or an absolute component when joined into a filesystem path. Fixed with an allowlist-validated id chokepoint (`[A-Za-z0-9_-]`) before any `Path::join`, mirrored into the native build's `SnapshotStore.validateID`. ([#46](https://github.com/msitarzewski/brew-browser/pull/46))
