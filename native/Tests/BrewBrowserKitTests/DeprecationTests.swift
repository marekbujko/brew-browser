import Testing
import Foundation
@testable import BrewBrowserKit

// Tests for the deprecation / disabled status plumbing (feature #2). The pure
// parse + collapse + badge-selection logic is the cross-shell parity core, so
// these cases mirror the Rust `brew/parse.rs` + `catalog/mod.rs` tests so the
// native and Tauri shells derive the SAME status object from the SAME upstream
// keys with the SAME precedence.
//
// Parity contract (both shells):
//   Baseline (catalog, every row): deprecated/disabled + reason/date, NO replacement.
//   Enriched (brew info, detail only): adds the replacement token.
//   Precedence: disabled > deprecated. Replacement collapse: formula then cask.
//   Clean (neither flag) → no badge, no notice (never a placeholder).

@Suite("Deprecation")
struct DeprecationTests {

    // MARK: collapse logic (mirrors Rust parse.rs replacement-collapse test)

    @Test func collapsePicksFormulaOverCask() {
        #expect(collapseReplacement(formula: "wget", cask: "wgetcask") == "wget")
    }

    @Test func collapseFallsBackToCask() {
        #expect(collapseReplacement(formula: nil, cask: "iterm2") == "iterm2")
        #expect(collapseReplacement(formula: "", cask: "iterm2") == "iterm2")
    }

    @Test func collapseNullNullIsNil() {
        #expect(collapseReplacement(formula: nil, cask: nil) == nil)
        #expect(collapseReplacement(formula: "", cask: "") == nil)
    }

    // MARK: badge selection (disabled wins over deprecated)

    @Test func disabledWinsOverDeprecated() {
        let s = DeprecationStatus(deprecated: true, disabled: true)
        #expect(s.badge == .disabled)
        #expect(s.badge?.label == "Disabled")
    }

    @Test func deprecatedOnlyShowsDeprecated() {
        let s = DeprecationStatus(deprecated: true, disabled: false)
        #expect(s.badge == .deprecated)
        #expect(s.badge?.label == "Deprecated")
    }

    @Test func cleanShowsNoBadge() {
        let s = DeprecationStatus()
        #expect(s.isClean)
        #expect(s.badge == nil)
    }

    @Test func activeAccessorsPreferDisabled() {
        let s = DeprecationStatus(
            deprecated: true, disabled: true,
            deprecationReason: "dep reason", disableReason: "dis reason",
            deprecationDate: "2023-01", disableDate: "2024-06",
            deprecationReplacement: "dep-repl", disableReplacement: "dis-repl"
        )
        #expect(s.activeReason == "dis reason")
        #expect(s.activeDate == "2024-06")
        #expect(s.activeReplacement == "dis-repl")
    }

    // MARK: parse from a brew-info dict (replacement honored)

    @Test func parsesFormulaDeprecatedWithReasonAndReplacement() {
        let o: [String: Any] = [
            "deprecated": true,
            "deprecation_date": "2024-01",
            "deprecation_reason": "repository unavailable",
            "deprecation_replacement_formula": "newformula",
            "deprecation_replacement_cask": NSNull(),
            "disabled": false,
        ]
        let s = parseDeprecationStatus(o, includeReplacement: true)
        #expect(s.deprecated)
        #expect(!s.disabled)
        #expect(s.deprecationReason == "repository unavailable")
        #expect(s.deprecationDate == "2024-01")
        #expect(s.deprecationReplacement == "newformula")  // formula collapse
        #expect(s.badge == .deprecated)
    }

    @Test func parsesCaskDisabledWithCaskReplacement() {
        let o: [String: Any] = [
            "disabled": true,
            "disable_date": "2025-02",
            "disable_reason": "discontinued",
            "disable_replacement_cask": "newcask",
            "deprecated": false,
        ]
        let s = parseDeprecationStatus(o, includeReplacement: true)
        #expect(s.disabled)
        #expect(s.disableReplacement == "newcask")  // cask fallback (no formula key)
        #expect(s.badge == .disabled)
    }

    @Test func wgetLikeCleanInfoYieldsNoFlagsNoReplacement() {
        // Mirrors the Rust wget-fixture case: all deprecation fields null/false →
        // clean status, every option nil, no false-positive badge.
        let o: [String: Any] = [
            "deprecated": false, "disabled": false,
            "deprecation_date": NSNull(), "deprecation_reason": NSNull(),
            "disable_date": NSNull(), "disable_reason": NSNull(),
        ]
        let s = parseDeprecationStatus(o, includeReplacement: true)
        #expect(s.isClean)
        #expect(s.deprecationReason == nil)
        #expect(s.deprecationReplacement == nil)
        #expect(s.disableReplacement == nil)
        #expect(s.badge == nil)
    }

    // MARK: catalog path drops the replacement (rows are flags-only on both shells)

    @Test func catalogParseKeepsFlagsButDropsReplacement() {
        let o: [String: Any] = [
            "deprecated": true,
            "deprecation_reason": "use foo",
            "deprecation_replacement_formula": "foo",  // present in the catalog…
        ]
        let s = parseDeprecationStatus(o, includeReplacement: false)
        #expect(s.deprecated)
        #expect(s.deprecationReason == "use foo")
        #expect(s.deprecationReplacement == nil)  // …but never surfaced for rows
    }

    @Test func deprecatedTrueButReasonNullShowsBadgeWithoutReason() {
        // Edge case: deprecated with no reason — badge still shows, no fabricated text.
        let o: [String: Any] = ["deprecated": true]
        let s = parseDeprecationStatus(o, includeReplacement: false)
        #expect(s.badge == .deprecated)
        #expect(s.activeReason == nil)
    }

    // MARK: BrewService.parseFormula / parseCask (live brew-info mapping)

    @Test func parseFormulaMapsDeprecationOntoPackageInfo() {
        let o: [String: Any] = [
            "name": "oldtool",
            "full_name": "oldtool",
            "versions": ["stable": "1.0"],
            "deprecated": true,
            "deprecation_reason": "unmaintained",
            "deprecation_replacement_formula": "newtool",
        ]
        let info = BrewService.parseFormula(o)
        #expect(info.deprecation.deprecated)
        #expect(info.deprecation.deprecationReason == "unmaintained")
        #expect(info.deprecation.deprecationReplacement == "newtool")
    }

    @Test func parseFormulaCleanCaseHasNoFlags() {
        let o: [String: Any] = ["name": "wget", "versions": ["stable": "1.24.5"]]
        let info = BrewService.parseFormula(o)
        #expect(info.deprecation.isClean)
        #expect(info.deprecation.badge == nil)
    }

    @Test func parseCaskMapsDisabledOntoPackageInfo() {
        let o: [String: Any] = [
            "token": "oldcask",
            "name": ["Old Cask"],
            "version": "2.0",
            "disabled": true,
            "disable_replacement_cask": "newcask",
        ]
        let info = BrewService.parseCask(o)
        #expect(info.deprecation.disabled)
        #expect(info.deprecation.disableReplacement == "newcask")
        #expect(info.deprecation.badge == .disabled)
    }

    // MARK: CatalogService maps the flags off real-ish dicts (parity with formulae(from:))

    @Test func realBundledCatalogHasDisabledFormulaeAndDeprecatedCasks() async {
        // Proves the flags plumb through real bundled data (mirrors the Rust
        // "catalog contains >=1 disabled formula AND >=1 deprecated cask" test).
        let all = await CatalogService().all()
        guard !all.isEmpty else {
            // No bundled catalog in this environment — skip rather than fail.
            return
        }
        let disabledFormula = all.first { $0.kind == .formula && $0.deprecation.disabled }
        let deprecatedCask = all.first { $0.kind == .cask && $0.deprecation.deprecated }
        #expect(disabledFormula != nil)
        #expect(deprecatedCask != nil)
        // Catalog rows carry NO replacement (flags-only baseline).
        #expect(disabledFormula?.deprecation.disableReplacement == nil)
        #expect(deprecatedCask?.deprecation.deprecationReplacement == nil)
    }
}

// AppModel-level lookup test: deprecationStatus(token:kind:) returns the catalog
// status for a seeded map, with the bare-token fallback for tap-qualified names.
@MainActor
@Suite("DeprecationStatusLookup")
struct DeprecationStatusLookupTests {

    @Test func statusOfHitsSeededCatalog() async {
        let m = AppModel()
        // Seed the catalog with a deprecated formula + a disabled cask, then run
        // loadCatalog's index build by going through the public catalog setter is
        // not exposed — instead drive it via the real loader and assert on a real
        // hit. Use a known disabled formula from the bundled catalog.
        await m.loadCatalog()
        guard !m.catalog.isEmpty,
              let flagged = m.catalog.first(where: { !$0.deprecation.isClean }) else {
            return  // no bundled catalog here — skip
        }
        let status = m.deprecationStatus(flagged.token, flagged.kind)
        #expect(!status.isClean)
        #expect(status.badge == flagged.deprecation.badge)
    }

    @Test func statusOfCleanForUnknownToken() async {
        let m = AppModel()
        await m.loadCatalog()
        let status = m.deprecationStatus("this-token-does-not-exist-xyz", .formula)
        #expect(status.isClean)
        #expect(status.badge == nil)
    }
}
