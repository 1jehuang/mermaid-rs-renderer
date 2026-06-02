// MermaidThemeOptions.swift
// MermaidRenderKit

import Cmmdr
import Foundation

/// Headless theme inputs for the C-ABI renderer.
///
/// All colors are raw `0xRRGGBBAA` values. The caller resolves any
/// adaptive/dynamic platform colors (light/dark, `Color`, `NSColor`,
/// `UIColor`) to concrete RGBA **before** constructing this value — the
/// renderer is headless and has no UI-framework dependency.
public struct MermaidThemeOptions: Sendable, Hashable {
    /// Background slot, `0xRRGGBBAA`.
    public let backgroundRGBA: UInt32
    /// Foreground (text + line) slot, `0xRRGGBBAA`.
    public let foregroundRGBA: UInt32
    /// Accent (primary) slot, `0xRRGGBBAA`.
    public let accentRGBA: UInt32

    public init(backgroundRGBA: UInt32, foregroundRGBA: UInt32, accentRGBA: UInt32) {
        self.backgroundRGBA = backgroundRGBA
        self.foregroundRGBA = foregroundRGBA
        self.accentRGBA = accentRGBA
    }

    /// Build the C-ABI ``MmdrOptions`` from these raw values.
    ///
    /// `abi_version`/`struct_size` are stamped so the binary can assert
    /// header/binary agreement; `base_theme` is `0` (the modern base theme) and
    /// all three color slots are overridden. The DoS caps (`max_source_bytes`,
    /// `max_nodes`, `timeout_ms`) are left `0`, which the FFI reads as "apply
    /// the built-in defaults".
    public func toMmdrOptions() -> MmdrOptions {
        var options = MmdrOptions()
        options.abi_version = UInt32(MMDR_ABI_VERSION)
        options.struct_size = UInt32(MemoryLayout<MmdrOptions>.size)
        options.base_theme = 0
        options.color_override_mask = UInt32(MASK_BACKGROUND | MASK_FOREGROUND | MASK_ACCENT)
        options.background_rgba = backgroundRGBA
        options.foreground_rgba = foregroundRGBA
        options.accent_rgba = accentRGBA
        return options
    }

    /// Pack 8-bit channels into a single `0xRRGGBBAA` value.
    public static func pack(r: UInt8, g: UInt8, b: UInt8, a: UInt8) -> UInt32 {
        (UInt32(r) << 24) | (UInt32(g) << 16) | (UInt32(b) << 8) | UInt32(a)
    }
}
