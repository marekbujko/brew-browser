import SwiftUI
import Charts
import AppKit

/// Dashboard — feature parity with the Tauri Dashboard.svelte. Sections, in
/// order: hero stat strip (installed · updates · brew version/prefix),
/// "Updates available" (kind pills, Choose…, Upgrade all (N), +N in Library),
/// Composition (stacked bar + on-request/pinned chips), Storage (full
/// breakdown with paths, open-in-Finder, total). Stock SwiftUI throughout;
/// every number is live brew data.
struct DashboardView: View {
    @Bindable var model: AppModel

    /// True when the content pane is wide enough to pair cards two-across.
    @State private var wide = false

    var body: some View {
        ScrollView {
            if !model.dashboardLoaded {
                ProgressView("Reading your Homebrew setup…")
                    .frame(maxWidth: .infinity, minHeight: 300)
            } else {
                VStack(alignment: .leading, spacing: 16) {
                    HeroStrip(model: model)
                    if model.outdatedCount > 0 { UpdatesCard(model: model) }

                    // Composition + Top categories sit side by side when the
                    // pane is wide enough, and stack when it's narrow (or the
                    // inspector is open). Updates and Storage stay full-width —
                    // they're tabular and want the room.
                    if wide {
                        // .fixedSize keeps each card's natural height, then
                        // maxHeight: .infinity stretches the shorter one to match
                        // the taller — so the two cards align bottom-to-bottom.
                        HStack(alignment: .top, spacing: 16) {
                            CompositionCard(model: model, compact: true)
                                .frame(maxWidth: .infinity, maxHeight: .infinity)
                            if !model.categories.isEmpty {
                                CategoriesCard(model: model)
                                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                            }
                        }
                        .fixedSize(horizontal: false, vertical: true)
                    } else {
                        CompositionCard(model: model)
                        if !model.categories.isEmpty { CategoriesCard(model: model) }
                    }

                    StorageCard(model: model)
                }
                .padding(20)
                // Fill the available content width (no fixed cap) so the
                // Dashboard uses the full pane instead of hugging the left.
                .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        // Measure the pane width to drive the two-across vs stacked breakpoint.
        .onGeometryChange(for: CGFloat.self) { proxy in
            proxy.size.width
        } action: { newWidth in
            wide = newWidth > 980
        }
        .task {
            if !model.dashboardLoaded { await model.loadDashboard() }
        }
    }
}

// MARK: - Hero strip

struct HeroStrip: View {
    @Bindable var model: AppModel

    var body: some View {
        // Three equal flexible columns that always span the full width, like
        // the Tauri hero strip — they shrink together rather than leaving a gap
        // on the right (StatTile's minimumScaleFactor handles narrow widths).
        HStack(spacing: 16) {
            StatTile(value: "\(model.totalPackages)", label: "installed", symbol: "shippingbox") {
                model.openLibrary()
            }
            StatTile(
                value: model.outdatedCount == 0 ? "All current" : "\(model.outdatedCount)",
                label: model.outdatedCount == 0 ? "" : "updates available",
                symbol: "arrow.up.circle"
            ) { if model.outdatedCount > 0 { model.openOutdatedInLibrary() } }
            .disabled(model.outdatedCount == 0)

            StatTile(value: model.brewVersion, label: model.brewPrefix, symbol: "internaldrive", monospaceValue: true, action: nil)
        }
    }
}

struct StatTile: View {
    let value: String
    let label: String
    let symbol: String
    var monospaceValue = false
    var action: (() -> Void)?

    var body: some View {
        GroupBox {
            HStack(spacing: 12) {
                Image(systemName: symbol).font(.title2).foregroundStyle(.secondary)
                VStack(alignment: .leading, spacing: 2) {
                    Text(value)
                        .font(monospaceValue ? .title3.weight(.semibold).monospaced() : .title2.weight(.semibold))
                        .lineLimit(1).minimumScaleFactor(0.5)
                    if !label.isEmpty {
                        Text(label).font(.caption).foregroundStyle(.secondary).lineLimit(1).truncationMode(.middle)
                    }
                }
                Spacer(minLength: 0)
            }
            .frame(maxWidth: .infinity, minHeight: 52, alignment: .leading)
            .contentShape(.rect)
        }
        .onTapGesture { action?() }
    }
}

// MARK: - Updates available

struct UpdatesCard: View {
    @Bindable var model: AppModel

    var body: some View {
        GroupBox {
            VStack(spacing: 0) {
                // Header row: title link + Choose… + Upgrade all (N)
                HStack {
                    Button {
                        model.openOutdatedInLibrary()
                    } label: {
                        HStack(spacing: 4) {
                            Text("Updates available").font(.headline)
                            Image(systemName: "arrow.right").font(.caption)
                        }
                    }
                    .buttonStyle(.plain)
                    Spacer()
                    Button("Choose…") { model.openOutdatedInLibrary() }
                        .controlSize(.small)
                    Button {
                        // wired to brew upgrade in full port
                    } label: {
                        Label("Upgrade all (\(model.outdatedCount))", systemImage: "arrow.up.circle")
                    }
                    .controlSize(.small)
                    .buttonStyle(.borderedProminent)
                }
                .padding(.bottom, 8)

                ForEach(model.outdatedPreview) { p in
                    Button {
                        model.openDetail(InstalledPackage(name: p.name, version: p.installedVersion, kind: p.kind))
                    } label: {
                        HStack(spacing: 10) {
                            // Name column — flexible, takes remaining space.
                            Text(p.name)
                                .frame(maxWidth: .infinity, alignment: .leading)
                            // Kind column — fixed width so pills align vertically.
                            KindPill(kind: p.kind)
                                .frame(width: 84, alignment: .leading)
                            // Version column — fixed width, right-aligned.
                            Text("\(p.installedVersion) → \(p.currentVersion)")
                                .font(.callout).foregroundStyle(.secondary).monospaced()
                                .frame(width: 200, alignment: .trailing)
                        }
                        .contentShape(.rect)
                    }
                    .buttonStyle(.plain)
                    .padding(.vertical, 7)
                    if p.id != model.outdatedPreview.last?.id { Divider() }
                }

                if model.outdatedCount > model.outdatedPreview.count {
                    HStack {
                        Button("+ \(model.outdatedCount - model.outdatedPreview.count) more in Library") {
                            model.openOutdatedInLibrary()
                        }
                        .buttonStyle(.link)
                        Spacer()
                    }
                    .padding(.top, 8)
                }
            }
            .padding(.top, 2)
        }
    }
}

/// formula/cask kind pill, like the Tauri `<Pill>`.
struct KindPill: View {
    let kind: InstalledPackage.Kind
    var body: some View {
        Text(kind.rawValue)
            .font(.caption2.weight(.medium))
            .padding(.horizontal, 7).padding(.vertical, 2)
            .background(kind == .formula ? Color.blue.opacity(0.18) : Color.orange.opacity(0.18),
                        in: .capsule)
            .foregroundStyle(kind == .formula ? .blue : .orange)
    }
}

// MARK: - Composition (stacked bar + chips)

struct CompositionCard: View {
    @Bindable var model: AppModel
    /// When paired side-by-side with Top categories, render a pie (fills a
    /// square-ish card); when full-width, render the stacked bar.
    var compact = false

    private var total: Int { max(1, model.formulaCount + model.caskCount) }
    private var formulaFraction: CGFloat { CGFloat(model.formulaCount) / CGFloat(total) }

    private var pieData: [(label: String, count: Int)] {
        [("Formulae", model.formulaCount), ("Casks", model.caskCount)]
    }

    private var formulaPct: Double { Double(model.formulaCount) / Double(total) * 100 }
    private var caskPct: Double { Double(model.caskCount) / Double(total) * 100 }

    var body: some View {
        GroupBox {
            VStack(alignment: .leading, spacing: 12) {
                // Title row — pills (on request / pinned) live here now.
                HStack {
                    Text("Composition").font(.headline)
                    Spacer()
                    Chip(text: "\(model.onRequestCount) on request")
                    Chip(text: "\(model.pinnedCount) pinned")
                }

                if compact {
                    // Pie + ranked legend, mirroring the Top-categories layout.
                    HStack(alignment: .center, spacing: 20) {
                        Chart(pieData, id: \.label) { item in
                            SectorMark(angle: .value("Count", item.count), angularInset: 1.5)
                                .cornerRadius(4)
                                .foregroundStyle(by: .value("Kind", item.label))
                        }
                        .chartForegroundStyleScale(["Formulae": Color.blue, "Casks": Color.orange])
                        .chartLegend(.hidden)
                        .frame(width: 180, height: 180)
                        .padding(.vertical, 16)
                        .padding(.leading, 16)

                        VStack(spacing: 0) {
                            compositionLegendRow(color: .blue, label: "Formulae",
                                                 count: model.formulaCount, pct: formulaPct)
                            compositionLegendRow(color: .orange, label: "Casks",
                                                 count: model.caskCount, pct: caskPct)
                        }
                        .frame(maxWidth: .infinity)
                    }
                    .frame(maxHeight: .infinity, alignment: .top)
                } else {
                    // Stacked horizontal bar — single rounded track with two
                    // segments meeting flush (no gap), clipped to a capsule.
                    GeometryReader { geo in
                        HStack(spacing: 0) {
                            Rectangle().fill(.blue)
                                .frame(width: geo.size.width * formulaFraction)
                            Rectangle().fill(.orange)
                        }
                        .clipShape(.capsule)
                    }
                    .frame(height: 14)

                    HStack(spacing: 16) {
                        LegendDot(color: .blue, text: "\(model.formulaCount) formulae")
                        LegendDot(color: .orange, text: "\(model.caskCount) casks")
                        Spacer()
                    }
                }
            }
            .padding(.top, 2)
        }
    }

    /// Legend row matching the Top-categories format: dot · label · count · %.
    private func compositionLegendRow(color: Color, label: String, count: Int, pct: Double) -> some View {
        HStack(spacing: 10) {
            Circle().fill(color).frame(width: 10, height: 10)
            Text(label).font(.callout)
            Spacer(minLength: 12)
            Text("\(count)").font(.callout.monospacedDigit())
            Text(String(format: "%.1f%%", pct))
                .font(.caption.monospacedDigit())
                .foregroundStyle(.secondary)
                .frame(width: 48, alignment: .trailing)
        }
        .padding(.vertical, 5)
        .padding(.horizontal, 8)
    }
}

struct LegendDot: View {
    let color: Color
    let text: String
    var body: some View {
        HStack(spacing: 6) {
            RoundedRectangle(cornerRadius: 3).fill(color).frame(width: 11, height: 11)
            Text(text).font(.callout)
        }
    }
}

struct Chip: View {
    let text: String
    var body: some View {
        Text(text)
            .font(.caption)
            .padding(.horizontal, 9).padding(.vertical, 3)
            .background(.quaternary, in: .capsule)
    }
}

// MARK: - Top categories (donut + ranked legend, with hover)

struct CategoriesCard: View {
    @Bindable var model: AppModel
    @State private var hovered: String?

    /// Stable palette across slices + legend.
    private let palette: [Color] = [
        .blue, .orange, .green, .yellow, .purple, .pink, .teal, .red, .gray
    ]

    private func color(_ index: Int) -> Color { palette[index % palette.count] }

    var body: some View {
        GroupBox {
            VStack(alignment: .leading, spacing: 12) {
                Text("Top categories in your library").font(.headline)
                    .frame(maxWidth: .infinity, alignment: .leading)

                HStack(alignment: .center, spacing: 20) {
                    donut
                        .frame(width: 180, height: 180)
                        .padding(.vertical, 16)
                        .padding(.leading, 16)

                    VStack(spacing: 0) {
                        ForEach(Array(model.categories.enumerated()), id: \.element.id) { idx, cat in
                            legendRow(idx: idx, cat: cat)
                        }
                    }
                    .frame(maxWidth: .infinity)
                }
            }
            .padding(.top, 2)
        }
    }

    private var donut: some View {
        Chart(Array(model.categories.enumerated()), id: \.element.id) { idx, cat in
            SectorMark(
                angle: .value("Count", cat.count),
                innerRadius: .ratio(0.6),
                angularInset: 1.5
            )
            .cornerRadius(3)
            .foregroundStyle(color(idx))
            .opacity(hovered == nil || hovered == cat.slug ? 1 : 0.3)
        }
        .chartLegend(.hidden)
    }

    private func legendRow(idx: Int, cat: CategoryBreakdown) -> some View {
        HStack(spacing: 10) {
            Circle().fill(color(idx)).frame(width: 10, height: 10)
            // Per-category glyph (matches the Tauri legend's icon column).
            Image(systemName: cat.icon)
                .font(.caption)
                .foregroundStyle(.secondary)
                .frame(width: 16)
            Text(cat.label)
                .font(.callout)
                .fontWeight(hovered == cat.slug ? .semibold : .regular)
            Spacer(minLength: 12)
            Text("\(cat.count)")
                .font(.callout.monospacedDigit())
            Text(String(format: "%.1f%%", cat.fraction * 100))
                .font(.caption.monospacedDigit())
                .foregroundStyle(.secondary)
                .frame(width: 48, alignment: .trailing)
        }
        .padding(.vertical, 5)
        .padding(.horizontal, 8)
        .background(
            hovered == cat.slug ? Color.secondary.opacity(0.12) : .clear,
            in: .rect(cornerRadius: 6)
        )
        .contentShape(.rect)
        .onHover { inside in
            hovered = inside ? cat.slug : (hovered == cat.slug ? nil : hovered)
        }
    }
}

// MARK: - Storage (full breakdown)

struct StorageCard: View {
    @Bindable var model: AppModel

    private func human(_ bytes: Int64) -> String {
        let gb = Double(bytes) / 1_073_741_824
        if gb >= 1 { return String(format: "%.2f GB", gb) }
        let mb = Double(bytes) / 1_048_576
        return String(format: "%.0f MB", mb)
    }

    var body: some View {
        GroupBox {
            VStack(spacing: 0) {
                HStack {
                    Text("Storage").font(.headline)
                    Spacer()
                    Text("\(human(model.storageTotalBytes)) total")
                        .font(.callout).foregroundStyle(.secondary)
                }
                .padding(.bottom, 8)

                ForEach(model.storage) { item in
                    HStack(spacing: 12) {
                        Text(item.label).frame(width: 160, alignment: .leading)
                        Text(item.path)
                            .font(.callout.monospaced()).foregroundStyle(.secondary)
                            .lineLimit(1).truncationMode(.middle)
                        Spacer()
                        Text(human(item.bytes)).font(.callout.monospaced())
                        Button {
                            NSWorkspace.shared.open(URL(fileURLWithPath: item.path))
                        } label: {
                            Image(systemName: "folder")
                        }
                        .buttonStyle(.borderless)
                    }
                    .padding(.vertical, 7)
                    if item.id != model.storage.last?.id { Divider() }
                }
            }
            .padding(.top, 2)
        }
    }
}

#if DEBUG
#Preview("Dashboard") {
    DashboardView(model: .preview())
        .frame(width: 980, height: 800)
}
#endif
