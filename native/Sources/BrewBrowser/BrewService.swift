import Foundation

/// Thin async wrapper over the `brew` CLI — the Swift equivalent of the Rust
/// `brew::exec` module. Every action shells out to `brew`; we never parse
/// formula files or touch the prefix ourselves. brew owns all of that.
///
/// Spike scope: just `list --versions`. The streaming-line design generalizes
/// to install/upgrade/search (the long-running, live-output commands) without
/// changing shape — the same `runStreaming` powers them in the full port.
struct InstalledPackage: Identifiable, Hashable, Sendable {
    var id: String { name }
    let name: String
    let version: String
    let kind: Kind

    enum Kind: String, Sendable { case formula, cask }
}

/// An outdated package with its installed → current version transition,
/// mirroring the Tauri Dashboard's "Updates available" rows.
struct OutdatedPackage: Identifiable, Hashable, Sendable {
    var id: String { name }
    let name: String
    let installedVersion: String
    let currentVersion: String
}

enum BrewError: Error, LocalizedError {
    case brewNotFound
    case nonZeroExit(code: Int32, stderr: String)

    var errorDescription: String? {
        switch self {
        case .brewNotFound:
            return "Couldn't find the brew executable. Is Homebrew installed?"
        case let .nonZeroExit(code, stderr):
            return "brew exited with code \(code): \(stderr)"
        }
    }
}

actor BrewService {
    /// Resolve the brew binary the same way the Rust backend does: prefer the
    /// Apple-Silicon prefix, fall back to the Intel path, then bare PATH lookup.
    private static func resolveBrewPath() -> String? {
        let candidates = ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"]
        for c in candidates where FileManager.default.isExecutableFile(atPath: c) {
            return c
        }
        return nil
    }

    /// Run a brew subcommand to completion and return captured stdout.
    /// `current_dir` is pinned to "/" to dodge the "cwd must be readable"
    /// failure the Linux build hit — harmless on macOS, future-proof.
    private func runCapture(_ args: [String]) async throws -> String {
        guard let brew = Self.resolveBrewPath() else { throw BrewError.brewNotFound }

        let process = Process()
        process.executableURL = URL(fileURLWithPath: brew)
        process.arguments = args
        process.currentDirectoryURL = URL(fileURLWithPath: "/")

        let stdout = Pipe()
        let stderr = Pipe()
        process.standardOutput = stdout
        process.standardError = stderr

        try process.run()

        // Read pipes off the main actor; Process.run is synchronous-launch but
        // we await its termination without blocking the UI.
        let outData = stdout.fileHandleForReading.readDataToEndOfFile()
        let errData = stderr.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()

        if process.terminationStatus != 0 {
            let err = String(decoding: errData, as: UTF8.self)
            throw BrewError.nonZeroExit(code: process.terminationStatus, stderr: err)
        }
        return String(decoding: outData, as: UTF8.self)
    }

    /// `brew list --formula --versions` → [InstalledPackage].
    /// Output is one package per line: "name version1 version2 ...".
    /// We take the first version (the linked one) for display, matching the
    /// existing app's behavior.
    func listInstalledFormulae() async throws -> [InstalledPackage] {
        let raw = try await runCapture(["list", "--formula", "--versions"])
        return raw
            .split(separator: "\n")
            .compactMap { line -> InstalledPackage? in
                let parts = line.split(separator: " ", maxSplits: 1)
                guard let name = parts.first else { return nil }
                let version = parts.count > 1
                    ? String(parts[1].split(separator: " ").first ?? "")
                    : "—"
                return InstalledPackage(name: String(name), version: version, kind: .formula)
            }
            .sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
    }

    /// `brew list --cask --versions` → [InstalledPackage] of kind `.cask`.
    /// Same line shape as the formula lister: "name version1 version2 …".
    func listInstalledCasks() async throws -> [InstalledPackage] {
        let raw = try await runCapture(["list", "--cask", "--versions"])
        return raw
            .split(separator: "\n")
            .compactMap { line -> InstalledPackage? in
                let parts = line.split(separator: " ", maxSplits: 1)
                guard let name = parts.first else { return nil }
                let version = parts.count > 1
                    ? String(parts[1].split(separator: " ").first ?? "")
                    : "—"
                return InstalledPackage(name: String(name), version: version, kind: .cask)
            }
            .sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
    }

    /// All installed packages — formulae + casks — merged and name-sorted.
    func listInstalledAll() async throws -> [InstalledPackage] {
        async let formulae = listInstalledFormulae()
        async let casks = listInstalledCasks()
        let merged = try await formulae + casks
        return merged.sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
    }

    /// Count of newline-delimited entries from a brew subcommand, ignoring
    /// blank lines. Used for the cheap "how many X" Dashboard stats.
    private func lineCount(_ args: [String]) async throws -> Int {
        let raw = try await runCapture(args)
        return raw.split(separator: "\n").filter { !$0.isEmpty }.count
    }

    func countCasks() async throws -> Int { try await lineCount(["list", "--cask"]) }

    func countFormulae() async throws -> Int { try await lineCount(["list", "--formula"]) }

    /// Explicitly-installed formulae (not pulled in as dependencies).
    func countLeaves() async throws -> Int { try await lineCount(["leaves"]) }

    /// Formulae the user explicitly requested (`brew install foo`), vs pulled
    /// in as dependencies. Mirrors the Tauri "N on request" chip.
    func countOnRequest() async throws -> Int {
        try await lineCount(["list", "--installed-on-request", "--formula"])
    }

    /// Pinned formulae (held back from upgrade). Mirrors the "N pinned" chip.
    func countPinned() async throws -> Int { try await lineCount(["list", "--pinned"]) }

    /// Count of running brew services (`brew services list`, status "started"/
    /// "scheduled"). Powers the Services sidebar badge. Best-effort: 0 on error.
    func countRunningServices() async -> Int {
        guard let raw = try? await runCapture(["services", "list"]) else { return 0 }
        // Skip the header line; a service is "running" if its 2nd column is
        // started or scheduled.
        return raw.split(separator: "\n").dropFirst().reduce(into: 0) { acc, line in
            let cols = line.split(separator: " ", omittingEmptySubsequences: true)
            if cols.count >= 2, cols[1] == "started" || cols[1] == "scheduled" { acc += 1 }
        }
    }

    /// brew version string, e.g. "5.1.14-112-g0d7d68d" (the build suffix kept).
    func version() async -> String {
        guard let raw = try? await runCapture(["--version"]) else { return "—" }
        // First line is "Homebrew 5.1.14-112-g0d7d68d"
        let first = raw.split(separator: "\n").first.map(String.init) ?? ""
        return first.replacingOccurrences(of: "Homebrew ", with: "")
    }

    /// The Homebrew prefix (e.g. /opt/homebrew).
    func prefix() async -> String {
        (try? await runCapture(["--prefix"]))?.trimmingCharacters(in: .whitespacesAndNewlines) ?? "/opt/homebrew"
    }

    /// Outdated packages (formulae + casks). `brew outdated` prints one per line.
    func countOutdated() async throws -> Int { try await lineCount(["outdated"]) }

    /// Outdated formulae with installed → current versions, parsed from
    /// `brew outdated --json=v2`. Used for the Dashboard "Updates available"
    /// list. Best-effort decode: malformed JSON yields an empty list.
    func outdatedPackages() async throws -> [OutdatedPackage] {
        let raw = try await runCapture(["outdated", "--json=v2"])
        guard let data = raw.data(using: .utf8),
              let root = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return [] }

        var out: [OutdatedPackage] = []
        for key in ["formulae", "casks"] {
            guard let arr = root[key] as? [[String: Any]] else { continue }
            for item in arr {
                guard let name = item["name"] as? String else { continue }
                let installed = (item["installed_versions"] as? [String])?.first
                    ?? (item["installed_versions"] as? [Any])?.first as? String
                    ?? "?"
                let current = item["current_version"] as? String ?? "?"
                out.append(OutdatedPackage(name: name, installedVersion: installed, currentVersion: current))
            }
        }
        return out.sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
    }

    /// Bytes used by a directory, via `du -sk` (read-only, fast). Returns nil if
    /// the path is missing or du fails.
    private func dirSizeBytes(_ path: String) async -> Int64? {
        guard FileManager.default.fileExists(atPath: path) else { return nil }
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/bin/du")
        p.arguments = ["-sk", path]
        p.currentDirectoryURL = URL(fileURLWithPath: "/")
        let out = Pipe()
        p.standardOutput = out
        p.standardError = Pipe()
        do {
            try p.run()
            let data = out.fileHandleForReading.readDataToEndOfFile()
            p.waitUntilExit()
            let text = String(decoding: data, as: UTF8.self)
            guard let kb = text.split(separator: "\t").first.flatMap({ Int64($0.trimmingCharacters(in: .whitespaces)) })
            else { return nil }
            return kb * 1024
        } catch {
            return nil
        }
    }

    /// Full storage breakdown, mirroring the Tauri Storage card: Cellar,
    /// Caskroom, Logs, and the Homebrew download cache, each with its path.
    func storageBreakdown() async -> [StorageItem] {
        let prefix = await prefix()
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        let entries: [(String, String)] = [
            ("Formulae (Cellar)", "\(prefix)/Cellar"),
            ("Casks (Caskroom)", "\(prefix)/Caskroom"),
            ("Logs (var/log)", "\(prefix)/var/log"),
            ("Download cache", "\(home)/Library/Caches/Homebrew"),
        ]
        var items: [StorageItem] = []
        for (label, path) in entries {
            if let bytes = await dirSizeBytes(path) {
                items.append(StorageItem(label: label, path: path, bytes: bytes))
            }
        }
        return items
    }
}

/// One row in the Storage breakdown.
struct StorageItem: Identifiable, Hashable, Sendable {
    var id: String { label }
    let label: String
    let path: String
    let bytes: Int64
}

/// Full single-package detail, mirroring the Tauri `PackageDetail` (types.rs:138)
/// as parsed from `brew info --json=v2`. Only the fields the native detail panel
/// renders are kept.
struct PackageInfo: Sendable, Hashable {
    let name: String
    let fullName: String
    let kind: InstalledPackage.Kind
    let installedVersion: String?
    let stableVersion: String?
    let desc: String?
    let homepage: String?
    let license: String?
    let tap: String?
    let caveats: String?
    let outdated: Bool
    let pinned: Bool
    let dependencies: [String]
    let buildDependencies: [String]
    let conflictsWith: [String]

    var isOutdated: Bool {
        guard let i = installedVersion, let s = stableVersion else { return outdated }
        return outdated || (i != s)
    }
}

extension BrewService {
    /// `brew info --json=v2 [--formula|--cask] <name>` → PackageInfo.
    /// Field names mirror `brew/parse.rs` / the Rust `to_detail` mapping.
    func info(name: String, kind: InstalledPackage.Kind) async throws -> PackageInfo {
        let kindFlag = kind == .cask ? "--cask" : "--formula"
        let raw = try await runCapture(["info", "--json=v2", kindFlag, name])
        guard let data = raw.data(using: .utf8),
              let root = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { throw BrewError.nonZeroExit(code: -1, stderr: "Unparseable brew info JSON") }

        // v2 nests under "formulae" / "casks".
        let arrayKey = kind == .cask ? "casks" : "formulae"
        guard let arr = root[arrayKey] as? [[String: Any]], let obj = arr.first else {
            throw BrewError.nonZeroExit(code: -1, stderr: "No \(arrayKey) entry for \(name)")
        }

        if kind == .cask {
            return Self.parseCask(obj)
        } else {
            return Self.parseFormula(obj)
        }
    }

    private static func parseFormula(_ o: [String: Any]) -> PackageInfo {
        let name = o["name"] as? String ?? o["full_name"] as? String ?? "?"
        let versions = o["versions"] as? [String: Any]
        let stable = versions?["stable"] as? String
        // installed: [{version, ...}] — take the linked/first.
        let installedArr = o["installed"] as? [[String: Any]]
        let installed = installedArr?.last?["version"] as? String
            ?? (o["linked_keg"] as? String)
        return PackageInfo(
            name: name,
            fullName: o["full_name"] as? String ?? name,
            kind: .formula,
            installedVersion: installed,
            stableVersion: stable,
            desc: o["desc"] as? String,
            homepage: o["homepage"] as? String,
            license: o["license"] as? String,
            tap: o["tap"] as? String,
            caveats: o["caveats"] as? String,
            outdated: o["outdated"] as? Bool ?? false,
            pinned: o["pinned"] as? Bool ?? false,
            dependencies: o["dependencies"] as? [String] ?? [],
            buildDependencies: o["build_dependencies"] as? [String] ?? [],
            conflictsWith: o["conflicts_with"] as? [String] ?? []
        )
    }

    private static func parseCask(_ o: [String: Any]) -> PackageInfo {
        // Casks: token, version (string), installed (string), name [array], desc.
        let token = o["token"] as? String ?? o["full_token"] as? String ?? "?"
        let nameArr = o["name"] as? [String]
        let display = nameArr?.first ?? token
        return PackageInfo(
            name: token,
            fullName: display,
            kind: .cask,
            installedVersion: o["installed"] as? String,
            stableVersion: o["version"] as? String,
            desc: o["desc"] as? String,
            homepage: o["homepage"] as? String,
            license: nil,
            tap: o["tap"] as? String,
            caveats: o["caveats"] as? String,
            outdated: o["outdated"] as? Bool ?? false,
            pinned: false,
            dependencies: [],
            buildDependencies: [],
            conflictsWith: []
        )
    }

    /// `brew upgrade <name>` — runs to completion, throws on failure.
    func upgrade(_ name: String) async throws {
        _ = try await runCapture(["upgrade", name])
    }

    /// `brew uninstall [--cask] <name>` — runs to completion, throws on failure.
    func uninstall(_ name: String, kind: InstalledPackage.Kind) async throws {
        var args = ["uninstall"]
        if kind == .cask { args.append("--cask") }
        args.append(name)
        _ = try await runCapture(args)
    }

    /// `brew install [--cask] <name>` — runs to completion, throws on failure.
    func install(_ name: String, kind: InstalledPackage.Kind) async throws {
        var args = ["install"]
        if kind == .cask { args.append("--cask") }
        args.append(name)
        _ = try await runCapture(args)
    }

    // MARK: - Homebrew analytics (Settings → Brew)

    /// Read Homebrew's own analytics setting (`brew analytics state`). Returns
    /// true when analytics are ON. nil if the state can't be determined.
    /// Mirrors the Tauri `brew_get_analytics` command.
    func getAnalytics() async -> Bool? {
        guard let raw = try? await runCapture(["analytics", "state"]) else { return nil }
        let lower = raw.lowercased()
        if lower.contains("enabled") || lower.contains("are on") { return true }
        if lower.contains("disabled") || lower.contains("are off") { return false }
        return nil
    }

    /// Flip Homebrew's analytics setting — same as `brew analytics on|off` at
    /// the terminal. Mirrors the Tauri `brew_set_analytics` command.
    func setAnalytics(_ enabled: Bool) async throws {
        _ = try await runCapture(["analytics", enabled ? "on" : "off"])
    }
}
