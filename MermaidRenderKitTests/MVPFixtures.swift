// MVPFixtures.swift
// MermaidRenderKitTests

import Foundation

/// Canonical source for each of the 7 MVP diagram types (PRD D4). Shared by the
/// PNG/SVG render matrix (Task 19) and the perceptual snapshot suite (Task 17).
enum MVPFixtures {

    struct Case {
        let name: String
        let source: String
    }

    static let all: [Case] = [
        Case(name: "flowchart", source: """
        flowchart LR
        A[Start] --> B{Choice}
        B --> C[Yes]
        B --> D[No]
        """),
        Case(name: "sequence", source: """
        sequenceDiagram
        Alice->>Bob: Hello Bob
        Bob-->>Alice: Hi Alice
        """),
        Case(name: "class", source: """
        classDiagram
        Animal <|-- Dog
        Animal : +int age
        Dog : +bark()
        """),
        Case(name: "state", source: """
        stateDiagram-v2
        [*] --> Idle
        Idle --> Running
        Running --> [*]
        """),
        Case(name: "er", source: """
        erDiagram
        CUSTOMER ||--o{ ORDER : places
        ORDER ||--|{ LINE_ITEM : contains
        """),
        Case(name: "pie", source: """
        pie title Pets
        "Dogs" : 50
        "Cats" : 30
        """),
        Case(name: "gantt", source: """
        gantt
        title Plan
        dateFormat YYYY-MM-DD
        section Phase
        Task1 : a1, 2024-01-01, 30d
        """),
    ]
}
