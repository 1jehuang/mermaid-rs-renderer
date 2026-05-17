use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

use crate::config::LayoutConfig;
use crate::ir::{DiagramKind, Graph};

use super::super::label_placement;
use super::super::routing::*;
use super::super::{
    EDGE_OCCUPANCY_CELL_RATIO, EdgeLayout, FLOWCHART_EDGE_LABEL_WRAP_TRIGGER_CHARS,
    LayoutStageMetrics, MIN_NODE_SPACING_FLOOR, MULTI_EDGE_OFFSET_RATIO, NodeLayout,
    SubgraphLayout, TextBlock, anchor_layout_for_edge,
};
use super::path_cleanup;
use super::plan;
use super::post_route;
use super::roles;
use super::route_labels;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PortTrack {
    Side(EdgeSide),
    Axis(PortAxis),
}

fn is_branching_port_shape(node: &NodeLayout) -> bool {
    matches!(
        node.shape,
        crate::ir::NodeShape::Diamond
            | crate::ir::NodeShape::Hexagon
            | crate::ir::NodeShape::Parallelogram
            | crate::ir::NodeShape::ParallelogramAlt
            | crate::ir::NodeShape::Trapezoid
            | crate::ir::NodeShape::TrapezoidAlt
            | crate::ir::NodeShape::Asymmetric
    )
}

fn opposite_side(side: EdgeSide) -> EdgeSide {
    match side {
        EdgeSide::Left => EdgeSide::Right,
        EdgeSide::Right => EdgeSide::Left,
        EdgeSide::Top => EdgeSide::Bottom,
        EdgeSide::Bottom => EdgeSide::Top,
    }
}

fn use_axis_wide_port_track(
    node: &NodeLayout,
    side: EdgeSide,
    degree: usize,
    side_counts: [usize; 4],
) -> bool {
    if degree <= 2 {
        return false;
    }
    let axis_total = side_counts[side_slot(side)] + side_counts[side_slot(opposite_side(side))];
    if axis_total <= 2 {
        return false;
    }
    let side_count = side_counts[side_slot(side)];
    let opposite_count = side_counts[side_slot(opposite_side(side))];
    side_count > 1 || opposite_count > 1 || (is_branching_port_shape(node) && axis_total >= 3)
}

fn port_track_for_assignment(
    node: &NodeLayout,
    side: EdgeSide,
    degree: usize,
    side_counts: [usize; 4],
) -> PortTrack {
    if use_axis_wide_port_track(node, side, degree, side_counts) {
        PortTrack::Axis(port_axis(side))
    } else {
        PortTrack::Side(side)
    }
}

fn port_track_node_len(node: &NodeLayout, track: PortTrack) -> f32 {
    match track {
        PortTrack::Side(side) => {
            if side_is_vertical(side) {
                node.height
            } else {
                node.width
            }
        }
        PortTrack::Axis(PortAxis::X) => node.width,
        PortTrack::Axis(PortAxis::Y) => node.height,
    }
}

fn port_track_node_start(node: &NodeLayout, track: PortTrack) -> f32 {
    match track {
        PortTrack::Side(side) => {
            if side_is_vertical(side) {
                node.y
            } else {
                node.x
            }
        }
        PortTrack::Axis(PortAxis::X) => node.x,
        PortTrack::Axis(PortAxis::Y) => node.y,
    }
}

#[derive(Debug, Clone, Copy)]
struct NodeBounds {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
}

fn visible_node_bounds(nodes: &BTreeMap<String, NodeLayout>) -> Option<NodeBounds> {
    let mut bounds = NodeBounds {
        min_x: f32::MAX,
        max_x: f32::MIN,
        min_y: f32::MAX,
        max_y: f32::MIN,
    };
    let mut any = false;
    for node in nodes.values() {
        if node.hidden || node.anchor_subgraph.is_some() {
            continue;
        }
        any = true;
        bounds.min_x = bounds.min_x.min(node.x);
        bounds.max_x = bounds.max_x.max(node.x + node.width);
        bounds.min_y = bounds.min_y.min(node.y);
        bounds.max_y = bounds.max_y.max(node.y + node.height);
    }
    any.then_some(bounds)
}

fn choose_outer_back_edge_sides(
    from: &NodeLayout,
    to: &NodeLayout,
    direction: crate::ir::Direction,
    bounds: Option<NodeBounds>,
    fallback: (EdgeSide, EdgeSide, bool),
) -> (EdgeSide, EdgeSide, bool) {
    let Some(bounds) = bounds else {
        return fallback;
    };

    if is_horizontal(direction) {
        let upper_clearance = from.y.min(to.y) - bounds.min_y;
        let lower_clearance = bounds.max_y - (from.y + from.height).max(to.y + to.height);
        let side = if (upper_clearance - lower_clearance).abs() <= 1.0 {
            let avg_y = (from.y + from.height * 0.5 + to.y + to.height * 0.5) * 0.5;
            if avg_y <= (bounds.min_y + bounds.max_y) * 0.5 {
                EdgeSide::Top
            } else {
                EdgeSide::Bottom
            }
        } else if upper_clearance <= lower_clearance {
            EdgeSide::Top
        } else {
            EdgeSide::Bottom
        };
        (side, side, fallback.2)
    } else {
        let left_clearance = from.x.min(to.x) - bounds.min_x;
        let right_clearance = bounds.max_x - (from.x + from.width).max(to.x + to.width);
        let side = if (left_clearance - right_clearance).abs() <= 1.0 {
            let avg_x = (from.x + from.width * 0.5 + to.x + to.width * 0.5) * 0.5;
            if avg_x <= (bounds.min_x + bounds.max_x) * 0.5 {
                EdgeSide::Left
            } else {
                EdgeSide::Right
            }
        } else if left_clearance <= right_clearance {
            EdgeSide::Left
        } else {
            EdgeSide::Right
        };
        (side, side, fallback.2)
    }
}

pub(in crate::layout) struct RoutedEdgeBuildContext<'a> {
    pub(in crate::layout) graph: &'a Graph,
    pub(in crate::layout) nodes: &'a BTreeMap<String, NodeLayout>,
    pub(in crate::layout) subgraphs: &'a [SubgraphLayout],
    pub(in crate::layout) config: &'a LayoutConfig,
    pub(in crate::layout) layout_node_count: usize,
    pub(in crate::layout) edge_route_labels: &'a [Option<TextBlock>],
    pub(in crate::layout) edge_start_labels: &'a [Option<TextBlock>],
    pub(in crate::layout) edge_end_labels: &'a [Option<TextBlock>],
    pub(in crate::layout) label_dummy_ids: &'a [Option<String>],
    pub(in crate::layout) tiny_graph: bool,
    pub(in crate::layout) stage_metrics: Option<&'a mut LayoutStageMetrics>,
}

pub(in crate::layout) fn build_routed_edges(ctx: RoutedEdgeBuildContext<'_>) -> Vec<EdgeLayout> {
    let RoutedEdgeBuildContext {
        graph,
        nodes,
        subgraphs,
        config,
        layout_node_count,
        edge_route_labels,
        edge_start_labels,
        edge_end_labels,
        label_dummy_ids,
        tiny_graph,
        stage_metrics,
    } = ctx;
    let obstacles = build_obstacles(nodes, subgraphs, config);
    let label_obstacles = build_label_obstacles_for_routing(nodes, subgraphs);
    let routing_grid = if config.flowchart.routing.enable_grid_router && !tiny_graph {
        build_routing_grid(&obstacles, config)
    } else {
        None
    };
    let mut stage_metrics = stage_metrics;

    let port_assignment_start = crate::time::Instant::now();
    let content_bounds = visible_node_bounds(nodes);
    let mut node_degrees: HashMap<String, usize> = HashMap::new();
    for edge in &graph.edges {
        *node_degrees.entry(edge.from.clone()).or_insert(0) += 1;
        *node_degrees.entry(edge.to.clone()).or_insert(0) += 1;
    }
    let edge_roles = roles::classify_edge_roles(graph);
    let mut side_loads: HashMap<String, [usize; 4]> = HashMap::new();
    let mut edge_ports: Vec<EdgePortInfo> = vec![
        EdgePortInfo {
            start_side: EdgeSide::Right,
            end_side: EdgeSide::Left,
            start_offset: 0.0,
            end_offset: 0.0,
        };
        graph.edges.len()
    ];
    let mut selected_edge_sides: Vec<(EdgeSide, EdgeSide)> =
        vec![(EdgeSide::Right, EdgeSide::Left); graph.edges.len()];
    let mut port_candidates: HashMap<(String, PortTrack), Vec<PortCandidate>> = HashMap::new();
    let mut side_choice_segments: Vec<Segment> = Vec::with_capacity(graph.edges.len());
    for (idx, edge) in graph.edges.iter().enumerate() {
        let (Some(from_layout), Some(to_layout)) = (nodes.get(&edge.from), nodes.get(&edge.to))
        else {
            continue;
        };
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
        let use_balanced_sides = !matches!(graph.kind, DiagramKind::Architecture);
        let from_degree = node_degrees.get(&edge.from).copied().unwrap_or(0);
        let to_degree = node_degrees.get(&edge.to).copied().unwrap_or(0);
        let edge_role = edge_roles.get(idx).copied().unwrap_or_default();
        let allow_low_degree_balancing = from_degree <= 4
            && to_degree <= 4
            && (edge.style == crate::ir::EdgeStyle::Dotted
                || edge_role.is_back_edge
                || edge_role.crosses_subgraph_boundary);
        let primary_sides = edge_sides(from, to, graph.direction);
        let mut selected_sides = if use_balanced_sides {
            let balanced = edge_sides_balanced(
                &edge.from,
                &edge.to,
                from,
                to,
                allow_low_degree_balancing,
                edge_role.is_back_edge,
                graph.direction,
                &node_degrees,
                &side_loads,
            );
            if edge_role.is_back_edge {
                choose_outer_back_edge_sides(from, to, graph.direction, content_bounds, balanced)
            } else {
                balanced
            }
        } else {
            primary_sides
        };
        if use_balanced_sides
            && !edge_role.is_back_edge
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
        edge_ports[idx] = EdgePortInfo {
            start_side,
            end_side,
            start_offset: 0.0,
            end_offset: 0.0,
        };
        selected_edge_sides[idx] = (start_side, end_side);

        let from_anchor = anchor_point_for_node(from, start_side, 0.0);
        let to_anchor = anchor_point_for_node(to, end_side, 0.0);
        side_choice_segments.push((from_anchor, to_anchor));
    }
    let mut node_side_counts: HashMap<String, [usize; 4]> = HashMap::new();
    for (idx, edge) in graph.edges.iter().enumerate() {
        let (start_side, end_side) = selected_edge_sides[idx];
        bump_side_load(&mut node_side_counts, &edge.from, start_side);
        bump_side_load(&mut node_side_counts, &edge.to, end_side);
    }
    for (idx, edge) in graph.edges.iter().enumerate() {
        let (Some(from_layout), Some(to_layout)) = (nodes.get(&edge.from), nodes.get(&edge.to))
        else {
            continue;
        };
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
        let from_degree = node_degrees.get(&edge.from).copied().unwrap_or(0);
        let to_degree = node_degrees.get(&edge.to).copied().unwrap_or(0);
        let (start_side, end_side) = selected_edge_sides[idx];
        let start_counts = node_side_counts.get(&edge.from).copied().unwrap_or([0; 4]);
        let end_counts = node_side_counts.get(&edge.to).copied().unwrap_or([0; 4]);
        let from_anchor = anchor_point_for_node(from, start_side, 0.0);
        let to_anchor = anchor_point_for_node(to, end_side, 0.0);
        let start_other = ideal_port_pos((to_anchor.0, to_anchor.1), from, start_side);
        let end_other = ideal_port_pos((from_anchor.0, from_anchor.1), to, end_side);
        let start_track = port_track_for_assignment(from, start_side, from_degree, start_counts);
        let end_track = port_track_for_assignment(to, end_side, to_degree, end_counts);
        port_candidates
            .entry((edge.from.clone(), start_track))
            .or_default()
            .push(PortCandidate {
                edge_idx: idx,
                is_start: true,
                other_pos: start_other,
            });
        port_candidates
            .entry((edge.to.clone(), end_track))
            .or_default()
            .push(PortCandidate {
                edge_idx: idx,
                is_start: false,
                other_pos: end_other,
            });
    }
    let routing_cell = routing_cell_size(config);
    for ((node_id, track), candidates) in port_candidates {
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
        let node_len = port_track_node_len(node, track);
        let pad = (node_len * config.flowchart.port_pad_ratio)
            .min(config.flowchart.port_pad_max)
            .max(config.flowchart.port_pad_min);
        let usable = (node_len - 2.0 * pad).max(1.0);
        let nominal_sep = usable / (candidates.len() as f32 + 1.0);
        let labeled_edges = candidates
            .iter()
            .filter(|candidate| {
                graph.edges.get(candidate.edge_idx).is_some_and(|edge| {
                    edge.label
                        .as_deref()
                        .is_some_and(|label| !label.trim().is_empty())
                        || edge
                            .start_label
                            .as_deref()
                            .is_some_and(|label| !label.trim().is_empty())
                        || edge
                            .end_label
                            .as_deref()
                            .is_some_and(|label| !label.trim().is_empty())
                })
            })
            .count() as f32;
        let congestion = candidates.len() as f32;
        let sep_boost =
            1.0 + (labeled_edges * 0.07).min(0.35) + ((congestion - 3.0).max(0.0) * 0.03).min(0.25);
        let grid_floor = if routing_cell > 0.0 {
            routing_cell * 0.85
        } else {
            0.0
        };
        let desired_sep = (nominal_sep * sep_boost).max(grid_floor);
        let feasible_sep = if candidates.len() <= 1 {
            usable
        } else {
            usable / (candidates.len() as f32 - 0.15)
        };
        let min_sep = desired_sep.min(feasible_sep.max(nominal_sep));
        let snap_to_grid = config.flowchart.routing.snap_ports_to_grid
            && routing_cell > 0.0
            && min_sep >= routing_cell * 0.75;
        let node_start = port_track_node_start(node, track);
        let ideal_span = span;
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
                let side_bias_scale = if candidates.len() > 2 {
                    1.0 + ((candidates.len() as f32 - 2.0) * 0.08).min(0.6)
                } else {
                    1.0
                };
                offset += config.flowchart.port_side_bias
                    * side_bias_scale
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
    if let Some(metrics) = stage_metrics.as_mut() {
        metrics.port_assignment_us = metrics
            .port_assignment_us
            .saturating_add(port_assignment_start.elapsed().as_micros());
    }

    let edge_routing_start = crate::time::Instant::now();
    let lane_assignments = plan::plan_edge_lanes(graph, nodes, subgraphs, config);
    let lane_offsets = lane_assignments.effective_offsets(&edge_ports, graph.kind, config);
    let pair_counts = lane_assignments.pair_counts;
    let pair_index = lane_assignments.pair_index;
    let cross_edge_offsets = lane_assignments.cross_edge_offsets;

    let mut route_order: Vec<(u8, f32, f32, usize)> = Vec::with_capacity(graph.edges.len());
    let dense_flowchart_routing = graph.kind == DiagramKind::Flowchart
        && graph.edges.len() >= 18
        && graph.edges.len() * 2 >= layout_node_count * 3;
    for (idx, edge) in graph.edges.iter().enumerate() {
        let (Some(from_layout), Some(to_layout)) = (nodes.get(&edge.from), nodes.get(&edge.to))
        else {
            continue;
        };
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
        let priority = if graph.kind == DiagramKind::Class {
            if has_open_triangle {
                0u8
            } else if is_secondary || is_backward {
                1u8
            } else {
                2u8
            }
        } else if graph.kind == DiagramKind::State {
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
            let dense_by_length = len_a.partial_cmp(&len_b).unwrap_or(Ordering::Equal);
            if use_priority_preorder {
                a.0.cmp(&b.0)
                    .then_with(|| {
                        if dense_flowchart_routing {
                            dense_by_length
                        } else {
                            by_length
                        }
                    })
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
    let route_labels_via = route_labels::should_route_labels_via(graph, nodes);
    let (edge_label_pad_x, edge_label_pad_y) =
        label_placement::edge_label_padding(graph.kind, config);
    let (mut route_label_plans, mut route_label_obstacles) =
        route_labels::initialize_route_label_plans(
            graph,
            nodes,
            subgraphs,
            &edge_ports,
            &pair_index,
            &lane_offsets,
            edge_route_labels,
            label_obstacles,
            config,
        );
    let mut existing_segments: Vec<Segment> = Vec::new();
    let mut label_anchors: Vec<Option<(f32, f32)>> = vec![None; graph.edges.len()];
    for (_, _, _, idx) in &route_order {
        let edge = &graph.edges[*idx];
        let key = edge_pair_key(edge);
        let total = *pair_counts.get(&key).unwrap_or(&1) as f32;
        let idx_in_pair = pair_index[*idx] as f32;
        let base_offset = if graph.kind == DiagramKind::Flowchart {
            lane_offsets.get(*idx).copied().unwrap_or_default()
        } else if total > 1.0 {
            (idx_in_pair - (total - 1.0) / 2.0) * (config.node_spacing * MULTI_EDGE_OFFSET_RATIO)
                + cross_edge_offsets[*idx]
        } else {
            cross_edge_offsets[*idx]
        };
        let (Some(from_layout), Some(to_layout)) = (nodes.get(&edge.from), nodes.get(&edge.to))
        else {
            continue;
        };
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
        let port_info = edge_ports.get(*idx).copied().unwrap_or(EdgePortInfo {
            start_side: EdgeSide::Right,
            end_side: EdgeSide::Left,
            start_offset: 0.0,
            end_offset: 0.0,
        });
        let default_stub = port_stub_length(config, from, to);
        let stub_len = match graph.kind {
            DiagramKind::Class | DiagramKind::Er | DiagramKind::Requirement => 0.0,
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
        let avoid_short_tie = graph.kind == DiagramKind::Flowchart
            && (has_endpoint_label
                || max_edge_label_chars >= FLOWCHART_EDGE_LABEL_WRAP_TRIGGER_CHARS);
        let preferred_label_plan = route_label_plans.get(*idx).and_then(|plan| plan.as_ref());
        let preferred_label_id = preferred_label_plan.map(|plan| plan.obstacle_id.as_str());
        let preferred_label_obstacle =
            preferred_label_plan.and_then(|plan| route_label_obstacles.get(plan.obstacle_index));
        let preferred_label_clearance = if graph.kind == DiagramKind::Flowchart {
            (edge_label_pad_x.max(edge_label_pad_y) + config.node_spacing * 0.25).max(8.0)
        } else {
            0.0
        };
        let preferred_label_center = if matches!(graph.kind, DiagramKind::State | DiagramKind::Er)
            || graph.kind == DiagramKind::Flowchart
        {
            None
        } else {
            preferred_label_plan.map(|plan| plan.center)
        };
        let start_inset = if edge.arrow_start {
            crate::edge_geometry::arrowhead_inset(graph.kind, edge.arrow_start_kind)
        } else {
            0.0
        };
        let end_inset = if edge.arrow_end {
            crate::edge_geometry::arrowhead_inset(graph.kind, edge.arrow_end_kind)
        } else {
            0.0
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
            preferred_label_obstacle,
            preferred_label_clearance,
            force_preferred_label_via: graph.kind != DiagramKind::Flowchart,
            coarse_grid_retry: graph.kind == DiagramKind::Flowchart,
        };
        let use_existing_for_edge = !(matches!(graph.kind, DiagramKind::Class | DiagramKind::Er)
            && edge.style == crate::ir::EdgeStyle::Dotted);
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
        if matches!(graph.kind, DiagramKind::Class | DiagramKind::Er) {
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
                preferred_label_obstacle: route_ctx.preferred_label_obstacle,
                preferred_label_clearance: route_ctx.preferred_label_clearance,
                force_preferred_label_via: route_ctx.force_preferred_label_via,
                coarse_grid_retry: route_ctx.coarse_grid_retry,
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
        if route_labels_via {
            let mut sync_ctx = route_labels::RouteLabelSyncContext {
                direction: graph.direction,
                kind: graph.kind,
                route_label_plans: &mut route_label_plans,
                label_anchors: &mut label_anchors,
                edge_route_labels,
                route_label_obstacles: &mut route_label_obstacles,
                edge_label_pad_x,
                edge_label_pad_y,
                update_obstacle: true,
            };
            route_labels::sync_route_label_plan_with_points(*idx, &mut points, &mut sync_ctx);
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

    post_route::apply_edge_path_cleanup(graph, nodes, &mut routed_points, config);

    if route_labels_via {
        for idx in 0..routed_points.len() {
            let mut sync_ctx = route_labels::RouteLabelSyncContext {
                direction: graph.direction,
                kind: graph.kind,
                route_label_plans: &mut route_label_plans,
                label_anchors: &mut label_anchors,
                edge_route_labels,
                route_label_obstacles: &mut route_label_obstacles,
                edge_label_pad_x,
                edge_label_pad_y,
                update_obstacle: false,
            };
            route_labels::sync_route_label_plan_with_points(
                idx,
                &mut routed_points[idx],
                &mut sync_ctx,
            );
        }
    }
    if graph.kind == DiagramKind::Flowchart {
        path_cleanup::detour_flowchart_paths_around_non_endpoint_nodes(
            graph,
            nodes,
            &mut routed_points,
            config,
        );
        path_cleanup::simplify_flowchart_axis_oscillations(&mut routed_points);
        path_cleanup::detour_flowchart_paths_around_non_endpoint_nodes(
            graph,
            nodes,
            &mut routed_points,
            config,
        );
    }

    route_labels::apply_label_dummy_anchors(
        nodes,
        label_dummy_ids,
        &mut routed_points,
        &mut label_anchors,
        graph.direction,
        graph.kind,
    );
    if graph.kind == DiagramKind::Flowchart {
        let route_label_centers = route_labels::route_label_centers(&route_label_plans);
        let plan_snapshot = plan::FlowchartLayoutPlan::from_current_pipeline(
            graph,
            nodes,
            subgraphs,
            &edge_ports,
            &pair_counts,
            &pair_index,
            &cross_edge_offsets,
            &routed_points,
            &label_anchors,
            &route_label_centers,
            edge_route_labels,
            config,
        );
        debug_assert!(plan_snapshot.is_consistent());
    }
    if let Some(metrics) = stage_metrics {
        metrics.edge_routing_us = metrics
            .edge_routing_us
            .saturating_add(edge_routing_start.elapsed().as_micros());
    }

    post_route::build_edge_layouts(
        graph,
        &routed_points,
        edge_route_labels,
        edge_start_labels,
        edge_end_labels,
        &label_anchors,
        config,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{NodeShape, NodeStyle};
    use crate::layout::TextBlock;

    fn node(shape: NodeShape) -> NodeLayout {
        NodeLayout {
            id: "n".to_string(),
            x: 0.0,
            y: 0.0,
            width: 120.0,
            height: 80.0,
            label: TextBlock {
                lines: Vec::new(),
                width: 0.0,
                height: 0.0,
            },
            shape,
            style: NodeStyle::default(),
            link: None,
            anchor_subgraph: None,
            hidden: false,
            icon: None,
        }
    }

    #[test]
    fn axis_wide_track_skips_simple_pass_through_axis() {
        let counts = [1, 0, 1, 1];
        assert!(!use_axis_wide_port_track(
            &node(NodeShape::Diamond),
            EdgeSide::Top,
            3,
            counts,
        ));
        assert_eq!(
            port_track_for_assignment(&node(NodeShape::Diamond), EdgeSide::Top, 3, counts),
            PortTrack::Side(EdgeSide::Top)
        );
    }

    #[test]
    fn axis_wide_track_engages_for_real_same_axis_contention() {
        let counts = [0, 0, 2, 1];
        assert!(use_axis_wide_port_track(
            &node(NodeShape::Diamond),
            EdgeSide::Top,
            3,
            counts,
        ));
        assert_eq!(
            port_track_for_assignment(&node(NodeShape::Diamond), EdgeSide::Top, 3, counts),
            PortTrack::Axis(PortAxis::X)
        );
    }
}
