use mermaid_rs_renderer::{
    ParseError, RenderOptions, parse_mermaid_strict, render_with_options, validator,
};

fn render(input: &str) -> String {
    render_with_options(input, RenderOptions::mermaid_default()).expect("valid diagram")
}

#[test]
fn public_api_accepts_continuations_but_not_missing_sources() {
    let input = "flowchart LR\n A[Source]\n %% comment\n -->|read| B[Viewer]\n --> C[Done]\n";
    let parsed = parse_mermaid_strict(input).unwrap();
    assert_eq!(parsed.graph.nodes.len(), 3);
    assert_eq!(parsed.graph.edges.len(), 2);
    assert_eq!(parsed.graph.edges[0].from, "A");
    assert_eq!(parsed.graph.edges[1].from, "B");
    assert!(render(input).contains("Viewer"));
    for invalid in [
        "flowchart LR\n --> B",
        "flowchart LR\n A;\n --> B",
        "flowchart LR\n subgraph S\n --> B\n end",
        "flowchart LR\n A\n -->",
    ] {
        assert!(
            render_with_options(invalid, RenderOptions::default()).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn inline_comments_do_not_hide_continuation_terminators() {
    for (input, line) in [
        ("flowchart LR\n A --> B; %% comment\n  --> C", 3),
        (
            "flowchart LR\n A\n --> B; %% comment\n %% separate comment\n  --> C",
            5,
        ),
        (
            "flowchart LR\n A[\"Unicode 🎵 %% quoted\"]; %% comment\n  --> C",
            3,
        ),
    ] {
        assert!(
            matches!(validator::validate(input), Err(ParseError::UnexpectedToken { line: actual, col: 3, .. }) if actual == line),
            "{input}"
        );
        assert!(parse_mermaid_strict(input).is_err(), "{input}");
    }
}

#[test]
fn inline_comments_and_quoted_percent_signs_preserve_valid_continuations() {
    for input in [
        "flowchart LR\n A --> B %% comment;\n --> C",
        "flowchart LR\n A\n --> B %% comment;\n --> C",
        "flowchart LR\n A[\"Unicode 🎵 %% quoted;\"] %% comment\n --> B\n --> C",
        "flowchart LR\n A['Unicode 🎵 %% quoted;'] %% comment\n --> B\n --> C",
    ] {
        validator::validate(input).unwrap();
        let parsed = parse_mermaid_strict(input).unwrap();
        let edges: Vec<_> = parsed
            .graph
            .edges
            .iter()
            .map(|edge| (edge.from.as_str(), edge.to.as_str()))
            .collect();
        assert_eq!(edges, vec![("A", "B"), ("B", "C")], "{input}");
    }
}
