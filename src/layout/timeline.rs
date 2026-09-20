use super::*;

struct MeasuredTimelineEvent {
    time: TextBlock,
    events: Vec<TextBlock>,
    width: f32,
    height: f32,
}

pub(super) fn compute_timeline_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> Layout {
    let data = &graph.timeline;
    let direction = data.direction.unwrap_or_else(|| {
        Direction::from_timeline_token(&config.timeline.direction).unwrap_or(Direction::LeftRight)
    });
    let font_size = theme.font_size;
    let event_font_size = theme.font_size * 0.9;
    let padding = 30.0_f32;
    let min_event_width = 120.0_f32;
    let max_event_width = if direction == Direction::TopDown {
        360.0_f32
    } else {
        min_event_width
    };
    let min_event_height = if direction == Direction::TopDown {
        0.0_f32
    } else {
        80.0_f32
    };
    let empty_event_height = 80.0_f32;
    let event_spacing = 40.0_f32;
    let event_text_width = (max_event_width - 16.0).max(1.0);
    let event_bottom_padding = 8.0_f32;
    let title_height = if data.title.is_some() { 40.0 } else { 0.0 };

    let title = data.title.as_ref().map(|t| measure_label(t, theme, config));
    let measured_sections: Vec<TextBlock> = data
        .sections
        .iter()
        .map(|section| measure_label(section, theme, config))
        .collect();
    let section_gap = 20.0_f32;
    let section_height = measured_sections
        .iter()
        .map(|label| label.height)
        .fold(0.0, f32::max);
    let section_row_height = if measured_sections.is_empty() {
        0.0
    } else {
        section_height + section_gap
    };

    let mut max_event_height = min_event_height;
    let mut measured_events = Vec::with_capacity(data.events.len());
    for event in &data.events {
        let time_block = measure_label_with_font_size(
            &event.time,
            font_size,
            config,
            false,
            theme.font_family.as_str(),
        );
        let event_blocks: Vec<TextBlock> = event
            .events
            .iter()
            .map(|e| {
                measure_label_with_max_width(
                    e,
                    event_font_size,
                    event_text_width,
                    config,
                    true,
                    theme.font_family.as_str(),
                )
            })
            .collect();

        let time_extra =
            time_block.lines.len().saturating_sub(1) as f32 * font_size * config.label_line_height;
        let description_line_count = event_blocks
            .iter()
            .map(|block| block.lines.len())
            .sum::<usize>();
        let event_width = if direction == Direction::TopDown {
            let time_width = time_block
                .lines
                .iter()
                .map(|line| {
                    text_width(
                        line,
                        font_size,
                        theme.font_family.as_str(),
                        config.fast_text_metrics,
                        config.metrics.as_deref(),
                    )
                })
                .fold(0.0, f32::max);
            let content_width = event_blocks
                .iter()
                .flat_map(|block| block.lines.iter())
                .map(|line| {
                    text_width(
                        line,
                        event_font_size,
                        theme.font_family.as_str(),
                        config.fast_text_metrics,
                        config.metrics.as_deref(),
                    )
                })
                .fold(time_width, f32::max);
            (content_width + 16.0).clamp(min_event_width, max_event_width)
        } else {
            min_event_width
        };
        let event_height = if description_line_count == 0 {
            20.0 + time_extra + font_size * 0.25 + event_bottom_padding
        } else {
            let event_line_height = event_font_size * config.label_line_height;
            40.0 + time_extra
                + description_line_count.saturating_sub(1) as f32 * event_line_height
                + event_font_size * 0.25
                + event_bottom_padding
        }
        .max(min_event_height);
        max_event_height = max_event_height.max(event_height);

        measured_events.push(MeasuredTimelineEvent {
            time: time_block,
            events: event_blocks,
            width: event_width,
            height: event_height,
        });
    }

    let (events, width, height, line_y, line_start_x, line_end_x, line_start_y, line_end_y) =
        if direction == Direction::TopDown {
            let content_top = padding + title_height + section_row_height + 30.0;
            let line_x = padding + 20.0;
            let event_x = line_x + 40.0;
            let content_height = if measured_events.is_empty() {
                empty_event_height
            } else {
                measured_events
                    .iter()
                    .map(|event| event.height)
                    .sum::<f32>()
                    + (measured_events.len() - 1) as f32 * event_spacing
            };
            let content_width = measured_events
                .iter()
                .map(|event| event.width)
                .fold(min_event_width, f32::max);
            let section_content_width = measured_sections
                .iter()
                .map(|label| label.width)
                .sum::<f32>()
                + measured_sections.len().saturating_sub(1) as f32 * section_gap;
            let width =
                (event_x + content_width + padding).max(padding * 2.0 + section_content_width);
            let height = content_top + content_height + padding;
            let mut y = content_top;
            let mut events = Vec::with_capacity(measured_events.len());

            for measured in measured_events {
                let circle_y = y + measured.height / 2.0;
                events.push(TimelineEventLayout {
                    time: measured.time,
                    events: measured.events,
                    x: event_x,
                    y,
                    width: measured.width,
                    height: measured.height,
                    circle_y,
                });
                y += measured.height + event_spacing;
            }

            (
                events,
                width,
                height,
                content_top,
                line_x,
                line_x,
                content_top,
                content_top + content_height,
            )
        } else {
            let line_y = padding + title_height + section_row_height + 60.0;
            let num_events = measured_events.len().max(1);
            let total_events_width =
                num_events as f32 * min_event_width + (num_events - 1) as f32 * event_spacing;
            let section_content_width = measured_sections
                .iter()
                .map(|label| label.width)
                .sum::<f32>()
                + measured_sections.len().saturating_sub(1) as f32 * section_gap;
            let width =
                (padding * 2.0 + total_events_width).max(padding * 2.0 + section_content_width);
            let height =
                padding * 2.0 + title_height + section_row_height + max_event_height + 100.0;
            let mut events = Vec::with_capacity(measured_events.len());

            for (i, measured) in measured_events.into_iter().enumerate() {
                let x = padding + i as f32 * (min_event_width + event_spacing);
                let y = line_y + 30.0;
                events.push(TimelineEventLayout {
                    time: measured.time,
                    events: measured.events,
                    x,
                    y,
                    width: measured.width,
                    height: measured.height,
                    circle_y: line_y,
                });
            }

            (
                events,
                width,
                height,
                line_y,
                padding,
                width - padding,
                line_y,
                line_y,
            )
        };

    let section_total_width = measured_sections
        .iter()
        .map(|label| label.width)
        .sum::<f32>()
        + measured_sections.len().saturating_sub(1) as f32 * section_gap;
    let mut section_x = ((width - section_total_width) / 2.0).max(padding);
    let sections: Vec<TimelineSectionLayout> = measured_sections
        .into_iter()
        .map(|label| {
            let label_width = label.width;
            TimelineSectionLayout {
                label,
                x: {
                    let x = section_x;
                    section_x += label_width + section_gap;
                    x
                },
                y: padding + title_height,
                width: label_width,
                height: section_height,
            }
        })
        .collect();

    let mut nodes = BTreeMap::new();
    nodes.insert(
        "__timeline_metrics_content".to_string(),
        NodeLayout {
            id: "__timeline_metrics_content".to_string(),
            x: padding,
            y: padding,
            width: (width - padding * 2.0).max(1.0),
            height: (height - padding * 2.0).max(1.0),
            label: TextBlock {
                lines: vec![String::new()],
                width: 0.0,
                height: 0.0,
            },
            shape: crate::ir::NodeShape::Rectangle,
            style: crate::ir::NodeStyle::default(),
            link: None,
            anchor_subgraph: None,
            hidden: false,
            icon: None,
        },
    );

    Layout {
        kind: graph.kind,
        nodes,
        edges: Vec::new(),
        subgraphs: Vec::new(),
        diagram: DiagramData::Timeline(TimelineLayout {
            title,
            title_y: padding + font_size,
            events,
            sections,
            direction,
            line_y,
            line_start_x,
            line_end_x,
            line_start_y,
            line_end_y,
            width,
            height,
        }),
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{DiagramKind, TimelineEvent};

    fn timeline_layout(direction: Direction) -> Layout {
        let mut graph = Graph::new();
        graph.kind = DiagramKind::Timeline;
        graph.timeline.title = Some("A timeline title".to_string());
        graph.timeline.direction = Some(direction);
        graph.timeline.sections = vec![
            "A very long measured section heading".to_string(),
            "Another very long measured section heading".to_string(),
        ];
        graph.timeline.events = vec![
            TimelineEvent {
                time: "2024".to_string(),
                events: vec!["First event".to_string()],
                section: Some(graph.timeline.sections[0].clone()),
            },
            TimelineEvent {
                time: "2025".to_string(),
                events: vec!["Second event with a longer description".to_string()],
                section: Some(graph.timeline.sections[1].clone()),
            },
        ];

        compute_timeline_layout(&graph, &Theme::mermaid_default(), &LayoutConfig::default())
    }

    fn assert_timeline_bboxes_within_layout(layout: &Layout) {
        let DiagramData::Timeline(timeline) = &layout.diagram else {
            panic!("expected timeline layout");
        };

        for section in &timeline.sections {
            assert!(section.x >= 0.0);
            assert!(section.y >= 0.0);
            assert!(section.x + section.width <= layout.width);
            assert!(section.y + section.height <= layout.height);
        }
        for event in &timeline.events {
            assert!(event.x >= 0.0);
            assert!(event.y >= 0.0);
            assert!(event.x + event.width <= layout.width);
            assert!(event.y + event.height <= layout.height);
        }
    }

    #[test]
    fn left_right_section_and_event_bboxes_fit_layout() {
        assert_timeline_bboxes_within_layout(&timeline_layout(Direction::LeftRight));
    }

    #[test]
    fn top_down_section_and_event_bboxes_fit_layout() {
        assert_timeline_bboxes_within_layout(&timeline_layout(Direction::TopDown));
    }
}
