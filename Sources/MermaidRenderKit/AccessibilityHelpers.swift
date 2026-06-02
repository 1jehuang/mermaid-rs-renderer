// AccessibilityHelpers.swift
// MermaidRenderKit

import Foundation

/// The MVP diagram types, for accessibility labelling (PRD F7 / T10).
public enum MermaidDiagramType: Equatable, Sendable {
    case flowchart
    case sequence
    case classDiagram
    case stateDiagram
    case erDiagram
    case pie
    case gantt
    /// Detected but outside the MVP allowlist, or undetectable.
    case unknown

    /// Human-readable name used in the VoiceOver label.
    public var displayName: String {
        switch self {
        case .flowchart: return "Flowchart"
        case .sequence: return "Sequence"
        case .classDiagram: return "Class"
        case .stateDiagram: return "State"
        case .erDiagram: return "Entity-Relationship"
        case .pie: return "Pie chart"
        case .gantt: return "Gantt"
        case .unknown: return "Mermaid"
        }
    }
}

/// Pure accessibility helpers for the native diagram view (PRD F7 / T10).
public enum MermaidAccessibility {

    /// Maximum source characters included in a label when no title is found.
    static let maxSourceChars = 500

    /// Detect the diagram type from the first significant line of `source`.
    public static func detectType(from source: String) -> MermaidDiagramType {
        guard let header = firstSignificantLine(source)?.lowercased() else {
            return .unknown
        }
        // Order matters: check the more specific keywords first.
        if header.hasPrefix("sequencediagram") { return .sequence }
        if header.hasPrefix("classdiagram") { return .classDiagram }
        if header.hasPrefix("statediagram") { return .stateDiagram }
        if header.hasPrefix("erdiagram") { return .erDiagram }
        if header.hasPrefix("gantt") { return .gantt }
        if header.hasPrefix("pie") { return .pie }
        if header.hasPrefix("flowchart") || header.hasPrefix("graph") { return .flowchart }
        return .unknown
    }

    /// Extract a human title from the source where the diagram type carries one
    /// (`pie title …`, `gantt`'s `title …`). Returns nil otherwise.
    public static func extractTitle(from source: String, type: MermaidDiagramType) -> String? {
        switch type {
        case .pie:
            // `pie title My Chart` / `pie title "My Chart"` (title on the header).
            if let header = firstSignificantLine(source) {
                return titleAfterKeyword(in: header)
            }
            return nil
        case .gantt:
            // gantt declares `title <text>` on its own line.
            for line in significantLines(source) {
                let trimmed = line.trimmingCharacters(in: .whitespaces)
                if trimmed.lowercased().hasPrefix("title ") {
                    return cleanTitle(String(trimmed.dropFirst("title ".count)))
                }
            }
            return nil
        default:
            return nil
        }
    }

    /// Compose the VoiceOver label (F7-AC1): "<Type> diagram: <title-or-source>".
    public static func label(for source: String, type: MermaidDiagramType? = nil) -> String {
        let resolved = type ?? detectType(from: source)
        let name = resolved.displayName
        if let title = extractTitle(from: source, type: resolved), !title.isEmpty {
            return "\(name) diagram: \(title)"
        }
        let trimmed = source.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.count > maxSourceChars {
            return "\(name) diagram: \(String(trimmed.prefix(maxSourceChars)))…"
        }
        return "\(name) diagram: \(trimmed)"
    }

    // MARK: - Helpers

    private static func significantLines(_ source: String) -> [Substring] {
        source.split(separator: "\n", omittingEmptySubsequences: false)
            .map { $0.trimmingCharacters(in: .whitespaces)[...] }
            .filter { !$0.isEmpty && !$0.hasPrefix("%%") } // %% = mermaid comment
    }

    private static func firstSignificantLine(_ source: String) -> String? {
        for line in source.split(separator: "\n", omittingEmptySubsequences: false) {
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            if !trimmed.isEmpty, !trimmed.hasPrefix("%%") {
                return trimmed
            }
        }
        return nil
    }

    /// Return the text after a `title` keyword in a line, or nil if absent.
    private static func titleAfterKeyword(in line: String) -> String? {
        let lower = line.lowercased()
        guard let range = lower.range(of: "title ") else { return nil }
        let after = String(line[range.upperBound...])
        let cleaned = cleanTitle(after)
        return cleaned.isEmpty ? nil : cleaned
    }

    /// Trim whitespace and surrounding quotes from a title string.
    private static func cleanTitle(_ raw: String) -> String {
        var t = raw.trimmingCharacters(in: .whitespaces)
        if t.count >= 2, let first = t.first, first == "\"" || first == "'", t.hasSuffix(String(first)) {
            t = String(t.dropFirst().dropLast())
        }
        return t.trimmingCharacters(in: .whitespaces)
    }
}
