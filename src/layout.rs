use crate::config::LayoutConfig;
use crate::ir::{Direction, Graph};
use crate::theme::Theme;
use dagre_rust::{
    GraphConfig as DagreConfig, GraphEdge as DagreEdge, GraphNode as DagreNode,
    layout as dagre_layout,
};
use graphlib_rust::{Graph as DagreGraph, GraphOption};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

#[derive(Debug, Clone)]
pub struct TextBlock {
    pub lines: Vec<String>,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct NodeLayout {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub label: TextBlock,
    pub shape: crate::ir::NodeShape,
    pub style: crate::ir::NodeStyle,
    pub link: Option<crate::ir::NodeLink>,
    pub anchor_subgraph: Option<usize>,
    pub hidden: bool,
}

#[derive(Debug, Clone)]
pub struct EdgeLayout {
    pub from: String,
    pub to: String,
    pub label: Option<TextBlock>,
    pub start_label: Option<TextBlock>,
    pub end_label: Option<TextBlock>,
    pub points: Vec<(f32, f32)>,
    pub directed: bool,
    pub arrow_start: bool,
    pub arrow_end: bool,
    pub arrow_start_kind: Option<crate::ir::EdgeArrowhead>,
    pub arrow_end_kind: Option<crate::ir::EdgeArrowhead>,
    pub start_decoration: Option<crate::ir::EdgeDecoration>,
    pub end_decoration: Option<crate::ir::EdgeDecoration>,
    pub style: crate::ir::EdgeStyle,
    pub override_style: crate::ir::EdgeStyleOverride,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum EdgeSide {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy)]
struct EdgePortInfo {
    start_side: EdgeSide,
    end_side: EdgeSide,
    start_offset: f32,
    end_offset: f32,
}

#[derive(Debug, Clone)]
struct PortCandidate {
    edge_idx: usize,
    is_start: bool,
    other_pos: f32,
}

#[derive(Debug, Clone)]
pub struct SubgraphLayout {
    pub label: String,
    pub label_block: TextBlock,
    pub nodes: Vec<String>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub style: crate::ir::NodeStyle,
}

#[derive(Debug, Clone)]
pub struct Lifeline {
    pub id: String,
    pub x: f32,
    pub y1: f32,
    pub y2: f32,
}

#[derive(Debug, Clone)]
pub struct SequenceLabel {
    pub x: f32,
    pub y: f32,
    pub text: TextBlock,
}

#[derive(Debug, Clone)]
pub struct SequenceFrameLayout {
    pub kind: crate::ir::SequenceFrameKind,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub label_box: (f32, f32, f32, f32),
    pub label: SequenceLabel,
    pub section_labels: Vec<SequenceLabel>,
    pub dividers: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct SequenceBoxLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub label: Option<TextBlock>,
    pub color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SequenceNoteLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub label: TextBlock,
    pub position: crate::ir::SequenceNotePosition,
    pub participants: Vec<String>,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct StateNoteLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub label: TextBlock,
    pub position: crate::ir::StateNotePosition,
    pub target: String,
}

#[derive(Debug, Clone)]
pub struct SequenceActivationLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub participant: String,
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub struct SequenceNumberLayout {
    pub x: f32,
    pub y: f32,
    pub value: usize,
}

#[derive(Debug, Clone)]
pub struct PieSliceLayout {
    pub label: TextBlock,
    pub value: f32,
    pub start_angle: f32,
    pub end_angle: f32,
    pub color: String,
}

#[derive(Debug, Clone)]
pub struct PieLegendItem {
    pub x: f32,
    pub y: f32,
    pub label: TextBlock,
    pub color: String,
    pub marker_size: f32,
}

#[derive(Debug, Clone)]
pub struct PieTitleLayout {
    pub x: f32,
    pub y: f32,
    pub text: TextBlock,
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub kind: crate::ir::DiagramKind,
    pub nodes: BTreeMap<String, NodeLayout>,
    pub edges: Vec<EdgeLayout>,
    pub subgraphs: Vec<SubgraphLayout>,
    pub lifelines: Vec<Lifeline>,
    pub sequence_footboxes: Vec<NodeLayout>,
    pub sequence_boxes: Vec<SequenceBoxLayout>,
    pub sequence_frames: Vec<SequenceFrameLayout>,
    pub sequence_notes: Vec<SequenceNoteLayout>,
    pub sequence_activations: Vec<SequenceActivationLayout>,
    pub sequence_numbers: Vec<SequenceNumberLayout>,
    pub state_notes: Vec<StateNoteLayout>,
    pub pie_slices: Vec<PieSliceLayout>,
    pub pie_legend: Vec<PieLegendItem>,
    pub pie_center: (f32, f32),
    pub pie_radius: f32,
    pub pie_title: Option<PieTitleLayout>,
    pub quadrant: Option<QuadrantLayout>,
    pub gantt: Option<GanttLayout>,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct QuadrantLayout {
    pub title: Option<TextBlock>,
    pub title_y: f32,
    pub x_axis_left: Option<TextBlock>,
    pub x_axis_right: Option<TextBlock>,
    pub y_axis_bottom: Option<TextBlock>,
    pub y_axis_top: Option<TextBlock>,
    pub quadrant_labels: [Option<TextBlock>; 4],
    pub points: Vec<QuadrantPointLayout>,
    pub grid_x: f32,
    pub grid_y: f32,
    pub grid_width: f32,
    pub grid_height: f32,
}

#[derive(Debug, Clone)]
pub struct QuadrantPointLayout {
    pub label: TextBlock,
    pub x: f32,
    pub y: f32,
    pub color: String,
}

#[derive(Debug, Clone)]
pub struct GanttLayout {
    pub title: Option<TextBlock>,
    pub sections: Vec<GanttSectionLayout>,
    pub tasks: Vec<GanttTaskLayout>,
    pub time_start: f32,
    pub time_end: f32,
    pub chart_x: f32,
    pub chart_y: f32,
    pub chart_width: f32,
    pub chart_height: f32,
}

#[derive(Debug, Clone)]
pub struct GanttSectionLayout {
    pub label: TextBlock,
    pub y: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct GanttTaskLayout {
    pub label: TextBlock,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: String,
}

#[derive(Debug, Clone)]
struct Obstacle {
    id: String,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    members: Option<HashSet<String>>,
}

fn is_horizontal(direction: Direction) -> bool {
    matches!(direction, Direction::LeftRight | Direction::RightLeft)
}

fn side_is_vertical(side: EdgeSide) -> bool {
    matches!(side, EdgeSide::Left | EdgeSide::Right)
}

fn edge_sides(
    from: &NodeLayout,
    to: &NodeLayout,
    direction: Direction,
) -> (EdgeSide, EdgeSide, bool) {
    let is_backward = if is_horizontal(direction) {
        to.x + to.width < from.x
    } else {
        to.y + to.height < from.y
    };

    if is_horizontal(direction) {
        if is_backward {
            (EdgeSide::Left, EdgeSide::Right, true)
        } else {
            (EdgeSide::Right, EdgeSide::Left, false)
        }
    } else if is_backward {
        (EdgeSide::Top, EdgeSide::Bottom, true)
    } else {
        (EdgeSide::Bottom, EdgeSide::Top, false)
    }
}

fn is_region_subgraph(sub: &crate::ir::Subgraph) -> bool {
    sub.label.trim().is_empty()
        && sub
            .id
            .as_deref()
            .map(|id| id.starts_with("__region_"))
            .unwrap_or(false)
}

pub fn compute_layout(graph: &Graph, theme: &Theme, config: &LayoutConfig) -> Layout {
    match graph.kind {
        crate::ir::DiagramKind::Sequence | crate::ir::DiagramKind::ZenUML => {
            compute_sequence_layout(graph, theme, config)
        }
        crate::ir::DiagramKind::Pie => compute_pie_layout(graph, theme, config),
        crate::ir::DiagramKind::Quadrant => compute_quadrant_layout(graph, theme, config),
        crate::ir::DiagramKind::Gantt => compute_gantt_layout(graph, theme, config),
        crate::ir::DiagramKind::Class
        | crate::ir::DiagramKind::State
        | crate::ir::DiagramKind::Er
        | crate::ir::DiagramKind::Mindmap
        | crate::ir::DiagramKind::Journey
        | crate::ir::DiagramKind::Timeline
        | crate::ir::DiagramKind::Requirement
        | crate::ir::DiagramKind::GitGraph
        | crate::ir::DiagramKind::C4
        | crate::ir::DiagramKind::Sankey
        | crate::ir::DiagramKind::Block
        | crate::ir::DiagramKind::Packet
        | crate::ir::DiagramKind::Kanban
        | crate::ir::DiagramKind::Architecture
        | crate::ir::DiagramKind::Radar
        | crate::ir::DiagramKind::Treemap
        | crate::ir::DiagramKind::XYChart
        | crate::ir::DiagramKind::Flowchart => compute_flowchart_layout(graph, theme, config),
    }
}

fn compute_pie_layout(graph: &Graph, theme: &Theme, config: &LayoutConfig) -> Layout {
    let mut slices = Vec::new();
    let mut legend = Vec::new();
    let title_block = graph
        .pie_title
        .as_ref()
        .map(|title| measure_label(title, theme, config));

    let palette = pie_palette(theme);
    let total: f32 = graph
        .pie_slices
        .iter()
        .map(|slice| slice.value.max(0.0))
        .sum();
    let fallback_total = graph.pie_slices.len().max(1) as f32;
    let total = if total > 0.0 { total } else { fallback_total };

    let mut angle = -std::f32::consts::PI / 2.0;
    for (idx, slice) in graph.pie_slices.iter().enumerate() {
        let value = slice.value.max(0.0);
        let span = if total > 0.0 {
            value / total * std::f32::consts::PI * 2.0
        } else {
            std::f32::consts::PI * 2.0 / fallback_total
        };
        let label_text = if graph.pie_show_data {
            format!("{}: {}", slice.label, format_pie_value(value))
        } else {
            slice.label.clone()
        };
        let label = measure_label(&label_text, theme, config);
        slices.push(PieSliceLayout {
            label,
            value,
            start_angle: angle,
            end_angle: angle + span,
            color: palette[idx % palette.len()].clone(),
        });
        angle += span;
    }

    let marker_size = (theme.font_size * 0.8).max(8.0);
    let legend_gap = (theme.font_size * 0.6).max(6.0);
    let mut legend_width: f32 = 0.0;
    let mut legend_height: f32 = 0.0;
    for slice in &slices {
        legend_width = legend_width.max(slice.label.width);
        legend_height += slice.label.height;
    }
    if !slices.is_empty() {
        legend_height += legend_gap * (slices.len() - 1) as f32;
    }
    legend_width += marker_size + theme.font_size * 0.6;

    let title_height = title_block
        .as_ref()
        .map(|title| title.height + theme.font_size)
        .unwrap_or(0.0);

    let mut radius = (theme.font_size * 7.0).max(90.0);
    if legend_height > 0.0 {
        radius = radius.max(legend_height / 2.0);
    }
    if radius < 20.0 {
        radius = 20.0;
    }

    let center_x = radius + theme.font_size * 2.0;
    let center_y = title_height + radius + theme.font_size * 1.5;
    let legend_x = center_x + radius + theme.font_size * 1.5;
    let mut legend_y = center_y - legend_height / 2.0;

    for slice in &slices {
        let item_y = legend_y + slice.label.height / 2.0;
        legend.push(PieLegendItem {
            x: legend_x,
            y: item_y,
            label: slice.label.clone(),
            color: slice.color.clone(),
            marker_size,
        });
        legend_y += slice.label.height + legend_gap;
    }

    let width = legend_x + legend_width + theme.font_size * 2.0;
    let height = title_height + radius * 2.0 + theme.font_size * 2.0;
    let title_layout = title_block.map(|text| PieTitleLayout {
        x: width / 2.0,
        y: theme.font_size + text.height / 2.0,
        text,
    });

    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        lifelines: Vec::new(),
        sequence_footboxes: Vec::new(),
        sequence_boxes: Vec::new(),
        sequence_frames: Vec::new(),
        sequence_notes: Vec::new(),
        sequence_activations: Vec::new(),
        sequence_numbers: Vec::new(),
        state_notes: Vec::new(),
        pie_slices: slices,
        pie_legend: legend,
        pie_center: (center_x, center_y),
        pie_radius: radius,
        pie_title: title_layout,
        quadrant: None,
        gantt: None,
        width: width.max(200.0),
        height: height.max(200.0),
    }
}

fn compute_quadrant_layout(graph: &Graph, theme: &Theme, config: &LayoutConfig) -> Layout {
    let padding = theme.font_size * 2.0;
    let grid_size = 400.0;
    let point_radius = 5.0;

    // Measure title
    let title = graph.quadrant.title.as_ref().map(|t| measure_label(t, theme, config));
    let title_height = title.as_ref().map(|t| t.height + padding).unwrap_or(0.0);

    // Measure axis labels
    let x_left = graph.quadrant.x_axis_left.as_ref().map(|t| measure_label(t, theme, config));
    let x_right = graph.quadrant.x_axis_right.as_ref().map(|t| measure_label(t, theme, config));
    let y_bottom = graph.quadrant.y_axis_bottom.as_ref().map(|t| measure_label(t, theme, config));
    let y_top = graph.quadrant.y_axis_top.as_ref().map(|t| measure_label(t, theme, config));

    // Measure quadrant labels
    let q_labels: [Option<TextBlock>; 4] = [
        graph.quadrant.quadrant_labels[0].as_ref().map(|t| measure_label(t, theme, config)),
        graph.quadrant.quadrant_labels[1].as_ref().map(|t| measure_label(t, theme, config)),
        graph.quadrant.quadrant_labels[2].as_ref().map(|t| measure_label(t, theme, config)),
        graph.quadrant.quadrant_labels[3].as_ref().map(|t| measure_label(t, theme, config)),
    ];

    let y_axis_width = y_bottom.as_ref().map(|t| t.height + padding).unwrap_or(padding);
    let x_axis_height = x_left.as_ref().map(|t| t.height + padding).unwrap_or(padding);

    let grid_x = y_axis_width + padding;
    let grid_y = title_height + padding;

    // Layout points
    let palette = quadrant_palette(theme);
    let points: Vec<QuadrantPointLayout> = graph.quadrant.points.iter().enumerate().map(|(i, p)| {
        let px = grid_x + p.x.clamp(0.0, 1.0) * grid_size;
        let py = grid_y + (1.0 - p.y.clamp(0.0, 1.0)) * grid_size; // Invert Y
        QuadrantPointLayout {
            label: measure_label(&p.label, theme, config),
            x: px,
            y: py,
            color: palette[i % palette.len()].clone(),
        }
    }).collect();

    let width = grid_x + grid_size + padding * 2.0;
    let height = grid_y + grid_size + x_axis_height + padding;

    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        lifelines: Vec::new(),
        sequence_footboxes: Vec::new(),
        sequence_boxes: Vec::new(),
        sequence_frames: Vec::new(),
        sequence_notes: Vec::new(),
        sequence_activations: Vec::new(),
        sequence_numbers: Vec::new(),
        state_notes: Vec::new(),
        pie_slices: Vec::new(),
        pie_legend: Vec::new(),
        pie_center: (0.0, 0.0),
        pie_radius: 0.0,
        pie_title: None,
        quadrant: Some(QuadrantLayout {
            title,
            title_y: title_height / 2.0,
            x_axis_left: x_left,
            x_axis_right: x_right,
            y_axis_bottom: y_bottom,
            y_axis_top: y_top,
            quadrant_labels: q_labels,
            points,
            grid_x,
            grid_y,
            grid_width: grid_size,
            grid_height: grid_size,
        }),
        gantt: None,
        width,
        height,
    }
}

fn quadrant_palette(theme: &Theme) -> Vec<String> {
    vec![
        "#6366f1".to_string(), // indigo
        "#f59e0b".to_string(), // amber
        "#10b981".to_string(), // emerald
        "#ef4444".to_string(), // red
        "#8b5cf6".to_string(), // violet
        "#06b6d4".to_string(), // cyan
    ]
}

fn compute_gantt_layout(graph: &Graph, theme: &Theme, config: &LayoutConfig) -> Layout {
    let padding = theme.font_size * 1.5;
    let row_height = theme.font_size * 2.5;
    let label_width = 150.0;
    let chart_width = 400.0;

    // Title
    let title = graph.gantt_title.as_ref().map(|t| measure_label(t, theme, config));
    let title_height = title.as_ref().map(|t| t.height + padding).unwrap_or(0.0);

    let chart_x = padding + label_width;
    let chart_y = title_height + padding;

    // Layout tasks
    let palette = gantt_palette(theme);
    let mut current_section: Option<String> = None;
    let mut sections: Vec<GanttSectionLayout> = Vec::new();
    let mut tasks: Vec<GanttTaskLayout> = Vec::new();
    let mut y = chart_y;

    for (i, task) in graph.gantt_tasks.iter().enumerate() {
        // Check for section change
        if task.section != current_section {
            if let Some(ref sec) = task.section {
                sections.push(GanttSectionLayout {
                    label: measure_label(sec, theme, config),
                    y,
                    height: row_height,
                });
                y += row_height;
            }
            current_section = task.section.clone();
        }

        tasks.push(GanttTaskLayout {
            label: measure_label(&task.label, theme, config),
            x: chart_x,
            y,
            width: chart_width * 0.6, // placeholder width
            height: row_height * 0.8,
            color: palette[i % palette.len()].clone(),
        });
        y += row_height;
    }

    let height = y + padding;
    let width = chart_x + chart_width + padding;

    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        lifelines: Vec::new(),
        sequence_footboxes: Vec::new(),
        sequence_boxes: Vec::new(),
        sequence_frames: Vec::new(),
        sequence_notes: Vec::new(),
        sequence_activations: Vec::new(),
        sequence_numbers: Vec::new(),
        state_notes: Vec::new(),
        pie_slices: Vec::new(),
        pie_legend: Vec::new(),
        pie_center: (0.0, 0.0),
        pie_radius: 0.0,
        pie_title: None,
        quadrant: None,
        gantt: Some(GanttLayout {
            title,
            sections,
            tasks,
            time_start: 0.0,
            time_end: 100.0,
            chart_x,
            chart_y,
            chart_width,
            chart_height: y - chart_y,
        }),
        width,
        height,
    }
}

fn gantt_palette(theme: &Theme) -> Vec<String> {
    vec![
        theme.primary_color.clone(),
        "#60a5fa".to_string(), // blue-400
        "#34d399".to_string(), // emerald-400
        "#a78bfa".to_string(), // violet-400
        "#fb923c".to_string(), // orange-400
    ]
}

fn pie_palette(theme: &Theme) -> Vec<String> {
    vec![
        theme.primary_color.clone(),
        theme.secondary_color.clone(),
        theme.tertiary_color.clone(),
        "#A3D5FF".to_string(),
        "#FFB6B6".to_string(),
        "#B9FBC0".to_string(),
        "#FFD6A5".to_string(),
        "#CABFFD".to_string(),
    ]
}

fn format_pie_value(value: f32) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    if (rounded - rounded.round()).abs() < 0.001 {
        format!("{:.0}", rounded)
    } else {
        format!("{:.2}", rounded)
    }
}

fn compute_flowchart_layout(graph: &Graph, theme: &Theme, config: &LayoutConfig) -> Layout {
    let config = config;
    let mut nodes = BTreeMap::new();

    for node in graph.nodes.values() {
        let label = measure_label(&node.label, theme, config);
        let label_empty = label.lines.len() == 1 && label.lines[0].trim().is_empty();
        let (mut width, mut height) = shape_size(node.shape, &label, config);
        if graph.kind == crate::ir::DiagramKind::State
            && label_empty
            && matches!(
                node.shape,
                crate::ir::NodeShape::Circle | crate::ir::NodeShape::DoubleCircle
            )
        {
            let size = (config.node_padding_y * 1.4).max(12.0);
            width = size;
            height = size;
        }
        let style = resolve_node_style(node.id.as_str(), graph);
        nodes.insert(
            node.id.clone(),
            NodeLayout {
                id: node.id.clone(),
                x: 0.0,
                y: 0.0,
                width,
                height,
                label,
                shape: node.shape,
                style,
                link: graph.node_links.get(&node.id).cloned(),
                anchor_subgraph: None,
                hidden: false,
            },
        );
    }

    let anchor_ids = mark_subgraph_anchor_nodes_hidden(graph, &mut nodes);
    let mut anchor_info = apply_subgraph_anchor_sizes(graph, &mut nodes, theme, config);
    let mut anchored_subgraph_nodes: HashSet<String> = HashSet::new();
    for info in anchor_info.values() {
        if let Some(sub) = graph.subgraphs.get(info.sub_idx) {
            anchored_subgraph_nodes.extend(sub.nodes.iter().cloned());
        }
    }

    let anchored_indices: HashSet<usize> = anchor_info.values().map(|info| info.sub_idx).collect();
    let mut edge_redirects: HashMap<String, String> = HashMap::new();
    if !graph.subgraphs.is_empty() {
        for (idx, sub) in graph.subgraphs.iter().enumerate() {
            let Some(anchor_id) = subgraph_anchor_id(sub, &nodes) else {
                continue;
            };
            if anchored_indices.contains(&idx) {
                continue;
            }
            if let Some(anchor_child) = pick_subgraph_anchor_child(sub, graph, &anchor_ids)
                && anchor_child != anchor_id
            {
                edge_redirects.insert(anchor_id.to_string(), anchor_child);
            }
        }
    }

    let mut layout_edges: Vec<crate::ir::Edge> = Vec::with_capacity(graph.edges.len());
    for edge in &graph.edges {
        let mut layout_edge = edge.clone();
        if let Some(new_from) = edge_redirects.get(&layout_edge.from) {
            layout_edge.from = new_from.clone();
        }
        if let Some(new_to) = edge_redirects.get(&layout_edge.to) {
            layout_edge.to = new_to.clone();
        }
        layout_edges.push(layout_edge);
    }

    let mut layout_node_ids: Vec<String> = graph.nodes.keys().cloned().collect();
    layout_node_ids.sort_by_key(|id| graph.node_order.get(id).copied().unwrap_or(usize::MAX));
    if !anchored_subgraph_nodes.is_empty() {
        layout_node_ids.retain(|id| !anchored_subgraph_nodes.contains(id));
    }
    let mut layout_set: HashSet<String> = layout_node_ids.iter().cloned().collect();

    let used_dagre = assign_positions_dagre(
        graph,
        &layout_node_ids,
        &layout_set,
        &mut nodes,
        config,
        &layout_edges,
    );
    if !used_dagre {
        if anchor_info.is_empty() {
            anchor_info = apply_subgraph_anchor_sizes(graph, &mut nodes, theme, config);
            anchored_subgraph_nodes.clear();
            for info in anchor_info.values() {
                if let Some(sub) = graph.subgraphs.get(info.sub_idx) {
                    anchored_subgraph_nodes.extend(sub.nodes.iter().cloned());
                }
            }
            if !anchored_subgraph_nodes.is_empty() {
                layout_node_ids.retain(|id| !anchored_subgraph_nodes.contains(id));
            }
            layout_set = layout_node_ids.iter().cloned().collect();
        }
        assign_positions_manual(
            graph,
            &layout_node_ids,
            &layout_set,
            &mut nodes,
            config,
            &layout_edges,
        );
    }

    let mut anchored_nodes: HashSet<String> = anchored_subgraph_nodes;
    if !graph.subgraphs.is_empty() {
        if graph.kind != crate::ir::DiagramKind::State {
            apply_subgraph_direction_overrides(graph, &mut nodes, config, &anchored_indices);
        }
        if !anchor_info.is_empty() {
            anchored_nodes =
                align_subgraphs_to_anchor_nodes(graph, &anchor_info, &mut nodes, config);
        }
        if graph.kind == crate::ir::DiagramKind::State && !anchor_info.is_empty() {
            apply_state_subgraph_layouts(graph, &mut nodes, config, &anchored_indices);
        }
        if !used_dagre {
            apply_orthogonal_region_bands(graph, &mut nodes, config);
            if graph.kind != crate::ir::DiagramKind::State {
                apply_subgraph_bands(graph, &mut nodes, &anchored_nodes, config);
            }
        }
    }

    // Separate overlapping sibling subgraphs
    separate_sibling_subgraphs(graph, &mut nodes, config);

    let mut subgraphs = build_subgraph_layouts(graph, &nodes, theme, config);
    apply_subgraph_anchors(graph, &subgraphs, &mut nodes);
    let obstacles = build_obstacles(&nodes, &subgraphs);
    let mut edge_ports: Vec<EdgePortInfo> = Vec::with_capacity(graph.edges.len());
    let mut port_candidates: HashMap<(String, EdgeSide), Vec<PortCandidate>> = HashMap::new();
    for (idx, edge) in graph.edges.iter().enumerate() {
        let from_layout = nodes.get(&edge.from).expect("from node missing");
        let to_layout = nodes.get(&edge.to).expect("to node missing");
        let temp_from = from_layout.anchor_subgraph.and_then(|anchor_idx| {
            subgraphs
                .get(anchor_idx)
                .map(|sub| anchor_layout_for_edge(from_layout, sub, graph.direction, true))
        });
        let temp_to = to_layout.anchor_subgraph.and_then(|anchor_idx| {
            subgraphs
                .get(anchor_idx)
                .map(|sub| anchor_layout_for_edge(to_layout, sub, graph.direction, false))
        });
        let from = temp_from.as_ref().unwrap_or(from_layout);
        let to = temp_to.as_ref().unwrap_or(to_layout);
        let (start_side, end_side, _is_backward) = edge_sides(from, to, graph.direction);
        edge_ports.push(EdgePortInfo {
            start_side,
            end_side,
            start_offset: 0.0,
            end_offset: 0.0,
        });

        let from_center = (from.x + from.width / 2.0, from.y + from.height / 2.0);
        let to_center = (to.x + to.width / 2.0, to.y + to.height / 2.0);
        let start_other = if side_is_vertical(start_side) {
            to_center.1
        } else {
            to_center.0
        };
        let end_other = if side_is_vertical(end_side) {
            from_center.1
        } else {
            from_center.0
        };
        port_candidates
            .entry((edge.from.clone(), start_side))
            .or_default()
            .push(PortCandidate {
                edge_idx: idx,
                is_start: true,
                other_pos: start_other,
            });
        port_candidates
            .entry((edge.to.clone(), end_side))
            .or_default()
            .push(PortCandidate {
                edge_idx: idx,
                is_start: false,
                other_pos: end_other,
            });
    }
    for ((node_id, side), mut candidates) in port_candidates {
        let Some(node) = nodes.get(&node_id) else {
            continue;
        };
        candidates.sort_by(|a, b| {
            a.other_pos
                .partial_cmp(&b.other_pos)
                .unwrap_or(Ordering::Equal)
        });
        let node_len = if side_is_vertical(side) {
            node.height
        } else {
            node.width
        };
        let pad = (node_len * 0.2).min(12.0).max(4.0);
        let usable = (node_len - 2.0 * pad).max(1.0);
        let step = usable / (candidates.len() as f32 + 1.0);
        for (i, candidate) in candidates.iter().enumerate() {
            let pos = pad + step * (i as f32 + 1.0);
            let offset = pos - node_len / 2.0;
            if let Some(info) = edge_ports.get_mut(candidate.edge_idx) {
                if candidate.is_start {
                    info.start_offset = offset;
                } else {
                    info.end_offset = offset;
                }
            }
        }
    }
    let pair_counts = build_edge_pair_counts(&graph.edges);
    let mut pair_seen: HashMap<(String, String), usize> = HashMap::new();
    let mut edges = Vec::new();
    for (idx, edge) in graph.edges.iter().enumerate() {
        let key = edge_pair_key(edge);
        let total = *pair_counts.get(&key).unwrap_or(&1) as f32;
        let seen = pair_seen.entry(key).or_insert(0usize);
        let idx_in_pair = *seen as f32;
        *seen += 1;
        let base_offset = if total > 1.0 {
            (idx_in_pair - (total - 1.0) / 2.0) * (config.node_spacing * 0.35)
        } else {
            0.0
        };
        let from_layout = nodes.get(&edge.from).expect("from node missing");
        let to_layout = nodes.get(&edge.to).expect("to node missing");
        let temp_from = from_layout.anchor_subgraph.and_then(|idx| {
            subgraphs
                .get(idx)
                .map(|sub| anchor_layout_for_edge(from_layout, sub, graph.direction, true))
        });
        let temp_to = to_layout.anchor_subgraph.and_then(|idx| {
            subgraphs
                .get(idx)
                .map(|sub| anchor_layout_for_edge(to_layout, sub, graph.direction, false))
        });
        let from = temp_from.as_ref().unwrap_or(from_layout);
        let to = temp_to.as_ref().unwrap_or(to_layout);
        let label = edge.label.as_ref().map(|l| measure_label(l, theme, config));
        let start_label = edge
            .start_label
            .as_ref()
            .map(|l| measure_label(l, theme, config));
        let end_label = edge
            .end_label
            .as_ref()
            .map(|l| measure_label(l, theme, config));
        let override_style = resolve_edge_style(idx, graph);

        let port_info = edge_ports
            .get(idx)
            .copied()
            .expect("edge port info missing");
        let route_ctx = RouteContext {
            from_id: &edge.from,
            to_id: &edge.to,
            from,
            to,
            direction: graph.direction,
            config,
            obstacles: &obstacles,
            base_offset,
            start_side: port_info.start_side,
            end_side: port_info.end_side,
            start_offset: port_info.start_offset,
            end_offset: port_info.end_offset,
        };
        let points = route_edge_with_avoidance(&route_ctx);
        edges.push(EdgeLayout {
            from: edge.from.clone(),
            to: edge.to.clone(),
            label,
            start_label,
            end_label,
            points,
            directed: edge.directed,
            arrow_start: edge.arrow_start,
            arrow_end: edge.arrow_end,
            arrow_start_kind: edge.arrow_start_kind,
            arrow_end_kind: edge.arrow_end_kind,
            start_decoration: edge.start_decoration,
            end_decoration: edge.end_decoration,
            style: edge.style,
            override_style,
        });
    }

    if !used_dagre && matches!(graph.direction, Direction::RightLeft | Direction::BottomTop) {
        apply_direction_mirror(graph.direction, &mut nodes, &mut edges, &mut subgraphs);
    }

    normalize_layout(&mut nodes, &mut edges, &mut subgraphs);
    let mut state_notes = Vec::new();
    if graph.kind == crate::ir::DiagramKind::State && !graph.state_notes.is_empty() {
        let note_pad_x = theme.font_size * 0.75;
        let note_pad_y = theme.font_size * 0.5;
        let note_gap = (theme.font_size * 0.9).max(10.0);
        for note in &graph.state_notes {
            let Some(target) = nodes.get(&note.target) else {
                continue;
            };
            let label = measure_label(&note.label, theme, config);
            let width = label.width + note_pad_x * 2.0;
            let height = label.height + note_pad_y * 2.0;
            let y = target.y + target.height / 2.0 - height / 2.0;
            let x = match note.position {
                crate::ir::StateNotePosition::LeftOf => target.x - note_gap - width,
                crate::ir::StateNotePosition::RightOf => target.x + target.width + note_gap,
            };
            state_notes.push(StateNoteLayout {
                x,
                y,
                width,
                height,
                label,
                position: note.position,
                target: note.target.clone(),
            });
        }
    }
    let (mut max_x, mut max_y) = bounds_without_padding(&nodes, &subgraphs);
    for note in &state_notes {
        max_x = max_x.max(note.x + note.width);
        max_y = max_y.max(note.y + note.height);
    }
    let width = max_x + 60.0;
    let height = max_y + 60.0;

    Layout {
        kind: graph.kind,
        nodes,
        edges,
        subgraphs,
        lifelines: Vec::new(),
        sequence_footboxes: Vec::new(),
        sequence_boxes: Vec::new(),
        sequence_frames: Vec::new(),
        sequence_notes: Vec::new(),
        sequence_activations: Vec::new(),
        sequence_numbers: Vec::new(),
        state_notes,
        pie_slices: Vec::new(),
        pie_legend: Vec::new(),
        pie_center: (0.0, 0.0),
        pie_radius: 0.0,
        pie_title: None,
        quadrant: None,
        gantt: None,
        width,
        height,
    }
}

fn assign_positions_dagre(
    graph: &Graph,
    layout_node_ids: &[String],
    layout_set: &HashSet<String>,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
    layout_edges: &[crate::ir::Edge],
) -> bool {
    if layout_node_ids.is_empty() {
        return false;
    }

    // Enable compound mode for diagrams with subgraphs so dagre treats them as clusters
    let compound_enabled = !graph.subgraphs.is_empty();
    let mut dagre_graph: DagreGraph<DagreConfig, DagreNode, DagreEdge> =
        DagreGraph::new(Some(GraphOption {
            directed: Some(true),
            multigraph: Some(false),
            compound: Some(compound_enabled),
        }));

    let mut graph_config = DagreConfig::default();
    graph_config.rankdir = Some(dagre_rankdir(graph.direction).to_string());
    graph_config.nodesep = Some(config.node_spacing);
    graph_config.ranksep = Some(config.rank_spacing);
    graph_config.marginx = Some(8.0);
    graph_config.marginy = Some(8.0);
    dagre_graph.set_graph(graph_config);

    for node_id in layout_node_ids {
        let Some(layout) = nodes.get(node_id) else {
            continue;
        };
        let mut node = DagreNode::default();
        node.width = layout.width;
        node.height = layout.height;
        if let Some(order) = graph.node_order.get(node_id) {
            node.order = Some(*order);
        }
        dagre_graph.set_node(node_id.clone(), Some(node));
    }

    let mut anchor_ids: HashMap<usize, String> = HashMap::new();
    if !graph.subgraphs.is_empty() {
        for (idx, sub) in graph.subgraphs.iter().enumerate() {
            let Some(anchor_id) = subgraph_anchor_id(sub, nodes) else {
                continue;
            };
            anchor_ids.insert(idx, anchor_id.to_string());
        }
    }

    if compound_enabled && !anchor_ids.is_empty() {
        let mut node_parent: HashMap<String, usize> = HashMap::new();
        for (idx, sub) in graph.subgraphs.iter().enumerate() {
            let Some(anchor_id) = anchor_ids.get(&idx) else {
                continue;
            };
            let sub_size = sub.nodes.len();
            for node_id in &sub.nodes {
                if !layout_set.contains(node_id) {
                    continue;
                }
                if node_id == anchor_id {
                    continue;
                }
                let entry = node_parent.entry(node_id.clone()).or_insert(idx);
                let current_size = graph
                    .subgraphs
                    .get(*entry)
                    .map(|s| s.nodes.len())
                    .unwrap_or(usize::MAX);
                if sub_size < current_size {
                    *entry = idx;
                }
            }
        }

        let mut subgraph_sets: Vec<HashSet<String>> = Vec::with_capacity(graph.subgraphs.len());
        for sub in &graph.subgraphs {
            subgraph_sets.push(sub.nodes.iter().cloned().collect());
        }

        for (child_idx, child_anchor) in &anchor_ids {
            let mut best_parent: Option<usize> = None;
            let mut best_size = usize::MAX;
            for (parent_idx, parent_anchor) in &anchor_ids {
                if child_idx == parent_idx || child_anchor == parent_anchor {
                    continue;
                }
                let parent_set = &subgraph_sets[*parent_idx];
                let child_set = &subgraph_sets[*child_idx];
                if child_set.is_subset(parent_set) {
                    let parent_size = parent_set.len();
                    if parent_size < best_size {
                        best_size = parent_size;
                        best_parent = Some(*parent_idx);
                    }
                }
            }
            if let Some(parent_idx) = best_parent
                && let Some(parent_anchor) = anchor_ids.get(&parent_idx)
            {
                let _ = dagre_graph.set_parent(child_anchor, Some(parent_anchor.clone()));
            }
        }

        for (node_id, parent_idx) in node_parent {
            if let Some(parent_anchor) = anchor_ids.get(&parent_idx) {
                let _ = dagre_graph.set_parent(&node_id, Some(parent_anchor.clone()));
            }
        }

        // Add invisible edges between top-level sibling subgraphs to prevent overlap
        // Find which anchors have no parent (top-level subgraphs)
        let mut top_level_anchors: Vec<String> = Vec::new();
        for (idx, anchor) in &anchor_ids {
            let sub = &graph.subgraphs[*idx];
            let is_nested = graph
                .subgraphs
                .iter()
                .enumerate()
                .any(|(other_idx, other)| {
                    other_idx != *idx
                        && sub.nodes.iter().all(|n| other.nodes.contains(n))
                        && other.nodes.len() > sub.nodes.len()
                });
            if !is_nested {
                top_level_anchors.push(anchor.clone());
            }
        }
        // Chain top-level anchors with invisible edges to force horizontal/vertical separation
        for i in 0..top_level_anchors.len().saturating_sub(1) {
            let from = &top_level_anchors[i];
            let to = &top_level_anchors[i + 1];
            let mut edge_label = DagreEdge::default();
            edge_label.minlen = Some(1.0);
            let _ = dagre_graph.set_edge(from, to, Some(edge_label), None);
        }
    }

    let mut edge_set: HashSet<(String, String)> = HashSet::new();
    for edge in layout_edges.iter() {
        if !layout_set.contains(&edge.from) || !layout_set.contains(&edge.to) {
            continue;
        }
        let from = edge.from.clone();
        let to = edge.to.clone();
        if !edge_set.insert((from.clone(), to.clone())) {
            continue;
        }
        let edge_label = DagreEdge::default();
        let _ = dagre_graph.set_edge(&from, &to, Some(edge_label), None);
    }

    dagre_layout::run_layout(&mut dagre_graph);

    let mut applied = false;
    for node_id in layout_node_ids {
        let Some(dagre_node) = dagre_graph.node(node_id) else {
            continue;
        };
        if let Some(node) = nodes.get_mut(node_id) {
            node.x = dagre_node.x - node.width / 2.0;
            node.y = dagre_node.y - node.height / 2.0;
            applied = true;
        }
    }

    applied
}

fn assign_positions_dagre_subset(
    node_ids: &[String],
    edges: &[crate::ir::Edge],
    nodes: &mut BTreeMap<String, NodeLayout>,
    direction: Direction,
    config: &LayoutConfig,
    node_order: Option<&HashMap<String, usize>>,
) -> bool {
    if node_ids.is_empty() {
        return false;
    }

    let mut dagre_graph: DagreGraph<DagreConfig, DagreNode, DagreEdge> =
        DagreGraph::new(Some(GraphOption {
            directed: Some(true),
            multigraph: Some(false),
            compound: Some(false),
        }));

    let mut graph_config = DagreConfig::default();
    graph_config.rankdir = Some(dagre_rankdir(direction).to_string());
    graph_config.nodesep = Some(config.node_spacing);
    graph_config.ranksep = Some(config.rank_spacing);
    graph_config.marginx = Some(8.0);
    graph_config.marginy = Some(8.0);
    dagre_graph.set_graph(graph_config);

    for node_id in node_ids {
        let Some(layout) = nodes.get(node_id) else {
            continue;
        };
        let mut node = DagreNode::default();
        node.width = layout.width;
        node.height = layout.height;
        if let Some(order_map) = node_order
            && let Some(order) = order_map.get(node_id)
        {
            node.order = Some(*order);
        }
        dagre_graph.set_node(node_id.clone(), Some(node));
    }

    let node_set: HashSet<String> = node_ids.iter().cloned().collect();
    let mut edge_set: HashSet<(String, String)> = HashSet::new();
    for edge in edges {
        if !node_set.contains(&edge.from) || !node_set.contains(&edge.to) {
            continue;
        }
        let from = edge.from.clone();
        let to = edge.to.clone();
        if !edge_set.insert((from.clone(), to.clone())) {
            continue;
        }
        let edge_label = DagreEdge::default();
        let _ = dagre_graph.set_edge(&from, &to, Some(edge_label), None);
    }

    dagre_layout::run_layout(&mut dagre_graph);

    let mut applied = false;
    for node_id in node_ids {
        let Some(dagre_node) = dagre_graph.node(node_id) else {
            continue;
        };
        if let Some(node) = nodes.get_mut(node_id) {
            node.x = dagre_node.x - node.width / 2.0;
            node.y = dagre_node.y - node.height / 2.0;
            applied = true;
        }
    }

    applied
}

fn assign_positions_manual(
    graph: &Graph,
    layout_node_ids: &[String],
    layout_set: &HashSet<String>,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
    layout_edges: &[crate::ir::Edge],
) {
    let ranks = compute_ranks_subset(layout_node_ids, layout_edges);
    let mut max_rank = 0usize;
    for rank in ranks.values() {
        max_rank = max_rank.max(*rank);
    }

    let layout_edges: Vec<crate::ir::Edge> = layout_edges
        .iter()
        .filter(|edge| layout_set.contains(&edge.from) && layout_set.contains(&edge.to))
        .cloned()
        .collect();
    let mut rank_nodes: Vec<Vec<String>> = vec![Vec::new(); max_rank + 1];
    for node_id in layout_node_ids {
        let rank = *ranks.get(node_id).unwrap_or(&0);
        if let Some(bucket) = rank_nodes.get_mut(rank) {
            bucket.push(node_id.clone());
        }
    }

    let mut expanded_edges: Vec<crate::ir::Edge> = Vec::new();
    let mut order_map = graph.node_order.clone();
    let mut dummy_counter = 0usize;

    for edge in &layout_edges {
        let Some(&from_rank) = ranks.get(&edge.from) else {
            continue;
        };
        let Some(&to_rank) = ranks.get(&edge.to) else {
            continue;
        };
        if to_rank <= from_rank {
            continue;
        }
        let span = to_rank - from_rank;
        if span <= 1 {
            expanded_edges.push(edge.clone());
            continue;
        }
        let mut prev = edge.from.clone();
        for step in 1..span {
            let dummy_id = format!("__dummy_{}__", dummy_counter);
            dummy_counter += 1;
            let order_idx = order_map.len();
            order_map.insert(dummy_id.clone(), order_idx);
            if let Some(bucket) = rank_nodes.get_mut(from_rank + step) {
                bucket.push(dummy_id.clone());
            }
            expanded_edges.push(crate::ir::Edge {
                from: prev.clone(),
                to: dummy_id.clone(),
                label: None,
                start_label: None,
                end_label: None,
                directed: true,
                arrow_start: false,
                arrow_end: false,
                arrow_start_kind: None,
                arrow_end_kind: None,
                start_decoration: None,
                end_decoration: None,
                style: crate::ir::EdgeStyle::Solid,
            });
            prev = dummy_id;
        }
        expanded_edges.push(crate::ir::Edge {
            from: prev,
            to: edge.to.clone(),
            label: None,
            start_label: None,
            end_label: None,
            directed: true,
            arrow_start: false,
            arrow_end: false,
            arrow_start_kind: None,
            arrow_end_kind: None,
            start_decoration: None,
            end_decoration: None,
            style: crate::ir::EdgeStyle::Solid,
        });
    }

    for bucket in &mut rank_nodes {
        bucket.sort_by_key(|id| order_map.get(id).copied().unwrap_or(usize::MAX));
    }
    order_rank_nodes(&mut rank_nodes, &expanded_edges, &order_map);

    let mut main_cursor = 0.0;
    for (rank_idx, bucket) in rank_nodes.iter().enumerate() {
        let mut max_main: f32 = 0.0;
        for node_id in bucket {
            if let Some(node_layout) = nodes.get_mut(node_id) {
                if is_horizontal(graph.direction) {
                    node_layout.x = main_cursor;
                    max_main = max_main.max(node_layout.width);
                } else {
                    node_layout.y = main_cursor;
                    max_main = max_main.max(node_layout.height);
                }
            }
        }
        main_cursor += max_main + config.rank_spacing;
        if rank_idx == max_rank {
            // Ensure no trailing spacing
        }
    }

    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    for edge in &layout_edges {
        incoming
            .entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());
        outgoing
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
    }

    let mut cross_pos: HashMap<String, f32> = HashMap::new();
    let mut place_rank = |rank_idx: usize,
                          use_incoming: bool,
                          nodes: &mut BTreeMap<String, NodeLayout>| {
        let bucket = &rank_nodes[rank_idx];
        if bucket.is_empty() {
            return;
        }
        let neighbors = if use_incoming { &incoming } else { &outgoing };
        let mut entries: Vec<(String, f32, f32)> = Vec::new();
        for node_id in bucket {
            let Some(node) = nodes.get(node_id) else {
                continue;
            };
            let mut sum = 0.0;
            let mut count = 0.0;
            if let Some(list) = neighbors.get(node_id) {
                for neighbor_id in list {
                    if let Some(center) = cross_pos.get(neighbor_id) {
                        sum += *center;
                        count += 1.0;
                    }
                }
            }
            let desired = if count > 0.0 { sum / count } else { 0.0 };
            let half = if is_horizontal(graph.direction) {
                node.height / 2.0
            } else {
                node.width / 2.0
            };
            entries.push((node_id.clone(), desired, half));
        }
        entries.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let desired_mean = entries.iter().map(|(_, d, _)| *d).sum::<f32>() / entries.len() as f32;
        let mut assigned: Vec<(String, f32, f32)> = Vec::new();
        let mut prev_center: Option<f32> = None;
        let mut prev_half = 0.0;
        for (node_id, desired, half) in entries {
            let center = if let Some(prev) = prev_center {
                let min_center = prev + prev_half + half + config.node_spacing;
                if desired < min_center {
                    min_center
                } else {
                    desired
                }
            } else {
                desired
            };
            assigned.push((node_id, center, half));
            prev_center = Some(center);
            prev_half = half;
        }
        let actual_mean = assigned.iter().map(|(_, c, _)| *c).sum::<f32>() / assigned.len() as f32;
        let delta = desired_mean - actual_mean;
        for (node_id, center, _half) in assigned {
            let center = center + delta;
            if let Some(node) = nodes.get_mut(&node_id) {
                if is_horizontal(graph.direction) {
                    node.y = center - node.height / 2.0;
                } else {
                    node.x = center - node.width / 2.0;
                }
            }
            cross_pos.insert(node_id, center);
        }
    };

    for _ in 0..2 {
        for rank_idx in 0..rank_nodes.len() {
            place_rank(rank_idx, true, nodes);
        }
        for rank_idx in (0..rank_nodes.len()).rev() {
            place_rank(rank_idx, false, nodes);
        }
    }
}

fn dagre_rankdir(direction: Direction) -> &'static str {
    match direction {
        Direction::TopDown => "tb",
        Direction::BottomTop => "bt",
        Direction::LeftRight => "lr",
        Direction::RightLeft => "rl",
    }
}

fn compute_sequence_layout(graph: &Graph, theme: &Theme, config: &LayoutConfig) -> Layout {
    let mut nodes = BTreeMap::new();
    let mut edges = Vec::new();
    let subgraphs = Vec::new();

    let mut participants = graph.sequence_participants.clone();
    for id in graph.nodes.keys() {
        if !participants.contains(id) {
            participants.push(id.clone());
        }
    }

    let mut label_blocks: HashMap<String, TextBlock> = HashMap::new();
    let mut max_label_width: f32 = 0.0;
    let mut max_label_height: f32 = 0.0;
    for id in &participants {
        let node = graph.nodes.get(id).expect("participant missing");
        let label = measure_label(&node.label, theme, config);
        max_label_width = max_label_width.max(label.width);
        max_label_height = max_label_height.max(label.height);
        label_blocks.insert(id.clone(), label);
    }

    let actor_width = (max_label_width + theme.font_size * 2.5).max(150.0);
    let actor_height = (max_label_height + theme.font_size * 2.5).max(65.0);
    let actor_gap = (theme.font_size * 3.125).max(40.0);

    let mut cursor_x = 0.0;
    for id in &participants {
        let node = graph.nodes.get(id).expect("participant missing");
        let label = label_blocks.get(id).cloned().unwrap_or_else(|| TextBlock {
            lines: vec![id.clone()],
            width: 0.0,
            height: 0.0,
        });
        nodes.insert(
            id.clone(),
            NodeLayout {
                id: id.clone(),
                x: cursor_x,
                y: 0.0,
                width: actor_width,
                height: actor_height,
                label,
                shape: node.shape,
                style: resolve_node_style(id.as_str(), graph),
                link: graph.node_links.get(id).cloned(),
                anchor_subgraph: None,
                hidden: false,
            },
        );
        cursor_x += actor_width + actor_gap;
    }

    let base_spacing = (theme.font_size * 2.8).max(24.0);
    let note_gap_y = (theme.font_size * 0.7).max(8.0);
    let note_gap_x = (theme.font_size * 0.8).max(10.0);
    let note_padding_x = (theme.font_size * 0.9).max(10.0);
    let note_padding_y = (theme.font_size * 0.6).max(6.0);
    let mut extra_before = vec![0.0; graph.edges.len()];
    let frame_end_pad = base_spacing * 0.25;
    for frame in &graph.sequence_frames {
        if frame.start_idx < extra_before.len() {
            extra_before[frame.start_idx] += base_spacing;
        }
        for section in frame.sections.iter().skip(1) {
            if section.start_idx < extra_before.len() {
                extra_before[section.start_idx] += base_spacing;
            }
        }
        if frame.end_idx < extra_before.len() {
            extra_before[frame.end_idx] += frame_end_pad;
        }
    }

    let mut notes_by_index = vec![Vec::new(); graph.edges.len().saturating_add(1)];
    for note in &graph.sequence_notes {
        let idx = note.index.min(graph.edges.len());
        notes_by_index[idx].push(note);
    }

    let mut message_cursor = actor_height + theme.font_size * 2.9;
    let mut message_ys = Vec::new();
    let mut sequence_notes = Vec::new();
    for idx in 0..=graph.edges.len() {
        if let Some(bucket) = notes_by_index.get(idx) {
            for note in bucket {
                message_cursor += note_gap_y;
                let label = measure_label(&note.label, theme, config);
                let mut width = label.width + note_padding_x * 2.0;
                let height = label.height + note_padding_y * 2.0;
                let mut lifeline_xs = note
                    .participants
                    .iter()
                    .filter_map(|id| nodes.get(id))
                    .map(|node| node.x + node.width / 2.0)
                    .collect::<Vec<_>>();
                if lifeline_xs.is_empty() {
                    lifeline_xs.push(0.0);
                }
                let base_x = lifeline_xs[0];
                let min_x = lifeline_xs.iter().copied().fold(f32::INFINITY, f32::min);
                let max_x = lifeline_xs
                    .iter()
                    .copied()
                    .fold(f32::NEG_INFINITY, f32::max);
                if note.position == crate::ir::SequenceNotePosition::Over
                    && note.participants.len() > 1
                {
                    let span = (max_x - min_x).abs();
                    width = width.max(span + note_gap_x * 2.0);
                }
                let x = match note.position {
                    crate::ir::SequenceNotePosition::LeftOf => base_x - note_gap_x - width,
                    crate::ir::SequenceNotePosition::RightOf => base_x + note_gap_x,
                    crate::ir::SequenceNotePosition::Over => (min_x + max_x) / 2.0 - width / 2.0,
                };
                let y = message_cursor;
                sequence_notes.push(SequenceNoteLayout {
                    x,
                    y,
                    width,
                    height,
                    label,
                    position: note.position,
                    participants: note.participants.clone(),
                    index: note.index,
                });
                message_cursor += height + note_gap_y;
            }
        }
        if idx < graph.edges.len() {
            message_cursor += extra_before[idx];
            message_ys.push(message_cursor);
            message_cursor += base_spacing;
        }
    }

    for (idx, edge) in graph.edges.iter().enumerate() {
        let from = nodes.get(&edge.from).expect("from node missing");
        let to = nodes.get(&edge.to).expect("to node missing");
        let y = message_ys.get(idx).copied().unwrap_or(message_cursor);
        let label = edge.label.as_ref().map(|l| measure_label(l, theme, config));
        let start_label = edge
            .start_label
            .as_ref()
            .map(|l| measure_label(l, theme, config));
        let end_label = edge
            .end_label
            .as_ref()
            .map(|l| measure_label(l, theme, config));

        let points = if edge.from == edge.to {
            let pad = config.node_spacing.max(20.0) * 0.6;
            let x = from.x + from.width / 2.0;
            vec![(x, y), (x + pad, y), (x + pad, y + pad), (x, y + pad)]
        } else {
            let from_x = from.x + from.width / 2.0;
            let to_x = to.x + to.width / 2.0;
            vec![(from_x, y), (to_x, y)]
        };

        let mut override_style = resolve_edge_style(idx, graph);
        if edge.style == crate::ir::EdgeStyle::Dotted && override_style.dasharray.is_none() {
            override_style.dasharray = Some("3 3".to_string());
        }
        edges.push(EdgeLayout {
            from: edge.from.clone(),
            to: edge.to.clone(),
            label,
            start_label,
            end_label,
            points,
            directed: edge.directed,
            arrow_start: edge.arrow_start,
            arrow_end: edge.arrow_end,
            arrow_start_kind: edge.arrow_start_kind,
            arrow_end_kind: edge.arrow_end_kind,
            start_decoration: edge.start_decoration,
            end_decoration: edge.end_decoration,
            style: edge.style,
            override_style,
        });
    }

    let mut sequence_frames = Vec::new();
    if !graph.sequence_frames.is_empty() && !message_ys.is_empty() {
        let mut frames = graph.sequence_frames.clone();
        frames.sort_by(|a, b| {
            a.start_idx
                .cmp(&b.start_idx)
                .then_with(|| b.end_idx.cmp(&a.end_idx))
        });
        for frame in frames {
            if frame.start_idx >= frame.end_idx || frame.start_idx >= message_ys.len() {
                continue;
            }
            let mut xs = Vec::new();
            for edge in graph
                .edges
                .iter()
                .skip(frame.start_idx)
                .take(frame.end_idx.saturating_sub(frame.start_idx))
            {
                if let Some(node) = nodes.get(&edge.from) {
                    xs.push(node.x + node.width / 2.0);
                }
                if let Some(node) = nodes.get(&edge.to) {
                    xs.push(node.x + node.width / 2.0);
                }
            }
            if xs.is_empty() {
                for node in nodes.values() {
                    xs.push(node.x + node.width / 2.0);
                }
            }
            let (min_x, max_x) = xs
                .iter()
                .fold((f32::INFINITY, f32::NEG_INFINITY), |acc, x| {
                    (acc.0.min(*x), acc.1.max(*x))
                });
            if !min_x.is_finite() || !max_x.is_finite() {
                continue;
            }
            let frame_pad_x = theme.font_size * 0.7;
            let frame_x = min_x - frame_pad_x;
            let frame_width = (max_x - min_x) + frame_pad_x * 2.0;

            let first_y = message_ys
                .get(frame.start_idx)
                .copied()
                .unwrap_or(message_cursor);
            let last_y = message_ys
                .get(frame.end_idx.saturating_sub(1))
                .copied()
                .unwrap_or(first_y);
            let mut min_y = first_y;
            let mut max_y = last_y;
            for note in &sequence_notes {
                if note.index >= frame.start_idx && note.index <= frame.end_idx {
                    min_y = min_y.min(note.y);
                    max_y = max_y.max(note.y + note.height);
                }
            }
            let header_offset = theme.font_size * 0.6;
            let top_offset = (2.0 * base_spacing - header_offset).max(base_spacing);
            let bottom_offset = header_offset;
            let frame_y = min_y - top_offset;
            let frame_height = (max_y - min_y).max(0.0) + top_offset + bottom_offset;

            let frame_label_text = match frame.kind {
                crate::ir::SequenceFrameKind::Alt => "alt",
                crate::ir::SequenceFrameKind::Opt => "opt",
                crate::ir::SequenceFrameKind::Loop => "loop",
                crate::ir::SequenceFrameKind::Par => "par",
                crate::ir::SequenceFrameKind::Rect => "rect",
                crate::ir::SequenceFrameKind::Critical => "critical",
                crate::ir::SequenceFrameKind::Break => "break",
            };
            let label_block = measure_label(frame_label_text, theme, config);
            let label_box_w =
                (label_block.width + theme.font_size * 2.0).max(theme.font_size * 3.0);
            let label_box_h = theme.font_size * 1.25;
            let label_box_x = frame_x;
            let label_box_y = frame_y;
            let label = SequenceLabel {
                x: label_box_x + label_box_w / 2.0,
                y: label_box_y + label_box_h / 2.0,
                text: label_block,
            };

            let mut dividers = Vec::new();
            let divider_offset = theme.font_size * 0.9;
            for window in frame.sections.windows(2) {
                let prev_end = window[0].end_idx;
                let base_y = message_ys
                    .get(prev_end.saturating_sub(1))
                    .copied()
                    .unwrap_or(first_y);
                dividers.push(base_y + divider_offset);
            }

            let mut section_labels = Vec::new();
            let label_offset = theme.font_size * 0.7;
            for (section_idx, section) in frame.sections.iter().enumerate() {
                if let Some(label) = &section.label {
                    let display = format!("[{}]", label);
                    let block = measure_label(&display, theme, config);
                    let label_y = if section_idx == 0 {
                        frame_y + label_offset
                    } else {
                        dividers
                            .get(section_idx - 1)
                            .copied()
                            .unwrap_or(frame_y + label_offset)
                            + label_offset
                    };
                    section_labels.push(SequenceLabel {
                        x: frame_x + frame_width / 2.0,
                        y: label_y,
                        text: block,
                    });
                }
            }

            sequence_frames.push(SequenceFrameLayout {
                kind: frame.kind,
                x: frame_x,
                y: frame_y,
                width: frame_width,
                height: frame_height,
                label_box: (label_box_x, label_box_y, label_box_w, label_box_h),
                label,
                section_labels,
                dividers,
            });
        }
    }

    let lifeline_start = actor_height;
    let mut last_message_y = message_ys
        .last()
        .copied()
        .unwrap_or(lifeline_start + base_spacing);
    for note in &sequence_notes {
        last_message_y = last_message_y.max(note.y + note.height);
    }
    let footbox_gap = (theme.font_size * 1.25).max(16.0);
    let lifeline_end = last_message_y + footbox_gap;
    let mut lifelines = participants
        .iter()
        .filter_map(|id| nodes.get(id))
        .map(|node| Lifeline {
            id: node.id.clone(),
            x: node.x + node.width / 2.0,
            y1: lifeline_start,
            y2: lifeline_end,
        })
        .collect::<Vec<_>>();

    let mut sequence_footboxes = participants
        .iter()
        .filter_map(|id| nodes.get(id))
        .map(|node| {
            let mut foot = node.clone();
            foot.y = lifeline_end;
            foot
        })
        .collect::<Vec<_>>();

    let mut sequence_boxes = Vec::new();
    if !graph.sequence_boxes.is_empty() {
        let pad_x = theme.font_size * 0.8;
        let pad_y = theme.font_size * 0.6;
        let bottom = sequence_footboxes
            .iter()
            .map(|foot| foot.y + foot.height)
            .fold(lifeline_end, f32::max);
        for seq_box in &graph.sequence_boxes {
            let mut min_x = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            for participant in &seq_box.participants {
                if let Some(node) = nodes.get(participant) {
                    min_x = min_x.min(node.x);
                    max_x = max_x.max(node.x + node.width);
                }
            }
            if !min_x.is_finite() || !max_x.is_finite() {
                continue;
            }
            let x = min_x - pad_x;
            let y = 0.0;
            let width = (max_x - min_x) + pad_x * 2.0;
            let height = bottom + pad_y;
            let label = seq_box
                .label
                .as_ref()
                .map(|text| measure_label(text, theme, config));
            sequence_boxes.push(SequenceBoxLayout {
                x,
                y,
                width,
                height,
                label,
                color: seq_box.color.clone(),
            });
        }
    }
    let activation_width = (theme.font_size * 0.75).max(10.0);
    let activation_offset = (activation_width * 0.6).max(4.0);
    let activation_end_default = message_ys
        .last()
        .copied()
        .unwrap_or(lifeline_start + base_spacing * 0.5)
        + base_spacing * 0.6;
    let mut sequence_activations = Vec::new();
    let mut activation_stacks: HashMap<String, Vec<(f32, usize)>> = HashMap::new();
    let mut events = graph
        .sequence_activations
        .iter()
        .cloned()
        .enumerate()
        .map(|(order, event)| (event.index, order, event))
        .collect::<Vec<_>>();
    events.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let activation_y_for = |idx: usize| {
        if idx < message_ys.len() {
            message_ys[idx]
        } else {
            activation_end_default
        }
    };
    for (_, _, event) in events {
        let y = activation_y_for(event.index);
        let stack = activation_stacks
            .entry(event.participant.clone())
            .or_default();
        match event.kind {
            crate::ir::SequenceActivationKind::Activate => {
                let depth = stack.len();
                stack.push((y, depth));
            }
            crate::ir::SequenceActivationKind::Deactivate => {
                if let Some((start_y, depth)) = stack.pop()
                    && let Some(node) = nodes.get(&event.participant)
                {
                    let base_x = node.x + node.width / 2.0 - activation_width / 2.0;
                    let x = base_x + depth as f32 * activation_offset;
                    let mut y0 = start_y.min(y);
                    let mut height = (y - start_y).abs();
                    if height < base_spacing * 0.6 {
                        height = base_spacing * 0.6;
                    }
                    if y0 < lifeline_start {
                        y0 = lifeline_start;
                    }
                    sequence_activations.push(SequenceActivationLayout {
                        x,
                        y: y0,
                        width: activation_width,
                        height,
                        participant: event.participant.clone(),
                        depth,
                    });
                }
            }
        }
    }
    for (participant, stack) in activation_stacks {
        for (start_y, depth) in stack {
            if let Some(node) = nodes.get(&participant) {
                let base_x = node.x + node.width / 2.0 - activation_width / 2.0;
                let x = base_x + depth as f32 * activation_offset;
                let mut y0 = start_y.min(activation_end_default);
                let mut height = (activation_end_default - start_y).abs();
                if height < base_spacing * 0.6 {
                    height = base_spacing * 0.6;
                }
                if y0 < lifeline_start {
                    y0 = lifeline_start;
                }
                sequence_activations.push(SequenceActivationLayout {
                    x,
                    y: y0,
                    width: activation_width,
                    height,
                    participant: participant.clone(),
                    depth,
                });
            }
        }
    }

    let mut sequence_numbers = Vec::new();
    if let Some(start) = graph.sequence_autonumber {
        let mut value = start;
        for (idx, edge) in graph.edges.iter().enumerate() {
            if let (Some(from), Some(y)) = (nodes.get(&edge.from), message_ys.get(idx).copied()) {
                let from_x = from.x + from.width / 2.0;
                let to_x = nodes
                    .get(&edge.to)
                    .map(|node| node.x + node.width / 2.0)
                    .unwrap_or(from_x);
                let offset = if to_x >= from_x { 16.0 } else { -16.0 };
                sequence_numbers.push(SequenceNumberLayout {
                    x: from_x + offset,
                    y,
                    value,
                });
                value += 1;
            }
        }
    }

    let (mut width, mut height) = bounds_from_layout(&nodes, &subgraphs);
    let mut max_x = width.max(cursor_x + 40.0) - 60.0;
    let mut max_y = height - 60.0;
    let mut min_x: f32 = 0.0;
    for note in &sequence_notes {
        min_x = min_x.min(note.x);
        max_x = max_x.max(note.x + note.width);
        max_y = max_y.max(note.y + note.height);
    }
    for frame in &sequence_frames {
        min_x = min_x.min(frame.x);
        max_x = max_x.max(frame.x + frame.width);
        max_y = max_y.max(frame.y + frame.height);
    }
    for activation in &sequence_activations {
        min_x = min_x.min(activation.x);
        max_x = max_x.max(activation.x + activation.width);
        max_y = max_y.max(activation.y + activation.height);
    }
    for number in &sequence_numbers {
        min_x = min_x.min(number.x);
        max_x = max_x.max(number.x);
        max_y = max_y.max(number.y);
    }

    let shift_x = if min_x < 0.0 { -min_x + 20.0 } else { 0.0 };
    if shift_x > 0.0 {
        for node in nodes.values_mut() {
            node.x += shift_x;
        }
        for edge in &mut edges {
            for point in &mut edge.points {
                point.0 += shift_x;
            }
        }
        for lifeline in &mut lifelines {
            lifeline.x += shift_x;
        }
        for footbox in &mut sequence_footboxes {
            footbox.x += shift_x;
        }
        for frame in &mut sequence_frames {
            frame.x += shift_x;
            frame.label_box.0 += shift_x;
            frame.label.x += shift_x;
            for label in &mut frame.section_labels {
                label.x += shift_x;
            }
        }
        for note in &mut sequence_notes {
            note.x += shift_x;
        }
        for activation in &mut sequence_activations {
            activation.x += shift_x;
        }
        for number in &mut sequence_numbers {
            number.x += shift_x;
        }
        max_x += shift_x;
    }

    let footbox_height = sequence_footboxes
        .iter()
        .map(|node| node.height)
        .fold(0.0, f32::max);
    max_y = max_y.max(lifeline_end + footbox_height);
    width = max_x + 60.0;
    height = max_y + 60.0;

    Layout {
        kind: graph.kind,
        nodes,
        edges,
        subgraphs,
        lifelines,
        sequence_footboxes,
        sequence_boxes,
        sequence_frames,
        sequence_notes,
        sequence_activations,
        sequence_numbers,
        state_notes: Vec::new(),
        pie_slices: Vec::new(),
        pie_legend: Vec::new(),
        pie_center: (0.0, 0.0),
        pie_radius: 0.0,
        pie_title: None,
        quadrant: None,
        gantt: None,
        width,
        height,
    }
}

fn resolve_edge_style(idx: usize, graph: &Graph) -> crate::ir::EdgeStyleOverride {
    let mut style = graph.edge_style_default.clone().unwrap_or_default();
    if let Some(edge_style) = graph.edge_styles.get(&idx) {
        merge_edge_style(&mut style, edge_style);
    }
    style
}

fn merge_edge_style(
    target: &mut crate::ir::EdgeStyleOverride,
    source: &crate::ir::EdgeStyleOverride,
) {
    if source.stroke.is_some() {
        target.stroke = source.stroke.clone();
    }
    if source.stroke_width.is_some() {
        target.stroke_width = source.stroke_width;
    }
    if source.dasharray.is_some() {
        target.dasharray = source.dasharray.clone();
    }
    if source.label_color.is_some() {
        target.label_color = source.label_color.clone();
    }
}

fn order_rank_nodes(
    rank_nodes: &mut [Vec<String>],
    edges: &[crate::ir::Edge],
    node_order: &HashMap<String, usize>,
) {
    if rank_nodes.len() <= 1 {
        return;
    }
    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();

    for edge in edges {
        outgoing
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        incoming
            .entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());
    }

    let mut positions: HashMap<String, usize> = HashMap::new();
    let update_positions = |rank_nodes: &mut [Vec<String>],
                            positions: &mut HashMap<String, usize>| {
        positions.clear();
        for bucket in rank_nodes.iter() {
            for (idx, node_id) in bucket.iter().enumerate() {
                positions.insert(node_id.clone(), idx);
            }
        }
    };

    update_positions(rank_nodes, &mut positions);

    let sort_bucket = |bucket: &mut Vec<String>,
                       neighbors: &HashMap<String, Vec<String>>,
                       positions: &HashMap<String, usize>| {
        let current_positions: HashMap<String, usize> = bucket
            .iter()
            .enumerate()
            .map(|(idx, id)| (id.clone(), idx))
            .collect();
        bucket.sort_by(|a, b| {
            let a_score = barycenter(a, neighbors, positions, &current_positions);
            let b_score = barycenter(b, neighbors, positions, &current_positions);
            match a_score.partial_cmp(&b_score) {
                Some(std::cmp::Ordering::Equal) | None => {
                    let a_pos = current_positions.get(a).copied().unwrap_or(0);
                    let b_pos = current_positions.get(b).copied().unwrap_or(0);
                    match a_pos.cmp(&b_pos) {
                        std::cmp::Ordering::Equal => node_order
                            .get(a)
                            .copied()
                            .unwrap_or(usize::MAX)
                            .cmp(&node_order.get(b).copied().unwrap_or(usize::MAX)),
                        other => other,
                    }
                }
                Some(ordering) => ordering,
            }
        });
    };

    for _ in 0..2 {
        for rank in 1..rank_nodes.len() {
            if rank_nodes[rank].len() <= 1 {
                continue;
            }
            sort_bucket(&mut rank_nodes[rank], &incoming, &positions);
            update_positions(rank_nodes, &mut positions);
        }
        for rank in (0..rank_nodes.len().saturating_sub(1)).rev() {
            if rank_nodes[rank].len() <= 1 {
                continue;
            }
            sort_bucket(&mut rank_nodes[rank], &outgoing, &positions);
            update_positions(rank_nodes, &mut positions);
        }
    }
}

fn barycenter(
    node_id: &str,
    neighbors: &HashMap<String, Vec<String>>,
    positions: &HashMap<String, usize>,
    current_positions: &HashMap<String, usize>,
) -> f32 {
    let Some(list) = neighbors.get(node_id) else {
        return *current_positions.get(node_id).unwrap_or(&0) as f32;
    };
    let mut total = 0.0;
    let mut count = 0.0;
    for neighbor in list {
        if let Some(pos) = positions.get(neighbor) {
            total += *pos as f32;
            count += 1.0;
        }
    }
    if count == 0.0 {
        *current_positions.get(node_id).unwrap_or(&0) as f32
    } else {
        total / count
    }
}

fn apply_subgraph_bands(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    anchored_nodes: &HashSet<String>,
    config: &LayoutConfig,
) {
    let mut group_nodes: Vec<Vec<String>> = Vec::new();
    let mut node_group: HashMap<String, usize> = HashMap::new();

    // Group 0: nodes not in any subgraph.
    group_nodes.push(Vec::new());
    for node_id in graph.nodes.keys() {
        if anchored_nodes.contains(node_id) {
            continue;
        }
        node_group.insert(node_id.clone(), 0);
    }

    let top_level = top_level_subgraph_indices(graph);
    for (pos, idx) in top_level.iter().enumerate() {
        let group_idx = pos + 1;
        let sub = &graph.subgraphs[*idx];
        group_nodes.push(Vec::new());
        for node_id in &sub.nodes {
            if anchored_nodes.contains(node_id) {
                continue;
            }
            if nodes.contains_key(node_id) {
                node_group.insert(node_id.clone(), group_idx);
            }
        }
    }

    for (node_id, group_idx) in &node_group {
        if let Some(bucket) = group_nodes.get_mut(*group_idx) {
            bucket.push(node_id.clone());
        }
    }

    let mut groups: Vec<(usize, f32, f32, f32, f32)> = Vec::new();
    for (idx, bucket) in group_nodes.iter().enumerate() {
        if bucket.is_empty() {
            continue;
        }
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for node_id in bucket {
            if let Some(node) = nodes.get(node_id) {
                min_x = min_x.min(node.x);
                min_y = min_y.min(node.y);
                max_x = max_x.max(node.x + node.width);
                max_y = max_y.max(node.y + node.height);
            }
        }
        if min_x != f32::MAX {
            groups.push((idx, min_x, min_y, max_x, max_y));
        }
    }

    // Order groups by their current position to minimize crossing shifts.
    // Keep the non-subgraph group first to bias subgraphs after the main flow.
    if is_horizontal(graph.direction) {
        groups.sort_by(|a, b| {
            let a_primary = if a.0 == 0 { 0 } else { 1 };
            let b_primary = if b.0 == 0 { 0 } else { 1 };
            a_primary
                .cmp(&b_primary)
                .then_with(|| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        });
    } else {
        groups.sort_by(|a, b| {
            let a_primary = if a.0 == 0 { 0 } else { 1 };
            let b_primary = if b.0 == 0 { 0 } else { 1 };
            a_primary
                .cmp(&b_primary)
                .then_with(|| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
        });
    }

    let spacing = config.rank_spacing * 0.8;
    if is_horizontal(graph.direction) {
        let mut cursor = groups
            .iter()
            .find(|group| group.0 == 0)
            .map(|group| group.3)
            .unwrap_or(0.0)
            + spacing;
        for (group_idx, min_x, _min_y, max_x, _max_y) in groups {
            if group_idx == 0 {
                continue;
            }
            let width = max_x - min_x;
            let offset = cursor - min_x;
            for node_id in group_nodes[group_idx].iter() {
                if let Some(node) = nodes.get_mut(node_id) {
                    node.x += offset;
                }
            }
            cursor += width + spacing;
        }
    } else {
        let mut cursor = groups
            .iter()
            .find(|group| group.0 == 0)
            .map(|group| group.4)
            .unwrap_or(0.0)
            + spacing;
        for (group_idx, _min_x, min_y, _max_x, max_y) in groups {
            if group_idx == 0 {
                continue;
            }
            let height = max_y - min_y;
            let offset = cursor - min_y;
            for node_id in group_nodes[group_idx].iter() {
                if let Some(node) = nodes.get_mut(node_id) {
                    node.y += offset;
                }
            }
            cursor += height + spacing;
        }
    }
}

fn apply_orthogonal_region_bands(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    let mut region_indices = Vec::new();
    for (idx, sub) in graph.subgraphs.iter().enumerate() {
        if is_region_subgraph(sub) {
            region_indices.push(idx);
        }
    }
    if region_indices.is_empty() {
        return;
    }

    let sets: Vec<HashSet<String>> = graph
        .subgraphs
        .iter()
        .map(|sub| sub.nodes.iter().cloned().collect())
        .collect();

    let mut parent_map: HashMap<usize, Vec<usize>> = HashMap::new();
    for region_idx in region_indices {
        let region_set = &sets[region_idx];
        let mut parent: Option<usize> = None;
        for (idx, set) in sets.iter().enumerate() {
            if idx == region_idx {
                continue;
            }
            if set.len() <= region_set.len() {
                continue;
            }
            if !region_set.is_subset(set) {
                continue;
            }
            if is_region_subgraph(&graph.subgraphs[idx]) {
                continue;
            }
            match parent {
                None => parent = Some(idx),
                Some(current) => {
                    if set.len() < sets[current].len() {
                        parent = Some(idx);
                    }
                }
            }
        }
        if let Some(parent_idx) = parent {
            parent_map.entry(parent_idx).or_default().push(region_idx);
        }
    }

    let spacing = config.rank_spacing * 0.6;
    let stack_along_x = is_horizontal(graph.direction);

    for region_list in parent_map.values() {
        let mut region_boxes: Vec<(usize, f32, f32, f32, f32)> = Vec::new();
        for &region_idx in region_list {
            let mut min_x = f32::MAX;
            let mut min_y = f32::MAX;
            let mut max_x = f32::MIN;
            let mut max_y = f32::MIN;
            for node_id in &graph.subgraphs[region_idx].nodes {
                if let Some(node) = nodes.get(node_id) {
                    min_x = min_x.min(node.x);
                    min_y = min_y.min(node.y);
                    max_x = max_x.max(node.x + node.width);
                    max_y = max_y.max(node.y + node.height);
                }
            }
            if min_x != f32::MAX {
                region_boxes.push((region_idx, min_x, min_y, max_x, max_y));
            }
        }
        if region_boxes.len() <= 1 {
            continue;
        }

        if stack_along_x {
            region_boxes.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            let mut cursor = region_boxes.first().map(|entry| entry.1).unwrap_or(0.0);
            for (region_idx, min_x, _min_y, max_x, _max_y) in region_boxes {
                let offset = cursor - min_x;
                for node_id in &graph.subgraphs[region_idx].nodes {
                    if let Some(node) = nodes.get_mut(node_id) {
                        node.x += offset;
                    }
                }
                cursor += (max_x - min_x) + spacing;
            }
        } else {
            region_boxes.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
            let mut cursor = region_boxes.first().map(|entry| entry.2).unwrap_or(0.0);
            for (region_idx, _min_x, min_y, _max_x, max_y) in region_boxes {
                let offset = cursor - min_y;
                for node_id in &graph.subgraphs[region_idx].nodes {
                    if let Some(node) = nodes.get_mut(node_id) {
                        node.y += offset;
                    }
                }
                cursor += (max_y - min_y) + spacing;
            }
        }
    }
}

fn top_level_subgraph_indices(graph: &Graph) -> Vec<usize> {
    let mut sets: Vec<HashSet<String>> = Vec::new();
    for sub in &graph.subgraphs {
        sets.push(sub.nodes.iter().cloned().collect());
    }

    let mut top_level = Vec::new();
    for i in 0..graph.subgraphs.len() {
        let mut nested = false;
        for j in 0..graph.subgraphs.len() {
            if i == j {
                continue;
            }
            if sets[j].len() > sets[i].len() && sets[i].is_subset(&sets[j]) {
                nested = true;
                break;
            }
        }
        if !nested {
            top_level.push(i);
        }
    }
    top_level
}

fn apply_subgraph_direction_overrides(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
    skip_indices: &HashSet<usize>,
) {
    for (idx, sub) in graph.subgraphs.iter().enumerate() {
        if skip_indices.contains(&idx) {
            continue;
        }
        if is_region_subgraph(sub) {
            continue;
        }
        let direction = match sub.direction {
            Some(direction) => direction,
            None => {
                if graph.kind != crate::ir::DiagramKind::Flowchart {
                    continue;
                }
                subgraph_layout_direction(graph, sub)
            }
        };
        if sub.nodes.is_empty() {
            continue;
        }
        if direction == graph.direction {
            continue;
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        for node_id in &sub.nodes {
            if let Some(node) = nodes.get(node_id) {
                min_x = min_x.min(node.x);
                min_y = min_y.min(node.y);
            }
        }
        if min_x == f32::MAX {
            continue;
        }

        let mut temp_nodes: BTreeMap<String, NodeLayout> = BTreeMap::new();
        for node_id in &sub.nodes {
            if let Some(node) = nodes.get(node_id) {
                let mut clone = node.clone();
                clone.x = 0.0;
                clone.y = 0.0;
                temp_nodes.insert(node_id.clone(), clone);
            }
        }
        let local_config = subgraph_layout_config(graph, false, config);
        let applied = assign_positions_dagre_subset(
            &sub.nodes,
            &graph.edges,
            &mut temp_nodes,
            direction,
            &local_config,
            Some(&graph.node_order),
        );
        if !applied {
            let ranks = compute_ranks_subset(&sub.nodes, &graph.edges);
            assign_positions(
                &sub.nodes,
                &ranks,
                direction,
                &local_config,
                &mut temp_nodes,
                0.0,
                0.0,
            );
        }
        let mut temp_min_x = f32::MAX;
        let mut temp_min_y = f32::MAX;
        for node_id in &sub.nodes {
            if let Some(node) = temp_nodes.get(node_id) {
                temp_min_x = temp_min_x.min(node.x);
                temp_min_y = temp_min_y.min(node.y);
            }
        }
        if temp_min_x == f32::MAX {
            continue;
        }
        for node_id in &sub.nodes {
            if let (Some(target), Some(source)) = (nodes.get_mut(node_id), temp_nodes.get(node_id))
            {
                target.x = source.x - temp_min_x + min_x;
                target.y = source.y - temp_min_y + min_y;
            }
        }

        if matches!(direction, Direction::RightLeft | Direction::BottomTop) {
            mirror_subgraph_nodes(&sub.nodes, nodes, direction);
        }
    }
}

fn subgraph_is_anchorable(
    sub: &crate::ir::Subgraph,
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
) -> bool {
    if sub.nodes.is_empty() {
        return false;
    }
    let anchor_id = subgraph_anchor_id(sub, nodes);
    let set: HashSet<&str> = sub.nodes.iter().map(|id| id.as_str()).collect();
    for edge in &graph.edges {
        if let Some(anchor) = anchor_id
            && (edge.from == anchor || edge.to == anchor)
        {
            return false;
        }
        let from_in = set.contains(edge.from.as_str());
        let to_in = set.contains(edge.to.as_str());
        if from_in ^ to_in {
            return false;
        }
    }
    true
}

fn subgraph_should_anchor(
    sub: &crate::ir::Subgraph,
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
) -> bool {
    if sub.nodes.is_empty() {
        return false;
    }
    // For flowcharts and state diagrams, anchor if there's an anchor node
    // State diagram composite states can have external edges, so we can't use
    // subgraph_is_anchorable which rejects subgraphs with external edges
    if graph.kind == crate::ir::DiagramKind::Flowchart
        || graph.kind == crate::ir::DiagramKind::State
    {
        return subgraph_anchor_id(sub, nodes).is_some();
    }
    subgraph_is_anchorable(sub, graph, nodes)
}

fn subgraph_anchor_id<'a>(
    sub: &'a crate::ir::Subgraph,
    nodes: &BTreeMap<String, NodeLayout>,
) -> Option<&'a str> {
    if let Some(id) = sub.id.as_deref()
        && nodes.contains_key(id)
        && !sub.nodes.iter().any(|node_id| node_id == id)
    {
        return Some(id);
    }
    let label = sub.label.as_str();
    if nodes.contains_key(label) && !sub.nodes.iter().any(|node_id| node_id == label) {
        return Some(label);
    }
    None
}

fn mark_subgraph_anchor_nodes_hidden(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
) -> HashSet<String> {
    let mut anchor_ids = HashSet::new();
    for sub in &graph.subgraphs {
        let Some(anchor_id) = subgraph_anchor_id(sub, nodes) else {
            continue;
        };
        anchor_ids.insert(anchor_id.to_string());
        if let Some(node) = nodes.get_mut(anchor_id) {
            node.hidden = true;
        }
    }
    anchor_ids
}

fn pick_subgraph_anchor_child(
    sub: &crate::ir::Subgraph,
    graph: &Graph,
    anchor_ids: &HashSet<String>,
) -> Option<String> {
    let mut candidates: Vec<&String> = sub
        .nodes
        .iter()
        .filter(|id| !anchor_ids.contains(*id))
        .collect();
    if candidates.is_empty() {
        candidates = sub.nodes.iter().collect();
    }
    candidates.sort_by_key(|id| graph.node_order.get(*id).copied().unwrap_or(usize::MAX));
    candidates.first().map(|id| (*id).clone())
}

#[derive(Debug, Clone)]
struct SubgraphAnchorInfo {
    sub_idx: usize,
    padding: f32,
    top_padding: f32,
}

fn subgraph_layout_direction(graph: &Graph, sub: &crate::ir::Subgraph) -> Direction {
    if let Some(direction) = sub.direction {
        return direction;
    }
    if graph.kind == crate::ir::DiagramKind::State {
        return graph.direction;
    }
    if sub.nodes.len() <= 1 {
        return graph.direction;
    }
    match graph.direction {
        Direction::TopDown | Direction::BottomTop => Direction::LeftRight,
        Direction::LeftRight | Direction::RightLeft => Direction::TopDown,
    }
}

fn subgraph_layout_config(graph: &Graph, anchorable: bool, config: &LayoutConfig) -> LayoutConfig {
    let mut local = config.clone();
    if graph.kind == crate::ir::DiagramKind::Flowchart && anchorable {
        local.rank_spacing = config.rank_spacing + 25.0;
    }
    local
}

fn subgraph_padding(
    graph: &Graph,
    sub: &crate::ir::Subgraph,
    theme: &Theme,
    config: &LayoutConfig,
) -> (f32, f32) {
    let label_empty = sub.label.trim().is_empty();
    let mut label_block = measure_label(&sub.label, theme, config);
    if label_empty {
        label_block.width = 0.0;
        label_block.height = 0.0;
    }
    let base_padding = if graph.kind == crate::ir::DiagramKind::State {
        16.0
    } else {
        24.0
    };
    let padding = if is_region_subgraph(sub) {
        0.0
    } else {
        base_padding
    };
    let label_height = if label_empty { 0.0 } else { label_block.height };
    let top_padding = if label_empty {
        padding
    } else if graph.kind == crate::ir::DiagramKind::State {
        (label_height + theme.font_size * 0.4).max(18.0)
    } else {
        padding + label_height + 8.0
    };
    (padding, top_padding)
}

fn estimate_subgraph_box_size(
    graph: &Graph,
    sub: &crate::ir::Subgraph,
    nodes: &BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
    anchorable: bool,
) -> Option<(f32, f32, f32, f32)> {
    if sub.nodes.is_empty() {
        return None;
    }
    let direction = subgraph_layout_direction(graph, sub);
    let mut temp_nodes: BTreeMap<String, NodeLayout> = BTreeMap::new();
    for node_id in &sub.nodes {
        if let Some(node) = nodes.get(node_id) {
            let mut clone = node.clone();
            clone.x = 0.0;
            clone.y = 0.0;
            temp_nodes.insert(node_id.clone(), clone);
        }
    }
    let local_config = subgraph_layout_config(graph, anchorable, config);
    let applied = assign_positions_dagre_subset(
        &sub.nodes,
        &graph.edges,
        &mut temp_nodes,
        direction,
        &local_config,
        Some(&graph.node_order),
    );
    if !applied {
        let ranks = compute_ranks_subset(&sub.nodes, &graph.edges);
        assign_positions(
            &sub.nodes,
            &ranks,
            direction,
            &local_config,
            &mut temp_nodes,
            0.0,
            0.0,
        );
    }
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for node_id in &sub.nodes {
        if let Some(node) = temp_nodes.get(node_id) {
            min_x = min_x.min(node.x);
            min_y = min_y.min(node.y);
            max_x = max_x.max(node.x + node.width);
            max_y = max_y.max(node.y + node.height);
        }
    }
    if min_x == f32::MAX {
        return None;
    }
    let (padding, top_padding) = subgraph_padding(graph, sub, theme, config);
    let width = (max_x - min_x) + padding * 2.0;
    let height = (max_y - min_y) + padding + top_padding;
    Some((width, height, padding, top_padding))
}

fn apply_subgraph_anchor_sizes(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) -> HashMap<String, SubgraphAnchorInfo> {
    let mut anchors: HashMap<String, SubgraphAnchorInfo> = HashMap::new();
    if graph.subgraphs.is_empty() {
        return anchors;
    }
    for (idx, sub) in graph.subgraphs.iter().enumerate() {
        if is_region_subgraph(sub) {
            continue;
        }
        if !subgraph_should_anchor(sub, graph, nodes) {
            continue;
        }
        let Some(anchor_id) = subgraph_anchor_id(sub, nodes) else {
            continue;
        };
        let Some((width, height, padding, top_padding)) =
            estimate_subgraph_box_size(graph, sub, nodes, theme, config, true)
        else {
            continue;
        };
        if let Some(node) = nodes.get_mut(anchor_id) {
            node.width = width;
            node.height = height;
        }
        anchors.insert(
            anchor_id.to_string(),
            SubgraphAnchorInfo {
                sub_idx: idx,
                padding,
                top_padding,
            },
        );
    }
    anchors
}

fn align_subgraphs_to_anchor_nodes(
    graph: &Graph,
    anchor_info: &HashMap<String, SubgraphAnchorInfo>,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) -> HashSet<String> {
    let mut anchored_nodes = HashSet::new();
    if anchor_info.is_empty() {
        return anchored_nodes;
    }
    for (anchor_id, info) in anchor_info {
        let (anchor_x, anchor_y) = {
            let Some(anchor) = nodes.get(anchor_id) else {
                continue;
            };
            (anchor.x, anchor.y)
        };
        let Some(sub) = graph.subgraphs.get(info.sub_idx) else {
            continue;
        };
        let direction = subgraph_layout_direction(graph, sub);
        let local_config = subgraph_layout_config(graph, true, config);
        let applied = assign_positions_dagre_subset(
            &sub.nodes,
            &graph.edges,
            nodes,
            direction,
            &local_config,
            Some(&graph.node_order),
        );
        if !applied {
            let ranks = compute_ranks_subset(&sub.nodes, &graph.edges);
            assign_positions(
                &sub.nodes,
                &ranks,
                direction,
                &local_config,
                nodes,
                anchor_x + info.padding,
                anchor_y + info.top_padding,
            );
        } else {
            for node_id in &sub.nodes {
                if let Some(node) = nodes.get_mut(node_id) {
                    node.x += anchor_x + info.padding;
                    node.y += anchor_y + info.top_padding;
                }
            }
        }
        if matches!(direction, Direction::RightLeft | Direction::BottomTop) {
            mirror_subgraph_nodes(&sub.nodes, nodes, direction);
        }
        anchored_nodes.extend(sub.nodes.iter().cloned());
    }
    anchored_nodes
}

fn apply_state_subgraph_layouts(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
    skip_indices: &HashSet<usize>,
) {
    for (idx, sub) in graph.subgraphs.iter().enumerate() {
        if skip_indices.contains(&idx) {
            continue;
        }
        if sub.nodes.len() <= 1 {
            continue;
        }
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        for node_id in &sub.nodes {
            if let Some(node) = nodes.get(node_id) {
                min_x = min_x.min(node.x);
                min_y = min_y.min(node.y);
            }
        }
        if min_x == f32::MAX {
            continue;
        }
        let ranks = compute_ranks_subset(&sub.nodes, &graph.edges);
        assign_positions(
            &sub.nodes,
            &ranks,
            graph.direction,
            config,
            nodes,
            min_x,
            min_y,
        );
    }
}

fn apply_subgraph_anchors(
    graph: &Graph,
    subgraphs: &[SubgraphLayout],
    nodes: &mut BTreeMap<String, NodeLayout>,
) {
    if subgraphs.is_empty() {
        return;
    }

    let mut label_to_index: HashMap<&str, usize> = HashMap::new();
    for (idx, sub) in subgraphs.iter().enumerate() {
        label_to_index.insert(sub.label.as_str(), idx);
    }

    for sub in &graph.subgraphs {
        let Some(&layout_idx) = label_to_index.get(sub.label.as_str()) else {
            continue;
        };
        let layout = &subgraphs[layout_idx];
        let mut anchor_ids: HashSet<&str> = HashSet::new();
        if let Some(id) = &sub.id {
            anchor_ids.insert(id.as_str());
        }
        anchor_ids.insert(sub.label.as_str());

        for anchor_id in anchor_ids {
            if sub.nodes.iter().any(|node_id| node_id == anchor_id) {
                continue;
            }
            let Some(node) = nodes.get_mut(anchor_id) else {
                continue;
            };
            node.anchor_subgraph = Some(layout_idx);
            let size = 2.0;
            node.width = size;
            node.height = size;
            node.x = layout.x + layout.width / 2.0 - size / 2.0;
            node.y = layout.y + layout.height / 2.0 - size / 2.0;
        }
    }
}

fn anchor_layout_for_edge(
    anchor: &NodeLayout,
    subgraph: &SubgraphLayout,
    direction: Direction,
    is_from: bool,
) -> NodeLayout {
    let size = 2.0;
    let mut node = anchor.clone();
    node.width = size;
    node.height = size;

    if is_horizontal(direction) {
        let x = if is_from {
            subgraph.x + subgraph.width - size
        } else {
            subgraph.x
        };
        let y = subgraph.y + subgraph.height / 2.0 - size / 2.0;
        node.x = x;
        node.y = y;
    } else {
        let x = subgraph.x + subgraph.width / 2.0 - size / 2.0;
        let y = if is_from {
            subgraph.y + subgraph.height - size
        } else {
            subgraph.y
        };
        node.x = x;
        node.y = y;
    }

    node
}

fn mirror_subgraph_nodes(
    node_ids: &[String],
    nodes: &mut BTreeMap<String, NodeLayout>,
    direction: Direction,
) {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    for node_id in node_ids {
        if let Some(node) = nodes.get(node_id) {
            min_x = min_x.min(node.x);
            min_y = min_y.min(node.y);
            max_x = max_x.max(node.x + node.width);
            max_y = max_y.max(node.y + node.height);
        }
    }

    if min_x == f32::MAX {
        return;
    }

    if matches!(direction, Direction::RightLeft) {
        for node_id in node_ids {
            if let Some(node) = nodes.get_mut(node_id) {
                node.x = min_x + (max_x - (node.x + node.width));
            }
        }
    }
    if matches!(direction, Direction::BottomTop) {
        for node_id in node_ids {
            if let Some(node) = nodes.get_mut(node_id) {
                node.y = min_y + (max_y - (node.y + node.height));
            }
        }
    }
}

fn compute_ranks_subset(node_ids: &[String], edges: &[crate::ir::Edge]) -> HashMap<String, usize> {
    let mut indeg: HashMap<String, usize> = HashMap::new();
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    let set: HashSet<String> = node_ids.iter().cloned().collect();

    for id in &set {
        indeg.insert(id.clone(), 0);
    }

    for edge in edges {
        if set.contains(&edge.from) && set.contains(&edge.to) {
            adj.entry(edge.from.clone())
                .or_default()
                .push(edge.to.clone());
            *indeg.entry(edge.to.clone()).or_insert(0) += 1;
        }
    }

    let mut queue: VecDeque<String> = indeg
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(id, _)| id.clone())
        .collect();

    let mut order = Vec::new();
    while let Some(node) = queue.pop_front() {
        order.push(node.clone());
        if let Some(nexts) = adj.get(&node) {
            for next in nexts {
                if let Some(deg) = indeg.get_mut(next) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(next.clone());
                    }
                }
            }
        }
    }

    if order.len() < set.len() {
        for id in node_ids {
            if !order.contains(id) {
                order.push(id.clone());
            }
        }
    }

    let order_index: HashMap<String, usize> = order
        .iter()
        .enumerate()
        .map(|(idx, id)| (id.clone(), idx))
        .collect();

    let mut ranks: HashMap<String, usize> = HashMap::new();
    for node in &order {
        let rank = *ranks.get(node).unwrap_or(&0);
        ranks.entry(node.clone()).or_insert(rank);
        if let Some(nexts) = adj.get(node) {
            let from_idx = *order_index.get(node).unwrap_or(&0);
            for next in nexts {
                let to_idx = *order_index.get(next).unwrap_or(&from_idx);
                if to_idx <= from_idx {
                    continue;
                }
                let entry = ranks.entry(next.clone()).or_insert(0);
                *entry = (*entry).max(rank + 1);
            }
        }
    }

    ranks
}

fn assign_positions(
    node_ids: &[String],
    ranks: &HashMap<String, usize>,
    direction: Direction,
    config: &LayoutConfig,
    nodes: &mut BTreeMap<String, NodeLayout>,
    origin_x: f32,
    origin_y: f32,
) {
    let mut max_rank = 0usize;
    for rank in ranks.values() {
        max_rank = max_rank.max(*rank);
    }

    let mut rank_nodes: Vec<Vec<String>> = vec![Vec::new(); max_rank + 1];
    for node_id in node_ids {
        let rank = *ranks.get(node_id).unwrap_or(&0);
        if let Some(bucket) = rank_nodes.get_mut(rank) {
            bucket.push(node_id.clone());
        }
    }
    for bucket in &mut rank_nodes {
        bucket.sort();
    }

    let mut main_cursor = 0.0;
    for bucket in rank_nodes {
        let mut cross_cursor = 0.0;
        let mut max_main: f32 = 0.0;
        for node_id in bucket {
            if let Some(node) = nodes.get_mut(&node_id) {
                if is_horizontal(direction) {
                    node.x = origin_x + main_cursor;
                    node.y = origin_y + cross_cursor;
                    cross_cursor += node.height + config.node_spacing;
                    max_main = max_main.max(node.width);
                } else {
                    node.x = origin_x + cross_cursor;
                    node.y = origin_y + main_cursor;
                    cross_cursor += node.width + config.node_spacing;
                    max_main = max_main.max(node.height);
                }
            }
        }
        main_cursor += max_main + config.rank_spacing;
    }
}

fn bounds_from_layout(
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
) -> (f32, f32) {
    let (max_x, max_y) = bounds_without_padding(nodes, subgraphs);
    (max_x + 60.0, max_y + 60.0)
}

fn bounds_without_padding(
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
) -> (f32, f32) {
    let mut max_x: f32 = 0.0;
    let mut max_y: f32 = 0.0;
    for node in nodes.values() {
        max_x = max_x.max(node.x + node.width);
        max_y = max_y.max(node.y + node.height);
    }
    for sub in subgraphs {
        let invisible_region = sub.label.trim().is_empty()
            && sub.style.stroke.as_deref() == Some("none")
            && sub.style.fill.as_deref() == Some("none");
        if invisible_region {
            continue;
        }
        max_x = max_x.max(sub.x + sub.width);
        max_y = max_y.max(sub.y + sub.height);
    }
    (max_x, max_y)
}

fn apply_direction_mirror(
    direction: Direction,
    nodes: &mut BTreeMap<String, NodeLayout>,
    edges: &mut [EdgeLayout],
    subgraphs: &mut [SubgraphLayout],
) {
    let (max_x, max_y) = bounds_without_padding(nodes, subgraphs);
    if matches!(direction, Direction::RightLeft) {
        for node in nodes.values_mut() {
            node.x = max_x - node.x - node.width;
        }
        for edge in edges.iter_mut() {
            for point in edge.points.iter_mut() {
                point.0 = max_x - point.0;
            }
        }
        for sub in subgraphs.iter_mut() {
            sub.x = max_x - sub.x - sub.width;
        }
    }
    if matches!(direction, Direction::BottomTop) {
        for node in nodes.values_mut() {
            node.y = max_y - node.y - node.height;
        }
        for edge in edges.iter_mut() {
            for point in edge.points.iter_mut() {
                point.1 = max_y - point.1;
            }
        }
        for sub in subgraphs.iter_mut() {
            sub.y = max_y - sub.y - sub.height;
        }
    }
}

fn normalize_layout(
    nodes: &mut BTreeMap<String, NodeLayout>,
    edges: &mut [EdgeLayout],
    subgraphs: &mut [SubgraphLayout],
) {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    for node in nodes.values() {
        min_x = min_x.min(node.x);
        min_y = min_y.min(node.y);
    }
    for sub in subgraphs.iter() {
        min_x = min_x.min(sub.x);
        min_y = min_y.min(sub.y);
    }

    let padding = 24.0;
    let shift_x = if min_x < padding {
        padding - min_x
    } else {
        0.0
    };
    let shift_y = if min_y < padding {
        padding - min_y
    } else {
        0.0
    };

    if shift_x == 0.0 && shift_y == 0.0 {
        return;
    }

    for node in nodes.values_mut() {
        node.x += shift_x;
        node.y += shift_y;
    }
    for edge in edges.iter_mut() {
        for point in edge.points.iter_mut() {
            point.0 += shift_x;
            point.1 += shift_y;
        }
    }
    for sub in subgraphs.iter_mut() {
        sub.x += shift_x;
        sub.y += shift_y;
    }
}

struct RouteContext<'a> {
    from_id: &'a str,
    to_id: &'a str,
    from: &'a NodeLayout,
    to: &'a NodeLayout,
    direction: Direction,
    config: &'a LayoutConfig,
    obstacles: &'a [Obstacle],
    base_offset: f32,
    start_side: EdgeSide,
    end_side: EdgeSide,
    start_offset: f32,
    end_offset: f32,
}

fn apply_port_offset(point: (f32, f32), side: EdgeSide, offset: f32) -> (f32, f32) {
    match side {
        EdgeSide::Left | EdgeSide::Right => (point.0, point.1 + offset),
        EdgeSide::Top | EdgeSide::Bottom => (point.0 + offset, point.1),
    }
}

fn shape_polygon_points(node: &NodeLayout) -> Option<Vec<(f32, f32)>> {
    let x = node.x;
    let y = node.y;
    let w = node.width;
    let h = node.height;
    match node.shape {
        crate::ir::NodeShape::Rectangle
        | crate::ir::NodeShape::RoundRect
        | crate::ir::NodeShape::ActorBox
        | crate::ir::NodeShape::Stadium
        | crate::ir::NodeShape::Subroutine
        | crate::ir::NodeShape::Text => Some(vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h)]),
        crate::ir::NodeShape::Diamond => {
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            Some(vec![(cx, y), (x + w, cy), (cx, y + h), (x, cy)])
        }
        crate::ir::NodeShape::Hexagon => {
            let x1 = x + w * 0.25;
            let x2 = x + w * 0.75;
            let y_mid = y + h / 2.0;
            Some(vec![
                (x1, y),
                (x2, y),
                (x + w, y_mid),
                (x2, y + h),
                (x1, y + h),
                (x, y_mid),
            ])
        }
        crate::ir::NodeShape::Parallelogram | crate::ir::NodeShape::ParallelogramAlt => {
            let offset = w * 0.18;
            let points = if node.shape == crate::ir::NodeShape::Parallelogram {
                vec![
                    (x + offset, y),
                    (x + w, y),
                    (x + w - offset, y + h),
                    (x, y + h),
                ]
            } else {
                vec![
                    (x, y),
                    (x + w - offset, y),
                    (x + w, y + h),
                    (x + offset, y + h),
                ]
            };
            Some(points)
        }
        crate::ir::NodeShape::Trapezoid | crate::ir::NodeShape::TrapezoidAlt => {
            let offset = w * 0.18;
            let points = if node.shape == crate::ir::NodeShape::Trapezoid {
                vec![
                    (x + offset, y),
                    (x + w - offset, y),
                    (x + w, y + h),
                    (x, y + h),
                ]
            } else {
                vec![
                    (x, y),
                    (x + w, y),
                    (x + w - offset, y + h),
                    (x + offset, y + h),
                ]
            };
            Some(points)
        }
        crate::ir::NodeShape::Asymmetric => {
            let slant = w * 0.22;
            Some(vec![
                (x, y),
                (x + w - slant, y),
                (x + w, y + h / 2.0),
                (x + w - slant, y + h),
                (x, y + h),
            ])
        }
        _ => None,
    }
}

fn ray_polygon_intersection(
    origin: (f32, f32),
    dir: (f32, f32),
    poly: &[(f32, f32)],
) -> Option<(f32, f32)> {
    let mut best_t = None;
    let ox = origin.0;
    let oy = origin.1;
    let rx = dir.0;
    let ry = dir.1;
    if poly.len() < 2 {
        return None;
    }
    for i in 0..poly.len() {
        let (x1, y1) = poly[i];
        let (x2, y2) = poly[(i + 1) % poly.len()];
        let sx = x2 - x1;
        let sy = y2 - y1;
        let qx = x1 - ox;
        let qy = y1 - oy;
        let denom = rx * sy - ry * sx;
        if denom.abs() < 1e-6 {
            continue;
        }
        let t = (qx * sy - qy * sx) / denom;
        let u = (qx * ry - qy * rx) / denom;
        if t >= 0.0 && (0.0..=1.0).contains(&u) {
            match best_t {
                Some(best) if t >= best => {}
                _ => best_t = Some(t),
            }
        }
    }
    best_t.map(|t| (ox + rx * t, oy + ry * t))
}

fn ray_ellipse_intersection(
    origin: (f32, f32),
    dir: (f32, f32),
    center: (f32, f32),
    rx: f32,
    ry: f32,
) -> Option<(f32, f32)> {
    let (ox, oy) = origin;
    let (dx, dy) = dir;
    let (cx, cy) = center;
    let ox = ox - cx;
    let oy = oy - cy;
    let a = (dx * dx) / (rx * rx) + (dy * dy) / (ry * ry);
    let b = 2.0 * ((ox * dx) / (rx * rx) + (oy * dy) / (ry * ry));
    let c = (ox * ox) / (rx * rx) + (oy * oy) / (ry * ry) - 1.0;
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 || a.abs() < 1e-6 {
        return None;
    }
    let sqrt_disc = disc.sqrt();
    let t1 = (-b - sqrt_disc) / (2.0 * a);
    let t2 = (-b + sqrt_disc) / (2.0 * a);
    let t = if t1 >= 0.0 {
        t1
    } else if t2 >= 0.0 {
        t2
    } else {
        return None;
    };
    Some((origin.0 + dx * t, origin.1 + dy * t))
}

fn anchor_point_for_node(node: &NodeLayout, side: EdgeSide, offset: f32) -> (f32, f32) {
    let cx = node.x + node.width / 2.0;
    let cy = node.y + node.height / 2.0;
    let (dir, perp, max_offset) = match side {
        EdgeSide::Left => ((-1.0, 0.0), (0.0, 1.0), node.height / 2.0 - 1.0),
        EdgeSide::Right => ((1.0, 0.0), (0.0, 1.0), node.height / 2.0 - 1.0),
        EdgeSide::Top => ((0.0, -1.0), (1.0, 0.0), node.width / 2.0 - 1.0),
        EdgeSide::Bottom => ((0.0, 1.0), (1.0, 0.0), node.width / 2.0 - 1.0),
    };
    let clamp = if max_offset > 0.0 {
        offset.clamp(-max_offset, max_offset)
    } else {
        0.0
    };
    let origin = (cx + perp.0 * clamp, cy + perp.1 * clamp);

    match node.shape {
        crate::ir::NodeShape::Circle | crate::ir::NodeShape::DoubleCircle => {
            let rx = node.width / 2.0;
            let ry = node.height / 2.0;
            if let Some(point) = ray_ellipse_intersection(origin, dir, (cx, cy), rx, ry) {
                return point;
            }
        }
        _ => {}
    }

    if let Some(poly) = shape_polygon_points(node)
        && let Some(point) = ray_polygon_intersection(origin, dir, &poly)
    {
        return point;
    }

    // Fallback to bounding box anchor.
    let base = match side {
        EdgeSide::Left => (node.x, cy),
        EdgeSide::Right => (node.x + node.width, cy),
        EdgeSide::Top => (cx, node.y),
        EdgeSide::Bottom => (cx, node.y + node.height),
    };
    apply_port_offset(base, side, clamp)
}

fn route_edge_with_avoidance(ctx: &RouteContext<'_>) -> Vec<(f32, f32)> {
    if ctx.from_id == ctx.to_id {
        return route_self_loop(ctx.from, ctx.direction, ctx.config);
    }

    let (_, _, is_backward) = edge_sides(ctx.from, ctx.to, ctx.direction);

    let start = anchor_point_for_node(ctx.from, ctx.start_side, ctx.start_offset);
    let end = anchor_point_for_node(ctx.to, ctx.end_side, ctx.end_offset);

    // For backward edges, try routing around obstacles (both left and right)
    if is_backward {
        let pad = ctx.config.node_spacing.max(30.0);

        // Find the extents of any obstacle that blocks the direct path
        let mut min_left = f32::MAX;
        let mut max_right = 0.0f32;
        for obstacle in ctx.obstacles {
            if obstacle.id == ctx.from_id || obstacle.id == ctx.to_id {
                continue;
            }
            if let Some(members) = &obstacle.members
                && (members.contains(ctx.from_id) || members.contains(ctx.to_id))
            {
                continue;
            }
            // Check if obstacle vertically overlaps the edge path
            let obs_top = obstacle.y;
            let obs_bottom = obstacle.y + obstacle.height;
            let path_top = end.1;
            let path_bottom = start.1;
            if obs_top < path_bottom && obs_bottom > path_top {
                min_left = min_left.min(obstacle.x);
                max_right = max_right.max(obstacle.x + obstacle.width);
            }
        }

        // Try routing around the right side first
        if max_right > 0.0 {
            let route_x = max_right + pad;
            let points = vec![start, (route_x, start.1), (route_x, end.1), end];
            if !path_intersects_obstacles(&points, ctx.obstacles, ctx.from_id, ctx.to_id) {
                return points;
            }
        }

        // Try routing around the left side
        if min_left < f32::MAX {
            let route_x = min_left - pad;
            let points = vec![start, (route_x, start.1), (route_x, end.1), end];
            if !path_intersects_obstacles(&points, ctx.obstacles, ctx.from_id, ctx.to_id) {
                return points;
            }
        }
    }

    let step = ctx.config.node_spacing.max(16.0) * 0.6;
    let mut offsets = vec![ctx.base_offset];
    for i in 1..=4 {
        let delta = step * i as f32;
        offsets.push(ctx.base_offset + delta);
        offsets.push(ctx.base_offset - delta);
    }

    for offset in offsets {
        let points = if is_horizontal(ctx.direction) {
            let mid_x = (start.0 + end.0) / 2.0 + offset;
            vec![start, (mid_x, start.1), (mid_x, end.1), end]
        } else {
            let mid_y = (start.1 + end.1) / 2.0 + offset;
            vec![start, (start.0, mid_y), (end.0, mid_y), end]
        };

        if !path_intersects_obstacles(&points, ctx.obstacles, ctx.from_id, ctx.to_id) {
            return points;
        }
    }

    if is_horizontal(ctx.direction) {
        let mid_x = (start.0 + end.0) / 2.0;
        vec![start, (mid_x, start.1), (mid_x, end.1), end]
    } else {
        let mid_y = (start.1 + end.1) / 2.0;
        vec![start, (start.0, mid_y), (end.0, mid_y), end]
    }
}

fn route_self_loop(
    node: &NodeLayout,
    direction: Direction,
    config: &LayoutConfig,
) -> Vec<(f32, f32)> {
    let pad = config.node_spacing.max(20.0) * 0.6;
    if is_horizontal(direction) {
        let start = (node.x + node.width, node.y + node.height / 2.0);
        let p1 = (node.x + node.width + pad, node.y + node.height / 2.0);
        let p2 = (node.x + node.width + pad, node.y - pad);
        let p3 = (node.x + node.width / 2.0, node.y - pad);
        let end = (node.x + node.width / 2.0, node.y);
        vec![start, p1, p2, p3, end]
    } else {
        let start = (node.x + node.width / 2.0, node.y + node.height);
        let p1 = (node.x + node.width / 2.0, node.y + node.height + pad);
        let p2 = (node.x + node.width + pad, node.y + node.height + pad);
        let p3 = (node.x + node.width + pad, node.y + node.height / 2.0);
        let end = (node.x + node.width, node.y + node.height / 2.0);
        vec![start, p1, p2, p3, end]
    }
}

fn build_obstacles(
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
) -> Vec<Obstacle> {
    let mut obstacles = Vec::new();
    for node in nodes.values() {
        if node.hidden {
            continue;
        }
        if node.anchor_subgraph.is_some() {
            continue;
        }
        obstacles.push(Obstacle {
            id: node.id.clone(),
            x: node.x - 6.0,
            y: node.y - 6.0,
            width: node.width + 12.0,
            height: node.height + 12.0,
            members: None,
        });
    }

    for (idx, sub) in subgraphs.iter().enumerate() {
        let invisible_region = sub.label.trim().is_empty()
            && sub.style.stroke.as_deref() == Some("none")
            && sub.style.fill.as_deref() == Some("none");
        if invisible_region {
            continue;
        }
        let mut members: HashSet<String> = sub.nodes.iter().cloned().collect();
        for node in nodes.values() {
            if node.anchor_subgraph == Some(idx) {
                members.insert(node.id.clone());
            }
        }
        let pad = 6.0;
        obstacles.push(Obstacle {
            id: format!("subgraph:{}", sub.label),
            x: sub.x - pad,
            y: sub.y - pad,
            width: sub.width + pad * 2.0,
            height: sub.height + pad * 2.0,
            members: Some(members),
        });
    }
    obstacles
}

fn edge_pair_key(edge: &crate::ir::Edge) -> (String, String) {
    if edge.from <= edge.to {
        (edge.from.clone(), edge.to.clone())
    } else {
        (edge.to.clone(), edge.from.clone())
    }
}

fn build_edge_pair_counts(edges: &[crate::ir::Edge]) -> HashMap<(String, String), usize> {
    let mut counts: HashMap<(String, String), usize> = HashMap::new();
    for edge in edges {
        let key = edge_pair_key(edge);
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
}

fn path_intersects_obstacles(
    points: &[(f32, f32)],
    obstacles: &[Obstacle],
    from_id: &str,
    to_id: &str,
) -> bool {
    if points.len() < 2 {
        return false;
    }

    for segment in points.windows(2) {
        let (a, b) = (segment[0], segment[1]);
        for obstacle in obstacles {
            if obstacle.id == from_id || obstacle.id == to_id {
                continue;
            }
            if let Some(members) = &obstacle.members
                && (members.contains(from_id) || members.contains(to_id))
            {
                continue;
            }
            if segment_intersects_rect(a, b, obstacle) {
                return true;
            }
        }
    }
    false
}

fn segment_intersects_rect(a: (f32, f32), b: (f32, f32), rect: &Obstacle) -> bool {
    let (x1, y1) = a;
    let (x2, y2) = b;
    if (x1 - x2).abs() < f32::EPSILON {
        let x = x1;
        if x >= rect.x && x <= rect.x + rect.width {
            let min_y = y1.min(y2);
            let max_y = y1.max(y2);
            return max_y >= rect.y && min_y <= rect.y + rect.height;
        }
    } else if (y1 - y2).abs() < f32::EPSILON {
        let y = y1;
        if y >= rect.y && y <= rect.y + rect.height {
            let min_x = x1.min(x2);
            let max_x = x1.max(x2);
            return max_x >= rect.x && min_x <= rect.x + rect.width;
        }
    }
    false
}

fn measure_label(text: &str, theme: &Theme, config: &LayoutConfig) -> TextBlock {
    let raw_lines = split_lines(text);
    let mut lines = Vec::new();
    for line in raw_lines {
        let wrapped = wrap_line(&line, config.max_label_width_chars);
        lines.extend(wrapped);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    let max_len = lines.iter().map(|l| l.chars().count()).max().unwrap_or(1);
    let approx_char_width = theme.font_size * 0.45;
    let width = max_len as f32 * approx_char_width;
    let height = lines.len() as f32 * theme.font_size * config.label_line_height;

    TextBlock {
        lines,
        width,
        height,
    }
}

fn split_lines(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = text.replace("<br/>", "\n").replace("<br>", "\n");
    current = current.replace("\\n", "\n");
    for line in current.split('\n') {
        lines.push(line.trim().to_string());
    }
    lines
}

fn wrap_line(line: &str, max_chars: usize) -> Vec<String> {
    if line.chars().count() <= max_chars {
        return vec![line.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    for word in line.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current, word)
        };
        if candidate.chars().count() > max_chars {
            if !current.is_empty() {
                lines.push(current.clone());
                current.clear();
            }
            current.push_str(word);
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn resolve_node_style(node_id: &str, graph: &Graph) -> crate::ir::NodeStyle {
    let mut style = crate::ir::NodeStyle::default();

    if let Some(classes) = graph.node_classes.get(node_id) {
        for class_name in classes {
            if let Some(class_style) = graph.class_defs.get(class_name) {
                merge_node_style(&mut style, class_style);
            }
        }
    }

    if let Some(node_style) = graph.node_styles.get(node_id) {
        merge_node_style(&mut style, node_style);
    }

    style
}

fn resolve_subgraph_style(sub: &crate::ir::Subgraph, graph: &Graph) -> crate::ir::NodeStyle {
    let mut style = crate::ir::NodeStyle::default();
    let Some(id) = sub.id.as_ref() else {
        return style;
    };

    if let Some(classes) = graph.subgraph_classes.get(id) {
        for class_name in classes {
            if let Some(class_style) = graph.class_defs.get(class_name) {
                merge_node_style(&mut style, class_style);
            }
        }
    }

    if let Some(sub_style) = graph.subgraph_styles.get(id) {
        merge_node_style(&mut style, sub_style);
    }

    style
}

/// Separate sibling subgraphs that don't share nodes to avoid overlap
fn separate_sibling_subgraphs(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    if graph.subgraphs.len() < 2 {
        return;
    }

    // Build node sets for each subgraph
    let sets: Vec<HashSet<String>> = graph
        .subgraphs
        .iter()
        .map(|sub| sub.nodes.iter().cloned().collect())
        .collect();

    // Find pairs of sibling subgraphs (non-overlapping node sets)
    let mut sibling_groups: Vec<Vec<usize>> = Vec::new();
    let mut assigned: HashSet<usize> = HashSet::new();

    for i in 0..graph.subgraphs.len() {
        if assigned.contains(&i) {
            continue;
        }
        let mut group = vec![i];
        assigned.insert(i);

        for j in (i + 1)..graph.subgraphs.len() {
            if assigned.contains(&j) {
                continue;
            }
            // Check if j is a sibling (not nested with any in group)
            let j_set = &sets[j];
            let is_sibling = group.iter().all(|&k| {
                let k_set = &sets[k];
                // Neither is subset of the other
                !j_set.is_subset(k_set) && !k_set.is_subset(j_set)
            });
            if is_sibling {
                group.push(j);
                assigned.insert(j);
            }
        }
        if group.len() > 1 {
            sibling_groups.push(group);
        }
    }

    // For each group of siblings, compute bounds and separate them
    let is_horizontal = is_horizontal(graph.direction);
    for group in sibling_groups {
        // Compute bounding box for each subgraph
        let mut bounds: Vec<(usize, f32, f32, f32, f32)> = Vec::new(); // (idx, min_x, min_y, max_x, max_y)
        for &idx in &group {
            let sub = &graph.subgraphs[idx];
            let mut min_x = f32::MAX;
            let mut min_y = f32::MAX;
            let mut max_x = f32::MIN;
            let mut max_y = f32::MIN;
            for node_id in &sub.nodes {
                if let Some(node) = nodes.get(node_id) {
                    min_x = min_x.min(node.x);
                    min_y = min_y.min(node.y);
                    max_x = max_x.max(node.x + node.width);
                    max_y = max_y.max(node.y + node.height);
                }
            }
            if min_x != f32::MAX {
                bounds.push((idx, min_x, min_y, max_x, max_y));
            }
        }

        if bounds.len() < 2 {
            continue;
        }

        // Sort by position (x for LR, y for TD)
        if is_horizontal {
            bounds.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap()); // sort by min_y
        } else {
            bounds.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap()); // sort by min_x
        }

        // Shift subgraphs to avoid overlap
        let mut offset = 0.0_f32;
        for i in 0..bounds.len() {
            let (idx, min_x, min_y, max_x, max_y) = bounds[i];
            let sub = &graph.subgraphs[idx];

            // Apply offset to all nodes in this subgraph
            if offset > 0.0 {
                for node_id in &sub.nodes {
                    if let Some(node) = nodes.get_mut(node_id) {
                        if is_horizontal {
                            node.y += offset;
                        } else {
                            node.x += offset;
                        }
                    }
                }
            }

            // Calculate next offset based on this subgraph's extent
            let extent = if is_horizontal {
                max_y - min_y
            } else {
                max_x - min_x
            };
            let next_start = if is_horizontal { min_y } else { min_x };
            let current_end = next_start + extent + offset;
            offset = current_end + config.node_spacing * 2.0;
        }
    }
}

fn build_subgraph_layouts(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) -> Vec<SubgraphLayout> {
    let mut subgraphs = Vec::new();
    for sub in &graph.subgraphs {
        let is_region = is_region_subgraph(sub);
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for node_id in &sub.nodes {
            if let Some(node) = nodes.get(node_id) {
                min_x = min_x.min(node.x);
                min_y = min_y.min(node.y);
                max_x = max_x.max(node.x + node.width);
                max_y = max_y.max(node.y + node.height);
            }
        }

        if min_x == f32::MAX {
            continue;
        }

        let style = resolve_subgraph_style(sub, graph);
        let mut label_block = measure_label(&sub.label, theme, config);
        let base_padding = if graph.kind == crate::ir::DiagramKind::State {
            16.0
        } else {
            24.0
        };
        let padding = if is_region { 0.0 } else { base_padding };
        let label_empty = sub.label.trim().is_empty();
        if label_empty {
            label_block.width = 0.0;
            label_block.height = 0.0;
        }
        let label_height = if label_empty { 0.0 } else { label_block.height };
        let top_padding = if label_empty {
            padding
        } else if graph.kind == crate::ir::DiagramKind::State {
            (label_height + theme.font_size * 0.4).max(18.0)
        } else {
            padding + label_height + 8.0
        };

        subgraphs.push(SubgraphLayout {
            label: sub.label.clone(),
            label_block,
            nodes: sub.nodes.clone(),
            x: min_x - padding,
            y: min_y - top_padding,
            width: (max_x - min_x) + padding * 2.0,
            height: (max_y - min_y) + padding + top_padding,
            style,
        });
    }

    if subgraphs.len() > 1 {
        let sets: Vec<HashSet<String>> = graph
            .subgraphs
            .iter()
            .map(|sub| sub.nodes.iter().cloned().collect())
            .collect();

        let mut order: Vec<usize> = (0..subgraphs.len()).collect();
        order.sort_by_key(|i| sets[*i].len());

        for &i in &order {
            for &j in &order {
                if i == j {
                    continue;
                }
                if is_region_subgraph(&graph.subgraphs[j]) {
                    continue;
                }
                if sets[j].len() >= sets[i].len() {
                    continue;
                }
                if !sets[j].is_subset(&sets[i]) {
                    continue;
                }
                let pad = 12.0;
                let (child_x, child_y, child_w, child_h) = {
                    let child = &subgraphs[j];
                    (child.x, child.y, child.width, child.height)
                };
                let parent = &mut subgraphs[i];
                let min_x = parent.x.min(child_x - pad);
                let min_y = parent.y.min(child_y - pad);
                let max_x = (parent.x + parent.width).max(child_x + child_w + pad);
                let max_y = (parent.y + parent.height).max(child_y + child_h + pad);
                parent.x = min_x;
                parent.y = min_y;
                parent.width = max_x - min_x;
                parent.height = max_y - min_y;
            }
        }
    }

    subgraphs.sort_by(|a, b| {
        let area_a = a.width * a.height;
        let area_b = b.width * b.height;
        area_b.partial_cmp(&area_a).unwrap_or(Ordering::Equal)
    });
    subgraphs
}

fn merge_node_style(target: &mut crate::ir::NodeStyle, source: &crate::ir::NodeStyle) {
    if source.fill.is_some() {
        target.fill = source.fill.clone();
    }
    if source.stroke.is_some() {
        target.stroke = source.stroke.clone();
    }
    if source.text_color.is_some() {
        target.text_color = source.text_color.clone();
    }
    if source.stroke_width.is_some() {
        target.stroke_width = source.stroke_width;
    }
    if source.stroke_dasharray.is_some() {
        target.stroke_dasharray = source.stroke_dasharray.clone();
    }
}

fn shape_size(shape: crate::ir::NodeShape, label: &TextBlock, config: &LayoutConfig) -> (f32, f32) {
    let mut width = label.width + config.node_padding_x * 2.0;
    let mut height = label.height + config.node_padding_y * 2.0;
    let label_empty = label.lines.len() == 1 && label.lines[0].trim().is_empty();

    match shape {
        crate::ir::NodeShape::Diamond => {
            width *= 1.4;
            height *= 1.4;
        }
        crate::ir::NodeShape::ForkJoin => {
            width = width.max(50.0);
            height = (config.node_padding_y * 0.4).max(8.0);
        }
        crate::ir::NodeShape::Circle | crate::ir::NodeShape::DoubleCircle => {
            let size = if label_empty {
                (config.node_padding_y * 1.4).max(14.0)
            } else {
                width.max(height)
            };
            width = size;
            height = size;
        }
        crate::ir::NodeShape::Stadium | crate::ir::NodeShape::RoundRect => {
            width *= 1.1;
            height *= 1.05;
        }
        crate::ir::NodeShape::Cylinder => {
            width *= 1.1;
            height *= 1.1;
        }
        crate::ir::NodeShape::Hexagon => {
            width *= 1.2;
            height *= 1.1;
        }
        crate::ir::NodeShape::Parallelogram
        | crate::ir::NodeShape::ParallelogramAlt
        | crate::ir::NodeShape::Trapezoid
        | crate::ir::NodeShape::TrapezoidAlt
        | crate::ir::NodeShape::Asymmetric => {
            width *= 1.2;
        }
        crate::ir::NodeShape::Subroutine => {
            width *= 1.1;
        }
        _ => {}
    }

    (width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Direction, Graph, NodeShape};

    #[test]
    fn wraps_long_labels() {
        let theme = Theme::modern();
        let mut config = LayoutConfig::default();
        config.max_label_width_chars = 8;
        let block = measure_label("this is a long label", &theme, &config);
        assert!(block.lines.len() > 1);
    }

    #[test]
    fn layout_places_nodes() {
        let mut graph = Graph::new();
        graph.direction = Direction::LeftRight;
        graph.ensure_node("A", Some("Alpha".to_string()), Some(NodeShape::Rectangle));
        graph.ensure_node("B", Some("Beta".to_string()), Some(NodeShape::Rectangle));
        graph.edges.push(crate::ir::Edge {
            from: "A".to_string(),
            to: "B".to_string(),
            label: None,
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
        let a = layout.nodes.get("A").unwrap();
        let b = layout.nodes.get("B").unwrap();
        assert!(b.x >= a.x);
    }

    #[test]
    fn edge_style_merges_default_and_override() {
        let mut graph = Graph::new();
        graph.ensure_node("A", Some("Alpha".to_string()), Some(NodeShape::Rectangle));
        graph.ensure_node("B", Some("Beta".to_string()), Some(NodeShape::Rectangle));
        graph.edges.push(crate::ir::Edge {
            from: "A".to_string(),
            to: "B".to_string(),
            label: None,
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

        graph.edge_style_default = Some(crate::ir::EdgeStyleOverride {
            stroke: Some("#111111".to_string()),
            stroke_width: None,
            dasharray: None,
            label_color: Some("#222222".to_string()),
        });
        graph.edge_styles.insert(
            0,
            crate::ir::EdgeStyleOverride {
                stroke: None,
                stroke_width: Some(4.0),
                dasharray: None,
                label_color: None,
            },
        );

        let layout = compute_layout(&graph, &Theme::modern(), &LayoutConfig::default());
        let edge = &layout.edges[0];
        assert_eq!(edge.override_style.stroke.as_deref(), Some("#111111"));
        assert_eq!(edge.override_style.stroke_width, Some(4.0));
        assert_eq!(edge.override_style.label_color.as_deref(), Some("#222222"));
    }
}
