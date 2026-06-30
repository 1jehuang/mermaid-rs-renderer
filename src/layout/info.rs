use std::collections::BTreeMap;

use crate::ir::Graph;
use crate::theme::Theme;

use super::{DiagramData, InfoLayout, Layout};

/// Width/height of the rendered `info` diagram, in viewbox units.
///
/// Mirrors the compact, fixed-size canvas Mermaid.js itself uses for the
/// `info` diagram type (it has no graph content to size around).
const INFO_VIEWBOX_WIDTH: f32 = 400.0;
const INFO_VIEWBOX_HEIGHT: f32 = 100.0;
const INFO_FONT_SCALE: f32 = 1.5;

pub(super) fn compute_info_layout(graph: &Graph, theme: &Theme) -> Layout {
    let width = INFO_VIEWBOX_WIDTH;
    let height = INFO_VIEWBOX_HEIGHT;
    let font_size = theme.font_size * INFO_FONT_SCALE;
    let text = format!("mermaid-rs {}", env!("CARGO_PKG_VERSION"));

    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        width,
        height,
        diagram: DiagramData::Info(InfoLayout {
            width,
            height,
            text,
            text_x: width / 2.0,
            // Baseline offset roughly centers the glyphs vertically.
            text_y: height / 2.0 + font_size * 0.35,
            font_size,
        }),
    }
}
