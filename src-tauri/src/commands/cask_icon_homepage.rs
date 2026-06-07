//! `cask_icon_from_homepage` — favicon/og-image cascade for casks that
//! are **not** installed but expose a homepage URL.
//!
//! Sibling of `cask_icon` (Phase 7): same on-disk cache, same data-URL
//! return shape, same `Option<String>` "absent is normal" contract. The
//! difference is the *source* — we can't read a local `.app` bundle for
//! an uninstalled cask, so we walk a small, polite cascade against the
//! cask's published homepage:
//!
//!   0. Appcasks — `github.com/App-Fair/appcasks/releases/download/
//!      cask-<token>/AppIcon.png` (community-curated real app icons; the
//!      source Applite / App Fair use). Tried first — best quality, no crawl.
//!   1. `<scheme>://<host>/apple-touch-icon.png` (Apple-blessed convention)
//!   2. `<meta property="og:image">` from the homepage HTML
//!   3. `<scheme>://<host>/favicon.ico`
//!   4. Google favicon service — `google.com/s2/favicons?domain=<host>`
//!      (near-universal fallback for the long tail).
//!
//! First 2xx + `image/*` content-type wins. We normalize whatever we get
//! to a 64×64 PNG via macOS-native `sips` (same as `cask_icon`) and
//! cache to `<state.cache_dir>/icons/<token>.png`. The cache slot is
//! shared with `cask_icon` — when a user later installs the cask, the
//! Phase 7 path will overwrite the homepage-derived icon with the real
//! bundle icon transparently.
//!
//! Hard requirements:
//! - Token runs through `validate_package_name` so it can't escape into
//!   filenames or `sips` argv (same gate as `cask_icon`).
//! - Homepage URL must be `http://` or `https://`; anything else → Ok(None).
//! - Single user-agent string identifies us as a polite client.
//! - 5s timeout per HTTP probe; redirects followed.
//! - Sticky-null marker file (`<token>.png.miss`) records "tried, no
//!   icon" so we don't re-hammer the homepage on every UI re-render.
//!   TTL matches the success cache (7 days).
//! - No mutex — read-only, parallel-safe across tokens.
//!
//! Error model: `Ok(None)` is the *common* case for casks whose homepage
//! has no usable icon (or no homepage at all). `Err(...)` is reserved for
//! genuine problems: cache dir IO, `sips` crashes, invalid token. Network
//! flakes collapse to `Ok(None)` so a flaky DNS lookup doesn't paint a
//! red toast over a Discover row.

use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use reqwest::header::CONTENT_TYPE;
use reqwest::redirect::Policy as RedirectPolicy;
use tauri::State;
use tokio::process::Command;
use tokio::sync::Semaphore;

use crate::commands::info::validate_cask_token;
use crate::commands::settings::{CaskIconMode, Settings, SettingsLoadState};
use crate::error::BrewError;
use crate::state::AppState;

/// Cache TTL — match the `cask_icon` command (Phase 7) so the two
/// commands have one consistent freshness story.
const ICON_CACHE_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// Display size for the rendered PNG. Same as `cask_icon` — keeps data
/// URLs tight and the cache directory uniform.
const ICON_PIXELS: u32 = 64;

/// HTTP probe timeout per request. Brief but not hostile — most CDNs
/// answer in under a second; 5s leaves margin for slow origins without
/// stalling the UI thread.
const HTTP_TIMEOUT: Duration = Duration::from_secs(5);

/// Cap on HTML body bytes when scraping for `<meta og:image>`. 64 KB is
/// generous (most landing pages are < 32 KB of HTML) and bounds memory.
const HTML_MAX_BYTES: usize = 64 * 1024;

/// User-Agent. Identifies the app and gives ops at the receiving end a
/// contact to file abuse reports against if we ever misbehave.
const USER_AGENT: &str =
    "brew-browser/0.1 (+https://github.com/msitarzewski/brew-browser)";

/// Global concurrency cap for outbound homepage probes (L4).
///
/// A single Trending or Discover render can resolve dozens of cask rows
/// at once, each kicking off a 3-step cascade. Without a cap we'd burst
/// ~100+ concurrent connections at fan-out time — impolite to the
/// receiving CDNs and a vector for accidental rate-limits / IP bans.
///
/// 16 is generous enough that one user's interactive scroll never
/// queues (the cascade short-circuits on first hit, so typical resolved
/// time is ~200-800ms per row) but bounded enough that we stay below
/// any reasonable per-host abuse threshold even when all 16 probes
/// happen to land on the same CDN.
const MAX_CONCURRENT_PROBES: usize = 16;

/// Process-wide semaphore. Initialised lazily on first use via
/// `OnceLock` — no AppState wiring required and zero cost when the
/// command is never called.
static PROBE_SEMAPHORE: std::sync::OnceLock<Arc<Semaphore>> = std::sync::OnceLock::new();

fn probe_semaphore() -> Arc<Semaphore> {
    PROBE_SEMAPHORE
        .get_or_init(|| Arc::new(Semaphore::new(MAX_CONCURRENT_PROBES)))
        .clone()
}

/// Decide whether `cask_icon_from_homepage` may proceed past the
/// settings gate. Pure function — extracted from the command so the
/// three-mode decision can be unit-tested without an `AppState`.
///
/// - [`CaskIconMode::Off`]            → false (silent skip)
/// - [`CaskIconMode::InstalledOnly`]  → only when `is_installed_cask`
/// - [`CaskIconMode::All`]            → true (current behaviour)
pub(crate) fn cask_icon_gate_decision(mode: CaskIconMode, is_installed_cask: bool) -> bool {
    match mode {
        CaskIconMode::Off => false,
        CaskIconMode::InstalledOnly => is_installed_cask,
        CaskIconMode::All => true,
    }
}

#[tauri::command]
pub async fn cask_icon_from_homepage(
    token: String,
    homepage: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, BrewError> {
    // Paranoid-mode gate (Phase 12d). Must run before any token or URL
    // validation — the goal is to silently refuse outbound traffic the
    // moment paranoid mode flips on, even for a perfectly valid cask.
    state.require_network("cask_icon_from_homepage").await?;

    // `cask_icon_mode` setting (Phase 13 — Finding 2 follow-up).
    // Resolve the snapshot up front so the gate decision is testable
    // (see `cask_icon_gate_decision` below) and so we never re-read
    // `settings` mid-cascade.
    let settings_snapshot = {
        let guard = state.settings.read().await;
        match &*guard {
            SettingsLoadState::Loaded(s) => s.clone(),
            // `Corrupt` is unreachable here — `require_network` above
            // would have already returned `ParanoidModeBlocked`. Defensive
            // only: fall back to defaults (= `All`) so a hypothetical
            // future caller that skips `require_network` still behaves
            // sensibly.
            _ => Settings::default(),
        }
    };

    // Short-circuit per the gate before touching the filesystem or the
    // network. `InstalledOnly` consults the cached installed list — we
    // snapshot it under a read lock so the matching is consistent for
    // this call.
    let proceed = {
        let installed_guard = state.installed_cache.read().await;
        let is_installed_cask = installed_guard
            .as_ref()
            .is_some_and(|list| list.casks.iter().any(|p| p.name == token));
        cask_icon_gate_decision(settings_snapshot.cask_icon_mode, is_installed_cask)
    };
    if !proceed {
        return Ok(None);
    }

    // Defense in depth — token reaches the filesystem (cache path), so
    // the stricter cask-token validator applies (L1). This rejects `/`
    // and `..` segments that `validate_package_name` would otherwise
    // accept, preventing `<cache_dir>/icons/<token>.png` from escaping
    // the cache root. Validation must happen *before* we construct any
    // cache path — the previous ordering wrote zero-byte miss markers
    // to attacker-influenced paths before brew ever saw the token.
    validate_cask_token(&token)?;

    let icons_dir = state.cache_dir.join("icons");
    ensure_dir(&icons_dir)?;
    let cache_path = icons_dir.join(format!("{}.png", token));
    let miss_path = miss_marker_path(&cache_path);

    // Fast path 1: serve from cache when fresh.
    if let Some(data_url) = read_fresh_cache(&cache_path).await? {
        return Ok(Some(data_url));
    }

    // Fast path 2: sticky-null marker. We previously walked the cascade
    // and came up empty within TTL; skip the network entirely so a
    // re-render of the Discover row doesn't re-probe the homepage.
    if miss_marker_is_fresh(&miss_path).await {
        return Ok(None);
    }

    // Parse + validate the homepage URL before touching the network.
    // SSRF gate: `parse_http_url` rejects non-public IPs and known
    // internal TLDs (`.local`, `.internal`, `localhost`) so a malicious
    // cask homepage can't probe loopback / RFC1918 / link-local /
    // cloud-metadata addresses before we even consider issuing a
    // request.
    let parsed = match parse_http_url(&homepage) {
        Some(u) => u,
        None => {
            // Non-http(s) URLs (data:, javascript:, etc.) or non-public
            // hosts collapse to a miss — record the sticky marker so we
            // don't re-check.
            touch_miss_marker(&miss_path).await;
            return Ok(None);
        }
    };

    let client = match build_http_client() {
        Ok(c) => c,
        Err(_) => {
            // reqwest builder failure is genuinely exceptional; rather
            // than propagating, treat as a miss so the UI stays quiet.
            touch_miss_marker(&miss_path).await;
            return Ok(None);
        }
    };

    // Concurrency gate — acquire a slot from the process-wide semaphore
    // before the network walk. If we can't acquire (cap reached), the
    // call queues briefly; the 5s per-probe timeout bounds wait time.
    let sem = probe_semaphore();
    let _permit = match sem.acquire_owned().await {
        Ok(p) => p,
        Err(_) => {
            // Semaphore closed (impossible — it's static + never closed),
            // but handle defensively rather than panicking.
            touch_miss_marker(&miss_path).await;
            return Ok(None);
        }
    };

    // Walk the cascade. First success short-circuits.
    //
    // Step 0 (parity with the native Swift IconService, 2026-06-01): Appcasks —
    // the community-curated icon repo Applite/App Fair use. A real, high-quality
    // app icon when the cask's dev opted in. Predictable github.com release-asset
    // URL (no homepage crawl), so it's tried first. github.com is already a
    // public host, so no extra SSRF surface. Coverage is sparse → the homepage
    // cascade + Google favicon below cover the long tail.
    if let Some(bytes) = probe_appcasks(&client, &token).await {
        if write_and_normalize(&bytes, &cache_path).await.is_ok() {
            clear_miss_marker(&miss_path).await;
            return encode_png_as_data_url(&cache_path).await.map(Some);
        }
    }

    if let Some(bytes) = probe_apple_touch_icon(&client, &parsed).await {
        if write_and_normalize(&bytes, &cache_path).await.is_ok() {
            clear_miss_marker(&miss_path).await;
            return encode_png_as_data_url(&cache_path).await.map(Some);
        }
    }

    if let Some(bytes) = probe_og_image(&client, &parsed).await {
        if write_and_normalize(&bytes, &cache_path).await.is_ok() {
            clear_miss_marker(&miss_path).await;
            return encode_png_as_data_url(&cache_path).await.map(Some);
        }
    }

    if let Some(bytes) = probe_favicon(&client, &parsed).await {
        if write_and_normalize(&bytes, &cache_path).await.is_ok() {
            clear_miss_marker(&miss_path).await;
            return encode_png_as_data_url(&cache_path).await.map(Some);
        }
    }

    // Final step (parity with native IconService): Google's favicon service.
    // One clean URL for any domain, no HTML scrape — near-universal coverage
    // for the long tail the homepage probes miss. google.com is a public host.
    if let Some(bytes) = probe_google_favicon(&client, &parsed).await {
        if write_and_normalize(&bytes, &cache_path).await.is_ok() {
            clear_miss_marker(&miss_path).await;
            return encode_png_as_data_url(&cache_path).await.map(Some);
        }
    }

    // All probes failed → sticky null so we don't re-probe for 7 days.
    touch_miss_marker(&miss_path).await;
    Ok(None)
}

// ---------- URL parsing ----------

/// Minimal HTTP(S) URL parse — we only need scheme + host + the base for
/// joining relative paths. Rejects anything that isn't `http://` or
/// `https://` so non-web schemes can't sneak into probe URLs.
#[derive(Debug, Clone)]
struct ParsedUrl {
    /// Always "http" or "https" — lowercased.
    scheme: String,
    /// Host portion (no port). Used by the Google-favicon probe (which keys
    /// off the bare domain) and kept for diagnostics.
    host: String,
    /// `<scheme>://<authority>` — the base for absolute path joins
    /// (apple-touch-icon, favicon). Includes any port the homepage used.
    origin: String,
}

fn parse_http_url(url: &str) -> Option<ParsedUrl> {
    let url = url.trim();
    // L2 — scheme check via ASCII-case-insensitive `starts_with` instead
    // of allocating a lowercase copy + slicing the original. Avoids:
    //   - allocation per call,
    //   - the slice-math fragility (`url.len()` vs `lower.len()` can differ
    //     for Unicode lowercase that changes byte length),
    //   - any non-ASCII edge case (the scheme is RFC-defined ASCII anyway).
    let (scheme, rest) = if url.len() >= 8 && url[..8].eq_ignore_ascii_case("https://") {
        ("https", &url[8..])
    } else if url.len() >= 7 && url[..7].eq_ignore_ascii_case("http://") {
        ("http", &url[7..])
    } else {
        return None;
    };
    if rest.is_empty() {
        return None;
    }
    // `authority` ends at the first `/`, `?`, or `#`.
    let end = rest
        .find(['/', '?', '#'])
        .unwrap_or(rest.len());
    let authority = &rest[..end];
    if authority.is_empty() {
        return None;
    }
    // Strip any userinfo prefix (`user@host`) — we won't preserve creds.
    let host_with_port = authority.rsplit('@').next().unwrap_or(authority);
    if host_with_port.is_empty() {
        return None;
    }
    // Pull the bare host (no port) for diagnostics; `origin` keeps the
    // port if present so absolute joins land on the right listener.
    // IPv6 hosts are bracketed in URLs (`[::1]:8443`); strip the brackets
    // here so the SSRF host filter sees the bare IP literal.
    let host = if host_with_port.starts_with('[') {
        // Find the closing `]`. If missing, the URL is malformed.
        let end = host_with_port.find(']')?;
        &host_with_port[1..end]
    } else {
        host_with_port.split(':').next().unwrap_or(host_with_port)
    };
    if host.is_empty() {
        return None;
    }
    // M2 — SSRF gate. Reject any host that resolves to a non-public IP
    // (loopback, link-local incl. cloud-metadata 169.254/16, RFC1918,
    // CGNAT 100.64/10, ULA fc00::/7, link-local fe80::/10), or sits in
    // a known-internal TLD (`.local`, `.internal`, `localhost`). The
    // initial probe URL is the easiest to reject — redirect hops are
    // covered separately by the custom redirect policy on the client.
    if !is_public_host(host) {
        return None;
    }
    let origin = format!("{}://{}", scheme, host_with_port);
    Some(ParsedUrl {
        scheme: scheme.into(),
        host: host.to_string(),
        origin,
    })
}

/// Return true when `host` is a public IP or hostname suitable for
/// outbound probes. Rejects:
///
/// - **IPv4 literals** that are loopback (`127.0.0.0/8`), private
///   (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`), link-local
///   (`169.254.0.0/16` — includes AWS IMDS `169.254.169.254`),
///   unspecified (`0.0.0.0`), broadcast (`255.255.255.255`), multicast
///   (`224.0.0.0/4`), documentation ranges, and CGNAT (`100.64.0.0/10`).
/// - **IPv6 literals** that are loopback (`::1`), unspecified (`::`),
///   multicast (`ff00::/8`), unique-local (`fc00::/7`), or link-local
///   (`fe80::/10`).
/// - **Hostnames** ending in `.local` (mDNS) or `.internal`, and the
///   literal `localhost`.
///
/// This is a string-based check — it does *not* resolve DNS and so does
/// not protect against an attacker who controls a public-DNS hostname
/// pointing at a private IP. That class of attack is mitigated by the
/// reqwest redirect policy (which re-checks every redirect hop's host)
/// and by the content-type filter (which discards non-`image/*` bodies
/// before any data reaches the renderer).
pub(crate) fn is_public_host(host: &str) -> bool {
    // IPv6 literals are wrapped in `[...]` in URLs but our parser strips
    // those before calling here — we still tolerate either form.
    let trimmed = host
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(host);

    if let Ok(ip) = trimmed.parse::<IpAddr>() {
        return is_public_ip(&ip);
    }

    // Hostnames: reject case-insensitively. `.local` is mDNS,
    // `.internal` is the de-facto private TLD, `localhost` is the magic
    // hostname that resolves to loopback.
    let lower = host.to_ascii_lowercase();
    if lower == "localhost" {
        return false;
    }
    if lower.ends_with(".local") || lower.ends_with(".internal") {
        return false;
    }
    // Reject the empty-label edge case — purely defensive.
    if lower.is_empty() || lower == "." {
        return false;
    }
    true
}

/// IP-level filter — extracted so it can be reused by the redirect
/// policy hook (`reqwest::redirect::Policy::custom`) which inspects
/// each hop's resolved URL.
fn is_public_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            if v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_multicast()
                || v4.is_documentation()
            {
                return false;
            }
            // CGNAT 100.64.0.0/10 is not flagged by `is_private`.
            let octets = v4.octets();
            if octets[0] == 100 && (octets[1] & 0xC0) == 0x40 {
                return false;
            }
            // Benchmarking / 198.18.0.0/15 — RFC 2544.
            if octets[0] == 198 && (octets[1] == 18 || octets[1] == 19) {
                return false;
            }
            true
        }
        IpAddr::V6(v6) => {
            if v6.is_loopback() || v6.is_unspecified() || v6.is_multicast() {
                return false;
            }
            let segments = v6.segments();
            // ULA fc00::/7
            if segments[0] & 0xfe00 == 0xfc00 {
                return false;
            }
            // Link-local fe80::/10
            if segments[0] & 0xffc0 == 0xfe80 {
                return false;
            }
            // IPv4-mapped IPv6 (::ffff:0:0/96) — recurse on the
            // embedded IPv4 so private-IPv4 ranges are caught even
            // when expressed in IPv6 form.
            if segments[0] == 0
                && segments[1] == 0
                && segments[2] == 0
                && segments[3] == 0
                && segments[4] == 0
                && segments[5] == 0xffff
            {
                let v4 = std::net::Ipv4Addr::new(
                    (segments[6] >> 8) as u8,
                    (segments[6] & 0xff) as u8,
                    (segments[7] >> 8) as u8,
                    (segments[7] & 0xff) as u8,
                );
                return is_public_ip(&IpAddr::V4(v4));
            }
            true
        }
    }
}

fn join_absolute(origin: &str, path: &str) -> String {
    if path.starts_with('/') {
        format!("{}{}", origin, path)
    } else {
        format!("{}/{}", origin, path)
    }
}

// ---------- HTTP client ----------

fn build_http_client() -> Result<reqwest::Client, reqwest::Error> {
    // M2 — SSRF defense at the redirect layer. Even if the initial
    // probe URL targets a public host, an attacker-controlled redirect
    // (302 to `http://169.254.169.254/...` or `http://127.0.0.1:8080/`)
    // can pivot the request to a non-public address. The custom policy
    // inspects every hop:
    //   - schemes other than http/https → stop
    //   - non-public host → stop
    //   - >10 hops → stop (matches reqwest's default cap)
    let policy = RedirectPolicy::custom(|attempt| {
        if attempt.previous().len() >= 10 {
            return attempt.error("too many redirects");
        }
        let url = attempt.url();
        let scheme = url.scheme();
        if scheme != "http" && scheme != "https" {
            return attempt.stop();
        }
        let host = match url.host_str() {
            Some(h) => h,
            None => return attempt.stop(),
        };
        if !is_public_host(host) {
            return attempt.stop();
        }
        attempt.follow()
    });
    reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .user_agent(USER_AGENT)
        .redirect(policy)
        .build()
}

/// True if `value` looks like an image content-type. Tolerates the
/// `image/png; charset=binary` shape some servers emit.
fn looks_like_image_content_type(value: &str) -> bool {
    let v = value.trim().to_lowercase();
    v.starts_with("image/")
}

// ---------- Probe: Appcasks (community icon repo) ----------

/// Base URL for App Fair's `appcasks` icon repository — the source Applite and
/// App Fair use. Each cask's icon is a release asset tagged `cask-<token>` with
/// filename `AppIcon.png`. Org is `App-Fair` (capital, hyphen), verified live.
const APPCASKS_BASE: &str =
    "https://github.com/App-Fair/appcasks/releases/download";

async fn probe_appcasks(client: &reqwest::Client, token: &str) -> Option<Vec<u8>> {
    // token is already `validate_cask_token`-checked by the caller, so it's safe
    // to interpolate into the URL (no path-escape / scheme-injection risk).
    let target = format!("{}/cask-{}/AppIcon.png", APPCASKS_BASE, token);
    fetch_image_bytes(client, &target).await
}

// ---------- Probe: Google favicon service ----------

/// Google's public favicon service — returns a PNG for any domain, sized to the
/// requested px. Universal fallback for casks whose homepage exposes no usable
/// icon. Uses the bare host from the (already SSRF-validated) homepage.
async fn probe_google_favicon(client: &reqwest::Client, url: &ParsedUrl) -> Option<Vec<u8>> {
    let target = format!(
        "https://www.google.com/s2/favicons?domain={}&sz={}",
        url.host, ICON_PIXELS
    );
    fetch_image_bytes(client, &target).await
}

// ---------- Probe: apple-touch-icon ----------

async fn probe_apple_touch_icon(client: &reqwest::Client, url: &ParsedUrl) -> Option<Vec<u8>> {
    let target = join_absolute(&url.origin, "/apple-touch-icon.png");
    fetch_image_bytes(client, &target).await
}

// ---------- Probe: og:image ----------

async fn probe_og_image(client: &reqwest::Client, url: &ParsedUrl) -> Option<Vec<u8>> {
    let html = fetch_html(client, &url.origin).await?;
    let og = extract_og_image(&html)?;
    // og:image may be absolute, protocol-relative, or root-relative.
    let absolute = if og.starts_with("http://") || og.starts_with("https://") {
        og
    } else if og.starts_with("//") {
        format!("{}:{}", url.scheme, og)
    } else if og.starts_with('/') {
        format!("{}{}", url.origin, og)
    } else {
        // Bare relative paths are rare for og:image; treat as origin-rooted.
        join_absolute(&url.origin, &og)
    };
    fetch_image_bytes(client, &absolute).await
}

/// Parse a meta-tag `og:image` content value out of HTML. Manual scan —
/// no regex dependency. Tolerates either attribute order
/// (`property=og:image content=...` or vice versa), single or double
/// quotes, and arbitrary whitespace within the tag.
///
/// Returns the first match in document order — the canonical og:image
/// is always near the top of `<head>` for SEO reasons.
pub(crate) fn extract_og_image(html: &str) -> Option<String> {
    // Walk through every `<meta` opener; for each, find its closing `>`,
    // then inspect the attribute window for both `property="og:image"`
    // and `content="..."`.
    let bytes = html.as_bytes();
    let lower = html.to_ascii_lowercase();
    let lower_bytes = lower.as_bytes();
    let mut i = 0;
    while let Some(rel) = find_subsequence(&lower_bytes[i..], b"<meta") {
        let start = i + rel + 5; // position right after `<meta`
        // Boundary check: next char must be whitespace or '>' so we don't
        // match `<metadata` (unlikely but cheap to guard).
        if start >= bytes.len() {
            break;
        }
        let next = bytes[start];
        if !(next == b' ' || next == b'\t' || next == b'\n' || next == b'\r' || next == b'>'
            || next == b'/')
        {
            i = start;
            continue;
        }
        // Find the closing `>`.
        let end = match memchr(b'>', &bytes[start..]) {
            Some(p) => start + p,
            None => break,
        };
        let attrs_window = &html[start..end];
        let attrs_lower = &lower[start..end];

        // Must mention og:image as a property/name AND have content=.
        let is_og = attrs_lower.contains("property=\"og:image\"")
            || attrs_lower.contains("property='og:image'")
            || attrs_lower.contains("name=\"og:image\"")
            || attrs_lower.contains("name='og:image'");
        if is_og {
            if let Some(content) = extract_attr_value(attrs_window, "content") {
                let content = content.trim();
                if !content.is_empty() {
                    return Some(content.to_string());
                }
            }
        }
        i = end + 1;
    }
    None
}

/// Extract `<attr>="value"` or `<attr>='value'` from a tag-attribute
/// window. Case-insensitive on the attribute name. Returns the raw
/// value without quotes.
fn extract_attr_value(attrs: &str, attr: &str) -> Option<String> {
    let lower = attrs.to_ascii_lowercase();
    let needle = format!("{}=", attr.to_ascii_lowercase());
    let mut search_from = 0;
    while let Some(rel) = lower[search_from..].find(&needle) {
        let pos = search_from + rel;
        // Ensure the char preceding `<attr>=` is a word-break (start of
        // window or whitespace) so we don't match `data-content=`.
        if pos > 0 {
            let prev = attrs.as_bytes()[pos - 1];
            if !(prev == b' ' || prev == b'\t' || prev == b'\n' || prev == b'\r') {
                search_from = pos + needle.len();
                continue;
            }
        }
        let after = pos + needle.len();
        if after >= attrs.len() {
            return None;
        }
        let bytes = attrs.as_bytes();
        let quote = bytes[after];
        if quote == b'"' || quote == b'\'' {
            let value_start = after + 1;
            if value_start >= attrs.len() {
                return None;
            }
            let close = match memchr(quote, &bytes[value_start..]) {
                Some(p) => value_start + p,
                None => return None,
            };
            return Some(attrs[value_start..close].to_string());
        }
        // Unquoted value — read until whitespace or end.
        let end = bytes[after..]
            .iter()
            .position(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'>' | b'/'))
            .map(|p| after + p)
            .unwrap_or(attrs.len());
        if after == end {
            return None;
        }
        return Some(attrs[after..end].to_string());
    }
    None
}

/// Locate `needle` in `haystack`. Replicates `memchr`-style early-exit
/// without pulling in the `memchr` crate.
fn memchr(target: u8, haystack: &[u8]) -> Option<usize> {
    haystack.iter().position(|&b| b == target)
}

/// Substring search — used to find `<meta` openings.
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
}

// ---------- Probe: favicon ----------

async fn probe_favicon(client: &reqwest::Client, url: &ParsedUrl) -> Option<Vec<u8>> {
    let target = join_absolute(&url.origin, "/favicon.ico");
    fetch_image_bytes(client, &target).await
}

// ---------- HTTP fetch helpers ----------

/// GET `url`, return the bytes only if the response is 2xx and the
/// content-type starts with `image/`. Any other shape → None. Network
/// errors collapse to None so the cascade can continue to the next step.
async fn fetch_image_bytes(client: &reqwest::Client, url: &str) -> Option<Vec<u8>> {
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let is_image = resp
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(looks_like_image_content_type)
        .unwrap_or(false);
    if !is_image {
        return None;
    }
    let bytes = resp.bytes().await.ok()?;
    if bytes.is_empty() {
        return None;
    }
    Some(bytes.to_vec())
}

/// GET `url`, return up to `HTML_MAX_BYTES` of body as a String. Bails
/// out on non-2xx, non-text, or transport errors — None signals "give up
/// on this probe, try the next one".
async fn fetch_html(client: &reqwest::Client, url: &str) -> Option<String> {
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    // Accept text/html and similar; if the server omits content-type or
    // serves XHTML, we still try to parse — the og:image regex tolerates
    // both.
    let bytes = resp.bytes().await.ok()?;
    let slice = if bytes.len() > HTML_MAX_BYTES {
        &bytes[..HTML_MAX_BYTES]
    } else {
        &bytes[..]
    };
    Some(String::from_utf8_lossy(slice).into_owned())
}

// ---------- Cache layer ----------

async fn read_fresh_cache(cache_path: &Path) -> Result<Option<String>, BrewError> {
    let meta = match tokio::fs::metadata(cache_path).await {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(BrewError::Io {
                message: format!("stat {}: {}", cache_path.display(), e),
            })
        }
    };
    if !is_fresh(&meta, ICON_CACHE_TTL) {
        return Ok(None);
    }
    Some(encode_png_as_data_url(cache_path).await).transpose()
}

fn is_fresh(meta: &std::fs::Metadata, ttl: Duration) -> bool {
    let modified = match meta.modified() {
        Ok(t) => t,
        Err(_) => return false,
    };
    match SystemTime::now().duration_since(modified) {
        Ok(age) => age < ttl,
        Err(_) => true, // Future mtime → treat as fresh.
    }
}

async fn encode_png_as_data_url(path: &Path) -> Result<String, BrewError> {
    let bytes = tokio::fs::read(path).await.map_err(|e| BrewError::Io {
        message: format!("read {}: {}", path.display(), e),
    })?;
    Ok(format!("data:image/png;base64,{}", B64.encode(&bytes)))
}

fn ensure_dir(dir: &Path) -> Result<(), BrewError> {
    if dir.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dir).map_err(|e| BrewError::Io {
        message: format!("create dir {}: {}", dir.display(), e),
    })
}

// ---------- Sticky-null marker ----------

/// Marker file path for a known-no-icon token. Lives next to the would-be
/// PNG with a `.miss` suffix so it can't collide with a real PNG payload.
fn miss_marker_path(cache_path: &Path) -> PathBuf {
    let mut s = cache_path.as_os_str().to_owned();
    s.push(".miss");
    PathBuf::from(s)
}

async fn miss_marker_is_fresh(path: &Path) -> bool {
    let meta = match tokio::fs::metadata(path).await {
        Ok(m) => m,
        Err(_) => return false,
    };
    is_fresh(&meta, ICON_CACHE_TTL)
}

async fn touch_miss_marker(path: &Path) {
    // Best-effort: an IO error here just means we re-probe next time,
    // which is the safe fallback.
    let _ = tokio::fs::write(path, b"").await;
}

async fn clear_miss_marker(path: &Path) {
    let _ = tokio::fs::remove_file(path).await;
}

// ---------- Normalize: sips ----------

/// Write raw bytes to a temp path next to the cache target, then have
/// `sips` convert + resize them into the final cache location. We use a
/// sibling temp file (rather than `std::env::temp_dir`) so the rename is
/// atomic on the same volume.
async fn write_and_normalize(bytes: &[u8], cache_path: &Path) -> Result<(), BrewError> {
    let mut staged = cache_path.as_os_str().to_owned();
    staged.push(".staging");
    let staged = PathBuf::from(staged);

    tokio::fs::write(&staged, bytes).await.map_err(|e| BrewError::Io {
        message: format!("write staging {}: {}", staged.display(), e),
    })?;

    let result = sips_convert_to_png(&staged, cache_path).await;
    // Clean up the staging file regardless of sips success.
    let _ = tokio::fs::remove_file(&staged).await;
    result
}

async fn sips_convert_to_png(input: &Path, output: &Path) -> Result<(), BrewError> {
    let out = Command::new("/usr/bin/sips")
        .args([
            "-s",
            "format",
            "png",
            "-z",
            &ICON_PIXELS.to_string(),
            &ICON_PIXELS.to_string(),
        ])
        .arg(input)
        .arg("--out")
        .arg(output)
        .output()
        .await
        .map_err(|e| BrewError::Io {
            message: format!("spawn sips: {}", e),
        })?;
    if !out.status.success() {
        // Sips failing on a probe payload (corrupt PNG, weird ICO variant)
        // is *expected* for some sites — treat as a probe miss, not a hard
        // error. Returning `Io` here would force the caller to either
        // surface it or special-case-discard, which clutters the cascade.
        return Err(BrewError::Io {
            message: format!(
                "sips failed (exit {:?}) for {}: {}",
                out.status.code(),
                input.display(),
                String::from_utf8_lossy(&out.stderr).trim()
            ),
        });
    }
    Ok(())
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- URL parsing ----------

    #[test]
    fn parse_http_url_accepts_https_with_path() {
        let p = parse_http_url("https://example.com/some/path").expect("parse");
        assert_eq!(p.scheme, "https");
        assert_eq!(p.host, "example.com");
        assert_eq!(p.origin, "https://example.com");
    }

    #[test]
    fn parse_http_url_accepts_http() {
        let p = parse_http_url("http://example.com").expect("parse");
        assert_eq!(p.scheme, "http");
    }

    #[test]
    fn parse_http_url_preserves_port_in_origin() {
        let p = parse_http_url("https://example.com:8443/").expect("parse");
        assert_eq!(p.origin, "https://example.com:8443");
        assert_eq!(p.host, "example.com");
    }

    #[test]
    fn parse_http_url_rejects_non_http_schemes() {
        assert!(parse_http_url("ftp://example.com").is_none());
        assert!(parse_http_url("file:///etc/passwd").is_none());
        assert!(parse_http_url("javascript:alert(1)").is_none());
        assert!(parse_http_url("data:image/png;base64,XXXX").is_none());
        assert!(parse_http_url("").is_none());
        assert!(parse_http_url("just a sentence").is_none());
    }

    #[test]
    fn parse_http_url_strips_userinfo() {
        // We never carry credentials forward into icon-probe requests.
        let p = parse_http_url("https://user:pass@example.com/").expect("parse");
        assert_eq!(p.host, "example.com");
        assert!(!p.origin.contains("user"));
        assert!(!p.origin.contains("pass"));
    }

    #[test]
    fn parse_http_url_rejects_scheme_only() {
        assert!(parse_http_url("https://").is_none());
        assert!(parse_http_url("http://").is_none());
    }

    // ---------- L2: scheme matching is ASCII case-insensitive ----------

    #[test]
    fn parse_http_url_handles_mixed_case_scheme() {
        // The scheme must match case-insensitively per RFC 3986. The
        // pre-fix implementation allocated a lower-case copy of the
        // *whole URL* and indexed back into the original; this caused
        // potential slice-boundary issues with multi-byte chars whose
        // lowercase length differs. The new impl uses
        // `str::eq_ignore_ascii_case` on the scheme prefix only.
        for url in &[
            "HTTPS://example.com",
            "Https://example.com",
            "hTTpS://example.com",
            "HTTP://example.com",
            "Http://example.com",
        ] {
            let p = parse_http_url(url)
                .unwrap_or_else(|| panic!("must parse case-variant {:?}", url));
            assert_eq!(p.host, "example.com");
        }
    }

    #[test]
    fn parse_http_url_handles_multibyte_path_segment_without_panic() {
        // The original impl's slice math could panic on inputs whose
        // lowercase form differs in byte length from the original
        // (rare Unicode codepoints). This test pins the no-panic
        // behaviour for paths with multibyte chars.
        let p = parse_http_url("https://example.com/路径");
        assert!(p.is_some());
        let p = p.unwrap();
        assert_eq!(p.host, "example.com");
    }

    // ---------- M2: SSRF host filter ----------

    #[test]
    fn is_public_host_rejects_loopback_ipv4() {
        assert!(!is_public_host("127.0.0.1"));
        assert!(!is_public_host("127.255.255.255"));
    }

    #[test]
    fn is_public_host_rejects_link_local_ipv4_including_imds() {
        // 169.254.169.254 is AWS Instance Metadata Service — the canonical
        // SSRF target for cloud creds.
        assert!(!is_public_host("169.254.169.254"));
        // GCE metadata uses metadata.google.internal (caught by `.internal`
        // suffix below), but its IP 169.254.169.254 is the same range.
        assert!(!is_public_host("169.254.0.1"));
    }

    #[test]
    fn is_public_host_rejects_rfc1918_ipv4() {
        for addr in &[
            "10.0.0.1",
            "10.255.255.255",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.0.1",
            "192.168.255.255",
        ] {
            assert!(!is_public_host(addr), "expected {} to be rejected", addr);
        }
    }

    #[test]
    fn is_public_host_rejects_cgnat_ipv4() {
        // 100.64.0.0/10 — Carrier-Grade NAT, RFC 6598.
        assert!(!is_public_host("100.64.0.1"));
        assert!(!is_public_host("100.127.255.255"));
        // 100.0.0.0/16 is NOT CGNAT — must still be accepted.
        assert!(is_public_host("100.63.255.255"));
        assert!(is_public_host("100.128.0.1"));
    }

    #[test]
    fn is_public_host_rejects_unspecified_broadcast_multicast() {
        assert!(!is_public_host("0.0.0.0"));
        assert!(!is_public_host("255.255.255.255"));
        assert!(!is_public_host("224.0.0.1"));
        assert!(!is_public_host("239.255.255.255"));
    }

    #[test]
    fn is_public_host_rejects_loopback_ipv6() {
        assert!(!is_public_host("::1"));
        assert!(!is_public_host("[::1]"));
    }

    #[test]
    fn is_public_host_rejects_unique_local_ipv6() {
        // fc00::/7
        assert!(!is_public_host("fc00::1"));
        assert!(!is_public_host("fd12:3456:789a::1"));
    }

    #[test]
    fn is_public_host_rejects_link_local_ipv6() {
        // fe80::/10
        assert!(!is_public_host("fe80::1"));
        assert!(!is_public_host("[fe80::1]"));
    }

    #[test]
    fn is_public_host_rejects_ipv4_mapped_private_ipv6() {
        // ::ffff:10.0.0.1 — IPv4-mapped form of an RFC1918 address.
        assert!(!is_public_host("::ffff:10.0.0.1"));
        assert!(!is_public_host("::ffff:127.0.0.1"));
    }

    #[test]
    fn is_public_host_rejects_internal_tlds() {
        assert!(!is_public_host("localhost"));
        assert!(!is_public_host("LOCALHOST"));
        assert!(!is_public_host("printer.local"));
        assert!(!is_public_host("Printer.Local"));
        assert!(!is_public_host("metadata.google.internal"));
        assert!(!is_public_host("foo.INTERNAL"));
    }

    #[test]
    fn is_public_host_accepts_public_hosts() {
        assert!(is_public_host("example.com"));
        assert!(is_public_host("formulae.brew.sh"));
        assert!(is_public_host("8.8.8.8"));
        assert!(is_public_host("1.1.1.1"));
        assert!(is_public_host("github.com"));
        // 2001:: is the Teredo block — public-routable.
        assert!(is_public_host("2606:4700:4700::1111")); // Cloudflare DNS
    }

    #[test]
    fn parse_http_url_rejects_private_hosts() {
        // Wire test — parse_http_url is the user-facing SSRF gate.
        assert!(parse_http_url("http://127.0.0.1/foo").is_none());
        assert!(parse_http_url("http://169.254.169.254/latest/meta-data/").is_none());
        assert!(parse_http_url("http://10.0.0.1/").is_none());
        assert!(parse_http_url("http://192.168.1.1/").is_none());
        assert!(parse_http_url("http://localhost:8080/").is_none());
        assert!(parse_http_url("http://printer.local/").is_none());
        assert!(parse_http_url("http://metadata.google.internal/").is_none());
        assert!(parse_http_url("https://[::1]/").is_none());
    }

    // ---------- og:image extraction ----------

    #[test]
    fn extract_og_image_finds_canonical_form() {
        let html = r#"<html><head>
            <meta property="og:image" content="https://example.com/cover.png">
            </head></html>"#;
        assert_eq!(
            extract_og_image(html).as_deref(),
            Some("https://example.com/cover.png")
        );
    }

    #[test]
    fn extract_og_image_tolerates_single_quotes() {
        let html =
            r#"<meta property='og:image' content='https://example.com/icon.png'>"#;
        assert_eq!(
            extract_og_image(html).as_deref(),
            Some("https://example.com/icon.png")
        );
    }

    #[test]
    fn extract_og_image_tolerates_reversed_attribute_order() {
        let html =
            r#"<meta content="https://example.com/x.png" property="og:image">"#;
        assert_eq!(
            extract_og_image(html).as_deref(),
            Some("https://example.com/x.png")
        );
    }

    #[test]
    fn extract_og_image_accepts_name_attribute() {
        // Some sites use `name="og:image"` instead of `property=`.
        let html = r#"<meta name="og:image" content="https://example.com/n.png">"#;
        assert_eq!(
            extract_og_image(html).as_deref(),
            Some("https://example.com/n.png")
        );
    }

    #[test]
    fn extract_og_image_returns_none_when_absent() {
        let html =
            r#"<html><head><meta property="og:title" content="X"></head></html>"#;
        assert_eq!(extract_og_image(html), None);
    }

    #[test]
    fn extract_og_image_ignores_unrelated_property() {
        let html = r#"<meta property="twitter:image" content="https://example.com/t.png">"#;
        assert_eq!(extract_og_image(html), None);
    }

    #[test]
    fn extract_og_image_ignores_empty_content() {
        let html = r#"<meta property="og:image" content="">"#;
        assert_eq!(extract_og_image(html), None);
    }

    #[test]
    fn extract_og_image_handles_truncated_html_without_panicking() {
        // Our caller caps HTML at 64KB and may slice mid-tag.
        let html = r#"<meta property="og:image" content="https://exam"#;
        // Unterminated quote → None, but must not panic.
        assert_eq!(extract_og_image(html), None);
    }

    #[test]
    fn extract_og_image_returns_first_match_when_multiple() {
        let html = r#"
            <meta property="og:image" content="https://example.com/first.png">
            <meta property="og:image" content="https://example.com/second.png">
        "#;
        assert_eq!(
            extract_og_image(html).as_deref(),
            Some("https://example.com/first.png")
        );
    }

    #[test]
    fn extract_og_image_does_not_match_metadata_substring() {
        // Word-boundary check on `<meta` — don't false-match `<metadata`.
        let html =
            r#"<metadata property="og:image" content="https://x/y.png"></metadata>"#;
        assert_eq!(extract_og_image(html), None);
    }

    #[test]
    fn extract_og_image_skips_data_content_attribute() {
        // `data-content=` is a different attribute; must not be read as `content=`.
        let html =
            r#"<meta property="og:image" data-content="https://wrong/" content="https://right/x.png">"#;
        assert_eq!(
            extract_og_image(html).as_deref(),
            Some("https://right/x.png")
        );
    }

    // ---------- content-type sniffing ----------

    #[test]
    fn content_type_sniffer_accepts_image_types() {
        assert!(looks_like_image_content_type("image/png"));
        assert!(looks_like_image_content_type("image/x-icon"));
        assert!(looks_like_image_content_type("image/vnd.microsoft.icon"));
        assert!(looks_like_image_content_type("IMAGE/PNG"));
        assert!(looks_like_image_content_type("image/jpeg; charset=binary"));
    }

    #[test]
    fn content_type_sniffer_rejects_non_image_types() {
        assert!(!looks_like_image_content_type("text/html"));
        assert!(!looks_like_image_content_type("application/json"));
        assert!(!looks_like_image_content_type(""));
        assert!(!looks_like_image_content_type("application/octet-stream"));
    }

    // ---------- join_absolute ----------

    #[test]
    fn join_absolute_with_root_relative_path() {
        assert_eq!(
            join_absolute("https://example.com", "/favicon.ico"),
            "https://example.com/favicon.ico"
        );
    }

    #[test]
    fn join_absolute_with_bare_path_adds_slash() {
        assert_eq!(
            join_absolute("https://example.com", "icon.png"),
            "https://example.com/icon.png"
        );
    }

    #[test]
    fn join_absolute_preserves_port() {
        assert_eq!(
            join_absolute("https://example.com:8443", "/a.png"),
            "https://example.com:8443/a.png"
        );
    }

    // ---------- new-source URL construction (Appcasks + Google favicon) ----------

    #[test]
    fn appcasks_url_uses_app_fair_org_and_cask_prefix() {
        // Pins the exact verified-live shape: org `App-Fair`, `cask-<token>`
        // release tag, `AppIcon.png` asset. A regression here = broken icons.
        let token = "iterm2";
        let url = format!("{}/cask-{}/AppIcon.png", APPCASKS_BASE, token);
        assert_eq!(
            url,
            "https://github.com/App-Fair/appcasks/releases/download/cask-iterm2/AppIcon.png"
        );
    }

    #[test]
    fn google_favicon_url_keys_off_bare_host_and_size() {
        let p = parse_http_url("https://slack.com/foo/bar").expect("parse");
        let url = format!(
            "https://www.google.com/s2/favicons?domain={}&sz={}",
            p.host, ICON_PIXELS
        );
        assert_eq!(
            url,
            "https://www.google.com/s2/favicons?domain=slack.com&sz=64"
        );
    }

    // ---------- miss-marker path ----------

    #[test]
    fn miss_marker_path_is_sibling_with_miss_suffix() {
        let cache = Path::new("/tmp/icons/firefox.png");
        assert_eq!(
            miss_marker_path(cache).to_string_lossy(),
            "/tmp/icons/firefox.png.miss"
        );
    }

    #[test]
    fn miss_marker_path_never_collides_with_real_png() {
        // The miss suffix must differ from anything we'd write as a PNG.
        let cache = Path::new("/tmp/icons/x.png");
        let miss = miss_marker_path(cache);
        assert_ne!(miss, cache);
        assert!(miss.to_string_lossy().ends_with(".png.miss"));
    }

    // ---------- cache fast path (filesystem) ----------

    #[tokio::test]
    async fn read_fresh_cache_serves_recent_file_as_data_url() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let cache = tmp.path().join("token.png");
        // Write a tiny "PNG signature" — encode is byte-pass-through.
        tokio::fs::write(&cache, [0x89, b'P', b'N', b'G', 0, 1, 2, 3])
            .await
            .expect("write");
        let out = read_fresh_cache(&cache).await.expect("read");
        let url = out.expect("Some(data url) for fresh cache");
        assert!(url.starts_with("data:image/png;base64,"));
        let body = url.trim_start_matches("data:image/png;base64,");
        let decoded = B64.decode(body).expect("decode");
        assert_eq!(decoded, [0x89, b'P', b'N', b'G', 0, 1, 2, 3]);
    }

    #[tokio::test]
    async fn read_fresh_cache_returns_none_when_file_missing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let missing = tmp.path().join("nope.png");
        assert!(read_fresh_cache(&missing).await.expect("read").is_none());
    }

    #[tokio::test]
    async fn miss_marker_round_trips_through_touch_and_clear() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let marker = tmp.path().join("token.png.miss");
        assert!(!miss_marker_is_fresh(&marker).await);
        touch_miss_marker(&marker).await;
        assert!(miss_marker_is_fresh(&marker).await);
        clear_miss_marker(&marker).await;
        assert!(!miss_marker_is_fresh(&marker).await);
    }

    // ---------- cask_icon_mode gate (Phase 13 — Finding 2) ----------

    /// `Off` → no probe regardless of installed state. Proves the
    /// command short-circuits to `Ok(None)` before any network attempt
    /// or filesystem write when the user opts out entirely.
    #[test]
    fn gate_off_returns_false_regardless_of_installed() {
        assert!(!cask_icon_gate_decision(CaskIconMode::Off, true));
        assert!(!cask_icon_gate_decision(CaskIconMode::Off, false));
    }

    /// `InstalledOnly` + token NOT in installed list → no probe.
    #[test]
    fn gate_installed_only_blocks_uninstalled() {
        assert!(!cask_icon_gate_decision(CaskIconMode::InstalledOnly, false));
    }

    /// `InstalledOnly` + token IN installed list → proceed.
    #[test]
    fn gate_installed_only_allows_installed() {
        assert!(cask_icon_gate_decision(CaskIconMode::InstalledOnly, true));
    }

    /// `All` (default) → always proceed.
    #[test]
    fn gate_all_always_proceeds() {
        assert!(cask_icon_gate_decision(CaskIconMode::All, true));
        assert!(cask_icon_gate_decision(CaskIconMode::All, false));
    }

    /// Defaults: `Settings::default().cask_icon_mode == CaskIconMode::All`,
    /// so first-launch and post-reset users see the historical Phase 8
    /// behaviour. Pins the default so a future re-default doesn't
    /// silently change the icon experience without a settings.md note.
    #[test]
    fn default_mode_is_all() {
        assert_eq!(Settings::default().cask_icon_mode, CaskIconMode::All);
    }
}
