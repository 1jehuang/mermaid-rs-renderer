use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::config::LayoutConfig;
use crate::ir::Graph;
use crate::theme::Theme;

use super::super::routing::is_horizontal;
use super::super::{
    MIN_NODE_SPACING_FLOOR, NodeLayout, SUBGRAPH_DESIRED_GAP_RATIO, build_subgraph_layouts,
    measure_label, subgraph_anchor_id, subgraph_padding_from_label, top_level_subgraph_indices,
};

fn is_region_subgraph(sub: &crate::ir::Subgraph) -> bool {
    sub.label.trim().is_empty()
        && sub
            .id
            .as_deref()
            .map(|id| id.starts_with("__region_"))
            .unwrap_or(false)
}

pub(in crate::layout) fn apply_flowchart_node_layout_cleanup(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart {
        return;
    }

    compress_linear_subgraphs(graph, nodes, config);

    if graph.subgraphs.is_empty() {
        align_disconnected_components(graph, nodes, config);
        return;
    }

    enforce_top_level_subgraph_gap(graph, nodes, theme, config);
    separate_sibling_subgraphs(graph, nodes, theme, config);
    align_single_entry_top_level_subgraphs(graph, nodes, config);
    align_disconnected_top_level_subgraphs(graph, nodes);
    evict_non_member_nodes_from_subgraphs(graph, nodes, theme, config);
    debug_assert_flowchart_node_layout_invariants(graph, nodes);
}

/// Push nodes that belong to no subgraph (or to a different subgraph chain)
/// out of subgraph boxes they visually landed in. A non-member node rendered
/// inside a subgraph box misstates containment semantics, which is worse than
/// slightly longer edges.
pub(in crate::layout) fn evict_non_member_nodes_from_subgraphs(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.subgraphs.is_empty() {
        return;
    }

    let horizontal = is_horizontal(graph.direction);
    let clearance = (config.node_spacing * 0.5).max(MIN_NODE_SPACING_FLOOR);

    // Materialize subgraph rects exactly the way rendering will (including
    // nested-child expansion), so eviction sees the true boxes.
    let compute_rects = |nodes: &BTreeMap<String, NodeLayout>| -> Vec<[f32; 4]> {
        build_subgraph_layouts(graph, nodes, theme, config)
            .into_iter()
            .filter(|sub| sub.width > 0.0 && sub.height > 0.0)
            .map(|sub| [sub.x, sub.y, sub.x + sub.width, sub.y + sub.height])
            .collect()
    };

    // Only move nodes that are members of no subgraph: nodes in *other*
    // subgraphs are handled by the sibling-separation pass, and moving them
    // here would tear their own box apart.
    let mut all_members: HashSet<&str> = HashSet::new();
    for sub in &graph.subgraphs {
        for node_id in &sub.nodes {
            all_members.insert(node_id.as_str());
        }
    }
    let free_ids: Vec<String> = nodes
        .values()
        .filter(|node| !node.hidden && node.anchor_subgraph.is_none())
        .filter(|node| !all_members.contains(node.id.as_str()))
        .map(|node| node.id.clone())
        .collect();
    if free_ids.is_empty() {
        return;
    }

    for _ in 0..4 {
        let rects = compute_rects(nodes);
        let mut moved = false;
        for id in &free_ids {
            let Some(node) = nodes.get(id) else {
                continue;
            };
            let nx1 = node.x;
            let ny1 = node.y;
            let nx2 = node.x + node.width;
            let ny2 = node.y + node.height;
            let overlaps_any = |x1: f32, y1: f32, x2: f32, y2: f32| -> usize {
                rects
                    .iter()
                    .filter(|[rx1, ry1, rx2, ry2]| {
                        x2.min(*rx2) - x1.max(*rx1) > 0.0 && y2.min(*ry2) - y1.max(*ry1) > 0.0
                    })
                    .count()
            };
            if overlaps_any(nx1, ny1, nx2, ny2) == 0 {
                continue;
            }
            // Candidate pushes: escape each overlapped rect on all four
            // sides, then pick the candidate that clears the most boxes with
            // the least displacement (cross-axis moves preferred slightly).
            let mut best: Option<(usize, f32, f32, f32)> = None;
            for [rx1, ry1, rx2, ry2] in &rects {
                let overlap_x = nx2.min(*rx2) - nx1.max(*rx1);
                let overlap_y = ny2.min(*ry2) - ny1.max(*ry1);
                if overlap_x <= 0.0 || overlap_y <= 0.0 {
                    continue;
                }
                let candidates = [
                    (-(nx2 - rx1 + clearance), 0.0),
                    (rx2 - nx1 + clearance, 0.0),
                    (0.0, -(ny2 - ry1 + clearance)),
                    (0.0, ry2 - ny1 + clearance),
                ];
                for (dx, dy) in candidates {
                    let remaining = overlaps_any(nx1 + dx, ny1 + dy, nx2 + dx, ny2 + dy);
                    let displacement = dx.abs() + dy.abs();
                    // Cross-axis moves keep rank ordering intact; give
                    // main-axis moves a mild penalty rather than a veto.
                    let main_axis_move = if horizontal { dx != 0.0 } else { dy != 0.0 };
                    let score = displacement * if main_axis_move { 1.25 } else { 1.0 };
                    let better = match &best {
                        None => true,
                        Some((best_remaining, best_score, _, _)) => {
                            remaining < *best_remaining
                                || (remaining == *best_remaining && score < *best_score)
                        }
                    };
                    if better {
                        best = Some((remaining, score, dx, dy));
                    }
                }
            }
            if let Some((_, _, dx, dy)) = best
                && let Some(node) = nodes.get_mut(id)
            {
                node.x += dx;
                node.y += dy;
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
}

pub(in crate::layout) fn debug_assert_flowchart_node_layout_invariants(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart {
        return;
    }

    for node in nodes.values() {
        debug_assert!(
            node.x.is_finite(),
            "flowchart node {} has non-finite x",
            node.id
        );
        debug_assert!(
            node.y.is_finite(),
            "flowchart node {} has non-finite y",
            node.id
        );
        debug_assert!(
            node.width.is_finite(),
            "flowchart node {} has non-finite width",
            node.id
        );
        debug_assert!(
            node.height.is_finite(),
            "flowchart node {} has non-finite height",
            node.id
        );
    }

    if graph.subgraphs.len() < 2 {
        return;
    }

    let mut seen = HashSet::new();
    for idx in top_level_subgraph_indices(graph) {
        for node_id in &graph.subgraphs[idx].nodes {
            if !seen.insert(node_id.as_str()) {
                return;
            }
        }
    }

    #[derive(Clone, Copy)]
    struct Bounds<'a> {
        label: &'a str,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
    }

    let mut bounds = Vec::new();
    for idx in top_level_subgraph_indices(graph) {
        let sub = &graph.subgraphs[idx];
        if is_region_subgraph(sub) {
            continue;
        }

        let Some((min_x, min_y, max_x, max_y)) = super::super::node_bounds(nodes, &sub.nodes)
        else {
            continue;
        };
        bounds.push(Bounds {
            label: sub.label.as_str(),
            min_x,
            min_y,
            max_x,
            max_y,
        });
    }

    for (idx, left) in bounds.iter().enumerate() {
        for right in bounds.iter().skip(idx + 1) {
            let overlaps_x = left.min_x < right.max_x && right.min_x < left.max_x;
            let overlaps_y = left.min_y < right.max_y && right.min_y < left.max_y;
            debug_assert!(
                !(overlaps_x && overlaps_y),
                "top-level flowchart subgraphs overlap after node cleanup: {} vs {}",
                left.label,
                right.label
            );
        }
    }
}

pub(in crate::layout) fn compress_linear_subgraphs(
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

pub(in crate::layout) fn enforce_top_level_subgraph_gap(
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

    let mut seen: HashSet<&str> = HashSet::new();
    for &idx in &top_level {
        for node_id in &graph.subgraphs[idx].nodes {
            if !seen.insert(node_id.as_str()) {
                return;
            }
        }
    }

    let node_to_top_level_sg: HashMap<&str, usize> = top_level
        .iter()
        .flat_map(|&idx| {
            graph.subgraphs[idx]
                .nodes
                .iter()
                .map(move |n| (n.as_str(), idx))
        })
        .collect();
    let has_cross_sg_edge = graph.edges.iter().any(|edge| {
        let from_sg = node_to_top_level_sg.get(edge.from.as_str());
        let to_sg = node_to_top_level_sg.get(edge.to.as_str());
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

    let horizontal = is_horizontal(graph.direction);
    let mut bounds: Vec<Bounds> = Vec::new();

    for &idx in &top_level {
        let sub = &graph.subgraphs[idx];
        if is_region_subgraph(sub) || sub.nodes.is_empty() {
            continue;
        }

        let Some((min_x, min_y, max_x, max_y)) = super::super::node_bounds(nodes, &sub.nodes)
        else {
            continue;
        };

        let (_, (pad_x, pad_y, top_padding)) =
            super::super::subgraph_label_metrics(graph, sub, theme, config);

        bounds.push(Bounds {
            idx,
            min_x: min_x - pad_x,
            min_y: min_y - top_padding,
            max_x: max_x + pad_x,
            max_y: max_y + pad_y,
            pad_main: if horizontal { pad_x } else { pad_y },
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

    let pad_main = bounds
        .iter()
        .map(|bound| bound.pad_main)
        .fold(0.0, f32::max);
    let desired_gap = (config.node_spacing * SUBGRAPH_DESIRED_GAP_RATIO).max(pad_main * 2.0);

    let mut prev_max_main: Option<f32> = None;
    for bound in &mut bounds {
        let min_main = if horizontal { bound.min_x } else { bound.min_y };
        let mut max_main = if horizontal { bound.max_x } else { bound.max_y };

        let mut delta = 0.0;
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

pub(in crate::layout) fn separate_sibling_subgraphs(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) {
    if graph.subgraphs.len() < 2 {
        return;
    }

    let tree = super::super::SubgraphTree::build(graph);
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

    let horizontal = is_horizontal(graph.direction);
    for group in sibling_groups {
        let mut bounds: Vec<(usize, f32, f32, f32, f32)> = Vec::new();
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
            if min_x == f32::MAX {
                continue;
            }

            let label_block = measure_label(&sub.label, theme, config);
            let (pad_x, pad_y, top_padding) =
                subgraph_padding_from_label(graph, sub, theme, &label_block);
            bounds.push((
                idx,
                min_x - pad_x,
                min_y - top_padding,
                max_x + pad_x,
                max_y + pad_y,
            ));
        }

        if bounds.len() < 2 {
            continue;
        }

        if horizontal {
            bounds.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(Ordering::Equal));
        } else {
            bounds.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        }

        let gap = config.node_spacing.max(8.0);
        let overlaps =
            |a_min: f32, a_max: f32, b_min: f32, b_max: f32| a_min < b_max && b_min < a_max;

        let mut placed: Vec<(usize, f32, f32, f32, f32)> = Vec::new();
        for (idx, min_x, min_y, max_x, max_y) in bounds {
            let mut shift = 0.0f32;

            for &(_, px1, py1, px2, py2) in &placed {
                let other_axis_overlaps = if horizontal {
                    overlaps(min_x, max_x, px1, px2)
                } else {
                    overlaps(min_y, max_y, py1, py2)
                };
                if !other_axis_overlaps {
                    continue;
                }

                let shifted_min = if horizontal {
                    min_y + shift
                } else {
                    min_x + shift
                };
                let shifted_max = if horizontal {
                    max_y + shift
                } else {
                    max_x + shift
                };
                let placed_min = if horizontal { py1 } else { px1 };
                let placed_max = if horizontal { py2 } else { px2 };

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
                        if horizontal {
                            node.y += shift;
                        } else {
                            node.x += shift;
                        }
                    }
                }
            }

            placed.push(if horizontal {
                (idx, min_x, min_y + shift, max_x, max_y + shift)
            } else {
                (idx, min_x + shift, min_y, max_x + shift, max_y)
            });
        }
    }
}

pub(in crate::layout) fn align_disconnected_top_level_subgraphs(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
) {
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
        let (mut min_x, mut min_y, mut max_x, mut max_y) = super::super::node_bounds(
            nodes, &sub.nodes,
        )
        .unwrap_or((f32::MAX, f32::MAX, f32::MIN, f32::MIN));
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
        if let Some(prev) = prev_max
            && min_main < prev
        {
            return;
        }
        prev_max = Some(max_main);
    }

    let target_cross = bounds
        .iter()
        .map(|bound| if horizontal { bound.min_y } else { bound.min_x })
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
        if let Some(anchor_id) = bound.anchor_id.as_deref()
            && let Some(node) = nodes.get_mut(anchor_id)
        {
            if horizontal {
                node.y += delta;
            } else {
                node.x += delta;
            }
        }
    }
}

pub(in crate::layout) fn align_single_entry_top_level_subgraphs(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.subgraphs.is_empty() {
        return;
    }

    let outer_horizontal = is_horizontal(graph.direction);
    let shift_limit = (config.node_spacing * 0.75).max(10.0);

    for idx in top_level_subgraph_indices(graph) {
        let sub = &graph.subgraphs[idx];
        if sub.nodes.is_empty() || is_region_subgraph(sub) {
            continue;
        }

        let inner_direction = sub.direction.unwrap_or(graph.direction);
        let inner_horizontal = is_horizontal(inner_direction);
        if inner_horizontal == outer_horizontal {
            continue;
        }

        let node_set: HashSet<&str> = sub.nodes.iter().map(String::as_str).collect();
        let external_incoming: Vec<_> = graph
            .edges
            .iter()
            .filter(|edge| {
                !node_set.contains(edge.from.as_str()) && node_set.contains(edge.to.as_str())
            })
            .collect();
        let external_outgoing: Vec<_> = graph
            .edges
            .iter()
            .filter(|edge| {
                node_set.contains(edge.from.as_str()) && !node_set.contains(edge.to.as_str())
            })
            .collect();

        if external_incoming.len() != 1 || !external_outgoing.is_empty() {
            continue;
        }

        let feeder = external_incoming[0];
        let Some(source) = nodes.get(&feeder.from) else {
            continue;
        };
        let Some(target) = nodes.get(&feeder.to) else {
            continue;
        };

        let source_center = if outer_horizontal {
            source.y + source.height * 0.5
        } else {
            source.x + source.width * 0.5
        };
        let target_center = if outer_horizontal {
            target.y + target.height * 0.5
        } else {
            target.x + target.width * 0.5
        };
        let delta = source_center - target_center;
        if delta.abs() < 0.5 || delta.abs() > shift_limit {
            continue;
        }

        let is_edge_member = if inner_horizontal {
            let extreme = sub
                .nodes
                .iter()
                .filter_map(|node_id| nodes.get(node_id))
                .map(|node| node.x)
                .fold(f32::MAX, f32::min);
            (target.x - extreme).abs() <= 1.0
        } else {
            let extreme = sub
                .nodes
                .iter()
                .filter_map(|node_id| nodes.get(node_id))
                .map(|node| node.y)
                .fold(f32::MAX, f32::min);
            (target.y - extreme).abs() <= 1.0
        };
        if !is_edge_member {
            continue;
        }

        for node_id in &sub.nodes {
            if let Some(node) = nodes.get_mut(node_id) {
                if outer_horizontal {
                    node.y += delta;
                } else {
                    node.x += delta;
                }
            }
        }
        if let Some(anchor_id) = subgraph_anchor_id(sub, nodes)
            && let Some(anchor) = nodes.get_mut(anchor_id)
        {
            if outer_horizontal {
                anchor.y += delta;
            } else {
                anchor.x += delta;
            }
        }
    }
}

pub(in crate::layout) fn align_disconnected_components(
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
        let mut component = Vec::new();
        visited.insert(node_id.clone());
        while let Some(current) = stack.pop() {
            component.push(current.clone());
            if let Some(neighbors) = adjacency.get(&current) {
                for next in neighbors {
                    if visited.insert(next.clone()) {
                        stack.push(next.clone());
                    }
                }
            }
        }
        if !component.is_empty() {
            components.push(component);
        }
    }

    if components.len() < 2 {
        return;
    }

    #[derive(Clone)]
    struct ComponentBounds {
        nodes: Vec<String>,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
    }

    let mut bounds: Vec<ComponentBounds> = Vec::new();
    for component in components {
        let Some((min_x, min_y, max_x, max_y)) = super::super::node_bounds(nodes, &component)
        else {
            continue;
        };
        bounds.push(ComponentBounds {
            nodes: component,
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
        .map(|bound| if horizontal { bound.min_y } else { bound.min_x })
        .fold(f32::MAX, f32::min);
    let spacing = config.node_spacing.max(MIN_NODE_SPACING_FLOOR);
    let mut cursor = if horizontal {
        bounds
            .iter()
            .map(|bound| bound.min_x)
            .fold(f32::MAX, f32::min)
    } else {
        bounds
            .iter()
            .map(|bound| bound.min_y)
            .fold(f32::MAX, f32::min)
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
        cursor += (max_main - min_main).max(1.0) + spacing;
    }
}
