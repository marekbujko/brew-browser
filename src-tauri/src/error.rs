//! Typed error model for brew-browser.
//!
//! `BrewError` is the single error type returned by every Tauri command.
//! It serializes to a tagged JSON shape (`code` discriminator) so the
//! frontend can `switch (err.code)` over a closed union.

use serde::Serialize;
use thiserror::Error;

/// Errors returned by every Tauri command.
///
/// Serializes with `#[serde(tag = "code")]` so the JSON shape on the
/// frontend matches `BrewErrorPayload` in `src/lib/types.ts`.
#[derive(Debug, Error, Serialize, Clone)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum BrewError {
    #[error("brew CLI not found on PATH")]
    BrewNotFound,

    #[error("brew exited with status {exit_code}: {stderr_excerpt}")]
    #[serde(rename_all = "camelCase")]
    BrewExitNonZero {
        command: String,
        exit_code: i32,
        stderr_excerpt: String,
        /// Optional one-sentence human-friendly explanation populated by
        /// `brew::error_patterns::friendlify` when the stderr matches a
        /// known upstream-bug pattern (e.g. the `brew bundle` topo-sort
        /// crash). When `Some`, the frontend toast should render this in
        /// place of the raw exit-code summary; the verbatim stderr is still
        /// available in `stderr_excerpt` for the Activity drawer.
        ///
        /// Serializes to `friendlyMessage` (camelCase) and is omitted from
        /// the JSON payload when `None` so existing frontend type guards
        /// continue to work unchanged.
        #[serde(skip_serializing_if = "Option::is_none")]
        friendly_message: Option<String>,
    },

    #[error("failed to parse brew JSON output: {message}")]
    #[serde(rename_all = "camelCase")]
    JsonParse {
        command: String,
        message: String,
        raw_excerpt: String,
    },

    #[error("I/O error: {message}")]
    Io { message: String },

    #[error("network error fetching {url}: {message}")]
    Network { url: String, message: String },

    #[error("HTTP {status} fetching {url}")]
    HttpStatus { url: String, status: u16 },

    #[error("invalid argument: {message}")]
    InvalidArgument { message: String },

    #[error("job {job_id} not found")]
    #[serde(rename_all = "camelCase")]
    JobNotFound { job_id: String },

    #[error("operation canceled")]
    Canceled,

    #[error("Brewfile {id} not found")]
    BrewfileNotFound { id: String },

    #[error("internal error: {message}")]
    Internal { message: String },

    /// Paranoid mode is on (or settings are corrupt → fail closed).
    /// The `feature` field identifies which outbound command was
    /// rejected, so the UI can route the toast to the right setting.
    #[error("paranoid mode is enabled; outbound feature {feature} is blocked")]
    ParanoidModeBlocked { feature: String },

    /// GitHub's REST API returned a rate-limit response (typically
    /// `403 Forbidden` with `X-RateLimit-Remaining: 0`). `reset_at` is
    /// the unix timestamp from the `X-RateLimit-Reset` header when the
    /// budget refills. **Callers must not retry** — per the §12c/12f
    /// review the only correct response is to surface the limit to the
    /// user with a "Sign in to lift the limit" CTA.
    #[error("github rate limit exceeded; resets at {reset_at}")]
    #[serde(rename_all = "camelCase")]
    GithubRateLimited { reset_at: u64 },

    /// The macOS Keychain refused to store or retrieve a credential.
    /// **No disk fallback** — per the §12e review the OAuth token never
    /// lands on disk. The frontend should surface this with a
    /// "Keychain unavailable" error rather than offering a workaround
    /// that weakens the security posture.
    #[error("keychain unavailable: {message}")]
    KeychainUnavailable { message: String },

    /// An authenticated GitHub action was attempted without a stored
    /// token. The frontend routes this to the "Sign in" CTA in
    /// Settings → GitHub. Emitted by the Phase 12f authed commands
    /// (`github_star`, `github_create_issue`, …) when the keychain
    /// has no `github_access_token` entry.
    #[error("github authentication required")]
    AuthRequired,

    /// An authenticated GitHub action requires an OAuth scope the
    /// stored token doesn't carry. Surfaces the missing scope so the
    /// frontend can prompt a re-sign-in with the expanded scope set.
    /// Emitted by the Phase 12f authed commands when the cached
    /// `KEYCHAIN_ACCOUNT_SCOPES` list omits the required scope (e.g.
    /// `public_repo`), so the frontend can route the user to Settings
    /// → GitHub for a re-grant before any network attempt.
    #[error("github scope required: {scope}")]
    ScopeRequired { scope: String },

    /// Phase 15 — the updater downloaded an artifact whose sha256 did
    /// not match the value declared in the manifest. **Fail closed**:
    /// the .dmg is deleted before this error returns. The cheap hash
    /// check runs *before* the expensive minisign verification so a
    /// corrupted download is rejected without wasting CPU on crypto
    /// over bytes we already know are wrong.
    ///
    /// Currently only constructed by the mock backend in
    /// `commands::updater::tests` — the production plugin path checks
    /// minisign only (see `tauri-plugin-updater::Update::download`'s
    /// `verify_signature` call). When we wire a separate manifest
    /// sha256 check (defense in depth against a wedge between the
    /// manifest fetch and the artifact fetch), this variant is what
    /// it fails closed with.
    #[allow(dead_code)]
    #[error("update artifact hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    /// Phase 15 — minisign verification of the downloaded artifact
    /// failed against the embedded public key. **Fail closed**: the
    /// .dmg is deleted before this error returns. Together with
    /// [`Self::HashMismatch`] these are the two crypto chokepoints
    /// from the §Phase 15 threat-model section; either one firing
    /// aborts the install with no on-disk side effects.
    #[error("update signature verification failed: {message}")]
    SignatureVerificationFailed { message: String },

    /// Phase 15 — the updater was asked to install a version that is
    /// the same as, or older than, the currently-running build. This
    /// is the explicit downgrade-attack defense (the plugin already
    /// refuses semver-older targets; this is defense in depth so a
    /// future plugin behaviour change cannot reopen the hole).
    #[error("update would downgrade {current} to {target}; refusing")]
    DowngradeRejected { current: String, target: String },
}

// ---------- From impls ----------

impl From<std::io::Error> for BrewError {
    fn from(e: std::io::Error) -> Self {
        // All std::io errors map to the generic Io variant; callers that
        // need to discriminate `NotFound` from other kinds (e.g. brew
        // binary missing → `BrewNotFound`) must inspect `kind()` *before*
        // converting. This is the safe default.
        BrewError::Io {
            message: e.to_string(),
        }
    }
}

impl From<serde_json::Error> for BrewError {
    fn from(e: serde_json::Error) -> Self {
        BrewError::JsonParse {
            command: String::new(),
            message: e.to_string(),
            raw_excerpt: String::new(),
        }
    }
}

impl From<reqwest::Error> for BrewError {
    fn from(e: reqwest::Error) -> Self {
        // BUG-3 fix: `e.url()` returns None for some reqwest error variants
        // that fire before the URL is attached (DNS, connect-time, redirect-
        // policy). Falling back to "" makes the frontend toast read
        // `network error fetching : <msg>` — surface a placeholder so the
        // message stays parseable.
        let url = e
            .url()
            .map(|u| u.as_str().to_string())
            .unwrap_or_else(|| "<unknown url>".to_string());
        if let Some(status) = e.status() {
            BrewError::HttpStatus {
                url,
                status: status.as_u16(),
            }
        } else {
            BrewError::Network {
                url,
                message: e.to_string(),
            }
        }
    }
}

/// Truncate a string from the end for inclusion in error excerpts.
pub fn truncate_tail(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let start = s.len() - max;
        // Walk forward to a char boundary.
        let mut idx = start;
        while idx < s.len() && !s.is_char_boundary(idx) {
            idx += 1;
        }
        format!("…{}", &s[idx..])
    }
}

/// Truncate a string from the start for inclusion in error excerpts.
pub fn truncate_head(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let mut idx = max;
        while idx > 0 && !s.is_char_boundary(idx) {
            idx -= 1;
        }
        format!("{}…", &s[..idx])
    }
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    /// Helper: serialize a BrewError and pull out the `code` discriminator.
    fn code_of(err: &BrewError) -> String {
        let v: Value = serde_json::to_value(err).expect("serialize");
        v.get("code")
            .and_then(|c| c.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| panic!("no `code` field in serialized error: {:?}", v))
    }

    // ---- Every variant serializes to the expected `code` string ----

    #[test]
    fn brew_not_found_serializes_to_brew_not_found_code() {
        assert_eq!(code_of(&BrewError::BrewNotFound), "brew_not_found");
    }

    #[test]
    fn brew_exit_non_zero_serializes_with_camel_case_fields() {
        let err = BrewError::BrewExitNonZero {
            command: "brew install foo".into(),
            exit_code: 1,
            stderr_excerpt: "boom".into(),
            friendly_message: None,
        };
        let v: Value = serde_json::to_value(&err).unwrap();
        assert_eq!(v["code"], "brew_exit_non_zero");
        assert_eq!(v["command"], "brew install foo");
        // Critical: must be camelCase for frontend's BrewErrorPayload type.
        assert_eq!(v["exitCode"], 1);
        assert_eq!(v["stderrExcerpt"], "boom");
        assert!(v.get("exit_code").is_none(), "must not emit snake_case `exit_code`");
        assert!(
            v.get("stderr_excerpt").is_none(),
            "must not emit snake_case `stderr_excerpt`"
        );
        // friendly_message is None → must be omitted entirely so the wire
        // shape stays backwards-compatible with existing frontend type
        // guards (`isBrewError` etc.).
        assert!(
            v.get("friendlyMessage").is_none(),
            "friendlyMessage must be omitted when None to preserve wire shape"
        );
        assert!(v.get("friendly_message").is_none());
    }

    #[test]
    fn brew_exit_non_zero_serializes_friendly_message_as_camel_case_when_some() {
        let err = BrewError::BrewExitNonZero {
            command: "brew bundle dump --file=/tmp/x --force".into(),
            exit_code: 1,
            stderr_excerpt: "Error: key not found: \"shivammathur/extensions/imap-uw\"".into(),
            friendly_message: Some("upstream brew bug — not your fault".into()),
        };
        let v: Value = serde_json::to_value(&err).unwrap();
        assert_eq!(v["friendlyMessage"], "upstream brew bug — not your fault");
        assert!(v.get("friendly_message").is_none());
    }

    #[test]
    fn json_parse_serializes_with_camel_case_fields() {
        let err = BrewError::JsonParse {
            command: "brew info foo".into(),
            message: "expected `,`".into(),
            raw_excerpt: "{...}".into(),
        };
        let v: Value = serde_json::to_value(&err).unwrap();
        assert_eq!(v["code"], "json_parse");
        assert_eq!(v["command"], "brew info foo");
        assert_eq!(v["message"], "expected `,`");
        assert_eq!(v["rawExcerpt"], "{...}");
        assert!(v.get("raw_excerpt").is_none());
    }

    #[test]
    fn io_serializes_with_message() {
        let err = BrewError::Io { message: "ENOENT".into() };
        let v: Value = serde_json::to_value(&err).unwrap();
        assert_eq!(v["code"], "io");
        assert_eq!(v["message"], "ENOENT");
    }

    #[test]
    fn network_serializes_with_url_and_message() {
        let err = BrewError::Network {
            url: "https://formulae.brew.sh/...".into(),
            message: "timeout".into(),
        };
        let v: Value = serde_json::to_value(&err).unwrap();
        assert_eq!(v["code"], "network");
        assert_eq!(v["url"], "https://formulae.brew.sh/...");
        assert_eq!(v["message"], "timeout");
    }

    #[test]
    fn http_status_serializes_with_url_and_status() {
        let err = BrewError::HttpStatus {
            url: "https://formulae.brew.sh/foo".into(),
            status: 503,
        };
        let v: Value = serde_json::to_value(&err).unwrap();
        assert_eq!(v["code"], "http_status");
        assert_eq!(v["url"], "https://formulae.brew.sh/foo");
        assert_eq!(v["status"], 503);
    }

    #[test]
    fn invalid_argument_serializes_with_message() {
        let err = BrewError::InvalidArgument {
            message: "package name is empty".into(),
        };
        let v: Value = serde_json::to_value(&err).unwrap();
        assert_eq!(v["code"], "invalid_argument");
        assert_eq!(v["message"], "package name is empty");
    }

    #[test]
    fn job_not_found_uses_camel_case_job_id() {
        let err = BrewError::JobNotFound {
            job_id: "00000000-0000-0000-0000-000000000000".into(),
        };
        let v: Value = serde_json::to_value(&err).unwrap();
        assert_eq!(v["code"], "job_not_found");
        assert_eq!(v["jobId"], "00000000-0000-0000-0000-000000000000");
        assert!(v.get("job_id").is_none(), "must not emit snake_case `job_id`");
    }

    #[test]
    fn canceled_serializes_to_canceled_code() {
        assert_eq!(code_of(&BrewError::Canceled), "canceled");
    }

    #[test]
    fn brewfile_not_found_serializes_with_id() {
        let err = BrewError::BrewfileNotFound { id: "snap1".into() };
        let v: Value = serde_json::to_value(&err).unwrap();
        assert_eq!(v["code"], "brewfile_not_found");
        assert_eq!(v["id"], "snap1");
    }

    #[test]
    fn internal_serializes_with_message() {
        let err = BrewError::Internal {
            message: "boom".into(),
        };
        let v: Value = serde_json::to_value(&err).unwrap();
        assert_eq!(v["code"], "internal");
        assert_eq!(v["message"], "boom");
    }

    #[test]
    fn paranoid_mode_blocked_serializes_with_feature() {
        let err = BrewError::ParanoidModeBlocked {
            feature: "trending_fetch".into(),
        };
        let v: Value = serde_json::to_value(&err).unwrap();
        assert_eq!(v["code"], "paranoid_mode_blocked");
        assert_eq!(v["feature"], "trending_fetch");
    }

    #[test]
    fn github_rate_limited_serializes_with_camel_case_reset_at() {
        let err = BrewError::GithubRateLimited {
            reset_at: 1_700_000_000,
        };
        let v: Value = serde_json::to_value(&err).unwrap();
        assert_eq!(v["code"], "github_rate_limited");
        assert_eq!(v["resetAt"], 1_700_000_000u64);
        assert!(
            v.get("reset_at").is_none(),
            "must not emit snake_case `reset_at`"
        );
    }

    #[test]
    fn keychain_unavailable_serializes_with_message() {
        let err = BrewError::KeychainUnavailable {
            message: "no entry".into(),
        };
        let v: Value = serde_json::to_value(&err).unwrap();
        assert_eq!(v["code"], "keychain_unavailable");
        assert_eq!(v["message"], "no entry");
    }

    #[test]
    fn auth_required_serializes_to_auth_required_code() {
        assert_eq!(code_of(&BrewError::AuthRequired), "auth_required");
    }

    #[test]
    fn scope_required_serializes_with_scope() {
        let err = BrewError::ScopeRequired {
            scope: "public_repo".into(),
        };
        let v: Value = serde_json::to_value(&err).unwrap();
        assert_eq!(v["code"], "scope_required");
        assert_eq!(v["scope"], "public_repo");
    }

    #[test]
    fn hash_mismatch_serializes_with_expected_and_actual() {
        let err = BrewError::HashMismatch {
            expected: "deadbeef".into(),
            actual: "feedface".into(),
        };
        let v: Value = serde_json::to_value(&err).unwrap();
        assert_eq!(v["code"], "hash_mismatch");
        assert_eq!(v["expected"], "deadbeef");
        assert_eq!(v["actual"], "feedface");
    }

    #[test]
    fn signature_verification_failed_serializes_with_message() {
        let err = BrewError::SignatureVerificationFailed {
            message: "bad signature".into(),
        };
        let v: Value = serde_json::to_value(&err).unwrap();
        assert_eq!(v["code"], "signature_verification_failed");
        assert_eq!(v["message"], "bad signature");
    }

    #[test]
    fn downgrade_rejected_serializes_with_current_and_target() {
        let err = BrewError::DowngradeRejected {
            current: "0.3.0".into(),
            target: "0.2.1".into(),
        };
        let v: Value = serde_json::to_value(&err).unwrap();
        assert_eq!(v["code"], "downgrade_rejected");
        assert_eq!(v["current"], "0.3.0");
        assert_eq!(v["target"], "0.2.1");
    }

    // ---- truncate helpers ----

    #[test]
    fn truncate_tail_keeps_short_input_intact() {
        assert_eq!(truncate_tail("abc", 10), "abc");
    }

    #[test]
    fn truncate_tail_truncates_with_ellipsis_prefix() {
        let s = "abcdefghij";
        let out = truncate_tail(s, 4);
        assert!(out.starts_with('…'), "expected ellipsis prefix, got {:?}", out);
        assert!(out.ends_with("ghij"));
    }

    #[test]
    fn truncate_head_keeps_short_input_intact() {
        assert_eq!(truncate_head("abc", 10), "abc");
    }

    #[test]
    fn truncate_head_truncates_with_ellipsis_suffix() {
        let s = "abcdefghij";
        let out = truncate_head(s, 4);
        assert!(out.ends_with('…'), "expected ellipsis suffix, got {:?}", out);
        assert!(out.starts_with("abcd"));
    }

    #[test]
    fn truncate_tail_safe_on_utf8_boundaries() {
        // Two 3-byte chars + ascii. Asking for max=2 must walk to a boundary.
        let s = "日本x";
        let out = truncate_tail(s, 2);
        // We don't pin the exact slice but we must not panic and must be valid utf-8.
        assert!(out.is_char_boundary(0));
        assert!(out.starts_with('…') || out == s);
    }

    #[test]
    fn truncate_head_safe_on_utf8_boundaries() {
        let s = "日本語";
        let out = truncate_head(s, 4);
        // Must not panic; result is valid utf-8.
        assert!(out.ends_with('…') || out == s);
    }

    // ---- From impls ----

    #[test]
    fn io_error_maps_to_brew_error_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "no access");
        let err: BrewError = io_err.into();
        match err {
            BrewError::Io { message } => assert!(message.contains("no access")),
            other => panic!("expected Io, got {:?}", other),
        }
    }

    #[test]
    fn serde_json_error_maps_to_brew_error_json_parse() {
        let bad: Result<serde_json::Value, _> = serde_json::from_str("{not json");
        let err: BrewError = bad.unwrap_err().into();
        match err {
            BrewError::JsonParse { .. } => {}
            other => panic!("expected JsonParse, got {:?}", other),
        }
    }
}
