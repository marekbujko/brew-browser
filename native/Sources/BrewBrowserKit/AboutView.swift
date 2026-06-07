import SwiftUI

/// Custom About box — the native port of the Tauri `AboutModal.svelte`. A stock
/// SwiftUI sheet: brand hero, a Version/Homebrew/License/Repo meta card, a
/// "Donate to the project" CTA, a "Built with" credits section, and the
/// zero-telemetry posture line. Opened from the app menu's "About brew-browser"
/// (`AppModel.aboutOpen`); replaces the bare standard AppKit About panel.
struct AboutView: View {
    @Bindable var model: AppModel
    @Environment(\.dismiss) private var dismiss

    private static let repoURL = URL(string: "https://github.com/msitarzewski/brew-browser")!
    private static let licenseURL = URL(string: "https://github.com/msitarzewski/brew-browser/blob/main/LICENSE")!
    private static let agencyURL = URL(string: "https://github.com/msitarzewski/agency-agents")!
    private static let sponsorURL = URL(string: "https://github.com/sponsors/msitarzewski")!

    private var appVersion: String {
        Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "—"
    }

    /// The real app icon (bundled `AppIcon.icns`, loaded via `Bundle.module` —
    /// same source as the Dock icon). Loaded once; falls back to the beer mark.
    private static let appIcon: NSImage? = {
        guard let url = Bundle.module.url(forResource: "AppIcon", withExtension: "icns") else { return nil }
        return NSImage(contentsOf: url)
    }()

    var body: some View {
        VStack(spacing: 20) {
            // Hero
            VStack(spacing: 6) {
                if let icon = Self.appIcon {
                    Image(nsImage: icon)
                        .resizable()
                        .interpolation(.high)
                        .frame(width: 72, height: 72)
                } else {
                    Text("🍺").font(.system(size: 48))
                }
                Text("Brew Browser").font(.title.weight(.semibold))
                Text("A native macOS GUI for Homebrew.")
                    .font(.callout).foregroundStyle(.secondary)
            }

            // Meta card
            VStack(spacing: 8) {
                metaRow("Version") { Text(appVersion).monospaced() }
                metaRow("Homebrew") { Text(model.brewVersion).monospaced() }
                metaRow("License") { Link("MIT", destination: Self.licenseURL) }
                metaRow("Repo") {
                    Link("github.com/msitarzewski/brew-browser", destination: Self.repoURL)
                        .lineLimit(1).truncationMode(.middle)
                }
            }
            .padding(.vertical, 12)
            .padding(.horizontal, 14)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(.quaternary.opacity(0.4), in: .rect(cornerRadius: 10))

            // Donate CTA
            Link(destination: Self.sponsorURL) {
                Label {
                    HStack(spacing: 4) {
                        Text("Donate to the project")
                        Image(systemName: "arrow.up.right.square").font(.caption2)
                    }
                } icon: {
                    Image(systemName: "heart.fill")
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, 8)
            }
            .buttonStyle(.borderedProminent)
            .tint(.orange)
            .help("Open GitHub Sponsors in your browser")

            // Built with
            VStack(alignment: .leading, spacing: 8) {
                Label("Built with", systemImage: "sparkles")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.secondary)
                    .textCase(.uppercase)
                Text("[Agency Agents](\(Self.agencyURL.absoluteString)) — the multi-agent toolkit that orchestrated the waves (Backend Architect, Frontend Developer, Security Engineer, Code Reviewer, Technical Writer, and friends), powered by **Claude Code** in the terminal, running **Opus 4.7 [1m]**.")
                    .font(.callout).foregroundStyle(.secondary)
                Text("Thanks also to the Homebrew project for the package data, every formula and cask maintainer for their work, and Tauri for the native shell.")
                    .font(.callout).italic().foregroundStyle(.secondary).opacity(0.85)
            }
            .frame(maxWidth: .infinity, alignment: .leading)

            // Posture
            Text("Zero telemetry. Zero accounts. Every outbound network call is documented in Settings → Network.")
                .font(.caption).foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
        }
        .padding(28)
        .frame(width: 440)
        .overlay(alignment: .topTrailing) {
            Button { dismiss() } label: { Image(systemName: "xmark") }
                .buttonStyle(.plain)
                .foregroundStyle(.secondary)
                .padding(12)
                .help("Close")
        }
        .onKeyPress(.escape) { dismiss(); return .handled }
    }

    private func metaRow<Content: View>(_ label: String, @ViewBuilder _ value: () -> Content) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 12) {
            Text(label).foregroundStyle(.secondary).frame(width: 80, alignment: .leading)
            value().frame(maxWidth: .infinity, alignment: .leading)
        }
        .font(.callout)
    }
}

#if DEBUG
#Preview("About") { AboutView(model: .preview()) }
#endif
