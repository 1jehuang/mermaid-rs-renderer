// Label placement and collision avoidance for edge labels.
// Moved from render.rs — all functions here work with pure geometry,
// no SVG dependency.

use super::{EdgeLayout, NodeLayout, SubgraphLayout, TextBlock};
use crate::config::LayoutConfig;
use crate::ir::DiagramKind;
use crate::theme::Theme;
use std::collections::{BTreeMap, HashMap, HashSet};

const LABEL_OVERLAP_WIDE_THRESHOLD: f32 = 1e-4;
const LABEL_ANCHOR_FRACTIONS: [f32; 5] = [0.5, 0.35, 0.65, 0.2, 0.8];
const LABEL_ANCHOR_POS_EPS: f32 = 1.0;
const LABEL_ANCHOR_DIR_EPS: f32 = 0.02;
const LABEL_EXTRA_SEGMENT_ANCHORS: usize = 6;
const FLOWCHART_LABEL_CLEARANCE_PAD: f32 = 1.5;
const FLOWCHART_LABEL_SOFT_GAP: f32 = 6.0;

type Rect = (f32, f32, f32, f32);
type EdgeObstacle = (usize, Rect);

#[derive(Clone, Copy, Debug)]
struct CenterLabelOverlapScore {
    count: usize,
    total: f32,
    max: f32,
}

impl CenterLabelOverlapScore {
    fn improved_by(self, other: Self) -> bool {
        self.count < other.count
            || (self.count == other.count && self.max + 0.1 < other.max)
            || (self.count == other.count
                && (self.max - other.max).abs() <= 0.1
                && self.total + 0.1 < other.total)
    }
}

#[derive(Clone)]
struct FlowchartCenterLabelEntry {
    edge_idx: usize,
    label_w: f32,
    label_h: f32,
    initial_center: (f32, f32),
    initial_s_norm: f32,
    initial_d_signed: f32,
    current_center: (f32, f32),
    edge_points: Vec<(f32, f32)>,
    candidates: Vec<(f32, f32)>,
}

#[derive(Clone, Copy)]
struct FlowchartCenterCandidate {
    center: (f32, f32),
    rect: Rect,
    cost: (f32, f32),
    own_gap: f32,
    center_dist: f32,
    center_target: f32,
    center_soft_max: f32,
    center_hard_max: f32,
    fixed_overlap_count: u32,
    fixed_overlap_area: f32,
    s_norm: f32,
    d_signed: f32,
}

#[derive(Clone)]
struct FlowchartBeamState {
    assignments: Vec<(usize, usize)>,
    rects: Vec<Rect>,
    primary: f32,
    drift: f32,
}

fn edge_distance_weight(kind: DiagramKind, overlap_pressure: f32) -> f32 {
    let base = match kind {
        DiagramKind::Flowchart => 0.72,
        DiagramKind::State => 0.38,
        DiagramKind::Class => 0.24,
        _ => 0.16,
    };
    if overlap_pressure <= 0.025 {
        base
    } else if overlap_pressure <= 0.10 {
        match kind {
            DiagramKind::Flowchart => base * 0.92,
            DiagramKind::State => base * 0.80,
            _ => base * 0.55,
        }
    } else {
        match kind {
            DiagramKind::Flowchart => base * 0.68,
            DiagramKind::State => base * 0.55,
            _ => base * 0.2,
        }
    }
}

fn center_label_node_obstacle_pad(
    kind: DiagramKind,
    theme: &Theme,
    label_pad_x: f32,
    label_pad_y: f32,
) -> f32 {
    match kind {
        DiagramKind::Flowchart => (theme.font_size * 0.55)
            .max(label_pad_x.max(label_pad_y + FLOWCHART_LABEL_CLEARANCE_PAD)),
        // State labels often sit in tight transition corridors; large node
        // inflation forces labels unnaturally far from their own edges.
        DiagramKind::State => (theme.font_size * 0.22).max(label_pad_y).max(2.0),
        DiagramKind::Class => (theme.font_size * 0.28).max(label_pad_y).max(2.2),
        _ => (theme.font_size * 0.45).max(label_pad_x.max(label_pad_y)),
    }
}

fn edge_target_distance(kind: DiagramKind, label_h: f32, label_pad_y: f32) -> f32 {
    match kind {
        // For flowcharts we want labels visually attached to the carrying edge.
        // Keep them close, but with enough clearance to avoid path contact.
        DiagramKind::Flowchart => (label_h * 0.52 + label_pad_y * 0.65 + 0.4).max(4.8),
        _ => (label_h * 0.65 + label_pad_y).max(6.0),
    }
}

fn flowchart_own_gap_allowed(gap: f32, max_gap: f32) -> bool {
    gap.is_finite() && (OWN_EDGE_GAP_TARGET_FLOWCHART * 0.5..=max_gap).contains(&gap)
}

fn sweep_bias(kind: DiagramKind, tangent_step: f32, normal_step: f32) -> f32 {
    let (normal_w, tangent_w) = match kind {
        DiagramKind::Flowchart => (0.018, 0.004),
        _ => (0.010, 0.003),
    };
    normal_step.abs() * normal_w + tangent_step.abs() * tangent_w
}

pub(crate) fn edge_label_padding(kind: DiagramKind, config: &LayoutConfig) -> (f32, f32) {
    match kind {
        DiagramKind::Requirement => (
            config.requirement.edge_label_padding_x,
            config.requirement.edge_label_padding_y,
        ),
        DiagramKind::State => (3.0, 1.6),
        DiagramKind::Flowchart => (4.5, 2.2),
        _ => (4.0, 2.0),
    }
}

pub(crate) fn endpoint_label_padding(kind: DiagramKind) -> (f32, f32) {
    match kind {
        DiagramKind::State => (2.6, 1.4),
        DiagramKind::Flowchart => (3.4, 1.8),
        DiagramKind::Class => (3.2, 1.6),
        _ => (3.0, 1.6),
    }
}

/// Resolve all edge label positions using collision avoidance.
///
/// After this function returns, every edge that has a label will have
/// `label_anchor` set to `Some(...)`. Edges with `start_label` or
/// `end_label` will have `start_label_anchor`/`end_label_anchor` set.
pub fn resolve_all_label_positions(
    layout: &mut super::Layout,
    theme: &Theme,
    config: &LayoutConfig,
) {
    if layout.kind == DiagramKind::Sequence || layout.kind == DiagramKind::ZenUML {
        super::sequence::resolve_sequence_label_positions(layout, theme);
        return;
    }

    let bounds = Some((layout.width, layout.height));

    // Step 1: Resolve center labels (label_anchor).
    //
    // Flowchart center labels may need to stack outside the initial routing
    // rectangle, especially for multiple long parallel edge labels.  A graph
    // bounds finalization pass runs after label placement and grows/translates
    // the layout to include labels, so do not clamp flowchart center-label
    // candidates to the pre-label bounds here.  Endpoint labels remain bounded
    // because they are visually tied to node sides and do not benefit from
    // expanding the canvas.
    let center_bounds = if layout.kind == DiagramKind::Flowchart {
        None
    } else {
        bounds
    };
    resolve_center_labels(
        &mut layout.edges,
        &layout.nodes,
        &layout.subgraphs,
        center_bounds,
        layout.kind,
        theme,
        config,
    );
    if layout.kind == DiagramKind::Flowchart {
        move_flowchart_labels_off_own_edges(&mut layout.edges);
    }

    // Step 2: Resolve endpoint labels (start_label_anchor, end_label_anchor).
    resolve_endpoint_labels(
        &mut layout.edges,
        &layout.nodes,
        &layout.subgraphs,
        bounds,
        layout.kind,
        theme,
        config,
    );
}

fn move_flowchart_labels_off_own_edges(edges: &mut [EdgeLayout]) {
    for edge in edges {
        let (Some(label), Some(anchor)) = (&edge.label, edge.label_anchor) else {
            continue;
        };
        let rect = (
            anchor.0 - label.width / 2.0,
            anchor.1 - label.height / 2.0,
            label.width,
            label.height,
        );
        if polyline_rect_distance(&edge.points, &rect) > 0.0 {
            continue;
        }
        let dy = label.height + 24.0;
        let dx = label.width * 0.5 + 24.0;
        for candidate in [
            (anchor.0, anchor.1 - dy),
            (anchor.0, anchor.1 + dy),
            (anchor.0 - dx, anchor.1),
            (anchor.0 + dx, anchor.1),
            (anchor.0 - dx, anchor.1 - dy),
            (anchor.0 + dx, anchor.1 - dy),
            (anchor.0 - dx, anchor.1 + dy),
            (anchor.0 + dx, anchor.1 + dy),
        ] {
            let candidate_rect = (
                candidate.0 - label.width / 2.0,
                candidate.1 - label.height / 2.0,
                label.width,
                label.height,
            );
            if polyline_rect_distance(&edge.points, &candidate_rect) > 0.0 {
                edge.label_anchor = Some(candidate);
                break;
            }
        }
    }
}

/// Resolve center label positions for all edges, writing into `edge.label_anchor`.
fn resolve_center_labels(
    edges: &mut [EdgeLayout],
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
    bounds: Option<(f32, f32)>,
    kind: DiagramKind,
    theme: &Theme,
    config: &LayoutConfig,
) {
    let (label_pad_x, label_pad_y) = edge_label_padding(kind, config);
    let node_obstacle_pad = center_label_node_obstacle_pad(kind, theme, label_pad_x, label_pad_y);
    let edge_obstacle_pad = (theme.font_size * 0.35).max(label_pad_y);
    let step_normal_pad = (theme.font_size * 0.22).max(label_pad_y);
    let step_tangent_pad = (theme.font_size * 0.28).max(label_pad_x);
    let subgraph_label_pad = (theme.font_size * 0.35).max(3.0);

    let mut occupied: Vec<Rect> = build_label_obstacles(
        nodes,
        subgraphs,
        kind,
        theme,
        node_obstacle_pad,
        subgraph_label_pad,
    );
    if kind == DiagramKind::Flowchart {
        occupied.extend(build_node_text_obstacles(
            nodes,
            (theme.font_size * 0.2).max(2.0),
        ));
    }
    let node_obstacle_count = occupied.len();
    let edge_obstacles = build_edge_obstacles(edges, edge_obstacle_pad);
    let edge_obs_rects: Vec<Rect> = edge_obstacles.iter().map(|(_, r)| *r).collect();
    let edge_grid = ObstacleGrid::new(48.0, &edge_obs_rects);
    let mut occupied_grid = ObstacleGrid::new(48.0, &occupied);
    let bundle_fractions = if kind == DiagramKind::Flowchart {
        edge_label_bundle_fractions(edges)
    } else {
        vec![None; edges.len()]
    };
    let mut fixed_center_indices: HashSet<usize> = HashSet::new();
    for (idx, edge) in edges.iter_mut().enumerate() {
        let (Some(label), Some(anchor)) = (&edge.label, edge.label_anchor) else {
            continue;
        };
        let clamped = if let Some(bound) = bounds {
            clamp_label_center_to_bounds(
                anchor,
                label.width,
                label.height,
                label_pad_x,
                label_pad_y,
                bound,
            )
        } else {
            anchor
        };
        let rect = (
            clamped.0 - label.width / 2.0 - label_pad_x,
            clamped.1 - label.height / 2.0 - label_pad_y,
            label.width + 2.0 * label_pad_x,
            label.height + 2.0 * label_pad_y,
        );
        let occupied_rect = if kind == DiagramKind::Flowchart {
            inflate_rect(rect, FLOWCHART_LABEL_CLEARANCE_PAD)
        } else {
            rect
        };
        edge.label_anchor = Some(clamped);
        if kind == DiagramKind::Flowchart {
            // Flowchart routing can seed labels with a reserved/preferred center so
            // the path router avoids the label area. That seed is a good starting
            // point, but it must not become an immovable constraint. Dense
            // bidirectional graphs can produce overlapping reserved centers, and
            // the flowchart-specific de-overlap pass below needs freedom to move
            // them to nearby clear positions.
            continue;
        }
        occupied_grid.insert(occupied.len(), &occupied_rect);
        occupied.push(occupied_rect);
        fixed_center_indices.insert(idx);
    }

    // Sort movable edges by constraint level: larger labels and shorter edges
    // get first pick of placement spots.
    let mut order: Vec<usize> = (0..edges.len())
        .filter(|&i| edges[i].label.is_some() && !fixed_center_indices.contains(&i))
        .collect();
    order.sort_by(|&a, &b| {
        let a_fixed = edges[a].label_anchor.is_some();
        let b_fixed = edges[b].label_anchor.is_some();
        // Pre-set anchors go first (they get first pick near their preferred spot).
        match (a_fixed, b_fixed) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }
        // Larger labels are harder to place; give them first pick.
        let area_a = edges[a]
            .label
            .as_ref()
            .map(|label| label.width * label.height)
            .unwrap_or(0.0);
        let area_b = edges[b]
            .label
            .as_ref()
            .map(|label| label.width * label.height)
            .unwrap_or(0.0);
        if (area_a - area_b).abs() > 1e-3 {
            return area_b
                .partial_cmp(&area_a)
                .unwrap_or(std::cmp::Ordering::Equal);
        }
        // Then by edge path length ascending (shorter = more constrained).
        let len_a = edge_path_length(&edges[a]);
        let len_b = edge_path_length(&edges[b]);
        len_a
            .partial_cmp(&len_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for idx in order {
        let label = match edges[idx].label.clone() {
            Some(l) => l,
            None => continue,
        };
        let pad_w = label.width + 2.0 * label_pad_x;
        let pad_h = label.height + 2.0 * label_pad_y;

        let edge = &edges[idx];
        let mut anchors: Vec<(f32, f32, f32, f32)> = Vec::new();

        if let Some((ax, ay)) = edges[idx].label_anchor
            && let Some(candidate) = edge_label_anchor_from_point(edge, (ax, ay))
        {
            push_anchor_unique(&mut anchors, candidate);
        }
        if let Some(bundle_fraction) = bundle_fractions.get(idx).and_then(|fraction| *fraction) {
            let side_bias = [0.0, -0.08, 0.08];
            for delta in side_bias {
                let frac = (bundle_fraction + delta).clamp(0.05, 0.95);
                if let Some(candidate) = edge_label_anchor_at_fraction(edge, frac) {
                    push_anchor_unique(&mut anchors, candidate);
                }
            }
        }
        for frac in LABEL_ANCHOR_FRACTIONS {
            if let Some(candidate) = edge_label_anchor_at_fraction(edge, frac) {
                push_anchor_unique(&mut anchors, candidate);
            }
        }
        for candidate in edge_segment_anchors(edge, LABEL_EXTRA_SEGMENT_ANCHORS) {
            push_anchor_unique(&mut anchors, candidate);
        }
        if kind == DiagramKind::Flowchart {
            for candidate in edge_terminal_segment_anchors(edge, 2) {
                push_anchor_unique(&mut anchors, candidate);
            }
        }
        if anchors.is_empty() {
            anchors.push(edge_label_anchor(edge));
        } else {
            push_anchor_unique(&mut anchors, edge_label_anchor(edge));
        }
        let (normal_steps, tangent_steps): (&[f32], &[f32]) = if kind == DiagramKind::Flowchart {
            // For flowcharts, prioritize candidate bands that keep labels clear of
            // their own edge while spreading along the edge before collapsing to
            // touching placements.
            (
                &[
                    0.6, -0.6, 1.0, -1.0, 1.4, -1.4, 0.35, -0.35, 2.0, -2.0, 2.8, -2.8, 0.0,
                ],
                &[0.0, 0.3, -0.3, 0.8, -0.8, 1.4, -1.4, 2.2, -2.2, 3.2, -3.2],
            )
        } else if kind == DiagramKind::State {
            (
                &[
                    0.0, 0.12, -0.12, 0.22, -0.22, 0.35, -0.35, 0.45, -0.45, 0.5, -0.5, 0.55,
                    -0.55, 0.6, -0.6, 1.0, -1.0, 2.0, -2.0, 3.0, -3.0,
                ],
                &[
                    0.0, 0.12, -0.12, 0.2, -0.2, 0.35, -0.35, 0.6, -0.6, 1.2, -1.2, 2.0, -2.0, 3.0,
                    -3.0,
                ],
            )
        } else {
            (
                &[
                    0.0, 0.15, -0.15, 0.35, -0.35, 0.6, -0.6, 1.0, -1.0, 2.0, -2.0, 3.0, -3.0,
                ],
                &[0.0, 0.2, -0.2, 0.6, -0.6, 1.2, -1.2, 2.0, -2.0, 3.0, -3.0],
            )
        };
        let mut best_pos = (anchors[0].0, anchors[0].1);
        let mut best_penalty = (f32::INFINITY, f32::INFINITY);
        let center_max_gap = center_label_hard_max_gap(kind);
        let penalty_ctx = LabelPenaltyContext {
            kind,
            occupied: &occupied,
            occupied_grid: &occupied_grid,
            node_obstacle_count,
            edge_obstacles: &edge_obstacles,
            edge_grid: &edge_grid,
            edge_idx: idx,
            own_edge_points: &edge.points,
            bounds,
        };
        let evaluate_candidates = |anchor: (f32, f32, f32, f32),
                                   tangents: &[f32],
                                   normals: &[f32],
                                   max_own_gap: Option<f32>,
                                   best_penalty: &mut (f32, f32),
                                   best_pos: &mut (f32, f32)|
         -> bool {
            let mut evaluated = false;
            let (anchor_x, anchor_y, dir_x, dir_y) = anchor;
            let normal_x = -dir_y;
            let normal_y = dir_x;
            let step_n = if normal_x.abs() > normal_y.abs() {
                label.width + label_pad_x + step_normal_pad
            } else {
                label.height + label_pad_y + step_normal_pad
            };
            let step_t = if dir_x.abs() > dir_y.abs() {
                label.width + label_pad_x + step_tangent_pad
            } else {
                label.height + label_pad_y + step_tangent_pad
            };
            let half_w = label.width * 0.5 + label_pad_x;
            let half_h = label.height * 0.5 + label_pad_y;
            let normal_extent = normal_x.abs() * half_w + normal_y.abs() * half_h;
            let mut score_center = |x: f32, y: f32, tangent_metric: f32, normal_metric: f32| {
                let center = if let Some(bound) = bounds {
                    clamp_label_center_to_bounds(
                        (x, y),
                        label.width,
                        label.height,
                        label_pad_x,
                        label_pad_y,
                        bound,
                    )
                } else {
                    (x, y)
                };
                let rect = (
                    center.0 - label.width / 2.0 - label_pad_x,
                    center.1 - label.height / 2.0 - label_pad_y,
                    pad_w,
                    pad_h,
                );
                if let Some(max_gap) = max_own_gap {
                    let own_gap = polyline_rect_distance(&edge.points, &rect);
                    if kind == DiagramKind::Flowchart {
                        if !flowchart_own_gap_allowed(own_gap, max_gap) {
                            return;
                        }
                    } else if own_gap.is_finite() && own_gap > max_gap {
                        return;
                    }
                }
                evaluated = true;
                let penalty = label_penalties(
                    rect,
                    (anchor_x, anchor_y),
                    label.width,
                    label.height,
                    &penalty_ctx,
                );
                let overlap_pressure = penalty.0;
                let edge_dist = point_polyline_distance(center, &edge.points);
                let edge_target = edge_target_distance(kind, label.height, label_pad_y);
                let edge_dist_weight = edge_distance_weight(kind, overlap_pressure);
                let edge_dist_penalty =
                    ((edge_dist - edge_target).max(0.0) / edge_target) * edge_dist_weight;
                let sweep_penalty = sweep_bias(kind, tangent_metric, normal_metric);
                let penalty = (penalty.0 + edge_dist_penalty + sweep_penalty, penalty.1);
                if candidate_better(penalty, *best_penalty) {
                    *best_penalty = penalty;
                    *best_pos = center;
                }
            };

            // Geometry-aware bands: place labels by explicit edge-clearance gaps,
            // independent of label dimensions, so large labels can still hug paths.
            let (gap_targets, tangent_focus): (&[f32], &[f32]) = match kind {
                DiagramKind::Flowchart => (
                    &[0.8, 1.4, 2.0, 3.0, 4.4, 6.2],
                    &[0.0, 0.3, -0.3, 0.8, -0.8, 1.4, -1.4],
                ),
                DiagramKind::State => (
                    &[0.8, 1.3, 1.9, 2.8, 3.9, 5.3],
                    &[0.0, 0.12, -0.12, 0.35, -0.35, 0.8, -0.8],
                ),
                _ => (
                    &[0.8, 1.4, 2.1, 3.0, 4.1, 5.6],
                    &[0.0, 0.2, -0.2, 0.6, -0.6, 1.2, -1.2],
                ),
            };
            for t in tangent_focus {
                let base_x = anchor_x + dir_x * step_t * *t;
                let base_y = anchor_y + dir_y * step_t * *t;
                for gap in gap_targets {
                    let offset = normal_extent + *gap;
                    let approx_normal = offset / step_n.max(1.0);
                    score_center(
                        base_x + normal_x * offset,
                        base_y + normal_y * offset,
                        *t,
                        approx_normal,
                    );
                    score_center(
                        base_x - normal_x * offset,
                        base_y - normal_y * offset,
                        *t,
                        -approx_normal,
                    );
                }
            }

            for t in tangents {
                let base_x = anchor_x + dir_x * step_t * *t;
                let base_y = anchor_y + dir_y * step_t * *t;
                for n in normals {
                    score_center(
                        base_x + normal_x * step_n * *n,
                        base_y + normal_y * step_n * *n,
                        *t,
                        *n,
                    );
                }
            }
            evaluated
        };
        let mut evaluated = false;
        for anchor in &anchors {
            if evaluate_candidates(
                *anchor,
                tangent_steps,
                normal_steps,
                center_max_gap,
                &mut best_penalty,
                &mut best_pos,
            ) {
                evaluated = true;
            }
        }
        if !evaluated && center_max_gap.is_some() {
            for anchor in &anchors {
                evaluate_candidates(
                    *anchor,
                    tangent_steps,
                    normal_steps,
                    None,
                    &mut best_penalty,
                    &mut best_pos,
                );
            }
        }
        if best_penalty.0 > LABEL_OVERLAP_WIDE_THRESHOLD {
            let (normal_steps_wide, tangent_steps_wide): (&[f32], &[f32]) =
                if kind == DiagramKind::Flowchart {
                    (
                        &[
                            0.6, -0.6, 1.2, -1.2, 2.0, -2.0, 3.0, -3.0, 4.0, -4.0, 5.2, -5.2, 6.5,
                            -6.5, 0.0,
                        ],
                        &[
                            0.0, 0.8, -0.8, 1.6, -1.6, 2.6, -2.6, 3.8, -3.8, 5.2, -5.2, 6.6, -6.6,
                            8.0, -8.0, 10.0, -10.0,
                        ],
                    )
                } else if kind == DiagramKind::State {
                    (
                        &[
                            0.0, 0.45, -0.45, 0.5, -0.5, 0.55, -0.55, 1.0, -1.0, 2.0, -2.0, 3.0,
                            -3.0, 4.0, -4.0, 5.0, -5.0,
                        ],
                        &[
                            0.0, 0.12, -0.12, 0.35, -0.35, 0.8, -0.8, 1.6, -1.6, 2.4, -2.4, 3.2,
                            -3.2, 4.2, -4.2, 5.4, -5.4,
                        ],
                    )
                } else {
                    (
                        &[0.0, 1.0, -1.0, 2.0, -2.0, 3.0, -3.0, 4.0, -4.0, 5.0, -5.0],
                        &[
                            0.0, 0.8, -0.8, 1.6, -1.6, 2.4, -2.4, 3.2, -3.2, 4.2, -4.2, 5.4, -5.4,
                        ],
                    )
                };
            let mut evaluated_wide = false;
            for anchor in &anchors {
                if evaluate_candidates(
                    *anchor,
                    tangent_steps_wide,
                    normal_steps_wide,
                    center_max_gap,
                    &mut best_penalty,
                    &mut best_pos,
                ) {
                    evaluated_wide = true;
                }
            }
            if !evaluated_wide && center_max_gap.is_some() {
                for anchor in &anchors {
                    evaluate_candidates(
                        *anchor,
                        tangent_steps_wide,
                        normal_steps_wide,
                        None,
                        &mut best_penalty,
                        &mut best_pos,
                    );
                }
            }
        }
        let clamped_pos = if let Some(bound) = bounds {
            clamp_label_center_to_bounds(
                best_pos,
                label.width,
                label.height,
                label_pad_x,
                label_pad_y,
                bound,
            )
        } else {
            best_pos
        };
        let rect = (
            clamped_pos.0 - label.width / 2.0 - label_pad_x,
            clamped_pos.1 - label.height / 2.0 - label_pad_y,
            pad_w,
            pad_h,
        );
        let occupied_rect = if kind == DiagramKind::Flowchart {
            inflate_rect(rect, FLOWCHART_LABEL_CLEARANCE_PAD)
        } else {
            rect
        };
        occupied_grid.insert(occupied.len(), &occupied_rect);
        occupied.push(occupied_rect);
        edges[idx].label_anchor = Some(clamped_pos);
    }

    if kind == DiagramKind::Flowchart {
        deoverlap_flowchart_center_labels(
            edges,
            nodes,
            subgraphs,
            bounds,
            theme,
            label_pad_x,
            label_pad_y,
            &fixed_center_indices,
        );
    }

    tighten_center_label_gaps(
        edges,
        nodes,
        subgraphs,
        bounds,
        kind,
        theme,
        label_pad_x,
        label_pad_y,
        &fixed_center_indices,
    );
    enforce_center_label_attachment_caps(
        edges,
        nodes,
        subgraphs,
        bounds,
        kind,
        theme,
        label_pad_x,
        label_pad_y,
        &fixed_center_indices,
    );
    if kind == DiagramKind::Flowchart {
        nudge_flowchart_labels_clear_of_own_paths(edges, bounds);
        let before_score = center_label_overlap_score(edges, label_pad_x, label_pad_y);
        let mut candidate_edges = edges.to_vec();
        deoverlap_flowchart_center_labels(
            &mut candidate_edges,
            nodes,
            subgraphs,
            bounds,
            theme,
            label_pad_x,
            label_pad_y,
            &fixed_center_indices,
        );
        let after_score = center_label_overlap_score(&candidate_edges, label_pad_x, label_pad_y);
        if after_score.improved_by(before_score) {
            edges.clone_from_slice(&candidate_edges);
        }
        // The final de-overlap candidate can trade a little own-edge clearance
        // for lower inter-label overlap. Finish with the dedicated route-clearance
        // nudge so labels do not end up sitting on top of their carrying paths.
        nudge_flowchart_labels_clear_of_own_paths(edges, bounds);
    }
}

fn center_label_overlap_score(
    edges: &[EdgeLayout],
    label_pad_x: f32,
    label_pad_y: f32,
) -> CenterLabelOverlapScore {
    let rects: Vec<Rect> = edges
        .iter()
        .filter_map(|edge| {
            let label = edge.label.as_ref()?;
            let center = edge.label_anchor?;
            Some(flowchart_center_label_rect(
                center,
                label.width,
                label.height,
                label_pad_x,
                label_pad_y,
            ))
        })
        .collect();

    let mut count = 0usize;
    let mut total = 0.0f32;
    let mut max = 0.0f32;
    for i in 0..rects.len() {
        for j in (i + 1)..rects.len() {
            let overlap = overlap_area(&rects[i], &rects[j]);
            if overlap > LABEL_OVERLAP_WIDE_THRESHOLD {
                count += 1;
                total += overlap;
                max = max.max(overlap);
            }
        }
    }

    CenterLabelOverlapScore { count, total, max }
}

fn label_core_rect(center: (f32, f32), label: &TextBlock) -> Rect {
    (
        center.0 - label.width / 2.0,
        center.1 - label.height / 2.0,
        label.width,
        label.height,
    )
}

fn path_intersects_rect(points: &[(f32, f32)], rect: &Rect) -> bool {
    points
        .windows(2)
        .any(|segment| segment_intersects_rect(segment[0], segment[1], rect))
}

fn nudge_flowchart_labels_clear_of_own_paths(edges: &mut [EdgeLayout], bounds: Option<(f32, f32)>) {
    let mut label_rects: Vec<Option<Rect>> = edges
        .iter()
        .map(|edge| {
            let label = edge.label.as_ref()?;
            let center = edge.label_anchor?;
            Some(label_core_rect(center, label))
        })
        .collect();

    for idx in 0..edges.len() {
        let Some(label) = edges[idx].label.as_ref() else {
            continue;
        };
        let Some(center) = edges[idx].label_anchor else {
            continue;
        };
        let current_rect = label_core_rect(center, label);
        if !path_intersects_rect(&edges[idx].points, &current_rect) {
            continue;
        }

        let mut best_center = None;
        let mut best_cost = f32::INFINITY;
        let directions = [
            (0.0, -1.0),
            (0.0, 1.0),
            (-1.0, 0.0),
            (1.0, 0.0),
            (-0.707, -0.707),
            (0.707, -0.707),
            (-0.707, 0.707),
            (0.707, 0.707),
        ];
        // A label only needs to escape its own path by roughly half of its
        // larger dimension plus the path's local extent. Small labels clear
        // within the original 32px ceiling, but a large multi-line label on a
        // short edge (the box can be taller than the edge's cross-axis span)
        // needs a much larger displacement. Extend the step ladder to scale
        // with the label so big labels can clear instead of being left
        // overlapping their route. This pass runs last and only touches labels
        // that already intersect their own path, so it cannot move a
        // previously-clear label.
        let reach = (label.width.max(label.height) + 48.0).max(64.0);
        let mut steps = vec![2.0, 4.0, 6.0, 8.0, 12.0, 16.0, 24.0, 32.0];
        let mut extra = 40.0;
        while extra <= reach {
            steps.push(extra);
            extra += 8.0;
        }
        for step in steps {
            for (dx, dy) in directions {
                let mut candidate = (center.0 + dx * step, center.1 + dy * step);
                if let Some(bound) = bounds {
                    candidate = clamp_label_center_to_bounds(
                        candidate,
                        label.width,
                        label.height,
                        0.0,
                        0.0,
                        bound,
                    );
                }
                let rect = label_core_rect(candidate, label);
                if path_intersects_rect(&edges[idx].points, &rect) {
                    continue;
                }
                let mut overlap = 0.0f32;
                for (other_idx, other) in label_rects.iter().enumerate() {
                    if other_idx == idx {
                        continue;
                    }
                    if let Some(other) = other {
                        overlap += overlap_area(&rect, other);
                    }
                }
                let outside = bounds
                    .map(|bound| outside_area(&rect, bound))
                    .unwrap_or(0.0);
                let cost = step + overlap * 10.0 + outside * 20.0;
                if cost < best_cost {
                    best_cost = cost;
                    best_center = Some(candidate);
                }
            }
            if best_center.is_some() {
                break;
            }
        }

        if let Some(center) = best_center {
            edges[idx].label_anchor = Some(center);
            label_rects[idx] = Some(label_core_rect(center, label));
        }
    }
}

fn deoverlap_flowchart_center_labels(
    edges: &mut [EdgeLayout],
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
    bounds: Option<(f32, f32)>,
    theme: &Theme,
    label_pad_x: f32,
    label_pad_y: f32,
    locked_indices: &HashSet<usize>,
) {
    let step_normal_pad = (theme.font_size * 0.25).max(label_pad_y);
    let step_tangent_pad = (theme.font_size * 0.35).max(label_pad_x);
    let mut entries: Vec<FlowchartCenterLabelEntry> = Vec::new();
    for (idx, edge) in edges.iter().enumerate() {
        if locked_indices.contains(&idx) {
            continue;
        }
        let (Some(label), Some(anchor)) = (&edge.label, edge.label_anchor) else {
            continue;
        };
        let current_center = if let Some(bound) = bounds {
            clamp_label_center_to_bounds(
                anchor,
                label.width,
                label.height,
                label_pad_x,
                label_pad_y,
                bound,
            )
        } else {
            anchor
        };
        let initial_center = edge_label_anchor_from_point(edge, current_center)
            .map(|(x, y, _, _)| (x, y))
            .unwrap_or_else(|| {
                let (x, y, _, _) = edge_label_anchor(edge);
                (x, y)
            });
        let (initial_s_norm, initial_d_signed) =
            edge_relative_pose(&edge.points, initial_center).unwrap_or((0.5, 0.0));
        let candidates = flowchart_center_label_candidates(
            edge,
            current_center,
            label.width,
            label.height,
            label_pad_x,
            label_pad_y,
            step_normal_pad,
            step_tangent_pad,
            bounds,
        );
        entries.push(FlowchartCenterLabelEntry {
            edge_idx: idx,
            label_w: label.width,
            label_h: label.height,
            initial_center,
            initial_s_norm,
            initial_d_signed,
            current_center,
            edge_points: edge.points.clone(),
            candidates,
        });
    }
    if entries.len() < 2 {
        return;
    }

    let node_obstacle_pad =
        (theme.font_size * 0.55).max(label_pad_x.max(label_pad_y + FLOWCHART_LABEL_CLEARANCE_PAD));
    let subgraph_label_pad = (theme.font_size * 0.35).max(3.0);
    let mut fixed_obstacles = build_label_obstacles(
        nodes,
        subgraphs,
        DiagramKind::Flowchart,
        theme,
        node_obstacle_pad,
        subgraph_label_pad,
    );
    fixed_obstacles.extend(build_node_text_obstacles(
        nodes,
        (theme.font_size * 0.2).max(2.0),
    ));
    for (idx, edge) in edges.iter().enumerate() {
        if !locked_indices.contains(&idx) {
            continue;
        }
        let (Some(label), Some(center)) = (&edge.label, edge.label_anchor) else {
            continue;
        };
        let rect = (
            center.0 - label.width / 2.0 - label_pad_x,
            center.1 - label.height / 2.0 - label_pad_y,
            label.width + 2.0 * label_pad_x,
            label.height + 2.0 * label_pad_y,
        );
        fixed_obstacles.push(inflate_rect(rect, FLOWCHART_LABEL_CLEARANCE_PAD));
    }
    let edge_obstacle_pad = (theme.font_size * 0.35).max(label_pad_y);
    let edge_obstacles = build_edge_obstacles(edges, edge_obstacle_pad);
    let edge_obs_rects: Vec<Rect> = edge_obstacles.iter().map(|(_, r)| *r).collect();
    let edge_grid = ObstacleGrid::new(48.0, &edge_obs_rects);

    // Deterministic global assignment pass: solve per conflict component so
    // labels stay close to owning paths while honoring non-overlap constraints.
    apply_flowchart_component_assignment(
        &mut entries,
        label_pad_x,
        label_pad_y,
        &fixed_obstacles,
        &edge_obstacles,
        &edge_grid,
    );

    // Iterative global refinement: resolve the most conflicted labels first and
    // re-score against all other current placements.
    for _ in 0..10 {
        let current_rects: Vec<Rect> = entries
            .iter()
            .map(|entry| {
                flowchart_center_label_obstacle_rect(
                    entry.current_center,
                    entry.label_w,
                    entry.label_h,
                    label_pad_x,
                    label_pad_y,
                )
            })
            .collect();
        let mut conflict_order: Vec<(f32, usize)> = Vec::new();
        for (i, rect) in current_rects.iter().enumerate() {
            let mut conflict_score = 0.0;
            for (j, other) in current_rects.iter().enumerate() {
                if i == j {
                    continue;
                }
                let ov = overlap_area(rect, other);
                if ov > 0.0 {
                    conflict_score += ov + 1.0;
                }
            }
            for obstacle in &fixed_obstacles {
                let ov = overlap_area(rect, obstacle);
                if ov > 0.0 {
                    conflict_score += ov * 1.6 + 1.0;
                }
            }
            if conflict_score > 0.0 {
                conflict_order.push((conflict_score, i));
            }
        }
        if conflict_order.is_empty() {
            break;
        }
        conflict_order.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut moved = false;
        for (_, entry_idx) in conflict_order {
            let entry_snapshot = entries[entry_idx].clone();
            let others: Vec<Rect> = entries
                .iter()
                .enumerate()
                .filter_map(|(i, entry)| {
                    if i == entry_idx {
                        None
                    } else {
                        Some(flowchart_center_label_obstacle_rect(
                            entry.current_center,
                            entry.label_w,
                            entry.label_h,
                            label_pad_x,
                            label_pad_y,
                        ))
                    }
                })
                .collect();
            let mut best_center = entry_snapshot.current_center;
            let mut best_cost = flowchart_center_label_refine_cost(
                &entry_snapshot,
                entry_snapshot.current_center,
                label_pad_x,
                label_pad_y,
                &others,
                &fixed_obstacles,
                &edge_obstacles,
                &edge_grid,
            );
            let mut evaluate = |enforce_gap_limit: bool, enforce_center_band: bool| -> bool {
                let mut considered = false;
                for candidate in entry_snapshot.candidates.iter().copied() {
                    if (candidate.0 - entry_snapshot.current_center.0).abs() <= 0.2
                        && (candidate.1 - entry_snapshot.current_center.1).abs() <= 0.2
                    {
                        continue;
                    }
                    if enforce_center_band {
                        let center_dist =
                            point_polyline_distance(candidate, &entry_snapshot.edge_points);
                        let center_hard_max = flowchart_pose_center_hard_max(
                            &entry_snapshot.edge_points,
                            candidate,
                            entry_snapshot.label_w,
                            entry_snapshot.label_h,
                            label_pad_x,
                            label_pad_y,
                        );
                        if center_dist.is_finite() && center_dist > center_hard_max {
                            continue;
                        }
                    }
                    if enforce_gap_limit {
                        let rect = flowchart_center_label_rect(
                            candidate,
                            entry_snapshot.label_w,
                            entry_snapshot.label_h,
                            label_pad_x,
                            label_pad_y,
                        );
                        let own_gap = polyline_rect_distance(&entry_snapshot.edge_points, &rect);
                        if !flowchart_own_gap_allowed(own_gap, FLOWCHART_OWN_EDGE_HARD_MAX_GAP) {
                            continue;
                        }
                    }
                    considered = true;
                    let cost = flowchart_center_label_refine_cost(
                        &entry_snapshot,
                        candidate,
                        label_pad_x,
                        label_pad_y,
                        &others,
                        &fixed_obstacles,
                        &edge_obstacles,
                        &edge_grid,
                    );
                    if candidate_better(cost, best_cost) {
                        best_cost = cost;
                        best_center = candidate;
                    }
                }
                considered
            };
            let considered_strict = evaluate(true, true);
            if !considered_strict {
                let considered_center_relaxed = evaluate(true, false);
                if !considered_center_relaxed {
                    let _ = evaluate(false, false);
                }
            }
            if (best_center.0 - entries[entry_idx].current_center.0).abs() > 0.2
                || (best_center.1 - entries[entry_idx].current_center.1).abs() > 0.2
            {
                entries[entry_idx].current_center = best_center;
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }

    // If overlaps still remain, force-separate conflicted pairs by selecting a
    // non-overlapping candidate for one side of the pair.
    let residual_overlap = flowchart_entry_label_overlap_score(&entries, label_pad_x, label_pad_y);
    let use_pair_separation_candidates = entries
        .iter()
        .filter(|entry| {
            let (span_x, span_y) = flowchart_path_span(&entry.edge_points);
            span_x >= span_y
        })
        .count()
        * 2
        >= entries.len()
        && residual_overlap.count > 0
        && residual_overlap.total >= 10.0;
    for _ in 0..6 {
        let current_rects: Vec<Rect> = entries
            .iter()
            .map(|entry| {
                flowchart_center_label_obstacle_rect(
                    entry.current_center,
                    entry.label_w,
                    entry.label_h,
                    label_pad_x,
                    label_pad_y,
                )
            })
            .collect();
        let mut adjusted = false;
        'pair_search: for i in 0..entries.len() {
            for j in (i + 1)..entries.len() {
                if overlap_area(&current_rects[i], &current_rects[j])
                    <= LABEL_OVERLAP_WIDE_THRESHOLD
                {
                    continue;
                }
                for &move_idx in &[i, j] {
                    let entry_snapshot = entries[move_idx].clone();
                    let blocking_rect = current_rects[if move_idx == i { j } else { i }];
                    let mut candidate_centers = entry_snapshot.candidates.clone();
                    if use_pair_separation_candidates {
                        for center in flowchart_pair_separation_candidates(
                            &entry_snapshot,
                            blocking_rect,
                            label_pad_x,
                            label_pad_y,
                            bounds,
                        ) {
                            push_center_unique(&mut candidate_centers, center);
                        }
                    }
                    let others: Vec<Rect> = current_rects
                        .iter()
                        .enumerate()
                        .filter_map(|(k, rect)| if k == move_idx { None } else { Some(*rect) })
                        .collect();
                    let mut best_center: Option<(f32, f32)> = None;
                    let mut best_cost = (f32::INFINITY, f32::INFINITY);
                    let rect_area = (entry_snapshot.label_w * entry_snapshot.label_h).max(1.0);
                    let mut evaluate = |enforce_gap_limit: bool,
                                        enforce_no_overlap: bool,
                                        enforce_center_band: bool|
                     -> bool {
                        let mut considered = false;
                        for candidate in candidate_centers.iter().copied() {
                            if enforce_center_band {
                                let center_dist =
                                    point_polyline_distance(candidate, &entry_snapshot.edge_points);
                                let center_hard_max = flowchart_pose_center_hard_max(
                                    &entry_snapshot.edge_points,
                                    candidate,
                                    entry_snapshot.label_w,
                                    entry_snapshot.label_h,
                                    label_pad_x,
                                    label_pad_y,
                                );
                                if center_dist.is_finite() && center_dist > center_hard_max {
                                    continue;
                                }
                            }
                            let rect = flowchart_center_label_rect(
                                candidate,
                                entry_snapshot.label_w,
                                entry_snapshot.label_h,
                                label_pad_x,
                                label_pad_y,
                            );
                            let obstacle_rect = flowchart_center_label_obstacle_rect(
                                candidate,
                                entry_snapshot.label_w,
                                entry_snapshot.label_h,
                                label_pad_x,
                                label_pad_y,
                            );
                            let mut overlap_penalty = 0.0f32;
                            let mut has_overlap = false;
                            for other in &others {
                                let ov = overlap_area(&obstacle_rect, other);
                                if ov > LABEL_OVERLAP_WIDE_THRESHOLD {
                                    has_overlap = true;
                                    overlap_penalty += (ov / rect_area) * 140.0;
                                }
                            }
                            for obstacle in &fixed_obstacles {
                                let ov = overlap_area(&obstacle_rect, obstacle);
                                if ov > LABEL_OVERLAP_WIDE_THRESHOLD {
                                    has_overlap = true;
                                    overlap_penalty += (ov / rect_area) * 180.0;
                                }
                            }
                            if enforce_no_overlap && has_overlap {
                                continue;
                            }
                            if enforce_gap_limit {
                                let own_gap =
                                    polyline_rect_distance(&entry_snapshot.edge_points, &rect);
                                if !flowchart_own_gap_allowed(
                                    own_gap,
                                    FLOWCHART_OWN_EDGE_HARD_MAX_GAP,
                                ) {
                                    continue;
                                }
                            }
                            considered = true;
                            let cost = flowchart_center_label_refine_cost(
                                &entry_snapshot,
                                candidate,
                                label_pad_x,
                                label_pad_y,
                                &others,
                                &fixed_obstacles,
                                &edge_obstacles,
                                &edge_grid,
                            );
                            let cost = (cost.0 + overlap_penalty, cost.1);
                            if best_center.is_none() || candidate_better(cost, best_cost) {
                                best_center = Some(candidate);
                                best_cost = cost;
                            }
                        }
                        considered
                    };
                    let considered_strict = evaluate(true, true, true);
                    if !considered_strict && use_pair_separation_candidates {
                        let considered_no_overlap_center_relaxed = evaluate(true, true, false);
                        if !considered_no_overlap_center_relaxed {
                            let considered_no_overlap_gap_relaxed = evaluate(false, true, false);
                            if !considered_no_overlap_gap_relaxed {
                                let considered_soft = evaluate(true, false, true);
                                if !considered_soft {
                                    let considered_center_relaxed = evaluate(true, false, false);
                                    if !considered_center_relaxed {
                                        let _ = evaluate(false, false, false);
                                    }
                                }
                            }
                        }
                    } else if !considered_strict {
                        let considered_soft = evaluate(true, false, true);
                        if !considered_soft {
                            let considered_center_relaxed = evaluate(true, false, false);
                            if !considered_center_relaxed {
                                let _ = evaluate(false, false, false);
                            }
                        }
                    }
                    if let Some(center) = best_center
                        && ((center.0 - entries[move_idx].current_center.0).abs() > 0.2
                            || (center.1 - entries[move_idx].current_center.1).abs() > 0.2)
                    {
                        entries[move_idx].current_center = center;
                        adjusted = true;
                        break 'pair_search;
                    }
                }
            }
        }
        if !adjusted {
            break;
        }
    }

    for entry in entries {
        edges[entry.edge_idx].label_anchor = Some(entry.current_center);
    }
}

fn center_label_gap_limits(kind: DiagramKind) -> (f32, f32, f32) {
    match kind {
        DiagramKind::Flowchart => (
            OWN_EDGE_GAP_TARGET_FLOWCHART,
            FLOWCHART_OWN_EDGE_SOFT_MAX_GAP,
            FLOWCHART_OWN_EDGE_HARD_MAX_GAP,
        ),
        DiagramKind::State => (1.7, 4.8, STATE_OWN_EDGE_HARD_MAX_GAP),
        DiagramKind::Class => (1.6, 4.6, CLASS_OWN_EDGE_HARD_MAX_GAP),
        _ => (1.5, 5.0, DEFAULT_OWN_EDGE_HARD_MAX_GAP),
    }
}

fn center_label_obstacle_rect(
    kind: DiagramKind,
    center: (f32, f32),
    label_w: f32,
    label_h: f32,
    label_pad_x: f32,
    label_pad_y: f32,
) -> Rect {
    let rect = (
        center.0 - label_w / 2.0 - label_pad_x,
        center.1 - label_h / 2.0 - label_pad_y,
        label_w + 2.0 * label_pad_x,
        label_h + 2.0 * label_pad_y,
    );
    if kind == DiagramKind::Flowchart {
        inflate_rect(rect, FLOWCHART_LABEL_CLEARANCE_PAD)
    } else {
        rect
    }
}

fn center_label_gap_penalty(
    gap: f32,
    target_gap: f32,
    soft_max_gap: f32,
    hard_max_gap: f32,
) -> f32 {
    if !gap.is_finite() {
        return 120.0;
    }
    let target = target_gap.max(1e-3);
    let mut penalty = 0.0f32;
    if gap <= 0.35 {
        penalty += 28.0;
    }
    let dev = (gap - target) / target;
    penalty += dev * dev * 0.8;
    if gap > soft_max_gap {
        let over = gap - soft_max_gap;
        penalty += over * over * 2.0;
    }
    if gap > hard_max_gap {
        let over = gap - hard_max_gap;
        penalty += over * 10.0;
    }
    penalty
}

fn center_label_tighten_candidates(
    edge: &EdgeLayout,
    current_center: (f32, f32),
    label_w: f32,
    label_h: f32,
    label_pad_x: f32,
    label_pad_y: f32,
    kind: DiagramKind,
    bounds: Option<(f32, f32)>,
) -> Vec<(f32, f32)> {
    let mut candidates = Vec::new();
    let mut push_candidate = |mut center: (f32, f32)| {
        if let Some(bound) = bounds {
            center = clamp_label_center_to_bounds(
                center,
                label_w,
                label_h,
                label_pad_x,
                label_pad_y,
                bound,
            );
        }
        push_center_unique(&mut candidates, center);
    };
    push_candidate(current_center);

    let mut anchors: Vec<(f32, f32, f32, f32)> = Vec::new();
    if let Some(anchor) = edge_label_anchor_from_point(edge, current_center) {
        push_anchor_unique(&mut anchors, anchor);
    }
    for frac in LABEL_ANCHOR_FRACTIONS {
        if let Some(anchor) = edge_label_anchor_at_fraction(edge, frac) {
            push_anchor_unique(&mut anchors, anchor);
        }
    }
    for anchor in edge_segment_anchors(edge, LABEL_EXTRA_SEGMENT_ANCHORS) {
        push_anchor_unique(&mut anchors, anchor);
    }
    if kind == DiagramKind::Flowchart || kind == DiagramKind::State {
        for anchor in edge_terminal_segment_anchors(edge, 2) {
            push_anchor_unique(&mut anchors, anchor);
        }
    }
    if anchors.is_empty() {
        anchors.push(edge_label_anchor(edge));
    } else {
        push_anchor_unique(&mut anchors, edge_label_anchor(edge));
    }

    let (gap_targets, tangent_steps): (&[f32], &[f32]) = match kind {
        DiagramKind::Flowchart => (
            &[0.9, 1.4, 1.9, 2.6, 3.6, 4.8, 6.2],
            &[
                0.0, 0.25, -0.25, 0.7, -0.7, 1.3, -1.3, 2.1, -2.1, 3.2, -3.2, 4.4, -4.4,
            ],
        ),
        DiagramKind::State => (
            &[0.9, 1.4, 1.9, 2.5, 3.4, 4.4, 5.8],
            &[
                0.0, 0.18, -0.18, 0.5, -0.5, 1.0, -1.0, 1.8, -1.8, 2.8, -2.8, 4.0, -4.0, 5.5, -5.5,
            ],
        ),
        _ => (
            &[0.8, 1.3, 1.8, 2.4, 3.2, 4.2, 5.6],
            &[0.0, 0.2, -0.2, 0.6, -0.6, 1.2, -1.2, 2.0, -2.0, 3.0, -3.0],
        ),
    };
    let local_tangent_steps: &[f32] = &[0.0, 0.35, -0.35, 0.8, -0.8];
    let local_normal_steps: &[f32] = &[0.0, 0.2, -0.2, 0.45, -0.45];

    for (anchor_x, anchor_y, dir_x, dir_y) in anchors {
        let normal_x = -dir_y;
        let normal_y = dir_x;
        let step_n = if normal_x.abs() > normal_y.abs() {
            label_w + label_pad_x
        } else {
            label_h + label_pad_y
        };
        let step_t = if dir_x.abs() > dir_y.abs() {
            label_w + label_pad_x
        } else {
            label_h + label_pad_y
        };
        let half_w = label_w * 0.5 + label_pad_x;
        let half_h = label_h * 0.5 + label_pad_y;
        let normal_extent = normal_x.abs() * half_w + normal_y.abs() * half_h;
        for t in tangent_steps {
            let base_x = anchor_x + dir_x * step_t * *t;
            let base_y = anchor_y + dir_y * step_t * *t;
            for gap in gap_targets {
                let offset = normal_extent + *gap;
                push_candidate((base_x + normal_x * offset, base_y + normal_y * offset));
                push_candidate((base_x - normal_x * offset, base_y - normal_y * offset));
            }
        }
        for t in local_tangent_steps {
            let base_x = anchor_x + dir_x * step_t * *t;
            let base_y = anchor_y + dir_y * step_t * *t;
            for n in local_normal_steps {
                push_candidate((
                    base_x + normal_x * step_n * *n,
                    base_y + normal_y * step_n * *n,
                ));
            }
        }
    }

    candidates
}

fn tighten_center_label_gaps(
    edges: &mut [EdgeLayout],
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
    bounds: Option<(f32, f32)>,
    kind: DiagramKind,
    theme: &Theme,
    label_pad_x: f32,
    label_pad_y: f32,
    locked_indices: &HashSet<usize>,
) {
    if edges
        .iter()
        .all(|edge| edge.label.is_none() || edge.label_anchor.is_none())
    {
        return;
    }

    let (target_gap, soft_max_gap, hard_max_gap) = center_label_gap_limits(kind);
    let iterations = match kind {
        DiagramKind::Flowchart => 4,
        DiagramKind::State | DiagramKind::Class => 6,
        _ => 4,
    };
    let node_obstacle_pad = center_label_node_obstacle_pad(kind, theme, label_pad_x, label_pad_y);
    let subgraph_label_pad = (theme.font_size * 0.35).max(3.0);
    let mut static_obstacles = build_label_obstacles(
        nodes,
        subgraphs,
        kind,
        theme,
        node_obstacle_pad,
        subgraph_label_pad,
    );
    if kind == DiagramKind::Flowchart {
        static_obstacles.extend(build_node_text_obstacles(
            nodes,
            (theme.font_size * 0.2).max(2.0),
        ));
    }
    let node_obstacle_count = static_obstacles.len();
    let edge_obstacle_pad = (theme.font_size * 0.35).max(label_pad_y);
    let edge_obstacles = build_edge_obstacles(edges, edge_obstacle_pad);
    let edge_obs_rects: Vec<Rect> = edge_obstacles.iter().map(|(_, r)| *r).collect();
    let edge_grid = ObstacleGrid::new(48.0, &edge_obs_rects);

    for _ in 0..iterations {
        let mut order: Vec<(usize, f32)> = edges
            .iter()
            .enumerate()
            .filter_map(|(idx, edge)| {
                if locked_indices.contains(&idx) {
                    return None;
                }
                let (Some(label), Some(center)) = (&edge.label, edge.label_anchor) else {
                    return None;
                };
                let rect = (
                    center.0 - label.width * 0.5 - label_pad_x,
                    center.1 - label.height * 0.5 - label_pad_y,
                    label.width + 2.0 * label_pad_x,
                    label.height + 2.0 * label_pad_y,
                );
                let gap = polyline_rect_distance(&edge.points, &rect);
                if gap.is_finite() && gap > target_gap + 0.3 {
                    Some((idx, gap))
                } else {
                    None
                }
            })
            .collect();
        if order.is_empty() {
            break;
        }
        order.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut moved = false;
        for (idx, _) in order {
            let (label, current_center, edge_points) = {
                let edge = &edges[idx];
                let (Some(label), Some(center)) = (&edge.label, edge.label_anchor) else {
                    continue;
                };
                (label.clone(), center, edge.points.clone())
            };
            if edge_points.len() < 2 {
                continue;
            }

            let mut occupied = static_obstacles.clone();
            for (other_idx, other) in edges.iter().enumerate() {
                if other_idx == idx {
                    continue;
                }
                let (Some(other_label), Some(other_center)) = (&other.label, other.label_anchor)
                else {
                    continue;
                };
                occupied.push(center_label_obstacle_rect(
                    kind,
                    other_center,
                    other_label.width,
                    other_label.height,
                    label_pad_x,
                    label_pad_y,
                ));
            }
            let occupied_grid = ObstacleGrid::new(48.0, &occupied);
            let current_rect = (
                current_center.0 - label.width * 0.5 - label_pad_x,
                current_center.1 - label.height * 0.5 - label_pad_y,
                label.width + 2.0 * label_pad_x,
                label.height + 2.0 * label_pad_y,
            );
            let current_anchor = edge_label_anchor_from_point(&edges[idx], current_center)
                .unwrap_or_else(|| edge_label_anchor(&edges[idx]));
            let current_gap = polyline_rect_distance(&edge_points, &current_rect);
            let penalty_ctx = LabelPenaltyContext {
                kind,
                occupied: &occupied,
                occupied_grid: &occupied_grid,
                node_obstacle_count,
                edge_obstacles: &edge_obstacles,
                edge_grid: &edge_grid,
                edge_idx: idx,
                own_edge_points: &edge_points,
                bounds,
            };
            let mut current_cost = label_penalties(
                current_rect,
                (current_anchor.0, current_anchor.1),
                label.width,
                label.height,
                &penalty_ctx,
            );
            current_cost.0 +=
                center_label_gap_penalty(current_gap, target_gap, soft_max_gap, hard_max_gap);

            let candidates = center_label_tighten_candidates(
                &edges[idx],
                current_center,
                label.width,
                label.height,
                label_pad_x,
                label_pad_y,
                kind,
                bounds,
            );
            let mut best_center = current_center;
            let mut best_cost = current_cost;
            let mut best_gap = current_gap;
            let evaluate = |center: (f32, f32),
                            allow_above_hard: bool,
                            best_center: &mut (f32, f32),
                            best_cost: &mut (f32, f32),
                            best_gap: &mut f32| {
                if (center.0 - current_center.0).abs() <= 0.2
                    && (center.1 - current_center.1).abs() <= 0.2
                {
                    return;
                }
                let rect = (
                    center.0 - label.width * 0.5 - label_pad_x,
                    center.1 - label.height * 0.5 - label_pad_y,
                    label.width + 2.0 * label_pad_x,
                    label.height + 2.0 * label_pad_y,
                );
                let gap = polyline_rect_distance(&edge_points, &rect);
                if !allow_above_hard && gap.is_finite() && gap > hard_max_gap {
                    return;
                }
                let anchor = edge_label_anchor_from_point(&edges[idx], center)
                    .unwrap_or_else(|| edge_label_anchor(&edges[idx]));
                let mut cost = label_penalties(
                    rect,
                    (anchor.0, anchor.1),
                    label.width,
                    label.height,
                    &penalty_ctx,
                );
                cost.0 += center_label_gap_penalty(gap, target_gap, soft_max_gap, hard_max_gap);
                let dx = center.0 - current_center.0;
                let dy = center.1 - current_center.1;
                cost.1 += (dx * dx + dy * dy).sqrt() / (label.width + label.height + 1.0) * 0.35;
                if candidate_better(cost, *best_cost) {
                    *best_center = center;
                    *best_cost = cost;
                    *best_gap = gap;
                }
            };

            for center in candidates.iter().copied() {
                evaluate(
                    center,
                    false,
                    &mut best_center,
                    &mut best_cost,
                    &mut best_gap,
                );
            }
            if (best_center.0 - current_center.0).abs() <= 0.2
                && (best_center.1 - current_center.1).abs() <= 0.2
            {
                for center in candidates {
                    evaluate(
                        center,
                        true,
                        &mut best_center,
                        &mut best_cost,
                        &mut best_gap,
                    );
                }
            }

            if (best_center.0 - current_center.0).abs() <= 0.2
                && (best_center.1 - current_center.1).abs() <= 0.2
            {
                continue;
            }
            let gap_improved = best_gap + 0.05 < current_gap;
            let needs_tightening = current_gap > soft_max_gap + 0.2;
            let cost_improved = candidate_better(best_cost, current_cost);
            let acceptable = if gap_improved {
                best_cost.0 <= current_cost.0 + 0.35
            } else {
                false
            };
            if (cost_improved && (gap_improved || needs_tightening)) || acceptable {
                edges[idx].label_anchor = Some(best_center);
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
}

fn center_label_attachment_cap(kind: DiagramKind) -> Option<f32> {
    match kind {
        DiagramKind::Flowchart | DiagramKind::Sequence | DiagramKind::ZenUML => None,
        DiagramKind::Er => Some(12.0),
        DiagramKind::Class => Some(10.0),
        DiagramKind::State => Some(11.0),
        _ => Some(12.0),
    }
}

fn enforce_center_label_attachment_caps(
    edges: &mut [EdgeLayout],
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
    bounds: Option<(f32, f32)>,
    kind: DiagramKind,
    theme: &Theme,
    label_pad_x: f32,
    label_pad_y: f32,
    locked_indices: &HashSet<usize>,
) {
    let Some(max_gap) = center_label_attachment_cap(kind) else {
        return;
    };
    if edges
        .iter()
        .all(|edge| edge.label.is_none() || edge.label_anchor.is_none())
    {
        return;
    }

    let node_obstacle_pad = center_label_node_obstacle_pad(kind, theme, label_pad_x, label_pad_y);
    let subgraph_label_pad = (theme.font_size * 0.35).max(3.0);
    let static_obstacles = build_label_obstacles(
        nodes,
        subgraphs,
        kind,
        theme,
        node_obstacle_pad,
        subgraph_label_pad,
    );
    let nudge_weight = if kind == DiagramKind::Er { 0.04 } else { 0.06 };

    for _ in 0..2 {
        let current_label_rects: Vec<Option<Rect>> = edges
            .iter()
            .map(|edge| {
                let (Some(label), Some(center)) = (&edge.label, edge.label_anchor) else {
                    return None;
                };
                Some((
                    center.0 - label.width * 0.5 - label_pad_x,
                    center.1 - label.height * 0.5 - label_pad_y,
                    label.width + 2.0 * label_pad_x,
                    label.height + 2.0 * label_pad_y,
                ))
            })
            .collect();

        for (idx, edge) in edges.iter_mut().enumerate() {
            if locked_indices.contains(&idx) {
                continue;
            }
            let (Some(label), Some(center)) = (&edge.label, edge.label_anchor) else {
                continue;
            };
            let current_rect = (
                center.0 - label.width * 0.5 - label_pad_x,
                center.1 - label.height * 0.5 - label_pad_y,
                label.width + 2.0 * label_pad_x,
                label.height + 2.0 * label_pad_y,
            );
            let current_gap = polyline_rect_distance(&edge.points, &current_rect);
            let current_center_dist = point_polyline_distance(center, &edge.points);
            if !current_gap.is_finite() || !current_center_dist.is_finite() {
                continue;
            }
            if current_gap <= max_gap + 0.05 && current_center_dist <= max_gap + 0.05 {
                continue;
            }
            let Some((anchor_x, anchor_y, dir_x, dir_y)) =
                edge_label_anchor_from_point(edge, center).or(Some(edge_label_anchor(edge)))
            else {
                continue;
            };
            let normal_x = -dir_y;
            let normal_y = dir_x;
            let sign = {
                let rel_x = center.0 - anchor_x;
                let rel_y = center.1 - anchor_y;
                if rel_x * normal_x + rel_y * normal_y >= 0.0 {
                    1.0
                } else {
                    -1.0
                }
            };
            let target_gap =
                edge_target_distance(kind, label.height, label_pad_y).clamp(1.2, max_gap * 0.75);
            let offsets = [
                target_gap,
                (target_gap * 0.72).max(0.9),
                (target_gap * 1.24).min(max_gap * 0.92),
                0.0,
            ];

            let mut candidates: Vec<(f32, f32)> = Vec::new();
            for offset in offsets {
                for side in [sign, -sign] {
                    let mut cand = (
                        anchor_x + normal_x * offset * side,
                        anchor_y + normal_y * offset * side,
                    );
                    if let Some(bound) = bounds {
                        cand = clamp_label_center_to_bounds(
                            cand,
                            label.width,
                            label.height,
                            label_pad_x,
                            label_pad_y,
                            bound,
                        );
                    }
                    push_center_unique(&mut candidates, cand);
                }
            }
            push_center_unique(&mut candidates, center);
            if candidates.is_empty() {
                continue;
            }

            let mut best = center;
            let mut best_score = f32::INFINITY;
            for cand in candidates {
                let rect = (
                    cand.0 - label.width * 0.5 - label_pad_x,
                    cand.1 - label.height * 0.5 - label_pad_y,
                    label.width + 2.0 * label_pad_x,
                    label.height + 2.0 * label_pad_y,
                );
                let gap = polyline_rect_distance(&edge.points, &rect);
                let center_dist = point_polyline_distance(cand, &edge.points);
                if !gap.is_finite() {
                    continue;
                }
                let mut overlap = 0.0f32;
                for obstacle in &static_obstacles {
                    overlap += overlap_area(&rect, obstacle);
                }
                for (other_idx, other_rect_opt) in current_label_rects.iter().enumerate() {
                    if other_idx == idx {
                        continue;
                    }
                    if let Some(other_rect) = other_rect_opt {
                        overlap += overlap_area(&rect, other_rect);
                    }
                }
                if let Some(bound) = bounds {
                    overlap += outside_area(&rect, bound);
                }
                let gap_over = (gap - max_gap).max(0.0);
                let move_dx = cand.0 - center.0;
                let move_dy = cand.1 - center.1;
                let move_dist = (move_dx * move_dx + move_dy * move_dy).sqrt();
                let center_over = (center_dist - max_gap).max(0.0);
                let score = gap_over * 32.0
                    + center_over * 40.0
                    + (gap - target_gap).abs() * 0.9
                    + overlap * 0.06
                    + move_dist * nudge_weight;
                if score < best_score {
                    best_score = score;
                    best = cand;
                }
            }
            edge.label_anchor = Some(best);
        }
    }
}

fn apply_flowchart_component_assignment(
    entries: &mut [FlowchartCenterLabelEntry],
    label_pad_x: f32,
    label_pad_y: f32,
    fixed_obstacles: &[Rect],
    edge_obstacles: &[EdgeObstacle],
    edge_grid: &ObstacleGrid,
) {
    if entries.is_empty() {
        return;
    }
    let candidate_table: Vec<Vec<FlowchartCenterCandidate>> = entries
        .iter()
        .map(|entry| {
            build_flowchart_candidate_set(
                entry,
                label_pad_x,
                label_pad_y,
                fixed_obstacles,
                edge_obstacles,
                edge_grid,
            )
        })
        .collect();
    let components = flowchart_entry_components(entries, label_pad_x, label_pad_y);
    for component in components {
        if component.is_empty() {
            continue;
        }
        // Very large components are better handled by downstream iterative passes.
        if component.len() > 12 {
            continue;
        }
        let assignment = solve_flowchart_component_assignment(
            &component,
            &candidate_table,
            entries,
            false,
            true,
        )
        .or_else(|| {
            solve_flowchart_component_assignment(
                &component,
                &candidate_table,
                entries,
                false,
                false,
            )
        });
        let Some(assignment) = assignment else {
            continue;
        };
        for (entry_idx, center) in assignment {
            entries[entry_idx].current_center = center;
        }
    }
}

fn solve_flowchart_component_assignment(
    component: &[usize],
    candidate_table: &[Vec<FlowchartCenterCandidate>],
    entries: &[FlowchartCenterLabelEntry],
    enforce_no_fixed_overlap: bool,
    enforce_center_band: bool,
) -> Option<Vec<(usize, (f32, f32))>> {
    let mut order: Vec<usize> = component.to_vec();
    order.sort_by(|&a, &b| {
        let a_center = candidate_table[a]
            .iter()
            .filter(|cand| cand.center_dist <= cand.center_hard_max)
            .count();
        let b_center = candidate_table[b]
            .iter()
            .filter(|cand| cand.center_dist <= cand.center_hard_max)
            .count();
        let a_strict = candidate_table[a]
            .iter()
            .filter(|cand| cand.fixed_overlap_count == 0)
            .count();
        let b_strict = candidate_table[b]
            .iter()
            .filter(|cand| cand.fixed_overlap_count == 0)
            .count();
        a_center
            .cmp(&b_center)
            .then_with(|| a_strict.cmp(&b_strict))
            .then_with(|| {
                let a_central_strict = candidate_table[a]
                    .iter()
                    .filter(|cand| {
                        cand.fixed_overlap_count == 0 && cand.center_dist <= cand.center_hard_max
                    })
                    .count();
                let b_central_strict = candidate_table[b]
                    .iter()
                    .filter(|cand| {
                        cand.fixed_overlap_count == 0 && cand.center_dist <= cand.center_hard_max
                    })
                    .count();
                a_central_strict.cmp(&b_central_strict)
            })
            .then_with(|| candidate_table[a].len().cmp(&candidate_table[b].len()))
            .then_with(|| {
                let area_a = entries[a].label_w * entries[a].label_h;
                let area_b = entries[b].label_w * entries[b].label_h;
                area_b
                    .partial_cmp(&area_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| entries[a].edge_idx.cmp(&entries[b].edge_idx))
    });

    let beam_width = match component.len() {
        0 | 1 => 1,
        2 => 36,
        3..=4 => 64,
        5..=6 => 84,
        _ => 108,
    };
    let cand_limit = match component.len() {
        0 | 1 => 48,
        2..=4 => 72,
        5..=7 => 92,
        _ => 112,
    };

    let mut beam: Vec<FlowchartBeamState> = vec![FlowchartBeamState {
        assignments: Vec::new(),
        rects: Vec::new(),
        primary: 0.0,
        drift: 0.0,
    }];

    for &entry_idx in &order {
        let candidates = &candidate_table[entry_idx];
        if candidates.is_empty() {
            return None;
        }
        let mut next: Vec<FlowchartBeamState> = Vec::new();
        for state in &beam {
            for (cand_idx, cand) in candidates.iter().enumerate().take(cand_limit) {
                if enforce_no_fixed_overlap && cand.fixed_overlap_count > 0 {
                    continue;
                }
                if enforce_center_band
                    && cand.center_dist.is_finite()
                    && cand.center_dist > cand.center_hard_max
                {
                    continue;
                }
                if state
                    .rects
                    .iter()
                    .any(|rect| overlap_area(rect, &cand.rect) > LABEL_OVERLAP_WIDE_THRESHOLD)
                {
                    continue;
                }

                let mut primary = state.primary + cand.cost.0;
                let drift = state.drift + cand.cost.1;

                if cand.own_gap.is_finite() {
                    if cand.own_gap > 3.5 {
                        let over = cand.own_gap - 3.5;
                        primary += over * over * 1.2;
                    }
                    if cand.own_gap < 1.1 {
                        let under = 1.1 - cand.own_gap;
                        primary += under * under * 16.0;
                    }
                    if cand.own_gap <= 0.35 {
                        primary += 48.0;
                    }
                }
                if cand.center_dist.is_finite() {
                    let norm = (cand.center_dist / cand.center_target.max(1e-3)).max(0.0);
                    if norm < 0.92 {
                        let shortage = 0.92 - norm;
                        primary += shortage * shortage * 4.5;
                    }
                    if cand.center_dist > cand.center_soft_max {
                        let over = cand.center_dist - cand.center_soft_max;
                        primary += over * over * 2.0;
                    }
                    if cand.center_dist > cand.center_hard_max {
                        let over = cand.center_dist - cand.center_hard_max;
                        primary += over * 14.0;
                    }
                    if norm > 1.0 {
                        let excess = norm - 1.0;
                        primary += excess * excess * 0.75;
                    }
                }
                let s_shift = (cand.s_norm - entries[entry_idx].initial_s_norm).abs();
                let d_shift = (cand.d_signed - entries[entry_idx].initial_d_signed).abs();
                primary += s_shift * 0.26;
                primary += d_shift * 0.02;

                if !enforce_no_fixed_overlap && cand.fixed_overlap_count > 0 {
                    primary += cand.fixed_overlap_count as f32 * 2.0;
                    primary += cand.fixed_overlap_area * 0.005;
                }
                for rect in &state.rects {
                    let gap = rect_gap(rect, &cand.rect);
                    if gap < 4.0 {
                        let shortage = 4.0 - gap;
                        primary += shortage * shortage * 0.08;
                    }
                }

                let mut assignments = state.assignments.clone();
                assignments.push((entry_idx, cand_idx));
                assignments.sort_unstable_by_key(|a| a.0);
                let mut rects = state.rects.clone();
                rects.push(cand.rect);
                next.push(FlowchartBeamState {
                    assignments,
                    rects,
                    primary,
                    drift,
                });
            }
        }
        if next.is_empty() {
            return None;
        }
        next.sort_by(|a, b| {
            a.primary
                .partial_cmp(&b.primary)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    a.drift
                        .partial_cmp(&b.drift)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| a.assignments.cmp(&b.assignments))
        });
        next.truncate(beam_width);
        beam = next;
    }

    let best = beam.into_iter().next()?;
    let resolved: Vec<(usize, (f32, f32))> = best
        .assignments
        .into_iter()
        .map(|(entry_idx, cand_idx)| {
            let center = candidate_table[entry_idx][cand_idx].center;
            (entry_idx, center)
        })
        .collect();
    Some(resolved)
}

fn build_flowchart_candidate_set(
    entry: &FlowchartCenterLabelEntry,
    label_pad_x: f32,
    label_pad_y: f32,
    fixed_obstacles: &[Rect],
    edge_obstacles: &[EdgeObstacle],
    edge_grid: &ObstacleGrid,
) -> Vec<FlowchartCenterCandidate> {
    let mut centers = entry.candidates.clone();
    push_center_unique(&mut centers, entry.current_center);
    push_center_unique(&mut centers, entry.initial_center);
    let (base_center_target, base_center_soft_max, base_center_hard_max) =
        flowchart_center_distance_limits(entry.label_h, label_pad_y);
    let center_soft_delta = (base_center_soft_max - base_center_target).max(0.0);
    let center_hard_delta = (base_center_hard_max - base_center_soft_max).max(0.0);

    let mut scored: Vec<FlowchartCenterCandidate> = Vec::new();
    for center in centers {
        let core_rect = flowchart_center_label_rect(
            center,
            entry.label_w,
            entry.label_h,
            label_pad_x,
            label_pad_y,
        );
        let obstacle_rect = flowchart_center_label_obstacle_rect(
            center,
            entry.label_w,
            entry.label_h,
            label_pad_x,
            label_pad_y,
        );
        let own_gap = polyline_rect_distance(&entry.edge_points, &core_rect);
        if own_gap.is_finite() && own_gap > FLOWCHART_OWN_EDGE_HARD_MAX_GAP + 4.0 {
            continue;
        }
        let center_dist = point_polyline_distance(center, &entry.edge_points);
        let center_target = flowchart_pose_center_target(
            &entry.edge_points,
            center,
            entry.label_w,
            entry.label_h,
            label_pad_x,
            label_pad_y,
        )
        .unwrap_or(base_center_target);
        let center_soft_max = center_target + center_soft_delta;
        let center_hard_max = center_soft_max + center_hard_delta;
        let (fixed_overlap_count, fixed_overlap_area) =
            overlap_stats(obstacle_rect, fixed_obstacles, LABEL_OVERLAP_WIDE_THRESHOLD);
        let mut cost = flowchart_center_label_refine_cost(
            entry,
            center,
            label_pad_x,
            label_pad_y,
            &[],
            fixed_obstacles,
            edge_obstacles,
            edge_grid,
        );
        if own_gap.is_finite() && own_gap > 4.0 {
            let over = own_gap - 4.0;
            cost.0 += over * over * 2.0;
        }
        if center_dist.is_finite() {
            if center_dist > center_soft_max {
                let over = center_dist - center_soft_max;
                cost.0 += over * over * 2.2;
            }
            if center_dist > center_hard_max {
                let over = center_dist - center_hard_max;
                cost.0 += over * 18.0;
            }
        }
        cost.0 += fixed_overlap_count as f32 * 1.5 + fixed_overlap_area * 0.002;
        let (s_norm, d_signed) =
            edge_relative_pose(&entry.edge_points, center).unwrap_or((0.5, 0.0));
        let s_shift = (s_norm - entry.initial_s_norm).abs();
        let d_shift = (d_signed - entry.initial_d_signed).abs();
        cost.0 += s_shift * 0.045 + d_shift * 0.012;
        scored.push(FlowchartCenterCandidate {
            center,
            rect: obstacle_rect,
            cost,
            own_gap,
            center_dist,
            center_target,
            center_soft_max,
            center_hard_max,
            fixed_overlap_count,
            fixed_overlap_area,
            s_norm,
            d_signed,
        });
    }
    scored.sort_by(|a, b| {
        a.cost
            .0
            .partial_cmp(&b.cost.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                let a_gap = (a.own_gap - OWN_EDGE_GAP_TARGET_FLOWCHART).abs();
                let b_gap = (b.own_gap - OWN_EDGE_GAP_TARGET_FLOWCHART).abs();
                a_gap
                    .partial_cmp(&b_gap)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                a.cost
                    .1
                    .partial_cmp(&b.cost.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                a.center
                    .0
                    .partial_cmp(&b.center.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                a.center
                    .1
                    .partial_cmp(&b.center.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    if scored.len() > 220 {
        scored.truncate(220);
    }
    scored
}

fn flowchart_entry_components(
    entries: &[FlowchartCenterLabelEntry],
    label_pad_x: f32,
    label_pad_y: f32,
) -> Vec<Vec<usize>> {
    if entries.is_empty() {
        return Vec::new();
    }
    let rects: Vec<Rect> = entries
        .iter()
        .map(|entry| {
            flowchart_center_label_obstacle_rect(
                entry.current_center,
                entry.label_w,
                entry.label_h,
                label_pad_x,
                label_pad_y,
            )
        })
        .collect();

    let mut neighbors: Vec<Vec<usize>> = vec![Vec::new(); entries.len()];
    for i in 0..entries.len() {
        for j in (i + 1)..entries.len() {
            let overlap = overlap_area(&rects[i], &rects[j]) > LABEL_OVERLAP_WIDE_THRESHOLD;
            let near = rect_gap(&rects[i], &rects[j]) <= 24.0;
            if !overlap && !near {
                continue;
            }
            neighbors[i].push(j);
            neighbors[j].push(i);
        }
    }

    let mut components: Vec<Vec<usize>> = Vec::new();
    let mut seen = vec![false; entries.len()];
    for start in 0..entries.len() {
        if seen[start] {
            continue;
        }
        seen[start] = true;
        let mut stack = vec![start];
        let mut comp = vec![start];
        while let Some(idx) = stack.pop() {
            for &next in &neighbors[idx] {
                if seen[next] {
                    continue;
                }
                seen[next] = true;
                stack.push(next);
                comp.push(next);
            }
        }
        comp.sort_unstable();
        components.push(comp);
    }
    components
}

fn edge_relative_pose(points: &[(f32, f32)], center: (f32, f32)) -> Option<(f32, f32)> {
    if points.len() < 2 {
        return None;
    }
    let mut seg_lengths = Vec::with_capacity(points.len().saturating_sub(1));
    let mut total_len = 0.0f32;
    for seg in points.windows(2) {
        let dx = seg[1].0 - seg[0].0;
        let dy = seg[1].1 - seg[0].1;
        let len = (dx * dx + dy * dy).sqrt();
        seg_lengths.push(len);
        total_len += len;
    }
    if total_len <= 1e-6 {
        return None;
    }

    let mut best_dist2 = f32::INFINITY;
    let mut best_proj = (center.0, center.1);
    let mut best_t = 0.0f32;
    let mut best_prefix = 0.0f32;
    let mut best_seg_len = 1.0f32;
    let mut best_dx = 1.0f32;
    let mut best_dy = 0.0f32;
    let mut prefix = 0.0f32;
    for (seg_idx, seg) in points.windows(2).enumerate() {
        let p1 = seg[0];
        let p2 = seg[1];
        let dx = p2.0 - p1.0;
        let dy = p2.1 - p1.1;
        let seg_len = seg_lengths[seg_idx];
        if seg_len <= 1e-6 {
            continue;
        }
        let seg_len2 = seg_len * seg_len;
        let t = ((center.0 - p1.0) * dx + (center.1 - p1.1) * dy) / seg_len2;
        let t_clamped = t.clamp(0.0, 1.0);
        let proj = (p1.0 + dx * t_clamped, p1.1 + dy * t_clamped);
        let ddx = center.0 - proj.0;
        let ddy = center.1 - proj.1;
        let dist2 = ddx * ddx + ddy * ddy;
        if dist2 < best_dist2 {
            best_dist2 = dist2;
            best_proj = proj;
            best_t = t_clamped;
            best_prefix = prefix;
            best_seg_len = seg_len;
            best_dx = dx / seg_len;
            best_dy = dy / seg_len;
        }
        prefix += seg_len;
    }

    let s = ((best_prefix + best_t * best_seg_len) / total_len).clamp(0.0, 1.0);
    let nx = -best_dy;
    let ny = best_dx;
    let d = (center.0 - best_proj.0) * nx + (center.1 - best_proj.1) * ny;
    Some((s, d))
}

fn flowchart_center_distance_limits(label_h: f32, label_pad_y: f32) -> (f32, f32, f32) {
    let target = edge_target_distance(DiagramKind::Flowchart, label_h, label_pad_y).max(2.0);
    let soft_add = (label_h * 0.18 + label_pad_y * 0.5 + 1.8).clamp(3.0, 6.0);
    let hard_add = (label_h * 0.26 + label_pad_y * 0.8 + 3.2).clamp(5.0, 10.0);
    let soft_max = target + soft_add;
    let hard_max = soft_max + hard_add;
    (target, soft_max, hard_max)
}

fn edge_nearest_segment_tangent(points: &[(f32, f32)], center: (f32, f32)) -> Option<(f32, f32)> {
    if points.len() < 2 {
        return None;
    }
    let mut best_dist2 = f32::INFINITY;
    let mut best_tangent: Option<(f32, f32)> = None;
    for seg in points.windows(2) {
        let p1 = seg[0];
        let p2 = seg[1];
        let dx = p2.0 - p1.0;
        let dy = p2.1 - p1.1;
        let seg_len2 = dx * dx + dy * dy;
        if seg_len2 <= 1e-6 {
            continue;
        }
        let t = ((center.0 - p1.0) * dx + (center.1 - p1.1) * dy) / seg_len2;
        let t_clamped = t.clamp(0.0, 1.0);
        let proj_x = p1.0 + dx * t_clamped;
        let proj_y = p1.1 + dy * t_clamped;
        let dist2 =
            (center.0 - proj_x) * (center.0 - proj_x) + (center.1 - proj_y) * (center.1 - proj_y);
        if dist2 < best_dist2 {
            best_dist2 = dist2;
            let seg_len = seg_len2.sqrt().max(1e-3);
            best_tangent = Some((dx / seg_len, dy / seg_len));
        }
    }
    best_tangent
}

fn flowchart_pose_center_target(
    points: &[(f32, f32)],
    center: (f32, f32),
    label_w: f32,
    label_h: f32,
    label_pad_x: f32,
    label_pad_y: f32,
) -> Option<f32> {
    let (tx, ty) = edge_nearest_segment_tangent(points, center)?;
    let nx = -ty;
    let ny = tx;
    let half_w = label_w * 0.5 + label_pad_x;
    let half_h = label_h * 0.5 + label_pad_y;
    let oriented_extent = nx.abs() * half_w + ny.abs() * half_h;
    let base_extent = half_h;
    let anisotropy_bonus = (oriented_extent - base_extent).max(0.0) * 0.45;
    Some(edge_target_distance(DiagramKind::Flowchart, label_h, label_pad_y) + anisotropy_bonus)
}

fn flowchart_pose_center_hard_max(
    points: &[(f32, f32)],
    center: (f32, f32),
    label_w: f32,
    label_h: f32,
    label_pad_x: f32,
    label_pad_y: f32,
) -> f32 {
    let (base_target, base_soft_max, base_hard_max) =
        flowchart_center_distance_limits(label_h, label_pad_y);
    let soft_delta = (base_soft_max - base_target).max(0.0);
    let hard_delta = (base_hard_max - base_soft_max).max(0.0);
    let target =
        flowchart_pose_center_target(points, center, label_w, label_h, label_pad_x, label_pad_y)
            .unwrap_or(base_target);
    target + soft_delta + hard_delta
}

fn flowchart_center_label_rect(
    center: (f32, f32),
    label_w: f32,
    label_h: f32,
    label_pad_x: f32,
    label_pad_y: f32,
) -> Rect {
    (
        center.0 - label_w / 2.0 - label_pad_x,
        center.1 - label_h / 2.0 - label_pad_y,
        label_w + 2.0 * label_pad_x,
        label_h + 2.0 * label_pad_y,
    )
}

fn flowchart_center_label_obstacle_rect(
    center: (f32, f32),
    label_w: f32,
    label_h: f32,
    label_pad_x: f32,
    label_pad_y: f32,
) -> Rect {
    inflate_rect(
        flowchart_center_label_rect(center, label_w, label_h, label_pad_x, label_pad_y),
        FLOWCHART_LABEL_CLEARANCE_PAD,
    )
}

fn push_center_unique(centers: &mut Vec<(f32, f32)>, candidate: (f32, f32)) {
    let duplicate = centers.iter().any(|center| {
        (center.0 - candidate.0).abs() <= 0.35 && (center.1 - candidate.1).abs() <= 0.35
    });
    if !duplicate {
        centers.push(candidate);
    }
}

fn flowchart_path_span(points: &[(f32, f32)]) -> (f32, f32) {
    if points.is_empty() {
        return (0.0, 0.0);
    }
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for &(x, y) in points {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    ((max_x - min_x).max(0.0), (max_y - min_y).max(0.0))
}

fn flowchart_entry_label_overlap_score(
    entries: &[FlowchartCenterLabelEntry],
    label_pad_x: f32,
    label_pad_y: f32,
) -> CenterLabelOverlapScore {
    let rects: Vec<Rect> = entries
        .iter()
        .map(|entry| {
            flowchart_center_label_rect(
                entry.current_center,
                entry.label_w,
                entry.label_h,
                label_pad_x,
                label_pad_y,
            )
        })
        .collect();
    let mut count = 0;
    let mut total = 0.0f32;
    let mut max = 0.0f32;
    for i in 0..rects.len() {
        for j in (i + 1)..rects.len() {
            let overlap = overlap_area(&rects[i], &rects[j]);
            if overlap > LABEL_OVERLAP_WIDE_THRESHOLD {
                count += 1;
                total += overlap;
                max = max.max(overlap);
            }
        }
    }
    CenterLabelOverlapScore { count, total, max }
}

fn flowchart_pair_separation_candidates(
    entry: &FlowchartCenterLabelEntry,
    blocking_rect: Rect,
    label_pad_x: f32,
    label_pad_y: f32,
    bounds: Option<(f32, f32)>,
) -> Vec<(f32, f32)> {
    let mut candidates = Vec::new();
    let obstacle_half_w = entry.label_w * 0.5 + label_pad_x + FLOWCHART_LABEL_CLEARANCE_PAD;
    let obstacle_half_h = entry.label_h * 0.5 + label_pad_y + FLOWCHART_LABEL_CLEARANCE_PAD;
    let gap = 2.0;
    let left_x = blocking_rect.0 - gap - obstacle_half_w;
    let right_x = blocking_rect.0 + blocking_rect.2 + gap + obstacle_half_w;
    let above_y = blocking_rect.1 - gap - obstacle_half_h;
    let below_y = blocking_rect.1 + blocking_rect.3 + gap + obstacle_half_h;
    let x_refs = [entry.current_center.0, entry.initial_center.0];
    let y_refs = [entry.current_center.1, entry.initial_center.1];

    let mut push_candidate = |mut center: (f32, f32)| {
        if let Some(bound) = bounds {
            center = clamp_label_center_to_bounds(
                center,
                entry.label_w,
                entry.label_h,
                label_pad_x,
                label_pad_y,
                bound,
            );
        }
        push_center_unique(&mut candidates, center);
    };

    for y in y_refs {
        push_candidate((left_x, y));
        push_candidate((right_x, y));
    }
    for x in x_refs {
        push_candidate((x, above_y));
        push_candidate((x, below_y));
    }

    candidates
}

fn flowchart_center_label_candidates(
    edge: &EdgeLayout,
    initial_center: (f32, f32),
    label_w: f32,
    label_h: f32,
    label_pad_x: f32,
    label_pad_y: f32,
    step_normal_pad: f32,
    step_tangent_pad: f32,
    bounds: Option<(f32, f32)>,
) -> Vec<(f32, f32)> {
    let mut candidates = Vec::new();
    let mut push_candidate = |mut center: (f32, f32)| {
        if let Some(bound) = bounds {
            center = clamp_label_center_to_bounds(
                center,
                label_w,
                label_h,
                label_pad_x,
                label_pad_y,
                bound,
            );
        }
        push_center_unique(&mut candidates, center);
    };
    push_candidate(initial_center);

    let mut anchors: Vec<(f32, f32, f32, f32)> = Vec::new();
    if let Some(anchor) = edge_label_anchor_from_point(edge, initial_center) {
        push_anchor_unique(&mut anchors, anchor);
    }
    for frac in LABEL_ANCHOR_FRACTIONS {
        if let Some(anchor) = edge_label_anchor_at_fraction(edge, frac) {
            push_anchor_unique(&mut anchors, anchor);
        }
    }
    for anchor in edge_segment_anchors(edge, LABEL_EXTRA_SEGMENT_ANCHORS) {
        push_anchor_unique(&mut anchors, anchor);
    }
    for anchor in edge_terminal_segment_anchors(edge, 2) {
        push_anchor_unique(&mut anchors, anchor);
    }
    if anchors.is_empty() {
        anchors.push(edge_label_anchor(edge));
    } else {
        push_anchor_unique(&mut anchors, edge_label_anchor(edge));
    }
    if let Some((initial_s, _)) = edge_relative_pose(&edge.points, initial_center) {
        let mut scored_anchors: Vec<((f32, f32, f32, f32), f32)> = anchors
            .iter()
            .copied()
            .map(|anchor| {
                let s = edge_relative_pose(&edge.points, (anchor.0, anchor.1))
                    .map(|(value, _)| value)
                    .unwrap_or(initial_s);
                (anchor, (s - initial_s).abs())
            })
            .collect();
        scored_anchors.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let mut filtered = Vec::new();
        for (anchor, s_delta) in scored_anchors {
            if s_delta <= 0.18 || filtered.len() < 3 {
                filtered.push(anchor);
            }
            if filtered.len() >= 6 {
                break;
            }
        }
        if !filtered.is_empty() {
            anchors = filtered;
        }
    }

    let normal_steps: &[f32] = &[
        0.45, -0.45, 0.8, -0.8, 1.2, -1.2, 0.25, -0.25, 1.8, -1.8, 2.5, -2.5, 3.4, -3.4, 4.4, -4.4,
        5.6, -5.6, 7.0, -7.0, 0.0,
    ];
    let tangent_steps: &[f32] = &[
        0.0, 0.22, -0.22, 0.55, -0.55, 1.0, -1.0, 1.6, -1.6, 2.3, -2.3, 2.8, -2.8,
    ];
    for (anchor_x, anchor_y, dir_x, dir_y) in anchors {
        let normal_x = -dir_y;
        let normal_y = dir_x;
        let step_n = if normal_x.abs() > normal_y.abs() {
            label_w + label_pad_x + step_normal_pad
        } else {
            label_h + label_pad_y + step_normal_pad
        };
        let step_t = if dir_x.abs() > dir_y.abs() {
            label_w + label_pad_x + step_tangent_pad
        } else {
            label_h + label_pad_y + step_tangent_pad
        };
        for t in tangent_steps {
            let base_x = anchor_x + dir_x * step_t * *t;
            let base_y = anchor_y + dir_y * step_t * *t;
            for n in normal_steps {
                let center = (
                    base_x + normal_x * step_n * *n,
                    base_y + normal_y * step_n * *n,
                );
                push_candidate(center);
            }
        }
    }
    candidates
}

fn flowchart_center_label_refine_cost(
    entry: &FlowchartCenterLabelEntry,
    center: (f32, f32),
    label_pad_x: f32,
    label_pad_y: f32,
    others: &[Rect],
    fixed_obstacles: &[Rect],
    edge_obstacles: &[EdgeObstacle],
    edge_grid: &ObstacleGrid,
) -> (f32, f32) {
    let core_rect = flowchart_center_label_rect(
        center,
        entry.label_w,
        entry.label_h,
        label_pad_x,
        label_pad_y,
    );
    let obstacle_rect = flowchart_center_label_obstacle_rect(
        center,
        entry.label_w,
        entry.label_h,
        label_pad_x,
        label_pad_y,
    );
    let area = (entry.label_w * entry.label_h).max(1.0);

    let mut overlap_area_sum = 0.0f32;
    let mut overlap_count = 0u32;
    let mut near_overlap_gap_sum = 0.0f32;
    for other in others {
        let ov = overlap_area(&obstacle_rect, other);
        if ov > 0.0 {
            overlap_area_sum += ov;
            overlap_count += 1;
            continue;
        }
        let gap = rect_gap(&obstacle_rect, other);
        if gap < FLOWCHART_LABEL_SOFT_GAP {
            near_overlap_gap_sum += FLOWCHART_LABEL_SOFT_GAP - gap;
        }
    }

    let mut fixed_overlap_area = 0.0f32;
    let mut fixed_overlap_count = 0u32;
    let mut fixed_near_gap_sum = 0.0f32;
    for obstacle in fixed_obstacles {
        let ov = overlap_area(&obstacle_rect, obstacle);
        if ov > 0.0 {
            fixed_overlap_area += ov;
            fixed_overlap_count += 1;
            continue;
        }
        let gap = rect_gap(&obstacle_rect, obstacle);
        if gap < FLOWCHART_LABEL_SOFT_GAP {
            fixed_near_gap_sum += FLOWCHART_LABEL_SOFT_GAP - gap;
        }
    }

    let own_edge_dist = polyline_rect_distance(&entry.edge_points, &core_rect);
    let mut own_edge_penalty = 0.0f32;
    if let Some((s_norm, d_signed)) = edge_relative_pose(&entry.edge_points, center) {
        let s_drift = (s_norm - entry.initial_s_norm).abs();
        own_edge_penalty += s_drift * s_drift * 2.2;
        let normal_scale = (entry.label_h + 2.0 * label_pad_y).max(1.0);
        let normal_drift = (d_signed - entry.initial_d_signed).abs() / normal_scale;
        own_edge_penalty += normal_drift * normal_drift * 1.6;
    }
    if own_edge_dist.is_finite() {
        let target_gap = OWN_EDGE_GAP_TARGET_FLOWCHART.max(1e-3);
        if own_edge_dist < target_gap {
            let shortage = (target_gap - own_edge_dist) / target_gap;
            own_edge_penalty += shortage * shortage * 7.8;
            if own_edge_dist < 1.0 {
                let under = 1.0 - own_edge_dist;
                own_edge_penalty += under * under * 8.0;
            }
        } else {
            let excess = (own_edge_dist - target_gap) / target_gap;
            own_edge_penalty += excess * excess * 1.35;
            if excess > 2.0 {
                own_edge_penalty += (excess - 2.0) * 2.8;
            }
        }
        if own_edge_dist > FLOWCHART_OWN_EDGE_SOFT_MAX_GAP {
            let over = own_edge_dist - FLOWCHART_OWN_EDGE_SOFT_MAX_GAP;
            own_edge_penalty += over * over * FLOWCHART_OWN_EDGE_SOFT_MAX_GAP_WEIGHT;
        }
        if own_edge_dist > FLOWCHART_OWN_EDGE_HARD_MAX_GAP {
            let over = own_edge_dist - FLOWCHART_OWN_EDGE_HARD_MAX_GAP;
            own_edge_penalty += over * FLOWCHART_OWN_EDGE_HARD_MAX_GAP_WEIGHT;
        }
        if own_edge_dist <= 0.35 {
            own_edge_penalty += 80.0;
        }
    }
    let mut foreign_edge_overlap_area = 0.0f32;
    let mut foreign_edge_touch = false;
    let mut foreign_edge_near_gap_sum = 0.0f32;
    for edge_obs_idx in edge_grid.query(&obstacle_rect) {
        let (obs_edge_idx, obs) = edge_obstacles[edge_obs_idx];
        if obs_edge_idx == entry.edge_idx {
            continue;
        }
        let ov = overlap_area(&obstacle_rect, &obs);
        if ov > 0.0 {
            foreign_edge_overlap_area += ov;
            foreign_edge_touch = true;
            continue;
        }
        let gap = rect_gap(&obstacle_rect, &obs);
        if gap < FLOWCHART_LABEL_SOFT_GAP {
            foreign_edge_near_gap_sum += FLOWCHART_LABEL_SOFT_GAP - gap;
        }
    }
    let foreign_edge_penalty = (foreign_edge_overlap_area / area) * 48.0
        + if foreign_edge_touch { 130.0 } else { 0.0 }
        + foreign_edge_near_gap_sum * 6.5;
    let edge_center_dist = point_polyline_distance(center, &entry.edge_points);
    let (base_target, base_soft_max, base_hard_max) =
        flowchart_center_distance_limits(entry.label_h, label_pad_y);
    let edge_target = flowchart_pose_center_target(
        &entry.edge_points,
        center,
        entry.label_w,
        entry.label_h,
        label_pad_x,
        label_pad_y,
    )
    .unwrap_or(base_target);
    let center_soft_max = edge_target + (base_soft_max - base_target).max(0.0);
    let center_hard_max = center_soft_max + (base_hard_max - base_soft_max).max(0.0);
    let mut edge_center_penalty = 0.0f32;
    if edge_center_dist < edge_target {
        let shortage = (edge_target - edge_center_dist) / edge_target.max(1e-3);
        edge_center_penalty += shortage * shortage * 2.4;
    }
    if edge_center_dist > edge_target {
        let excess = (edge_center_dist - edge_target) / edge_target.max(1e-3);
        edge_center_penalty += excess * excess * 1.3;
    }
    if edge_center_dist > center_soft_max {
        let over = edge_center_dist - center_soft_max;
        edge_center_penalty += over * over * 2.8;
    }
    if edge_center_dist > center_hard_max {
        let over = edge_center_dist - center_hard_max;
        edge_center_penalty += over * 20.0;
    }
    if let Some((s_norm, d_signed)) = edge_relative_pose(&entry.edge_points, center) {
        let path_len = polyline_path_length(&entry.edge_points).max(1.0);
        let tangent_shift_px = (s_norm - entry.initial_s_norm).abs() * path_len;
        let tangent_soft_px = (entry.label_w * 0.95 + entry.label_h * 0.35 + 8.0)
            .clamp(10.0, 28.0)
            .min(path_len * 0.28 + 2.0);
        let tangent_hard_px = (tangent_soft_px * 2.2).max(tangent_soft_px + 10.0);
        if tangent_shift_px > tangent_soft_px {
            let over = tangent_shift_px - tangent_soft_px;
            edge_center_penalty += (over / tangent_soft_px.max(1.0)).powi(2) * 12.0;
        }
        if tangent_shift_px > tangent_hard_px {
            let over = tangent_shift_px - tangent_hard_px;
            edge_center_penalty += over * 0.45;
        }
        let d_shift = (d_signed - entry.initial_d_signed).abs();
        edge_center_penalty += d_shift * 0.03;
    }
    let primary = fixed_overlap_count as f32 * 130.0
        + (fixed_overlap_area / area) * 48.0
        + fixed_near_gap_sum * 6.0
        + overlap_count as f32 * 115.0
        + (overlap_area_sum / area) * 42.0
        + near_overlap_gap_sum * 5.0
        + own_edge_penalty
        + foreign_edge_penalty
        + edge_center_penalty;
    let dx = center.0 - entry.initial_center.0;
    let dy = center.1 - entry.initial_center.1;
    let drift = (dx * dx + dy * dy).sqrt() / (entry.label_w + entry.label_h + 1.0);
    (primary, drift)
}

/// Resolve start/end label positions for all edges.
fn resolve_endpoint_labels(
    edges: &mut [EdgeLayout],
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
    bounds: Option<(f32, f32)>,
    kind: DiagramKind,
    theme: &Theme,
    config: &LayoutConfig,
) {
    let has_endpoint_labels = edges
        .iter()
        .any(|e| e.start_label.is_some() || e.end_label.is_some());
    if !has_endpoint_labels {
        return;
    }

    let (center_pad_x, center_pad_y) = edge_label_padding(kind, config);
    let node_obstacle_pad = match kind {
        DiagramKind::Class => (theme.font_size * 0.12).max(1.5),
        _ => (theme.font_size * 0.45).max(center_pad_x.max(center_pad_y)),
    };
    let edge_obstacle_pad = (theme.font_size * 0.35).max(center_pad_y);
    let subgraph_label_pad = (theme.font_size * 0.35).max(3.0);
    let (endpoint_pad_x, endpoint_pad_y) = endpoint_label_padding(kind);

    let edge_obstacles = build_edge_obstacles(edges, edge_obstacle_pad);
    let edge_obs_rects: Vec<Rect> = edge_obstacles.iter().map(|(_, r)| *r).collect();
    let endpoint_edge_grid = ObstacleGrid::new(48.0, &edge_obs_rects);

    // Start with node/subgraph obstacles + center label positions as obstacles.
    let endpoint_node_obstacle_pad = match kind {
        DiagramKind::Class => node_obstacle_pad * 0.4,
        DiagramKind::State => node_obstacle_pad * 0.65,
        _ => node_obstacle_pad,
    };
    let mut endpoint_occupied = build_label_obstacles(
        nodes,
        subgraphs,
        kind,
        theme,
        endpoint_node_obstacle_pad,
        subgraph_label_pad,
    );
    let endpoint_node_obstacle_count = endpoint_occupied.len();
    for edge in edges.iter() {
        if let (Some(label), Some((ax, ay))) = (&edge.label, edge.label_anchor) {
            let rect = (
                ax - label.width / 2.0 - center_pad_x,
                ay - label.height / 2.0 - center_pad_y,
                label.width + 2.0 * center_pad_x,
                label.height + 2.0 * center_pad_y,
            );
            let occupied_rect = if kind == DiagramKind::Flowchart {
                inflate_rect(rect, FLOWCHART_LABEL_CLEARANCE_PAD)
            } else {
                rect
            };
            endpoint_occupied.push(occupied_rect);
        }
    }

    let end_label_offset = match kind {
        DiagramKind::Class => (theme.font_size * 0.18).max(2.8),
        DiagramKind::Flowchart => (theme.font_size * 0.75).max(9.0),
        _ => (theme.font_size * 0.6).max(8.0),
    };
    let state_font_size = if kind == DiagramKind::State {
        theme.font_size * 0.85
    } else {
        theme.font_size
    };
    let endpoint_label_scale = if kind == DiagramKind::State {
        (state_font_size / theme.font_size).min(1.0)
    } else {
        1.0
    };

    let mut endpoint_grid = ObstacleGrid::new(48.0, &endpoint_occupied);

    for idx in 0..edges.len() {
        // Start label
        if let Some(label) = edges[idx].start_label.clone() {
            let label_w = label.width * endpoint_label_scale;
            let label_h = label.height * endpoint_label_scale;
            let endpoint_avoid_ctx = EndpointLabelAvoidContext {
                kind,
                offset: end_label_offset,
                occupied: &endpoint_occupied,
                occupied_grid: &endpoint_grid,
                node_obstacle_count: endpoint_node_obstacle_count,
                edge_obstacles: &edge_obstacles,
                edge_grid: &endpoint_edge_grid,
                bounds,
            };
            if let Some((x, y)) = edge_endpoint_label_position_with_avoid(
                &edges[idx],
                idx,
                true,
                label_w,
                label_h,
                endpoint_pad_x,
                endpoint_pad_y,
                &endpoint_avoid_ctx,
            ) {
                edges[idx].start_label_anchor = Some((x, y));
                let rect = (
                    x - label_w / 2.0 - endpoint_pad_x,
                    y - label_h / 2.0 - endpoint_pad_y,
                    label_w + endpoint_pad_x * 2.0,
                    label_h + endpoint_pad_y * 2.0,
                );
                let occupied_rect = if kind == DiagramKind::Flowchart {
                    inflate_rect(rect, FLOWCHART_LABEL_CLEARANCE_PAD)
                } else {
                    rect
                };
                endpoint_grid.insert(endpoint_occupied.len(), &occupied_rect);
                endpoint_occupied.push(occupied_rect);
            }
        }

        // End label
        if let Some(label) = edges[idx].end_label.clone() {
            let label_w = label.width * endpoint_label_scale;
            let label_h = label.height * endpoint_label_scale;
            let endpoint_avoid_ctx = EndpointLabelAvoidContext {
                kind,
                offset: end_label_offset,
                occupied: &endpoint_occupied,
                occupied_grid: &endpoint_grid,
                node_obstacle_count: endpoint_node_obstacle_count,
                edge_obstacles: &edge_obstacles,
                edge_grid: &endpoint_edge_grid,
                bounds,
            };
            if let Some((x, y)) = edge_endpoint_label_position_with_avoid(
                &edges[idx],
                idx,
                false,
                label_w,
                label_h,
                endpoint_pad_x,
                endpoint_pad_y,
                &endpoint_avoid_ctx,
            ) {
                edges[idx].end_label_anchor = Some((x, y));
                let rect = (
                    x - label_w / 2.0 - endpoint_pad_x,
                    y - label_h / 2.0 - endpoint_pad_y,
                    label_w + endpoint_pad_x * 2.0,
                    label_h + endpoint_pad_y * 2.0,
                );
                let occupied_rect = if kind == DiagramKind::Flowchart {
                    inflate_rect(rect, FLOWCHART_LABEL_CLEARANCE_PAD)
                } else {
                    rect
                };
                endpoint_grid.insert(endpoint_occupied.len(), &occupied_rect);
                endpoint_occupied.push(occupied_rect);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Geometry helpers (moved from render.rs)
// ---------------------------------------------------------------------------

fn edge_path_length(edge: &EdgeLayout) -> f32 {
    let mut total = 0.0f32;
    for pair in edge.points.windows(2) {
        let dx = pair[1].0 - pair[0].0;
        let dy = pair[1].1 - pair[0].1;
        total += (dx * dx + dy * dy).sqrt();
    }
    total
}

fn polyline_path_length(points: &[(f32, f32)]) -> f32 {
    let mut total = 0.0f32;
    for pair in points.windows(2) {
        let dx = pair[1].0 - pair[0].0;
        let dy = pair[1].1 - pair[0].1;
        total += (dx * dx + dy * dy).sqrt();
    }
    total
}

fn point_segment_distance(point: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let vx = b.0 - a.0;
    let vy = b.1 - a.1;
    let len2 = vx * vx + vy * vy;
    if len2 <= 1e-6 {
        let dx = point.0 - a.0;
        let dy = point.1 - a.1;
        return (dx * dx + dy * dy).sqrt();
    }
    let t = ((point.0 - a.0) * vx + (point.1 - a.1) * vy) / len2;
    let t = t.clamp(0.0, 1.0);
    let proj_x = a.0 + vx * t;
    let proj_y = a.1 + vy * t;
    let dx = point.0 - proj_x;
    let dy = point.1 - proj_y;
    (dx * dx + dy * dy).sqrt()
}

fn point_polyline_distance(point: (f32, f32), points: &[(f32, f32)]) -> f32 {
    if points.len() < 2 {
        return 0.0;
    }
    let mut best = f32::INFINITY;
    for seg in points.windows(2) {
        let dist = point_segment_distance(point, seg[0], seg[1]);
        if dist < best {
            best = dist;
        }
    }
    if best.is_finite() { best } else { 0.0 }
}

fn point_rect_distance(point: (f32, f32), rect: &Rect) -> f32 {
    let min_x = rect.0;
    let min_y = rect.1;
    let max_x = rect.0 + rect.2;
    let max_y = rect.1 + rect.3;
    let dx = if point.0 < min_x {
        min_x - point.0
    } else if point.0 > max_x {
        point.0 - max_x
    } else {
        0.0
    };
    let dy = if point.1 < min_y {
        min_y - point.1
    } else if point.1 > max_y {
        point.1 - max_y
    } else {
        0.0
    };
    (dx * dx + dy * dy).sqrt()
}

fn point_inside_rect(point: (f32, f32), rect: &Rect) -> bool {
    point.0 >= rect.0
        && point.0 <= rect.0 + rect.2
        && point.1 >= rect.1
        && point.1 <= rect.1 + rect.3
}

fn orientation(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

fn point_on_segment(point: (f32, f32), a: (f32, f32), b: (f32, f32), eps: f32) -> bool {
    point.0 >= a.0.min(b.0) - eps
        && point.0 <= a.0.max(b.0) + eps
        && point.1 >= a.1.min(b.1) - eps
        && point.1 <= a.1.max(b.1) + eps
}

fn segments_intersect(a: (f32, f32), b: (f32, f32), c: (f32, f32), d: (f32, f32)) -> bool {
    let eps = 1e-4;
    let o1 = orientation(a, b, c);
    let o2 = orientation(a, b, d);
    let o3 = orientation(c, d, a);
    let o4 = orientation(c, d, b);
    let crosses = ((o1 > eps && o2 < -eps) || (o1 < -eps && o2 > eps))
        && ((o3 > eps && o4 < -eps) || (o3 < -eps && o4 > eps));
    if crosses {
        return true;
    }
    if o1.abs() <= eps && point_on_segment(c, a, b, eps) {
        return true;
    }
    if o2.abs() <= eps && point_on_segment(d, a, b, eps) {
        return true;
    }
    if o3.abs() <= eps && point_on_segment(a, c, d, eps) {
        return true;
    }
    if o4.abs() <= eps && point_on_segment(b, c, d, eps) {
        return true;
    }
    false
}

fn segment_intersects_rect(a: (f32, f32), b: (f32, f32), rect: &Rect) -> bool {
    if point_inside_rect(a, rect) || point_inside_rect(b, rect) {
        return true;
    }
    let x0 = rect.0;
    let y0 = rect.1;
    let x1 = rect.0 + rect.2;
    let y1 = rect.1 + rect.3;
    let corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
    corners
        .iter()
        .zip(corners.iter().cycle().skip(1))
        .take(4)
        .any(|(c0, c1)| segments_intersect(a, b, *c0, *c1))
}

fn segment_rect_distance(a: (f32, f32), b: (f32, f32), rect: &Rect) -> f32 {
    if segment_intersects_rect(a, b, rect) {
        return 0.0;
    }
    let mut best = point_rect_distance(a, rect).min(point_rect_distance(b, rect));
    let x0 = rect.0;
    let y0 = rect.1;
    let x1 = rect.0 + rect.2;
    let y1 = rect.1 + rect.3;
    for corner in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
        best = best.min(point_segment_distance(corner, a, b));
    }
    best
}

fn polyline_rect_distance(points: &[(f32, f32)], rect: &Rect) -> f32 {
    if points.len() < 2 {
        return f32::INFINITY;
    }
    let mut best = f32::INFINITY;
    for seg in points.windows(2) {
        let dist = segment_rect_distance(seg[0], seg[1], rect);
        if dist < best {
            best = dist;
        }
        if best <= 0.0 {
            break;
        }
    }
    best
}

fn subgraph_label_rect(sub: &SubgraphLayout, kind: DiagramKind, theme: &Theme) -> Option<Rect> {
    if sub.label.trim().is_empty() {
        return None;
    }
    let width = sub.label_block.width;
    let height = sub.label_block.height;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    if kind == DiagramKind::State {
        let header_h = (height + theme.font_size * 0.75).max(theme.font_size * 1.4);
        let label_pad_x = (theme.font_size * 0.6).max(height * 0.35);
        let x = sub.x + label_pad_x;
        let y = sub.y + header_h / 2.0 - height / 2.0;
        Some((x, y, width, height))
    } else {
        let x = sub.x + sub.width / 2.0 - width / 2.0;
        let y = sub.y + 12.0;
        Some((x, y, width, height))
    }
}

fn build_label_obstacles(
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
    kind: DiagramKind,
    theme: &Theme,
    node_obstacle_pad: f32,
    subgraph_label_pad: f32,
) -> Vec<Rect> {
    let mut occupied: Vec<Rect> = Vec::new();
    for node in nodes.values() {
        if node.anchor_subgraph.is_some() || node.hidden {
            continue;
        }
        occupied.push((
            node.x - node_obstacle_pad,
            node.y - node_obstacle_pad,
            node.width + 2.0 * node_obstacle_pad,
            node.height + 2.0 * node_obstacle_pad,
        ));
    }
    for sub in subgraphs {
        if let Some(rect) = subgraph_label_rect(sub, kind, theme) {
            occupied.push((
                rect.0 - subgraph_label_pad,
                rect.1 - subgraph_label_pad,
                rect.2 + subgraph_label_pad * 2.0,
                rect.3 + subgraph_label_pad * 2.0,
            ));
        }
    }
    occupied
}

fn build_node_text_obstacles(nodes: &BTreeMap<String, NodeLayout>, pad: f32) -> Vec<Rect> {
    let mut occupied = Vec::new();
    for node in nodes.values() {
        if node.anchor_subgraph.is_some() || node.hidden {
            continue;
        }
        if node.label.width <= 0.0 || node.label.height <= 0.0 {
            continue;
        }
        let cx = node.x + node.width * 0.5;
        let cy = node.y + node.height * 0.5;
        occupied.push((
            cx - node.label.width * 0.5 - pad,
            cy - node.label.height * 0.5 - pad,
            node.label.width + pad * 2.0,
            node.label.height + pad * 2.0,
        ));
    }
    occupied
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

fn edge_label_anchor(edge: &EdgeLayout) -> (f32, f32, f32, f32) {
    if edge.points.len() < 2 {
        return (0.0, 0.0, 1.0, 0.0);
    }
    let segment_count = edge.points.len() - 1;
    let mut best_idx: Option<usize> = None;
    let mut best_len = 0.0;

    let (start_idx, end_idx) = if segment_count >= 3 {
        (1, segment_count - 1)
    } else {
        (0, segment_count)
    };

    for idx in start_idx..end_idx {
        let p1 = edge.points[idx];
        let p2 = edge.points[idx + 1];
        let dx = p2.0 - p1.0;
        let dy = p2.1 - p1.1;
        let len = dx * dx + dy * dy;
        if len > best_len {
            best_len = len;
            best_idx = Some(idx);
        }
    }

    if best_idx.is_none() {
        for idx in 0..segment_count {
            let p1 = edge.points[idx];
            let p2 = edge.points[idx + 1];
            let dx = p2.0 - p1.0;
            let dy = p2.1 - p1.1;
            let len = dx * dx + dy * dy;
            if len > best_len {
                best_len = len;
                best_idx = Some(idx);
            }
        }
    }

    let idx = best_idx.unwrap_or(0);
    let p1 = edge.points[idx];
    let p2 = edge.points[idx + 1];
    let dx = p2.0 - p1.0;
    let dy = p2.1 - p1.1;
    let len = (dx * dx + dy * dy).sqrt().max(1e-3);
    ((p1.0 + p2.0) / 2.0, (p1.1 + p2.1) / 2.0, dx / len, dy / len)
}

fn edge_label_anchor_at_fraction(edge: &EdgeLayout, t: f32) -> Option<(f32, f32, f32, f32)> {
    if edge.points.len() < 2 {
        return None;
    }
    let segment_count = edge.points.len() - 1;
    let (mut start_idx, mut end_idx) = if segment_count >= 3 {
        (1, segment_count - 1)
    } else {
        (0, segment_count)
    };
    if start_idx >= end_idx {
        start_idx = 0;
        end_idx = segment_count;
    }

    let mut total_len = 0.0f32;
    for idx in start_idx..end_idx {
        let p1 = edge.points[idx];
        let p2 = edge.points[idx + 1];
        let dx = p2.0 - p1.0;
        let dy = p2.1 - p1.1;
        total_len += (dx * dx + dy * dy).sqrt();
    }

    if total_len <= 1e-3 {
        return Some(edge_label_anchor(edge));
    }

    let mut remaining = total_len * t.clamp(0.0, 1.0);
    for idx in start_idx..end_idx {
        let p1 = edge.points[idx];
        let p2 = edge.points[idx + 1];
        let dx = p2.0 - p1.0;
        let dy = p2.1 - p1.1;
        let seg_len = (dx * dx + dy * dy).sqrt();
        if seg_len <= 1e-6 {
            continue;
        }
        if remaining <= seg_len {
            let alpha = (remaining / seg_len).clamp(0.0, 1.0);
            return Some((
                p1.0 + dx * alpha,
                p1.1 + dy * alpha,
                dx / seg_len,
                dy / seg_len,
            ));
        }
        remaining -= seg_len;
    }

    Some(edge_label_anchor(edge))
}

fn edge_label_anchor_from_point(
    edge: &EdgeLayout,
    point: (f32, f32),
) -> Option<(f32, f32, f32, f32)> {
    if edge.points.len() < 2 {
        return None;
    }
    let mut best_dist2 = f32::INFINITY;
    let mut best_proj: Option<(f32, f32)> = None;
    let mut best_dir: Option<(f32, f32)> = None;
    for segment in edge.points.windows(2) {
        let p1 = segment[0];
        let p2 = segment[1];
        let dx = p2.0 - p1.0;
        let dy = p2.1 - p1.1;
        let seg_len2 = dx * dx + dy * dy;
        if seg_len2 <= 1e-6 {
            continue;
        }
        let t = ((point.0 - p1.0) * dx + (point.1 - p1.1) * dy) / seg_len2;
        let t_clamped = t.clamp(0.0, 1.0);
        let proj_x = p1.0 + dx * t_clamped;
        let proj_y = p1.1 + dy * t_clamped;
        let dist2 =
            (point.0 - proj_x) * (point.0 - proj_x) + (point.1 - proj_y) * (point.1 - proj_y);
        if dist2 < best_dist2 {
            best_dist2 = dist2;
            best_proj = Some((proj_x, proj_y));
            best_dir = Some((dx, dy));
        }
    }
    let (proj_x, proj_y) = best_proj?;
    let (dx, dy) = best_dir?;
    let len = (dx * dx + dy * dy).sqrt().max(1e-3);
    Some((proj_x, proj_y, dx / len, dy / len))
}

fn edge_segment_anchors(edge: &EdgeLayout, max_count: usize) -> Vec<(f32, f32, f32, f32)> {
    if edge.points.len() < 2 || max_count == 0 {
        return Vec::new();
    }
    let segment_count = edge.points.len() - 1;
    let (mut start_idx, mut end_idx) = if segment_count >= 3 {
        (1, segment_count - 1)
    } else {
        (0, segment_count)
    };
    if start_idx >= end_idx {
        start_idx = 0;
        end_idx = segment_count;
    }
    let mut scored: Vec<(f32, (f32, f32, f32, f32))> = Vec::new();
    for idx in start_idx..end_idx {
        let p1 = edge.points[idx];
        let p2 = edge.points[idx + 1];
        let dx = p2.0 - p1.0;
        let dy = p2.1 - p1.1;
        let len = (dx * dx + dy * dy).sqrt();
        if len <= 1.0 {
            continue;
        }
        let dir_x = dx / len;
        let dir_y = dy / len;
        scored.push((
            len,
            ((p1.0 + p2.0) * 0.5, (p1.1 + p2.1) * 0.5, dir_x, dir_y),
        ));
    }
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored
        .into_iter()
        .take(max_count)
        .map(|(_, anchor)| anchor)
        .collect()
}

fn edge_terminal_segment_anchors(edge: &EdgeLayout, max_count: usize) -> Vec<(f32, f32, f32, f32)> {
    if edge.points.len() < 2 || max_count == 0 {
        return Vec::new();
    }
    let mut result: Vec<(f32, f32, f32, f32)> = Vec::new();
    let seg_count = edge.points.len() - 1;
    for seg_idx in [0usize, seg_count.saturating_sub(1)] {
        if result.len() >= max_count {
            break;
        }
        if seg_idx >= seg_count {
            continue;
        }
        let p1 = edge.points[seg_idx];
        let p2 = edge.points[seg_idx + 1];
        let dx = p2.0 - p1.0;
        let dy = p2.1 - p1.1;
        let len = (dx * dx + dy * dy).sqrt();
        if len <= 8.0 {
            continue;
        }
        let dir_x = dx / len;
        let dir_y = dy / len;
        let anchor = ((p1.0 + p2.0) * 0.5, (p1.1 + p2.1) * 0.5, dir_x, dir_y);
        if !result.iter().any(|existing| {
            (existing.0 - anchor.0).abs() <= LABEL_ANCHOR_POS_EPS
                && (existing.1 - anchor.1).abs() <= LABEL_ANCHOR_POS_EPS
                && (existing.2 - anchor.2).abs() <= LABEL_ANCHOR_DIR_EPS
                && (existing.3 - anchor.3).abs() <= LABEL_ANCHOR_DIR_EPS
        }) {
            result.push(anchor);
        }
    }
    result.truncate(max_count);
    result
}

fn push_anchor_unique(anchors: &mut Vec<(f32, f32, f32, f32)>, candidate: (f32, f32, f32, f32)) {
    let duplicate = anchors.iter().any(|anchor| {
        (anchor.0 - candidate.0).abs() <= LABEL_ANCHOR_POS_EPS
            && (anchor.1 - candidate.1).abs() <= LABEL_ANCHOR_POS_EPS
            && (anchor.2 - candidate.2).abs() <= LABEL_ANCHOR_DIR_EPS
            && (anchor.3 - candidate.3).abs() <= LABEL_ANCHOR_DIR_EPS
    });
    if !duplicate {
        anchors.push(candidate);
    }
}

fn edge_label_bundle_fractions(edges: &[EdgeLayout]) -> Vec<Option<f32>> {
    let mut bundle_map: HashMap<(String, String), Vec<usize>> = HashMap::new();
    for (idx, edge) in edges.iter().enumerate() {
        if edge.label.is_none() {
            continue;
        }
        bundle_map
            .entry((edge.from.clone(), edge.to.clone()))
            .or_default()
            .push(idx);
    }
    let mut preferred = vec![None; edges.len()];
    for indices in bundle_map.values_mut() {
        if indices.len() <= 1 {
            continue;
        }
        indices.sort_unstable();
        let count = indices.len();
        let left = 0.16f32;
        let right = 0.84f32;
        let span = (right - left).max(0.0);
        for (rank, edge_idx) in indices.iter().enumerate() {
            let fraction = if count == 2 {
                if rank == 0 { 0.34 } else { 0.66 }
            } else {
                left + span * (rank as f32 / (count.saturating_sub(1) as f32))
            };
            preferred[*edge_idx] = Some(fraction.clamp(0.05, 0.95));
        }
    }
    preferred
}

fn overlap_area(a: &Rect, b: &Rect) -> f32 {
    let x0 = a.0.max(b.0);
    let y0 = a.1.max(b.1);
    let x1 = (a.0 + a.2).min(b.0 + b.2);
    let y1 = (a.1 + a.3).min(b.1 + b.3);
    let w = (x1 - x0).max(0.0);
    let h = (y1 - y0).max(0.0);
    w * h
}

fn overlap_stats(rect: Rect, obstacles: &[Rect], threshold: f32) -> (u32, f32) {
    let mut count = 0u32;
    let mut area = 0.0f32;
    for obstacle in obstacles {
        let ov = overlap_area(&rect, obstacle);
        if ov > threshold {
            count += 1;
            area += ov;
        }
    }
    (count, area)
}

fn rect_gap(a: &Rect, b: &Rect) -> f32 {
    let ax0 = a.0;
    let ay0 = a.1;
    let ax1 = a.0 + a.2;
    let ay1 = a.1 + a.3;
    let bx0 = b.0;
    let by0 = b.1;
    let bx1 = b.0 + b.2;
    let by1 = b.1 + b.3;
    let dx = if ax1 < bx0 {
        bx0 - ax1
    } else if bx1 < ax0 {
        ax0 - bx1
    } else {
        0.0
    };
    let dy = if ay1 < by0 {
        by0 - ay1
    } else if by1 < ay0 {
        ay0 - by1
    } else {
        0.0
    };
    (dx * dx + dy * dy).sqrt()
}

fn inflate_rect(rect: Rect, pad: f32) -> Rect {
    if pad <= 0.0 {
        return rect;
    }
    (
        rect.0 - pad,
        rect.1 - pad,
        rect.2 + pad * 2.0,
        rect.3 + pad * 2.0,
    )
}

fn outside_area(rect: &Rect, bounds: (f32, f32)) -> f32 {
    let (w, h) = bounds;
    let rect_area = rect.2.max(0.0) * rect.3.max(0.0);
    if rect_area <= 0.0 {
        return 0.0;
    }
    let x0 = rect.0.max(0.0);
    let y0 = rect.1.max(0.0);
    let x1 = (rect.0 + rect.2).min(w);
    let y1 = (rect.1 + rect.3).min(h);
    let inside_w = (x1 - x0).max(0.0);
    let inside_h = (y1 - y0).max(0.0);
    rect_area - inside_w * inside_h
}

fn clamp_label_center_to_bounds(
    center: (f32, f32),
    label_w: f32,
    label_h: f32,
    pad_x: f32,
    pad_y: f32,
    bounds: (f32, f32),
) -> (f32, f32) {
    let (w, h) = bounds;
    if w <= 0.0 || h <= 0.0 {
        return center;
    }
    let min_x = label_w * 0.5 + pad_x;
    let min_y = label_h * 0.5 + pad_y;
    let max_x = w - label_w * 0.5 - pad_x;
    let max_y = h - label_h * 0.5 - pad_y;

    let x = if max_x < min_x {
        center.0
    } else {
        center.0.clamp(min_x, max_x)
    };
    let y = if max_y < min_y {
        center.1
    } else {
        center.1.clamp(min_y, max_y)
    };
    (x, y)
}

/// Spatial index for fast overlap queries during label placement.
struct ObstacleGrid {
    cell: f32,
    /// Maps grid cell (ix, iy) to indices into the obstacle list.
    cells: HashMap<(i32, i32), Vec<usize>>,
}

impl ObstacleGrid {
    fn new(cell: f32, rects: &[Rect]) -> Self {
        let cell = cell.max(16.0);
        let mut cells: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (i, rect) in rects.iter().enumerate() {
            let x0 = (rect.0 / cell).floor() as i32;
            let y0 = (rect.1 / cell).floor() as i32;
            let x1 = ((rect.0 + rect.2) / cell).floor() as i32;
            let y1 = ((rect.1 + rect.3) / cell).floor() as i32;
            for ix in x0..=x1 {
                for iy in y0..=y1 {
                    cells.entry((ix, iy)).or_default().push(i);
                }
            }
        }
        Self { cell, cells }
    }

    /// Add a new obstacle at the given index to the grid.
    fn insert(&mut self, idx: usize, rect: &Rect) {
        let x0 = (rect.0 / self.cell).floor() as i32;
        let y0 = (rect.1 / self.cell).floor() as i32;
        let x1 = ((rect.0 + rect.2) / self.cell).floor() as i32;
        let y1 = ((rect.1 + rect.3) / self.cell).floor() as i32;
        for ix in x0..=x1 {
            for iy in y0..=y1 {
                self.cells.entry((ix, iy)).or_default().push(idx);
            }
        }
    }

    /// Return indices of obstacles that could overlap with `rect`.
    fn query(&self, rect: &Rect) -> impl Iterator<Item = usize> + '_ {
        let x0 = (rect.0 / self.cell).floor() as i32;
        let y0 = (rect.1 / self.cell).floor() as i32;
        let x1 = ((rect.0 + rect.2) / self.cell).floor() as i32;
        let y1 = ((rect.1 + rect.3) / self.cell).floor() as i32;
        let mut seen = HashSet::new();
        (x0..=x1)
            .flat_map(move |ix| (y0..=y1).map(move |iy| (ix, iy)))
            .flat_map(move |key| {
                self.cells
                    .get(&key)
                    .map(|v| v.as_slice())
                    .unwrap_or(&[])
                    .iter()
                    .copied()
            })
            .filter(move |idx| seen.insert(*idx))
    }
}

// Overlap penalty weights: node/subgraph overlap is worst, label overlap is
// moderate, edge overlap is mild (labels on edges is common and often acceptable).
const WEIGHT_NODE_OVERLAP: f32 = 1.6;
const WEIGHT_NODE_OVERLAP_FLOWCHART: f32 = 2.6;
const WEIGHT_LABEL_OVERLAP: f32 = 1.0;
const WEIGHT_FLOWCHART_LABEL_OVERLAP: f32 = 1.5;
const WEIGHT_EDGE_OVERLAP: f32 = 0.45;
const WEIGHT_FLOWCHART_EDGE_OVERLAP: f32 = 1.15;
const WEIGHT_OUTSIDE: f32 = 1.2;
const OWN_EDGE_GAP_TARGET: f32 = 1.2;
const OWN_EDGE_GAP_TARGET_FLOWCHART: f32 = 1.8;
const OWN_EDGE_GAP_TARGET_CLASS: f32 = 0.35;
const OWN_EDGE_GAP_UNDER_WEIGHT: f32 = 0.7;
const OWN_EDGE_GAP_UNDER_WEIGHT_FLOWCHART: f32 = 1.6;
const OWN_EDGE_GAP_UNDER_WEIGHT_CLASS: f32 = 0.08;
const OWN_EDGE_GAP_OVER_WEIGHT: f32 = 0.06;
const OWN_EDGE_GAP_OVER_WEIGHT_FLOWCHART: f32 = 0.65;
const OWN_EDGE_GAP_OVER_WEIGHT_CLASS: f32 = 0.18;
const OWN_EDGE_TOUCH_HARD_PENALTY: f32 = 0.25;
const OWN_EDGE_TOUCH_HARD_PENALTY_FLOWCHART: f32 = 1.25;
const OWN_EDGE_TOUCH_HARD_PENALTY_CLASS: f32 = 0.02;
const FLOWCHART_OWN_EDGE_SOFT_MAX_GAP: f32 = 6.0;
const FLOWCHART_OWN_EDGE_HARD_MAX_GAP: f32 = 10.0;
const FLOWCHART_OWN_EDGE_SOFT_MAX_GAP_WEIGHT: f32 = 0.85;
const FLOWCHART_OWN_EDGE_HARD_MAX_GAP_WEIGHT: f32 = 4.5;
const FLOWCHART_FOREIGN_EDGE_OVERLAP_WEIGHT: f32 = 0.9;
const FLOWCHART_FOREIGN_EDGE_TOUCH_HARD_PENALTY: f32 = 2.0;
const STATE_OWN_EDGE_HARD_MAX_GAP: f32 = 7.0;
const CLASS_OWN_EDGE_HARD_MAX_GAP: f32 = 7.0;
const DEFAULT_OWN_EDGE_HARD_MAX_GAP: f32 = 8.0;

fn center_label_hard_max_gap(kind: DiagramKind) -> Option<f32> {
    match kind {
        DiagramKind::Flowchart => Some(FLOWCHART_OWN_EDGE_HARD_MAX_GAP),
        DiagramKind::State => Some(STATE_OWN_EDGE_HARD_MAX_GAP),
        DiagramKind::Class => Some(CLASS_OWN_EDGE_HARD_MAX_GAP),
        DiagramKind::Sequence | DiagramKind::ZenUML => None,
        _ => Some(DEFAULT_OWN_EDGE_HARD_MAX_GAP),
    }
}

struct LabelPenaltyContext<'a> {
    kind: DiagramKind,
    occupied: &'a [Rect],
    occupied_grid: &'a ObstacleGrid,
    node_obstacle_count: usize,
    edge_obstacles: &'a [EdgeObstacle],
    edge_grid: &'a ObstacleGrid,
    edge_idx: usize,
    own_edge_points: &'a [(f32, f32)],
    bounds: Option<(f32, f32)>,
}

struct EndpointLabelAvoidContext<'a> {
    kind: DiagramKind,
    offset: f32,
    occupied: &'a [Rect],
    occupied_grid: &'a ObstacleGrid,
    node_obstacle_count: usize,
    edge_obstacles: &'a [EdgeObstacle],
    edge_grid: &'a ObstacleGrid,
    bounds: Option<(f32, f32)>,
}

fn label_penalties(
    rect: Rect,
    anchor: (f32, f32),
    label_w: f32,
    label_h: f32,
    ctx: &LabelPenaltyContext<'_>,
) -> (f32, f32) {
    let kind = ctx.kind;
    let area = (label_w * label_h).max(1.0);
    let mut overlap = 0.0;
    let label_weight = if kind == DiagramKind::Flowchart {
        WEIGHT_FLOWCHART_LABEL_OVERLAP
    } else {
        WEIGHT_LABEL_OVERLAP
    };
    let edge_weight = if kind == DiagramKind::Flowchart {
        WEIGHT_FLOWCHART_EDGE_OVERLAP
    } else {
        WEIGHT_EDGE_OVERLAP
    };
    let mut foreign_edge_overlap = 0.0f32;
    let mut foreign_edge_touch = false;
    for i in ctx.occupied_grid.query(&rect) {
        let ov = overlap_area(&rect, &ctx.occupied[i]);
        if ov > 0.0 {
            let weight = if i < ctx.node_obstacle_count {
                if kind == DiagramKind::Flowchart {
                    WEIGHT_NODE_OVERLAP_FLOWCHART
                } else {
                    WEIGHT_NODE_OVERLAP
                }
            } else {
                label_weight
            };
            overlap += ov * weight;
        }
    }
    for i in ctx.edge_grid.query(&rect) {
        let (idx, ref obs) = ctx.edge_obstacles[i];
        if idx == ctx.edge_idx {
            continue;
        }
        let ov = overlap_area(&rect, obs);
        overlap += ov * edge_weight;
        if kind == DiagramKind::Flowchart && ov > 0.0 {
            foreign_edge_overlap += ov;
            foreign_edge_touch = true;
        }
    }
    if kind == DiagramKind::Flowchart {
        overlap += foreign_edge_overlap * FLOWCHART_FOREIGN_EDGE_OVERLAP_WEIGHT;
        if foreign_edge_touch {
            overlap += area * FLOWCHART_FOREIGN_EDGE_TOUCH_HARD_PENALTY;
        }
    }
    if let Some(bound) = ctx.bounds {
        overlap += outside_area(&rect, bound) * WEIGHT_OUTSIDE;
    }
    let own_edge_rect = if kind == DiagramKind::State {
        let pad_x = ((rect.2 - label_w).max(0.0)) * 0.5;
        let pad_y = ((rect.3 - label_h).max(0.0)) * 0.5;
        (rect.0 + pad_x, rect.1 + pad_y, label_w, label_h)
    } else {
        rect
    };
    let own_edge_dist = polyline_rect_distance(ctx.own_edge_points, &own_edge_rect);
    if own_edge_dist.is_finite() {
        let (target_gap, under_weight, over_weight, hard_penalty) =
            if kind == DiagramKind::Flowchart {
                (
                    OWN_EDGE_GAP_TARGET_FLOWCHART,
                    OWN_EDGE_GAP_UNDER_WEIGHT_FLOWCHART,
                    OWN_EDGE_GAP_OVER_WEIGHT_FLOWCHART,
                    OWN_EDGE_TOUCH_HARD_PENALTY_FLOWCHART,
                )
            } else if kind == DiagramKind::Class {
                (
                    OWN_EDGE_GAP_TARGET_CLASS,
                    OWN_EDGE_GAP_UNDER_WEIGHT_CLASS,
                    OWN_EDGE_GAP_OVER_WEIGHT_CLASS,
                    OWN_EDGE_TOUCH_HARD_PENALTY_CLASS,
                )
            } else {
                (
                    OWN_EDGE_GAP_TARGET,
                    OWN_EDGE_GAP_UNDER_WEIGHT,
                    OWN_EDGE_GAP_OVER_WEIGHT,
                    OWN_EDGE_TOUCH_HARD_PENALTY,
                )
            };
        if own_edge_dist < target_gap {
            let shortage = (target_gap - own_edge_dist) / target_gap.max(1e-3);
            overlap += area * (shortage * shortage * under_weight);
        }
        if own_edge_dist > target_gap {
            let excess = (own_edge_dist - target_gap) / target_gap.max(1e-3);
            overlap += area * (excess * excess * over_weight);
        }
        if kind == DiagramKind::Flowchart && own_edge_dist > FLOWCHART_OWN_EDGE_SOFT_MAX_GAP {
            let over = own_edge_dist - FLOWCHART_OWN_EDGE_SOFT_MAX_GAP;
            overlap += area * (over * over * FLOWCHART_OWN_EDGE_SOFT_MAX_GAP_WEIGHT);
        }
        if kind == DiagramKind::Flowchart && own_edge_dist > FLOWCHART_OWN_EDGE_HARD_MAX_GAP {
            let over = own_edge_dist - FLOWCHART_OWN_EDGE_HARD_MAX_GAP;
            overlap += area * (over * FLOWCHART_OWN_EDGE_HARD_MAX_GAP_WEIGHT);
        }
        if own_edge_dist <= 0.35 {
            overlap += area * hard_penalty;
        }
    }
    let dx = (rect.0 + rect.2 * 0.5) - anchor.0;
    let dy = (rect.1 + rect.3 * 0.5) - anchor.1;
    let dist = (dx * dx + dy * dy).sqrt();
    (overlap / area, dist / (label_w + label_h + 1.0))
}

fn candidate_better(candidate: (f32, f32), best: (f32, f32)) -> bool {
    if candidate.0 + 1e-6 < best.0 {
        return true;
    }
    (candidate.0 - best.0).abs() <= 1e-6 && candidate.1 + 1e-6 < best.1
}

pub(crate) fn edge_endpoint_label_position(
    edge: &EdgeLayout,
    start: bool,
    offset: f32,
) -> Option<(f32, f32)> {
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

fn edge_endpoint_label_position_with_avoid(
    edge: &EdgeLayout,
    edge_idx: usize,
    start: bool,
    label_w: f32,
    label_h: f32,
    pad_x: f32,
    pad_y: f32,
    ctx: &EndpointLabelAvoidContext<'_>,
) -> Option<(f32, f32)> {
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
    let kind = ctx.kind;
    let offset = ctx.offset;
    let dir_x = dx / len;
    let dir_y = dy / len;
    let perp_x = -dir_y;
    let perp_y = dir_x;
    let anchor_along_factor = match kind {
        DiagramKind::Class => 0.55,
        DiagramKind::Flowchart => 1.4,
        DiagramKind::Requirement => 1.1,
        _ => 1.0,
    };
    let anchor_x = p0.0 + dir_x * offset * anchor_along_factor;
    let anchor_y = p0.1 + dir_y * offset * anchor_along_factor;
    let along_steps: &[f32] = match kind {
        DiagramKind::Class => &[0.0, 0.15, -0.15, 0.35, -0.35, 0.55, -0.55],
        _ => &[0.0, 0.8, -0.8, 1.6, -1.6],
    };
    let perp_steps: &[f32] = match kind {
        DiagramKind::Class => &[0.0, 0.35, -0.35, 0.7, -0.7, 1.05, -1.05, 1.5, -1.5],
        _ => &[
            1.0, -1.0, 1.7, -1.7, 2.4, -2.4, 3.2, -3.2, 3.9, -3.9, 4.6, -4.6,
        ],
    };
    let mut best_pos = (anchor_x, anchor_y);
    let mut best_penalty = (f32::INFINITY, f32::INFINITY);
    let penalty_ctx = LabelPenaltyContext {
        kind,
        occupied: ctx.occupied,
        occupied_grid: ctx.occupied_grid,
        node_obstacle_count: ctx.node_obstacle_count,
        edge_obstacles: ctx.edge_obstacles,
        edge_grid: ctx.edge_grid,
        edge_idx,
        own_edge_points: &edge.points,
        bounds: ctx.bounds,
    };
    for &along in along_steps {
        let base_x = p0.0 + dir_x * offset * (1.4 + along);
        let base_y = p0.1 + dir_y * offset * (1.4 + along);
        for &step in perp_steps {
            let x = base_x + perp_x * offset * step;
            let y = base_y + perp_y * offset * step;
            let rect = (
                x - label_w / 2.0 - pad_x,
                y - label_h / 2.0 - pad_y,
                label_w + pad_x * 2.0,
                label_h + pad_y * 2.0,
            );
            let mut penalty =
                label_penalties(rect, (anchor_x, anchor_y), label_w, label_h, &penalty_ctx);
            // Keep endpoint labels near the endpoint along their carrying edge.
            // This prevents class/state endpoint labels from drifting deep into
            // the middle of long edges when nearby obstacles are sparse.
            let along = (x - p0.0) * dir_x + (y - p0.1) * dir_y;
            let (soft_max, under_weight, over_weight) = match kind {
                DiagramKind::Class => (offset * 0.9, 0.45, 1.2),
                DiagramKind::State => (offset * 1.3, 0.28, 0.7),
                DiagramKind::Flowchart => (offset * 2.6, 0.14, 0.28),
                _ => (offset * 1.7, 0.22, 0.45),
            };
            let under = (0.0 - along).max(0.0);
            let over = (along - soft_max).max(0.0);
            if under > 0.0 || over > 0.0 {
                let endpoint_drift_penalty = (under * under * under_weight
                    + over * over * over_weight)
                    / (label_w + label_h + 1.0).max(1.0);
                penalty.0 += endpoint_drift_penalty;
            }
            if candidate_better(penalty, best_penalty) {
                best_penalty = penalty;
                best_pos = (x, y);
            }
        }
    }
    if let Some(bound) = ctx.bounds {
        let clamped = clamp_label_center_to_bounds(best_pos, label_w, label_h, pad_x, pad_y, bound);
        return Some(clamped);
    }
    Some(best_pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_label_penalty_context<'a>(
        kind: DiagramKind,
        occupied: &'a [Rect],
        occupied_grid: &'a ObstacleGrid,
        node_obstacle_count: usize,
        edge_obstacles: &'a [EdgeObstacle],
        edge_grid: &'a ObstacleGrid,
        edge_idx: usize,
        own_edge_points: &'a [(f32, f32)],
    ) -> LabelPenaltyContext<'a> {
        LabelPenaltyContext {
            kind,
            occupied,
            occupied_grid,
            node_obstacle_count,
            edge_obstacles,
            edge_grid,
            edge_idx,
            own_edge_points,
            bounds: None,
        }
    }

    #[test]
    fn overlap_area_no_overlap() {
        let a: Rect = (0.0, 0.0, 10.0, 10.0);
        let b: Rect = (20.0, 20.0, 10.0, 10.0);
        assert_eq!(overlap_area(&a, &b), 0.0);
    }

    #[test]
    fn overlap_area_partial_overlap() {
        let a: Rect = (0.0, 0.0, 10.0, 10.0);
        let b: Rect = (5.0, 5.0, 10.0, 10.0);
        assert_eq!(overlap_area(&a, &b), 25.0);
    }

    #[test]
    fn overlap_area_contained() {
        let a: Rect = (0.0, 0.0, 20.0, 20.0);
        let b: Rect = (5.0, 5.0, 5.0, 5.0);
        assert_eq!(overlap_area(&a, &b), 25.0);
    }

    #[test]
    fn outside_area_fully_inside() {
        let rect: Rect = (10.0, 10.0, 20.0, 20.0);
        assert_eq!(outside_area(&rect, (100.0, 100.0)), 0.0);
    }

    #[test]
    fn outside_area_partially_outside() {
        let rect: Rect = (90.0, 0.0, 20.0, 10.0);
        // 10 pixels overhang on x, so 10*10 = 100 pixels outside
        assert_eq!(outside_area(&rect, (100.0, 100.0)), 100.0);
    }

    #[test]
    fn outside_area_fully_outside() {
        let rect: Rect = (200.0, 200.0, 10.0, 10.0);
        assert_eq!(outside_area(&rect, (100.0, 100.0)), 100.0);
    }

    #[test]
    fn polyline_rect_distance_zero_when_segment_crosses_rect() {
        let rect: Rect = (10.0, 10.0, 20.0, 20.0);
        let points = vec![(0.0, 20.0), (40.0, 20.0)];
        let dist = polyline_rect_distance(&points, &rect);
        assert!(
            dist <= 1e-4,
            "expected intersection distance ~0, got {dist}"
        );
    }

    #[test]
    fn polyline_rect_distance_positive_when_clear() {
        let rect: Rect = (10.0, 10.0, 20.0, 20.0);
        let points = vec![(0.0, 40.0), (40.0, 40.0)];
        let dist = polyline_rect_distance(&points, &rect);
        assert!(
            (dist - 10.0).abs() < 1e-3,
            "expected 10px gap below rectangle, got {dist}"
        );
    }

    #[test]
    fn label_penalties_increase_when_touching_own_edge() {
        let rect_touch: Rect = (10.0, 10.0, 20.0, 10.0);
        let rect_clear: Rect = (10.0, 16.0, 20.0, 10.0);
        let edge_points = vec![(0.0, 15.0), (40.0, 15.0)];
        let occupied: Vec<Rect> = Vec::new();
        let occupied_grid = ObstacleGrid::new(20.0, &occupied);
        let edge_obstacles: Vec<EdgeObstacle> = Vec::new();
        let edge_rects: Vec<Rect> = Vec::new();
        let edge_grid = ObstacleGrid::new(20.0, &edge_rects);
        let ctx = test_label_penalty_context(
            DiagramKind::Flowchart,
            &occupied,
            &occupied_grid,
            0,
            &edge_obstacles,
            &edge_grid,
            0,
            &edge_points,
        );

        let touch = label_penalties(rect_touch, (20.0, 15.0), 20.0, 10.0, &ctx);
        let clear = label_penalties(rect_clear, (20.0, 15.0), 20.0, 10.0, &ctx);

        assert!(
            touch.0 > clear.0,
            "touching own edge should cost more than clear placement"
        );
    }

    #[test]
    fn label_penalties_increase_when_too_far_from_own_edge() {
        let rect_near: Rect = (10.0, 14.0, 20.0, 10.0);
        let rect_far: Rect = (10.0, 44.0, 20.0, 10.0);
        let edge_points = vec![(0.0, 15.0), (40.0, 15.0)];
        let occupied: Vec<Rect> = Vec::new();
        let occupied_grid = ObstacleGrid::new(20.0, &occupied);
        let edge_obstacles: Vec<EdgeObstacle> = Vec::new();
        let edge_rects: Vec<Rect> = Vec::new();
        let edge_grid = ObstacleGrid::new(20.0, &edge_rects);
        let ctx = test_label_penalty_context(
            DiagramKind::Flowchart,
            &occupied,
            &occupied_grid,
            0,
            &edge_obstacles,
            &edge_grid,
            0,
            &edge_points,
        );

        let near = label_penalties(rect_near, (20.0, 15.0), 20.0, 10.0, &ctx);
        let far = label_penalties(rect_far, (20.0, 15.0), 20.0, 10.0, &ctx);

        assert!(
            far.0 > near.0,
            "large own-edge gap should cost more than near-target placement"
        );
    }

    #[test]
    fn label_penalties_increase_when_touching_foreign_edge() {
        let rect_touch_foreign: Rect = (24.0, 10.0, 20.0, 10.0);
        let rect_clear_foreign: Rect = (60.0, 10.0, 20.0, 10.0);
        let own_edge_points = vec![(0.0, 15.0), (100.0, 15.0)];
        let occupied: Vec<Rect> = Vec::new();
        let occupied_grid = ObstacleGrid::new(20.0, &occupied);
        let edge_obstacles: Vec<EdgeObstacle> = vec![(1, (29.5, 0.0, 1.0, 50.0))];
        let edge_rects: Vec<Rect> = edge_obstacles.iter().map(|(_, rect)| *rect).collect();
        let edge_grid = ObstacleGrid::new(20.0, &edge_rects);
        let ctx = test_label_penalty_context(
            DiagramKind::Flowchart,
            &occupied,
            &occupied_grid,
            0,
            &edge_obstacles,
            &edge_grid,
            0,
            &own_edge_points,
        );

        let touch = label_penalties(rect_touch_foreign, (34.0, 15.0), 20.0, 10.0, &ctx);
        let clear = label_penalties(rect_clear_foreign, (70.0, 15.0), 20.0, 10.0, &ctx);

        assert!(
            touch.0 > clear.0,
            "touching a non-owner edge should cost more than a clear placement"
        );
    }

    #[test]
    fn clamp_label_center_stays_inside() {
        // Label 20x10 with 2px padding, bounds 100x100
        let result = clamp_label_center_to_bounds((5.0, 5.0), 20.0, 10.0, 2.0, 2.0, (100.0, 100.0));
        assert!(result.0 >= 12.0, "x should be clamped away from left edge");
        assert!(result.1 >= 7.0, "y should be clamped away from top edge");
    }

    #[test]
    fn clamp_label_center_no_op_when_inside() {
        let result =
            clamp_label_center_to_bounds((50.0, 50.0), 20.0, 10.0, 2.0, 2.0, (100.0, 100.0));
        assert_eq!(result, (50.0, 50.0));
    }

    #[test]
    fn obstacle_grid_query_finds_nearby_rect() {
        let rects = vec![(10.0, 10.0, 30.0, 30.0)];
        let grid = ObstacleGrid::new(20.0, &rects);
        let hits: Vec<usize> = grid.query(&(15.0, 15.0, 5.0, 5.0)).collect();
        assert!(hits.contains(&0), "grid should find overlapping rect");
    }

    #[test]
    fn obstacle_grid_query_misses_distant_rect() {
        let rects = vec![(10.0, 10.0, 30.0, 30.0)];
        let grid = ObstacleGrid::new(20.0, &rects);
        let hits: Vec<usize> = grid.query(&(200.0, 200.0, 5.0, 5.0)).collect();
        assert!(hits.is_empty(), "grid should not find distant rect");
    }

    #[test]
    fn obstacle_grid_insert_finds_new_item() {
        let initial: Vec<Rect> = vec![];
        let mut grid = ObstacleGrid::new(20.0, &initial);
        let new_rect: Rect = (50.0, 50.0, 10.0, 10.0);
        grid.insert(0, &new_rect);
        let hits: Vec<usize> = grid.query(&(55.0, 55.0, 1.0, 1.0)).collect();
        assert!(hits.contains(&0));
    }

    #[test]
    fn edge_label_anchor_midpoint() {
        let edge = EdgeLayout {
            from: "A".into(),
            to: "B".into(),
            points: vec![(0.0, 0.0), (100.0, 0.0)],
            label: None,
            start_label: None,
            end_label: None,
            label_anchor: None,
            start_label_anchor: None,
            end_label_anchor: None,
            directed: true,
            arrow_end: true,
            arrow_start: false,
            arrow_end_kind: None,
            arrow_start_kind: None,
            end_decoration: None,
            start_decoration: None,
            style: crate::ir::EdgeStyle::Solid,
            override_style: crate::ir::EdgeStyleOverride::default(),
        };
        let (x, y, _dx, _dy) = edge_label_anchor(&edge);
        assert!(
            (x - 50.0).abs() < 1.0,
            "midpoint x should be ~50, got {}",
            x
        );
        assert!((y - 0.0).abs() < 1.0, "midpoint y should be ~0, got {}", y);
    }

    #[test]
    fn edge_label_anchor_from_point_uses_nearest_segment() {
        let edge = EdgeLayout {
            from: "A".into(),
            to: "B".into(),
            points: vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0)],
            label: None,
            start_label: None,
            end_label: None,
            label_anchor: None,
            start_label_anchor: None,
            end_label_anchor: None,
            directed: true,
            arrow_end: true,
            arrow_start: false,
            arrow_end_kind: None,
            arrow_start_kind: None,
            end_decoration: None,
            start_decoration: None,
            style: crate::ir::EdgeStyle::Solid,
            override_style: crate::ir::EdgeStyleOverride::default(),
        };
        let (_x, _y, dx, dy) =
            edge_label_anchor_from_point(&edge, (100.0, 60.0)).expect("anchor should resolve");
        assert!(
            dx.abs() < 0.1,
            "dx should be ~0 for vertical segment, got {}",
            dx
        );
        assert!(
            dy > 0.9,
            "dy should be positive for vertical segment, got {}",
            dy
        );
    }

    #[test]
    fn class_endpoint_label_anchor_stays_near_endpoint() {
        let edge = EdgeLayout {
            from: "A".into(),
            to: "B".into(),
            label: None,
            start_label: Some(crate::layout::TextBlock {
                lines: vec!["1".into()],
                width: 12.0,
                height: 16.0,
            }),
            end_label: None,
            label_anchor: None,
            start_label_anchor: None,
            end_label_anchor: None,
            points: vec![(0.0, 0.0), (0.0, 120.0)],
            directed: true,
            arrow_start: false,
            arrow_end: true,
            arrow_start_kind: None,
            arrow_end_kind: None,
            start_decoration: None,
            end_decoration: None,
            style: crate::ir::EdgeStyle::Solid,
            override_style: crate::ir::EdgeStyleOverride::default(),
        };
        let occupied: Vec<Rect> = Vec::new();
        let occupied_grid = ObstacleGrid::new(48.0, &occupied);
        let edge_obstacles: Vec<EdgeObstacle> = Vec::new();
        let edge_grid = ObstacleGrid::new(48.0, &[]);
        let offset = 3.8;
        let ctx = EndpointLabelAvoidContext {
            kind: DiagramKind::Class,
            offset,
            occupied: &occupied,
            occupied_grid: &occupied_grid,
            node_obstacle_count: 0,
            edge_obstacles: &edge_obstacles,
            edge_grid: &edge_grid,
            bounds: None,
        };
        let pos =
            edge_endpoint_label_position_with_avoid(&edge, 0, true, 12.0, 16.0, 3.2, 1.6, &ctx)
                .expect("expected endpoint anchor");
        assert!(
            pos.1 <= offset * 1.4 + 0.25,
            "expected class endpoint label near endpoint, got y={}",
            pos.1
        );
    }

    #[test]
    fn edge_label_bundle_fractions_spread_parallel_edges() {
        let mk_edge = |to: &str| EdgeLayout {
            from: "S".into(),
            to: to.into(),
            points: vec![(0.0, 0.0), (100.0, 0.0)],
            label: Some(crate::layout::TextBlock {
                lines: vec!["x".into()],
                width: 8.0,
                height: 8.0,
            }),
            start_label: None,
            end_label: None,
            label_anchor: None,
            start_label_anchor: None,
            end_label_anchor: None,
            directed: true,
            arrow_end: true,
            arrow_start: false,
            arrow_end_kind: None,
            arrow_start_kind: None,
            end_decoration: None,
            start_decoration: None,
            style: crate::ir::EdgeStyle::Solid,
            override_style: crate::ir::EdgeStyleOverride::default(),
        };
        let mut edges = vec![mk_edge("G"), mk_edge("G"), mk_edge("G"), mk_edge("X")];
        edges[3].label = None;
        let fractions = edge_label_bundle_fractions(&edges);
        let f0 = fractions[0].expect("first parallel edge fraction");
        let f1 = fractions[1].expect("second parallel edge fraction");
        let f2 = fractions[2].expect("third parallel edge fraction");
        assert!(
            f0 < f1 && f1 < f2,
            "fractions should be strictly increasing"
        );
        assert!(
            fractions[3].is_none(),
            "non-labeled edge should not get a fraction"
        );
    }

    #[test]
    fn flowchart_refine_cost_penalizes_along_edge_drift() {
        let entry = FlowchartCenterLabelEntry {
            edge_idx: 0,
            label_w: 18.0,
            label_h: 10.0,
            initial_center: (50.0, 12.0),
            initial_s_norm: 0.5,
            initial_d_signed: 12.0,
            current_center: (50.0, 12.0),
            edge_points: vec![(0.0, 0.0), (100.0, 0.0)],
            candidates: Vec::new(),
        };
        let edge_obstacles: Vec<EdgeObstacle> = Vec::new();
        let edge_rects: Vec<Rect> = Vec::new();
        let edge_grid = ObstacleGrid::new(20.0, &edge_rects);
        let fixed: Vec<Rect> = Vec::new();

        let near = flowchart_center_label_refine_cost(
            &entry,
            (54.0, 12.0),
            2.0,
            1.5,
            &[],
            &fixed,
            &edge_obstacles,
            &edge_grid,
        );
        let far = flowchart_center_label_refine_cost(
            &entry,
            (90.0, 12.0),
            2.0,
            1.5,
            &[],
            &fixed,
            &edge_obstacles,
            &edge_grid,
        );

        assert!(
            far.0 > near.0,
            "moving far along edge direction should cost more (near={}, far={})",
            near.0,
            far.0
        );
    }
}
