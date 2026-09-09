use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

use crate::ir::Graph;

pub(super) fn rank_edges_for_manual_layout(
    graph: &Graph,
    layout_node_ids: &[String],
    layout_edges: &[crate::ir::Edge],
) -> Vec<crate::ir::Edge> {
    if graph.kind != crate::ir::DiagramKind::Flowchart || layout_edges.len() < 3 {
        return layout_edges.to_vec();
    }

    let primary: Vec<crate::ir::Edge> = layout_edges
        .iter()
        .filter(|edge| edge.style != crate::ir::EdgeStyle::Dotted)
        .cloned()
        .collect();
    if primary.is_empty() {
        return layout_edges.to_vec();
    }

    let mut covered: HashSet<&str> = HashSet::new();
    for edge in &primary {
        covered.insert(edge.from.as_str());
        covered.insert(edge.to.as_str());
    }
    let min_covered = layout_node_ids.len().div_ceil(2);
    let base = if covered.len() >= min_covered {
        primary
    } else {
        layout_edges.to_vec()
    };

    with_subgraph_band_constraints(graph, layout_node_ids, layout_edges, base)
}

/// Cap on synthetic edges generated for one subgraph pair, so a pair of wide
/// frames cannot blow the rank graph up quadratically.
const MAX_BAND_CONSTRAINTS_PER_PAIR: usize = 64;

/// Keep the members of chained subgraphs in disjoint rank bands.
///
/// Rank assignment is otherwise subgraph-blind: a member with no incoming edge
/// (the top row of a frame) lands on rank 0 whichever subgraph it belongs to,
/// so `A -> B -> C` chains leave all three frames starting on the same rank and
/// overlapping. When a cross-subgraph edge orders two frames, every member of
/// the source frame should rank before every member of the target frame; edges
/// from the source's internal sinks to the target's internal sources impose
/// exactly that. These are ranking-only inputs and are never rendered.
///
/// Deliberately conservative: nested subgraphs, overlapping membership, and
/// mutually-connected frames are all left alone.
fn with_subgraph_band_constraints(
    graph: &Graph,
    layout_node_ids: &[String],
    layout_edges: &[crate::ir::Edge],
    mut edges: Vec<crate::ir::Edge>,
) -> Vec<crate::ir::Edge> {
    let top_level = super::subgraphs::top_level_subgraph_indices(graph);
    // Nested frames make "the band of a node" ambiguous, so only handle the
    // flat case where every declared subgraph is top level.
    if top_level.len() < 2 || top_level.len() != graph.subgraphs.len() {
        return edges;
    }

    let layout_set: HashSet<&str> = layout_node_ids.iter().map(String::as_str).collect();
    let mut owner: HashMap<&str, usize> = HashMap::new();
    for &idx in &top_level {
        for member in &graph.subgraphs[idx].nodes {
            if !layout_set.contains(member.as_str()) {
                continue;
            }
            if owner.insert(member.as_str(), idx).is_some() {
                // A node claimed by two frames: bands are not well defined.
                return edges;
            }
        }
    }

    // Order frames by the real edges running between them, ignoring any pair
    // that is connected both ways (that is a cycle, not a chain).
    let mut forward: HashSet<(usize, usize)> = HashSet::new();
    for edge in layout_edges {
        let (Some(&from_sg), Some(&to_sg)) =
            (owner.get(edge.from.as_str()), owner.get(edge.to.as_str()))
        else {
            continue;
        };
        if from_sg != to_sg {
            forward.insert((from_sg, to_sg));
        }
    }
    let mut pairs: Vec<(usize, usize)> = forward
        .iter()
        .copied()
        .filter(|&(a, b)| !forward.contains(&(b, a)))
        .collect();
    if pairs.is_empty() {
        return edges;
    }
    pairs.sort_unstable();

    // Internal sinks/sources, computed over intra-frame edges only.
    let mut has_incoming: HashSet<&str> = HashSet::new();
    let mut has_outgoing: HashSet<&str> = HashSet::new();
    for edge in layout_edges {
        let (Some(&from_sg), Some(&to_sg)) =
            (owner.get(edge.from.as_str()), owner.get(edge.to.as_str()))
        else {
            continue;
        };
        if from_sg == to_sg {
            has_outgoing.insert(edge.from.as_str());
            has_incoming.insert(edge.to.as_str());
        }
    }
    let frame_members = |idx: usize| -> Vec<&str> {
        graph.subgraphs[idx]
            .nodes
            .iter()
            .map(String::as_str)
            .filter(|id| layout_set.contains(id))
            .collect()
    };

    let template = edges.first().or(layout_edges.first()).cloned();
    let Some(template) = template else {
        return edges;
    };

    for (from_sg, to_sg) in pairs {
        let sinks: Vec<&str> = frame_members(from_sg)
            .into_iter()
            .filter(|id| !has_outgoing.contains(id))
            .collect();
        let sources: Vec<&str> = frame_members(to_sg)
            .into_iter()
            .filter(|id| !has_incoming.contains(id))
            .collect();
        if sinks.is_empty()
            || sources.is_empty()
            || sinks.len() * sources.len() > MAX_BAND_CONSTRAINTS_PER_PAIR
        {
            continue;
        }
        for sink in &sinks {
            for source in &sources {
                let mut constraint = template.clone();
                constraint.from = (*sink).to_string();
                constraint.to = (*source).to_string();
                constraint.label = None;
                constraint.start_label = None;
                constraint.end_label = None;
                edges.push(constraint);
            }
        }
    }

    edges
}

pub(super) fn order_rank_nodes(
    rank_nodes: &mut [Vec<String>],
    edges: &[crate::ir::Edge],
    node_order: &HashMap<String, usize>,
    passes: usize,
) {
    if rank_nodes.len() <= 1 {
        return;
    }
    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();

    for edge in edges {
        outgoing
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        incoming
            .entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());
    }

    let mut positions: HashMap<String, usize> = HashMap::new();
    let update_positions = |rank_nodes: &mut [Vec<String>],
                            positions: &mut HashMap<String, usize>| {
        positions.clear();
        for bucket in rank_nodes.iter() {
            for (idx, node_id) in bucket.iter().enumerate() {
                positions.insert(node_id.clone(), idx);
            }
        }
    };

    update_positions(rank_nodes, &mut positions);

    let sort_bucket = |bucket: &mut Vec<String>,
                       neighbors: &HashMap<String, Vec<String>>,
                       positions: &HashMap<String, usize>| {
        let current_positions: HashMap<String, usize> = bucket
            .iter()
            .enumerate()
            .map(|(idx, id)| (id.clone(), idx))
            .collect();
        bucket.sort_by(|a, b| {
            let a_score = median_position(a, neighbors, positions, &current_positions);
            let b_score = median_position(b, neighbors, positions, &current_positions);
            match a_score.partial_cmp(&b_score) {
                Some(std::cmp::Ordering::Equal) | None => {
                    let a_pos = current_positions.get(a).copied().unwrap_or(0);
                    let b_pos = current_positions.get(b).copied().unwrap_or(0);
                    match a_pos.cmp(&b_pos) {
                        std::cmp::Ordering::Equal => node_order
                            .get(a)
                            .copied()
                            .unwrap_or(usize::MAX)
                            .cmp(&node_order.get(b).copied().unwrap_or(usize::MAX)),
                        other => other,
                    }
                }
                Some(ordering) => ordering,
            }
        });
    };

    let passes = passes.max(1);
    for _ in 0..passes {
        for rank in 1..rank_nodes.len() {
            if rank_nodes[rank].len() <= 1 {
                continue;
            }
            sort_bucket(&mut rank_nodes[rank], &incoming, &positions);
            transpose_bucket(&mut rank_nodes[rank], &incoming, &positions, node_order);
            update_positions(rank_nodes, &mut positions);
        }
        for rank in (0..rank_nodes.len().saturating_sub(1)).rev() {
            if rank_nodes[rank].len() <= 1 {
                continue;
            }
            sort_bucket(&mut rank_nodes[rank], &outgoing, &positions);
            transpose_bucket(&mut rank_nodes[rank], &outgoing, &positions, node_order);
            update_positions(rank_nodes, &mut positions);
        }
    }
}

/// Alternating median sweeps with Dagre-style bias changes and explicit
/// retention of the best crossing count seen during the search.
pub(super) fn order_rank_nodes_dagre_style(
    rank_nodes: &mut [Vec<String>],
    edges: &[crate::ir::Edge],
    node_order: &HashMap<String, usize>,
    passes: usize,
) {
    if rank_nodes.len() <= 1 {
        return;
    }
    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    for edge in edges {
        outgoing
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        incoming
            .entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());
    }

    let mut work = rank_nodes.to_vec();
    let mut best = work.clone();
    let mut best_crossings = adjacent_rank_crossing_count(&best, edges);
    let mut positions = HashMap::new();
    let update_positions = |layers: &[Vec<String>], positions: &mut HashMap<String, usize>| {
        positions.clear();
        for bucket in layers {
            for (index, id) in bucket.iter().enumerate() {
                positions.insert(id.clone(), index);
            }
        }
    };
    update_positions(&work, &mut positions);

    let sweep_count = passes.max(4) * 2;
    let mut since_improvement = 0usize;
    for sweep in 0..sweep_count {
        let downward = sweep % 2 == 0;
        let bias_right = sweep % 4 >= 2;
        let neighbors = if downward { &incoming } else { &outgoing };
        let ranks: Vec<usize> = if downward {
            (1..work.len()).collect()
        } else {
            (0..work.len().saturating_sub(1)).rev().collect()
        };
        for rank in ranks {
            if work[rank].len() <= 1 {
                continue;
            }
            let current_positions = work[rank]
                .iter()
                .enumerate()
                .map(|(index, id)| (id.clone(), index))
                .collect::<HashMap<_, _>>();
            work[rank].sort_by(|a, b| {
                let a_score = median_position(a, neighbors, &positions, &current_positions);
                let b_score = median_position(b, neighbors, &positions, &current_positions);
                match a_score.partial_cmp(&b_score) {
                    Some(std::cmp::Ordering::Equal) | None => {
                        let a_pos = current_positions.get(a).copied().unwrap_or(0);
                        let b_pos = current_positions.get(b).copied().unwrap_or(0);
                        let biased = if bias_right {
                            b_pos.cmp(&a_pos)
                        } else {
                            a_pos.cmp(&b_pos)
                        };
                        biased.then_with(|| {
                            node_order
                                .get(a)
                                .copied()
                                .unwrap_or(usize::MAX)
                                .cmp(&node_order.get(b).copied().unwrap_or(usize::MAX))
                        })
                    }
                    Some(ordering) => ordering,
                }
            });
            transpose_bucket(&mut work[rank], neighbors, &positions, node_order);
            update_positions(&work, &mut positions);
        }

        let crossings = adjacent_rank_crossing_count(&work, edges);
        if crossings < best_crossings {
            best_crossings = crossings;
            best.clone_from(&work);
            since_improvement = 0;
        } else {
            since_improvement += 1;
            if crossings == best_crossings && work < best {
                best.clone_from(&work);
            }
        }
        if since_improvement >= 4 {
            break;
        }
    }
    rank_nodes.clone_from_slice(&best);
}

pub(super) fn adjacent_rank_crossing_count(
    rank_nodes: &[Vec<String>],
    edges: &[crate::ir::Edge],
) -> usize {
    let mut positions: HashMap<&str, (usize, usize)> = HashMap::new();
    for (rank, bucket) in rank_nodes.iter().enumerate() {
        for (order, id) in bucket.iter().enumerate() {
            positions.insert(id.as_str(), (rank, order));
        }
    }
    let mut by_gap: HashMap<usize, Vec<(&str, &str, usize, usize)>> = HashMap::new();
    for edge in edges {
        let Some(&(from_rank, from_order)) = positions.get(edge.from.as_str()) else {
            continue;
        };
        let Some(&(to_rank, to_order)) = positions.get(edge.to.as_str()) else {
            continue;
        };
        if to_rank != from_rank + 1 {
            continue;
        }
        by_gap.entry(from_rank).or_default().push((
            edge.from.as_str(),
            edge.to.as_str(),
            from_order,
            to_order,
        ));
    }
    let mut crossings = 0usize;
    for edges in by_gap.values() {
        for i in 0..edges.len() {
            for j in (i + 1)..edges.len() {
                let (a_from, a_to, a0, a1) = edges[i];
                let (b_from, b_to, b0, b1) = edges[j];
                if a_from == b_from || a_to == b_to {
                    continue;
                }
                if (a0 < b0 && a1 > b1) || (a0 > b0 && a1 < b1) {
                    crossings += 1;
                }
            }
        }
    }
    crossings
}

fn pair_crossings(
    a: &str,
    b: &str,
    neighbors: &HashMap<String, Vec<String>>,
    positions: &HashMap<String, usize>,
) -> (usize, usize) {
    let mut a_pos: Vec<usize> = neighbors
        .get(a)
        .into_iter()
        .flatten()
        .filter_map(|id| positions.get(id).copied())
        .collect();
    let mut b_pos: Vec<usize> = neighbors
        .get(b)
        .into_iter()
        .flatten()
        .filter_map(|id| positions.get(id).copied())
        .collect();
    if a_pos.is_empty() || b_pos.is_empty() {
        return (0, 0);
    }
    a_pos.sort_unstable();
    b_pos.sort_unstable();
    let mut crossings_ab = 0usize;
    let mut crossings_ba = 0usize;
    for pa in &a_pos {
        for pb in &b_pos {
            if pa > pb {
                crossings_ab += 1;
            } else if pb > pa {
                crossings_ba += 1;
            }
        }
    }
    (crossings_ab, crossings_ba)
}

fn transpose_bucket(
    bucket: &mut [String],
    neighbors: &HashMap<String, Vec<String>>,
    positions: &HashMap<String, usize>,
    node_order: &HashMap<String, usize>,
) {
    if bucket.len() <= 1 {
        return;
    }
    let mut improved = true;
    while improved {
        improved = false;
        for i in 0..bucket.len().saturating_sub(1) {
            let a = bucket[i].as_str();
            let b = bucket[i + 1].as_str();
            let (crossings_ab, crossings_ba) = pair_crossings(a, b, neighbors, positions);
            let should_swap = if crossings_ba < crossings_ab {
                true
            } else if crossings_ba > crossings_ab {
                false
            } else {
                node_order.get(a).copied().unwrap_or(usize::MAX)
                    > node_order.get(b).copied().unwrap_or(usize::MAX)
            };
            if should_swap {
                bucket.swap(i, i + 1);
                improved = true;
            }
        }
    }
}

pub(super) fn median_position(
    node_id: &str,
    neighbors: &HashMap<String, Vec<String>>,
    positions: &HashMap<String, usize>,
    current_positions: &HashMap<String, usize>,
) -> f32 {
    let Some(list) = neighbors.get(node_id) else {
        return *current_positions.get(node_id).unwrap_or(&0) as f32;
    };
    let mut values = Vec::new();
    for neighbor in list {
        if let Some(pos) = positions.get(neighbor) {
            values.push(*pos as f32);
        }
    }
    if values.is_empty() {
        return *current_positions.get(node_id).unwrap_or(&0) as f32;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        values[mid]
    } else {
        (values[mid - 1] + values[mid]) * 0.5
    }
}

pub(super) fn compute_ranks_subset(
    node_ids: &[String],
    edges: &[crate::ir::Edge],
    node_order: &HashMap<String, usize>,
) -> HashMap<String, usize> {
    let set: HashSet<String> = node_ids.iter().cloned().collect();
    let subset_edges: Vec<crate::ir::Edge> = edges
        .iter()
        .filter(|edge| set.contains(&edge.from) && set.contains(&edge.to))
        .cloned()
        .collect();

    let mut fallback_order: HashMap<&str, usize> = HashMap::new();
    for (idx, id) in node_ids.iter().enumerate() {
        fallback_order.insert(id.as_str(), idx);
    }
    let order_key = |id: &str| -> usize {
        node_order
            .get(id)
            .copied()
            .unwrap_or_else(|| fallback_order.get(id).copied().unwrap_or(usize::MAX))
    };

    let components = strongly_connected_components(node_ids, &subset_edges);
    let mut node_to_component: HashMap<String, usize> = HashMap::new();
    for (comp_idx, component) in components.iter().enumerate() {
        for node_id in component {
            node_to_component.insert(node_id.clone(), comp_idx);
        }
    }

    let mut comp_adj: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut comp_rev: HashMap<usize, Vec<usize>> = HashMap::new();
    for edge in &subset_edges {
        let Some(&from_comp) = node_to_component.get(&edge.from) else {
            continue;
        };
        let Some(&to_comp) = node_to_component.get(&edge.to) else {
            continue;
        };
        if from_comp == to_comp {
            continue;
        }
        comp_adj.entry(from_comp).or_default().push(to_comp);
        comp_rev.entry(to_comp).or_default().push(from_comp);
    }
    for nexts in comp_adj.values_mut() {
        nexts.sort_unstable();
        nexts.dedup();
    }
    for prevs in comp_rev.values_mut() {
        prevs.sort_unstable();
        prevs.dedup();
    }

    let component_order = stable_topology_order(
        &(0..components.len()).collect::<Vec<_>>(),
        &comp_adj,
        &comp_rev,
        |comp_idx| {
            components[*comp_idx]
                .iter()
                .map(|id| order_key(id.as_str()))
                .min()
                .unwrap_or(usize::MAX)
        },
    );

    let mut local_ranks_by_component: Vec<HashMap<String, usize>> =
        vec![HashMap::new(); components.len()];
    for (comp_idx, component) in components.iter().enumerate() {
        let internal_order = stable_component_node_order(component, &subset_edges, &order_key);
        let component_set: HashSet<&str> = component.iter().map(String::as_str).collect();
        let mut internal_adj: HashMap<String, Vec<String>> = HashMap::new();
        for edge in &subset_edges {
            if component_set.contains(edge.from.as_str())
                && component_set.contains(edge.to.as_str())
            {
                internal_adj
                    .entry(edge.from.clone())
                    .or_default()
                    .push(edge.to.clone());
            }
        }
        local_ranks_by_component[comp_idx] =
            layered_ranks_from_order(&internal_order, &internal_adj);
    }

    let mut weighted_comp_edges: HashMap<usize, Vec<(usize, isize)>> = HashMap::new();
    for edge in &subset_edges {
        let Some(&from_comp) = node_to_component.get(&edge.from) else {
            continue;
        };
        let Some(&to_comp) = node_to_component.get(&edge.to) else {
            continue;
        };
        if from_comp == to_comp {
            continue;
        }
        let from_local = local_ranks_by_component[from_comp]
            .get(&edge.from)
            .copied()
            .unwrap_or(0) as isize;
        let to_local = local_ranks_by_component[to_comp]
            .get(&edge.to)
            .copied()
            .unwrap_or(0) as isize;
        weighted_comp_edges
            .entry(from_comp)
            .or_default()
            .push((to_comp, from_local + 1 - to_local));
    }

    let mut component_start = vec![0isize; components.len()];
    for comp_idx in &component_order {
        let start = component_start[*comp_idx];
        if let Some(nexts) = weighted_comp_edges.get(comp_idx) {
            for (next, weight) in nexts {
                component_start[*next] = component_start[*next].max(start + *weight);
            }
        }
    }

    let mut ranks: HashMap<String, usize> = HashMap::new();
    for (comp_idx, component) in components.iter().enumerate() {
        let base_rank = component_start[comp_idx].max(0) as usize;
        for node_id in component {
            let local_rank = local_ranks_by_component[comp_idx]
                .get(node_id)
                .copied()
                .unwrap_or(0);
            ranks.insert(node_id.clone(), base_rank + local_rank);
        }
    }

    ranks
}

/// Dagre-style rank assignment for the experimental layered engine.
///
/// The implementation follows the same staged contract as Dagre without
/// importing a graph runtime: a deterministic greedy feedback-arc ordering
/// makes the graph acyclic, longest-path ranking establishes a feasible
/// solution, and bounded median relaxation reduces weighted edge span while
/// preserving every unit `minlen` constraint.
pub(super) fn compute_ranks_dagre_style(
    node_ids: &[String],
    edges: &[crate::ir::Edge],
    node_order: &HashMap<String, usize>,
) -> (HashMap<String, usize>, Vec<(String, String)>) {
    let node_set: HashSet<&str> = node_ids.iter().map(String::as_str).collect();
    let subset_edges = edges
        .iter()
        .filter(|edge| node_set.contains(edge.from.as_str()) && node_set.contains(edge.to.as_str()))
        .collect::<Vec<_>>();

    let mut fallback_order = HashMap::new();
    for (index, id) in node_ids.iter().enumerate() {
        fallback_order.insert(id.as_str(), index);
    }
    let order_key = |id: &str| {
        node_order
            .get(id)
            .copied()
            .unwrap_or_else(|| fallback_order.get(id).copied().unwrap_or(usize::MAX))
    };

    // Eades-style greedy feedback-arc ordering. Sinks are peeled to the right,
    // sources to the left, then the node with maximum out-in balance is chosen.
    let mut active: HashSet<&str> = node_ids.iter().map(String::as_str).collect();
    let mut left = Vec::with_capacity(node_ids.len());
    let mut right = Vec::new();
    while !active.is_empty() {
        let mut degrees = active
            .iter()
            .map(|id| {
                let mut incoming = 0usize;
                let mut outgoing = 0usize;
                for edge in &subset_edges {
                    if edge.from == **id && edge.to != **id && active.contains(edge.to.as_str()) {
                        outgoing += 1;
                    }
                    if edge.to == **id && edge.from != **id && active.contains(edge.from.as_str()) {
                        incoming += 1;
                    }
                }
                (*id, incoming, outgoing)
            })
            .collect::<Vec<_>>();
        degrees.sort_by_key(|(id, _, _)| (order_key(id), *id));

        let selected = if let Some((id, _, _)) = degrees.iter().find(|(_, _, out)| *out == 0) {
            right.push((*id).to_string());
            *id
        } else if let Some((id, _, _)) = degrees.iter().find(|(_, incoming, _)| *incoming == 0) {
            left.push((*id).to_string());
            *id
        } else {
            let (id, _, _) = degrees
                .iter()
                .max_by(|(a_id, a_in, a_out), (b_id, b_in, b_out)| {
                    let a_balance = *a_out as isize - *a_in as isize;
                    let b_balance = *b_out as isize - *b_in as isize;
                    a_balance.cmp(&b_balance).then_with(|| {
                        order_key(b_id)
                            .cmp(&order_key(a_id))
                            .then_with(|| b_id.cmp(a_id))
                    })
                })
                .expect("active graph has a node");
            left.push((*id).to_string());
            *id
        };
        active.remove(selected);
    }
    right.reverse();
    left.extend(right);

    let positions = left
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut feedback_edges = Vec::new();
    let mut oriented_weights: HashMap<(String, String), usize> = HashMap::new();
    for edge in subset_edges {
        if edge.from == edge.to {
            feedback_edges.push((edge.from.clone(), edge.to.clone()));
            continue;
        }
        let from_pos = positions.get(edge.from.as_str()).copied().unwrap_or(0);
        let to_pos = positions.get(edge.to.as_str()).copied().unwrap_or(0);
        let (from, to) = if from_pos < to_pos {
            (edge.from.clone(), edge.to.clone())
        } else {
            feedback_edges.push((edge.from.clone(), edge.to.clone()));
            (edge.to.clone(), edge.from.clone())
        };
        *oriented_weights.entry((from, to)).or_insert(0) += 1;
    }
    feedback_edges.sort();
    feedback_edges.dedup();

    let mut incoming: HashMap<String, Vec<(String, usize)>> = HashMap::new();
    let mut outgoing: HashMap<String, Vec<(String, usize)>> = HashMap::new();
    for ((from, to), weight) in &oriented_weights {
        outgoing
            .entry(from.clone())
            .or_default()
            .push((to.clone(), *weight));
        incoming
            .entry(to.clone())
            .or_default()
            .push((from.clone(), *weight));
    }

    let mut ranks = HashMap::new();
    for node in &left {
        let rank = incoming
            .get(node)
            .into_iter()
            .flatten()
            .filter_map(|(pred, _)| ranks.get(pred).copied())
            .map(|rank: usize| rank + 1)
            .max()
            .unwrap_or(0);
        ranks.insert(node.clone(), rank);
    }

    // Network-simplex-inspired feasible relaxation. Every move is clamped by
    // predecessor/successor constraints, so the rank assignment remains valid.
    let max_rank = ranks.values().copied().max().unwrap_or(0);
    for pass in 0..8 {
        let traversal: Box<dyn Iterator<Item = &String>> = if pass % 2 == 0 {
            Box::new(left.iter().rev())
        } else {
            Box::new(left.iter())
        };
        for node in traversal {
            let lower = incoming
                .get(node)
                .into_iter()
                .flatten()
                .filter_map(|(pred, _)| ranks.get(pred).copied())
                .map(|rank| rank + 1)
                .max()
                .unwrap_or(0);
            let upper = outgoing
                .get(node)
                .into_iter()
                .flatten()
                .filter_map(|(succ, _)| ranks.get(succ).copied())
                .map(|rank| rank.saturating_sub(1))
                .min()
                .unwrap_or(max_rank);
            if lower > upper {
                continue;
            }
            let mut targets = Vec::new();
            if let Some(preds) = incoming.get(node) {
                for (pred, weight) in preds {
                    if let Some(rank) = ranks.get(pred) {
                        targets.extend(std::iter::repeat_n(rank + 1, *weight));
                    }
                }
            }
            if let Some(succs) = outgoing.get(node) {
                for (succ, weight) in succs {
                    if let Some(rank) = ranks.get(succ) {
                        targets.extend(std::iter::repeat_n(rank.saturating_sub(1), *weight));
                    }
                }
            }
            if targets.is_empty() {
                continue;
            }
            targets.sort_unstable();
            let desired = targets[targets.len() / 2].clamp(lower, upper);
            ranks.insert(node.clone(), desired);
        }
    }

    let min_rank = ranks.values().copied().min().unwrap_or(0);
    if min_rank > 0 {
        for rank in ranks.values_mut() {
            *rank -= min_rank;
        }
    }
    (ranks, feedback_edges)
}

fn layered_ranks_from_order(
    order: &[String],
    adj: &HashMap<String, Vec<String>>,
) -> HashMap<String, usize> {
    let order_index: HashMap<&str, usize> = order
        .iter()
        .enumerate()
        .map(|(idx, id)| (id.as_str(), idx))
        .collect();

    let mut ranks: HashMap<String, usize> = HashMap::new();
    for node in order {
        let rank = *ranks.get(node).unwrap_or(&0);
        ranks.entry(node.clone()).or_insert(rank);
        if let Some(nexts) = adj.get(node) {
            let from_idx = *order_index.get(node.as_str()).unwrap_or(&0);
            for next in nexts {
                let to_idx = *order_index.get(next.as_str()).unwrap_or(&from_idx);
                if to_idx <= from_idx {
                    continue;
                }
                let entry = ranks.entry(next.clone()).or_insert(0);
                *entry = (*entry).max(rank + 1);
            }
        }
    }
    ranks
}

fn stable_component_node_order<F>(
    component: &[String],
    edges: &[crate::ir::Edge],
    order_key: &F,
) -> Vec<String>
where
    F: Fn(&str) -> usize,
{
    let component_set: HashSet<&str> = component.iter().map(String::as_str).collect();
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    let mut rev: HashMap<String, Vec<String>> = HashMap::new();
    for edge in edges {
        if component_set.contains(edge.from.as_str()) && component_set.contains(edge.to.as_str()) {
            adj.entry(edge.from.clone())
                .or_default()
                .push(edge.to.clone());
            rev.entry(edge.to.clone())
                .or_default()
                .push(edge.from.clone());
        }
    }

    stable_topology_order(component, &adj, &rev, |node_id| order_key(node_id.as_str()))
}

fn stable_topology_order<T, F>(
    items: &[T],
    adj: &HashMap<T, Vec<T>>,
    rev: &HashMap<T, Vec<T>>,
    order_key: F,
) -> Vec<T>
where
    T: Clone + Eq + std::hash::Hash + Ord,
    F: Fn(&T) -> usize,
{
    let mut indeg: HashMap<T, usize> = HashMap::new();
    for item in items {
        let count = rev.get(item).map(|v| v.len()).unwrap_or(0);
        indeg.insert(item.clone(), count);
    }

    let mut ready: BinaryHeap<Reverse<(usize, T)>> = BinaryHeap::new();
    for item in items {
        if *indeg.get(item).unwrap_or(&0) == 0 {
            ready.push(Reverse((order_key(item), item.clone())));
        }
    }

    let item_set: HashSet<T> = items.iter().cloned().collect();
    let mut ordered = Vec::with_capacity(items.len());
    let mut processed: HashSet<T> = HashSet::new();
    loop {
        while let Some(Reverse((_key, item))) = ready.pop() {
            if processed.contains(&item) {
                continue;
            }
            ordered.push(item.clone());
            processed.insert(item.clone());
            if let Some(nexts) = adj.get(&item) {
                for next in nexts {
                    if processed.contains(next) {
                        continue;
                    }
                    if let Some(deg) = indeg.get_mut(next) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 {
                            ready.push(Reverse((order_key(next), next.clone())));
                        }
                    }
                }
            }
        }

        if processed.len() >= items.len() {
            break;
        }

        let mut best: Option<(usize, T)> = None;
        for item in &item_set {
            if !processed.contains(item) {
                let key = order_key(item);
                if best.as_ref().is_none_or(|(best_key, _)| key < *best_key) {
                    best = Some((key, item.clone()));
                }
            }
        }
        if let Some((key, item)) = best {
            ready.push(Reverse((key, item)));
        } else {
            break;
        }
    }

    ordered
}

fn strongly_connected_components(
    node_ids: &[String],
    edges: &[crate::ir::Edge],
) -> Vec<Vec<String>> {
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    let mut rev: HashMap<String, Vec<String>> = HashMap::new();
    for edge in edges {
        adj.entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        rev.entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());
    }

    let mut visited: HashSet<String> = HashSet::new();
    let mut finish_order = Vec::with_capacity(node_ids.len());
    for node_id in node_ids {
        dfs_finish_order(node_id, &adj, &mut visited, &mut finish_order);
    }

    let mut assigned: HashSet<String> = HashSet::new();
    let mut components = Vec::new();
    while let Some(node_id) = finish_order.pop() {
        if !assigned.insert(node_id.clone()) {
            continue;
        }
        let mut component = Vec::new();
        let mut stack = vec![node_id];
        while let Some(current) = stack.pop() {
            component.push(current.clone());
            if let Some(prevs) = rev.get(&current) {
                for prev in prevs {
                    if assigned.insert(prev.clone()) {
                        stack.push(prev.clone());
                    }
                }
            }
        }
        components.push(component);
    }

    components
}

fn dfs_finish_order(
    node_id: &str,
    adj: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    finish_order: &mut Vec<String>,
) {
    if !visited.insert(node_id.to_string()) {
        return;
    }
    if let Some(nexts) = adj.get(node_id) {
        for next in nexts {
            dfs_finish_order(next, adj, visited, finish_order);
        }
    }
    finish_order.push(node_id.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn edge(from: &str, to: &str) -> crate::ir::Edge {
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
            style: crate::ir::EdgeStyle::Solid,
        }
    }

    #[test]
    fn compute_ranks_linear_chain() {
        let nodes = vec!["A".into(), "B".into(), "C".into()];
        let edges = vec![edge("A", "B"), edge("B", "C")];
        let ranks = compute_ranks_subset(&nodes, &edges, &HashMap::new());
        assert_eq!(ranks["A"], 0);
        assert_eq!(ranks["B"], 1);
        assert_eq!(ranks["C"], 2);
    }

    #[test]
    fn compute_ranks_diamond() {
        let nodes = vec!["A".into(), "B".into(), "C".into(), "D".into()];
        let edges = vec![
            edge("A", "B"),
            edge("A", "C"),
            edge("B", "D"),
            edge("C", "D"),
        ];
        let ranks = compute_ranks_subset(&nodes, &edges, &HashMap::new());
        assert_eq!(ranks["A"], 0);
        assert_eq!(ranks["B"], 1);
        assert_eq!(ranks["C"], 1);
        assert_eq!(ranks["D"], 2);
    }

    #[test]
    fn compute_ranks_handles_cycle() {
        let nodes = vec!["A".into(), "B".into(), "C".into()];
        let edges = vec![edge("A", "B"), edge("B", "C"), edge("C", "A")];
        let ranks = compute_ranks_subset(&nodes, &edges, &HashMap::new());
        assert_eq!(ranks.len(), 3);
        assert_eq!(ranks["A"], 0);
        assert_eq!(ranks["B"], 1);
        assert_eq!(ranks["C"], 2);
    }

    #[test]
    fn compute_ranks_places_downstream_after_cycle_component() {
        let nodes = vec!["A".into(), "B".into(), "C".into(), "D".into()];
        let edges = vec![
            edge("A", "B"),
            edge("B", "C"),
            edge("C", "A"),
            edge("C", "D"),
        ];
        let ranks = compute_ranks_subset(&nodes, &edges, &HashMap::new());
        let cycle_max = ranks["A"].max(ranks["B"]).max(ranks["C"]);
        assert_eq!(cycle_max, 2);
        assert_eq!(ranks["D"], cycle_max + 1);
    }

    #[test]
    fn compute_ranks_cycle_with_entry_and_exit_respects_external_precedence() {
        let nodes = vec!["S".into(), "A".into(), "B".into(), "T".into()];
        let edges = vec![
            edge("S", "A"),
            edge("A", "B"),
            edge("B", "A"),
            edge("B", "T"),
        ];
        let ranks = compute_ranks_subset(&nodes, &edges, &HashMap::new());
        let cycle_min = ranks["A"].min(ranks["B"]);
        let cycle_max = ranks["A"].max(ranks["B"]);
        assert!(cycle_min > ranks["S"]);
        assert!(ranks["T"] > cycle_max);
    }

    #[test]
    fn compute_ranks_cycle_respects_entry_and_exit_when_cycle_order_flips() {
        let nodes = vec!["S".into(), "B".into(), "A".into(), "T".into()];
        let edges = vec![
            edge("S", "B"),
            edge("A", "B"),
            edge("B", "A"),
            edge("A", "T"),
        ];
        let ranks = compute_ranks_subset(&nodes, &edges, &HashMap::new());
        let cycle_min = ranks["A"].min(ranks["B"]);
        let cycle_max = ranks["A"].max(ranks["B"]);
        assert!(cycle_min > ranks["S"]);
        assert!(ranks["T"] > cycle_max);
    }

    #[test]
    fn compute_ranks_disconnected_nodes() {
        let nodes = vec!["A".into(), "B".into(), "C".into()];
        let edges = vec![edge("A", "B")];
        let ranks = compute_ranks_subset(&nodes, &edges, &HashMap::new());
        assert_eq!(ranks["A"], 0);
        assert_eq!(ranks["B"], 1);
        assert_eq!(ranks["C"], 0); // disconnected → rank 0
    }

    #[test]
    fn dagre_style_ranking_breaks_cycle_and_preserves_forward_constraints() {
        let nodes = vec!["A".into(), "B".into(), "C".into(), "D".into()];
        let edges = vec![
            edge("A", "B"),
            edge("B", "C"),
            edge("C", "A"),
            edge("C", "D"),
        ];
        let (ranks, feedback) = compute_ranks_dagre_style(&nodes, &edges, &HashMap::new());
        assert_eq!(feedback.len(), 1);
        for edge in &edges {
            if feedback.contains(&(edge.from.clone(), edge.to.clone())) {
                assert!(ranks[&edge.from] >= ranks[&edge.to]);
            } else {
                assert!(ranks[&edge.to] > ranks[&edge.from]);
            }
        }
        assert!(ranks["D"] > ranks["C"]);
    }

    #[test]
    fn dagre_style_ranking_is_deterministic() {
        let nodes = vec!["S".into(), "A".into(), "B".into(), "T".into()];
        let edges = vec![
            edge("S", "A"),
            edge("S", "B"),
            edge("A", "B"),
            edge("B", "A"),
            edge("A", "T"),
            edge("B", "T"),
        ];
        let first = compute_ranks_dagre_style(&nodes, &edges, &HashMap::new());
        for _ in 0..8 {
            assert_eq!(
                compute_ranks_dagre_style(&nodes, &edges, &HashMap::new()),
                first
            );
        }
    }

    #[test]
    fn median_position_with_no_neighbors() {
        let neighbors: HashMap<String, Vec<String>> = HashMap::new();
        let positions: HashMap<String, usize> = HashMap::new();
        let current: HashMap<String, usize> = [("X".to_string(), 3)].into();
        assert_eq!(median_position("X", &neighbors, &positions, &current), 3.0);
    }

    #[test]
    fn median_position_odd_count() {
        let neighbors: HashMap<String, Vec<String>> =
            [("X".to_string(), vec!["A".into(), "B".into(), "C".into()])].into();
        let positions: HashMap<String, usize> =
            [("A".into(), 1), ("B".into(), 5), ("C".into(), 9)].into();
        let current: HashMap<String, usize> = [("X".to_string(), 0)].into();
        assert_eq!(median_position("X", &neighbors, &positions, &current), 5.0);
    }

    #[test]
    fn order_rank_nodes_reduces_crossings() {
        // A→D, B→E, C→F — rank1 starts in wrong order [F,D,E]
        // median-based ordering should move D before E before F
        let edges = vec![edge("A", "D"), edge("B", "E"), edge("C", "F")];
        let mut rank_nodes = vec![
            vec!["A".into(), "B".into(), "C".into()],
            vec!["F".into(), "D".into(), "E".into()],
        ];
        order_rank_nodes(&mut rank_nodes, &edges, &HashMap::new(), 3);
        // D should end up before E which should be before F
        let pos_d = rank_nodes[1].iter().position(|n| n == "D").unwrap();
        let pos_e = rank_nodes[1].iter().position(|n| n == "E").unwrap();
        let pos_f = rank_nodes[1].iter().position(|n| n == "F").unwrap();
        assert!(pos_d < pos_e, "D should precede E, got {:?}", rank_nodes[1]);
        assert!(pos_e < pos_f, "E should precede F, got {:?}", rank_nodes[1]);
    }

    #[test]
    fn dagre_style_ordering_retains_best_crossing_count() {
        let edges = vec![
            edge("A", "F"),
            edge("B", "D"),
            edge("C", "E"),
            edge("D", "H"),
            edge("E", "I"),
            edge("F", "G"),
        ];
        let mut rank_nodes = vec![
            vec!["A".into(), "B".into(), "C".into()],
            vec!["D".into(), "E".into(), "F".into()],
            vec!["G".into(), "H".into(), "I".into()],
        ];
        let before = adjacent_rank_crossing_count(&rank_nodes, &edges);
        order_rank_nodes_dagre_style(&mut rank_nodes, &edges, &HashMap::new(), 4);
        let after = adjacent_rank_crossing_count(&rank_nodes, &edges);
        assert!(after <= before, "{before} -> {after}: {rank_nodes:?}");

        let expected = rank_nodes.clone();
        let mut rerun = vec![
            vec!["A".into(), "B".into(), "C".into()],
            vec!["D".into(), "E".into(), "F".into()],
            vec!["G".into(), "H".into(), "I".into()],
        ];
        order_rank_nodes_dagre_style(&mut rerun, &edges, &HashMap::new(), 4);
        assert_eq!(rerun, expected);
    }
}
