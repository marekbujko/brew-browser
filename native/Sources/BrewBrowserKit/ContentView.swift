import SwiftUI
import AppKit

/// Sets the Dock / ⌘-Tab icon at runtime from the bundled `AppIcon.icns`.
/// Needed because the bare `swift build` / Xcode ⌘R binary has no `.app`
/// Info.plist icon — only the `build-app.sh` bundle does. Loading from
/// `Bundle.module` makes the real icon show in BOTH run paths. Call once at
/// launch (from the executable's AppDelegate).
@MainActor
public func applyDockIcon() {
    guard let url = Bundle.module.url(forResource: "AppIcon", withExtension: "icns"),
          let image = NSImage(contentsOf: url) else { return }
    NSApplication.shared.applicationIconImage = image
}

/// Root chrome — stock `NavigationSplitView`, no overrides. Apple renders the
/// sidebar, the unified title bar, the toolbar, and all materials. We don't
/// touch the window, tint, transparency, or title bar.
public struct ContentView: View {
    @State private var model = AppModel()

    public init() {}

    public var body: some View {
      VStack(spacing: 0) {
        NavigationSplitView {
            List(Section.allCases, selection: $model.selection) { section in
                Label(section.rawValue, systemImage: section.symbol)
                    .badge(model.badge(for: section) ?? 0)  // stock count badge; 0 hides it
                    .tag(section)
            }
            .navigationTitle("brew-browser")
            // Wider sidebar so section labels + count badges never crowd, and
            // there's room for the toolbar's icon+text mode without overflow.
            .navigationSplitViewColumnWidth(min: 220, ideal: 240, max: 300)
        } detail: {
            detail
                // Give the main pane a firm minimum width so dragging the
                // inspector divider takes space from the content down to this
                // floor and then STOPS — instead of collapsing the pane (which
                // the window then grabs as a resize, hiding the inspector).
                // Pairs with .windowResizability(.contentMinSize) on the scene.
                .frame(minWidth: 420, maxWidth: .infinity, maxHeight: .infinity)
                .navigationTitle(model.selection.rawValue)
                .toolbar {
                    ToolbarItemGroup(placement: .primaryAction) {
                        Button {
                            Task { await model.refresh() }
                        } label: {
                            Label("Refresh", systemImage: "arrow.clockwise")
                        }
                        .disabled(model.isLoading)

                        Button {
                            // Install new package — wired in full port.
                        } label: {
                            Label("Add", systemImage: "plus")
                        }

                        // Stock SettingsLink opens the native Settings scene.
                        SettingsLink {
                            Label("Settings", systemImage: "gearshape")
                        }
                    }
                }
                // Search lives on the DETAIL column (the main content area), not
                // the split-view root — so it stays in the content toolbar and
                // doesn't drift over the inspector (and the inspector's boundary
                // divider falls cleanly at the panel edge, not through the
                // toolbar between our icons and the field).
                .searchable(text: $model.globalQuery, placement: .toolbar, prompt: "Search packages")
                .searchSuggestions {
                    ForEach(model.suggestions) { pkg in
                        HStack(spacing: 8) {
                            PackageIcon(model: model, token: pkg.name, kind: pkg.kind, size: 16)
                            Text(pkg.name)
                        }
                        .searchCompletion(pkg.name)
                    }
                }
                .onSubmit(of: .search) {
                    if let match = model.installed.first(where: {
                        $0.name.caseInsensitiveCompare(model.globalQuery) == .orderedSame
                    }) ?? model.suggestions.first {
                        model.openInLibrary(match)
                    }
                }
                // Package detail — stock right-side inspector, on the detail column.
                .inspector(isPresented: Binding(
                    get: { model.showDetail },
                    // A drag-collapse flips this to false but KEEPS the loaded
                    // package, so re-expanding mid-gesture restores content. The
                    // ⊗ close box calls closeDetail() to actually clear the data.
                    set: { model.showDetail = $0 }
                )) {
                    // The inspector must present a STABLE size contract while the
                    // user drags the divider. Putting .inspectorColumnWidth on an
                    // always-present Group (not on the conditionally-created
                    // PackageDetailView) and letting the content fill the column
                    // stops the hosting view from re-reporting min/max mid-drag.
                    // That re-report otherwise triggers a re-entrant NSWindow
                    // constraint update during -[NSSplitView mouseDown:], which
                    // AppKit aborts (SIGABRT) — or resolves by resizing the
                    // window / hiding the inspector instead of resizing it.
                    //
                    // NOTE: stock `.inspector` dismisses itself when the divider
                    // is dragged below `min` — that's Apple's built-in collapse
                    // behavior, not removable without fighting NSSplitView. We set
                    // a generous `min` (360) so collapsing takes a deliberate hard
                    // drag rather than an accidental nudge; the in-panel close box
                    // (xmark.circle in PackageDetailView) is the intended dismiss.
                    Group {
                        if let pkg = model.detailPackage {
                            PackageDetailView(model: model, pkg: pkg)
                                .frame(maxWidth: .infinity, maxHeight: .infinity)
                        }
                    }
                    .inspectorColumnWidth(min: 360, ideal: 400, max: 560)
                }
        }
        // Activity drawer as a true full-width bottom bar BELOW the whole split
        // view (sibling, not an inset/overlay) — so the split view + the
        // inspector's own footer live entirely above it and nothing is covered.
        ActivityDrawer(model: model)
      }
      .task {
            model.loadJobs()
            if model.installed.isEmpty { await model.loadLibrary() }
      }
    }

    @ViewBuilder
    private var detail: some View {
        switch model.selection {
        case .dashboard:
            DashboardView(model: model)
        case .library:
            LibraryView(model: model)
        case .discover:
            DiscoverView(model: model)
        case .trending:
            TrendingView(model: model)
        case .activity:
            ActivityView(model: model)
        case .snapshots:
            SnapshotsView(model: model)
        default:
            PlaceholderView(section: model.selection)
        }
    }
}

/// Installed packages — a stock SwiftUI `Table` (the macOS sortable
/// multi-column control: click-to-sort headers, resizable columns, native
/// selection). A segmented type filter sits in a thin bar above it. Selection
/// drives the shared package-detail inspector. No custom row chrome.
struct LibraryView: View {
    @Bindable var model: AppModel

    /// The table's selection — the package `id` (name). Mirrors the inspector's
    /// open package so the highlighted row tracks the detail panel.
    @State private var selectedID: LibraryRow.ID?

    var body: some View {
        Group {
            if model.isLoading && model.installed.isEmpty {
                ProgressView("Reading your Homebrew install…")
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let err = model.loadError {
                ContentUnavailableView(
                    "Couldn't load packages",
                    systemImage: "exclamationmark.triangle",
                    description: Text(err)
                )
            } else {
                VStack(spacing: 0) {
                    filterBar
                    Divider()
                    table
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
            }
        }
        // NOTE: no `.searchable` here — Library filters off the shared toolbar
        // search field (`globalQuery`, declared on the detail column). A second
        // `.searchable` would register a duplicate toolbar search item and crash
        // AppKit on layout ("NSToolbar already contains com.apple.SwiftUI.search").
        // Keep the table highlight in sync with the inspector: if detail is
        // closed elsewhere (⊗ box), clear the row selection too.
        .onChange(of: model.showDetail) { _, shown in
            if !shown { selectedID = nil }
        }
    }

    // Segmented type filter with per-filter counts. Stock control, no overrides.
    // Centered in the bar — the macOS view-switcher convention (Finder/Preview).
    private var filterBar: some View {
        Picker("Filter", selection: $model.libraryFilter) {
            ForEach(LibraryFilter.allCases) { f in
                Text("\(f.rawValue) (\(model.libraryFilterCount(f)))").tag(f)
            }
        }
        .pickerStyle(.segmented)
        .labelsHidden()
        .fixedSize()
        .frame(maxWidth: .infinity, alignment: .center)
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    @ViewBuilder
    private var table: some View {
        if model.sortedLibraryRows.isEmpty {
            if model.globalQuery.isEmpty {
                ContentUnavailableView(
                    "No packages",
                    systemImage: "shippingbox",
                    description: Text("Nothing matches the \(model.libraryFilter.rawValue.lowercased()) filter.")
                )
            } else {
                ContentUnavailableView.search(text: model.globalQuery)
            }
        } else {
            // Two fixed-column-set variants instead of a conditional
            // `TableColumn` inside one builder: wrapping a column in `if`
            // makes the underlying NSTableColumn set unstable and AppKit
            // throws an NSException mid-layout (SIGTRAP via _crashOnException).
            // The AI-gated Description column lives in its own static variant.
            if model.settings.aiFeaturesVisible {
                tableWithDescription
            } else {
                tableNoDescription
            }
        }
    }

    private var tableWithDescription: some View {
        Table(model.sortedLibraryRows, selection: $selectedID, sortOrder: $model.librarySort) {
            TableColumn("Name", value: \.name) { row in
                HStack(spacing: 8) {
                    PackageIcon(model: model, token: row.name, kind: row.kind)
                    VStack(alignment: .leading, spacing: 1) {
                        Text(row.name)
                        if !row.friendlyName.isEmpty {
                            Text(row.friendlyName).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                        }
                    }
                }
            }
            .width(min: 140, ideal: 200)

            TableColumn("Description", value: \.summary) { row in
                Text(row.summary).foregroundStyle(.secondary).lineLimit(1)
            }
            .width(min: 160, ideal: 320)

            TableColumn("Version", value: \.version) { row in
                Text(row.version).foregroundStyle(.secondary).monospacedDigit()
            }
            .width(min: 80, ideal: 120)

            TableColumn("Type", value: \.kind.rawValue) { row in
                KindPill(kind: row.kind)
            }
            .width(min: 64, ideal: 80)

            TableColumn("Outdated", value: \.outdatedRank) { row in
                outdatedCell(row)
            }
            .width(min: 56, ideal: 72)
        }
        .onChange(of: selectedID, openSelected)
    }

    private var tableNoDescription: some View {
        Table(model.sortedLibraryRows, selection: $selectedID, sortOrder: $model.librarySort) {
            TableColumn("Name", value: \.name) { row in
                HStack(spacing: 8) {
                    PackageIcon(model: model, token: row.name, kind: row.kind)
                    VStack(alignment: .leading, spacing: 1) {
                        Text(row.name)
                        if !row.friendlyName.isEmpty {
                            Text(row.friendlyName).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                        }
                    }
                }
            }
            .width(min: 140, ideal: 240)

            TableColumn("Version", value: \.version) { row in
                Text(row.version).foregroundStyle(.secondary).monospacedDigit()
            }
            .width(min: 80, ideal: 120)

            TableColumn("Type", value: \.kind.rawValue) { row in
                KindPill(kind: row.kind)
            }
            .width(min: 64, ideal: 80)

            TableColumn("Outdated", value: \.outdatedRank) { row in
                outdatedCell(row)
            }
            .width(min: 56, ideal: 72)
        }
        .onChange(of: selectedID, openSelected)
    }

    @ViewBuilder
    private func outdatedCell(_ row: LibraryRow) -> some View {
        if row.isOutdated {
            Image(systemName: "arrow.up.circle.fill")
                .foregroundStyle(.orange)
                .help("Update available")
        }
    }

    private func openSelected() {
        guard let id = selectedID,
              let pkg = model.installed.first(where: { $0.id == id }) else { return }
        model.openDetail(pkg)
    }
}

#if DEBUG
#Preview("Library") {
    LibraryView(model: .preview())
        .frame(width: 820, height: 560)
}
#endif

struct PlaceholderView: View {
    let section: Section

    var body: some View {
        ContentUnavailableView(
            section.rawValue,
            systemImage: section.symbol,
            description: Text("Wired in the full port. The spike proves Dashboard + Library end-to-end.")
        )
    }
}
