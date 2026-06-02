// MermaidRenderGuardTests.swift
// MermaidRenderKitTests

import XCTest

@testable import MermaidRenderKit

/// DoS protection layers (PRD §4.3 / §5.6 / T7).
final class MermaidRenderGuardTests: XCTestCase {

    // T7(a): source over 256 KB is rejected immediately (no FFI call).
    func testOversizedSourceReturnsRenderErrorWithoutRendering() async {
        let oversized = String(repeating: "A", count: MermaidRenderGuard.maxSourceBytes + 1)
        let result = await MermaidRenderGuard.render(code: oversized, format: .vectorSVG)
        guard case let .renderError(message) = result else {
            return XCTFail("expected .renderError for oversized source, got \(result)")
        }
        XCTAssertTrue(message.contains("256 KB"))
    }

    func testAtCapSourceIsNotRejectedByTheSizeGate() async {
        // Exactly at the cap passes the size gate (then fails as a parse error,
        // not the size error — proving the gate boundary is inclusive).
        let atCap = String(repeating: "A", count: MermaidRenderGuard.maxSourceBytes)
        let result = await MermaidRenderGuard.render(code: atCap, format: .vectorSVG)
        if case let .renderError(message) = result {
            XCTAssertFalse(message.contains("256 KB"), "at-cap source must not trip the size gate")
        }
    }

    // T7(b): a source exceeding the 5000-node cap returns ErrTooLarge →
    // .renderError (enforced FFI-side, pre-layout).
    func testNodeCapTrips() async {
        var lines = ["flowchart LR"]
        for i in 0..<6000 { lines.append("N\(i)-->N\(i + 1)") }
        let source = lines.joined(separator: "\n")
        XCTAssertLessThanOrEqual(source.utf8.count, MermaidRenderGuard.maxSourceBytes,
                                 "this case must pass the size gate to exercise the node cap")
        let result = await MermaidRenderGuard.render(code: source, format: .vectorSVG)
        guard case .renderError = result else {
            return XCTFail("expected .renderError (ErrTooLarge) for >5000 nodes, got \(result)")
        }
    }

    // T7(c): the timeout mechanism abandons a slow operation.
    func testTimeoutAbandonsSlowOperation() async {
        let result = await MermaidRenderGuard.withTimeout(seconds: 0.05) { () async -> String in
            try? await Task.sleep(nanoseconds: 5_000_000_000) // 5 s — far past the deadline
            return "operation-finished"
        } onTimeout: {
            "timed-out"
        }
        XCTAssertEqual(result, "timed-out")
    }

    func testFastOperationReturnsItsValue() async {
        let result = await MermaidRenderGuard.withTimeout(seconds: 5.0) { () async -> String in
            "fast"
        } onTimeout: {
            "timed-out"
        }
        XCTAssertEqual(result, "fast")
    }

    // A normal MVP render still succeeds through the guard.
    func testNormalRenderSucceedsThroughGuard() async {
        let result = await MermaidRenderGuard.render(code: "flowchart LR\nA-->B", format: .vectorSVG)
        guard case .success = result else {
            return XCTFail("expected .success, got \(result)")
        }
    }
}
