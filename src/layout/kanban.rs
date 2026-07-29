use super::*;

pub(super) fn compute_kanban_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
    stage_metrics: Option<&mut LayoutStageMetrics>,
) -> Layout {
    if !graph.edges.is_empty() {
        return compute_flowchart_layout(graph, theme, config, stage_metrics);
    }

    let mut nodes = build_graph_node_layouts(graph, theme, config);
    if graph.kind == crate::ir::DiagramKind::Requirement {
        for node in nodes.values_mut() {
            if node.style.fill.is_none() {
                node.style.fill = Some(config.requirement.fill.clone());
            }
            if node.style.stroke.is_none() {
                node.style.stroke = Some(config.requirement.box_stroke.clone());
            }
            if node.style.stroke_width.is_none() {
                node.style.stroke_width = Some(config.requirement.box_stroke_width);
            }
            if node.style.text_color.is_none() {
                node.style.text_color = Some(config.requirement.label_color.clone());
            }
        }
    }

    let node_gap = (theme.font_size * 0.45).max(4.0);
    let column_gap = (theme.font_size * 0.3).max(3.0);
    let origin_x = 6.0;
    let origin_y = 6.0;
    let mut column_x = origin_x;
    let mut assigned: HashSet<String> = HashSet::new();

    for sub in &graph.subgraphs {
        let column_nodes: Vec<String> = sub
            .nodes
            .iter()
            .filter(|id| nodes.contains_key(*id))
            .cloned()
            .collect();
        if column_nodes.is_empty() {
            continue;
        }
        assigned.extend(column_nodes.iter().cloned());

        let (label_block, (pad_x, _pad_y, top_padding)) =
            subgraph_label_metrics(graph, sub, theme, config);

        let max_node_width = column_nodes
            .iter()
            .filter_map(|id| nodes.get(id).map(|n| n.width))
            .fold(0.0_f32, f32::max);
        let inner_width = max_node_width.max(label_block.width);
        let column_width = inner_width + pad_x * 2.0;

        let mut y_cursor = origin_y + top_padding;
        let last_idx = column_nodes.len().saturating_sub(1);
        for (idx, node_id) in column_nodes.iter().enumerate() {
            if let Some(node) = nodes.get_mut(node_id) {
                let x = column_x + pad_x + (inner_width - node.width) / 2.0;
                node.x = x;
                node.y = y_cursor;
                y_cursor += node.height;
                if idx < last_idx {
                    y_cursor += node_gap;
                }
            }
        }

        column_x += column_width + column_gap;
    }

    let mut free_x = column_x;
    for node in nodes.values_mut() {
        if assigned.contains(&node.id) {
            continue;
        }
        node.x = free_x;
        node.y = origin_y;
        free_x += node.width + column_gap;
    }

    let mut edges: Vec<EdgeLayout> = Vec::new();
    let mut subgraphs = build_subgraph_layouts(graph, &nodes, theme, config);
    normalize_layout(&mut nodes, edges.as_mut_slice(), &mut subgraphs);

    let (max_x, max_y) = bounds_without_padding(&nodes, &subgraphs);
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
