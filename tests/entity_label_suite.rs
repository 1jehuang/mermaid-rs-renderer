use mermaid_rs_renderer::{RenderOptions, parse_mermaid_strict, render_with_options};

fn render(input: &str) -> String {
    render_with_options(input, RenderOptions::mermaid_default()).expect("valid diagram")
}

#[test]
fn entities_decode_in_display_labels_without_becoming_svg_markup() {
    let input = r##"flowchart LR
 A["A #amp; B #35; &#x41;"] -->|#quot;read#quot;| B["&lt;script&gt;alert(1)&lt;/script&gt;"]
 subgraph group["#quot;Group#quot;"]
 C["&amp;lt;"]
 end
"##;
    let parsed = parse_mermaid_strict(input).unwrap();
    assert_eq!(parsed.graph.nodes["A"].label, "A & B # A");
    assert_eq!(parsed.graph.nodes["B"].label, "<script>alert(1)</script>");
    assert_eq!(parsed.graph.nodes["C"].label, "&lt;");
    assert_eq!(parsed.graph.edges[0].label.as_deref(), Some("\"read\""));
    assert_eq!(parsed.graph.subgraphs[0].label, "\"Group\"");
    let svg = render(input);
    assert!(!svg.contains("<script>"));
    assert!(svg.contains("&lt;script&gt;"));
}
