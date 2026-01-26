use crate::config::LayoutConfig;
#[cfg(feature = "png")]
use crate::config::RenderConfig;
use crate::layout::{
    C4BoundaryLayout, C4Layout, C4RelLayout, C4ShapeLayout, EdgeLayout, ErrorLayout,
    GitGraphLayout, Layout, SankeyLayout, TextBlock,
};
use crate::theme::Theme;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

pub fn render_svg(layout: &Layout, theme: &Theme, config: &LayoutConfig) -> String {
    let mut svg = String::new();
    let (mut width, mut height, mut viewbox_x, mut viewbox_y, mut viewbox_width, mut viewbox_height) =
        if let Some(error) = layout.error.as_ref() {
            (
                error.render_width,
                error.render_height,
                0.0,
                0.0,
                error.viewbox_width,
                error.viewbox_height,
            )
        } else if layout.kind == crate::ir::DiagramKind::Requirement {
            let pad_x = config.requirement.render_padding_x;
            let pad_y = config.requirement.render_padding_y;
            let mut width = layout.width + pad_x * 2.0;
            let mut height = layout.height + pad_y * 2.0;
            width = width.max(1.0);
            height = height.max(1.0);
            (width, height, 0.0, 0.0, width, height)
        } else if let Some(c4) = layout.c4.as_ref() {
            let width = layout.width.max(1.0);
            let height = layout.height.max(1.0);
            (
                width,
                height,
                c4.viewbox_x,
                c4.viewbox_y,
                c4.viewbox_width,
                c4.viewbox_height,
            )
        } else if let Some(gitgraph) = layout.gitgraph.as_ref() {
            let width = layout.width.max(1.0);
            let height = layout.height.max(1.0);
            let viewbox_x = -gitgraph.offset_x;
            let viewbox_y = -gitgraph.offset_y;
            (width, height, viewbox_x, viewbox_y, gitgraph.width, gitgraph.height)
        } else if layout.kind == crate::ir::DiagramKind::Mindmap {
            let pad = config.mindmap.padding;
            let mut min_x = f32::MAX;
            let mut min_y = f32::MAX;
            let mut max_x = f32::MIN;
            let mut max_y = f32::MIN;
            for node in layout.nodes.values() {
                min_x = min_x.min(node.x);
                min_y = min_y.min(node.y);
                max_x = max_x.max(node.x + node.width);
                max_y = max_y.max(node.y + node.height);
            }
            if min_x == f32::MAX {
                min_x = 0.0;
                max_x = 1.0;
            }
            if min_y == f32::MAX {
                min_y = 0.0;
                max_y = 1.0;
            }
            let width = (max_x - min_x + pad * 2.0).max(1.0);
            let height = (max_y - min_y + pad * 2.0).max(1.0);
            let viewbox_x = min_x - pad;
            let viewbox_y = min_y - pad;
            (width, height, viewbox_x, viewbox_y, width, height)
        } else {
            let width = layout.width.max(1.0);
            let height = layout.height.max(1.0);
            (width, height, 0.0, 0.0, width, height)
        };
    let is_sequence = !layout.sequence_footboxes.is_empty();
    let is_state = layout.kind == crate::ir::DiagramKind::State;
    let is_class = layout.kind == crate::ir::DiagramKind::Class;
    let is_pie = layout.kind == crate::ir::DiagramKind::Pie;
    let is_c4 = layout.c4.is_some();
    let has_links = is_c4
        || layout
            .nodes
            .values()
            .any(|node| node.link.is_some())
        || layout.sequence_footboxes.iter().any(|node| node.link.is_some());

    let mut width_attr = width.to_string();
    let mut height_attr = height.to_string();
    let mut style_attr = String::new();
    if layout.error.is_none() {
        if let Some(c4) = layout.c4.as_ref() {
            if c4.use_max_width {
                width_attr = "100%".to_string();
                height_attr.clear();
                style_attr = format!(" style=\"max-width: {:.3}px;\"", viewbox_width);
            }
        } else if layout.gitgraph.is_some() && config.gitgraph.use_max_width {
            width_attr = "100%".to_string();
            height_attr.clear();
            style_attr = format!(" style=\"max-width: {:.3}px;\"", viewbox_width);
        } else if layout.kind == crate::ir::DiagramKind::Mindmap && config.mindmap.use_max_width {
            width_attr = "100%".to_string();
            height_attr.clear();
            style_attr = format!(" style=\"max-width: {:.3}px;\"", viewbox_width);
        } else if layout.kind == crate::ir::DiagramKind::Pie && config.pie.use_max_width {
            width_attr = "100%".to_string();
            height_attr.clear();
            style_attr = format!(" style=\"max-width: {:.3}px;\"", viewbox_width);
        }
    }
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\"{} width=\"{width_attr}\"{} viewBox=\"{viewbox_x} {viewbox_y} {viewbox_width} {viewbox_height}\"{style_attr}>",
        if has_links {
            " xmlns:xlink=\"http://www.w3.org/1999/xlink\""
        } else {
            ""
        },
        if height_attr.is_empty() {
            String::new()
        } else {
            format!(" height=\"{height_attr}\"")
        }
    ));

    if layout.error.is_some() {
        svg.push_str(&error_style_block(theme));
    }

    svg.push_str(&format!(
        "<rect x=\"{viewbox_x}\" y=\"{viewbox_y}\" width=\"{viewbox_width}\" height=\"{viewbox_height}\" fill=\"{}\"/>",
        theme.background
    ));

    if let Some(ref c4) = layout.c4 {
        svg.push_str(&render_c4(c4, config));
        svg.push_str("</svg>");
        return svg;
    }

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

    if let Some(ref error) = layout.error {
        svg.push_str(&render_error(error, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let Some(ref sankey) = layout.sankey {
        svg.push_str(&render_sankey(sankey, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if layout.kind == crate::ir::DiagramKind::Architecture {
        svg.push_str(&render_architecture(layout, theme, config, &color_ids));
        svg.push_str("</svg>");
        return svg;
    }

    if layout.kind == crate::ir::DiagramKind::Radar {
        svg.push_str(&render_radar(layout, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if layout.kind == crate::ir::DiagramKind::Requirement {
        svg.push_str(&render_requirement(layout, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if is_pie {
        svg.push_str(&render_pie(layout, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let Some(ref quadrant) = layout.quadrant {
        svg.push_str(&render_quadrant(quadrant, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let Some(ref gantt) = layout.gantt {
        svg.push_str(&render_gantt(gantt, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let Some(ref xychart) = layout.xychart {
        svg.push_str(&render_xychart(xychart, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let Some(ref timeline) = layout.timeline {
        svg.push_str(&render_timeline(timeline, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let Some(ref gitgraph) = layout.gitgraph {
        svg.push_str(&render_gitgraph(gitgraph, theme, config));
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
                    "<rect x=\"{rect_x:.2}\" y=\"{rect_y:.2}\" width=\"{rect_w:.2}\" height=\"{rect_h:.2}\" rx=\"2\" ry=\"2\" fill=\"{}\" stroke=\"none\"/>",
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
            svg.push_str(&shape_svg(node, theme, config));
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
            svg.push_str(&shape_svg(footbox, theme, config));
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

fn format_sankey_value(value: f32) -> String {
    let rounded_2 = (value * 100.0).round() / 100.0;
    if (rounded_2 - rounded_2.round()).abs() < 0.001 {
        return format!("{rounded_2:.0}");
    }
    let rounded_1 = (value * 10.0).round() / 10.0;
    if (rounded_1 - rounded_2).abs() < 0.001 {
        format!("{rounded_1:.1}")
    } else {
        format!("{rounded_2:.2}")
    }
}

fn render_sankey(layout: &SankeyLayout, theme: &Theme, _config: &LayoutConfig) -> String {
    let mut svg = String::new();
    let max_rank = layout.nodes.iter().map(|node| node.rank).max().unwrap_or(0);
    let label_font_size = 14.0f32;

    svg.push_str("<g class=\"nodes\">");
    for (idx, node) in layout.nodes.iter().enumerate() {
        let node_id = idx + 1;
        svg.push_str(&format!(
            "<g class=\"node\" id=\"node-{node_id}\" transform=\"translate({:.3},{:.3})\" x=\"{:.3}\" y=\"{:.3}\">",
            node.x, node.y, node.x, node.y
        ));
        svg.push_str(&format!(
            "<rect height=\"{}\" width=\"{}\" fill=\"{}\"/>",
            node.height,
            layout.node_width,
            escape_xml(&node.color)
        ));
        svg.push_str("</g>");
    }
    svg.push_str("</g>");

    svg.push_str(&format!(
        "<g class=\"node-labels\" font-size=\"{}\" fill=\"{}\">",
        label_font_size, theme.primary_text_color
    ));
    for node in &layout.nodes {
        let is_last_rank = node.rank == max_rank;
        let text_anchor = if is_last_rank { "end" } else { "start" };
        let x = if is_last_rank {
            node.x - 6.0
        } else {
            node.x + layout.node_width + 6.0
        };
        let y = node.y + node.height / 2.0;
        let label = escape_xml(&node.label);
        let value = format_sankey_value(node.total);
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" dy=\"0em\" text-anchor=\"{}\">{}\n{}</text>",
            x, y, text_anchor, label, value
        ));
    }
    svg.push_str("</g>");

    svg.push_str("<g class=\"links\" fill=\"none\" stroke-opacity=\"0.5\">");
    for link in &layout.links {
        let mid_x = (link.start.0 + link.end.0) / 2.0;
        let gradient_id = escape_xml(&link.gradient_id);
        svg.push_str("<g class=\"link\" style=\"mix-blend-mode: multiply;\">");
        svg.push_str(&format!(
            "<linearGradient id=\"{}\" gradientUnits=\"userSpaceOnUse\" x1=\"{}\" x2=\"{}\">",
            gradient_id, link.start.0, link.end.0
        ));
        svg.push_str(&format!(
            "<stop offset=\"0%\" stop-color=\"{}\"/>",
            escape_xml(&link.color_start)
        ));
        svg.push_str(&format!(
            "<stop offset=\"100%\" stop-color=\"{}\"/>",
            escape_xml(&link.color_end)
        ));
        svg.push_str("</linearGradient>");
        svg.push_str(&format!(
            "<path d=\"M{:.3},{:.3}C{:.3},{:.3},{:.3},{:.3},{:.3},{:.3}\" stroke=\"url(#{})\" stroke-width=\"{}\"/>",
            link.start.0,
            link.start.1,
            mid_x,
            link.start.1,
            mid_x,
            link.end.1,
            link.end.0,
            link.end.1,
            gradient_id,
            link.thickness
        ));
        svg.push_str("</g>");
    }
    svg.push_str("</g>");

    svg
}

fn render_error(layout: &ErrorLayout, _theme: &Theme, _config: &LayoutConfig) -> String {
    // Mermaid CLI renders a dedicated error diagram for unsupported syntax.
    // We mirror that here so treemap diagrams can match CLI output closely.
    const ERROR_ICON_PATHS: [&str; 6] = [
        "m411.313,123.313c6.25-6.25 6.25-16.375 0-22.625s-16.375-6.25-22.625,0l-32,32-9.375,9.375-20.688-20.688c-12.484-12.5-32.766-12.5-45.25,0l-16,16c-1.261,1.261-2.304,2.648-3.31,4.051-21.739-8.561-45.324-13.426-70.065-13.426-105.867,0-192,86.133-192,192s86.133,192 192,192 192-86.133 192-192c0-24.741-4.864-48.327-13.426-70.065 1.402-1.007 2.79-2.049 4.051-3.31l16-16c12.5-12.492 12.5-32.758 0-45.25l-20.688-20.688 9.375-9.375 32.001-31.999zm-219.313,100.687c-52.938,0-96,43.063-96,96 0,8.836-7.164,16-16,16s-16-7.164-16-16c0-70.578 57.422-128 128-128 8.836,0 16,7.164 16,16s-7.164,16-16,16z",
        "m459.02,148.98c-6.25-6.25-16.375-6.25-22.625,0s-6.25,16.375 0,22.625l16,16c3.125,3.125 7.219,4.688 11.313,4.688 4.094,0 8.188-1.563 11.313-4.688 6.25-6.25 6.25-16.375 0-22.625l-16.001-16z",
        "m340.395,75.605c3.125,3.125 7.219,4.688 11.313,4.688 4.094,0 8.188-1.563 11.313-4.688 6.25-6.25 6.25-16.375 0-22.625l-16-16c-6.25-6.25-16.375-6.25-22.625,0s-6.25,16.375 0,22.625l15.999,16z",
        "m400,64c8.844,0 16-7.164 16-16v-32c0-8.836-7.156-16-16-16-8.844,0-16,7.164-16,16v32c0,8.836 7.156,16 16,16z",
        "m496,96.586h-32c-8.844,0-16,7.164-16,16 0,8.836 7.156,16 16,16h32c8.844,0 16-7.164 16-16 0-8.836-7.156-16-16-16z",
        "m436.98,75.605c3.125,3.125 7.219,4.688 11.313,4.688 4.094,0 8.188-1.563 11.313-4.688l32-32c6.25-6.25 6.25-16.375 0-22.625s-16.375-6.25-22.625,0l-32,32c-6.251,6.25-6.251,16.375-0.001,22.625z",
    ];

    let mut svg = String::new();
    let needs_transform = layout.icon_scale != 1.0 || layout.icon_tx != 0.0 || layout.icon_ty != 0.0;

    let fmt = |value: f32| -> String {
        if (value - value.round()).abs() < 0.001 {
            format!("{:.0}", value)
        } else {
            format!("{:.2}", value)
        }
    };

    svg.push_str("<g>");
    if needs_transform {
        let transform = format!(
            "translate({},{}) scale({})",
            fmt(layout.icon_tx),
            fmt(layout.icon_ty),
            fmt(layout.icon_scale)
        );
        svg.push_str(&format!("<g transform=\"{transform}\">"));
    }
    for path in ERROR_ICON_PATHS {
        svg.push_str(&format!("<path class=\"error-icon\" d=\"{path}\"/>"));
    }
    if needs_transform {
        svg.push_str("</g>");
    }

    let message = escape_xml(&layout.message);
    let version = escape_xml(&format!("mermaid version {}", layout.version));
    svg.push_str(&format!(
        "<text class=\"error-text\" x=\"{}\" y=\"{}\" font-size=\"{}px\" style=\"text-anchor: middle;\">{}</text>",
        fmt(layout.text_x),
        fmt(layout.text_y),
        fmt(layout.text_size),
        message
    ));
    svg.push_str(&format!(
        "<text class=\"error-text\" x=\"{}\" y=\"{}\" font-size=\"{}px\" style=\"text-anchor: middle;\">{}</text>",
        fmt(layout.version_x),
        fmt(layout.version_y),
        fmt(layout.version_size),
        version
    ));
    svg.push_str("</g>");

    svg
}

fn normalize_font_family_css(font_family: &str) -> String {
    font_family
        .split(',')
        .map(|part| part.trim().trim_matches('\'').trim_matches('"'))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(",")
}

fn error_style_block(theme: &Theme) -> String {
    let font_family = normalize_font_family_css(&theme.font_family);
    format!(
        "<style>svg{{font-family:{font_family};font-size:{font_size};fill:{fill};}}.error-icon{{fill:#552222;}}.error-text{{fill:#552222;stroke:#552222;}}</style>",
        font_family = font_family,
        font_size = theme.font_size,
        fill = theme.text_color
    )
}

fn render_requirement(layout: &Layout, theme: &Theme, config: &LayoutConfig) -> String {
    let mut svg = String::new();
    let req = &config.requirement;
    let font_family = escape_xml(&theme.font_family);
    let measure_font_size = theme.font_size.max(16.0);
    let line_height = measure_font_size * config.label_line_height;

    let render_line = |x: f32, y: f32, text: &str, color: &str, bold: bool| -> String {
        let weight = if bold { " font-weight=\"bold\"" } else { "" };
        format!(
            "<text x=\"{x:.2}\" y=\"{y:.2}\" text-anchor=\"start\" font-family=\"{font_family}\" font-size=\"{size}\" fill=\"{color}\"{weight}>{text}</text>",
            x = x,
            y = y,
            font_family = font_family,
            size = theme.font_size,
            color = color,
            weight = weight,
            text = escape_xml(text)
        )
    };

    // Requirement-specific markers.
    let edge_stroke = escape_xml(&req.edge_stroke);
    svg.push_str("<defs>");
    svg.push_str(&format!(
        "<marker id=\"req-contains-start\" refX=\"0\" refY=\"10\" markerWidth=\"20\" markerHeight=\"20\" orient=\"auto\"><g><circle cx=\"10\" cy=\"10\" r=\"9\" fill=\"none\" stroke=\"{edge_stroke}\" stroke-width=\"1\"/><line x1=\"1\" x2=\"19\" y1=\"10\" y2=\"10\" stroke=\"{edge_stroke}\"/><line y1=\"1\" y2=\"19\" x1=\"10\" x2=\"10\" stroke=\"{edge_stroke}\"/></g></marker>"
    ));
    svg.push_str(&format!(
        "<marker id=\"req-arrow-end\" refX=\"20\" refY=\"10\" markerWidth=\"20\" markerHeight=\"20\" orient=\"auto\"><path d=\"M0,0 L20,10 M20,10 L0,20\" fill=\"none\" stroke=\"{edge_stroke}\" stroke-width=\"1\"/></marker>"
    ));
    svg.push_str("</defs>");

    let pad_x = req.render_padding_x;
    let pad_y = req.render_padding_y;
    let has_padding = pad_x.abs() > f32::EPSILON || pad_y.abs() > f32::EPSILON;
    if has_padding {
        svg.push_str(&format!(
            "<g transform=\"translate({:.2},{:.2})\">",
            pad_x, pad_y
        ));
    }

    let label_positions =
        compute_edge_label_positions(&layout.edges, &layout.nodes, &layout.subgraphs);
    for (idx, edge) in layout.edges.iter().enumerate() {
        let stroke = edge
            .override_style
            .stroke
            .as_deref()
            .unwrap_or(req.edge_stroke.as_str());
        let stroke_width = edge
            .override_style
            .stroke_width
            .unwrap_or(req.edge_stroke_width);
        let dash = edge
            .override_style
            .dasharray
            .as_deref()
            .map(|value| format!(" stroke-dasharray=\"{}\"", value))
            .unwrap_or_default();
        let marker_start = if edge.arrow_start {
            " marker-start=\"url(#req-contains-start)\""
        } else {
            ""
        };
        let marker_end = if edge.arrow_end {
            " marker-end=\"url(#req-arrow-end)\""
        } else {
            ""
        };
        let d = points_to_path(&edge.points);
        svg.push_str(&format!(
            "<path d=\"{d}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{dash}{marker_start}{marker_end} stroke-linecap=\"round\" stroke-linejoin=\"round\"/>"
        ));

        if let Some((x, y, label)) = label_positions.get(&idx).and_then(|v| v.clone()) {
            let pad_x = req.edge_label_padding_x;
            let pad_y = req.edge_label_padding_y;
            let rect_x = x - label.width / 2.0 - pad_x;
            let rect_y = y - label.height / 2.0 - pad_y;
            let rect_w = label.width + pad_x * 2.0;
            let rect_h = label.height + pad_y * 2.0;
            if req.edge_label_background != "none" {
                svg.push_str(&format!(
                    "<rect x=\"{rect_x:.2}\" y=\"{rect_y:.2}\" width=\"{rect_w:.2}\" height=\"{rect_h:.2}\" rx=\"2\" ry=\"2\" fill=\"{}\" stroke=\"none\"/>",
                    req.edge_label_background
                ));
            }
            let label_color = edge
                .override_style
                .label_color
                .as_deref()
                .unwrap_or(req.edge_label_color.as_str());
            svg.push_str(&text_block_svg(
                x,
                y,
                &label,
                theme,
                config,
                true,
                Some(label_color),
            ));
        }
    }

    for node in layout.nodes.values() {
        if node.hidden {
            continue;
        }
        if node.anchor_subgraph.is_some() {
            continue;
        }
        let fill = node
            .style
            .fill
            .as_deref()
            .unwrap_or(req.fill.as_str());
        let base_stroke = node
            .style
            .stroke
            .as_deref()
            .unwrap_or(req.box_stroke.as_str());
        let base_stroke_width = node
            .style
            .stroke_width
            .unwrap_or(req.box_stroke_width);
        let label_color = node
            .style
            .text_color
            .as_deref()
            .unwrap_or(req.label_color.as_str());

        svg.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>",
            node.x, node.y, node.width, node.height, fill, base_stroke, base_stroke_width
        ));
        if req.stroke != "none" && req.stroke_width > 0.0 {
            svg.push_str(&format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/>",
                node.x,
                node.y,
                node.width,
                node.height,
                req.stroke,
                req.stroke_width
            ));
        }

        let lines = &node.label.lines;
        let header_count = lines.len().min(2);
        let body_lines = if lines.len() > 2 { &lines[2..] } else { &[] };
        let header_x = node.x + req.label_padding_x;
        let header_y = node.y + req.label_padding_y;
        if header_count >= 1 {
            svg.push_str(&render_line(header_x, header_y, &lines[0], label_color, false));
        }
        if header_count >= 2 {
            let id_y = header_y + req.header_line_gap;
            svg.push_str(&render_line(header_x, id_y, &lines[1], label_color, true));
        }

        if !body_lines.is_empty() {
            let divider_y = node.y + req.header_band_height;
            svg.push_str(&format!(
                "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"{}\"/>",
                node.x,
                divider_y,
                node.x + node.width,
                divider_y,
                req.divider_color,
                req.divider_width
            ));
            let mut body_y = divider_y + req.label_padding_y;
            for line in body_lines {
                svg.push_str(&render_line(header_x, body_y, line, label_color, false));
                body_y += line_height;
            }
        }
    }

    if has_padding {
        svg.push_str("</g>");
    }

    svg
}

fn render_radar(layout: &Layout, theme: &Theme, _config: &LayoutConfig) -> String {
    use std::f32::consts::PI;

    const WIDTH: f32 = 700.0;
    const HEIGHT: f32 = 700.0;
    const CENTER_X: f32 = WIDTH / 2.0;
    const CENTER_Y: f32 = HEIGHT / 2.0;
    const MAX_RADIUS: f32 = 300.0;
    const GRID_STEPS: usize = 5;
    const AXIS_LABEL_OFFSET: f32 = 15.0;
    const LEGEND_BOX_SIZE: f32 = 12.0;
    const LEGEND_GAP: f32 = 4.0;
    const GRID_COLOR: &str = "#DEDEDE";
    const AXIS_COLOR: &str = "#333333";
    const RADAR_HUES: [i32; 12] = [240, 60, 80, 270, 300, 330, 0, 30, 90, 150, 180, 210];
    const RADAR_LIGHTNESS: &str = "76.2745098039%";

    fn radar_index(id: &str) -> usize {
        id.rsplit('_')
            .next()
            .and_then(|part| part.parse::<usize>().ok())
            .unwrap_or(usize::MAX)
    }

    fn parse_series(node: &crate::layout::NodeLayout) -> Option<(String, Vec<(String, f32)>)> {
        let mut lines = node
            .label
            .lines
            .iter()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty());
        let name = lines.next()?.to_string();
        let mut pairs = Vec::new();
        for line in lines {
            let Some((axis_raw, value_raw)) = line.split_once(':') else {
                continue;
            };
            let axis = axis_raw.trim();
            let value_str = value_raw.trim();
            if axis.is_empty() || value_str.is_empty() {
                continue;
            }
            let Ok(value) = value_str.parse::<f32>() else {
                continue;
            };
            pairs.push((axis.to_string(), value.max(0.0)));
        }
        if pairs.is_empty() {
            None
        } else {
            Some((name, pairs))
        }
    }

    let mut nodes: Vec<&crate::layout::NodeLayout> = layout
        .nodes
        .values()
        .filter(|node| !node.hidden)
        .collect();
    nodes.sort_by_key(|node| radar_index(&node.id));

    let mut raw_series = Vec::new();
    for node in nodes {
        if let Some(series) = parse_series(node) {
            raw_series.push(series);
        }
    }
    let Some((_, first_pairs)) = raw_series.first() else {
        return String::new();
    };

    let axes: Vec<String> = first_pairs.iter().map(|(axis, _)| axis.clone()).collect();
    let axis_count = axes.len();
    if axis_count == 0 {
        return String::new();
    }

    let mut series_values: Vec<(String, Vec<f32>)> = Vec::new();
    let mut max_value = 0.0f32;
    for (name, pairs) in &raw_series {
        let mut values = Vec::with_capacity(axis_count);
        for axis in &axes {
            let value = pairs
                .iter()
                .find_map(|(a, v)| (a == axis).then_some(*v))
                .unwrap_or(0.0);
            max_value = max_value.max(value);
            values.push(value);
        }
        series_values.push((name.clone(), values));
    }

    if max_value <= 0.0 {
        max_value = 1.0;
    }
    let scale = MAX_RADIUS / max_value;
    let angle_step = 2.0 * PI / axis_count as f32;
    let start_angle = -PI / 2.0;

    let mut svg = String::new();
    svg.push_str(&format!(
        "<g transform=\"translate({:.3}, {:.3})\">",
        CENTER_X, CENTER_Y
    ));

    for step in 1..=GRID_STEPS {
        let r = MAX_RADIUS * step as f32 / GRID_STEPS as f32;
        svg.push_str(&format!(
            "<circle r=\"{:.3}\" fill=\"{}\" fill-opacity=\"0.3\" stroke=\"{}\" stroke-width=\"1\" />",
            r, GRID_COLOR, GRID_COLOR
        ));
    }

    for (idx, axis) in axes.iter().enumerate() {
        let angle = start_angle + angle_step * idx as f32;
        let x = MAX_RADIUS * angle.cos();
        let y = MAX_RADIUS * angle.sin();
        svg.push_str(&format!(
            "<line x1=\"0\" y1=\"0\" x2=\"{:.3}\" y2=\"{:.3}\" stroke=\"{}\" stroke-width=\"2\" />",
            x, y, AXIS_COLOR
        ));
        let label_r = MAX_RADIUS + AXIS_LABEL_OFFSET;
        let lx = label_r * angle.cos();
        let ly = label_r * angle.sin();
        svg.push_str(&format!(
            "<text x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"{}\" font-size=\"12\" fill=\"{}\">{}</text>",
            lx,
            ly,
            escape_xml(&theme.font_family),
            AXIS_COLOR,
            escape_xml(axis)
        ));
    }

    for (series_idx, (name, values)) in series_values.iter().enumerate() {
        let hue = RADAR_HUES[series_idx % RADAR_HUES.len()];
        let color = format!("hsl({}, 100%, {})", hue, RADAR_LIGHTNESS);
        let mut points = Vec::with_capacity(axis_count);
        for (idx, value) in values.iter().enumerate() {
            let angle = start_angle + angle_step * idx as f32;
            let r = value * scale;
            points.push((r * angle.cos(), r * angle.sin()));
        }
        if points.is_empty() {
            continue;
        }
        let mut d = String::new();
        d.push_str(&format!("M{:.3},{:.3}", points[0].0, points[0].1));
        for point in points.iter().skip(1) {
            d.push_str(&format!(" L{:.3},{:.3}", point.0, point.1));
        }
        d.push_str(" Z");
        svg.push_str(&format!(
            "<path d=\"{}\" fill=\"{}\" fill-opacity=\"0.5\" stroke=\"{}\" stroke-width=\"2\" />",
            d,
            escape_xml(&color),
            escape_xml(&color)
        ));

        let legend_offset = MAX_RADIUS * 0.875;
        let legend_x = legend_offset;
        let legend_y = -legend_offset + series_idx as f32 * (theme.font_size + 6.0);
        svg.push_str(&format!(
            "<rect x=\"{:.3}\" y=\"{:.3}\" width=\"{}\" height=\"{}\" fill=\"{}\" fill-opacity=\"0.5\" stroke=\"{}\" />",
            legend_x,
            legend_y,
            LEGEND_BOX_SIZE,
            LEGEND_BOX_SIZE,
            escape_xml(&color),
            escape_xml(&color)
        ));
        svg.push_str(&format!(
            "<text x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"start\" dominant-baseline=\"hanging\" font-family=\"{}\" font-size=\"12\" fill=\"{}\">{}</text>",
            legend_x + LEGEND_BOX_SIZE + LEGEND_GAP,
            legend_y,
            escape_xml(&theme.font_family),
            AXIS_COLOR,
            escape_xml(name)
        ));
    }

    svg.push_str(&format!(
        "<text x=\"0\" y=\"{:.3}\" text-anchor=\"middle\" dominant-baseline=\"hanging\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\"></text>",
        -(MAX_RADIUS + 50.0),
        escape_xml(&theme.font_family),
        theme.font_size,
        AXIS_COLOR
    ));

    svg.push_str("</g>");
    svg
}

fn render_architecture(
    layout: &Layout,
    theme: &Theme,
    _config: &LayoutConfig,
    color_ids: &HashMap<String, usize>,
) -> String {
    const ICON_FILL: &str = "#087ebf";
    const ICON_TEXT_FILL: &str = "#ffffff";
    const GROUP_ICON_SIZE: f32 = 30.0;
    const GROUP_ICON_OFFSET: f32 = 1.0;
    const GROUP_STROKE: &str = "hsl(240, 60%, 86.2745098039%)";

    fn sanitize_group_suffix(label: &str) -> String {
        let mut out = String::with_capacity(label.len());
        for ch in label.chars() {
            if ch.is_ascii_alphanumeric() {
                out.push(ch.to_ascii_lowercase());
            } else if ch == '_' || ch == '-' {
                out.push(ch);
            } else {
                out.push('-');
            }
        }
        let trimmed = out.trim_matches('-');
        if trimmed.is_empty() {
            "group".to_string()
        } else {
            trimmed.to_string()
        }
    }

    fn first_line(text: &str) -> &str {
        text.lines().find(|line| !line.trim().is_empty()).unwrap_or(text)
    }

    let default_marker_idx = color_ids.get(&theme.line_color).copied().unwrap_or(0);
    let mut svg = String::new();

    svg.push_str("<g class=\"architecture-edges\">");
    for edge in &layout.edges {
        if edge.points.len() < 2 {
            continue;
        }
        let (x0, y0) = edge.points[0];
        let (x1, y1) = edge.points[edge.points.len() - 1];
        let stroke = edge
            .override_style
            .stroke
            .as_ref()
            .unwrap_or(&theme.line_color);
        let stroke_width = edge.override_style.stroke_width.unwrap_or(3.0);
        let marker_idx = color_ids.get(stroke).copied().unwrap_or(default_marker_idx);
        let dash_attr = edge
            .override_style
            .dasharray
            .as_ref()
            .map(|dash| format!(" stroke-dasharray=\"{}\"", dash))
            .unwrap_or_default();
        svg.push_str(&format!(
            "<path d=\"M {:.3} {:.3} L {:.3} {:.3}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" marker-end=\"url(#arrow-{})\"{} />",
            x0,
            y0,
            x1,
            y1,
            escape_xml(stroke),
            stroke_width,
            marker_idx,
            dash_attr,
        ));
    }
    svg.push_str("</g>");

    svg.push_str("<g class=\"architecture-services\">");
    for node in layout.nodes.values() {
        if node.hidden {
            continue;
        }
        let icon_fill = node.style.fill.as_deref().unwrap_or(ICON_FILL);
        let label_text = node
            .label
            .lines
            .iter()
            .find(|line| !line.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| node.id.clone());
        let label_y = node.height + theme.font_size + 8.0;
        svg.push_str(&format!(
            "<g id=\"service-{}\" class=\"architecture-service\" transform=\"translate({:.3},{:.3})\">",
            escape_xml(&node.id),
            node.x,
            node.y
        ));
        svg.push_str(&format!(
            "<rect width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"none\" />",
            node.width,
            node.height,
            escape_xml(icon_fill)
        ));
        svg.push_str(&format!(
            "<text x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"{}\" font-family=\"{}\" font-size=\"64\">?</text>",
            node.width / 2.0,
            node.height / 2.0 + 6.0,
            ICON_TEXT_FILL,
            escape_xml(&theme.font_family)
        ));
        svg.push_str(&format!(
            "<text x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
            node.width / 2.0,
            label_y,
            escape_xml(&theme.font_family),
            theme.font_size,
            escape_xml(&theme.primary_text_color),
            escape_xml(&label_text)
        ));
        svg.push_str("</g>");
    }
    svg.push_str("</g>");

    svg.push_str("<g class=\"architecture-groups\">");
    for subgraph in &layout.subgraphs {
        let stroke = subgraph.style.stroke.as_deref().unwrap_or(GROUP_STROKE);
        let stroke_width = subgraph.style.stroke_width.unwrap_or(2.0);
        let dash_attr = subgraph
            .style
            .stroke_dasharray
            .as_ref()
            .map(|dash| format!(" stroke-dasharray=\"{}\"", dash))
            .unwrap_or_default();
        let group_id = sanitize_group_suffix(&subgraph.label);
        svg.push_str(&format!(
            "<rect id=\"group-{}\" x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{} />",
            escape_xml(&group_id),
            subgraph.x,
            subgraph.y,
            subgraph.width,
            subgraph.height,
            escape_xml(stroke),
            stroke_width,
            dash_attr,
        ));
        let icon_x = subgraph.x + GROUP_ICON_OFFSET;
        let icon_y = subgraph.y + GROUP_ICON_OFFSET;
        svg.push_str(&format!(
            "<rect x=\"{:.3}\" y=\"{:.3}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"none\" />",
            icon_x,
            icon_y,
            GROUP_ICON_SIZE,
            GROUP_ICON_SIZE,
            ICON_FILL
        ));
        svg.push_str(&format!(
            "<text x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"{}\" font-family=\"{}\" font-size=\"24\">?</text>",
            icon_x + GROUP_ICON_SIZE / 2.0,
            icon_y + GROUP_ICON_SIZE / 2.0 + 2.0,
            ICON_TEXT_FILL,
            escape_xml(&theme.font_family)
        ));
        let label_x = subgraph.x + GROUP_ICON_SIZE + 4.0;
        let label_y = subgraph.y + GROUP_ICON_SIZE * 0.7;
        svg.push_str(&format!(
            "<text x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"start\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
            label_x,
            label_y,
            escape_xml(&theme.font_family),
            theme.font_size,
            escape_xml(&theme.primary_text_color),
            escape_xml(first_line(&subgraph.label))
        ));
    }
    svg.push_str("</g>");

    svg
}

fn render_pie(layout: &Layout, theme: &Theme, config: &LayoutConfig) -> String {
    let mut svg = String::new();
    let (cx, cy) = layout.pie_center;
    let radius = layout.pie_radius;
    if radius <= 0.0 {
        return svg;
    }

    let pie_cfg = &config.pie;
    let mut total: f32 = layout.pie_legend.iter().map(|s| s.value.max(0.0)).sum();
    if total <= 0.0 {
        total = layout.pie_slices.iter().map(|s| s.value.max(0.0)).sum();
    }

    for slice in &layout.pie_slices {
        let span = (slice.end_angle - slice.start_angle).abs();
        if span <= 0.0001 {
            continue;
        }
        let percent_text = if total > 0.0 {
            format!("{:.0}", slice.value / total * 100.0)
        } else {
            "0".to_string()
        };
        if percent_text == "0" {
            continue;
        }
        if span >= std::f32::consts::PI * 2.0 - 0.001 {
            svg.push_str(&format!(
                "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.3}\" opacity=\"{:.3}\"/>",
                cx,
                cy,
                radius,
                escape_xml(&slice.color),
                escape_xml(&theme.pie_stroke_color),
                theme.pie_stroke_width,
                theme.pie_opacity
            ));
            continue;
        }
        let path = pie_slice_path(cx, cy, radius, slice.start_angle, slice.end_angle);
        svg.push_str(&format!(
            "<path d=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.3}\" opacity=\"{:.3}\"/>",
            escape_xml(&path),
            escape_xml(&slice.color),
            escape_xml(&theme.pie_stroke_color),
            theme.pie_stroke_width,
            theme.pie_opacity
        ));
    }

    if theme.pie_outer_stroke_width > 0.0 {
        let outer_radius = radius + theme.pie_outer_stroke_width / 2.0;
        svg.push_str(&format!(
            "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.3}\"/>",
            cx,
            cy,
            outer_radius,
            escape_xml(&theme.pie_outer_stroke_color),
            theme.pie_outer_stroke_width
        ));
    }

    // Add percentage labels on slices
    for slice in &layout.pie_slices {
        let span = (slice.end_angle - slice.start_angle).abs();
        if span <= 0.0001 || total <= 0.0 {
            continue;
        }
        let percent_text = format!("{:.0}", slice.value / total * 100.0);
        if percent_text == "0" {
            continue;
        }
        let mid_angle = (slice.start_angle + slice.end_angle) / 2.0;
        let label_radius = radius * pie_cfg.text_position;
        let label_x = cx + label_radius * mid_angle.cos();
        let label_y = cy + label_radius * mid_angle.sin();
        svg.push_str(&format!(
            "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}%</text>",
            label_x,
            label_y,
            theme.font_family,
            theme.pie_section_text_size,
            escape_xml(&theme.pie_section_text_color),
            percent_text
        ));
    }

    for item in &layout.pie_legend {
        let rect_x = item.x;
        let rect_y = item.y;
        svg.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.3}\"/>",
            rect_x,
            rect_y,
            item.marker_size,
            item.marker_size,
            escape_xml(&item.color),
            escape_xml(&item.color),
            theme.pie_stroke_width
        ));
        let label_x = rect_x + item.marker_size + pie_cfg.legend_spacing;
        let label_y = rect_y + item.marker_size - pie_cfg.legend_spacing;
        svg.push_str(&text_block_svg_with_font_size(
            label_x,
            label_y,
            &item.label,
            theme,
            config,
            theme.pie_legend_text_size,
            "start",
            Some(theme.pie_legend_text_color.as_str()),
            true,
        ));
    }

    if let Some(title) = &layout.pie_title {
        svg.push_str(&text_block_svg_with_font_size(
            title.x,
            title.y,
            &title.text,
            theme,
            config,
            theme.pie_title_text_size,
            "middle",
            Some(theme.pie_title_text_color.as_str()),
            true,
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

fn render_quadrant(
    layout: &crate::layout::QuadrantLayout,
    theme: &Theme,
    config: &LayoutConfig,
) -> String {
    let mut svg = String::new();
    let grid_x = layout.grid_x;
    let grid_y = layout.grid_y;
    let w = layout.grid_width;
    let h = layout.grid_height;
    let half_w = w / 2.0;
    let half_h = h / 2.0;

    // Quadrant background colors
    let q_colors = ["#ECECFF", "#f1f1ff", "#f6f6ff", "#fbfbff"];

    // Draw 4 quadrant backgrounds
    // Q1 top-right, Q2 top-left, Q3 bottom-left, Q4 bottom-right
    svg.push_str(&format!(
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\"/>",
        grid_x + half_w, grid_y, half_w, half_h, q_colors[0]
    ));
    svg.push_str(&format!(
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\"/>",
        grid_x, grid_y, half_w, half_h, q_colors[1]
    ));
    svg.push_str(&format!(
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\"/>",
        grid_x, grid_y + half_h, half_w, half_h, q_colors[2]
    ));
    svg.push_str(&format!(
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\"/>",
        grid_x + half_w, grid_y + half_h, half_w, half_h, q_colors[3]
    ));

    // Draw border
    svg.push_str(&format!(
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"none\" stroke=\"#c7c7f1\" stroke-width=\"2\"/>",
        grid_x, grid_y, w, h
    ));
    // Center lines
    svg.push_str(&format!(
        "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"#c7c7f1\" stroke-width=\"1\"/>",
        grid_x + half_w, grid_y, grid_x + half_w, grid_y + h
    ));
    svg.push_str(&format!(
        "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"#c7c7f1\" stroke-width=\"1\"/>",
        grid_x, grid_y + half_h, grid_x + w, grid_y + half_h
    ));

    // Title
    if let Some(ref title) = layout.title {
        svg.push_str(&text_block_svg(
            grid_x + half_w,
            layout.title_y,
            title,
            theme,
            config,
            false,
            Some(theme.primary_text_color.as_str()),
        ));
    }

    // Quadrant labels
    let label_positions = [
        (grid_x + half_w + half_w / 2.0, grid_y + 15.0), // Q1 top-right
        (grid_x + half_w / 2.0, grid_y + 15.0),          // Q2 top-left
        (grid_x + half_w / 2.0, grid_y + half_h + 15.0), // Q3 bottom-left
        (grid_x + half_w + half_w / 2.0, grid_y + half_h + 15.0), // Q4 bottom-right
    ];
    for (i, label_opt) in layout.quadrant_labels.iter().enumerate() {
        if let Some(label) = label_opt {
            let (lx, ly) = label_positions[i];
            svg.push_str(&text_block_svg(lx, ly, label, theme, config, false, Some("#131300")));
        }
    }

    // Axis labels
    if let Some(ref x_left) = layout.x_axis_left {
        svg.push_str(&text_block_svg(
            grid_x + half_w / 2.0,
            grid_y + h + 20.0,
            x_left,
            theme,
            config,
            false,
            Some("#131300"),
        ));
    }
    if let Some(ref x_right) = layout.x_axis_right {
        svg.push_str(&text_block_svg(
            grid_x + half_w + half_w / 2.0,
            grid_y + h + 20.0,
            x_right,
            theme,
            config,
            false,
            Some("#131300"),
        ));
    }
    if let Some(ref y_bottom) = layout.y_axis_bottom {
        svg.push_str(&format!(
            "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" transform=\"rotate(-90 {:.2} {:.2})\" font-family=\"{}\" font-size=\"{}\" fill=\"#131300\"><tspan>{}</tspan></text>",
            grid_x - 15.0,
            grid_y + half_h + half_h / 2.0,
            grid_x - 15.0,
            grid_y + half_h + half_h / 2.0,
            theme.font_family,
            theme.font_size,
            y_bottom.lines.first().map(|s| s.as_str()).unwrap_or("")
        ));
    }
    if let Some(ref y_top) = layout.y_axis_top {
        svg.push_str(&format!(
            "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" transform=\"rotate(-90 {:.2} {:.2})\" font-family=\"{}\" font-size=\"{}\" fill=\"#131300\"><tspan>{}</tspan></text>",
            grid_x - 15.0,
            grid_y + half_h / 2.0,
            grid_x - 15.0,
            grid_y + half_h / 2.0,
            theme.font_family,
            theme.font_size,
            y_top.lines.first().map(|s| s.as_str()).unwrap_or("")
        ));
    }

    // Data points
    for point in &layout.points {
        svg.push_str(&format!(
            "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"5\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            point.x, point.y, point.color, point.color
        ));
        svg.push_str(&text_block_svg(
            point.x,
            point.y + 15.0,
            &point.label,
            theme,
            config,
            false,
            Some("#131300"),
        ));
    }

    svg
}

fn render_gantt(
    layout: &crate::layout::GanttLayout,
    theme: &Theme,
    config: &LayoutConfig,
) -> String {
    let mut svg = String::new();

    // Title
    if let Some(ref title) = layout.title {
        svg.push_str(&text_block_svg(
            layout.chart_x + layout.chart_width / 2.0,
            20.0,
            title,
            theme,
            config,
            false,
            Some(theme.primary_text_color.as_str()),
        ));
    }

    // Draw sections
    for section in &layout.sections {
        svg.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"none\"/>",
            0.0,
            section.y,
            layout.chart_x + layout.chart_width,
            section.height,
            "#FFFFDE"
        ));
        svg.push_str(&text_block_svg(
            10.0 + section.label.width / 2.0,
            section.y + section.height / 2.0,
            &section.label,
            theme,
            config,
            false,
            Some(theme.primary_text_color.as_str()),
        ));
    }

    // Draw tasks as bars
    for task in &layout.tasks {
        // Task bar
        svg.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"3\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            task.x, task.y + 2.0, task.width, task.height - 4.0, task.color, theme.primary_border_color
        ));
        // Task label
        svg.push_str(&text_block_svg(
            task.x - 5.0 - task.label.width / 2.0,
            task.y + task.height / 2.0,
            &task.label,
            theme,
            config,
            false,
            Some(theme.primary_text_color.as_str()),
        ));
    }

    svg
}

fn render_xychart(
    layout: &crate::layout::XYChartLayout,
    theme: &Theme,
    config: &LayoutConfig,
) -> String {
    let mut svg = String::new();
    
    // Background
    svg.push_str(&format!(
        "<rect x=\"0\" y=\"0\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\"/>",
        layout.width, layout.height, theme.background
    ));
    
    // Title
    if let Some(ref title) = layout.title {
        svg.push_str(&text_block_svg(
            layout.width / 2.0,
            layout.title_y,
            title,
            theme,
            config,
            false,
            Some(theme.primary_text_color.as_str()),
        ));
    }
    
    // Plot area border
    svg.push_str(&format!(
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\"/>",
        layout.plot_x, layout.plot_y, layout.plot_width, layout.plot_height, theme.line_color
    ));
    
    // Y-axis ticks and labels
    for (label, y) in &layout.y_axis_ticks {
        // Tick line
        svg.push_str(&format!(
            "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"2,2\"/>",
            layout.plot_x, y, layout.plot_x + layout.plot_width, y, "#ccc"
        ));
        // Label
        svg.push_str(&format!(
            "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"end\" font-family=\"{}\" font-size=\"{:.1}\" fill=\"{}\">{}</text>",
            layout.plot_x - 5.0, y + theme.font_size / 3.0, 
            escape_xml(&theme.font_family), theme.font_size * 0.8, 
            theme.primary_text_color, escape_xml(label)
        ));
    }
    
    // X-axis categories
    for (label, x) in &layout.x_axis_categories {
        svg.push_str(&format!(
            "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{:.1}\" fill=\"{}\">{}</text>",
            x, layout.plot_y + layout.plot_height + 20.0,
            escape_xml(&theme.font_family), theme.font_size * 0.9,
            theme.primary_text_color, escape_xml(label)
        ));
    }
    
    // Y-axis label
    if let Some(ref y_label) = layout.y_axis_label {
        svg.push_str(&format!(
            "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{:.1}\" fill=\"{}\" transform=\"rotate(-90, {:.2}, {:.2})\">{}</text>",
            layout.y_axis_label_x, layout.plot_y + layout.plot_height / 2.0,
            escape_xml(&theme.font_family), theme.font_size,
            theme.primary_text_color,
            layout.y_axis_label_x, layout.plot_y + layout.plot_height / 2.0,
            escape_xml(&y_label.lines.join(" "))
        ));
    }
    
    // Bars
    for bar in &layout.bars {
        svg.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"none\"/>",
            bar.x, bar.y, bar.width, bar.height, escape_xml(&bar.color)
        ));
    }
    
    // Lines
    for line in &layout.lines {
        if line.points.len() >= 2 {
            let path: String = line.points.iter().enumerate().map(|(i, (x, y))| {
                if i == 0 {
                    format!("M {:.2},{:.2}", x, y)
                } else {
                    format!(" L {:.2},{:.2}", x, y)
                }
            }).collect();
            svg.push_str(&format!(
                "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>",
                path, escape_xml(&line.color)
            ));
            // Draw points
            for (x, y) in &line.points {
                svg.push_str(&format!(
                    "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"4\" fill=\"{}\" stroke=\"white\" stroke-width=\"1\"/>",
                    x, y, escape_xml(&line.color)
                ));
            }
        }
    }
    
    svg
}

fn render_timeline(
    layout: &crate::layout::TimelineLayout,
    theme: &Theme,
    config: &LayoutConfig,
) -> String {
    let mut svg = String::new();
    
    // Background
    svg.push_str(&format!(
        "<rect x=\"0\" y=\"0\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\"/>",
        layout.width, layout.height, theme.background
    ));
    
    // Title
    if let Some(ref title) = layout.title {
        svg.push_str(&text_block_svg(
            layout.width / 2.0,
            layout.title_y,
            title,
            theme,
            config,
            false,
            Some(theme.primary_text_color.as_str()),
        ));
    }
    
    // Main timeline line
    svg.push_str(&format!(
        "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"3\"/>",
        layout.line_start_x, layout.line_y, layout.line_end_x, layout.line_y, theme.primary_border_color
    ));
    
    // Colors for events
    let colors = [
        "#ECECFF", "#FFE6CC", "#D5E8D4", "#F8CECC", "#FFF2CC", "#E1D5E7"
    ];
    
    // Events
    for (i, event) in layout.events.iter().enumerate() {
        let color = colors[i % colors.len()];
        let center_x = event.x + event.width / 2.0;
        
        // Circle on timeline
        svg.push_str(&format!(
            "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
            center_x, event.circle_y, theme.primary_color, theme.primary_border_color
        ));
        
        // Vertical connector line
        svg.push_str(&format!(
            "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"2\" stroke-dasharray=\"4,2\"/>",
            center_x, event.circle_y + 8.0, center_x, event.y, theme.primary_border_color
        ));
        
        // Event box
        svg.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"5\" ry=\"5\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            event.x, event.y, event.width, event.height, color, theme.primary_border_color
        ));
        
        // Time label (bold, at top of box)
        svg.push_str(&format!(
            "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{:.1}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            center_x, event.y + 20.0,
            escape_xml(&theme.font_family), theme.font_size,
            theme.primary_text_color, escape_xml(&event.time.lines.join(" "))
        ));
        
        // Event descriptions
        let mut y_offset = 40.0;
        for evt in &event.events {
            svg.push_str(&format!(
                "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{:.1}\" fill=\"{}\">{}</text>",
                center_x, event.y + y_offset,
                escape_xml(&theme.font_family), theme.font_size * 0.9,
                theme.primary_text_color, escape_xml(&evt.lines.join(" "))
            ));
            y_offset += theme.font_size * 1.2;
        }
    }
    
    svg
}

fn render_gitgraph(gitgraph: &GitGraphLayout, theme: &Theme, config: &LayoutConfig) -> String {
    let gg = &config.gitgraph;
    let mut svg = String::new();
    svg.push_str("<g>");

    if gg.show_branches {
        for branch in &gitgraph.branches {
            let (x1, y1, x2, y2) = match gitgraph.direction {
                crate::ir::Direction::TopDown => (branch.pos, gg.default_pos, branch.pos, gitgraph.max_pos),
                crate::ir::Direction::BottomTop => {
                    (branch.pos, gitgraph.max_pos, branch.pos, gg.default_pos)
                }
                _ => (0.0, branch.pos, gitgraph.max_pos, branch.pos),
            };
            svg.push_str(&format!(
                "<line x1=\"{x1:.2}\" y1=\"{y1:.2}\" x2=\"{x2:.2}\" y2=\"{y2:.2}\" stroke=\"{}\" stroke-width=\"{}\" stroke-dasharray=\"{}\"/>",
                escape_xml(&theme.line_color),
                gg.branch_stroke_width,
                escape_xml(&gg.branch_dasharray)
            ));

            let color_idx = branch.index % theme.git_colors.len();
            let label_color = theme.git_colors[color_idx].as_str();
            let text_color = theme.git_branch_label_colors[color_idx].as_str();
            let label = &branch.label;

            svg.push_str(&format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.2}\" ry=\"{:.2}\" fill=\"{}\" stroke=\"none\"/>",
                label.bg_x,
                label.bg_y,
                label.bg_width,
                label.bg_height,
                gg.branch_label_corner_radius,
                gg.branch_label_corner_radius,
                escape_xml(label_color)
            ));

            let branch_font_size = if gg.branch_label_font_size > 0.0 {
                gg.branch_label_font_size
            } else {
                theme.font_size
            };
            svg.push_str(&render_gitgraph_multiline_text(
                label.text_x,
                label.text_y,
                &branch.name,
                &theme.font_family,
                branch_font_size,
                gg.branch_label_line_height,
                text_color,
            ));
        }
    }

    if !gitgraph.arrows.is_empty() {
        svg.push_str("<g class=\"commit-arrows\">");
        for arrow in &gitgraph.arrows {
            let color_idx = arrow.color_index % theme.git_colors.len();
            let stroke = theme.git_colors[color_idx].as_str();
            svg.push_str(&format!(
                "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" stroke-linecap=\"round\"/>",
                escape_xml(&arrow.path),
                escape_xml(stroke),
                gg.arrow_stroke_width
            ));
        }
        svg.push_str("</g>");
    }

    svg.push_str("<g class=\"commit-bullets\">");
    for commit in &gitgraph.commits {
        let color_idx = commit.branch_index % theme.git_colors.len();
        let color = theme.git_colors[color_idx].as_str();
        let highlight_color = theme.git_inv_colors[color_idx].as_str();
        let commit_symbol_type = commit.custom_type.unwrap_or(commit.commit_type);
        match commit_symbol_type {
            crate::ir::GitGraphCommitType::Highlight => {
                let outer_size = gg.highlight_outer_size;
                let inner_size = gg.highlight_inner_size;
                svg.push_str(&format!(
                    "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"{}\"/>",
                    commit.x - outer_size / 2.0,
                    commit.y - outer_size / 2.0,
                    outer_size,
                    outer_size,
                    escape_xml(highlight_color),
                    escape_xml(highlight_color)
                ));
                svg.push_str(&format!(
                    "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"{}\"/>",
                    commit.x - inner_size / 2.0,
                    commit.y - inner_size / 2.0,
                    inner_size,
                    inner_size,
                    escape_xml(&theme.primary_color),
                    escape_xml(&theme.primary_color)
                ));
            }
            crate::ir::GitGraphCommitType::CherryPick => {
                svg.push_str(&format!(
                    "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\"/>",
                    commit.x,
                    commit.y,
                    gg.commit_radius,
                    escape_xml(color),
                    escape_xml(color)
                ));
                let accent = escape_xml(&gg.cherry_pick_accent_color);
                svg.push_str(&format!(
                    "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"none\"/>",
                    commit.x - gg.cherry_pick_dot_offset_x,
                    commit.y + gg.cherry_pick_dot_offset_y,
                    gg.cherry_pick_dot_radius,
                    accent
                ));
                svg.push_str(&format!(
                    "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"none\"/>",
                    commit.x + gg.cherry_pick_dot_offset_x,
                    commit.y + gg.cherry_pick_dot_offset_y,
                    gg.cherry_pick_dot_radius,
                    accent
                ));
                svg.push_str(&format!(
                    "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"{}\"/>",
                    commit.x + gg.cherry_pick_dot_offset_x,
                    commit.y + gg.cherry_pick_stem_start_offset_y,
                    commit.x,
                    commit.y + gg.cherry_pick_stem_end_offset_y,
                    accent,
                    gg.cherry_pick_stem_stroke_width
                ));
                svg.push_str(&format!(
                    "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"{}\"/>",
                    commit.x - gg.cherry_pick_dot_offset_x,
                    commit.y + gg.cherry_pick_stem_start_offset_y,
                    commit.x,
                    commit.y + gg.cherry_pick_stem_end_offset_y,
                    accent,
                    gg.cherry_pick_stem_stroke_width
                ));
            }
            _ => {
                let radius = if commit.commit_type == crate::ir::GitGraphCommitType::Merge {
                    gg.merge_radius_outer
                } else {
                    gg.commit_radius
                };
                svg.push_str(&format!(
                    "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\"/>",
                    commit.x,
                    commit.y,
                    radius,
                    escape_xml(color),
                    escape_xml(color)
                ));
                if commit_symbol_type == crate::ir::GitGraphCommitType::Merge {
                    svg.push_str(&format!(
                        "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\"/>",
                        commit.x,
                        commit.y,
                        gg.merge_radius_inner,
                        escape_xml(&theme.primary_color),
                        escape_xml(&theme.primary_color)
                    ));
                }
                if commit_symbol_type == crate::ir::GitGraphCommitType::Reverse {
                    let size = gg.reverse_cross_size;
                    svg.push_str(&format!(
                        "<path d=\"M {x1:.2},{y1:.2} L {x2:.2},{y2:.2} M {x3:.2},{y3:.2} L {x4:.2},{y4:.2}\" stroke=\"{}\" stroke-width=\"{}\" fill=\"none\"/>",
                        escape_xml(&theme.primary_color),
                        gg.reverse_stroke_width,
                        x1 = commit.x - size,
                        y1 = commit.y - size,
                        x2 = commit.x + size,
                        y2 = commit.y + size,
                        x3 = commit.x - size,
                        y3 = commit.y + size,
                        x4 = commit.x + size,
                        y4 = commit.y - size,
                    ));
                }
            }
        }
    }
    svg.push_str("</g>");

    svg.push_str("<g class=\"commit-labels\">");
    for commit in &gitgraph.commits {
        if let Some(label) = &commit.label {
            let mut inner = String::new();
            inner.push_str(&format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" opacity=\"{}\"/>",
                label.bg_x,
                label.bg_y,
                label.bg_width,
                label.bg_height,
                escape_xml(&theme.git_commit_label_background),
                gg.commit_label_bg_opacity
            ));
            inner.push_str(&format!(
                "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"start\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
                label.text_x,
                label.text_y,
                theme.font_family,
                gg.commit_label_font_size,
                escape_xml(&theme.git_commit_label_color),
                escape_xml(&label.text)
            ));
            if let Some(transform) = &label.transform {
                svg.push_str(&format!(
                    "<g transform=\"translate({:.2}, {:.2}) rotate({:.2}, {:.2}, {:.2})\">{}</g>",
                    transform.translate_x,
                    transform.translate_y,
                    transform.rotate_deg,
                    transform.rotate_cx,
                    transform.rotate_cy,
                    inner
                ));
            } else {
                svg.push_str(&inner);
            }
        }

        if !commit.tags.is_empty() {
            for tag in &commit.tags {
                let points = tag
                    .points
                    .iter()
                    .map(|(x, y)| format!("{:.2},{:.2}", x, y))
                    .collect::<Vec<_>>()
                    .join(" ");
                let mut tag_inner = String::new();
                tag_inner.push_str(&format!(
                    "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\"/>",
                    points,
                    escape_xml(&theme.git_tag_label_background),
                    escape_xml(&theme.git_tag_label_border)
                ));
                tag_inner.push_str(&format!(
                    "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\"/>",
                    tag.hole_x,
                    tag.hole_y,
                    gg.tag_hole_radius,
                    escape_xml(&theme.text_color)
                ));
                tag_inner.push_str(&format!(
                    "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"start\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
                    tag.text_x,
                    tag.text_y,
                    theme.font_family,
                    gg.tag_label_font_size,
                    escape_xml(&theme.git_tag_label_color),
                    escape_xml(&tag.text)
                ));
                if let Some(transform) = &tag.transform {
                    svg.push_str(&format!(
                        "<g transform=\"translate({:.2}, {:.2}) rotate({:.2}, {:.2}, {:.2})\">{}</g>",
                        transform.translate_x,
                        transform.translate_y,
                        transform.rotate_deg,
                        transform.rotate_cx,
                        transform.rotate_cy,
                        tag_inner
                    ));
                } else {
                    svg.push_str(&tag_inner);
                }
            }
        }
    }
    svg.push_str("</g>");

    svg.push_str("</g>");
    svg
}

fn render_gitgraph_multiline_text(
    x: f32,
    y: f32,
    text: &str,
    font_family: &str,
    font_size: f32,
    line_height: f32,
    color: &str,
) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    if lines.is_empty() {
        return String::new();
    }
    let start_y = y + font_size;
    let mut out = String::new();
    out.push_str(&format!(
        "<text x=\"{x:.2}\" y=\"{start_y:.2}\" text-anchor=\"start\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">",
        escape_xml(font_family),
        font_size,
        escape_xml(color)
    ));
    let line_height_px = font_size * line_height;
    for (idx, line) in lines.iter().enumerate() {
        let dy = if idx == 0 { 0.0 } else { line_height_px };
        out.push_str(&format!(
            "<tspan x=\"{x:.2}\" dy=\"{dy:.2}\">{}</tspan>",
            escape_xml(line)
        ));
    }
    out.push_str("</text>");
    out
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
    text_block_svg_with_font_size(
        x,
        y,
        label,
        theme,
        config,
        theme.font_size,
        "middle",
        override_color,
        false,
    )
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
    text_block_svg_with_font_size(
        x,
        y,
        label,
        theme,
        config,
        theme.font_size,
        anchor,
        override_color,
        false,
    )
}

fn text_block_svg_with_font_size(
    x: f32,
    y: f32,
    label: &TextBlock,
    theme: &Theme,
    config: &LayoutConfig,
    font_size: f32,
    anchor: &str,
    override_color: Option<&str>,
    baseline: bool,
) -> String {
    let total_height = label.lines.len() as f32 * font_size * config.label_line_height;
    let start_y = if baseline {
        y
    } else {
        y - total_height / 2.0 + font_size
    };
    let mut text = String::new();
    let default_fill = theme.primary_text_color.as_str();
    let fill = override_color.unwrap_or(default_fill);

    text.push_str(&format!(
        "<text x=\"{x:.2}\" y=\"{start_y:.2}\" text-anchor=\"{anchor}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">",
        theme.font_family,
        font_size,
        fill
    ));

    let line_height = font_size * config.label_line_height;
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

const C4_PERSON_ICON: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADAAAAAwCAIAAADYYG7QAAACD0lEQVR4Xu2YoU4EMRCGT+4j8Ai8AhaH4QHgAUjQuFMECUgMIUgwJAgMhgQsAYUiJCiQIBBY+EITsjfTdme6V24v4c8vyGbb+ZjOtN0bNcvjQXmkH83WvYBWto6PLm6v7p7uH1/w2fXD+PBycX1Pv2l3IdDm/vn7x+dXQiAubRzoURa7gRZWd0iGRIiJbOnhnfYBQZNJjNbuyY2eJG8fkDE3bbG4ep6MHUAsgYxmE3nVs6VsBWJSGccsOlFPmLIViMzLOB7pCVO2AtHJMohH7Fh6zqitQK7m0rJvAVYgGcEpe//PLdDz65sM4pF9N7ICcXDKIB5Nv6j7tD0NoSdM2QrU9Gg0ewE1LqBhHR3BBdvj2vapnidjHxD/q6vd7Pvhr31AwcY8eXMTXAKECZZJFXuEq27aLgQK5uLMohCenGGuGewOxSjBvYBqeG6B+Nqiblggdjnc+ZXDy+FNFpFzw76O3UBAROuXh6FoiAcf5g9eTvUgzy0nWg6I8cXHRUpg5bOVBCo+KDpFajOf23GgPme7RSQ+lacIENUgJ6gg1k6HjgOlqnLqip4tEuhv0hNEMXUD0clyXE3p6pZA0S2nnvTlXwLJEZWlb7cTQH1+USgTN4VhAenm/wea1OCAOmqo6fE1WCb9WSKBah+rbUWPWAmE2Rvk0ApiB45eOyNAzU8xcTvj8KvkKEoOaIYeHNA3ZuygAvFMUO0AAAAASUVORK5CYII=";
const C4_EXTERNAL_PERSON_ICON: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADAAAAAwCAIAAADYYG7QAAAB6ElEQVR4Xu2YLY+EMBCG9+dWr0aj0Wg0Go1Go0+j8Xdv2uTCvv1gpt0ebHKPuhDaeW4605Z9mJvx4AdXUyTUdd08z+u6flmWZRnHsWkafk9DptAwDPu+f0eAYtu2PEaGWuj5fCIZrBAC2eLBAnRCsEkkxmeaJp7iDJ2QMDdHsLg8SxKFEJaAo8lAXnmuOFIhTMpxxKATebo4UiFknuNo4OniSIXQyRxEA3YsnjGCVEjVXD7yLUAqxBGUyPv/Y4W2beMgGuS7kVQIBycH0fD+oi5pezQETxdHKmQKGk1eQEYldK+jw5GxPfZ9z7Mk0Qnhf1W1m3w//EUn5BDmSZsbR44QQLBEqrBHqOrmSKaQAxdnLArCrxZcM7A7ZKs4ioRq8LFC+NpC3WCBJsvpVw5edm9iEXFuyNfxXAgSwfrFQ1c0iNda8AdejvUgnktOtJQQxmcfFzGglc5WVCj7oDgFqU18boeFSs52CUh8LE8BIVQDT1ABrB0HtgSEYlX5doJnCwv9TXocKCaKbnwhdDKPq4lf3SwU3HLq4V/+WYhHVMa/3b4IlfyikAduCkcBc7mQ3/z/Qq/cTuikhkzB12Ae/mcJC9U+Vo8Ej1gWAtgbeGgFsAMHr50BIWOLCbezvhpBFUdY6EJuJ/QDW0XoMX60zZ0AAAAASUVORK5CYII=";

fn render_c4(c4: &C4Layout, config: &LayoutConfig) -> String {
    let conf = &config.c4;
    let mut svg = String::new();

    svg.push_str("<defs><symbol id=\"computer\" width=\"24\" height=\"24\"><path transform=\"scale(.5)\" d=\"M2 2v13h20v-13h-20zm18 11h-16v-9h16v9zm-10.228 6l.466-1h3.524l.467 1h-4.457zm14.228 3h-24l2-6h2.104l-1.33 4h18.45l-1.297-4h2.073l2 6zm-5-10h-14v-7h14v7z\"/></symbol></defs>");
    svg.push_str("<defs><symbol id=\"database\" fill-rule=\"evenodd\" clip-rule=\"evenodd\"><path transform=\"scale(.5)\" d=\"M12.258.001l.256.004.255.005.253.008.251.01.249.012.247.015.246.016.242.019.241.02.239.023.236.024.233.027.231.028.229.031.225.032.223.034.22.036.217.038.214.04.211.041.208.043.205.045.201.046.198.048.194.05.191.051.187.053.183.054.18.056.175.057.172.059.168.06.163.061.16.063.155.064.15.066.074.033.073.033.071.034.07.034.069.035.068.035.067.035.066.035.064.036.064.036.062.036.06.036.06.037.058.037.058.037.055.038.055.038.053.038.052.038.051.039.05.039.048.039.047.039.045.04.044.04.043.04.041.04.04.041.039.041.037.041.036.041.034.041.033.042.032.042.03.042.029.042.027.042.026.043.024.043.023.043.021.043.02.043.018.044.017.043.015.044.013.044.012.044.011.045.009.044.007.045.006.045.004.045.002.045.001.045v17l-.001.045-.002.045-.004.045-.006.045-.007.045-.009.044-.011.045-.012.044-.013.044-.015.044-.017.043-.018.044-.02.043-.021.043-.023.043-.024.043-.026.043-.027.042-.029.042-.03.042-.032.042-.033.042-.034.041-.036.041-.037.041-.039.041-.04.041-.041.04-.043.04-.044.04-.045.04-.047.039-.048.039-.05.039-.051.039-.052.038-.053.038-.055.038-.055.038-.058.037-.058.037-.06.037-.06.036-.062.036-.064.036-.064.036-.066.035-.067.035-.068.035-.069.035-.07.034-.071.034-.073.033-.074.033-.15.066-.155.064-.16.063-.163.061-.168.06-.172.059-.175.057-.18.056-.183.054-.187.053-.191.051-.194.05-.198.048-.201.046-.205.045-.208.043-.211.041-.214.04-.217.038-.22.036-.223.034-.225.032-.229.031-.231.028-.233.027-.236.024-.239.023-.241.02-.242.019-.246.016-.247.015-.249.012-.251.01-.253.008-.255.005-.256.004-.258.001-.258-.001-.256-.004-.255-.005-.253-.008-.251-.01-.249-.012-.247-.015-.245-.016-.243-.019-.241-.02-.238-.023-.236-.024-.234-.027-.231-.028-.228-.031-.226-.032-.223-.034-.22-.036-.217-.038-.214-.04-.211-.041-.208-.043-.204-.045-.201-.046-.198-.048-.195-.05-.19-.051-.187-.053-.184-.054-.179-.056-.176-.057-.172-.059-.167-.06-.164-.061-.159-.063-.155-.064-.151-.066-.074-.033-.072-.033-.072-.034-.07-.034-.069-.035-.068-.035-.067-.035-.066-.035-.064-.036-.063-.036-.062-.036-.061-.036-.06-.037-.058-.037-.057-.037-.056-.038-.055-.038-.053-.038-.052-.038-.051-.039-.049-.039-.049-.039-.046-.039-.046-.04-.044-.04-.043-.04-.041-.04-.04-.041-.039-.041-.037-.041-.036-.041-.034-.041-.033-.042-.032-.042-.03-.042-.029-.042-.027-.042-.026-.043-.024-.043-.023-.043-.021-.043-.02-.043-.018-.044-.017-.043-.015-.044-.013-.044-.012-.044-.011-.045-.009-.044-.007-.045-.006-.045-.004-.045-.002-.045-.001-.045v-17l.001-.045.002-.045.004-.045.006-.045.007-.045.009-.044.011-.045.012-.044.013-.044.015-.044.017-.043.018-.044.02-.043.021-.043.023-.043.024-.043.026-.043.027-.042.029-.042.03-.042.032-.042.033-.042.034-.041.036-.041.037-.041.039-.041.04-.041.041-.04.043-.04.044-.04.046-.04.046-.039.049-.039.049-.039.051-.039.052-.038.053-.038.055-.038.056-.038.057-.037.058-.037.06-.037.061-.036.062-.036.063-.036.064-.036.066-.035.067-.035.068-.035.069-.035.07-.034.072-.034.072-.033.074-.033.151-.066.155-.064.159-.063.164-.061.167-.06.172-.059.176-.057.179-.056.184-.054.187-.053.19-.051.195-.05.198-.048.201-.046.204-.045.208-.043.211-.041.214-.04.217-.038.22-.036.223-.034.226-.032.228-.031.231-.028.234-.027.236-.024.238-.023.241-.02.243-.019.245-.016.247-.015.249-.012.251-.01.253-.008.255-.005.256-.004.258-.001.258.001z\"/></symbol></defs>");
    svg.push_str("<defs><symbol id=\"clock\" width=\"24\" height=\"24\"><path transform=\"scale(.5)\" d=\"M12 2c5.514 0 10 4.486 10 10s-4.486 10-10 10-10-4.486-10-10 4.486-10 10-10zm0-2c-6.627 0-12 5.373-12 12s5.373 12 12 12 12-5.373 12-12-5.373-12-12-12zm5.848 12.459c.202.038.202.333.001.372-1.907.361-6.045 1.111-6.547 1.111-.719 0-1.301-.582-1.301-1.301 0-.512.77-5.447 1.125-7.445.034-.192.312-.181.343.014l.985 6.238 5.394 1.011z\"/></symbol></defs>");

    for shape in &c4.shapes {
        svg.push_str(&render_c4_shape(shape, conf));
    }

    for boundary in &c4.boundaries {
        svg.push_str(&render_c4_boundary(boundary, conf));
    }

    svg.push_str("<defs><marker id=\"arrowhead\" refX=\"9\" refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"12\" markerHeight=\"12\" orient=\"auto\"><path d=\"M 0 0 L 10 5 L 0 10 z\"/></marker></defs>");
    svg.push_str("<defs><marker id=\"arrowend\" refX=\"1\" refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"12\" markerHeight=\"12\" orient=\"auto\"><path d=\"M 10 0 L 0 5 L 10 10 z\"/></marker></defs>");
    svg.push_str("<defs><marker id=\"crosshead\" markerWidth=\"15\" markerHeight=\"8\" orient=\"auto\" refX=\"16\" refY=\"4\"><path fill=\"black\" stroke=\"#000000\" stroke-width=\"1px\" d=\"M 9,2 V 6 L16,4 Z\" style=\"stroke-dasharray: 0, 0;\"/><path fill=\"none\" stroke=\"#000000\" stroke-width=\"1px\" d=\"M 0,1 L 6,7 M 6,1 L 0,7\" style=\"stroke-dasharray: 0, 0;\"/></marker></defs>");
    svg.push_str("<defs><marker id=\"filled-head\" refX=\"18\" refY=\"7\" markerWidth=\"20\" markerHeight=\"28\" orient=\"auto\"><path d=\"M 18,7 L9,13 L14,7 L9,1 Z\"/></marker></defs>");

    svg.push_str("<g>");
    for (idx, rel) in c4.rels.iter().enumerate() {
        svg.push_str(&render_c4_rel(rel, conf, idx == 0));
    }
    svg.push_str("</g>");

    svg
}

fn render_c4_shape(shape: &C4ShapeLayout, conf: &crate::config::C4Config) -> String {
    let (default_fill, default_stroke) = c4_shape_colors(conf, shape.kind);
    let fill = shape
        .bg_color
        .as_deref()
        .unwrap_or(default_fill);
    let stroke = shape
        .border_color
        .as_deref()
        .unwrap_or(default_stroke);
    let font_color = shape.font_color.as_deref().unwrap_or("#FFFFFF");
    let fill = escape_xml(fill);
    let stroke = escape_xml(stroke);
    let font_color = escape_xml(font_color);
    let mut svg = String::new();
    svg.push_str("<g class=\"person-man\">");
    match shape.kind {
        crate::ir::C4ShapeKind::SystemDb
        | crate::ir::C4ShapeKind::ExternalSystemDb
        | crate::ir::C4ShapeKind::ContainerDb
        | crate::ir::C4ShapeKind::ExternalContainerDb
        | crate::ir::C4ShapeKind::ComponentDb
        | crate::ir::C4ShapeKind::ExternalComponentDb => {
            let half = shape.width / 2.0;
            let ellipse = conf.db_ellipse_height;
            svg.push_str(&format!(
                "<path fill=\"{}\" stroke-width=\"{}\" stroke=\"{}\" d=\"M{:.0},{:.0}c0,-{ellipse} {half:.0},-{ellipse} {half:.0},-{ellipse}c0,0 {half:.0},0 {half:.0},{ellipse}l0,{:.0}c0,{ellipse} -{half:.0},{ellipse} -{half:.0},{ellipse}c0,0 -{half:.0},0 -{half:.0},-{ellipse}l0,-{:.0}\"/>",
                fill,
                conf.shape_stroke_width,
                stroke,
                shape.x,
                shape.y,
                shape.height,
                shape.height
            ));
            svg.push_str(&format!(
                "<path fill=\"none\" stroke-width=\"{}\" stroke=\"{}\" d=\"M{:.0},{:.0}c0,{ellipse} {half:.0},{ellipse} {half:.0},{ellipse}c0,0 {half:.0},0 {half:.0},-{ellipse}\"/>",
                conf.shape_stroke_width,
                stroke,
                shape.x,
                shape.y,
            ));
        }
        crate::ir::C4ShapeKind::SystemQueue
        | crate::ir::C4ShapeKind::ExternalSystemQueue
        | crate::ir::C4ShapeKind::ContainerQueue
        | crate::ir::C4ShapeKind::ExternalContainerQueue
        | crate::ir::C4ShapeKind::ComponentQueue
        | crate::ir::C4ShapeKind::ExternalComponentQueue => {
            let half = shape.height / 2.0;
            let curve = conf.queue_curve_radius;
            svg.push_str(&format!(
                "<path fill=\"{}\" stroke-width=\"{}\" stroke=\"{}\" d=\"M{:.0},{:.0}l{:.0},0c{curve},0 {curve},{half} {curve},{half}c0,0 0,{half} -{curve},{half}l-{:.0},0c-{curve},0 -{curve},-{half} -{curve},-{half}c0,0 0,-{half} {curve},-{half}\"/>",
                fill,
                conf.shape_stroke_width,
                stroke,
                shape.x,
                shape.y,
                shape.width,
                shape.width
            ));
            svg.push_str(&format!(
                "<path fill=\"none\" stroke-width=\"{}\" stroke=\"{}\" d=\"M{:.0},{:.0}c-{curve},0 -{curve},{half} -{curve},{half}c0,{half} {curve},{half} {curve},{half}\"/>",
                conf.shape_stroke_width,
                stroke,
                shape.x + shape.width,
                shape.y,
            ));
        }
        _ => {
            svg.push_str(&format!(
                "<rect x=\"{:.0}\" y=\"{:.0}\" fill=\"{}\" stroke=\"{}\" width=\"{:.0}\" height=\"{:.0}\" rx=\"{:.1}\" ry=\"{:.1}\" stroke-width=\"{}\"/>",
                shape.x,
                shape.y,
                fill,
                stroke,
                shape.width,
                shape.height,
                conf.shape_corner_radius,
                conf.shape_corner_radius,
                conf.shape_stroke_width
            ));
        }
    }

    let type_font_size = c4_shape_font_size(conf, shape.kind) - 2.0;
    let type_font_family = c4_shape_font_family(conf, shape.kind);
    svg.push_str(&format!(
        "<text fill=\"{}\" font-family=\"{}\" font-size=\"{}\" font-style=\"italic\" lengthAdjust=\"spacing\" textLength=\"{:.0}\" x=\"{:.0}\" y=\"{:.0}\">{}</text>",
        font_color,
        escape_xml(type_font_family),
        type_font_size,
        shape.type_label.width.round(),
        shape.x + shape.width / 2.0 - shape.type_label.width / 2.0,
        shape.y + shape.type_label.y,
        escape_xml(&shape.type_label.text)
    ));

    if let Some(image_y) = shape.image_y {
        if matches!(
            shape.kind,
            crate::ir::C4ShapeKind::Person | crate::ir::C4ShapeKind::ExternalPerson
        ) {
            let icon = match shape.kind {
                crate::ir::C4ShapeKind::ExternalPerson => C4_EXTERNAL_PERSON_ICON,
                crate::ir::C4ShapeKind::Person => C4_PERSON_ICON,
                _ => C4_PERSON_ICON,
            };
            svg.push_str(&format!(
                "<image width=\"{:.0}\" height=\"{:.0}\" x=\"{:.0}\" y=\"{:.0}\" xlink:href=\"{}\"/>",
                conf.person_icon_size,
                conf.person_icon_size,
                shape.x + shape.width / 2.0 - conf.person_icon_size / 2.0,
                shape.y + image_y,
                icon
            ));
        }
    }

    let label_font_size = c4_shape_font_size(conf, shape.kind) + 2.0;
    let label_font_family = c4_shape_font_family(conf, shape.kind);
    let label_font_weight = "bold";
    svg.push_str(&c4_text_svg(
        shape.x + shape.width / 2.0,
        shape.y + shape.label.y,
        &shape.label.lines,
        label_font_family,
        label_font_size,
        label_font_weight,
        &font_color,
        false,
    ));

    if let Some(type_or_techn) = &shape.type_or_techn {
        let font_family = c4_shape_font_family(conf, shape.kind);
        let font_weight = c4_shape_font_weight(conf, shape.kind);
        let font_size = c4_shape_font_size(conf, shape.kind);
        svg.push_str(&c4_text_svg(
            shape.x + shape.width / 2.0,
            shape.y + type_or_techn.y,
            &type_or_techn.lines,
            font_family,
            font_size,
            font_weight,
            &font_color,
            true,
        ));
    }

    if let Some(descr) = &shape.descr {
        let font_family = c4_shape_font_family(conf, shape.kind);
        let font_weight = c4_shape_font_weight(conf, shape.kind);
        let font_size = c4_shape_font_size(conf, shape.kind);
        svg.push_str(&c4_text_svg(
            shape.x + shape.width / 2.0,
            shape.y + descr.y,
            &descr.lines,
            font_family,
            font_size,
            font_weight,
            &font_color,
            false,
        ));
    }

    svg.push_str("</g>");
    svg
}

fn render_c4_boundary(boundary: &C4BoundaryLayout, conf: &crate::config::C4Config) -> String {
    let mut svg = String::new();
    svg.push_str("<g>");
    let fill = boundary
        .bg_color
        .as_deref()
        .unwrap_or(&conf.boundary_fill);
    let stroke = boundary
        .border_color
        .as_deref()
        .unwrap_or(&conf.boundary_stroke);
    let font_color = boundary
        .font_color
        .as_deref()
        .unwrap_or(&conf.boundary_stroke);
    let fill_attr = escape_xml(fill);
    let stroke_attr = escape_xml(stroke);
    let font_color_attr = escape_xml(font_color);
    let mut rect_attrs = format!(
        "<rect x=\"{:.0}\" y=\"{:.0}\" fill=\"{}\" stroke=\"{}\" width=\"{:.0}\" height=\"{:.0}\" rx=\"{:.1}\" ry=\"{:.1}\" stroke-width=\"{}\"",
        boundary.x,
        boundary.y,
        fill_attr,
        stroke_attr,
        boundary.width,
        boundary.height,
        conf.boundary_corner_radius,
        conf.boundary_corner_radius,
        conf.boundary_stroke_width
    );
    if !conf.boundary_dasharray.is_empty() {
        rect_attrs.push_str(&format!(" stroke-dasharray=\"{}\"", escape_xml(&conf.boundary_dasharray)));
    }
    if conf.boundary_fill != "none" && conf.boundary_fill_opacity < 1.0 {
        rect_attrs.push_str(&format!(" fill-opacity=\"{:.2}\"", conf.boundary_fill_opacity));
    }
    rect_attrs.push_str("/>");
    svg.push_str(&rect_attrs);

    let label_font_size = conf.boundary_font_size + 2.0;
    svg.push_str(&c4_text_svg(
        boundary.x + boundary.width / 2.0,
        boundary.y + boundary.label.y,
        &boundary.label.lines,
        &conf.boundary_font_family,
        label_font_size,
        "bold",
        &font_color_attr,
        false,
    ));

    if let Some(boundary_type) = &boundary.boundary_type {
        svg.push_str(&c4_text_svg(
            boundary.x + boundary.width / 2.0,
            boundary.y + boundary_type.y,
            &boundary_type.lines,
            &conf.boundary_font_family,
            conf.boundary_font_size,
            &conf.boundary_font_weight,
            &font_color_attr,
            false,
        ));
    }

    if let Some(descr) = &boundary.descr {
        svg.push_str(&c4_text_svg(
            boundary.x + boundary.width / 2.0,
            boundary.y + descr.y,
            &descr.lines,
            &conf.boundary_font_family,
            conf.boundary_font_size - 2.0,
            &conf.boundary_font_weight,
            &font_color_attr,
            false,
        ));
    }

    svg.push_str("</g>");
    svg
}

fn render_c4_rel(rel: &C4RelLayout, conf: &crate::config::C4Config, straight: bool) -> String {
    let mut svg = String::new();
    let stroke = rel
        .line_color
        .as_deref()
        .unwrap_or(&conf.boundary_stroke);
    if straight {
        let mut attrs = String::new();
        if rel.kind != crate::ir::C4RelKind::RelBack {
            attrs.push_str(" marker-end=\"url(#arrowhead)\"");
        }
        if matches!(rel.kind, crate::ir::C4RelKind::BiRel | crate::ir::C4RelKind::RelBack) {
            attrs.push_str(" marker-start=\"url(#arrowend)\"");
        }
        svg.push_str(&format!(
            "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke-width=\"1\" stroke=\"{}\" style=\"fill: none;\"{attrs} />",
            rel.start.0,
            rel.start.1,
            rel.end.0,
            rel.end.1,
            escape_xml(stroke),
        ));
    } else {
        let control_x = rel.start.0 + (rel.end.0 - rel.start.0) / 4.0;
        let control_y = rel.start.1 + (rel.end.1 - rel.start.1) / 2.0;
        let mut path = format!(
            "<path fill=\"none\" stroke-width=\"1\" stroke=\"{}\" d=\"M{:.2},{:.2} Q{:.2},{:.2} {:.2},{:.2}\"",
            escape_xml(stroke),
            rel.start.0,
            rel.start.1,
            control_x,
            control_y,
            rel.end.0,
            rel.end.1
        );
        if rel.kind != crate::ir::C4RelKind::RelBack {
            path.push_str(" marker-end=\"url(#arrowhead)\"");
        }
        if matches!(rel.kind, crate::ir::C4RelKind::BiRel | crate::ir::C4RelKind::RelBack) {
            path.push_str(" marker-start=\"url(#arrowend)\"");
        }
        path.push_str("/>");
        svg.push_str(&path);
    }

    let text_color = rel
        .text_color
        .as_deref()
        .unwrap_or(&conf.boundary_stroke);
    let mid_x = rel.start.0.min(rel.end.0) + (rel.start.0 - rel.end.0).abs() / 2.0 + rel.offset_x;
    let mid_y = rel.start.1.min(rel.end.1) + (rel.start.1 - rel.end.1).abs() / 2.0 + rel.offset_y;
    svg.push_str(&c4_text_svg(
        mid_x,
        mid_y,
        &rel.label.lines,
        &conf.message_font_family,
        conf.message_font_size,
        &conf.message_font_weight,
        text_color,
        false,
    ));
    if let Some(techn) = &rel.techn {
        svg.push_str(&c4_text_svg(
            mid_x,
            mid_y + conf.message_font_size + 5.0,
            &techn.lines,
            &conf.message_font_family,
            conf.message_font_size,
            &conf.message_font_weight,
            text_color,
            true,
        ));
    }
    svg
}

fn c4_text_svg(
    x: f32,
    y: f32,
    lines: &[String],
    font_family: &str,
    font_size: f32,
    font_weight: &str,
    fill: &str,
    italic: bool,
) -> String {
    let mut out = String::new();
    let line_count = lines.len() as f32;
    for (idx, line) in lines.iter().enumerate() {
        let dy = idx as f32 * font_size - font_size * (line_count - 1.0) / 2.0;
        out.push_str(&format!(
            "<text x=\"{x:.2}\" y=\"{y:.2}\" dominant-baseline=\"middle\" fill=\"{}\" style=\"text-anchor: middle; font-size: {}px; font-weight: {}; font-family: {}\"{}><tspan dy=\"{dy:.2}\" alignment-baseline=\"mathematical\">{}</tspan></text>",
            escape_xml(fill),
            font_size,
            escape_xml(font_weight),
            escape_xml(font_family),
            if italic { " font-style=\"italic\"" } else { "" },
            escape_xml(line)
        ));
    }
    out
}

fn c4_shape_colors(conf: &crate::config::C4Config, kind: crate::ir::C4ShapeKind) -> (&str, &str) {
    match kind {
        crate::ir::C4ShapeKind::Person => (&conf.person_bg_color, &conf.person_border_color),
        crate::ir::C4ShapeKind::ExternalPerson => (
            &conf.external_person_bg_color,
            &conf.external_person_border_color,
        ),
        crate::ir::C4ShapeKind::System => (&conf.system_bg_color, &conf.system_border_color),
        crate::ir::C4ShapeKind::SystemDb => (&conf.system_db_bg_color, &conf.system_db_border_color),
        crate::ir::C4ShapeKind::SystemQueue => (
            &conf.system_queue_bg_color,
            &conf.system_queue_border_color,
        ),
        crate::ir::C4ShapeKind::ExternalSystem => (
            &conf.external_system_bg_color,
            &conf.external_system_border_color,
        ),
        crate::ir::C4ShapeKind::ExternalSystemDb => (
            &conf.external_system_db_bg_color,
            &conf.external_system_db_border_color,
        ),
        crate::ir::C4ShapeKind::ExternalSystemQueue => (
            &conf.external_system_queue_bg_color,
            &conf.external_system_queue_border_color,
        ),
        crate::ir::C4ShapeKind::Container => (&conf.container_bg_color, &conf.container_border_color),
        crate::ir::C4ShapeKind::ContainerDb => (
            &conf.container_db_bg_color,
            &conf.container_db_border_color,
        ),
        crate::ir::C4ShapeKind::ContainerQueue => (
            &conf.container_queue_bg_color,
            &conf.container_queue_border_color,
        ),
        crate::ir::C4ShapeKind::ExternalContainer => (
            &conf.external_container_bg_color,
            &conf.external_container_border_color,
        ),
        crate::ir::C4ShapeKind::ExternalContainerDb => (
            &conf.external_container_db_bg_color,
            &conf.external_container_db_border_color,
        ),
        crate::ir::C4ShapeKind::ExternalContainerQueue => (
            &conf.external_container_queue_bg_color,
            &conf.external_container_queue_border_color,
        ),
        crate::ir::C4ShapeKind::Component => (&conf.component_bg_color, &conf.component_border_color),
        crate::ir::C4ShapeKind::ComponentDb => (
            &conf.component_db_bg_color,
            &conf.component_db_border_color,
        ),
        crate::ir::C4ShapeKind::ComponentQueue => (
            &conf.component_queue_bg_color,
            &conf.component_queue_border_color,
        ),
        crate::ir::C4ShapeKind::ExternalComponent => (
            &conf.external_component_bg_color,
            &conf.external_component_border_color,
        ),
        crate::ir::C4ShapeKind::ExternalComponentDb => (
            &conf.external_component_db_bg_color,
            &conf.external_component_db_border_color,
        ),
        crate::ir::C4ShapeKind::ExternalComponentQueue => (
            &conf.external_component_queue_bg_color,
            &conf.external_component_queue_border_color,
        ),
    }
}

fn c4_shape_font_family(conf: &crate::config::C4Config, kind: crate::ir::C4ShapeKind) -> &str {
    match kind {
        crate::ir::C4ShapeKind::Person => &conf.person_font_family,
        crate::ir::C4ShapeKind::ExternalPerson => &conf.external_person_font_family,
        crate::ir::C4ShapeKind::System => &conf.system_font_family,
        crate::ir::C4ShapeKind::SystemDb => &conf.system_db_font_family,
        crate::ir::C4ShapeKind::SystemQueue => &conf.system_queue_font_family,
        crate::ir::C4ShapeKind::ExternalSystem => &conf.external_system_font_family,
        crate::ir::C4ShapeKind::ExternalSystemDb => &conf.external_system_db_font_family,
        crate::ir::C4ShapeKind::ExternalSystemQueue => &conf.external_system_queue_font_family,
        crate::ir::C4ShapeKind::Container => &conf.container_font_family,
        crate::ir::C4ShapeKind::ContainerDb => &conf.container_db_font_family,
        crate::ir::C4ShapeKind::ContainerQueue => &conf.container_queue_font_family,
        crate::ir::C4ShapeKind::ExternalContainer => &conf.external_container_font_family,
        crate::ir::C4ShapeKind::ExternalContainerDb => &conf.external_container_db_font_family,
        crate::ir::C4ShapeKind::ExternalContainerQueue => &conf.external_container_queue_font_family,
        crate::ir::C4ShapeKind::Component => &conf.component_font_family,
        crate::ir::C4ShapeKind::ComponentDb => &conf.component_db_font_family,
        crate::ir::C4ShapeKind::ComponentQueue => &conf.component_queue_font_family,
        crate::ir::C4ShapeKind::ExternalComponent => &conf.external_component_font_family,
        crate::ir::C4ShapeKind::ExternalComponentDb => &conf.external_component_db_font_family,
        crate::ir::C4ShapeKind::ExternalComponentQueue => &conf.external_component_queue_font_family,
    }
}

fn c4_shape_font_size(conf: &crate::config::C4Config, kind: crate::ir::C4ShapeKind) -> f32 {
    match kind {
        crate::ir::C4ShapeKind::Person => conf.person_font_size,
        crate::ir::C4ShapeKind::ExternalPerson => conf.external_person_font_size,
        crate::ir::C4ShapeKind::System => conf.system_font_size,
        crate::ir::C4ShapeKind::SystemDb => conf.system_db_font_size,
        crate::ir::C4ShapeKind::SystemQueue => conf.system_queue_font_size,
        crate::ir::C4ShapeKind::ExternalSystem => conf.external_system_font_size,
        crate::ir::C4ShapeKind::ExternalSystemDb => conf.external_system_db_font_size,
        crate::ir::C4ShapeKind::ExternalSystemQueue => conf.external_system_queue_font_size,
        crate::ir::C4ShapeKind::Container => conf.container_font_size,
        crate::ir::C4ShapeKind::ContainerDb => conf.container_db_font_size,
        crate::ir::C4ShapeKind::ContainerQueue => conf.container_queue_font_size,
        crate::ir::C4ShapeKind::ExternalContainer => conf.external_container_font_size,
        crate::ir::C4ShapeKind::ExternalContainerDb => conf.external_container_db_font_size,
        crate::ir::C4ShapeKind::ExternalContainerQueue => conf.external_container_queue_font_size,
        crate::ir::C4ShapeKind::Component => conf.component_font_size,
        crate::ir::C4ShapeKind::ComponentDb => conf.component_db_font_size,
        crate::ir::C4ShapeKind::ComponentQueue => conf.component_queue_font_size,
        crate::ir::C4ShapeKind::ExternalComponent => conf.external_component_font_size,
        crate::ir::C4ShapeKind::ExternalComponentDb => conf.external_component_db_font_size,
        crate::ir::C4ShapeKind::ExternalComponentQueue => conf.external_component_queue_font_size,
    }
}

fn c4_shape_font_weight(conf: &crate::config::C4Config, kind: crate::ir::C4ShapeKind) -> &str {
    match kind {
        crate::ir::C4ShapeKind::Person => &conf.person_font_weight,
        crate::ir::C4ShapeKind::ExternalPerson => &conf.external_person_font_weight,
        crate::ir::C4ShapeKind::System => &conf.system_font_weight,
        crate::ir::C4ShapeKind::SystemDb => &conf.system_db_font_weight,
        crate::ir::C4ShapeKind::SystemQueue => &conf.system_queue_font_weight,
        crate::ir::C4ShapeKind::ExternalSystem => &conf.external_system_font_weight,
        crate::ir::C4ShapeKind::ExternalSystemDb => &conf.external_system_db_font_weight,
        crate::ir::C4ShapeKind::ExternalSystemQueue => &conf.external_system_queue_font_weight,
        crate::ir::C4ShapeKind::Container => &conf.container_font_weight,
        crate::ir::C4ShapeKind::ContainerDb => &conf.container_db_font_weight,
        crate::ir::C4ShapeKind::ContainerQueue => &conf.container_queue_font_weight,
        crate::ir::C4ShapeKind::ExternalContainer => &conf.external_container_font_weight,
        crate::ir::C4ShapeKind::ExternalContainerDb => &conf.external_container_db_font_weight,
        crate::ir::C4ShapeKind::ExternalContainerQueue => &conf.external_container_queue_font_weight,
        crate::ir::C4ShapeKind::Component => &conf.component_font_weight,
        crate::ir::C4ShapeKind::ComponentDb => &conf.component_db_font_weight,
        crate::ir::C4ShapeKind::ComponentQueue => &conf.component_queue_font_weight,
        crate::ir::C4ShapeKind::ExternalComponent => &conf.external_component_font_weight,
        crate::ir::C4ShapeKind::ExternalComponentDb => &conf.external_component_db_font_weight,
        crate::ir::C4ShapeKind::ExternalComponentQueue => &conf.external_component_queue_font_weight,
    }
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
    if let Some(color) = parse_hex_color(&theme.background) {
        pixmap.fill(color);
    }

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

#[cfg(feature = "png")]
fn parse_hex_color(input: &str) -> Option<resvg::tiny_skia::Color> {
    let color = input.trim();
    let hex = color.strip_prefix('#')?;
    let (r, g, b, a) = match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            (r, g, b, 255)
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            (r, g, b, 255)
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            (r, g, b, a)
        }
        _ => return None,
    };
    Some(resvg::tiny_skia::Color::from_rgba8(r, g, b, a))
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
        // Crow's foot notation for ER diagrams
        crate::ir::EdgeDecoration::CrowsFootOne => format!(
            "<path d=\"M 0 -6 L 0 6 M 5 -6 L 5 6\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{join}/>",
            stroke, stroke_width
        ),
        crate::ir::EdgeDecoration::CrowsFootZeroOne => format!(
            "<g><circle cx=\"-4\" cy=\"0\" r=\"4\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/><path d=\"M 4 -6 L 4 6\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{join}/></g>",
            stroke, stroke_width, stroke, stroke_width
        ),
        crate::ir::EdgeDecoration::CrowsFootMany => format!(
            "<path d=\"M 0 -6 L 0 6 M 0 0 L 8 -6 M 0 0 L 8 6\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{join}/>",
            stroke, stroke_width
        ),
        crate::ir::EdgeDecoration::CrowsFootZeroMany => format!(
            "<g><circle cx=\"-4\" cy=\"0\" r=\"4\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/><path d=\"M 4 0 L 12 -6 M 4 0 L 12 6\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{join}/></g>",
            stroke, stroke_width, stroke, stroke_width
        ),
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

fn shape_svg(node: &crate::layout::NodeLayout, theme: &Theme, config: &LayoutConfig) -> String {
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
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"3\" ry=\"3\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
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
        crate::ir::NodeShape::MindmapDefault => {
            let rd = config.mindmap.default_corner_radius.max(0.0);
            let inner_h = (h - 2.0 * rd).max(0.0);
            let inner_w = (w - 2.0 * rd).max(0.0);
            let rect_path = format!(
                "M{:.2} {:.2} v{:.2} q0,-{rd:.2} {rd:.2},-{rd:.2} h{:.2} q{rd:.2},0 {rd:.2},{rd:.2} v{:.2} q0,{rd:.2} -{rd:.2},{rd:.2} h{:.2} q-{rd:.2},0 -{rd:.2},-{rd:.2} Z",
                x,
                y + h - rd,
                -inner_h,
                inner_w,
                inner_h,
                -inner_w
            );
            let stroke_width = node.style.stroke_width.unwrap_or(1.0);
            let mut svg = format!(
                "<path d=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
                rect_path, fill, stroke, stroke_width
            );
            let line_color = node.style.line_color.as_ref().unwrap_or(stroke);
            let line_width = config.mindmap.divider_line_width;
            svg.push_str(&format!(
                "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"{:.2}\"/>",
                x,
                y + h,
                x + w,
                y + h,
                line_color,
                line_width
            ));
            svg
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
