import Foundation

/// A streaming brew job (install / upgrade / uninstall) — mirrors the Tauri
/// `ActivityJob` (`src/lib/types.ts`, `stores/activity.svelte.ts`). Lives in the
/// Activity drawer (live) and the Activity panel (history). Codable so the
/// history persists to UserDefaults across launches.
struct ActivityJob: Identifiable, Hashable, Codable, Sendable {
    let id: UUID
    /// Human label, e.g. "Installing wget".
    let label: String
    /// The brew argv, joined for display ("brew install --cask iterm2").
    let command: String
    /// Epoch seconds when the job started (Codable-friendly; no Date() in
    /// preview/build contexts that forbid it).
    let startedAt: Double
    var status: JobStatus
    var lines: [ActivityLine]
    var exitCode: Int32?
    var durationMs: Int?

    enum JobStatus: String, Codable, Sendable {
        case running, succeeded, failed, canceled
    }
}

/// One line of streamed output.
struct ActivityLine: Hashable, Codable, Sendable {
    enum Stream: String, Codable, Sendable { case stdout, stderr }
    let stream: Stream
    let text: String
}
