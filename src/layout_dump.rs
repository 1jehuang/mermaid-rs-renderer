use crate::ir::Graph;
use crate::layout::Layout;
use serde::Serialize;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct LayoutDump {
    pub kind: String,
    pub direction: String,
    pub width: f32,
    pub height: f32,
    pub nodes: Vec<NodeDump>,
    pub edges: Vec<EdgeDump>,
    pub subgraphs: Vec<SubgraphDump>,
}

#[derive(Debug, Serialize)]
pub struct NodeDump {
    pub id: String,
    pub shape: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub label_width: f32,
    pub label_height: f32,
    pub label_lines: Vec<String>,
    pub anchor_subgraph: Option<usize>,
    pub hidden: bool,
}

#[derive(Debug, Serialize)]
pub struct EdgeDump {
    pub from: String,
    pub to: String,
    pub directed: bool,
    pub arrow_start: bool,
    pub arrow_end: bool,
    pub points: Vec<[f32; 2]>,
}

#[derive(Debug, Serialize)]
pub struct SubgraphDump {
    pub index: usize,
    pub id: Option<String>,
    pub label: String,
    pub nodes: Vec<String>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl LayoutDump {
    pub fn from_layout(layout: &Layout, graph: &Graph) -> Self {
        let nodes = layout
            .nodes
            .values()
            .map(|node| NodeDump {
                id: node.id.clone(),
                shape: format!("{:?}", node.shape),
                x: node.x,
                y: node.y,
                width: node.width,
                height: node.height,
                label_width: node.label.width,
                label_height: node.label.height,
                label_lines: node.label.lines.clone(),
                anchor_subgraph: node.anchor_subgraph,
                hidden: node.hidden,
            })
            .collect();

        let edges = layout
            .edges
            .iter()
            .map(|edge| EdgeDump {
                from: edge.from.clone(),
                to: edge.to.clone(),
                directed: edge.directed,
                arrow_start: edge.arrow_start,
                arrow_end: edge.arrow_end,
                points: edge.points.iter().map(|(x, y)| [*x, *y]).collect(),
            })
            .collect();

        let mut subgraphs = Vec::new();
        for (idx, sub) in layout.subgraphs.iter().enumerate() {
            let mut sub_id = None;
            for candidate in &graph.subgraphs {
                if candidate.label == sub.label {
                    sub_id = candidate.id.clone();
                    break;
                }
            }
            subgraphs.push(SubgraphDump {
                index: idx,
                id: sub_id,
                label: sub.label.clone(),
                nodes: sub.nodes.clone(),
                x: sub.x,
                y: sub.y,
                width: sub.width,
                height: sub.height,
            });
        }

        LayoutDump {
            kind: format!("{:?}", layout.kind),
            direction: format!("{:?}", graph.direction),
            width: layout.width,
            height: layout.height,
            nodes,
            edges,
            subgraphs,
        }
    }
}

pub fn write_layout_dump(path: &Path, layout: &Layout, graph: &Graph) -> anyhow::Result<()> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    let dump = LayoutDump::from_layout(layout, graph);
    serde_json::to_writer_pretty(writer, &dump)?;
    Ok(())
}
