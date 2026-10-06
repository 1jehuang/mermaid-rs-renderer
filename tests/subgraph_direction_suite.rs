use mermaid_rs_renderer::{LayoutConfig, Theme, compute_layout, parse_mermaid_strict};

#[test]
fn connected_subgraphs_inherit_direction_without_mutating_the_graph() {
    for boundary_edge in ["Outside --> B", "B --> Outside"] {
        let input =
            format!("flowchart LR\n subgraph G\n direction TB\n A-->B\n end\n {boundary_edge}");
        let parsed = parse_mermaid_strict(&input).unwrap();
        let original_direction = parsed.graph.subgraphs[0].direction;
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let inherited = parse_mermaid_strict(&input.replace("direction TB", "")).unwrap();
        let expected = compute_layout(&inherited.graph, &Theme::modern(), &LayoutConfig::default());
        for id in ["A", "B", "Outside"] {
            assert_eq!(layout.nodes[id].x, expected.nodes[id].x);
            assert_eq!(layout.nodes[id].y, expected.nodes[id].y);
        }
        assert_eq!(parsed.graph.subgraphs[0].direction, original_direction);
    }
    let isolated =
        parse_mermaid_strict("flowchart LR\n subgraph G\n direction TB\n A-->B\n end").unwrap();
    let layout = compute_layout(&isolated.graph, &Theme::modern(), &LayoutConfig::default());
    assert!(layout.nodes["B"].y > layout.nodes["A"].y);
}

#[test]
fn links_to_group_itself_preserve_its_internal_direction() {
    let parsed = parse_mermaid_strict(
        "flowchart LR\n subgraph G\n direction TB\n A-->B\n end\n Outside-->G",
    )
    .unwrap();
    let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
    assert!(layout.nodes["B"].y > layout.nodes["A"].y);
}
