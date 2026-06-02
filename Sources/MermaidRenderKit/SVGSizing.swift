// SVGSizing.swift
// MermaidRenderKit

import CoreGraphics
import Foundation

/// Intrinsic dimensions of a rendered diagram (PRD F6).
public struct SVGIntrinsicSize: Equatable, Sendable {
    public let width: CGFloat
    public let height: CGFloat

    public init(width: CGFloat, height: CGFloat) {
        self.width = width
        self.height = height
    }

    /// Width-over-height; used to scale to the available container width.
    public var aspectRatio: CGFloat {
        height > 0 ? width / height : 1
    }
}

/// Geometry helpers for sizing a native diagram from its SVG (PRD F6).
public enum SVGSizing {

    /// The minimum displayed height in points, regardless of intrinsic size
    /// (F6-AC5) — keeps tiny diagrams legible and tappable.
    public static let minimumHeight: CGFloat = 80

    /// Extract the intrinsic size from an SVG string: the `viewBox`'s
    /// width/height (preferred), falling back to the `width`/`height`
    /// attributes. Returns `nil` when neither yields positive dimensions.
    public static func intrinsicSize(fromSVG svg: String) -> SVGIntrinsicSize? {
        if let box = viewBoxSize(from: svg) {
            return box
        }
        return widthHeightSize(from: svg)
    }

    /// Height for a diagram scaled to fill `availableWidth`, never below the
    /// `minimumHeight` floor (F6-AC5). Falls back to the floor when the size is
    /// unknown or degenerate.
    public static func displayHeight(
        for size: SVGIntrinsicSize?,
        availableWidth: CGFloat
    ) -> CGFloat {
        guard let size, size.aspectRatio > 0, availableWidth > 0 else {
            return minimumHeight
        }
        return max(availableWidth / size.aspectRatio, minimumHeight)
    }

    // MARK: - Parsing

    /// `viewBox="minX minY width height"` → the last two values.
    private static func viewBoxSize(from svg: String) -> SVGIntrinsicSize? {
        guard let raw = attributeValue("viewBox", in: svg) else { return nil }
        let nums = raw
            .split(whereSeparator: { $0 == " " || $0 == "," || $0 == "\t" || $0 == "\n" })
            .compactMap { Double($0) }
        guard nums.count == 4, nums[2] > 0, nums[3] > 0 else { return nil }
        return SVGIntrinsicSize(width: CGFloat(nums[2]), height: CGFloat(nums[3]))
    }

    /// `width="…"` / `height="…"` attributes (numeric, ignoring unit suffixes).
    private static func widthHeightSize(from svg: String) -> SVGIntrinsicSize? {
        guard let w = attributeValue("width", in: svg).flatMap(leadingNumber),
              let h = attributeValue("height", in: svg).flatMap(leadingNumber),
              w > 0, h > 0
        else { return nil }
        return SVGIntrinsicSize(width: CGFloat(w), height: CGFloat(h))
    }

    /// First value of attribute `name` in `svg` (single/double quotes).
    private static func attributeValue(_ name: String, in svg: String) -> String? {
        let pattern = "\\b\(name)\\s*=\\s*(\"([^\"]*)\"|'([^']*)')"
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]),
              let match = regex.firstMatch(in: svg, range: NSRange(svg.startIndex..., in: svg))
        else { return nil }
        for group in [2, 3] {
            if let r = Range(match.range(at: group), in: svg) {
                return String(svg[r])
            }
        }
        return nil
    }

    /// Leading numeric portion of a length string (e.g. `"320px"` → `320`).
    private static func leadingNumber(_ s: String) -> Double? {
        let trimmed = s.trimmingCharacters(in: .whitespaces)
        var end = trimmed.startIndex
        while end < trimmed.endIndex {
            let c = trimmed[end]
            // A `-` is only valid as the very first character (a sign), never
            // mid-token; `.` and digits continue the number.
            let signOK = c == "-" && end == trimmed.startIndex
            guard c.isNumber || c == "." || signOK else { break }
            end = trimmed.index(after: end)
        }
        return Double(trimmed[..<end])
    }
}
