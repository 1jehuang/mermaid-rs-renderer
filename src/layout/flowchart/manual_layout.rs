use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::config::{FlowchartLayoutEngine, LayoutConfig};
use crate::ir::{DiagramKind, Graph};
use crate::theme::Theme;

use super::super::label_placement;
use super::super::ranking::{
    compute_ranks_dagre_style, compute_ranks_subset, order_rank_nodes,
    order_rank_nodes_dagre_style, rank_edges_for_manual_layout,
};
use super::super::{NodeLayout, TextBlock, is_horizontal};
use super::analysis::FlowchartRelationshipAnalysis;

const LABEL_RANK_FONT_SCALE: f32 = 0.5;
const LABEL_RANK_MIN_GAP: f32 = 8.0;
const EDGE_AWARE_CENTER_PULL_LIMIT_RATIO: f32 = 0.28;
const EDGE_AWARE_CENTER_PULL_BLEND: f32 = 0.35;
const FLOWCHART_LABEL_MAIN_GAP_PAD_RATIO: f32 = 0.35;
const FLOWCHART_LABEL_MAIN_GAP_MAX_SCALE: f32 = 3.0;

fn median_center(values: &mut [f32]) -> Option<f32> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let mid = values.len() / 2;
    Some(if values.len() % 2 == 1 {
        values[mid]
    } else {
        (values[mid - 1] + values[mid]) * 0.5
    })
}

fn weighted_median_center(values: &mut [(f32, f32)]) -> Option<f32> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
    let total_weight = values
        .iter()
        .map(|(_, weight)| weight.max(0.0))
        .sum::<f32>();
    if total_weight <= f32::EPSILON {
        return Some(values[values.len() / 2].0);
    }
    let threshold = total_weight * 0.5;
    let mut cumulative = 0.0;
    for idx in 0..values.len() {
        let (center, weight) = values[idx];
        cumulative += weight.max(0.0);
        if (cumulative - threshold).abs() <= f32::EPSILON {
            let next_center = values
                .iter()
                .skip(idx + 1)
                .find(|(_, next_weight)| *next_weight > 0.0)
                .map(|(next_center, _)| *next_center);
            return Some(next_center.map_or(center, |next| (center + next) * 0.5));
        }
        if cumulative > threshold {
            return Some(center);
        }
    }
    values.last().map(|(center, _)| *center)
}

fn bounded_edge_aware_desired_center(
    unweighted: f32,
    weighted: Option<f32>,
    current: f32,
    node_spacing: f32,
) -> f32 {
    let Some(weighted) = weighted else {
        return unweighted;
    };
    let max_pull = (node_spacing * EDGE_AWARE_CENTER_PULL_LIMIT_RATIO).max(4.0);
    let bounded_pull = (weighted - unweighted).clamp(-max_pull, max_pull);
    let relationship_target = unweighted + bounded_pull * EDGE_AWARE_CENTER_PULL_BLEND;
    relationship_target * 0.85 + current * 0.15
}

fn node_cross_spacing_half(
    node_id: &str,
    visual_half: f32,
    relationship_analysis: Option<&FlowchartRelationshipAnalysis>,
    config: &LayoutConfig,
) -> f32 {
    visual_half
        + relationship_analysis
            .and_then(|analysis| analysis.node_relation(node_id))
            .map(|relation| relation.extra_cross_padding(config))
            .unwrap_or(0.0)
}

fn flowchart_label_main_gap_budgets(
    graph: &Graph,
    layout_edges: &[crate::ir::Edge],
    edge_labels: &[Option<TextBlock>],
    shifted_ranks: &HashMap<String, usize>,
    config: &LayoutConfig,
) -> HashMap<usize, f32> {
    if graph.kind != DiagramKind::Flowchart {
        return HashMap::new();
    }

    let horizontal = is_horizontal(graph.direction);
    let (label_pad_x, label_pad_y) = label_placement::edge_label_padding(graph.kind, config);
    let mut gap_budgets: HashMap<usize, f32> = HashMap::new();
    let mut label_counts: HashMap<usize, usize> = HashMap::new();

    for (idx, edge) in layout_edges.iter().enumerate() {
        let Some(label) = edge_labels.get(idx).and_then(|label| label.as_ref()) else {
            continue;
        };
        if label.width <= 0.0 || label.height <= 0.0 {
            continue;
        }
        let Some(&from_rank) = shifted_ranks.get(&edge.from) else {
            continue;
        };
        let Some(&to_rank) = shifted_ranks.get(&edge.to) else {
            continue;
        };
        let lo = from_rank.min(to_rank);
        let hi = from_rank.max(to_rank);
        if hi <= lo {
            continue;
        }

        let gap_after_rank = lo + (hi - lo - 1) / 2;
        let label_main = if horizontal {
            label.width + 2.0 * label_pad_x
        } else {
            label.height + 2.0 * label_pad_y
        };
        let label_cross = if horizontal {
            label.height + 2.0 * label_pad_y
        } else {
            label.width + 2.0 * label_pad_x
        };
        let stack_index = label_counts.entry(gap_after_rank).or_insert(0);
        let stacked_cross_allowance = *stack_index as f32 * label_cross * 0.35;
        *stack_index += 1;

        let desired_gap = (label_main
            + config.node_spacing * FLOWCHART_LABEL_MAIN_GAP_PAD_RATIO
            + stacked_cross_allowance)
            .min(config.rank_spacing * FLOWCHART_LABEL_MAIN_GAP_MAX_SCALE)
            .max(config.rank_spacing);
        let extra = (desired_gap - config.rank_spacing).max(0.0);
        gap_budgets
            .entry(gap_after_rank)
            .and_modify(|current| *current = current.max(extra))
            .or_insert(extra);
    }

    gap_budgets
}

fn build_ordering_edges(
    layout_edges: &[crate::ir::Edge],
    shifted_ranks: &HashMap<String, usize>,
    rank_nodes: &mut [Vec<String>],
    order_map: &mut HashMap<String, usize>,
    label_dummy_at_rank: &HashMap<usize, (usize, String)>,
    dummy_counter: &mut usize,
) -> Vec<crate::ir::Edge> {
    let mut ordering_edges: Vec<crate::ir::Edge> = Vec::new();

    for (edge_idx, edge) in layout_edges.iter().enumerate() {
        let Some(&from_rank) = shifted_ranks.get(&edge.from) else {
            continue;
        };
        let Some(&to_rank) = shifted_ranks.get(&edge.to) else {
            continue;
        };
        if to_rank <= from_rank {
            ordering_edges.push(edge.clone());
            continue;
        }
        let span = to_rank - from_rank;
        if span <= 1 {
            ordering_edges.push(edge.clone());
            continue;
        }
        let label_dummy_info = label_dummy_at_rank.get(&edge_idx);
        let mut prev = edge.from.clone();
        for step in 1..span {
            let current_rank = from_rank + step;
            let dummy_id = if let Some((lr, lid)) = label_dummy_info {
                if current_rank == *lr {
                    lid.clone()
                } else {
                    let id = format!("__dummy_{}__", *dummy_counter);
                    *dummy_counter += 1;
                    let order_idx = order_map.len();
                    order_map.insert(id.clone(), order_idx);
                    if let Some(bucket) = rank_nodes.get_mut(current_rank) {
                        bucket.push(id.clone());
                    }
                    id
                }
            } else {
                let id = format!("__dummy_{}__", *dummy_counter);
                *dummy_counter += 1;
                let order_idx = order_map.len();
                order_map.insert(id.clone(), order_idx);
                if let Some(bucket) = rank_nodes.get_mut(current_rank) {
                    bucket.push(id.clone());
                }
                id
            };
            ordering_edges.push(crate::ir::Edge {
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
        ordering_edges.push(crate::ir::Edge {
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

    ordering_edges
}

/// Balance whole-rank offsets toward adjacent-rank neighbors without changing
/// within-rank order or spacing. This is a conservative Brandes-Kopf-inspired
/// coordinate refinement: ranks translate as rigid blocks, so it can improve
/// alignment and centerline length without introducing node overlap.
fn balance_rank_cross_offsets(
    rank_nodes: &[Vec<String>],
    ordering_edges: &[crate::ir::Edge],
    nodes: &mut BTreeMap<String, NodeLayout>,
    direction: crate::ir::Direction,
    node_spacing: f32,
    passes: usize,
) {
    if rank_nodes.len() <= 1 {
        return;
    }
    let mut rank_by_id = HashMap::new();
    for (rank, bucket) in rank_nodes.iter().enumerate() {
        for id in bucket {
            rank_by_id.insert(id.as_str(), rank);
        }
    }
    let cross_center = |node: &NodeLayout| {
        if is_horizontal(direction) {
            node.y + node.height / 2.0
        } else {
            node.x + node.width / 2.0
        }
    };
    let max_shift = (node_spacing * 1.5).max(12.0);
    for pass in 0..passes.max(2) {
        let ranks: Vec<usize> = if pass % 2 == 0 {
            (1..rank_nodes.len()).collect()
        } else {
            (1..rank_nodes.len()).rev().collect()
        };
        for rank in ranks {
            let mut deltas = Vec::new();
            for edge in ordering_edges {
                let Some(&from_rank) = rank_by_id.get(edge.from.as_str()) else {
                    continue;
                };
                let Some(&to_rank) = rank_by_id.get(edge.to.as_str()) else {
                    continue;
                };
                if from_rank == to_rank {
                    continue;
                }
                if from_rank == rank {
                    if let (Some(from), Some(to)) = (nodes.get(&edge.from), nodes.get(&edge.to)) {
                        deltas.push(cross_center(to) - cross_center(from));
                    }
                } else if to_rank == rank
                    && let (Some(from), Some(to)) = (nodes.get(&edge.from), nodes.get(&edge.to))
                {
                    deltas.push(cross_center(from) - cross_center(to));
                }
            }
            let Some(desired_shift) = median_center(&mut deltas) else {
                continue;
            };
            let shift = (desired_shift * 0.5).clamp(-max_shift, max_shift);
            if shift.abs() <= 1e-3 {
                continue;
            }
            for id in &rank_nodes[rank] {
                if let Some(node) = nodes.get_mut(id) {
                    if is_horizontal(direction) {
                        node.y += shift;
                    } else {
                        node.x += shift;
                    }
                }
            }
        }
    }
}

/// Ordered rank buckets produced by [`assign_positions_manual`], exposed so
/// later passes (e.g. the aspect-ratio band fold) can reuse the exact rank
/// structure instead of re-deriving it.
///
/// Buckets are in main-axis order and each bucket is in cross-axis order as
/// established by `order_rank_nodes`. Buckets may contain ordering-dummy ids
/// (`__dummy_*__`) that have no `NodeLayout` entry; consumers must skip ids
/// missing from the node map.
#[derive(Debug, Default)]
pub(in crate::layout) struct ManualLayoutRanks {
    pub(in crate::layout) rank_nodes: Vec<Vec<String>>,
    pub(in crate::layout) feedback_edges: Vec<(String, String)>,
}

pub(in crate::layout) fn assign_positions_manual(
    graph: &Graph,
    layout_node_ids: &[String],
    layout_set: &HashSet<String>,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
    layout_edges: &[crate::ir::Edge],
    theme: &Theme,
    pre_measured_labels: &[Option<TextBlock>],
    label_dummy_ids: &mut [Option<String>],
) -> ManualLayoutRanks {
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
    let (mut ranks, mut engine_feedback_edges) = match config.flowchart.engine {
        FlowchartLayoutEngine::Current => (
            compute_ranks_subset(layout_node_ids, &rank_edges, &graph.node_order),
            Vec::new(),
        ),
        FlowchartLayoutEngine::Dagre | FlowchartLayoutEngine::Auto => {
            compute_ranks_dagre_style(layout_node_ids, &rank_edges, &graph.node_order)
        }
    };
    let relationship_analysis = FlowchartRelationshipAnalysis::analyze(
        graph,
        layout_node_ids,
        &layout_edges,
        &edge_labels,
        &ranks,
    );
    if graph.kind == DiagramKind::Class {
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
                // Only classes that hang off the hierarchy alone are moved
                // beside it. A class that is also related to other
                // non-hierarchy classes keeps its rank: moving it would put
                // it level with, or below, classes its own edges rank it
                // above, turning a tidy chain into sideways and upward edges.
                let mut satellite = true;
                for edge in &layout_edges {
                    let other = if edge.from == *node_id {
                        &edge.to
                    } else if edge.to == *node_id {
                        &edge.from
                    } else {
                        continue;
                    };
                    if other == node_id {
                        continue;
                    }
                    if !hierarchy_nodes.contains(other) {
                        satellite = false;
                        break;
                    }
                    if let Some(rank) = ranks.get(other) {
                        sum += *rank as f32;
                        count += 1;
                    }
                }
                if !satellite || count == 0 {
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
        DiagramKind::Flowchart
            | DiagramKind::Class
            | DiagramKind::Er
            | DiagramKind::Requirement
            | DiagramKind::State
    );
    let gaps_needing_label_rank: Vec<usize> = if use_label_dummies {
        let mut gap_set: HashSet<usize> = HashSet::new();
        for (idx, edge) in layout_edges.iter().enumerate() {
            if edge_labels[idx].is_none() {
                continue;
            }
            let from_rank = ranks.get(&edge.from).copied().unwrap_or(0);
            let to_rank = ranks.get(&edge.to).copied().unwrap_or(0);
            let lo = from_rank.min(to_rank);
            let hi = from_rank.max(to_rank);
            if hi > lo {
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

    if total_new_ranks > 0 {
        let new_max_rank = max_rank + total_new_ranks;
        let mut new_rank_nodes: Vec<Vec<String>> = vec![Vec::new(); new_max_rank + 1];
        for (old_rank, bucket) in rank_nodes.iter().enumerate() {
            let new_rank = old_rank + rank_shift[old_rank];
            new_rank_nodes[new_rank] = bucket.clone();
        }
        rank_nodes = new_rank_nodes;
    }

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
            let label_rank = mid_gap + rank_shift[mid_gap] + 1;
            label_dummy_ranks.insert(label_rank);

            let dummy_id = format!("__elabel_{}_{}_{dummy_counter}__", edge.from, edge.to);
            dummy_counter += 1;
            let order_idx = order_map.len();
            order_map.insert(dummy_id.clone(), order_idx);

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

            if let Some(&orig_idx) = original_edge_indices.get(idx)
                && orig_idx < label_dummy_ids.len()
            {
                label_dummy_ids[orig_idx] = Some(dummy_id.clone());
            }

            if let Some(bucket) = rank_nodes.get_mut(label_rank) {
                bucket.push(dummy_id);
            }
        }
    }

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
        if let Some(&orig_idx) = original_edge_indices.get(idx)
            && let Some(Some(dummy_id)) = label_dummy_ids.get(orig_idx)
        {
            label_dummy_at_rank.insert(idx, (label_rank, dummy_id.clone()));
        }
    }

    let shifted_ranks: HashMap<String, usize> = ranks
        .iter()
        .map(|(id, &r)| (id.clone(), r + rank_shift[r]))
        .collect();
    let flowchart_label_gap_budgets = flowchart_label_main_gap_budgets(
        graph,
        &layout_edges,
        &edge_labels,
        &shifted_ranks,
        config,
    );

    let ordering_edges = build_ordering_edges(
        &layout_edges,
        &shifted_ranks,
        &mut rank_nodes,
        &mut order_map,
        &label_dummy_at_rank,
        &mut dummy_counter,
    );

    for bucket in &mut rank_nodes {
        bucket.sort_by_key(|id| order_map.get(id).copied().unwrap_or(usize::MAX));
    }
    match config.flowchart.engine {
        FlowchartLayoutEngine::Current => order_rank_nodes(
            &mut rank_nodes,
            &ordering_edges,
            &order_map,
            config.flowchart.order_passes,
        ),
        FlowchartLayoutEngine::Dagre | FlowchartLayoutEngine::Auto => order_rank_nodes_dagre_style(
            &mut rank_nodes,
            &ordering_edges,
            &order_map,
            config.flowchart.order_passes,
        ),
    }

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
            let gap = if is_label_rank {
                (theme.font_size * LABEL_RANK_FONT_SCALE).max(LABEL_RANK_MIN_GAP)
            } else {
                config.rank_spacing
            } + flowchart_label_gap_budgets
                .get(&rank_idx)
                .copied()
                .unwrap_or(0.0);
            main_cursor += max_main + gap;
        }
    }

    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    let mut incoming_weighted: HashMap<String, Vec<(String, f32)>> = HashMap::new();
    let mut outgoing_weighted: HashMap<String, Vec<(String, f32)>> = HashMap::new();
    for edge in &ordering_edges {
        incoming
            .entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());
        outgoing
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        let weight = relationship_analysis
            .as_ref()
            .map(|analysis| analysis.edge_weight_between(&edge.from, &edge.to))
            .unwrap_or(1.0);
        incoming_weighted
            .entry(edge.to.clone())
            .or_default()
            .push((edge.from.clone(), weight));
        outgoing_weighted
            .entry(edge.from.clone())
            .or_default()
            .push((edge.to.clone(), weight));
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

    let relationship_analysis_ref = relationship_analysis.as_ref();

    let mut place_rank = |rank_idx: usize,
                          use_incoming: bool,
                          nodes: &mut BTreeMap<String, NodeLayout>| {
        let bucket = &rank_nodes[rank_idx];
        if bucket.is_empty() {
            return;
        }
        let neighbors = if use_incoming { &incoming } else { &outgoing };
        let weighted_neighbors = if use_incoming {
            &incoming_weighted
        } else {
            &outgoing_weighted
        };
        let mut entries: Vec<(String, f32, f32, usize)> = Vec::new();
        for (idx, node_id) in bucket.iter().enumerate() {
            let Some(node) = nodes.get(node_id) else {
                continue;
            };
            let mut neighbor_centers: Vec<f32> = Vec::new();
            let mut weighted_neighbor_centers: Vec<(f32, f32)> = Vec::new();
            if let Some(list) = neighbors.get(node_id) {
                for neighbor_id in list {
                    if let Some(center) = cross_pos.get(neighbor_id) {
                        neighbor_centers.push(*center);
                    }
                }
            }
            if let Some(list) = weighted_neighbors.get(node_id) {
                for (neighbor_id, weight) in list {
                    if let Some(center) = cross_pos.get(neighbor_id) {
                        weighted_neighbor_centers.push((*center, *weight));
                    }
                }
            }
            let mut desired = if neighbor_centers.is_empty() {
                cross_pos.get(node_id).copied().unwrap_or(0.0)
            } else {
                median_center(&mut neighbor_centers)
                    .unwrap_or_else(|| cross_pos.get(node_id).copied().unwrap_or(0.0))
            };
            if let Some(current) = cross_pos.get(node_id) {
                if !neighbor_centers.is_empty() {
                    let weighted = weighted_median_center(&mut weighted_neighbor_centers);
                    desired = bounded_edge_aware_desired_center(
                        desired,
                        weighted,
                        *current,
                        config.node_spacing,
                    );
                } else {
                    desired = *current;
                }
            }
            let visual_half = if is_horizontal(graph.direction) {
                node.height / 2.0
            } else {
                node.width / 2.0
            };
            let half =
                node_cross_spacing_half(node_id, visual_half, relationship_analysis_ref, config);
            entries.push((node_id.clone(), desired, half, idx));
        }
        entries.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(Ordering::Equal)
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
    if matches!(
        config.flowchart.engine,
        FlowchartLayoutEngine::Dagre | FlowchartLayoutEngine::Auto
    ) {
        balance_rank_cross_offsets(
            &rank_nodes,
            &ordering_edges,
            nodes,
            graph.direction,
            config.node_spacing,
            config.flowchart.order_passes,
        );
    }
    let mut rank_by_id: HashMap<&str, usize> = HashMap::new();
    for (rank, bucket) in rank_nodes.iter().enumerate() {
        for id in bucket {
            rank_by_id.insert(id.as_str(), rank);
        }
    }
    engine_feedback_edges.extend(layout_edges.iter().filter_map(|edge| {
        let from_rank = rank_by_id.get(edge.from.as_str())?;
        let to_rank = rank_by_id.get(edge.to.as_str())?;
        (*to_rank <= *from_rank).then(|| (edge.from.clone(), edge.to.clone()))
    }));
    engine_feedback_edges.sort();
    engine_feedback_edges.dedup();
    ManualLayoutRanks {
        rank_nodes,
        feedback_edges: engine_feedback_edges,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node_layout(id: &str) -> NodeLayout {
        NodeLayout {
            id: id.to_string(),
            x: 0.0,
            y: 0.0,
            width: 60.0,
            height: 36.0,
            label: TextBlock {
                lines: vec![id.to_string()],
                width: 20.0,
                height: 14.0,
            },
            shape: crate::ir::NodeShape::Rectangle,
            style: crate::ir::NodeStyle::default(),
            link: None,
            anchor_subgraph: None,
            hidden: false,
            icon: None,
        }
    }

    fn make_graph_node(id: &str) -> crate::ir::Node {
        crate::ir::Node {
            id: id.to_string(),
            label: id.to_string(),
            shape: crate::ir::NodeShape::Rectangle,
            value: None,
            icon: None,
        }
    }

    fn make_edge(from: &str, to: &str, label: Option<&str>) -> crate::ir::Edge {
        crate::ir::Edge {
            from: from.to_string(),
            to: to.to_string(),
            label: label.map(str::to_string),
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
        }
    }

    #[test]
    fn dagre_balance_translates_rank_toward_neighbors_without_changing_spacing() {
        let mut nodes = BTreeMap::new();
        let mut a = make_node_layout("A");
        a.x = 0.0;
        let mut b = make_node_layout("B");
        b.x = 120.0;
        let mut c = make_node_layout("C");
        c.x = 230.0;
        nodes.insert("A".into(), a);
        nodes.insert("B".into(), b);
        nodes.insert("C".into(), c);
        let ranks = vec![vec!["A".into()], vec!["B".into(), "C".into()]];
        let edges = vec![make_edge("A", "B", None)];
        let before_gap = nodes["C"].x - nodes["B"].x;
        let before_alignment = (nodes["B"].x - nodes["A"].x).abs();

        balance_rank_cross_offsets(
            &ranks,
            &edges,
            &mut nodes,
            crate::ir::Direction::TopDown,
            50.0,
            4,
        );

        assert!((nodes["C"].x - nodes["B"].x - before_gap).abs() < 1e-4);
        assert!((nodes["B"].x - nodes["A"].x).abs() < before_alignment);
    }

    #[test]
    fn weighted_median_center_preserves_equal_weight_median() {
        let mut two = [(0.0, 1.0), (100.0, 1.0)];
        assert_eq!(weighted_median_center(&mut two), Some(50.0));

        let mut four = [(0.0, 1.0), (40.0, 1.0), (80.0, 1.0), (120.0, 1.0)];
        assert_eq!(weighted_median_center(&mut four), Some(60.0));

        let mut weighted = [(0.0, 1.0), (100.0, 4.0)];
        assert_eq!(weighted_median_center(&mut weighted), Some(100.0));
    }

    #[test]
    fn flowchart_hub_relation_expands_cross_axis_spacing_half() {
        let mut graph = Graph::new();
        graph.kind = DiagramKind::Flowchart;
        for (idx, id) in ["A", "B", "H", "C", "D"].iter().enumerate() {
            graph.nodes.insert((*id).to_string(), make_graph_node(id));
            graph.node_order.insert((*id).to_string(), idx);
        }
        let edges = vec![
            make_edge("A", "H", None),
            make_edge("B", "H", None),
            make_edge("H", "C", Some("important label")),
            make_edge("H", "D", None),
        ];
        graph.edges = edges.clone();
        let labels = vec![
            None,
            None,
            Some(TextBlock {
                lines: vec!["important label".to_string()],
                width: 120.0,
                height: 20.0,
            }),
            None,
        ];
        let ranks: HashMap<String, usize> = [
            ("A".to_string(), 0),
            ("B".to_string(), 0),
            ("H".to_string(), 1),
            ("C".to_string(), 2),
            ("D".to_string(), 2),
        ]
        .into_iter()
        .collect();
        let node_ids = vec!["A", "B", "H", "C", "D"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let analysis =
            FlowchartRelationshipAnalysis::analyze(&graph, &node_ids, &edges, &labels, &ranks)
                .expect("flowchart analysis should run");
        let config = LayoutConfig::default();

        let hub_half = node_cross_spacing_half("H", 20.0, Some(&analysis), &config);
        let leaf_half = node_cross_spacing_half("A", 20.0, Some(&analysis), &config);
        assert!(
            hub_half > 20.0,
            "hub should reserve extra routing clearance"
        );
        assert_eq!(
            leaf_half, 20.0,
            "low-degree leaves should keep compact spacing"
        );
    }

    #[test]
    fn non_flowchart_layouts_can_insert_label_dummy_nodes() {
        let mut graph = Graph::new();
        graph.kind = DiagramKind::Packet;
        graph.direction = crate::ir::Direction::TopDown;
        for (idx, id) in ["A", "B", "C"].iter().enumerate() {
            graph.nodes.insert((*id).to_string(), make_graph_node(id));
            graph.node_order.insert((*id).to_string(), idx);
        }
        let edges = vec![
            make_edge("A", "B", None),
            make_edge("B", "C", Some("label")),
        ];

        let layout_node_ids = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let layout_set: HashSet<String> = layout_node_ids.iter().cloned().collect();
        let mut nodes = BTreeMap::new();
        for id in &layout_node_ids {
            nodes.insert(id.clone(), make_node_layout(id));
        }
        let pre_measured_labels = vec![
            None,
            Some(TextBlock {
                lines: vec!["label".to_string()],
                width: 44.0,
                height: 14.0,
            }),
        ];
        let mut label_dummy_ids = vec![None; edges.len()];

        assign_positions_manual(
            &graph,
            &layout_node_ids,
            &layout_set,
            &mut nodes,
            &LayoutConfig::default(),
            &edges,
            &Theme::modern(),
            &pre_measured_labels,
            &mut label_dummy_ids,
        );

        let dummy_id = label_dummy_ids[1].as_ref().expect("label dummy id");
        let dummy = nodes.get(dummy_id).expect("dummy node present");
        assert!(dummy.hidden);
    }

    #[test]
    fn flowchart_center_labels_expand_main_rank_gap_before_routing() {
        let mut graph = Graph::new();
        graph.kind = DiagramKind::Flowchart;
        graph.direction = crate::ir::Direction::LeftRight;
        for (idx, id) in ["A", "B"].iter().enumerate() {
            graph.nodes.insert((*id).to_string(), make_graph_node(id));
            graph.node_order.insert((*id).to_string(), idx);
        }
        let edges = vec![make_edge("A", "B", Some("long label"))];

        let layout_node_ids = vec!["A".to_string(), "B".to_string()];
        let layout_set: HashSet<String> = layout_node_ids.iter().cloned().collect();
        let mut nodes = BTreeMap::new();
        for id in &layout_node_ids {
            nodes.insert(id.clone(), make_node_layout(id));
        }
        let pre_measured_labels = vec![Some(TextBlock {
            lines: vec!["long label".to_string()],
            width: 180.0,
            height: 20.0,
        })];
        let mut label_dummy_ids = vec![None; edges.len()];
        let config = LayoutConfig::default();

        assign_positions_manual(
            &graph,
            &layout_node_ids,
            &layout_set,
            &mut nodes,
            &config,
            &edges,
            &Theme::modern(),
            &pre_measured_labels,
            &mut label_dummy_ids,
        );

        let a = nodes.get("A").expect("A layout");
        let b = nodes.get("B").expect("B layout");
        let gap = b.x - (a.x + a.width);
        assert_eq!(label_dummy_ids[0], None);
        assert!(
            gap > config.rank_spacing + 40.0,
            "flowchart label should reserve main-axis space before routing, gap={gap}"
        );
    }

    #[test]
    fn build_ordering_edges_retains_non_forward_edges() {
        let edges = vec![make_edge("A", "B", None), make_edge("B", "A", None)];
        let shifted_ranks = HashMap::from([("A".to_string(), 0usize), ("B".to_string(), 1usize)]);
        let mut rank_nodes = vec![vec!["A".to_string()], vec!["B".to_string()]];
        let mut order_map = HashMap::from([("A".to_string(), 0usize), ("B".to_string(), 1usize)]);
        let mut dummy_counter = 0usize;

        let ordering_edges = build_ordering_edges(
            &edges,
            &shifted_ranks,
            &mut rank_nodes,
            &mut order_map,
            &HashMap::new(),
            &mut dummy_counter,
        );

        assert!(
            ordering_edges
                .iter()
                .any(|edge| edge.from == "A" && edge.to == "B")
        );
        assert!(
            ordering_edges
                .iter()
                .any(|edge| edge.from == "B" && edge.to == "A")
        );
    }
}
