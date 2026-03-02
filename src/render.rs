use crate::config::LayoutConfig;
#[cfg(feature = "png")]
use crate::config::RenderConfig;
use crate::layout::label_placement::{
    edge_endpoint_label_position, edge_label_padding, endpoint_label_padding,
};
use crate::layout::{
    C4BoundaryLayout, C4Layout, C4RelLayout, C4ShapeLayout, DiagramData, ErrorLayout,
    GitGraphLayout, JourneyLayout, Layout, PieData, SankeyLayout, TextBlock,
};
use crate::text_metrics;
use crate::theme::{Theme, adjust_color, parse_color_to_hsl};
use anyhow::Result;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::Path;

fn fit_dimensions_to_preferred_ratio(
    width: f32,
    height: f32,
    preferred_ratio: Option<f32>,
) -> (f32, f32) {
    let mut width = width.max(1.0);
    let mut height = height.max(1.0);
    let Some(target_ratio) = preferred_ratio else {
        return (width, height);
    };
    if !target_ratio.is_finite() || target_ratio <= 0.0 {
        return (width, height);
    }
    let current_ratio = width / height;
    if (current_ratio - target_ratio).abs() < 1e-6 {
        return (width, height);
    }
    if current_ratio < target_ratio {
        width = height * target_ratio;
    } else {
        height = width / target_ratio;
    }
    (width.max(1.0), height.max(1.0))
}

fn edge_dom_id(edge_idx: usize) -> String {
    format!("edge-{edge_idx}")
}

pub fn render_svg(layout: &Layout, theme: &Theme, config: &LayoutConfig) -> String {
    let mut svg = String::new();
    let state_font_size = if layout.kind == crate::ir::DiagramKind::State {
        theme.font_size * 0.85
    } else {
        theme.font_size
    };
    let (width, height, viewbox_x, viewbox_y, viewbox_width, viewbox_height) =
        if let DiagramData::Error(error) = &layout.diagram {
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
        } else if let DiagramData::C4(c4) = &layout.diagram {
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
        } else if let DiagramData::GitGraph(gitgraph) = &layout.diagram {
            let width = layout.width.max(1.0);
            let height = layout.height.max(1.0);
            let viewbox_x = -gitgraph.offset_x;
            let viewbox_y = -gitgraph.offset_y;
            (
                width,
                height,
                viewbox_x,
                viewbox_y,
                gitgraph.width,
                gitgraph.height,
            )
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
    let seq_data = if let DiagramData::Sequence(s) = &layout.diagram {
        Some(s)
    } else {
        None
    };
    let is_sequence = seq_data.is_some();
    let is_state = layout.kind == crate::ir::DiagramKind::State;
    let is_class = layout.kind == crate::ir::DiagramKind::Class;
    let is_c4 = matches!(layout.diagram, DiagramData::C4(_));
    let has_links = is_c4
        || layout.nodes.values().any(|node| node.link.is_some())
        || seq_data
            .iter()
            .flat_map(|s| s.footboxes.iter())
            .any(|node| node.link.is_some());

    let preferred_ratio = config
        .preferred_aspect_ratio
        .filter(|ratio| ratio.is_finite() && *ratio > 0.0);
    let (target_width, target_height) =
        fit_dimensions_to_preferred_ratio(width, height, preferred_ratio);

    let mut width_attr = target_width.to_string();
    let mut height_attr = target_height.to_string();
    let mut style_attr = String::new();
    let preferred_ratio_style = preferred_ratio
        .map(|ratio| format!("aspect-ratio: {:.6};", ratio))
        .unwrap_or_default();
    if !matches!(layout.diagram, DiagramData::Error(_)) {
        if let DiagramData::C4(c4) = &layout.diagram {
            if c4.use_max_width {
                width_attr = "100%".to_string();
                height_attr.clear();
                style_attr = format!(
                    " style=\"max-width: {:.3}px;{}\"",
                    viewbox_width, preferred_ratio_style
                );
            }
        } else if matches!(layout.diagram, DiagramData::GitGraph(_))
            && config.gitgraph.use_max_width
        {
            width_attr = "100%".to_string();
            height_attr.clear();
            style_attr = format!(
                " style=\"max-width: {:.3}px;{}\"",
                viewbox_width, preferred_ratio_style
            );
        } else if layout.kind == crate::ir::DiagramKind::Mindmap && config.mindmap.use_max_width {
            width_attr = "100%".to_string();
            height_attr.clear();
            style_attr = format!(
                " style=\"max-width: {:.3}px;{}\"",
                viewbox_width, preferred_ratio_style
            );
        } else if layout.kind == crate::ir::DiagramKind::Pie && config.pie.use_max_width {
            width_attr = "100%".to_string();
            height_attr.clear();
            style_attr = format!(
                " style=\"max-width: {:.3}px;{}\"",
                viewbox_width, preferred_ratio_style
            );
        } else if !preferred_ratio_style.is_empty() {
            style_attr = format!(" style=\"{preferred_ratio_style}\"");
        }
    } else if !preferred_ratio_style.is_empty() {
        style_attr = format!(" style=\"{preferred_ratio_style}\"");
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

    if matches!(layout.diagram, DiagramData::Error(_)) {
        svg.push_str(&error_style_block(theme));
    }

    svg.push_str(&format!(
        "<rect x=\"{viewbox_x}\" y=\"{viewbox_y}\" width=\"{viewbox_width}\" height=\"{viewbox_height}\" fill=\"{}\"/>",
        theme.background
    ));

    if let DiagramData::C4(ref c4) = layout.diagram {
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
            "<marker id=\"arrow-{idx}\" viewBox=\"0 0 10 10\" refX=\"5\" refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto\"><path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"1,0\"/></marker>",
            color, color
        ));
        svg.push_str(&format!(
            "<marker id=\"arrow-start-{idx}\" viewBox=\"0 0 10 10\" refX=\"4.5\" refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto\"><path d=\"M 0 5 L 10 10 L 10 0 z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"1,0\"/></marker>",
            color, color
        ));
        if is_sequence {
            svg.push_str(&format!(
                "<marker id=\"arrow-seq-{idx}\" viewBox=\"-1 0 12 10\" refX=\"7.9\" refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"12\" markerHeight=\"12\" orient=\"auto-start-reverse\"><path d=\"M -1 0 L 10 5 L 0 10 z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"1,0\"/></marker>",
                color,
                color
            ));
            svg.push_str(&format!(
                "<marker id=\"arrow-start-seq-{idx}\" viewBox=\"-1 0 12 10\" refX=\"2.1\" refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"12\" markerHeight=\"12\" orient=\"auto\"><path d=\"M 11 0 L 0 5 L 11 10 z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"1,0\"/></marker>",
                color,
                color
            ));
        }
        if is_state {
            svg.push_str(&format!(
                "<marker id=\"arrow-state-{idx}\" viewBox=\"0 0 20 14\" refX=\"19\" refY=\"7\" markerUnits=\"userSpaceOnUse\" markerWidth=\"20\" markerHeight=\"14\" orient=\"auto\"><path d=\"M 19 7 L 9 13 L 14 7 L 9 1 Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"1,0\"/></marker>",
                color, color
            ));
        }
        if is_class {
            svg.push_str(&format!(
                "<marker id=\"arrow-class-open-{idx}\" viewBox=\"0 0 20 14\" refX=\"1\" refY=\"7\" markerUnits=\"userSpaceOnUse\" markerWidth=\"20\" markerHeight=\"14\" orient=\"auto\"><path d=\"M 1 7 L 18 13 V 1 Z\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"1,0\"/></marker>",
                color
            ));
            svg.push_str(&format!(
                "<marker id=\"arrow-class-open-start-{idx}\" viewBox=\"0 0 20 14\" refX=\"18\" refY=\"7\" markerUnits=\"userSpaceOnUse\" markerWidth=\"20\" markerHeight=\"14\" orient=\"auto\"><path d=\"M 1 7 L 18 13 V 1 Z\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"1,0\"/></marker>",
                color
            ));
            svg.push_str(&format!(
                "<marker id=\"arrow-class-dep-{idx}\" viewBox=\"0 0 20 14\" refX=\"13\" refY=\"7\" markerUnits=\"userSpaceOnUse\" markerWidth=\"20\" markerHeight=\"14\" orient=\"auto\"><path d=\"M 18 7 L 9 13 L 14 7 L 9 1 Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"1,0\"/></marker>",
                color, color
            ));
            svg.push_str(&format!(
                "<marker id=\"arrow-class-dep-start-{idx}\" viewBox=\"0 0 20 14\" refX=\"6\" refY=\"7\" markerUnits=\"userSpaceOnUse\" markerWidth=\"20\" markerHeight=\"14\" orient=\"auto\"><path d=\"M 5 7 L 9 13 L 1 7 L 9 1 Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"1,0\"/></marker>",
                color, color
            ));
        }
    }
    svg.push_str("</defs>");

    if let DiagramData::Error(ref error) = layout.diagram {
        svg.push_str(&render_error(error, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let DiagramData::Sankey(ref sankey) = layout.diagram {
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

    if let DiagramData::Pie(ref pie) = layout.diagram {
        svg.push_str(&render_pie(pie, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let DiagramData::Quadrant(ref quadrant) = layout.diagram {
        svg.push_str(&render_quadrant(quadrant, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let DiagramData::Gantt(ref gantt) = layout.diagram {
        svg.push_str(&render_gantt(gantt, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let DiagramData::XYChart(ref xychart) = layout.diagram {
        svg.push_str(&render_xychart(xychart, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let DiagramData::Timeline(ref timeline) = layout.diagram {
        svg.push_str(&render_timeline(timeline, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let DiagramData::Journey(ref journey) = layout.diagram {
        svg.push_str(&render_journey(journey, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let DiagramData::GitGraph(ref gitgraph) = layout.diagram {
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
            let header_h = if label_empty {
                0.0
            } else {
                (subgraph.label_block.height + theme.font_size * 0.75).max(theme.font_size * 1.4)
            };
            let header_fill = if sub_fill.as_str() == "none" {
                "none".to_string()
            } else {
                adjust_color(sub_fill, 0.0, 0.0, -4.0)
            };
            let body_fill = if sub_fill.as_str() == "none" {
                theme.background.clone()
            } else {
                adjust_color(sub_fill, 0.0, -12.0, 10.0)
            };
            if header_h > 0.0 {
                svg.push_str(&format!(
                    "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"none\"/>",
                    subgraph.x,
                    subgraph.y,
                    subgraph.width,
                    header_h,
                    header_fill
                ));
            }
            let inner_y = subgraph.y + header_h;
            let inner_h = (subgraph.height - header_h).max(0.0);
            if inner_h > 0.0 {
                svg.push_str(&format!(
                    "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"none\"/>",
                    subgraph.x,
                    inner_y,
                    subgraph.width,
                    inner_h,
                    body_fill
                ));
            }
            if header_h > 0.0 {
                svg.push_str(&format!(
                    "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"1\"/>",
                    subgraph.x,
                    inner_y,
                    subgraph.x + subgraph.width,
                    inner_y,
                    sub_stroke
                ));
            }
            svg.push_str(&format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"6\" ry=\"6\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/>",
                subgraph.x,
                subgraph.y,
                subgraph.width,
                subgraph.height,
                sub_stroke,
                sub_stroke_width
            ));
            if !label_empty {
                let label_pad_x = (theme.font_size * 0.6).max(subgraph.label_block.height * 0.35);
                let label_x = subgraph.x + label_pad_x;
                let label_y = subgraph.y + header_h / 2.0;
                svg.push_str(&text_block_svg_with_font_size_weight(
                    label_x,
                    label_y,
                    &subgraph.label_block,
                    theme,
                    config,
                    state_font_size,
                    "start",
                    subgraph.style.text_color.as_deref(),
                    Some("600"),
                    false,
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

    let overlay_flowchart = layout.kind == crate::ir::DiagramKind::Flowchart;
    let mut overlay_arrows: Vec<(bool, (f32, f32), f32, String, f32)> = Vec::new();

    if let Some(seq) = seq_data {
        for seq_box in &seq.boxes {
            let stroke = theme.primary_border_color.as_str();
            let fill = seq_box.color.as_deref().unwrap_or("none");
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

    for frame in seq_data.map(|s| s.frames.as_slice()).unwrap_or_default() {
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

    for lifeline in seq_data.map(|s| s.lifelines.as_slice()).unwrap_or_default() {
        svg.push_str(&format!(
            "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
            lifeline.x,
            lifeline.y1,
            lifeline.x,
            lifeline.y2,
            theme.sequence_actor_line
        ));
    }

    for activation in seq_data
        .map(|s| s.activations.as_slice())
        .unwrap_or_default()
    {
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

    for note in seq_data.map(|s| s.notes.as_slice()).unwrap_or_default() {
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

    if let DiagramData::Graph { state_notes } = &layout.diagram {
        for note in state_notes {
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
            svg.push_str(&text_block_svg_with_font_size(
                center_x,
                center_y,
                &note.label,
                theme,
                config,
                state_font_size,
                "middle",
                Some(theme.primary_text_color.as_str()),
                false,
            ));
        }
    }

    if is_sequence {
        for (edge_idx, edge) in layout.edges.iter().enumerate() {
            let d = points_to_path(&edge.points);
            let mut stroke = theme.line_color.clone();
            let edge_id = edge_dom_id(edge_idx);
            if let Some(color) = &edge.override_style.stroke {
                stroke = color.clone();
            }
            let edge_label_fill = theme.edge_label_background.as_str();
            let edge_label_stroke = theme.primary_border_color.as_str();
            let (center_pad_x, center_pad_y) = edge_label_padding(layout.kind, config);
            let (endpoint_pad_x, endpoint_pad_y) = endpoint_label_padding(layout.kind);
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
                dash = "stroke-dasharray=\"2,2\"".to_string();
            }
            if let Some(dash_override) = &edge.override_style.dasharray {
                dash = format!("stroke-dasharray=\"{}\"", dash_override);
            }
            let stroke_width = edge.override_style.stroke_width.unwrap_or(1.5);
            svg.push_str(&format!(
                "<path id=\"{edge_id}\" class=\"edgePath\" data-edge-id=\"{edge_id}\" d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" {} {} {} stroke-linecap=\"round\" stroke-linejoin=\"round\" />",
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
                let (mid_x, label_y) = edge.label_anchor.unwrap_or_else(|| {
                    let start = edge.points.first().copied().unwrap_or((0.0, 0.0));
                    let end = edge.points.last().copied().unwrap_or(start);
                    let mid_x = (start.0 + end.0) / 2.0;
                    let line_y = start.1;
                    let gap = (theme.font_size * 0.6).max(8.0);
                    (mid_x, line_y - gap - label.height / 2.0)
                });
                let label_color = edge
                    .override_style
                    .label_color
                    .as_deref()
                    .unwrap_or(theme.primary_text_color.as_str());
                if edge_label_fill != "none" {
                    let rect = LabelRect::from_center(
                        mid_x,
                        label_y,
                        label.width,
                        label.height,
                        center_pad_x,
                        center_pad_y,
                    );
                    let visible = edge_label_background_visible(
                        layout.kind,
                        EdgeLabelKind::Center,
                        &edge.points,
                        rect,
                    );
                    let fill_opacity = if visible { 0.90 } else { 0.0 };
                    let stroke_opacity = if visible { 0.30 } else { 0.0 };
                    svg.push_str(&format!(
                        "<rect class=\"edgeLabel sequenceEdgeLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"center\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"2\" ry=\"2\" fill=\"{}\" fill-opacity=\"{:.2}\" stroke=\"{}\" stroke-opacity=\"{:.2}\" stroke-width=\"0.8\"/>",
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        edge_label_fill,
                        fill_opacity,
                        edge_label_stroke,
                        stroke_opacity
                    ));
                }
                svg.push_str(&format!(
                    "<g class=\"edgeLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"center\">"
                ));
                svg.push_str(&text_block_svg(
                    mid_x,
                    label_y,
                    label,
                    theme,
                    config,
                    false,
                    Some(label_color),
                ));
                svg.push_str("</g>");
            }

            let end_label_offset = (theme.font_size * 0.6).max(8.0);
            let label_color = edge
                .override_style
                .label_color
                .as_deref()
                .unwrap_or(theme.primary_text_color.as_str());
            if let Some(label) = edge.start_label.as_ref()
                && let Some((x, y)) = edge
                    .start_label_anchor
                    .or_else(|| edge_endpoint_label_position(edge, true, end_label_offset))
            {
                if edge_label_fill != "none" {
                    let rect = LabelRect::from_center(
                        x,
                        y,
                        label.width,
                        label.height,
                        endpoint_pad_x,
                        endpoint_pad_y,
                    );
                    let visible = edge_label_background_visible(
                        layout.kind,
                        EdgeLabelKind::Start,
                        &edge.points,
                        rect,
                    );
                    let fill_opacity = if visible { 0.88 } else { 0.0 };
                    let stroke_opacity = if visible { 0.28 } else { 0.0 };
                    svg.push_str(&format!(
                        "<rect class=\"edgeLabel sequenceEndpointLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"start\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"2\" ry=\"2\" fill=\"{}\" fill-opacity=\"{:.2}\" stroke=\"{}\" stroke-opacity=\"{:.2}\" stroke-width=\"0.75\"/>",
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        edge_label_fill,
                        fill_opacity,
                        edge_label_stroke,
                        stroke_opacity
                    ));
                }
                svg.push_str(&format!(
                    "<g class=\"edgeLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"start\">"
                ));
                svg.push_str(&text_block_svg(
                    x,
                    y,
                    label,
                    theme,
                    config,
                    false,
                    Some(label_color),
                ));
                svg.push_str("</g>");
            }
            if let Some(label) = edge.end_label.as_ref()
                && let Some((x, y)) = edge
                    .end_label_anchor
                    .or_else(|| edge_endpoint_label_position(edge, false, end_label_offset))
            {
                if edge_label_fill != "none" {
                    let rect = LabelRect::from_center(
                        x,
                        y,
                        label.width,
                        label.height,
                        endpoint_pad_x,
                        endpoint_pad_y,
                    );
                    let visible = edge_label_background_visible(
                        layout.kind,
                        EdgeLabelKind::End,
                        &edge.points,
                        rect,
                    );
                    let fill_opacity = if visible { 0.88 } else { 0.0 };
                    let stroke_opacity = if visible { 0.28 } else { 0.0 };
                    svg.push_str(&format!(
                        "<rect class=\"edgeLabel sequenceEndpointLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"end\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"2\" ry=\"2\" fill=\"{}\" fill-opacity=\"{:.2}\" stroke=\"{}\" stroke-opacity=\"{:.2}\" stroke-width=\"0.75\"/>",
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        edge_label_fill,
                        fill_opacity,
                        edge_label_stroke,
                        stroke_opacity
                    ));
                }
                svg.push_str(&format!(
                    "<g class=\"edgeLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"end\">"
                ));
                svg.push_str(&text_block_svg(
                    x,
                    y,
                    label,
                    theme,
                    config,
                    false,
                    Some(label_color),
                ));
                svg.push_str("</g>");
            }
        }

        for number in seq_data.map(|s| s.numbers.as_slice()).unwrap_or_default() {
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
        let base_edge_width = match layout.kind {
            crate::ir::DiagramKind::Class
            | crate::ir::DiagramKind::State
            | crate::ir::DiagramKind::Er => 1.0,
            _ => 2.0,
        };
        for (edge_idx, edge) in layout.edges.iter().enumerate() {
            let d = points_to_path(&edge.points);
            let mut stroke = theme.line_color.clone();
            let edge_id = edge_dom_id(edge_idx);
            let (mut dash, mut stroke_width) = match edge.style {
                crate::ir::EdgeStyle::Solid => (String::new(), base_edge_width),
                crate::ir::EdgeStyle::Dotted => {
                    ("stroke-dasharray=\"4 4\"".to_string(), base_edge_width)
                }
                crate::ir::EdgeStyle::Thick => (String::new(), 3.5),
            };

            if let Some(color) = &edge.override_style.stroke {
                stroke = color.clone();
            }
            let marker_id = color_ids.get(&stroke).copied().unwrap_or(0);
            let marker_end = if edge.arrow_end && !overlay_flowchart {
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
            let marker_start = if edge.arrow_start && !overlay_flowchart {
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
                "<path id=\"{edge_id}\" class=\"edgePath\" data-edge-id=\"{edge_id}\" d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" {} {} {} stroke-linecap=\"round\" stroke-linejoin=\"round\" />",
                d, stroke, stroke_width, marker_end, marker_start, dash
            ));

            if overlay_flowchart {
                if edge.arrow_start {
                    if let Some(point) = edge.points.first().copied() {
                        let angle = edge_endpoint_angle(&edge.points, true);
                        overlay_arrows.push((true, point, angle, stroke.clone(), stroke_width));
                    }
                }
                if edge.arrow_end {
                    if let Some(point) = edge.points.last().copied() {
                        let angle = edge_endpoint_angle(&edge.points, false);
                        overlay_arrows.push((false, point, angle, stroke.clone(), stroke_width));
                    }
                }
            }

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

            if let Some(label) = edge.label.as_ref()
                && let Some((x, y)) = edge.label_anchor
            {
                let (pad_x, pad_y) = edge_label_padding(layout.kind, config);
                let (fill_opacity, stroke_opacity) = match layout.kind {
                    crate::ir::DiagramKind::State => (0.7, 0.25),
                    crate::ir::DiagramKind::Flowchart => (0.95, 0.45),
                    _ => (0.85, 0.35),
                };
                let label_scale = if layout.kind == crate::ir::DiagramKind::State {
                    (state_font_size / theme.font_size).min(1.0)
                } else {
                    1.0
                };
                let label_w = label.width * label_scale;
                let label_h = label.height * label_scale;
                let rect = LabelRect::from_center(x, y, label_w, label_h, pad_x, pad_y);
                let label_fill = theme.edge_label_background.as_str();
                if label_fill != "none" {
                    let visible = edge_label_background_visible(
                        layout.kind,
                        EdgeLabelKind::Center,
                        &edge.points,
                        rect,
                    );
                    let fill_opacity = if visible { fill_opacity } else { 0.0 };
                    let stroke_opacity = if visible { stroke_opacity } else { 0.0 };
                    svg.push_str(&format!(
                        "<rect data-edge-id=\"{edge_id}\" data-label-kind=\"center\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"2\" ry=\"2\" fill=\"{}\" fill-opacity=\"{:.2}\" stroke=\"{}\" stroke-opacity=\"{:.2}\" stroke-width=\"0.8\"/>",
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        label_fill,
                        fill_opacity,
                        theme.primary_border_color,
                        stroke_opacity
                    ));
                }
                if layout.kind == crate::ir::DiagramKind::State {
                    svg.push_str(&format!(
                        "<g class=\"edgeLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"center\">"
                    ));
                    svg.push_str(&text_block_svg_with_font_size(
                        x,
                        y,
                        label,
                        theme,
                        config,
                        state_font_size,
                        "middle",
                        edge.override_style.label_color.as_deref(),
                        false,
                    ));
                    svg.push_str("</g>");
                } else {
                    svg.push_str(&format!(
                        "<g class=\"edgeLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"center\">"
                    ));
                    svg.push_str(&text_block_svg(
                        x,
                        y,
                        label,
                        theme,
                        config,
                        true,
                        edge.override_style.label_color.as_deref(),
                    ));
                    svg.push_str("</g>");
                }
            }

            let endpoint_label_scale = if layout.kind == crate::ir::DiagramKind::State {
                (state_font_size / theme.font_size).min(1.0)
            } else {
                1.0
            };
            let (endpoint_pad_x, endpoint_pad_y) = endpoint_label_padding(layout.kind);
            let (endpoint_fill_opacity, endpoint_stroke_opacity) = match layout.kind {
                crate::ir::DiagramKind::State => (0.7, 0.25),
                crate::ir::DiagramKind::Flowchart => (0.95, 0.45),
                crate::ir::DiagramKind::Class => (0.9, 0.4),
                _ => (0.85, 0.35),
            };
            let endpoint_label_fill = theme.edge_label_background.as_str();
            let label_color = edge
                .override_style
                .label_color
                .as_deref()
                .unwrap_or(theme.primary_text_color.as_str());
            if let Some(label) = edge.start_label.as_ref()
                && let Some((x, y)) = edge.start_label_anchor
            {
                let label_w = label.width * endpoint_label_scale;
                let label_h = label.height * endpoint_label_scale;
                let rect =
                    LabelRect::from_center(x, y, label_w, label_h, endpoint_pad_x, endpoint_pad_y);
                if endpoint_label_fill != "none" {
                    let visible = edge_label_background_visible(
                        layout.kind,
                        EdgeLabelKind::Start,
                        &edge.points,
                        rect,
                    );
                    let fill_opacity = if visible { endpoint_fill_opacity } else { 0.0 };
                    let stroke_opacity = if visible {
                        endpoint_stroke_opacity
                    } else {
                        0.0
                    };
                    svg.push_str(&format!(
                        "<rect data-edge-id=\"{edge_id}\" data-label-kind=\"start\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"2\" ry=\"2\" fill=\"{}\" fill-opacity=\"{:.2}\" stroke=\"{}\" stroke-opacity=\"{:.2}\" stroke-width=\"0.8\"/>",
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        endpoint_label_fill,
                        fill_opacity,
                        theme.primary_border_color,
                        stroke_opacity
                    ));
                }
                if layout.kind == crate::ir::DiagramKind::State {
                    svg.push_str(&format!(
                        "<g class=\"edgeLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"start\">"
                    ));
                    svg.push_str(&text_block_svg_with_font_size(
                        x,
                        y,
                        label,
                        theme,
                        config,
                        state_font_size,
                        "middle",
                        Some(label_color),
                        false,
                    ));
                    svg.push_str("</g>");
                } else {
                    svg.push_str(&format!(
                        "<g class=\"edgeLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"start\">"
                    ));
                    svg.push_str(&text_block_svg(
                        x,
                        y,
                        label,
                        theme,
                        config,
                        false,
                        Some(label_color),
                    ));
                    svg.push_str("</g>");
                }
            }
            if let Some(label) = edge.end_label.as_ref()
                && let Some((x, y)) = edge.end_label_anchor
            {
                let label_w = label.width * endpoint_label_scale;
                let label_h = label.height * endpoint_label_scale;
                let rect =
                    LabelRect::from_center(x, y, label_w, label_h, endpoint_pad_x, endpoint_pad_y);
                if endpoint_label_fill != "none" {
                    let visible = edge_label_background_visible(
                        layout.kind,
                        EdgeLabelKind::End,
                        &edge.points,
                        rect,
                    );
                    let fill_opacity = if visible { endpoint_fill_opacity } else { 0.0 };
                    let stroke_opacity = if visible {
                        endpoint_stroke_opacity
                    } else {
                        0.0
                    };
                    svg.push_str(&format!(
                        "<rect data-edge-id=\"{edge_id}\" data-label-kind=\"end\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"2\" ry=\"2\" fill=\"{}\" fill-opacity=\"{:.2}\" stroke=\"{}\" stroke-opacity=\"{:.2}\" stroke-width=\"0.8\"/>",
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        endpoint_label_fill,
                        fill_opacity,
                        theme.primary_border_color,
                        stroke_opacity
                    ));
                }
                if layout.kind == crate::ir::DiagramKind::State {
                    svg.push_str(&format!(
                        "<g class=\"edgeLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"end\">"
                    ));
                    svg.push_str(&text_block_svg_with_font_size(
                        x,
                        y,
                        label,
                        theme,
                        config,
                        state_font_size,
                        "middle",
                        Some(label_color),
                        false,
                    ));
                    svg.push_str("</g>");
                } else {
                    svg.push_str(&format!(
                        "<g class=\"edgeLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"end\">"
                    ));
                    svg.push_str(&text_block_svg(
                        x,
                        y,
                        label,
                        theme,
                        config,
                        false,
                        Some(label_color),
                    ));
                    svg.push_str("</g>");
                }
            }
        }
    }

    if !is_sequence {
        let mut nodes_to_draw: Vec<&crate::layout::NodeLayout> =
            if layout.kind == crate::ir::DiagramKind::Treemap {
                let mut nodes: Vec<&crate::layout::NodeLayout> = layout.nodes.values().collect();
                nodes.sort_by(|a, b| {
                    let area_a = a.width * a.height;
                    let area_b = b.width * b.height;
                    area_b.partial_cmp(&area_a).unwrap_or(Ordering::Equal)
                });
                nodes
            } else {
                layout.nodes.values().collect()
            };

        for node in nodes_to_draw.drain(..) {
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
            if layout.kind == crate::ir::DiagramKind::Er {
                svg.push_str(&render_er_node(node, theme, config));
                if node.link.is_some() {
                    svg.push_str("</a>");
                }
                continue;
            }
            svg.push_str(&shape_svg(node, theme, config));
            if layout.kind != crate::ir::DiagramKind::Er {
                let divider_line_height = if layout.kind == crate::ir::DiagramKind::Class {
                    theme.font_size * config.class_label_line_height()
                } else {
                    theme.font_size * config.label_line_height
                };
                svg.push_str(&divider_lines_svg(node, theme, divider_line_height));
            }
            let center_x = node.x + node.width / 2.0;
            let center_y = node.y + node.height / 2.0;
            let hide_label = node.label.lines.iter().all(|line| line.trim().is_empty())
                || node.id.starts_with("__start_")
                || node.id.starts_with("__end_");
            if !hide_label {
                let label_svg = if layout.kind == crate::ir::DiagramKind::Treemap {
                    let label_x = node.x + config.treemap.label_padding_x;
                    let label_y = node.y + config.treemap.label_padding_y + node.label.height / 2.0;
                    text_block_svg_anchor(
                        label_x,
                        label_y,
                        &node.label,
                        theme,
                        config,
                        "start",
                        node.style.text_color.as_deref(),
                    )
                } else if layout.kind == crate::ir::DiagramKind::Er {
                    render_er_node_label(node, theme, config).unwrap_or_else(|| {
                        if node.label.lines.iter().any(|line| is_divider_line(line)) {
                            text_block_svg_class(
                                node,
                                theme,
                                config,
                                node.style.text_color.as_deref(),
                            )
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
                        }
                    })
                } else if node.label.lines.iter().any(|line| is_divider_line(line)) {
                    text_block_svg_class(node, theme, config, node.style.text_color.as_deref())
                } else if layout.kind == crate::ir::DiagramKind::State {
                    text_block_svg_with_font_size(
                        center_x,
                        center_y,
                        &node.label,
                        theme,
                        config,
                        state_font_size,
                        "middle",
                        node.style.text_color.as_deref(),
                        false,
                    )
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

        if overlay_flowchart && !overlay_arrows.is_empty() {
            for (is_start, point, angle, stroke, stroke_width) in overlay_arrows {
                let final_angle = if is_start { angle + 180.0 } else { angle };
                svg.push_str(&arrowhead_svg(
                    point,
                    final_angle,
                    stroke.as_str(),
                    stroke_width,
                ));
            }
        }

        for footbox in seq_data.map(|s| s.footboxes.as_slice()).unwrap_or_default() {
            if let Some(link) = footbox.link.as_ref() {
                svg.push_str(&format!("<a {}>", link_attrs(link)));
                if let Some(title) = link.title.as_deref() {
                    svg.push_str(&format!("<title>{}</title>", escape_xml(title)));
                }
            }
            svg.push_str(&shape_svg(footbox, theme, config));
            let divider_line_height = theme.font_size * config.label_line_height;
            svg.push_str(&divider_lines_svg(footbox, theme, divider_line_height));
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
        for footbox in seq_data.map(|s| s.footboxes.as_slice()).unwrap_or_default() {
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
    let deduped = dedupe_points(points);
    if deduped.len() == 1 {
        return format!("M {:.3},{:.3}", deduped[0].0, deduped[0].1);
    }
    let mut d = format!("M {:.3},{:.3}", deduped[0].0, deduped[0].1);
    for (x, y) in deduped.iter().skip(1) {
        d.push_str(&format!(" L {:.3},{:.3}", x, y));
    }
    d
}

fn dedupe_points(points: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut out = Vec::with_capacity(points.len());
    for point in points.iter().copied() {
        if out
            .last()
            .map(|prev: &(f32, f32)| {
                (prev.0 - point.0).abs() < 1e-3 && (prev.1 - point.1).abs() < 1e-3
            })
            .unwrap_or(false)
        {
            continue;
        }
        out.push(point);
    }
    out
}

#[derive(Debug, Clone, Copy)]
struct LabelRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl LabelRect {
    fn from_center(
        center_x: f32,
        center_y: f32,
        label_w: f32,
        label_h: f32,
        pad_x: f32,
        pad_y: f32,
    ) -> Self {
        let width = (label_w + pad_x * 2.0).max(0.0);
        let height = (label_h + pad_y * 2.0).max(0.0);
        Self {
            x: center_x - width * 0.5,
            y: center_y - height * 0.5,
            width,
            height,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EdgeLabelKind {
    Center,
    Start,
    End,
}

fn edge_label_background_visible(
    diagram_kind: crate::ir::DiagramKind,
    label_kind: EdgeLabelKind,
    edge_points: &[(f32, f32)],
    rect: LabelRect,
) -> bool {
    if edge_points.len() < 2 || rect.width <= 0.0 || rect.height <= 0.0 {
        return true;
    }
    let gap = polyline_rect_gap(edge_points, &rect);
    match label_kind {
        EdgeLabelKind::Center => {
            let gap_limit = match diagram_kind {
                crate::ir::DiagramKind::Flowchart => 1.2,
                crate::ir::DiagramKind::Sequence => (rect.height * 0.16).clamp(1.2, 2.4),
                crate::ir::DiagramKind::Requirement => 1.0,
                _ => 0.9,
            };
            gap <= gap_limit
        }
        EdgeLabelKind::Start | EdgeLabelKind::End => match diagram_kind {
            crate::ir::DiagramKind::Sequence => gap <= (rect.height * 0.12).clamp(0.6, 1.4),
            crate::ir::DiagramKind::Flowchart | crate::ir::DiagramKind::Requirement => gap <= 0.35,
            _ => false,
        },
    }
}

fn polyline_rect_gap(points: &[(f32, f32)], rect: &LabelRect) -> f32 {
    if points.len() < 2 {
        return f32::INFINITY;
    }
    let mut best = f32::INFINITY;
    for segment in points.windows(2) {
        let dist = segment_rect_gap(segment[0], segment[1], rect);
        best = best.min(dist);
        if best <= 1e-6 {
            return 0.0;
        }
    }
    best
}

fn segment_rect_gap(a: (f32, f32), b: (f32, f32), rect: &LabelRect) -> f32 {
    if segment_intersects_rect(a, b, rect) {
        return 0.0;
    }
    let mut best = point_rect_distance(a, rect).min(point_rect_distance(b, rect));
    let corners = [
        (rect.x, rect.y),
        (rect.x + rect.width, rect.y),
        (rect.x + rect.width, rect.y + rect.height),
        (rect.x, rect.y + rect.height),
    ];
    for corner in corners {
        best = best.min(point_segment_distance(corner, a, b));
    }
    best
}

fn point_rect_distance(point: (f32, f32), rect: &LabelRect) -> f32 {
    let (px, py) = point;
    let x1 = rect.x;
    let y1 = rect.y;
    let x2 = rect.x + rect.width;
    let y2 = rect.y + rect.height;
    let dx = if px < x1 {
        x1 - px
    } else if px > x2 {
        px - x2
    } else {
        0.0
    };
    let dy = if py < y1 {
        y1 - py
    } else if py > y2 {
        py - y2
    } else {
        0.0
    };
    (dx * dx + dy * dy).sqrt()
}

fn point_segment_distance(point: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let ab_x = b.0 - a.0;
    let ab_y = b.1 - a.1;
    let len_sq = ab_x * ab_x + ab_y * ab_y;
    if len_sq <= 1e-9 {
        let dx = point.0 - a.0;
        let dy = point.1 - a.1;
        return (dx * dx + dy * dy).sqrt();
    }
    let t = ((point.0 - a.0) * ab_x + (point.1 - a.1) * ab_y) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let proj_x = a.0 + ab_x * t;
    let proj_y = a.1 + ab_y * t;
    let dx = point.0 - proj_x;
    let dy = point.1 - proj_y;
    (dx * dx + dy * dy).sqrt()
}

fn segment_intersects_rect(a: (f32, f32), b: (f32, f32), rect: &LabelRect) -> bool {
    if point_in_rect(a, rect) || point_in_rect(b, rect) {
        return true;
    }
    let corners = [
        (rect.x, rect.y),
        (rect.x + rect.width, rect.y),
        (rect.x + rect.width, rect.y + rect.height),
        (rect.x, rect.y + rect.height),
    ];
    let edges = [
        (corners[0], corners[1]),
        (corners[1], corners[2]),
        (corners[2], corners[3]),
        (corners[3], corners[0]),
    ];
    edges
        .iter()
        .any(|(c0, c1)| segments_intersect(a, b, *c0, *c1))
}

fn point_in_rect(point: (f32, f32), rect: &LabelRect) -> bool {
    point.0 >= rect.x
        && point.0 <= rect.x + rect.width
        && point.1 >= rect.y
        && point.1 <= rect.y + rect.height
}

fn segments_intersect(a: (f32, f32), b: (f32, f32), c: (f32, f32), d: (f32, f32)) -> bool {
    let eps = 1e-6;
    let o1 = orient(a, b, c);
    let o2 = orient(a, b, d);
    let o3 = orient(c, d, a);
    let o4 = orient(c, d, b);

    if o1.abs() < eps && on_segment(a, b, c, eps) {
        return true;
    }
    if o2.abs() < eps && on_segment(a, b, d, eps) {
        return true;
    }
    if o3.abs() < eps && on_segment(c, d, a, eps) {
        return true;
    }
    if o4.abs() < eps && on_segment(c, d, b, eps) {
        return true;
    }

    (o1 > 0.0) != (o2 > 0.0) && (o3 > 0.0) != (o4 > 0.0)
}

fn orient(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

fn on_segment(a: (f32, f32), b: (f32, f32), c: (f32, f32), eps: f32) -> bool {
    c.0 >= a.0.min(b.0) - eps
        && c.0 <= a.0.max(b.0) + eps
        && c.1 >= a.1.min(b.1) - eps
        && c.1 <= a.1.max(b.1) + eps
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

    let mut label_y: Vec<f32> = layout
        .nodes
        .iter()
        .map(|node| node.y + node.height / 2.0)
        .collect();
    let label_line_height = label_font_size * 1.2;
    let label_half_heights: Vec<f32> = layout
        .nodes
        .iter()
        .map(|node| {
            let text_lines = node
                .label
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count()
                .max(1) as f32;
            // Node labels always render one additional value line below the title.
            (text_lines + 1.0) * label_line_height * 0.5
        })
        .collect();
    let mut rank_min_x = vec![f32::INFINITY; max_rank + 1];
    for node in &layout.nodes {
        let slot = &mut rank_min_x[node.rank];
        *slot = (*slot).min(node.x);
    }
    for x in &mut rank_min_x {
        if !x.is_finite() {
            *x = 0.0;
        }
    }
    let edge_margin = label_font_size * 0.3;
    let preferred_gap = label_font_size * 0.25;
    for rank in 0..=max_rank {
        let mut indices: Vec<usize> = layout
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(idx, node)| (node.rank == rank).then_some(idx))
            .collect();
        if indices.len() < 2 {
            continue;
        }
        indices.sort_by(|a, b| {
            label_y[*a]
                .partial_cmp(&label_y[*b])
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let first_idx = indices[0];
        let last_idx = *indices.last().unwrap_or(&first_idx);
        let top = edge_margin + label_half_heights[first_idx];
        let bottom = (layout.height - edge_margin - label_half_heights[last_idx]).max(top);

        if indices.len() == 1 {
            label_y[first_idx] = label_y[first_idx].clamp(top, bottom);
            continue;
        }

        let mut half_pair_span = 0.0;
        for pair in indices.windows(2) {
            half_pair_span += label_half_heights[pair[0]] + label_half_heights[pair[1]];
        }
        let available_span = (bottom - top).max(0.0);
        let max_gap = (available_span - half_pair_span) / (indices.len() - 1) as f32;
        let gap = preferred_gap.min(max_gap.max(0.0));
        let required_span = half_pair_span + gap * (indices.len() - 1) as f32;
        let first_max = (bottom - required_span).max(top);

        label_y[first_idx] = label_y[first_idx].clamp(top, first_max);
        for pair in indices.windows(2) {
            let prev_idx = pair[0];
            let cur_idx = pair[1];
            let min_gap = label_half_heights[prev_idx] + label_half_heights[cur_idx] + gap;
            label_y[cur_idx] = label_y[prev_idx] + min_gap;
        }
    }

    svg.push_str(&format!(
        "<g class=\"node-labels\" font-size=\"{}\" fill=\"{}\">",
        label_font_size, theme.primary_text_color
    ));
    for (idx, node) in layout.nodes.iter().enumerate() {
        let align_left_of_node = node.rank > 0;
        let text_anchor = if align_left_of_node { "end" } else { "start" };
        let x = if align_left_of_node {
            let mut inward_offset = 6.0;
            if node.rank < max_rank {
                let prev_x = rank_min_x[node.rank.saturating_sub(1)];
                let rank_gap = (node.x - prev_x).max(0.0);
                inward_offset = (rank_gap * 0.2).clamp(6.0, label_font_size * 3.0);
            }
            node.x - inward_offset
        } else {
            node.x + layout.node_width + 6.0
        };
        let y = label_y[idx];
        let label = escape_xml(&node.label);
        let value = format_sankey_value(node.total);
        let first_y = y - label_font_size * 0.4;
        svg.push_str(&format!(
            "<text x=\"{x:.2}\" y=\"{first_y:.2}\" dy=\"0em\" text-anchor=\"{text_anchor}\" font-size=\"{label_font_size:.1}\"><tspan x=\"{x:.2}\" dy=\"0em\">{label}</tspan><tspan x=\"{x:.2}\" dy=\"1.15em\">{value}</tspan></text>"
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
    let needs_transform =
        layout.icon_scale != 1.0 || layout.icon_tx != 0.0 || layout.icon_ty != 0.0;

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

fn normalize_font_family(font_family: &str) -> String {
    font_family
        .split(',')
        .map(|part| part.trim().trim_matches('\'').trim_matches('"'))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(",")
}

fn error_style_block(theme: &Theme) -> String {
    let font_family = normalize_font_family(&theme.font_family);
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
    let font_family = normalize_font_family(&theme.font_family);
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

    for (edge_idx, edge) in layout.edges.iter().enumerate() {
        let edge_id = edge_dom_id(edge_idx);
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
            "<path id=\"{edge_id}\" data-edge-id=\"{edge_id}\" d=\"{d}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{dash}{marker_start}{marker_end} stroke-linecap=\"round\" stroke-linejoin=\"round\"/>"
        ));

        if let Some(label) = edge.label.as_ref()
            && let Some((x, y)) = edge.label_anchor
        {
            let (pad_x, pad_y) = edge_label_padding(layout.kind, config);
            let rect = LabelRect::from_center(x, y, label.width, label.height, pad_x, pad_y);
            if req.edge_label_background != "none" {
                let visible = edge_label_background_visible(
                    layout.kind,
                    EdgeLabelKind::Center,
                    &edge.points,
                    rect,
                );
                let fill_opacity = if visible { 0.5 } else { 0.0 };
                svg.push_str(&format!(
                    "<rect data-edge-id=\"{edge_id}\" data-label-kind=\"center\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"2\" ry=\"2\" fill=\"{}\" fill-opacity=\"{:.2}\" stroke=\"none\"/>",
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    req.edge_label_background,
                    fill_opacity
                ));
            }
            let label_color = edge
                .override_style
                .label_color
                .as_deref()
                .unwrap_or(req.edge_label_color.as_str());
            svg.push_str(&format!(
                "<g class=\"edgeLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"center\">"
            ));
            svg.push_str(&text_block_svg(
                x,
                y,
                label,
                theme,
                config,
                true,
                Some(label_color),
            ));
            svg.push_str("</g>");
        }
    }

    for node in layout.nodes.values() {
        if node.hidden {
            continue;
        }
        if node.anchor_subgraph.is_some() {
            continue;
        }
        let fill = node.style.fill.as_deref().unwrap_or(req.fill.as_str());
        let base_stroke = node
            .style
            .stroke
            .as_deref()
            .unwrap_or(req.box_stroke.as_str());
        let base_stroke_width = node.style.stroke_width.unwrap_or(req.box_stroke_width);
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
            svg.push_str(&render_line(
                header_x,
                header_y,
                &lines[0],
                label_color,
                false,
            ));
        }
        if header_count >= 2 {
            let min_header_gap = theme.font_size * 1.25;
            let id_y = header_y + req.header_line_gap.max(min_header_gap);
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

    let mut nodes: Vec<&crate::layout::NodeLayout> =
        layout.nodes.values().filter(|node| !node.hidden).collect();
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
        let mut lx = label_r * angle.cos();
        let ly = label_r * angle.sin();
        let anchor = if angle.cos() > 0.35 {
            lx -= 6.0;
            "end"
        } else if angle.cos() < -0.35 {
            lx += 6.0;
            "start"
        } else {
            "middle"
        };
        svg.push_str(&format!(
            "<text x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"{}\" dominant-baseline=\"middle\" font-family=\"{}\" font-size=\"12\" fill=\"{}\">{}</text>",
            lx,
            ly,
            anchor,
            normalize_font_family(&theme.font_family),
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

        let legend_offset = MAX_RADIUS * 0.8;
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
            normalize_font_family(&theme.font_family),
            AXIS_COLOR,
            escape_xml(name)
        ));
    }

    svg.push_str(&format!(
        "<text x=\"0\" y=\"{:.3}\" text-anchor=\"middle\" dominant-baseline=\"hanging\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\"></text>",
        -(MAX_RADIUS + 50.0),
        normalize_font_family(&theme.font_family),
        theme.font_size,
        AXIS_COLOR
    ));

    svg.push_str("</g>");
    svg
}

/// Render an architecture diagram icon as SVG.
/// Returns SVG elements (paths/circles) drawn within the given width/height box.
fn architecture_icon_svg(icon_type: Option<&str>, w: f32, h: f32, fill: &str) -> String {
    let cx = w / 2.0;
    let cy = h / 2.0;
    let r = w.min(h) * 0.35;
    let sw = (w * 0.02).max(1.5);
    let style = format!(
        "fill=\"none\" stroke=\"{}\" stroke-width=\"{:.1}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"",
        fill, sw
    );
    match icon_type {
        Some("internet") | Some("globe") => {
            // Globe: circle + vertical ellipse + horizontal line + vertical line
            format!(
                "<circle cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"{r:.1}\" {style}/>\
                 <ellipse cx=\"{cx:.1}\" cy=\"{cy:.1}\" rx=\"{rx:.1}\" ry=\"{r:.1}\" {style}/>\
                 <line x1=\"{x1:.1}\" y1=\"{cy:.1}\" x2=\"{x2:.1}\" y2=\"{cy:.1}\" {style}/>\
                 <line x1=\"{cx:.1}\" y1=\"{y1:.1}\" x2=\"{cx:.1}\" y2=\"{y2:.1}\" {style}/>",
                rx = r * 0.5,
                x1 = cx - r,
                x2 = cx + r,
                y1 = cy - r,
                y2 = cy + r,
            )
        }
        Some("server") => {
            // Server rack: stacked rectangles
            let bx = cx - r;
            let by = cy - r;
            let bw = r * 2.0;
            let bh = r * 2.0;
            let rows = 3;
            let row_h = bh / rows as f32;
            let mut s = String::new();
            for i in 0..rows {
                let ry = by + i as f32 * row_h;
                s.push_str(&format!(
                    "<rect x=\"{bx:.1}\" y=\"{ry:.1}\" width=\"{bw:.1}\" height=\"{row_h:.1}\" rx=\"2\" {style}/>"
                ));
                // Small indicator circle in each row
                let dot_x = bx + bw - row_h * 0.35;
                let dot_y = ry + row_h * 0.5;
                let dot_r = row_h * 0.12;
                s.push_str(&format!(
                    "<circle cx=\"{dot_x:.1}\" cy=\"{dot_y:.1}\" r=\"{dot_r:.1}\" fill=\"{fill}\" stroke=\"none\"/>"
                ));
            }
            s
        }
        Some("database") | Some("disk") => {
            // Database cylinder: rect body + ellipses top/bottom
            let bx = cx - r;
            let bw = r * 2.0;
            let ell_ry = r * 0.3;
            let body_top = cy - r + ell_ry;
            let body_bot = cy + r - ell_ry;
            let body_h = body_bot - body_top;
            format!(
                "<rect x=\"{bx:.1}\" y=\"{body_top:.1}\" width=\"{bw:.1}\" height=\"{body_h:.1}\" {style}/>\
                 <ellipse cx=\"{cx:.1}\" cy=\"{body_top:.1}\" rx=\"{r:.1}\" ry=\"{ell_ry:.1}\" {style}/>\
                 <ellipse cx=\"{cx:.1}\" cy=\"{body_bot:.1}\" rx=\"{r:.1}\" ry=\"{ell_ry:.1}\" {style}/>\
                 <line x1=\"{x1:.1}\" y1=\"{body_top:.1}\" x2=\"{x1:.1}\" y2=\"{body_bot:.1}\" {style}/>\
                 <line x1=\"{x2:.1}\" y1=\"{body_top:.1}\" x2=\"{x2:.1}\" y2=\"{body_bot:.1}\" {style}/>",
                x1 = bx,
                x2 = bx + bw,
            )
        }
        Some("cloud") => {
            // Cloud: rounded bumpy shape using cubic bezier
            let s = r * 0.7;
            format!(
                "<path d=\"M {x1:.1} {y_mid:.1} \
                 Q {x1:.1} {y_top:.1} {cx:.1} {y_top:.1} \
                 Q {x2:.1} {y_top:.1} {x2:.1} {y_mid:.1} \
                 Q {x2:.1} {y_bot:.1} {cx:.1} {y_bot:.1} \
                 Q {x1:.1} {y_bot:.1} {x1:.1} {y_mid:.1} Z\" {style}/>",
                x1 = cx - s,
                x2 = cx + s,
                y_mid = cy,
                y_top = cy - s * 0.8,
                y_bot = cy + s * 0.6,
            )
        }
        _ => {
            // Fallback: question mark
            format!(
                "<text x=\"{cx:.1}\" y=\"{y:.1}\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"{fill}\" font-size=\"{fs:.0}\">?</text>",
                y = cy + w * 0.08,
                fs = w * 0.7,
            )
        }
    }
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
        text.lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or(text)
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
        svg.push_str(&architecture_icon_svg(
            node.icon.as_deref(),
            node.width,
            node.height,
            ICON_TEXT_FILL,
        ));
        svg.push_str(&format!(
            "<text x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
            node.width / 2.0,
            label_y,
            normalize_font_family(&theme.font_family),
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
            "<g transform=\"translate({:.3},{:.3})\">",
            icon_x, icon_y
        ));
        svg.push_str(&format!(
            "<rect width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"none\" />",
            GROUP_ICON_SIZE, GROUP_ICON_SIZE, ICON_FILL
        ));
        svg.push_str(&architecture_icon_svg(
            subgraph.icon.as_deref(),
            GROUP_ICON_SIZE,
            GROUP_ICON_SIZE,
            ICON_TEXT_FILL,
        ));
        svg.push_str("</g>");
        let label_x = subgraph.x + GROUP_ICON_SIZE + 4.0;
        let label_y = subgraph.y + GROUP_ICON_SIZE * 0.7;
        svg.push_str(&format!(
            "<text x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"start\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
            label_x,
            label_y,
            normalize_font_family(&theme.font_family),
            theme.font_size,
            escape_xml(&theme.primary_text_color),
            escape_xml(first_line(&subgraph.label))
        ));
    }
    svg.push_str("</g>");

    svg
}

fn render_pie(pie: &PieData, theme: &Theme, config: &LayoutConfig) -> String {
    let mut svg = String::new();
    let (cx, cy) = pie.center;
    let radius = pie.radius;
    if radius <= 0.0 {
        return svg;
    }

    let pie_cfg = &config.pie;
    let mut total: f32 = pie.legend.iter().map(|s| s.value.max(0.0)).sum();
    if total <= 0.0 {
        total = pie.slices.iter().map(|s| s.value.max(0.0)).sum();
    }

    let slice_stroke = theme.background.as_str();
    let slice_stroke_width = theme.pie_stroke_width.max(1.2);

    for slice in &pie.slices {
        let span = (slice.end_angle - slice.start_angle).abs();
        if span <= 0.0001 {
            continue;
        }
        if span >= std::f32::consts::PI * 2.0 - 0.001 {
            svg.push_str(&format!(
                "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.3}\" opacity=\"{:.3}\"/>",
                cx,
                cy,
                radius,
                escape_xml(&slice.color),
                escape_xml(slice_stroke),
                slice_stroke_width,
                theme.pie_opacity
            ));
            continue;
        }
        let path = pie_slice_path(cx, cy, radius, slice.start_angle, slice.end_angle);
        svg.push_str(&format!(
            "<path d=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.3}\" opacity=\"{:.3}\"/>",
            escape_xml(&path),
            escape_xml(&slice.color),
            escape_xml(slice_stroke),
            slice_stroke_width,
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

    // Add labels on slices (percent inside, category outside)
    #[derive(Clone)]
    struct PieLabel {
        text: String,
        font_size: f32,
        outside: bool,
        side: i32,
        x: f32,
        y: f32,
        edge_x: f32,
        edge_y: f32,
        line_color: String,
    }

    let mut labels: Vec<PieLabel> = Vec::new();
    let suppress_outside_labels = pie.legend.len() >= 4;
    for slice in &pie.slices {
        let span = (slice.end_angle - slice.start_angle).abs();
        if span <= 0.0001 || total <= 0.0 {
            continue;
        }
        let percent = slice.value / total * 100.0;
        if percent < pie_cfg.min_percent {
            continue;
        }
        let percent_text = format!("{:.0}%", percent);
        let mid_angle = (slice.start_angle + slice.end_angle) / 2.0;
        let font_size = theme.pie_section_text_size;
        let arc_len = radius * span;
        let percent_width =
            text_metrics::measure_text_width(&percent_text, font_size, theme.font_family.as_str())
                .unwrap_or(percent_text.chars().count() as f32 * font_size * 0.55);
        let outside = !suppress_outside_labels && (arc_len < percent_width * 1.35 || span < 0.4);
        let label_text = if outside {
            slice.label.lines.join(" ")
        } else {
            percent_text.clone()
        };
        let edge_x = cx + radius * mid_angle.cos();
        let edge_y = cy + radius * mid_angle.sin();
        let bump = (font_size * 1.6).max(radius * 0.18);
        let (label_x, label_y) = if outside {
            (
                cx + (radius + bump) * mid_angle.cos(),
                cy + (radius + bump) * mid_angle.sin(),
            )
        } else {
            let label_radius = radius * pie_cfg.text_position;
            (
                cx + label_radius * mid_angle.cos(),
                cy + label_radius * mid_angle.sin(),
            )
        };
        labels.push(PieLabel {
            text: label_text,
            font_size,
            outside,
            side: if mid_angle.cos() >= 0.0 { 1 } else { -1 },
            x: label_x,
            y: label_y,
            edge_x,
            edge_y,
            line_color: slice.color.clone(),
        });
    }

    let min_y = cy - radius * 1.1;
    let max_y = cy + radius * 1.1;
    let min_gap = theme.pie_section_text_size * 1.2;

    let mut left: Vec<usize> = Vec::new();
    let mut right: Vec<usize> = Vec::new();
    for (idx, label) in labels.iter().enumerate() {
        if label.outside {
            if label.side >= 0 {
                right.push(idx);
            } else {
                left.push(idx);
            }
        }
    }

    let distribute = |indices: &mut Vec<usize>, labels: &mut [PieLabel]| {
        indices.sort_by(|&a, &b| {
            labels[a]
                .y
                .partial_cmp(&labels[b].y)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut prev = min_y - min_gap;
        for &idx in indices.iter() {
            let y = labels[idx].y.max(prev + min_gap);
            labels[idx].y = y;
            prev = y;
        }
        if let Some(&last_idx) = indices.last() {
            let overflow = labels[last_idx].y - max_y;
            if overflow > 0.0 {
                for &idx in indices.iter() {
                    labels[idx].y -= overflow;
                }
            }
        }
        if let Some(&first_idx) = indices.first() {
            let underflow = min_y - labels[first_idx].y;
            if underflow > 0.0 {
                for &idx in indices.iter() {
                    labels[idx].y += underflow;
                }
            }
        }
    };

    distribute(&mut left, &mut labels);
    distribute(&mut right, &mut labels);

    for label in labels {
        let mut anchor = "middle";
        let mut label_x = label.x;
        if label.outside {
            let bump = (label.font_size * 1.6).max(radius * 0.18);
            if label.side >= 0 {
                label_x = cx + radius + bump;
                anchor = "start";
            } else {
                label_x = cx - radius - bump;
                anchor = "end";
            }
            let elbow_x = if label.side >= 0 {
                label_x - 6.0
            } else {
                label_x + 6.0
            };
            svg.push_str(&format!(
                "<path d=\"M {sx:.2},{sy:.2} L {mx:.2},{ly:.2} L {lx:.2},{ly:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\"/>",
                escape_xml(&label.line_color),
                sx = label.edge_x,
                sy = label.edge_y,
                mx = elbow_x,
                lx = label_x,
                ly = label.y
            ));
            let label_width = text_metrics::measure_text_width(
                label.text.as_str(),
                label.font_size,
                theme.font_family.as_str(),
            )
            .unwrap_or(label.text.chars().count() as f32 * label.font_size * 0.55);
            let pad_x = (label.font_size * 0.35).max(4.0);
            let pad_y = (label.font_size * 0.25).max(2.5);
            let rect_w = label_width + pad_x * 2.0;
            let rect_h = label.font_size + pad_y * 2.0;
            let rect_x = if label.side >= 0 {
                label_x - pad_x
            } else {
                label_x - rect_w + pad_x
            };
            let rect_y = label.y - rect_h / 2.0;
            let bg = if theme.edge_label_background == "none" {
                theme.background.as_str()
            } else {
                theme.edge_label_background.as_str()
            };
            svg.push_str(&format!(
                "<rect x=\"{rect_x:.2}\" y=\"{rect_y:.2}\" width=\"{rect_w:.2}\" height=\"{rect_h:.2}\" rx=\"2\" ry=\"2\" fill=\"{}\" stroke=\"none\"/>",
                escape_xml(bg)
            ));
        }
        svg.push_str(&format!(
            "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"{}\" dominant-baseline=\"middle\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
            label_x,
            label.y,
            anchor,
            normalize_font_family(&theme.font_family),
            label.font_size,
            escape_xml(&theme.pie_section_text_color),
            label.text
        ));
    }

    for item in &pie.legend {
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
        let label_y = rect_y + item.marker_size / 2.0;
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

    if let Some(title) = &pie.title {
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
        grid_x + half_w,
        grid_y,
        half_w,
        half_h,
        q_colors[0]
    ));
    svg.push_str(&format!(
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\"/>",
        grid_x, grid_y, half_w, half_h, q_colors[1]
    ));
    svg.push_str(&format!(
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\"/>",
        grid_x,
        grid_y + half_h,
        half_w,
        half_h,
        q_colors[2]
    ));
    svg.push_str(&format!(
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\"/>",
        grid_x + half_w,
        grid_y + half_h,
        half_w,
        half_h,
        q_colors[3]
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
            svg.push_str(&text_block_svg(
                lx,
                ly,
                label,
                theme,
                config,
                false,
                Some("#131300"),
            ));
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
        let axis_x = grid_x - theme.font_size * 2.2;
        let axis_y = grid_y + half_h + half_h / 2.0;
        svg.push_str(&format!(
            "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"end\" dominant-baseline=\"middle\" font-family=\"{}\" font-size=\"{}\" fill=\"#131300\"><tspan>{}</tspan></text>",
            axis_x,
            axis_y,
            normalize_font_family(&theme.font_family),
            theme.font_size,
            y_bottom.lines.first().map(|s| s.as_str()).unwrap_or("")
        ));
    }
    if let Some(ref y_top) = layout.y_axis_top {
        let axis_x = grid_x - theme.font_size * 2.2;
        let axis_y = grid_y + half_h / 2.0;
        svg.push_str(&format!(
            "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"end\" dominant-baseline=\"middle\" font-family=\"{}\" font-size=\"{}\" fill=\"#131300\"><tspan>{}</tspan></text>",
            axis_x,
            axis_y,
            normalize_font_family(&theme.font_family),
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
    let chart_left = layout.chart_x;
    let chart_right = layout.chart_x + layout.chart_width;
    let full_width = chart_right + layout.label_x;
    let bar_height = (layout.row_height * 0.82)
        .min(layout.row_height - 4.0)
        .max(theme.font_size * 1.1);

    // Title
    if let Some(ref title) = layout.title {
        svg.push_str(&text_block_svg(
            layout.chart_x + layout.chart_width / 2.0,
            layout.title_y,
            title,
            theme,
            config,
            false,
            Some(theme.primary_text_color.as_str()),
        ));
    }

    // Grid/ticks
    let axis_y = layout.chart_y + layout.chart_height + layout.row_height * 0.85;
    let tick_font = theme.font_size * 0.8;
    for tick in &layout.ticks {
        svg.push_str(&format!(
            "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"#E2E8F0\" stroke-width=\"1\"/>",
            tick.x, layout.chart_y, tick.x, layout.chart_y + layout.chart_height
        ));
        if !tick.label.trim().is_empty() {
            svg.push_str(&text_line_svg_with_font_size(
                tick.x,
                axis_y,
                tick.label.as_str(),
                theme,
                tick_font,
                theme.text_color.as_str(),
                "middle",
            ));
        }
    }
    svg.push_str(&format!(
        "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"1\"/>",
        chart_left,
        layout.chart_y + layout.chart_height,
        chart_right,
        layout.chart_y + layout.chart_height,
        theme.line_color
    ));
    svg.push_str(&format!(
        "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"#E2E8F0\" stroke-width=\"1\"/>",
        chart_left,
        layout.chart_y,
        chart_left,
        layout.chart_y + layout.chart_height
    ));

    // Draw sections
    let section_font = theme.font_size * 0.9;
    let task_font = theme.font_size * 0.85;
    for section in &layout.sections {
        let label_band_width = layout.chart_x;
        svg.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" fill-opacity=\"0.22\" stroke=\"none\"/>",
            0.0,
            section.y,
            label_band_width,
            section.height,
            section.band_color
        ));
        svg.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" fill-opacity=\"0.12\" stroke=\"none\"/>",
            layout.chart_x,
            section.y,
            layout.chart_width,
            section.height,
            section.band_color
        ));
        svg.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" fill-opacity=\"0.9\" stroke=\"none\"/>",
            0.0,
            section.y,
            (theme.font_size * 0.3).max(3.0),
            section.height,
            section.color
        ));
        let label_y = (section.y + layout.row_height * 0.55)
            .min(section.y + section.height - layout.row_height * 0.45);
        svg.push_str(&text_block_svg_with_font_size(
            layout.section_label_x,
            label_y,
            &section.label,
            theme,
            config,
            section_font,
            "start",
            Some(theme.primary_text_color.as_str()),
            false,
        ));
    }

    let mut row_lines: Vec<f32> = Vec::new();
    row_lines.push(layout.chart_y);
    for section in &layout.sections {
        row_lines.push(section.y);
        row_lines.push(section.y + section.height);
    }
    for task in &layout.tasks {
        row_lines.push(task.y);
    }
    row_lines.push(layout.chart_y + layout.chart_height);
    row_lines.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    row_lines.dedup_by(|a, b| (*a - *b).abs() < 0.5);
    for y in row_lines {
        svg.push_str(&format!(
            "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"#E2E8F0\" stroke-width=\"1\"/>",
            0.0, y, full_width, y
        ));
    }

    let gantt_label_color = |fill: &str| -> String {
        if let Some((_, _, l)) = parse_color_to_hsl(fill) {
            if l < 55.0 {
                "#FFFFFF".to_string()
            } else {
                "#0F172A".to_string()
            }
        } else {
            theme.primary_text_color.clone()
        }
    };

    // Draw tasks as bars
    for task in &layout.tasks {
        let row_center = task.y + layout.row_height / 2.0;
        let bar_y = row_center - bar_height / 2.0;
        let mut label_rendered_inside = false;
        if matches!(task.status, Some(crate::ir::GanttStatus::Milestone)) {
            let size = bar_height * 0.6;
            let cx = task.x;
            let cy = row_center;
            let points = format!(
                "{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
                cx,
                cy - size,
                cx + size,
                cy,
                cx,
                cy + size,
                cx - size,
                cy
            );
            svg.push_str(&format!(
                "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                points, task.color, theme.primary_border_color
            ));
        } else {
            svg.push_str(&format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"3\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                task.x,
                bar_y,
                task.width,
                bar_height,
                task.color,
                theme.primary_border_color
            ));
            let label_text = task
                .label
                .lines
                .iter()
                .find(|line| !line.trim().is_empty())
                .map(|s| s.as_str())
                .unwrap_or("");
            if !label_text.is_empty() {
                let font_size = task_font * 0.95;
                let text_width = text_metrics::measure_text_width(
                    label_text,
                    font_size,
                    theme.font_family.as_str(),
                )
                .unwrap_or(label_text.chars().count() as f32 * font_size * 0.55);
                let pad = (font_size * 0.6).max(6.0);
                if task.width >= text_width + pad * 2.0 && bar_height >= font_size * 1.1 {
                    let text_x = task.x + task.width / 2.0;
                    let text_y = row_center;
                    svg.push_str(&format!(
                        "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"{}\" font-size=\"{:.2}\" fill=\"{}\">{}</text>",
                        text_x,
                        text_y,
                        normalize_font_family(&theme.font_family),
                        font_size,
                        escape_xml(&gantt_label_color(&task.color)),
                        escape_xml(label_text)
                    ));
                    label_rendered_inside = true;
                }
            }
        }
        // Task label
        if !label_rendered_inside {
            svg.push_str(&text_block_svg_with_font_size(
                layout.task_label_x,
                row_center,
                &task.label,
                theme,
                config,
                task_font,
                "start",
                Some(theme.primary_text_color.as_str()),
                false,
            ));
        }
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
            normalize_font_family(&theme.font_family), theme.font_size * 0.8,
            theme.primary_text_color, escape_xml(label)
        ));
    }

    // X-axis categories
    for (label, x) in &layout.x_axis_categories {
        svg.push_str(&format!(
            "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{:.1}\" fill=\"{}\">{}</text>",
            x, layout.plot_y + layout.plot_height + 20.0,
            normalize_font_family(&theme.font_family), theme.font_size * 0.9,
            theme.primary_text_color, escape_xml(label)
        ));
    }

    // Y-axis label
    if let Some(ref y_label) = layout.y_axis_label {
        svg.push_str(&format!(
            "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{:.1}\" fill=\"{}\" transform=\"rotate(-90, {:.2}, {:.2})\">{}</text>",
            layout.y_axis_label_x, layout.plot_y + layout.plot_height / 2.0,
            normalize_font_family(&theme.font_family), theme.font_size,
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
            let path: String = line
                .points
                .iter()
                .enumerate()
                .map(|(i, (x, y))| {
                    if i == 0 {
                        format!("M {:.2},{:.2}", x, y)
                    } else {
                        format!(" L {:.2},{:.2}", x, y)
                    }
                })
                .collect();
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
        "#ECECFF", "#FFE6CC", "#D5E8D4", "#F8CECC", "#FFF2CC", "#E1D5E7",
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
            normalize_font_family(&theme.font_family), theme.font_size,
            theme.primary_text_color, escape_xml(&event.time.lines.join(" "))
        ));

        // Event descriptions
        let mut y_offset = 40.0;
        for evt in &event.events {
            svg.push_str(&format!(
                "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{:.1}\" fill=\"{}\">{}</text>",
                center_x, event.y + y_offset,
                normalize_font_family(&theme.font_family), theme.font_size * 0.9,
                theme.primary_text_color, escape_xml(&evt.lines.join(" "))
            ));
            y_offset += theme.font_size * 1.2;
        }
    }

    svg
}

fn render_journey(layout: &JourneyLayout, theme: &Theme, config: &LayoutConfig) -> String {
    let mut svg = String::new();

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

    let mut actor_colors: HashMap<String, String> = HashMap::new();
    for actor in &layout.actors {
        actor_colors.insert(actor.name.clone(), actor.color.clone());
        svg.push_str(&format!(
            "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            actor.x,
            actor.y,
            actor.radius,
            actor.color,
            theme.line_color
        ));
        let label_x = actor.x + actor.radius + layout.actor_gap;
        svg.push_str(&text_line_svg(
            label_x,
            layout.actor_label_y,
            actor.name.as_str(),
            theme,
            theme.primary_text_color.as_str(),
            "start",
        ));
    }

    for section in &layout.sections {
        let fill = section.color.as_str();
        svg.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"8\" ry=\"8\" fill=\"{}\" fill-opacity=\"0.18\" stroke=\"{}\" stroke-width=\"1\"/>",
            section.x,
            section.y,
            section.width,
            section.height,
            fill,
            theme.cluster_border
        ));
        if !section.label.lines.is_empty()
            && !section.label.lines.iter().all(|l| l.trim().is_empty())
        {
            let label_x = section.x + section.width / 2.0;
            let label_y = section.y + section.height / 2.0;
            svg.push_str(&text_block_svg(
                label_x,
                label_y,
                &section.label,
                theme,
                config,
                false,
                Some(theme.primary_text_color.as_str()),
            ));
        }
    }

    for task in &layout.tasks {
        svg.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"10\" ry=\"10\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.2\"/>",
            task.x,
            task.y,
            task.width,
            task.height,
            theme.primary_color,
            theme.primary_border_color
        ));
        let label_x = task.x + task.width / 2.0;
        let label_y = task.y + task.height / 2.0;
        svg.push_str(&text_block_svg(
            label_x,
            label_y,
            &task.label,
            theme,
            config,
            false,
            Some(theme.primary_text_color.as_str()),
        ));

        if let Some(score) = task.score {
            let score_x = task.x + layout.score_radius + theme.font_size * 0.2;
            svg.push_str(&format!(
                "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                score_x,
                task.score_y,
                layout.score_radius,
                task.score_color,
                theme.line_color
            ));
            let score_text = format!("{:.0}", score);
            svg.push_str(&text_line_svg(
                score_x,
                task.score_y + theme.font_size * 0.35,
                score_text.as_str(),
                theme,
                theme.primary_text_color.as_str(),
                "middle",
            ));
        }

        if let Some(actor_y) = task.actor_y {
            let count = task.actors.len();
            if count > 0 {
                let total_width = count as f32 * layout.actor_radius * 2.0
                    + (count.saturating_sub(1)) as f32 * layout.actor_gap;
                let start_x = task.x + task.width / 2.0 - total_width / 2.0;
                for (idx, actor) in task.actors.iter().enumerate() {
                    let color = actor_colors
                        .get(actor)
                        .map(|c| c.as_str())
                        .unwrap_or(theme.secondary_color.as_str());
                    let cx = start_x
                        + idx as f32 * (layout.actor_radius * 2.0 + layout.actor_gap)
                        + layout.actor_radius;
                    svg.push_str(&format!(
                        "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                        cx,
                        actor_y,
                        layout.actor_radius,
                        color,
                        theme.line_color
                    ));
                }
            }
        }
    }

    if let Some((x1, y, x2)) = layout.baseline {
        svg.push_str(&format!(
            "<line x1=\"{x1:.2}\" y1=\"{y:.2}\" x2=\"{x2:.2}\" y2=\"{y:.2}\" stroke=\"{}\" stroke-width=\"2\"/>",
            theme.line_color
        ));
        let arrow = 8.0;
        svg.push_str(&format!(
            "<polygon points=\"{x2:.2},{y:.2} {ax:.2},{ay1:.2} {ax:.2},{ay2:.2}\" fill=\"{}\"/>",
            theme.line_color,
            ax = x2 - arrow,
            ay1 = y - arrow * 0.6,
            ay2 = y + arrow * 0.6
        ));
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
                crate::ir::Direction::TopDown => {
                    (branch.pos, gg.default_pos, branch.pos, gitgraph.max_pos)
                }
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
                normalize_font_family(&theme.font_family),
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
                    normalize_font_family(&theme.font_family),
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
        normalize_font_family(font_family),
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
        normalize_font_family(&theme.font_family),
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

fn text_block_svg_with_font_size_weight(
    x: f32,
    y: f32,
    label: &TextBlock,
    theme: &Theme,
    config: &LayoutConfig,
    font_size: f32,
    anchor: &str,
    override_color: Option<&str>,
    font_weight: Option<&str>,
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
    let weight_attr = font_weight
        .filter(|w| !w.trim().is_empty())
        .map(|w| format!(" font-weight=\"{}\"", w))
        .unwrap_or_default();

    text.push_str(&format!(
        "<text x=\"{x:.2}\" y=\"{start_y:.2}\" text-anchor=\"{anchor}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\"{weight_attr}>",
        normalize_font_family(&theme.font_family),
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

fn text_line_svg_with_font_size(
    x: f32,
    y: f32,
    text: &str,
    theme: &Theme,
    font_size: f32,
    fill: &str,
    anchor: &str,
) -> String {
    format!(
        "<text x=\"{x:.2}\" y=\"{y:.2}\" text-anchor=\"{anchor}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
        normalize_font_family(&theme.font_family),
        font_size,
        fill,
        escape_xml(text)
    )
}

fn text_line_svg(x: f32, y: f32, text: &str, theme: &Theme, fill: &str, anchor: &str) -> String {
    format!(
        "<text x=\"{x:.2}\" y=\"{y:.2}\" text-anchor=\"{anchor}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
        normalize_font_family(&theme.font_family),
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
    let fill = shape.bg_color.as_deref().unwrap_or(default_fill);
    let stroke = shape.border_color.as_deref().unwrap_or(default_stroke);
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
        normalize_font_family(type_font_family),
        type_font_size,
        shape.type_label.width.round(),
        shape.x + shape.width / 2.0 - shape.type_label.width / 2.0,
        shape.y + shape.type_label.y,
        escape_xml(&shape.type_label.text)
    ));

    if let Some(image_y) = shape.image_y
        && matches!(
            shape.kind,
            crate::ir::C4ShapeKind::Person | crate::ir::C4ShapeKind::ExternalPerson
        )
    {
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
    let fill = boundary.bg_color.as_deref().unwrap_or(&conf.boundary_fill);
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
        rect_attrs.push_str(&format!(
            " stroke-dasharray=\"{}\"",
            escape_xml(&conf.boundary_dasharray)
        ));
    }
    if conf.boundary_fill != "none" && conf.boundary_fill_opacity < 1.0 {
        rect_attrs.push_str(&format!(
            " fill-opacity=\"{:.2}\"",
            conf.boundary_fill_opacity
        ));
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
    let stroke = rel.line_color.as_deref().unwrap_or(&conf.boundary_stroke);
    if straight {
        let mut attrs = String::new();
        if rel.kind != crate::ir::C4RelKind::RelBack {
            attrs.push_str(" marker-end=\"url(#arrowhead)\"");
        }
        if matches!(
            rel.kind,
            crate::ir::C4RelKind::BiRel | crate::ir::C4RelKind::RelBack
        ) {
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
        if matches!(
            rel.kind,
            crate::ir::C4RelKind::BiRel | crate::ir::C4RelKind::RelBack
        ) {
            path.push_str(" marker-start=\"url(#arrowend)\"");
        }
        path.push_str("/>");
        svg.push_str(&path);
    }

    let text_color = rel.text_color.as_deref().unwrap_or(&conf.boundary_stroke);
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
            normalize_font_family(font_family),
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
        crate::ir::C4ShapeKind::SystemDb => {
            (&conf.system_db_bg_color, &conf.system_db_border_color)
        }
        crate::ir::C4ShapeKind::SystemQueue => {
            (&conf.system_queue_bg_color, &conf.system_queue_border_color)
        }
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
        crate::ir::C4ShapeKind::Container => {
            (&conf.container_bg_color, &conf.container_border_color)
        }
        crate::ir::C4ShapeKind::ContainerDb => {
            (&conf.container_db_bg_color, &conf.container_db_border_color)
        }
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
        crate::ir::C4ShapeKind::Component => {
            (&conf.component_bg_color, &conf.component_border_color)
        }
        crate::ir::C4ShapeKind::ComponentDb => {
            (&conf.component_db_bg_color, &conf.component_db_border_color)
        }
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
        crate::ir::C4ShapeKind::ExternalContainerQueue => {
            &conf.external_container_queue_font_family
        }
        crate::ir::C4ShapeKind::Component => &conf.component_font_family,
        crate::ir::C4ShapeKind::ComponentDb => &conf.component_db_font_family,
        crate::ir::C4ShapeKind::ComponentQueue => &conf.component_queue_font_family,
        crate::ir::C4ShapeKind::ExternalComponent => &conf.external_component_font_family,
        crate::ir::C4ShapeKind::ExternalComponentDb => &conf.external_component_db_font_family,
        crate::ir::C4ShapeKind::ExternalComponentQueue => {
            &conf.external_component_queue_font_family
        }
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
        crate::ir::C4ShapeKind::ExternalContainerQueue => {
            &conf.external_container_queue_font_weight
        }
        crate::ir::C4ShapeKind::Component => &conf.component_font_weight,
        crate::ir::C4ShapeKind::ComponentDb => &conf.component_db_font_weight,
        crate::ir::C4ShapeKind::ComponentQueue => &conf.component_queue_font_weight,
        crate::ir::C4ShapeKind::ExternalComponent => &conf.external_component_font_weight,
        crate::ir::C4ShapeKind::ExternalComponentDb => &conf.external_component_db_font_weight,
        crate::ir::C4ShapeKind::ExternalComponentQueue => {
            &conf.external_component_queue_font_weight
        }
    }
}

fn text_block_svg_class(
    node: &crate::layout::NodeLayout,
    theme: &Theme,
    config: &LayoutConfig,
    override_color: Option<&str>,
) -> String {
    let line_height = theme.font_size * config.class_label_line_height();
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
        let lines: Vec<(usize, &str)> = node
            .label
            .lines
            .iter()
            .enumerate()
            .map(|(idx, line)| (idx, line.as_str()))
            .collect();
        return text_lines_svg(
            &lines,
            center_x,
            start_y,
            line_height,
            "middle",
            theme,
            fill,
            false,
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

fn render_er_node_label(
    node: &crate::layout::NodeLayout,
    theme: &Theme,
    config: &LayoutConfig,
) -> Option<String> {
    let divider_idx = node
        .label
        .lines
        .iter()
        .position(|line| is_divider_line(line))?;
    let line_height = theme.font_size * config.class_label_line_height();
    let total_height = node.label.lines.len() as f32 * line_height;
    let start_y = node.y + node.height / 2.0 - total_height / 2.0 + theme.font_size;
    let center_x = node.x + node.width / 2.0;
    let left_x = node.x + config.node_padding_x.max(10.0);
    let fill = node
        .style
        .text_color
        .as_deref()
        .unwrap_or(theme.primary_text_color.as_str());

    let mut title_lines: Vec<(usize, &str)> = Vec::new();
    for (idx, line) in node.label.lines.iter().enumerate().take(divider_idx) {
        if !line.trim().is_empty() {
            title_lines.push((idx, line.as_str()));
        }
    }
    let mut attr_lines: Vec<(usize, &str)> = Vec::new();
    for (idx, line) in node.label.lines.iter().enumerate().skip(divider_idx + 1) {
        if !line.trim().is_empty() && !is_divider_line(line) {
            attr_lines.push((idx, line.as_str()));
        }
    }

    let mut svg = String::new();
    if !title_lines.is_empty() {
        let divider_baseline = start_y + divider_idx as f32 * line_height;
        let header_bottom = divider_baseline - line_height * 0.3;
        let header_top = (start_y - line_height * 0.9).min(header_bottom);
        let header_height = (header_bottom - header_top).max(0.0);
        if header_height > 0.0 {
            svg.push_str(&format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"6\" ry=\"6\" fill=\"{}\" fill-opacity=\"0.22\" stroke=\"none\"/>",
                node.x + 0.5,
                header_top,
                (node.width - 1.0).max(0.0),
                header_height,
                theme.cluster_background
            ));
        }
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
        svg.push_str(&format!(
            "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"1\" stroke-opacity=\"0.35\"/>",
            node.x + 0.8,
            header_bottom,
            node.x + node.width - 0.8,
            header_bottom,
            theme.primary_border_color
        ));
    }

    if !attr_lines.is_empty() {
        let mut parsed: Vec<(usize, String, String)> = Vec::new();
        let mut max_type_width: f32 = 0.0;
        let mut use_columns = true;
        for (idx, line) in &attr_lines {
            let trimmed = line.trim();
            let mut parts = trimmed.split_whitespace();
            let Some(first) = parts.next() else {
                continue;
            };
            let rest = trimmed[first.len()..].trim();
            if rest.is_empty() {
                use_columns = false;
                break;
            }
            let width = text_metrics::measure_text_width(
                first,
                theme.font_size,
                theme.font_family.as_str(),
            )
            .unwrap_or(first.chars().count() as f32 * theme.font_size * 0.6);
            max_type_width = max_type_width.max(width);
            parsed.push((*idx, first.to_string(), rest.to_string()));
        }

        let pad_x = config.node_padding_x.max(10.0);
        let content_width = (node.width - pad_x * 2.0).max(0.0);
        let gap = theme.font_size * 0.65;
        let name_x = left_x + max_type_width + gap;
        let body_top = start_y + (divider_idx as f32 + 0.15) * line_height;
        let body_bottom = node.y + node.height - line_height * 0.25;

        for (row_idx, (idx, _)) in attr_lines.iter().enumerate() {
            if row_idx % 2 == 0 {
                let row_top = start_y + *idx as f32 * line_height - line_height * 0.85;
                let row_height = line_height;
                svg.push_str(&format!(
                    "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" fill-opacity=\"0.12\" stroke=\"none\"/>",
                    node.x + 0.5,
                    row_top,
                    (node.width - 1.0).max(0.0),
                    row_height,
                    theme.secondary_color
                ));
            }
        }

        if use_columns && name_x < node.x + pad_x + content_width {
            let divider_x = name_x - gap * 0.5;
            svg.push_str(&format!(
                "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"1\" stroke-opacity=\"0.2\"/>",
                divider_x,
                body_top,
                divider_x,
                body_bottom,
                theme.primary_border_color
            ));
            for (idx, ty, name) in parsed {
                let y = start_y + idx as f32 * line_height;
                svg.push_str(&format!(
                    "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"start\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\" fill-opacity=\"0.75\">{}</text>",
                    left_x,
                    y,
                    normalize_font_family(&theme.font_family),
                    theme.font_size,
                    fill,
                    escape_xml(&ty)
                ));
                svg.push_str(&format!(
                    "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"start\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
                    name_x,
                    y,
                    normalize_font_family(&theme.font_family),
                    theme.font_size,
                    fill,
                    escape_xml(&name)
                ));
            }
        } else {
            svg.push_str(&text_lines_svg(
                &attr_lines,
                left_x,
                start_y,
                line_height,
                "start",
                theme,
                fill,
                false,
            ));
        }
    }

    Some(svg)
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
        normalize_font_family(&theme.font_family),
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

fn is_divider_line(line: &str) -> bool {
    line.trim() == "---"
}

fn divider_lines_svg(node: &crate::layout::NodeLayout, theme: &Theme, line_height: f32) -> String {
    if !node.label.lines.iter().any(|line| is_divider_line(line)) {
        return String::new();
    }

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

#[derive(Debug, Clone)]
struct ErAttribute {
    name: String,
    data_type: String,
    keys: Vec<String>,
}

fn parse_er_attributes(lines: &[String]) -> (String, Vec<ErAttribute>) {
    let mut title = lines
        .first()
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let mut attrs = Vec::new();
    let mut in_body = false;
    for line in lines.iter().skip(1) {
        if is_divider_line(line) {
            in_body = true;
            continue;
        }
        if !in_body {
            if !line.trim().is_empty() {
                title = line.trim().to_string();
            }
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut keys = Vec::new();
        let mut parts: Vec<String> = Vec::new();
        for token in trimmed.split_whitespace() {
            let cleaned = token
                .trim_matches(|ch: char| ch == ',' || ch == ';')
                .to_ascii_uppercase();
            if cleaned == "PK" || cleaned == "FK" || cleaned == "UK" {
                keys.push(cleaned);
                continue;
            }
            if cleaned.contains(',') {
                let mut handled = false;
                for piece in cleaned.split(',') {
                    if piece == "PK" || piece == "FK" || piece == "UK" {
                        keys.push(piece.to_string());
                        handled = true;
                    }
                }
                if handled {
                    continue;
                }
            }
            parts.push(token.to_string());
        }
        if parts.is_empty() {
            continue;
        }
        let (data_type, name) = if parts.len() >= 2 {
            (parts[0].clone(), parts[1..].join(" "))
        } else {
            (String::new(), parts[0].clone())
        };
        attrs.push(ErAttribute {
            name,
            data_type,
            keys,
        });
    }
    (title, attrs)
}

fn er_badge_svg(
    x: f32,
    y: f32,
    text: &str,
    font_size: f32,
    fill: &str,
    text_color: &str,
    font_family: &str,
) -> (String, f32) {
    let font_family = normalize_font_family(font_family);
    let pad_x = (font_size * 0.45).max(4.0);
    let text_width = text_metrics::measure_text_width(text, font_size * 0.72, &font_family)
        .unwrap_or(font_size * 0.9);
    let width = text_width + pad_x * 2.0;
    let height = (font_size * 0.9).max(10.0);
    let rect_y = y - height / 2.0;
    let rx = (height / 2.0).max(4.0);
    let mut svg = String::new();
    svg.push_str(&format!(
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.2}\" ry=\"{:.2}\" fill=\"{}\"/>",
        x, rect_y, width, height, rx, rx, fill
    ));
    svg.push_str(&format!(
        "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{:.2}\" font-weight=\"600\" fill=\"{}\">{}</text>",
        x + width / 2.0,
        y + font_size * 0.26,
        font_family,
        font_size * 0.72,
        text_color,
        escape_xml(text)
    ));
    (svg, width)
}

fn render_er_node(
    node: &crate::layout::NodeLayout,
    theme: &Theme,
    config: &LayoutConfig,
) -> String {
    let (title, attrs) = parse_er_attributes(&node.label.lines);
    let font_size = theme.font_size;
    let line_height = font_size * config.label_line_height;
    let header_height = if attrs.is_empty() {
        node.height
    } else {
        (line_height + font_size * 0.6)
            .min(node.height * 0.5)
            .max(line_height + 6.0)
    };

    let border = node
        .style
        .stroke
        .as_ref()
        .unwrap_or(&theme.primary_border_color);
    let body_fill = node.style.fill.as_ref().unwrap_or(&theme.background);
    let header_fill = theme.cluster_background.as_str();
    let grid_color = theme.cluster_border.as_str();
    let header_text_color = theme.primary_text_color.as_str();
    let name_text_color = theme.primary_text_color.as_str();
    let type_text_color = theme.line_color.as_str();

    let x = node.x;
    let y = node.y;
    let w = node.width;
    let h = node.height;
    let radius = 6.0;

    let mut svg = String::new();
    svg.push_str(&format!(
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.2}\" ry=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>",
        x,
        y,
        w,
        h,
        radius,
        radius,
        body_fill,
        border,
        node.style.stroke_width.unwrap_or(1.2)
    ));

    svg.push_str(&format!(
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.2}\" ry=\"{:.2}\" fill=\"{}\"/>",
        x,
        y,
        w,
        header_height,
        radius,
        radius,
        header_fill
    ));

    let header_label = TextBlock {
        lines: vec![title.clone()],
        width: 0.0,
        height: 0.0,
    };
    let header_y = y + header_height / 2.0;
    svg.push_str(&text_block_svg_anchor(
        x + w / 2.0,
        header_y,
        &header_label,
        theme,
        config,
        "middle",
        Some(header_text_color),
    ));

    if attrs.is_empty() {
        return svg;
    }

    let pad_x = (font_size * 0.8).max(10.0);
    let mut max_type_width = 0.0f32;
    let mut max_name_width = 0.0f32;
    let mut max_badge_width = 0.0f32;
    for attr in &attrs {
        if !attr.data_type.is_empty() {
            if let Some(width) =
                text_metrics::measure_text_width(&attr.data_type, font_size, &theme.font_family)
            {
                max_type_width = max_type_width.max(width);
            }
        }
        if let Some(width) =
            text_metrics::measure_text_width(&attr.name, font_size, &theme.font_family)
        {
            max_name_width = max_name_width.max(width);
        }
        if !attr.keys.is_empty() {
            let mut row_badge_width = 0.0f32;
            for key in attr.keys.iter().take(2) {
                let text_width =
                    text_metrics::measure_text_width(key, font_size * 0.72, &theme.font_family)
                        .unwrap_or(font_size * 0.9);
                let badge_width = text_width + (font_size * 0.45).max(4.0) * 2.0;
                row_badge_width += badge_width + font_size * 0.4;
            }
            if row_badge_width > 0.0 {
                row_badge_width -= font_size * 0.4;
            }
            max_badge_width = max_badge_width.max(row_badge_width);
        }
    }

    let type_col_pad = font_size * 0.9;
    let available = (w - pad_x * 2.0).max(font_size * 4.0);
    let mut type_col_width = if max_type_width > 0.0 {
        (max_type_width + type_col_pad * 2.0).min(available * 0.45)
    } else {
        0.0
    };
    let min_name_width = (max_name_width + font_size * 0.6).min(available * 0.7);
    let min_type_width = if max_type_width > 0.0 {
        (font_size * 2.8).max(36.0)
    } else {
        0.0
    };
    if type_col_width < min_type_width {
        type_col_width = min_type_width;
    }
    let mut col_x = x + w - pad_x - type_col_width;
    let min_col_x = x + pad_x + max_badge_width + min_name_width;
    if col_x < min_col_x {
        col_x = min_col_x;
    }
    let show_type_col = type_col_width > 0.0 && col_x < x + w - pad_x - 8.0;

    svg.push_str(&format!(
        "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"1.0\" stroke-opacity=\"0.6\"/>",
        x,
        y + header_height,
        x + w,
        y + header_height,
        grid_color
    ));

    if show_type_col {
        svg.push_str(&format!(
            "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"1.0\" stroke-opacity=\"0.45\"/>",
            col_x,
            y + header_height,
            col_x,
            y + h,
            grid_color
        ));
    }

    let mut row_height = line_height;
    let body_height = (h - header_height).max(line_height);
    if !attrs.is_empty() {
        let needed = attrs.len() as f32 * row_height;
        if needed > body_height {
            row_height = body_height / attrs.len() as f32;
        }
    }
    for (idx, attr) in attrs.iter().enumerate() {
        let row_top = y + header_height + idx as f32 * row_height;
        let row_center = row_top + row_height / 2.0;
        if idx > 0 {
            svg.push_str(&format!(
                "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"1.0\" stroke-opacity=\"0.35\"/>",
                x,
                row_top,
                x + w,
                row_top,
                grid_color
            ));
        }

        let mut cursor_x = x + pad_x;
        for key in attr.keys.iter().take(2) {
            let fill = match key.as_str() {
                "PK" => "#1D4ED8",
                "FK" => "#0F766E",
                "UK" => "#7C3AED",
                _ => "#475569",
            };
            let (badge_svg, badge_width) = er_badge_svg(
                cursor_x,
                row_center,
                key,
                font_size,
                fill,
                "#FFFFFF",
                &theme.font_family,
            );
            svg.push_str(&badge_svg);
            cursor_x += badge_width + font_size * 0.4;
        }

        let name_label = TextBlock {
            lines: vec![attr.name.clone()],
            width: 0.0,
            height: 0.0,
        };
        svg.push_str(&text_block_svg_anchor(
            cursor_x,
            row_center,
            &name_label,
            theme,
            config,
            "start",
            Some(name_text_color),
        ));

        if show_type_col && !attr.data_type.is_empty() {
            let type_label = TextBlock {
                lines: vec![attr.data_type.clone()],
                width: 0.0,
                height: 0.0,
            };
            svg.push_str(&text_block_svg_anchor(
                x + w - pad_x,
                row_center,
                &type_label,
                theme,
                config,
                "end",
                Some(type_text_color),
            ));
        }
    }

    svg
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
            "<circle cx=\"0\" cy=\"0\" r=\"5\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/>",
            stroke, stroke_width
        ),
        crate::ir::EdgeDecoration::Cross => format!(
            "<path d=\"M -5 -5 L 5 5 M -5 5 L 5 -5\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{join}/>",
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

fn arrowhead_svg(point: (f32, f32), angle_deg: f32, stroke: &str, stroke_width: f32) -> String {
    let size = (stroke_width * 2.2 + 6.0).clamp(6.0, 14.0);
    let half = size * 0.6;
    let (x, y) = point;
    let join = " stroke-linejoin=\"round\" stroke-linecap=\"round\"";
    format!(
        "<g transform=\"translate({x:.2} {y:.2}) rotate({angle_deg:.2})\"><polygon points=\"0,0 {neg_size:.2},{half:.2} {neg_size:.2},{neg_half:.2}\" fill=\"{stroke}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{join}/></g>",
        neg_size = -size,
        half = half,
        neg_half = -half,
    )
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
            let rd = config
                .mindmap
                .default_corner_radius
                .max(theme.font_size * 0.55)
                .max(4.0);
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
            let line_y = y + h - stroke_width.max(0.8);
            svg.push_str(&format!(
                "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"{:.2}\" stroke-opacity=\"0.35\"/>",
                x,
                line_y,
                x + w,
                line_y,
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
        assert!(svg.contains("id=\"edge-0\""));
        assert!(svg.contains("data-edge-id=\"edge-0\""));
        assert!(svg.contains("data-label-kind=\"center\""));
    }

    #[test]
    fn center_label_background_hidden_when_path_is_clear() {
        let points = vec![(0.0, 0.0), (120.0, 0.0)];
        let touching = LabelRect {
            x: 40.0,
            y: -5.0,
            width: 24.0,
            height: 10.0,
        };
        assert!(edge_label_background_visible(
            crate::ir::DiagramKind::Flowchart,
            EdgeLabelKind::Center,
            &points,
            touching
        ));

        let detached = LabelRect {
            x: 40.0,
            y: -30.0,
            width: 24.0,
            height: 10.0,
        };
        assert!(!edge_label_background_visible(
            crate::ir::DiagramKind::Flowchart,
            EdgeLabelKind::Center,
            &points,
            detached
        ));
    }

    #[test]
    fn endpoint_label_background_prefers_no_box_when_not_touching() {
        let points = vec![(0.0, 0.0), (120.0, 0.0)];
        let detached = LabelRect {
            x: 8.0,
            y: -14.0,
            width: 16.0,
            height: 8.0,
        };
        assert!(!edge_label_background_visible(
            crate::ir::DiagramKind::Class,
            EdgeLabelKind::Start,
            &points,
            detached
        ));

        let touching = LabelRect {
            x: 8.0,
            y: -4.0,
            width: 16.0,
            height: 8.0,
        };
        assert!(!edge_label_background_visible(
            crate::ir::DiagramKind::Class,
            EdgeLabelKind::Start,
            &points,
            touching
        ));
        assert!(edge_label_background_visible(
            crate::ir::DiagramKind::Sequence,
            EdgeLabelKind::Start,
            &points,
            touching
        ));
    }

    #[test]
    fn sequence_center_label_background_visible_for_near_clearance() {
        let points = vec![(0.0, 0.0), (120.0, 0.0)];
        let near = LabelRect {
            x: 40.0,
            y: -11.5,
            width: 24.0,
            height: 10.0,
        };
        assert!(edge_label_background_visible(
            crate::ir::DiagramKind::Sequence,
            EdgeLabelKind::Center,
            &points,
            near
        ));
        assert!(!edge_label_background_visible(
            crate::ir::DiagramKind::Flowchart,
            EdgeLabelKind::Center,
            &points,
            near
        ));
    }

    #[test]
    fn sequence_endpoint_label_background_visible_for_small_non_touch_gap() {
        let points = vec![(0.0, 0.0), (120.0, 0.0)];
        let near = LabelRect {
            x: 8.0,
            y: -8.9,
            width: 16.0,
            height: 8.0,
        };
        assert!(edge_label_background_visible(
            crate::ir::DiagramKind::Sequence,
            EdgeLabelKind::Start,
            &points,
            near
        ));
        assert!(!edge_label_background_visible(
            crate::ir::DiagramKind::Class,
            EdgeLabelKind::Start,
            &points,
            near
        ));
    }
}
