// MermaidThemeOptionsTests.swift
// MermaidRenderKitTests

import Cmmdr
import XCTest

@testable import MermaidRenderKit

final class MermaidThemeOptionsTests: XCTestCase {

    func testPackComposesRRGGBBAA() {
        XCTAssertEqual(MermaidThemeOptions.pack(r: 0xFF, g: 0x00, b: 0x00, a: 0xFF), 0xFF00_00FF)
        XCTAssertEqual(MermaidThemeOptions.pack(r: 0x00, g: 0x00, b: 0x00, a: 0xFF), 0x0000_00FF)
        XCTAssertEqual(MermaidThemeOptions.pack(r: 0x12, g: 0x34, b: 0x56, a: 0x78), 0x1234_5678)
    }

    func testToMmdrOptionsStampsAbiAndOverridesAllSlots() {
        let theme = MermaidThemeOptions(
            backgroundRGBA: 0x1122_33FF,
            foregroundRGBA: 0x4455_66FF,
            accentRGBA: 0x7788_99FF
        )
        let o = theme.toMmdrOptions()

        XCTAssertEqual(o.abi_version, UInt32(MMDR_ABI_VERSION))
        XCTAssertEqual(o.struct_size, UInt32(MemoryLayout<MmdrOptions>.size))
        XCTAssertEqual(o.base_theme, 0)
        XCTAssertEqual(o.color_override_mask, UInt32(MASK_BACKGROUND | MASK_FOREGROUND | MASK_ACCENT))
        XCTAssertEqual(o.background_rgba, 0x1122_33FF)
        XCTAssertEqual(o.foreground_rgba, 0x4455_66FF)
        XCTAssertEqual(o.accent_rgba, 0x7788_99FF)
        // DoS caps left zero → FFI applies its built-in defaults.
        XCTAssertEqual(o.max_source_bytes, 0)
        XCTAssertEqual(o.max_nodes, 0)
        XCTAssertEqual(o.timeout_ms, 0)
    }

    func testRoundTripPreservesColors() {
        let bg = MermaidThemeOptions.pack(r: 10, g: 20, b: 30, a: 255)
        let fg = MermaidThemeOptions.pack(r: 200, g: 210, b: 220, a: 255)
        let ac = MermaidThemeOptions.pack(r: 90, g: 80, b: 70, a: 128)
        let o = MermaidThemeOptions(backgroundRGBA: bg, foregroundRGBA: fg, accentRGBA: ac).toMmdrOptions()
        XCTAssertEqual(o.background_rgba, bg)
        XCTAssertEqual(o.foreground_rgba, fg)
        XCTAssertEqual(o.accent_rgba, ac)
    }

    func testHashableEquality() {
        let a = MermaidThemeOptions(backgroundRGBA: 1, foregroundRGBA: 2, accentRGBA: 3)
        let b = MermaidThemeOptions(backgroundRGBA: 1, foregroundRGBA: 2, accentRGBA: 3)
        let c = MermaidThemeOptions(backgroundRGBA: 9, foregroundRGBA: 2, accentRGBA: 3)
        XCTAssertEqual(a, b)
        XCTAssertNotEqual(a, c)
        XCTAssertEqual(Set([a, b, c]).count, 2)
    }

    func testSendableConformance() {
        func requireSendable<T: Sendable>(_: T.Type) {}
        requireSendable(MermaidThemeOptions.self)
    }
}
