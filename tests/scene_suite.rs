#![cfg(feature = "scene")]

use mermaid_rs_renderer::{
    Paint, PathCommand, RenderOptions, Scene, SceneCommand, measure, render_scene,
};

fn validate(scene: &Scene) {
    assert!(scene.width.is_finite() && scene.width > 0.0);
    assert!(scene.height.is_finite() && scene.height > 0.0);
    let mut stack = Vec::new();
    let mut fills = 0;
    for command in &scene.commands {
        let path = match command {
            SceneCommand::FillPath { path, paint, .. } => {
                fills += 1;
                match paint {
                    Paint::Solid(c) => assert!((0.0..=1.0).contains(&c.a)),
                    Paint::LinearGradient { start, end, stops } => {
                        assert!(
                            [start.0, start.1, end.0, end.1]
                                .iter()
                                .all(|v| v.is_finite())
                        );
                        assert!(!stops.is_empty());
                        for s in stops {
                            assert!((0.0..=1.0).contains(&s.color.a));
                        }
                    }
                }
                path
            }
            SceneCommand::PushClip { path, .. } => {
                stack.push("clip");
                path
            }
            SceneCommand::PopClip => {
                assert_eq!(stack.pop(), Some("clip"));
                continue;
            }
            SceneCommand::PushLayer { opacity, .. } => {
                assert!((0.0..=1.0).contains(opacity));
                stack.push("layer");
                continue;
            }
            SceneCommand::PopLayer => {
                assert_eq!(stack.pop(), Some("layer"));
                continue;
            }
        };
        assert!(!path.is_empty());
        for c in path {
            let values = match c {
                PathCommand::MoveTo { x, y } | PathCommand::LineTo { x, y } => vec![*x, *y],
                PathCommand::QuadTo { x1, y1, x, y } => vec![*x1, *y1, *x, *y],
                PathCommand::CubicTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                } => vec![*x1, *y1, *x2, *y2, *x, *y],
                PathCommand::Close => vec![],
            };
            assert!(values.iter().all(|v| v.is_finite()));
        }
    }
    assert!(stack.is_empty());
    assert!(fills > 0);
}

#[test]
fn all_23_diagram_kinds_produce_vector_scenes() {
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
        let scene = render_scene(&input, RenderOptions::default())
            .unwrap_or_else(|e| panic!("{name}: {e:#}"));
        validate(&scene);
        let dimensions = measure(&input, RenderOptions::default()).unwrap();
        assert!(
            (scene.width - dimensions.width).abs() < 0.02,
            "{name} canvas width"
        );
        assert!(
            (scene.height - dimensions.height).abs() < 0.02,
            "{name} canvas height"
        );
    }
}

#[test]
fn td_complete_canvas_and_last_node_survive() {
    let input = "flowchart TD\n A[Top label] --> B[Middle label]\n B --> C[Bottom label]\n C --> D[Complete final label]";
    let options = RenderOptions::default().with_rank_spacing(150.0);
    let scene = render_scene(input, options.clone()).unwrap();
    validate(&scene);
    let dimensions = measure(input, options).unwrap();
    assert!((scene.height - dimensions.height).abs() < 0.02);
    assert!(scene.height > scene.width);
    let mut last_content_y = 0.0f32;
    for command in &scene.commands {
        if let SceneCommand::FillPath { path, .. } = command {
            // Ignore the background rectangle, require actual lower-diagram geometry.
            if path.len() > 5 {
                for c in path {
                    let y = match c {
                        PathCommand::MoveTo { y, .. }
                        | PathCommand::LineTo { y, .. }
                        | PathCommand::QuadTo { y, .. }
                        | PathCommand::CubicTo { y, .. } => *y,
                        PathCommand::Close => 0.0,
                    };
                    last_content_y = last_content_y.max(y);
                }
            }
        }
    }
    assert!(
        last_content_y > scene.height * 0.8,
        "bottom node/label must not disappear"
    );
}

#[test]
fn parse_errors_and_init_options_match_svg_api() {
    let error = render_scene("not a diagram", RenderOptions::default()).unwrap_err();
    assert!(
        error
            .downcast_ref::<mermaid_rs_renderer::ParseError>()
            .is_some()
    );
    let plain = render_scene("flowchart TD; A-->B", RenderOptions::default()).unwrap();
    let dark = render_scene(
        "%%{init: {'theme':'dark'}}%%\nflowchart TD; A-->B",
        RenderOptions::default(),
    )
    .unwrap();
    assert_ne!(plain.commands, dark.commands);
}
