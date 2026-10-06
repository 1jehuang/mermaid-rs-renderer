use mermaid_rs_renderer::{LayoutConfig, Theme, compute_layout, parse_mermaid_strict};

#[test]
fn sequence_loops_enclose_self_messages_and_keep_following_notes_outside() {
    let input = "sequenceDiagram\n participant A\n loop Retry request\n A->>A: Compare document contents\n Note over A: Inside\n end\n Note over A: Outside";
    let parsed = parse_mermaid_strict(input).unwrap();
    assert_eq!(parsed.graph.sequence_frames[0].note_range, 0..1);
    for theme in [Theme::modern(), Theme::mermaid_default()] {
        let layout = compute_layout(&parsed.graph, &theme, &LayoutConfig::default());
        let mermaid_rs_renderer::layout::DiagramData::Sequence(sequence) = &layout.diagram else {
            panic!("sequence layout expected")
        };
        let frame = &sequence.frames[0];
        let inside = &sequence.notes[0];
        let outside = &sequence.notes[1];
        let self_bottom = layout.edges[0]
            .points
            .iter()
            .map(|p| p.1)
            .fold(0.0, f32::max);
        assert!(inside.y > self_bottom, "note overlaps self-message");
        assert!(frame.y + frame.height >= inside.y + inside.height);
        assert!(
            outside.y > frame.y + frame.height,
            "following note included in frame"
        );
        for (x, y) in &layout.edges[0].points {
            assert!(*x >= frame.x && *x <= frame.x + frame.width);
            assert!(*y >= frame.y && *y <= frame.y + frame.height);
        }
        let condition = &frame.section_labels[0];
        let tab_right = frame.label_box.0 + frame.label_box.2;
        assert!(condition.x - condition.text.width / 2.0 > tab_right);
        assert!(condition.x + condition.text.width / 2.0 < frame.x + frame.width);
        assert!(sequence.lifelines[0].y2 > outside.y + outside.height);
    }
}

#[test]
fn sequence_frame_note_ranges_preserve_nested_boundary_ownership() {
    let parsed = parse_mermaid_strict("sequenceDiagram\n participant A\n Note over A: Before\n loop Outer\n Note over A: Outer note\n loop Inner\n A->>A: Work\n Note over A: Inner note\n end\n Note over A: Outer end\n end\n Note over A: After").unwrap();
    assert_eq!(parsed.graph.sequence_frames[0].note_range, 2..3);
    assert_eq!(parsed.graph.sequence_frames[1].note_range, 1..4);
}
