import SwiftUI

/// The sidebar sections, mirroring the shipped app's navigation. Spike wires up
/// Library for real; the rest are placeholders to prove the NavigationSplitView
/// chrome and selection model.
enum Section: String, CaseIterable, Identifiable, Hashable {
    case dashboard = "Dashboard"
    case library   = "Library"
    case discover  = "Discover"
    case trending  = "Trending"
    case snapshots = "Snapshots"
    case services  = "Services"
    case activity  = "Activity"

    var id: String { rawValue }

    /// SF Symbol for the sidebar row — all system symbols, no custom assets.
    var symbol: String {
        switch self {
        case .dashboard: return "square.grid.2x2"
        case .library:   return "books.vertical"
        case .discover:  return "sparkles.rectangle.stack"
        case .trending:  return "chart.line.uptrend.xyaxis"
        case .snapshots: return "camera"
        case .services:  return "gearshape.2"
        case .activity:  return "list.bullet.rectangle"
        }
    }
}

/// Library type filter — drives the segmented control above the Library table.
/// Mirrors the Tauri Library pill set minus "vulnerable" (which needs a
/// library-wide scan-all, still deferred on the native side).
enum LibraryFilter: String, CaseIterable, Identifiable, Hashable {
    case all      = "All"
    case formulae = "Formulae"
    case casks    = "Casks"
    case outdated = "Outdated"

    var id: String { rawValue }
}

/// A flattened Library table row. Built in `AppModel.libraryRows` so the
/// `Table` columns are pure value reads (name/version/kind + the precomputed
/// outdated flag and AI-gated enrichment summary).
struct LibraryRow: Identifiable, Hashable, Sendable {
    var id: String { name }
    let name: String
    let version: String
    let kind: InstalledPackage.Kind
    let isOutdated: Bool
    let summary: String

    /// Comparable proxy for sorting the Outdated column (`Bool` isn't
    /// `Comparable`). Outdated rows sort high so descending surfaces them first.
    var outdatedRank: Int { isOutdated ? 1 : 0 }
}

@MainActor
@Observable
final class AppModel {
    var selection: Section = .dashboard
    var installed: [InstalledPackage] = []
    var isLoading = false
    var loadError: String?

    /// Global toolbar search text (the single `.searchable` field on the detail
    /// column). Drives both the toolbar suggestions and the Library table
    /// filter — there is exactly one search field in the toolbar.
    var globalQuery: String = ""

    /// Library type filter — the segmented control above the table. Dashboard
    /// entry points set `.outdated` (Updates) or `.all` (installed / search).
    var libraryFilter: LibraryFilter = .all

    /// Sort order for the Library `Table`, bound to its `sortOrder:`. Defaults
    /// to ascending by name (the stock first-column-ascending convention).
    var librarySort: [KeyPathComparator<LibraryRow>] = [
        KeyPathComparator(\LibraryRow.name, order: .forward)
    ]

    /// Set of outdated package names for O(1) per-row tagging.
    private var outdatedNames: Set<String> { Set(outdated.map(\.name)) }

    /// The Library rows after the type filter + text query, before sorting.
    /// Each row carries its outdated flag and (AI-gated) enrichment summary so
    /// the `Table` columns are pure value reads.
    var libraryRows: [LibraryRow] {
        let outdatedSet = outdatedNames
        let showSummary = settings.aiFeaturesVisible
        // Filter off the single shared toolbar search field (`globalQuery`).
        // Library must NOT add its own `.searchable` — two .searchable in one
        // toolbar both claim the `com.apple.SwiftUI.search` item id and AppKit
        // throws "NSToolbar already contains an item…" → SIGTRAP on layout.
        let q = globalQuery.trimmingCharacters(in: .whitespaces)

        return installed.compactMap { pkg in
            switch libraryFilter {
            case .all:      break
            case .formulae: guard pkg.kind == .formula else { return nil }
            case .casks:    guard pkg.kind == .cask else { return nil }
            case .outdated: guard outdatedSet.contains(pkg.name) else { return nil }
            }
            if !q.isEmpty, !pkg.name.localizedCaseInsensitiveContains(q) { return nil }
            return LibraryRow(
                name: pkg.name,
                version: pkg.version,
                kind: pkg.kind,
                isOutdated: outdatedSet.contains(pkg.name),
                summary: showSummary ? (enrichment?.entry(for: pkg.name)?.summary ?? "") : ""
            )
        }
    }

    /// `libraryRows` with the table's current `sortOrder` applied.
    var sortedLibraryRows: [LibraryRow] {
        libraryRows.sorted(using: librarySort)
    }

    /// Per-filter row counts for the segmented control labels.
    func libraryFilterCount(_ filter: LibraryFilter) -> Int {
        switch filter {
        case .all:      return installed.count
        case .formulae: return installed.lazy.filter { $0.kind == .formula }.count
        case .casks:    return installed.lazy.filter { $0.kind == .cask }.count
        case .outdated: return outdatedNames.count
        }
    }

    /// Type-ahead suggestions for the toolbar search — top installed matches.
    var suggestions: [InstalledPackage] {
        let q = globalQuery.trimmingCharacters(in: .whitespaces)
        guard !q.isEmpty else { return [] }
        return Array(
            installed
                .filter { $0.name.localizedCaseInsensitiveContains(q) }
                .prefix(8)
        )
    }

    /// Commit a search selection: jump to Library, pre-filtered to that name
    /// via the shared toolbar search field.
    func openInLibrary(_ pkg: InstalledPackage) {
        globalQuery = pkg.name
        libraryFilter = .all
        selection = .library
    }

    /// Open the full Library (cleared filters). Used by the "installed" stat.
    func openLibrary() {
        globalQuery = ""
        libraryFilter = .all
        selection = .library
    }

    /// Open Library filtered to outdated packages. Used by the Updates stat/card.
    func openOutdatedInLibrary() {
        globalQuery = ""
        libraryFilter = .outdated
        selection = .library
    }

    // ---- Dashboard stats (all from real brew data) ----
    var formulaCount = 0
    var caskCount = 0
    var leavesCount = 0
    var onRequestCount = 0
    var pinnedCount = 0
    var outdatedCount = 0
    var outdated: [OutdatedPackage] = []
    var storage: [StorageItem] = []
    var brewVersion = "—"
    var brewPrefix = "/opt/homebrew"
    var categories: [CategoryBreakdown] = []
    var runningServices = 0
    var dashboardLoaded = false

    // ---- Package detail (inspector) ----
    /// The package whose detail is loaded. This is the DATA, decoupled from
    /// presentation: it's only set by `openDetail` and only cleared by
    /// `closeDetail` (the ⊗ close box). A drag-collapse of the inspector must
    /// NOT clear it, or re-expanding within the same gesture shows an empty pane.
    var detailPackage: InstalledPackage?
    /// Inspector PRESENTATION flag — what the `.inspector(isPresented:)` binding
    /// reads/writes. A divider drag past `min` flips this to false (collapse)
    /// while `detailPackage` survives, so the panel can re-expand with content.
    var showDetail = false
    var detailInfo: PackageInfo?
    var detailEnrichment: EnrichmentEntry?
    var detailCategories: [String] = []        // category labels for this package
    var detailLoading = false
    var detailError: String?
    /// Streaming action state (upgrade/uninstall/install) for the footer.
    var actionRunning = false
    var actionLabel: String?

    // ---- Vulnerabilities (Security card) ----
    var detailVulns: [VulnFinding] = []
    var detailVulnsScanned = false
    var detailVulnsLoading = false
    var brewVulnsInstalled = false

    // ---- Install trend ----
    var detailTrend: TrendingHistorySeries?

    // ---- GitHub ----
    var detailRepoStats: RepoStats?
    var detailStarred: Bool?
    var githubStatus: GithubStatus?

    private let catalog = CategoryCatalog.loadBundled()
    private let enrichment = EnrichmentCatalog.loadBundled()
    let settings = AppSettings.shared
    private let vulns = VulnsService()
    private let trendingHistory = TrendingHistoryService()
    private let githubService = GitHubService()

    init() {}

    /// Count badge for a sidebar section (nil = no badge). Library shows the
    /// outdated count; Services shows running services.
    func badge(for section: Section) -> Int? {
        switch section {
        case .library:  return outdatedCount > 0 ? outdatedCount : nil
        case .services: return runningServices > 0 ? runningServices : nil
        default:        return nil
        }
    }

    /// Dependencies = installed formulae that weren't explicitly requested.
    var dependencyCount: Int { max(0, formulaCount - leavesCount) }
    var totalPackages: Int { formulaCount + caskCount }

    /// First 5 outdated packages for the Dashboard preview list.
    var outdatedPreview: [OutdatedPackage] { Array(outdated.prefix(5)) }

    /// Total bytes across all storage categories.
    var storageTotalBytes: Int64 { storage.reduce(0) { $0 + $1.bytes } }

    private let brew = BrewService()

    func loadLibrary() async {
        isLoading = true
        loadError = nil
        do {
            // Library lists BOTH formulae and casks (the Casks filter was empty
            // because we only loaded formulae). Dashboard's formula/cask counts
            // come from their own brew calls in loadDashboard.
            installed = try await brew.listInstalledAll()
            formulaCount = installed.lazy.filter { $0.kind == .formula }.count
            caskCount = installed.lazy.filter { $0.kind == .cask }.count
        } catch {
            loadError = error.localizedDescription
        }
        isLoading = false
    }

    /// Load all Dashboard stats. Reuses the Library load for the formula count
    /// (and triggers it if it hasn't run), then fans out the cheap counts.
    func loadDashboard() async {
        if installed.isEmpty { await loadLibrary() } else { formulaCount = installed.lazy.filter { $0.kind == .formula }.count }
        async let casks = try? brew.countCasks()
        async let leaves = try? brew.countLeaves()
        async let onRequest = try? brew.countOnRequest()
        async let pinned = try? brew.countPinned()
        async let outdatedList = try? brew.outdatedPackages()
        async let storageList = brew.storageBreakdown()
        async let ver = brew.version()
        async let pfx = brew.prefix()
        async let services = brew.countRunningServices()

        caskCount = await casks ?? 0
        leavesCount = await leaves ?? 0
        onRequestCount = await onRequest ?? 0
        pinnedCount = await pinned ?? 0
        outdated = await outdatedList ?? []
        outdatedCount = outdated.count
        storage = await storageList
        brewVersion = await ver
        brewPrefix = await pfx
        categories = catalog?.breakdown(installed: installed) ?? []
        runningServices = await services
        dashboardLoaded = true
    }

    /// Toolbar Refresh — reload whichever surface is showing.
    func refresh() async {
        await loadLibrary()
        dashboardLoaded = false
        await loadDashboard()
    }

    // MARK: - Package detail

    /// Open the inspector for a package and load its full detail.
    func openDetail(_ pkg: InstalledPackage) {
        detailPackage = pkg
        showDetail = true
        Task { await loadDetail(pkg) }
    }

    func closeDetail() {
        showDetail = false
        detailPackage = nil
        detailInfo = nil
        detailEnrichment = nil
        detailCategories = []
        detailVulns = []
        detailVulnsScanned = false
        detailTrend = nil
        detailRepoStats = nil
        detailStarred = nil
        detailError = nil
    }

    /// Load `brew info` + bundled enrichment/categories for the package, then
    /// kick off the opt-in network sections (trend, github) per settings gates.
    func loadDetail(_ pkg: InstalledPackage) async {
        detailLoading = true
        detailError = nil
        detailInfo = nil
        detailVulns = []
        detailVulnsScanned = false
        detailTrend = nil
        detailRepoStats = nil
        detailStarred = nil

        // Bundled, synchronous-ish lookups first (instant).
        detailEnrichment = settings.aiFeaturesVisible ? enrichment?.entry(for: pkg.name) : nil
        detailCategories = catalog?.categoryLabels(for: pkg.name, kind: pkg.kind) ?? []

        do {
            let info = try await brew.info(name: pkg.name, kind: pkg.kind)
            // Guard against a stale load if the user clicked another package.
            guard detailPackage?.id == pkg.id else { return }
            detailInfo = info
        } catch {
            guard detailPackage?.id == pkg.id else { return }
            detailError = error.localizedDescription
        }
        detailLoading = false

        // Opt-in network sections — fire-and-forget, gated by settings.
        if settings.enhancedTrendingAllowed {
            Task { await loadTrend(pkg) }
        }
        if settings.githubAllowed, let hp = detailInfo?.homepage {
            Task { await loadGitHub(homepage: hp, pkgId: pkg.id) }
        }
        // Probe brew-vulns availability so the Security card shows the right CTA.
        if settings.vulnerabilityScanningAllowed {
            let installed = await vulns.isBrewVulnsInstalled()
            guard detailPackage?.id == pkg.id else { return }
            brewVulnsInstalled = installed
        }
    }

    func loadTrend(_ pkg: InstalledPackage) async {
        let series = await trendingHistory.series(name: pkg.name, isCask: pkg.kind == .cask)
        guard detailPackage?.id == pkg.id else { return }
        detailTrend = series
    }

    func loadGitHub(homepage: String, pkgId: String) async {
        let stats = try? await githubService.repoStats(homepage: homepage)
        guard detailPackage?.id == pkgId else { return }
        detailRepoStats = stats
        let status = await githubService.status()
        if status.signedIn {
            let starred = try? await githubService.isStarred(homepage: homepage)
            guard detailPackage?.id == pkgId else { return }
            detailStarred = starred
        }
    }

    /// Run a `brew vulns` scan for the detail package (Security card "Check now").
    func scanDetailVulns() async {
        guard let pkg = detailPackage else { return }
        detailVulnsLoading = true
        let findings = (try? await vulns.scanOne(name: pkg.name, isCask: pkg.kind == .cask)) ?? []
        guard detailPackage?.id == pkg.id else { return }
        detailVulns = findings
        detailVulnsScanned = true
        detailVulnsLoading = false
    }

    // MARK: - Detail actions

    func upgradeDetail() async {
        guard let pkg = detailPackage else { return }
        await runAction("Upgrading \(pkg.name)") { try await self.brew.upgrade(pkg.name) }
    }

    func uninstallDetail() async {
        guard let pkg = detailPackage else { return }
        await runAction("Uninstalling \(pkg.name)") {
            try await self.brew.uninstall(pkg.name, kind: pkg.kind)
        }
        closeDetail()
    }

    private func runAction(_ label: String, _ work: @escaping () async throws -> Void) async {
        actionRunning = true
        actionLabel = label
        do { try await work() } catch { detailError = error.localizedDescription }
        actionRunning = false
        actionLabel = nil
        // Refresh installed/outdated state after a mutation.
        await refresh()
    }
}
