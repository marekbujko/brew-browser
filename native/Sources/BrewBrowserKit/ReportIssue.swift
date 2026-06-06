import AppKit
import Foundation

/// "Report to brew-browser" — builds a pre-filled GitHub new-issue URL from a
/// failed Activity job and opens it in the browser. Mirrors the Tauri
/// `src/lib/util/reportIssue.ts` (`openReportIssueFromJob`): same repo, same
/// templated body (app + Homebrew version, command, exit code, stderr excerpt),
/// same `from-app` label.
enum ReportIssue {
    private static let newIssueURL = "https://github.com/msitarzewski/brew-browser/issues/new"

    /// Cap on the stderr excerpt, keeping the URL well under GitHub's ~8 KiB
    /// limit even with the rest of the templated context (matches Tauri).
    private static let stderrMaxChars = 2000

    /// This app's short version string (CFBundleShortVersionString), or "unknown".
    static var appVersion: String {
        Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "unknown"
    }

    /// Build the pre-filled new-issue URL for a job. `brewVersion` comes from the
    /// app model (already resolved via `brew --version`).
    static func url(for job: ActivityJob, brewVersion: String) -> URL? {
        var components = URLComponents(string: newIssueURL)
        components?.queryItems = [
            URLQueryItem(name: "title", value: "[brew-browser] \(job.label)"),
            URLQueryItem(name: "labels", value: "from-app"),
            URLQueryItem(name: "body", value: body(for: job, brewVersion: brewVersion)),
        ]
        return components?.url
    }

    /// Open the pre-filled issue page in the default browser.
    static func open(for job: ActivityJob, brewVersion: String) {
        guard let url = url(for: job, brewVersion: brewVersion) else { return }
        NSWorkspace.shared.open(url)
    }

    private static func body(for job: ActivityJob, brewVersion: String) -> String {
        var lines: [String] = [
            "**brew-browser:** \(appVersion)",
            "**Homebrew:** \(brewVersion)",
            "**Command:** `\(job.command)`",
        ]
        if let code = job.exitCode {
            lines.append("**Exit code:** \(code)")
        }

        // Prefer the stderr stream; fall back to the full log tail if a failure
        // produced nothing on stderr (some brew errors print to stdout).
        let stderr = job.lines.filter { $0.stream == .stderr }.map(\.text).joined(separator: "\n")
        let excerptSource = stderr.isEmpty
            ? job.lines.map(\.text).joined(separator: "\n")
            : stderr
        let trimmed = excerptSource.trimmingCharacters(in: .whitespacesAndNewlines)
        if !trimmed.isEmpty {
            let capped = trimmed.count > stderrMaxChars
                ? "…(truncated)…\n" + String(trimmed.suffix(stderrMaxChars))
                : trimmed
            lines.append(contentsOf: ["", "**Output excerpt:**", "", "```", capped, "```"])
        }

        lines.append(contentsOf: [
            "",
            "---",
            "",
            "_Replace this line with what you were doing when the error appeared, and what you expected to happen._",
        ])
        return lines.joined(separator: "\n")
    }
}
