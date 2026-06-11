//! Locate the `brew` binary.

use std::path::PathBuf;

/// Resolve the path to the `brew` binary by checking the known prefixes,
/// then falling back to a PATH lookup.
///
/// Order: macOS Apple Silicon (`/opt/homebrew`), macOS Intel
/// (`/usr/local`), Linuxbrew shared install
/// (`/home/linuxbrew/.linuxbrew`), Linuxbrew per-user install
/// (`~/.linuxbrew`). The prefixes are checked unconditionally rather than
/// `#[cfg]`-gated by OS: the `is_file()` guard makes a macOS prefix
/// harmless on Linux (and vice versa), and an explicit list avoids
/// relying on the PATH fallback — a GUI-launched app inherits a minimal
/// PATH that often omits the Homebrew bin dir.
///
/// Returns `None` if brew can't be located. Callers should map this to
/// `BrewError::BrewNotFound`.
pub fn resolve_brew_path() -> Option<PathBuf> {
    // macOS Apple Silicon default
    let arm = PathBuf::from("/opt/homebrew/bin/brew");
    if arm.is_file() {
        return Some(arm);
    }
    // macOS Intel default
    let intel = PathBuf::from("/usr/local/bin/brew");
    if intel.is_file() {
        return Some(intel);
    }
    // Linuxbrew shared install (default for the official install script).
    let linux_shared = PathBuf::from("/home/linuxbrew/.linuxbrew/bin/brew");
    if linux_shared.is_file() {
        return Some(linux_shared);
    }
    // Linuxbrew per-user install (~/.linuxbrew/bin/brew).
    if let Some(home) = dirs::home_dir() {
        let linux_user = home.join(".linuxbrew").join("bin").join("brew");
        if linux_user.is_file() {
            return Some(linux_user);
        }
    }
    // PATH fallback
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join("brew");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    /// On the dev host (Beast), brew should always resolve. Skipped if not.
    #[test]
    fn resolve_brew_path_on_dev_host_returns_some() {
        match resolve_brew_path() {
            Some(p) => {
                assert!(
                    p.ends_with("brew"),
                    "resolved path should end with `brew`, got {}",
                    p.display()
                );
                assert!(p.is_file(), "resolved path must be a file: {}", p.display());
            }
            None => {
                // CI without brew: tolerable but warn.
                eprintln!("brew not installed; skipping resolve_brew_path positive test");
            }
        }
    }

    #[test]
    fn resolve_brew_path_prefers_apple_silicon_when_present() {
        // We can't override the filesystem in unit tests, but if running
        // on Apple Silicon (and brew is installed at /opt/homebrew), the
        // resolver must return that path verbatim.
        let arm = std::path::PathBuf::from("/opt/homebrew/bin/brew");
        if arm.is_file() {
            assert_eq!(
                resolve_brew_path().as_deref(),
                Some(arm.as_path()),
                "must prefer /opt/homebrew/bin/brew over PATH lookup"
            );
        }
    }

    /// On a Linuxbrew host where the shared install exists, the resolver
    /// must return it verbatim. Mirrors the Apple-Silicon test: it can't
    /// override the filesystem, so it only asserts when the path is
    /// actually present (a no-op on macOS CI / dev hosts).
    #[test]
    fn resolve_brew_path_finds_linuxbrew_shared_when_present() {
        let shared = std::path::PathBuf::from("/home/linuxbrew/.linuxbrew/bin/brew");
        if shared.is_file() {
            // The macOS prefixes won't exist on a Linux box, so the
            // shared Linuxbrew prefix is the first explicit hit.
            assert_eq!(
                resolve_brew_path().as_deref(),
                Some(shared.as_path()),
                "must resolve /home/linuxbrew/.linuxbrew/bin/brew when no macOS prefix exists"
            );
        }
    }
}
