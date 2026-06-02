// MermaidRenderResult.swift
// MermaidRenderKit

import CoreGraphics
import Foundation

/// Which output format the native renderer produces (PRD §4.6, D1).
///
/// The prebuilt binary carries BOTH paths; this selects which FFI entry point
/// the adapter calls. Default is `.vectorSVG` for crispness at any zoom.
public enum MermaidDisplayFormat: Sendable, Hashable {
    /// Vector SVG (`mmdr_render_svg`) — handed to the SVGView sanitizer/display.
    case vectorSVG
    /// Raster PNG (`mmdr_render_png`) — wrapped in `Data` for `UIImage`/`NSImage`.
    case rasterPNG
}

/// The decoded payload of a successful native render (PRD §4.6).
///
/// `.svg` carries UTF-8 SVG text; `.png` carries raw binary PNG bytes (NEVER
/// decode PNG as UTF-8 — wrap in `Data`).
public enum MermaidRenderPayload: Sendable, Hashable {
    case svg(String)
    case png(Data)
}

/// The result of a native Mermaid render, mapped from the FFI `MmdrStatus`
/// (PRD §4.6). The status→case mapping is identical for both render functions.
public enum MermaidRenderResult: Sendable {
    /// `MmdrStatus_Ok` (0). For `.svg`, `intrinsicSize` is the SVG's
    /// `viewBox`/`width`-`height` (F6); for `.png` it is `.zero` (no viewBox —
    /// size a PNG from the image's own pixel aspect).
    case success(payload: MermaidRenderPayload, intrinsicSize: CGSize)
    /// `MmdrStatus_ErrUnsupported` (5) — diagram type outside the MVP allowlist.
    case unsupported(type: String?)
    /// `MmdrStatus` 1–4 (parser errors). `line`/`col` are `nil` when the FFI
    /// reports `loc_valid == 0`.
    case parseError(message: String, line: Int?, col: Int?)
    /// `MmdrStatus` 99 (panic, redacted payload) plus the defensive map-only
    /// 6/7 (UTF-8/null guards) and 8/9 (version/size, too-large). Status 10
    /// (`ErrTimeout`) is never emitted by Rust in MVP — its branch is a
    /// defensive no-op here (the MVP timeout is Swift-side; Task 15).
    case renderError(message: String)
}
