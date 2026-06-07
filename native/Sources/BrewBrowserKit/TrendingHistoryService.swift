//
//  TrendingHistoryService.swift
//  BrewBrowser
//
//  Native Swift port of the per-package "Install trend" sparkline shown in the
//  package detail panel. Fetches a historical install-count series from the
//  project's own infrastructure.
//
//  Trust boundary
//  --------------
//  This talks to first-party project infrastructure
//  (`https://brew-browser.zerologic.com/trending-history/...`), not to Homebrew
//  or any third party. It is an **opt-in**, non-load-bearing nicety: every code
//  path soft-fails to `nil` on any network or decode error so the detail panel
//  simply omits the sparkline rather than surfacing an error.
//
//  Rust source of truth
//  --------------------
//  Mirrors the Tauri/Rust implementation that this native app is porting:
//    - `src-tauri/src/trending/history/client.rs`
//        * endpoint layout for the per-package series + index
//        * the strict `name` allowlist used to reject path traversal
//          (per-package fetch, ~line 80)
//    - `src-tauri/src/commands/trending.rs`
//        * `trending_history_fetch` / `trending_history_index` commands
//    - `src/lib/types.ts`
//        * `TrendingHistoryPoint` / `TrendingHistorySeries` /
//          `TrendingHistoryIndexEntry`
//
//  JSON key casing (verified)
//  --------------------------
//  The wire format is **camelCase**, confirmed directly against the Rust serde
//  attributes in `src-tauri/src/types.rs`:
//    - The file-level doc comment states every DTO uses
//      `#[serde(rename_all = "camelCase")]`.
//    - `TrendingHistoryPoint` (line ~402) carries
//      `#[serde(rename_all = "camelCase")]`; its Rust fields
//      `count_30d` / `count_90d` / `count_365d` / `estimated_daily_installs`
//      therefore serialize as `count30d` / `count90d` / `count365d` /
//      `estimatedDailyInstalls`. The full set on the wire (per `src/lib/types.ts`
//      lines ~477-488) is: `date`, `count30d?`, `count90d?`, `count365d?`,
//      `countInstallOnRequest30d?`, `estimatedDailyInstalls?`, `source`.
//    - `TrendingHistorySeries` (line ~429) also camelCase: `name`, `kind`,
//      `points`, plus envelope fields `generatedAt` / `cacheAgeSeconds`.
//    - `TrendingHistoryIndexEntry` (line ~459): `name`, `kind`, `velocityIndex?`
//      (Rust `Option<f64>` → optional on the wire), `sparkline: [Double]`.
//
//  The `source` enum (`TrendingHistorySource`, line ~388) is
//  `#[serde(rename_all = "lowercase")]` → `"seed"` / `"daily"`. The URL kind
//  segments (`formula` / `cask`) match `PackageKind`'s
//  `#[serde(rename_all = "lowercase")]` (line ~14).
//
//  The `CodingKeys` below pin these names explicitly so decoding does not depend
//  on a global key-decoding strategy. `countInstallOnRequest30d` exists on the
//  wire but is intentionally not surfaced (Codable ignores unknown keys).
//

import Foundation

// MARK: - Wire types

/// One sample in a package's historical install-count series.
///
/// Mirrors `TrendingHistoryPoint` in `src/lib/types.ts`. All count fields are
/// optional because older "seed" points may not carry every window.
public struct TrendingHistoryPoint: Sendable, Hashable, Codable {
    /// Sample date, formatted `YYYY-MM-DD` (kept as a string to match the wire
    /// format exactly and avoid timezone ambiguity).
    public let date: String
    /// 30-day install count window.
    public let count30d: Int?
    /// 90-day install count window.
    public let count90d: Int?
    /// 365-day install count window.
    public let count365d: Int?
    /// Project-estimated installs for the day represented by this point.
    public let estimatedDailyInstalls: Int?
    /// Provenance of the point: `"seed"` (backfilled) or `"daily"` (live).
    public let source: String

    public init(
        date: String,
        count30d: Int?,
        count90d: Int?,
        count365d: Int?,
        estimatedDailyInstalls: Int?,
        source: String
    ) {
        self.date = date
        self.count30d = count30d
        self.count90d = count90d
        self.count365d = count365d
        self.estimatedDailyInstalls = estimatedDailyInstalls
        self.source = source
    }

    // camelCase wire keys (see file header re: serde rename_all = "camelCase").
    // `countInstallOnRequest30d` exists on the wire but is intentionally not
    // surfaced by this type; unknown keys are ignored by Codable.
    private enum CodingKeys: String, CodingKey {
        case date
        case count30d
        case count90d
        case count365d
        case estimatedDailyInstalls
        case source
    }
}

/// A package's full historical install-count series.
///
/// Mirrors `TrendingHistorySeries` in `src/lib/types.ts`. The `generatedAt` and
/// `cacheAgeSeconds` envelope fields are present on the wire but not retained
/// here, since the sparkline only needs the points.
public struct TrendingHistorySeries: Sendable, Hashable, Codable {
    /// Package name (formula or cask token).
    public let name: String
    /// `"formula"` or `"cask"`.
    public let kind: String
    /// Ordered samples; assumed chronological as delivered by the backend.
    public let points: [TrendingHistoryPoint]

    public init(name: String, kind: String, points: [TrendingHistoryPoint]) {
        self.name = name
        self.kind = kind
        self.points = points
    }

    private enum CodingKeys: String, CodingKey {
        case name
        case kind
        case points
    }
}

// MARK: - Service

/// Fetches per-package historical install-count series for the detail-panel
/// sparkline, plus the global index.
///
/// All methods soft-fail (`nil` / empty) on any error. This is an actor so the
/// (cheap, stateless) fetches are isolated and safe to call from anywhere in a
/// Swift 6 concurrency world.
public actor TrendingHistoryService {

    /// Base URL for the project's trending-history infrastructure.
    ///
    /// First-party, opt-in (see file header trust-boundary note).
    private let baseURL: URL
    private let session: URLSession

    /// - Parameters:
    ///   - baseURL: Override for the trending-history host (tests / staging).
    ///   - session: Override for the network session (tests). Defaults to a
    ///     short-timeout configuration so a stalled fetch never blocks the UI.
    public init(
        baseURL: URL = URL(string: "https://brew-browser.zerologic.com/trending-history/")!,
        session: URLSession? = nil
    ) {
        self.baseURL = baseURL
        if let session {
            self.session = session
        } else {
            let config = URLSessionConfiguration.ephemeral
            config.timeoutIntervalForRequest = 8
            config.timeoutIntervalForResource = 15
            self.session = URLSession(configuration: config)
        }
    }

    // MARK: Public API

    /// Fetch the historical install-count series for a single package.
    ///
    /// Returns `nil` on any failure (invalid name, network error, non-2xx
    /// status, or decode error) — the sparkline is a nicety, never a hard
    /// dependency.
    ///
    /// - Parameters:
    ///   - name: Package name. Validated against a strict allowlist; values
    ///     containing path separators or characters outside
    ///     `[a-zA-Z0-9._+@-]` are rejected (returns `nil`) to prevent path
    ///     traversal, mirroring the validation in
    ///     `src-tauri/src/trending/history/client.rs` (~line 80).
    ///   - isCask: `true` for casks (`/cask/...`), `false` for formulae
    ///     (`/formula/...`).
    public func series(name: String, isCask: Bool) async -> TrendingHistorySeries? {
        guard Self.isValidPackageName(name) else { return nil }

        let kindSegment = isCask ? "cask" : "formula"
        // Both segments are constrained: kindSegment is a literal and `name`
        // passed the strict allowlist, so neither can introduce traversal.
        let url = baseURL
            .appendingPathComponent(kindSegment, isDirectory: true)
            .appendingPathComponent("\(name).json", isDirectory: false)

        return await decode(TrendingHistorySeries.self, from: url)
    }

    /// Fetch the global trending-history index.
    ///
    /// Returns an empty array on any failure. Each entry carries a precomputed
    /// `sparkline` so the list view can render without per-package fetches.
    /// Mirrors `trending_history_index` in `src-tauri/src/commands/trending.rs`.
    public func index() async -> [TrendingHistoryIndexEntry] {
        let url = baseURL.appendingPathComponent("index.json", isDirectory: false)
        // The endpoint returns an OBJECT — {generatedAt, packages:[…],
        // cacheAgeSeconds} — not a bare array. Decode the wrapper and return
        // `.packages`. (Decoding a bare array silently failed → no velocity.)
        return await decode(TrendingHistoryIndex.self, from: url)?.packages ?? []
    }

    /// Convenience for the sparkline: the per-point values to plot.
    ///
    /// Prefers `estimatedDailyInstalls`, then `count30d`. Points where neither
    /// is present are dropped (rather than plotted as zero) so gaps in early
    /// "seed" data don't create misleading dips.
    public nonisolated func sparklineValues(_ series: TrendingHistorySeries) -> [Double] {
        series.points.compactMap { point in
            if let v = point.estimatedDailyInstalls { return Double(v) }
            if let v = point.count30d { return Double(v) }
            return nil
        }
    }

    // MARK: Validation

    /// Strict package-name allowlist mirroring the Rust path-traversal guard.
    ///
    /// Accepts only non-empty strings of `[a-zA-Z0-9._+@-]`. This rejects `/`,
    /// `\`, whitespace, `..` traversal sequences (the `.`/`/` combination), and
    /// any other character that could escape the intended URL path component.
    nonisolated static func isValidPackageName(_ name: String) -> Bool {
        guard !name.isEmpty else { return false }
        // Defense in depth: explicitly reject traversal even though the
        // character class below already forbids `/`.
        guard !name.contains("..") else { return false }
        let allowed = CharacterSet(charactersIn:
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._+@-")
        return name.unicodeScalars.allSatisfy { allowed.contains($0) }
    }

    // MARK: Networking

    /// Fetch + decode, soft-failing to `nil` on any error.
    private func decode<T: Decodable>(_ type: T.Type, from url: URL) async -> T? {
        do {
            let (data, response) = try await session.data(from: url)
            if let http = response as? HTTPURLResponse,
               !(200...299).contains(http.statusCode) {
                return nil
            }
            return try JSONDecoder().decode(T.self, from: data)
        } catch {
            // Non-load-bearing: any failure just hides the sparkline.
            return nil
        }
    }
}

// MARK: - Index entry

/// Wrapper for the `index.json` payload: `{generatedAt, packages, cacheAgeSeconds}`.
/// The leaderboard entries live under `packages`.
struct TrendingHistoryIndex: Sendable, Codable {
    let packages: [TrendingHistoryIndexEntry]
}

/// One row of the global trending-history index.
///
/// Mirrors `TrendingHistoryIndexEntry` in `src/lib/types.ts`. The `sparkline`
/// is precomputed by the backend so list views avoid per-package fetches.
public struct TrendingHistoryIndexEntry: Sendable, Hashable, Codable {
    public let name: String
    /// `"formula"` or `"cask"`.
    public let kind: String
    /// Backend-computed velocity ranking signal. Optional on the wire
    /// (`Option<f64>` in Rust / `velocityIndex?` in TS) — `nil` when the
    /// collector couldn't compute a stable ratio for this package.
    public let velocityIndex: Double?
    /// Precomputed values to plot for this package's mini sparkline.
    public let sparkline: [Double]

    public init(name: String, kind: String, velocityIndex: Double?, sparkline: [Double]) {
        self.name = name
        self.kind = kind
        self.velocityIndex = velocityIndex
        self.sparkline = sparkline
    }

    private enum CodingKeys: String, CodingKey {
        case name
        case kind
        case velocityIndex
        case sparkline
    }
}
