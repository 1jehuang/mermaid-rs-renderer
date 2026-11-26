use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::config::LayoutConfig;
use crate::ir::{DiagramKind, Graph};
use crate::theme::Theme;

use super::super::{
    DUAL_ENDPOINT_EXTRA_PAD_SCALE, EDGE_LABEL_PAD_SCALE, EDGE_RELAX_GAP_TOLERANCE,
    EDGE_RELAX_STEP_MIN, ENDPOINT_LABEL_PAD_SCALE, MAX_MAIN_GAP_FACTOR, NodeLayout,
    OVERLAP_CENTER_THRESHOLD, OVERLAP_MIN_GAP_FLOOR, OVERLAP_MIN_GAP_RATIO, OVERLAP_RESOLVE_PASSES,
    TextBlock, is_horizontal, is_region_subgraph, measure_label, top_level_subgraph_indices,
};

pub(in crate::layout) fn apply_visual_objectives(
    graph: &Graph,
    layout_edges: &[crate::ir::Edge],
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
    skip_whitespace_compaction: bool,
) {
    if !config.flowchart.objective.enabled {
        return;
    }
    // The whitespace compactor shifts nodes along the main axis per
    // cross-axis band, which would deform the serpentine bands produced by
    // the aspect-ratio fold; skip it when a fold was applied.
    if !skip_whitespace_compaction {
        compact_large_flowchart_whitespace(graph, nodes, config);
    }
    relax_edge_span_constraints(graph, layout_edges, nodes, theme, config);
    rebalance_top_level_subgraphs_aspect(graph, nodes, config);
    let overlap_pass_enabled = match graph.kind {
        DiagramKind::Class => true,
        DiagramKind::Flowchart
        | DiagramKind::State
        | DiagramKind::Er
        | DiagramKind::Requirement => has_visible_node_overlap(nodes),
        _ => false,
    };
    if overlap_pass_enabled {
        resolve_node_overlaps(graph, nodes, config);
    }
}

pub(in crate::layout) fn compact_large_flowchart_whitespace(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    if graph.kind != DiagramKind::Flowchart || !graph.subgraphs.is_empty() {
        return;
    }
    let visible_ids: Vec<String> = nodes
        .values()
        .filter(|node| !node.hidden && node.anchor_subgraph.is_none())
        .map(|node| node.id.clone())
        .collect();
    let node_count = visible_ids.len();
    if node_count < 16 {
        return;
    }
    let edge_count = graph
        .edges
        .iter()
        .filter(|edge| nodes.contains_key(&edge.from) && nodes.contains_key(&edge.to))
        .count();
    if edge_count < 24 {
        return;
    }
    let density = edge_count as f32 / node_count as f32;
    if density < 1.2 {
        return;
    }

    let horizontal = is_horizontal(graph.direction);
    let mut min_main = f32::MAX;
    let mut max_main = f32::MIN;
    let mut min_cross = f32::MAX;
    let mut max_cross = f32::MIN;
    for node in nodes.values() {
        if node.hidden || node.anchor_subgraph.is_some() {
            continue;
        }
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
    if min_main == f32::MAX || min_cross == f32::MAX {
        return;
    }
    let main_span = (max_main - min_main).max(1.0);
    let cross_span = (max_cross - min_cross).max(1.0);
    let aspect = main_span / cross_span;
    if aspect <= config.flowchart.objective.max_aspect_ratio.max(1.0) {
        return;
    }

    let band_size = (config.node_spacing * 1.25).max(24.0);
    let desired_gap = (config.node_spacing * 0.72).max(10.0);
    let mut bands: HashMap<i32, Vec<(String, f32, f32)>> = HashMap::new();
    for node_id in visible_ids {
        let Some(node) = nodes.get(&node_id) else {
            continue;
        };
        let cross_center = if horizontal {
            node.y + node.height * 0.5
        } else {
            node.x + node.width * 0.5
        };
        let main_start = if horizontal { node.x } else { node.y };
        let main_end = if horizontal {
            node.x + node.width
        } else {
            node.y + node.height
        };
        let band = (cross_center / band_size).round() as i32;
        bands
            .entry(band)
            .or_default()
            .push((node_id, main_start, main_end));
    }
    if bands.is_empty() {
        return;
    }

    let mut main_deltas: HashMap<String, f32> = HashMap::new();
    for entries in bands.values_mut() {
        if entries.len() < 2 {
            continue;
        }
        entries.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        let mut cumulative_shift = 0.0f32;
        let mut prev_end: Option<f32> = None;
        for (node_id, start, end) in entries.iter() {
            let adjusted_start = *start - cumulative_shift;
            if let Some(prev) = prev_end {
                let gap = adjusted_start - prev;
                if gap > desired_gap {
                    cumulative_shift += gap - desired_gap;
                }
            }
            if cumulative_shift > 0.0 {
                main_deltas.insert(node_id.clone(), -cumulative_shift);
            }
            prev_end = Some(*end - cumulative_shift);
        }
    }
    if main_deltas.is_empty() {
        return;
    }

    for (node_id, delta) in main_deltas {
        if let Some(node) = nodes.get_mut(&node_id) {
            shift_node_main(node, horizontal, delta);
        }
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
        DiagramKind::Class
        | DiagramKind::Flowchart
        | DiagramKind::State
        | DiagramKind::Er
        | DiagramKind::Requirement => {}
        _ => return,
    }
    let horizontal = is_horizontal(graph.direction);
    let objective = &config.flowchart.objective;
    let passes = objective.edge_relax_passes.max(1);
    let step_limit = (config.rank_spacing + config.node_spacing).max(EDGE_RELAX_STEP_MIN);
    let mut label_cache: HashMap<String, TextBlock> = HashMap::new();
    let flowchart_groups = collect_flowchart_top_level_groups(graph);

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
            if graph.kind == DiagramKind::Flowchart && edge.style == crate::ir::EdgeStyle::Dotted {
                continue;
            }
            if !has_center_label && !has_endpoint_label {
                continue;
            }
            // Class diagrams: an edge between classes on the same rank
            // carries its label sideways, so there is no main-axis gap to
            // widen. Pushing one end down a step only drops it onto the next
            // rank, among the classes there.
            if graph.kind == DiagramKind::Class && current_main_gap < 0.0 {
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
                if shift_cross_subgraph_flowchart_group(
                    &flowchart_groups,
                    edge,
                    nodes,
                    horizontal,
                    main_delta,
                    delta,
                ) {
                    changed = true;
                    continue;
                }
                let (ahead_id, behind_id) = if main_delta >= 0.0 {
                    (edge.to.as_str(), edge.from.as_str())
                } else {
                    (edge.from.as_str(), edge.to.as_str())
                };
                if graph.kind == DiagramKind::Class
                    && let Some(behind) = nodes.get(behind_id)
                {
                    // Widen the gap between ranks rather than moving one
                    // class: everything past the near end's rank moves
                    // along, so ranks stay aligned and the far class's own
                    // edges (an implementation under an interface, say)
                    // keep their room instead of being squeezed or pushed
                    // aside by the overlap pass.
                    let behind_end =
                        node_main_center(behind, horizontal) + node_main_half(behind, horizontal);
                    for node in nodes.values_mut() {
                        let start =
                            node_main_center(node, horizontal) - node_main_half(node, horizontal);
                        if start >= behind_end - 0.5 {
                            shift_node_main(node, horizontal, delta);
                        }
                    }
                    changed = true;
                    continue;
                }
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

struct FlowchartTopLevelGroups {
    node_to_group: HashMap<String, usize>,
    group_nodes: HashMap<usize, Vec<String>>,
}

fn collect_flowchart_top_level_groups(graph: &Graph) -> Option<FlowchartTopLevelGroups> {
    if graph.kind != DiagramKind::Flowchart || graph.subgraphs.is_empty() {
        return None;
    }

    let mut node_to_group = HashMap::new();
    let mut group_nodes = HashMap::new();
    for idx in top_level_subgraph_indices(graph) {
        let sub = &graph.subgraphs[idx];
        if is_region_subgraph(sub) {
            continue;
        }

        let mut nodes_in_group = Vec::new();
        for node_id in &sub.nodes {
            if let Some(existing) = node_to_group.insert(node_id.clone(), idx)
                && existing != idx
            {
                return None;
            }
            nodes_in_group.push(node_id.clone());
        }
        if !nodes_in_group.is_empty() {
            group_nodes.insert(idx, nodes_in_group);
        }
    }

    if group_nodes.is_empty() {
        None
    } else {
        Some(FlowchartTopLevelGroups {
            node_to_group,
            group_nodes,
        })
    }
}

fn shift_cross_subgraph_flowchart_group(
    flowchart_groups: &Option<FlowchartTopLevelGroups>,
    edge: &crate::ir::Edge,
    nodes: &mut BTreeMap<String, NodeLayout>,
    horizontal: bool,
    main_delta: f32,
    delta: f32,
) -> bool {
    let Some(groups) = flowchart_groups.as_ref() else {
        return false;
    };

    let Some(from_group) = groups.node_to_group.get(&edge.from).copied() else {
        return false;
    };
    let Some(to_group) = groups.node_to_group.get(&edge.to).copied() else {
        return false;
    };
    if from_group == to_group {
        return false;
    }

    let ahead_group = if main_delta >= 0.0 {
        to_group
    } else {
        from_group
    };
    let Some(group_nodes) = groups.group_nodes.get(&ahead_group) else {
        return false;
    };

    for node_id in group_nodes {
        if let Some(node) = nodes.get_mut(node_id) {
            shift_node_main(node, horizontal, delta);
        }
    }
    true
}

fn shift_overlap_flowchart_group(
    flowchart_groups: &Option<FlowchartTopLevelGroups>,
    source_id: &str,
    target_id: &str,
    nodes: &mut BTreeMap<String, NodeLayout>,
    horizontal: bool,
    delta: f32,
) -> bool {
    let Some(groups) = flowchart_groups.as_ref() else {
        return false;
    };

    let Some(source_group) = groups.node_to_group.get(source_id).copied() else {
        return false;
    };
    let Some(target_group) = groups.node_to_group.get(target_id).copied() else {
        return false;
    };
    if source_group == target_group {
        return false;
    }

    let Some(group_nodes) = groups.group_nodes.get(&target_group) else {
        return false;
    };
    for node_id in group_nodes {
        if let Some(node) = nodes.get_mut(node_id) {
            shift_node_cross(node, horizontal, delta);
        }
    }
    true
}

fn resolve_node_overlaps(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    let horizontal = is_horizontal(graph.direction);
    let min_gap = (config.node_spacing * OVERLAP_MIN_GAP_RATIO).max(OVERLAP_MIN_GAP_FLOOR);
    let flowchart_groups = collect_flowchart_top_level_groups(graph);
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
                if shift_overlap_flowchart_group(
                    &flowchart_groups,
                    id_a,
                    id_b,
                    nodes,
                    horizontal,
                    sign * delta,
                ) {
                    moved = true;
                    continue;
                }
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
    if graph.kind != DiagramKind::Flowchart {
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{relax_edge_span_constraints, resolve_node_overlaps};
    use crate::config::LayoutConfig;
    use crate::ir::{DiagramKind, Direction, Edge, EdgeStyle, Graph, NodeShape, Subgraph};
    use crate::layout::{NodeLayout, TextBlock};
    use crate::theme::Theme;

    fn make_node(id: &str, x: f32, y: f32) -> NodeLayout {
        NodeLayout {
            id: id.to_string(),
            x,
            y,
            width: 60.0,
            height: 36.0,
            label: TextBlock {
                lines: vec![id.to_string()],
                width: 20.0,
                height: 16.0,
            },
            shape: NodeShape::Rectangle,
            style: crate::ir::NodeStyle::default(),
            link: None,
            anchor_subgraph: None,
            hidden: false,
            icon: None,
        }
    }

    fn make_edge(from: &str, to: &str, label: &str) -> Edge {
        Edge {
            from: from.to_string(),
            to: to.to_string(),
            label: Some(label.to_string()),
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

    #[test]
    fn relax_edge_span_constraints_moves_cross_subgraph_group_together() {
        let mut graph = Graph::new();
        graph.kind = DiagramKind::Flowchart;
        graph.direction = Direction::LeftRight;
        for id in ["A", "B", "C", "D"] {
            graph.ensure_node(id, Some(id.to_string()), Some(NodeShape::Rectangle));
        }
        graph.subgraphs.push(Subgraph {
            id: Some("left".to_string()),
            label: "Left".to_string(),
            nodes: vec!["A".to_string(), "B".to_string()],
            direction: None,
            icon: None,
        });
        graph.subgraphs.push(Subgraph {
            id: Some("right".to_string()),
            label: "Right".to_string(),
            nodes: vec!["C".to_string(), "D".to_string()],
            direction: None,
            icon: None,
        });

        let edge = make_edge("A", "C", "cross subgraph request payload");
        let layout_edges = vec![edge.clone()];
        graph.edges.push(edge);

        let mut nodes = BTreeMap::from([
            ("A".to_string(), make_node("A", 0.0, 0.0)),
            ("B".to_string(), make_node("B", 90.0, 0.0)),
            ("C".to_string(), make_node("C", 110.0, 0.0)),
            ("D".to_string(), make_node("D", 200.0, 0.0)),
        ]);
        let before_right_gap = nodes["D"].x - nodes["C"].x;
        let before_left_gap = nodes["B"].x - nodes["A"].x;

        relax_edge_span_constraints(
            &graph,
            &layout_edges,
            &mut nodes,
            &Theme::modern(),
            &LayoutConfig::default(),
        );

        assert!(
            nodes["C"].x > 110.0,
            "expected the destination subgraph to shift for a labeled cross-subgraph edge"
        );
        assert!(
            (nodes["D"].x - nodes["C"].x - before_right_gap).abs() < 0.1,
            "cross-subgraph relaxation should preserve destination subgraph spacing"
        );
        assert!(
            (nodes["B"].x - nodes["A"].x - before_left_gap).abs() < 0.1,
            "moving the destination subgraph should not shear the source subgraph"
        );
    }

    #[test]
    fn resolve_node_overlaps_moves_cross_subgraph_group_together() {
        let mut graph = Graph::new();
        graph.kind = DiagramKind::Flowchart;
        graph.direction = Direction::LeftRight;
        for id in ["A", "B", "C", "D"] {
            graph.ensure_node(id, Some(id.to_string()), Some(NodeShape::Rectangle));
        }
        graph.subgraphs.push(Subgraph {
            id: Some("left".to_string()),
            label: "Left".to_string(),
            nodes: vec!["A".to_string(), "B".to_string()],
            direction: None,
            icon: None,
        });
        graph.subgraphs.push(Subgraph {
            id: Some("right".to_string()),
            label: "Right".to_string(),
            nodes: vec!["C".to_string(), "D".to_string()],
            direction: None,
            icon: None,
        });

        let mut nodes = BTreeMap::from([
            ("A".to_string(), make_node("A", 0.0, 0.0)),
            ("B".to_string(), make_node("B", 90.0, 0.0)),
            ("C".to_string(), make_node("C", 90.0, 12.0)),
            ("D".to_string(), make_node("D", 180.0, 12.0)),
        ]);
        let before_right_delta = nodes["D"].y - nodes["C"].y;
        let before_left_delta = nodes["B"].y - nodes["A"].y;

        resolve_node_overlaps(&graph, &mut nodes, &LayoutConfig::default());

        assert!(
            nodes["C"].y > 12.0,
            "expected the overlapping destination subgraph to move on the cross axis"
        );
        assert!(
            (nodes["D"].y - nodes["C"].y - before_right_delta).abs() < 0.1,
            "cross-subgraph overlap resolution should preserve destination subgraph spacing"
        );
        assert!(
            (nodes["B"].y - nodes["A"].y - before_left_delta).abs() < 0.1,
            "overlap resolution should not shear the source subgraph"
        );
    }
}
