// SVGSizingTests.swift
// MermaidRenderKitTests

import CoreGraphics
import XCTest

@testable import MermaidRenderKit

/// Intrinsic-sizing helpers for the native display path (PRD F6). Task 16
/// extends this with the T13 narrow-container assertion.
final class SVGSizingTests: XCTestCase {

    func testViewBoxExtractionReturnsWidthHeightAspect() {
        let svg = #"<svg viewBox="0 0 320 160" width="100%"><rect/></svg>"#
        let size = SVGSizing.intrinsicSize(fromSVG: svg)
        XCTAssertEqual(size?.width, 320)
        XCTAssertEqual(size?.height, 160)
        XCTAssertEqual(size?.aspectRatio ?? 0, 2.0, accuracy: 0.0001)
    }

    func testFallsBackToWidthHeightAttributesWithoutViewBox() {
        let svg = #"<svg width="240px" height="120px"><rect/></svg>"#
        let size = SVGSizing.intrinsicSize(fromSVG: svg)
        XCTAssertEqual(size?.width, 240)
        XCTAssertEqual(size?.height, 120)
    }

    func testNoDimensionsReturnsNil() {
        XCTAssertNil(SVGSizing.intrinsicSize(fromSVG: "<svg><rect/></svg>"))
        XCTAssertNil(SVGSizing.intrinsicSize(fromSVG: ""))
    }

    func testDisplayHeightFollowsAspectAboveFloor() {
        let size = SVGIntrinsicSize(width: 200, height: 100) // aspect 2:1
        // 600 wide / 2.0 = 300, well above the floor.
        XCTAssertEqual(SVGSizing.displayHeight(for: size, availableWidth: 600), 300, accuracy: 0.0001)
    }

    func testDisplayHeightRespects80ptFloor() {
        let size = SVGIntrinsicSize(width: 200, height: 20) // very wide/short, aspect 10:1
        // 300 wide / 10 = 30 → floored to 80 (F6-AC5).
        XCTAssertEqual(SVGSizing.displayHeight(for: size, availableWidth: 300), SVGSizing.minimumHeight)
    }

    func testDisplayHeightFloorWhenSizeUnknown() {
        XCTAssertEqual(SVGSizing.displayHeight(for: nil, availableWidth: 400), SVGSizing.minimumHeight)
    }

    // MARK: - F6-AC3 (non-square) + T13 (narrow container)

    func testNonSquareDiagramSizedByAspect() {
        // Tall diagram (aspect 0.5 = 1:2): width 200 → height 400 (preserves
        // aspect, above floor) — no squashing to square.
        let tall = SVGIntrinsicSize(width: 100, height: 200)
        XCTAssertEqual(tall.aspectRatio, 0.5, accuracy: 0.0001)
        XCTAssertEqual(SVGSizing.displayHeight(for: tall, availableWidth: 200), 400, accuracy: 0.0001)
    }

    /// T13: a real ≥20-element diagram in a narrow container keeps a height at
    /// or above the 80-pt floor (legible), and the view's aspect-fit sizing
    /// (`aspectRatio(_:.fit)` + `maxWidth: .infinity`) bounds width to the
    /// container — no horizontal clipping. The native path uses `.task`, never
    /// `DispatchQueue.asyncAfter` polling (contrast: the WebView coordinator).
    func testManyElementDiagramInNarrowContainerRespectsFloor() {
        var lines = ["flowchart TD"]
        for i in 0..<24 { lines.append("N\(i)-->N\(i + 1)") } // ≥20 elements
        let result = MermaidNativeRenderer.render(code: lines.joined(separator: "\n"), format: .vectorSVG)
        guard case let .success(.svg(svg), _) = result else {
            return XCTFail("expected an SVG success, got \(result)")
        }
        let size = SVGSizing.intrinsicSize(fromSVG: svg)
        XCTAssertNotNil(size, "a 25-node flowchart must carry a viewBox")
        let narrow: CGFloat = 50
        let height = SVGSizing.displayHeight(for: size, availableWidth: narrow)
        XCTAssertGreaterThanOrEqual(height, SVGSizing.minimumHeight, "narrow container must not collapse height below 80pt")
    }
}

/// End-to-end egress: a REAL mmdr render of an MVP diagram, run through the
/// sanitizer, carries no external reference (defense-in-depth over the already
/// clean fork-side baseline — PRD F1-AC5) and yields a usable intrinsic size.
final class NativeRenderEgressTests: XCTestCase {

    func testRenderedMVPSVGIsEgressFreeAndSized() {
        let result = MermaidNativeRenderer.render(code: "flowchart LR\nA-->B", format: .vectorSVG)
        guard case let .success(.svg(svg), _) = result else {
            return XCTFail("expected an SVG success, got \(result)")
        }
        let sanitized = SVGSanitizer.sanitize(svg)
        // No fetchable external reference. `xmlns="http://www.w3.org/2000/svg"`
        // is a namespace identifier (never fetched), so we assert on href/url()
        // egress surfaces rather than on a bare "http://" substring.
        XCTAssertFalse(sanitized.contains("href=\"http"))
        XCTAssertFalse(sanitized.contains("href='http"))
        XCTAssertFalse(sanitized.lowercased().contains("url(http"))
        XCTAssertFalse(sanitized.lowercased().contains("<script"))
        XCTAssertNotNil(SVGSizing.intrinsicSize(fromSVG: sanitized), "rendered SVG should carry a viewBox/size")
    }
}
