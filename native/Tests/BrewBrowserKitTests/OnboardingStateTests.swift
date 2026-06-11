import Testing
import Foundation
@testable import BrewBrowserKit

// Tests for the missing-Homebrew onboarding gate — the AppModel state machine
// behind OnboardingView. The poll loop's per-pass logic lives in
// `updateOnboarding(brewFound:cltFound:)` precisely so these transitions are
// drivable without a real (or really-missing) Homebrew install.

@Suite("Missing-Homebrew onboarding state")
@MainActor
struct OnboardingStateTests {
    @Test func missingResolutionStaysInOnboarding() {
        let model = AppModel()
        model.brewMissing = true   // simulate a failed launch resolution

        // A poll pass with brew still missing keeps onboarding up and tracks
        // the CLT probe for the step list.
        #expect(model.updateOnboarding(brewFound: false, cltFound: false) == false)
        #expect(model.brewMissing)
        #expect(!model.cltInstalled)
    }

    @Test func cltProbeUpdatesIndependentlyOfBrew() {
        let model = AppModel()
        model.brewMissing = true

        // CLT lands first (the installer prompt finished) while brew is still
        // installing — the step list updates, onboarding stays up.
        #expect(model.updateOnboarding(brewFound: false, cltFound: true) == false)
        #expect(model.brewMissing)
        #expect(model.cltInstalled)
    }

    @Test func brewFoundExitsOnboarding() {
        let model = AppModel()
        model.brewMissing = true

        // The pass where brew resolves ends onboarding (true = stop polling);
        // ContentView then builds the normal root, whose `.task`s run the
        // standard initial load sequence.
        #expect(model.updateOnboarding(brewFound: true, cltFound: true))
        #expect(!model.brewMissing)
    }

    @Test func brewFoundStaysExitedOnLaterPasses() {
        let model = AppModel()
        model.brewMissing = true
        model.updateOnboarding(brewFound: true, cltFound: true)

        // A stray extra pass (e.g. an in-flight probe finishing late) must not
        // flip the app back into onboarding.
        #expect(model.updateOnboarding(brewFound: true, cltFound: true))
        #expect(!model.brewMissing)
    }

    @Test func initialResolutionMatchesSharedResolver() {
        // The launch gate keys off the same shared resolver as the services:
        // brewMissing at init is exactly "resolution returned nil".
        let model = AppModel()
        #expect(model.brewMissing == (BrewService.resolveBrewPath() == nil))
    }
}
