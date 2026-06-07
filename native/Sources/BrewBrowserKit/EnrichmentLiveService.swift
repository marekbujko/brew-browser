import Foundation

/// Freshness probe shape (`enrichment/version.json`).
struct LiveEnrichmentVersion: Decodable, Sendable {
    let version: String
    let generatedAt: String
    let categoriesVersion: String
}

/// Opt-in live fetch for fresher categories + descriptions, the native mirror
/// of the Tauri `enrichment/live.rs` client (and structurally identical to
/// `TrendingHistoryService`): an actor, soft-fail everywhere, strict token
/// allowlist. The caller (`AppModel`) gates on `settings.liveEnrichmentAllowed`
/// before invoking; this service is pure transport.
///
/// Endpoints (rendered nightly by `tools/pipeline/render_served.py`):
///   GET /enrichment/version.json
///   GET /enrichment/categories.json
///   GET /enrichment/entry/<token>.json
public actor EnrichmentLiveService {
    private let baseURL: URL
    private let session: URLSession

    public init(
        baseURL: URL = URL(string: "https://brew-browser.zerologic.com/enrichment/")!,
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

    /// `version.json` — soft-fail `nil`.
    func version() async -> LiveEnrichmentVersion? {
        await decode(LiveEnrichmentVersion.self,
                     from: baseURL.appendingPathComponent("version.json", isDirectory: false))
    }

    /// `categories.json` raw bytes (parsed by `CategoryCatalog.parse`). `nil` on failure.
    func categoriesData() async -> Data? {
        await fetchData(baseURL.appendingPathComponent("categories.json", isDirectory: false))
    }

    /// `entry/<token>.json` → `EnrichmentEntry`. `nil` for an invalid token,
    /// network error, non-2xx (404 for an uncovered token is normal), or decode
    /// failure — the caller keeps the bundled entry.
    func entry(token: String) async -> EnrichmentEntry? {
        guard Self.isValidToken(token) else { return nil }
        let url = baseURL
            .appendingPathComponent("entry", isDirectory: true)
            .appendingPathComponent("\(token).json", isDirectory: false)
        guard let data = await fetchData(url),
              let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return nil }
        return EnrichmentCatalog.parseLiveEntry(obj)
    }

    /// Strict token allowlist mirroring the Rust path-traversal guard
    /// (`enrichment/live.rs::fetch_entry`).
    nonisolated static func isValidToken(_ name: String) -> Bool {
        guard !name.isEmpty, !name.contains("..") else { return false }
        let allowed = CharacterSet(charactersIn:
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._+@-")
        return name.unicodeScalars.allSatisfy { allowed.contains($0) }
    }

    private func fetchData(_ url: URL) async -> Data? {
        do {
            let (data, response) = try await session.data(from: url)
            if let http = response as? HTTPURLResponse, !(200...299).contains(http.statusCode) {
                return nil
            }
            return data
        } catch {
            return nil
        }
    }

    private func decode<T: Decodable>(_ type: T.Type, from url: URL) async -> T? {
        guard let data = await fetchData(url) else { return nil }
        return try? JSONDecoder().decode(T.self, from: data)
    }
}
