use mermaid_rs_renderer::{
    DiagramKind, LayoutConfig, NodeShape, Theme, compute_layout, parse_mermaid, render_svg,
};

fn render(
    input: &str,
) -> (
    mermaid_rs_renderer::Graph,
    mermaid_rs_renderer::Layout,
    String,
) {
    let parsed = parse_mermaid(input).expect("diagram should parse");
    let theme = Theme::modern();
    let config = LayoutConfig::default();
    let layout = compute_layout(&parsed.graph, &theme, &config);
    let svg = render_svg(&layout, &theme, &config);
    (parsed.graph, layout, svg)
}

#[test]
fn architecture_iconify_icons_render_as_symbols_not_broken_question_marks() {
    let input = r#"architecture-beta
    group api(logos:aws-lambda)[API]

    service db(logos:aws-aurora)[Database] in api
    service disk1(logos:aws-glacier)[Storage] in api
    service disk2(logos:aws-s3)[Storage] in api
    service server(logos:aws-ec2)[Server] in api

    db:L -- R:server
    disk1:T -- B:server
    disk2:T -- B:db
"#;

    let (graph, layout, svg) = render(input);
    assert_eq!(graph.kind, DiagramKind::Architecture);
    assert_eq!(graph.nodes.len(), 4);
    assert_eq!(layout.edges.len(), 3);
    assert!(
        !svg.contains(">?</text>") && !svg.contains(">?</tspan>"),
        "registered/Iconify icons should not render as broken question marks"
    );
    assert!(
        svg.contains('λ'),
        "lambda icon should get a symbolic fallback"
    );
    // Port directions must drive grid placement (issue #112): server sits to
    // the left of db (db:L -- R:server), disk1 below server, disk2 below db.
    let node = |id: &str| layout.nodes.get(id).expect(id);
    let (db, server, disk1, disk2) = (node("db"), node("server"), node("disk1"), node("disk2"));
    assert!(
        server.x + server.width <= db.x + 1.0,
        "server should be left of db"
    );
    assert!(
        disk1.y >= server.y + server.height - 1.0,
        "disk1 should be below server"
    );
    assert!(
        disk2.y >= db.y + db.height - 1.0,
        "disk2 should be below db"
    );
}

#[test]
fn architecture_group_edge_modifiers_do_not_create_phantom_nodes() {
    let input = r#"architecture-beta
    group groupOne(cloud)[One]
    group groupTwo(cloud)[Two]
    service server(server)[Server] in groupOne
    service subnet(database)[Subnet] in groupTwo
    server{group}:B --> T:subnet{group}
"#;

    let (graph, layout, svg) = render(input);
    assert!(graph.nodes.contains_key("server"));
    assert!(graph.nodes.contains_key("subnet"));
    assert!(
        graph.nodes.keys().all(|id| !id.contains("{group}")),
        "{{group}} edge modifiers must not become phantom service ids"
    );
    assert_eq!(graph.edges[0].from, "server");
    assert_eq!(graph.edges[0].to, "subnet");
    assert_eq!(layout.nodes.len(), 2);
    assert!(svg.contains("marker-end"));
}

#[test]
fn architecture_junctions_are_compact_routing_points() {
    let input = r#"architecture-beta
    service left_disk(disk)[Disk]
    service top_gateway(internet)[Gateway]
    junction junctionCenter
    junction junctionRight

    left_disk:R -- L:junctionCenter
    junctionCenter:R -- L:junctionRight
    top_gateway:B -- T:junctionRight
"#;

    let (graph, layout, svg) = render(input);
    let center = graph.nodes.get("junctionCenter").expect("junction parsed");
    assert_eq!(center.shape, NodeShape::Circle);
    assert_eq!(center.icon.as_deref(), Some("junction"));

    let center_layout = layout
        .nodes
        .get("junctionCenter")
        .expect("junction laid out");
    assert!(
        center_layout.width <= 24.0 && center_layout.height <= 24.0,
        "junctions should be compact routing dots, got {}x{}",
        center_layout.width,
        center_layout.height
    );
    assert!(svg.contains("<circle"));
    assert!(
        !svg.contains(">junctionCenter<"),
        "junction ids should not render as service labels"
    );
}

#[test]
fn display_math_labels_are_rendered_readably_in_svg_text() {
    let input = r#"graph LR
      A["$$x^2$$"] -->|"$$\sqrt{x+3}$$"| B("$$\frac{1}{2}$$")
      A -->|"$$\overbrace{a+b+c}^{\text{note}}$$"| C("$$\pi r^2$$")
"#;

    let (_graph, _layout, svg) = render(input);
    assert!(svg.contains("x²"));
    assert!(svg.contains("√"));
    assert!(svg.contains("(1)/(2)"));
    assert!(svg.contains("π r²"));
    assert!(
        !svg.contains("$$") && !svg.contains("\\sqrt") && !svg.contains("\\frac"),
        "raw TeX delimiters/commands should not leak into visible SVG text"
    );
}

/// Issue #69: with the default theme the derived pie palette used
/// tertiary == primary, so "Dogs" and "Rats" got the same fill.
#[test]
fn pie_slices_get_distinct_colors_with_default_theme() {
    let input = r#"pie
"Dogs" : 386
"Cats" : 85.9
"Rats" : 15
"#;
    let parsed = parse_mermaid(input).expect("diagram should parse");
    for theme in [Theme::mermaid_default(), Theme::modern()] {
        let config = LayoutConfig::default();
        let layout = compute_layout(&parsed.graph, &theme, &config);
        let mermaid_rs_renderer::layout::DiagramData::Pie(pie) = &layout.diagram else {
            panic!("expected pie layout");
        };
        let colors: Vec<&str> = pie.slices.iter().map(|s| s.color.as_str()).collect();
        let mut deduped = colors.clone();
        deduped.sort_unstable();
        deduped.dedup();
        assert_eq!(
            deduped.len(),
            colors.len(),
            "pie slices should get distinct palette colors, got {colors:?}"
        );
    }
}

/// Issue #69: the small-slice outside label ("Rats") overlapped the legend
/// because layout and render used different formulas for the label extent.
#[test]
fn pie_outside_label_background_does_not_overlap_legend() {
    let input = r#"pie
"Dogs" : 386
"Cats" : 85.9
"Rats" : 15
"#;
    let parsed = parse_mermaid(input).expect("diagram should parse");
    let theme = Theme::mermaid_default();
    let config = LayoutConfig::default();
    let layout = compute_layout(&parsed.graph, &theme, &config);
    let svg = render_svg(&layout, &theme, &config);

    // Legend marker rects are 14x14; find the leftmost legend x.
    let mut legend_left = f32::INFINITY;
    let mut label_rect_right = 0.0f32;
    for rect in svg.split("<rect ").skip(1) {
        let attr = |name: &str| -> Option<f32> {
            let key = format!("{name}=\"");
            let start = rect.find(&key)? + key.len();
            let end = start + rect[start..].find('"')?;
            rect[start..end].parse::<f32>().ok()
        };
        let (Some(x), Some(w)) = (attr("x"), attr("width")) else {
            continue;
        };
        if (w - 14.0).abs() < 0.01 {
            legend_left = legend_left.min(x);
        } else if rect.contains("rx=\"2\"") {
            label_rect_right = label_rect_right.max(x + w);
        }
    }
    assert!(legend_left.is_finite(), "legend rects should render");
    assert!(
        label_rect_right > 0.0,
        "outside label background should render"
    );
    assert!(
        label_rect_right <= legend_left,
        "outside pie label background (right edge {label_rect_right}) must not overlap the legend (left edge {legend_left})"
    );
}

/// Issue #112: wide CJK pie titles clipped at both sides of the viewbox
/// because layout ignored the measured title width.
#[test]
fn pie_cjk_title_fits_inside_viewbox() {
    let input = "pie\n    title 这是一个非常非常非常非常长的标题文字测试标题文字测试\n    \"甲\" : 40\n    \"乙\" : 60\n";
    let parsed = parse_mermaid(input).expect("diagram should parse");
    let theme = Theme::mermaid_default();
    let config = LayoutConfig::default();
    let layout = compute_layout(&parsed.graph, &theme, &config);
    let mermaid_rs_renderer::layout::DiagramData::Pie(pie) = &layout.diagram else {
        panic!("expected pie layout");
    };
    let title = pie.title.as_ref().expect("title should be laid out");
    let left = title.x - title.text.width / 2.0;
    let right = title.x + title.text.width / 2.0;
    assert!(
        left >= 0.0,
        "title should not clip on the left: left edge {left}"
    );
    assert!(
        right <= layout.width,
        "title should not clip on the right: right edge {right}, layout width {}",
        layout.width
    );
}

/// Issue #49: `mindmap.edgeColor` config should force all mindmap edge
/// strokes to one color, independent of the section palette.
#[test]
fn mindmap_edge_color_config_overrides_section_palette() {
    let input = r#"mindmap
  root((Root))
    A
      A1
    B
      B1
"#;
    let parsed = parse_mermaid(input).expect("diagram should parse");
    let theme = Theme::mermaid_default();

    let default_config = LayoutConfig::default();
    let default_layout = compute_layout(&parsed.graph, &theme, &default_config);
    let default_strokes: Vec<String> = default_layout
        .edges
        .iter()
        .filter_map(|edge| edge.override_style.stroke.clone())
        .collect();
    assert!(
        default_strokes.iter().any(|s| s != "#ff00aa"),
        "default mindmap edges should use palette colors"
    );

    let mut config = LayoutConfig::default();
    config.mindmap.edge_color = Some("#ff00aa".to_string());
    let layout = compute_layout(&parsed.graph, &theme, &config);
    assert!(!layout.edges.is_empty(), "mindmap should produce edges");
    for edge in &layout.edges {
        assert_eq!(
            edge.override_style.stroke.as_deref(),
            Some("#ff00aa"),
            "every mindmap edge stroke should use the configured edgeColor"
        );
    }
    let svg = render_svg(&layout, &theme, &config);
    assert!(
        svg.contains("stroke=\"#ff00aa\""),
        "configured mindmap edgeColor should appear in the SVG output"
    );
}

#[test]
fn state_final_marker_renders_ring_plus_filled_dot_bullseye() {
    let input = r#"stateDiagram-v2
    [*] --> Working
    Working --> [*]
"#;
    let (_, layout, svg) = render(input);
    let end_node = layout
        .nodes
        .values()
        .find(|n| n.id.starts_with("__end_"))
        .expect("state diagram should have an end marker node");
    assert_eq!(end_node.shape, NodeShape::DoubleCircle);
    let cx = end_node.x + end_node.width / 2.0;
    let cy = end_node.y + end_node.height / 2.0;
    let at_center: Vec<&str> = svg
        .split("<circle ")
        .skip(1)
        .filter(|c| {
            c.contains(&format!("cx=\"{:.2}\"", cx)) && c.contains(&format!("cy=\"{:.2}\"", cy))
        })
        .collect();
    assert!(
        at_center.len() >= 2,
        "final-state marker should be a bullseye with an outer ring and an inner dot, got {} circles",
        at_center.len()
    );
    let theme = Theme::modern();
    let ring = at_center
        .iter()
        .find(|c| c.contains(&format!("stroke=\"{}\"", theme.primary_border_color)))
        .expect("bullseye should have a ring stroked with the border color");
    assert!(
        ring.contains(&format!("fill=\"{}\"", theme.background)),
        "bullseye ring should have a hollow (background) fill"
    );
    let dot = at_center
        .iter()
        .find(|c| c.contains(&format!("fill=\"{}\"", theme.primary_border_color)))
        .expect("bullseye should have an inner dot filled with the border color");
    assert!(
        dot.contains("stroke=\"none\""),
        "inner dot should be a plain filled circle"
    );
}

#[test]
fn state_final_marker_bullseye_inside_composite_state() {
    let input = r#"stateDiagram-v2
    state Comp {
        [*] --> Inner
        Inner --> [*]
    }
"#;
    let (_, layout, svg) = render(input);
    let end_nodes: Vec<_> = layout
        .nodes
        .values()
        .filter(|n| n.id.starts_with("__end_"))
        .collect();
    assert!(
        !end_nodes.is_empty(),
        "composite state should contain an end marker node"
    );
    for end_node in end_nodes {
        let cx = end_node.x + end_node.width / 2.0;
        let cy = end_node.y + end_node.height / 2.0;
        let at_center = svg
            .split("<circle ")
            .skip(1)
            .filter(|c| {
                c.contains(&format!("cx=\"{:.2}\"", cx)) && c.contains(&format!("cy=\"{:.2}\"", cy))
            })
            .count();
        assert!(
            at_center >= 2,
            "composite end marker should render as a bullseye (ring + dot), got {at_center} circles"
        );
    }
}

#[test]
fn labeled_double_circle_keeps_concentric_outline() {
    let input = r#"flowchart TD
    A(((Label))) --> B
"#;
    let (_, layout, svg) = render(input);
    let node = layout.nodes.get("A").expect("double circle node A");
    assert_eq!(node.shape, NodeShape::DoubleCircle);
    let cx = node.x + node.width / 2.0;
    let cy = node.y + node.height / 2.0;
    let at_center: Vec<&str> = svg
        .split("<circle ")
        .skip(1)
        .filter(|c| {
            c.contains(&format!("cx=\"{:.2}\"", cx)) && c.contains(&format!("cy=\"{:.2}\"", cy))
        })
        .collect();
    assert_eq!(
        at_center.len(),
        2,
        "labeled double circle should keep exactly two concentric circles"
    );
    assert!(
        at_center.iter().any(|c| c.contains("fill=\"none\"")),
        "labeled double circle inner ring should stay unfilled so the label remains readable"
    );
}

#[test]
fn radar_negative_values_clamp_to_center_matching_upstream_min_default() {
    // Upstream mermaid radar clips values to [min, max] with min defaulting to 0
    // (packages/mermaid/src/diagrams/radar/db.ts `min: 0`, renderer.ts
    // relativeRadius clippedValue = min(max(value, minValue), maxValue)).
    // Our renderer clamps via value.max(0.0) in parse_series
    // (src/render.rs:~2477); negative entries must land at the chart center
    // (radius 0) with no panic.
    let input = r#"radar-beta
  axis A, B, C
  curve Neg {-3, 5, -1}
  curve Pos {2, 4, 6}
"#;
    let (graph, _layout, svg) = render(input);
    assert_eq!(graph.kind, DiagramKind::Radar);
    let curve_paths: Vec<&str> = svg
        .split("<path d=\"")
        .skip(1)
        .filter(|chunk| chunk.contains("fill-opacity=\"0.5\""))
        .map(|chunk| chunk.split('"').next().unwrap())
        .collect();
    assert_eq!(curve_paths.len(), 2, "both curves should render");
    // max_value across the chart is 6 and MAX_RADIUS is 300, so B (value 5)
    // sits at r=250 => (250*cos30, 250*sin30) = (216.506, 125). A and C clamp
    // to the center.
    let neg = curve_paths[0];
    assert!(
        neg.starts_with("M-0.000,-0.000") || neg.starts_with("M0.000,0.000"),
        "negative A value should clamp to center, got {neg}"
    );
    assert!(
        neg.contains("L216.506,125.000"),
        "positive B value should scale normally, got {neg}"
    );
    for path in &curve_paths {
        for point in path
            .trim_start_matches('M')
            .trim_end_matches(" Z")
            .split(" L")
        {
            let (x, y) = point.split_once(',').expect("point format x,y");
            let (x, y): (f32, f32) = (x.parse().unwrap(), y.parse().unwrap());
            let r = (x * x + y * y).sqrt();
            assert!(
                r <= 300.0 + 0.01,
                "curve point ({x},{y}) escapes the radar radius"
            );
        }
    }
}

#[test]
fn radar_all_negative_values_render_degenerate_center_polygon_without_panic() {
    let input = r#"radar-beta
  axis A, B, C
  curve AllNeg {-3, -5, -1}
"#;
    let (graph, _layout, svg) = render(input);
    assert_eq!(graph.kind, DiagramKind::Radar);
    let path = svg
        .split("<path d=\"")
        .skip(1)
        .find(|chunk| chunk.contains("fill-opacity=\"0.5\""))
        .expect("curve should still render");
    let d = path.split('"').next().unwrap();
    for point in d.trim_start_matches('M').trim_end_matches(" Z").split(" L") {
        let (x, y) = point.split_once(',').expect("point format x,y");
        let (x, y): (f32, f32) = (x.parse().unwrap(), y.parse().unwrap());
        assert!(
            x.abs() < 0.001 && y.abs() < 0.001,
            "all-negative curve should collapse to the center, got ({x},{y})"
        );
        assert!(x.is_finite() && y.is_finite(), "coordinates must be finite");
    }
}

/// Radar `title` directive was parsed then silently discarded, and render_radar
/// emitted an empty `<text></text>` placeholder. The title must appear in the
/// SVG output and sit above the top axis label without colliding with it.
#[test]
fn radar_title_renders_above_top_axis_label() {
    let input = "radar-beta\n  title Skill Assessment\n  axis A, B, C\n  curve Alpha {1,2,3}\n";
    let (graph, _layout, svg) = render(input);
    assert_eq!(graph.kind, DiagramKind::Radar);
    assert_eq!(graph.radar.title.as_deref(), Some("Skill Assessment"));
    assert!(
        svg.contains(">Skill Assessment</text>"),
        "radar title text should be rendered in the SVG output"
    );
    // No empty text placeholder should remain.
    assert!(
        !svg.contains("></text><text") || !svg.contains("<text x=\"0\" y=\"-3"),
        "no empty placeholder title text expected"
    );

    // The title hangs downward from its y, the top axis label is centered on its y.
    // Title bottom (y + font_size) must stay above the top axis label's top edge.
    let title_y: f32 = svg
        .split(">Skill Assessment</text>")
        .next()
        .unwrap()
        .rsplit("<text x=\"0\" y=\"")
        .next()
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .parse()
        .expect("title y coordinate");
    let axis_label_y: f32 = svg
        .split(">A</text>")
        .next()
        .unwrap()
        .rsplit(" y=\"")
        .next()
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .parse()
        .expect("top axis label y coordinate");
    let title_font_size = 16.0; // Theme::modern().font_size
    let axis_font_size = 12.0;
    assert!(
        title_y + title_font_size < axis_label_y - axis_font_size / 2.0,
        "radar title (bottom {}) must not collide with top axis label (top {})",
        title_y + title_font_size,
        axis_label_y - axis_font_size / 2.0
    );
}

/// A radar diagram without a title must not emit any empty title text element.
#[test]
fn radar_without_title_emits_no_empty_text_placeholder() {
    let input = "radar-beta\n  axis A, B, C\n  curve Alpha {1,2,3}\n";
    let (graph, _layout, svg) = render(input);
    assert_eq!(graph.kind, DiagramKind::Radar);
    assert!(graph.radar.title.is_none());
    assert!(
        !svg.contains("></text>") || !svg.contains("y=\"-350"),
        "no empty title placeholder expected when there is no title"
    );
}

/// Quoted radar titles should have their quotes stripped and XML escaped.
#[test]
fn radar_title_quoted_and_escaped() {
    let input = "radar-beta\n  title \"Q1 <Goals> & Metrics\"\n  axis A, B\n  curve C {1,2}\n";
    let (graph, _layout, svg) = render(input);
    assert_eq!(graph.radar.title.as_deref(), Some("Q1 <Goals> & Metrics"));
    assert!(
        svg.contains("Q1 &lt;Goals&gt; &amp; Metrics"),
        "radar title should be XML-escaped in SVG output"
    );
}

#[test]
fn radar_curve_with_fewer_values_than_axes_pads_missing_axes_at_center() {
    // 5 axes, second curve provides only 3 values. Missing values must
    // default to 0 (center of the chart) with no panic and one polygon
    // point per axis (src/render.rs parse_series axis-fill unwrap_or(0.0)).
    let input = r#"radar-beta
  axis A, B, C, D, E
  curve Full {5, 4, 3, 2, 1}
  curve Short {2, 3, 4}
"#;
    let (graph, _layout, svg) = render(input);
    assert_eq!(graph.kind, DiagramKind::Radar);

    // Extract the radar curve polygons (paths with fill-opacity 0.5).
    let curve_paths: Vec<Vec<(f32, f32)>> = svg
        .split("<path d=\"")
        .skip(1)
        .filter(|chunk| chunk.contains("fill-opacity=\"0.5\""))
        .map(|chunk| {
            let d = chunk.split('"').next().expect("path d attribute");
            d.trim_start_matches('M')
                .trim_end_matches(" Z")
                .split(" L")
                .map(|pt| {
                    let (x, y) = pt.split_once(',').expect("point should be x,y");
                    (
                        x.trim().parse::<f32>().expect("x coord"),
                        y.trim().parse::<f32>().expect("y coord"),
                    )
                })
                .collect()
        })
        .collect();
    assert_eq!(curve_paths.len(), 2, "both curves should render a polygon");

    let axis_count = 5usize;
    for (idx, points) in curve_paths.iter().enumerate() {
        assert_eq!(
            points.len(),
            axis_count,
            "curve {idx} polygon must have one point per axis"
        );
    }

    // Geometry: max value 5 maps to MAX_RADIUS 300, so scale = 60.
    let scale = 300.0f32 / 5.0;
    let step = 2.0 * std::f32::consts::PI / axis_count as f32;
    let start = -std::f32::consts::PI / 2.0;
    let expected_short = [2.0f32, 3.0, 4.0, 0.0, 0.0];
    let short = &curve_paths[1];
    for (idx, value) in expected_short.iter().enumerate() {
        let angle = start + step * idx as f32;
        let (ex, ey) = (value * scale * angle.cos(), value * scale * angle.sin());
        let (x, y) = short[idx];
        assert!(
            (x - ex).abs() < 0.01 && (y - ey).abs() < 0.01,
            "short curve point {idx} should be at ({ex:.3},{ey:.3}) got ({x:.3},{y:.3})"
        );
    }
    // Missing axes (D, E) sit exactly at the center radius.
    for (idx, (x, y)) in short.iter().copied().enumerate().take(5).skip(3) {
        assert!(
            x.abs() < 0.001 && y.abs() < 0.001,
            "missing axis {idx} should default to value 0 at the center, got ({x},{y})"
        );
    }
}

#[test]
fn radar_curve_with_more_values_than_axes_ignores_extras() {
    // Extra values beyond the axis list get numeric-only label lines that
    // fail the axis lookup, so they must not distort the polygon.
    let input = r#"radar-beta
  axis A, B, C
  curve Over {1, 2, 3, 4, 5}
"#;
    let (graph, _layout, svg) = render(input);
    assert_eq!(graph.kind, DiagramKind::Radar);
    let polygon = svg
        .split("<path d=\"")
        .skip(1)
        .find(|chunk| chunk.contains("fill-opacity=\"0.5\""))
        .expect("curve polygon should render");
    let d = polygon.split('"').next().unwrap();
    let point_count = d
        .trim_start_matches('M')
        .trim_end_matches(" Z")
        .split(" L")
        .count();
    assert_eq!(
        point_count, 3,
        "curve polygon must have exactly one point per axis even with extra values"
    );
}

#[test]
fn state_concurrent_regions_render_dividers_and_per_region_starts() {
    // stateDiagram-v2 concurrency ('--') defects: missing region divider,
    // per-region [*] merged into one fork bar, and that bar sitting exactly
    // on the composite's title separator line.
    let input = r#"stateDiagram-v2
  state Active {
    [*] --> One
    --
    [*] --> Two
  }
"#;
    let (graph, layout, svg) = render(input);
    assert_eq!(graph.kind, DiagramKind::State);

    // (2) One independent start pseudostate per region, no fork bar.
    let start_ids: Vec<&String> = graph
        .nodes
        .keys()
        .filter(|id| id.starts_with("__start_"))
        .collect();
    assert_eq!(
        start_ids.len(),
        2,
        "each concurrent region keeps its own [*] start state: {start_ids:?}"
    );
    assert!(
        graph
            .nodes
            .values()
            .all(|node| node.shape != NodeShape::ForkJoin),
        "per-region [*] must not merge into a fork/join bar"
    );

    // (1) A divider line separates the two regions.
    assert!(
        svg.contains("stroke-dasharray=\"4 4\""),
        "concurrent regions should be separated by a divider line"
    );

    // (3) Children stay inside the composite body with an inset below the
    // title separator (header band is label + padding; give it clearance).
    let active = layout
        .subgraphs
        .iter()
        .find(|sub| sub.label == "Active")
        .expect("composite subgraph");
    let header_bottom = active.y + active.label_block.height;
    for node in layout.nodes.values() {
        if node.hidden {
            continue;
        }
        assert!(
            node.y > header_bottom + 4.0,
            "child {} at y={} must clear the composite title band ending at {}",
            node.id,
            node.y,
            header_bottom
        );
    }

    // Three regions produce two dividers.
    let input3 = r#"stateDiagram-v2
  state Active {
    [*] --> One
    --
    [*] --> Two
    --
    [*] --> Three
  }
"#;
    let (graph3, _layout3, svg3) = render(input3);
    let start_count = graph3
        .nodes
        .keys()
        .filter(|id| id.starts_with("__start_"))
        .count();
    assert_eq!(start_count, 3, "three regions keep three [*] markers");
    let divider_count = svg3.matches("stroke-dasharray=\"4 4\"").count();
    assert_eq!(
        divider_count, 2,
        "three concurrent regions need exactly two dividers"
    );
}

/// No edge label's text may sit on a node or be crossed by another edge, and
/// every label lies inside the canvas. A state diagram routes a back-edge down
/// the outer margin; its label was clamped into the pre-label canvas, onto the
/// node beside it, and fixed there. An ER label clear of its node was pulled
/// back onto it by the attachment cap. The larger state diagram, with two
/// back-edges down the margin, must stay clean as well.
#[test]
fn state_and_er_edge_labels_stay_clear_of_nodes() {
    let cases = [
        "stateDiagram-v2\n    [*] --> Clean\n    Clean --> Dirty : keystroke\n    Dirty --> Saving : pause\n    Saving --> Clean : written\n    Saving --> Dirty : keystroke\n",
        "erDiagram\n    NOTE ||--o{ VERSION : keeps\n    NOTE ||--o| DRAFT : has\n    NOTE }o--o{ TAG : carries\n",
        // A larger state diagram with two back-edges down the margin.
        "stateDiagram-v2\n    [*] --> CLOSED\n    CLOSED --> OPEN : 5 consecutive failures\n    OPEN --> HALF_OPEN : probe interval elapsed\n    HALF_OPEN --> CLOSED : probe succeeds\n    HALF_OPEN --> OPEN : probe fails (increased backoff)\n\n    CLOSED : All DB calls pass through\n    CLOSED : Counting consecutive failures\n    OPEN : DB calls skipped (sleep for probe interval)\n    OPEN : No writes attempted\n    HALF_OPEN : One probe call allowed through\n",
    ];
    for input in cases {
        let (_, layout, _) = render(input);
        for (i, edge) in layout.edges.iter().enumerate() {
            let (Some(label), Some((cx, cy))) = (&edge.label, edge.label_anchor) else {
                continue;
            };
            let text = label.lines.join(" ");
            let (x0, y0) = (cx - label.width / 2.0, cy - label.height / 2.0);
            let (x1, y1) = (x0 + label.width, y0 + label.height);
            assert!(
                x0 >= -0.5 && y0 >= -0.5 && x1 <= layout.width + 0.5 && y1 <= layout.height + 0.5,
                "label {text:?} at ({x0:.1},{y0:.1})-({x1:.1},{y1:.1}) outside the {}x{} canvas",
                layout.width,
                layout.height
            );
            for (id, node) in &layout.nodes {
                let overlap = x0 < node.x + node.width
                    && node.x < x1
                    && y0 < node.y + node.height
                    && node.y < y1;
                assert!(
                    !overlap,
                    "label {text:?} at ({x0:.1},{y0:.1})-({x1:.1},{y1:.1}) overlaps node {id} at ({:.1},{:.1}) {}x{}",
                    node.x, node.y, node.width, node.height
                );
            }
            // Strictly inside the text, sampled along each segment of every
            // other edge.
            let inside =
                |(x, y): (f32, f32)| x > x0 + 1.0 && x < x1 - 1.0 && y > y0 + 1.0 && y < y1 - 1.0;
            for (j, other) in layout.edges.iter().enumerate() {
                let crosses = j != i
                    && other.points.windows(2).any(|s| {
                        (0..=64).any(|k| {
                            let t = k as f32 / 64.0;
                            inside((
                                s[0].0 + (s[1].0 - s[0].0) * t,
                                s[0].1 + (s[1].1 - s[0].1) * t,
                            ))
                        })
                    });
                assert!(
                    !crosses,
                    "label {text:?} is crossed by edge {}->{}",
                    other.from, other.to
                );
            }
        }
    }
}
