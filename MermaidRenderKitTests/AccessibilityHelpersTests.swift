// AccessibilityHelpersTests.swift
// MermaidRenderKitTests

import XCTest

@testable import MermaidRenderKit

/// Accessibility label + title extraction (PRD F7 / T10).
final class AccessibilityHelpersTests: XCTestCase {

    // MARK: - Type detection

    func testDetectsEachMVPType() {
        XCTAssertEqual(MermaidAccessibility.detectType(from: "flowchart LR\nA-->B"), .flowchart)
        XCTAssertEqual(MermaidAccessibility.detectType(from: "graph TD\nA-->B"), .flowchart)
        XCTAssertEqual(MermaidAccessibility.detectType(from: "sequenceDiagram\nA->>B: hi"), .sequence)
        XCTAssertEqual(MermaidAccessibility.detectType(from: "classDiagram\nClass01 <|-- Class02"), .classDiagram)
        XCTAssertEqual(MermaidAccessibility.detectType(from: "stateDiagram-v2\n[*] --> S1"), .stateDiagram)
        XCTAssertEqual(MermaidAccessibility.detectType(from: "erDiagram\nA ||--o{ B : has"), .erDiagram)
        XCTAssertEqual(MermaidAccessibility.detectType(from: "pie title Pets\n\"Dogs\": 50"), .pie)
        XCTAssertEqual(MermaidAccessibility.detectType(from: "gantt\ntitle Plan"), .gantt)
    }

    func testIgnoresLeadingCommentsAndBlankLines() {
        let src = "\n%% a comment\n\nflowchart LR\nA-->B"
        XCTAssertEqual(MermaidAccessibility.detectType(from: src), .flowchart)
    }

    func testUnknownType() {
        XCTAssertEqual(MermaidAccessibility.detectType(from: "journey\n title x"), .unknown)
        XCTAssertEqual(MermaidAccessibility.detectType(from: ""), .unknown)
    }

    // MARK: - Title extraction

    func testPieTitleExtractedQuotedAndUnquoted() {
        XCTAssertEqual(MermaidAccessibility.extractTitle(from: "pie title Favourite Pets", type: .pie), "Favourite Pets")
        XCTAssertEqual(MermaidAccessibility.extractTitle(from: #"pie title "My Pets""#, type: .pie), "My Pets")
    }

    func testGanttTitleExtractedFromOwnLine() {
        let src = "gantt\n  dateFormat YYYY-MM-DD\n  title Project Plan\n  section A"
        XCTAssertEqual(MermaidAccessibility.extractTitle(from: src, type: .gantt), "Project Plan")
    }

    func testNoTitleReturnsNil() {
        XCTAssertNil(MermaidAccessibility.extractTitle(from: "flowchart LR\nA-->B", type: .flowchart))
        XCTAssertNil(MermaidAccessibility.extractTitle(from: "pie\n\"Dogs\": 1", type: .pie))
    }

    // MARK: - Label composition (F7-AC1 titled, F7-AC4 untitled)

    func testTitledLabelContainsTypeAndTitle() {
        let label = MermaidAccessibility.label(for: "pie title Favourite Pets\n\"Dogs\": 50")
        XCTAssertEqual(label, "Pie chart diagram: Favourite Pets")
    }

    func testUntitledLabelContainsTypeAndTrimmedSource() {
        let label = MermaidAccessibility.label(for: "flowchart LR\nA-->B")
        XCTAssertEqual(label, "Flowchart diagram: flowchart LR\nA-->B")
    }

    func testLongUntitledSourceIsTruncatedWithEllipsis() {
        let long = "flowchart LR\n" + String(repeating: "A-->B\n", count: 400)
        let label = MermaidAccessibility.label(for: long)
        XCTAssertTrue(label.hasPrefix("Flowchart diagram: "))
        XCTAssertTrue(label.hasSuffix("…"))
        // "Flowchart diagram: " (19) + 500 source chars + "…".
        XCTAssertEqual(label.count, 19 + MermaidAccessibility.maxSourceChars + 1)
    }
}
