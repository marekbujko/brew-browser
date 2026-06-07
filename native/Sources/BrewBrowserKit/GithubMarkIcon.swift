import SwiftUI
import AppKit

/// The GitHub Octocat mark. SwiftUI can't load SVG, so it ships as a bundled
/// vector PDF (Primer/Octicons, MIT) loaded as a **template** NSImage — that
/// lets `.foregroundStyle` tint it like an SF Symbol while staying crisp at any
/// size. Falls back to a code-brackets SF Symbol if the asset ever fails to load.
struct GithubMarkIcon: View {
    var size: CGFloat = 14

    var body: some View {
        Group {
            if let img = Self.template {
                Image(nsImage: img)
                    .resizable()
                    .renderingMode(.template)
                    .interpolation(.high)
            } else {
                Image(systemName: "chevron.left.forwardslash.chevron.right")
                    .resizable()
            }
        }
        .scaledToFit()
        .frame(width: size, height: size)
    }

    /// Loaded once; `isTemplate` so SwiftUI tints it via `.foregroundStyle`.
    private static let template: NSImage? = {
        guard let url = Bundle.module.url(forResource: "github-mark", withExtension: "pdf"),
              let img = NSImage(contentsOf: url) else { return nil }
        img.isTemplate = true
        return img
    }()
}
