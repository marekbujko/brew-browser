## brew-browser 0.6.0 / Brew Browser native 0.2.0 — feature-request batch

Staged for the next signed + notarized release. Tauri remains the macOS 13+ /
Linux build; native remains the macOS 26 SwiftUI build.

> **Staging file.** Add notes here as changes land; rename to
> `docs/release-notes/<version>.md` when the next version is cut.

### What's new

- **Reverse dependencies in package detail.** Package pages can show which
  installed packages require the selected formula.
- **Deprecated / disabled indicators.** Rows and detail panels surface Homebrew's
  deprecation and disablement metadata instead of hiding lifecycle state.
- **Manual vs Dependency Library filters.** Library filtering can separate
  packages the user requested from formulae installed only as dependencies.
- **Per-package disk size.** Package detail can show the on-disk Cellar/Caskroom
  footprint for installed packages.
- **Discover subcategories.** Large Discover categories gain a second-level
  grouping layer so broad buckets are easier to scan.

### Bug fixes

- Documentation and package metadata are aligned with the next release train:
  Tauri `0.6.0`, native `0.2.0`.

### Acknowledgments

- Reddit feature-request feedback that shaped this batch.
