use std::path::Path;

use mermaid_rs_renderer::{LayoutConfig, Theme, parse_mermaid, render_svg};

fn assert_valid_svg(svg: &str, fixture: &str) {
    assert!(svg.contains("<svg"), "{fixture}: missing <svg tag");
    assert!(svg.contains("</svg>"), "{fixture}: missing </svg tag");
}

fn render_fixture(path: &Path) -> String {
    let input = std::fs::read_to_string(path).expect("fixture read failed");
    let parsed = parse_mermaid(&input).expect("parse failed");
    let theme = Theme::modern();
    let layout_config = LayoutConfig::default();
    let layout = mermaid_rs_renderer::compute_layout(&parsed.graph, &theme, &layout_config);
    render_svg(&layout, &theme, &layout_config)
}

#[test]
fn render_all_fixtures() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let mut fixtures: Vec<String> = Vec::new();

    // Keep this list explicit so new diagram types must be added intentionally.
    let candidates = [
        "architecture/basic.mmd",
        "block/basic.mmd",
        "c4/basic.mmd",
        "class/basic.mmd",
        "class/multiplicity.mmd",
        "er/basic.mmd",
        "flowchart/basic.mmd",
        "flowchart/complex.mmd",
        "flowchart/edges.mmd",
        "flowchart/dense.mmd",
        "flowchart/ports.mmd",
        "flowchart/styles.mmd",
        "flowchart/subgraph.mmd",
        "flowchart/subgraph_direction.mmd",
        "flowchart/subgraph_empty.mmd",
        "flowchart/cycles.mmd",
        "gantt/basic.mmd",
        "gitgraph/basic.mmd",
        "journey/basic.mmd",
        "kanban/basic.mmd",
        "mindmap/basic.mmd",
        "packet/basic.mmd",
        "pie/basic.mmd",
        "quadrant/basic.mmd",
        "radar/basic.mmd",
        "requirement/basic.mmd",
        "sankey/basic.mmd",
        "sequence/basic.mmd",
        "sequence/frames.mmd",
        "state/basic.mmd",
        "state/note.mmd",
        "timeline/basic.mmd",
        "treemap/basic.mmd",
        "xychart/basic.mmd",
        "zenuml/basic.mmd",
    ];

    for rel in candidates {
        fixtures.push(rel.to_string());
    }

    for rel in fixtures {
        let path = root.join(&rel);
        assert!(path.exists(), "fixture missing: {}", rel);
        let svg = render_fixture(&path);
        assert_valid_svg(&svg, &rel);
    }
}
