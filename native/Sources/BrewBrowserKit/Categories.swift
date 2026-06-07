import Foundation

/// Loads the bundled `categories.json` (package→category-slug map) and computes
/// the "Top categories in your library" breakdown from installed packages —
/// the real data behind the Dashboard donut, same source as the Tauri app.
struct CategoryBreakdown: Identifiable, Hashable, Sendable {
    var id: String { slug }
    let slug: String
    let label: String
    let count: Int
    let fraction: Double
    /// SF Symbol name for the category, read straight from `categories.json`'s
    /// `iconSF` field (chosen once in `tools/categorize/categorize.py`). No
    /// per-icon mapping in UI code — the data decides the glyph.
    var icon: String = "questionmark.circle"
}

/// One Discover category tile: glyph + label + the number of catalog packages in
/// the category. Backs the Discover browse grid (mirrors the Tauri
/// `CategoryTile` in `src/lib/stores/categories.svelte.ts`).
struct CategoryTile: Identifiable, Hashable, Sendable {
    var id: String { slug }
    let slug: String
    let label: String
    /// SF Symbol from `categories.json` `iconSF` (data-driven, no UI mapping).
    let icon: String
    let count: Int
}

struct CategoryCatalog: Sendable {
    /// slug → display label
    private let labels: [String: String]
    /// slug → SF Symbol name (from `categories.json` `iconSF`)
    private let sfIcons: [String: String]
    /// package name → [slug]
    private let formulae: [String: [String]]
    private let casks: [String: [String]]

    /// Decode the bundled JSON. Returns nil if the resource is missing or
    /// malformed (the Dashboard then just hides the categories card).
    static func loadBundled() -> CategoryCatalog? {
        guard let url = Bundle.module.url(forResource: "categories", withExtension: "json"),
              let data = try? Data(contentsOf: url)
        else { return nil }
        return parse(data: data)
    }

    /// Parse a `categories.json` blob — bundled OR live-fetched from the
    /// `…/enrichment/categories.json` endpoint — into a catalog.
    static func parse(data: Data) -> CategoryCatalog? {
        guard let root = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return nil }

        var labels: [String: String] = [:]
        var sfIcons: [String: String] = [:]
        if let cats = root["categories"] as? [String: Any] {
            for (slug, v) in cats {
                guard let obj = v as? [String: Any] else { continue }
                if let label = obj["label"] as? String { labels[slug] = label }
                if let sf = obj["iconSF"] as? String { sfIcons[slug] = sf }
            }
        }
        let formulae = (root["formulae"] as? [String: [String]]) ?? [:]
        let casks = (root["casks"] as? [String: [String]]) ?? [:]
        return CategoryCatalog(labels: labels, sfIcons: sfIcons, formulae: formulae, casks: casks)
    }

    /// Category slugs for an installed package (formula first, then cask map).
    private func slugs(for name: String, kind: InstalledPackage.Kind) -> [String] {
        let map = kind == .cask ? casks : formulae
        return map[name] ?? []
    }

    /// Display labels for a single package's categories (for the detail panel
    /// pills), excluding the "uncategorized" bucket.
    func categoryLabels(for name: String, kind: InstalledPackage.Kind) -> [String] {
        slugs(for: name, kind: kind)
            .filter { $0 != "uncategorized" }
            .map { labels[$0] ?? $0.capitalized }
    }

    /// All known categories as (slug, label), alphabetised by label — powers the
    /// Discover category Picker. Excludes the "uncategorized" bucket.
    func allCategories() -> [(slug: String, label: String)] {
        labels
            .filter { $0.key != "uncategorized" }
            .map { (slug: $0.key, label: $0.value) }
            .sorted { $0.label.localizedCaseInsensitiveCompare($1.label) == .orderedAscending }
    }

    /// True if a package (by token + kind) belongs to the given category slug —
    /// the Discover category filter predicate.
    func isMember(token: String, kind: InstalledPackage.Kind, slug: String) -> Bool {
        slugs(for: token, kind: kind).contains(slug)
    }

    /// Category tiles for the Discover browse grid: every known category with its
    /// glyph + the count of catalog packages (formulae + casks) in it, sorted by
    /// descending count. Mirrors the Tauri `categories.tiles` derivation
    /// (`src/lib/stores/categories.svelte.ts:62-81`); `uncategorized` is excluded
    /// (the Tauri grid sinks it to last, native simply omits the noise bucket).
    func tiles() -> [CategoryTile] {
        var counts: [String: Int] = [:]
        for cats in formulae.values { for c in cats { counts[c, default: 0] += 1 } }
        for cats in casks.values { for c in cats { counts[c, default: 0] += 1 } }
        return labels
            .filter { $0.key != "uncategorized" }
            .map { slug, label in
                CategoryTile(slug: slug, label: label,
                             icon: sfIcons[slug] ?? "questionmark.circle",
                             count: counts[slug] ?? 0)
            }
            .sorted { $0.count > $1.count }
    }

    /// Top-N category breakdown across the installed set. Each package
    /// contributes 1 to each of its categories (multi-membership), matching the
    /// Tauri model. "uncategorized" is folded into an "Other" bucket along with
    /// the long tail beyond `top`.
    func breakdown(installed: [InstalledPackage], top: Int = 8) -> [CategoryBreakdown] {
        var counts: [String: Int] = [:]
        for pkg in installed {
            for slug in slugs(for: pkg.name, kind: pkg.kind) {
                counts[slug, default: 0] += 1
            }
        }
        let uncategorized = counts.removeValue(forKey: "uncategorized") ?? 0
        let totalMemberships = max(1, counts.values.reduce(0, +) + uncategorized)

        let ranked = counts.sorted { $0.value > $1.value }
        var result: [CategoryBreakdown] = []
        for (slug, count) in ranked.prefix(top) {
            result.append(CategoryBreakdown(
                slug: slug,
                label: labels[slug] ?? slug.capitalized,
                count: count,
                fraction: Double(count) / Double(totalMemberships),
                icon: sfIcons[slug] ?? "questionmark.circle"
            ))
        }
        let tail = ranked.dropFirst(top).reduce(0) { $0 + $1.value } + uncategorized
        if tail > 0 {
            result.append(CategoryBreakdown(
                slug: "other",
                label: "Other",
                count: tail,
                fraction: Double(tail) / Double(totalMemberships),
                icon: "questionmark.circle"
            ))
        }
        return result
    }
}
