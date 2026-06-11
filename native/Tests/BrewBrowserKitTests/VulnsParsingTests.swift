import Testing
import Foundation
@testable import BrewBrowserKit

// Tests for the vulnerability parsing — the subsystem that produced the worst
// bug (every package flagged with the whole install set's CVEs). Mirrors the
// Rust `vulns::client` parse/keying behavior.

@Suite("VulnSeverity")
struct VulnSeverityTests {
    @Test func caseFoldsWireValues() {
        #expect(VulnSeverity(wire: "CRITICAL") == .critical)
        #expect(VulnSeverity(wire: "critical") == .critical)
        #expect(VulnSeverity(wire: "High") == .high)
        #expect(VulnSeverity(wire: "MEDIUM") == .medium)
        // GHSA "MODERATE" maps to OSV "MEDIUM" (GOTCHA #2).
        #expect(VulnSeverity(wire: "MODERATE") == .medium)
        #expect(VulnSeverity(wire: "moderate") == .medium)
        #expect(VulnSeverity(wire: "LOW") == .low)
    }

    @Test func unknownAndNilFallBackToUnknown() {
        #expect(VulnSeverity(wire: "bogus") == .unknown)
        #expect(VulnSeverity(wire: "") == .unknown)
        #expect(VulnSeverity(wire: nil) == .unknown)
    }

    @Test func ordersByRisk() {
        #expect(VulnSeverity.critical > .high)
        #expect(VulnSeverity.high > .medium)
        #expect(VulnSeverity.medium > .low)
        #expect(VulnSeverity.low > .unknown)
        // max() surfaces the worst.
        #expect([VulnSeverity.low, .critical, .medium].max() == .critical)
    }
}

@Suite("VulnsService.parseScanOutputKeyed")
struct VulnsParseTests {
    // Two formulae, each with its OWN distinct finding. The regression we guard:
    // findings must attribute to the right package, never smear across all.
    static let twoFormulae = """
    [
      { "formula": "openssl@3", "version": "3.0.0", "vulnerabilities": [
          { "id": "CVE-AAA", "severity": "HIGH", "summary": "s1", "details": "d1", "fixed_versions": ["3.0.1"], "references": [] }
        ] },
      { "formula": "curl", "version": "8.0.0", "vulnerabilities": [
          { "id": "CVE-BBB", "severity": "MODERATE", "summary": "s2", "details": "d2", "fixed_versions": [], "references": [] }
        ] }
    ]
    """

    @Test func keysFindingsByFormulaWithoutSmearing() throws {
        let out = try VulnsService.parseScanOutputKeyed(Self.twoFormulae)
        #expect(Set(out.keys) == ["openssl@3", "curl"])

        let openssl = try #require(out["openssl@3"])
        #expect(openssl.count == 1)
        #expect(openssl.first?.rawId == "CVE-AAA")
        #expect(openssl.first?.severity == .high)
        // The other formula's CVE must NOT appear here (the smearing bug).
        #expect(!openssl.contains { $0.rawId == "CVE-BBB" })

        let curl = try #require(out["curl"])
        #expect(curl.first?.rawId == "CVE-BBB")
        #expect(curl.first?.severity == .medium)  // MODERATE → medium
    }

    @Test func cleanFormulaKeptWithEmptyFindings() throws {
        // A scanned-but-clean formula is a real signal (key present, [] findings).
        let json = """
        [ { "formula": "wget", "version": "1.0", "vulnerabilities": [] } ]
        """
        let out = try VulnsService.parseScanOutputKeyed(json)
        #expect(out["wget"] != nil)
        #expect(out["wget"]?.isEmpty == true)
    }

    @Test func emptyOutputYieldsNoRecords() throws {
        #expect(try VulnsService.parseScanOutputKeyed("").isEmpty)
        #expect(try VulnsService.parseScanOutputKeyed("   \n  ").isEmpty)
    }

    @Test func malformedOutputThrows() {
        #expect(throws: (any Error).self) {
            _ = try VulnsService.parseScanOutputKeyed("this is not json")
        }
    }
}

@Suite("VulnsService brew resolution")
struct VulnsResolutionTests {
    // The regression we guard: a failed resolution used to be papered over
    // with a hardcoded "/opt/homebrew/bin/brew". The default init must carry
    // exactly what the shared resolver returns — including nil.
    @Test func defaultInitNeverFabricatesAPath() {
        #expect(VulnsService().brewPath == BrewService.resolveBrewPath())
    }

    @Test func nilBrewPathSurfacesBrewNotFound() async {
        let service = VulnsService(brewPath: nil)
        do {
            _ = try await service.scanOne(name: "wget", isCask: false)
            Issue.record("scanOne should throw .brewNotFound when no brew path resolved")
        } catch let error as VulnsServiceError {
            guard case .brewNotFound = error else {
                Issue.record("expected .brewNotFound, got \(error)")
                return
            }
        } catch {
            Issue.record("expected VulnsServiceError.brewNotFound, got \(error)")
        }
    }

    @Test func nilBrewPathReportsHelperNotInstalled() async {
        // The install probe is best-effort (`try?`) — with no brew it must
        // report not-installed, never crash or pretend a probe ran.
        let service = VulnsService(brewPath: nil)
        #expect(await service.isBrewVulnsInstalled() == false)
    }
}
