import SwiftUI
import AppKit

/// Resolves and caches app icons for Discover/Library rows — the "full icon
/// discovery" competitors (Applite, App Fair) ship. Cascade, in order:
///
///   1. **Appcasks** — `github.com/App-Fair/appcasks/releases/download/
///      cask-<token>/AppIcon.png`. Real, curated app icons; sparse coverage
///      (only casks whose devs opted in by forking appcasks).
///   2. **Google favicon service** — `google.com/s2/favicons?domain=<host>&sz=64`
///      derived from the cask homepage. Near-universal fallback, one clean URL,
///      no HTML scraping / SSRF surface. (A google.com outbound call — disclosed,
///      Offline-Mode gated.)
///   3. **SF Symbol placeholder** (handled by the view) when both miss, and for
///      ALL formulae (CLI tools have no app icon).
///
/// Gating: only casks fetch (formulae always use the SF Symbol). Respects
/// Offline Mode + the cask-icon setting via the `enabled` flag the caller passes
/// (mirrors the Tauri `cask_icon_from_homepage` gate + `CaskIconMode`).
/// On-disk cache at `~/Library/Application Support/brew-browser/native-icons/`.
actor IconService {
    private let cacheDir: URL
    private let session: URLSession
    /// In-flight + resolved memo so concurrent rows for the same token coalesce.
    private var memo: [String: URL?] = [:]

    init() {
        let base = FileManager.default
            .urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
            .appendingPathComponent("brew-browser/native-icons", isDirectory: true)
        try? FileManager.default.createDirectory(at: base, withIntermediateDirectories: true)
        cacheDir = base
        let cfg = URLSessionConfiguration.ephemeral
        cfg.timeoutIntervalForRequest = 12
        session = URLSession(configuration: cfg)
    }

    /// Resolve a cached-on-disk PNG file URL for a cask token, fetching through
    /// the cascade if not cached. Returns nil when nothing resolves (caller then
    /// shows the SF Symbol). Only call for casks with `enabled == true`.
    func iconFileURL(token: String, homepage: String, enabled: Bool) async -> URL? {
        guard enabled else { return nil }
        if let memoed = memo[token] { return memoed }

        let dest = cacheDir.appendingPathComponent("\(token).png")
        if FileManager.default.fileExists(atPath: dest.path) {
            memo[token] = dest
            return dest
        }

        // 1. Appcasks (real icon)
        if let data = await fetch(Self.appcasksURL(token: token)), data.count > 64 {
            try? data.write(to: dest)
            memo[token] = dest
            return dest
        }
        // 2. Google favicon (universal fallback)
        if let host = Self.host(from: homepage),
           let data = await fetch(Self.googleFaviconURL(host: host)), data.count > 64 {
            try? data.write(to: dest)
            memo[token] = dest
            return dest
        }
        memo[token] = .some(nil)
        return nil
    }

    // MARK: - URLs

    private static func appcasksURL(token: String) -> URL {
        URL(string: "https://github.com/App-Fair/appcasks/releases/download/cask-\(token)/AppIcon.png")!
    }

    private static func googleFaviconURL(host: String) -> URL {
        URL(string: "https://www.google.com/s2/favicons?domain=\(host)&sz=64")!
    }

    /// Bare host from a homepage URL (`https://slack.com/foo` → `slack.com`).
    private static func host(from homepage: String) -> String? {
        guard let h = URLComponents(string: homepage)?.host, !h.isEmpty else { return nil }
        return h
    }

    private func fetch(_ url: URL) async -> Data? {
        guard let (data, resp) = try? await session.data(from: url),
              let http = resp as? HTTPURLResponse, http.statusCode == 200 else { return nil }
        return data
    }
}
