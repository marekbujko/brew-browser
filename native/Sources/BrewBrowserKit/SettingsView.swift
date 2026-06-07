import SwiftUI

/// Native Settings scene — stock `Settings { }` + `TabView`, opened by ⌘, or
/// the toolbar gear (SettingsLink). Ports the Tauri Settings modal, with the
/// previously-nested subsections (Updates, Vulnerabilities, Trending History)
/// promoted to their own top-level tabs per the macOS preferences convention.
///
/// Persistence is split exactly like the Tauri app:
///   - gated/network feature toggles → `AppSettings.shared` (settings.json)
///   - local UI prefs → `LocalPrefs.shared` (UserDefaults)
/// Native Settings — stock `TabView` with the default top tab bar. This is the
/// canonical SwiftUI macOS preferences shape; we tried sidebar variants and the
/// default is cleaner and simpler. Opened by ⌘, and the toolbar gear.
/// Settings tabs, shared so other surfaces (e.g. the toolbar Octocat) can open
/// Settings to a specific pane via the `SettingsTab.deepLink` AppStorage key.
public enum SettingsTab: String { case appearance, network, github, brew, updates, security, trending, activity, about }

public struct SettingsView: View {
    /// Persisted selection — also the deep-link target: writing this key before
    /// `openSettings()` makes Settings open to that pane.
    @AppStorage("settings.selectedTab") private var selected = SettingsTab.appearance.rawValue

    /// The Sparkle updater, owned by the app scene and passed in so the Updates
    /// tab's "Check now" / auto-check controls drive the shared instance (Bundle C).
    private let updater: UpdaterController

    public init(updater: UpdaterController) { self.updater = updater }

    public var body: some View {
        TabView(selection: $selected) {
            AppearanceSettings()
                .tabItem { Label("Appearance", systemImage: "paintbrush") }.tag(SettingsTab.appearance.rawValue)
            NetworkSettings()
                .tabItem { Label("Network", systemImage: "globe") }.tag(SettingsTab.network.rawValue)
            GitHubSettings()
                .tabItem { Label("GitHub", systemImage: "chevron.left.forwardslash.chevron.right") }.tag(SettingsTab.github.rawValue)
            BrewSettings()
                .tabItem { Label("Brew", systemImage: "mug") }.tag(SettingsTab.brew.rawValue)
            UpdatesSettings(updater: updater)
                .tabItem { Label("Updates", systemImage: "arrow.down.circle") }.tag(SettingsTab.updates.rawValue)
            VulnerabilitySettings()
                .tabItem { Label("Security", systemImage: "shield") }.tag(SettingsTab.security.rawValue)
            TrendingSettings()
                .tabItem { Label("Trending", systemImage: "chart.line.uptrend.xyaxis") }.tag(SettingsTab.trending.rawValue)
            ActivitySettings()
                .tabItem { Label("Activity", systemImage: "list.bullet.rectangle") }.tag(SettingsTab.activity.rawValue)
            AboutSettings()
                .tabItem { Label("About", systemImage: "info.circle") }.tag(SettingsTab.about.rawValue)
        }
        .frame(width: 560, height: 480)
    }
}

// MARK: - Appearance

private struct AppearanceSettings: View {
    @State private var prefs = LocalPrefs.shared
    @State private var settings = AppSettings.shared

    var body: some View {
        Form {
            Picker("Theme", selection: $prefs.theme) {
                ForEach(AppTheme.allCases, id: \.self) { Text($0.label).tag($0) }
            }
            .pickerStyle(.segmented)
            .onChange(of: prefs.theme) { _, _ in prefs.applyTheme() }

            Picker("Default landing", selection: $prefs.defaultSection) {
                ForEach(LandingSection.allCases, id: \.self) { Text($0.label).tag($0) }
            }

            SwiftUI.Section {
                Toggle("AI features", isOn: Binding(
                    get: { settings.aiFeaturesEnabled },
                    set: { settings.aiFeaturesEnabled = $0; try? settings.save() }
                ))
                Text("Shows extra metadata generated at build time: friendly names, descriptions, use-cases, similar packages, and category tags. Zero LLM calls are made from your machine — all enrichment is baked into the app. When off, only Homebrew's native metadata appears (categories are hidden).")
                    .font(.caption).foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .padding(20)
    }
}

// MARK: - Network

private struct NetworkSettings: View {
    @State private var settings = AppSettings.shared

    var body: some View {
        Form {
            SwiftUI.Section {
                Toggle("Offline Mode", isOn: Binding(
                    get: { settings.paranoidMode },
                    set: { settings.paranoidMode = $0; try? settings.save() }
                ))
                Text("Blocks every outbound network call: catalog refresh, Trending, GitHub stats + sign-in, cask icon probes, update checks. brew itself still runs normally. UI that needs the network shows a 'disabled by Offline Mode' notice.")
                    .font(.caption).foregroundStyle(.secondary)
                if settings.paranoidMode {
                    Label("Offline Mode is on — Trending, Catalog refresh, and Cask icon probes are blocked.",
                          systemImage: "exclamationmark.triangle.fill")
                        .font(.caption).foregroundStyle(.orange)
                }
            }

            SwiftUI.Section {
                Picker("Catalog auto-refresh", selection: Binding(
                    get: { settings.catalogAutoRefresh },
                    set: { settings.catalogAutoRefresh = $0; try? settings.save() }
                )) {
                    Text("Off").tag(CatalogAutoRefresh.off)
                    Text("Weekly").tag(CatalogAutoRefresh.weekly)
                    Text("Daily").tag(CatalogAutoRefresh.daily)
                }
                .pickerStyle(.segmented)
                .disabled(settings.isCorrupt)

                Stepper(value: Binding(
                    get: { Int(settings.catalogStaleBannerDays) },
                    set: { settings.catalogStaleBannerDays = UInt32($0); try? settings.save() }
                ), in: 1...365) {
                    Text("Catalog stale-banner threshold: \(settings.catalogStaleBannerDays) days")
                }

                Picker("Cask icon fetching", selection: Binding(
                    get: { settings.caskIconMode },
                    set: { settings.caskIconMode = $0; try? settings.save() }
                )) {
                    Text("Off").tag(CaskIconMode.off)
                    Text("Installed only").tag(CaskIconMode.installedOnly)
                    Text("All").tag(CaskIconMode.all)
                }
                .pickerStyle(.segmented)

                Stepper(value: Binding(
                    get: { Int(settings.trendingTtlMinutes) },
                    set: { settings.trendingTtlMinutes = UInt32($0); try? settings.save() }
                ), in: 5...1440, step: 5) {
                    Text("Trending cache TTL: \(settings.trendingTtlMinutes) min")
                }
            }

            if settings.isCorrupt {
                SwiftUI.Section {
                    Label("Settings file is unreadable.", systemImage: "exclamationmark.octagon.fill")
                        .foregroundStyle(.red)
                    Button("Reset to defaults") { settings.reset() }
                }
            }
        }
        .formStyle(.grouped)
        .padding(20)
    }
}

// MARK: - GitHub

private struct GitHubSettings: View {
    @State private var settings = AppSettings.shared
    @State private var github = GitHubService()
    @State private var status: GithubStatus?
    @State private var flow: DeviceFlowStart?
    @State private var signingIn = false
    @State private var error: String?

    var body: some View {
        Form {
            SwiftUI.Section {
                Toggle("Show GitHub stats on package pages", isOn: Binding(
                    get: { settings.githubEnabled },
                    set: { settings.githubEnabled = $0; try? settings.save() }
                ))
                Text("Show repo stars, forks, and last release for any package whose homepage is a GitHub URL. Fetches public metadata from api.github.com.")
                    .font(.caption).foregroundStyle(.secondary)
            }

            SwiftUI.Section("Sign in") {
                if let status, status.signedIn {
                    LabeledContent("Signed in as", value: "@\(status.username ?? "?")")
                    if !status.scopes.isEmpty {
                        LabeledContent("Scopes", value: status.scopes.joined(separator: ", "))
                            .font(.caption)
                    }
                    Button("Sign out") {
                        Task {
                            github.signOut()
                            self.status = github.status()
                        }
                    }
                } else if let flow {
                    LabeledContent("Your code", value: flow.userCode)
                        .font(.body.monospaced())
                    Text("Enter this code at \(flow.verificationUri) (opened in your browser). Waiting for authorization…")
                        .font(.caption).foregroundStyle(.secondary)
                    ProgressView().controlSize(.small)
                } else {
                    Button {
                        Task { await startSignIn() }
                    } label: {
                        Label("Sign in with GitHub", systemImage: "person.crop.circle.badge.plus")
                    }
                    .disabled(signingIn)
                    Text("Opens GitHub's standard authorize flow in your browser. No password is entered into brew-browser. Required to Star, Watch, and file issues.")
                        .font(.caption).foregroundStyle(.secondary)
                }
                if let error {
                    Text(error).font(.caption).foregroundStyle(.red)
                }
            }

            SwiftUI.Section {
                Text("Your token is stored in the macOS Keychain — never sent over IPC, written to disk, or logged. Sign-in is optional; minimum scopes are read:user, public_repo, and notifications.")
                    .font(.caption).foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .padding(20)
        .task { status = github.status() }
    }

    private func startSignIn() async {
        signingIn = true
        error = nil
        do {
            let start = try await github.startDeviceFlow()
            flow = start
            if let url = URL(string: start.verificationUri) { NSWorkspace.shared.open(url) }
            // Copy the code so the user can paste it.
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(start.userCode, forType: .string)
            let result = try await github.pollDeviceFlow(deviceCode: start.deviceCode, interval: start.interval)
            status = result
            flow = nil
        } catch {
            self.error = error.localizedDescription
            flow = nil
        }
        signingIn = false
    }
}

// MARK: - Brew

private struct BrewSettings: View {
    @State private var prefs = LocalPrefs.shared
    @State private var brew = BrewService()
    @State private var analytics: Bool?
    @State private var analyticsBusy = false

    var body: some View {
        Form {
            SwiftUI.Section("Analytics") {
                Toggle("Send Homebrew install analytics", isOn: Binding(
                    get: { analytics ?? false },
                    set: { newValue in
                        Task {
                            analyticsBusy = true
                            try? await brew.setAnalytics(newValue)
                            analytics = newValue
                            analyticsBusy = false
                        }
                    }
                ))
                .disabled(analytics == nil || analyticsBusy)
                Text("Homebrew sends anonymous install analytics to formulae.brew.sh by default. This flips Homebrew's own setting (same as `brew analytics on`/`off`).")
                    .font(.caption).foregroundStyle(.secondary)
            }

            SwiftUI.Section("Confirmations") {
                Toggle("Confirm before uninstall / zap", isOn: $prefs.confirmDestructive)
                Text("Destructive actions (Uninstall, Zap, Delete Brewfile) ask before proceeding.")
                    .font(.caption).foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .padding(20)
        .task { analytics = await brew.getAnalytics() }
    }
}

// MARK: - Updates

private struct UpdatesSettings: View {
    @State private var settings = AppSettings.shared
    /// The shared Sparkle updater (Bundle C). `@Bindable` so the "Automatically
    /// check" toggle can bind through to it and the view re-renders when its
    /// observable state (canCheckForUpdates, lastUpdateCheckDate) changes.
    @Bindable var updater: UpdaterController

    /// Offline Mode gates the manual check — same posture as the Tauri "Check
    /// now" button (`SettingsSectionUpdates.svelte:107`).
    private var offline: Bool { settings.paranoidMode }

    var body: some View {
        Form {
            // Check now — runs Sparkle's check, which presents the standard
            // update UI if a newer build is on the feed. Disabled while a check
            // is already in flight (canCheckForUpdates) or in Offline Mode.
            SwiftUI.Section {
                HStack {
                    Button {
                        updater.checkForUpdates()
                    } label: {
                        Label("Check now", systemImage: "arrow.clockwise")
                    }
                    .disabled(offline || !updater.canCheckForUpdates)
                    Spacer()
                    Text("Last checked: \(lastCheckedLabel)")
                        .font(.caption).foregroundStyle(.secondary)
                }
                if offline {
                    Label("Offline Mode is on — manual update checks are blocked. Turn it off in Network to check the feed.",
                          systemImage: "exclamationmark.triangle.fill")
                        .font(.caption).foregroundStyle(.orange)
                } else {
                    Text("Fetches brew-browser.zerologic.com/appcast.xml and compares the published version to the one you're running. No version number is sent.")
                        .font(.caption).foregroundStyle(.secondary)
                }
            }

            // Auto-check — binds Sparkle's automaticallyChecksForUpdates (its
            // standard, persisted preference). Stays toggleable even in Offline
            // Mode so the preference is set for the next time it's off, matching
            // the Tauri auto-check toggle and every other network toggle.
            SwiftUI.Section {
                Toggle("Automatically check for updates", isOn: $updater.automaticallyChecksForUpdates)
                Text("When on, brew-browser checks for a newer version once a day and shows a notice in the title bar if one is available. Suspended while Offline Mode is on.")
                    .font(.caption).foregroundStyle(.secondary)
            }

            SwiftUI.Section("Update channel") {
                LabeledContent("Channel", value: "Stable")
                Text("No beta channel in this release.")
                    .font(.caption).foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .padding(20)
    }

    /// Last-checked relative date ("2 hours ago"), or "Never" before the first
    /// check. Mirrors the Tauri `lastCheckedLabel`.
    private var lastCheckedLabel: String {
        guard let date = updater.lastUpdateCheckDate else { return "Never" }
        return date.formatted(.relative(presentation: .named))
    }
}

// MARK: - Vulnerabilities

private struct VulnerabilitySettings: View {
    @State private var settings = AppSettings.shared
    @State private var vulns = VulnsService()
    @State private var helperInstalled: Bool?
    @State private var installing = false

    private var offline: Bool { settings.paranoidMode }

    var body: some View {
        Form {
            SwiftUI.Section {
                Toggle("Scan installed packages for known vulnerabilities", isOn: Binding(
                    get: { settings.vulnerabilityScanningEnabled },
                    set: { settings.vulnerabilityScanningEnabled = $0; try? settings.save() }
                ))
                .disabled(offline || settings.isCorrupt)
                Text("Opt-in, off by default. Shells out to the official `brew vulns` subcommand, which queries OSV.dev for known vulnerabilities in your installed formulae. If signed in to GitHub, GHSA IDs are enriched from api.github.com. No package list leaves your machine except the queries brew vulns makes.")
                    .font(.caption).foregroundStyle(.secondary)
                if offline {
                    Label("Offline Mode is on — scanning is suppressed even if this is on.",
                          systemImage: "exclamationmark.triangle.fill")
                        .font(.caption).foregroundStyle(.orange)
                }
            }

            if settings.vulnerabilityScanningEnabled && !offline {
                SwiftUI.Section {
                    if helperInstalled == false {
                        Text("The brew-vulns subcommand isn't installed. Install it now? This runs `brew install homebrew/brew-vulns/brew-vulns`.")
                            .font(.caption).foregroundStyle(.secondary)
                        Button {
                            Task {
                                installing = true
                                _ = try? await vulns.installHelper()
                                helperInstalled = await vulns.isBrewVulnsInstalled()
                                installing = false
                            }
                        } label: {
                            if installing {
                                HStack { ProgressView().controlSize(.small); Text("Installing…") }
                            } else {
                                Label("Install brew-vulns", systemImage: "arrow.down.circle")
                            }
                        }
                        .disabled(installing)
                    } else if helperInstalled == true {
                        Label("brew vulns is installed.", systemImage: "checkmark.seal.fill")
                            .foregroundStyle(.green)
                    } else {
                        ProgressView().controlSize(.small)
                    }
                }
            }
        }
        .formStyle(.grouped)
        .padding(20)
        .task { helperInstalled = await vulns.isBrewVulnsInstalled() }
    }
}

// MARK: - Trending History

private struct TrendingSettings: View {
    @State private var settings = AppSettings.shared
    private var offline: Bool { settings.paranoidMode }

    var body: some View {
        Form {
            SwiftUI.Section {
                Toggle("Fetch trending history", isOn: Binding(
                    get: { settings.enhancedTrendingEnabled },
                    set: { settings.enhancedTrendingEnabled = $0; try? settings.save() }
                ))
                .disabled(offline || settings.isCorrupt)
                Text("When on, brew-browser fetches per-package historical install trends from brew-browser.zerologic.com/trending-history/* to power sparklines on Trending and each package's detail panel. Only the package name you're viewing is sent. Operated by the brew-browser project — a distinct trust boundary from the Homebrew analytics paths.")
                    .font(.caption).foregroundStyle(.secondary)
                if offline {
                    Label("Offline Mode is on — this toggle is locked off.",
                          systemImage: "exclamationmark.triangle.fill")
                        .font(.caption).foregroundStyle(.orange)
                }
            }

            SwiftUI.Section {
                Toggle("Fetch latest categories & descriptions", isOn: Binding(
                    get: { settings.liveEnrichmentEnabled },
                    set: { settings.liveEnrichmentEnabled = $0; try? settings.save() }
                ))
                .disabled(offline || settings.isCorrupt)
                Text("brew-browser ships with built-in AI categories and descriptions. When on, it refreshes them from brew-browser.zerologic.com/enrichment/* — a version check on refresh, the full category list when newer, and a per-package description when you open its detail. Only the package name you're viewing is sent. Same first-party host as Enhanced Trending, a distinct /enrichment/* path. Requires AI features on.")
                    .font(.caption).foregroundStyle(.secondary)
                if offline {
                    Label("Offline Mode is on — this toggle is locked off.",
                          systemImage: "exclamationmark.triangle.fill")
                        .font(.caption).foregroundStyle(.orange)
                }
            }
        }
        .formStyle(.grouped)
        .padding(20)
    }
}

// MARK: - Activity

private struct ActivitySettings: View {
    @State private var prefs = LocalPrefs.shared

    var body: some View {
        Form {
            SwiftUI.Section {
                Stepper(value: $prefs.activityMaxJobs, in: 1...1000) {
                    Text("Keep last \(prefs.activityMaxJobs) completed jobs")
                }
                Stepper(value: $prefs.activityMaxLines, in: 100...10000, step: 50) {
                    Text("Lines per job: \(prefs.activityMaxLines)")
                }
                Text("These limits apply to future job persistence. Existing retained data is not trimmed retroactively.")
                    .font(.caption).foregroundStyle(.secondary)
            }

            SwiftUI.Section {
                Toggle("Notify when brew tasks finish", isOn: Binding(
                    get: { prefs.notifyOnTaskCompletion },
                    set: { on in
                        prefs.notifyOnTaskCompletion = on
                        if on { NotificationService.requestAuthorization() }
                    }
                ))
                Text("Posts a macOS notification when an install, upgrade, or other brew task finishes while brew-browser isn't the front app. When it's in front, the Activity drawer shows progress instead. Turning this on asks macOS for notification permission.")
                    .font(.caption).foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .padding(20)
    }
}

// MARK: - About

private struct AboutSettings: View {
    var body: some View {
        Form {
            SwiftUI.Section {
                LabeledContent("App version", value: appVersion())
                LabeledContent("License", value: "MIT")
                Link("github.com/msitarzewski/brew-browser",
                     destination: URL(string: "https://github.com/msitarzewski/brew-browser")!)
            }
            SwiftUI.Section {
                Text("Zero telemetry. Zero accounts. brew-browser does not collect telemetry, phone home, or have user accounts. Every outbound request is documented in Settings → Network and only fires when you take an action that requires it.")
                    .font(.caption).foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .padding(20)
    }

    private func appVersion() -> String {
        Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "—"
    }
}
