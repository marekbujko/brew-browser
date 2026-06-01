import SwiftUI
import Charts

/// The package detail inspector — full Tauri-parity port of `PackageDetail.svelte`.
/// Stock SwiftUI only. Sections render in the same order as the Tauri panel and
/// gate on the same settings (AI features, vuln scanning, GitHub, enhanced
/// trending). Network/scan sections show only when their data is present or
/// their feature is enabled.
struct PackageDetailView: View {
    @Bindable var model: AppModel
    let pkg: InstalledPackage

    @State private var confirmUninstall = false

    private var info: PackageInfo? { model.detailInfo }
    private var enrichment: EnrichmentEntry? { model.detailEnrichment }

    /// Display title: enriched friendly name when available, else the token.
    private var title: String { enrichment?.friendlyName ?? pkg.name }

    /// Installed = brew reported an installed version. nil (Discover packages
    /// not on disk) → not installed. Drives the footer's Install/Uninstall.
    private var isInstalled: Bool { info?.installedVersion != nil }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                // Close box pinned top-right; the icon + title center beneath it.
                HStack {
                    Spacer()
                    Button {
                        model.closeDetail()
                    } label: {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundStyle(.secondary)
                            .font(.title3)
                    }
                    .buttonStyle(.plain)
                    .help("Close")
                }
                .padding(.bottom, -8)  // tuck the title block up under the ✕ row

                // Centered icon (casks) + title + kind pill — the app's identity
                // at the top of the inspector, above the meta table.
                VStack(spacing: 8) {
                    DetailIcon(model: model, token: pkg.name, kind: pkg.kind,
                               homepage: info?.homepage ?? "")
                    Text(title)
                        .font(.title2.weight(.semibold))
                        .multilineTextAlignment(.center)
                        .textSelection(.enabled)
                    KindPill(kind: pkg.kind)
                }
                .frame(maxWidth: .infinity)

                if model.detailLoading && info == nil {
                    ProgressView("Loading \(pkg.name)…")
                        .frame(maxWidth: .infinity, minHeight: 200)
                } else if let err = model.detailError, info == nil {
                    ContentUnavailableView("Couldn't load \(pkg.name)",
                                           systemImage: "exclamationmark.triangle",
                                           description: Text(err))
                } else {
                    metaCard
                    if let summary = enrichment?.summary, !summary.isEmpty {
                        Text(summary)
                            .font(.callout)
                            .padding(12)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .background(.quaternary, in: .rect(cornerRadius: 10))
                    }
                    if let desc = info?.desc, !desc.isEmpty {
                        Text(desc).font(.subheadline).foregroundStyle(.secondary)
                    }
                    if let hp = info?.homepage, let url = URL(string: hp) {
                        Link(destination: url) {
                            Label(hp, systemImage: "safari").lineLimit(1).truncationMode(.middle)
                        }
                    }
                    if !model.detailCategories.isEmpty { categoriesRow }
                    if let tags = enrichment?.tags, !tags.isEmpty { tagsRow(tags) }

                    if AppSettings.shared.vulnerabilityScanningAllowed { securityCard }
                    if model.detailTrend != nil { trendCard }
                    if let useCases = enrichment?.useCases, !useCases.isEmpty { useCasesCard(useCases) }
                    if let similar = enrichment?.similar, !similar.isEmpty { similarCard(similar) }
                    if AppSettings.shared.githubAllowed, model.detailRepoStats != nil { githubCard }
                    if let caveats = info?.caveats, !caveats.isEmpty { caveatsCard(caveats) }
                    dependenciesSection
                }
            }
            .padding(16)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .safeAreaInset(edge: .bottom) { footer }
    }

    // MARK: meta

    private var metaCard: some View {
        VStack(spacing: 0) {
            metaRow("Token", pkg.name, mono: true)
            Divider()
            metaRow("Installed", info?.installedVersion ?? "Not installed")
            Divider()
            HStack {
                Text("Latest").foregroundStyle(.secondary)
                    .frame(width: 90, alignment: .leading)
                Text(info?.stableVersion ?? "—").monospaced()
                if info?.isOutdated == true {
                    Text("Upgrade available")
                        .font(.caption).foregroundStyle(.orange)
                }
                Spacer()
            }
            .padding(.vertical, 6)
            if let lic = info?.license {
                Divider(); metaRow("License", lic)
            }
            if let tap = info?.tap {
                Divider(); metaRow("Tap", tap, mono: true)
            }
        }
        .padding(12)
        .background(.quaternary, in: .rect(cornerRadius: 10))
    }

    private func metaRow(_ label: String, _ value: String, mono: Bool = false) -> some View {
        HStack {
            Text(label).foregroundStyle(.secondary).frame(width: 90, alignment: .leading)
            Text(value).font(mono ? .body.monospaced() : .body)
                .textSelection(.enabled)
            Spacer()
        }
        .padding(.vertical, 6)
    }

    private var categoriesRow: some View {
        FlowRow(spacing: 6) {
            ForEach(model.detailCategories, id: \.self) { label in
                Chip(text: label)
            }
        }
    }

    private func tagsRow(_ tags: [String]) -> some View {
        FlowRow(spacing: 6) {
            ForEach(tags, id: \.self) { Chip(text: $0) }
        }
    }

    // MARK: security

    @ViewBuilder private var securityCard: some View {
        GroupBox {
            VStack(alignment: .leading, spacing: 10) {
                if !model.brewVulnsInstalled {
                    Text("Install `brew vulns` to scan this package for known vulnerabilities.")
                        .font(.callout).foregroundStyle(.secondary)
                } else if model.detailVulnsLoading {
                    ProgressView("Scanning…")
                } else if !model.detailVulnsScanned {
                    Text("Check this package for known vulnerabilities.")
                        .font(.callout).foregroundStyle(.secondary)
                    Button("Check now") { Task { await model.scanDetailVulns() } }
                } else if model.detailVulns.isEmpty {
                    Label("No known vulnerabilities", systemImage: "checkmark.seal.fill")
                        .foregroundStyle(.green)
                } else {
                    ForEach(model.detailVulns) { v in
                        VStack(alignment: .leading, spacing: 2) {
                            HStack {
                                Text(v.rawId.isEmpty ? "Advisory" : v.rawId)
                                    .font(.callout.monospaced())
                                SeverityPill(severity: v.severity)
                                Spacer()
                                if let fixed = v.fixedIn {
                                    Text("Patched in \(fixed)").font(.caption).foregroundStyle(.secondary)
                                }
                            }
                            if !v.summary.isEmpty {
                                Text(v.summary).font(.caption).foregroundStyle(.secondary)
                            }
                        }
                        .padding(.vertical, 4)
                    }
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.top, 2)
        } label: {
            Label("Security", systemImage: "shield")
        }
    }

    // MARK: trend

    @ViewBuilder private var trendCard: some View {
        if let series = model.detailTrend {
            GroupBox {
                let values = sparkValues(series)
                if values.count > 1 {
                    Chart(Array(values.enumerated()), id: \.offset) { i, v in
                        LineMark(x: .value("i", i), y: .value("installs", v))
                            .interpolationMethod(.catmullRom)
                    }
                    .frame(height: 80)
                    .chartXAxis(.hidden)
                    .chartYAxis(.hidden)
                    .padding(.top, 4)
                } else {
                    Text("Not enough data yet").font(.caption).foregroundStyle(.secondary)
                }
            } label: {
                Label("Install trend", systemImage: "chart.line.uptrend.xyaxis")
            }
        }
    }

    private func sparkValues(_ s: TrendingHistorySeries) -> [Double] {
        s.points.compactMap { p in
            if let e = p.estimatedDailyInstalls { return Double(e) }
            if let c = p.count30d { return Double(c) }
            return nil
        }
    }

    private func useCasesCard(_ cases: [String]) -> some View {
        GroupBox {
            VStack(alignment: .leading, spacing: 6) {
                ForEach(cases, id: \.self) { c in
                    Label(c, systemImage: "checkmark.circle").font(.callout)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.top, 2)
        } label: {
            Label("Why install this?", systemImage: "lightbulb")
        }
    }

    private func similarCard(_ similar: [String]) -> some View {
        GroupBox {
            FlowRow(spacing: 6) {
                ForEach(similar, id: \.self) { name in
                    Button(name) { model.openDetail(InstalledPackage(name: name, version: "—", kind: .formula)) }
                        .buttonStyle(.plain)
                        .padding(.horizontal, 8).padding(.vertical, 3)
                        .background(.quaternary, in: .capsule)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.top, 2)
        } label: {
            Label("Similar packages", systemImage: "square.stack.3d.up")
        }
    }

    // MARK: github

    @ViewBuilder private var githubCard: some View {
        if let stats = model.detailRepoStats {
            GroupBox {
                VStack(alignment: .leading, spacing: 8) {
                    HStack(spacing: 16) {
                        Label("\(stats.stars)", systemImage: "star")
                        Label("\(stats.forks)", systemImage: "tuningfork")
                        if let tag = stats.lastReleaseTag {
                            Label(tag, systemImage: "tag")
                        }
                        Spacer()
                    }
                    .font(.callout)
                    HStack {
                        Button {
                            Task {
                                if let hp = info?.homepage {
                                    try? await GitHubService().setStar(homepage: hp, starred: !(model.detailStarred ?? false))
                                }
                            }
                        } label: {
                            Label(model.detailStarred == true ? "Starred" : "Star",
                                  systemImage: model.detailStarred == true ? "star.fill" : "star")
                        }
                        Spacer()
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.top, 2)
            } label: {
                Label("GitHub", systemImage: "chevron.left.forwardslash.chevron.right")
            }
        }
    }

    private func caveatsCard(_ caveats: String) -> some View {
        GroupBox {
            Text(caveats)
                .font(.callout.monospaced())
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.top, 2)
        } label: {
            Label("Caveats", systemImage: "exclamationmark.bubble")
        }
    }

    @ViewBuilder private var dependenciesSection: some View {
        if let info, !info.dependencies.isEmpty {
            DisclosureGroup("Dependencies (\(info.dependencies.count))") {
                FlowRow(spacing: 6) {
                    ForEach(info.dependencies, id: \.self) { Chip(text: $0) }
                }
            }
        }
        if let info, !info.conflictsWith.isEmpty {
            DisclosureGroup("Conflicts with (\(info.conflictsWith.count))") {
                FlowRow(spacing: 6) {
                    ForEach(info.conflictsWith, id: \.self) { Chip(text: $0) }
                }
            }
        }
    }

    // MARK: footer

    private var footer: some View {
        HStack {
            if model.actionRunning {
                ProgressView().controlSize(.small)
                Text(model.actionLabel ?? "Working…").font(.caption).foregroundStyle(.secondary)
            }
            Spacer()
            // Footer adapts to install state — only once info has loaded, so we
            // never flash the wrong button. `installedVersion == nil` means not
            // installed (Discover surfaces these) → Install. Installed →
            // Uninstall, plus Upgrade when outdated.
            if info != nil {
                if isInstalled {
                    if info?.isOutdated == true {
                        Button {
                            Task { await model.upgradeDetail() }
                        } label: { Label("Upgrade", systemImage: "arrow.up.circle") }
                        .buttonStyle(.borderedProminent)
                        .disabled(model.actionRunning)
                    }
                    Button(role: .destructive) {
                        confirmUninstall = true
                    } label: { Label("Uninstall", systemImage: "trash") }
                    .disabled(model.actionRunning)
                } else {
                    Button {
                        Task { await model.installDetail() }
                    } label: { Label("Install", systemImage: "arrow.down.circle") }
                    .buttonStyle(.borderedProminent)
                    .disabled(model.actionRunning)
                }
            }
        }
        .padding(12)
        .background(.bar)
        .confirmationDialog("Uninstall \(pkg.name)?", isPresented: $confirmUninstall, titleVisibility: .visible) {
            Button("Uninstall", role: .destructive) { Task { await model.uninstallDetail() } }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("This runs `brew uninstall \(pkg.name)`.")
        }
    }
}

/// Severity pill for vulnerability findings.
struct SeverityPill: View {
    let severity: VulnSeverity
    private var color: Color {
        switch severity {
        case .critical, .high: return .red
        case .medium: return .orange
        case .low: return .yellow
        case .unknown: return .gray
        }
    }
    var body: some View {
        Text(severity.rawValue.capitalized)
            .font(.caption2.weight(.medium))
            .padding(.horizontal, 7).padding(.vertical, 2)
            .background(color.opacity(0.2), in: .capsule)
            .foregroundStyle(color)
    }
}

/// Minimal flow layout (wrapping HStack) for pills — stock SwiftUI `Layout`.
struct FlowRow: Layout {
    var spacing: CGFloat = 6

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let maxWidth = proposal.width ?? .infinity
        var x: CGFloat = 0, y: CGFloat = 0, rowHeight: CGFloat = 0
        for sub in subviews {
            let size = sub.sizeThatFits(.unspecified)
            if x + size.width > maxWidth, x > 0 {
                x = 0; y += rowHeight + spacing; rowHeight = 0
            }
            x += size.width + spacing
            rowHeight = max(rowHeight, size.height)
        }
        return CGSize(width: maxWidth == .infinity ? x : maxWidth, height: y + rowHeight)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var x = bounds.minX, y = bounds.minY, rowHeight: CGFloat = 0
        for sub in subviews {
            let size = sub.sizeThatFits(.unspecified)
            if x + size.width > bounds.maxX, x > bounds.minX {
                x = bounds.minX; y += rowHeight + spacing; rowHeight = 0
            }
            sub.place(at: CGPoint(x: x, y: y), proposal: ProposedViewSize(size))
            x += size.width + spacing
            rowHeight = max(rowHeight, size.height)
        }
    }
}

/// Large centered app icon for the detail panel header. Resolves the real icon
/// (Appcasks → Google favicon) for casks via the shared IconService; formulae
/// (CLI tools, no app icon) show a terminal SF Symbol. 64pt to anchor the
/// inspector's identity block above the meta table.
private struct DetailIcon: View {
    @Bindable var model: AppModel
    let token: String
    let kind: InstalledPackage.Kind
    let homepage: String

    var body: some View {
        Group {
            if kind == .cask, let url = model.iconCache[token],
               let img = NSImage(contentsOf: url) {
                Image(nsImage: img).resizable().interpolation(.high)
                    .frame(width: 64, height: 64)
                    .clipShape(.rect(cornerRadius: 14))
            } else {
                Image(systemName: kind == .cask ? "app.dashed" : "terminal")
                    .font(.system(size: 44))
                    .foregroundStyle(.secondary)
                    .frame(width: 64, height: 64)
            }
        }
        .task(id: token) {
            await model.resolveIcon(token: token, kind: kind, homepage: homepage)
        }
    }
}
