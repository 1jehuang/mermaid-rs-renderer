// MermaidNativeRendererTests.swift
// MermaidRenderKitTests

import XCTest

@testable import MermaidRenderKit

/// Phase 6 / Task 9 acceptance: the native FFI adapter round-trips one MVP
/// diagram to `.success` for BOTH formats, and maps non-OK statuses safely.
final class MermaidNativeRendererTests: XCTestCase {

    /// A minimal MVP-allowlisted flowchart (matches the fork's own FFI tests).
    private let mvpFlowchart = "flowchart LR\nA-->B"

    // MARK: - Round-trip (T9 acceptance: BOTH formats → .success)

    func testRoundTripVectorSVGSucceeds() {
        let result = MermaidNativeRenderer.render(code: mvpFlowchart, format: .vectorSVG)
        guard case let .success(payload, _) = result else {
            return XCTFail("expected .success, got \(result)")
        }
        guard case let .svg(svg) = payload else {
            return XCTFail("expected .svg payload, got \(payload)")
        }
        XCTAssertFalse(svg.isEmpty, "SVG payload should be non-empty")
        XCTAssertTrue(svg.contains("<svg"), "payload should be SVG markup")
    }

    func testRoundTripRasterPNGSucceeds() {
        let result = MermaidNativeRenderer.render(code: mvpFlowchart, format: .rasterPNG)
        guard case let .success(payload, _) = result else {
            return XCTFail("expected .success, got \(result)")
        }
        guard case let .png(data) = payload else {
            return XCTFail("expected .png payload, got \(payload)")
        }
        XCTAssertFalse(data.isEmpty, "PNG payload should be non-empty")
        // PNG 8-byte magic number: 89 50 4E 47 0D 0A 1A 0A.
        let signature: [UInt8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        XCTAssertEqual(Array(data.prefix(8)), signature, "payload should be a PNG")
    }

    // MARK: - Status mapping

    func testUnsupportedDiagramTypeMapsToUnsupported() {
        // `journey` is detected but outside the MVP allowlist → status 5.
        let result = MermaidNativeRenderer.render(
            code: "journey\n  title My Day\n  Wake: 5: Me",
            format: .vectorSVG
        )
        guard case .unsupported = result else {
            return XCTFail("expected .unsupported, got \(result)")
        }
    }

    func testEmptySourceDoesNotCrashAndReturnsResult() {
        // Empty input must not crash; any defined case is acceptable here —
        // the point is the adapter copies/frees safely and returns.
        _ = MermaidNativeRenderer.render(code: "", format: .vectorSVG)
        _ = MermaidNativeRenderer.render(code: "", format: .rasterPNG)
    }

    // MARK: - Binary metadata

    func testVersionIsNonEmpty() {
        XCTAssertFalse(MermaidNativeRenderer.version.isEmpty)
    }

    func testWarmUpIsIdempotent() {
        // Latency hint only; must be safe to call repeatedly.
        MermaidNativeRenderer.warmUp()
        MermaidNativeRenderer.warmUp()
    }
}
