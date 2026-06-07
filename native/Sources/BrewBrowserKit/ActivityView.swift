import SwiftUI

/// Bottom Activity drawer — live console for the active streaming job (install /
/// upgrade / uninstall / update). Mirrors the Tauri `ActivityDrawer`: collapsed
/// shows a one-line status; expanded shows the scrolling output with copy,
/// cancel, a completion footer, and (with 2+ jobs) a view-only segmented
/// switcher. Mounted as a bottom bar below the split view.
struct ActivityDrawer: View {
    @Bindable var model: AppModel

    /// The drawer shows the explicitly-active job only. nil `activeJobId`
    /// (after Close) hides the drawer entirely; selecting a job in the Activity
    /// panel or starting a new one re-populates it.
    private var job: ActivityJob? {
        guard let id = model.activeJobId else { return nil }
        return model.jobs.first { $0.id == id }
    }

    /// Status-aware header label. The job's stored `label` is the in-progress
    /// form ("Installing X"); on completion show the terminal form so a green ✓
    /// doesn't sit next to "Installing".
    static func displayLabel(_ job: ActivityJob) -> String {
        switch job.status {
        case .running:   return job.label
        case .succeeded: return job.label
                .replacingOccurrences(of: "Installing ", with: "Installed ")
                .replacingOccurrences(of: "Upgrading ", with: "Upgraded ")
                .replacingOccurrences(of: "Uninstalling ", with: "Uninstalled ")
        case .failed:    return "Failed: \(job.label)"
        case .canceled:  return "Canceled: \(job.label)"
        }
    }

    var body: some View {
        if let job {
            VStack(spacing: 0) {
                Divider()
                header(job)
                if model.drawerOpen {
                    console(job)
                    footer(job)
                }
            }
            // Full bleed edge-to-edge so the bar covers the window's rounded
            // bottom corners (no dark notch where the split-view corner shows
            // through). .bar material fills the whole width.
            .frame(maxWidth: .infinity)
            .background(.bar)
            // Auto-collapse the console when a job finishes successfully — keeps
            // the one-line status visible (+ in Activity history) without the
            // 200px console lingering. Failures stay expanded so the error is seen.
            .onChange(of: job.status) { _, status in
                if status == .succeeded { model.drawerOpen = false }
            }
        }
    }

    private func header(_ job: ActivityJob) -> some View {
        HStack(spacing: 8) {
            statusIcon(job.status)
            Text(Self.displayLabel(job)).font(.callout.weight(.medium)).lineLimit(1)
            if job.status == .running {
                // Live elapsed timer, refreshed each second off the timeline's
                // own date (no Date() in the view body).
                TimelineView(.periodic(from: .now, by: 1)) { ctx in
                    Text(Self.elapsed(since: job.startedAt, now: ctx.date))
                        .font(.callout.monospacedDigit()).foregroundStyle(.secondary)
                }
                ProgressView().controlSize(.small)
            }
            Spacer()
            if job.status == .running {
                Button {
                    model.cancelJob(job.id)
                } label: { Label("Cancel", systemImage: "stop.circle") }
                .buttonStyle(.borderless)
            }
            Button {
                copyLog(job)
            } label: { Image(systemName: "doc.on.doc") }
            .buttonStyle(.borderless)
            .help("Copy output")
            Button {
                model.drawerOpen.toggle()
            } label: {
                Image(systemName: model.drawerOpen ? "chevron.down" : "chevron.up")
            }
            .buttonStyle(.borderless)
            .help(model.drawerOpen ? "Collapse" : "Expand")
            // Close — dismiss the drawer for this job (stays in Activity history).
            // Cancels first if it's still running. Mirrors the Tauri drawer's X.
            Button {
                if job.status == .running { model.cancelJob(job.id) }
                model.dismissDrawer()
            } label: {
                Image(systemName: "xmark")
            }
            .buttonStyle(.borderless)
            .help("Close")
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .contentShape(.rect)
        .onTapGesture { model.drawerOpen.toggle() }
    }

    private func console(_ job: ActivityJob) -> some View {
        ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 1) {
                    ForEach(Array(job.lines.enumerated()), id: \.offset) { idx, line in
                        Text(Self.stripANSI(line.text))
                            .font(.caption.monospaced())
                            .foregroundStyle(Self.lineColor(line))
                            .textSelection(.enabled)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .id(idx)
                    }
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 6)
            }
            .frame(height: 200)
            .onChange(of: job.lines.count) { _, count in
                if count > 0 { proxy.scrollTo(count - 1, anchor: .bottom) }
            }
        }
    }

    /// Completion footer — duration + (on failure) a Report button. Nothing
    /// while running. Mirrors the Tauri drawer footer.
    @ViewBuilder
    private func footer(_ job: ActivityJob) -> some View {
        if job.status != .running {
            HStack(spacing: 8) {
                Text(Self.footerText(job))
                    .font(.caption)
                    .foregroundStyle(Self.footerColor(job.status))
                if job.status == .failed {
                    Button("Report to brew-browser") {
                        ReportIssue.open(for: job, brewVersion: model.brewVersion)
                    }
                    .buttonStyle(.link)
                    .font(.caption)
                }
                Spacer()
            }
            .padding(.horizontal, 12)
            .padding(.bottom, 8)
        }
    }

    @ViewBuilder
    private func statusIcon(_ status: ActivityJob.JobStatus) -> some View {
        switch status {
        case .running:   Image(systemName: "circle.dotted").foregroundStyle(.secondary)
        case .succeeded: Image(systemName: "checkmark.circle.fill").foregroundStyle(.green)
        case .failed:    Image(systemName: "xmark.circle.fill").foregroundStyle(.red)
        case .canceled:  Image(systemName: "minus.circle.fill").foregroundStyle(.secondary)
        }
    }

    private func copyLog(_ job: ActivityJob) {
        let text = job.lines.map(\.text).joined(separator: "\n")
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    }

    // MARK: - Formatting helpers (static; pure)

    /// "m:ss" elapsed since an epoch-seconds start.
    static func elapsed(since start: Double, now: Date) -> String {
        let s = max(0, Int(now.timeIntervalSince1970 - start))
        return String(format: "%d:%02d", s / 60, s % 60)
    }

    /// Compact duration like "4.2s" from durationMs.
    static func duration(_ ms: Int?) -> String? {
        guard let ms else { return nil }
        return String(format: "%.1fs", Double(ms) / 1000)
    }

    static func footerText(_ job: ActivityJob) -> String {
        let dur = duration(job.durationMs)
        switch job.status {
        case .running:   return ""
        case .succeeded: return dur.map { "Done in \($0)." } ?? "Done."
        case .failed:
            let base = dur.map { "Failed after \($0)." } ?? "Failed."
            if let code = job.exitCode, code != 0 { return "\(base) Exit \(code)." }
            return base
        case .canceled:  return "Stopped."
        }
    }

    static func footerColor(_ status: ActivityJob.JobStatus) -> Color {
        switch status {
        case .succeeded: return .green
        case .failed:    return .red
        default:         return .secondary
        }
    }

    /// Content-based line coloring, matching the Tauri drawer's `classifyLine`.
    static func lineColor(_ line: ActivityLine) -> Color {
        let text = stripANSI(line.text)
        let lower = text.lowercased()
        if text.hasPrefix("==>") { return .accentColor }
        if lower.contains("error:") { return .red }
        if lower.contains("warning:") { return .orange }
        if lower.hasPrefix("downloading") || lower.hasPrefix("pouring")
            || lower.hasPrefix("installing") || lower.hasPrefix("fetching") {
            return .green
        }
        if line.stream == .stderr { return .orange }
        return .primary
    }

    /// Strip ANSI CSI escape sequences. brew runs with HOMEBREW_NO_COLOR so it
    /// shouldn't emit any, but strip defensively (parity with Tauri).
    static func stripANSI(_ s: String) -> String {
        s.replacingOccurrences(
            of: "\u{1B}\\[[0-9;]*[A-Za-z]",
            with: "",
            options: .regularExpression
        )
    }
}

/// The Activity panel — job history. Click a job to open it in the drawer;
/// swipe or right-click to remove a single job; "Clear Finished" clears them all.
struct ActivityView: View {
    @Bindable var model: AppModel

    /// Row the pointer is over — drives the hover-revealed remove button (parity
    /// with the Tauri history list's hover trash).
    @State private var hovered: UUID?

    var body: some View {
        Group {
            if model.jobs.isEmpty {
                ContentUnavailableView(
                    "No activity yet",
                    systemImage: "list.bullet.rectangle",
                    description: Text("Installs, upgrades, uninstalls, and updates show up here.")
                )
            } else {
                List(model.jobs) { job in
                    HStack(spacing: 6) {
                        Button {
                            model.activeJobId = job.id
                            model.drawerOpen = true
                        } label: {
                            HStack(spacing: 10) {
                                icon(job.status)
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(job.label).fontWeight(.medium)
                                    Text(job.command).font(.caption.monospaced()).foregroundStyle(.secondary)
                                }
                                Spacer()
                                VStack(alignment: .trailing, spacing: 2) {
                                    Text(statusText(job)).font(.caption).foregroundStyle(.secondary)
                                    if let dur = ActivityDrawer.duration(job.durationMs), job.status != .running {
                                        Text(dur).font(.caption2).foregroundStyle(.secondary)
                                    }
                                }
                            }
                            .contentShape(.rect)
                        }
                        .buttonStyle(.plain)

                        // Hover-revealed remove (parity with Tauri's row trash).
                        // Reserves its gutter always so the row doesn't reflow on
                        // hover; only hit-testable while shown.
                        Button {
                            model.removeJob(job.id)
                        } label: {
                            Image(systemName: "trash")
                                .foregroundStyle(hovered == job.id ? Color.red : Color.secondary)
                        }
                        .buttonStyle(.borderless)
                        .frame(width: 24)
                        .opacity(hovered == job.id ? 1 : 0)
                        .allowsHitTesting(hovered == job.id)
                        .help("Remove from history")
                    }
                    .onHover { inside in
                        if inside { hovered = job.id }
                        else if hovered == job.id { hovered = nil }
                    }
                    .swipeActions(edge: .trailing) {
                        Button(role: .destructive) { model.removeJob(job.id) } label: {
                            Label("Remove", systemImage: "trash")
                        }
                    }
                    .contextMenu {
                        Button(role: .destructive) { model.removeJob(job.id) } label: {
                            Label("Remove", systemImage: "trash")
                        }
                    }
                }
            }
        }
        .toolbar {
            if model.jobs.contains(where: { $0.status != .running }) {
                ToolbarItem(placement: .primaryAction) {
                    Button("Clear Finished") { model.clearFinishedJobs() }
                }
            }
        }
    }

    /// Right-aligned status string; failures append their exit code.
    private func statusText(_ job: ActivityJob) -> String {
        if job.status == .failed, let code = job.exitCode, code != 0 {
            return "Failed · exit \(code)"
        }
        return job.status.rawValue.capitalized
    }

    @ViewBuilder
    private func icon(_ status: ActivityJob.JobStatus) -> some View {
        switch status {
        case .running:   Image(systemName: "circle.dotted").foregroundStyle(.secondary)
        case .succeeded: Image(systemName: "checkmark.circle.fill").foregroundStyle(.green)
        case .failed:    Image(systemName: "xmark.circle.fill").foregroundStyle(.red)
        case .canceled:  Image(systemName: "minus.circle.fill").foregroundStyle(.secondary)
        }
    }
}
