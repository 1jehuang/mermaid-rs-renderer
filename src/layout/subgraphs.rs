use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::config::LayoutConfig;
use crate::ir::{DiagramKind, Direction, Graph};
use crate::theme::Theme;

use super::geometry::EdgeSide;
use super::ranking::compute_ranks_subset;
use super::routing::is_horizontal;
use super::text::measure_label;
use super::types::{NodeLayout, SubgraphLayout, TextBlock};
use super::{
    FLOWCHART_PAD_CROSS, FLOWCHART_PAD_MAIN, GENERIC_SUBGRAPH_BASE_PAD, KANBAN_SUBGRAPH_PAD,
    STATE_RANK_SPACING_BOOST, STATE_SUBGRAPH_BASE_PAD, STATE_SUBGRAPH_TOP_LABEL_SCALE,
    STATE_SUBGRAPH_TOP_MIN_SCALE, SUBGRAPH_LABEL_GAP_FLOWCHART, SUBGRAPH_LABEL_GAP_GENERIC,
    SUBGRAPH_LABEL_GAP_KANBAN, merge_node_style,
};

pub(super) fn is_region_subgraph(sub: &crate::ir::Subgraph) -> bool {
    sub.label.trim().is_empty()
        && sub
            .id
            .as_deref()
            .map(|id| id.starts_with("__region_"))
            .unwrap_or(false)
}

pub(super) fn apply_subgraph_bands(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    let mut group_nodes: Vec<Vec<String>> = Vec::new();
    let mut node_group: HashMap<String, usize> = HashMap::new();

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
        if let Some(anchor_id) = subgraph_anchor_id(sub, nodes)
            && nodes.contains_key(anchor_id)
        {
            node_group.insert(anchor_id.to_string(), group_idx);
        }
    }

    for node_id in graph.nodes.keys() {
        if !node_group.contains_key(node_id) {
            node_group.insert(node_id.clone(), 0);
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
        if let Some((min_x, min_y, max_x, max_y)) = super::node_bounds(nodes, bucket) {
            groups.push((idx, min_x, min_y, max_x, max_y));
        }
    }

    let mut inter_group_edges = 0usize;
    let mut group_links: HashSet<(usize, usize)> = HashSet::new();
    let mut group_degree: HashMap<usize, usize> = HashMap::new();
    for edge in &graph.edges {
        let from_group = node_group.get(&edge.from);
        let to_group = node_group.get(&edge.to);
        if let (Some(a), Some(b)) = (from_group, to_group)
            && a != b
        {
            inter_group_edges += 1;
            let (min_g, max_g) = if a < b { (*a, *b) } else { (*b, *a) };
            group_links.insert((min_g, max_g));
            *group_degree.entry(*a).or_insert(0) += 1;
            *group_degree.entry(*b).or_insert(0) += 1;
        }
    }
    let max_degree = group_degree.values().copied().max().unwrap_or(0);
    let path_like = inter_group_edges > 0
        && group_links.len() <= groups.len().saturating_sub(1)
        && max_degree <= 2;
    let grid_pack = inter_group_edges == 0;
    let align_cross = path_like;

    if is_horizontal(graph.direction) {
        groups.sort_by(|a, b| {
            let a_primary = if a.0 == 0 { 0 } else { 1 };
            let b_primary = if b.0 == 0 { 0 } else { 1 };
            a_primary
                .cmp(&b_primary)
                .then_with(|| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))
        });
    } else {
        groups.sort_by(|a, b| {
            let a_primary = if a.0 == 0 { 0 } else { 1 };
            let b_primary = if b.0 == 0 { 0 } else { 1 };
            a_primary
                .cmp(&b_primary)
                .then_with(|| a.2.partial_cmp(&b.2).unwrap_or(Ordering::Equal))
        });
    }

    let spacing = config.rank_spacing * 0.8;
    if is_horizontal(graph.direction) {
        if align_cross && !groups.is_empty() {
            let target_y = groups.iter().map(|group| group.2).fold(f32::MAX, f32::min);
            for (group_idx, _min_x, min_y, _max_x, _max_y) in &groups {
                let offset_y = target_y - *min_y;
                for node_id in &group_nodes[*group_idx] {
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
                    for node_id in &group_nodes[group_idx] {
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
                for node_id in &group_nodes[group_idx] {
                    if let Some(node) = nodes.get_mut(node_id) {
                        node.x += offset;
                    }
                }
                cursor += width + spacing;
            }
        }
    } else if align_cross && !groups.is_empty() {
        let target_x = groups.iter().map(|group| group.1).fold(f32::MAX, f32::min);
        for (group_idx, min_x, _min_y, _max_x, _max_y) in &groups {
            let offset_x = target_x - *min_x;
            for node_id in &group_nodes[*group_idx] {
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
            let cols = groups.len().div_ceil(rows);
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
                for node_id in &group_nodes[group_idx] {
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
            for node_id in &group_nodes[group_idx] {
                if let Some(node) = nodes.get_mut(node_id) {
                    node.y += offset;
                }
            }
            cursor += height + spacing;
        }
    }
}

pub(super) fn apply_orthogonal_region_bands(
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
            if idx == region_idx || set.len() <= region_set.len() || !region_set.is_subset(set) {
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
            if let Some((min_x, min_y, max_x, max_y)) =
                super::node_bounds(nodes, &graph.subgraphs[region_idx].nodes)
            {
                region_boxes.push((region_idx, min_x, min_y, max_x, max_y));
            }
        }
        if region_boxes.len() <= 1 {
            continue;
        }

        if stack_along_x {
            region_boxes.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
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
            region_boxes.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(Ordering::Equal));
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

#[derive(Debug, Clone)]
pub(super) struct SubgraphTree {
    parent: Vec<Option<usize>>,
    children: Vec<Vec<usize>>,
    top_level: Vec<usize>,
}

impl SubgraphTree {
    pub(super) fn build(graph: &Graph) -> Self {
        let n = graph.subgraphs.len();
        let sets: Vec<HashSet<String>> = graph
            .subgraphs
            .iter()
            .map(|sub| sub.nodes.iter().cloned().collect())
            .collect();

        let mut by_size: Vec<usize> = (0..n).collect();
        by_size.sort_by_key(|&i| sets[i].len());

        let mut parent: Vec<Option<usize>> = vec![None; n];
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];

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

    pub(super) fn are_siblings(&self, a: usize, b: usize) -> bool {
        a != b && !self.is_ancestor(a, b) && !self.is_ancestor(b, a)
    }
}

pub(super) fn top_level_subgraph_indices(graph: &Graph) -> Vec<usize> {
    SubgraphTree::build(graph).top_level
}

pub(super) fn apply_subgraph_node_layout_passes(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
    anchored_indices: &HashSet<usize>,
    anchor_info: &HashMap<String, SubgraphAnchorInfo>,
) {
    if graph.subgraphs.is_empty() {
        return;
    }

    if graph.kind != DiagramKind::State {
        apply_subgraph_direction_overrides(graph, nodes, theme, config, anchored_indices);
    }
    if !anchor_info.is_empty() {
        let _ = align_subgraphs_to_anchor_nodes(graph, anchor_info, nodes, theme, config);
    }
    if graph.kind == DiagramKind::State && !anchor_info.is_empty() {
        apply_state_subgraph_layouts(graph, nodes, config, anchored_indices);
    }

    apply_orthogonal_region_bands(graph, nodes, config);

    // Flowcharts have their own dedicated subgraph spacing pipeline. Let the
    // flowchart-specific stages own final grouping instead of stacking the
    // generic banding pass on top of them.
    if !matches!(graph.kind, DiagramKind::State | DiagramKind::Flowchart) {
        apply_subgraph_bands(graph, nodes, config);
    }
}

pub(super) fn apply_subgraph_direction_overrides(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
    skip_indices: &HashSet<usize>,
) {
    let tree = SubgraphTree::build(graph);
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
        if sub.nodes.is_empty() || direction == graph.direction {
            continue;
        }

        let Some((min_x, min_y, _, _)) = super::node_bounds(nodes, &sub.nodes) else {
            continue;
        };

        // Re-place the members with the same recursive routine the anchored
        // path uses, so a nested child keeps its own direction and stays whole
        // instead of dissolving into this subgraph's rank graph. Keeping the
        // existing top-left corner leaves surrounding layout undisturbed.
        layout_cluster_contents(
            graph,
            idx,
            &tree,
            nodes,
            theme,
            config,
            false,
            (min_x, min_y),
        );
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
    if graph.kind == crate::ir::DiagramKind::Flowchart
        || graph.kind == crate::ir::DiagramKind::State
    {
        return subgraph_anchor_id(sub, nodes).is_some();
    }
    subgraph_is_anchorable(sub, graph, nodes)
}

pub(super) fn subgraph_anchor_id<'a>(
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

pub(super) fn mark_subgraph_anchor_nodes_hidden(
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

pub(super) fn pick_subgraph_anchor_child(
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
pub(super) struct SubgraphAnchorInfo {
    pub(super) sub_idx: usize,
    pub(super) padding_x: f32,
    pub(super) top_padding: f32,
}

fn subgraph_layout_direction(graph: &Graph, sub: &crate::ir::Subgraph) -> Direction {
    if graph.kind == crate::ir::DiagramKind::State {
        return graph.direction;
    }
    sub.direction.unwrap_or(graph.direction)
}

fn subgraph_layout_config(graph: &Graph, anchorable: bool, config: &LayoutConfig) -> LayoutConfig {
    let mut local = config.clone();
    if graph.kind == crate::ir::DiagramKind::Flowchart && anchorable {
        local.rank_spacing = config.rank_spacing + STATE_RANK_SPACING_BOOST;
    }
    local
}

fn flowchart_subgraph_padding(direction: Direction) -> (f32, f32) {
    if is_horizontal(direction) {
        (FLOWCHART_PAD_MAIN, FLOWCHART_PAD_CROSS)
    } else {
        (FLOWCHART_PAD_CROSS, FLOWCHART_PAD_MAIN)
    }
}

fn flowchart_subgraph_internal_edge_count(graph: &Graph, sub: &crate::ir::Subgraph) -> usize {
    let node_set: HashSet<&str> = sub.nodes.iter().map(String::as_str).collect();
    graph
        .edges
        .iter()
        .filter(|edge| node_set.contains(edge.from.as_str()) && node_set.contains(edge.to.as_str()))
        .count()
}

pub(super) fn subgraph_padding_from_label(
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
    let internal_edge_count = if graph.kind == crate::ir::DiagramKind::Flowchart {
        flowchart_subgraph_internal_edge_count(graph, sub)
    } else {
        0
    };
    let has_internal_cycle = graph.kind == crate::ir::DiagramKind::Flowchart
        && internal_edge_count >= sub.nodes.len().max(1);

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
        let reduction = if has_internal_cycle {
            1.0
        } else if label_empty {
            0.7
        } else {
            0.9
        };
        pad_x *= reduction;
        pad_y *= reduction;
    }
    if graph.kind == crate::ir::DiagramKind::Flowchart && has_internal_cycle {
        let cycle_extra = (theme.font_size * 0.75).max(10.0);
        pad_y += cycle_extra;
    }

    let top_padding = if label_empty {
        pad_y
    } else if graph.kind == crate::ir::DiagramKind::Flowchart {
        let title_clearance = (theme.font_size * 0.75).max(10.0);
        pad_y.max(label_height + SUBGRAPH_LABEL_GAP_FLOWCHART + title_clearance)
    } else if graph.kind == crate::ir::DiagramKind::Kanban {
        pad_y.max(label_height + SUBGRAPH_LABEL_GAP_KANBAN)
    } else if graph.kind == crate::ir::DiagramKind::State {
        // Header height (matches the rendered title band) plus a body inset
        // so the topmost child clears the title separator line instead of
        // sitting directly on it.
        (label_height + theme.font_size * STATE_SUBGRAPH_TOP_LABEL_SCALE)
            .max(theme.font_size * STATE_SUBGRAPH_TOP_MIN_SCALE)
            + STATE_SUBGRAPH_BASE_PAD
    } else {
        pad_y + label_height + SUBGRAPH_LABEL_GAP_GENERIC
    };

    (pad_x, pad_y, top_padding)
}

/// A subgraph's rendered title block and the padding its frame puts around its
/// contents, as `(label_block, (pad_x, pad_y, top_padding))`.
///
/// An untitled subgraph measures as a zero-sized block so it gets no title
/// band. Every caller needs that same pairing, so it lives here rather than
/// being spelled out again at each frame-sizing site.
pub(in crate::layout) fn subgraph_label_metrics(
    graph: &Graph,
    sub: &crate::ir::Subgraph,
    theme: &Theme,
    config: &LayoutConfig,
) -> (TextBlock, (f32, f32, f32)) {
    let mut label_block = measure_label(&sub.label, theme, config);
    if sub.label.trim().is_empty() {
        label_block.width = 0.0;
        label_block.height = 0.0;
    }
    let padding = subgraph_padding_from_label(graph, sub, theme, &label_block);
    (label_block, padding)
}

/// Lay out one cluster's contents, innermost first, and return the bounding box
/// its members ended up occupying.
///
/// A cluster's members are stored flattened (`TOP.nodes` is `[i1,f1,i2,f2]`,
/// not `[B1,B2]`), so ranking them directly makes nested clusters dissolve into
/// their parent's rank graph: sibling clusters land on shared ranks and an edge
/// written between two of them (`B1 --> B2`) is dropped, because it targets
/// their anchors rather than any member.
///
/// So resolve the nesting explicitly instead. Each child cluster is laid out
/// first, then stands in its parent's rank graph as a single unit sized to its
/// own result; edges are rewritten onto those units. Once the parent has placed
/// the units, each child is translated rigidly into its slot. Every cluster is
/// therefore ranked once, with its own `direction`, over its real children.
fn layout_cluster_contents(
    graph: &Graph,
    idx: usize,
    tree: &SubgraphTree,
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
    anchorable: bool,
    origin: (f32, f32),
) -> Option<(f32, f32, f32, f32)> {
    let sub = graph.subgraphs.get(idx)?;
    if sub.nodes.is_empty() {
        return None;
    }

    let units = lay_out_child_clusters(graph, idx, tree, nodes, theme, config, anchorable);
    let unit_ids = cluster_unit_ids(graph, sub, &units, nodes);
    if unit_ids.is_empty() {
        return None;
    }
    let unit_edges = cluster_unit_edges(graph, &units, &unit_ids);

    let borrowed = insert_cluster_placeholders(graph, &units, nodes);
    let direction = subgraph_layout_direction(graph, sub);
    let ranks = compute_ranks_subset(
        &unit_ids,
        &unit_edges,
        &cluster_unit_order(graph, &unit_ids),
    );
    super::assign_positions(
        &unit_ids,
        &ranks,
        direction,
        &subgraph_layout_config(graph, anchorable, config),
        nodes,
        origin.0,
        origin.1,
    );
    if matches!(direction, Direction::RightLeft | Direction::BottomTop) {
        mirror_subgraph_nodes(&unit_ids, nodes, direction);
    }

    let child_rects: Vec<(f32, f32, f32, f32)> = units
        .iter()
        .filter_map(|unit| move_child_into_slot(graph, unit, nodes))
        .collect();
    restore_borrowed_nodes(nodes, borrowed);

    // A child's frame extends past its members by its own padding, so the
    // parent has to contain the frames, not just the leaves.
    let members = super::node_bounds(nodes, &sub.nodes);
    child_rects
        .into_iter()
        .chain(members)
        .reduce(|a, b| (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)))
}

/// A nested cluster, laid out and ready to stand in its parent's rank graph as
/// one sized block.
struct ClusterUnit {
    child: usize,
    /// Id of the hidden stand-in node placed in the parent's map.
    id: String,
    width: f32,
    height: f32,
    /// `(pad_x, pad_y, top_padding)` the child's own frame adds.
    padding: (f32, f32, f32),
}

/// Lay each nested child out first, so it has a real size to stand in with.
fn lay_out_child_clusters(
    graph: &Graph,
    idx: usize,
    tree: &SubgraphTree,
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
    anchorable: bool,
) -> Vec<ClusterUnit> {
    let mut units = Vec::new();
    for &child in &tree.children[idx] {
        let Some(bounds) = layout_cluster_contents(
            graph,
            child,
            tree,
            nodes,
            theme,
            config,
            anchorable,
            (0.0, 0.0),
        ) else {
            continue;
        };
        let padding = subgraph_label_metrics(graph, &graph.subgraphs[child], theme, config).1;
        units.push(ClusterUnit {
            child,
            id: format!("__cluster_unit_{child}__"),
            width: bounds.2 - bounds.0 + padding.0 * 2.0,
            height: bounds.3 - bounds.1 + padding.2 + padding.1,
            padding,
        });
    }
    units
}

/// What this cluster ranks over: its own direct members, then one stand-in per
/// nested child. A member claimed by a child belongs to that child, not here.
fn cluster_unit_ids(
    graph: &Graph,
    sub: &crate::ir::Subgraph,
    units: &[ClusterUnit],
    nodes: &BTreeMap<String, NodeLayout>,
) -> Vec<String> {
    let claimed: HashSet<&str> = units
        .iter()
        .flat_map(|unit| graph.subgraphs[unit.child].nodes.iter().map(String::as_str))
        .collect();
    let mut ids: Vec<String> = sub
        .nodes
        .iter()
        .filter(|member| !claimed.contains(member.as_str()) && nodes.contains_key(*member))
        .cloned()
        .collect();
    ids.extend(units.iter().map(|unit| unit.id.clone()));
    ids
}

/// The graph's edges rewritten onto those units, dropping any that does not
/// touch this cluster and any that becomes a self-edge.
///
/// An edge may name a child cluster rather than one of its members
/// (`B1 --> B2`). Those names are resolved from the subgraph itself: consulting
/// the node map instead would silently drop the edge whenever the map does not
/// happen to carry the anchor, which is exactly the case while sizing.
fn cluster_unit_edges(
    graph: &Graph,
    units: &[ClusterUnit],
    unit_ids: &[String],
) -> Vec<crate::ir::Edge> {
    let mut stands_for: HashMap<&str, &str> = HashMap::new();
    for unit in units {
        let child = &graph.subgraphs[unit.child];
        for member in &child.nodes {
            stands_for.insert(member.as_str(), unit.id.as_str());
        }
        for alias in [child.id.as_deref(), Some(child.label.as_str())]
            .into_iter()
            .flatten()
        {
            if !child.nodes.iter().any(|member| member == alias) {
                stands_for.insert(alias, unit.id.as_str());
            }
        }
    }
    let resolve = |id: &str| -> Option<String> {
        stands_for
            .get(id)
            .map(|unit| (*unit).to_string())
            .or_else(|| unit_ids.iter().find(|known| known.as_str() == id).cloned())
    };

    let mut edges = Vec::new();
    for edge in &graph.edges {
        let (Some(from), Some(to)) = (resolve(&edge.from), resolve(&edge.to)) else {
            continue;
        };
        if from != to {
            let mut mapped = edge.clone();
            mapped.from = from;
            mapped.to = to;
            edges.push(mapped);
        }
    }
    edges
}

/// Declaration order for the units, so ranking ties break the way the source
/// reads.
fn cluster_unit_order(graph: &Graph, unit_ids: &[String]) -> HashMap<String, usize> {
    unit_ids
        .iter()
        .enumerate()
        .map(|(position, id)| {
            let key = graph.node_order.get(id).copied().unwrap_or(position);
            (id.clone(), key)
        })
        .collect()
}

/// Put each child into `nodes` as a hidden node of the child's own size, so the
/// placement pass treats it as one block. Returns what to put back afterwards.
fn insert_cluster_placeholders(
    graph: &Graph,
    units: &[ClusterUnit],
    nodes: &mut BTreeMap<String, NodeLayout>,
) -> Vec<(String, Option<NodeLayout>)> {
    let mut borrowed = Vec::new();
    for unit in units {
        // Clone a real member so the stand-in carries valid defaults for every
        // field the placement pass reads; only id/size/visibility matter here.
        let Some(mut placeholder) = graph.subgraphs[unit.child]
            .nodes
            .iter()
            .find_map(|member| nodes.get(member))
            .cloned()
        else {
            continue;
        };
        placeholder.id = unit.id.clone();
        placeholder.x = 0.0;
        placeholder.y = 0.0;
        placeholder.width = unit.width;
        placeholder.height = unit.height;
        placeholder.hidden = true;
        placeholder.anchor_subgraph = None;
        borrowed.push((unit.id.clone(), nodes.insert(unit.id.clone(), placeholder)));
    }
    borrowed
}

fn restore_borrowed_nodes(
    nodes: &mut BTreeMap<String, NodeLayout>,
    borrowed: Vec<(String, Option<NodeLayout>)>,
) {
    for (id, previous) in borrowed {
        match previous {
            Some(node) => nodes.insert(id, node),
            None => nodes.remove(&id),
        };
    }
}

/// Translate a child's members rigidly from where they were laid out to the
/// slot its stand-in ended up in, returning the rect its frame occupies.
fn move_child_into_slot(
    graph: &Graph,
    unit: &ClusterUnit,
    nodes: &mut BTreeMap<String, NodeLayout>,
) -> Option<(f32, f32, f32, f32)> {
    let placed = nodes.get(unit.id.as_str())?;
    let rect = (
        placed.x,
        placed.y,
        placed.x + placed.width,
        placed.y + placed.height,
    );
    let (target_x, target_y) = (rect.0 + unit.padding.0, rect.1 + unit.padding.2);
    let members = &graph.subgraphs[unit.child].nodes;
    let (min_x, min_y, _, _) = super::node_bounds(nodes, members)?;
    let (dx, dy) = (target_x - min_x, target_y - min_y);
    for member in members {
        if let Some(node) = nodes.get_mut(member) {
            node.x += dx;
            node.y += dy;
        }
    }
    Some(rect)
}

fn estimate_subgraph_box_size(
    graph: &Graph,
    sub_idx: usize,
    sub: &crate::ir::Subgraph,
    tree: &SubgraphTree,
    nodes: &BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
    anchorable: bool,
) -> Option<(f32, f32, f32, f32)> {
    if sub.nodes.is_empty() {
        return None;
    }
    // Size the cluster with the same routine that will later place it, so the
    // anchor standing in for it during the parent's layout is exactly the size
    // the cluster really occupies. Anything else leaves neighbours ranked
    // against a box that does not exist.
    let mut temp_nodes: BTreeMap<String, NodeLayout> = BTreeMap::new();
    for node_id in &sub.nodes {
        if let Some(node) = nodes.get(node_id) {
            let mut clone = node.clone();
            clone.x = 0.0;
            clone.y = 0.0;
            temp_nodes.insert(node_id.clone(), clone);
        }
    }
    let (min_x, min_y, max_x, max_y) = layout_cluster_contents(
        graph,
        sub_idx,
        tree,
        &mut temp_nodes,
        theme,
        config,
        anchorable,
        (0.0, 0.0),
    )?;
    let (_, (padding_x, padding_y, top_padding)) =
        subgraph_label_metrics(graph, sub, theme, config);

    let width = (max_x - min_x) + padding_x * 2.0;
    let height = (max_y - min_y) + padding_y + top_padding;
    Some((width, height, padding_x, top_padding))
}

pub(super) fn apply_subgraph_anchor_sizes(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) -> HashMap<String, SubgraphAnchorInfo> {
    let mut anchors: HashMap<String, SubgraphAnchorInfo> = HashMap::new();
    if graph.subgraphs.is_empty() {
        return anchors;
    }
    let tree = SubgraphTree::build(graph);
    for (idx, sub) in graph.subgraphs.iter().enumerate() {
        if is_region_subgraph(sub) || !subgraph_should_anchor(sub, graph, nodes) {
            continue;
        }
        let Some(anchor_id) = subgraph_anchor_id(sub, nodes) else {
            continue;
        };
        let Some((width, height, padding_x, top_padding)) =
            estimate_subgraph_box_size(graph, idx, sub, &tree, nodes, theme, config, true)
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

pub(super) fn align_subgraphs_to_anchor_nodes(
    graph: &Graph,
    anchor_info: &HashMap<String, SubgraphAnchorInfo>,
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
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
        // A nested cluster is placed by its parent's recursion, as a rigid
        // block. Re-placing it here from its own anchor -- which came from the
        // top-level rank graph and knows nothing of the parent's interior --
        // would tear it back out of that arrangement.
        if tree
            .parent
            .get(info.sub_idx)
            .and_then(|parent| *parent)
            .is_some()
        {
            if let Some(sub) = graph.subgraphs.get(info.sub_idx) {
                anchored_nodes.extend(sub.nodes.iter().cloned());
            }
            continue;
        }

        let (anchor_x, anchor_y) = {
            let Some(anchor) = nodes.get(anchor_id) else {
                continue;
            };
            (anchor.x, anchor.y)
        };
        let Some(sub) = graph.subgraphs.get(info.sub_idx) else {
            continue;
        };
        layout_cluster_contents(
            graph,
            info.sub_idx,
            &tree,
            nodes,
            theme,
            config,
            true,
            (anchor_x + info.padding_x, anchor_y + info.top_padding),
        );
        anchored_nodes.extend(sub.nodes.iter().cloned());
    }
    anchored_nodes
}

pub(super) fn apply_state_subgraph_layouts(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
    skip_indices: &HashSet<usize>,
) {
    let sub_count = graph.subgraphs.len();
    let mut depth: Vec<usize> = vec![0; sub_count];
    let mut parent_of: Vec<Option<usize>> = vec![None; sub_count];

    for (i, sub_a) in graph.subgraphs.iter().enumerate() {
        for (j, sub_b) in graph.subgraphs.iter().enumerate() {
            if i == j {
                continue;
            }
            let b_id = sub_b.id.as_deref().unwrap_or("");
            if (sub_a.nodes.iter().any(|n| n == b_id)
                || sub_a.nodes.iter().any(|n| n == &sub_b.label))
                && parent_of[j].is_none()
            {
                parent_of[j] = Some(i);
            }
        }
    }

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

    let mut order: Vec<usize> = (0..sub_count).collect();
    order.sort_by(|a, b| depth[*b].cmp(&depth[*a]));

    let mut inner_boxes: HashMap<usize, (f32, f32, f32, f32)> = HashMap::new();

    for idx in order {
        let sub = &graph.subgraphs[idx];
        if skip_indices.contains(&idx) || sub.nodes.len() <= 1 {
            continue;
        }
        let Some((min_x, min_y, _, _)) = super::node_bounds(nodes, &sub.nodes) else {
            continue;
        };

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
        super::assign_positions(
            &sub.nodes,
            &ranks,
            graph.direction,
            config,
            nodes,
            min_x,
            min_y,
        );

        let nested_anchor_min_y = min_y + (config.node_spacing * 0.4).max(20.0);
        for anchor_id in &inner_anchor_ids {
            if let Some(anchor) = nodes.get_mut(anchor_id)
                && anchor.y < nested_anchor_min_y
            {
                anchor.y = nested_anchor_min_y;
            }
        }

        for (id, w, h) in saved_sizes {
            if let Some(node) = nodes.get_mut(&id) {
                node.width = w;
                node.height = h;
            }
        }

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
        for (j, inner_sub) in graph.subgraphs.iter().enumerate() {
            if inner_boxes.contains_key(&j) {
                let inner_id = inner_sub.id.as_deref().unwrap_or("");
                if sub
                    .nodes
                    .iter()
                    .any(|n| n == inner_id || n == &inner_sub.label)
                {
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

pub(super) fn apply_subgraph_anchors(
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

pub(super) fn anchor_layout_for_edge(
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

pub(super) fn anchor_layout_for_edge_towards(
    anchor: &NodeLayout,
    subgraph: &SubgraphLayout,
    remote: &NodeLayout,
    direction: Direction,
    is_from: bool,
) -> NodeLayout {
    let remote_center = (
        remote.x + remote.width * 0.5,
        remote.y + remote.height * 0.5,
    );
    let inside_x = remote_center.0 >= subgraph.x && remote_center.0 <= subgraph.x + subgraph.width;
    let inside_y = remote_center.1 >= subgraph.y && remote_center.1 <= subgraph.y + subgraph.height;
    if inside_x && inside_y {
        return anchor_layout_for_edge(anchor, subgraph, direction, is_from);
    }

    let size = 2.0;
    let min_x = subgraph.x;
    let max_x = subgraph.x + subgraph.width;
    let min_y = subgraph.y;
    let max_y = subgraph.y + subgraph.height;
    let pad = (subgraph.width.min(subgraph.height) * 0.08).clamp(0.0, 18.0);
    let clamp_x = |x: f32| x.clamp(min_x + pad, max_x - pad);
    let clamp_y = |y: f32| y.clamp(min_y + pad, max_y - pad);
    let preferred_side = match (is_horizontal(direction), is_from) {
        (true, true) => EdgeSide::Right,
        (true, false) => EdgeSide::Left,
        (false, true) => EdgeSide::Bottom,
        (false, false) => EdgeSide::Top,
    };

    let candidates = [
        (EdgeSide::Left, (min_x, clamp_y(remote_center.1))),
        (EdgeSide::Right, (max_x, clamp_y(remote_center.1))),
        (EdgeSide::Top, (clamp_x(remote_center.0), min_y)),
        (EdgeSide::Bottom, (clamp_x(remote_center.0), max_y)),
    ];
    let mut best = candidates[0];
    let mut best_score = f32::INFINITY;
    for (side, point) in candidates {
        let manhattan = (point.0 - remote_center.0).abs() + (point.1 - remote_center.1).abs();
        let tie_bias = if side == preferred_side { -0.01 } else { 0.0 };
        let score = manhattan + tie_bias;
        if score < best_score {
            best = (side, point);
            best_score = score;
        }
    }

    let mut node = anchor.clone();
    node.width = size;
    node.height = size;
    match best.0 {
        EdgeSide::Left => {
            node.x = min_x;
            node.y = best.1.1 - size * 0.5;
        }
        EdgeSide::Right => {
            node.x = max_x - size;
            node.y = best.1.1 - size * 0.5;
        }
        EdgeSide::Top => {
            node.x = best.1.0 - size * 0.5;
            node.y = min_y;
        }
        EdgeSide::Bottom => {
            node.x = best.1.0 - size * 0.5;
            node.y = max_y - size;
        }
    }
    node
}

fn mirror_subgraph_nodes(
    node_ids: &[String],
    nodes: &mut BTreeMap<String, NodeLayout>,
    direction: Direction,
) {
    let Some((min_x, min_y, max_x, max_y)) = super::node_bounds(nodes, node_ids) else {
        return;
    };

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

pub(super) fn resolve_subgraph_style(
    sub: &crate::ir::Subgraph,
    graph: &Graph,
) -> crate::ir::NodeStyle {
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

pub(super) fn build_subgraph_layouts(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) -> Vec<SubgraphLayout> {
    let (mut subgraphs, _) = build_subgraph_layouts_indexed(graph, nodes, theme, config);
    subgraphs.sort_by(|a, b| {
        let area_a = a.width * a.height;
        let area_b = b.width * b.height;
        area_b.partial_cmp(&area_a).unwrap_or(Ordering::Equal)
    });
    subgraphs
}

/// The rect each subgraph will actually be drawn with, keyed by its index in
/// `graph.subgraphs` (`None` when the subgraph has no placed member). Spacing
/// passes need this rather than a frame's own members: a parent grows to
/// contain its nested children, and a pass that measures only the members it
/// declares will leave two frames overlapping.
pub(in crate::layout) fn rendered_subgraph_rects(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) -> Vec<Option<[f32; 4]>> {
    let (subgraphs, graph_to_local) = build_subgraph_layouts_indexed(graph, nodes, theme, config);
    graph_to_local
        .into_iter()
        .map(|local| {
            local.map(|local| {
                let sub = &subgraphs[local];
                [sub.x, sub.y, sub.x + sub.width, sub.y + sub.height]
            })
        })
        .collect()
}

/// Shared body of the two above: frames in `graph.subgraphs` order, plus the
/// index mapping that ordering implies.
fn build_subgraph_layouts_indexed(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) -> (Vec<SubgraphLayout>, Vec<Option<usize>>) {
    let mut subgraphs = Vec::new();
    // Maps graph.subgraphs index -> local subgraphs index (None if skipped).
    let mut graph_to_local: Vec<Option<usize>> = Vec::with_capacity(graph.subgraphs.len());
    // Title room each frame reserves above its contents, kept so that growing a
    // parent around a nested child cannot swallow the parent's own title band.
    let mut top_paddings: Vec<f32> = Vec::with_capacity(graph.subgraphs.len());
    for sub in &graph.subgraphs {
        let Some((min_x, min_y, max_x, max_y)) = super::node_bounds(nodes, &sub.nodes) else {
            graph_to_local.push(None);
            top_paddings.push(0.0);
            continue;
        };

        let style = resolve_subgraph_style(sub, graph);
        let label_empty = sub.label.trim().is_empty();
        let (label_block, (padding_x, padding_y, top_padding)) =
            subgraph_label_metrics(graph, sub, theme, config);

        let node_width = max_x - min_x;
        let base_width = node_width + padding_x * 2.0;
        let min_label_width = if label_empty {
            base_width
        } else {
            label_block.width + padding_x * 2.0
        };
        let width = base_width.max(min_label_width);
        let extra_width = width - base_width;

        graph_to_local.push(Some(subgraphs.len()));
        top_paddings.push(top_padding);
        subgraphs.push(SubgraphLayout {
            id: sub.id.clone(),
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
        let n = graph.subgraphs.len();
        let mut all_descendants: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut order: Vec<usize> = Vec::with_capacity(n);
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

        for &idx in &order {
            let mut descs = Vec::new();
            for &child in &tree.children[idx] {
                descs.push(child);
                descs.extend(all_descendants[child].iter().copied());
            }
            all_descendants[idx] = descs;
        }

        for &i in &order {
            let Some(local_i) = graph_to_local[i] else {
                continue;
            };
            for &j in &all_descendants[i] {
                if is_region_subgraph(&graph.subgraphs[j]) {
                    continue;
                }
                let Some(local_j) = graph_to_local[j] else {
                    continue;
                };
                let pad = if graph.kind == crate::ir::DiagramKind::State {
                    (theme.font_size * 1.8).max(24.0)
                } else {
                    12.0
                };
                let (child_x, child_y, child_w, child_h) = {
                    let child = &subgraphs[local_j];
                    (child.x, child.y, child.width, child.height)
                };
                // The top side is not free space: it carries the parent's own
                // title. Clearing the child by `pad` there would let the child
                // box slide up under the parent's label.
                let top_pad = top_paddings[i].max(pad);
                let parent = &mut subgraphs[local_i];
                let min_x = parent.x.min(child_x - pad);
                let min_y = parent.y.min(child_y - top_pad);
                let max_x = (parent.x + parent.width).max(child_x + child_w + pad);
                let max_y = (parent.y + parent.height).max(child_y + child_h + pad);
                parent.x = min_x;
                parent.y = min_y;
                parent.width = max_x - min_x;
                parent.height = max_y - min_y;
            }
        }
    }

    (subgraphs, graph_to_local)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Direction, Edge, EdgeStyle, Graph, NodeShape, Subgraph};

    fn make_node(id: &str, x: f32, y: f32, width: f32, height: f32) -> NodeLayout {
        NodeLayout {
            id: id.to_string(),
            x,
            y,
            width,
            height,
            label: TextBlock {
                lines: vec![id.to_string()],
                width: width * 0.5,
                height: height * 0.4,
            },
            shape: NodeShape::Rectangle,
            style: crate::ir::NodeStyle::default(),
            link: None,
            anchor_subgraph: None,
            hidden: false,
            icon: None,
        }
    }

    fn add_node(
        graph: &mut Graph,
        nodes: &mut BTreeMap<String, NodeLayout>,
        id: &str,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        graph.ensure_node(id, Some(id.to_string()), Some(NodeShape::Rectangle));
        nodes.insert(id.to_string(), make_node(id, x, y, width, height));
    }

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

    #[test]
    fn subgraph_anchor_id_prefers_explicit_id_before_label() {
        let mut graph = Graph::new();
        let mut nodes = BTreeMap::new();
        add_node(&mut graph, &mut nodes, "cluster_1", 0.0, 0.0, 20.0, 20.0);
        add_node(&mut graph, &mut nodes, "Cluster", 30.0, 0.0, 20.0, 20.0);
        let sub = Subgraph {
            id: Some("cluster_1".to_string()),
            label: "Cluster".to_string(),
            nodes: vec!["A".to_string(), "B".to_string()],
            direction: None,
            icon: None,
        };

        assert_eq!(subgraph_anchor_id(&sub, &nodes), Some("cluster_1"));
    }

    #[test]
    fn subgraph_anchor_id_ignores_anchor_when_it_is_a_member() {
        let mut graph = Graph::new();
        let mut nodes = BTreeMap::new();
        add_node(&mut graph, &mut nodes, "cluster_1", 0.0, 0.0, 20.0, 20.0);
        let sub = Subgraph {
            id: Some("cluster_1".to_string()),
            label: "Cluster".to_string(),
            nodes: vec!["cluster_1".to_string(), "A".to_string()],
            direction: None,
            icon: None,
        };

        assert_eq!(subgraph_anchor_id(&sub, &nodes), None);
    }

    #[test]
    fn apply_subgraph_anchors_marks_external_anchor_nodes() {
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let mut graph = Graph::new();
        let mut nodes = BTreeMap::new();
        add_node(&mut graph, &mut nodes, "A", 0.0, 0.0, 40.0, 24.0);
        add_node(&mut graph, &mut nodes, "B", 60.0, 0.0, 40.0, 24.0);
        add_node(
            &mut graph,
            &mut nodes,
            "cluster_anchor",
            160.0,
            80.0,
            24.0,
            24.0,
        );
        graph.subgraphs.push(Subgraph {
            id: Some("cluster_anchor".to_string()),
            label: "Cluster".to_string(),
            nodes: vec!["A".to_string(), "B".to_string()],
            direction: None,
            icon: None,
        });

        let subgraphs = build_subgraph_layouts(&graph, &nodes, &theme, &config);
        apply_subgraph_anchors(&graph, &subgraphs, &mut nodes);

        let anchor = nodes.get("cluster_anchor").expect("anchor node");
        assert_eq!(anchor.anchor_subgraph, Some(0));
        assert!((anchor.width - 2.0).abs() < 1e-3);
        assert!((anchor.height - 2.0).abs() < 1e-3);
    }

    #[test]
    fn anchor_layout_for_edge_uses_expected_perimeter_side() {
        let anchor = make_node("anchor", 0.0, 0.0, 10.0, 10.0);
        let subgraph = SubgraphLayout {
            id: None,
            label: "Cluster".to_string(),
            label_block: TextBlock {
                lines: vec!["Cluster".to_string()],
                width: 40.0,
                height: 12.0,
            },
            nodes: vec!["A".to_string(), "B".to_string()],
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 60.0,
            style: crate::ir::NodeStyle::default(),
            icon: None,
        };

        let lr_from = anchor_layout_for_edge(&anchor, &subgraph, Direction::LeftRight, true);
        assert!((lr_from.x - 108.0).abs() < 1e-3);
        assert!((lr_from.y - 49.0).abs() < 1e-3);

        let td_to = anchor_layout_for_edge(&anchor, &subgraph, Direction::TopDown, false);
        assert!((td_to.x - 59.0).abs() < 1e-3);
        assert!((td_to.y - 20.0).abs() < 1e-3);
    }

    #[test]
    fn anchor_layout_for_edge_towards_uses_nearest_boundary_port() {
        let anchor = make_node("anchor", 0.0, 0.0, 10.0, 10.0);
        let subgraph = SubgraphLayout {
            id: None,
            label: "Cluster".to_string(),
            label_block: TextBlock {
                lines: vec!["Cluster".to_string()],
                width: 40.0,
                height: 12.0,
            },
            nodes: vec!["A".to_string(), "B".to_string()],
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 60.0,
            style: crate::ir::NodeStyle::default(),
            icon: None,
        };
        let remote_left = make_node("remote", -80.0, 45.0, 20.0, 20.0);
        let remote_bottom = make_node("remote", 58.0, 140.0, 20.0, 20.0);

        let left = anchor_layout_for_edge_towards(
            &anchor,
            &subgraph,
            &remote_left,
            Direction::LeftRight,
            true,
        );
        assert!((left.x - 10.0).abs() < 1e-3);
        assert!((left.y + left.height * 0.5 - 55.0).abs() < 1.0);

        let bottom = anchor_layout_for_edge_towards(
            &anchor,
            &subgraph,
            &remote_bottom,
            Direction::LeftRight,
            false,
        );
        assert!((bottom.y + bottom.height - 80.0).abs() < 1e-3);
        assert!((bottom.x + bottom.width * 0.5 - 68.0).abs() < 1.0);
    }

    #[test]
    fn build_subgraph_layouts_expands_parent_to_contain_nested_child_layout() {
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let mut graph = Graph::new();
        let mut nodes = BTreeMap::new();
        add_node(&mut graph, &mut nodes, "A", 0.0, 0.0, 30.0, 24.0);
        add_node(&mut graph, &mut nodes, "B", 40.0, 0.0, 30.0, 24.0);
        add_node(&mut graph, &mut nodes, "C", 160.0, 0.0, 30.0, 24.0);
        graph.subgraphs.push(Subgraph {
            id: Some("parent".to_string()),
            label: "Parent".to_string(),
            nodes: vec!["A".to_string(), "B".to_string(), "C".to_string()],
            direction: None,
            icon: None,
        });
        graph.subgraphs.push(Subgraph {
            id: Some("child".to_string()),
            label: "A very wide child label".to_string(),
            nodes: vec!["A".to_string(), "B".to_string()],
            direction: None,
            icon: None,
        });

        let subgraphs = build_subgraph_layouts(&graph, &nodes, &theme, &config);
        let parent = subgraphs
            .iter()
            .find(|sub| sub.label == "Parent")
            .expect("parent layout");
        let child = subgraphs
            .iter()
            .find(|sub| sub.label == "A very wide child label")
            .expect("child layout");

        assert!(
            parent.x <= child.x - 11.5,
            "parent should expand left to contain child layout padding"
        );
        assert!(
            parent.x + parent.width >= child.x + child.width + 11.5,
            "parent should expand right to contain child layout padding"
        );
    }

    #[test]
    fn apply_subgraph_bands_aligns_path_like_groups_on_cross_axis() {
        let mut graph = Graph::new();
        graph.kind = crate::ir::DiagramKind::Flowchart;
        graph.direction = Direction::LeftRight;
        let mut nodes = BTreeMap::new();
        add_node(&mut graph, &mut nodes, "A", 0.0, 10.0, 30.0, 20.0);
        add_node(&mut graph, &mut nodes, "B", 40.0, 10.0, 30.0, 20.0);
        add_node(&mut graph, &mut nodes, "C", 120.0, 90.0, 30.0, 20.0);
        add_node(&mut graph, &mut nodes, "D", 160.0, 90.0, 30.0, 20.0);
        graph.subgraphs.push(Subgraph {
            id: Some("sg1".to_string()),
            label: "One".to_string(),
            nodes: vec!["A".to_string(), "B".to_string()],
            direction: None,
            icon: None,
        });
        graph.subgraphs.push(Subgraph {
            id: Some("sg2".to_string()),
            label: "Two".to_string(),
            nodes: vec!["C".to_string(), "D".to_string()],
            direction: None,
            icon: None,
        });
        graph.edges.push(edge("B", "C"));

        apply_subgraph_bands(&graph, &mut nodes, &LayoutConfig::default());

        let a = nodes.get("A").unwrap();
        let c = nodes.get("C").unwrap();
        assert!((a.y - c.y).abs() < 1.0);
    }

    #[test]
    fn apply_orthogonal_region_bands_separates_sibling_regions() {
        let mut graph = Graph::new();
        graph.kind = crate::ir::DiagramKind::Flowchart;
        graph.direction = Direction::LeftRight;
        let mut nodes = BTreeMap::new();
        add_node(&mut graph, &mut nodes, "A", 0.0, 0.0, 30.0, 20.0);
        add_node(&mut graph, &mut nodes, "B", 35.0, 0.0, 30.0, 20.0);
        add_node(&mut graph, &mut nodes, "C", 10.0, 40.0, 30.0, 20.0);
        add_node(&mut graph, &mut nodes, "D", 45.0, 40.0, 30.0, 20.0);
        graph.subgraphs.push(Subgraph {
            id: Some("parent".to_string()),
            label: "Parent".to_string(),
            nodes: vec![
                "A".to_string(),
                "B".to_string(),
                "C".to_string(),
                "D".to_string(),
            ],
            direction: None,
            icon: None,
        });
        graph.subgraphs.push(Subgraph {
            id: Some("__region_1".to_string()),
            label: "".to_string(),
            nodes: vec!["A".to_string(), "B".to_string()],
            direction: None,
            icon: None,
        });
        graph.subgraphs.push(Subgraph {
            id: Some("__region_2".to_string()),
            label: "".to_string(),
            nodes: vec!["C".to_string(), "D".to_string()],
            direction: None,
            icon: None,
        });
        let mut config = LayoutConfig::default();
        config.rank_spacing = 40.0;

        apply_orthogonal_region_bands(&graph, &mut nodes, &config);

        let region1_max_x = ["A", "B"]
            .iter()
            .filter_map(|id| nodes.get(*id))
            .map(|node| node.x + node.width)
            .fold(f32::MIN, f32::max);
        let region2_min_x = ["C", "D"]
            .iter()
            .filter_map(|id| nodes.get(*id))
            .map(|node| node.x)
            .fold(f32::MAX, f32::min);

        assert!(
            region2_min_x >= region1_max_x + config.rank_spacing * 0.6 - 1.0,
            "region bands should separate sibling regions along the main axis"
        );
    }

    #[test]
    fn apply_subgraph_direction_overrides_honors_explicit_lr_direction() {
        let mut graph = Graph::new();
        graph.kind = crate::ir::DiagramKind::Flowchart;
        graph.direction = Direction::TopDown;
        let mut nodes = BTreeMap::new();
        add_node(&mut graph, &mut nodes, "A", 0.0, 0.0, 60.0, 36.0);
        add_node(&mut graph, &mut nodes, "B", 0.0, 80.0, 60.0, 36.0);
        add_node(&mut graph, &mut nodes, "C", 0.0, 160.0, 60.0, 36.0);
        graph.edges.push(edge("A", "B"));
        graph.edges.push(edge("B", "C"));
        graph.subgraphs.push(Subgraph {
            id: Some("sg".to_string()),
            label: "Loop".to_string(),
            nodes: vec!["A".to_string(), "B".to_string(), "C".to_string()],
            direction: Some(Direction::LeftRight),
            icon: None,
        });

        apply_subgraph_direction_overrides(
            &graph,
            &mut nodes,
            &Theme::modern(),
            &LayoutConfig::default(),
            &HashSet::new(),
        );

        let a = nodes.get("A").unwrap();
        let b = nodes.get("B").unwrap();
        let c = nodes.get("C").unwrap();
        assert!(a.x < b.x && b.x < c.x);
        assert!((a.y - b.y).abs() < 1.0);
        assert!((b.y - c.y).abs() < 1.0);
    }
}
