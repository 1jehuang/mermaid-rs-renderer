use super::*;

fn svg(body: &str) -> Scene {
    normalize_svg(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100">{body}</svg>"#
    ))
    .unwrap()
}

fn paths(scene: &Scene) -> Vec<&Vec<PathCommand>> {
    scene
        .commands
        .iter()
        .filter_map(|c| match c {
            SceneCommand::FillPath { path, .. } => Some(path),
            _ => None,
        })
        .collect()
}

#[test]
fn transforms_and_viewbox_are_canvas_local() {
    let scene = normalize_svg(r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100" viewBox="-10 -20 100 50"><g transform="translate(5 7)"><g transform="scale(2 3)"><path d="M0 0L10 0L10 10Z"/></g></g></svg>"#).unwrap();
    assert_eq!((scene.width, scene.height), (200.0, 100.0));
    assert_eq!(
        paths(&scene)[0][0],
        PathCommand::MoveTo { x: 30.0, y: 54.0 }
    );
    assert_eq!(
        paths(&scene)[0][2],
        PathCommand::LineTo { x: 70.0, y: 114.0 }
    );
}

#[test]
fn text_is_shaped_outlines_with_transform() {
    let plain = svg(
        r#"<text x="10" y="40" font-family="sans-serif" font-size="20">Office ffi مرحبا</text>"#,
    );
    let moved = svg(
        r#"<g transform="translate(13 17)"><text x="10" y="40" font-family="sans-serif" font-size="20">Office ffi مرحبا</text></g>"#,
    );
    assert!(
        !paths(&plain).is_empty(),
        "install system fonts to run text rendering tests"
    );
    assert!(
        paths(&plain)
            .iter()
            .flat_map(|p| p.iter())
            .any(|c| matches!(c, PathCommand::QuadTo { .. } | PathCommand::CubicTo { .. }))
    );
    let a = paths(&plain);
    let b = paths(&moved);
    assert_eq!(a.len(), b.len());
    for (a, b) in a.iter().zip(b.iter()) {
        assert_eq!(a.len(), b.len());
        for (a, b) in a.iter().zip(b.iter()) {
            if let (PathCommand::MoveTo { x: ax, y: ay }, PathCommand::MoveTo { x: bx, y: by }) =
                (a, b)
            {
                assert!((bx - ax - 13.0).abs() < 0.001);
                assert!((by - ay - 17.0).abs() < 0.001);
            }
        }
    }
}

#[test]
fn strokes_expand_dashes_caps_and_nonuniform_scale() {
    let scene = svg(
        r#"<g transform="scale(2 3)"><path d="M10 10L40 10" fill="none" stroke="red" stroke-width="4" stroke-linecap="square" stroke-dasharray="5 5"/></g>"#,
    );
    let p = paths(&scene);
    assert_eq!(p.len(), 1);
    assert_eq!(
        p[0].iter()
            .filter(|c| matches!(c, PathCommand::Close))
            .count(),
        3
    );
    let xs: Vec<f32> = p[0]
        .iter()
        .filter_map(|c| match c {
            PathCommand::MoveTo { x, .. } | PathCommand::LineTo { x, .. } => Some(*x),
            _ => None,
        })
        .collect();
    assert_eq!(xs.iter().copied().fold(f32::INFINITY, f32::min), 16.0);
    let ys: Vec<f32> = p[0]
        .iter()
        .filter_map(|c| match c {
            PathCommand::MoveTo { y, .. } | PathCommand::LineTo { y, .. } => Some(*y),
            _ => None,
        })
        .collect();
    assert_eq!(ys.iter().copied().fold(f32::INFINITY, f32::min), 24.0);
    assert_eq!(ys.iter().copied().fold(f32::NEG_INFINITY, f32::max), 36.0);
}

#[test]
fn transparency_fill_rule_and_paint_order_survive() {
    let scene = svg(
        r#"<g opacity="0.4"><path d="M1 1H30V30H1Z" fill="red" fill-opacity="0.3" fill-rule="evenodd" stroke="blue" stroke-opacity="0.7" stroke-width="2" paint-order="stroke fill"/></g>"#,
    );
    assert!(
        matches!(scene.commands[0], SceneCommand::PushLayer {opacity,blend_mode:BlendMode::Normal} if (opacity-0.4).abs()<0.001)
    );
    assert!(
        matches!(scene.commands[1], SceneCommand::FillPath {paint:Paint::Solid(Color {b:255,a,..}),..} if (a-0.7).abs()<0.001)
    );
    assert!(
        matches!(scene.commands[2], SceneCommand::FillPath {paint:Paint::Solid(Color {r:255,a,..}),fill_rule:FillRule::EvenOdd,..} if (a-0.3).abs()<0.001)
    );
    assert!(matches!(scene.commands[3], SceneCommand::PopLayer));
    assert_eq!(
        scene.commands.len(),
        4,
        "no implicit background is introduced"
    );
}

#[test]
fn clip_transform_is_canvas_local() {
    let scene = svg(
        r#"<defs><clipPath id="c" transform="translate(3 4)"><rect x="1" y="2" width="10" height="20"/></clipPath></defs><g transform="translate(7 8)" clip-path="url(#c)"><rect width="100" height="100"/></g>"#,
    );
    assert!(
        matches!(&scene.commands[0], SceneCommand::PushClip {path,..} if path[0]==PathCommand::MoveTo{x:11.0,y:14.0})
    );
    assert!(matches!(scene.commands.last(), Some(SceneCommand::PopClip)));
}

#[test]
fn skewed_gradient_preserves_scalar_field_and_stop_alpha() {
    let scene = svg(
        r#"<defs><linearGradient id="g" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="10" y2="0" gradientTransform="matrix(2 0 1 3 5 7)"><stop stop-color="red" stop-opacity="0.5"/><stop offset="1" stop-color="blue"/></linearGradient></defs><path d="M0 0H100V100H0Z" fill="url(#g)" fill-opacity="0.4"/>"#,
    );
    let SceneCommand::FillPath {
        paint: Paint::LinearGradient { start, end, stops },
        ..
    } = &scene.commands[0]
    else {
        panic!()
    };
    assert_eq!(*start, (5.0, 7.0));
    // Both transformed original x=10 points must evaluate to t=1, despite skew.
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    for (x, y) in [(25.0, 7.0), (35.0, 37.0)] {
        let t = ((x - start.0) * dx + (y - start.1) * dy) / (dx * dx + dy * dy);
        assert!((t - 1.0).abs() < 0.0001);
    }
    assert!((stops[0].color.a - 0.2).abs() < 0.001);
    assert!((stops[1].color.a - 0.4).abs() < 0.001);
}

#[test]
fn unsupported_images_fail_instead_of_disappearing_or_rasterizing() {
    assert!(normalize_svg(r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><image href="file:///does-not-exist.png" width="20" height="20"/></svg>"#).unwrap_err().to_string().contains("images"));
}

// This test-only consumer serializes the renderer-neutral commands back to SVG
// and compares the result with the original emitter using the existing PNG
// backend. Production scene construction never invokes a rasterizer.
#[cfg(feature = "png")]
#[test]
fn vector_scene_replays_like_original_svg_for_all_diagrams() {
    use std::fmt::Write;
    fn path_data(path: &[PathCommand]) -> String {
        let mut d = String::new();
        for c in path {
            match c {
                PathCommand::MoveTo { x, y } => write!(d, "M{x} {y}"),
                PathCommand::LineTo { x, y } => write!(d, "L{x} {y}"),
                PathCommand::QuadTo { x1, y1, x, y } => write!(d, "Q{x1} {y1} {x} {y}"),
                PathCommand::CubicTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                } => write!(d, "C{x1} {y1} {x2} {y2} {x} {y}"),
                PathCommand::Close => write!(d, "Z"),
            }
            .unwrap();
        }
        d
    }
    fn rule_name(rule: &FillRule) -> &str {
        match rule {
            FillRule::NonZero => "nonzero",
            FillRule::EvenOdd => "evenodd",
        }
    }
    fn replay(scene: &Scene) -> String {
        let mut s = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}">"#,
            scene.width, scene.height
        );
        for (id, c) in scene.commands.iter().enumerate() {
            match c {
            SceneCommand::FillPath{path,paint,fill_rule}=>{
                let fill=match paint {
                    Paint::Solid(c)=>format!(r#"fill="rgb({},{},{})" fill-opacity="{}""#,c.r,c.g,c.b,c.a),
                    Paint::LinearGradient{start,end,stops}=>{
                        write!(s,r#"<defs><linearGradient id="g{id}" gradientUnits="userSpaceOnUse" x1="{}" y1="{}" x2="{}" y2="{}">"#,start.0,start.1,end.0,end.1).unwrap();
                        for stop in stops {let c=stop.color;write!(s,r#"<stop offset="{}" stop-color="rgb({},{},{})" stop-opacity="{}"/>"#,stop.offset,c.r,c.g,c.b,c.a).unwrap();}
                        s.push_str("</linearGradient></defs>");format!(r#"fill="url(#g{id})""#)
                    }
                };
                write!(s,r#"<path d="{}" {} fill-rule="{}"/>"#,path_data(path),fill,rule_name(fill_rule)).unwrap();
            },
            SceneCommand::PushClip{path,fill_rule}=>write!(s,r#"<defs><clipPath id="c{id}"><path d="{}" clip-rule="{}"/></clipPath></defs><g clip-path="url(#c{id})">"#,path_data(path),rule_name(fill_rule)).unwrap(),
            SceneCommand::PopClip|SceneCommand::PopLayer=>s.push_str("</g>"),
            SceneCommand::PushLayer{opacity,blend_mode}=>write!(s,r#"<g opacity="{opacity}" style="isolation:isolate;mix-blend-mode:{}">"#,match blend_mode {BlendMode::Normal=>"normal",BlendMode::Multiply=>"multiply"}).unwrap(),
        }
        }
        s.push_str("</svg>");
        s
    }
    fn raster(svg: &str) -> Vec<u8> {
        let tree = usvg::Tree::from_str(
            svg,
            &usvg::Options {
                fontdb: Arc::clone(&FONTS),
                ..Default::default()
            },
        )
        .unwrap();
        let scale = (700.0 / tree.size().width().max(tree.size().height())).min(1.0);
        let mut pix = resvg::tiny_skia::Pixmap::new(
            (tree.size().width() * scale).ceil() as u32,
            (tree.size().height() * scale).ceil() as u32,
        )
        .unwrap();
        resvg::render(
            &tree,
            Transform::from_scale(scale, scale),
            &mut pix.as_mut(),
        );
        pix.data().to_vec()
    }
    for name in [
        "flowchart",
        "sequence",
        "class",
        "state",
        "er",
        "pie",
        "xychart",
        "quadrant",
        "gantt",
        "timeline",
        "journey",
        "mindmap",
        "gitgraph",
        "requirement",
        "c4",
        "sankey",
        "zenuml",
        "block",
        "packet",
        "kanban",
        "architecture",
        "radar",
        "treemap",
    ] {
        let input = std::fs::read_to_string(format!(
            "{}/benches/fixtures/{name}_medium.mmd",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let original = crate::render(&input).unwrap();
        let scene = normalize_svg(&original).unwrap();
        let a = raster(&original);
        let b = raster(&replay(&scene));
        assert_eq!(a.len(), b.len());
        let differences: Vec<u8> = a.iter().zip(&b).map(|(a, b)| a.abs_diff(*b)).collect();
        let mean = differences.iter().map(|d| *d as f64).sum::<f64>() / a.len() as f64;
        let severe = differences.iter().filter(|d| **d > 32).count() as f64 / a.len() as f64;
        assert!(
            mean < 1.5 && severe < 0.003,
            "{name}: replay mean channel error {mean:.4}, severe fraction {severe:.4}"
        );
    }
}

#[test]
fn viewbox_letterboxing_is_baked_into_paths() {
    let scene=normalize_svg(r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100" viewBox="0 0 100 100"><path d="M0 0H100V100Z"/></svg>"#).unwrap();
    assert_eq!(paths(&scene)[0][0], PathCommand::MoveTo { x: 50.0, y: 0.0 });
}

#[test]
fn dash_gap_is_empty_not_an_error() {
    let scene = svg(
        r#"<path d="M0 20H1" stroke="black" fill="none" stroke-width="2" stroke-dasharray="2 20" stroke-dashoffset="10"/>"#,
    );
    assert!(scene.commands.is_empty());
}

#[test]
fn round_caps_and_join_styles_are_expanded() {
    let round = svg(
        r#"<path d="M10 10L40 10L40 40" stroke="black" fill="none" stroke-width="10" stroke-linecap="round" stroke-linejoin="round"/>"#,
    );
    let bevel = svg(
        r#"<path d="M10 10L40 10L40 40" stroke="black" fill="none" stroke-width="10" stroke-linecap="butt" stroke-linejoin="bevel"/>"#,
    );
    assert_ne!(round.commands, bevel.commands);
    assert!(
        paths(&round)[0]
            .iter()
            .any(|c| matches!(c, PathCommand::QuadTo { .. } | PathCommand::CubicTo { .. }))
    );
}

#[test]
fn c4_person_icons_are_vectors_in_shared_svg_output() {
    let svg =
        crate::render("C4Context\n Person(user, \"User\")\n Person_Ext(guest, \"Guest\")").unwrap();
    assert_eq!(svg.matches("c4-person-icon").count(), 2);
    assert!(!svg.contains("<image"));
    assert!(!normalize_svg(&svg).unwrap().commands.is_empty());
}
