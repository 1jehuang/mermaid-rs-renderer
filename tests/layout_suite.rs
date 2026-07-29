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
        "architecture/ports.mmd",
        "architecture/junctions.mmd",
        "architecture/bends.mmd",
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
        "flowchart/decision_shapes.mmd",
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
        "radar/negative.mmd",
        "radar/short_curve.mmd",
        "requirement/basic.mmd",
        "sankey/basic.mmd",
        "sequence/basic.mmd",
        "sequence/frames.mmd",
        "state/basic.mmd",
        "state/note.mmd",
        "state/note_block.mmd",
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

/// Issue #129: peer branches from a TD decision must fan through the lower
/// diamond sides instead of sharing the bottom apex and hooking sideways.
#[test]
fn decision_branches_use_lower_sides_independent_of_declaration_order() {
    fn assert_branch_ports(input: &str, decision_id: &str, target_ids: [&str; 2]) {
        let parsed = parse_mermaid(input).expect("parse failed");
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let layout = compute_layout(&parsed.graph, &theme, &config);
        let decision = layout
            .nodes
            .get(decision_id)
            .unwrap_or_else(|| panic!("decision node {decision_id}"));
        let decision_cx = decision.x + decision.width * 0.5;
        let decision_cy = decision.y + decision.height * 0.5;
        let mut starts = Vec::new();

        for target_id in target_ids {
            let target = layout.nodes.get(target_id).expect("branch target");
            let target_cx = target.x + target.width * 0.5;
            let edge = layout
                .edges
                .iter()
                .find(|edge| edge.from == decision_id && edge.to == target_id)
                .expect("decision branch edge");
            let first = *edge.points.first().expect("branch start point");
            let last = *edge.points.last().expect("branch end point");
            let normalized_boundary_distance = (first.0 - decision_cx).abs()
                / (decision.width * 0.5)
                + (first.1 - decision_cy).abs() / (decision.height * 0.5);
            let below_center_fraction = (first.1 - decision_cy) / (decision.height * 0.5);
            let exits_toward_target = (target_cx < decision_cx && first.0 < decision_cx)
                || (target_cx > decision_cx && first.0 > decision_cx);

            assert!(
                (normalized_boundary_distance - 1.0).abs() <= 0.03
                    && (0.14..=0.85).contains(&below_center_fraction)
                    && exits_toward_target,
                "{decision_id}->{target_id} should leave through the lower diamond slant toward its target, never a side vertex or bottom apex; got {first:?} for decision {decision:?}"
            );
            assert!(
                (last.1 - target.y).abs() <= 0.5 && (last.0 - target_cx).abs() <= 5.0,
                "{decision_id}->{target_id} should enter near the target's top center, got {last:?} for target {target:?}"
            );
            assert!(
                edge.points
                    .windows(2)
                    .all(|segment| segment[1].1 + 0.5 >= segment[0].1),
                "{decision_id}->{target_id} should be y-monotone, got {:?}",
                edge.points
            );
            starts.push(first);
        }

        assert_eq!(starts.len(), 2);
        assert!(
            ((starts[0].0 - decision_cx) + (starts[1].0 - decision_cx)).abs() <= 2.0
                && (starts[0].1 - starts[1].1).abs() <= 1.0,
            "peer branches should leave the diamond symmetrically, got {starts:?}"
        );
    }

    for input in [
        r#"flowchart TD
C{Decision}
C -->|One| D[Result one]
C -->|Two| E[Result two]
"#,
        r#"flowchart TD
C{Decision}
C -->|Two| E[Result two]
C -->|One| D[Result one]
"#,
    ] {
        assert_branch_ports(input, "C", ["D", "E"]);
    }

    let screenshot_repro = r#"flowchart TD
A[Hard edge] -->|Link text| B(Round edge)
B --> C{Decision}
C -->|One| D[Result one]
C -->|Two| E[Result two]
E --> F{Let me think}
F -->|Yes| G[G]
F -->|No| H[H]
"#;
    assert_branch_ports(screenshot_repro, "C", ["D", "E"]);
    assert_branch_ports(screenshot_repro, "F", ["G", "H"]);
}

/// Issues #122 and #123: aligned downstream targets must not make decision
/// routing depend on declaration order or spacing overrides.
#[test]
fn aligned_decision_branches_avoid_apex_hooks_independent_of_order_and_spacing() {
    fn assert_branch_ports(input: &str, config: &LayoutConfig) {
        let parsed = parse_mermaid(input).expect("parse failed");
        let theme = Theme::modern();
        let layout = compute_layout(&parsed.graph, &theme, config);
        let decision = layout.nodes.get("drawn").expect("decision node drawn");
        let decision_cx = decision.x + decision.width * 0.5;
        let decision_cy = decision.y + decision.height * 0.5;
        let mut left_exits = 0usize;
        let mut right_exits = 0usize;

        for target_id in ["ai", "createDraft"] {
            let target = layout.nodes.get(target_id).expect("branch target");
            let edge = layout
                .edges
                .iter()
                .find(|edge| edge.from == "drawn" && edge.to == target_id)
                .expect("decision branch edge");
            let first = *edge.points.first().expect("branch start point");
            let second = *edge.points.get(1).expect("branch start stub");
            let penultimate = *edge
                .points
                .get(edge.points.len().saturating_sub(2))
                .expect("branch end stub");
            let last = *edge.points.last().expect("branch end point");
            let normalized_boundary_distance = (first.0 - decision_cx).abs()
                / (decision.width * 0.5)
                + (first.1 - decision_cy).abs() / (decision.height * 0.5);
            let exits_left = first.0 < decision_cx - 2.0;
            let exits_right = first.0 > decision_cx + 2.0;
            left_exits += usize::from(exits_left);
            right_exits += usize::from(exits_right);

            assert!(
                (normalized_boundary_distance - 1.0).abs() <= 0.03
                    && first.1 >= decision_cy - 0.5
                    && decision.y + decision.height - first.1 > 2.0
                    && (exits_left || exits_right),
                "drawn->{target_id} should leave through a lower decision side, never the bottom apex; got {first:?} for decision {decision:?}"
            );
            assert!(
                (second.1 - first.1).abs() <= 0.5
                    && ((exits_left && second.0 < first.0) || (exits_right && second.0 > first.0)),
                "drawn->{target_id} should start with an outward horizontal stub, got {first:?} -> {second:?}"
            );
            assert!(
                (last.1 - target.y).abs() <= 0.5
                    && last.0 >= target.x - 0.5
                    && last.0 <= target.x + target.width + 0.5,
                "drawn->{target_id} should enter the target through its top border, got {last:?} for target {target:?}"
            );
            assert!(
                (penultimate.0 - last.0).abs() <= 0.5 && penultimate.1 < last.1,
                "drawn->{target_id} should finish with a downward vertical stub, got {penultimate:?} -> {last:?}"
            );
        }
        assert_eq!(
            (left_exits, right_exits),
            (1, 1),
            "the two aligned decision branches should fan through opposite sides"
        );

        let improve = layout.nodes.get("improve").expect("decision node improve");
        let forward = layout
            .edges
            .iter()
            .find(|edge| edge.from == "improve" && edge.to == "e")
            .expect("single forward decision branch");
        let first = *forward.points.first().expect("forward branch start");
        assert!(
            (first.0 - (improve.x + improve.width * 0.5)).abs() <= 0.5
                && (first.1 - (improve.y + improve.height)).abs() <= 0.5,
            "a decision with only one forward branch should keep its bottom-apex exit, got {first:?} for improve {improve:?}"
        );
    }

    let inputs = [
        r#"flowchart TD
start(Need flowchart) --> drawn{Have a<br>drawing or<br>draft flowchart?}
drawn -->|Yes| ai[Provide to AI<br>and create flowchart]
drawn -->|No| createDraft[Make a draft]
createDraft --> ai

ai --> improve{Needs<br>improvement?}
improve -->|Yes| drawn
improve -->|No| e(End)
"#,
        r#"flowchart TD
start(Need flowchart) --> drawn{Have a<br>drawing or<br>draft flowchart?}
drawn -->|No| createDraft[Make a draft]
drawn -->|Yes| ai[Provide to AI<br>and create flowchart]
createDraft --> ai

ai --> improve{Needs<br>improvement?}
improve -->|Yes| drawn
improve -->|No| e(End)
"#,
    ];
    let spaced = LayoutConfig {
        node_spacing: 100.0,
        rank_spacing: 80.0,
        ..LayoutConfig::default()
    };

    for input in inputs {
        assert_branch_ports(input, &LayoutConfig::default());
        assert_branch_ports(input, &spaced);
    }

    let parsed = parse_mermaid(
        r#"flowchart TD
C{Decision}
C --> D[Only target]
C --> D
"#,
    )
    .expect("parse duplicate-edge case");
    let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
    let decision = layout.nodes.get("C").expect("decision node C");
    let duplicate_edges: Vec<_> = layout
        .edges
        .iter()
        .filter(|edge| edge.from == "C" && edge.to == "D")
        .collect();
    assert_eq!(duplicate_edges.len(), 2, "expected two parallel edges");
    for edge in duplicate_edges {
        let first = edge.points.first().expect("duplicate branch start");
        assert!(
            (first.0 - (decision.x + decision.width * 0.5)).abs() <= 0.5
                && (first.1 - (decision.y + decision.height)).abs() <= 0.5,
            "parallel edges to one target should not count as a multi-target fan; got {first:?}"
        );
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
fn pie_all_zero_values_use_equal_finite_slices() {
    let parsed = parse_mermaid("pie\n\"A\" : 0\n\"B\" : 0\n\"C\" : 0\n").unwrap();
    let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
    let DiagramData::Pie(pie) = &layout.diagram else {
        panic!("expected pie")
    };
    assert_eq!(pie.slices.len(), 3);
    for slice in &pie.slices {
        let span = slice.end_angle - slice.start_angle;
        assert!(span.is_finite());
        assert!((span - std::f32::consts::TAU / 3.0).abs() < 1e-4);
    }
}

#[test]
fn quadrant_point_label_bboxes_stay_inside_canvas() {
    let parsed = parse_mermaid(
        "quadrantChart\n  Very long left boundary label: [0, 1]\n  Much much longer right boundary point label: [1, 0]\n",
    )
    .unwrap();
    let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
    let DiagramData::Quadrant(quadrant) = &layout.diagram else {
        panic!("expected quadrant")
    };
    for point in &quadrant.points {
        assert!(point.x - point.label.width / 2.0 >= 0.0);
        assert!(point.x + point.label.width / 2.0 <= layout.width);
        assert!(point.y - point.label.height / 2.0 >= 0.0);
        assert!(point.y + point.label.height / 2.0 <= layout.height);
    }

    let top_left = &quadrant.points[0];
    assert_eq!(top_left.x, quadrant.grid_x);
    assert_eq!(top_left.y, quadrant.grid_y);

    let bottom_right = &quadrant.points[1];
    assert_eq!(bottom_right.x, quadrant.grid_x + quadrant.grid_width);
    assert_eq!(bottom_right.y, quadrant.grid_y + quadrant.grid_height);
}

#[test]
fn xychart_category_labels_have_collision_safe_spacing_and_band() {
    let parsed = parse_mermaid("xychart-beta\n  x-axis [A very long category, Another very long category, Third very long category]\n  bar [1, 2, 3]\n").unwrap();
    let theme = Theme::modern();
    let config = LayoutConfig::default();
    let layout = compute_layout(&parsed.graph, &theme, &config);
    let DiagramData::XYChart(xy) = &layout.diagram else {
        panic!("expected xychart")
    };
    for pair in xy.x_axis_categories.windows(2) {
        assert!(
            pair[1].1 - pair[0].1 > 140.0,
            "long categories remain too close"
        );
    }
    assert!(layout.height - (xy.plot_y + xy.plot_height) >= theme.font_size + 16.0);
}

#[test]
fn radar_multiline_title_increases_top_band() {
    let mut parsed = parse_mermaid("radar-beta\n  axis A, B, C\n  curve S { 1, 2, 3 }\n").unwrap();
    parsed.graph.radar.title = Some("First title line\nSecond title line".to_string());
    let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
    let DiagramData::Radar(radar) = &layout.diagram else {
        panic!("expected radar")
    };
    assert!(
        radar.center_y > 350.0,
        "multiline title did not move chart down"
    );
}

#[test]
fn sankey_disconnected_nodes_do_not_overlap_and_cycles_expand_ranks() {
    let parsed = parse_mermaid("sankey-beta\nA,B,5\nC,D,5\n").unwrap();
    let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
    let DiagramData::Sankey(sankey) = &layout.diagram else {
        panic!("expected sankey")
    };
    for (index, node) in sankey.nodes.iter().enumerate() {
        for other in sankey
            .nodes
            .iter()
            .skip(index + 1)
            .filter(|other| other.rank == node.rank)
        {
            assert!(node.y + node.height <= other.y || other.y + other.height <= node.y);
        }
    }

    let parsed = parse_mermaid("sankey-beta\nA,B,1\nB,C,1\nC,A,1\n").unwrap();
    let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
    let DiagramData::Sankey(sankey) = &layout.diagram else {
        panic!("expected sankey")
    };
    let ranks: std::collections::BTreeSet<_> = sankey.nodes.iter().map(|node| node.rank).collect();
    assert!(ranks.len() > 1, "cyclic graph collapsed into one column");
}

#[test]
fn sankey_cycles_preserve_condensation_ranks_and_forward_edges() {
    let parsed = parse_mermaid(
        "sankey-beta\nSource,A,1\nA,B,1\nB,A,1\nB,Sink,1\nOther,Leaf,1\nLoop,Loop,1\n",
    )
    .unwrap();
    let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
    let DiagramData::Sankey(sankey) = &layout.diagram else {
        panic!("expected sankey")
    };
    let ranks: std::collections::BTreeMap<_, _> = sankey
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node.rank))
        .collect();

    assert_eq!(ranks["Source"], 0);
    assert_eq!(ranks["Other"], 0);
    assert_eq!(ranks["Leaf"], 1);
    assert_eq!(
        ranks["Loop"], 0,
        "self-loop should not shift other components"
    );
    assert_ne!(ranks["A"], ranks["B"], "SCC members must not collapse");
    assert!(ranks["A"] >= 1 && ranks["B"] >= 1);
    assert!(ranks["Sink"] > ranks["A"] && ranks["Sink"] > ranks["B"]);

    for link in &sankey.links {
        let source_component = matches!(
            (link.source.as_str(), link.target.as_str()),
            ("A", "B") | ("B", "A")
        );
        let self_loop = link.source == link.target;
        if !source_component && !self_loop {
            assert!(ranks[link.source.as_str()] < ranks[link.target.as_str()]);
        }
    }
}

#[test]
fn sankey_scc_member_order_follows_declaration_order_deterministically() {
    fn ranks(input: &str) -> std::collections::BTreeMap<String, usize> {
        let parsed = parse_mermaid(input).unwrap();
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let DiagramData::Sankey(sankey) = layout.diagram else {
            panic!("expected sankey")
        };
        sankey
            .nodes
            .into_iter()
            .map(|node| (node.id, node.rank))
            .collect()
    }

    let first = ranks("sankey-beta\nA,B,1\nB,C,1\nC,A,1\nC,D,1\n");
    let repeated = ranks("sankey-beta\nA,B,1\nB,C,1\nC,A,1\nC,D,1\n");
    let reordered = ranks("sankey-beta\nC,A,1\nA,B,1\nB,C,1\nC,D,1\n");

    assert_eq!(first, repeated);
    assert!(first["A"] < first["B"] && first["B"] < first["C"]);
    assert!(reordered["C"] < reordered["A"] && reordered["A"] < reordered["B"]);
    assert!(first["D"] > first["C"] && reordered["D"] > reordered["B"]);
}

/// Three frames of three stacked rows each, chained through their middle rows.
fn three_stacked_frames_input() -> String {
    let frames: String = ["A", "B", "C"]
        .iter()
        .map(|c| {
            format!(
                "  subgraph F{c}[\"frame {c}\"]\n    {c}1[\"row one\"]\n    {c}2[\"row two\"]\n\
                     {c}3[\"row three\"]\n    {c}1 --- {c}2\n    {c}2 --- {c}3\n  end\n"
            )
        })
        .collect();
    format!("flowchart TB\n{frames}  A2 --> B2\n  B2 --> C2\n")
}

/// Chained subgraphs must stack in edge order, and that order must not depend
/// on how tall the frame titles happen to render.
///
/// Ranking is subgraph-blind, so `B1`/`C1` (no incoming edge) used to land on
/// rank 0 beside `A1`, leaving all three frames overlapping. The pass that
/// pulled them apart then sorted on a bound that included the title padding,
/// so a two-line title reordered the frames: here `foo()` is the only one-line
/// title, which was enough to sink it below `main()`.
#[test]
fn chained_subgraphs_stack_in_edge_order_regardless_of_title_height() {
    fn frame_tops(titles: [&str; 3]) -> [f32; 3] {
        let [ta, tb, tc] = titles;
        let input = format!(
            "flowchart TB\n\
             subgraph FA[\"{ta}\"]\n A1[\"a one\"]\n A2[\"a two\"]\n A3[\"a three\"]\n A1 --- A2\n A2 --- A3\n end\n\
             subgraph FB[\"{tb}\"]\n B1[\"b one\"]\n B2[\"b two\"]\n B3[\"b three\"]\n B1 --- B2\n B2 --- B3\n end\n\
             subgraph FC[\"{tc}\"]\n C1[\"c one\"]\n C2[\"c two\"]\n C3[\"c three\"]\n C1 --- C2\n C2 --- C3\n end\n\
             A2 --> B2\n B2 --> C2\n"
        );
        let parsed = parse_mermaid(&input).expect("parse failed");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        ["A", "B", "C"].map(|frame| {
            (1..=3)
                .map(|row| {
                    let id = format!("{frame}{row}");
                    layout.nodes.get(&id).unwrap_or_else(|| panic!("{id}")).y
                })
                .fold(f32::MAX, f32::min)
        })
    }

    // The title lengths that originally broke it: only the middle frame fits
    // on one line.
    let uneven = frame_tops([
        "baz() — innermost, lowest address",
        "foo()",
        "main() — outermost, highest address",
    ]);
    let even = frame_tops(["baz()", "foo()", "main()"]);

    for (case, tops) in [("uneven titles", uneven), ("even titles", even)] {
        assert!(
            tops[0] < tops[1] && tops[1] < tops[2],
            "{case}: frames must stack A -> B -> C, got tops {tops:?}"
        );
    }

    // Each frame owns a contiguous rank band, so no two frames interleave.
    let parsed = parse_mermaid(
        "flowchart TB\n\
         subgraph FA[\"one\"]\n A1 --- A2\n end\n\
         subgraph FB[\"two\"]\n B1 --- B2\n end\n\
         A2 --> B2\n",
    )
    .expect("parse failed");
    let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
    let bottom_of_a = ["A1", "A2"]
        .iter()
        .map(|id| {
            let node = layout.nodes.get(*id).expect("node");
            node.y + node.height
        })
        .fold(f32::MIN, f32::max);
    let top_of_b = ["B1", "B2"]
        .iter()
        .map(|id| layout.nodes.get(*id).expect("node").y)
        .fold(f32::MAX, f32::min);
    assert!(
        bottom_of_a < top_of_b,
        "frames must occupy disjoint rank bands, got A bottom {bottom_of_a} vs B top {top_of_b}"
    );

    // Rank bands also have to keep each frame's rows in one column. When the
    // frames share ranks instead, a row is placed as a sibling of the *other*
    // frames' rows and is dragged far across the diagram -- the frames still
    // stack, but their contents scatter.
    let parsed = parse_mermaid(&three_stacked_frames_input()).expect("parse failed");
    let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
    for frame in ["A", "B", "C"] {
        let centres: Vec<f32> = (1..=3)
            .map(|row| {
                let id = format!("{frame}{row}");
                let node = layout.nodes.get(&id).unwrap_or_else(|| panic!("{id}"));
                node.x + node.width * 0.5
            })
            .collect();
        let spread = centres.iter().fold(f32::MIN, |a, b| a.max(*b))
            - centres.iter().fold(f32::MAX, |a, b| a.min(*b));
        let widest = (1..=3)
            .map(|row| layout.nodes[&format!("{frame}{row}")].width)
            .fold(f32::MIN, f32::max);
        assert!(
            spread <= widest * 0.5,
            "frame {frame} rows must stay in one column, centres {centres:?} spread {spread:.1}px for rows at most {widest:.1}px wide"
        );
    }
}

/// An edge between stacked frames must use the free channel beside them rather
/// than climbing back over the source frame.
///
/// When a wide neighbour sits directly below the source and another directly
/// above the target, every opposite-side port pairing is obstructed. The side
/// search used to offer only those, so it picked the least-broken one: the
/// route left `A2` rightward, climbed above the whole first frame, crossed to
/// the far left, then dropped ~475px. Same-flank ports make it a straight drop.
#[test]
fn edges_between_stacked_frames_do_not_backtrack_over_the_source() {
    let parsed = parse_mermaid(&three_stacked_frames_input()).expect("parse failed");
    let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

    for (from, to) in [("A2", "B2"), ("B2", "C2")] {
        let edge = layout
            .edges
            .iter()
            .find(|edge| edge.from == from && edge.to == to)
            .unwrap_or_else(|| panic!("{from}->{to} edge"));

        // In a TB flowchart a forward edge should never climb back up.
        let backtrack: f32 = edge
            .points
            .windows(2)
            .map(|seg| (seg[0].1 - seg[1].1).max(0.0))
            .sum();
        assert!(
            backtrack <= 1.0,
            "{from}->{to} climbs {backtrack:.1}px back up the diagram: {:?}",
            edge.points
        );

        // The old route wandered far wider than the nodes it connects.
        let from_node = layout.nodes.get(from).expect("source node");
        let to_node = layout.nodes.get(to).expect("target node");
        let widest = from_node.width.max(to_node.width);
        let span = edge
            .points
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), p| {
                (lo.min(p.0), hi.max(p.0))
            });
        assert!(
            span.1 - span.0 <= widest,
            "{from}->{to} spans {:.1}px horizontally for nodes only {widest:.1}px wide: {:?}",
            span.1 - span.0,
            edge.points
        );
    }
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

// ── Architecture-beta layout semantics (issues #112, #59) ────────────────

/// Helpers for asserting relative grid placement of architecture services.
fn arch_layout(fixture: &str) -> Layout {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(fixture);
    let (layout, _svg) = render_fixture(&root);
    layout
}

fn assert_left_of(layout: &Layout, left: &str, right: &str) {
    let a = layout.nodes.get(left).unwrap_or_else(|| panic!("{left}"));
    let b = layout.nodes.get(right).unwrap_or_else(|| panic!("{right}"));
    assert!(
        a.x + a.width <= b.x + 1.0,
        "{left} (x={}) should be left of {right} (x={})",
        a.x,
        b.x
    );
}

fn assert_above(layout: &Layout, top: &str, bottom: &str) {
    let a = layout.nodes.get(top).unwrap_or_else(|| panic!("{top}"));
    let b = layout
        .nodes
        .get(bottom)
        .unwrap_or_else(|| panic!("{bottom}"));
    assert!(
        a.y + a.height <= b.y + 1.0,
        "{top} (y={}) should be above {bottom} (y={})",
        a.y,
        b.y
    );
}

fn assert_same_row(layout: &Layout, a_id: &str, b_id: &str) {
    let a = layout.nodes.get(a_id).unwrap_or_else(|| panic!("{a_id}"));
    let b = layout.nodes.get(b_id).unwrap_or_else(|| panic!("{b_id}"));
    let a_cy = a.y + a.height / 2.0;
    let b_cy = b.y + b.height / 2.0;
    assert!(
        (a_cy - b_cy).abs() < 1.0,
        "{a_id} (cy={a_cy}) and {b_id} (cy={b_cy}) should share a row"
    );
}

fn assert_same_column(layout: &Layout, a_id: &str, b_id: &str) {
    let a = layout.nodes.get(a_id).unwrap_or_else(|| panic!("{a_id}"));
    let b = layout.nodes.get(b_id).unwrap_or_else(|| panic!("{b_id}"));
    let a_cx = a.x + a.width / 2.0;
    let b_cx = b.x + b.width / 2.0;
    assert!(
        (a_cx - b_cx).abs() < 1.0,
        "{a_id} (cx={a_cx}) and {b_id} (cx={b_cx}) should share a column"
    );
}

/// Issue #112: `db:L -- R:server` places server one cell to the LEFT of db,
/// `disk1:T -- B:server` places disk1 BELOW server, `disk2:T -- B:db` places
/// disk2 BELOW db (mermaid-js spatial map semantics).
#[test]
fn architecture_ports_drive_grid_placement() {
    let layout = arch_layout("architecture/ports.mmd");
    assert_left_of(&layout, "server", "db");
    assert_above(&layout, "server", "disk1");
    assert_above(&layout, "db", "disk2");
    assert_same_row(&layout, "server", "db");
    assert_same_row(&layout, "disk1", "disk2");
    assert_same_column(&layout, "server", "disk1");
    assert_same_column(&layout, "db", "disk2");

    // The group box must contain all four services.
    let group = layout
        .subgraphs
        .first()
        .expect("api group should be laid out");
    for id in ["db", "disk1", "disk2", "server"] {
        let node = layout.nodes.get(id).unwrap();
        assert!(
            node.x >= group.x
                && node.y >= group.y
                && node.x + node.width <= group.x + group.width
                && node.y + node.height <= group.y + group.height,
            "{id} should be inside the api group box"
        );
    }
}

/// Issue #59: the official junction example must form a 2D grid with the
/// junctions centered between their connected disks/gateways.
#[test]
fn architecture_junctions_form_two_crosses() {
    let layout = arch_layout("architecture/junctions.mmd");
    assert_left_of(&layout, "left_disk", "junctionCenter");
    assert_left_of(&layout, "junctionCenter", "junctionRight");
    assert_above(&layout, "top_disk", "junctionCenter");
    assert_above(&layout, "junctionCenter", "bottom_disk");
    assert_above(&layout, "top_gateway", "junctionRight");
    assert_above(&layout, "junctionRight", "bottom_gateway");
    assert_same_row(&layout, "junctionCenter", "junctionRight");
    assert_same_column(&layout, "top_disk", "bottom_disk");
    assert_same_column(&layout, "top_gateway", "bottom_gateway");
}

/// Port-aware routing: edges must leave from the declared node side, and
/// mixed-axis port pairs get an L-shaped (single-bend) orthogonal path.
#[test]
fn architecture_edges_route_from_declared_ports() {
    let layout = arch_layout("architecture/bends.mmd");
    let node = |id: &str| layout.nodes.get(id).unwrap();
    let edge = |from: &str, to: &str| {
        layout
            .edges
            .iter()
            .find(|e| e.from == from && e.to == to)
            .unwrap_or_else(|| panic!("edge {from}->{to}"))
    };

    // a:R -- L:b leaves a's right side and enters b's left side, straight.
    let ab = edge("a", "b");
    let a = node("a");
    let b = node("b");
    let start = ab.points.first().unwrap();
    let end = ab.points.last().unwrap();
    assert!((start.0 - (a.x + a.width)).abs() < 1.0, "a:R start");
    assert!((end.0 - b.x).abs() < 1.0, "L:b end");
    assert_eq!(ab.points.len(), 2, "aligned R--L edge should be straight");

    // a:B -- T:c leaves a's bottom and enters c's top, straight vertical.
    let ac = edge("a", "c");
    let c = node("c");
    let start = ac.points.first().unwrap();
    let end = ac.points.last().unwrap();
    assert!((start.1 - (a.y + a.height)).abs() < 1.0, "a:B start");
    assert!((end.1 - c.y).abs() < 1.0, "T:c end");

    // c:R -- L:b is a cross-axis placement (c below-left of b): the L/R port
    // pair on offset rows must produce a Z-shaped path with two bends.
    let cb = edge("c", "b");
    let start = cb.points.first().unwrap();
    let end = cb.points.last().unwrap();
    assert!((start.0 - (c.x + c.width)).abs() < 1.0, "c:R start");
    assert!((end.0 - b.x).abs() < 1.0, "L:b end");
    assert_eq!(
        cb.points.len(),
        4,
        "offset R--L edge should be Z-shaped (2 bends), got {:?}",
        cb.points
    );
    // All segments must stay orthogonal.
    for pair in cb.points.windows(2) {
        let dx = (pair[0].0 - pair[1].0).abs();
        let dy = (pair[0].1 - pair[1].1).abs();
        assert!(
            dx < 0.01 || dy < 0.01,
            "architecture edge segments must be orthogonal: {:?}",
            cb.points
        );
    }
}

#[test]
fn state_block_note_renders_multiline_text() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let path = root.join("state/note_block.mmd");
    let (layout, svg) = render_fixture(&path);

    let DiagramData::Graph { state_notes } = &layout.diagram else {
        panic!("state/note_block.mmd: expected graph diagram data");
    };
    assert_eq!(
        state_notes.len(),
        2,
        "expected both block notes (right + left) in layout"
    );

    let right = state_notes
        .iter()
        .find(|note| note.position == mermaid_rs_renderer::StateNotePosition::RightOf)
        .expect("right-of block note missing");
    assert_eq!(right.target, "Comp");
    assert!(
        right.label.lines.len() >= 2,
        "block note body should preserve line breaks, got {:?}",
        right.label.lines
    );

    let left = state_notes
        .iter()
        .find(|note| note.position == mermaid_rs_renderer::StateNotePosition::LeftOf)
        .expect("left-of block note missing");
    assert_eq!(left.target, "Comp");

    // Composite target: the note should sit beside the composite subgraph.
    let comp = layout
        .subgraphs
        .iter()
        .find(|sub| sub.label == "Comp")
        .expect("composite subgraph Comp missing");
    assert!(
        right.x >= comp.x + comp.width - 1.0,
        "right note should be to the right of composite: note.x={} comp right={}",
        right.x,
        comp.x + comp.width
    );
    assert!(
        left.x + left.width <= comp.x + 1.0,
        "left note should be to the left of composite: note right={} comp.x={}",
        left.x + left.width,
        comp.x
    );
    let note_rect = |note: &mermaid_rs_renderer::layout::StateNoteLayout| {
        (note.x, note.y, note.width, note.height)
    };
    for note in state_notes {
        for (id, node) in &layout.nodes {
            if id != &note.target && !node.hidden {
                assert_eq!(
                    rect_overlap_area(note_rect(note), (node.x, node.y, node.width, node.height)),
                    0.0,
                    "state note for {} overlaps node {id}",
                    note.target
                );
            }
        }
        assert!(note.x >= 0.0 && note.y >= 0.0, "note must be within bounds");
        assert!(note.x + note.width <= layout.width);
        assert!(note.y + note.height <= layout.height);
    }

    // Note text is emitted in the SVG.
    assert!(
        svg.contains("Block note body"),
        "note text missing from svg"
    );
    assert!(
        svg.contains("with a second line"),
        "second note line missing from svg"
    );
    assert!(
        svg.contains("Left side note"),
        "left note text missing from svg"
    );
}

#[test]
fn multiple_state_notes_on_one_side_search_outward() {
    let input = "stateDiagram-v2\n  direction LR\n  A --> B\n  note right of A: first\n  note right of A: second\n";
    let parsed = parse_mermaid(input).expect("parse failed");
    let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
    let DiagramData::Graph { state_notes } = &layout.diagram else {
        panic!("expected graph diagram data");
    };
    assert_eq!(state_notes.len(), 2);
    let first = &state_notes[0];
    let second = &state_notes[1];
    assert_eq!(
        rect_overlap_area(
            (first.x, first.y, first.width, first.height),
            (second.x, second.y, second.width, second.height)
        ),
        0.0,
        "notes requested on the same side must not overlap"
    );
    assert!(second.x > first.x, "later right note should move outward");
    assert!(second.x + second.width <= layout.width);
}

/// Conformance contract for nested subgraph layout, derived from mermaid's own
/// rendering of the canonical nested-direction example (docs/flowchart syntax).
///
/// Cluster geometry used to be derived independently in several places, each
/// re-ranking the *flattened* member list from a different origin, so they
/// disagreed. It now comes from one recursive routine that sizes a cluster
/// once, applies its `direction` once, and places it into its parent as a
/// rigid sized block.
#[test]
fn nested_subgraph_layout_contract() {
    let input = r#"flowchart LR
  subgraph TOP
    direction TB
    subgraph B1
        direction RL
        i1 -->f1
    end
    subgraph B2
        direction BT
        i2 -->f2
    end
  end
  A --> TOP --> B
  B1 --> B2
"#;
    let parsed = parse_mermaid(input).expect("parse failed");
    let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

    let raw_sg = |label: &str| {
        layout
            .subgraphs
            .iter()
            .find(|s| s.label == label)
            .unwrap_or_else(|| panic!("subgraph {label}"))
    };
    let sg = |label: &str| {
        let s = raw_sg(label);
        (s.x, s.y, s.x + s.width, s.y + s.height)
    };
    let nd = |id: &str| {
        let n = layout.nodes.get(id).unwrap_or_else(|| panic!("node {id}"));
        (n.x, n.y, n.x + n.width, n.y + n.height)
    };
    let inside = |i: (f32, f32, f32, f32), o: (f32, f32, f32, f32)| {
        i.0 >= o.0 - 0.5 && i.1 >= o.1 - 0.5 && i.2 <= o.2 + 0.5 && i.3 <= o.3 + 0.5
    };
    let cy = |b: (f32, f32, f32, f32)| (b.1 + b.3) * 0.5;

    let (top, b1, b2) = (sg("TOP"), sg("B1"), sg("B2"));
    let (a, b) = (nd("A"), nd("B"));

    // Parent flow is LR: A, then the TOP cluster as a unit, then B.
    assert!(a.2 <= top.0, "A must sit entirely left of TOP");
    assert!(top.2 <= b.0, "TOP must sit entirely left of B");

    // A cluster behaves as one sized node, so its neighbours centre on it.
    assert!((cy(a) - cy(top)).abs() <= 20.0, "A must be centred on TOP");
    assert!((cy(b) - cy(top)).abs() <= 20.0, "B must be centred on TOP");

    // TOP's own `direction TB` orders its children as whole blocks.
    assert!(b1.3 <= b2.1, "TOP direction TB must put B1 above B2");

    // Containment holds at every level.
    assert!(inside(b1, top) && inside(b2, top), "children inside TOP");

    // A parent's title band belongs to the parent: a nested child box starts
    // below it, so the two labels never collide. Mermaid leaves 37.5px here.
    let top_title = raw_sg("TOP").label_block.height;
    assert!(
        b1.1 - top.1 >= top_title,
        "TOP's title band ({:.1}px) must clear B1's box, which starts {:.1}px below TOP",
        top_title,
        b1.1 - top.1
    );

    // Sibling clusters joined by an edge centre on each other along the parent's
    // cross axis, exactly as two connected plain nodes would.
    let cx = |b: (f32, f32, f32, f32)| (b.0 + b.2) * 0.5;
    assert!(
        (cx(b1) - cx(b2)).abs() <= 2.0,
        "B1 and B2 must be centre-aligned in TOP's TB flow, got {:.1} vs {:.1}",
        cx(b1),
        cx(b2)
    );
    assert!(inside(nd("i1"), b1) && inside(nd("f1"), b1), "leaves in B1");
    assert!(inside(nd("i2"), b2) && inside(nd("f2"), b2), "leaves in B2");

    // Innermost directions still apply within their own cluster.
    assert!(nd("f1").2 <= nd("i1").0, "B1 direction RL: f1 left of i1");
    assert!(nd("f2").3 <= nd("i2").1, "B2 direction BT: f2 above i2");

    // An edge between sibling clusters stays in their parent and flows with it.
    let edge = layout
        .edges
        .iter()
        .find(|e| e.from == "B1" && e.to == "B2")
        .expect("B1->B2 edge");
    let (min_x, max_x) = edge
        .points
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), p| {
            (lo.min(p.0), hi.max(p.0))
        });
    assert!(
        min_x >= top.0 - 2.0 && max_x <= top.2 + 2.0,
        "B1->B2 must stay within TOP, got x {min_x:.1}..{max_x:.1} vs TOP {:.1}..{:.1}",
        top.0,
        top.2
    );
    assert!(
        edge.points.windows(2).all(|s| s[1].1 >= s[0].1 - 1.0),
        "B1->B2 must flow downward with TOP's TB direction: {:?}",
        edge.points
    );
}
