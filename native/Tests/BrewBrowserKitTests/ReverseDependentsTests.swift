import Testing
import Foundation
@testable import BrewBrowserKit

// Tests for `buildReverseDependentsIndex` — the pure catalog-graph inversion
// behind the detail panel's "Required by" section (feature #1, Reverse
// dependencies). Mirrors the Rust `invert_dependents` test cases so the two
// shells stay in parity (same bundled JSON → identical dependent sets + edge
// labels).
//
// Parity contract: source S is a dependent of target T iff T appears in
//   S.dependencies(→required) / S.build_dependencies(→build) /
//   S.recommended_dependencies(→recommended) / S.optional_dependencies(→optional),
//   or S is a cask and T ∈ S.depends_on.formula(→required, kind cask);
// self-loops excluded; deduped by (name,kind) keeping strongest edge
// (required > recommended > build > optional); sorted ascending by name.

@Suite("ReverseDependents")
struct ReverseDependentsTests {

    /// A small synthetic catalog covering every edge type + the edge cases.
    /// `wget` requires `openssl@3` (required) and build-depends on `pkgconf`.
    /// `curl` requires `openssl@3`. `ruby` recommends `openssl@3` and
    /// optional-depends on `readline`. `bad` lists itself (self-loop). The cask
    /// `aptible` depends_on.formula `libfido2`. Built fresh per call (a `[[String:
    /// Any]]` is not Sendable, so it can't be a `static let` under Swift 6).
    private func makeFormulae() -> [[String: Any]] {
        [
        [
            "name": "wget",
            "dependencies": ["openssl@3", "libidn2"],
            "build_dependencies": ["pkgconf"],
        ],
        [
            "name": "curl",
            "dependencies": ["openssl@3"],
        ],
        [
            "name": "ruby",
            "recommended_dependencies": ["openssl@3"],
            "optional_dependencies": ["readline"],
        ],
        [
            // Source that lists the target in BOTH dependencies and
            // build_dependencies — must dedupe to the strongest (required).
            "name": "dupe",
            "dependencies": ["zlib"],
            "build_dependencies": ["zlib"],
        ],
        [
            // Self-loop: must be excluded from its own dependents.
            "name": "bad",
            "dependencies": ["bad"],
        ],
        [
            // Leaf — depends on nothing, nothing depends on it.
            "name": "leaf",
        ],
        ]
    }

    private func makeCasks() -> [[String: Any]] {
        [
        [
            "token": "aptible",
            "depends_on": ["formula": ["libfido2"]],
        ],
        [
            // depends_on.formula as a bare string (the catalog uses both shapes).
            "token": "single",
            "depends_on": ["formula": "openssl@3"],
        ],
        [
            // Cask with no depends_on — contributes no edges.
            "token": "plain",
        ],
        ]
    }

    private func index() -> [String: [ReverseDependent]] {
        buildReverseDependentsIndex(formulae: makeFormulae(), casks: makeCasks())
    }

    // MARK: required edge

    @Test func requiredDependentsAreFound() {
        let deps = index()["openssl@3"] ?? []
        // wget, curl (required), ruby (recommended), single (cask, required).
        let names = deps.map(\.name)
        #expect(names.contains("wget"))
        #expect(names.contains("curl"))
        // wget + curl reference openssl@3 via `dependencies` → required.
        let wget = deps.first { $0.name == "wget" }
        #expect(wget?.edge == .required)
        #expect(wget?.kind == .formula)
    }

    // MARK: build edge classified distinctly

    @Test func buildDependencyIsClassifiedAsBuild() {
        let deps = index()["pkgconf"] ?? []
        #expect(deps.count == 1)
        #expect(deps.first?.name == "wget")
        #expect(deps.first?.edge == .build)
    }

    // MARK: recommended + optional distinct from required

    @Test func recommendedAndOptionalAreDistinct() {
        let recommended = index()["openssl@3"]?.first { $0.name == "ruby" }
        #expect(recommended?.edge == .recommended)

        let optional = index()["readline"] ?? []
        #expect(optional.count == 1)
        #expect(optional.first?.name == "ruby")
        #expect(optional.first?.edge == .optional)
    }

    // MARK: leaf → empty

    @Test func leafHasNoDependents() {
        #expect(index()["leaf"] == nil)
        // libidn2 is required by wget, so NOT empty — sanity that the fixture
        // distinguishes leaves from non-leaves.
        #expect(index()["libidn2"]?.count == 1)
    }

    // MARK: self-loop excluded

    @Test func selfLoopExcluded() {
        // `bad` lists itself; it must not appear in its own dependents.
        let deps = index()["bad"] ?? []
        #expect(deps.allSatisfy { $0.name != "bad" })
        #expect(deps.isEmpty)
    }

    // MARK: dedupe with deterministic precedence

    @Test func duplicateSourceDedupedToStrongestEdge() {
        let deps = index()["zlib"] ?? []
        // `dupe` lists zlib in both dependencies (required) and
        // build_dependencies (build) → one entry, required wins.
        #expect(deps.count == 1)
        #expect(deps.first?.name == "dupe")
        #expect(deps.first?.edge == .required)
    }

    // MARK: cask depends_on.formula → cask-kind dependent

    @Test func caskDependsOnFormulaProducesCaskDependent() {
        let deps = index()["libfido2"] ?? []
        #expect(deps.count == 1)
        #expect(deps.first?.name == "aptible")
        #expect(deps.first?.kind == .cask)
        #expect(deps.first?.edge == .required)
    }

    @Test func caskDependsOnFormulaAsStringIsParsed() {
        // `single` cask depends_on.formula is a bare string "openssl@3".
        let single = index()["openssl@3"]?.first { $0.name == "single" }
        #expect(single?.kind == .cask)
        #expect(single?.edge == .required)
    }

    // MARK: sorted by name (stable for UI)

    @Test func dependentsSortedByName() {
        let deps = (index()["openssl@3"] ?? []).map(\.name)
        #expect(deps == deps.sorted { $0.localizedCaseInsensitiveCompare($1) == .orderedAscending })
    }

    // MARK: unknown token / empty catalog → empty, never throws

    @Test func unknownTokenIsEmpty() {
        #expect(index()["does-not-exist"] == nil)
    }

    @Test func emptyCatalogYieldsEmptyIndex() {
        let idx = buildReverseDependentsIndex(formulae: [], casks: [])
        #expect(idx.isEmpty)
    }

    // MARK: real bundled catalog — high-fan-in + leaf parity smoke test

    @Test func bundledCatalogReverseDependents() async {
        let service = CatalogService()
        // openssl@3 is depended on by a large number of formulae → non-empty.
        let openssl = await service.reverseDependents(of: "openssl@3")
        #expect(!openssl.isEmpty)
        #expect(openssl.count > 50)
        // Sorted ascending by name (stable for the UI).
        let names = openssl.map(\.name)
        #expect(names == names.sorted { $0.localizedCaseInsensitiveCompare($1) == .orderedAscending })
        // A synthetic token nothing depends on → empty, no throw.
        let bogus = await service.reverseDependents(of: "zzz-not-a-real-formula-xyz")
        #expect(bogus.isEmpty)
    }
}
