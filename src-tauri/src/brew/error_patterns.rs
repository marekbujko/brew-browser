//! Friendly error mapping for known upstream Homebrew bugs.
//!
//! Some `brew` failures are not user error — they're upstream Homebrew Ruby
//! bugs (e.g. `bundle/brew.rb:686 Homebrew::Bundle::Brew::Topo#tsort_each_child`
//! exploding on certain tap-formula combinations). Surfacing the raw
//! `BrewExitNonZero { code, stderr_excerpt }` to the toast layer is technically
//! honest but unhelpful: the user has no way to know "this is a brew bug, not
//! your Brewfile."
//!
//! This module pattern-matches the captured stderr against a small, hand-
//! curated catalog and returns a one-sentence friendly message when a known
//! pattern hits. The full stderr is still preserved in the original error
//! variant and shown verbatim in the Activity drawer — `friendlify` only
//! drives the toast.
//!
//! **Polish, not a rules engine.** Keep the catalog tiny and conservative.
//! When in doubt, return `None` and let the generic error surface unchanged.

/// Return a friendly one-sentence message if `stderr_excerpt` matches a known
/// upstream-bug pattern for the given `command`. Returns `None` when nothing
/// matches — callers should fall back to the generic error rendering.
///
/// `command` is the user-facing form (e.g. `"brew bundle dump --file=… --force"`).
/// We use it to gate patterns to specific subcommands (bundle-only patterns
/// shouldn't fire on `brew install`).
///
/// `stderr_excerpt` may be the bounded ring snapshot from `run_brew_streaming`
/// — up to ~4 KB — so pattern checks must be cheap substring scans, not
/// regex. UTF-8 multibyte content is safe: `str::contains` operates on bytes
/// but `&str` itself is always valid UTF-8.
pub fn friendlify(stderr_excerpt: &str, command: &str) -> Option<String> {
    let is_bundle = command.contains("bundle dump") || command.contains("bundle install");

    // Pattern 1 — `brew bundle` topo-sort key-not-found.
    //
    // Real failure shape (verified 2026-05-23, brew 5.1.13):
    //   Error: key not found: "shivammathur/extensions/imap-uw"
    //   /opt/homebrew/Library/Homebrew/bundle/brew.rb:686:in
    //     'Homebrew::Bundle::Brew::Topo#tsort_each_child'
    //
    // This is an upstream Homebrew Ruby bug, reproducible outside our app.
    // Both substrings must be present so we don't false-positive on unrelated
    // "key not found" messages.
    if is_bundle
        && stderr_excerpt.contains("key not found:")
        && stderr_excerpt.contains("Homebrew::Bundle::Brew::Topo")
    {
        return Some(
            "Homebrew's `brew bundle` hit an internal topo-sort error on one of \
             your installed formulae. This is an upstream Homebrew bug, not a \
             brew-browser issue. Try `brew untap` on a recently-added tap, or \
             see the full output in Activity."
                .to_string(),
        );
    }

    // Pattern 2 — Homebrew explicitly asks the user to report the issue.
    //
    // Real failure shape: brew prints
    //   Please report this issue:
    //     https://docs.brew.sh/Troubleshooting
    // on any internal Ruby exception. Surfacing the friendly hint nudges the
    // user toward Homebrew's troubleshooting docs instead of blaming us.
    if stderr_excerpt.contains("Please report this issue:")
        && stderr_excerpt.contains("docs.brew.sh/Troubleshooting")
    {
        return Some(
            "Homebrew reported an internal error and asked you to report it \
             upstream. See the full output in Activity, and visit \
             https://docs.brew.sh/Troubleshooting for next steps."
                .to_string(),
        );
    }

    // Pattern 3 — `brew services start|stop|restart` failed because the
    // service plist lives in a launchd domain that the current user
    // session does not own (user vs. system launchd domain, a common
    // Sonoma+ failure mode). The stderr shape we key on:
    //
    //   Could not find service "ollama" in domain for current user
    //   (gui/501/...). Run `launchctl bootstrap` with the right
    //   domain, or move the plist to ~/Library/LaunchAgents.
    //
    // We match on the first line of the canonical upstream message.
    // The command is matched on `services` so we don't fire on every
    // unrelated "domain" mention.
    if command.contains("services")
        && (stderr_excerpt.contains("Could not find service")
            || stderr_excerpt.contains("not found in domain"))
    {
        return Some(
            "brew could not find this service in the current launchd \
             domain. The plist is likely registered for a different user \
             or session. Try `brew services restart`, or move the plist to \
             ~/Library/LaunchAgents and run `launchctl bootstrap gui/$UID`."
                .to_string(),
        );
    }

    // Pattern 4 — another brew process holds the lock. Environmental, not a
    // bug in either brew or us: a concurrent `brew` (often a background
    // `brew upgrade` or a second app) is running. Real shape:
    //   Error: A `brew upgrade` process has already locked
    //   /opt/homebrew/Cellar/ca-certificates. Please wait for it to finish…
    if stderr_excerpt.contains("has already locked")
        || stderr_excerpt.contains("Please wait for it to finish or terminate it")
    {
        return Some(
            "Another Homebrew process is already running and holds the lock. \
             Wait for it to finish (or quit the other process) and try again — \
             this isn't a brew-browser problem."
                .to_string(),
        );
    }

    // Pattern 5 — a cask installer/remover invoked sudo, but brew-browser runs
    // brew without an interactive terminal, so sudo cannot prompt for an admin
    // password. Real Docker Desktop shape:
    //
    //   sudo: a terminal is required to read the password; either use the -S
    //   option to read from standard input or configure an askpass helper
    //   sudo: a password is required
    //
    // This is expected for some casks that install or remove privileged helper
    // tools. It is not warning-only and not a brew-browser bug; the user needs
    // to run the same brew command in Terminal, where macOS/sudo can prompt.
    if sudo_password_prompt_required(stderr_excerpt) {
        if let Some(token) = sudo_cask_token(stderr_excerpt, command) {
            return Some(format!(
                "The cask `{token}` needs an administrator password, but \
                 brew-browser runs brew without an interactive terminal so sudo \
                 cannot prompt here. Run this in Terminal: `brew upgrade --cask \
                 {token}`, then refresh brew-browser."
            ));
        }
        return Some(
            "This Homebrew cask needs an administrator password, but \
             brew-browser runs brew without an interactive terminal so sudo \
             cannot prompt here. Run the cask upgrade in Terminal, then refresh \
             brew-browser."
                .to_string(),
        );
    }

    None
}

fn sudo_password_prompt_required(stderr_excerpt: &str) -> bool {
    stderr_excerpt.contains("sudo: a terminal is required to read the password")
        || stderr_excerpt.contains("sudo: a password is required")
        || stderr_excerpt.contains("sudo: no tty present and no askpass program specified")
}

fn sudo_cask_token(stderr_excerpt: &str, command: &str) -> Option<String> {
    token_from_brew_error(stderr_excerpt).or_else(|| token_from_command(command))
}

fn token_from_brew_error(stderr_excerpt: &str) -> Option<String> {
    stderr_excerpt.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix("Error: ")?;
        let token = rest.split(':').next()?.trim();
        valid_cask_token(token).then(|| token.to_string())
    })
}

fn token_from_command(command: &str) -> Option<String> {
    let mut saw_action = false;
    for part in command.split_whitespace() {
        if !saw_action {
            saw_action = part == "upgrade" || part == "install" || part == "reinstall";
            continue;
        }
        if part.starts_with('-') {
            continue;
        }
        return valid_cask_token(part).then(|| part.to_string());
    }
    None
}

fn valid_cask_token(token: &str) -> bool {
    !token.is_empty()
        && token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '+' | '@'))
}

/// True when a non-zero `brew upgrade`/`install` exit carries ONLY non-fatal
/// warnings — the kind brew escalates to exit code 1 even though it actually
/// did the work. These are the dominant source of bogus "Upgrade-all failed"
/// reports (post-install warnings, link conflicts on a single keg, kegs that
/// were installed-but-not-linked because another version is linked, an
/// already-present binary that blocks the symlink). In every one of these
/// cases the upgrade succeeded for the rest of the set; surfacing a red
/// "failed" toast with a file-an-issue button is wrong.
///
/// Conservative by construction: returns `true` only when (a) the command is
/// an upgrade/install, (b) at least one known non-fatal marker is present, and
/// (c) no hard-fatal marker is present. Anything outside that falls through to
/// the normal failure surface, so we never hide a real failure that lacks a
/// recognized warning marker.
pub fn upgrade_warnings_only(stderr_excerpt: &str, command: &str) -> bool {
    let is_upgrade_or_install = command.contains("upgrade") || command.contains("install");
    if !is_upgrade_or_install {
        return false;
    }

    // Hard-fatal signatures: if any of these is present, it's a real failure
    // regardless of accompanying warnings — never downgrade to "warnings only".
    const FATAL: &[&str] = &[
        "No available formula",
        "No such file or directory",
        "Permission denied",
        "Failed to download",
        "Download failed",
        "Could not resolve host",
        "has already locked", // concurrent lock — handled as an env. failure
        "sudo: a terminal is required to read the password",
        "sudo: a password is required",
        "sudo: no tty present and no askpass program specified",
        "Please report this issue:",
        "Homebrew::Bundle::Brew::Topo",
        "checksum does not match",
        "SHA256 mismatch",
    ];
    if FATAL.iter().any(|f| stderr_excerpt.contains(f)) {
        return false;
    }

    // Known non-fatal warning markers. brew exits 1 on these, but the work is
    // done. (The `brew link` "Error:" line is intentionally treated as
    // non-fatal here — it's a link conflict on one keg, not a failed upgrade.)
    const NONFATAL: &[&str] = &[
        "post-install step did not complete successfully",
        "not linked because",
        "already linked",
        "skipping link",
        "already a Binary at",
        "`brew link` step did not complete successfully",
    ];
    NONFATAL.iter().any(|w| stderr_excerpt.contains(w))
}

/// True when `command` is a `brew doctor` invocation. `brew doctor` exits 1
/// whenever it finds advisories ("Warning: ..."), but those are diagnostics to
/// read — not a job failure. The streaming layer treats a non-zero doctor exit
/// as effective-success so it surfaces the advisory output in the Activity log
/// instead of throwing a "doctor failed, file an issue" error. A clean run
/// (exit 0, "Your system is ready to brew.") is unaffected.
pub fn doctor_advisory_exit(command: &str) -> bool {
    let mut parts = command.split_whitespace();
    parts.next() == Some("brew") && parts.next() == Some("doctor")
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    /// Captured from a real `brew bundle dump --force` run on a machine with
    /// the `shivammathur/extensions/imap-uw` tap-formula installed.
    const REAL_TOPO_STDERR: &str = r#"Error: key not found: "shivammathur/extensions/imap-uw"
/opt/homebrew/Library/Homebrew/bundle/brew.rb:686:in 'Homebrew::Bundle::Brew::Topo#tsort_each_child'
/opt/homebrew/Library/Homebrew/vendor/portable-ruby/3.3.5/lib/ruby/3.3.0/tsort.rb:413:in 'block in each_strongly_connected_component'
"#;

    const REAL_REPORT_STDERR: &str = r#"Error: undefined method 'foo' for nil:NilClass
Please report this issue:
  https://docs.brew.sh/Troubleshooting

These open issues may also help:
  https://github.com/Homebrew/brew/issues
"#;

    const REAL_SUDO_PASSWORD_STDERR: &str = r#"==> Removing launchctl service `com.docker.vmnetd`
sudo: a terminal is required to read the password; either use the -S option to read from standard input or configure an askpass helper
sudo: a password is required
Error: docker-desktop: Failure while executing; `/usr/bin/sudo -E -- /usr/bin/xargs -0 -- /bin/rm -r -f --` exited with 1.
"#;

    // ---- positive matches ----

    #[test]
    fn topo_sort_pattern_matches_on_bundle_dump() {
        let msg = friendlify(REAL_TOPO_STDERR, "brew bundle dump --file=/tmp/x --force")
            .expect("topo-sort pattern should match real stderr on `bundle dump`");
        assert!(
            msg.contains("upstream Homebrew bug"),
            "friendly msg should call out upstream bug; got {msg:?}"
        );
        assert!(
            msg.contains("brew untap"),
            "friendly msg should hint at brew untap as a workaround; got {msg:?}"
        );
    }

    #[test]
    fn topo_sort_pattern_matches_on_bundle_install() {
        // Same class of bug; bundle install can hit the same topo path.
        let msg = friendlify(REAL_TOPO_STDERR, "brew bundle install --file=/tmp/x");
        assert!(
            msg.is_some(),
            "topo-sort pattern should match on `bundle install`"
        );
    }

    #[test]
    fn please_report_pattern_matches_when_both_substrings_present() {
        let msg = friendlify(REAL_REPORT_STDERR, "brew bundle dump --file=/tmp/x --force")
            .expect("please-report pattern should match real stderr");
        assert!(
            msg.contains("docs.brew.sh/Troubleshooting"),
            "friendly msg should link to brew troubleshooting docs; got {msg:?}"
        );
    }

    #[test]
    fn sudo_password_prompt_pattern_matches() {
        let msg = friendlify(REAL_SUDO_PASSWORD_STDERR, "brew upgrade docker-desktop")
            .expect("sudo password pattern should match real cask stderr");
        assert!(
            msg.contains("administrator password")
                && msg.contains("Terminal")
                && msg.contains("docker-desktop")
                && msg.contains("brew upgrade --cask docker-desktop"),
            "friendly msg should explain that Terminal/admin prompt is required; got {msg:?}"
        );
    }

    #[test]
    fn sudo_password_prompt_falls_back_to_command_token() {
        let stderr = "sudo: a password is required\n";
        let msg = friendlify(stderr, "brew upgrade --cask docker-desktop")
            .expect("sudo password pattern should match command-only stderr");
        assert!(msg.contains("brew upgrade --cask docker-desktop"), "{msg}");
    }

    // ---- non-matches ----

    #[test]
    fn returns_none_for_generic_install_failure() {
        let stderr = "Error: No available formula with the name \"definitely-not-a-real-pkg\".\n";
        assert!(
            friendlify(stderr, "brew install definitely-not-a-real-pkg").is_none(),
            "unknown-formula errors must fall through to the generic surface"
        );
    }

    #[test]
    fn topo_pattern_does_not_fire_on_non_bundle_command() {
        // Even if the stderr happened to match, the topo pattern is gated to
        // bundle subcommands — we don't want to overreach onto `brew install`
        // stderr that mentions "Topo" for unrelated reasons.
        assert!(
            friendlify(REAL_TOPO_STDERR, "brew install foo").is_none(),
            "topo pattern must not fire on non-bundle commands"
        );
    }

    #[test]
    fn returns_none_when_only_one_topo_substring_present() {
        // "key not found:" without the Topo frame is too generic to claim.
        let stderr = "Error: key not found: \"some/other/thing\"\n";
        assert!(
            friendlify(stderr, "brew bundle dump --file=/tmp/x --force").is_none(),
            "must require both substrings to avoid false positives"
        );
    }

    // ---- edge cases ----

    #[test]
    fn handles_very_long_stderr_without_panic() {
        // Simulate the bounded ring at its 4 KB cap: noise lines + a real
        // match embedded somewhere in the middle.
        let noise = "noise line that is reasonably long and repeats\n".repeat(80);
        let mut stderr = String::new();
        stderr.push_str(&noise);
        stderr.push_str(REAL_TOPO_STDERR);
        stderr.push_str(&noise);

        let msg = friendlify(&stderr, "brew bundle dump --file=/tmp/x --force");
        assert!(
            msg.is_some(),
            "should still find the pattern when sandwiched in noise"
        );
    }

    #[test]
    fn multibyte_stderr_is_safe() {
        // Make sure substring scan doesn't choke on non-ASCII content. The
        // `str::contains` API is char-safe, but pin a regression test anyway.
        let stderr = "ログ: 日本語のエラーメッセージ\n\
                      Error: key not found: \"foo/bar/baz\"\n\
                      Homebrew::Bundle::Brew::Topo crashed\n\
                      終わり\n";
        let msg = friendlify(stderr, "brew bundle dump --file=/tmp/x --force");
        assert!(
            msg.is_some(),
            "multibyte content must not prevent pattern detection"
        );
    }

    #[test]
    fn empty_inputs_return_none() {
        assert!(friendlify("", "").is_none());
        assert!(friendlify("", "brew bundle dump").is_none());
        assert!(friendlify("Error: anything", "").is_none());
    }

    // ---- launchd domain ----

    const REAL_LAUNCHD_STDERR: &str = "Error: Could not find service \"ollama\" in domain for current user (gui/501/16).\nTry running launchctl bootstrap under the right domain, or move the plist to ~/Library/LaunchAgents.\n";

    #[test]
    fn launchd_domain_pattern_matches_on_services() {
        let msg = friendlify(REAL_LAUNCHD_STDERR, "brew services start ollama")
            .expect("launchd pattern should match real services-start stderr");
        assert!(
            msg.contains("launchd") && msg.contains("~/Library/LaunchAgents"),
            "friendly msg should mention launchd and the plist location; got {msg:?}"
        );
    }

    #[test]
    fn launchd_pattern_does_not_fire_on_unrelated_command() {
        // The substring 'Could not find service' alone must not trigger the
        // hint; the gate is command contains services.
        assert!(
            friendlify(REAL_LAUNCHD_STDERR, "brew install ollama").is_none(),
            "launchd pattern must not fire outside services commands"
        );
    }

    #[test]
    fn launchd_pattern_does_not_fire_on_empty_stderr() {
        assert!(
            friendlify("", "brew services start ollama").is_none(),
            "empty stderr must fall through"
        );
    }

    // ---- concurrent lock (Pattern 4) ----

    #[test]
    fn locked_pattern_matches() {
        // Real shape from issue #52.
        let stderr = "Error: A `brew upgrade` process has already locked /opt/homebrew/Cellar/ca-certificates.\nPlease wait for it to finish or terminate it to continue.\n";
        let msg = friendlify(stderr, "brew upgrade").expect("locked pattern should match");
        assert!(
            msg.contains("Another Homebrew process"),
            "friendly msg should call out the concurrent process; got {msg:?}"
        );
    }

    // ---- upgrade_warnings_only ----

    // Real stderr tails captured from the bogus "Upgrade-all failed" reports
    // (#28, #53, #55). Each upgraded the set fine but exited 1 on a warning.
    const WARN_POSTINSTALL: &str = "Warning: ffmpeg@7 was installed but not linked because ffmpeg is already linked.\nWarning: The post-install step did not complete successfully\n";
    const WARN_SKIP_LINK: &str = "Warning: The post-install step did not complete successfully\nWarning: It seems there is already a Binary at '/opt/homebrew/bin/codex' from formula codex; skipping link.\n";
    const WARN_LINK_STEP: &str = "Error: The `brew link` step did not complete successfully\n";

    #[test]
    fn upgrade_warnings_only_matches_postinstall_warning() {
        assert!(upgrade_warnings_only(WARN_POSTINSTALL, "brew upgrade"));
    }

    #[test]
    fn upgrade_warnings_only_matches_skip_link() {
        assert!(upgrade_warnings_only(WARN_SKIP_LINK, "brew upgrade"));
    }

    #[test]
    fn upgrade_warnings_only_matches_link_step_error() {
        // The `brew link` "Error:" line is a non-fatal link conflict.
        assert!(upgrade_warnings_only(WARN_LINK_STEP, "brew upgrade git"));
    }

    #[test]
    fn upgrade_warnings_only_false_on_real_failure() {
        // A genuine download failure must NOT be downgraded to warnings even
        // if a post-install warning rode along earlier in the output.
        let mixed = "Warning: The post-install step did not complete successfully\nError: Failed to download resource \"foo\"\n";
        assert!(!upgrade_warnings_only(mixed, "brew upgrade"));
    }

    #[test]
    fn upgrade_warnings_only_false_on_lock() {
        let locked = "Error: A `brew upgrade` process has already locked /opt/homebrew/Cellar/x.\n";
        assert!(!upgrade_warnings_only(locked, "brew upgrade"));
    }

    #[test]
    fn upgrade_warnings_only_false_on_sudo_password_prompt() {
        let mixed = format!(
            "Warning: The post-install step did not complete successfully\n{REAL_SUDO_PASSWORD_STDERR}"
        );
        assert!(!upgrade_warnings_only(
            &mixed,
            "brew upgrade docker-desktop"
        ));
    }

    #[test]
    fn upgrade_warnings_only_gated_to_upgrade_install() {
        // Same warning text on a non-upgrade command must not classify.
        assert!(!upgrade_warnings_only(
            WARN_POSTINSTALL,
            "brew services list"
        ));
    }

    #[test]
    fn upgrade_warnings_only_false_when_no_marker() {
        assert!(!upgrade_warnings_only(
            "✔︎ Bottle foo (1.0)\n",
            "brew upgrade"
        ));
    }

    // ---- issue #80: doctor advisory exit ----

    #[test]
    fn doctor_advisory_exit_matches_brew_doctor() {
        assert!(doctor_advisory_exit("brew doctor"));
    }

    #[test]
    fn doctor_advisory_exit_rejects_other_commands() {
        assert!(!doctor_advisory_exit("brew upgrade"));
        assert!(!doctor_advisory_exit("brew cleanup --prune=all --scrub"));
        assert!(!doctor_advisory_exit("brew install doctor")); // installing a formula named "doctor"
        assert!(!doctor_advisory_exit(""));
        assert!(!doctor_advisory_exit("doctor"));
    }

    // ---- robustness / fuzz (no panic on adversarial input) ----

    #[test]
    fn classifiers_never_panic_on_adversarial_input() {
        // brew stderr is semi-trusted; harden the substring scanners against
        // empty, huge, multibyte, control-char, and near-miss inputs.
        let mut inputs: Vec<String> = vec![
            String::new(),
            " ".into(),
            "\n\0\t".into(),
            "Error:".into(),
            "Warning:".into(),
            "post-install step did not complete successfully".into(),
            "has already locked".into(),
        ];
        inputs.push("A".repeat(200_000));
        inputs.push("日本語".repeat(20_000));
        inputs.push((0u8..=255).map(|b| b as char).collect()); // all Latin-1 code points
        inputs.push("\u{0}\u{1}\u{2}\u{7f}control".into());

        let commands = [
            "",
            "brew upgrade",
            "brew install x",
            "brew services start y",
            "brew bundle dump",
        ];
        for inp in &inputs {
            for cmd in commands {
                // Sole assertion is "returns without panicking".
                let _ = friendlify(inp, cmd);
                let _ = upgrade_warnings_only(inp, cmd);
            }
        }
    }
}
