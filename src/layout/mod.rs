mod architecture;
mod block;
mod c4;
mod error;
mod gantt;
mod gitgraph;
mod journey;
mod kanban;
pub(crate) mod label_placement;
mod mindmap;
mod pie;
mod quadrant;
mod radar;
mod ranking;
mod routing;
mod sankey;
mod sequence;
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
use journey::*;
use kanban::*;
use mindmap::*;
use pie::*;
use quadrant::*;
use radar::*;
use ranking::*;
use routing::*;
use sankey::*;
use sequence::*;
use text::*;
use timeline::*;
use treemap::*;
pub use types::*;
use xychart::*;

use crate::config::{LayoutConfig, PieRenderMode, TreemapRenderMode};
use crate::ir::{Direction, Graph};
use crate::text_metrics;
use crate::theme::{Theme, adjust_color, parse_color_to_hsl};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Instant;

// Label placement padding (resolved per diagram kind).
const LABEL_RANK_FONT_SCALE: f32 = 0.5;
const LABEL_RANK_MIN_GAP: f32 = 8.0;

// Minimum padding around the entire layout bounding box.
const LAYOUT_BOUNDARY_PAD: f32 = 8.0;
const PREFERRED_ASPECT_TOLERANCE: f32 = 0.02;
const PREFERRED_ASPECT_MAX_EXPANSION: f32 = 6.0;

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

#[derive(Clone)]
struct RouteLabelPlan {
    obstacle_id: String,
    obstacle_index: usize,
    progress: f32,
    center: (f32, f32),
}

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

fn is_region_subgraph(sub: &crate::ir::Subgraph) -> bool {
    sub.label.trim().is_empty()
        && sub
            .id
            .as_deref()
            .map(|id| id.starts_with("__region_"))
            .unwrap_or(false)
}

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
    stage_metrics.label_placement_us = stage_metrics
        .label_placement_us
        .saturating_add(label_start.elapsed().as_micros());

    (layout, stage_metrics)
}

fn adaptive_spacing_for_nodes(
    nodes: &BTreeMap<String, NodeLayout>,
    min_spacing: f32,
    max_spacing: f32,
) -> f32 {
    let mut total = 0.0f32;
    let mut count = 0usize;
    for node in nodes.values() {
        if node.hidden || node.anchor_subgraph.is_some() {
            continue;
        }
        total += (node.width + node.height) * 0.5;
        count += 1;
    }
    if count == 0 {
        return max_spacing;
    }
    let avg = total / count as f32;
    let target = (avg * 0.5).max(min_spacing);
    target.min(max_spacing)
}

fn compute_flowchart_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
    mut stage_metrics: Option<&mut LayoutStageMetrics>,
) -> Layout {
    let mut effective_config = config.clone();
    let mut hub_compaction_scale: Option<f32> = None;
    let mut hub_compaction_floor = 0.0f32;
    let mut prefer_direct_hub_routing = false;
    if graph.kind == crate::ir::DiagramKind::Requirement {
        effective_config.max_label_width_chars = effective_config.max_label_width_chars.max(32);
    }
    if graph.kind == crate::ir::DiagramKind::Er {
        // ER diagrams are relationship-dense; tighter packing improves readability
        // and significantly reduces long connector spans.
        effective_config.node_spacing *= 0.80;
        effective_config.rank_spacing *= 0.80;
        // Extra rank-order sweeps reduce crossing-prone left/right inversions
        // in dense relationship graphs.
        effective_config.flowchart.order_passes = effective_config.flowchart.order_passes.max(10);
    }
    if graph.kind == crate::ir::DiagramKind::Flowchart {
        let node_count = graph.nodes.len();
        let edge_count = graph.edges.len() as f32;
        let density = if node_count > 0 {
            edge_count / node_count as f32
        } else {
            0.0
        };
        let auto = &config.flowchart.auto_spacing;
        if auto.enabled && !auto.buckets.is_empty() {
            let mut scale = auto.buckets[0].scale;
            for bucket in &auto.buckets {
                if node_count >= bucket.min_nodes {
                    scale = bucket.scale;
                }
            }
            if density > auto.density_threshold {
                scale = scale.max(auto.dense_scale_floor);
            }
            effective_config.node_spacing =
                (effective_config.node_spacing * scale).max(auto.min_spacing);
            effective_config.rank_spacing =
                (effective_config.rank_spacing * scale).max(auto.min_spacing);
        }

        // Hub-and-spoke flowcharts (one high-degree node) tend to over-expand
        // with generic spacing and produce long radial connectors. Compress
        // spacing slightly when hub dominance is high.
        let mut degree_by_node: HashMap<&str, usize> = HashMap::new();
        for edge in &graph.edges {
            *degree_by_node.entry(edge.from.as_str()).or_insert(0) += 1;
            *degree_by_node.entry(edge.to.as_str()).or_insert(0) += 1;
        }
        let max_degree = degree_by_node.values().copied().max().unwrap_or(0) as f32;
        let hub_ratio = if node_count > 0 {
            max_degree / node_count as f32
        } else {
            0.0
        };
        if node_count >= 10 && hub_ratio >= 0.30 && density <= 3.0 {
            let hub_scale = (0.92 - (hub_ratio - 0.30) * 0.55).clamp(0.62, 0.92);
            hub_compaction_scale = Some(hub_scale);
            hub_compaction_floor = auto.min_spacing * 0.5;
        }
        if node_count >= 12 && hub_ratio >= 0.40 && density <= 2.5 {
            prefer_direct_hub_routing = true;
        }
    }
    let node_count = graph.nodes.len();
    let edge_count = graph.edges.len();
    let tiny_graph = graph.subgraphs.is_empty() && node_count <= 4 && edge_count <= 4;
    if tiny_graph {
        effective_config.flowchart.order_passes = 1;
        effective_config.flowchart.routing.enable_grid_router = false;
        effective_config.flowchart.routing.snap_ports_to_grid = false;
    }
    if prefer_direct_hub_routing {
        effective_config.flowchart.routing.enable_grid_router = false;
        effective_config.flowchart.routing.snap_ports_to_grid = false;
    }
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
        let style = resolve_node_style(node.id.as_str(), graph);
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

    let adaptive_node_spacing = adaptive_spacing_for_nodes(
        &nodes,
        effective_config.flowchart.auto_spacing.min_spacing,
        effective_config.node_spacing,
    );
    let adaptive_rank_spacing = adaptive_spacing_for_nodes(
        &nodes,
        effective_config.flowchart.auto_spacing.min_spacing,
        effective_config.rank_spacing,
    );
    if adaptive_node_spacing < effective_config.node_spacing {
        effective_config.node_spacing = adaptive_node_spacing;
    }
    if adaptive_rank_spacing < effective_config.rank_spacing {
        effective_config.rank_spacing = adaptive_rank_spacing;
    }
    if let Some(scale) = hub_compaction_scale {
        let floor = hub_compaction_floor.max(14.0);
        effective_config.node_spacing = (effective_config.node_spacing * scale).max(floor);
        effective_config.rank_spacing = (effective_config.rank_spacing * scale).max(floor);
    }

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
    assign_positions_manual(
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

    if !graph.subgraphs.is_empty() {
        if graph.kind != crate::ir::DiagramKind::State {
            apply_subgraph_direction_overrides(graph, &mut nodes, config, &anchored_indices);
        }
        if !anchor_info.is_empty() {
            let _anchored_nodes =
                align_subgraphs_to_anchor_nodes(graph, &anchor_info, &mut nodes, config);
        }
        if graph.kind == crate::ir::DiagramKind::State && !anchor_info.is_empty() {
            apply_state_subgraph_layouts(graph, &mut nodes, config, &anchored_indices);
        }
        apply_orthogonal_region_bands(graph, &mut nodes, config);
        if graph.kind != crate::ir::DiagramKind::State {
            apply_subgraph_bands(graph, &mut nodes, config);
        }
    }

    compress_linear_subgraphs(graph, &mut nodes, config);
    enforce_top_level_subgraph_gap(graph, &mut nodes, theme, config);

    // Separate overlapping sibling subgraphs
    separate_sibling_subgraphs(graph, &mut nodes, theme, config);
    align_disconnected_top_level_subgraphs(graph, &mut nodes);
    align_disconnected_components(graph, &mut nodes, config);
    apply_visual_objectives(graph, &layout_edges, &mut nodes, theme, &effective_config);

    // For state diagrams, push non-member nodes outside subgraph bounds
    if graph.kind == crate::ir::DiagramKind::State && !graph.subgraphs.is_empty() {
        push_non_members_out_of_subgraphs(graph, &mut nodes, theme, config);
    }

    let mut subgraphs = build_subgraph_layouts(graph, &nodes, theme, config);
    apply_subgraph_anchors(graph, &subgraphs, &mut nodes);
    let obstacles = build_obstacles(&nodes, &subgraphs, config);
    let label_obstacles = build_label_obstacles_for_routing(&nodes, &subgraphs);
    let routing_grid = if config.flowchart.routing.enable_grid_router && !tiny_graph {
        build_routing_grid(&obstacles, config)
    } else {
        None
    };
    let port_assignment_start = Instant::now();
    let mut node_degrees: HashMap<String, usize> = HashMap::new();
    for edge in &graph.edges {
        *node_degrees.entry(edge.from.clone()).or_insert(0) += 1;
        *node_degrees.entry(edge.to.clone()).or_insert(0) += 1;
    }
    let mut side_loads: HashMap<String, [usize; 4]> = HashMap::new();
    let mut edge_ports: Vec<EdgePortInfo> = Vec::with_capacity(graph.edges.len());
    let mut port_candidates: HashMap<(String, EdgeSide), Vec<PortCandidate>> = HashMap::new();
    let mut side_choice_segments: Vec<Segment> = Vec::with_capacity(graph.edges.len());
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
        let use_balanced_sides = !matches!(graph.kind, crate::ir::DiagramKind::Architecture);
        let from_degree = node_degrees.get(&edge.from).copied().unwrap_or(0);
        let to_degree = node_degrees.get(&edge.to).copied().unwrap_or(0);
        let allow_low_degree_balancing =
            edge.style == crate::ir::EdgeStyle::Dotted && from_degree <= 4 && to_degree <= 4;
        let primary_sides = edge_sides(from, to, graph.direction);
        let mut selected_sides = if use_balanced_sides {
            edge_sides_balanced(
                &edge.from,
                &edge.to,
                from,
                to,
                allow_low_degree_balancing,
                graph.direction,
                &node_degrees,
                &side_loads,
            )
        } else {
            primary_sides
        };
        if use_balanced_sides
            && (selected_sides.0 != primary_sides.0 || selected_sides.1 != primary_sides.1)
        {
            let candidate_points = [
                anchor_point_for_node(from, selected_sides.0, 0.0),
                anchor_point_for_node(to, selected_sides.1, 0.0),
            ];
            let primary_points = [
                anchor_point_for_node(from, primary_sides.0, 0.0),
                anchor_point_for_node(to, primary_sides.1, 0.0),
            ];
            let (candidate_crossings, _) =
                edge_crossings_with_existing(&candidate_points, &side_choice_segments);
            let (primary_crossings, _) =
                edge_crossings_with_existing(&primary_points, &side_choice_segments);
            if candidate_crossings > primary_crossings {
                selected_sides = primary_sides;
            }
        }
        let (start_side, end_side, _is_backward) = selected_sides;
        bump_side_load(&mut side_loads, &edge.from, start_side);
        bump_side_load(&mut side_loads, &edge.to, end_side);
        edge_ports.push(EdgePortInfo {
            start_side,
            end_side,
            start_offset: 0.0,
            end_offset: 0.0,
        });

        let from_anchor = anchor_point_for_node(from, start_side, 0.0);
        let to_anchor = anchor_point_for_node(to, end_side, 0.0);
        // Compute the ideal port position: where a straight line from the
        // remote anchor to this node's centre crosses the node boundary on
        // the given side.  This produces positions in the node's coordinate
        // space, so ports naturally cluster where the geometry dictates
        // rather than being spread across the full node width.
        let start_other = ideal_port_pos((to_anchor.0, to_anchor.1), from, start_side);
        let end_other = ideal_port_pos((from_anchor.0, from_anchor.1), to, end_side);
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
        side_choice_segments.push((from_anchor, to_anchor));
    }
    let routing_cell = routing_cell_size(config);
    for ((node_id, side), candidates) in port_candidates {
        let Some(node) = nodes.get(&node_id) else {
            continue;
        };
        let mut min_other = f32::MAX;
        let mut max_other = f32::MIN;
        for candidate in &candidates {
            min_other = min_other.min(candidate.other_pos);
            max_other = max_other.max(candidate.other_pos);
        }
        let span = (max_other - min_other).max(0.0);
        let mut order: Vec<usize> = (0..candidates.len()).collect();
        order.sort_by(|&a, &b| {
            candidates[a]
                .other_pos
                .partial_cmp(&candidates[b].other_pos)
                .unwrap_or(Ordering::Equal)
        });
        let node_len = if side_is_vertical(side) {
            node.height
        } else {
            node.width
        };
        let pad = (node_len * config.flowchart.port_pad_ratio)
            .min(config.flowchart.port_pad_max)
            .max(config.flowchart.port_pad_min);
        let usable = (node_len - 2.0 * pad).max(1.0);
        let min_sep = usable / (candidates.len() as f32 + 1.0);
        let snap_to_grid = config.flowchart.routing.snap_ports_to_grid
            && routing_cell > 0.0
            && min_sep >= routing_cell * 0.75;
        // other_pos is now an ideal port coordinate (x or y) in absolute
        // space.  Normalise it within the node's usable range so that ports
        // land where straight-line geometry dictates.
        let node_start = if side_is_vertical(side) {
            node.y
        } else {
            node.x
        };
        let ideal_span = span; // span of ideal positions across the node
        let span_frac = if usable > 1.0 {
            (ideal_span / usable).min(2.0)
        } else {
            1.0
        };
        let position_weight = (0.5 + 0.35 * span_frac).clamp(0.50, 0.85);
        let rank_weight = 1.0 - position_weight;
        let desired: Vec<(usize, f32)> = order
            .iter()
            .enumerate()
            .map(|(rank, &idx)| {
                let candidate = &candidates[idx];
                let pos_in_node = candidate.other_pos - node_start;
                let t_pos = ((pos_in_node - pad) / usable).clamp(0.0, 1.0);
                let t_rank = (rank as f32 + 0.5) / candidates.len() as f32;
                let t = t_pos * position_weight + t_rank * rank_weight;
                let pos = pad + t * usable;
                (idx, pos)
            })
            .collect();
        let mut assigned = vec![0.0; candidates.len()];
        let mut prev = pad;
        for (order_idx, (cand_idx, pos)) in desired.iter().enumerate() {
            let mut p = *pos;
            if order_idx == 0 {
                p = p.max(pad);
            } else {
                p = p.max(prev + min_sep);
            }
            assigned[*cand_idx] = p;
            prev = p;
        }
        let mut next = pad + usable;
        for (order_idx, (cand_idx, _pos)) in desired.iter().enumerate().rev() {
            let mut p = assigned[*cand_idx];
            if order_idx + 1 == desired.len() {
                p = p.min(next);
            } else {
                p = p.min(next - min_sep);
            }
            assigned[*cand_idx] = p;
            next = p;
        }
        for (rank, &cand_idx) in order.iter().enumerate() {
            let candidate = &candidates[cand_idx];
            let mut offset = assigned[cand_idx] - node_len / 2.0;
            if snap_to_grid {
                offset = (offset / routing_cell).round() * routing_cell;
            }
            if config.flowchart.port_side_bias != 0.0 {
                offset += config.flowchart.port_side_bias
                    * (rank as f32 - (candidates.len() as f32 - 1.0) / 2.0);
            }
            if let Some(info) = edge_ports.get_mut(candidate.edge_idx) {
                if candidate.is_start {
                    info.start_offset = offset;
                } else {
                    info.end_offset = offset;
                }
            }
        }
    }
    if let Some(metrics) = stage_metrics.as_deref_mut() {
        metrics.port_assignment_us = metrics
            .port_assignment_us
            .saturating_add(port_assignment_start.elapsed().as_micros());
    }

    let edge_routing_start = Instant::now();
    let pair_counts = build_edge_pair_counts(&graph.edges);
    let mut pair_seen: HashMap<(String, String), usize> = HashMap::new();
    let mut pair_index: Vec<usize> = vec![0; graph.edges.len()];
    for (idx, edge) in graph.edges.iter().enumerate() {
        let key = edge_pair_key(edge);
        let seen = pair_seen.entry(key).or_insert(0usize);
        pair_index[idx] = *seen;
        *seen += 1;
    }

    let mut cross_edge_offsets = vec![0.0f32; graph.edges.len()];
    if graph.kind == crate::ir::DiagramKind::Flowchart {
        let is_horizontal_layout = is_horizontal(graph.direction);
        let band_size = (config.node_spacing * 2.0).max(30.0);
        let mut groups: HashMap<i32, Vec<(usize, f32)>> = HashMap::new();
        for (idx, edge) in graph.edges.iter().enumerate() {
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
            let from_center = (from.x + from.width / 2.0, from.y + from.height / 2.0);
            let to_center = (to.x + to.width / 2.0, to.y + to.height / 2.0);
            let dx = to_center.0 - from_center.0;
            let dy = to_center.1 - from_center.1;
            let cross_axis = if is_horizontal_layout {
                dy.abs()
            } else {
                dx.abs()
            };
            let main_axis = if is_horizontal_layout {
                dx.abs()
            } else {
                dy.abs()
            };
            let is_secondary = edge.style == crate::ir::EdgeStyle::Dotted || edge.label.is_some();
            if !is_secondary || cross_axis <= main_axis * 1.2 {
                continue;
            }
            let band_coord = if is_horizontal_layout {
                (from_center.0 + to_center.0) * 0.5
            } else {
                (from_center.1 + to_center.1) * 0.5
            };
            let bucket = (band_coord / band_size).round() as i32;
            let sort_key = if is_horizontal_layout {
                (from_center.1 + to_center.1) * 0.5
            } else {
                (from_center.0 + to_center.0) * 0.5
            };
            groups.entry(bucket).or_default().push((idx, sort_key));
        }
        let spacing = (config.node_spacing * 0.45).max(8.0);
        for (_bucket, mut group) in groups {
            if group.len() <= 1 {
                continue;
            }
            group.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
            let center = (group.len() as f32 - 1.0) * 0.5;
            for (pos, (idx, _)) in group.iter().enumerate() {
                cross_edge_offsets[*idx] = (pos as f32 - center) * spacing;
            }
        }
    }

    let mut route_order: Vec<(u8, f32, f32, usize)> = Vec::with_capacity(graph.edges.len());
    let dense_flowchart_routing = graph.kind == crate::ir::DiagramKind::Flowchart
        && graph.edges.len() >= 18
        && graph.edges.len() * 2 >= layout_node_ids.len() * 3;
    for (idx, edge) in graph.edges.iter().enumerate() {
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
        let from_center = (from.x + from.width / 2.0, from.y + from.height / 2.0);
        let to_center = (to.x + to.width / 2.0, to.y + to.height / 2.0);
        let dx = to_center.0 - from_center.0;
        let dy = to_center.1 - from_center.1;
        let cross_axis = if is_horizontal(graph.direction) {
            dy.abs()
        } else {
            dx.abs()
        };
        let main_axis = if is_horizontal(graph.direction) {
            dx.abs()
        } else {
            dy.abs()
        };
        let (_, _, is_backward) = edge_sides(from, to, graph.direction);
        let is_dotted = edge.style == crate::ir::EdgeStyle::Dotted;
        let has_label = edge.label.is_some();
        let is_secondary = is_dotted || has_label;
        let has_open_triangle = matches!(
            edge.arrow_start_kind,
            Some(crate::ir::EdgeArrowhead::OpenTriangle)
        ) || matches!(
            edge.arrow_end_kind,
            Some(crate::ir::EdgeArrowhead::OpenTriangle)
        );
        let priority = if graph.kind == crate::ir::DiagramKind::Class {
            if has_open_triangle {
                0u8
            } else if is_secondary {
                2u8
            } else if is_backward {
                1u8
            } else {
                1u8
            }
        } else if graph.kind == crate::ir::DiagramKind::State {
            // State machines often have long back-edges to earlier states.
            // Route those first so later local transitions can avoid them.
            if is_backward {
                0u8
            } else if has_label || is_dotted {
                1u8
            } else {
                2u8
            }
        } else if is_dotted {
            if dense_flowchart_routing { 1u8 } else { 2u8 }
        } else if has_label || is_backward {
            1u8
        } else {
            0u8
        };
        route_order.push((priority, cross_axis, main_axis, idx));
    }
    let steep_count = route_order
        .iter()
        .filter(|(_, cross_axis, main_axis, _)| *cross_axis > *main_axis * 0.8)
        .count();
    let use_cross_axis_order = graph.edges.len() >= 10 && steep_count * 4 >= graph.edges.len();
    if use_cross_axis_order {
        route_order.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))
                .then_with(|| b.2.partial_cmp(&a.2).unwrap_or(Ordering::Equal))
                .then_with(|| a.3.cmp(&b.3))
        });
    } else {
        let use_priority_preorder = graph.edges.len() >= 10;
        route_order.sort_by(|a, b| {
            let len_a = a.1 * a.1 + a.2 * a.2;
            let len_b = b.1 * b.1 + b.2 * b.2;
            let by_length = len_b.partial_cmp(&len_a).unwrap_or(Ordering::Equal);
            if use_priority_preorder {
                a.0.cmp(&b.0)
                    .then_with(|| by_length)
                    .then_with(|| a.3.cmp(&b.3))
            } else {
                by_length.then_with(|| a.3.cmp(&b.3))
            }
        });
    }

    let mut routed_points: Vec<Vec<(f32, f32)>> = vec![Vec::new(); graph.edges.len()];
    let use_occupancy = !tiny_graph && graph.edges.len() > 2;
    let mut edge_occupancy = if use_occupancy {
        Some(EdgeOccupancy::new(
            config.node_spacing.max(MIN_NODE_SPACING_FLOOR) * EDGE_OCCUPANCY_CELL_RATIO,
        ))
    } else {
        None
    };
    let has_label_dummies = nodes
        .keys()
        .any(|id| id.starts_with("__elabel_") && id.ends_with("__"));
    let mut route_label_obstacles = label_obstacles;
    let (edge_label_pad_x, edge_label_pad_y) =
        label_placement::edge_label_padding(graph.kind, config);
    let mut route_label_plans: Vec<Option<RouteLabelPlan>> = vec![None; graph.edges.len()];
    if !has_label_dummies {
        for idx in 0..graph.edges.len() {
            let Some(label) = edge_route_labels.get(idx).and_then(|label| label.as_ref()) else {
                continue;
            };
            if label.width <= 0.0 || label.height <= 0.0 {
                continue;
            }
            let edge = &graph.edges[idx];
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
            let port_info = edge_ports
                .get(idx)
                .copied()
                .expect("edge port info missing");
            let start = anchor_point_for_node(from, port_info.start_side, port_info.start_offset);
            let end = anchor_point_for_node(to, port_info.end_side, port_info.end_offset);

            let key = edge_pair_key(edge);
            let total = *pair_counts.get(&key).unwrap_or(&1) as f32;
            let idx_in_pair = pair_index[idx] as f32;
            let mut base_offset = if total > 1.0 {
                (idx_in_pair - (total - 1.0) / 2.0)
                    * (config.node_spacing * MULTI_EDGE_OFFSET_RATIO)
            } else {
                0.0
            } + cross_edge_offsets[idx];
            if graph.kind == crate::ir::DiagramKind::Flowchart {
                let raw_bias = (port_info.start_offset - port_info.end_offset)
                    * FLOWCHART_PORT_ROUTE_BIAS_RATIO;
                let max_bias = (config.node_spacing * FLOWCHART_PORT_ROUTE_BIAS_MAX_RATIO).max(8.0);
                base_offset += raw_bias.clamp(-max_bias, max_bias);
            }

            let mut center = ((start.0 + end.0) * 0.5, (start.1 + end.1) * 0.5);
            if is_horizontal(graph.direction) {
                center.0 += base_offset;
            } else {
                center.1 += base_offset;
            }
            let obstacle_id = format!("edge-label-reserved:{idx}");
            let obstacle_index = route_label_obstacles.len();
            route_label_obstacles.push(Obstacle {
                id: obstacle_id.clone(),
                x: center.0 - label.width / 2.0 - edge_label_pad_x,
                y: center.1 - label.height / 2.0 - edge_label_pad_y,
                width: label.width + 2.0 * edge_label_pad_x,
                height: label.height + 2.0 * edge_label_pad_y,
                members: None,
            });
            route_label_plans[idx] = Some(RouteLabelPlan {
                obstacle_id,
                obstacle_index,
                progress: 0.5,
                center,
            });
        }
    }
    let mut existing_segments: Vec<Segment> = Vec::new();
    let mut label_anchors: Vec<Option<(f32, f32)>> = vec![None; graph.edges.len()];
    for (_, _, _, idx) in &route_order {
        let edge = &graph.edges[*idx];
        let key = edge_pair_key(edge);
        let total = *pair_counts.get(&key).unwrap_or(&1) as f32;
        let idx_in_pair = pair_index[*idx] as f32;
        let mut base_offset = if total > 1.0 {
            (idx_in_pair - (total - 1.0) / 2.0) * (config.node_spacing * MULTI_EDGE_OFFSET_RATIO)
        } else {
            0.0
        } + cross_edge_offsets[*idx];
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
        let port_info = edge_ports
            .get(*idx)
            .copied()
            .expect("edge port info missing");
        if graph.kind == crate::ir::DiagramKind::Flowchart {
            let raw_bias =
                (port_info.start_offset - port_info.end_offset) * FLOWCHART_PORT_ROUTE_BIAS_RATIO;
            let max_bias = (config.node_spacing * FLOWCHART_PORT_ROUTE_BIAS_MAX_RATIO).max(8.0);
            base_offset += raw_bias.clamp(-max_bias, max_bias);
        }
        let default_stub = port_stub_length(config, from, to);
        let stub_len = match graph.kind {
            crate::ir::DiagramKind::Class
            | crate::ir::DiagramKind::Er
            | crate::ir::DiagramKind::Requirement => 0.0,
            _ => default_stub,
        };
        let max_edge_label_chars = [
            edge.label.as_deref(),
            edge.start_label.as_deref(),
            edge.end_label.as_deref(),
        ]
        .into_iter()
        .flatten()
        .map(|label| label.chars().count())
        .max()
        .unwrap_or(0);
        let has_endpoint_label = edge.start_label.is_some() || edge.end_label.is_some();
        let avoid_short_tie = graph.kind == crate::ir::DiagramKind::Flowchart
            && (has_endpoint_label
                || max_edge_label_chars >= FLOWCHART_EDGE_LABEL_WRAP_TRIGGER_CHARS);
        let preferred_label_id = route_label_plans
            .get(*idx)
            .and_then(|plan| plan.as_ref())
            .map(|plan| plan.obstacle_id.as_str());
        let preferred_label_center = if matches!(
            graph.kind,
            crate::ir::DiagramKind::Flowchart | crate::ir::DiagramKind::State
        ) {
            None
        } else {
            route_label_plans
                .get(*idx)
                .and_then(|plan| plan.as_ref())
                .map(|plan| plan.center)
        };
        let (start_inset, end_inset) = {
            let start = if edge.arrow_start {
                crate::render::arrowhead_inset(graph.kind, edge.arrow_start_kind)
            } else {
                0.0
            };
            let end = if edge.arrow_end {
                crate::render::arrowhead_inset(graph.kind, edge.arrow_end_kind)
            } else {
                0.0
            };
            (start, end)
        };
        let route_ctx = RouteContext {
            from_id: &edge.from,
            to_id: &edge.to,
            from,
            to,
            direction: graph.direction,
            config,
            obstacles: &obstacles,
            label_obstacles: &route_label_obstacles,
            fast_route: tiny_graph,
            base_offset,
            start_side: port_info.start_side,
            end_side: port_info.end_side,
            start_offset: port_info.start_offset,
            end_offset: port_info.end_offset,
            stub_len,
            start_inset,
            end_inset,
            prefer_shorter_ties: !avoid_short_tie,
            preferred_label_id,
            preferred_label_center,
        };
        let use_existing_for_edge = !(matches!(
            graph.kind,
            crate::ir::DiagramKind::Class | crate::ir::DiagramKind::Er
        ) && edge.style == crate::ir::EdgeStyle::Dotted);
        let existing_for_edge = if use_existing_for_edge {
            Some(existing_segments.as_slice())
        } else {
            None
        };
        let mut points = route_edge_with_avoidance(
            &route_ctx,
            edge_occupancy.as_ref(),
            routing_grid.as_ref(),
            existing_for_edge,
        );
        if matches!(
            graph.kind,
            crate::ir::DiagramKind::Class | crate::ir::DiagramKind::Er
        ) {
            let fast_ctx = RouteContext {
                from_id: route_ctx.from_id,
                to_id: route_ctx.to_id,
                from: route_ctx.from,
                to: route_ctx.to,
                direction: route_ctx.direction,
                config: route_ctx.config,
                obstacles: route_ctx.obstacles,
                label_obstacles: route_ctx.label_obstacles,
                fast_route: true,
                base_offset: route_ctx.base_offset,
                start_side: route_ctx.start_side,
                end_side: route_ctx.end_side,
                start_offset: route_ctx.start_offset,
                end_offset: route_ctx.end_offset,
                stub_len: route_ctx.stub_len,
                start_inset: route_ctx.start_inset,
                end_inset: route_ctx.end_inset,
                prefer_shorter_ties: route_ctx.prefer_shorter_ties,
                preferred_label_id: route_ctx.preferred_label_id,
                preferred_label_center: route_ctx.preferred_label_center,
            };
            let fast_points = route_edge_with_avoidance(&fast_ctx, None, None, existing_for_edge);
            let fast_hits = path_obstacle_intersections(
                &fast_points,
                route_ctx.obstacles,
                route_ctx.from_id,
                route_ctx.to_id,
            );
            let fast_label_hits = path_label_intersections(
                &fast_points,
                route_ctx.label_obstacles,
                route_ctx.preferred_label_id,
            );
            if fast_hits == 0 && fast_label_hits == 0 {
                let (fast_cross, fast_overlap) =
                    edge_crossings_with_existing(&fast_points, &existing_segments);
                let (cur_cross, cur_overlap) =
                    edge_crossings_with_existing(&points, &existing_segments);
                if fast_cross < cur_cross
                    || (fast_cross == cur_cross && fast_overlap + 0.25 < cur_overlap)
                {
                    points = fast_points;
                }
            }
        }
        if !has_label_dummies
            && let Some(plan) = route_label_plans
                .get_mut(*idx)
                .and_then(|plan| plan.as_mut())
        {
            let label_center = path_point_at_progress(&points, plan.progress)
                .or_else(|| edge_label_anchor_from_points(&points))
                .unwrap_or(plan.center);
            plan.center = label_center;
            label_anchors[*idx] = Some(label_center);
            if graph.kind != crate::ir::DiagramKind::State && points.len() >= 2 {
                insert_label_via_point(&mut points, label_center, graph.direction);
            }
            if let Some(label) = edge_route_labels.get(*idx).and_then(|label| label.as_ref())
                && let Some(obstacle) = route_label_obstacles.get_mut(plan.obstacle_index)
            {
                obstacle.x = label_center.0 - label.width / 2.0 - edge_label_pad_x;
                obstacle.y = label_center.1 - label.height / 2.0 - edge_label_pad_y;
                obstacle.width = label.width + 2.0 * edge_label_pad_x;
                obstacle.height = label.height + 2.0 * edge_label_pad_y;
            }
        }
        if let Some(occ) = edge_occupancy.as_mut() {
            occ.add_path(&points);
        }
        if points.len() >= 2 {
            for segment in points.windows(2) {
                existing_segments.push((segment[0], segment[1]));
            }
        }
        routed_points[*idx] = points;
    }

    if graph.kind == crate::ir::DiagramKind::Flowchart {
        reduce_orthogonal_path_crossings(graph, &nodes, &mut routed_points, config);
        deoverlap_flowchart_paths(graph, &nodes, &mut routed_points, config);
    } else if matches!(
        graph.kind,
        crate::ir::DiagramKind::Class | crate::ir::DiagramKind::Er | crate::ir::DiagramKind::State
    ) {
        reduce_orthogonal_path_crossings(graph, &nodes, &mut routed_points, config);
        if graph.kind == crate::ir::DiagramKind::Er {
            deoverlap_flowchart_paths(graph, &nodes, &mut routed_points, config);
        }
    }

    // Global post-routing passes (crossing reduction/deoverlap) can move paths
    // after we seeded label anchors. Re-apply the reserved label via-points so
    // center labels stay attached to their owning edge paths.
    if !has_label_dummies {
        for idx in 0..routed_points.len() {
            let Some(plan) = route_label_plans
                .get_mut(idx)
                .and_then(|plan| plan.as_mut())
            else {
                continue;
            };
            let points = &mut routed_points[idx];
            if points.len() < 2 {
                continue;
            }
            let refreshed_center = path_point_at_progress(points, plan.progress)
                .or_else(|| edge_label_anchor_from_points(points))
                .unwrap_or(plan.center);
            plan.center = refreshed_center;
            if graph.kind != crate::ir::DiagramKind::State {
                insert_label_via_point(points, refreshed_center, graph.direction);
            }
            label_anchors[idx] = Some(refreshed_center);
        }
    }

    // Insert label dummy via-points so edges pass through label positions.
    // For each edge with a label dummy, insert the dummy center into the
    // routed path at the correct main-axis position.
    for (idx, dummy_id_opt) in label_dummy_ids.iter().enumerate() {
        let Some(dummy_id) = dummy_id_opt else {
            continue;
        };
        let Some(dummy_node) = nodes.get(dummy_id) else {
            continue;
        };
        let cx = dummy_node.x + dummy_node.width / 2.0;
        let cy = dummy_node.y + dummy_node.height / 2.0;
        label_anchors[idx] = Some((cx, cy));

        let points = &mut routed_points[idx];
        if graph.kind != crate::ir::DiagramKind::State && points.len() >= 2 {
            insert_label_via_point(points, (cx, cy), graph.direction);
        }
    }
    if let Some(metrics) = stage_metrics.as_deref_mut() {
        metrics.edge_routing_us = metrics
            .edge_routing_us
            .saturating_add(edge_routing_start.elapsed().as_micros());
    }

    let mut edges = Vec::new();
    for (idx, edge) in graph.edges.iter().enumerate() {
        let label = edge_route_labels[idx].clone();
        let start_label = edge_start_labels[idx].clone();
        let end_label = edge_end_labels[idx].clone();
        let mut override_style = resolve_edge_style(idx, graph);
        if graph.kind == crate::ir::DiagramKind::Requirement {
            if override_style.stroke.is_none() {
                override_style.stroke = Some(config.requirement.edge_stroke.clone());
            }
            override_style.stroke_width = Some(
                override_style
                    .stroke_width
                    .unwrap_or(config.requirement.edge_stroke_width),
            );
            if override_style.dasharray.is_none() {
                override_style.dasharray = Some(config.requirement.edge_dasharray.clone());
            }
            if override_style.label_color.is_none() {
                override_style.label_color = Some(config.requirement.edge_label_color.clone());
            }
        }
        edges.push(EdgeLayout {
            from: edge.from.clone(),
            to: edge.to.clone(),
            label,
            start_label,
            end_label,
            points: routed_points[idx].clone(),
            directed: edge.directed,
            arrow_start: edge.arrow_start,
            arrow_end: edge.arrow_end,
            arrow_start_kind: edge.arrow_start_kind,
            arrow_end_kind: edge.arrow_end_kind,
            start_decoration: edge.start_decoration,
            end_decoration: edge.end_decoration,
            style: edge.style,
            override_style,
            label_anchor: label_anchors[idx],
            start_label_anchor: None,
            end_label_anchor: None,
        });
    }

    if matches!(graph.direction, Direction::RightLeft | Direction::BottomTop) {
        apply_direction_mirror(graph.direction, &mut nodes, &mut edges, &mut subgraphs);
    }

    normalize_layout(&mut nodes, &mut edges, &mut subgraphs);
    let mut state_notes = Vec::new();
    if graph.kind == crate::ir::DiagramKind::State && !graph.state_notes.is_empty() {
        let note_pad_x = theme.font_size * STATE_NOTE_PAD_X_SCALE;
        let note_pad_y = theme.font_size * STATE_NOTE_PAD_Y_SCALE;
        let note_gap = (theme.font_size * STATE_NOTE_GAP_SCALE).max(STATE_NOTE_GAP_MIN);
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
    let (mut max_x, mut max_y) = bounds_with_edges(&nodes, &subgraphs, &edges);
    for note in &state_notes {
        max_x = max_x.max(note.x + note.width);
        max_y = max_y.max(note.y + note.height);
    }
    let width = max_x + LAYOUT_BOUNDARY_PAD;
    let height = max_y + LAYOUT_BOUNDARY_PAD;

    Layout {
        kind: graph.kind,
        nodes,
        edges,
        subgraphs,
        width,
        height,
        diagram: DiagramData::Graph { state_notes },
    }
}

fn assign_positions_manual(
    graph: &Graph,
    layout_node_ids: &[String],
    layout_set: &HashSet<String>,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
    layout_edges: &[crate::ir::Edge],
    theme: &Theme,
    pre_measured_labels: &[Option<TextBlock>],
    label_dummy_ids: &mut Vec<Option<String>>,
) {
    let mut edge_labels_vec: Vec<Option<TextBlock>> = Vec::new();
    let mut original_edge_indices: Vec<usize> = Vec::new();
    let layout_edges: Vec<crate::ir::Edge> = layout_edges
        .iter()
        .enumerate()
        .filter(|(_, edge)| layout_set.contains(&edge.from) && layout_set.contains(&edge.to))
        .map(|(i, edge)| {
            edge_labels_vec.push(pre_measured_labels.get(i).cloned().unwrap_or(None));
            original_edge_indices.push(i);
            edge.clone()
        })
        .collect();
    let edge_labels = edge_labels_vec;
    let rank_edges = rank_edges_for_manual_layout(graph, layout_node_ids, &layout_edges);
    let mut ranks = compute_ranks_subset(layout_node_ids, &rank_edges, &graph.node_order);
    if graph.kind == crate::ir::DiagramKind::Class {
        let mut hierarchy_nodes: HashSet<String> = HashSet::new();
        for edge in &layout_edges {
            let has_open_triangle = matches!(
                edge.arrow_start_kind,
                Some(crate::ir::EdgeArrowhead::OpenTriangle)
            ) || matches!(
                edge.arrow_end_kind,
                Some(crate::ir::EdgeArrowhead::OpenTriangle)
            );
            if has_open_triangle {
                hierarchy_nodes.insert(edge.from.clone());
                hierarchy_nodes.insert(edge.to.clone());
            }
        }
        if !hierarchy_nodes.is_empty() {
            let min_hierarchy_rank = hierarchy_nodes
                .iter()
                .filter_map(|id| ranks.get(id).copied())
                .min()
                .unwrap_or(0);
            let mut pending_updates: Vec<(String, usize)> = Vec::new();
            for node_id in layout_node_ids {
                if hierarchy_nodes.contains(node_id) {
                    continue;
                }
                let mut sum = 0.0f32;
                let mut count = 0usize;
                for edge in &layout_edges {
                    if edge.from == *node_id {
                        if let Some(rank) = ranks.get(&edge.to) {
                            sum += *rank as f32;
                            count += 1;
                        }
                    } else if edge.to == *node_id
                        && let Some(rank) = ranks.get(&edge.from)
                    {
                        sum += *rank as f32;
                        count += 1;
                    }
                }
                if count == 0 {
                    continue;
                }
                let avg_neighbor_rank = (sum / count as f32).round().max(0.0) as usize;
                let target_rank = avg_neighbor_rank.max(min_hierarchy_rank + 1);
                let current_rank = ranks.get(node_id).copied().unwrap_or(0);
                if target_rank > current_rank {
                    pending_updates.push((node_id.clone(), target_rank));
                }
            }
            for (node_id, rank) in pending_updates {
                ranks.insert(node_id, rank);
            }
        }
    }
    let mut max_rank = 0usize;
    for rank in ranks.values() {
        max_rank = max_rank.max(*rank);
    }
    let mut rank_nodes: Vec<Vec<String>> = vec![Vec::new(); max_rank + 1];
    for node_id in layout_node_ids {
        let rank = *ranks.get(node_id).unwrap_or(&0);
        if let Some(bucket) = rank_nodes.get_mut(rank) {
            bucket.push(node_id.clone());
        }
    }

    let use_label_dummies = !matches!(
        graph.kind,
        crate::ir::DiagramKind::Flowchart
            | crate::ir::DiagramKind::Class
            | crate::ir::DiagramKind::Er
            | crate::ir::DiagramKind::Requirement
            | crate::ir::DiagramKind::State
    );
    // Collect gaps (original rank index) where at least one labeled forward edge exists.
    let gaps_needing_label_rank: Vec<usize> = if use_label_dummies {
        let mut gap_set: HashSet<usize> = HashSet::new();
        for (idx, edge) in layout_edges.iter().enumerate() {
            if edge_labels[idx].is_none() {
                continue;
            }
            let from_rank = ranks.get(&edge.from).copied().unwrap_or(0);
            let to_rank = ranks.get(&edge.to).copied().unwrap_or(0);
            // Forward edges: insert label rank in the gap.
            // Back-edges (to_rank <= from_rank): insert label rank in the gap too,
            // using min/max so both directions share the same label rank.
            let lo = from_rank.min(to_rank);
            let hi = from_rank.max(to_rank);
            if hi > lo {
                // For span-1 edges, the gap index is lo.
                // For longer spans, use the midpoint gap.
                let mid_gap = lo + (hi - lo - 1) / 2;
                gap_set.insert(mid_gap);
            }
        }
        let mut v: Vec<usize> = gap_set.into_iter().collect();
        v.sort();
        v
    } else {
        Vec::new()
    };

    // Build a rank shift table: for each original rank r, the new rank is r + shift[r].
    let mut rank_shift: Vec<usize> = vec![0; max_rank + 2];
    {
        let mut cumulative = 0;
        for r in 0..=max_rank {
            rank_shift[r] = cumulative;
            if gaps_needing_label_rank.contains(&r) {
                cumulative += 1;
            }
        }
        rank_shift[max_rank + 1] = cumulative;
    }
    let total_new_ranks = if gaps_needing_label_rank.is_empty() {
        0
    } else {
        rank_shift[max_rank + 1]
    };

    // Apply rank shifts: expand rank_nodes to accommodate new label ranks.
    if total_new_ranks > 0 {
        let new_max_rank = max_rank + total_new_ranks;
        let mut new_rank_nodes: Vec<Vec<String>> = vec![Vec::new(); new_max_rank + 1];
        for (old_rank, bucket) in rank_nodes.iter().enumerate() {
            let new_rank = old_rank + rank_shift[old_rank];
            new_rank_nodes[new_rank] = bucket.clone();
        }
        rank_nodes = new_rank_nodes;
    }

    // Create label dummy nodes in the inserted label ranks.
    let mut label_dummy_ranks: HashSet<usize> = HashSet::new();
    let mut order_map = graph.node_order.clone();
    let mut dummy_counter = 0usize;

    if use_label_dummies {
        for (idx, edge) in layout_edges.iter().enumerate() {
            let Some(label) = &edge_labels[idx] else {
                continue;
            };
            let from_rank = ranks.get(&edge.from).copied().unwrap_or(0);
            let to_rank = ranks.get(&edge.to).copied().unwrap_or(0);
            let lo = from_rank.min(to_rank);
            let hi = from_rank.max(to_rank);
            if hi <= lo {
                continue;
            }
            let mid_gap = lo + (hi - lo - 1) / 2;
            // The label rank is the new rank inserted after the shifted gap position.
            let label_rank = mid_gap + rank_shift[mid_gap] + 1;
            label_dummy_ranks.insert(label_rank);

            let dummy_id = format!("__elabel_{}_{}_{dummy_counter}__", edge.from, edge.to);
            dummy_counter += 1;
            let order_idx = order_map.len();
            order_map.insert(dummy_id.clone(), order_idx);

            // Determine dimensions: for horizontal layouts, main-axis = width, cross-axis = height.
            // Cap the main-axis size so long edge labels don't explode rank spacing.
            let label_main_cap = (theme.font_size * 8.0).max(config.node_spacing * 1.3);
            let (raw_main, raw_cross) = if is_horizontal(graph.direction) {
                (label.width, label.height)
            } else {
                (label.height, label.width)
            };
            let main_dim = if raw_main > 0.0 {
                raw_main.min(label_main_cap)
            } else {
                raw_main
            };
            let cross_dim = raw_cross;

            nodes.insert(
                dummy_id.clone(),
                NodeLayout {
                    id: dummy_id.clone(),
                    x: 0.0,
                    y: 0.0,
                    width: if is_horizontal(graph.direction) {
                        main_dim
                    } else {
                        cross_dim
                    },
                    height: if is_horizontal(graph.direction) {
                        cross_dim
                    } else {
                        main_dim
                    },
                    label: TextBlock {
                        lines: vec![],
                        width: 0.0,
                        height: 0.0,
                    },
                    shape: crate::ir::NodeShape::Rectangle,
                    style: crate::ir::NodeStyle::default(),
                    link: None,
                    anchor_subgraph: None,
                    hidden: true,
                    icon: None,
                },
            );

            // Record original edge index → dummy node ID mapping.
            if let Some(&orig_idx) = original_edge_indices.get(idx) {
                if orig_idx < label_dummy_ids.len() {
                    label_dummy_ids[orig_idx] = Some(dummy_id.clone());
                }
            }

            if let Some(bucket) = rank_nodes.get_mut(label_rank) {
                bucket.push(dummy_id);
            }
        }
    }

    // Build a lookup: for each layout edge index, the (shifted_label_rank, dummy_id)
    // so the span-dummy expansion loop can reuse label dummies instead of creating
    // new span dummies at the same rank (which would leave label dummies disconnected).
    let mut label_dummy_at_rank: HashMap<usize, (usize, String)> = HashMap::new();
    for (idx, edge) in layout_edges.iter().enumerate() {
        if edge_labels[idx].is_none() {
            continue;
        }
        let from_rank = ranks.get(&edge.from).copied().unwrap_or(0);
        let to_rank = ranks.get(&edge.to).copied().unwrap_or(0);
        let lo = from_rank.min(to_rank);
        let hi = from_rank.max(to_rank);
        if hi <= lo {
            continue;
        }
        let mid_gap = lo + (hi - lo - 1) / 2;
        let label_rank = mid_gap + rank_shift[mid_gap] + 1;
        if let Some(&orig_idx) = original_edge_indices.get(idx) {
            if let Some(Some(dummy_id)) = label_dummy_ids.get(orig_idx) {
                label_dummy_at_rank.insert(idx, (label_rank, dummy_id.clone()));
            }
        }
    }

    // Update ranks for existing nodes to use shifted values (for the existing dummy expansion).
    let shifted_ranks: HashMap<String, usize> = ranks
        .iter()
        .map(|(id, &r)| (id.clone(), r + rank_shift[r]))
        .collect();

    // --- End label dummy nodes ---

    let mut expanded_edges: Vec<crate::ir::Edge> = Vec::new();

    for (edge_idx, edge) in layout_edges.iter().enumerate() {
        let Some(&from_rank) = shifted_ranks.get(&edge.from) else {
            continue;
        };
        let Some(&to_rank) = shifted_ranks.get(&edge.to) else {
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
        // Look up whether this edge has a label dummy at some rank.
        let label_dummy_info = label_dummy_at_rank.get(&edge_idx);
        let mut prev = edge.from.clone();
        for step in 1..span {
            let current_rank = from_rank + step;
            // Reuse the label dummy if it exists at this rank, instead of
            // creating a new span dummy. This connects the label dummy into
            // the expanded edge chain so it gets proper cross-axis positioning.
            let dummy_id = if let Some((lr, lid)) = label_dummy_info {
                if current_rank == *lr {
                    lid.clone()
                } else {
                    let id = format!("__dummy_{}__", dummy_counter);
                    dummy_counter += 1;
                    let order_idx = order_map.len();
                    order_map.insert(id.clone(), order_idx);
                    if let Some(bucket) = rank_nodes.get_mut(current_rank) {
                        bucket.push(id.clone());
                    }
                    id
                }
            } else {
                let id = format!("__dummy_{}__", dummy_counter);
                dummy_counter += 1;
                let order_idx = order_map.len();
                order_map.insert(id.clone(), order_idx);
                if let Some(bucket) = rank_nodes.get_mut(current_rank) {
                    bucket.push(id.clone());
                }
                id
            };
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
    order_rank_nodes(
        &mut rank_nodes,
        &expanded_edges,
        &order_map,
        config.flowchart.order_passes,
    );

    let mut main_cursor = 0.0;
    for (rank_idx, bucket) in rank_nodes.iter().enumerate() {
        let mut max_main: f32 = 0.0;
        let is_label_rank = label_dummy_ranks.contains(&rank_idx);
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
        if max_main > 0.0 {
            // Use reduced spacing for label-only ranks to avoid excessive width.
            let gap = if is_label_rank {
                (theme.font_size * LABEL_RANK_FONT_SCALE).max(LABEL_RANK_MIN_GAP)
            } else {
                config.rank_spacing
            };
            main_cursor += max_main + gap;
        }
    }

    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    // Use expanded_edges so dummy nodes (both span dummies and label
    // dummies) get proper neighbor connectivity for cross-axis positioning.
    for edge in &expanded_edges {
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
    for bucket in &rank_nodes {
        for (idx, node_id) in bucket.iter().enumerate() {
            if let Some(node) = nodes.get(node_id) {
                let center = if is_horizontal(graph.direction) {
                    node.y + node.height / 2.0
                } else {
                    node.x + node.width / 2.0
                };
                cross_pos.insert(node_id.clone(), center + idx as f32 * 0.01);
            }
        }
    }

    let mut place_rank = |rank_idx: usize,
                          use_incoming: bool,
                          nodes: &mut BTreeMap<String, NodeLayout>| {
        let bucket = &rank_nodes[rank_idx];
        if bucket.is_empty() {
            return;
        }
        let neighbors = if use_incoming { &incoming } else { &outgoing };
        let mut entries: Vec<(String, f32, f32, usize)> = Vec::new();
        for (idx, node_id) in bucket.iter().enumerate() {
            let Some(node) = nodes.get(node_id) else {
                continue;
            };
            let mut neighbor_centers: Vec<f32> = Vec::new();
            if let Some(list) = neighbors.get(node_id) {
                for neighbor_id in list {
                    if let Some(center) = cross_pos.get(neighbor_id) {
                        neighbor_centers.push(*center);
                    }
                }
            }
            let mut desired = if neighbor_centers.is_empty() {
                cross_pos.get(node_id).copied().unwrap_or(0.0)
            } else {
                neighbor_centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
                let mid = neighbor_centers.len() / 2;
                if neighbor_centers.len() % 2 == 1 {
                    neighbor_centers[mid]
                } else {
                    (neighbor_centers[mid - 1] + neighbor_centers[mid]) * 0.5
                }
            };
            if let Some(current) = cross_pos.get(node_id) {
                if !neighbor_centers.is_empty() {
                    desired = desired * 0.85 + current * 0.15;
                } else {
                    desired = *current;
                }
            }
            let half = if is_horizontal(graph.direction) {
                node.height / 2.0
            } else {
                node.width / 2.0
            };
            entries.push((node_id.clone(), desired, half, idx));
        }
        entries.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.3.cmp(&b.3))
        });
        let desired_mean =
            entries.iter().map(|(_, d, _, _)| *d).sum::<f32>() / entries.len() as f32;
        let mut assigned: Vec<(String, f32, f32)> = Vec::new();
        let mut prev_center: Option<f32> = None;
        let mut prev_half = 0.0;
        for (node_id, desired, half, _idx) in entries {
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

    for _ in 0..config.flowchart.order_passes.max(1) {
        for rank_idx in 0..rank_nodes.len() {
            place_rank(rank_idx, true, nodes);
        }
        for rank_idx in (0..rank_nodes.len()).rev() {
            place_rank(rank_idx, false, nodes);
        }
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

fn apply_subgraph_bands(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    let mut group_nodes: Vec<Vec<String>> = Vec::new();
    let mut node_group: HashMap<String, usize> = HashMap::new();

    // Group 0: nodes not in any subgraph.
    group_nodes.push(Vec::new());

    let top_level = top_level_subgraph_indices(graph);
    for (pos, idx) in top_level.iter().enumerate() {
        let group_idx = pos + 1;
        let sub = &graph.subgraphs[*idx];
        group_nodes.push(Vec::new());
        for node_id in &sub.nodes {
            if nodes.contains_key(node_id) {
                node_group.insert(node_id.clone(), group_idx);
            }
        }
        if let Some(anchor_id) = subgraph_anchor_id(sub, nodes) {
            if nodes.contains_key(anchor_id) {
                node_group.insert(anchor_id.to_string(), group_idx);
            }
        }
    }

    for node_id in graph.nodes.keys() {
        if node_group.contains_key(node_id) {
            continue;
        }
        node_group.insert(node_id.clone(), 0);
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

    let mut inter_group_edges = 0usize;
    let mut group_links: HashSet<(usize, usize)> = HashSet::new();
    let mut group_degree: HashMap<usize, usize> = HashMap::new();
    for edge in &graph.edges {
        let from_group = node_group.get(&edge.from);
        let to_group = node_group.get(&edge.to);
        if let (Some(a), Some(b)) = (from_group, to_group) {
            if a != b {
                inter_group_edges += 1;
                let (min_g, max_g) = if a < b { (*a, *b) } else { (*b, *a) };
                group_links.insert((min_g, max_g));
                *group_degree.entry(*a).or_insert(0) += 1;
                *group_degree.entry(*b).or_insert(0) += 1;
            }
        }
    }
    let max_degree = group_degree.values().copied().max().unwrap_or(0);
    let path_like = inter_group_edges > 0
        && group_links.len() <= groups.len().saturating_sub(1)
        && max_degree <= 2;
    let grid_pack = inter_group_edges == 0;
    let align_cross = path_like;

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
        if align_cross && !groups.is_empty() {
            let target_y = groups.iter().map(|group| group.2).fold(f32::MAX, f32::min);
            for (group_idx, _min_x, min_y, _max_x, _max_y) in &groups {
                let offset_y = target_y - *min_y;
                for node_id in group_nodes[*group_idx].iter() {
                    if let Some(node) = nodes.get_mut(node_id) {
                        node.y += offset_y;
                    }
                }
            }
        } else if grid_pack && groups.len() > 1 {
            let mut bounds: HashMap<usize, (f32, f32, f32, f32)> = HashMap::new();
            for (group_idx, min_x, min_y, max_x, max_y) in &groups {
                bounds.insert(*group_idx, (*min_x, *min_y, max_x - min_x, max_y - min_y));
            }
            let origin_x = groups.iter().map(|group| group.1).fold(f32::MAX, f32::min);
            let origin_y = groups.iter().map(|group| group.2).fold(f32::MAX, f32::min);

            let mut best_area = f32::MAX;
            let mut best_rows: Vec<Vec<usize>> = Vec::new();
            for cols in 1..=groups.len() {
                let mut rows: Vec<Vec<usize>> = Vec::new();
                let mut idx = 0usize;
                while idx < groups.len() {
                    let mut row = Vec::new();
                    for _ in 0..cols {
                        if idx >= groups.len() {
                            break;
                        }
                        row.push(groups[idx].0);
                        idx += 1;
                    }
                    rows.push(row);
                }
                let mut max_row_width = 0.0f32;
                let mut total_height = 0.0f32;
                for row in &rows {
                    let mut row_width = 0.0f32;
                    let mut row_height = 0.0f32;
                    for (pos, group_idx) in row.iter().enumerate() {
                        if let Some((_, _, width, height)) = bounds.get(group_idx) {
                            row_width += *width;
                            if pos + 1 < row.len() {
                                row_width += spacing;
                            }
                            row_height = row_height.max(*height);
                        }
                    }
                    max_row_width = max_row_width.max(row_width);
                    total_height += row_height;
                }
                if !rows.is_empty() {
                    total_height += spacing * (rows.len().saturating_sub(1) as f32);
                }
                let area = max_row_width * total_height;
                if area < best_area {
                    best_area = area;
                    best_rows = rows;
                }
            }

            let mut cursor_y = origin_y;
            for row in best_rows {
                let mut row_height = 0.0f32;
                let mut cursor_x = origin_x;
                for group_idx in row {
                    let Some((min_x, min_y, width, height)) = bounds.get(&group_idx) else {
                        continue;
                    };
                    let offset_x = cursor_x - min_x;
                    let offset_y = cursor_y - min_y;
                    for node_id in group_nodes[group_idx].iter() {
                        if let Some(node) = nodes.get_mut(node_id) {
                            node.x += offset_x;
                            node.y += offset_y;
                        }
                    }
                    cursor_x += width + spacing;
                    row_height = row_height.max(*height);
                }
                cursor_y += row_height + spacing;
            }
        } else {
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
        }
    } else {
        if align_cross && !groups.is_empty() {
            let target_x = groups.iter().map(|group| group.1).fold(f32::MAX, f32::min);
            for (group_idx, min_x, _min_y, _max_x, _max_y) in &groups {
                let offset_x = target_x - *min_x;
                for node_id in group_nodes[*group_idx].iter() {
                    if let Some(node) = nodes.get_mut(node_id) {
                        node.x += offset_x;
                    }
                }
            }
        } else if grid_pack && groups.len() > 1 {
            let mut bounds: HashMap<usize, (f32, f32, f32, f32)> = HashMap::new();
            for (group_idx, min_x, min_y, max_x, max_y) in &groups {
                bounds.insert(*group_idx, (*min_x, *min_y, max_x - min_x, max_y - min_y));
            }
            let origin_x = groups.iter().map(|group| group.1).fold(f32::MAX, f32::min);
            let origin_y = groups.iter().map(|group| group.2).fold(f32::MAX, f32::min);

            let mut best_rows = Vec::new();
            let mut best_area = f32::MAX;
            for rows in 1..=groups.len() {
                let cols = (groups.len() + rows - 1) / rows;
                let mut grid: Vec<Vec<usize>> = Vec::new();
                let mut idx = 0usize;
                for _ in 0..rows {
                    let mut col = Vec::new();
                    for _ in 0..cols {
                        if idx >= groups.len() {
                            break;
                        }
                        col.push(groups[idx].0);
                        idx += 1;
                    }
                    grid.push(col);
                }
                let mut max_col_height = 0.0f32;
                let mut total_width = 0.0f32;
                for col in &grid {
                    let mut col_height = 0.0f32;
                    let mut col_width = 0.0f32;
                    for (pos, group_idx) in col.iter().enumerate() {
                        if let Some((_, _, width, height)) = bounds.get(group_idx) {
                            col_height += *height;
                            if pos + 1 < col.len() {
                                col_height += spacing;
                            }
                            col_width = col_width.max(*width);
                        }
                    }
                    max_col_height = max_col_height.max(col_height);
                    total_width += col_width;
                }
                if !grid.is_empty() {
                    total_width += spacing * (grid.len().saturating_sub(1) as f32);
                }
                let area = total_width * max_col_height;
                if area < best_area {
                    best_area = area;
                    best_rows = grid;
                }
            }

            let mut cursor_x = origin_x;
            for col in best_rows {
                let mut col_width = 0.0f32;
                let mut cursor_y = origin_y;
                for group_idx in col {
                    let Some((min_x, min_y, width, height)) = bounds.get(&group_idx) else {
                        continue;
                    };
                    let offset_x = cursor_x - min_x;
                    let offset_y = cursor_y - min_y;
                    for node_id in group_nodes[group_idx].iter() {
                        if let Some(node) = nodes.get_mut(node_id) {
                            node.x += offset_x;
                            node.y += offset_y;
                        }
                    }
                    cursor_y += height + spacing;
                    col_width = col_width.max(*width);
                }
                cursor_x += col_width + spacing;
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
}

fn compress_linear_subgraphs(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.subgraphs.is_empty() {
        return;
    }
    let gap = config.flowchart.auto_spacing.min_spacing;
    let horizontal = is_horizontal(graph.direction);

    for sub in &graph.subgraphs {
        if sub.nodes.len() < 3 {
            continue;
        }
        let sub_set: HashSet<&str> = sub.nodes.iter().map(|id| id.as_str()).collect();
        let mut in_deg: HashMap<String, usize> = HashMap::new();
        let mut out_deg: HashMap<String, usize> = HashMap::new();
        let mut next_map: HashMap<String, String> = HashMap::new();
        let mut edges_in_sub = 0usize;

        for node_id in &sub.nodes {
            in_deg.insert(node_id.clone(), 0);
            out_deg.insert(node_id.clone(), 0);
        }

        for edge in &graph.edges {
            if !sub_set.contains(edge.from.as_str()) || !sub_set.contains(edge.to.as_str()) {
                continue;
            }
            edges_in_sub += 1;
            let out = out_deg.entry(edge.from.clone()).or_insert(0);
            *out += 1;
            if *out == 1 {
                next_map.insert(edge.from.clone(), edge.to.clone());
            } else {
                next_map.remove(&edge.from);
            }
            let entry = in_deg.entry(edge.to.clone()).or_insert(0);
            *entry += 1;
        }

        if edges_in_sub + 1 != sub.nodes.len() {
            continue;
        }
        if in_deg.values().any(|&d| d > 1) || out_deg.values().any(|&d| d > 1) {
            continue;
        }

        let starts: Vec<&String> = sub
            .nodes
            .iter()
            .filter(|id| *in_deg.get(*id).unwrap_or(&0) == 0)
            .collect();
        if starts.len() != 1 {
            continue;
        }

        let mut order: Vec<String> = Vec::with_capacity(sub.nodes.len());
        let mut visited: HashSet<String> = HashSet::new();
        let mut current = starts[0].clone();
        while visited.insert(current.clone()) {
            order.push(current.clone());
            if let Some(next) = next_map.get(&current) {
                current = next.clone();
            } else {
                break;
            }
        }
        if order.len() != sub.nodes.len() {
            continue;
        }

        let min_main = order
            .iter()
            .filter_map(|id| nodes.get(id))
            .map(|node| if horizontal { node.x } else { node.y })
            .fold(f32::MAX, f32::min);
        let mut cursor = min_main;
        for node_id in order {
            if let Some(node) = nodes.get_mut(&node_id) {
                if horizontal {
                    node.x = cursor;
                    cursor += node.width + gap;
                } else {
                    node.y = cursor;
                    cursor += node.height + gap;
                }
            }
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

/// Pre-computed containment tree for subgraphs.
///
/// Built once from `graph.subgraphs` by checking subset relationships between
/// node sets.  Each subgraph is assigned an optional `parent` (the *smallest*
/// containing subgraph) and a list of `children`.  Top-level subgraphs have
/// `parent == None`.
struct SubgraphTree {
    /// `parent[i]` = index of the immediate parent subgraph, or `None` if top-level.
    parent: Vec<Option<usize>>,
    /// `children[i]` = indices of immediate child subgraphs.
    children: Vec<Vec<usize>>,
    /// Indices of subgraphs that have no parent.
    top_level: Vec<usize>,
}

impl SubgraphTree {
    fn build(graph: &Graph) -> Self {
        let n = graph.subgraphs.len();
        let sets: Vec<HashSet<String>> = graph
            .subgraphs
            .iter()
            .map(|sub| sub.nodes.iter().cloned().collect())
            .collect();

        // Sort indices by set size ascending so we can find the *smallest*
        // containing parent efficiently.
        let mut by_size: Vec<usize> = (0..n).collect();
        by_size.sort_by_key(|&i| sets[i].len());

        let mut parent: Vec<Option<usize>> = vec![None; n];
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];

        // For each subgraph (from smallest to largest), find its immediate
        // parent: the smallest subgraph that strictly contains it.
        for (pos, &i) in by_size.iter().enumerate() {
            for &j in &by_size[pos + 1..] {
                if sets[j].len() > sets[i].len() && sets[i].is_subset(&sets[j]) {
                    parent[i] = Some(j);
                    children[j].push(i);
                    break;
                }
            }
        }

        let top_level: Vec<usize> = (0..n).filter(|&i| parent[i].is_none()).collect();

        SubgraphTree {
            parent,
            children,
            top_level,
        }
    }

    /// Returns `true` if subgraph `ancestor` contains subgraph `descendant`
    /// (i.e. `descendant`'s node set is a subset of `ancestor`'s).
    fn is_ancestor(&self, ancestor: usize, descendant: usize) -> bool {
        let mut cur = descendant;
        loop {
            match self.parent[cur] {
                Some(p) if p == ancestor => return true,
                Some(p) => cur = p,
                None => return false,
            }
        }
    }

    /// Two subgraphs are siblings if neither is an ancestor of the other.
    fn are_siblings(&self, a: usize, b: usize) -> bool {
        a != b && !self.is_ancestor(a, b) && !self.is_ancestor(b, a)
    }
}

fn top_level_subgraph_indices(graph: &Graph) -> Vec<usize> {
    SubgraphTree::build(graph).top_level
}

fn apply_subgraph_direction_overrides(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
    skip_indices: &HashSet<usize>,
) {
    if graph.kind == crate::ir::DiagramKind::Flowchart {
        return;
    }
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
        let ranks = compute_ranks_subset(&sub.nodes, &graph.edges, &graph.node_order);
        assign_positions(
            &sub.nodes,
            &ranks,
            direction,
            &local_config,
            &mut temp_nodes,
            0.0,
            0.0,
        );
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
    padding_x: f32,
    top_padding: f32,
}

fn subgraph_layout_direction(graph: &Graph, sub: &crate::ir::Subgraph) -> Direction {
    if graph.kind == crate::ir::DiagramKind::State {
        return graph.direction;
    }
    let _ = sub; // Subgraph direction is currently ignored for CLI parity.
    graph.direction
}

fn subgraph_layout_config(graph: &Graph, anchorable: bool, config: &LayoutConfig) -> LayoutConfig {
    let mut local = config.clone();
    if graph.kind == crate::ir::DiagramKind::Flowchart && anchorable {
        local.rank_spacing = config.rank_spacing + STATE_RANK_SPACING_BOOST;
    }
    local
}

fn flowchart_subgraph_padding(direction: Direction) -> (f32, f32) {
    // Mermaid CLI uses larger padding along the main axis and slightly
    // smaller padding along the cross axis.
    if is_horizontal(direction) {
        (FLOWCHART_PAD_MAIN, FLOWCHART_PAD_CROSS)
    } else {
        (FLOWCHART_PAD_CROSS, FLOWCHART_PAD_MAIN)
    }
}

fn subgraph_padding_from_label(
    graph: &Graph,
    sub: &crate::ir::Subgraph,
    theme: &Theme,
    label_block: &TextBlock,
) -> (f32, f32, f32) {
    if is_region_subgraph(sub) {
        return (0.0, 0.0, 0.0);
    }

    let label_empty = sub.label.trim().is_empty();
    let label_height = if label_empty { 0.0 } else { label_block.height };

    let (mut pad_x, mut pad_y) = if graph.kind == crate::ir::DiagramKind::Flowchart {
        flowchart_subgraph_padding(graph.direction)
    } else if graph.kind == crate::ir::DiagramKind::Kanban {
        (KANBAN_SUBGRAPH_PAD, KANBAN_SUBGRAPH_PAD)
    } else {
        let base_padding = if graph.kind == crate::ir::DiagramKind::State {
            STATE_SUBGRAPH_BASE_PAD
        } else {
            GENERIC_SUBGRAPH_BASE_PAD
        };
        (base_padding, base_padding)
    };
    if graph.kind == crate::ir::DiagramKind::Flowchart
        && sub.nodes.len() <= 3
        && ((is_horizontal(graph.direction) && graph.edges.len() <= 20)
            || (!is_horizontal(graph.direction) && graph.edges.len() <= 13))
        && !graph.edges.iter().any(|edge| {
            edge.label
                .as_ref()
                .map(|label| label.chars().count() > 24)
                .unwrap_or(false)
        })
    {
        pad_x *= 0.7;
        pad_y *= 0.7;
    }

    let top_padding = if label_empty {
        pad_y
    } else if graph.kind == crate::ir::DiagramKind::Flowchart {
        // Keep the label comfortably inside the top band without over-expanding
        // the cluster height.
        pad_y.max(label_height + SUBGRAPH_LABEL_GAP_FLOWCHART)
    } else if graph.kind == crate::ir::DiagramKind::Kanban {
        pad_y.max(label_height + SUBGRAPH_LABEL_GAP_KANBAN)
    } else if graph.kind == crate::ir::DiagramKind::State {
        (label_height + theme.font_size * STATE_SUBGRAPH_TOP_LABEL_SCALE)
            .max(theme.font_size * STATE_SUBGRAPH_TOP_MIN_SCALE)
    } else {
        pad_y + label_height + SUBGRAPH_LABEL_GAP_GENERIC
    };

    (pad_x, pad_y, top_padding)
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
    let ranks = compute_ranks_subset(&sub.nodes, &graph.edges, &graph.node_order);
    assign_positions(
        &sub.nodes,
        &ranks,
        direction,
        &local_config,
        &mut temp_nodes,
        0.0,
        0.0,
    );
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
    let label_empty = sub.label.trim().is_empty();
    let mut label_block = measure_label(&sub.label, theme, config);
    if label_empty {
        label_block.width = 0.0;
        label_block.height = 0.0;
    }
    let (padding_x, padding_y, top_padding) =
        subgraph_padding_from_label(graph, sub, theme, &label_block);

    let width = (max_x - min_x) + padding_x * 2.0;
    let height = (max_y - min_y) + padding_y + top_padding;
    Some((width, height, padding_x, top_padding))
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
        let Some((width, height, padding_x, top_padding)) =
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
                padding_x,
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
    let tree = SubgraphTree::build(graph);
    let sub_count = graph.subgraphs.len();
    let mut subgraph_depth: Vec<usize> = vec![0; sub_count];
    for idx in 0..sub_count {
        let mut depth = 0usize;
        let mut cur = idx;
        while let Some(parent_idx) = tree.parent.get(cur).and_then(|parent| *parent) {
            depth += 1;
            cur = parent_idx;
            if depth > sub_count {
                break;
            }
        }
        subgraph_depth[idx] = depth;
    }

    let mut ordered_anchors: Vec<(&String, &SubgraphAnchorInfo)> = anchor_info.iter().collect();
    ordered_anchors.sort_by(|(a_id, a_info), (b_id, b_info)| {
        let a_depth = subgraph_depth
            .get(a_info.sub_idx)
            .copied()
            .unwrap_or(usize::MAX);
        let b_depth = subgraph_depth
            .get(b_info.sub_idx)
            .copied()
            .unwrap_or(usize::MAX);
        a_depth
            .cmp(&b_depth)
            .then_with(|| a_info.sub_idx.cmp(&b_info.sub_idx))
            .then_with(|| a_id.cmp(b_id))
    });

    for (anchor_id, info) in ordered_anchors {
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
        let ranks = compute_ranks_subset(&sub.nodes, &graph.edges, &graph.node_order);
        assign_positions(
            &sub.nodes,
            &ranks,
            direction,
            &local_config,
            nodes,
            anchor_x + info.padding_x,
            anchor_y + info.top_padding,
        );
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
    // Build nesting hierarchy: for each subgraph, find which other subgraphs are direct children
    let sub_count = graph.subgraphs.len();
    let mut depth: Vec<usize> = vec![0; sub_count];
    let mut parent_of: Vec<Option<usize>> = vec![None; sub_count];

    // A subgraph B is nested in subgraph A if A's nodes list contains B's ID/label
    for (i, sub_a) in graph.subgraphs.iter().enumerate() {
        for (j, sub_b) in graph.subgraphs.iter().enumerate() {
            if i == j {
                continue;
            }
            let b_id = sub_b.id.as_deref().unwrap_or("");
            if sub_a.nodes.iter().any(|n| n == b_id)
                || sub_a.nodes.iter().any(|n| n == &sub_b.label)
            {
                if parent_of[j].is_none() {
                    parent_of[j] = Some(i);
                }
            }
        }
    }

    // Compute depth: walk from each subgraph up to root
    for i in 0..sub_count {
        let mut d = 0;
        let mut cur = i;
        while let Some(p) = parent_of[cur] {
            d += 1;
            cur = p;
            if d > sub_count {
                break;
            }
        }
        depth[i] = d;
    }

    // Process from deepest (innermost) to shallowest (outermost)
    let mut order: Vec<usize> = (0..sub_count).collect();
    order.sort_by(|a, b| depth[*b].cmp(&depth[*a]));

    // Track computed inner subgraph boxes (idx -> (x, y, width, height))
    let mut inner_boxes: HashMap<usize, (f32, f32, f32, f32)> = HashMap::new();

    for idx in order {
        let sub = &graph.subgraphs[idx];
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

        // For nodes in this subgraph that are also inner subgraph anchors,
        // temporarily set their size to the inner subgraph's box size
        let mut saved_sizes: Vec<(String, f32, f32)> = Vec::new();
        let mut inner_anchor_ids: Vec<String> = Vec::new();
        for node_id in &sub.nodes {
            for (j, inner_sub) in graph.subgraphs.iter().enumerate() {
                if let Some((_, _, w, h)) = inner_boxes.get(&j) {
                    let inner_id = inner_sub.id.as_deref().unwrap_or("");
                    if node_id == inner_id || node_id == &inner_sub.label {
                        if !inner_anchor_ids.iter().any(|id| id == node_id) {
                            inner_anchor_ids.push(node_id.clone());
                        }
                        if let Some(node) = nodes.get(node_id) {
                            saved_sizes.push((node_id.clone(), node.width, node.height));
                        }
                        if let Some(node) = nodes.get_mut(node_id) {
                            node.width = *w;
                            node.height = *h;
                        }
                    }
                }
            }
        }

        let ranks = compute_ranks_subset(&sub.nodes, &graph.edges, &graph.node_order);
        assign_positions(
            &sub.nodes,
            &ranks,
            graph.direction,
            config,
            nodes,
            min_x,
            min_y,
        );

        // Keep nested composite-state headers clear of parent headers.
        let nested_anchor_min_y = min_y + (config.node_spacing * 0.4).max(20.0);
        for anchor_id in &inner_anchor_ids {
            if let Some(anchor) = nodes.get_mut(anchor_id)
                && anchor.y < nested_anchor_min_y
            {
                anchor.y = nested_anchor_min_y;
            }
        }

        // Restore original sizes for anchor nodes
        for (id, w, h) in saved_sizes {
            if let Some(node) = nodes.get_mut(&id) {
                node.width = w;
                node.height = h;
            }
        }

        // After positioning, re-position inner subgraph contents to match their anchor position
        for (j, inner_sub) in graph.subgraphs.iter().enumerate() {
            if let Some(&(old_x, old_y, _, _)) = inner_boxes.get(&j) {
                let inner_id = inner_sub.id.as_deref().unwrap_or("");
                if !sub
                    .nodes
                    .iter()
                    .any(|n| n == inner_id || n == &inner_sub.label)
                {
                    continue;
                }
                // Find the anchor node's new position
                let anchor_id = if sub.nodes.iter().any(|n| n == inner_id) {
                    inner_id
                } else {
                    inner_sub.label.as_str()
                };
                if let Some(anchor) = nodes.get(anchor_id) {
                    let dx = anchor.x - old_x;
                    let dy = anchor.y - old_y;
                    if dx.abs() > 0.01 || dy.abs() > 0.01 {
                        for inner_node_id in &inner_sub.nodes {
                            if let Some(inner_node) = nodes.get_mut(inner_node_id) {
                                inner_node.x += dx;
                                inner_node.y += dy;
                            }
                        }
                    }
                }
            }
        }

        // Compute and save this subgraph's box
        let mut bmin_x = f32::MAX;
        let mut bmin_y = f32::MAX;
        let mut bmax_x = f32::MIN;
        let mut bmax_y = f32::MIN;
        for node_id in &sub.nodes {
            if let Some(node) = nodes.get(node_id) {
                bmin_x = bmin_x.min(node.x);
                bmin_y = bmin_y.min(node.y);
                bmax_x = bmax_x.max(node.x + node.width);
                bmax_y = bmax_y.max(node.y + node.height);
            }
        }
        // Also include inner subgraph boxes
        for (j, inner_sub) in graph.subgraphs.iter().enumerate() {
            if let Some(&(_, _, _, _)) = inner_boxes.get(&j) {
                let inner_id = inner_sub.id.as_deref().unwrap_or("");
                if sub
                    .nodes
                    .iter()
                    .any(|n| n == inner_id || n == &inner_sub.label)
                {
                    // Use inner node positions
                    for inner_node_id in &inner_sub.nodes {
                        if let Some(node) = nodes.get(inner_node_id) {
                            bmin_x = bmin_x.min(node.x);
                            bmin_y = bmin_y.min(node.y);
                            bmax_x = bmax_x.max(node.x + node.width);
                            bmax_y = bmax_y.max(node.y + node.height);
                        }
                    }
                }
            }
        }
        let padding = config.node_spacing;
        if bmin_x < f32::MAX {
            inner_boxes.insert(
                idx,
                (
                    bmin_x,
                    bmin_y,
                    bmax_x - bmin_x + padding,
                    bmax_y - bmin_y + padding,
                ),
            );
        }
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

fn bounds_without_padding(
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
) -> (f32, f32) {
    bounds_with_edges(nodes, subgraphs, &[])
}

fn bounds_with_edges(
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

    let width = layout.width.max(1.0);
    let height = layout.height.max(1.0);
    let current_ratio = width / height;
    if (current_ratio - target_ratio).abs() <= PREFERRED_ASPECT_TOLERANCE {
        return;
    }

    let mut scale_x = 1.0f32;
    let mut scale_y = 1.0f32;
    if current_ratio < target_ratio {
        scale_x = (target_ratio / current_ratio).clamp(1.0, PREFERRED_ASPECT_MAX_EXPANSION);
    } else {
        scale_y = (current_ratio / target_ratio).clamp(1.0, PREFERRED_ASPECT_MAX_EXPANSION);
    }
    if (scale_x - 1.0).abs() <= 1e-3 && (scale_y - 1.0).abs() <= 1e-3 {
        return;
    }

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

    let (mut max_x, mut max_y) = bounds_with_edges(&layout.nodes, &layout.subgraphs, &layout.edges);
    if let DiagramData::Graph { state_notes } = &layout.diagram {
        for note in state_notes {
            max_x = max_x.max(note.x + note.width);
            max_y = max_y.max(note.y + note.height);
        }
    }
    layout.width = (max_x + LAYOUT_BOUNDARY_PAD).max(1.0);
    layout.height = (max_y + LAYOUT_BOUNDARY_PAD).max(1.0);
}

fn flowchart_path_overlap_with_prior(path: &[(f32, f32)], prior: &[Vec<(f32, f32)>]) -> f32 {
    let mut overlap = 0.0f32;
    for segment in path.windows(2) {
        let a1 = segment[0];
        let a2 = segment[1];
        for other in prior {
            for other_segment in other.windows(2) {
                overlap += collinear_overlap_length(a1, a2, other_segment[0], other_segment[1]);
            }
        }
    }
    overlap
}

fn append_path_segments(path: &[(f32, f32)], segments: &mut Vec<Segment>) {
    if path.len() < 2 {
        return;
    }
    for window in path.windows(2) {
        segments.push((window[0], window[1]));
    }
}

fn perimeter_route_candidates(
    start: (f32, f32),
    end: (f32, f32),
    outer_left: f32,
    outer_right: f32,
    outer_top: f32,
    outer_bottom: f32,
) -> Vec<Vec<(f32, f32)>> {
    vec![
        vec![
            start,
            (outer_right, start.1),
            (outer_right, outer_bottom),
            (outer_left, outer_bottom),
            (outer_left, end.1),
            end,
        ],
        vec![
            start,
            (outer_right, start.1),
            (outer_right, outer_top),
            (outer_left, outer_top),
            (outer_left, end.1),
            end,
        ],
        vec![
            start,
            (outer_left, start.1),
            (outer_left, outer_bottom),
            (outer_right, outer_bottom),
            (outer_right, end.1),
            end,
        ],
        vec![
            start,
            (outer_left, start.1),
            (outer_left, outer_top),
            (outer_right, outer_top),
            (outer_right, end.1),
            end,
        ],
    ]
}

fn reduce_crossing_sweep(
    order: &[usize],
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    routed_points: &mut [Vec<(f32, f32)>],
    deltas: &[f32],
    use_perimeter_candidates: bool,
    outer_left: f32,
    outer_right: f32,
    outer_top: f32,
    outer_bottom: f32,
) -> bool {
    let mut changed = false;
    let mut existing_segments: Vec<Segment> = Vec::new();
    // Keep crossing fixes from introducing visually extreme detours.
    const MAX_LEN_RATIO_HARD: f32 = 2.8;
    const MAX_LEN_RATIO_NO_GAIN: f32 = 1.12;
    const MAX_LEN_RATIO_ONE_GAIN: f32 = 1.8;
    const MAX_LEN_RATIO_MULTI_GAIN: f32 = 2.6;
    for &idx in order {
        if routed_points[idx].len() < 2 {
            append_path_segments(&routed_points[idx], &mut existing_segments);
            continue;
        }
        let from_id = graph.edges[idx].from.as_str();
        let to_id = graph.edges[idx].to.as_str();
        let (baseline_cross, baseline_overlap) =
            edge_crossings_with_existing(&routed_points[idx], &existing_segments);
        if baseline_cross == 0 {
            append_path_segments(&routed_points[idx], &mut existing_segments);
            continue;
        }
        let mut best_cross = baseline_cross;
        let mut best_overlap = baseline_overlap;
        let baseline_len = path_length(&routed_points[idx]);
        let mut best_len = baseline_len;
        let mut best_points = routed_points[idx].clone();
        let segment_count = routed_points[idx].len().saturating_sub(1);
        for seg_idx in 0..segment_count {
            for &delta in deltas {
                let Some(candidate) = bump_orthogonal_segment(&routed_points[idx], seg_idx, delta)
                else {
                    continue;
                };
                if flowchart_path_hits_non_endpoint_nodes(&candidate, from_id, to_id, nodes) {
                    continue;
                }
                let (crossings, overlap) =
                    edge_crossings_with_existing(&candidate, &existing_segments);
                let len = path_length(&candidate);
                if len > baseline_len * MAX_LEN_RATIO_HARD {
                    continue;
                }
                if crossings < best_cross
                    || (crossings == best_cross && overlap + 0.05 < best_overlap)
                    || (crossings == best_cross
                        && (overlap - best_overlap).abs() <= 0.05
                        && len + 1.0 < best_len)
                {
                    best_cross = crossings;
                    best_overlap = overlap;
                    best_len = len;
                    best_points = candidate;
                }
            }
        }
        if use_perimeter_candidates
            && let (Some(&start), Some(&end)) =
                (routed_points[idx].first(), routed_points[idx].last())
        {
            for candidate in perimeter_route_candidates(
                start,
                end,
                outer_left,
                outer_right,
                outer_top,
                outer_bottom,
            ) {
                let candidate = compress_path(&candidate);
                if flowchart_path_hits_non_endpoint_nodes(&candidate, from_id, to_id, nodes) {
                    continue;
                }
                let (crossings, overlap) =
                    edge_crossings_with_existing(&candidate, &existing_segments);
                let len = path_length(&candidate);
                if len > baseline_len * MAX_LEN_RATIO_HARD {
                    continue;
                }
                let crossing_gain = baseline_cross.saturating_sub(crossings);
                let max_ratio = if crossing_gain >= 2 {
                    MAX_LEN_RATIO_MULTI_GAIN
                } else if crossing_gain == 1 {
                    MAX_LEN_RATIO_ONE_GAIN
                } else {
                    MAX_LEN_RATIO_NO_GAIN
                };
                if len > baseline_len * max_ratio {
                    continue;
                }
                if crossings < best_cross
                    || (crossings == best_cross && overlap + 0.05 < best_overlap)
                    || (crossings == best_cross
                        && (overlap - best_overlap).abs() <= 0.05
                        && len + 1.0 < best_len)
                {
                    best_cross = crossings;
                    best_overlap = overlap;
                    best_len = len;
                    best_points = candidate;
                }
            }
        }
        let best_gain = baseline_cross.saturating_sub(best_cross);
        let max_ratio = if best_gain >= 2 {
            MAX_LEN_RATIO_MULTI_GAIN
        } else if best_gain == 1 {
            MAX_LEN_RATIO_ONE_GAIN
        } else {
            MAX_LEN_RATIO_NO_GAIN
        };
        let allow_apply = best_len <= baseline_len * max_ratio;
        if best_cross < baseline_cross
            || (best_cross == baseline_cross && best_overlap + 0.05 < baseline_overlap)
        {
            if !allow_apply {
                append_path_segments(&routed_points[idx], &mut existing_segments);
                continue;
            }
            routed_points[idx] = best_points;
            changed = true;
        }
        append_path_segments(&routed_points[idx], &mut existing_segments);
    }
    changed
}

fn reduce_orthogonal_path_crossings(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    routed_points: &mut [Vec<(f32, f32)>],
    config: &LayoutConfig,
) {
    if graph.edges.len() < 2 {
        return;
    }
    let base_delta = (config.node_spacing * 0.22).max(8.0);
    let deltas = [
        base_delta,
        -base_delta,
        base_delta * 1.5,
        -base_delta * 1.5,
        base_delta * 2.0,
        -base_delta * 2.0,
        base_delta * 3.0,
        -base_delta * 3.0,
        base_delta * 4.0,
        -base_delta * 4.0,
    ];
    let min_x = nodes
        .values()
        .filter(|node| !node.hidden && node.anchor_subgraph.is_none())
        .map(|node| node.x)
        .fold(f32::MAX, f32::min);
    let max_x = nodes
        .values()
        .filter(|node| !node.hidden && node.anchor_subgraph.is_none())
        .map(|node| node.x + node.width)
        .fold(f32::MIN, f32::max);
    let min_y = nodes
        .values()
        .filter(|node| !node.hidden && node.anchor_subgraph.is_none())
        .map(|node| node.y)
        .fold(f32::MAX, f32::min);
    let max_y = nodes
        .values()
        .filter(|node| !node.hidden && node.anchor_subgraph.is_none())
        .map(|node| node.y + node.height)
        .fold(f32::MIN, f32::max);
    let outer_pad = (config.node_spacing * 0.8).max(24.0);
    let outer_left = min_x - outer_pad;
    let outer_right = max_x + outer_pad;
    let outer_top = min_y - outer_pad;
    let outer_bottom = max_y + outer_pad;
    let use_perimeter_candidates = matches!(
        graph.kind,
        crate::ir::DiagramKind::Er | crate::ir::DiagramKind::State
    );
    let forward: Vec<usize> = (0..routed_points.len()).collect();
    let reverse: Vec<usize> = (0..routed_points.len()).rev().collect();

    for _ in 0..3 {
        let mut changed = reduce_crossing_sweep(
            &forward,
            graph,
            nodes,
            routed_points,
            &deltas,
            use_perimeter_candidates,
            outer_left,
            outer_right,
            outer_top,
            outer_bottom,
        );
        changed |= reduce_crossing_sweep(
            &reverse,
            graph,
            nodes,
            routed_points,
            &deltas,
            use_perimeter_candidates,
            outer_left,
            outer_right,
            outer_top,
            outer_bottom,
        );
        if !changed {
            break;
        }
    }
}

fn flowchart_path_hits_non_endpoint_nodes(
    path: &[(f32, f32)],
    from_id: &str,
    to_id: &str,
    nodes: &BTreeMap<String, NodeLayout>,
) -> bool {
    for segment in path.windows(2) {
        let a = segment[0];
        let b = segment[1];
        for node in nodes.values() {
            if node.id == from_id
                || node.id == to_id
                || node.hidden
                || node.anchor_subgraph.is_some()
            {
                continue;
            }
            let obstacle = Obstacle {
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                width: node.width,
                height: node.height,
                members: None,
            };
            if segment_intersects_rect(a, b, &obstacle) {
                return true;
            }
        }
    }
    false
}

fn bump_orthogonal_segment(
    points: &[(f32, f32)],
    seg_idx: usize,
    delta: f32,
) -> Option<Vec<(f32, f32)>> {
    if seg_idx + 1 >= points.len() {
        return None;
    }
    let a = points[seg_idx];
    let b = points[seg_idx + 1];
    let horizontal = (a.1 - b.1).abs() < 1e-3;
    let vertical = (a.0 - b.0).abs() < 1e-3;
    if !horizontal && !vertical {
        return None;
    }
    let mut bumped = Vec::with_capacity(points.len() + 2);
    bumped.extend_from_slice(&points[..=seg_idx]);
    if horizontal {
        let y = a.1 + delta;
        bumped.push((a.0, y));
        bumped.push((b.0, y));
    } else {
        let x = a.0 + delta;
        bumped.push((x, a.1));
        bumped.push((x, b.1));
    }
    bumped.extend_from_slice(&points[(seg_idx + 1)..]);
    Some(compress_path(&bumped))
}

fn deoverlap_flowchart_paths(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    routed_points: &mut [Vec<(f32, f32)>],
    config: &LayoutConfig,
) {
    if graph.edges.len() < 2 {
        return;
    }
    let overlap_threshold = 0.75f32;
    let base_delta = (config.node_spacing * 0.25).max(8.0);
    let deltas = [
        base_delta,
        -base_delta,
        base_delta * 1.5,
        -base_delta * 1.5,
        base_delta * 2.0,
        -base_delta * 2.0,
    ];

    for _ in 0..3 {
        let mut changed = false;
        for idx in 1..routed_points.len() {
            if routed_points[idx].len() < 2 {
                continue;
            }
            let from_id = graph.edges[idx].from.as_str();
            let to_id = graph.edges[idx].to.as_str();
            let baseline =
                flowchart_path_overlap_with_prior(&routed_points[idx], &routed_points[..idx]);
            if baseline < overlap_threshold {
                continue;
            }
            let mut best_overlap = baseline;
            let mut best_points = routed_points[idx].clone();
            let segment_count = routed_points[idx].len().saturating_sub(1);
            for seg_idx in 0..segment_count {
                for delta in deltas {
                    let Some(candidate) =
                        bump_orthogonal_segment(&routed_points[idx], seg_idx, delta)
                    else {
                        continue;
                    };
                    if flowchart_path_hits_non_endpoint_nodes(&candidate, from_id, to_id, nodes) {
                        continue;
                    }
                    let overlap =
                        flowchart_path_overlap_with_prior(&candidate, &routed_points[..idx]);
                    if overlap + 0.05 < best_overlap {
                        best_overlap = overlap;
                        best_points = candidate;
                    }
                }
            }
            if best_overlap + 0.05 < baseline {
                routed_points[idx] = best_points;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
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

/// Enforce a minimum gap between top-level subgraphs along the main axis.
fn enforce_top_level_subgraph_gap(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.subgraphs.len() < 2 {
        return;
    }

    let top_level = top_level_subgraph_indices(graph);
    if top_level.len() < 2 {
        return;
    }

    // Only attempt this when top-level subgraphs are disjoint to avoid
    // double-shifting shared nodes.
    let mut seen: HashSet<&str> = HashSet::new();
    for &idx in &top_level {
        for node_id in &graph.subgraphs[idx].nodes {
            if !seen.insert(node_id.as_str()) {
                return;
            }
        }
    }

    // If no edges connect top-level subgraphs, skip this function.
    // Let `separate_sibling_subgraphs` handle them on the cross axis instead.
    let node_to_top_level_sg: HashMap<&str, usize> = top_level
        .iter()
        .flat_map(|&idx| {
            graph.subgraphs[idx]
                .nodes
                .iter()
                .map(move |n| (n.as_str(), idx))
        })
        .collect();
    let has_cross_sg_edge = graph.edges.iter().any(|e| {
        let from_sg = node_to_top_level_sg.get(e.from.as_str());
        let to_sg = node_to_top_level_sg.get(e.to.as_str());
        matches!((from_sg, to_sg), (Some(a), Some(b)) if a != b)
    });
    if !has_cross_sg_edge {
        return;
    }

    #[derive(Clone, Copy)]
    struct Bounds {
        idx: usize,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        pad_main: f32,
    }

    let direction = graph.direction;
    let horizontal = is_horizontal(direction);
    let mut bounds: Vec<Bounds> = Vec::new();

    for &idx in &top_level {
        let sub = &graph.subgraphs[idx];
        if is_region_subgraph(sub) || sub.nodes.is_empty() {
            continue;
        }

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

        let label_empty = sub.label.trim().is_empty();
        let mut label_block = measure_label(&sub.label, theme, config);
        if label_empty {
            label_block.width = 0.0;
            label_block.height = 0.0;
        }
        let (pad_x, pad_y, top_padding) =
            subgraph_padding_from_label(graph, sub, theme, &label_block);

        let padded_min_x = min_x - pad_x;
        let padded_max_x = max_x + pad_x;
        let padded_min_y = min_y - top_padding;
        let padded_max_y = max_y + pad_y;
        let pad_main = if horizontal { pad_x } else { pad_y };

        bounds.push(Bounds {
            idx,
            min_x: padded_min_x,
            min_y: padded_min_y,
            max_x: padded_max_x,
            max_y: padded_max_y,
            pad_main,
        });
    }

    if bounds.len() < 2 {
        return;
    }

    bounds.sort_by(|a, b| {
        let a_key = if horizontal { a.min_x } else { a.min_y };
        let b_key = if horizontal { b.min_x } else { b.min_y };
        a_key
            .partial_cmp(&b_key)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.idx.cmp(&b.idx))
    });

    let pad_main = bounds.iter().map(|b| b.pad_main).fold(0.0_f32, f32::max);
    let desired_gap = (config.node_spacing * SUBGRAPH_DESIRED_GAP_RATIO).max(pad_main * 2.0);

    let mut prev_max_main: Option<f32> = None;
    for bound in &mut bounds {
        let min_main = if horizontal { bound.min_x } else { bound.min_y };
        let mut max_main = if horizontal { bound.max_x } else { bound.max_y };

        let mut delta = 0.0_f32;
        if let Some(prev_max) = prev_max_main {
            let required_min = prev_max + desired_gap;
            if min_main < required_min {
                delta = required_min - min_main;
            }
        }

        if delta > 0.0 {
            let sub = &graph.subgraphs[bound.idx];
            for node_id in &sub.nodes {
                if let Some(node) = nodes.get_mut(node_id) {
                    if horizontal {
                        node.x += delta;
                    } else {
                        node.y += delta;
                    }
                }
            }

            if horizontal {
                bound.min_x += delta;
                bound.max_x += delta;
            } else {
                bound.min_y += delta;
                bound.max_y += delta;
            }

            max_main += delta;
        }

        prev_max_main = Some(max_main);
    }
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

/// Separate sibling subgraphs that don't share nodes to avoid overlap
fn separate_sibling_subgraphs(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) {
    if graph.subgraphs.len() < 2 {
        return;
    }

    let tree = SubgraphTree::build(graph);

    // Find groups of sibling subgraphs using the containment tree.
    // Two subgraphs are siblings if neither is an ancestor of the other.
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
            let is_sibling = group.iter().all(|&k| tree.are_siblings(j, k));
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
                // Include subgraph padding in bounds calculation
                let label_block = measure_label(&sub.label, theme, config);
                let (pad_x, pad_y, top_padding) =
                    subgraph_padding_from_label(graph, sub, theme, &label_block);
                let padded_min_x = min_x - pad_x;
                let padded_min_y = min_y - top_padding;
                let padded_max_x = max_x + pad_x;
                let padded_max_y = max_y + pad_y;
                bounds.push((idx, padded_min_x, padded_min_y, padded_max_x, padded_max_y));
            }
        }

        if bounds.len() < 2 {
            continue;
        }

        // Sort by position along the separation axis for stable, deterministic shifts.
        if is_horizontal {
            bounds.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(Ordering::Equal));
        } else {
            bounds.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        }

        let gap = config.node_spacing.max(8.0);
        let overlaps =
            |a_min: f32, a_max: f32, b_min: f32, b_max: f32| a_min < b_max && b_min < a_max;

        let mut placed: Vec<(usize, f32, f32, f32, f32)> = Vec::new();
        for (idx, min_x, min_y, max_x, max_y) in bounds {
            let mut shift = 0.0_f32;

            for &(_, px1, py1, px2, py2) in &placed {
                let other_axis_overlaps = if is_horizontal {
                    overlaps(min_x, max_x, px1, px2)
                } else {
                    overlaps(min_y, max_y, py1, py2)
                };
                if !other_axis_overlaps {
                    continue;
                }

                let shifted_min = if is_horizontal {
                    min_y + shift
                } else {
                    min_x + shift
                };
                let shifted_max = if is_horizontal {
                    max_y + shift
                } else {
                    max_x + shift
                };
                let placed_min = if is_horizontal { py1 } else { px1 };
                let placed_max = if is_horizontal { py2 } else { px2 };

                if overlaps(shifted_min, shifted_max, placed_min, placed_max) {
                    let needed = placed_max + gap - shifted_min;
                    if needed > shift {
                        shift = needed;
                    }
                }
            }

            if shift > 0.0 {
                let sub = &graph.subgraphs[idx];
                for node_id in &sub.nodes {
                    if let Some(node) = nodes.get_mut(node_id) {
                        if is_horizontal {
                            node.y += shift;
                        } else {
                            node.x += shift;
                        }
                    }
                }
            }

            let shifted_bounds = if is_horizontal {
                (idx, min_x, min_y + shift, max_x, max_y + shift)
            } else {
                (idx, min_x + shift, min_y, max_x + shift, max_y)
            };
            placed.push(shifted_bounds);
        }
    }
}

fn align_disconnected_top_level_subgraphs(graph: &Graph, nodes: &mut BTreeMap<String, NodeLayout>) {
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.subgraphs.len() < 2 {
        return;
    }

    let top_level = top_level_subgraph_indices(graph);
    if top_level.len() < 2 {
        return;
    }

    let mut seen: HashSet<&str> = HashSet::new();
    let mut union_count = 0usize;
    for &idx in &top_level {
        let sub = &graph.subgraphs[idx];
        for node_id in &sub.nodes {
            if !seen.insert(node_id.as_str()) {
                return;
            }
            union_count += 1;
        }
        if let Some(anchor_id) = subgraph_anchor_id(sub, nodes) {
            if !seen.insert(anchor_id) {
                return;
            }
            union_count += 1;
        }
    }
    if union_count != graph.nodes.len() {
        return;
    }

    let mut node_to_top_level: HashMap<&str, usize> = HashMap::new();
    for &idx in &top_level {
        let sub = &graph.subgraphs[idx];
        for node_id in &sub.nodes {
            node_to_top_level.insert(node_id.as_str(), idx);
        }
        if let Some(anchor_id) = subgraph_anchor_id(sub, nodes) {
            node_to_top_level.insert(anchor_id, idx);
        }
    }
    let has_cross_edges = graph.edges.iter().any(|edge| {
        let from = node_to_top_level.get(edge.from.as_str());
        let to = node_to_top_level.get(edge.to.as_str());
        matches!((from, to), (Some(a), Some(b)) if a != b)
    });
    if has_cross_edges {
        return;
    }

    #[derive(Clone)]
    struct Bounds {
        idx: usize,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        anchor_id: Option<String>,
    }

    let mut bounds: Vec<Bounds> = Vec::new();
    for &idx in &top_level {
        let sub = &graph.subgraphs[idx];
        if sub.nodes.is_empty() {
            continue;
        }
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
        let anchor_id = subgraph_anchor_id(sub, nodes).map(|id| id.to_string());
        if let Some(anchor) = anchor_id.as_deref().and_then(|id| nodes.get(id)) {
            min_x = min_x.min(anchor.x);
            min_y = min_y.min(anchor.y);
            max_x = max_x.max(anchor.x + anchor.width);
            max_y = max_y.max(anchor.y + anchor.height);
        }
        if min_x == f32::MAX {
            continue;
        }
        bounds.push(Bounds {
            idx,
            min_x,
            min_y,
            max_x,
            max_y,
            anchor_id,
        });
    }

    if bounds.len() < 2 {
        return;
    }

    let horizontal = is_horizontal(graph.direction);
    bounds.sort_by(|a, b| {
        let a_key = if horizontal { a.min_x } else { a.min_y };
        let b_key = if horizontal { b.min_x } else { b.min_y };
        a_key
            .partial_cmp(&b_key)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.idx.cmp(&b.idx))
    });

    let mut prev_max: Option<f32> = None;
    for bound in &bounds {
        let min_main = if horizontal { bound.min_x } else { bound.min_y };
        let max_main = if horizontal { bound.max_x } else { bound.max_y };
        if let Some(prev) = prev_max {
            if min_main < prev {
                return;
            }
        }
        prev_max = Some(max_main);
    }

    let target_cross = bounds
        .iter()
        .map(|b| if horizontal { b.min_y } else { b.min_x })
        .fold(f32::MAX, f32::min);

    for bound in bounds {
        let current_cross = if horizontal { bound.min_y } else { bound.min_x };
        let delta = target_cross - current_cross;
        if delta.abs() < 0.5 {
            continue;
        }
        let sub = &graph.subgraphs[bound.idx];
        for node_id in &sub.nodes {
            if let Some(node) = nodes.get_mut(node_id) {
                if horizontal {
                    node.y += delta;
                } else {
                    node.x += delta;
                }
            }
        }
        if let Some(anchor_id) = bound.anchor_id.as_deref() {
            if let Some(node) = nodes.get_mut(anchor_id) {
                if horizontal {
                    node.y += delta;
                } else {
                    node.x += delta;
                }
            }
        }
    }
}

fn align_disconnected_components(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart || !graph.subgraphs.is_empty() {
        return;
    }

    let mut visible_nodes: Vec<String> = nodes
        .values()
        .filter(|node| !node.hidden)
        .map(|node| node.id.clone())
        .collect();
    if visible_nodes.len() < 2 {
        return;
    }
    visible_nodes.sort();

    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    for node_id in &visible_nodes {
        adjacency.entry(node_id.clone()).or_default();
    }
    for edge in &graph.edges {
        if !adjacency.contains_key(&edge.from) || !adjacency.contains_key(&edge.to) {
            continue;
        }
        adjacency
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        adjacency
            .entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());
    }

    let mut visited: HashSet<String> = HashSet::new();
    let mut components: Vec<Vec<String>> = Vec::new();
    for node_id in &visible_nodes {
        if visited.contains(node_id) {
            continue;
        }
        let mut stack = vec![node_id.clone()];
        let mut comp = Vec::new();
        visited.insert(node_id.clone());
        while let Some(cur) = stack.pop() {
            comp.push(cur.clone());
            if let Some(neigh) = adjacency.get(&cur) {
                for next in neigh {
                    if visited.insert(next.clone()) {
                        stack.push(next.clone());
                    }
                }
            }
        }
        if comp.len() > 0 {
            components.push(comp);
        }
    }

    if components.len() < 2 {
        return;
    }

    #[derive(Clone)]
    struct CompBounds {
        nodes: Vec<String>,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
    }

    let mut bounds: Vec<CompBounds> = Vec::new();
    for comp in components {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for node_id in &comp {
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
        bounds.push(CompBounds {
            nodes: comp,
            min_x,
            min_y,
            max_x,
            max_y,
        });
    }

    if bounds.len() < 2 {
        return;
    }

    let horizontal = is_horizontal(graph.direction);
    bounds.sort_by(|a, b| {
        let a_key = if horizontal { a.min_x } else { a.min_y };
        let b_key = if horizontal { b.min_x } else { b.min_y };
        a_key.partial_cmp(&b_key).unwrap_or(Ordering::Equal)
    });

    let target_cross = bounds
        .iter()
        .map(|b| if horizontal { b.min_y } else { b.min_x })
        .fold(f32::MAX, f32::min);
    let spacing = config.node_spacing.max(MIN_NODE_SPACING_FLOOR);
    let mut cursor = if horizontal {
        bounds.iter().map(|b| b.min_x).fold(f32::MAX, f32::min)
    } else {
        bounds.iter().map(|b| b.min_y).fold(f32::MAX, f32::min)
    };

    for bound in bounds {
        let min_main = if horizontal { bound.min_x } else { bound.min_y };
        let max_main = if horizontal { bound.max_x } else { bound.max_y };
        let current_cross = if horizontal { bound.min_y } else { bound.min_x };
        let delta_main = cursor - min_main;
        let delta_cross = target_cross - current_cross;
        for node_id in &bound.nodes {
            if let Some(node) = nodes.get_mut(node_id) {
                if horizontal {
                    node.x += delta_main;
                    node.y += delta_cross;
                } else {
                    node.x += delta_cross;
                    node.y += delta_main;
                }
            }
        }
        let size = (max_main - min_main).max(1.0);
        cursor += size + spacing;
    }
}

fn apply_visual_objectives(
    graph: &Graph,
    layout_edges: &[crate::ir::Edge],
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) {
    if !config.flowchart.objective.enabled {
        return;
    }
    relax_edge_span_constraints(graph, layout_edges, nodes, theme, config);
    rebalance_top_level_subgraphs_aspect(graph, nodes, config);
    let overlap_pass_enabled = match graph.kind {
        crate::ir::DiagramKind::Class => true,
        crate::ir::DiagramKind::Flowchart
        | crate::ir::DiagramKind::State
        | crate::ir::DiagramKind::Er
        | crate::ir::DiagramKind::Requirement => has_visible_node_overlap(nodes),
        _ => false,
    };
    if overlap_pass_enabled {
        resolve_node_overlaps(graph, nodes, config);
    }
}

fn node_main_center(node: &NodeLayout, horizontal: bool) -> f32 {
    if horizontal {
        node.x + node.width / 2.0
    } else {
        node.y + node.height / 2.0
    }
}

fn node_main_half(node: &NodeLayout, horizontal: bool) -> f32 {
    if horizontal {
        node.width / 2.0
    } else {
        node.height / 2.0
    }
}

fn shift_node_main(node: &mut NodeLayout, horizontal: bool, delta: f32) {
    if horizontal {
        node.x += delta;
    } else {
        node.y += delta;
    }
}

fn shift_node_cross(node: &mut NodeLayout, horizontal: bool, delta: f32) {
    if horizontal {
        node.y += delta;
    } else {
        node.x += delta;
    }
}

fn relax_edge_span_constraints(
    graph: &Graph,
    layout_edges: &[crate::ir::Edge],
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) {
    if layout_edges.is_empty() {
        return;
    }
    match graph.kind {
        crate::ir::DiagramKind::Class
        | crate::ir::DiagramKind::Flowchart
        | crate::ir::DiagramKind::State
        | crate::ir::DiagramKind::Er
        | crate::ir::DiagramKind::Requirement => {}
        _ => return,
    }
    let horizontal = is_horizontal(graph.direction);
    let objective = &config.flowchart.objective;
    let passes = objective.edge_relax_passes.max(1);
    let step_limit = (config.rank_spacing + config.node_spacing).max(EDGE_RELAX_STEP_MIN);
    let mut label_cache: HashMap<String, TextBlock> = HashMap::new();

    for _ in 0..passes {
        let mut changed = false;
        for edge in layout_edges {
            let Some(from_node) = nodes.get(&edge.from) else {
                continue;
            };
            let Some(to_node) = nodes.get(&edge.to) else {
                continue;
            };
            if from_node.hidden || to_node.hidden {
                continue;
            }
            let from_main = node_main_center(from_node, horizontal);
            let to_main = node_main_center(to_node, horizontal);
            let from_main_half = node_main_half(from_node, horizontal);
            let to_main_half = node_main_half(to_node, horizontal);
            let main_delta = to_main - from_main;
            let current_main_gap = if main_delta >= 0.0 {
                (to_main - to_main_half) - (from_main + from_main_half)
            } else {
                (from_main - from_main_half) - (to_main + to_main_half)
            };

            let has_center_label = edge
                .label
                .as_deref()
                .is_some_and(|label| !label.trim().is_empty());
            let has_start_label = edge
                .start_label
                .as_deref()
                .is_some_and(|label| !label.trim().is_empty());
            let has_end_label = edge
                .end_label
                .as_deref()
                .is_some_and(|label| !label.trim().is_empty());
            let has_endpoint_label = has_start_label || has_end_label;
            // Flowchart dotted links are usually secondary annotations.
            // Let routing/label placement handle them without re-ranking rows.
            if graph.kind == crate::ir::DiagramKind::Flowchart
                && edge.style == crate::ir::EdgeStyle::Dotted
            {
                continue;
            }
            if !has_center_label && !has_endpoint_label {
                continue;
            }

            let mut required_main_gap =
                (config.node_spacing * objective.edge_gap_floor_ratio).max(8.0);
            if let Some(label) = edge
                .label
                .as_deref()
                .filter(|label| !label.trim().is_empty())
            {
                let label_block = label_cache
                    .entry(label.to_string())
                    .or_insert_with(|| measure_label(label, theme, config))
                    .clone();
                let label_extent = if horizontal {
                    label_block.width
                } else {
                    label_block.height
                };
                required_main_gap += label_extent * objective.edge_label_weight;
                required_main_gap += theme.font_size * EDGE_LABEL_PAD_SCALE;
            }
            if let Some(label) = edge
                .start_label
                .as_deref()
                .filter(|label| !label.trim().is_empty())
            {
                let label_block = label_cache
                    .entry(label.to_string())
                    .or_insert_with(|| measure_label(label, theme, config))
                    .clone();
                let label_extent = if horizontal {
                    label_block.width
                } else {
                    label_block.height
                };
                required_main_gap += label_extent * objective.endpoint_label_weight;
                required_main_gap += theme.font_size * ENDPOINT_LABEL_PAD_SCALE;
            }
            if let Some(label) = edge
                .end_label
                .as_deref()
                .filter(|label| !label.trim().is_empty())
            {
                let label_block = label_cache
                    .entry(label.to_string())
                    .or_insert_with(|| measure_label(label, theme, config))
                    .clone();
                let label_extent = if horizontal {
                    label_block.width
                } else {
                    label_block.height
                };
                required_main_gap += label_extent * objective.endpoint_label_weight;
                required_main_gap += theme.font_size * ENDPOINT_LABEL_PAD_SCALE;
            }
            if has_start_label && has_end_label {
                required_main_gap += theme.font_size * DUAL_ENDPOINT_EXTRA_PAD_SCALE;
            }
            let max_main_gap = (config.rank_spacing + config.node_spacing) * MAX_MAIN_GAP_FACTOR;
            required_main_gap = required_main_gap.min(max_main_gap);

            if current_main_gap + EDGE_RELAX_GAP_TOLERANCE < required_main_gap {
                let delta = (required_main_gap - current_main_gap).min(step_limit);
                let ahead_id = if main_delta >= 0.0 {
                    edge.to.as_str()
                } else {
                    edge.from.as_str()
                };
                if let Some(node) = nodes.get_mut(ahead_id) {
                    shift_node_main(node, horizontal, delta);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
}

fn resolve_node_overlaps(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    let horizontal = is_horizontal(graph.direction);
    let min_gap = (config.node_spacing * OVERLAP_MIN_GAP_RATIO).max(OVERLAP_MIN_GAP_FLOOR);
    let mut ids: Vec<String> = nodes
        .values()
        .filter(|node| !node.hidden)
        .map(|node| node.id.clone())
        .collect();
    if ids.len() < 2 {
        return;
    }
    ids.sort_by_key(|id| graph.node_order.get(id).copied().unwrap_or(usize::MAX));

    for _ in 0..OVERLAP_RESOLVE_PASSES {
        let mut moved = false;
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let id_a = &ids[i];
                let id_b = &ids[j];
                let (ax, ay, aw, ah, bx, by, bw, bh) = {
                    let Some(a) = nodes.get(id_a) else {
                        continue;
                    };
                    let Some(b) = nodes.get(id_b) else {
                        continue;
                    };
                    (a.x, a.y, a.width, a.height, b.x, b.y, b.width, b.height)
                };
                let overlap_x = (ax + aw).min(bx + bw) - ax.max(bx);
                let overlap_y = (ay + ah).min(by + bh) - ay.max(by);
                if overlap_x <= 0.0 || overlap_y <= 0.0 {
                    continue;
                }
                let (center_a, center_b) = if horizontal {
                    (ay + ah / 2.0, by + bh / 2.0)
                } else {
                    (ax + aw / 2.0, bx + bw / 2.0)
                };
                let mut sign = if center_b >= center_a { 1.0 } else { -1.0 };
                if (center_b - center_a).abs() < OVERLAP_CENTER_THRESHOLD {
                    let order_a = graph.node_order.get(id_a).copied().unwrap_or(usize::MAX);
                    let order_b = graph.node_order.get(id_b).copied().unwrap_or(usize::MAX);
                    sign = if order_b >= order_a { 1.0 } else { -1.0 };
                }
                let delta = if horizontal {
                    overlap_y + min_gap
                } else {
                    overlap_x + min_gap
                };
                if let Some(node_b) = nodes.get_mut(id_b) {
                    shift_node_cross(node_b, horizontal, sign * delta);
                    moved = true;
                }
            }
        }
        if !moved {
            break;
        }
    }
}

fn has_visible_node_overlap(nodes: &BTreeMap<String, NodeLayout>) -> bool {
    let mut visible: Vec<&NodeLayout> = nodes.values().filter(|node| !node.hidden).collect();
    if visible.len() < 2 {
        return false;
    }
    visible.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(Ordering::Equal));
    for i in 0..visible.len() {
        let a = visible[i];
        for b in visible.iter().skip(i + 1) {
            if b.x >= a.x + a.width {
                break;
            }
            let overlap_x = (a.x + a.width).min(b.x + b.width) - a.x.max(b.x);
            let overlap_y = (a.y + a.height).min(b.y + b.height) - a.y.max(b.y);
            if overlap_x > 0.0 && overlap_y > 0.0 {
                return true;
            }
        }
    }
    false
}

#[derive(Clone)]
struct VisualGroup {
    sub_idx: usize,
    nodes: Vec<String>,
    min_main: f32,
    max_main: f32,
    min_cross: f32,
    max_cross: f32,
}

fn rebalance_top_level_subgraphs_aspect(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart {
        return;
    }
    if graph.subgraphs.len() < 2 {
        return;
    }
    if graph.nodes.len() < 120 {
        return;
    }
    let horizontal = is_horizontal(graph.direction);
    let mut groups = collect_top_level_visual_groups(graph, nodes, horizontal);
    let objective = &config.flowchart.objective;
    if groups.len() < objective.wrap_min_groups {
        return;
    }

    let min_main = groups
        .iter()
        .map(|group| group.min_main)
        .fold(f32::MAX, f32::min);
    let max_main = groups
        .iter()
        .map(|group| group.max_main)
        .fold(f32::MIN, f32::max);
    let min_cross = groups
        .iter()
        .map(|group| group.min_cross)
        .fold(f32::MAX, f32::min);
    let max_cross = groups
        .iter()
        .map(|group| group.max_cross)
        .fold(f32::MIN, f32::max);
    if min_main == f32::MAX || min_cross == f32::MAX {
        return;
    }

    let main_span = (max_main - min_main).max(1.0);
    let cross_span = (max_cross - min_cross).max(1.0);
    let target_aspect = objective.max_aspect_ratio.max(1.0);
    let aspect = main_span / cross_span;
    if aspect <= target_aspect {
        return;
    }

    let row_count = if top_level_subgraph_chain_like(graph, &groups) {
        ((aspect / target_aspect).ceil() as usize).clamp(2, groups.len())
    } else {
        ((aspect / target_aspect).sqrt().ceil() as usize).clamp(2, groups.len())
    };
    let base_row_len = groups.len() / row_count;
    let extra_rows = groups.len() % row_count;
    let gap_main = config.node_spacing.max(12.0) * objective.wrap_main_gap_scale.max(0.1);
    let gap_cross = config.rank_spacing.max(12.0) * objective.wrap_cross_gap_scale.max(0.1);

    let mut row_start = 0usize;
    let mut cursor_cross = min_cross;
    for row in 0..row_count {
        let row_len = base_row_len + usize::from(row < extra_rows);
        if row_len == 0 {
            continue;
        }
        let row_end = row_start + row_len;
        let mut cursor_main = min_main;
        let mut row_cross_span = 0.0_f32;
        for group in &mut groups[row_start..row_end] {
            let delta_main = cursor_main - group.min_main;
            let delta_cross = cursor_cross - group.min_cross;
            for node_id in &group.nodes {
                if let Some(node) = nodes.get_mut(node_id) {
                    shift_node_main(node, horizontal, delta_main);
                    shift_node_cross(node, horizontal, delta_cross);
                }
            }
            group.min_main += delta_main;
            group.max_main += delta_main;
            group.min_cross += delta_cross;
            group.max_cross += delta_cross;
            cursor_main = group.max_main + gap_main;
            row_cross_span = row_cross_span.max(group.max_cross - group.min_cross);
        }
        cursor_cross += row_cross_span + gap_cross;
        row_start = row_end;
    }
}

fn collect_top_level_visual_groups(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    horizontal: bool,
) -> Vec<VisualGroup> {
    let top_level = top_level_subgraph_indices(graph);
    if top_level.len() < 2 {
        return Vec::new();
    }

    let mut seen: HashSet<&str> = HashSet::new();
    for &idx in &top_level {
        for node_id in &graph.subgraphs[idx].nodes {
            if !seen.insert(node_id.as_str()) {
                return Vec::new();
            }
        }
    }

    let mut groups = Vec::new();
    for &idx in &top_level {
        let sub = &graph.subgraphs[idx];
        if is_region_subgraph(sub) {
            continue;
        }
        let mut ids: Vec<String> = Vec::new();
        let mut min_main = f32::MAX;
        let mut max_main = f32::MIN;
        let mut min_cross = f32::MAX;
        let mut max_cross = f32::MIN;
        for node_id in &sub.nodes {
            let Some(node) = nodes.get(node_id) else {
                continue;
            };
            if node.hidden {
                continue;
            }
            ids.push(node_id.clone());
            let (main_start, main_end) = if horizontal {
                (node.x, node.x + node.width)
            } else {
                (node.y, node.y + node.height)
            };
            let (cross_start, cross_end) = if horizontal {
                (node.y, node.y + node.height)
            } else {
                (node.x, node.x + node.width)
            };
            min_main = min_main.min(main_start);
            max_main = max_main.max(main_end);
            min_cross = min_cross.min(cross_start);
            max_cross = max_cross.max(cross_end);
        }
        if ids.is_empty() {
            continue;
        }
        groups.push(VisualGroup {
            sub_idx: idx,
            nodes: ids,
            min_main,
            max_main,
            min_cross,
            max_cross,
        });
    }
    groups.sort_by(|a, b| {
        a.min_main
            .partial_cmp(&b.min_main)
            .unwrap_or(Ordering::Equal)
    });
    groups
}

fn top_level_subgraph_chain_like(graph: &Graph, groups: &[VisualGroup]) -> bool {
    if groups.len() < 3 {
        return false;
    }
    let mut node_to_subgraph: HashMap<&str, usize> = HashMap::new();
    for group in groups {
        for node_id in &group.nodes {
            node_to_subgraph.insert(node_id.as_str(), group.sub_idx);
        }
    }

    let mut indegree: HashMap<usize, usize> = HashMap::new();
    let mut outdegree: HashMap<usize, usize> = HashMap::new();
    let mut cross_edges = 0usize;
    for edge in &graph.edges {
        let Some(&from_sub) = node_to_subgraph.get(edge.from.as_str()) else {
            continue;
        };
        let Some(&to_sub) = node_to_subgraph.get(edge.to.as_str()) else {
            continue;
        };
        if from_sub == to_sub {
            continue;
        }
        cross_edges += 1;
        *outdegree.entry(from_sub).or_default() += 1;
        *indegree.entry(to_sub).or_default() += 1;
    }
    if cross_edges < groups.len().saturating_sub(1) {
        return false;
    }
    for group in groups {
        if indegree.get(&group.sub_idx).copied().unwrap_or(0) > 1 {
            return false;
        }
        if outdegree.get(&group.sub_idx).copied().unwrap_or(0) > 1 {
            return false;
        }
    }
    true
}

fn build_subgraph_layouts(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) -> Vec<SubgraphLayout> {
    let mut subgraphs = Vec::new();
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

        if min_x == f32::MAX {
            continue;
        }

        let style = resolve_subgraph_style(sub, graph);
        let mut label_block = measure_label(&sub.label, theme, config);
        let label_empty = sub.label.trim().is_empty();
        if label_empty {
            label_block.width = 0.0;
            label_block.height = 0.0;
        }
        let (padding_x, padding_y, top_padding) =
            subgraph_padding_from_label(graph, sub, theme, &label_block);

        let node_width = max_x - min_x;
        let base_width = node_width + padding_x * 2.0;
        let min_label_width = if label_empty {
            base_width
        } else {
            label_block.width + padding_x * 2.0
        };
        let width = base_width.max(min_label_width);
        let extra_width = width - base_width;

        subgraphs.push(SubgraphLayout {
            label: sub.label.clone(),
            label_block,
            nodes: sub.nodes.clone(),
            x: min_x - padding_x - extra_width / 2.0,
            y: min_y - top_padding,
            width,
            height: (max_y - min_y) + padding_y + top_padding,
            style,
            icon: sub.icon.clone(),
        });
    }

    if subgraphs.len() > 1 {
        let tree = SubgraphTree::build(graph);

        // Collect all descendants for each subgraph via the tree so we only
        // visit actual parent-child pairs instead of every O(n²) combination.
        // Process from leaves up so that child bounds are final before parents
        // expand to contain them.
        let mut all_descendants: Vec<Vec<usize>> = vec![Vec::new(); subgraphs.len()];
        // Post-order traversal: collect leaves first, then parents.
        let mut order: Vec<usize> = Vec::with_capacity(subgraphs.len());
        let mut stack: Vec<(usize, bool)> =
            tree.top_level.iter().rev().map(|&i| (i, false)).collect();
        while let Some((idx, visited)) = stack.pop() {
            if visited {
                order.push(idx);
                continue;
            }
            stack.push((idx, true));
            for &child in tree.children[idx].iter().rev() {
                stack.push((child, false));
            }
        }

        // Build transitive descendant lists bottom-up.
        for &idx in &order {
            let mut descs = Vec::new();
            for &child in &tree.children[idx] {
                descs.push(child);
                descs.extend(all_descendants[child].iter().copied());
            }
            all_descendants[idx] = descs;
        }

        // Expand each parent's bounds to contain all its descendants.
        for &i in &order {
            for &j in &all_descendants[i] {
                if is_region_subgraph(&graph.subgraphs[j]) {
                    continue;
                }
                let pad = if graph.kind == crate::ir::DiagramKind::State {
                    (theme.font_size * 1.8).max(24.0)
                } else {
                    12.0
                };
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
        };
        let points = route_edge_with_avoidance(&ctx, None, None, None);
        let dist = polyline_point_distance(&points, preferred);
        assert!(
            dist <= 0.51,
            "expected routed path to pass through preferred label center, got distance {dist:.3}"
        );
    }
}
