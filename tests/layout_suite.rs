use std::path::Path;

use mermaid_rs_renderer::layout::{DiagramData, validate_layout_invariants};
use mermaid_rs_renderer::{
    Direction, Layout, LayoutConfig, Theme, compute_layout, parse_mermaid, render_svg,
};

fn assert_valid_svg(svg: &str, fixture: &str) {
    assert!(svg.contains("<svg"), "{fixture}: missing <svg tag");
    assert!(svg.contains("</svg>"), "{fixture}: missing </svg tag");
    assert!(!svg.contains("NaN"), "{fixture}: svg contains NaN");
    assert!(!svg.contains("inf"), "{fixture}: svg contains inf");
}

fn assert_finite(value: f32, fixture: &str, label: &str) {
    assert!(value.is_finite(), "{fixture}: {label} is not finite");
}

fn segment_intersects_rect(a: (f32, f32), b: (f32, f32), rect: (f32, f32, f32, f32)) -> bool {
    let (rx, ry, rw, rh) = rect;
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let p = [-dx, dx, -dy, dy];
    let q = [a.0 - rx, rx + rw - a.0, a.1 - ry, ry + rh - a.1];
    let mut u1 = 0.0f32;
    let mut u2 = 1.0f32;

    for (pi, qi) in p.into_iter().zip(q) {
        if pi.abs() <= f32::EPSILON {
            if qi < 0.0 {
                return false;
            }
            continue;
        }
        let t = qi / pi;
        if pi < 0.0 {
            if t > u2 {
                return false;
            }
            if t > u1 {
                u1 = t;
            }
        } else {
            if t < u1 {
                return false;
            }
            if t < u2 {
                u2 = t;
            }
        }
    }

    true
}

fn rect_overlap_area(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> f32 {
    let overlap_x = (a.0 + a.2).min(b.0 + b.2) - a.0.max(b.0);
    let overlap_y = (a.1 + a.3).min(b.1 + b.3) - a.1.max(b.1);
    if overlap_x > 0.0 && overlap_y > 0.0 {
        overlap_x * overlap_y
    } else {
        0.0
    }
}

fn assert_layout_is_well_formed(layout: &Layout, fixture: &str) {
    assert_finite(layout.width, fixture, "layout.width");
    assert_finite(layout.height, fixture, "layout.height");
    assert!(
        layout.width > 0.0,
        "{fixture}: layout.width must be positive"
    );
    assert!(
        layout.height > 0.0,
        "{fixture}: layout.height must be positive"
    );

    for (id, node) in &layout.nodes {
        assert_finite(node.x, fixture, &format!("node {id} x"));
        assert_finite(node.y, fixture, &format!("node {id} y"));
        assert_finite(node.width, fixture, &format!("node {id} width"));
        assert_finite(node.height, fixture, &format!("node {id} height"));
        assert!(
            node.width >= 0.0,
            "{fixture}: node {id} width must be non-negative"
        );
        assert!(
            node.height >= 0.0,
            "{fixture}: node {id} height must be non-negative"
        );
        assert!(
            node.x >= -0.1,
            "{fixture}: node {id} x should not be negative"
        );
        assert!(
            node.y >= -0.1,
            "{fixture}: node {id} y should not be negative"
        );
        assert!(
            node.x + node.width <= layout.width + 0.1,
            "{fixture}: node {id} exceeds layout width"
        );
        assert!(
            node.y + node.height <= layout.height + 0.1,
            "{fixture}: node {id} exceeds layout height"
        );
        assert_finite(node.label.width, fixture, &format!("node {id} label width"));
        assert_finite(
            node.label.height,
            fixture,
            &format!("node {id} label height"),
        );
    }

    for sub in &layout.subgraphs {
        assert_finite(sub.x, fixture, &format!("subgraph {} x", sub.label));
        assert_finite(sub.y, fixture, &format!("subgraph {} y", sub.label));
        assert_finite(sub.width, fixture, &format!("subgraph {} width", sub.label));
        assert_finite(
            sub.height,
            fixture,
            &format!("subgraph {} height", sub.label),
        );
        assert!(
            sub.width >= 0.0,
            "{fixture}: subgraph {} width must be non-negative",
            sub.label
        );
        assert!(
            sub.height >= 0.0,
            "{fixture}: subgraph {} height must be non-negative",
            sub.label
        );
    }

    for edge in &layout.edges {
        for (idx, point) in edge.points.iter().enumerate() {
            assert_finite(
                point.0,
                fixture,
                &format!("edge {}->{} point {idx} x", edge.from, edge.to),
            );
            assert_finite(
                point.1,
                fixture,
                &format!("edge {}->{} point {idx} y", edge.from, edge.to),
            );
        }
        if let Some((x, y)) = edge.label_anchor {
            assert_finite(
                x,
                fixture,
                &format!("edge {}->{} label anchor x", edge.from, edge.to),
            );
            assert_finite(
                y,
                fixture,
                &format!("edge {}->{} label anchor y", edge.from, edge.to),
            );
        }
        if let Some((x, y)) = edge.start_label_anchor {
            assert_finite(
                x,
                fixture,
                &format!("edge {}->{} start label anchor x", edge.from, edge.to),
            );
            assert_finite(
                y,
                fixture,
                &format!("edge {}->{} start label anchor y", edge.from, edge.to),
            );
        }
        if let Some((x, y)) = edge.end_label_anchor {
            assert_finite(
                x,
                fixture,
                &format!("edge {}->{} end label anchor x", edge.from, edge.to),
            );
            assert_finite(
                y,
                fixture,
                &format!("edge {}->{} end label anchor y", edge.from, edge.to),
            );
        }
    }

    if let DiagramData::Graph { state_notes } = &layout.diagram {
        for (idx, note) in state_notes.iter().enumerate() {
            assert_finite(note.x, fixture, &format!("state note {idx} x"));
            assert_finite(note.y, fixture, &format!("state note {idx} y"));
            assert_finite(note.width, fixture, &format!("state note {idx} width"));
            assert_finite(note.height, fixture, &format!("state note {idx} height"));
        }
    }
}

fn assert_flowchart_visual_invariants(layout: &Layout, fixture: &str) {
    if !fixture.starts_with("flowchart/") {
        return;
    }

    for (idx, left) in layout.subgraphs.iter().enumerate() {
        let left_nodes: std::collections::HashSet<&str> =
            left.nodes.iter().map(|node| node.as_str()).collect();
        for right in layout.subgraphs.iter().skip(idx + 1) {
            let shares_nodes = right
                .nodes
                .iter()
                .any(|node| left_nodes.contains(node.as_str()));
            if shares_nodes {
                continue;
            }
            let overlaps_x = left.x < right.x + right.width && right.x < left.x + left.width;
            let overlaps_y = left.y < right.y + right.height && right.y < left.y + left.height;
            assert!(
                !(overlaps_x && overlaps_y),
                "{fixture}: subgraphs {} and {} overlap",
                left.label,
                right.label
            );
        }
    }

    for edge in &layout.edges {
        let (Some(label), Some(anchor)) = (&edge.label, edge.label_anchor) else {
            continue;
        };
        let label_rect = (
            anchor.0 - label.width / 2.0,
            anchor.1 - label.height / 2.0,
            label.width,
            label.height,
        );
        let intersects = edge
            .points
            .windows(2)
            .any(|segment| segment_intersects_rect(segment[0], segment[1], label_rect));
        assert!(
            !intersects,
            "{fixture}: edge {}->{} route overlaps its own label box",
            edge.from, edge.to
        );
    }

    if fixture == "flowchart/bidirectional_labels.mmd" {
        let labels: Vec<_> = layout
            .edges
            .iter()
            .filter_map(|edge| {
                let label = edge.label.as_ref()?;
                let anchor = edge.label_anchor?;
                Some((
                    edge.from.as_str(),
                    edge.to.as_str(),
                    (
                        anchor.0 - label.width / 2.0,
                        anchor.1 - label.height / 2.0,
                        label.width,
                        label.height,
                    ),
                ))
            })
            .collect();
        for (idx, (from, to, rect)) in labels.iter().enumerate() {
            for (other_from, other_to, other_rect) in labels.iter().skip(idx + 1) {
                let overlap = rect_overlap_area(*rect, *other_rect);
                assert!(
                    overlap <= 1.0,
                    "{fixture}: edge labels {from}->{to} and {other_from}->{other_to} overlap by {overlap:.2}px²"
                );
            }
        }
    }
}

fn assert_sequence_label_clear_of_lifelines(layout: &Layout, fixture: &str) {
    let DiagramData::Sequence(seq) = &layout.diagram else {
        return;
    };

    for edge in &layout.edges {
        let (Some(label), Some(anchor)) = (&edge.label, edge.label_anchor) else {
            continue;
        };
        let label_rect = (
            anchor.0 - label.width / 2.0 - 4.0,
            anchor.1 - label.height / 2.0 - 2.0,
            label.width + 8.0,
            label.height + 4.0,
        );
        for lifeline in &seq.lifelines {
            if lifeline.id == edge.from || lifeline.id == edge.to {
                continue;
            }
            let line_rect = (
                lifeline.x - 1.5,
                lifeline.y1,
                3.0,
                lifeline.y2 - lifeline.y1,
            );
            let overlaps_x = label_rect.0 < line_rect.0 + line_rect.2
                && line_rect.0 < label_rect.0 + label_rect.2;
            let overlaps_y = label_rect.1 < line_rect.1 + line_rect.3
                && line_rect.1 < label_rect.1 + label_rect.3;
            assert!(
                !(overlaps_x && overlaps_y),
                "{fixture}: edge label for {}->{} overlaps lifeline {}",
                edge.from,
                edge.to,
                lifeline.id
            );
        }
    }
}

fn render_fixture(path: &Path) -> (Layout, String) {
    let input = std::fs::read_to_string(path).expect("fixture read failed");
    let parsed = parse_mermaid(&input).expect("parse failed");
    let theme = Theme::modern();
    let layout_config = LayoutConfig::default();
    let layout = compute_layout(&parsed.graph, &theme, &layout_config);
    let svg = render_svg(&layout, &theme, &layout_config);
    (layout, svg)
}

fn parse_viewbox(svg: &str) -> Option<(f32, f32, f32, f32)> {
    let marker = "viewBox=\"";
    let start = svg.find(marker)? + marker.len();
    let end = svg[start..].find('"')? + start;
    let parts: Vec<f32> = svg[start..end]
        .split(|ch: char| ch.is_ascii_whitespace() || ch == ',')
        .filter(|part| !part.is_empty())
        .map(|part| part.parse::<f32>().ok())
        .collect::<Option<Vec<_>>>()?;
    if parts.len() == 4 {
        Some((parts[0], parts[1], parts[2], parts[3]))
    } else {
        None
    }
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
        "flowchart/bidirectional_labels.mmd",
        "gantt/basic.mmd",
        "gitgraph/basic.mmd",
        "journey/basic.mmd",
        "kanban/basic.mmd",
        "mindmap/basic.mmd",
        "mindmap/tidy_tree.mmd",
        "mindmap/lr_tree.mmd",
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
        let (layout, svg) = render_fixture(&path);
        assert_layout_is_well_formed(&layout, &rel);
        if let Err(errors) = validate_layout_invariants(&layout) {
            panic!(
                "{rel}: layout invariant violations:\n{}",
                errors
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
        assert_flowchart_visual_invariants(&layout, &rel);
        assert_sequence_label_clear_of_lifelines(&layout, &rel);
        assert_valid_svg(&svg, &rel);
    }
}

#[test]
fn timeline_event_descriptions_wrap_inside_cards() {
    let input = r#"timeline
    title Timeline of Industrial Revolution
    Industry 1.0 : Machinery, Water power, Steam <br>power
    Industry 2.0 : Electricity, Internal combustion engine, Mass production
"#;
    let parsed = parse_mermaid(input).unwrap();
    let theme = Theme::modern();
    let config = LayoutConfig::default();
    let layout = compute_layout(&parsed.graph, &theme, &config);
    let DiagramData::Timeline(timeline) = &layout.diagram else {
        panic!("expected timeline layout");
    };

    let wrapped = timeline
        .events
        .iter()
        .find(|event| event.time.lines.join(" ") == "Industry 2.0")
        .expect("expected Industry 2.0 event");
    assert!(
        wrapped.events[0].lines.len() > 1,
        "expected long description to wrap: {:?}",
        wrapped.events[0].lines
    );
    assert!(
        wrapped.height > 80.0,
        "event card height should expand for wrapped descriptions"
    );

    let explicit_break = timeline
        .events
        .iter()
        .find(|event| event.time.lines.join(" ") == "Industry 1.0")
        .expect("expected Industry 1.0 event");
    assert!(
        explicit_break.events[0]
            .lines
            .iter()
            .any(|line| line == "power"),
        "expected explicit <br> to survive as a separate rendered line"
    );

    let svg = render_svg(&layout, &theme, &config);
    assert!(!svg.contains(">Electricity, Internal combustion engine, Mass production</text>"));
    assert!(svg.contains(">power</tspan>"));
}

#[test]
fn timeline_direction_headers_and_config_default() {
    let input = "timeline TD\n  title History\n  2020 : Launch\n  2021 : Growth\n  2022 : Scale";
    let parsed = parse_mermaid(input).unwrap();
    let theme = Theme::modern();
    let config = LayoutConfig::default();
    let layout = compute_layout(&parsed.graph, &theme, &config);
    let DiagramData::Timeline(timeline) = &layout.diagram else {
        panic!("expected timeline layout");
    };
    assert_eq!(timeline.direction, Direction::TopDown);
    assert_eq!(timeline.line_start_x, timeline.line_end_x);
    assert!(timeline.line_end_y > timeline.line_start_y);
    assert!(timeline.height > timeline.width);
    assert!(timeline.events.windows(2).all(|pair| pair[0].y < pair[1].y));

    let input = "timeline\n  2020 : Launch\n  2021 : Growth\n  2022 : Scale";
    let parsed = parse_mermaid(input).unwrap();
    assert_eq!(parsed.graph.timeline.direction, None);
    let mut config = LayoutConfig::default();
    config.timeline.direction = "TD".to_string();
    let layout = compute_layout(&parsed.graph, &theme, &config);
    let DiagramData::Timeline(timeline) = &layout.diagram else {
        panic!("expected timeline layout");
    };
    assert_eq!(timeline.direction, Direction::TopDown);
    assert!(timeline.height > timeline.width);

    let input = "timeline LR\n  2020 : Launch\n  2021 : Growth\n  2022 : Scale";
    let parsed = parse_mermaid(input).unwrap();
    let layout = compute_layout(&parsed.graph, &theme, &config);
    let DiagramData::Timeline(timeline) = &layout.diagram else {
        panic!("expected timeline layout");
    };
    assert_eq!(timeline.direction, Direction::LeftRight);
    assert_eq!(timeline.line_start_y, timeline.line_end_y);
    assert!(timeline.line_end_x > timeline.line_start_x);
    assert!(timeline.width > timeline.height);
}

#[test]
fn timeline_vertical_cards_adapt_width_and_height() {
    let input = r#"timeline TD
  Short : A
  Long : This vertical card should expand beyond the minimum width while remaining capped by the maximum width even with multiple words and wrap if needed
"#;
    let parsed = parse_mermaid(input).unwrap();
    let theme = Theme::modern();
    let config = LayoutConfig::default();
    let layout = compute_layout(&parsed.graph, &theme, &config);
    let DiagramData::Timeline(timeline) = &layout.diagram else {
        panic!("expected timeline layout");
    };

    let short = &timeline.events[0];
    let long = &timeline.events[1];
    assert_eq!(short.width, 120.0);
    assert!(
        short.height < 80.0,
        "short card height was {}",
        short.height
    );
    assert!(long.width > 120.0, "long card width was {}", long.width);
    assert!(long.width <= 360.0, "long card width was {}", long.width);
    assert!(long.height > 80.0, "long card height was {}", long.height);

    let svg = render_svg(&layout, &theme, &config);
    assert!(svg.contains("text-anchor=\"start\""));
}

#[test]
fn pie_outside_labels_do_not_intrude_into_right_legend() {
    let input = r#"pie
"Dogs" : 386
"Cats" : 85.9
"Rats" : 15
"#;
    let parsed = parse_mermaid(input).unwrap();
    let theme = Theme::modern();
    let config = LayoutConfig::default();
    let layout = compute_layout(&parsed.graph, &theme, &config);
    let DiagramData::Pie(pie) = &layout.diagram else {
        panic!("expected pie layout");
    };

    let legend_left = pie
        .legend
        .iter()
        .map(|item| item.x)
        .fold(f32::INFINITY, f32::min);
    let right_outside_label_right = pie
        .slices
        .iter()
        .filter_map(|slice| {
            let span = (slice.end_angle - slice.start_angle).abs();
            let mid_angle = (slice.start_angle + slice.end_angle) / 2.0;
            if span < 0.4 && mid_angle.cos() >= 0.0 {
                Some(pie.center.0 + pie.radius + slice.label.width)
            } else {
                None
            }
        })
        .fold(0.0, f32::max);

    assert!(
        legend_left > right_outside_label_right,
        "right-side outside pie labels should have reserved space before the legend: legend_left={legend_left}, label_right={right_outside_label_right}"
    );
}

#[test]
fn bidirectional_flowchart_labels_do_not_overlap() {
    let input = r#"flowchart TD
    dep1 -->|subs| link1
    link1 -->|sub| sub1
    sub1 -->|deps| link1
    link1 -->|dep| dep1

    link1 -->|nextSub| link2
    link2 -->|prevSub| link1

    link2 -->|sub| sub2
    sub2 -->|deps| link2
"#;
    let parsed = parse_mermaid(input).unwrap();
    let theme = Theme::modern();
    let config = LayoutConfig::default();
    let layout = compute_layout(&parsed.graph, &theme, &config);
    assert_layout_is_well_formed(&layout, "flowchart/issue63-inline.mmd");

    let labels: Vec<_> = layout
        .edges
        .iter()
        .filter_map(|edge| {
            let label = edge.label.as_ref()?;
            let anchor = edge.label_anchor?;
            Some((
                edge.from.as_str(),
                edge.to.as_str(),
                (
                    anchor.0 - label.width / 2.0,
                    anchor.1 - label.height / 2.0,
                    label.width,
                    label.height,
                ),
            ))
        })
        .collect();
    assert_eq!(labels.len(), 8, "all edge labels should be placed");
    for (idx, (from, to, rect)) in labels.iter().enumerate() {
        for (other_from, other_to, other_rect) in labels.iter().skip(idx + 1) {
            let overlap = rect_overlap_area(*rect, *other_rect);
            assert!(
                overlap <= 1.0,
                "edge labels {from}->{to} and {other_from}->{other_to} overlap by {overlap:.2}px²"
            );
        }
    }
}

#[test]
fn parallel_long_flowchart_labels_expand_bounds_and_do_not_overlap() {
    let input = r#"flowchart LR
  A[Short] -->|this is a very long parallel edge label number one| B[Other]
  A -->|this is a very long parallel edge label number two| B
  A -->|this is a very long parallel edge label number three| B
"#;
    let parsed = parse_mermaid(input).unwrap();
    let theme = Theme::modern();
    let config = LayoutConfig::default();
    let layout = compute_layout(&parsed.graph, &theme, &config);

    let labels: Vec<_> = layout
        .edges
        .iter()
        .filter_map(|edge| {
            let label = edge.label.as_ref()?;
            let anchor = edge.label_anchor?;
            Some((
                anchor.0 - label.width / 2.0,
                anchor.1 - label.height / 2.0,
                label.width,
                label.height,
            ))
        })
        .collect();
    assert_eq!(labels.len(), 3, "all parallel labels should be placed");
    for rect in &labels {
        assert!(rect.0 >= -0.1, "label extends left of layout: {rect:?}");
        assert!(rect.1 >= -0.1, "label extends above layout: {rect:?}");
        assert!(
            rect.0 + rect.2 <= layout.width + 0.1,
            "label exceeds layout width: {rect:?} width={}",
            layout.width
        );
        assert!(
            rect.1 + rect.3 <= layout.height + 0.1,
            "label exceeds layout height: {rect:?} height={}",
            layout.height
        );
    }
    for (idx, rect) in labels.iter().enumerate() {
        for other in labels.iter().skip(idx + 1) {
            let overlap = rect_overlap_area(*rect, *other);
            assert!(overlap <= 1.0, "parallel labels overlap by {overlap:.2}px²");
        }
    }
}

#[test]
fn long_edge_label_flowchart_keeps_top_level_subgraphs_separate() {
    let input = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("benches")
            .join("fixtures")
            .join("flowchart_long_edge_labels.mmd"),
    )
    .unwrap();
    let parsed = parse_mermaid(&input).unwrap();
    let theme = Theme::modern();
    let config = LayoutConfig::default();
    let layout = compute_layout(&parsed.graph, &theme, &config);

    assert_flowchart_visual_invariants(&layout, "flowchart/long_edge_labels.mmd");
}

#[test]
fn flowchart_label_collision_fixture_routes_around_non_endpoint_nodes() {
    let input = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("benches")
            .join("fixtures")
            .join("flowchart_label_collision.mmd"),
    )
    .unwrap();
    let parsed = parse_mermaid(&input).unwrap();
    let theme = Theme::modern();
    let config = LayoutConfig::default();
    let layout = compute_layout(&parsed.graph, &theme, &config);

    for edge in &layout.edges {
        for segment in edge.points.windows(2) {
            for node in layout.nodes.values() {
                if node.id == edge.from || node.id == edge.to || node.hidden {
                    continue;
                }
                let rect = (node.x, node.y, node.width, node.height);
                assert!(
                    !segment_intersects_rect(segment[0], segment[1], rect),
                    "edge {}->{} crosses non-endpoint node {}",
                    edge.from,
                    edge.to,
                    node.id
                );
            }
        }
    }
}

#[test]
fn td_loopback_uses_outer_left_ports_and_orthogonal_lane() {
    let input = r#"flowchart TD
  Start([Start]) --> Input[/Read Input/]
  Input --> Check{Valid?}
  Check -->|Yes| Process[Process Data]
  Check -->|No| Error[Show Error]
  Error --> Input
  Process --> Save[(Save to DB)]
  Save --> Done([End])
"#;
    let parsed = parse_mermaid(input).unwrap();
    let theme = Theme::modern();
    let config = LayoutConfig::default();
    let layout = compute_layout(&parsed.graph, &theme, &config);
    assert_layout_is_well_formed(&layout, "flowchart/td-loopback-ports.mmd");

    let error = layout.nodes.get("Error").expect("Error node");
    let input_node = layout.nodes.get("Input").expect("Input node");
    let edge = layout
        .edges
        .iter()
        .find(|edge| edge.from == "Error" && edge.to == "Input")
        .expect("Error->Input loopback edge");

    assert!(
        edge.points.len() >= 4,
        "loopback should use an outer lane with bends, got {:?}",
        edge.points
    );
    assert!(
        edge.points.windows(2).all(|segment| {
            (segment[1].0 - segment[0].0).abs() <= 1e-3
                || (segment[1].1 - segment[0].1).abs() <= 1e-3
        }),
        "loopback should stay orthogonal, got {:?}",
        edge.points
    );

    let first = edge.points[0];
    let second = edge.points[1];
    let penultimate = edge.points[edge.points.len() - 2];
    let last = edge.points[edge.points.len() - 1];

    assert!(
        (first.0 - error.x).abs() <= 1.0,
        "loopback should leave Error from the diagram's outer-left side, got {:?} for Error {:?}",
        edge.points,
        error
    );
    assert!(
        second.0 < first.0 - 1.0 && (second.1 - first.1).abs() <= 1.0,
        "loopback should move outward immediately instead of crossing the source node, got {:?}",
        edge.points
    );
    assert!(
        second.0 < error.x && penultimate.0 < input_node.x,
        "loopback lane should run outside the left edge of the involved nodes, got {:?}",
        edge.points
    );
    assert!(
        last.0 < input_node.x + input_node.width * 0.35
            && penultimate.0 < last.0
            && (penultimate.1 - last.1).abs() <= 1.0,
        "loopback should re-enter Input from its left-side port, got {:?}",
        edge.points
    );
}

#[test]
fn sequence_nested_alt_wide_section_labels_do_not_panic() {
    let fixture = "sequence/nested_alt.mmd";
    let input = std::fs::read_to_string(Path::new("tests/fixtures").join(fixture)).unwrap();
    let parsed = parse_mermaid(&input).unwrap();
    let theme = Theme::mermaid_default();
    let config = LayoutConfig::default();
    let layout = compute_layout(&parsed.graph, &theme, &config);
    let svg = render_svg(&layout, &theme, &config);
    assert_valid_svg(&svg, fixture);
}

#[test]
fn sequence_basic_uses_mermaid_like_actor_geometry_and_framing() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("sequence")
        .join("basic.mmd");
    let (layout, svg) = render_fixture(&root);

    let alice = layout.nodes.get("Alice").expect("Alice node");
    let bob = layout.nodes.get("Bob").expect("Bob node");

    assert!(
        (alice.width - 150.0).abs() < 0.01,
        "Alice width={}",
        alice.width
    );
    assert!((bob.width - 150.0).abs() < 0.01, "Bob width={}", bob.width);
    assert!(
        (alice.height - 65.0).abs() < 0.01,
        "Alice height={}",
        alice.height
    );
    assert!(
        (bob.height - 65.0).abs() < 0.01,
        "Bob height={}",
        bob.height
    );
    let alice_center = alice.x + alice.width / 2.0;
    let bob_center = bob.x + bob.width / 2.0;
    assert!(
        (alice_center - 75.0).abs() < 0.01,
        "Alice center={alice_center}"
    );
    assert!((bob_center - 275.0).abs() < 0.01, "Bob center={bob_center}");
    assert!(
        (bob_center - alice_center - 200.0).abs() < 0.01,
        "lane pitch={}",
        bob_center - alice_center
    );

    let viewbox = parse_viewbox(&svg).expect("sequence viewBox");
    assert!((viewbox.0 + 50.0).abs() < 0.01, "viewBox x={}", viewbox.0);
    assert!((viewbox.1 + 10.0).abs() < 0.01, "viewBox y={}", viewbox.1);
    assert!(
        (viewbox.2 - 450.0).abs() < 0.01,
        "viewBox width={}",
        viewbox.2
    );
    assert!(
        (viewbox.3 - 265.0).abs() < 8.0,
        "viewBox height={}",
        viewbox.3
    );
}

#[test]
fn sequence_frames_keeps_mermaid_like_lane_pitch() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("sequence")
        .join("frames.mmd");
    let (layout, _svg) = render_fixture(&root);

    let client = layout.nodes.get("Client").expect("Client node");
    let api = layout.nodes.get("API").expect("API node");
    let db = layout.nodes.get("DB").expect("DB node");
    let centers = [
        client.x + client.width / 2.0,
        api.x + api.width / 2.0,
        db.x + db.width / 2.0,
    ];
    assert!(
        (centers[1] - centers[0] - 200.0).abs() < 0.01,
        "first pitch={}",
        centers[1] - centers[0]
    );
    assert!(
        (centers[2] - centers[1] - 200.0).abs() < 0.01,
        "second pitch={}",
        centers[2] - centers[1]
    );
    assert!(
        (layout.width - 550.0).abs() < 0.01,
        "layout width={}",
        layout.width
    );
}

#[test]
fn sequence_alt_frame_geometry_matches_mermaid() {
    use mermaid_rs_renderer::layout::DiagramData;

    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("sequence")
        .join("frames.mmd");
    let (layout, _svg) = render_fixture(&root);

    let DiagramData::Sequence(seq) = &layout.diagram else {
        panic!("expected sequence diagram data");
    };

    assert!(!seq.frames.is_empty(), "should have at least one frame");
    let frame = &seq.frames[0];

    let client = layout.nodes.get("Client").expect("Client node");
    let api = layout.nodes.get("API").expect("API node");
    let client_center = client.x + client.width / 2.0;
    let api_center = api.x + api.width / 2.0;

    assert!(
        frame.x < client_center,
        "frame x ({}) should be left of Client center ({})",
        frame.x,
        client_center
    );
    assert!(
        frame.x + frame.width > api_center,
        "frame right edge ({}) should be right of API center ({})",
        frame.x + frame.width,
        api_center
    );

    assert!(
        (frame.x - 64.0).abs() < 5.0,
        "frame x should be ~64 (got {})",
        frame.x
    );
    assert!(
        (frame.width - 226.0).abs() < 12.0,
        "frame width should be ~226 (got {})",
        frame.width
    );

    let (lbx, lby, lbw, lbh) = frame.label_box;
    assert!(
        (lbx - frame.x).abs() < 0.01,
        "label box x should match frame x"
    );
    assert!(
        (lby - frame.y).abs() < 0.01,
        "label box y should match frame y"
    );
    assert!(
        lbw > 30.0 && lbw < 80.0,
        "label box width should be reasonable (got {})",
        lbw
    );
    assert!(
        lbh > 10.0 && lbh < 30.0,
        "label box height should be reasonable (got {})",
        lbh
    );

    assert!(
        !frame.dividers.is_empty(),
        "alt frame should have at least one divider"
    );
    let div_y = frame.dividers[0];
    assert!(
        div_y > frame.y && div_y < frame.y + frame.height,
        "divider y ({}) should be inside frame ({} to {})",
        div_y,
        frame.y,
        frame.y + frame.height
    );
}

/// Total straight-line length of all relationship arrows in a C4 layout.
fn c4_total_rel_length(layout: &Layout) -> f32 {
    layout
        .edges
        .iter()
        .map(|e| {
            e.points
                .windows(2)
                .map(|w| {
                    let (ax, ay) = w[0];
                    let (bx, by) = w[1];
                    ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt()
                })
                .sum::<f32>()
        })
        .sum()
}

/// Externals are declared before the boundary (so row packing strands them in
/// a top row far from the container they all connect to) and several of them
/// fan into the same `api` container — the situation the force pass targets.
const C4_EXTERNALS_AROUND_BOUNDARY: &str = r#"C4Container
    title Force layout test

    Person(dev, "Developer", "Builds things")
    System_Ext(auth, "Auth Provider", "OIDC")
    System_Ext(ai, "AI API", "LLM")
    System_Ext(pay, "Payments", "Stripe")
    System_Ext(mail, "Mail", "SES")

    System_Boundary(sys, "The System") {
        Container(spa, "Web SPA", "React", "UI")
        Container(api, "API Server", "Rust", "Backend")
        ContainerDb(db, "Database", "SQLite", "Storage")
    }

    Rel(dev, spa, "Uses")
    Rel(spa, api, "Calls")
    Rel(api, db, "Reads/writes")
    Rel(api, auth, "Validates JWT")
    Rel(api, ai, "Chats")
    Rel(api, pay, "Charges")
    Rel(api, mail, "Sends mail")
"#;

#[test]
fn c4_force_layout_shortens_relationships() {
    let parsed = parse_mermaid(C4_EXTERNALS_AROUND_BOUNDARY).unwrap();
    let theme = Theme::modern();

    let mut on = LayoutConfig::default();
    on.c4.force_layout = true;
    let layout_on = compute_layout(&parsed.graph, &theme, &on);

    let mut off = LayoutConfig::default();
    off.c4.force_layout = false;
    let layout_off = compute_layout(&parsed.graph, &theme, &off);

    let len_on = c4_total_rel_length(&layout_on);
    let len_off = c4_total_rel_length(&layout_off);

    assert!(
        len_on < len_off,
        "force layout should shorten total relationship length (on={len_on}, off={len_off})"
    );
}

#[test]
fn c4_force_layout_keeps_boundary_contents_rigid() {
    let parsed = parse_mermaid(C4_EXTERNALS_AROUND_BOUNDARY).unwrap();
    let theme = Theme::modern();

    let mut off = LayoutConfig::default();
    off.c4.force_layout = false;
    let layout_off = compute_layout(&parsed.graph, &theme, &off);

    let mut on = LayoutConfig::default();
    on.c4.force_layout = true;
    let layout_on = compute_layout(&parsed.graph, &theme, &on);

    // The boundary moves as a rigid body: the relative offsets between its
    // contained shapes must be identical with and without the force pass.
    let rel_offset = |layout: &Layout, a: &str, b: &str| -> (f32, f32) {
        let na = &layout.nodes[a];
        let nb = &layout.nodes[b];
        (nb.x - na.x, nb.y - na.y)
    };
    for (a, b) in [("spa", "api"), ("api", "db"), ("spa", "db")] {
        let off_off = rel_offset(&layout_off, a, b);
        let on_off = rel_offset(&layout_on, a, b);
        assert!(
            (off_off.0 - on_off.0).abs() < 0.5 && (off_off.1 - on_off.1).abs() < 0.5,
            "boundary content {a}->{b} should keep its relative position \
             (off={off_off:?}, on={on_off:?})"
        );
    }
}

#[test]
fn c4_force_layout_never_lengthens_relationships() {
    // A small, already-compact diagram: the force pass must not make it worse.
    // The internal guard restores the row-packed placement when refinement
    // wouldn't help, so the length is identical (within float noise).
    let input = r#"C4Container
    title Tidy
    System_Boundary(sys, "Sys") {
        Container(a, "A", "x", "")
        Container(b, "B", "x", "")
    }
    Rel(a, b, "Calls")
"#;
    let parsed = parse_mermaid(input).unwrap();
    let theme = Theme::modern();

    let mut on = LayoutConfig::default();
    on.c4.force_layout = true;
    let len_on = c4_total_rel_length(&compute_layout(&parsed.graph, &theme, &on));

    let mut off = LayoutConfig::default();
    off.c4.force_layout = false;
    let len_off = c4_total_rel_length(&compute_layout(&parsed.graph, &theme, &off));

    assert!(
        len_on <= len_off + 0.5,
        "force layout must not lengthen relationships on a tidy diagram \
         (on={len_on}, off={len_off})"
    );
}

#[test]
fn c4_grid_placement_and_routing_avoid_crossings_and_boxes() {
    // Externals declared before the boundary that all fan into one container:
    // grid placement should remove edge crossings and routing should keep
    // every line clear of non-endpoint shape boxes.
    let input = r#"C4Container
    title Routing test
    Person(dev, "Developer", "x")
    System_Ext(a, "Auth", "x")
    System_Ext(b, "AI", "x")
    System_Ext(c, "Pay", "x")
    System_Boundary(sys, "System") {
        Container(spa, "SPA", "x", "")
        Container(api, "API", "x", "")
        ContainerDb(db, "DB", "x", "")
    }
    Rel(dev, spa, "Uses")
    Rel(spa, api, "Calls")
    Rel(api, db, "RW")
    Rel(api, a, "Auth")
    Rel(api, b, "Chat")
    Rel(api, c, "Pay")
"#;
    let parsed = parse_mermaid(input).unwrap();
    let theme = Theme::modern();
    let config = LayoutConfig::default(); // grid + routing on by default
    let layout = compute_layout(&parsed.graph, &theme, &config);

    // Box obstacles keyed by node id.
    let boxes: Vec<(String, (f32, f32, f32, f32))> = layout
        .nodes
        .values()
        .map(|n| (n.id.clone(), (n.x, n.y, n.width, n.height)))
        .collect();

    // No relationship segment may pass through a non-endpoint shape box.
    for e in &layout.edges {
        for w in e.points.windows(2) {
            for (id, rect) in &boxes {
                if *id == e.from || *id == e.to {
                    continue;
                }
                // shrink the rect slightly so a line grazing the border or
                // terminating at it doesn't count.
                let inset = (rect.0 + 2.0, rect.1 + 2.0, rect.2 - 4.0, rect.3 - 4.0);
                if inset.2 <= 0.0 || inset.3 <= 0.0 {
                    continue;
                }
                assert!(
                    !segment_intersects_rect(w[0], w[1], inset),
                    "edge {}->{} passes through box {id}",
                    e.from,
                    e.to
                );
            }
        }
    }
}

#[test]
fn c4_multiple_edges_on_one_shape_get_distinct_ports() {
    // Several relationships all touch `hub`; their attachment points on hub
    // must be distinct (not all the same border point) so each arrow reads
    // separately.
    let input = r#"C4Container
    title Ports test
    System(hub, "Hub", "core")
    System_Ext(a, "A", "x")
    System_Ext(b, "B", "x")
    System_Ext(c, "C", "x")
    System_Ext(d, "D", "x")
    Rel(a, hub, "ra")
    Rel(b, hub, "rb")
    Rel(c, hub, "rc")
    Rel(d, hub, "rd")
"#;
    let parsed = parse_mermaid(input).unwrap();
    let theme = Theme::modern();
    let config = LayoutConfig::default();
    let layout = compute_layout(&parsed.graph, &theme, &config);

    // Collect the endpoint that lands on `hub` for each edge.
    let mut hub_points: Vec<(f32, f32)> = Vec::new();
    for e in &layout.edges {
        if e.from == "hub" {
            hub_points.push(e.points[0]);
        } else if e.to == "hub" {
            hub_points.push(*e.points.last().unwrap());
        }
    }
    assert!(hub_points.len() >= 2, "expected multiple edges on hub");
    // No two attachment points should coincide.
    for i in 0..hub_points.len() {
        for j in (i + 1)..hub_points.len() {
            let d = (hub_points[i].0 - hub_points[j].0).hypot(hub_points[i].1 - hub_points[j].1);
            assert!(
                d > 2.0,
                "two edges share the same port on hub: {:?} vs {:?}",
                hub_points[i],
                hub_points[j]
            );
        }
    }
}

#[test]
fn c4_routed_lines_are_orthogonal_with_min_stub() {
    let input = r#"C4Container
    title Ortho test
    Person(dev, "Dev", "x")
    System_Ext(a, "A", "x")
    System_Ext(b, "B", "x")
    System_Boundary(sys, "Sys") {
        Container(spa, "SPA", "x", "")
        Container(api, "API", "x", "")
        ContainerDb(db, "DB", "x", "")
    }
    Rel(dev, spa, "Uses")
    Rel(spa, api, "Calls")
    Rel(api, db, "RW")
    Rel(api, a, "X")
    Rel(api, b, "Y")
"#;
    let parsed = parse_mermaid(input).unwrap();
    let theme = Theme::modern();
    let mut config = LayoutConfig::default();
    config.c4.rel_routing = "ortho".to_string(); // this test asserts ortho geometry
    let layout = compute_layout(&parsed.graph, &theme, &config);

    for e in &layout.edges {
        // Every segment must be axis-aligned (no diagonals).
        for w in e.points.windows(2) {
            let dx = (w[0].0 - w[1].0).abs();
            let dy = (w[0].1 - w[1].1).abs();
            assert!(
                dx < 0.5 || dy < 0.5,
                "edge {}->{} has a diagonal segment {:?}->{:?}",
                e.from,
                e.to,
                w[0],
                w[1]
            );
        }
        // The first straight run leaving the source (sum of the leading
        // colinear segments) must be at least ~3x the arrowhead (~30px).
        if e.points.len() >= 2 {
            let first_len = (e.points[0].0 - e.points[1].0).hypot(e.points[0].1 - e.points[1].1);
            assert!(
                first_len >= 28.0,
                "edge {}->{} first stub too short: {first_len}",
                e.from,
                e.to
            );
        }
    }
}

#[test]
fn c4_arc_mode_curves_and_distinct_ports() {
    // Arc mode: 2-point curved edges with distinct ports per shape side, and
    // no edge crossings on a simple hub diagram (where ortho can tangle).
    let input = r#"C4Container
    title Arc test
    System_Boundary(p, "Platform") {
        System(api, "API", "core")
        SystemDb(db, "DB", "store")
        System(cache, "Cache", "redis")
        System(queue, "Queue", "mq")
    }
    Rel(api, db, "Queries")
    Rel(api, cache, "Caches")
    Rel(api, queue, "Publishes")
"#;
    let parsed = parse_mermaid(input).unwrap();
    let theme = Theme::modern();
    let mut config = LayoutConfig::default();
    config.c4.rel_routing = "arc".to_string();
    let layout = compute_layout(&parsed.graph, &theme, &config);

    let DiagramData::C4(c4) = &layout.diagram else {
        panic!("expected C4 layout");
    };
    // Every relationship is a 2-point curved arc.
    for rel in &c4.rels {
        assert!(rel.curved, "arc mode rel {}->{} should be curved", rel.from, rel.to);
        assert_eq!(
            rel.points.len(),
            2,
            "arc rel {}->{} should have 2 points",
            rel.from,
            rel.to
        );
    }
    // The three edges leaving `api` attach at distinct ports.
    let api_ports: Vec<(f32, f32)> = c4
        .rels
        .iter()
        .filter(|r| r.from == "api")
        .map(|r| r.start)
        .collect();
    assert!(api_ports.len() >= 3);
    for i in 0..api_ports.len() {
        for j in (i + 1)..api_ports.len() {
            let d =
                (api_ports[i].0 - api_ports[j].0).hypot(api_ports[i].1 - api_ports[j].1);
            assert!(d > 2.0, "api arcs share a port: {:?} {:?}", api_ports[i], api_ports[j]);
        }
    }
}

#[test]
fn c4_auto_routing_picks_lowest_quality_score() {
    use mermaid_rs_renderer::layout::c4_quality_for_layout;
    // 'auto' must never score worse than any single mode it chooses among.
    let input = r#"C4Container
    title Auto test
    System_Boundary(p, "Platform") {
        System(api, "API", "core")
        SystemDb(db, "DB", "store")
        System(cache, "Cache", "redis")
        System(queue, "Queue", "mq")
    }
    Rel(api, db, "Queries")
    Rel(api, cache, "Caches")
    Rel(api, queue, "Publishes")
"#;
    let parsed = parse_mermaid(input).unwrap();
    let theme = Theme::modern();

    let score_for = |mode: &str| -> f32 {
        let mut config = LayoutConfig::default();
        config.c4.rel_routing = mode.to_string();
        let layout = compute_layout(&parsed.graph, &theme, &config);
        c4_quality_for_layout(&layout).unwrap().score
    };

    let auto = score_for("auto");
    let ortho = score_for("ortho");
    let arc = score_for("arc");
    let straight = score_for("straight");
    let best = ortho.min(arc).min(straight);
    assert!(
        auto <= best + 1.0,
        "auto ({auto}) should match the best single mode ({best}): \
         ortho={ortho} arc={arc} straight={straight}"
    );
}

#[test]
fn c4_optimize_eliminates_box_hits_and_crossings() {
    use mermaid_rs_renderer::layout::c4_quality_for_layout;
    // A boundary with a container (desktop) that connects to others above it;
    // with c4ShapeInRow=2 and optimize on, the internal reorder + annealing
    // should reach 0 crossings and 0 lines-through-boxes.
    let input = r#"C4Container
    title Optimize test
    System_Boundary(sys, "Sys") {
        Container(spa, "SPA", "x", "")
        Container(wasm, "WASM", "x", "")
        Container(api, "API", "x", "")
        ContainerDb(db, "DB", "x", "")
        Container(desktop, "Desktop", "x", "")
    }
    System_Ext(a, "A", "x")
    System_Ext(b, "B", "x")
    Rel(spa, wasm, "loads")
    Rel(spa, api, "calls")
    Rel(api, db, "rw")
    Rel(desktop, api, "spawns")
    Rel(desktop, spa, "renders")
    Rel(api, a, "x")
    Rel(api, b, "y")
    UpdateLayoutConfig($c4ShapeInRow="2")
"#;
    let parsed = parse_mermaid(input).unwrap();
    let theme = Theme::modern();
    let mut config = LayoutConfig::default();
    config.c4.optimize = true;
    config.c4.optimize_iterations = 600;
    let layout = compute_layout(&parsed.graph, &theme, &config);
    let q = c4_quality_for_layout(&layout).unwrap();
    assert_eq!(q.box_hits, 0, "optimize should remove lines through boxes");
    assert_eq!(q.crossings, 0, "optimize should remove crossings");
}

#[test]
fn c4_none_routing_reattaches_edges_after_anneal() {
    // Regression: with relRouting="none" AND optimize on, the annealing pass
    // moves shapes around. The "none" branch must recompute each edge's
    // endpoints from the CURRENT shape positions; otherwise the drawn lines stay
    // anchored where the boxes used to be and float away from them. We assert
    // every relationship's start and end sit on some shape's border.
    let input = r#"C4Container
    title None routing test
    System(a, "A", "x")
    System(b, "B", "y")
    System(c, "C", "z")
    System(d, "D", "w")
    Rel(a, b, "r1")
    Rel(b, c, "r2")
    Rel(c, d, "r3")
    Rel(a, d, "r4")
"#;
    let parsed = parse_mermaid(input).unwrap();
    let theme = Theme::modern();
    let mut config = LayoutConfig::default();
    config.c4.optimize = true;
    config.c4.optimize_iterations = 200;
    config.c4.rel_routing = "none".to_string();
    let layout = compute_layout(&parsed.graph, &theme, &config);

    let DiagramData::C4(c4) = &layout.diagram else {
        panic!("expected C4 layout");
    };
    let boxes: Vec<(f32, f32, f32, f32)> = c4
        .shapes
        .iter()
        .map(|s| (s.x, s.y, s.width, s.height))
        .collect();
    let on_any_border = |p: (f32, f32)| -> bool {
        let eps = 2.5;
        boxes.iter().any(|&(x, y, w, h)| {
            let on_x = (p.0 - x).abs() < eps || (p.0 - (x + w)).abs() < eps;
            let on_y = (p.1 - y).abs() < eps || (p.1 - (y + h)).abs() < eps;
            let in_x = p.0 >= x - eps && p.0 <= x + w + eps;
            let in_y = p.1 >= y - eps && p.1 <= y + h + eps;
            (on_x && in_y) || (on_y && in_x)
        })
    };
    for rel in &c4.rels {
        assert!(
            on_any_border(rel.start),
            "none-mode edge {}->{} start {:?} detached from all boxes after anneal",
            rel.from, rel.to, rel.start
        );
        assert!(
            on_any_border(rel.end),
            "none-mode edge {}->{} end {:?} detached from all boxes after anneal",
            rel.from, rel.to, rel.end
        );
    }
}

#[test]
fn c4_shape_in_row_directive_is_honored() {
    // $c4ShapeInRow="2" must wrap the boundary's containers two per row
    // (regression: the quoted value wasn't being parsed).
    let input = r#"C4Container
    System_Boundary(sys, "Sys") {
        Container(a, "A", "x", "")
        Container(b, "B", "x", "")
        Container(c, "C", "x", "")
        Container(d, "D", "x", "")
    }
    UpdateLayoutConfig($c4ShapeInRow="2")
"#;
    let parsed = parse_mermaid(input).unwrap();
    assert_eq!(parsed.graph.c4.c4_shape_in_row_override, Some(2));
    let theme = Theme::modern();
    let config = LayoutConfig::default();
    let layout = compute_layout(&parsed.graph, &theme, &config);
    // a and b should share a row (same y); c and d on the next row.
    let ay = layout.nodes["a"].y;
    let by = layout.nodes["b"].y;
    let cy = layout.nodes["c"].y;
    assert!((ay - by).abs() < 1.0, "a and b should be on the same row");
    assert!(cy > ay + 1.0, "c should wrap to the next row");
}

#[test]
fn c4_rel_labels_avoid_nodes_and_lines() {
    // On a simple diagram, every relationship label should land clear of all
    // node boxes (its text isn't buried in a component).
    let input = r#"C4Container
    title Label test
    Person(u, "User", "x")
    System(api, "API", "core")
    SystemDb(db, "DB", "store")
    System_Ext(ext, "Ext", "y")
    Rel(u, api, "Uses")
    Rel(api, db, "Reads/Writes")
    Rel(api, ext, "Calls")
"#;
    let parsed = parse_mermaid(input).unwrap();
    let theme = Theme::modern();
    let config = LayoutConfig::default();
    let layout = compute_layout(&parsed.graph, &theme, &config);
    let DiagramData::C4(c4) = &layout.diagram else {
        panic!("expected C4 layout");
    };
    let node_rects: Vec<(f32, f32, f32, f32)> = layout
        .nodes
        .values()
        .map(|n| (n.x, n.y, n.width, n.height))
        .collect();
    for rel in &c4.rels {
        // label center
        let cx = rel.label_base.0 + rel.offset_x;
        let cy = rel.label_base.1 + rel.offset_y;
        for (nx, ny, nw, nh) in &node_rects {
            let inside = cx > *nx && cx < nx + nw && cy > *ny && cy < ny + nh;
            assert!(
                !inside,
                "label for {}->{} at ({cx},{cy}) is inside a node box",
                rel.from, rel.to
            );
        }
    }
}

#[test]
fn c4_rel_label_drawn_at_scored_center() {
    // Regression: a relationship label must be DRAWN centred on the very anchor
    // the layout scorer placed it at (`rel.label_base + offset`), not anchored
    // by its top. Top-anchoring shifts the block down by half its height — more
    // for multi-line labels — pushing the visible text off the collision-checked
    // spot. We assert the drawn primary-label block's vertical centre equals the
    // scored anchor's y.
    let input = r#"C4Container
    title Anchor test
    System(a, "A", "x")
    System(b, "B", "y")
    Rel(a, b, "Sends events to the other system here", "HTTPS/JSON")
"#;
    let parsed = parse_mermaid(input).unwrap();
    let theme = Theme::modern();
    let config = LayoutConfig::default();
    let layout = compute_layout(&parsed.graph, &theme, &config);
    let svg = render_svg(&layout, &theme, &config);

    let DiagramData::C4(c4) = &layout.diagram else {
        panic!("expected C4 layout");
    };
    let rel = c4
        .rels
        .iter()
        .find(|r| r.from == "a" && r.to == "b")
        .expect("a->b rel missing");
    // The anchor the scorer used to place the label's rect.
    let scored_center_y = rel.label_base.1 + rel.offset_y;
    let n_lines = rel.label.lines.len().max(1) as f32;

    // Collect each drawn line's absolute centre (text y + tspan dy), keyed by
    // text content, then reconstruct the primary block's centre as the mean of
    // its line centres (a top- vs centre-anchored block differ by half height).
    let mut line_centers: Vec<f32> = Vec::new();
    for chunk in svg.split("<text ").skip(1) {
        let Some(y) = attr_f32(chunk, "y=\"") else {
            continue;
        };
        let Some(dy_pos) = chunk.find("dy=\"") else {
            continue;
        };
        let dy = attr_f32(&chunk[dy_pos..], "dy=\"").unwrap_or(0.0);
        let after = &chunk[dy_pos..];
        let Some(gt) = after.find('>') else { continue };
        let Some(end) = after[gt + 1..].find("</tspan>") else {
            continue;
        };
        let text = &after[gt + 1..gt + 1 + end];
        if text.contains("Sends events") || text.contains("other system") {
            line_centers.push(y + dy);
        }
    }
    assert_eq!(
        line_centers.len() as f32, n_lines,
        "expected {n_lines} primary-label lines drawn, found {}",
        line_centers.len()
    );
    let drawn_center_y = line_centers.iter().sum::<f32>() / line_centers.len() as f32;

    assert!(
        (drawn_center_y - scored_center_y).abs() < 1.0,
        "primary label drawn centre {drawn_center_y} should match scored anchor \
         {scored_center_y}; a top-anchored render shifts it down ~{} px",
        12.0 * n_lines / 2.0
    );
}

/// Parse the float value of the first occurrence of `key` (e.g. `y=\"`) in `s`.
fn attr_f32(s: &str, key: &str) -> Option<f32> {
    let start = s.find(key)? + key.len();
    let end = s[start..].find('"')? + start;
    s[start..end].parse::<f32>().ok()
}
