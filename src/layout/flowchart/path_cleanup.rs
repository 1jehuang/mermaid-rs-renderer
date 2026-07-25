use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::config::LayoutConfig;
use crate::ir::Graph;

use super::super::NodeLayout;
use super::super::geometry::{
    endpoint_side_for_point, point_inside_node_shape_strict, segment_hits_node_shape_interior,
    segment_intrudes_endpoint_rect, source_exits_outward, target_enters_from_outside,
};
use super::super::routing::{
    Obstacle, Segment, collinear_overlap_length, compress_path, edge_crossings_with_existing,
    path_bend_count, path_length, segment_intersects_rect,
};
use super::super::types::SubgraphLayout;

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

pub(in crate::layout) fn reduce_orthogonal_path_crossings(
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
        crate::ir::DiagramKind::Flowchart
            | crate::ir::DiagramKind::Er
            | crate::ir::DiagramKind::State
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

pub(in crate::layout) fn collapse_axis_aligned_flowchart_handoffs(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
    routed_points: &mut [Vec<(f32, f32)>],
) {
    let mut pair_counts: HashMap<(String, String), usize> = HashMap::new();
    for edge in &graph.edges {
        *pair_counts
            .entry((edge.from.clone(), edge.to.clone()))
            .or_insert(0) += 1;
    }

    for (idx, points) in routed_points.iter_mut().enumerate() {
        if points.len() < 3 {
            continue;
        }
        let Some(edge) = graph.edges.get(idx) else {
            continue;
        };
        if edge.from == edge.to
            || edge.label.is_some()
            || edge.start_label.is_some()
            || edge.end_label.is_some()
            || pair_counts
                .get(&(edge.from.clone(), edge.to.clone()))
                .copied()
                .unwrap_or(0)
                > 1
        {
            continue;
        }
        let (Some(from), Some(to)) = (nodes.get(&edge.from), nodes.get(&edge.to)) else {
            continue;
        };
        if from.hidden
            || to.hidden
            || from.anchor_subgraph.is_some()
            || to.anchor_subgraph.is_some()
        {
            continue;
        }
        let from_center = (from.x + from.width / 2.0, from.y + from.height / 2.0);
        let to_center = (to.x + to.width / 2.0, to.y + to.height / 2.0);
        let candidate = if (from_center.0 - to_center.0).abs() <= 1.0 {
            let x = (from_center.0 + to_center.0) * 0.5;
            let from_y = if to_center.1 >= from_center.1 {
                from.y + from.height
            } else {
                from.y
            };
            let to_y = if to_center.1 >= from_center.1 {
                to.y
            } else {
                to.y + to.height
            };
            Some(vec![(x, from_y), (x, to_y)])
        } else if (from_center.1 - to_center.1).abs() <= 1.0 {
            let y = (from_center.1 + to_center.1) * 0.5;
            let from_x = if to_center.0 >= from_center.0 {
                from.x + from.width
            } else {
                from.x
            };
            let to_x = if to_center.0 >= from_center.0 {
                to.x
            } else {
                to.x + to.width
            };
            Some(vec![(from_x, y), (to_x, y)])
        } else {
            None
        };
        let Some(candidate) = candidate else {
            continue;
        };
        if path_length(&candidate) + 1.0 >= path_length(points) {
            continue;
        }
        if flowchart_path_hits_non_endpoint_nodes(&candidate, &edge.from, &edge.to, nodes) {
            continue;
        }
        let baseline_subgraph_hits =
            flowchart_path_foreign_subgraph_hit_count(points, &edge.from, &edge.to, subgraphs);
        let candidate_subgraph_hits =
            flowchart_path_foreign_subgraph_hit_count(&candidate, &edge.from, &edge.to, subgraphs);
        if candidate_subgraph_hits > baseline_subgraph_hits {
            continue;
        }
        *points = candidate;
    }
}

pub(in crate::layout) fn repair_flowchart_orthogonal_crossings(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
    routed_points: &mut [Vec<(f32, f32)>],
    config: &LayoutConfig,
) {
    if graph.edges.len() < 2 {
        return;
    }
    let margin = (config.node_spacing * 0.24).clamp(10.0, 24.0);

    // Each accepted candidate strictly reduces this route's crossings against
    // the current set, so let the pass drain dense graphs instead of stopping
    // after an arbitrary four repairs. Keep a defensive cap for pathological
    // inputs while scaling the budget with the number of edges.
    let repair_budget = graph.edges.len().saturating_mul(2).clamp(4, 64);
    for _ in 0..repair_budget {
        let mut changed = false;
        'pairs: for fixed_idx in 0..routed_points.len() {
            for repair_idx in 0..routed_points.len() {
                if fixed_idx == repair_idx || routed_points[repair_idx].len() < 2 {
                    continue;
                }
                let Some(edge) = graph.edges.get(repair_idx) else {
                    continue;
                };
                if edge.from == edge.to {
                    continue;
                }
                for fixed_segment in routed_points[fixed_idx].windows(2) {
                    for seg_idx in 0..routed_points[repair_idx].len().saturating_sub(1) {
                        let a = routed_points[repair_idx][seg_idx];
                        let b = routed_points[repair_idx][seg_idx + 1];
                        let Some(crossing) =
                            orthogonal_crossing(a, b, fixed_segment[0], fixed_segment[1])
                        else {
                            continue;
                        };
                        if let Some(candidate) = best_crossing_notch_candidate(
                            graph,
                            nodes,
                            subgraphs,
                            routed_points,
                            repair_idx,
                            seg_idx,
                            crossing,
                            (fixed_segment[0], fixed_segment[1]),
                            margin,
                        ) {
                            routed_points[repair_idx] = candidate;
                            changed = true;
                            break 'pairs;
                        }
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }

    // Dense orthogonal layouts can reach a local minimum where removing one
    // specific pairwise crossing merely exchanges it for different crossings.
    // Make one deterministic pair-priority repair per edge. Prefer candidates
    // that do not increase the global count, but let a substantially longer
    // edge take a tightly bounded tradeoff to preserve a short interior edge.
    // Consider local notches before outer lanes to avoid graph-wide detours.
    let mut pair_priority_repaired = HashSet::new();
    for fixed_idx in 0..routed_points.len() {
        for repair_idx in 0..routed_points.len() {
            if fixed_idx == repair_idx
                || pair_priority_repaired.contains(&repair_idx)
                || routed_points[repair_idx].len() < 4
            {
                continue;
            }
            if let Some(candidate) = best_pair_priority_crossing_candidate(
                graph,
                nodes,
                subgraphs,
                routed_points,
                fixed_idx,
                repair_idx,
                margin,
            ) {
                routed_points[repair_idx] = candidate;
                pair_priority_repaired.insert(repair_idx);
            }
        }
    }
}

fn best_pair_priority_crossing_candidate(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
    routed_points: &[Vec<(f32, f32)>],
    fixed_idx: usize,
    repair_idx: usize,
    margin: f32,
) -> Option<Vec<(f32, f32)>> {
    const LONG_EDGE_RATIO: f32 = 1.5;
    const MAX_CROSSING_TRADEOFF: usize = 2;

    let edge = graph.edges.get(repair_idx)?;
    if edge.from == edge.to {
        return None;
    }
    let fixed_points = routed_points.get(fixed_idx)?;
    let repair_points = routed_points.get(repair_idx)?;
    let mut fixed_segments = Vec::new();
    append_path_segments(fixed_points, &mut fixed_segments);
    let baseline_pair_crossings = edge_crossings_with_existing(repair_points, &fixed_segments).0;
    if baseline_pair_crossings == 0 {
        return None;
    }

    let mut other_segments = Vec::new();
    for (idx, points) in routed_points.iter().enumerate() {
        if idx != repair_idx {
            append_path_segments(points, &mut other_segments);
        }
    }
    let baseline_crossings = edge_crossings_with_existing(repair_points, &other_segments).0;
    let baseline_subgraph_hits =
        flowchart_path_foreign_subgraph_hit_count(repair_points, &edge.from, &edge.to, subgraphs);
    let baseline_len = path_length(repair_points);
    let fixed_len = path_length(fixed_points);
    let mut candidates = Vec::new();
    for fixed_segment in fixed_points.windows(2) {
        for (seg_idx, segment) in repair_points.windows(2).enumerate() {
            let Some(crossing) =
                orthogonal_crossing(segment[0], segment[1], fixed_segment[0], fixed_segment[1])
            else {
                continue;
            };
            candidates.extend(crossing_notch_candidates(
                repair_points,
                seg_idx,
                crossing,
                (fixed_segment[0], fixed_segment[1]),
                margin,
            ));
        }
    }
    candidates.extend(outer_crossing_detour_candidates(
        repair_points,
        nodes,
        margin,
    ));

    let mut best: Option<(usize, f32, Vec<(f32, f32)>)> = None;
    for raw_candidate in candidates {
        for candidate in crossing_candidate_clearance_variants(&raw_candidate, edge, nodes, margin)
        {
            if flowchart_endpoint_reentry_count(&candidate, edge, nodes) > 0
                || flowchart_endpoint_direction_violation_count(&candidate, edge, nodes) > 0
                || flowchart_path_foreign_subgraph_hit_count(
                    &candidate, &edge.from, &edge.to, subgraphs,
                ) > baseline_subgraph_hits
            {
                continue;
            }
            let candidate_len = path_length(&candidate);
            if candidate_len > baseline_len * 2.4 {
                continue;
            }
            let pair_crossings = edge_crossings_with_existing(&candidate, &fixed_segments).0;
            if pair_crossings >= baseline_pair_crossings {
                continue;
            }
            let candidate_crossings = edge_crossings_with_existing(&candidate, &other_segments).0;
            let bounded_long_edge_tradeoff = baseline_len >= fixed_len * LONG_EDGE_RATIO
                && candidate_len <= baseline_len * LONG_EDGE_RATIO
                && candidate_crossings <= baseline_crossings.saturating_add(MAX_CROSSING_TRADEOFF);
            if candidate_crossings > baseline_crossings && !bounded_long_edge_tradeoff {
                continue;
            }
            let replace = best.as_ref().is_none_or(|(best_crossings, best_len, _)| {
                candidate_crossings < *best_crossings
                    || (candidate_crossings == *best_crossings && candidate_len < *best_len)
            });
            if replace {
                best = Some((candidate_crossings, candidate_len, candidate));
            }
        }
    }
    best.map(|(_, _, candidate)| candidate)
}

fn orthogonal_crossing(
    a1: (f32, f32),
    a2: (f32, f32),
    b1: (f32, f32),
    b2: (f32, f32),
) -> Option<(f32, f32)> {
    const EPS: f32 = 1e-3;
    let a_vertical = (a1.0 - a2.0).abs() <= EPS;
    let a_horizontal = (a1.1 - a2.1).abs() <= EPS;
    let b_vertical = (b1.0 - b2.0).abs() <= EPS;
    let b_horizontal = (b1.1 - b2.1).abs() <= EPS;

    let strictly_between =
        |value: f32, p: f32, q: f32| value > p.min(q) + EPS && value < p.max(q) - EPS;

    if a_vertical && b_horizontal {
        let x = a1.0;
        let y = b1.1;
        if strictly_between(x, b1.0, b2.0) && strictly_between(y, a1.1, a2.1) {
            return Some((x, y));
        }
    } else if a_horizontal && b_vertical {
        let x = b1.0;
        let y = a1.1;
        if strictly_between(x, a1.0, a2.0) && strictly_between(y, b1.1, b2.1) {
            return Some((x, y));
        }
    }
    None
}

fn best_crossing_notch_candidate(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
    routed_points: &[Vec<(f32, f32)>],
    repair_idx: usize,
    seg_idx: usize,
    crossing: (f32, f32),
    fixed_segment: Segment,
    margin: f32,
) -> Option<Vec<(f32, f32)>> {
    let edge = graph.edges.get(repair_idx)?;
    let mut other_segments = Vec::new();
    for (idx, points) in routed_points.iter().enumerate() {
        if idx == repair_idx {
            continue;
        }
        append_path_segments(points, &mut other_segments);
    }
    let baseline_crossings =
        edge_crossings_with_existing(&routed_points[repair_idx], &other_segments).0;
    if baseline_crossings == 0 {
        return None;
    }

    let baseline_len = path_length(&routed_points[repair_idx]);
    let baseline_subgraph_hits = flowchart_path_foreign_subgraph_hit_count(
        &routed_points[repair_idx],
        &edge.from,
        &edge.to,
        subgraphs,
    );
    let mut best: Option<(usize, f32, Vec<(f32, f32)>)> = None;
    let mut candidates = crossing_notch_candidates(
        &routed_points[repair_idx],
        seg_idx,
        crossing,
        fixed_segment,
        margin,
    );
    candidates.extend(outer_crossing_detour_candidates(
        &routed_points[repair_idx],
        nodes,
        margin,
    ));
    for raw_candidate in candidates {
        for candidate in crossing_candidate_clearance_variants(&raw_candidate, edge, nodes, margin)
        {
            if flowchart_endpoint_reentry_count(&candidate, edge, nodes) > 0
                || flowchart_endpoint_direction_violation_count(&candidate, edge, nodes) > 0
                || flowchart_path_foreign_subgraph_hit_count(
                    &candidate, &edge.from, &edge.to, subgraphs,
                ) > baseline_subgraph_hits
            {
                continue;
            }
            let candidate_len = path_length(&candidate);
            if candidate_len > baseline_len * 2.4 {
                continue;
            }
            let candidate_crossings = edge_crossings_with_existing(&candidate, &other_segments).0;
            if candidate_crossings >= baseline_crossings {
                continue;
            }
            let replace = best.as_ref().is_none_or(|(best_crossings, best_len, _)| {
                candidate_crossings < *best_crossings
                    || (candidate_crossings == *best_crossings && candidate_len < *best_len)
            });
            if replace {
                best = Some((candidate_crossings, candidate_len, candidate));
            }
        }
    }
    best.map(|(_, _, candidate)| candidate)
}

fn crossing_candidate_clearance_variants(
    candidate: &[(f32, f32)],
    edge: &crate::ir::Edge,
    nodes: &BTreeMap<String, NodeLayout>,
    clearance: f32,
) -> Vec<Vec<(f32, f32)>> {
    let Some((first_seg_idx, last_seg_idx, obstacle)) =
        first_non_endpoint_node_hit(candidate, &edge.from, &edge.to, nodes)
    else {
        return vec![candidate.to_vec()];
    };

    let mut variants =
        node_detour_candidates(candidate, first_seg_idx, last_seg_idx, &obstacle, clearance);
    if first_seg_idx == last_seg_idx {
        let a = candidate[first_seg_idx];
        let b = candidate[first_seg_idx + 1];
        if (a.1 - b.1).abs() <= 1e-3 {
            for y in [
                obstacle.y - clearance,
                obstacle.y + obstacle.height + clearance,
            ] {
                if let Some(variant) = bump_orthogonal_segment(candidate, first_seg_idx, y - a.1) {
                    variants.push(variant);
                }
            }
        } else if (a.0 - b.0).abs() <= 1e-3 {
            for x in [
                obstacle.x - clearance,
                obstacle.x + obstacle.width + clearance,
            ] {
                if let Some(variant) = bump_orthogonal_segment(candidate, first_seg_idx, x - a.0) {
                    variants.push(variant);
                }
            }
        }
    }
    variants.retain(|variant| {
        !flowchart_path_hits_non_endpoint_nodes(variant, &edge.from, &edge.to, nodes)
    });
    variants
}

fn crossing_notch_candidates(
    points: &[(f32, f32)],
    seg_idx: usize,
    crossing: (f32, f32),
    fixed_segment: Segment,
    margin: f32,
) -> Vec<Vec<(f32, f32)>> {
    if seg_idx + 1 >= points.len() {
        return Vec::new();
    }
    let a = points[seg_idx];
    let b = points[seg_idx + 1];
    let horizontal = (a.1 - b.1).abs() <= 1e-3;
    let vertical = (a.0 - b.0).abs() <= 1e-3;
    if !horizontal && !vertical {
        return Vec::new();
    }

    let mut out = Vec::new();
    if horizontal {
        let dir = if b.0 >= a.0 { 1.0 } else { -1.0 };
        let before = crossing.0 - dir * margin;
        let after = crossing.0 + dir * margin;
        let fixed_min_y = fixed_segment.0.1.min(fixed_segment.1.1);
        let fixed_max_y = fixed_segment.0.1.max(fixed_segment.1.1);
        if seg_idx > 0 {
            let prev = points[seg_idx - 1];
            let prev_is_vertical = (prev.0 - a.0).abs() <= 1e-3;
            let prev_lane_is_clear = prev.1 < fixed_min_y - 1e-3 || prev.1 > fixed_max_y + 1e-3;
            if prev_is_vertical && prev_lane_is_clear {
                let mut candidate = Vec::with_capacity(points.len());
                candidate.extend_from_slice(&points[..seg_idx]);
                candidate.push((b.0, prev.1));
                candidate.extend_from_slice(&points[(seg_idx + 1)..]);
                out.push(compress_path(&candidate));
            }
        }
        if seg_idx + 2 < points.len() {
            let next = points[seg_idx + 2];
            let next_is_vertical = (next.0 - b.0).abs() <= 1e-3;
            let next_lane_is_clear = next.1 < fixed_min_y - 1e-3 || next.1 > fixed_max_y + 1e-3;
            if next_is_vertical && next_lane_is_clear {
                let mut candidate = Vec::with_capacity(points.len());
                candidate.extend_from_slice(&points[..=seg_idx]);
                candidate.push((a.0, next.1));
                candidate.extend_from_slice(&points[(seg_idx + 2)..]);
                out.push(compress_path(&candidate));
            }
        }
        for detour_y in [fixed_min_y - 1.0, fixed_max_y + 1.0] {
            if let Some(candidate) = bump_orthogonal_segment(points, seg_idx, detour_y - a.1) {
                out.push(candidate);
            }
        }
        for detour_y in [fixed_min_y - margin, fixed_max_y + margin] {
            if let Some(candidate) = bump_orthogonal_segment(points, seg_idx, detour_y - a.1) {
                out.push(candidate);
            }
            let mut candidate = Vec::with_capacity(points.len() + 4);
            candidate.extend_from_slice(&points[..=seg_idx]);
            candidate.push((before, a.1));
            candidate.push((before, detour_y));
            candidate.push((after, detour_y));
            candidate.push((after, a.1));
            candidate.extend_from_slice(&points[(seg_idx + 1)..]);
            out.push(compress_path(&candidate));
        }
    } else if vertical {
        let dir = if b.1 >= a.1 { 1.0 } else { -1.0 };
        let before = crossing.1 - dir * margin;
        let after = crossing.1 + dir * margin;
        let fixed_min_x = fixed_segment.0.0.min(fixed_segment.1.0);
        let fixed_max_x = fixed_segment.0.0.max(fixed_segment.1.0);
        if seg_idx > 0 {
            let prev = points[seg_idx - 1];
            let prev_is_horizontal = (prev.1 - a.1).abs() <= 1e-3;
            let prev_lane_is_clear = prev.0 < fixed_min_x - 1e-3 || prev.0 > fixed_max_x + 1e-3;
            if prev_is_horizontal && prev_lane_is_clear {
                let mut candidate = Vec::with_capacity(points.len());
                candidate.extend_from_slice(&points[..seg_idx]);
                candidate.push((prev.0, b.1));
                candidate.extend_from_slice(&points[(seg_idx + 1)..]);
                out.push(compress_path(&candidate));
            }
        }
        if seg_idx + 2 < points.len() {
            let next = points[seg_idx + 2];
            let next_is_horizontal = (next.1 - b.1).abs() <= 1e-3;
            let next_lane_is_clear = next.0 < fixed_min_x - 1e-3 || next.0 > fixed_max_x + 1e-3;
            if next_is_horizontal && next_lane_is_clear {
                let mut candidate = Vec::with_capacity(points.len());
                candidate.extend_from_slice(&points[..=seg_idx]);
                candidate.push((next.0, a.1));
                candidate.extend_from_slice(&points[(seg_idx + 2)..]);
                out.push(compress_path(&candidate));
            }
        }
        for detour_x in [fixed_min_x - 1.0, fixed_max_x + 1.0] {
            if let Some(candidate) = bump_orthogonal_segment(points, seg_idx, detour_x - a.0) {
                out.push(candidate);
            }
        }
        for detour_x in [fixed_min_x - margin, fixed_max_x + margin] {
            if let Some(candidate) = bump_orthogonal_segment(points, seg_idx, detour_x - a.0) {
                out.push(candidate);
            }
            let mut candidate = Vec::with_capacity(points.len() + 4);
            candidate.extend_from_slice(&points[..=seg_idx]);
            candidate.push((a.0, before));
            candidate.push((detour_x, before));
            candidate.push((detour_x, after));
            candidate.push((a.0, after));
            candidate.extend_from_slice(&points[(seg_idx + 1)..]);
            out.push(compress_path(&candidate));
        }
    }
    out
}

fn outer_crossing_detour_candidates(
    points: &[(f32, f32)],
    nodes: &BTreeMap<String, NodeLayout>,
    margin: f32,
) -> Vec<Vec<(f32, f32)>> {
    if points.len() < 4 {
        return Vec::new();
    }
    let visible = nodes
        .values()
        .filter(|node| !node.hidden && node.anchor_subgraph.is_none());
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for node in visible {
        min_x = min_x.min(node.x);
        max_x = max_x.max(node.x + node.width);
        min_y = min_y.min(node.y);
        max_y = max_y.max(node.y + node.height);
    }
    if !min_x.is_finite() || !max_x.is_finite() || !min_y.is_finite() || !max_y.is_finite() {
        return Vec::new();
    }

    let pad = margin.max(12.0) * 2.0;
    let first = points[0];
    let start_stub = points[1];
    let end_stub = points[points.len() - 2];
    let last = points[points.len() - 1];
    let top = min_y - pad;
    let bottom = max_y + pad;
    let left = min_x - pad;
    let right = max_x + pad;

    [
        vec![
            first,
            start_stub,
            (start_stub.0, top),
            (end_stub.0, top),
            end_stub,
            last,
        ],
        vec![
            first,
            start_stub,
            (start_stub.0, bottom),
            (end_stub.0, bottom),
            end_stub,
            last,
        ],
        vec![
            first,
            start_stub,
            (left, start_stub.1),
            (left, end_stub.1),
            end_stub,
            last,
        ],
        vec![
            first,
            start_stub,
            (right, start_stub.1),
            (right, end_stub.1),
            end_stub,
            last,
        ],
    ]
    .into_iter()
    .map(|candidate| compress_path(&candidate))
    .collect()
}

pub(in crate::layout) fn flowchart_path_hits_non_endpoint_nodes(
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

fn flowchart_path_non_endpoint_hit_count(
    path: &[(f32, f32)],
    from_id: &str,
    to_id: &str,
    nodes: &BTreeMap<String, NodeLayout>,
) -> usize {
    let mut hit_ids = std::collections::BTreeSet::new();
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
                hit_ids.insert(node.id.clone());
            }
        }
    }
    hit_ids.len()
}

fn first_non_endpoint_node_hit(
    path: &[(f32, f32)],
    from_id: &str,
    to_id: &str,
    nodes: &BTreeMap<String, NodeLayout>,
) -> Option<(usize, usize, Obstacle)> {
    for (seg_idx, segment) in path.windows(2).enumerate() {
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
                let mut merged = obstacle;
                let mut last_idx = seg_idx;
                for (later_idx, later_segment) in path.windows(2).enumerate().skip(seg_idx) {
                    let la = later_segment[0];
                    let lb = later_segment[1];
                    for other in nodes.values() {
                        if other.id == from_id
                            || other.id == to_id
                            || other.hidden
                            || other.anchor_subgraph.is_some()
                        {
                            continue;
                        }
                        let other_obstacle = Obstacle {
                            id: other.id.clone(),
                            x: other.x,
                            y: other.y,
                            width: other.width,
                            height: other.height,
                            members: None,
                        };
                        if segment_intersects_rect(la, lb, &other_obstacle) {
                            last_idx = later_idx;
                            merge_obstacle(&mut merged, &other_obstacle);
                        }
                    }
                }
                return Some((seg_idx, last_idx, merged));
            }
        }
    }
    None
}

fn merge_obstacle(target: &mut Obstacle, other: &Obstacle) {
    let min_x = target.x.min(other.x);
    let min_y = target.y.min(other.y);
    let max_x = (target.x + target.width).max(other.x + other.width);
    let max_y = (target.y + target.height).max(other.y + other.height);
    target.id.push('+');
    target.id.push_str(&other.id);
    target.x = min_x;
    target.y = min_y;
    target.width = max_x - min_x;
    target.height = max_y - min_y;
}

fn node_detour_candidates(
    path: &[(f32, f32)],
    first_seg_idx: usize,
    last_seg_idx: usize,
    obstacle: &Obstacle,
    clearance: f32,
) -> Vec<Vec<(f32, f32)>> {
    if first_seg_idx + 1 >= path.len() || last_seg_idx + 1 >= path.len() {
        return Vec::new();
    }
    let left = obstacle.x - clearance;
    let right = obstacle.x + obstacle.width + clearance;
    let top = obstacle.y - clearance;
    let bottom = obstacle.y + obstacle.height + clearance;
    let entry = path[first_seg_idx];
    let exit = path[last_seg_idx + 1];

    perimeter_route_candidates(entry, exit, left, right, top, bottom)
        .into_iter()
        .map(|route| {
            let mut candidate = Vec::with_capacity(path.len() + 2);
            candidate.extend_from_slice(&path[..=first_seg_idx]);
            if route.len() > 2 {
                candidate.extend_from_slice(&route[1..(route.len() - 1)]);
            }
            candidate.extend_from_slice(&path[(last_seg_idx + 1)..]);
            compress_path(&candidate)
        })
        .collect()
}

fn graph_detour_candidates(
    path: &[(f32, f32)],
    first_seg_idx: usize,
    last_seg_idx: usize,
    nodes: &BTreeMap<String, NodeLayout>,
    from_id: &str,
    to_id: &str,
    clearance: f32,
) -> Vec<Vec<(f32, f32)>> {
    if first_seg_idx + 1 >= path.len() || last_seg_idx + 1 >= path.len() {
        return Vec::new();
    }

    let mut left = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;
    let mut top = f32::INFINITY;
    let mut bottom = f32::NEG_INFINITY;
    for node in nodes.values() {
        if node.id == from_id || node.id == to_id || node.hidden || node.anchor_subgraph.is_some() {
            continue;
        }
        left = left.min(node.x);
        right = right.max(node.x + node.width);
        top = top.min(node.y);
        bottom = bottom.max(node.y + node.height);
    }
    if !left.is_finite() || !right.is_finite() || !top.is_finite() || !bottom.is_finite() {
        return Vec::new();
    }

    let entry = path[first_seg_idx];
    let exit = path[last_seg_idx + 1];
    perimeter_route_candidates(
        entry,
        exit,
        left - clearance,
        right + clearance,
        top - clearance,
        bottom + clearance,
    )
    .into_iter()
    .map(|route| {
        let mut candidate = Vec::with_capacity(path.len() + route.len());
        candidate.extend_from_slice(&path[..=first_seg_idx]);
        if route.len() > 2 {
            candidate.extend_from_slice(&route[1..route.len() - 1]);
        }
        candidate.extend_from_slice(&path[last_seg_idx + 1..]);
        compress_path(&candidate)
    })
    .collect()
}

fn graph_perimeter_detour_candidates(
    path: &[(f32, f32)],
    nodes: &BTreeMap<String, NodeLayout>,
    from_id: &str,
    to_id: &str,
    clearance: f32,
) -> Vec<Vec<(f32, f32)>> {
    if path.len() < 2 {
        return Vec::new();
    }

    let mut left = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;
    let mut top = f32::INFINITY;
    let mut bottom = f32::NEG_INFINITY;
    for node in nodes.values() {
        if node.id == from_id || node.id == to_id || node.hidden || node.anchor_subgraph.is_some() {
            continue;
        }
        left = left.min(node.x);
        right = right.max(node.x + node.width);
        top = top.min(node.y);
        bottom = bottom.max(node.y + node.height);
    }
    if !left.is_finite() || !right.is_finite() || !top.is_finite() || !bottom.is_finite() {
        return Vec::new();
    }

    let start = path[0];
    let end = *path.last().unwrap_or(&start);
    perimeter_route_candidates(
        start,
        end,
        left - clearance,
        right + clearance,
        top - clearance,
        bottom + clearance,
    )
    .into_iter()
    .map(|candidate| compress_path(&candidate))
    .collect()
}

pub(in crate::layout) fn detour_flowchart_paths_around_non_endpoint_nodes(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    routed_points: &mut [Vec<(f32, f32)>],
    config: &LayoutConfig,
) {
    let clearance = (config.node_spacing * 0.12).max(8.0);
    for (idx, points) in routed_points.iter_mut().enumerate() {
        let Some(edge) = graph.edges.get(idx) else {
            continue;
        };
        for _ in 0..8 {
            let Some((first_seg_idx, last_seg_idx, obstacle)) =
                first_non_endpoint_node_hit(points, &edge.from, &edge.to, nodes)
            else {
                break;
            };
            let mut best: Option<Vec<(f32, f32)>> = None;
            let mut best_cost = f32::INFINITY;
            let mut best_hits = usize::MAX;
            for clearance_scale in [1.0, 1.5, 2.0, 3.0, 4.0] {
                let candidate_clearance = clearance * clearance_scale;
                let candidates = node_detour_candidates(
                    points,
                    first_seg_idx,
                    last_seg_idx,
                    &obstacle,
                    candidate_clearance,
                )
                .into_iter()
                .chain(graph_detour_candidates(
                    points,
                    first_seg_idx,
                    last_seg_idx,
                    nodes,
                    &edge.from,
                    &edge.to,
                    candidate_clearance,
                ));
                for candidate in candidates {
                    let hits = flowchart_path_non_endpoint_hit_count(
                        &candidate, &edge.from, &edge.to, nodes,
                    );
                    let cost = path_length(&candidate)
                        + path_bend_count(&candidate) as f32 * candidate_clearance;
                    if hits < best_hits || (hits == best_hits && cost < best_cost) {
                        best_hits = hits;
                        best_cost = cost;
                        best = Some(candidate);
                    }
                }
            }
            if best.is_none() {
                for candidate in graph_perimeter_detour_candidates(
                    points,
                    nodes,
                    &edge.from,
                    &edge.to,
                    clearance * 2.0,
                ) {
                    if flowchart_path_hits_non_endpoint_nodes(
                        &candidate, &edge.from, &edge.to, nodes,
                    ) {
                        continue;
                    }
                    let cost =
                        path_length(&candidate) + path_bend_count(&candidate) as f32 * clearance;
                    if cost < best_cost {
                        best_cost = cost;
                        best = Some(candidate);
                    }
                }
            }
            let Some(candidate) = best else {
                break;
            };
            *points = candidate;
        }
    }
}

/// Subgraph rects an edge must stay out of: every subgraph that contains
/// neither endpoint of the edge (and is not an ancestor chain member of an
/// anchored endpoint).
fn foreign_subgraph_obstacles(
    edge_from: &str,
    edge_to: &str,
    subgraphs: &[SubgraphLayout],
    clearance: f32,
) -> Vec<Obstacle> {
    let mut obstacles = Vec::new();
    for sub in subgraphs {
        let members: HashSet<&str> = sub.nodes.iter().map(|s| s.as_str()).collect();
        if members.contains(edge_from) || members.contains(edge_to) {
            continue;
        }
        // Cluster-attached edges (composite states / subgraph anchors) use
        // the subgraph label as the endpoint id.
        if sub.label == edge_from || sub.label == edge_to {
            continue;
        }
        if sub.width <= 0.0 || sub.height <= 0.0 {
            continue;
        }
        obstacles.push(Obstacle {
            id: format!("subgraph:{}", sub.label),
            x: sub.x - clearance,
            y: sub.y - clearance,
            width: sub.width + clearance * 2.0,
            height: sub.height + clearance * 2.0,
            members: None,
        });
    }
    obstacles
}

fn path_foreign_subgraph_hits(path: &[(f32, f32)], obstacles: &[Obstacle]) -> usize {
    let mut hits = 0usize;
    for obstacle in obstacles {
        let mut hit = false;
        for segment in path.windows(2) {
            if segment_intersects_rect(segment[0], segment[1], obstacle) {
                hit = true;
                break;
            }
        }
        if hit {
            hits += 1;
        }
    }
    hits
}

pub(in crate::layout) fn flowchart_path_foreign_subgraph_hit_count(
    path: &[(f32, f32)],
    edge_from: &str,
    edge_to: &str,
    subgraphs: &[SubgraphLayout],
) -> usize {
    let obstacles = foreign_subgraph_obstacles(edge_from, edge_to, subgraphs, 0.0);
    path_foreign_subgraph_hits(path, &obstacles)
}

fn first_foreign_subgraph_hit(
    path: &[(f32, f32)],
    obstacles: &[Obstacle],
) -> Option<(usize, usize, Obstacle)> {
    for (seg_idx, segment) in path.windows(2).enumerate() {
        for obstacle in obstacles {
            if segment_intersects_rect(segment[0], segment[1], obstacle) {
                let mut last_idx = seg_idx;
                for (later_idx, later_segment) in path.windows(2).enumerate().skip(seg_idx) {
                    if segment_intersects_rect(later_segment[0], later_segment[1], obstacle) {
                        last_idx = later_idx;
                    }
                }
                return Some((seg_idx, last_idx, obstacle.clone()));
            }
        }
    }
    None
}

/// Reroute edges that cut through subgraph boxes they have no business in.
/// Runs after node-level detours; treats each foreign subgraph rect as a hard
/// obstacle and prefers detours that clear them without introducing new node
/// hits.
pub(in crate::layout) fn detour_flowchart_paths_around_foreign_subgraphs(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
    routed_points: &mut [Vec<(f32, f32)>],
    config: &LayoutConfig,
) {
    if subgraphs.is_empty() {
        return;
    }
    let clearance = (config.node_spacing * 0.12).max(8.0);
    for (idx, points) in routed_points.iter_mut().enumerate() {
        let Some(edge) = graph.edges.get(idx) else {
            continue;
        };
        if edge.from == edge.to {
            continue;
        }
        let obstacles = foreign_subgraph_obstacles(&edge.from, &edge.to, subgraphs, 0.0);
        if obstacles.is_empty() {
            continue;
        }
        for _ in 0..6 {
            let Some((first_seg_idx, last_seg_idx, obstacle)) =
                first_foreign_subgraph_hit(points, &obstacles)
            else {
                break;
            };
            let current_node_hits =
                flowchart_path_non_endpoint_hit_count(points, &edge.from, &edge.to, nodes);
            let current_sub_hits = path_foreign_subgraph_hits(points, &obstacles);
            let mut best: Option<Vec<(f32, f32)>> = None;
            let mut best_cost = f32::INFINITY;
            let mut best_hits = (current_sub_hits, current_node_hits);
            for clearance_scale in [1.0f32, 1.5, 2.0, 3.0] {
                let candidate_clearance = clearance * clearance_scale;
                for candidate in node_detour_candidates(
                    points,
                    first_seg_idx,
                    last_seg_idx,
                    &obstacle,
                    candidate_clearance,
                ) {
                    let sub_hits = path_foreign_subgraph_hits(&candidate, &obstacles);
                    let node_hits = flowchart_path_non_endpoint_hit_count(
                        &candidate, &edge.from, &edge.to, nodes,
                    );
                    let hits = (sub_hits, node_hits);
                    let cost = path_length(&candidate)
                        + path_bend_count(&candidate) as f32 * candidate_clearance;
                    if hits < best_hits || (hits == best_hits && cost < best_cost) {
                        best_hits = hits;
                        best_cost = cost;
                        best = Some(candidate);
                    }
                }
            }
            let Some(candidate) = best else {
                break;
            };
            *points = candidate;
        }
    }
}

fn endpoint_node_obstacle(node: &NodeLayout) -> Obstacle {
    Obstacle {
        id: node.id.clone(),
        x: node.x,
        y: node.y,
        width: node.width,
        height: node.height,
        members: None,
    }
}

fn endpoint_reentry_count(points: &[(f32, f32)], node: &NodeLayout, is_source: bool) -> usize {
    if points.len() < 3 {
        return 0;
    }
    let last_segment_idx = points.len().saturating_sub(2);
    points
        .windows(2)
        .enumerate()
        .filter(|(idx, segment)| {
            let allowed_endpoint_stub = if is_source {
                *idx == 0
            } else {
                *idx == last_segment_idx
            };
            !allowed_endpoint_stub && segment_hits_node_shape_interior(segment[0], segment[1], node)
        })
        .count()
}

pub(in crate::layout) fn flowchart_endpoint_reentry_count(
    points: &[(f32, f32)],
    edge: &crate::ir::Edge,
    nodes: &BTreeMap<String, NodeLayout>,
) -> usize {
    let mut count = 0usize;
    if let Some(from) = nodes.get(&edge.from) {
        count += endpoint_reentry_count(points, from, true);
    }
    if edge.to != edge.from
        && let Some(to) = nodes.get(&edge.to)
    {
        count += endpoint_reentry_count(points, to, false);
    }
    count
}

fn first_endpoint_reentry_span(
    points: &[(f32, f32)],
    node: &NodeLayout,
    is_source: bool,
) -> Option<(usize, usize)> {
    if points.len() < 3 {
        return None;
    }
    let last_segment_idx = points.len().saturating_sub(2);
    let mut idx = 0usize;
    while idx < last_segment_idx + 1 {
        let allowed_endpoint_stub = if is_source {
            idx == 0
        } else {
            idx == last_segment_idx
        };
        if !allowed_endpoint_stub
            && segment_hits_node_shape_interior(points[idx], points[idx + 1], node)
        {
            let first = idx;
            let mut last = idx;
            while last < last_segment_idx {
                let next_idx = last + 1;
                let next_allowed = if is_source {
                    next_idx == 0
                } else {
                    next_idx == last_segment_idx
                };
                // A short segment can start inside the endpoint yet have no sampled
                // interior point. Keep extending until the replacement endpoint is
                // known to be outside, otherwise every detour remains anchored inside
                // the node and cannot reduce the re-entry count.
                if next_allowed
                    || (!point_inside_node_shape_strict(node, points[next_idx])
                        && !segment_hits_node_shape_interior(
                            points[next_idx],
                            points[next_idx + 1],
                            node,
                        ))
                {
                    break;
                }
                last = next_idx;
            }
            return Some((first, last));
        }
        idx += 1;
    }
    None
}

pub(in crate::layout) fn flowchart_endpoint_direction_violation_count(
    points: &[(f32, f32)],
    edge: &crate::ir::Edge,
    nodes: &BTreeMap<String, NodeLayout>,
) -> usize {
    if points.len() < 2 {
        return 2;
    }
    let mut violations = 0usize;
    if let Some(from) = nodes.get(&edge.from) {
        let start = points[0];
        let next = points[1];
        let side = endpoint_side_for_point(from, start);
        if !source_exits_outward(side, start, next) {
            violations += 1;
        }
        if segment_intrudes_endpoint_rect(side, next, start, from) {
            violations += 1;
        }
    }
    if let Some(to) = nodes.get(&edge.to) {
        let end = points[points.len() - 1];
        let prev = points[points.len() - 2];
        let side = endpoint_side_for_point(to, end);
        if !target_enters_from_outside(side, prev, end) {
            violations += 1;
        }
        if segment_intrudes_endpoint_rect(side, prev, end, to) {
            violations += 1;
        }
    }
    violations
}

fn endpoint_reentry_detour_candidates(
    path: &[(f32, f32)],
    first_seg_idx: usize,
    last_seg_idx: usize,
    obstacle: &Obstacle,
    clearance: f32,
    is_source: bool,
) -> Vec<Vec<(f32, f32)>> {
    if first_seg_idx + 1 >= path.len() || last_seg_idx + 1 >= path.len() {
        return Vec::new();
    }

    // Endpoint doglegs often have the first offending segment starting on the
    // endpoint boundary or just inside the shape. Widen the replacement window so
    // the detour can start from the prior known-outside point. Preserve the first
    // source stub when the re-entry starts immediately after it.
    let start_idx = if first_seg_idx > 0 && (!is_source || first_seg_idx > 1) {
        first_seg_idx - 1
    } else {
        first_seg_idx
    };
    let end_idx = last_seg_idx + 1;
    if start_idx >= end_idx || end_idx >= path.len() {
        return Vec::new();
    }

    let left = obstacle.x - clearance;
    let right = obstacle.x + obstacle.width + clearance;
    let top = obstacle.y - clearance;
    let bottom = obstacle.y + obstacle.height + clearance;
    let entry = path[start_idx];
    let exit = path[end_idx];

    perimeter_route_candidates(entry, exit, left, right, top, bottom)
        .into_iter()
        .map(|route| {
            let mut candidate = Vec::with_capacity(path.len() + 2);
            candidate.extend_from_slice(&path[..=start_idx]);
            if route.len() > 2 {
                candidate.extend_from_slice(&route[1..(route.len() - 1)]);
            }
            candidate.extend_from_slice(&path[end_idx..]);
            compress_path(&candidate)
        })
        .collect()
}

fn repair_endpoint_reentry_once(
    points: &[(f32, f32)],
    edge: &crate::ir::Edge,
    nodes: &BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) -> Option<Vec<(f32, f32)>> {
    let baseline_reentries = flowchart_endpoint_reentry_count(points, edge, nodes);
    if baseline_reentries == 0
        || flowchart_endpoint_direction_violation_count(points, edge, nodes) > 0
    {
        return None;
    }
    let baseline_len = path_length(points);
    let clearance = (config.node_spacing * 0.12).max(8.0);
    let mut best: Option<Vec<(f32, f32)>> = None;
    let mut best_reentries = baseline_reentries;
    let mut best_cost = f32::INFINITY;

    let mut endpoint_specs: Vec<(&NodeLayout, bool)> = Vec::new();
    if let Some(from) = nodes.get(&edge.from) {
        endpoint_specs.push((from, true));
    }
    if edge.to != edge.from
        && let Some(to) = nodes.get(&edge.to)
    {
        endpoint_specs.push((to, false));
    }

    for (node, is_source) in endpoint_specs {
        let Some((first_seg_idx, last_seg_idx)) =
            first_endpoint_reentry_span(points, node, is_source)
        else {
            continue;
        };
        let obstacle = endpoint_node_obstacle(node);
        for clearance_scale in [1.0, 1.5, 2.0, 3.0, 4.0] {
            let candidate_clearance = clearance * clearance_scale;
            let candidates = node_detour_candidates(
                points,
                first_seg_idx,
                last_seg_idx,
                &obstacle,
                candidate_clearance,
            )
            .into_iter()
            .chain(endpoint_reentry_detour_candidates(
                points,
                first_seg_idx,
                last_seg_idx,
                &obstacle,
                candidate_clearance,
                is_source,
            ));
            for candidate in candidates {
                let violations =
                    flowchart_endpoint_direction_violation_count(&candidate, edge, nodes);
                let hits_non_endpoint =
                    flowchart_path_hits_non_endpoint_nodes(&candidate, &edge.from, &edge.to, nodes);
                let reentries = flowchart_endpoint_reentry_count(&candidate, edge, nodes);
                if violations > 0 {
                    continue;
                }
                if hits_non_endpoint {
                    continue;
                }
                if reentries >= best_reentries {
                    continue;
                }
                let len = path_length(&candidate);
                if len > baseline_len * 4.0 + clearance * 8.0 {
                    continue;
                }
                let bends = path_bend_count(&candidate);
                let cost = len + bends as f32 * clearance + reentries as f32 * clearance * 20.0;
                if reentries < best_reentries || cost < best_cost {
                    best_reentries = reentries;
                    best_cost = cost;
                    best = Some(candidate);
                }
            }
        }
    }

    (best_reentries < baseline_reentries).then_some(best?)
}

pub(in crate::layout) fn repair_flowchart_endpoint_reentries(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    routed_points: &mut [Vec<(f32, f32)>],
    config: &LayoutConfig,
) {
    for (idx, points) in routed_points.iter_mut().enumerate() {
        let Some(edge) = graph.edges.get(idx) else {
            continue;
        };
        for _ in 0..6 {
            let Some(candidate) = repair_endpoint_reentry_once(points, edge, nodes, config) else {
                break;
            };
            *points = candidate;
        }
    }
}

pub(in crate::layout) fn detour_flowchart_paths_wider_clearance(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    routed_points: &mut [Vec<(f32, f32)>],
    config: &LayoutConfig,
) {
    let base_clearance = (config.node_spacing * 0.12).max(8.0);
    for (idx, points) in routed_points.iter_mut().enumerate() {
        let Some(edge) = graph.edges.get(idx) else {
            continue;
        };
        let starting_hits = count_non_endpoint_node_hits(points, &edge.from, &edge.to, nodes);
        if starting_hits == 0 {
            continue;
        }
        for _ in 0..6 {
            let Some((first_seg_idx, last_seg_idx, obstacle)) =
                first_non_endpoint_node_hit(points, &edge.from, &edge.to, nodes)
            else {
                break;
            };
            let mut best: Option<Vec<(f32, f32)>> = None;
            let mut best_cost = f32::INFINITY;
            let mut best_hits = starting_hits;
            for clearance_scale in [1.0_f32, 1.5, 2.5, 4.0] {
                let clearance = base_clearance * clearance_scale;
                for candidate in node_detour_candidates(
                    points,
                    first_seg_idx,
                    last_seg_idx,
                    &obstacle,
                    clearance,
                ) {
                    let hits =
                        count_non_endpoint_node_hits(&candidate, &edge.from, &edge.to, nodes);
                    if hits > best_hits {
                        continue;
                    }
                    let cost =
                        path_length(&candidate) + path_bend_count(&candidate) as f32 * clearance;
                    if hits < best_hits || cost < best_cost {
                        best_cost = cost;
                        best_hits = hits;
                        best = Some(candidate);
                    }
                }
                if best_hits == 0 {
                    break;
                }
            }
            let Some(candidate) = best else {
                break;
            };
            *points = candidate;
        }
    }
}

fn count_non_endpoint_node_hits(
    path: &[(f32, f32)],
    from_id: &str,
    to_id: &str,
    nodes: &BTreeMap<String, NodeLayout>,
) -> usize {
    let mut count = 0;
    for node in nodes.values() {
        if node.id == from_id || node.id == to_id || node.hidden || node.anchor_subgraph.is_some() {
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
        if path
            .windows(2)
            .any(|seg| segment_intersects_rect(seg[0], seg[1], &obstacle))
        {
            count += 1;
        }
    }
    count
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

pub(in crate::layout) fn deoverlap_flowchart_paths(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    routed_points: &mut [Vec<(f32, f32)>],
    config: &LayoutConfig,
) {
    if graph.edges.len() < 2 {
        return;
    }
    let overlap_threshold = 0.68f32;
    let base_delta = (config.node_spacing * 0.25).max(8.0);
    let deltas = [
        base_delta,
        -base_delta,
        base_delta * 1.5,
        -base_delta * 1.5,
        base_delta * 2.0,
        -base_delta * 2.0,
        base_delta * 2.8,
        -base_delta * 2.8,
    ];
    let min_segment_len = (base_delta * 1.2).max(6.0);

    for _ in 0..4 {
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
            let mut segment_order: Vec<(usize, f32)> = routed_points[idx]
                .windows(2)
                .enumerate()
                .map(|(seg_idx, seg)| {
                    let dx = seg[1].0 - seg[0].0;
                    let dy = seg[1].1 - seg[0].1;
                    (seg_idx, (dx * dx + dy * dy).sqrt())
                })
                .collect();
            segment_order.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
            for (seg_idx, seg_len) in segment_order {
                if seg_len < min_segment_len {
                    continue;
                }
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
                    if overlap + 0.03 < best_overlap {
                        best_overlap = overlap;
                        best_points = candidate;
                    }
                }
            }
            if best_overlap + 0.03 < baseline {
                routed_points[idx] = best_points;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

fn is_axis_aligned_segment(a: (f32, f32), b: (f32, f32)) -> bool {
    (a.0 - b.0).abs() <= 1e-3 || (a.1 - b.1).abs() <= 1e-3
}

fn collapse_near_axis_aligned_path(points: &[(f32, f32)]) -> Option<Vec<(f32, f32)>> {
    if points.len() < 3 {
        return None;
    }

    let min_x = points.iter().map(|point| point.0).fold(f32::MAX, f32::min);
    let max_x = points.iter().map(|point| point.0).fold(f32::MIN, f32::max);
    let min_y = points.iter().map(|point| point.1).fold(f32::MAX, f32::min);
    let max_y = points.iter().map(|point| point.1).fold(f32::MIN, f32::max);
    let x_span = max_x - min_x;
    let y_span = max_y - min_y;
    let axis_epsilon = 1.0f32;
    let nearly_vertical = points
        .windows(2)
        .all(|segment| (segment[1].0 - segment[0].0).abs() <= axis_epsilon);
    let nearly_horizontal = points
        .windows(2)
        .all(|segment| (segment[1].1 - segment[0].1).abs() <= axis_epsilon);

    if x_span <= axis_epsilon && y_span > axis_epsilon && nearly_vertical {
        let x = (min_x + max_x) * 0.5;
        return Some(vec![(x, points[0].1), (x, points[points.len() - 1].1)]);
    }
    if y_span <= axis_epsilon && x_span > axis_epsilon && nearly_horizontal {
        let y = (min_y + max_y) * 0.5;
        return Some(vec![(points[0].0, y), (points[points.len() - 1].0, y)]);
    }

    None
}

fn collapse_axis_aligned_runs(points: &[(f32, f32)]) -> Vec<(f32, f32)> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    let mut collapsed = Vec::with_capacity(points.len());
    let mut idx = 0usize;
    collapsed.push(points[0]);

    while idx + 1 < points.len() {
        let current = points[idx];
        let next = points[idx + 1];
        let same_x = (next.0 - current.0).abs() <= 1e-3;
        let same_y = (next.1 - current.1).abs() <= 1e-3;

        if !same_x && !same_y {
            if (next.0 - collapsed[collapsed.len() - 1].0).abs() > 1e-3
                || (next.1 - collapsed[collapsed.len() - 1].1).abs() > 1e-3
            {
                collapsed.push(next);
            }
            idx += 1;
            continue;
        }

        let mut end_idx = idx + 1;
        while end_idx + 1 < points.len() {
            let candidate = points[end_idx + 1];
            let continues_run = if same_x {
                (candidate.0 - current.0).abs() <= 1e-3
            } else {
                (candidate.1 - current.1).abs() <= 1e-3
            };
            if !continues_run {
                break;
            }
            end_idx += 1;
        }

        let terminal = points[end_idx];
        if (terminal.0 - collapsed[collapsed.len() - 1].0).abs() > 1e-3
            || (terminal.1 - collapsed[collapsed.len() - 1].1).abs() > 1e-3
        {
            collapsed.push(terminal);
        }
        idx = end_idx;
    }

    compress_path(&collapsed)
}

pub(in crate::layout) fn simplify_flowchart_axis_oscillations(
    routed_points: &mut [Vec<(f32, f32)>],
) {
    for path in routed_points.iter_mut() {
        let collapsed = collapse_axis_aligned_runs(path);
        *path = collapse_near_axis_aligned_path(&collapsed).unwrap_or(collapsed);
    }
}

fn detour_rectangle_simplification_candidates(points: &[(f32, f32)]) -> Vec<Vec<(f32, f32)>> {
    if points.len() != 6 {
        return Vec::new();
    }
    if !points
        .windows(2)
        .all(|segment| is_axis_aligned_segment(segment[0], segment[1]))
    {
        return Vec::new();
    }
    let vertical_first = (points[0].0 - points[1].0).abs() <= 1e-3;
    let vertical_pattern = [
        vertical_first,
        !vertical_first,
        vertical_first,
        !vertical_first,
        vertical_first,
    ];
    for (idx, segment) in points.windows(2).enumerate() {
        let is_vertical = (segment[0].0 - segment[1].0).abs() <= 1e-3;
        if is_vertical != vertical_pattern[idx] {
            return Vec::new();
        }
    }

    let mut candidates = Vec::new();
    if vertical_first {
        for &cross_y in &[points[1].1, points[3].1] {
            candidates.push(compress_path(&[
                points[0],
                (points[0].0, cross_y),
                (points[5].0, cross_y),
                points[5],
            ]));
        }
    } else {
        for &cross_x in &[points[1].0, points[3].0] {
            candidates.push(compress_path(&[
                points[0],
                (cross_x, points[0].1),
                (cross_x, points[5].1),
                points[5],
            ]));
        }
    }
    candidates
}

fn shoulder_simplification_candidates(points: &[(f32, f32)]) -> Vec<Vec<(f32, f32)>> {
    if points.len() != 6 {
        return Vec::new();
    }
    let mut candidates = Vec::new();
    let vertical_pattern = points
        .windows(2)
        .map(|segment| (segment[0].0 - segment[1].0).abs() <= 1e-3)
        .collect::<Vec<_>>();
    if vertical_pattern == [true, false, true, false, true] {
        candidates.push(compress_path(&[
            points[0],
            points[1],
            points[2],
            (points[2].0, points[5].1),
            points[5],
        ]));
        candidates.push(compress_path(&[
            points[0],
            (points[0].0, points[3].1),
            points[3],
            points[4],
            points[5],
        ]));
    } else if vertical_pattern == [false, true, false, true, false] {
        candidates.push(compress_path(&[
            points[0],
            points[1],
            points[2],
            (points[5].0, points[2].1),
            points[5],
        ]));
        candidates.push(compress_path(&[
            points[0],
            (points[3].0, points[0].1),
            points[3],
            points[4],
            points[5],
        ]));
    }
    candidates
}

fn point_on_vertical_edge(point: (f32, f32), node: &NodeLayout) -> bool {
    let on_top = (point.1 - node.y).abs() <= 3.0;
    let on_bottom = (point.1 - (node.y + node.height)).abs() <= 3.0;
    (on_top || on_bottom) && point.0 >= node.x - 3.0 && point.0 <= node.x + node.width + 3.0
}

fn point_on_horizontal_edge(point: (f32, f32), node: &NodeLayout) -> bool {
    let on_left = (point.0 - node.x).abs() <= 3.0;
    let on_right = (point.0 - (node.x + node.width)).abs() <= 3.0;
    (on_left || on_right) && point.1 >= node.y - 3.0 && point.1 <= node.y + node.height + 3.0
}

fn spine_simplification_candidates(
    points: &[(f32, f32)],
    from: &NodeLayout,
    to: &NodeLayout,
) -> Vec<Vec<(f32, f32)>> {
    if points.len() < 4 {
        return Vec::new();
    }
    let from_center = (from.x + from.width * 0.5, from.y + from.height * 0.5);
    let to_center = (to.x + to.width * 0.5, to.y + to.height * 0.5);
    let dominant_vertical =
        (to_center.1 - from_center.1).abs() >= (to_center.0 - from_center.0).abs();
    let first_on_vertical = point_on_vertical_edge(points[0], from);
    let last_on_vertical = point_on_vertical_edge(points[points.len() - 1], to);
    let first_on_horizontal = point_on_horizontal_edge(points[0], from);
    let last_on_horizontal = point_on_horizontal_edge(points[points.len() - 1], to);
    let first_vertical = first_on_vertical || (!first_on_horizontal && dominant_vertical);
    let last_vertical = last_on_vertical || (!last_on_horizontal && dominant_vertical);
    let first_horizontal = first_on_horizontal || (!first_on_vertical && !dominant_vertical);
    let last_horizontal = last_on_horizontal || (!last_on_vertical && !dominant_vertical);
    let mut candidates = Vec::new();

    if first_vertical && last_vertical {
        let mut cross_levels: Vec<f32> = points[1..points.len() - 1].iter().map(|p| p.1).collect();
        cross_levels.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        cross_levels.dedup_by(|a, b| (*a - *b).abs() <= 1e-3);
        for cross_y in cross_levels {
            candidates.push(compress_path(&[
                points[0],
                (points[0].0, cross_y),
                (points[points.len() - 1].0, cross_y),
                points[points.len() - 1],
            ]));
        }
    } else if first_horizontal && last_horizontal {
        let mut cross_levels: Vec<f32> = points[1..points.len() - 1].iter().map(|p| p.0).collect();
        cross_levels.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        cross_levels.dedup_by(|a, b| (*a - *b).abs() <= 1e-3);
        for cross_x in cross_levels {
            candidates.push(compress_path(&[
                points[0],
                (cross_x, points[0].1),
                (cross_x, points[points.len() - 1].1),
                points[points.len() - 1],
            ]));
        }
    }

    candidates
}

pub(in crate::layout) fn simplify_flowchart_detour_rectangles(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    routed_points: &mut [Vec<(f32, f32)>],
) {
    if graph.edges.len() < 2 {
        return;
    }

    for idx in 0..routed_points.len() {
        let baseline = routed_points[idx].clone();
        let baseline_bends = path_bend_count(&baseline);
        if baseline_bends < 4 {
            continue;
        }

        let from_id = graph.edges[idx].from.as_str();
        let to_id = graph.edges[idx].to.as_str();
        let mut other_segments: Vec<Segment> = Vec::new();
        for (other_idx, path) in routed_points.iter().enumerate() {
            if other_idx == idx {
                continue;
            }
            append_path_segments(path, &mut other_segments);
        }
        let (baseline_cross, baseline_overlap) =
            edge_crossings_with_existing(&baseline, &other_segments);
        let baseline_len = path_length(&baseline);
        let mut best = baseline.clone();
        let mut best_bends = baseline_bends;
        let mut best_cross = baseline_cross;
        let mut best_overlap = baseline_overlap;
        let mut best_len = baseline_len;

        let Some(from) = nodes.get(from_id) else {
            continue;
        };
        let Some(to) = nodes.get(to_id) else {
            continue;
        };
        let mut candidates = detour_rectangle_simplification_candidates(&baseline);
        candidates.extend(shoulder_simplification_candidates(&baseline));
        candidates.extend(spine_simplification_candidates(&baseline, from, to));
        for candidate in candidates {
            if candidate.len() >= baseline.len() {
                continue;
            }
            if flowchart_path_hits_non_endpoint_nodes(&candidate, from_id, to_id, nodes) {
                continue;
            }
            let bends = path_bend_count(&candidate);
            if bends >= best_bends {
                continue;
            }
            let (crossings, overlap) = edge_crossings_with_existing(&candidate, &other_segments);
            let len = path_length(&candidate);
            let better = crossings < best_cross
                || (crossings == best_cross
                    && overlap <= best_overlap + 0.05
                    && bends < best_bends)
                || (crossings == best_cross
                    && (overlap - best_overlap).abs() <= 0.05
                    && bends == best_bends
                    && len + 1.0 < best_len);
            if better {
                best = candidate;
                best_bends = bends;
                best_cross = crossings;
                best_overlap = overlap;
                best_len = len;
            }
        }

        if best_bends < baseline_bends && best_cross <= baseline_cross {
            routed_points[idx] = best;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::config::LayoutConfig;
    use crate::ir::{Edge, EdgeStyle, Graph, NodeShape, NodeStyle};
    use crate::layout::routing::edge_crossings_with_existing;
    use crate::layout::{NodeLayout, SubgraphLayout, TextBlock};

    use super::{
        append_path_segments, best_pair_priority_crossing_candidate,
        collapse_axis_aligned_flowchart_handoffs, collapse_axis_aligned_runs,
        detour_flowchart_paths_around_non_endpoint_nodes,
        flowchart_endpoint_direction_violation_count, flowchart_endpoint_reentry_count,
        flowchart_path_foreign_subgraph_hit_count, flowchart_path_hits_non_endpoint_nodes,
        repair_flowchart_endpoint_reentries,
    };

    fn edge(from: &str, to: &str) -> Edge {
        Edge {
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
            style: EdgeStyle::Solid,
        }
    }

    fn node_layout(id: &str, x: f32, y: f32, width: f32, height: f32) -> NodeLayout {
        NodeLayout {
            id: id.to_string(),
            x,
            y,
            width,
            height,
            label: TextBlock {
                lines: vec![id.to_string()],
                width: 20.0,
                height: 12.0,
            },
            shape: NodeShape::Rectangle,
            style: NodeStyle::default(),
            link: None,
            anchor_subgraph: None,
            hidden: false,
            icon: None,
        }
    }

    fn subgraph_layout(label: &str, x: f32, y: f32, width: f32, height: f32) -> SubgraphLayout {
        SubgraphLayout {
            id: Some(label.to_string()),
            label: label.to_string(),
            label_block: TextBlock {
                lines: vec![label.to_string()],
                width: 20.0,
                height: 12.0,
            },
            nodes: vec![format!("{label}-member")],
            x,
            y,
            width,
            height,
            style: NodeStyle::default(),
            icon: None,
        }
    }

    #[test]
    fn endpoint_reentry_repair_expands_past_short_inside_self_loop_segment() {
        let mut graph = Graph::new();
        graph.edges = vec![edge("B", "B")];
        let mut nodes = BTreeMap::new();
        nodes.insert("B".to_string(), node_layout("B", 0.0, 10.0, 120.0, 50.0));
        let mut routed_points = vec![vec![
            (120.0, 35.0),
            (150.0, 35.0),
            (150.0, 12.9),
            (60.0, 12.9),
            (60.0, 0.4),
            (60.0, 10.0),
        ]];

        assert_eq!(
            flowchart_endpoint_direction_violation_count(
                &routed_points[0],
                &graph.edges[0],
                &nodes,
            ),
            0
        );
        assert_eq!(
            flowchart_endpoint_reentry_count(&routed_points[0], &graph.edges[0], &nodes),
            1,
            "the regression path must contain the sampled self-loop re-entry"
        );

        repair_flowchart_endpoint_reentries(
            &graph,
            &nodes,
            &mut routed_points,
            &LayoutConfig::default(),
        );

        assert_eq!(
            flowchart_endpoint_reentry_count(&routed_points[0], &graph.edges[0], &nodes),
            0,
            "repair endpoints must be outside the endpoint shape"
        );
        assert_eq!(
            flowchart_endpoint_direction_violation_count(
                &routed_points[0],
                &graph.edges[0],
                &nodes,
            ),
            0,
            "repair must preserve both self-loop port directions"
        );
    }

    #[test]
    fn long_crossing_route_can_take_bounded_local_detour_around_short_route() {
        let mut graph = Graph::new();
        graph.edges = vec![
            edge("fixed", "fixed-target"),
            edge("repair", "repair-target"),
            edge("top-a", "top-a-target"),
            edge("top-b", "top-b-target"),
            edge("bottom-a", "bottom-a-target"),
            edge("bottom-b", "bottom-b-target"),
        ];
        let routed_points = vec![
            vec![(10.0, -10.0), (10.0, 10.0)],
            vec![(0.0, -20.0), (0.0, 0.0), (20.0, 0.0), (20.0, 20.0)],
            vec![(5.0, -30.0), (5.0, -5.0)],
            vec![(7.0, -30.0), (7.0, -5.0)],
            vec![(13.0, 5.0), (13.0, 30.0)],
            vec![(15.0, 5.0), (15.0, 30.0)],
        ];
        let nodes = BTreeMap::new();
        let candidate =
            best_pair_priority_crossing_candidate(&graph, &nodes, &[], &routed_points, 0, 1, 10.0)
                .expect("the longer route should take a bounded detour around the short route");

        let mut fixed_segments = Vec::new();
        append_path_segments(&routed_points[0], &mut fixed_segments);
        assert_eq!(
            edge_crossings_with_existing(&candidate, &fixed_segments).0,
            0,
            "the neutral candidate should remove the selected pairwise crossing"
        );

        let mut other_segments = Vec::new();
        for (idx, points) in routed_points.iter().enumerate() {
            if idx != 1 {
                append_path_segments(points, &mut other_segments);
            }
        }
        let baseline_crossings = edge_crossings_with_existing(&routed_points[1], &other_segments).0;
        let candidate_crossings = edge_crossings_with_existing(&candidate, &other_segments).0;
        assert!(
            candidate_crossings > baseline_crossings,
            "the fixture should exercise the bounded long-edge tradeoff"
        );
        assert!(candidate_crossings <= baseline_crossings + 2);
    }

    #[test]
    fn crossing_repair_rejects_detours_through_foreign_subgraphs() {
        let mut graph = Graph::new();
        graph.edges = vec![
            edge("fixed", "fixed-target"),
            edge("repair", "repair-target"),
            edge("top-a", "top-a-target"),
            edge("top-b", "top-b-target"),
            edge("bottom-a", "bottom-a-target"),
            edge("bottom-b", "bottom-b-target"),
        ];
        let routed_points = vec![
            vec![(10.0, -10.0), (10.0, 10.0)],
            vec![(0.0, -20.0), (0.0, 0.0), (20.0, 0.0), (20.0, 20.0)],
            vec![(5.0, -30.0), (5.0, -5.0)],
            vec![(7.0, -30.0), (7.0, -5.0)],
            vec![(13.0, 5.0), (13.0, 30.0)],
            vec![(15.0, 5.0), (15.0, 30.0)],
        ];
        let subgraphs = vec![
            subgraph_layout("top-lane", 1.0, -21.0, 18.0, 12.0),
            subgraph_layout("bottom-lane", 1.0, 9.0, 18.0, 12.0),
        ];

        assert_eq!(
            flowchart_path_foreign_subgraph_hit_count(
                &routed_points[1],
                "repair",
                "repair-target",
                &subgraphs,
            ),
            0,
            "the baseline route must be containment-clean"
        );
        assert!(
            best_pair_priority_crossing_candidate(
                &graph,
                &BTreeMap::new(),
                &subgraphs,
                &routed_points,
                0,
                1,
                10.0,
            )
            .is_none(),
            "crossing repair must not trade a crossing for a foreign-subgraph intrusion"
        );
    }

    #[test]
    fn axis_aligned_handoff_collapse_preserves_foreign_subgraph_detour() {
        let mut graph = Graph::new();
        graph.edges = vec![edge("A", "B")];
        let mut nodes = BTreeMap::new();
        nodes.insert("A".to_string(), node_layout("A", 0.0, 0.0, 20.0, 20.0));
        nodes.insert("B".to_string(), node_layout("B", 100.0, 0.0, 20.0, 20.0));
        let subgraphs = vec![subgraph_layout("foreign", 40.0, 0.0, 40.0, 20.0)];
        let original = vec![(20.0, 10.0), (20.0, -20.0), (100.0, -20.0), (100.0, 10.0)];
        let mut routed_points = vec![original.clone()];

        collapse_axis_aligned_flowchart_handoffs(&graph, &nodes, &subgraphs, &mut routed_points);

        assert_eq!(
            routed_points[0], original,
            "a shorter direct handoff must not cut through an unrelated subgraph"
        );
    }

    #[test]
    fn non_endpoint_detour_repairs_diagonal_state_terminal_leg() {
        let mut graph = Graph::new();
        graph.edges = vec![edge("Published", "End")];
        let mut nodes = BTreeMap::new();
        nodes.insert(
            "Published".to_string(),
            node_layout("Published", 128.0, 326.0, 120.0, 47.0),
        );
        nodes.insert(
            "Archived".to_string(),
            node_layout("Archived", 133.0, 417.0, 111.0, 47.0),
        );
        nodes.insert(
            "End".to_string(),
            node_layout("End", 245.0, 440.0, 15.0, 15.0),
        );
        let mut routed_points = vec![vec![(220.0, 373.0), (220.0, 379.0), (250.0, 440.0)]];

        assert!(flowchart_path_hits_non_endpoint_nodes(
            &routed_points[0],
            "Published",
            "End",
            &nodes,
        ));
        detour_flowchart_paths_around_non_endpoint_nodes(
            &graph,
            &nodes,
            &mut routed_points,
            &LayoutConfig::default(),
        );
        assert!(
            !flowchart_path_hits_non_endpoint_nodes(&routed_points[0], "Published", "End", &nodes,),
            "state terminal legs must route around sibling state boxes"
        );
    }

    #[test]
    fn collapse_axis_aligned_runs_removes_redundant_backtracking() {
        let points = vec![
            (10.0, 10.0),
            (10.0, 20.0),
            (10.0, 32.0),
            (10.0, 24.0),
            (10.0, 32.0),
            (10.0, 24.0),
            (50.0, 24.0),
            (50.0, 18.0),
        ];

        let collapsed = collapse_axis_aligned_runs(&points);

        assert_eq!(
            collapsed,
            vec![(10.0, 10.0), (10.0, 24.0), (50.0, 24.0), (50.0, 18.0)]
        );
    }

    #[test]
    fn collapse_near_axis_aligned_path_reduces_vertical_jitter() {
        let points = vec![(20.0, 10.0), (20.0, 18.0), (20.4, 25.0), (20.2, 40.0)];

        let collapsed =
            super::collapse_near_axis_aligned_path(&points).expect("expected simplification");

        assert_eq!(collapsed.len(), 2);
        assert!((collapsed[0].0 - collapsed[1].0).abs() <= 1e-3);
        assert!((collapsed[0].1 - 10.0).abs() <= 1e-3);
        assert!((collapsed[1].1 - 40.0).abs() <= 1e-3);
    }
}
