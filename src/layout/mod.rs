mod architecture;
mod block;
mod c4;
mod error;
mod flowchart;
mod gantt;
mod gitgraph;
mod invariants;
mod journey;
mod kanban;
pub(crate) mod label_placement;
mod mindmap;
mod pie;
mod quadrant;
mod radar;
mod ranking;
pub(crate) mod routing;
mod sankey;
mod sequence;
mod subgraphs;
mod text;
mod timeline;
mod treemap;
pub(crate) mod types;
mod xychart;
use architecture::*;
use block::*;
use c4::*;
use error::*;
use gantt::*;
use gitgraph::*;
pub use invariants::{LayoutInvariantError, validate_layout_invariants};
use journey::*;
use kanban::*;
use mindmap::*;
use pie::*;
use quadrant::*;
use radar::*;
use routing::*;
use sankey::*;
use sequence::*;
use subgraphs::*;
use text::*;
use timeline::*;
use treemap::*;
pub use types::*;
use xychart::*;

use crate::config::{LayoutConfig, PieRenderMode, TreemapRenderMode};
use crate::ir::{Direction, Graph};
use crate::text_metrics;
use crate::theme::{Theme, adjust_color, parse_color_to_hsl};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use crate::time::Instant;

// Label placement padding (resolved per diagram kind).
// Minimum padding around the entire layout bounding box.
const LAYOUT_BOUNDARY_PAD: f32 = 8.0;
const PREFERRED_ASPECT_TOLERANCE: f32 = 0.02;
const PREFERRED_ASPECT_MAX_EXPANSION: f32 = 24.0;
const PREFERRED_ASPECT_MAX_PASSES: usize = 6;

// ── State diagram constants ───────────────────────────────────────────
const STATE_MARKER_FONT_SCALE: f32 = 0.75;
const STATE_MARKER_MIN_SIZE: f32 = 10.0;
const STATE_DEFAULT_HEIGHT_SCALE: f32 = 2.4;
const STATE_MARKER_DIV: f32 = 3.0;
const STATE_MARKER_MIN_SCALE: f32 = 0.5;
const STATE_MARKER_MAX_SCALE: f32 = 0.95;
const STATE_NOTE_PAD_X_SCALE: f32 = 0.75;
const STATE_NOTE_PAD_Y_SCALE: f32 = 0.5;
const STATE_NOTE_GAP_SCALE: f32 = 0.9;
const STATE_NOTE_GAP_MIN: f32 = 10.0;
const STATE_PAD_X_SCALE: f32 = 0.9;
const STATE_PAD_Y_SCALE: f32 = 0.65;
const STATE_PAD_X_LABEL_RATIO: f32 = 0.12;
const STATE_PAD_Y_LABEL_RATIO: f32 = 0.22;

// ── Subgraph padding ─────────────────────────────────────────────────
const FLOWCHART_PAD_MAIN: f32 = 40.0;
const FLOWCHART_PAD_CROSS: f32 = 30.0;
const FLOWCHART_PORT_ROUTE_BIAS_RATIO: f32 = 0.5;
const FLOWCHART_PORT_ROUTE_BIAS_MAX_RATIO: f32 = 0.8;
const KANBAN_SUBGRAPH_PAD: f32 = 8.0;
const STATE_SUBGRAPH_BASE_PAD: f32 = 16.0;
const GENERIC_SUBGRAPH_BASE_PAD: f32 = 24.0;
const SUBGRAPH_LABEL_GAP_FLOWCHART: f32 = 6.0;
const SUBGRAPH_LABEL_GAP_KANBAN: f32 = 4.0;
const SUBGRAPH_LABEL_GAP_GENERIC: f32 = 8.0;
const STATE_SUBGRAPH_TOP_LABEL_SCALE: f32 = 0.75;
const STATE_SUBGRAPH_TOP_MIN_SCALE: f32 = 1.4;

// ── Shape size constants ─────────────────────────────────────────────
const DIAMOND_SCALE: f32 = 0.95;
const FORK_JOIN_MIN_WIDTH: f32 = 50.0;
const FORK_JOIN_HEIGHT_SCALE: f32 = 0.4;
const FORK_JOIN_MIN_HEIGHT: f32 = 8.0;
const CIRCLE_EMPTY_HEIGHT_SCALE: f32 = 1.4;
const CIRCLE_EMPTY_MIN_SIZE: f32 = 14.0;
const ROUND_RECT_WIDTH_SCALE: f32 = 1.1;
const ROUND_RECT_HEIGHT_SCALE: f32 = 1.05;
const CYLINDER_SCALE: f32 = 1.1;
const HEXAGON_WIDTH_SCALE: f32 = 1.2;
const HEXAGON_HEIGHT_SCALE: f32 = 1.1;
const TRAPEZOID_WIDTH_SCALE: f32 = 1.2;
const CLASS_MIN_HEIGHT_SCALE: f32 = 6.5;
const REQUIREMENT_MIN_WIDTH_SCALE: f32 = 9.5;
const KANBAN_MIN_WIDTH_SCALE: f32 = 11.0;
const KANBAN_MIN_HEIGHT_SCALE: f32 = 2.6;

// ── Edge label relaxation constants ──────────────────────────────────
const EDGE_LABEL_PAD_SCALE: f32 = 0.35;
const ENDPOINT_LABEL_PAD_SCALE: f32 = 0.2;
const DUAL_ENDPOINT_EXTRA_PAD_SCALE: f32 = 0.45;
const EDGE_RELAX_STEP_MIN: f32 = 24.0;
const EDGE_RELAX_GAP_TOLERANCE: f32 = 0.5;
const MAX_MAIN_GAP_FACTOR: f32 = 6.0;
const FLOWCHART_EDGE_LABEL_WRAP_TRIGGER_CHARS: usize = 34;
const FLOWCHART_EDGE_LABEL_WRAP_MAX_CHARS: usize = 18;

// ── Overlap resolution ───────────────────────────────────────────────
const OVERLAP_RESOLVE_PASSES: u32 = 6;
const OVERLAP_MIN_GAP_RATIO: f32 = 0.2;
const OVERLAP_MIN_GAP_FLOOR: f32 = 4.0;
const OVERLAP_CENTER_THRESHOLD: f32 = 0.5;

// ── Subgraph gap enforcement ─────────────────────────────────────────
const SUBGRAPH_DESIRED_GAP_RATIO: f32 = 1.6;

// ── Edge occupancy / multi-edge offset ───────────────────────────────
const MIN_NODE_SPACING_FLOOR: f32 = 16.0;
const EDGE_OCCUPANCY_CELL_RATIO: f32 = 0.6;
const MULTI_EDGE_OFFSET_RATIO: f32 = 0.35;

// ── State subgraph rank spacing boost ────────────────────────────────
const STATE_RANK_SPACING_BOOST: f32 = 25.0;

#[derive(Debug, Clone, Default)]
pub struct LayoutStageMetrics {
    pub port_assignment_us: u128,
    pub edge_routing_us: u128,
    pub label_placement_us: u128,
}

impl LayoutStageMetrics {
    pub fn total_us(&self) -> u128 {
        self.port_assignment_us + self.edge_routing_us + self.label_placement_us
    }
}

pub fn compute_layout(graph: &Graph, theme: &Theme, config: &LayoutConfig) -> Layout {
    compute_layout_with_metrics(graph, theme, config).0
}

pub fn compute_layout_with_metrics(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> (Layout, LayoutStageMetrics) {
    let graph = normalize_graph_for_layout(graph);
    let graph = graph.as_ref();
    let mut stage_metrics = LayoutStageMetrics::default();
    let mut layout = match graph.kind {
        crate::ir::DiagramKind::Sequence | crate::ir::DiagramKind::ZenUML => {
            compute_sequence_layout(graph, theme, config)
        }
        crate::ir::DiagramKind::Pie => {
            if config.pie.render_mode == PieRenderMode::Error {
                compute_pie_error_layout(graph, config)
            } else {
                compute_pie_layout(graph, theme, config)
            }
        }
        crate::ir::DiagramKind::Quadrant => compute_quadrant_layout(graph, theme, config),
        crate::ir::DiagramKind::Gantt => compute_gantt_layout(graph, theme, config),
        crate::ir::DiagramKind::Kanban => {
            compute_kanban_layout(graph, theme, config, Some(&mut stage_metrics))
        }
        crate::ir::DiagramKind::Block => compute_block_layout(graph, theme, config),
        crate::ir::DiagramKind::Sankey => compute_sankey_layout(graph, theme, config),
        crate::ir::DiagramKind::Architecture => compute_architecture_layout(graph, theme, config),
        crate::ir::DiagramKind::Radar => compute_radar_layout(graph, theme, config),
        crate::ir::DiagramKind::Treemap => {
            if config.treemap.render_mode == TreemapRenderMode::Error {
                compute_error_layout(graph, config)
            } else {
                compute_treemap_layout(graph, theme, config)
            }
        }
        crate::ir::DiagramKind::GitGraph => compute_gitgraph_layout(graph, theme, config),
        crate::ir::DiagramKind::C4 => compute_c4_layout(graph, config),
        crate::ir::DiagramKind::Mindmap => compute_mindmap_layout(graph, theme, config),
        crate::ir::DiagramKind::XYChart => compute_xychart_layout(graph, theme, config),
        crate::ir::DiagramKind::Timeline => compute_timeline_layout(graph, theme, config),
        crate::ir::DiagramKind::Journey => compute_journey_layout(graph, theme, config),
        crate::ir::DiagramKind::Class
        | crate::ir::DiagramKind::State
        | crate::ir::DiagramKind::Er
        | crate::ir::DiagramKind::Requirement
        | crate::ir::DiagramKind::Packet
        | crate::ir::DiagramKind::Flowchart => {
            compute_flowchart_layout(graph, theme, config, Some(&mut stage_metrics))
        }
    };

    apply_preferred_aspect_ratio_layout(&mut layout, config);

    // Final pass: resolve all edge label positions using collision avoidance.
    let label_start = Instant::now();
    label_placement::resolve_all_label_positions(&mut layout, theme, config);
    if matches!(layout.diagram, DiagramData::Sequence(_)) {
        sequence::finalize_sequence_layout_bounds(&mut layout);
    } else if matches!(layout.diagram, DiagramData::Graph { .. }) {
        finalize_graph_label_bounds(&mut layout, config);
    }
    stage_metrics.label_placement_us = stage_metrics
        .label_placement_us
        .saturating_add(label_start.elapsed().as_micros());

    (layout, stage_metrics)
}

fn normalize_graph_for_layout(graph: &Graph) -> Cow<'_, Graph> {
    let needs_edge_nodes = graph
        .edges
        .iter()
        .any(|edge| !graph.nodes.contains_key(&edge.from) || !graph.nodes.contains_key(&edge.to));
    let needs_sequence_nodes = matches!(
        graph.kind,
        crate::ir::DiagramKind::Sequence | crate::ir::DiagramKind::ZenUML
    ) && graph
        .sequence_participants
        .iter()
        .any(|id| !graph.nodes.contains_key(id));

    if !needs_edge_nodes && !needs_sequence_nodes {
        return Cow::Borrowed(graph);
    }

    let mut normalized = graph.clone();
    for edge in &graph.edges {
        normalized.ensure_node(&edge.from, None, None);
        normalized.ensure_node(&edge.to, None, None);
    }
    if matches!(
        graph.kind,
        crate::ir::DiagramKind::Sequence | crate::ir::DiagramKind::ZenUML
    ) {
        for id in &graph.sequence_participants {
            normalized.ensure_node(id, None, None);
        }
    }
    Cow::Owned(normalized)
}

fn include_rect_bounds(
    min_x: &mut f32,
    min_y: &mut f32,
    max_x: &mut f32,
    max_y: &mut f32,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) {
    *min_x = (*min_x).min(x);
    *min_y = (*min_y).min(y);
    *max_x = (*max_x).max(x + width);
    *max_y = (*max_y).max(y + height);
}

fn translate_graph_layout(layout: &mut Layout, dx: f32, dy: f32) {
    if dx == 0.0 && dy == 0.0 {
        return;
    }
    for node in layout.nodes.values_mut() {
        node.x += dx;
        node.y += dy;
    }
    for subgraph in &mut layout.subgraphs {
        subgraph.x += dx;
        subgraph.y += dy;
    }
    for edge in &mut layout.edges {
        for point in &mut edge.points {
            point.0 += dx;
            point.1 += dy;
        }
        if let Some(anchor) = &mut edge.label_anchor {
            anchor.0 += dx;
            anchor.1 += dy;
        }
        if let Some(anchor) = &mut edge.start_label_anchor {
            anchor.0 += dx;
            anchor.1 += dy;
        }
        if let Some(anchor) = &mut edge.end_label_anchor {
            anchor.0 += dx;
            anchor.1 += dy;
        }
    }
    if let DiagramData::Graph { state_notes } = &mut layout.diagram {
        for note in state_notes {
            note.x += dx;
            note.y += dy;
        }
    }
}

fn finalize_graph_label_bounds(layout: &mut Layout, config: &LayoutConfig) {
    let mut min_x = 0.0f32;
    let mut min_y = 0.0f32;
    let mut max_x = layout.width;
    let mut max_y = layout.height;
    let (label_pad_x, label_pad_y) = label_placement::edge_label_padding(layout.kind, config);

    for node in layout.nodes.values() {
        include_rect_bounds(
            &mut min_x,
            &mut min_y,
            &mut max_x,
            &mut max_y,
            node.x,
            node.y,
            node.width,
            node.height,
        );
    }
    for subgraph in &layout.subgraphs {
        include_rect_bounds(
            &mut min_x,
            &mut min_y,
            &mut max_x,
            &mut max_y,
            subgraph.x,
            subgraph.y,
            subgraph.width,
            subgraph.height,
        );
    }
    for edge in &layout.edges {
        for point in &edge.points {
            min_x = min_x.min(point.0);
            min_y = min_y.min(point.1);
            max_x = max_x.max(point.0);
            max_y = max_y.max(point.1);
        }
        if let (Some(label), Some((x, y))) = (&edge.label, edge.label_anchor) {
            include_rect_bounds(
                &mut min_x,
                &mut min_y,
                &mut max_x,
                &mut max_y,
                x - label.width / 2.0 - label_pad_x,
                y - label.height / 2.0 - label_pad_y,
                label.width + label_pad_x * 2.0,
                label.height + label_pad_y * 2.0,
            );
        }
        if let (Some(label), Some((x, y))) = (&edge.start_label, edge.start_label_anchor) {
            include_rect_bounds(
                &mut min_x,
                &mut min_y,
                &mut max_x,
                &mut max_y,
                x - label.width / 2.0 - label_pad_x,
                y - label.height / 2.0 - label_pad_y,
                label.width + label_pad_x * 2.0,
                label.height + label_pad_y * 2.0,
            );
        }
        if let (Some(label), Some((x, y))) = (&edge.end_label, edge.end_label_anchor) {
            include_rect_bounds(
                &mut min_x,
                &mut min_y,
                &mut max_x,
                &mut max_y,
                x - label.width / 2.0 - label_pad_x,
                y - label.height / 2.0 - label_pad_y,
                label.width + label_pad_x * 2.0,
                label.height + label_pad_y * 2.0,
            );
        }
    }
    if let DiagramData::Graph { state_notes } = &layout.diagram {
        for note in state_notes {
            include_rect_bounds(
                &mut min_x,
                &mut min_y,
                &mut max_x,
                &mut max_y,
                note.x,
                note.y,
                note.width,
                note.height,
            );
        }
    }

    let dx = if min_x < 0.0 {
        -min_x + LAYOUT_BOUNDARY_PAD
    } else {
        0.0
    };
    let dy = if min_y < 0.0 {
        -min_y + LAYOUT_BOUNDARY_PAD
    } else {
        0.0
    };
    translate_graph_layout(layout, dx, dy);
    layout.width = (max_x + dx + LAYOUT_BOUNDARY_PAD).max(layout.width + dx);
    layout.height = (max_y + dy + LAYOUT_BOUNDARY_PAD).max(layout.height + dy);
}

fn compute_flowchart_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
    stage_metrics: Option<&mut LayoutStageMetrics>,
) -> Layout {
    let mut effective_config = flowchart::policy::apply_initial_config_heuristics(graph, config);
    let tiny_graph = flowchart::policy::is_tiny_graph_layout(graph);
    let mut nodes = BTreeMap::new();
    let measure_font_size = theme.font_size;
    let mut label_config = effective_config.clone();
    if graph.kind == crate::ir::DiagramKind::Class {
        label_config.label_line_height = label_config.class_label_line_height();
    }
    let mut state_marker_ids: Vec<String> = Vec::new();
    let mut state_height_total = 0.0f32;
    let mut state_height_count = 0usize;

    for node in graph.nodes.values() {
        let label = measure_label_with_font_size(
            &node.label,
            measure_font_size,
            &label_config,
            true,
            theme.font_family.as_str(),
        );
        let label_empty = label.lines.len() == 1 && label.lines[0].trim().is_empty();
        let (mut width, mut height) =
            shape_size(node.shape, &label, &effective_config, theme, graph.kind);
        if graph.kind == crate::ir::DiagramKind::State
            && label_empty
            && matches!(
                node.shape,
                crate::ir::NodeShape::Circle | crate::ir::NodeShape::DoubleCircle
            )
        {
            let size = (theme.font_size * STATE_MARKER_FONT_SCALE).max(STATE_MARKER_MIN_SIZE);
            width = size;
            height = size;
            state_marker_ids.push(node.id.clone());
        } else if graph.kind == crate::ir::DiagramKind::State {
            state_height_total += height;
            state_height_count += 1;
        }
        let mut style = resolve_node_style(node.id.as_str(), graph);
        if graph.kind == crate::ir::DiagramKind::State
            && node.shape == crate::ir::NodeShape::ForkJoin
            && label_empty
        {
            // Fork/join bars in state diagrams should render as solid markers.
            if style.fill.is_none() {
                style.fill = Some(theme.line_color.clone());
            }
            if style.stroke.is_none() {
                style.stroke = Some(theme.line_color.clone());
            }
            if style.stroke_width.is_none() {
                style.stroke_width = Some(1.0);
            }
        }
        nodes.insert(
            node.id.clone(),
            build_node_layout(node, label, width, height, style, graph),
        );
    }

    if graph.kind == crate::ir::DiagramKind::State && !state_marker_ids.is_empty() {
        let avg_height = if state_height_count > 0 {
            state_height_total / state_height_count as f32
        } else {
            theme.font_size * STATE_DEFAULT_HEIGHT_SCALE
        };
        let marker_size = (avg_height / STATE_MARKER_DIV).clamp(
            theme.font_size * STATE_MARKER_MIN_SCALE,
            theme.font_size * STATE_MARKER_MAX_SCALE,
        );
        for id in state_marker_ids {
            if let Some(node) = nodes.get_mut(&id) {
                node.width = marker_size;
                node.height = marker_size;
            }
        }
    }

    flowchart::policy::apply_measured_spacing_heuristics(
        graph,
        theme,
        &mut effective_config,
        &nodes,
    );

    let config = &effective_config;

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
    layout_node_ids.sort_by(|a, b| {
        graph
            .node_order
            .get(a)
            .copied()
            .unwrap_or(usize::MAX)
            .cmp(&graph.node_order.get(b).copied().unwrap_or(usize::MAX))
            .then_with(|| a.cmp(b))
    });
    if !anchored_subgraph_nodes.is_empty() {
        layout_node_ids.retain(|id| !anchored_subgraph_nodes.contains(id));
    }
    let mut layout_set: HashSet<String> = layout_node_ids.iter().cloned().collect();

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

    // Pre-measure all edge labels once (reused across layout, routing, and edge construction).
    let measure_edge_field = |field: &Option<String>| -> Option<TextBlock> {
        field.as_ref().map(|label| {
            let label_text = if graph.kind == crate::ir::DiagramKind::Requirement {
                requirement_edge_label_text(label, config)
            } else {
                label.clone()
            };
            if graph.kind == crate::ir::DiagramKind::Flowchart
                && !label_text.contains('\n')
                && !label_text.contains("<br")
                && label_text.chars().count() >= FLOWCHART_EDGE_LABEL_WRAP_TRIGGER_CHARS
            {
                // Keep very long flowchart edge labels in a narrower block so
                // routing + placement can avoid large cross-label collisions.
                let mut wrap_cfg = config.clone();
                wrap_cfg.max_label_width_chars = wrap_cfg
                    .max_label_width_chars
                    .min(FLOWCHART_EDGE_LABEL_WRAP_MAX_CHARS);
                measure_label_with_font_size(
                    &label_text,
                    theme.font_size.max(16.0),
                    &wrap_cfg,
                    true,
                    theme.font_family.as_str(),
                )
            } else {
                measure_label(&label_text, theme, config)
            }
        })
    };
    let edge_route_labels: Vec<Option<TextBlock>> = graph
        .edges
        .iter()
        .map(|e| measure_edge_field(&e.label))
        .collect();
    let edge_start_labels: Vec<Option<TextBlock>> = graph
        .edges
        .iter()
        .map(|e| measure_edge_field(&e.start_label))
        .collect();
    let edge_end_labels: Vec<Option<TextBlock>> = graph
        .edges
        .iter()
        .map(|e| measure_edge_field(&e.end_label))
        .collect();

    let mut label_dummy_ids: Vec<Option<String>> = vec![None; graph.edges.len()];
    flowchart::manual_layout::assign_positions_manual(
        graph,
        &layout_node_ids,
        &layout_set,
        &mut nodes,
        config,
        &layout_edges,
        theme,
        &edge_route_labels,
        &mut label_dummy_ids,
    );

    apply_subgraph_node_layout_passes(graph, &mut nodes, config, &anchored_indices, &anchor_info);

    flowchart::subgraph_spacing::apply_flowchart_node_layout_cleanup(
        graph, &mut nodes, theme, config,
    );
    flowchart::objectives::apply_visual_objectives(
        graph,
        &layout_edges,
        &mut nodes,
        theme,
        &effective_config,
    );
    apply_subgraph_direction_overrides(graph, &mut nodes, config, &anchored_indices);
    // Objective and direction passes can shift whole top-level groups after the
    // initial subgraph cleanup. Re-apply the non-overlap spacing constraints
    // before subgraph boxes are materialized so dense cross-subgraph flowcharts
    // keep sibling regions visually distinct.
    flowchart::subgraph_spacing::enforce_top_level_subgraph_gap(graph, &mut nodes, theme, config);
    flowchart::subgraph_spacing::separate_sibling_subgraphs(graph, &mut nodes, theme, config);
    flowchart::subgraph_spacing::debug_assert_flowchart_node_layout_invariants(graph, &nodes);

    // For state diagrams, push non-member nodes outside subgraph bounds
    if graph.kind == crate::ir::DiagramKind::State && !graph.subgraphs.is_empty() {
        push_non_members_out_of_subgraphs(graph, &mut nodes, theme, config);
    }

    let subgraphs = build_subgraph_layouts(graph, &nodes, theme, config);
    apply_subgraph_anchors(graph, &subgraphs, &mut nodes);

    let edges = flowchart::edge_pipeline::build_routed_edges(
        flowchart::edge_pipeline::RoutedEdgeBuildContext {
            graph,
            nodes: &nodes,
            subgraphs: &subgraphs,
            config,
            layout_node_count: layout_node_ids.len(),
            edge_route_labels: &edge_route_labels,
            edge_start_labels: &edge_start_labels,
            edge_end_labels: &edge_end_labels,
            label_dummy_ids: &label_dummy_ids,
            tiny_graph,
            stage_metrics,
        },
    );

    flowchart::finalize::finalize_graph_layout(graph, nodes, edges, subgraphs, theme, config)
}

pub(in crate::layout) fn resolve_edge_style(
    idx: usize,
    graph: &Graph,
) -> crate::ir::EdgeStyleOverride {
    let mut style = graph.edge_style_default.clone().unwrap_or_default();
    if let Some(edge_style) = graph.edge_styles.get(&idx) {
        merge_edge_style(&mut style, edge_style);
    }
    style
}

fn sanitize_stroke_width(width: f32) -> Option<f32> {
    width.is_finite().then_some(width.max(0.0))
}

fn merge_edge_style(
    target: &mut crate::ir::EdgeStyleOverride,
    source: &crate::ir::EdgeStyleOverride,
) {
    if source.stroke.is_some() {
        target.stroke = source.stroke.clone();
    }
    if let Some(width) = source.stroke_width.and_then(sanitize_stroke_width) {
        target.stroke_width = Some(width);
    }
    if source.dasharray.is_some() {
        target.dasharray = source.dasharray.clone();
    }
    if source.label_color.is_some() {
        target.label_color = source.label_color.clone();
    }
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
    let node_order: HashMap<&str, usize> = node_ids
        .iter()
        .enumerate()
        .map(|(idx, node_id)| (node_id.as_str(), idx))
        .collect();
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
        bucket.sort_by_key(|id| node_order.get(id.as_str()).copied().unwrap_or(usize::MAX));
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

fn bounds_without_padding(
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
) -> (f32, f32) {
    bounds_with_edges(nodes, subgraphs, &[])
}

pub(in crate::layout) fn bounds_with_edges(
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
    edges: &[EdgeLayout],
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
    // Also include edge points - routing can place waypoints outside node bounds
    for edge in edges {
        for point in &edge.points {
            max_x = max_x.max(point.0);
            max_y = max_y.max(point.1);
        }
    }
    (max_x, max_y)
}

fn apply_preferred_aspect_ratio_layout(layout: &mut Layout, config: &LayoutConfig) {
    let Some(target_ratio) = config
        .preferred_aspect_ratio
        .filter(|ratio| ratio.is_finite() && *ratio > 0.0)
    else {
        return;
    };
    if !matches!(layout.diagram, DiagramData::Graph { .. }) {
        return;
    }

    let mut total_scale_x = 1.0f32;
    let mut total_scale_y = 1.0f32;
    for _ in 0..PREFERRED_ASPECT_MAX_PASSES {
        let (width, height) = graph_layout_dimensions(layout);
        let current_ratio = width / height;
        if (current_ratio - target_ratio).abs() <= PREFERRED_ASPECT_TOLERANCE {
            break;
        }

        let mut scale_x = 1.0f32;
        let mut scale_y = 1.0f32;
        if current_ratio < target_ratio {
            let remaining = (PREFERRED_ASPECT_MAX_EXPANSION / total_scale_x).max(1.0);
            scale_x = (target_ratio / current_ratio).min(remaining);
        } else {
            let remaining = (PREFERRED_ASPECT_MAX_EXPANSION / total_scale_y).max(1.0);
            scale_y = (current_ratio / target_ratio).min(remaining);
        }
        if (scale_x - 1.0).abs() <= 1e-3 && (scale_y - 1.0).abs() <= 1e-3 {
            break;
        }

        scale_graph_geometry(layout, scale_x, scale_y);
        total_scale_x *= scale_x;
        total_scale_y *= scale_y;
    }

    let (width, height) = graph_layout_dimensions(layout);
    layout.width = width;
    layout.height = height;
}

fn scale_graph_geometry(layout: &mut Layout, scale_x: f32, scale_y: f32) {
    for node in layout.nodes.values_mut() {
        node.x *= scale_x;
        node.y *= scale_y;
    }
    for edge in &mut layout.edges {
        for point in &mut edge.points {
            point.0 *= scale_x;
            point.1 *= scale_y;
        }
        if let Some(anchor) = edge.label_anchor.as_mut() {
            anchor.0 *= scale_x;
            anchor.1 *= scale_y;
        }
        if let Some(anchor) = edge.start_label_anchor.as_mut() {
            anchor.0 *= scale_x;
            anchor.1 *= scale_y;
        }
        if let Some(anchor) = edge.end_label_anchor.as_mut() {
            anchor.0 *= scale_x;
            anchor.1 *= scale_y;
        }
    }
    for sub in &mut layout.subgraphs {
        sub.x *= scale_x;
        sub.y *= scale_y;
        sub.width *= scale_x;
        sub.height *= scale_y;
    }
    if let DiagramData::Graph { state_notes } = &mut layout.diagram {
        for note in state_notes {
            note.x *= scale_x;
            note.y *= scale_y;
        }
    }
}

fn graph_layout_dimensions(layout: &Layout) -> (f32, f32) {
    let (mut max_x, mut max_y) = bounds_with_edges(&layout.nodes, &layout.subgraphs, &layout.edges);
    if let DiagramData::Graph { state_notes } = &layout.diagram {
        for note in state_notes {
            max_x = max_x.max(note.x + note.width);
            max_y = max_y.max(note.y + note.height);
        }
    }
    (
        (max_x + LAYOUT_BOUNDARY_PAD).max(1.0),
        (max_y + LAYOUT_BOUNDARY_PAD).max(1.0),
    )
}

pub(in crate::layout) fn apply_direction_mirror(
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
            if let Some(anchor) = edge.label_anchor.as_mut() {
                anchor.0 = max_x - anchor.0;
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
            if let Some(anchor) = edge.label_anchor.as_mut() {
                anchor.1 = max_y - anchor.1;
            }
        }
        for sub in subgraphs.iter_mut() {
            sub.y = max_y - sub.y - sub.height;
        }
    }
}

pub(in crate::layout) fn normalize_layout(
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
    // Also check edge points - routing can place waypoints outside node bounds
    for edge in edges.iter() {
        for point in &edge.points {
            min_x = min_x.min(point.0);
            min_y = min_y.min(point.1);
        }
    }

    if !min_x.is_finite() || !min_y.is_finite() {
        return;
    }
    let padding = LAYOUT_BOUNDARY_PAD;
    let shift_x = padding - min_x;
    let shift_y = padding - min_y;

    if shift_x.abs() < 1e-3 && shift_y.abs() < 1e-3 {
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
        if let Some(anchor) = edge.label_anchor.as_mut() {
            anchor.0 += shift_x;
            anchor.1 += shift_y;
        }
    }
    for sub in subgraphs.iter_mut() {
        sub.x += shift_x;
        sub.y += shift_y;
    }
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

/// Build a `NodeLayout` with the standard defaults (position at origin, no
/// anchor, not hidden, no icon).  Callers that need custom x/y or
/// width/height can mutate the returned value.
fn build_node_layout(
    node: &crate::ir::Node,
    label: TextBlock,
    width: f32,
    height: f32,
    style: crate::ir::NodeStyle,
    graph: &Graph,
) -> NodeLayout {
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
        icon: None,
    }
}

/// Build `NodeLayout`s for every node in `graph` using the standard pipeline:
/// `measure_label → shape_size → resolve_node_style → NodeLayout`.
///
/// Returns a `BTreeMap` ready to assign into a `Layout`.
fn build_graph_node_layouts(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> BTreeMap<String, NodeLayout> {
    let mut nodes = BTreeMap::new();
    for node in graph.nodes.values() {
        let label = measure_label(&node.label, theme, config);
        let (width, height) = shape_size(node.shape, &label, config, theme, graph.kind);
        let style = resolve_node_style(node.id.as_str(), graph);
        nodes.insert(
            node.id.clone(),
            build_node_layout(node, label, width, height, style, graph),
        );
    }
    nodes
}

/// For state diagrams, push nodes that are not members of any subgraph
/// outside the subgraph bounds so they don't visually appear inside composites.
fn push_non_members_out_of_subgraphs(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) {
    if graph.subgraphs.is_empty() {
        return;
    }

    // Collect which nodes belong to which subgraphs
    let mut node_subgraphs: HashSet<String> = HashSet::new();
    for sub in &graph.subgraphs {
        for node_id in &sub.nodes {
            node_subgraphs.insert(node_id.clone());
        }
    }

    // Also treat subgraph IDs/labels as "member" since they're anchor nodes
    let mut subgraph_ids: HashSet<String> = HashSet::new();
    for sub in &graph.subgraphs {
        if let Some(ref id) = sub.id {
            subgraph_ids.insert(id.clone());
        }
        if !sub.label.is_empty() {
            subgraph_ids.insert(sub.label.clone());
        }
    }

    let gap = config.node_spacing * 0.5;

    // Compute subgraph bounds from their member nodes
    let mut sub_bounds: Vec<(f32, f32, f32, f32)> = Vec::new();
    for sub in &graph.subgraphs {
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
        let (pad_x, pad_y, top_pad) = subgraph_padding_from_label(
            graph,
            sub,
            theme,
            &measure_label(&sub.label, theme, config),
        );
        if min_x < f32::MAX {
            sub_bounds.push((min_x - pad_x, min_y - top_pad, max_x + pad_x, max_y + pad_y));
        } else {
            sub_bounds.push((0.0, 0.0, 0.0, 0.0));
        }
    }

    // For each non-member node, check if it overlaps with any subgraph bounds
    let node_ids: Vec<String> = nodes.keys().cloned().collect();
    for node_id in &node_ids {
        if node_subgraphs.contains(node_id) || subgraph_ids.contains(node_id) {
            continue;
        }
        let node = match nodes.get(node_id) {
            Some(n) => n,
            None => continue,
        };
        let nx = node.x;
        let ny = node.y;
        let nw = node.width;
        let nh = node.height;

        for (sx, sy, sx2, sy2) in &sub_bounds {
            // Check if node rectangle overlaps with subgraph rectangle
            if nx + nw > *sx && nx < *sx2 && ny + nh > *sy && ny < *sy2 {
                // Push node below the subgraph
                let new_y = *sy2 + gap;
                if let Some(node_mut) = nodes.get_mut(node_id) {
                    node_mut.y = new_y;
                }
                break;
            }
        }
    }
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
    if let Some(width) = source.stroke_width.and_then(sanitize_stroke_width) {
        target.stroke_width = Some(width);
    }
    if source.stroke_dasharray.is_some() {
        target.stroke_dasharray = source.stroke_dasharray.clone();
    }
    if source.line_color.is_some() {
        target.line_color = source.line_color.clone();
    }
}

fn shape_padding_factors(shape: crate::ir::NodeShape) -> (f32, f32) {
    match shape {
        crate::ir::NodeShape::Stadium => (0.43, 0.5),
        crate::ir::NodeShape::Subroutine => (0.54, 0.5),
        crate::ir::NodeShape::Parallelogram => (0.894, 0.5),
        crate::ir::NodeShape::ParallelogramAlt => (0.904, 0.5),
        _ => (1.0, 1.0),
    }
}

fn has_divider_line(label: &TextBlock) -> bool {
    label.lines.iter().any(|line| line.trim() == "---")
}

fn shape_size(
    shape: crate::ir::NodeShape,
    label: &TextBlock,
    config: &LayoutConfig,
    theme: &Theme,
    kind: crate::ir::DiagramKind,
) -> (f32, f32) {
    let (pad_x_factor, pad_y_factor) = shape_padding_factors(shape);
    let (kind_pad_x_scale, kind_pad_y_scale) = match kind {
        crate::ir::DiagramKind::Class => {
            let pad_x_scale = if has_divider_line(label) { 0.85 } else { 0.4 };
            (pad_x_scale, 0.8)
        }
        crate::ir::DiagramKind::Er => (1.05, 1.15),
        crate::ir::DiagramKind::Kanban => (2.3, 0.67),
        crate::ir::DiagramKind::Requirement => (0.1, 1.0),
        crate::ir::DiagramKind::Block => (0.5, 0.35),
        _ => (1.0, 1.0),
    };
    let mut pad_x = config.node_padding_x * pad_x_factor * kind_pad_x_scale;
    let mut pad_y = config.node_padding_y * pad_y_factor * kind_pad_y_scale;
    if kind == crate::ir::DiagramKind::State {
        let dynamic_pad_x =
            (theme.font_size * STATE_PAD_X_SCALE).max(label.width * STATE_PAD_X_LABEL_RATIO);
        let dynamic_pad_y =
            (theme.font_size * STATE_PAD_Y_SCALE).max(label.height * STATE_PAD_Y_LABEL_RATIO);
        pad_x = dynamic_pad_x;
        pad_y = dynamic_pad_y;
    }
    let base_width = label.width + pad_x * 2.0;
    let base_height = label.height + pad_y * 2.0;
    let mut width = base_width;
    let mut height = base_height;
    let label_empty = label.lines.len() == 1 && label.lines[0].trim().is_empty();

    match shape {
        crate::ir::NodeShape::Diamond => {
            // Mermaid renders diamonds as squares sized off the larger
            // dimension rather than stretching width/height independently.
            let size = base_width.max(base_height) * DIAMOND_SCALE;
            width = size;
            height = size;
        }
        crate::ir::NodeShape::ForkJoin => {
            width = width.max(FORK_JOIN_MIN_WIDTH);
            height = (config.node_padding_y * FORK_JOIN_HEIGHT_SCALE).max(FORK_JOIN_MIN_HEIGHT);
        }
        crate::ir::NodeShape::Circle | crate::ir::NodeShape::DoubleCircle => {
            let size = if label_empty {
                (config.node_padding_y * CIRCLE_EMPTY_HEIGHT_SCALE).max(CIRCLE_EMPTY_MIN_SIZE)
            } else {
                width.max(height)
            };
            width = size;
            height = size;
        }
        crate::ir::NodeShape::Stadium => {}
        crate::ir::NodeShape::RoundRect => {
            width *= ROUND_RECT_WIDTH_SCALE;
            height *= ROUND_RECT_HEIGHT_SCALE;
        }
        crate::ir::NodeShape::Cylinder => {
            width *= CYLINDER_SCALE;
            height *= CYLINDER_SCALE;
        }
        crate::ir::NodeShape::Hexagon => {
            width *= HEXAGON_WIDTH_SCALE;
            height *= HEXAGON_HEIGHT_SCALE;
        }
        crate::ir::NodeShape::Parallelogram | crate::ir::NodeShape::ParallelogramAlt => {}
        crate::ir::NodeShape::Trapezoid
        | crate::ir::NodeShape::TrapezoidAlt
        | crate::ir::NodeShape::Asymmetric => {
            width *= TRAPEZOID_WIDTH_SCALE;
        }
        crate::ir::NodeShape::Subroutine => {}
        _ => {}
    }

    if kind == crate::ir::DiagramKind::Class {
        let min_height = theme.font_size * CLASS_MIN_HEIGHT_SCALE;
        height = height.max(min_height);
    }

    if kind == crate::ir::DiagramKind::Requirement {
        let min_width = theme.font_size * REQUIREMENT_MIN_WIDTH_SCALE;
        width = width.max(min_width);
    }

    if kind == crate::ir::DiagramKind::Kanban {
        let min_width = theme.font_size * KANBAN_MIN_WIDTH_SCALE;
        let min_height = theme.font_size * KANBAN_MIN_HEIGHT_SCALE;
        width = width.max(min_width);
        height = height.max(min_height);
    }

    (width, height)
}

fn requirement_edge_label_text(label: &str, config: &LayoutConfig) -> String {
    let trimmed = label
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if config.requirement.edge_label_brackets {
        format!("<<{}>>", trimmed)
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Direction, Graph, NodeShape};
    use crate::layout::ranking::rank_edges_for_manual_layout;
    use crate::parser::parse_mermaid;

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
    fn layout_normalizes_missing_edge_endpoint_nodes() {
        let mut graph = Graph::new();
        graph.direction = Direction::LeftRight;
        graph.ensure_node("A", Some("Alpha".to_string()), Some(NodeShape::Rectangle));
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

        assert!(layout.nodes.contains_key("A"));
        assert!(layout.nodes.contains_key("B"));
        assert_eq!(layout.edges.len(), 1);
    }

    #[test]
    fn sequence_layout_normalizes_missing_participants() {
        let mut graph = Graph::new();
        graph.kind = crate::ir::DiagramKind::Sequence;
        graph.sequence_participants.push("Alice".to_string());
        graph.sequence_participants.push("Bob".to_string());

        let layout = compute_layout(&graph, &Theme::modern(), &LayoutConfig::default());

        assert!(layout.nodes.contains_key("Alice"));
        assert!(layout.nodes.contains_key("Bob"));
    }

    #[test]
    fn right_left_layout_places_successor_to_left() {
        let parsed = parse_mermaid(
            r#"
flowchart RL
    A --> B
"#,
        )
        .expect("parse failed");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let a = layout.nodes.get("A").unwrap();
        let b = layout.nodes.get("B").unwrap();
        assert!(
            b.x + b.width <= a.x + 1.0,
            "expected successor to be laid out to the left in RL mode"
        );
    }

    #[test]
    fn bottom_top_layout_places_successor_above() {
        let parsed = parse_mermaid(
            r#"
flowchart BT
    A --> B
"#,
        )
        .expect("parse failed");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let a = layout.nodes.get("A").unwrap();
        let b = layout.nodes.get("B").unwrap();
        assert!(
            b.y + b.height <= a.y + 1.0,
            "expected successor to be laid out above in BT mode"
        );
    }

    #[test]
    fn flowchart_subgraph_layouts_enclose_member_nodes() {
        let parsed = parse_mermaid(
            r#"
flowchart LR
    subgraph API
        A[Gateway]
        B[Worker]
    end
    subgraph Data
        C[Store]
    end
    A --> B
    B --> C
"#,
        )
        .expect("parse failed");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        for sub in &layout.subgraphs {
            for node_id in &sub.nodes {
                let node = layout
                    .nodes
                    .get(node_id)
                    .unwrap_or_else(|| panic!("missing node {node_id} for subgraph {}", sub.label));
                assert!(
                    node.x >= sub.x - 1.0,
                    "node {node_id} should stay inside subgraph {} on the left edge",
                    sub.label
                );
                assert!(
                    node.y >= sub.y - 1.0,
                    "node {node_id} should stay inside subgraph {} on the top edge",
                    sub.label
                );
                assert!(
                    node.x + node.width <= sub.x + sub.width + 1.0,
                    "node {node_id} should stay inside subgraph {} on the right edge",
                    sub.label
                );
                assert!(
                    node.y + node.height <= sub.y + sub.height + 1.0,
                    "node {node_id} should stay inside subgraph {} on the bottom edge",
                    sub.label
                );
            }
        }
    }

    #[test]
    fn flowchart_top_level_subgraphs_do_not_overlap_after_layout() {
        let parsed = parse_mermaid(
            r#"
flowchart LR
    subgraph API
        A[Gateway]
        B[Worker]
    end
    subgraph Data
        C[Store]
        D[Replica]
    end
    A --> C
    B --> D
"#,
        )
        .expect("parse failed");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        assert!(
            layout.subgraphs.len() >= 2,
            "expected at least two subgraphs in layout"
        );

        for (idx, left) in layout.subgraphs.iter().enumerate() {
            for right in layout.subgraphs.iter().skip(idx + 1) {
                let overlaps_x = left.x < right.x + right.width && right.x < left.x + left.width;
                let overlaps_y = left.y < right.y + right.height && right.y < left.y + left.height;
                assert!(
                    !(overlaps_x && overlaps_y),
                    "subgraphs {} and {} should not overlap",
                    left.label,
                    right.label
                );
            }
        }
    }

    #[test]
    fn tiny_flowchart_cycle_routes_around_non_endpoint_nodes() {
        let parsed = parse_mermaid(
            r#"
flowchart LR
    A --> B
    B --> C
    C --> A
"#,
        )
        .expect("parse failed");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        for edge in &layout.edges {
            assert!(
                !flowchart::path_cleanup::flowchart_path_hits_non_endpoint_nodes(
                    &edge.points,
                    &edge.from,
                    &edge.to,
                    &layout.nodes,
                ),
                "edge {}->{} should not pass through a non-endpoint node",
                edge.from,
                edge.to
            );
        }
    }

    #[test]
    fn flowchart_cycle_places_downstream_after_cycle_block() {
        let parsed = parse_mermaid(
            r#"
flowchart LR
    A --> B
    B --> C
    C --> A
    C --> D
"#,
        )
        .expect("parse failed");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        let a = layout.nodes.get("A").unwrap();
        let b = layout.nodes.get("B").unwrap();
        let c = layout.nodes.get("C").unwrap();
        let d = layout.nodes.get("D").unwrap();

        assert!(
            a.x < b.x,
            "expected cycle nodes to progress on the main axis"
        );
        assert!(
            b.x < c.x,
            "expected cycle nodes to progress on the main axis"
        );
        assert!(
            d.x >= c.x + c.width - 1.0,
            "expected downstream node to appear after the cycle component"
        );
    }

    #[test]
    fn right_left_cycle_places_downstream_before_cycle_block() {
        let parsed = parse_mermaid(
            r#"
flowchart RL
    A --> B
    B --> C
    C --> A
    C --> D
"#,
        )
        .expect("parse failed");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        let a = layout.nodes.get("A").unwrap();
        let b = layout.nodes.get("B").unwrap();
        let c = layout.nodes.get("C").unwrap();
        let d = layout.nodes.get("D").unwrap();

        assert!(a.x > b.x, "expected mirrored cycle to progress leftward");
        assert!(b.x > c.x, "expected mirrored cycle to progress leftward");
        assert!(
            d.x + d.width <= c.x + 1.0,
            "expected downstream node to appear before the mirrored cycle component"
        );
    }

    #[test]
    fn cycle_fixture_keeps_forward_backbone_edges_reasonably_simple() {
        let source = include_str!("../../docs/comparison_sources/flowchart_cycles.mmd");
        let parsed = parse_mermaid(source).expect("failed to parse flowchart fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        for (from, to) in [("B", "C"), ("C", "D")] {
            let edge = layout
                .edges
                .iter()
                .find(|edge| edge.from == from && edge.to == to)
                .unwrap_or_else(|| panic!("missing edge {from}->{to}"));
            let bends = path_bend_count(&edge.points);
            assert!(
                bends <= 3,
                "expected cycle backbone edge {from}->{to} to stay reasonably direct, got {bends} bends: {:?}",
                edge.points
            );
        }
    }

    #[test]
    fn cycle_fixture_back_edge_uses_outer_side_ports() {
        let source = include_str!("../../docs/comparison_sources/flowchart_cycles.mmd");
        let parsed = parse_mermaid(source).expect("failed to parse flowchart fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        let edge = layout
            .edges
            .iter()
            .find(|edge| edge.from == "D" && edge.to == "B")
            .expect("missing D->B back edge");
        assert!(
            edge.points.len() >= 3,
            "expected routed back edge to contain at least one bend"
        );

        let first = edge.points[0];
        let second = edge.points[1];
        let penultimate = edge.points[edge.points.len() - 2];
        let last = edge.points[edge.points.len() - 1];

        assert!(
            (second.1 - first.1).abs() <= 1.0,
            "expected back edge to leave D through a side port, got {:?}",
            edge.points
        );
        assert!(
            (last.1 - penultimate.1).abs() <= 1.0,
            "expected back edge to enter B through a side port, got {:?}",
            edge.points
        );
    }

    #[test]
    fn cycle_fixture_backbone_edges_collapse_to_straight_segments() {
        let source = include_str!("../../docs/comparison_sources/flowchart_cycles.mmd");
        let parsed = parse_mermaid(source).expect("failed to parse flowchart fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        for (from, to) in [("A", "B"), ("B", "C"), ("C", "D"), ("D", "E")] {
            let edge = layout
                .edges
                .iter()
                .find(|edge| edge.from == from && edge.to == to)
                .unwrap_or_else(|| panic!("missing edge {from}->{to}"));
            assert_eq!(
                edge.points.len(),
                2,
                "expected cycle backbone edge {from}->{to} to collapse to a straight segment, got {:?}",
                edge.points
            );
        }
    }

    #[test]
    fn cycle_fixture_subgraph_has_room_for_title_and_return_lane() {
        let source = include_str!("../../docs/comparison_sources/flowchart_cycles.mmd");
        let parsed = parse_mermaid(source).expect("failed to parse flowchart fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        let subgraph = layout
            .subgraphs
            .iter()
            .find(|sub| sub.label == "LoopBlock")
            .expect("missing LoopBlock subgraph");
        let member_bounds = subgraph
            .nodes
            .iter()
            .filter_map(|node_id| layout.nodes.get(node_id))
            .fold(None, |bounds, node| match bounds {
                None => Some((node.x, node.y, node.x + node.width, node.y + node.height)),
                Some((min_x, min_y, max_x, max_y)) => Some((
                    min_x.min(node.x),
                    min_y.min(node.y),
                    max_x.max(node.x + node.width),
                    max_y.max(node.y + node.height),
                )),
            })
            .expect("expected member bounds");

        let (_min_x, min_y, _max_x, max_y) = member_bounds;
        let top_gap = min_y - subgraph.y;
        let bottom_gap = (subgraph.y + subgraph.height) - max_y;

        assert!(
            top_gap >= 40.0,
            "expected LoopBlock title clearance >= 40px, got {top_gap}"
        );
        assert!(
            bottom_gap >= 40.0,
            "expected LoopBlock bottom clearance >= 40px, got {bottom_gap}"
        );
    }

    #[test]
    fn cycle_fixture_subgraph_entry_aligns_with_spine() {
        let source = include_str!("../../docs/comparison_sources/flowchart_cycles.mmd");
        let parsed = parse_mermaid(source).expect("failed to parse flowchart fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        let edge = layout
            .edges
            .iter()
            .find(|edge| edge.from == "E" && edge.to == "F")
            .expect("missing E->F edge");

        assert_eq!(
            edge.points.len(),
            2,
            "expected E->F to collapse to a straight handoff, got {:?}",
            edge.points
        );
        assert!(
            (edge.points[0].0 - edge.points[1].0).abs() <= 1.0,
            "expected E->F to stay vertically aligned, got {:?}",
            edge.points
        );
    }

    fn has_axis_oscillation(points: &[(f32, f32)]) -> bool {
        if points.len() < 4 {
            return false;
        }

        let mut idx = 0usize;
        while idx + 2 < points.len() {
            let a = points[idx];
            let b = points[idx + 1];
            let c = points[idx + 2];
            let same_x = (a.0 - b.0).abs() <= 1.0 && (b.0 - c.0).abs() <= 1.0;
            let same_y = (a.1 - b.1).abs() <= 1.0 && (b.1 - c.1).abs() <= 1.0;
            if same_x || same_y {
                let delta1 = if same_x { b.1 - a.1 } else { b.0 - a.0 };
                let delta2 = if same_x { c.1 - b.1 } else { c.0 - b.0 };
                if delta1.abs() > 1.0 && delta2.abs() > 1.0 && delta1.signum() != delta2.signum() {
                    return true;
                }
            }
            idx += 1;
        }
        false
    }

    #[test]
    fn opaque_flowchart_challenge_edge_has_no_axis_oscillation() {
        let source = include_str!("../../docs/comparison_sources/flowchart_opaque.mmd");
        let parsed = parse_mermaid(source).expect("failed to parse opaque flowchart fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        let edge = layout
            .edges
            .iter()
            .find(|edge| {
                edge.from == "Baldr"
                    && edge.to == "Forseti"
                    && edge
                        .label
                        .as_ref()
                        .map(|label| label.lines.join(" "))
                        .as_deref()
                        == Some("4. Sends challenge")
            })
            .expect("missing Baldr->Forseti challenge edge");

        assert!(
            !has_axis_oscillation(&edge.points),
            "expected challenge edge to avoid same-axis backtracking, got {:?}",
            edge.points
        );
    }

    #[test]
    fn disconnected_flowchart_components_align_on_cross_axis() {
        let parsed = parse_mermaid(
            r#"
flowchart LR
    A --> B
    C --> D
"#,
        )
        .expect("parse failed");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        let a = layout.nodes.get("A").unwrap();
        let b = layout.nodes.get("B").unwrap();
        let c = layout.nodes.get("C").unwrap();
        let d = layout.nodes.get("D").unwrap();

        assert!(
            (a.y - c.y).abs() < 1.0,
            "components should share a cross-axis baseline"
        );
        assert!(
            (b.y - d.y).abs() < 1.0,
            "components should stay aligned after packing"
        );
        assert!(
            c.x >= a.x.max(b.x + b.width) - 1.0,
            "second component should remain after the first on the main axis"
        );
    }

    #[test]
    fn flowchart_subgraph_direction_fixture_keeps_lr_members_horizontal() {
        let source = include_str!("../../docs/comparison_sources/flowchart_subgraph_dir.mmd");
        let parsed = parse_mermaid(source).expect("failed to parse flowchart fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        let a = layout.nodes.get("A").unwrap();
        let b = layout.nodes.get("B").unwrap();
        let c = layout.nodes.get("C").unwrap();

        assert!(
            a.x < b.x && b.x < c.x,
            "LR subgraph should progress horizontally"
        );
        assert!(
            (a.y - b.y).abs() < 1.0 && (b.y - c.y).abs() < 1.0,
            "LR subgraph should stay aligned on the cross-axis"
        );
    }

    #[test]
    fn dense_flowchart_avoids_crossing_between_middle_and_far_edges() {
        let source = include_str!("../../docs/comparison_sources/flowchart_dense.mmd");
        let parsed = parse_mermaid(source).expect("failed to parse flowchart fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        let be = layout
            .edges
            .iter()
            .find(|edge| edge.from == "B" && edge.to == "E")
            .expect("B->E edge");
        let dh = layout
            .edges
            .iter()
            .find(|edge| edge.from == "D" && edge.to == "H")
            .expect("D->H edge");

        for a in be.points.windows(2) {
            for b in dh.points.windows(2) {
                assert!(
                    !segments_intersect(a[0], a[1], b[0], b[1]),
                    "dense routing should avoid B->E crossing D->H"
                );
            }
        }
    }

    #[test]
    fn dense_flowchart_keeps_mid_span_edge_reasonably_direct() {
        let source = include_str!("../../docs/comparison_sources/flowchart_dense.mmd");
        let parsed = parse_mermaid(source).expect("failed to parse flowchart fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        let be = layout
            .edges
            .iter()
            .find(|edge| edge.from == "B" && edge.to == "E")
            .expect("B->E edge");

        let path_len: f32 = be
            .points
            .windows(2)
            .map(|segment| {
                let dx = segment[1].0 - segment[0].0;
                let dy = segment[1].1 - segment[0].1;
                (dx * dx + dy * dy).sqrt()
            })
            .sum();
        let manhattan = match (be.points.first(), be.points.last()) {
            (Some(start), Some(end)) => (end.0 - start.0).abs() + (end.1 - start.1).abs(),
            _ => 0.0,
        };

        assert!(
            manhattan > 0.0 && path_len / manhattan <= 2.5,
            "dense routing should keep B->E reasonably direct (path={path_len:.1}, manhattan={manhattan:.1})"
        );
    }

    #[test]
    fn opaque_flowchart_routes_around_large_label_boxes() {
        let source = include_str!("../../docs/comparison_sources/flowchart_opaque.mmd");
        let parsed = parse_mermaid(source).expect("failed to parse flowchart fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        let edge = layout
            .edges
            .iter()
            .find(|edge| {
                edge.from == "Forseti"
                    && edge.to == "Baldr"
                    && edge
                        .label
                        .as_ref()
                        .is_some_and(|label| label.lines.join(" ").contains("Check connectivity"))
            })
            .expect("Forseti->Baldr connectivity edge");
        let label = edge.label.as_ref().expect("edge label");
        let anchor = edge.label_anchor.expect("edge label anchor");
        let label_box = Obstacle {
            id: "label".to_string(),
            x: anchor.0 - label.width / 2.0,
            y: anchor.1 - label.height / 2.0,
            width: label.width,
            height: label.height,
            members: None,
        };

        let intersects = edge
            .points
            .windows(2)
            .any(|segment| segment_intersects_rect(segment[0], segment[1], &label_box));
        assert!(
            !intersects,
            "expected routed path to avoid its own large label box"
        );
    }

    #[test]
    fn assign_positions_preserves_input_order_within_rank() {
        let node_ids = vec!["B".to_string(), "A".to_string()];
        let ranks = HashMap::from([("A".to_string(), 0usize), ("B".to_string(), 0usize)]);
        let mut nodes = BTreeMap::from([
            (
                "A".to_string(),
                NodeLayout {
                    id: "A".to_string(),
                    x: 0.0,
                    y: 0.0,
                    width: 60.0,
                    height: 36.0,
                    label: TextBlock {
                        lines: vec!["A".to_string()],
                        width: 10.0,
                        height: 10.0,
                    },
                    shape: NodeShape::Rectangle,
                    style: crate::ir::NodeStyle::default(),
                    link: None,
                    anchor_subgraph: None,
                    hidden: false,
                    icon: None,
                },
            ),
            (
                "B".to_string(),
                NodeLayout {
                    id: "B".to_string(),
                    x: 0.0,
                    y: 0.0,
                    width: 60.0,
                    height: 36.0,
                    label: TextBlock {
                        lines: vec!["B".to_string()],
                        width: 10.0,
                        height: 10.0,
                    },
                    shape: NodeShape::Rectangle,
                    style: crate::ir::NodeStyle::default(),
                    link: None,
                    anchor_subgraph: None,
                    hidden: false,
                    icon: None,
                },
            ),
        ]);

        assign_positions(
            &node_ids,
            &ranks,
            Direction::TopDown,
            &LayoutConfig::default(),
            &mut nodes,
            0.0,
            0.0,
        );

        let a = nodes.get("A").unwrap();
        let b = nodes.get("B").unwrap();
        assert!(
            b.x < a.x,
            "shared placement should preserve caller ordering within a rank"
        );
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

    #[test]
    fn er_labels_stay_attached_after_path_postprocess() {
        let source = include_str!("../../docs/comparison_sources/er_blog.mmd");
        let parsed = parse_mermaid(source).expect("failed to parse ER fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        let mut labeled_edges = 0usize;
        for edge in &layout.edges {
            let (Some(_label), Some(anchor)) = (&edge.label, edge.label_anchor) else {
                continue;
            };
            labeled_edges += 1;
            let dist = polyline_point_distance(&edge.points, anchor);
            assert!(
                dist <= 12.0,
                "edge {}->{} label anchor drifted {:.2}px from own path",
                edge.from,
                edge.to,
                dist
            );
        }
        assert!(
            labeled_edges > 0,
            "fixture must contain at least one labeled edge"
        );
    }

    #[test]
    fn flowchart_labels_stay_attached_after_path_postprocess() {
        let parsed = parse_mermaid(
            r#"
flowchart LR
    A[Client] -->|Request payload| B[Gateway]
    B -->|Route lookup| C[Service]
    C -->|Response body| A
"#,
        )
        .expect("failed to parse flowchart fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        let mut labeled_edges = 0usize;
        for edge in &layout.edges {
            let (Some(label), Some(anchor)) = (&edge.label, edge.label_anchor) else {
                continue;
            };
            labeled_edges += 1;
            let label_box = Obstacle {
                id: "label".to_string(),
                x: anchor.0 - label.width / 2.0,
                y: anchor.1 - label.height / 2.0,
                width: label.width,
                height: label.height,
                members: None,
            };
            let intersects = edge
                .points
                .windows(2)
                .any(|segment| segment_intersects_rect(segment[0], segment[1], &label_box));
            assert!(
                !intersects,
                "flowchart edge {}->{} should avoid its own label box",
                edge.from, edge.to
            );
        }
        assert!(
            labeled_edges > 0,
            "fixture must contain at least one labeled edge"
        );
    }

    fn make_node(id: &str, x: f32, y: f32, width: f32, height: f32) -> NodeLayout {
        NodeLayout {
            id: id.to_string(),
            x,
            y,
            width,
            height,
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
        }
    }

    fn make_edge(from: &str, to: &str, style: crate::ir::EdgeStyle) -> crate::ir::Edge {
        crate::ir::Edge {
            from: from.to_string(),
            to: to.to_string(),
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
            style,
        }
    }

    #[test]
    fn compact_large_flowchart_whitespace_reduces_main_axis_spread() {
        let mut graph = Graph::new();
        graph.kind = crate::ir::DiagramKind::Flowchart;
        graph.direction = Direction::LeftRight;

        let mut nodes = BTreeMap::new();
        for idx in 0..16 {
            let id = format!("N{idx}");
            let row = idx / 8;
            let col = idx % 8;
            let x = 24.0 + col as f32 * 140.0;
            let y = 16.0 + row as f32 * 110.0;
            nodes.insert(id.clone(), make_node(&id, x, y, 60.0, 36.0));
        }

        for idx in 0..16 {
            let from = format!("N{idx}");
            let to = format!("N{}", (idx + 1) % 16);
            graph
                .edges
                .push(make_edge(&from, &to, crate::ir::EdgeStyle::Solid));
        }
        for idx in 0..8 {
            let from = format!("N{idx}");
            let to = format!("N{}", idx + 8);
            graph
                .edges
                .push(make_edge(&from, &to, crate::ir::EdgeStyle::Solid));
        }

        let mut config = LayoutConfig::default();
        config.flowchart.objective.max_aspect_ratio = 4.0;
        let before_min_x = nodes.values().map(|node| node.x).fold(f32::MAX, f32::min);
        let before_max_x = nodes
            .values()
            .map(|node| node.x + node.width)
            .fold(f32::MIN, f32::max);
        let before_span = before_max_x - before_min_x;
        let before_cross: std::collections::HashMap<String, f32> = nodes
            .iter()
            .map(|(id, node)| (id.clone(), node.y))
            .collect();

        flowchart::objectives::compact_large_flowchart_whitespace(&graph, &mut nodes, &config);

        let after_min_x = nodes.values().map(|node| node.x).fold(f32::MAX, f32::min);
        let after_max_x = nodes
            .values()
            .map(|node| node.x + node.width)
            .fold(f32::MIN, f32::max);
        let after_span = after_max_x - after_min_x;

        assert!(
            after_span + 1e-3 < before_span,
            "expected whitespace compaction to reduce horizontal spread (before={before_span:.2}, after={after_span:.2})"
        );
        for (id, before_y) in before_cross {
            let after_y = nodes.get(&id).map(|node| node.y).unwrap_or(before_y);
            assert!(
                (after_y - before_y).abs() <= 1e-3,
                "expected compaction to preserve row/cross placement for {id}: before={before_y:.3} after={after_y:.3}"
            );
        }
    }

    #[test]
    fn path_bend_count_tracks_turns() {
        let straight = vec![(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)];
        let orth = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (20.0, 10.0)];
        assert_eq!(path_bend_count(&straight), 0);
        assert_eq!(path_bend_count(&orth), 2);
    }

    #[test]
    fn edge_label_anchor_uses_path_progress_midpoint() {
        let points = vec![(0.0, 0.0), (20.0, 0.0), (20.0, 100.0)];
        let center = edge_label_anchor_from_points(&points).expect("anchor");
        assert!((center.0 - 20.0).abs() <= 1e-3);
        assert!((center.1 - 40.0).abs() <= 1e-3);
    }

    #[test]
    fn rank_edges_prefers_non_dotted_flow_edges_when_coverage_is_good() {
        let graph = Graph::new();
        let nodes = vec![
            "A".to_string(),
            "B".to_string(),
            "C".to_string(),
            "D".to_string(),
        ];
        let edges = vec![
            make_edge("A", "B", crate::ir::EdgeStyle::Solid),
            make_edge("B", "C", crate::ir::EdgeStyle::Solid),
            make_edge("C", "D", crate::ir::EdgeStyle::Solid),
            make_edge("A", "D", crate::ir::EdgeStyle::Dotted),
        ];
        let rank_edges = rank_edges_for_manual_layout(&graph, &nodes, &edges);
        assert_eq!(rank_edges.len(), 3);
        assert!(
            rank_edges
                .iter()
                .all(|edge| edge.style != crate::ir::EdgeStyle::Dotted)
        );
    }

    #[test]
    fn rank_edges_falls_back_when_primary_coverage_is_too_small() {
        let graph = Graph::new();
        let nodes = vec![
            "A".to_string(),
            "B".to_string(),
            "C".to_string(),
            "D".to_string(),
            "E".to_string(),
        ];
        let edges = vec![
            make_edge("A", "B", crate::ir::EdgeStyle::Solid),
            make_edge("C", "D", crate::ir::EdgeStyle::Dotted),
            make_edge("D", "E", crate::ir::EdgeStyle::Dotted),
            make_edge("E", "C", crate::ir::EdgeStyle::Dotted),
        ];
        let rank_edges = rank_edges_for_manual_layout(&graph, &nodes, &edges);
        assert_eq!(rank_edges.len(), edges.len());
    }

    #[test]
    fn routing_avoids_occupied_lane_when_possible() {
        let config = LayoutConfig::default();
        let from = make_node("A", 0.0, 0.0, 40.0, 40.0);
        let to = make_node("B", 200.0, 0.0, 40.0, 40.0);
        let obstacles: Vec<Obstacle> = Vec::new();
        let label_obstacles: Vec<Obstacle> = Vec::new();
        let ctx = RouteContext {
            from_id: &from.id,
            to_id: &to.id,
            from: &from,
            to: &to,
            direction: Direction::LeftRight,
            config: &config,
            obstacles: &obstacles,
            label_obstacles: &label_obstacles,
            base_offset: 0.0,
            start_side: EdgeSide::Right,
            end_side: EdgeSide::Left,
            start_offset: 0.0,
            end_offset: 0.0,
            fast_route: false,
            stub_len: port_stub_length(&config, &from, &to),
            start_inset: 0.0,
            end_inset: 0.0,
            prefer_shorter_ties: true,
            preferred_label_id: None,
            preferred_label_center: None,
            preferred_label_obstacle: None,
            preferred_label_clearance: 0.0,
            force_preferred_label_via: true,
            coarse_grid_retry: true,
        };
        let mut occupancy = EdgeOccupancy::new(
            config.node_spacing.max(MIN_NODE_SPACING_FLOOR) * EDGE_OCCUPANCY_CELL_RATIO,
        );
        let start = anchor_point_for_node(&from, EdgeSide::Right, 0.0);
        let end = anchor_point_for_node(&to, EdgeSide::Left, 0.0);
        occupancy.add_path(&[start, end]);

        let points = route_edge_with_avoidance(&ctx, Some(&occupancy), None, None);
        assert!(
            points.len() > 2,
            "expected a detoured path to avoid occupied lane"
        );
    }

    #[test]
    fn routing_handles_tiny_nodes_without_panicking() {
        let config = LayoutConfig::default();
        let from = make_node("A", 0.0, 0.0, 1.0, 1.0);
        let to = make_node("B", 50.0, 0.0, 1.0, 1.0);
        let obstacles: Vec<Obstacle> = Vec::new();
        let label_obstacles: Vec<Obstacle> = Vec::new();
        let ctx = RouteContext {
            from_id: &from.id,
            to_id: &to.id,
            from: &from,
            to: &to,
            direction: Direction::LeftRight,
            config: &config,
            obstacles: &obstacles,
            label_obstacles: &label_obstacles,
            base_offset: 0.0,
            start_side: EdgeSide::Right,
            end_side: EdgeSide::Left,
            start_offset: 0.0,
            end_offset: 0.0,
            fast_route: false,
            stub_len: port_stub_length(&config, &from, &to),
            start_inset: 0.0,
            end_inset: 0.0,
            prefer_shorter_ties: true,
            preferred_label_id: None,
            preferred_label_center: None,
            preferred_label_obstacle: None,
            preferred_label_clearance: 0.0,
            force_preferred_label_via: true,
            coarse_grid_retry: true,
        };
        let points = route_edge_with_avoidance(&ctx, None, None, None);
        assert!(!points.is_empty());
    }

    #[test]
    fn grid_router_avoids_blocking_obstacle() {
        let mut config = LayoutConfig::default();
        config.flowchart.routing.enable_grid_router = true;
        config.flowchart.routing.grid_cell = 10.0;
        let from = make_node("A", 0.0, 0.0, 40.0, 40.0);
        let to = make_node("B", 220.0, 0.0, 40.0, 40.0);
        let obstacles = vec![Obstacle {
            id: "blocker".to_string(),
            x: 90.0,
            y: -10.0,
            width: 80.0,
            height: 60.0,
            members: None,
        }];
        let label_obstacles: Vec<Obstacle> = Vec::new();
        let grid = build_routing_grid(&obstacles, &config).expect("routing grid");
        let ctx = RouteContext {
            from_id: &from.id,
            to_id: &to.id,
            from: &from,
            to: &to,
            direction: Direction::LeftRight,
            config: &config,
            obstacles: &obstacles,
            label_obstacles: &label_obstacles,
            base_offset: 0.0,
            start_side: EdgeSide::Right,
            end_side: EdgeSide::Left,
            start_offset: 0.0,
            end_offset: 0.0,
            fast_route: false,
            stub_len: port_stub_length(&config, &from, &to),
            start_inset: 0.0,
            end_inset: 0.0,
            prefer_shorter_ties: true,
            preferred_label_id: None,
            preferred_label_center: None,
            preferred_label_obstacle: None,
            preferred_label_clearance: 0.0,
            force_preferred_label_via: true,
            coarse_grid_retry: true,
        };
        let start = anchor_point_for_node(&from, EdgeSide::Right, 0.0);
        let end = anchor_point_for_node(&to, EdgeSide::Left, 0.0);
        let stub_len = port_stub_length(&config, &from, &to);
        let start_stub = port_stub_point(start, EdgeSide::Right, stub_len);
        let end_stub = port_stub_point(end, EdgeSide::Left, stub_len);
        let points =
            route_edge_with_grid(&ctx, &grid, None, start_stub, end_stub).expect("grid route");
        let hits = path_obstacle_intersections(&points, &obstacles, &from.id, &to.id);
        assert_eq!(hits, 0, "grid path should avoid obstacle");
    }

    #[test]
    fn path_label_intersections_can_ignore_owned_reservation() {
        let path = vec![(0.0, 0.0), (100.0, 0.0)];
        let labels = vec![
            Obstacle {
                id: "edge-label-reserved:0".to_string(),
                x: 40.0,
                y: -5.0,
                width: 20.0,
                height: 10.0,
                members: None,
            },
            Obstacle {
                id: "edge-label-reserved:1".to_string(),
                x: 70.0,
                y: -5.0,
                width: 20.0,
                height: 10.0,
                members: None,
            },
        ];
        let all_hits = path_label_intersections(&path, &labels, None);
        assert_eq!(all_hits, 2);
        let own_ignored = path_label_intersections(&path, &labels, Some("edge-label-reserved:0"));
        assert_eq!(own_ignored, 1);
    }

    #[test]
    fn routing_prefers_path_through_preferred_label_center() {
        let config = LayoutConfig::default();
        let from = make_node("A", 0.0, 0.0, 40.0, 40.0);
        let to = make_node("B", 220.0, 0.0, 40.0, 40.0);
        let obstacles: Vec<Obstacle> = Vec::new();
        let label_obstacles: Vec<Obstacle> = Vec::new();
        let preferred = (120.0, 84.0);
        let ctx = RouteContext {
            from_id: &from.id,
            to_id: &to.id,
            from: &from,
            to: &to,
            direction: Direction::LeftRight,
            config: &config,
            obstacles: &obstacles,
            label_obstacles: &label_obstacles,
            base_offset: 0.0,
            start_side: EdgeSide::Right,
            end_side: EdgeSide::Left,
            start_offset: 0.0,
            end_offset: 0.0,
            fast_route: false,
            stub_len: port_stub_length(&config, &from, &to),
            start_inset: 0.0,
            end_inset: 0.0,
            prefer_shorter_ties: true,
            preferred_label_id: Some("edge-label-reserved:0"),
            preferred_label_center: Some(preferred),
            preferred_label_obstacle: None,
            preferred_label_clearance: 0.0,
            force_preferred_label_via: true,
            coarse_grid_retry: true,
        };
        let points = route_edge_with_avoidance(&ctx, None, None, None);
        let dist = polyline_point_distance(&points, preferred);
        assert!(
            dist <= 0.51,
            "expected routed path to pass through preferred label center, got distance {dist:.3}"
        );
    }

    #[test]
    fn flowchart_routing_avoids_reserved_label_corridor_during_route_selection() {
        let config = LayoutConfig::default();
        let from = make_node("A", 0.0, 0.0, 40.0, 40.0);
        let to = make_node("B", 220.0, 0.0, 40.0, 40.0);
        let obstacles: Vec<Obstacle> = Vec::new();
        let label_obstacles = vec![Obstacle {
            id: "edge-label-reserved:0".to_string(),
            x: 95.0,
            y: -12.0,
            width: 70.0,
            height: 24.0,
            members: None,
        }];
        let clearance = 10.0;
        let ctx = RouteContext {
            from_id: &from.id,
            to_id: &to.id,
            from: &from,
            to: &to,
            direction: Direction::LeftRight,
            config: &config,
            obstacles: &obstacles,
            label_obstacles: &label_obstacles,
            base_offset: 0.0,
            start_side: EdgeSide::Right,
            end_side: EdgeSide::Left,
            start_offset: 0.0,
            end_offset: 0.0,
            fast_route: false,
            stub_len: port_stub_length(&config, &from, &to),
            start_inset: 0.0,
            end_inset: 0.0,
            prefer_shorter_ties: false,
            preferred_label_id: Some("edge-label-reserved:0"),
            preferred_label_center: None,
            preferred_label_obstacle: Some(&label_obstacles[0]),
            preferred_label_clearance: clearance,
            force_preferred_label_via: false,
            coarse_grid_retry: true,
        };
        let points = route_edge_with_avoidance(&ctx, None, None, None);
        let label_corridor = Obstacle {
            id: "expanded".to_string(),
            x: label_obstacles[0].x - clearance,
            y: label_obstacles[0].y - clearance,
            width: label_obstacles[0].width + clearance * 2.0,
            height: label_obstacles[0].height + clearance * 2.0,
            members: None,
        };
        let intersects = points
            .windows(2)
            .any(|segment| segment_intersects_rect(segment[0], segment[1], &label_corridor));
        assert!(
            !intersects,
            "expected routed path to avoid reserved label corridor, got {points:?}"
        );
    }
}
