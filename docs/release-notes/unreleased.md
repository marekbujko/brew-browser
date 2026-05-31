## brew-browser (unreleased) — Window behavior fixes

Signed + notarized. macOS 13+, Apple Silicon. Auto-updates via the in-app updater.

> **Staging file.** Rename to `docs/release-notes/<version>.md` when the next version is cut.

### What's new

**The window remembers its size and position.** Resize or move the window, quit, and relaunch — it reopens exactly where and how you left it. Powered by `tauri-plugin-window-state`, which saves geometry on move/resize and on exit, then restores it on the next launch. The previous `1100×720` default is now used only on a true first launch. (#17, #19)

### Bug fixes

**Window stays draggable with Settings open.** Opening Settings dims the main window with a scrim; previously that scrim covered the title bar's drag region, so you couldn't move the window without closing Settings first. The scrim is now inset below the 36px title bar, so the window drags normally with Settings open. (#8, #10)

### Acknowledgments

- @bytepl (Maciej Chojnacki) for reporting the window-state issue (#17) with a clean, reproducible repro.
- @unluckyquote (Nik) for reporting the unmovable-window-with-Settings-open bug (#8).
