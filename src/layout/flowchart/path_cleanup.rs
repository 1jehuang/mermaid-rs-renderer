use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::config::LayoutConfig;
use crate::ir::Graph;

use super::super::NodeLayout;
use super::super::routing::{
    Obstacle, Segment, collinear_overlap_length, compress_path, edge_crossings_with_existing,
    path_bend_count, path_length, segment_intersects_rect,
};

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
        for _ in 0..4 {
            let Some((first_seg_idx, last_seg_idx, obstacle)) =
                first_non_endpoint_node_hit(points, &edge.from, &edge.to, nodes)
            else {
                break;
            };
            let mut best: Option<Vec<(f32, f32)>> = None;
            let mut best_cost = f32::INFINITY;
            for candidate in
                node_detour_candidates(points, first_seg_idx, last_seg_idx, &obstacle, clearance)
            {
                if flowchart_path_hits_non_endpoint_nodes(&candidate, &edge.from, &edge.to, nodes) {
                    continue;
                }
                let cost = path_length(&candidate) + path_bend_count(&candidate) as f32 * clearance;
                if cost < best_cost {
                    best_cost = cost;
                    best = Some(candidate);
                }
            }
            let Some(candidate) = best else {
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
    use super::collapse_axis_aligned_runs;

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
