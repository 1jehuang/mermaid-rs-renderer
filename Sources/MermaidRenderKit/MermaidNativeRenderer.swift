// MermaidNativeRenderer.swift
// MermaidRenderKit

import Cmmdr
import CoreGraphics
import Foundation

/// Thin Swift adapter over the prebuilt C-ABI renderer (`Cmmdr`, PRD §4.6).
///
/// This is the INITIAL adapter (Phase 6 / Task 9): it calls
/// `mmdr_render_svg` / `mmdr_render_png` with default options, validates the
/// result's ABI guards, copies the payload, frees the handle, and maps the
/// status to a `MermaidRenderResult`. Theme mapping (Task 13), the SVGView
/// sanitizer + display (Task 10), the registered-renderer seam (Task 11), and
/// the Swift-side DoS timeout (Task 15) layer on top in later phases.
public enum MermaidNativeRenderer {

    /// A `'static`, NUL-terminated renderer version string from the binary.
    public static var version: String {
        String(cString: mmdr_version())
    }

    /// Force the embedded-font `OnceLock`s to evaluate early, off the render
    /// hot path (PRD Arbitration A). Idempotent, thread-safe, panic-isolated;
    /// a latency hint only — NOT required for correctness.
    public static func warmUp() {
        mmdr_init()
    }

    /// Render `code` to the requested `format`, returning a status-mapped
    /// result. When `options` is nil the FFI applies its own defaults (256 KB
    /// source cap, 5000-node cap, modern theme); pass a
    /// ``MermaidThemeOptions/toMmdrOptions()`` value to override
    /// theme/spacing/caps.
    public static func render(
        code: String,
        format: MermaidDisplayFormat,
        options: MmdrOptions? = nil
    ) -> MermaidRenderResult {
        let bytes = Array(code.utf8)
        let raw: UnsafeMutablePointer<MmdrResult>? = bytes.withUnsafeBufferPointer { buffer in
            withOptionsPointer(options) { optsPtr in
                switch format {
                case .vectorSVG:
                    return mmdr_render_svg(buffer.baseAddress, buffer.count, optsPtr)
                case .rasterPNG:
                    return mmdr_render_png(buffer.baseAddress, buffer.count, optsPtr)
                }
            }
        }

        // Null only on allocation failure (PRD §4.3).
        guard let result = raw else {
            return .renderError(message: "renderer returned a null result")
        }
        // The handle is freed exactly once, via the only correct deallocator.
        defer { mmdr_result_free(result) }

        let value = result.pointee

        // ABI guards: the header and the linked binary must agree (PRD §4.6).
        guard value.abi_version == UInt32(MMDR_ABI_VERSION),
              value.struct_size == UInt32(MemoryLayout<MmdrResult>.size)
        else {
            return .renderError(message: "ABI mismatch between header and binary")
        }

        return mapResult(value, format: format)
    }

    /// Call `body` with a pointer to `options` (or nil for FFI defaults),
    /// keeping the value alive for the duration of the call.
    private static func withOptionsPointer<R>(
        _ options: MmdrOptions?,
        _ body: (UnsafePointer<MmdrOptions>?) -> R
    ) -> R {
        guard var options else { return body(nil) }
        return withUnsafePointer(to: &options) { body($0) }
    }

    /// Map a validated `MmdrResult` to the Swift result enum. The status→case
    /// mapping is identical for both render functions (PRD §4.6).
    private static func mapResult(
        _ value: MmdrResult,
        format: MermaidDisplayFormat
    ) -> MermaidRenderResult {
        // `MmdrResult.status` is `int32_t`; the `MmdrStatus` enum imports with a
        // `UInt32` raw value — convert each constant to `Int32` to match.
        switch value.status {
        case Int32(MmdrStatus_Ok.rawValue):
            let payload = decodePayload(value, format: format)
            return .success(payload: payload, intrinsicSize: intrinsicSize(of: payload))

        case Int32(MmdrStatus_ErrUnsupported.rawValue):
            // Payload is the redacted "diagram type not yet supported" message;
            // the concrete type name is not exposed by the FFI in MVP.
            return .unsupported(type: nil)

        case Int32(MmdrStatus_ErrUnexpectedToken.rawValue),
             Int32(MmdrStatus_ErrUnknownParticipant.rawValue),
             Int32(MmdrStatus_ErrUnclosedSubgraph.rawValue),
             Int32(MmdrStatus_ErrInvalidDirective.rawValue):
            let locValid = value.loc_valid != 0
            return .parseError(
                message: decodeMessage(value),
                line: locValid ? Int(value.err_line) : nil,
                col: (locValid && value.err_col != 0) ? Int(value.err_col) : nil
            )

        default:
            // 6/7 (defensive UTF-8/null guards), 8 (version), 9 (too large),
            // 99 (panic, redacted), and any unexpected code map to a generic,
            // safe render error. Status 10 (ErrTimeout) is never emitted in MVP.
            return .renderError(message: decodeMessage(value))
        }
    }

    /// Intrinsic size of a successful payload. SVG → its `viewBox`/`width`-
    /// `height` (F6); PNG carries no viewBox, so `.zero` (the view sizes a PNG
    /// from the image's own pixel aspect).
    private static func intrinsicSize(of payload: MermaidRenderPayload) -> CGSize {
        switch payload {
        case let .svg(svg):
            guard let size = SVGSizing.intrinsicSize(fromSVG: svg) else { return .zero }
            return CGSize(width: size.width, height: size.height)
        case .png:
            return .zero
        }
    }

    /// Decode a successful payload per format: SVG → UTF-8 `String`, PNG → raw
    /// `Data`. Empty payload yields an empty value of the matching kind.
    private static func decodePayload(
        _ value: MmdrResult,
        format: MermaidDisplayFormat
    ) -> MermaidRenderPayload {
        switch format {
        case .vectorSVG:
            return .svg(decodeMessage(value))
        case .rasterPNG:
            guard let ptr = value.data_ptr, let count = payloadCount(value) else {
                return .png(Data())
            }
            return .png(Data(bytes: ptr, count: count))
        }
    }

    /// Decode the payload bytes as a UTF-8 string (SVG text or a redacted
    /// error message). Returns "" on a null/empty/oversized/invalid payload.
    private static func decodeMessage(_ value: MmdrResult) -> String {
        guard let ptr = value.data_ptr, let count = payloadCount(value) else { return "" }
        let buffer = UnsafeBufferPointer(start: ptr, count: count)
        return String(decoding: buffer, as: UTF8.self)
    }

    /// Validate `data_len` (a `u64` from the binary) and narrow it to `Int`.
    /// Returns nil for an empty payload or one larger than `Int.max` — a
    /// `UInt64 → Int` conversion that overflowed would trap, so it is rejected
    /// defensively rather than trusted.
    private static func payloadCount(_ value: MmdrResult) -> Int? {
        let len = value.data_len
        guard len > 0, len <= UInt64(Int.max) else { return nil }
        return Int(len)
    }
}
