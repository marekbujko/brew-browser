import Foundation
import Sparkle
import Combine

/// Wraps Sparkle's `SPUStandardUpdaterController` so the SwiftUI views can read
/// updater state reactively. This is the native parity for the Tauri updater
/// (`src/lib/stores/updater.svelte`, `src-tauri/src/commands/updater.rs`): the
/// "Check for Updates…" menu item, the Settings → Updates tab, and the titlebar
/// "update available" pill all read this single instance, owned at the scene
/// root in `BrewBrowserApp`.
///
/// Sparkle owns the actual check/download/install/relaunch flow and shows its
/// own standard UI (the macOS-native updater alert + progress sheet), so we only
/// bridge a few KVO-published properties into `@Observable` reactivity and expose
/// the user-facing controls. `startingUpdater: true` means Sparkle schedules its
/// background checks per the Info.plist `SUEnableAutomaticChecks` /
/// `SUScheduledCheckInterval` keys from launch.
///
/// The feed URL + public EdDSA key live in the app's Info.plist
/// (`SUFeedURL` = the public `brew-browser.zerologic.com/appcast.xml`,
/// `SUPublicEDKey`), so there's no host configuration here — and no private host
/// name anywhere.
@Observable
@MainActor
public final class UpdaterController {
    /// Sparkle's standard controller — creates the updater + the standard user
    /// driver (the native update alert/progress UI). Held for the app's lifetime.
    /// `nil` when running UNBUNDLED (Xcode Run / `swift run`): Bundle.main is the
    /// bare build dir ("Debug") with no Info.plist + no embedded Sparkle.framework,
    /// so the updater can't start — we stay inert instead of throwing Sparkle's
    /// "updater failed to start" alert. Real updates require the assembled .app.
    private let controller: SPUStandardUpdaterController?

    /// True only when launched as the assembled .app (Info.plist carries the
    /// Sparkle keys). False from an Xcode/`swift run` binary — the updater UI is
    /// then disabled rather than failing loudly.
    public let isSupported: Bool

    /// The `SPUUpdaterDelegate`. Sparkle takes the delegate at controller-init
    /// time (`SPUUpdater.delegate` is read-only), and it can't be `self` before
    /// `super.init`, so it's a small standalone object that calls back into us.
    private let delegate = UpdaterDelegate()

    /// KVO → `@Observable` bridge. Mirrors Sparkle's documented
    /// `CheckForUpdatesViewModel` pattern: the updater publishes
    /// `canCheckForUpdates` whenever a check starts/finishes, and we surface it
    /// so the menu item + "Check now" button disable while a check is in flight.
    public private(set) var canCheckForUpdates = false

    /// True once Sparkle has found a valid update (set via the delegate callback).
    /// Drives the titlebar pill. Cleared the moment the user opens the updater
    /// (`checkForUpdates()`), since Sparkle's own UI then owns the flow.
    public private(set) var updateAvailable = false

    /// Combine subscription keeping `canCheckForUpdates` in sync. Retained for the
    /// controller's lifetime (the app's lifetime).
    private var cancellable: AnyCancellable?

    public init() {
        // Only wire Sparkle when launched as the real .app — the Info.plist's
        // SUFeedURL is the tell (absent from an unbundled Xcode/`swift run`
        // binary). Otherwise stay inert: canCheckForUpdates stays false → the
        // menu item + "Check now" disable themselves, no scary "failed to start".
        isSupported = Bundle.main.object(forInfoDictionaryKey: "SUFeedURL") != nil
        guard isSupported else {
            controller = nil
            return
        }

        // Pass the delegate at construction (the only place Sparkle accepts one).
        // `startingUpdater: true` begins scheduled checks per the Info.plist
        // SUEnableAutomaticChecks / SUScheduledCheckInterval keys.
        let c = SPUStandardUpdaterController(
            startingUpdater: true,
            updaterDelegate: delegate,
            userDriverDelegate: nil
        )
        controller = c

        // Route the delegate's callbacks back to our @Observable flags.
        delegate.onFindValidUpdate = { [weak self] in self?.updateAvailable = true }
        delegate.onNotFindUpdate = { [weak self] in self?.updateAvailable = false }

        // Bridge Sparkle's KVO `canCheckForUpdates` into our @Observable property.
        cancellable = c.updater
            .publisher(for: \.canCheckForUpdates)
            .sink { [weak self] value in
                self?.canCheckForUpdates = value
            }
    }

    /// Whether Sparkle automatically checks for updates on its schedule. Bound by
    /// the Settings → Updates "Automatically check for updates" toggle; setting it
    /// writes through to Sparkle (which persists it in UserDefaults, the standard
    /// `SUEnableAutomaticChecks` user preference).
    public var automaticallyChecksForUpdates: Bool {
        get { controller?.updater.automaticallyChecksForUpdates ?? false }
        set { controller?.updater.automaticallyChecksForUpdates = newValue }
    }

    /// The last time Sparkle checked the feed (manual or scheduled). `nil` until
    /// the first check. Surfaced as a relative date in Settings → Updates.
    public var lastUpdateCheckDate: Date? {
        controller?.updater.lastUpdateCheckDate
    }

    /// Show Sparkle's update UI: checks the feed and, if an update is available,
    /// presents the standard alert → download → install → relaunch flow. Bound to
    /// the "Check for Updates…" menu item, the Settings "Check now" button, and
    /// the titlebar pill. Opening the flow clears `updateAvailable` — Sparkle's UI
    /// now owns the interaction, so the pill shouldn't keep nagging.
    public func checkForUpdates() {
        guard let controller else { return }
        updateAvailable = false
        controller.checkForUpdates(nil)
    }
}

// MARK: - SPUUpdaterDelegate

/// Standalone `SPUUpdaterDelegate` for the updater. Kept separate from
/// `UpdaterController` because Sparkle takes the delegate at controller-init time
/// (before `self` exists) — this object is constructed first, then has its
/// closures wired back to the controller's `@Observable` flags. Sparkle calls the
/// delegate on the main thread, so the closures hop to the main actor.
private final class UpdaterDelegate: NSObject, SPUUpdaterDelegate {
    /// Called when Sparkle finds a valid update (scheduled or manual check).
    var onFindValidUpdate: (@MainActor () -> Void)?
    /// Called when a check finds no update (clears any stale "available" flag).
    var onNotFindUpdate: (@MainActor () -> Void)?

    func updater(_ updater: SPUUpdater, didFindValidUpdate item: SUAppcastItem) {
        Task { @MainActor in onFindValidUpdate?() }
    }

    func updaterDidNotFindUpdate(_ updater: SPUUpdater) {
        Task { @MainActor in onNotFindUpdate?() }
    }
}
