use crate::config::LayoutConfig;
#[cfg(feature = "png")]
use crate::config::RenderConfig;
use crate::layout::{EdgeLayout, Layout, TextBlock};
use crate::theme::Theme;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

pub fn render_svg(layout: &Layout, theme: &Theme, config: &LayoutConfig) -> String {
    let mut svg = String::new();
    let width = layout.width.max(200.0);
    let height = layout.height.max(200.0);
    let is_sequence = !layout.sequence_footboxes.is_empty();
    let is_state = layout.kind == crate::ir::DiagramKind::State;
    let is_class = layout.kind == crate::ir::DiagramKind::Class;
    let is_pie = layout.kind == crate::ir::DiagramKind::Pie;
    let has_links = layout
        .nodes
        .values()
        .any(|node| node.link.is_some())
        || layout.sequence_footboxes.iter().any(|node| node.link.is_some());

    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\"{} width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">",
        if has_links {
            " xmlns:xlink=\"http://www.w3.org/1999/xlink\""
        } else {
            ""
        }
    ));

    svg.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        theme.background
    ));

    let mut colors = Vec::new();
    colors.push(theme.line_color.clone());
    for edge in &layout.edges {
        if let Some(color) = &edge.override_style.stroke
            && !colors.contains(color)
        {
            colors.push(color.clone());
        }
    }
    let mut color_ids: HashMap<String, usize> = HashMap::new();
    for (idx, color) in colors.iter().enumerate() {
        color_ids.insert(color.clone(), idx);
    }

    svg.push_str("<defs>");
    for color in &colors {
        let idx = color_ids.get(color).copied().unwrap_or(0);
        svg.push_str(&format!(
            "<marker id=\"arrow-{idx}\" viewBox=\"0 0 10 10\" refX=\"5\" refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto\"><path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"{}\"/></marker>",
            color
        ));
        svg.push_str(&format!(
            "<marker id=\"arrow-start-{idx}\" viewBox=\"0 0 10 10\" refX=\"4.5\" refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto\"><path d=\"M 0 5 L 10 10 L 10 0 z\" fill=\"{}\"/></marker>",
            color
        ));
        if is_sequence {
            svg.push_str(&format!(
                "<marker id=\"arrow-seq-{idx}\" viewBox=\"-1 0 12 10\" refX=\"7.9\" refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"12\" markerHeight=\"12\" orient=\"auto-start-reverse\"><path d=\"M -1 0 L 10 5 L 0 10 z\" fill=\"{}\" stroke=\"{}\"/></marker>",
                color,
                color
            ));
            svg.push_str(&format!(
                "<marker id=\"arrow-start-seq-{idx}\" viewBox=\"-1 0 12 10\" refX=\"2.1\" refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"12\" markerHeight=\"12\" orient=\"auto\"><path d=\"M 11 0 L 0 5 L 11 10 z\" fill=\"{}\" stroke=\"{}\"/></marker>",
                color,
                color
            ));
        }
        if is_state {
            svg.push_str(&format!(
                "<marker id=\"arrow-state-{idx}\" viewBox=\"0 0 20 14\" refX=\"19\" refY=\"7\" markerUnits=\"userSpaceOnUse\" markerWidth=\"20\" markerHeight=\"14\" orient=\"auto\"><path d=\"M 19 7 L 9 13 L 14 7 L 9 1 Z\" fill=\"{}\" stroke=\"{}\"/></marker>",
                color, color
            ));
        }
        if is_class {
            svg.push_str(&format!(
                "<marker id=\"arrow-class-open-{idx}\" viewBox=\"0 0 20 14\" refX=\"1\" refY=\"7\" markerUnits=\"userSpaceOnUse\" markerWidth=\"20\" markerHeight=\"14\" orient=\"auto\"><path d=\"M 1 7 L 18 13 V 1 Z\" fill=\"none\" stroke=\"{}\"/></marker>",
                color
            ));
            svg.push_str(&format!(
                "<marker id=\"arrow-class-open-start-{idx}\" viewBox=\"0 0 20 14\" refX=\"18\" refY=\"7\" markerUnits=\"userSpaceOnUse\" markerWidth=\"20\" markerHeight=\"14\" orient=\"auto\"><path d=\"M 1 7 L 18 13 V 1 Z\" fill=\"none\" stroke=\"{}\"/></marker>",
                color
            ));
            svg.push_str(&format!(
                "<marker id=\"arrow-class-dep-{idx}\" viewBox=\"0 0 20 14\" refX=\"13\" refY=\"7\" markerUnits=\"userSpaceOnUse\" markerWidth=\"20\" markerHeight=\"14\" orient=\"auto\"><path d=\"M 18 7 L 9 13 L 14 7 L 9 1 Z\" fill=\"{}\" stroke=\"{}\"/></marker>",
                color, color
            ));
            svg.push_str(&format!(
                "<marker id=\"arrow-class-dep-start-{idx}\" viewBox=\"0 0 20 14\" refX=\"6\" refY=\"7\" markerUnits=\"userSpaceOnUse\" markerWidth=\"20\" markerHeight=\"14\" orient=\"auto\"><path d=\"M 5 7 L 9 13 L 1 7 L 9 1 Z\" fill=\"{}\" stroke=\"{}\"/></marker>",
                color, color
            ));
        }
    }
    svg.push_str("</defs>");

    if is_pie {
        svg.push_str(&render_pie(layout, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    for subgraph in &layout.subgraphs {
        let label_empty = subgraph.label.trim().is_empty();
        if is_state {
            let sub_fill = subgraph.style.fill.as_ref().unwrap_or(&theme.primary_color);
            let sub_stroke = subgraph
                .style
                .stroke
                .as_ref()
                .unwrap_or(&theme.primary_border_color);
            let sub_stroke_width = subgraph.style.stroke_width.unwrap_or(1.0);
            let invisible = label_empty
                && sub_fill.as_str() == "none"
                && sub_stroke.as_str() == "none"
                && sub_stroke_width <= 0.0;
            if invisible {
                continue;
            }
            svg.push_str(&format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"5\" ry=\"5\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>",
                subgraph.x,
                subgraph.y,
                subgraph.width,
                subgraph.height,
                sub_fill,
                sub_stroke,
                sub_stroke_width
            ));
            let header_h = (theme.font_size * 1.4).max(subgraph.label_block.height + 4.0);
            let inner_y = subgraph.y + header_h;
            let inner_h = (subgraph.height - header_h).max(0.0);
            svg.push_str(&format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"none\"/>",
                subgraph.x,
                inner_y,
                subgraph.width,
                inner_h,
                theme.background
            ));
            if !label_empty {
                let label_x = subgraph.x + 12.0;
                let label_y = subgraph.y + header_h / 2.0;
                svg.push_str(&text_block_svg_anchor(
                    label_x,
                    label_y,
                    &subgraph.label_block,
                    theme,
                    config,
                    "start",
                    subgraph.style.text_color.as_deref(),
                ));
            }
        } else {
            let sub_fill = subgraph
                .style
                .fill
                .as_ref()
                .unwrap_or(&theme.cluster_background);
            let sub_stroke = subgraph
                .style
                .stroke
                .as_ref()
                .unwrap_or(&theme.cluster_border);
            let sub_dash = subgraph
                .style
                .stroke_dasharray
                .as_ref()
                .map(|value| format!(" stroke-dasharray=\"{}\"", value))
                .unwrap_or_default();
            let sub_stroke_width = subgraph.style.stroke_width.unwrap_or(1.0);
            let invisible = label_empty
                && sub_fill.as_str() == "none"
                && sub_stroke.as_str() == "none"
                && sub_stroke_width <= 0.0;
            if !invisible {
                svg.push_str(&format!(
                    "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"10\" ry=\"10\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{} />",
                    subgraph.x,
                    subgraph.y,
                    subgraph.width,
                    subgraph.height,
                    sub_fill,
                    sub_stroke,
                    sub_stroke_width,
                    sub_dash
                ));
            }
            if !label_empty {
                let label_x = subgraph.x + subgraph.width / 2.0;
                let label_y = subgraph.y + 12.0 + subgraph.label_block.height / 2.0;
                let label_color = subgraph
                    .style
                    .text_color
                    .as_ref()
                    .unwrap_or(&theme.primary_text_color);
                svg.push_str(&text_block_svg(
                    label_x,
                    label_y,
                    &subgraph.label_block,
                    theme,
                    config,
                    false,
                    Some(label_color),
                ));
            }
        }
    }

    if is_sequence {
        for seq_box in &layout.sequence_boxes {
            let stroke = theme.primary_border_color.as_str();
            let fill = seq_box
                .color
                .as_deref()
                .unwrap_or("none");
            let mut fill_attr = format!("fill=\"{}\"", fill);
            if seq_box.color.is_some() && fill != "none" {
                fill_attr.push_str(" fill-opacity=\"0.12\"");
            }
            svg.push_str(&format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" {fill_attr} stroke=\"{}\" stroke-width=\"1.2\"/>",
                seq_box.x, seq_box.y, seq_box.width, seq_box.height, stroke
            ));
            if let Some(label) = seq_box.label.as_ref() {
                let pad_x = theme.font_size * 0.8;
                let pad_y = theme.font_size * 0.9;
                let label_x = seq_box.x + pad_x;
                let label_y = seq_box.y + pad_y + label.height / 2.0;
                svg.push_str(&text_block_svg_anchor(
                    label_x,
                    label_y,
                    label,
                    theme,
                    config,
                    "start",
                    Some(theme.primary_text_color.as_str()),
                ));
            }
        }
    }

    for frame in &layout.sequence_frames {
        let stroke = theme.primary_border_color.as_str();
        svg.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2.0\" stroke-dasharray=\"2 2\"/>",
            frame.x, frame.y, frame.width, frame.height, stroke
        ));
        for divider_y in &frame.dividers {
            svg.push_str(&format!(
                "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"2.0\" stroke-dasharray=\"3 3\"/>",
                frame.x,
                divider_y,
                frame.x + frame.width,
                divider_y,
                stroke
            ));
        }
        let (box_x, box_y, box_w, box_h) = frame.label_box;
        let notch_x = box_x + box_w * 0.8;
        let notch_y = box_y + box_h;
        let mid_y = box_y + box_h * 0.65;
        svg.push_str(&format!(
            "<polygon points=\"{box_x:.2},{box_y:.2} {end_x:.2},{box_y:.2} {end_x:.2},{mid_y:.2} {notch_x:.2},{notch_y:.2} {box_x:.2},{notch_y:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.1\"/>",
            theme.primary_color,
            stroke,
            end_x = box_x + box_w,
            mid_y = mid_y,
            notch_x = notch_x,
            notch_y = notch_y
        ));
        svg.push_str(&text_block_svg(
            frame.label.x,
            frame.label.y,
            &frame.label.text,
            theme,
            config,
            false,
            Some(theme.primary_text_color.as_str()),
        ));
        for label in &frame.section_labels {
            svg.push_str(&text_block_svg(
                label.x,
                label.y,
                &label.text,
                theme,
                config,
                false,
                None,
            ));
        }
    }

    for lifeline in &layout.lifelines {
        svg.push_str(&format!(
            "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
            lifeline.x,
            lifeline.y1,
            lifeline.x,
            lifeline.y2,
            theme.sequence_actor_line
        ));
    }

    for activation in &layout.sequence_activations {
        svg.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            activation.x,
            activation.y,
            activation.width,
            activation.height,
            theme.sequence_activation_fill,
            theme.sequence_activation_border
        ));
    }

    for note in &layout.sequence_notes {
        let fill = theme.sequence_note_fill.as_str();
        let stroke = theme.sequence_note_border.as_str();
        let fold = (theme.font_size * 0.8)
            .max(8.0)
            .min(note.width.min(note.height) * 0.3);
        let x = note.x;
        let y = note.y;
        let x2 = note.x + note.width;
        let y2 = note.y + note.height;
        let fold_x = x2 - fold;
        let fold_y = y + fold;
        svg.push_str(&format!(
            "<path d=\"M {x:.2} {y:.2} L {fold_x:.2} {y:.2} L {x2:.2} {fold_y:.2} L {x2:.2} {y2:.2} L {x:.2} {y2:.2} Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.1\"/>"
        ));
        svg.push_str(&format!(
            "<polyline points=\"{fold_x:.2},{y:.2} {fold_x:.2},{fold_y:.2} {x2:.2},{fold_y:.2}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.0\"/>"
        ));
        let center_x = note.x + note.width / 2.0;
        let center_y = note.y + note.height / 2.0;
        svg.push_str(&text_block_svg(
            center_x,
            center_y,
            &note.label,
            theme,
            config,
            false,
            Some(theme.primary_text_color.as_str()),
        ));
    }

    if is_state {
        for note in &layout.state_notes {
            let fill = theme.sequence_note_fill.as_str();
            let stroke = theme.sequence_note_border.as_str();
            let fold = (theme.font_size * 0.8)
                .max(8.0)
                .min(note.width.min(note.height) * 0.3);
            let x = note.x;
            let y = note.y;
            let x2 = note.x + note.width;
            let y2 = note.y + note.height;
            let fold_x = x2 - fold;
            let fold_y = y + fold;
            svg.push_str(&format!(
                "<path d=\"M {x:.2} {y:.2} L {fold_x:.2} {y:.2} L {x2:.2} {fold_y:.2} L {x2:.2} {y2:.2} L {x:.2} {y2:.2} Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.1\"/>"
            ));
            svg.push_str(&format!(
                "<polyline points=\"{fold_x:.2},{y:.2} {fold_x:.2},{fold_y:.2} {x2:.2},{fold_y:.2}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.0\"/>"
            ));
            let center_x = note.x + note.width / 2.0;
            let center_y = note.y + note.height / 2.0;
            svg.push_str(&text_block_svg(
                center_x,
                center_y,
                &note.label,
                theme,
                config,
                false,
                Some(theme.primary_text_color.as_str()),
            ));
        }
    }

    if is_sequence {
        for edge in &layout.edges {
            let d = points_to_path(&edge.points);
            let mut stroke = theme.line_color.clone();
            if let Some(color) = &edge.override_style.stroke {
                stroke = color.clone();
            }
            let marker_id = color_ids.get(&stroke).copied().unwrap_or(0);
            let marker_end = if edge.arrow_end {
                format!("marker-end=\"url(#arrow-seq-{marker_id})\"")
            } else {
                String::new()
            };
            let marker_start = if edge.arrow_start {
                format!("marker-start=\"url(#arrow-start-seq-{marker_id})\"")
            } else {
                String::new()
            };

            let mut dash = String::new();
            if edge.style == crate::ir::EdgeStyle::Dotted {
                dash = "stroke-dasharray=\"3 3\"".to_string();
            }
            if let Some(dash_override) = &edge.override_style.dasharray {
                dash = format!("stroke-dasharray=\"{}\"", dash_override);
            }
            let stroke_width = edge.override_style.stroke_width.unwrap_or(2.0);
            svg.push_str(&format!(
                "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" {} {} {} />",
                d, stroke, stroke_width, marker_end, marker_start, dash
            ));

            if let Some(point) = edge.points.first().copied()
                && let Some(decoration) = edge.start_decoration
            {
                let angle = edge_endpoint_angle(&edge.points, true);
                svg.push_str(&edge_decoration_svg(
                    point,
                    angle,
                    decoration,
                    &stroke,
                    stroke_width,
                    true,
                ));
            }
            if let Some(point) = edge.points.last().copied()
                && let Some(decoration) = edge.end_decoration
            {
                let angle = edge_endpoint_angle(&edge.points, false);
                svg.push_str(&edge_decoration_svg(
                    point,
                    angle,
                    decoration,
                    &stroke,
                    stroke_width,
                    false,
                ));
            }

            if let Some(label) = edge.label.as_ref() {
                let start = edge.points.first().copied().unwrap_or((0.0, 0.0));
                let end = edge.points.last().copied().unwrap_or(start);
                let mid_x = (start.0 + end.0) / 2.0;
                let line_y = start.1;
                let label_y = line_y - theme.font_size * 1.95;
                let label_text = label.lines.join("\n");
                svg.push_str(&text_line_svg(
                    mid_x,
                    label_y,
                    label_text.trim(),
                    theme,
                    edge.override_style
                        .label_color
                        .as_deref()
                        .unwrap_or(theme.primary_text_color.as_str()),
                    "middle",
                ));
            }

            let end_label_offset = (theme.font_size * 0.6).max(8.0);
            let label_color = edge
                .override_style
                .label_color
                .as_deref()
                .unwrap_or(theme.primary_text_color.as_str());
            if let Some(label) = edge.start_label.as_ref()
                && let Some((x, y)) = edge_endpoint_label_position(edge, true, end_label_offset)
            {
                svg.push_str(&text_block_svg(
                    x, y, label, theme, config, false, Some(label_color),
                ));
            }
            if let Some(label) = edge.end_label.as_ref()
                && let Some((x, y)) = edge_endpoint_label_position(edge, false, end_label_offset)
            {
                svg.push_str(&text_block_svg(
                    x, y, label, theme, config, false, Some(label_color),
                ));
            }
        }

        for number in &layout.sequence_numbers {
            let r = (theme.font_size * 0.45).max(6.0);
            svg.push_str(&format!(
                "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                number.x,
                number.y,
                r,
                theme.sequence_activation_fill,
                theme.sequence_activation_border
            ));
            let label = number.value.to_string();
            svg.push_str(&text_line_svg(
                number.x,
                number.y + theme.font_size * 0.35,
                label.as_str(),
                theme,
                theme.primary_text_color.as_str(),
                "middle",
            ));
        }

        for number in &layout.sequence_numbers {
            let r = (theme.font_size * 0.45).max(6.0);
            svg.push_str(&format!(
                "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                number.x,
                number.y,
                r,
                theme.sequence_activation_fill,
                theme.sequence_activation_border
            ));
            let label = number.value.to_string();
            svg.push_str(&text_line_svg(
                number.x,
                number.y + theme.font_size * 0.35,
                label.as_str(),
                theme,
                theme.primary_text_color.as_str(),
                "middle",
            ));
        }
    } else {
        let label_positions =
            compute_edge_label_positions(&layout.edges, &layout.nodes, &layout.subgraphs);

        let base_edge_width = match layout.kind {
            crate::ir::DiagramKind::Class
            | crate::ir::DiagramKind::State
            | crate::ir::DiagramKind::Er => 1.0,
            _ => 2.0,
        };
        for (idx, edge) in layout.edges.iter().enumerate() {
            let d = points_to_path(&edge.points);
            let mut stroke = theme.line_color.clone();
            let (mut dash, mut stroke_width) = match edge.style {
                crate::ir::EdgeStyle::Solid => (String::new(), base_edge_width),
                crate::ir::EdgeStyle::Dotted => {
                    ("stroke-dasharray=\"2\"".to_string(), base_edge_width)
                }
                crate::ir::EdgeStyle::Thick => (String::new(), 3.5),
            };

            if let Some(color) = &edge.override_style.stroke {
                stroke = color.clone();
            }
            let marker_id = color_ids.get(&stroke).copied().unwrap_or(0);
            let marker_end = if edge.arrow_end {
                match layout.kind {
                    crate::ir::DiagramKind::State => {
                        format!("marker-end=\"url(#arrow-state-{marker_id})\"")
                    }
                    crate::ir::DiagramKind::Class => match edge.arrow_end_kind {
                        Some(crate::ir::EdgeArrowhead::OpenTriangle) => {
                            format!("marker-end=\"url(#arrow-class-open-{marker_id})\"")
                        }
                        Some(crate::ir::EdgeArrowhead::ClassDependency) => {
                            format!("marker-end=\"url(#arrow-class-dep-{marker_id})\"")
                        }
                        None => format!("marker-end=\"url(#arrow-{marker_id})\""),
                    },
                    _ => format!("marker-end=\"url(#arrow-{marker_id})\""),
                }
            } else {
                String::new()
            };
            let marker_start = if edge.arrow_start {
                match layout.kind {
                    crate::ir::DiagramKind::State => {
                        format!("marker-start=\"url(#arrow-state-{marker_id})\"")
                    }
                    crate::ir::DiagramKind::Class => match edge.arrow_start_kind {
                        Some(crate::ir::EdgeArrowhead::OpenTriangle) => {
                            format!("marker-start=\"url(#arrow-class-open-start-{marker_id})\"")
                        }
                        Some(crate::ir::EdgeArrowhead::ClassDependency) => {
                            format!("marker-start=\"url(#arrow-class-dep-start-{marker_id})\"")
                        }
                        None => format!("marker-start=\"url(#arrow-start-{marker_id})\""),
                    },
                    _ => format!("marker-start=\"url(#arrow-start-{marker_id})\""),
                }
            } else {
                String::new()
            };
            if let Some(width) = edge.override_style.stroke_width {
                stroke_width = width;
            }
            if let Some(dash_override) = &edge.override_style.dasharray {
                dash = format!("stroke-dasharray=\"{}\"", dash_override);
            }
            svg.push_str(&format!(
                "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" {} {} {} stroke-linecap=\"round\" stroke-linejoin=\"round\" />",
                d, stroke, stroke_width, marker_end, marker_start, dash
            ));

            if let Some(point) = edge.points.first().copied()
                && let Some(decoration) = edge.start_decoration
            {
                let angle = edge_endpoint_angle(&edge.points, true);
                svg.push_str(&edge_decoration_svg(
                    point,
                    angle,
                    decoration,
                    &stroke,
                    stroke_width,
                    true,
                ));
            }
            if let Some(point) = edge.points.last().copied()
                && let Some(decoration) = edge.end_decoration
            {
                let angle = edge_endpoint_angle(&edge.points, false);
                svg.push_str(&edge_decoration_svg(
                    point,
                    angle,
                    decoration,
                    &stroke,
                    stroke_width,
                    false,
                ));
            }

            if let Some((x, y, label)) = label_positions.get(&idx).and_then(|v| v.clone()) {
                let pad_x = 4.0;
                let pad_y = 2.0;
                let rect_x = x - label.width / 2.0 - pad_x;
                let rect_y = y - label.height / 2.0 - pad_y;
                let rect_w = label.width + pad_x * 2.0;
                let rect_h = label.height + pad_y * 2.0;
                let label_fill = match layout.kind {
                    crate::ir::DiagramKind::Class | crate::ir::DiagramKind::State => {
                        theme.primary_color.as_str()
                    }
                    _ => theme.edge_label_background.as_str(),
                };
                svg.push_str(&format!(
                    "<rect x=\"{rect_x:.2}\" y=\"{rect_y:.2}\" width=\"{rect_w:.2}\" height=\"{rect_h:.2}\" rx=\"0\" ry=\"0\" fill=\"{}\" fill-opacity=\"0.5\" stroke=\"none\"/>",
                    label_fill
                ));
                svg.push_str(&text_block_svg(
                    x,
                    y,
                    &label,
                    theme,
                    config,
                    true,
                    edge.override_style.label_color.as_deref(),
                ));
            }

            let end_label_offset = (theme.font_size * 0.6).max(8.0);
            let label_color = edge
                .override_style
                .label_color
                .as_deref()
                .unwrap_or(theme.primary_text_color.as_str());
            if let Some(label) = edge.start_label.as_ref()
                && let Some((x, y)) = edge_endpoint_label_position(edge, true, end_label_offset)
            {
                svg.push_str(&text_block_svg(
                    x, y, label, theme, config, false, Some(label_color),
                ));
            }
            if let Some(label) = edge.end_label.as_ref()
                && let Some((x, y)) = edge_endpoint_label_position(edge, false, end_label_offset)
            {
                svg.push_str(&text_block_svg(
                    x, y, label, theme, config, false, Some(label_color),
                ));
            }
        }
    }

    if !is_sequence {
        for node in layout.nodes.values() {
            if node.hidden {
                continue;
            }
            if node.anchor_subgraph.is_some() {
                continue;
            }
            if let Some(link) = node.link.as_ref() {
                svg.push_str(&format!("<a {}>", link_attrs(link)));
                if let Some(title) = link.title.as_deref() {
                    svg.push_str(&format!("<title>{}</title>", escape_xml(title)));
                }
            }
            svg.push_str(&shape_svg(node, theme));
            svg.push_str(&divider_lines_svg(node, theme, config));
            let center_x = node.x + node.width / 2.0;
            let center_y = node.y + node.height / 2.0;
            let hide_label = node.label.lines.iter().all(|line| line.trim().is_empty())
                || node.id.starts_with("__start_")
                || node.id.starts_with("__end_");
            if !hide_label {
                let label_svg = if node.label.lines.iter().any(|line| is_divider_line(line)) {
                    text_block_svg_class(node, theme, config, node.style.text_color.as_deref())
                } else {
                    text_block_svg(
                        center_x,
                        center_y,
                        &node.label,
                        theme,
                        config,
                        false,
                        node.style.text_color.as_deref(),
                    )
                };
                svg.push_str(&label_svg);
            }
            if node.link.is_some() {
                svg.push_str("</a>");
            }
        }

        for footbox in &layout.sequence_footboxes {
            if let Some(link) = footbox.link.as_ref() {
                svg.push_str(&format!("<a {}>", link_attrs(link)));
                if let Some(title) = link.title.as_deref() {
                    svg.push_str(&format!("<title>{}</title>", escape_xml(title)));
                }
            }
            svg.push_str(&shape_svg(footbox, theme));
            svg.push_str(&divider_lines_svg(footbox, theme, config));
            let center_x = footbox.x + footbox.width / 2.0;
            let center_y = footbox.y + footbox.height / 2.0;
            let hide_label = footbox
                .label
                .lines
                .iter()
                .all(|line| line.trim().is_empty())
                || footbox.id.starts_with("__start_")
                || footbox.id.starts_with("__end_");
            if !hide_label {
                let label_svg = if footbox.label.lines.iter().any(|line| is_divider_line(line)) {
                    text_block_svg_class(
                        footbox,
                        theme,
                        config,
                        footbox.style.text_color.as_deref(),
                    )
                } else {
                    text_block_svg(
                        center_x,
                        center_y,
                        &footbox.label,
                        theme,
                        config,
                        false,
                        footbox.style.text_color.as_deref(),
                    )
                };
                svg.push_str(&label_svg);
            }
            if footbox.link.is_some() {
                svg.push_str("</a>");
            }
        }
    } else {
        for node in layout.nodes.values() {
            if node.hidden {
                continue;
            }
            if node.anchor_subgraph.is_some() {
                continue;
            }
            if let Some(link) = node.link.as_ref() {
                svg.push_str(&format!("<a {}>", link_attrs(link)));
                if let Some(title) = link.title.as_deref() {
                    svg.push_str(&format!("<title>{}</title>", escape_xml(title)));
                }
            }
            svg.push_str(&format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"3\" ry=\"3\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.0\"/>",
                node.x,
                node.y,
                node.width,
                node.height,
                theme.sequence_actor_fill,
                theme.sequence_actor_border
            ));
            let center_x = node.x + node.width / 2.0;
            let center_y = node.y + node.height / 2.0;
            let hide_label = node.label.lines.iter().all(|line| line.trim().is_empty())
                || node.id.starts_with("__start_")
                || node.id.starts_with("__end_");
            if !hide_label {
                svg.push_str(&text_block_svg(
                    center_x,
                    center_y,
                    &node.label,
                    theme,
                    config,
                    false,
                    node.style.text_color.as_deref(),
                ));
            }
            if node.link.is_some() {
                svg.push_str("</a>");
            }
        }
        for footbox in &layout.sequence_footboxes {
            if let Some(link) = footbox.link.as_ref() {
                svg.push_str(&format!("<a {}>", link_attrs(link)));
                if let Some(title) = link.title.as_deref() {
                    svg.push_str(&format!("<title>{}</title>", escape_xml(title)));
                }
            }
            svg.push_str(&format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"3\" ry=\"3\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.0\"/>",
                footbox.x,
                footbox.y,
                footbox.width,
                footbox.height,
                theme.sequence_actor_fill,
                theme.sequence_actor_border
            ));
            let center_x = footbox.x + footbox.width / 2.0;
            let center_y = footbox.y + footbox.height / 2.0;
            let hide_label = footbox
                .label
                .lines
                .iter()
                .all(|line| line.trim().is_empty())
                || footbox.id.starts_with("__start_")
                || footbox.id.starts_with("__end_");
            if !hide_label {
                svg.push_str(&text_block_svg(
                    center_x,
                    center_y,
                    &footbox.label,
                    theme,
                    config,
                    false,
                    footbox.style.text_color.as_deref(),
                ));
            }
            if footbox.link.is_some() {
                svg.push_str("</a>");
            }
        }
    }

    svg.push_str("</svg>");
    svg
}

fn points_to_path(points: &[(f32, f32)]) -> String {
    if points.is_empty() {
        return String::new();
    }
    if points.len() == 1 {
        return format!("M {:.2} {:.2}", points[0].0, points[0].1);
    }
    if points.len() == 2 {
        // Simple straight line for 2 points
        return format!(
            "M {:.2} {:.2} L {:.2} {:.2}",
            points[0].0, points[0].1, points[1].0, points[1].1
        );
    }

    // Use monotone cubic interpolation (similar to d3.curveMonotone)
    // This produces smooth curves that don't overshoot
    let mut d = String::new();
    d.push_str(&format!("M {:.3},{:.3}", points[0].0, points[0].1));

    // Calculate tangents using finite differences with monotonicity constraint
    let n = points.len();
    let mut tangents: Vec<(f32, f32)> = Vec::with_capacity(n);

    for i in 0..n {
        if i == 0 {
            // First point: use forward difference
            tangents.push((points[1].0 - points[0].0, points[1].1 - points[0].1));
        } else if i == n - 1 {
            // Last point: use backward difference
            tangents.push((
                points[n - 1].0 - points[n - 2].0,
                points[n - 1].1 - points[n - 2].1,
            ));
        } else {
            // Interior points: average of adjacent slopes
            let dx1 = points[i].0 - points[i - 1].0;
            let dy1 = points[i].1 - points[i - 1].1;
            let dx2 = points[i + 1].0 - points[i].0;
            let dy2 = points[i + 1].1 - points[i].1;
            tangents.push(((dx1 + dx2) * 0.5, (dy1 + dy2) * 0.5));
        }
    }

    // Generate cubic bezier segments
    for i in 0..n - 1 {
        let p1 = points[i];
        let p2 = points[i + 1];
        let t1 = tangents[i];
        let t2 = tangents[i + 1];

        // Control points at 1/3 and 2/3 of the segment
        let cp1x = p1.0 + t1.0 / 3.0;
        let cp1y = p1.1 + t1.1 / 3.0;
        let cp2x = p2.0 - t2.0 / 3.0;
        let cp2y = p2.1 - t2.1 / 3.0;

        d.push_str(&format!(
            "C{:.3},{:.3},{:.3},{:.3},{:.3},{:.3}",
            cp1x, cp1y, cp2x, cp2y, p2.0, p2.1
        ));
    }

    d
}

fn render_pie(layout: &Layout, theme: &Theme, config: &LayoutConfig) -> String {
    let mut svg = String::new();
    let (cx, cy) = layout.pie_center;
    let radius = layout.pie_radius;
    if radius <= 0.0 {
        return svg;
    }

    for slice in &layout.pie_slices {
        let span = (slice.end_angle - slice.start_angle).abs();
        if span <= 0.0001 {
            continue;
        }
        if span >= std::f32::consts::PI * 2.0 - 0.001 {
            svg.push_str(&format!(
                "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                cx,
                cy,
                radius,
                slice.color,
                theme.line_color
            ));
            continue;
        }
        let path = pie_slice_path(cx, cy, radius, slice.start_angle, slice.end_angle);
        svg.push_str(&format!(
            "<path d=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            path, slice.color, theme.line_color
        ));
    }

    for item in &layout.pie_legend {
        let rect_x = item.x - item.marker_size / 2.0;
        let rect_y = item.y - item.marker_size / 2.0;
        svg.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            rect_x,
            rect_y,
            item.marker_size,
            item.marker_size,
            item.color,
            theme.line_color
        ));
        let label_x = item.x + item.marker_size / 2.0 + theme.font_size * 0.4;
        svg.push_str(&text_block_svg_anchor(
            label_x,
            item.y,
            &item.label,
            theme,
            config,
            "start",
            Some(theme.primary_text_color.as_str()),
        ));
    }

    if let Some(title) = &layout.pie_title {
        svg.push_str(&text_block_svg(
            title.x,
            title.y,
            &title.text,
            theme,
            config,
            false,
            Some(theme.primary_text_color.as_str()),
        ));
    }

    svg
}

fn pie_slice_path(cx: f32, cy: f32, radius: f32, start_angle: f32, end_angle: f32) -> String {
    let sx = cx + radius * start_angle.cos();
    let sy = cy + radius * start_angle.sin();
    let ex = cx + radius * end_angle.cos();
    let ey = cy + radius * end_angle.sin();
    let large_arc = if (end_angle - start_angle).abs() > std::f32::consts::PI {
        1
    } else {
        0
    };
    let sweep = 1;
    format!(
        "M {cx:.2} {cy:.2} L {sx:.2} {sy:.2} A {radius:.2} {radius:.2} 0 {large_arc} {sweep} {ex:.2} {ey:.2} Z"
    )
}

fn text_block_svg(
    x: f32,
    y: f32,
    label: &TextBlock,
    theme: &Theme,
    config: &LayoutConfig,
    _edge: bool,
    override_color: Option<&str>,
) -> String {
    let total_height = label.lines.len() as f32 * theme.font_size * config.label_line_height;
    let start_y = y - total_height / 2.0 + theme.font_size;
    let mut text = String::new();
    let anchor = "middle";
    let default_fill = theme.primary_text_color.as_str();
    let fill = override_color.unwrap_or(default_fill);

    text.push_str(&format!(
        "<text x=\"{x:.2}\" y=\"{start_y:.2}\" text-anchor=\"{anchor}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">",
        theme.font_family,
        theme.font_size,
        fill
    ));

    let line_height = theme.font_size * config.label_line_height;
    for (idx, line) in label.lines.iter().enumerate() {
        let dy = if idx == 0 { 0.0 } else { line_height };
        let rendered = if is_divider_line(line) {
            String::new()
        } else {
            escape_xml(line)
        };
        text.push_str(&format!(
            "<tspan x=\"{x:.2}\" dy=\"{dy:.2}\">{}</tspan>",
            rendered
        ));
    }

    text.push_str("</text>");
    text
}

fn text_block_svg_anchor(
    x: f32,
    y: f32,
    label: &TextBlock,
    theme: &Theme,
    config: &LayoutConfig,
    anchor: &str,
    override_color: Option<&str>,
) -> String {
    let total_height = label.lines.len() as f32 * theme.font_size * config.label_line_height;
    let start_y = y - total_height / 2.0 + theme.font_size;
    let mut text = String::new();
    let default_fill = theme.primary_text_color.as_str();
    let fill = override_color.unwrap_or(default_fill);

    text.push_str(&format!(
        "<text x=\"{x:.2}\" y=\"{start_y:.2}\" text-anchor=\"{anchor}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">",
        theme.font_family,
        theme.font_size,
        fill
    ));

    let line_height = theme.font_size * config.label_line_height;
    for (idx, line) in label.lines.iter().enumerate() {
        let dy = if idx == 0 { 0.0 } else { line_height };
        let rendered = if is_divider_line(line) {
            String::new()
        } else {
            escape_xml(line)
        };
        text.push_str(&format!(
            "<tspan x=\"{x:.2}\" dy=\"{dy:.2}\">{}</tspan>",
            rendered
        ));
    }

    text.push_str("</text>");
    text
}

fn text_line_svg(x: f32, y: f32, text: &str, theme: &Theme, fill: &str, anchor: &str) -> String {
    format!(
        "<text x=\"{x:.2}\" y=\"{y:.2}\" text-anchor=\"{anchor}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
        theme.font_family,
        theme.font_size,
        fill,
        escape_xml(text)
    )
}

fn text_block_svg_class(
    node: &crate::layout::NodeLayout,
    theme: &Theme,
    config: &LayoutConfig,
    override_color: Option<&str>,
) -> String {
    let line_height = theme.font_size * config.label_line_height;
    let total_height = node.label.lines.len() as f32 * line_height;
    let start_y = node.y + node.height / 2.0 - total_height / 2.0 + theme.font_size;
    let center_x = node.x + node.width / 2.0;
    let left_x = node.x + config.node_padding_x.max(10.0);
    let fill = override_color.unwrap_or(theme.primary_text_color.as_str());

    let Some(divider_idx) = node
        .label
        .lines
        .iter()
        .position(|line| is_divider_line(line))
    else {
        return text_block_svg(
            center_x,
            node.y + node.height / 2.0,
            &node.label,
            theme,
            config,
            false,
            override_color,
        );
    };

    let mut title_lines: Vec<(usize, &str)> = Vec::new();
    for (idx, line) in node.label.lines.iter().enumerate().take(divider_idx) {
        if !line.trim().is_empty() {
            title_lines.push((idx, line.as_str()));
        }
    }
    let mut member_lines: Vec<(usize, &str)> = Vec::new();
    for (idx, line) in node.label.lines.iter().enumerate().skip(divider_idx + 1) {
        if !line.trim().is_empty() && !is_divider_line(line) {
            member_lines.push((idx, line.as_str()));
        }
    }

    let mut svg = String::new();
    if !title_lines.is_empty() {
        svg.push_str(&text_lines_svg(
            &title_lines,
            center_x,
            start_y,
            line_height,
            "middle",
            theme,
            fill,
            true,
        ));
    }
    if !member_lines.is_empty() {
        svg.push_str(&text_lines_svg(
            &member_lines,
            left_x,
            start_y,
            line_height,
            "start",
            theme,
            fill,
            false,
        ));
    }
    svg
}

fn text_lines_svg(
    lines: &[(usize, &str)],
    x: f32,
    start_y: f32,
    line_height: f32,
    anchor: &str,
    theme: &Theme,
    fill: &str,
    bold_first: bool,
) -> String {
    let Some((first_idx, _)) = lines.first() else {
        return String::new();
    };
    let first_y = start_y + *first_idx as f32 * line_height;
    let mut text = String::new();
    text.push_str(&format!(
        "<text x=\"{x:.2}\" y=\"{first_y:.2}\" text-anchor=\"{anchor}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">",
        theme.font_family,
        theme.font_size,
        fill
    ));

    let mut prev_idx = *first_idx;
    for (pos, (idx, line)) in lines.iter().enumerate() {
        let dy = if pos == 0 {
            0.0
        } else {
            (*idx - prev_idx) as f32 * line_height
        };
        let weight = if pos == 0 && bold_first {
            " font-weight=\"600\""
        } else {
            ""
        };
        text.push_str(&format!(
            "<tspan x=\"{x:.2}\" dy=\"{dy:.2}\"{weight}>{}</tspan>",
            escape_xml(line)
        ));
        prev_idx = *idx;
    }
    text.push_str("</text>");
    text
}

fn compute_edge_label_positions(
    edges: &[EdgeLayout],
    nodes: &std::collections::BTreeMap<String, crate::layout::NodeLayout>,
    subgraphs: &[crate::layout::SubgraphLayout],
) -> HashMap<usize, Option<(f32, f32, TextBlock)>> {
    let mut occupied: Vec<(f32, f32, f32, f32)> = Vec::new();
    let edge_obstacles = build_edge_obstacles(edges, 6.0);

    for node in nodes.values() {
        if node.anchor_subgraph.is_some() {
            continue;
        }
        occupied.push((
            node.x - 6.0,
            node.y - 6.0,
            node.width + 12.0,
            node.height + 12.0,
        ));
    }

    for sub in subgraphs {
        if sub.label.trim().is_empty() {
            continue;
        }
        let width = sub.label_block.width;
        let height = sub.label_block.height;
        let x = sub.x + 12.0;
        let y = sub.y + 6.0;
        occupied.push((x - 4.0, y, width + 8.0, height + 4.0));
    }
    let mut positions = HashMap::new();

    for (idx, edge) in edges.iter().enumerate() {
        let Some(label) = edge.label.clone() else {
            positions.insert(idx, None);
            continue;
        };
        let (mid_x, mid_y, offset_axis) = edge_label_anchor(edge);
        let mut placed = None;

        let step = match offset_axis {
            OffsetAxis::X => (label.width / 2.0 + 12.0).max(18.0),
            OffsetAxis::Y => (label.height + 8.0).max(18.0),
        };

        for attempt in 0..6 {
            let dir = if attempt % 2 == 0 { 1.0 } else { -1.0 };
            let offset = if attempt == 0 {
                0.0
            } else {
                let step_mul = ((attempt + 1) / 2) as f32;
                dir * step_mul * step
            };

            let (x, y) = match offset_axis {
                OffsetAxis::X => (mid_x + offset, mid_y),
                OffsetAxis::Y => (mid_x, mid_y + offset),
            };
            let rect = (
                x - label.width / 2.0 - 6.0,
                y - label.height / 2.0 - 4.0,
                label.width + 12.0,
                label.height + 8.0,
            );
            if !collides(&rect, &occupied) && !collides_edges(&rect, &edge_obstacles, idx) {
                occupied.push(rect);
                placed = Some((x, y, label.clone()));
                break;
            }
        }

        if placed.is_none() {
            placed = Some((mid_x, mid_y, label));
        }

        positions.insert(idx, placed);
    }

    positions
}

fn is_divider_line(line: &str) -> bool {
    line.trim() == "---"
}

fn divider_lines_svg(
    node: &crate::layout::NodeLayout,
    theme: &Theme,
    config: &LayoutConfig,
) -> String {
    if !node.label.lines.iter().any(|line| is_divider_line(line)) {
        return String::new();
    }

    let line_height = theme.font_size * config.label_line_height;
    let total_height = node.label.lines.len() as f32 * line_height;
    let start_y = node.y + node.height / 2.0 - total_height / 2.0 + theme.font_size;
    let stroke = node
        .style
        .stroke
        .as_ref()
        .unwrap_or(&theme.primary_border_color);
    let x1 = node.x + 6.0;
    let x2 = node.x + node.width - 6.0;

    let mut svg = String::new();
    for (idx, line) in node.label.lines.iter().enumerate() {
        if !is_divider_line(line) {
            continue;
        }
        let baseline_y = start_y + idx as f32 * line_height;
        let y = baseline_y - theme.font_size * 0.35;
        svg.push_str(&format!(
            "<line x1=\"{x1:.2}\" y1=\"{y:.2}\" x2=\"{x2:.2}\" y2=\"{y:.2}\" stroke=\"{stroke}\" stroke-width=\"1.0\"/>",
        ));
    }

    svg
}

#[derive(Debug, Clone, Copy)]
enum OffsetAxis {
    X,
    Y,
}

fn edge_label_anchor(edge: &EdgeLayout) -> (f32, f32, OffsetAxis) {
    if edge.points.len() >= 4 {
        let p1 = edge.points[1];
        let p2 = edge.points[2];
        let dx = (p2.0 - p1.0).abs();
        let dy = (p2.1 - p1.1).abs();
        let axis = if dx > dy {
            OffsetAxis::Y
        } else {
            OffsetAxis::X
        };
        return ((p1.0 + p2.0) / 2.0, (p1.1 + p2.1) / 2.0, axis);
    }
    if edge.points.len() >= 2 {
        let p1 = edge.points[0];
        let p2 = edge.points[edge.points.len() - 1];
        let dx = (p2.0 - p1.0).abs();
        let dy = (p2.1 - p1.1).abs();
        let axis = if dx > dy {
            OffsetAxis::Y
        } else {
            OffsetAxis::X
        };
        return ((p1.0 + p2.0) / 2.0, (p1.1 + p2.1) / 2.0, axis);
    }
    (0.0, 0.0, OffsetAxis::Y)
}

type Rect = (f32, f32, f32, f32);
type EdgeObstacle = (usize, Rect);

fn collides(rect: &Rect, occupied: &[Rect]) -> bool {
    for (x, y, w, h) in occupied {
        if rect.0 < x + w && rect.0 + rect.2 > *x && rect.1 < y + h && rect.1 + rect.3 > *y {
            return true;
        }
    }
    false
}

fn build_edge_obstacles(edges: &[EdgeLayout], pad: f32) -> Vec<EdgeObstacle> {
    let mut obstacles = Vec::new();
    for (idx, edge) in edges.iter().enumerate() {
        for segment in edge.points.windows(2) {
            let (a, b) = (segment[0], segment[1]);
            let min_x = a.0.min(b.0) - pad;
            let max_x = a.0.max(b.0) + pad;
            let min_y = a.1.min(b.1) - pad;
            let max_y = a.1.max(b.1) + pad;
            obstacles.push((idx, (min_x, min_y, max_x - min_x, max_y - min_y)));
        }
    }
    obstacles
}

fn collides_edges(rect: &Rect, obstacles: &[EdgeObstacle], edge_idx: usize) -> bool {
    for (idx, (x, y, w, h)) in obstacles {
        if *idx == edge_idx {
            continue;
        }
        if rect.0 < x + w && rect.0 + rect.2 > *x && rect.1 < y + h && rect.1 + rect.3 > *y {
            return true;
        }
    }
    false
}

pub fn write_output_svg(svg: &str, output: Option<&Path>) -> Result<()> {
    match output {
        Some(path) => {
            std::fs::write(path, svg)?;
        }
        None => {
            print!("{}", svg);
        }
    }
    Ok(())
}

#[cfg(feature = "png")]
pub fn write_output_png(
    svg: &str,
    output: &Path,
    render_cfg: &RenderConfig,
    theme: &Theme,
) -> Result<()> {
    let mut opt = usvg::Options {
        font_family: primary_font(&theme.font_family),
        default_size: usvg::Size::from_wh(render_cfg.width, render_cfg.height)
            .unwrap_or(usvg::Size::from_wh(800.0, 600.0).unwrap()),
        ..Default::default()
    };

    opt.fontdb_mut().load_system_fonts();

    let tree = usvg::Tree::from_str(svg, &opt)?;
    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or_else(|| anyhow::anyhow!("Failed to allocate pixmap"))?;

    let mut pixmap_mut = pixmap.as_mut();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap_mut,
    );
    pixmap.save_png(output)?;
    Ok(())
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn link_attrs(link: &crate::ir::NodeLink) -> String {
    let url = escape_xml(&link.url);
    let mut attrs = format!("href=\"{}\" xlink:href=\"{}\"", url, url);
    if let Some(target) = link.target.as_deref() {
        let target = escape_xml(target);
        attrs.push_str(&format!(" target=\"{}\"", target));
        if target == "_blank" {
            attrs.push_str(" rel=\"noopener noreferrer\"");
        }
    }
    attrs
}

fn edge_decoration_svg(
    point: (f32, f32),
    angle_deg: f32,
    decoration: crate::ir::EdgeDecoration,
    stroke: &str,
    stroke_width: f32,
    at_start: bool,
) -> String {
    let (x, y) = point;
    let mut angle = angle_deg;
    if matches!(
        decoration,
        crate::ir::EdgeDecoration::Diamond | crate::ir::EdgeDecoration::DiamondFilled
    ) && !at_start
    {
        angle += 180.0;
    }
    let join = " stroke-linejoin=\"round\" stroke-linecap=\"round\"";
    let shape = match decoration {
        crate::ir::EdgeDecoration::Circle => format!(
            "<circle cx=\"0\" cy=\"0\" r=\"4\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/>",
            stroke, stroke_width
        ),
        crate::ir::EdgeDecoration::Cross => format!(
            "<path d=\"M -4 -4 L 4 4 M -4 4 L 4 -4\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{join}/>",
            stroke, stroke_width
        ),
        crate::ir::EdgeDecoration::Diamond => {
            let points = "0,0 9,6 18,0 9,-6";
            format!(
                "<polygon points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{join}/>",
                points, stroke, stroke_width
            )
        }
        crate::ir::EdgeDecoration::DiamondFilled => {
            let points = "0,0 9,6 18,0 9,-6";
            format!(
                "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{join}/>",
                points, stroke, stroke, stroke_width
            )
        }
    };
    format!("<g transform=\"translate({x:.2} {y:.2}) rotate({angle:.2})\">{shape}</g>")
}

fn edge_endpoint_angle(points: &[(f32, f32)], start: bool) -> f32 {
    if points.len() < 2 {
        return 0.0;
    }
    let (p0, p1) = if start {
        (points[0], points[1])
    } else {
        (points[points.len() - 2], points[points.len() - 1])
    };
    let dx = p1.0 - p0.0;
    let dy = p1.1 - p0.1;
    dy.atan2(dx).to_degrees()
}

fn edge_endpoint_label_position(edge: &EdgeLayout, start: bool, offset: f32) -> Option<(f32, f32)> {
    if edge.points.len() < 2 {
        return None;
    }
    let (p0, p1) = if start {
        (edge.points[0], edge.points[1])
    } else {
        (
            edge.points[edge.points.len() - 1],
            edge.points[edge.points.len() - 2],
        )
    };
    let dx = p1.0 - p0.0;
    let dy = p1.1 - p0.1;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= f32::EPSILON {
        return None;
    }
    let dir_x = dx / len;
    let dir_y = dy / len;
    let base_x = p0.0 + dir_x * offset * 1.4;
    let base_y = p0.1 + dir_y * offset * 1.4;
    let perp_x = -dir_y;
    let perp_y = dir_x;
    Some((base_x + perp_x * offset, base_y + perp_y * offset))
}

#[cfg(feature = "png")]
fn primary_font(fonts: &str) -> String {
    fonts
        .split(',')
        .map(|s| s.trim().trim_matches('"'))
        .find(|s| !s.is_empty())
        .unwrap_or("Inter")
        .to_string()
}

fn shape_svg(node: &crate::layout::NodeLayout, theme: &Theme) -> String {
    let stroke = node
        .style
        .stroke
        .as_ref()
        .unwrap_or(&theme.primary_border_color);
    let fill = node.style.fill.as_ref().unwrap_or(&theme.primary_color);
    let dash = node
        .style
        .stroke_dasharray
        .as_ref()
        .map(|value| format!(" stroke-dasharray=\"{}\"", value))
        .unwrap_or_default();
    let join = " stroke-linejoin=\"round\" stroke-linecap=\"round\"";
    let x = node.x;
    let y = node.y;
    let w = node.width;
    let h = node.height;
    match node.shape {
        crate::ir::NodeShape::Rectangle => format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"0\" ry=\"0\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
            x,
            y,
            w,
            h,
            fill,
            stroke,
            node.style.stroke_width.unwrap_or(1.0)
        ),
        crate::ir::NodeShape::ForkJoin => format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"2\" ry=\"2\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
            x,
            y,
            w,
            h,
            fill,
            stroke,
            node.style.stroke_width.unwrap_or(1.0)
        ),
        crate::ir::NodeShape::ActorBox => format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"3\" ry=\"3\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
            x,
            y,
            w,
            h,
            fill,
            stroke,
            node.style.stroke_width.unwrap_or(1.0)
        ),
        crate::ir::NodeShape::Diamond => {
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            let points = format!(
                "{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
                cx,
                y,
                x + w,
                cy,
                cx,
                y + h,
                x,
                cy
            );
            format!(
                "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
                points,
                fill,
                stroke,
                node.style.stroke_width.unwrap_or(1.0)
            )
        }
        crate::ir::NodeShape::Circle | crate::ir::NodeShape::DoubleCircle => {
            let label_empty = node.label.lines.iter().all(|line| line.trim().is_empty());
            let is_state_start = node.id.starts_with("__start_");
            let is_state_end = node.id.starts_with("__end_");
            let (circle_fill, circle_stroke) = if is_state_start {
                (theme.line_color.as_str(), theme.line_color.as_str())
            } else if is_state_end {
                (
                    theme.primary_border_color.as_str(),
                    theme.primary_border_color.as_str(),
                )
            } else if label_empty {
                if node.shape == crate::ir::NodeShape::Circle {
                    (
                        theme.primary_text_color.as_str(),
                        theme.primary_text_color.as_str(),
                    )
                } else {
                    (
                        theme.primary_border_color.as_str(),
                        theme.background.as_str(),
                    )
                }
            } else {
                (fill.as_str(), stroke.as_str())
            };
            let stroke_width = node.style.stroke_width.unwrap_or(1.0);
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            let r = (w.min(h)) / 2.0;
            let mut svg = format!(
                "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
                cx, cy, r, circle_fill, circle_stroke, stroke_width
            );
            if node.shape == crate::ir::NodeShape::DoubleCircle {
                let r2 = r - 4.0;
                if r2 > 0.0 {
                    let inner_fill = if label_empty || is_state_end {
                        theme.background.as_str()
                    } else {
                        "none"
                    };
                    let inner_stroke = if label_empty || is_state_end {
                        theme.background.as_str()
                    } else {
                        circle_stroke
                    };
                    let inner_stroke_width = if label_empty || is_state_end {
                        1.2
                    } else {
                        1.0
                    };
                    svg.push_str(&format!(
                        "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{join}/>",
                        cx, cy, r2, inner_fill, inner_stroke, inner_stroke_width
                    ));
                }
            }
            svg
        }
        crate::ir::NodeShape::Stadium => format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.2}\" ry=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
            x,
            y,
            w,
            h,
            h / 2.0,
            h / 2.0,
            fill,
            stroke,
            node.style.stroke_width.unwrap_or(1.0)
        ),
        crate::ir::NodeShape::RoundRect => format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"10\" ry=\"10\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
            x,
            y,
            w,
            h,
            fill,
            stroke,
            node.style.stroke_width.unwrap_or(1.0)
        ),
        crate::ir::NodeShape::Cylinder => {
            let stroke_width = node.style.stroke_width.unwrap_or(1.0);
            let cx = x + w / 2.0;
            let ry = (h * 0.12).clamp(6.0, 14.0);
            let rx = w / 2.0;
            let mut svg = String::new();
            svg.push_str(&format!(
                "<ellipse cx=\"{:.2}\" cy=\"{:.2}\" rx=\"{:.2}\" ry=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
                cx,
                y + ry,
                rx,
                ry,
                fill,
                stroke,
                stroke_width
            ));
            svg.push_str(&format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
                x,
                y + ry,
                w,
                (h - 2.0 * ry).max(0.0),
                fill,
                stroke,
                stroke_width
            ));
            svg.push_str(&format!(
                "<ellipse cx=\"{:.2}\" cy=\"{:.2}\" rx=\"{:.2}\" ry=\"{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
                cx,
                y + h - ry,
                rx,
                ry,
                stroke,
                stroke_width
            ));
            svg
        }
        crate::ir::NodeShape::Subroutine => {
            let stroke_width = node.style.stroke_width.unwrap_or(1.0);
            let inset = 6.0;
            let mut svg = format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
                x, y, w, h, fill, stroke, stroke_width
            );
            let y1 = y + 2.0;
            let y2 = y + h - 2.0;
            let x1 = x + inset;
            let x2 = x + w - inset;
            svg.push_str(&format!(
                "<line x1=\"{x1:.2}\" y1=\"{y1:.2}\" x2=\"{x1:.2}\" y2=\"{y2:.2}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{join}/>"
            ));
            svg.push_str(&format!(
                "<line x1=\"{x2:.2}\" y1=\"{y1:.2}\" x2=\"{x2:.2}\" y2=\"{y2:.2}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{join}/>"
            ));
            svg
        }
        crate::ir::NodeShape::Hexagon => {
            let x1 = x + w * 0.25;
            let x2 = x + w * 0.75;
            let y_mid = y + h / 2.0;
            let points = format!(
                "{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
                x1,
                y,
                x2,
                y,
                x + w,
                y_mid,
                x2,
                y + h,
                x1,
                y + h,
                x,
                y_mid
            );
            format!(
                "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
                points,
                fill,
                stroke,
                node.style.stroke_width.unwrap_or(1.0)
            )
        }
        crate::ir::NodeShape::Parallelogram | crate::ir::NodeShape::ParallelogramAlt => {
            let offset = w * 0.18;
            let (p1, p2, p3, p4) = if node.shape == crate::ir::NodeShape::Parallelogram {
                (
                    (x + offset, y),
                    (x + w, y),
                    (x + w - offset, y + h),
                    (x, y + h),
                )
            } else {
                (
                    (x, y),
                    (x + w - offset, y),
                    (x + w, y + h),
                    (x + offset, y + h),
                )
            };
            let points = format!(
                "{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
                p1.0, p1.1, p2.0, p2.1, p3.0, p3.1, p4.0, p4.1
            );
            format!(
                "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
                points,
                fill,
                stroke,
                node.style.stroke_width.unwrap_or(1.0)
            )
        }
        crate::ir::NodeShape::Trapezoid | crate::ir::NodeShape::TrapezoidAlt => {
            let offset = w * 0.18;
            let (p1, p2, p3, p4) = if node.shape == crate::ir::NodeShape::Trapezoid {
                (
                    (x + offset, y),
                    (x + w - offset, y),
                    (x + w, y + h),
                    (x, y + h),
                )
            } else {
                (
                    (x, y),
                    (x + w, y),
                    (x + w - offset, y + h),
                    (x + offset, y + h),
                )
            };
            let points = format!(
                "{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
                p1.0, p1.1, p2.0, p2.1, p3.0, p3.1, p4.0, p4.1
            );
            format!(
                "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
                points,
                fill,
                stroke,
                node.style.stroke_width.unwrap_or(1.0)
            )
        }
        crate::ir::NodeShape::Asymmetric => {
            let slant = w * 0.22;
            let points = format!(
                "{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
                x,
                y,
                x + w - slant,
                y,
                x + w,
                y + h / 2.0,
                x + w - slant,
                y + h,
                x,
                y + h
            );
            format!(
                "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
                points,
                fill,
                stroke,
                node.style.stroke_width.unwrap_or(1.0)
            )
        }
        _ => format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
            x,
            y,
            w,
            h,
            fill,
            stroke,
            node.style.stroke_width.unwrap_or(1.0)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LayoutConfig;
    use crate::ir::{Direction, Graph};
    use crate::layout::compute_layout;

    #[test]
    fn render_svg_basic() {
        let mut graph = Graph::new();
        graph.direction = Direction::LeftRight;
        graph.ensure_node(
            "A",
            Some("Alpha".to_string()),
            Some(crate::ir::NodeShape::Rectangle),
        );
        graph.ensure_node(
            "B",
            Some("Beta".to_string()),
            Some(crate::ir::NodeShape::Rectangle),
        );
        graph.edges.push(crate::ir::Edge {
            from: "A".to_string(),
            to: "B".to_string(),
            label: Some("go".to_string()),
            start_label: None,
            end_label: None,
            directed: true,
            arrow_start: false,
            arrow_end: true,
            arrow_start_kind: None,
            arrow_end_kind: None,
            start_decoration: None,
            end_decoration: None,
            style: crate::ir::EdgeStyle::Solid,
        });
        let layout = compute_layout(&graph, &Theme::modern(), &LayoutConfig::default());
        let svg = render_svg(&layout, &Theme::modern(), &LayoutConfig::default());
        assert!(svg.contains("<svg"));
        assert!(svg.contains("Alpha"));
    }
}
