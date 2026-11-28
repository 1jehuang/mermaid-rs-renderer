use super::geometry::{ray_ellipse_intersection, ray_polygon_intersection, shape_polygon_points};
use super::*;

/// The point where the straight line from the center of `node` towards
/// `target` crosses the border of the node. Block edges run from center to
/// center, and nodes are painted over the edges, so an edge that ends at a
/// center has its arrowhead hidden under the node.
fn block_edge_endpoint(node: &NodeLayout, target: (f32, f32)) -> (f32, f32) {
    let center = (node.x + node.width / 2.0, node.y + node.height / 2.0);
    let dir = (target.0 - center.0, target.1 - center.1);
    if dir.0.abs() < f32::EPSILON && dir.1.abs() < f32::EPSILON {
        return center;
    }
    let hit = match node.shape {
        crate::ir::NodeShape::Circle | crate::ir::NodeShape::DoubleCircle => {
            ray_ellipse_intersection(center, dir, center, node.width / 2.0, node.height / 2.0)
        }
        _ => {
            shape_polygon_points(node).and_then(|poly| ray_polygon_intersection(center, dir, &poly))
        }
    };
    hit.unwrap_or(center)
}

pub(super) fn compute_block_layout(graph: &Graph, theme: &Theme, config: &LayoutConfig) -> Layout {
    let mut nodes = build_graph_node_layouts(graph, theme, config);

    let node_gap = (theme.font_size * 0.4).max(4.0);
    let column_gap = (theme.font_size * 0.45).max(6.0);
    let origin_x = 6.0;
    let origin_y = 6.0;

    let mut edges: Vec<EdgeLayout> = Vec::new();

    let Some(block) = graph.block.as_ref() else {
        let mut subgraphs = build_subgraph_layouts(graph, &nodes, theme, config);
        normalize_layout(&mut nodes, edges.as_mut_slice(), &mut subgraphs);
        let (max_x, max_y) = bounds_without_padding(&nodes, &subgraphs);
        return Layout {
            kind: graph.kind,
            nodes,
            edges,
            subgraphs,
            width: max_x + 6.0,
            height: max_y + 6.0,
            diagram: DiagramData::Graph {
                state_notes: Vec::new(),
            },
        };
    };

    let (placement_nodes, inferred_columns) = if block.nodes.is_empty() {
        infer_block_grid(graph)
    } else {
        (block.nodes.clone(), 0)
    };
    let columns = block
        .columns
        .filter(|columns| *columns > 0)
        .unwrap_or_else(|| {
            if placement_nodes.is_empty() {
                1
            } else if inferred_columns > 0 {
                inferred_columns
            } else {
                placement_nodes.len().max(1)
            }
        });
    let mut column_widths = vec![0.0f32; columns];
    let mut column_x = vec![0.0f32; columns];
    let mut row_y = Vec::<f32>::new();

    let mut row = 0usize;
    let mut col = 0usize;
    let mut row_heights: Vec<f32> = vec![0.0];

    for node in &placement_nodes {
        if col >= columns {
            col = 0;
            row += 1;
            row_heights.push(0.0);
        }
        let span = node.span.max(1).min(columns);
        if col + span > columns {
            col = 0;
            row += 1;
            row_heights.push(0.0);
        }
        if !node.is_space
            && let Some(layout) = nodes.get(&node.id)
        {
            let per_col = layout.width / span as f32;
            for i in 0..span {
                let idx = col + i;
                if idx < columns {
                    column_widths[idx] = column_widths[idx].max(per_col);
                }
            }
            row_heights[row] = row_heights[row].max(layout.height);
        }
        col += span;
    }

    column_x[0] = origin_x;
    for i in 1..columns {
        column_x[i] = column_x[i - 1] + column_widths[i - 1] + column_gap;
    }

    let mut y_cursor = origin_y;
    for h in &row_heights {
        row_y.push(y_cursor);
        y_cursor += *h + node_gap;
    }

    row = 0;
    col = 0;
    for node in &placement_nodes {
        if col >= columns {
            col = 0;
            row += 1;
        }
        let span = node.span.max(1).min(columns);
        if col + span > columns {
            col = 0;
            row += 1;
        }
        if !node.is_space
            && let Some(layout) = nodes.get_mut(&node.id)
        {
            let start_x = column_x[col];
            let mut span_width = 0.0;
            for i in 0..span {
                let idx = col + i;
                if idx < columns {
                    span_width += column_widths[idx];
                    if i + 1 < span {
                        span_width += column_gap;
                    }
                }
            }
            let x = start_x + (span_width - layout.width) / 2.0;
            let y = row_y[row] + (row_heights[row] - layout.height) / 2.0;
            layout.x = x;
            layout.y = y;
        }
        col += span;
    }

    for edge in &graph.edges {
        let Some(from_layout) = nodes.get(&edge.from) else {
            continue;
        };
        let Some(to_layout) = nodes.get(&edge.to) else {
            continue;
        };
        let from_center = (
            from_layout.x + from_layout.width / 2.0,
            from_layout.y + from_layout.height / 2.0,
        );
        let to_center = (
            to_layout.x + to_layout.width / 2.0,
            to_layout.y + to_layout.height / 2.0,
        );
        let label = edge.label.as_ref().map(|l| measure_label(l, theme, config));
        let start_label = edge
            .start_label
            .as_ref()
            .map(|l| measure_label(l, theme, config));
        let end_label = edge
            .end_label
            .as_ref()
            .map(|l| measure_label(l, theme, config));
        let mut override_style = resolve_edge_style(edges.len(), graph);
        if edge.style == crate::ir::EdgeStyle::Dotted && override_style.dasharray.is_none() {
            override_style.dasharray = Some("3 3".to_string());
        }
        edges.push(EdgeLayout {
            from: edge.from.clone(),
            to: edge.to.clone(),
            label,
            start_label,
            end_label,
            label_anchor: None,
            start_label_anchor: None,
            end_label_anchor: None,
            points: vec![
                block_edge_endpoint(from_layout, to_center),
                block_edge_endpoint(to_layout, from_center),
            ],
            directed: edge.directed,
            arrow_start: edge.arrow_start,
            arrow_end: edge.arrow_end,
            arrow_start_kind: edge.arrow_start_kind,
            arrow_end_kind: edge.arrow_end_kind,
            start_decoration: edge.start_decoration,
            end_decoration: edge.end_decoration,
            style: edge.style,
            override_style,
        });
    }

    let mut subgraphs = build_subgraph_layouts(graph, &nodes, theme, config);
    normalize_layout(&mut nodes, edges.as_mut_slice(), &mut subgraphs);

    let (max_x, max_y) = bounds_with_edges(&nodes, &subgraphs, &edges);
    let width = max_x + 6.0;
    let height = max_y + 6.0;

    Layout {
        kind: graph.kind,
        nodes,
        edges,
        subgraphs,
        width,
        height,
        diagram: DiagramData::Graph {
            state_notes: Vec::new(),
        },
    }
}

fn infer_block_grid(graph: &Graph) -> (Vec<crate::ir::BlockNode>, usize) {
    let mut ids: Vec<String> = graph.nodes.keys().cloned().collect();
    ids.sort_by(|a, b| {
        let ao = graph.node_order.get(a).copied().unwrap_or(usize::MAX);
        let bo = graph.node_order.get(b).copied().unwrap_or(usize::MAX);
        ao.cmp(&bo).then_with(|| a.cmp(b))
    });
    if ids.is_empty() {
        return (Vec::new(), 1);
    }

    let mut indegree: HashMap<String, usize> = ids.iter().cloned().map(|id| (id, 0usize)).collect();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    for edge in &graph.edges {
        if edge.from == edge.to {
            continue;
        }
        if !indegree.contains_key(&edge.from) || !indegree.contains_key(&edge.to) {
            continue;
        }
        outgoing
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        if let Some(value) = indegree.get_mut(&edge.to) {
            *value += 1;
        }
    }
    for children in outgoing.values_mut() {
        children.sort_by(|a, b| {
            let ao = graph.node_order.get(a).copied().unwrap_or(usize::MAX);
            let bo = graph.node_order.get(b).copied().unwrap_or(usize::MAX);
            ao.cmp(&bo).then_with(|| a.cmp(b))
        });
        children.dedup();
    }

    let mut queue: Vec<String> = ids
        .iter()
        .filter(|id| indegree.get(*id).copied().unwrap_or(0) == 0)
        .cloned()
        .collect();
    let mut rank: HashMap<String, usize> = HashMap::new();
    let mut head = 0usize;
    while head < queue.len() {
        let id = queue[head].clone();
        head += 1;
        let base_rank = rank.get(&id).copied().unwrap_or(0);
        if let Some(children) = outgoing.get(&id) {
            for child in children {
                rank.entry(child.clone())
                    .and_modify(|r| *r = (*r).max(base_rank + 1))
                    .or_insert(base_rank + 1);
                if let Some(value) = indegree.get_mut(child) {
                    *value = value.saturating_sub(1);
                    if *value == 0 {
                        queue.push(child.clone());
                    }
                }
            }
        }
    }

    if rank.len() < ids.len() {
        for id in &ids {
            if rank.contains_key(id) {
                continue;
            }
            let mut inferred_rank = None;
            for edge in &graph.edges {
                if edge.to != *id {
                    continue;
                }
                if let Some(parent_rank) = rank.get(&edge.from).copied() {
                    inferred_rank = Some(
                        inferred_rank.map_or(parent_rank + 1, |r: usize| r.max(parent_rank + 1)),
                    );
                }
            }
            rank.insert(id.clone(), inferred_rank.unwrap_or(0));
        }
    }

    let mut rows: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for id in ids {
        let row = rank.get(&id).copied().unwrap_or(0);
        rows.entry(row).or_default().push(id);
    }
    for row_ids in rows.values_mut() {
        row_ids.sort_by(|a, b| {
            let ao = graph.node_order.get(a).copied().unwrap_or(usize::MAX);
            let bo = graph.node_order.get(b).copied().unwrap_or(usize::MAX);
            ao.cmp(&bo).then_with(|| a.cmp(b))
        });
    }

    let columns = rows.values().map(Vec::len).max().unwrap_or(1).max(1);
    let mut block_nodes = Vec::new();
    for row_ids in rows.values() {
        for id in row_ids {
            block_nodes.push(crate::ir::BlockNode {
                id: id.clone(),
                span: 1,
                is_space: false,
            });
        }
        let missing = columns.saturating_sub(row_ids.len());
        for _ in 0..missing {
            block_nodes.push(crate::ir::BlockNode {
                id: "__space".to_string(),
                span: 1,
                is_space: true,
            });
        }
    }
    (block_nodes, columns)
}
