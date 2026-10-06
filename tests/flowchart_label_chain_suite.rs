use mermaid_rs_renderer::parse_mermaid_strict;

#[test]
fn pipe_labels_preserve_unicode_semicolons_and_arrow_text_in_chains() {
    let parsed =
        parse_mermaid_strict("flowchart LR\n A -->|日本語 --> #amp; text; more| B -->|next| C")
            .unwrap();
    assert_eq!(parsed.graph.nodes.len(), 3);
    assert_eq!(parsed.graph.edges.len(), 2);
    assert_eq!(
        parsed.graph.edges[0].label.as_deref(),
        Some("日本語 --> & text; more")
    );
    assert_eq!(parsed.graph.edges[1].label.as_deref(), Some("next"));
    assert_eq!(parsed.graph.edges[1].from, "B");
}
