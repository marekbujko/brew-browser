import SwiftUI
import AppKit
import BrewBrowserKit

@main
struct BrewBrowserApp: App {
    /// When launched as a bare SPM binary (Xcode ⌘R uses the DerivedData
    /// executable, which has no Info.plist), macOS treats the process as a
    /// background/accessory app — the window never comes forward and there's no
    /// Dock icon or menu bar. Forcing `.regular` + activating makes it a normal
    /// foreground app. Harmless when run from the real `.app` bundle (already
    /// `.regular`); this just makes Xcode Run + the debugger usable directly.
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate

    /// The shared app model, owned at the scene root so the scene-level
    /// `.commands` keyboard shortcuts and `ContentView`'s views act on one
    /// instance. (SwiftUI's stock pattern for shared scene + command state.)
    @State private var model = AppModel()

    var body: some Scene {
        WindowGroup {
            ContentView(model: model)
        }
        .windowStyle(.automatic)
        // Native macOS toolbar style — the unified title bar that hosts the
        // Liquid Glass toolbar buttons.
        .windowToolbarStyle(.unified)
        .defaultSize(width: 1100, height: 720)
        // Keep a coherent minimum window size (sidebar + main-pane min +
        // inspector min) while staying freely resizable. Without this, dragging
        // the inspector near the window edge can get grabbed as a window resize.
        .windowResizability(.contentMinSize)
        // Keyboard shortcuts — ⌘K palette, ⌘0–6 section nav, ⌘L drawer, ⌘R
        // refresh, ⌘⇧L theme cycle. Stock `.commands` menus; the bound model is
        // the same one ContentView renders.
        .commands {
            AppCommands(model: model)
        }

        // Native Settings scene — opened by ⌘, the app menu, or the toolbar
        // gear (SettingsLink in ContentView). ⌘, is provided by SwiftUI.
        Settings {
            SettingsView()
        }
    }
}

/// The app's keyboard-shortcut menus, ported from the Tauri global-key handler
/// (`src/routes/+page.svelte:46-128`) and the sidebar ⌘0–6 map
/// (`Sidebar.svelte:35-43`). All stock SwiftUI `Commands` — they surface in the
/// menu bar AND register the shortcuts. Async model calls are wrapped in `Task`.
struct AppCommands: Commands {
    @Bindable var model: AppModel

    var body: some Commands {
        // Replace the stock "About brew-browser" menu item with one that opens
        // the standard AppKit About panel carrying custom credits (the
        // multi-agent build + Apple-native version/Homebrew lines) and a
        // clickable Sponsor link. The standard panel is the most native option —
        // it reads app name/version/icon from the Info.plist automatically; we
        // only inject the credits + a few extra options. Mirrors the Tauri
        // AboutModal (`AboutModal.svelte`). A "Sponsor brew-browser…" item is
        // appended right below so the donate CTA lives in the app menu too.
        CommandGroup(replacing: .appInfo) {
            Button("About brew-browser") { showAboutPanel() }
            Button("Sponsor brew-browser…") {
                if let url = URL(string: AboutInfo.sponsorURL) {
                    NSWorkspace.shared.open(url)
                }
            }
        }

        // A dedicated "Go" menu hosts the section-nav shortcuts (⌘0–6) so they
        // appear together with their key equivalents, like a stock View menu.
        CommandMenu("Go") {
            Button("Dashboard") { model.go(toSectionNumber: 0) }
                .keyboardShortcut("0", modifiers: .command)
            Button("Library")   { model.go(toSectionNumber: 1) }
                .keyboardShortcut("1", modifiers: .command)
            Button("Discover")  { model.go(toSectionNumber: 2) }
                .keyboardShortcut("2", modifiers: .command)
            Button("Trending")  { model.go(toSectionNumber: 3) }
                .keyboardShortcut("3", modifiers: .command)
            Button("Snapshots") { model.go(toSectionNumber: 4) }
                .keyboardShortcut("4", modifiers: .command)
            Button("Services")  { model.go(toSectionNumber: 5) }
                .keyboardShortcut("5", modifiers: .command)
            Button("Activity")  { model.go(toSectionNumber: 6) }
                .keyboardShortcut("6", modifiers: .command)

            Divider()

            // ⌘K command palette.
            Button("Command Palette…") { model.paletteOpen = true }
                .keyboardShortcut("k", modifiers: .command)
        }

        // ⌘R refresh + ⌘L drawer toggle replace the stock toolbar/sidebar
        // command group so they slot into the View menu.
        CommandGroup(after: .toolbar) {
            Button("Refresh") {
                Task { await model.refreshCurrent() }
            }
            .keyboardShortcut("r", modifiers: .command)
            .disabled(model.isLoading)

            Button(model.drawerOpen ? "Hide Activity Drawer" : "Show Activity Drawer") {
                model.toggleDrawer()
            }
            .keyboardShortcut("l", modifiers: .command)

            Button("Cycle Theme") { model.cycleTheme() }
                .keyboardShortcut("l", modifiers: [.command, .shift])
        }
    }
}

/// Shared About / Sponsor strings — the single source of truth for the standard
/// About panel credits and the Sponsor menu item. The sponsor URL matches the
/// toolbar heart + Settings → About (`ContentView.swift`, `SettingsView.swift`).
enum AboutInfo {
    static let sponsorURL = "https://github.com/sponsors/msitarzewski"
    static let repoURL = "https://github.com/msitarzewski/brew-browser"
}

/// Open the standard AppKit About panel with custom credits. App name, version,
/// and icon come from the Info.plist automatically (so a notarized release shows
/// the right version with no extra wiring); we add the multi-agent build credits,
/// the MIT/zero-telemetry affirmation, a repo line, and a clickable Sponsor link
/// — the native parity for the Tauri AboutModal.
@MainActor
private func showAboutPanel() {
    let credits = NSMutableAttributedString()
    let body: [NSAttributedString.Key: Any] = [
        .font: NSFont.systemFont(ofSize: 11),
        .foregroundColor: NSColor.secondaryLabelColor,
    ]
    credits.append(NSAttributedString(
        string: "A native macOS GUI for Homebrew. MIT licensed — zero telemetry, zero accounts. Every outbound network call is documented in Settings → Network.\n\n",
        attributes: body))
    credits.append(NSAttributedString(
        string: "Built with Agency Agents — the multi-agent toolkit that orchestrated the waves — powered by Claude Code running Opus. Thanks to the Homebrew project, every formula and cask maintainer, and Apple's SwiftUI.\n\n",
        attributes: body))

    // Clickable links (Sponsor + repo) — NSAttributedString `.link` makes the
    // About panel render them as real, clickable links.
    let linkBase = body.merging([.foregroundColor: NSColor.linkColor]) { _, new in new }
    if let sponsor = URL(string: AboutInfo.sponsorURL) {
        credits.append(NSAttributedString(
            string: "Sponsor brew-browser",
            attributes: linkBase.merging([.link: sponsor]) { _, new in new }))
        credits.append(NSAttributedString(string: "   ", attributes: body))
    }
    if let repo = URL(string: AboutInfo.repoURL) {
        credits.append(NSAttributedString(
            string: "GitHub repo",
            attributes: linkBase.merging([.link: repo]) { _, new in new }))
    }

    NSApp.orderFrontStandardAboutPanel(options: [.credits: credits])
    NSApp.activate(ignoringOtherApps: true)
}

/// Promotes a bare/unbundled launch (Xcode ⌘R) to a normal foreground app.
final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        if NSApp.activationPolicy() != .regular {
            NSApp.setActivationPolicy(.regular)
        }
        NSApp.activate(ignoringOtherApps: true)
        // Real Dock/⌘-Tab icon, loaded from BrewBrowserKit's bundle — works for
        // the bare binary (Xcode ⌘R) too, not just the build-app.sh .app.
        applyDockIcon()
        // Window size/position persistence (Bundle F — parity with Tauri PR #17).
        // SwiftUI's WindowGroup leans on macOS state restoration, which doesn't
        // reliably persist the frame for the SPM/bare-binary run paths (and is
        // skipped after a force-quit). Setting an NSWindow `frameAutosaveName`
        // makes AppKit save+restore the frame to UserDefaults independent of
        // state restoration, so size+position survive relaunch in every run path.
        persistWindowFrame()
    }

    /// Give the main window a `frameAutosaveName` so AppKit persists its size +
    /// position across launches. The SwiftUI window isn't attached at
    /// `applicationDidFinishLaunching`, so retry on the next run-loop turn until
    /// it exists (bounded), then set the autosave name once.
    @MainActor
    private func persistWindowFrame(attempt: Int = 0) {
        guard let window = NSApp.windows.first(where: { $0.canBecomeMain }) else {
            // Window not created yet — try again shortly (cap the retries so a
            // truly window-less run, e.g. tests, doesn't spin).
            if attempt < 20 {
                Task { @MainActor in
                    try? await Task.sleep(for: .milliseconds(50))
                    self.persistWindowFrame(attempt: attempt + 1)
                }
            }
            return
        }
        if window.frameAutosaveName.isEmpty {
            window.setFrameAutosaveName("BrewBrowserMainWindow")
        }
    }
}
