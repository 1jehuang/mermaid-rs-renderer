// SVGSanitizer.swift
// MermaidRenderKit

import Foundation

/// Egress sanitizer for the vector (SVGView) display path
/// (PRD §4.7a / §5.2 / F1-AC5; CWE-20/79/918).
///
/// `SVGView` is a renderer, not a sanitizer (F-07), so before any SVG reaches
/// it the adapter neutralizes every reference that could trigger a network or
/// filesystem fetch. mmdr's emitter is already clean (fork-side T2 asserts MVP
/// SVGs never emit these); this pass is defense-in-depth against adversarial or
/// future-emitter SVG. `SVGView`'s default `SVGLinker.none` (which returns nil
/// from `load(src:)`) is a second, runtime backstop covering resource LOAD — a
/// distinct interception point from this string sanitizer, so both are kept.
///
/// `XMLDocument` is macOS-only, so this is a portable `NSRegularExpression`
/// string-scan pass that runs identically on iOS and macOS. It neutralizes:
///   1. `<image>` whose `href` OR `xlink:href` is not a `data:` URI (CWE-20 —
///      both attribute forms checked: SVG 1.1 `xlink:href`, SVG 2 `href`).
///   2. `<script>` elements (paired and self-closing).
///   3. Event-handler attributes (`onload`, `onclick`, …).
///   4. External `<use>` references — any `href`/`xlink:href` not a local
///      `#fragment` reference (CWE-918).
///   5. CSS `url()` with non-`data:` schemes in inline `style=` attributes and
///      in `<style>` block content (CWE-918/CWE-79).
public enum SVGSanitizer {

    /// Return a sanitized copy of `svg` with every non-`data:` external
    /// reference, `<script>`, and event handler neutralized. Order matters:
    /// scripts and `<style>` blocks are removed/cleaned before attribute passes
    /// so their internal `on*=`/`href=` text cannot survive as a false match.
    public static func sanitize(_ svg: String) -> String {
        var result = svg
        result = stripScripts(result)
        result = stripForeignObjects(result)
        result = cleanStyleBlocks(result)
        result = neutralizeImageHrefs(result)
        result = neutralizeUseHrefs(result)
        result = neutralizeAnchorHrefs(result)
        result = stripEventHandlers(result)
        result = neutralizeStyleAttributeURLs(result)
        return result
    }

    // MARK: - Passes

    /// A safe inline-resource value: a `data:image/*` URI (but never the
    /// script-capable `data:image/svg+xml`, which can carry its own handlers).
    private static func isSafeDataImage(_ value: String) -> Bool {
        let v = value.lowercased()
        return v.hasPrefix("data:image/") && !v.hasPrefix("data:image/svg+xml")
    }

    /// A same-document fragment reference (`#id`) — never external.
    private static func isLocalFragment(_ value: String) -> Bool {
        value.hasPrefix("#")
    }

    /// (2) Remove `<script>…</script>`, self-closing `<script .../>`, and any
    /// residual bare-open `<script …>` (malformed but adversarial) tag.
    private static func stripScripts(_ svg: String) -> String {
        var s = replaceAll(in: svg, pattern: "<script\\b[\\s\\S]*?</script\\s*>", with: "")
        s = replaceAll(in: s, pattern: "<script\\b[^>]*/\\s*>", with: "")
        s = replaceAll(in: s, pattern: "<script\\b[^>]*>", with: "")
        return s
    }

    /// Remove `<foreignObject>…</foreignObject>` (can embed arbitrary HTML).
    private static func stripForeignObjects(_ svg: String) -> String {
        var s = replaceAll(in: svg, pattern: "<foreignObject\\b[\\s\\S]*?</foreignObject\\s*>", with: "")
        s = replaceAll(in: s, pattern: "<foreignObject\\b[^>]*/\\s*>", with: "")
        return s
    }

    /// (5a) Within each `<style>…</style>` block, strip `url(<non-data>)` tokens
    /// and any `@import` rule (which can pull external CSS without `url()`).
    private static func cleanStyleBlocks(_ svg: String) -> String {
        rewriteMatches(in: svg, pattern: "<style\\b[^>]*>[\\s\\S]*?</style\\s*>") { block in
            stripCSSImports(neutralizeCSSURLs(in: block))
        }
    }

    /// (1) For every `<image …>`/`<feImage …>` start tag, drop any
    /// `href`/`xlink:href` that is not a safe `data:image/*` URI (CWE-918). The
    /// SVG filter primitive `<feImage>` is an independent external-load vector.
    private static func neutralizeImageHrefs(_ svg: String) -> String {
        rewriteMatches(in: svg, pattern: "<(?:image|feImage)\\b[^>]*>") { tag in
            neutralizeHrefs(in: tag, keep: isSafeDataImage)
        }
    }

    /// (4) For every `<use …>` start tag, drop any `href`/`xlink:href` that is
    /// not a local `#fragment` reference (blocks external-document `<use>`).
    private static func neutralizeUseHrefs(_ svg: String) -> String {
        rewriteMatches(in: svg, pattern: "<use\\b[^>]*>") { tag in
            neutralizeHrefs(in: tag, keep: isLocalFragment)
        }
    }

    /// For every `<a …>` start tag, drop any `href`/`xlink:href` that is not a
    /// local `#fragment` — Mermaid's `click NodeId "URL"` directive emits an
    /// `<a href="URL">` wrapper, so untrusted source can otherwise inject an
    /// external/`javascript:` navigation target (CWE-601/CWE-918/CWE-79).
    private static func neutralizeAnchorHrefs(_ svg: String) -> String {
        rewriteMatches(in: svg, pattern: "<a\\b[^>]*>") { tag in
            neutralizeHrefs(in: tag, keep: isLocalFragment)
        }
    }

    /// (3) Remove every `on<event>=…` attribute — quoted OR unquoted value.
    private static func stripEventHandlers(_ svg: String) -> String {
        replaceAll(
            in: svg,
            pattern: "\\s+on[a-zA-Z]+\\s*=\\s*(\"[^\"]*\"|'[^']*'|[^\\s>]+)",
            with: ""
        )
    }

    /// (5b) Strip `url(<non-data>)` inside inline `style=…` (quoted OR unquoted).
    private static func neutralizeStyleAttributeURLs(_ svg: String) -> String {
        rewriteMatches(in: svg, pattern: "\\bstyle\\s*=\\s*(\"[^\"]*\"|'[^']*'|[^\\s>]+)") { attr in
            neutralizeCSSURLs(in: attr)
        }
    }

    // MARK: - Helpers

    /// Drop `href` and `xlink:href` attributes within a single start-tag string
    /// whose value fails `keep`. The whole attribute (name + value) is removed
    /// so no dangling reference remains. Handles quoted and unquoted values.
    private static func neutralizeHrefs(
        in tag: String,
        keep: (String) -> Bool
    ) -> String {
        rewriteMatches(
            in: tag,
            pattern: "\\s+(?:xlink:href|href)\\s*=\\s*(\"[^\"]*\"|'[^']*'|[^\\s>]+)"
        ) { attr in
            let value = attributeValue(from: attr)
            return keep(value) ? attr : ""
        }
    }

    /// Replace `url(<target>)` with `url()` unless the target is a safe
    /// `data:image/*` URI or a local `#fragment`; everything else (http(s)://,
    /// file://, protocol-relative `//`, `data:text/html`, …) is neutralized.
    private static func neutralizeCSSURLs(in css: String) -> String {
        rewriteMatches(in: css, pattern: "url\\(\\s*(['\"]?)([^'\")]*)\\1\\s*\\)") { token in
            let inner = cssURLTarget(from: token)
            if isSafeDataImage(inner) || isLocalFragment(inner) {
                return token
            }
            return "url()"
        }
    }

    /// Remove `@import …;` rules (and a trailing unterminated `@import`).
    private static func stripCSSImports(_ css: String) -> String {
        var s = replaceAll(in: css, pattern: "@import\\b[^;]*;", with: "")
        s = replaceAll(in: s, pattern: "@import\\b[^;<]*", with: "")
        return s
    }

    /// Extract the quoted/unquoted value from an `name="value"` attribute slice.
    private static func attributeValue(from attribute: String) -> String {
        guard let eq = attribute.firstIndex(of: "=") else { return "" }
        var v = attribute[attribute.index(after: eq)...]
            .trimmingCharacters(in: .whitespaces)
        if let first = v.first, first == "\"" || first == "'" {
            v = String(v.dropFirst())
            if let close = v.firstIndex(of: first) {
                v = String(v[..<close])
            }
        }
        return v.trimmingCharacters(in: .whitespaces)
    }

    /// Extract the target inside a `url(...)` token.
    private static func cssURLTarget(from token: String) -> String {
        guard let open = token.firstIndex(of: "("),
              let close = token.lastIndex(of: ")")
        else { return "" }
        var inner = token[token.index(after: open)..<close]
            .trimmingCharacters(in: .whitespaces)
        if let first = inner.first, first == "\"" || first == "'" {
            inner = inner.trimmingCharacters(in: CharacterSet(charactersIn: "\"'"))
        }
        return inner.trimmingCharacters(in: .whitespaces)
    }

    // MARK: - Regex primitives

    /// Replace every match of `pattern` (case-insensitive, dot-matches-line)
    /// with a fixed `replacement`.
    private static func replaceAll(in input: String, pattern: String, with replacement: String) -> String {
        rewriteMatches(in: input, pattern: pattern) { _ in replacement }
    }

    /// Rewrite every match of `pattern` via `transform`, applied right-to-left
    /// so earlier match ranges stay valid. Returns the input unchanged if the
    /// pattern fails to compile (a sanitizer must never throw into the caller).
    private static func rewriteMatches(
        in input: String,
        pattern: String,
        transform: (String) -> String
    ) -> String {
        guard let regex = try? NSRegularExpression(
            pattern: pattern,
            options: [.caseInsensitive, .dotMatchesLineSeparators]
        ) else {
            return input
        }
        var output = input
        let full = NSRange(input.startIndex..., in: input)
        let matches = regex.matches(in: input, range: full)
        for match in matches.reversed() {
            guard let range = Range(match.range, in: output) else { continue }
            let slice = String(output[range])
            output.replaceSubrange(range, with: transform(slice))
        }
        return output
    }
}
