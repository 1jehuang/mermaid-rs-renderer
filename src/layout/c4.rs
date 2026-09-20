use super::*;
use crate::metrics::TextMetrics;

pub(super) fn compute_c4_layout(graph: &Graph, config: &LayoutConfig) -> Layout {
    let c4 = &graph.c4;
    let fast_metrics = config.fast_text_metrics;
    let mut conf = config.c4.clone();
    if let Some(val) = c4.c4_shape_in_row_override {
        conf.c4_shape_in_row = val;
    }
    if let Some(val) = c4.c4_boundary_in_row_override {
        conf.c4_boundary_in_row = val;
    }
    let conf = &conf;
    let mut shapes_out = Vec::new();
    let mut boundaries_out = Vec::new();
    let mut rels_out = Vec::new();

    let mut shapes_by_boundary: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut shape_map: std::collections::HashMap<String, &crate::ir::C4Shape> =
        std::collections::HashMap::new();
    for shape in &c4.shapes {
        shapes_by_boundary
            .entry(shape.parent_boundary.clone())
            .or_default()
            .push(shape.id.clone());
        shape_map.insert(shape.id.clone(), shape);
    }

    let mut boundaries_by_parent: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut boundary_map: std::collections::HashMap<String, &crate::ir::C4Boundary> =
        std::collections::HashMap::new();
    for boundary in &c4.boundaries {
        boundaries_by_parent
            .entry(boundary.parent_boundary.clone())
            .or_default()
            .push(boundary.id.clone());
        boundary_map.insert(boundary.id.clone(), boundary);
    }

    let root_boundaries = boundaries_by_parent.get("").cloned().unwrap_or_default();

    let mut global_max_x = conf.diagram_margin_x;
    let mut global_max_y = conf.diagram_margin_y;

    let mut screen_bounds = C4Bounds::new(conf);
    let width_limit = 1920.0;
    screen_bounds.set_data(
        conf.diagram_margin_x,
        conf.diagram_margin_x,
        conf.diagram_margin_y,
        conf.diagram_margin_y,
        width_limit,
    );

    layout_c4_boundaries(
        &mut screen_bounds,
        &root_boundaries,
        &mut shapes_out,
        &mut boundaries_out,
        &mut global_max_x,
        &mut global_max_y,
        &shapes_by_boundary,
        &shape_map,
        &boundaries_by_parent,
        &boundary_map,
        conf,
        fast_metrics,
        config.metrics.as_deref(),
    );

    for rel in &c4.rels {
        let Some(from_shape) = shapes_out.iter().find(|s| s.id == rel.from) else {
            continue;
        };
        let Some(to_shape) = shapes_out.iter().find(|s| s.id == rel.to) else {
            continue;
        };
        let (start, end) = c4_intersect_points(from_shape, to_shape);
        let label_font_size = conf.message_font_size;
        let rel_font_family = conf.message_font_family.as_str();
        let label_layout = c4_text_layout(
            &rel.label,
            label_font_size,
            0.0,
            conf.wrap,
            estimate_text_width(
                &rel.label,
                label_font_size,
                rel_font_family,
                fast_metrics,
                config.metrics.as_deref(),
            ),
            c4_text_line_height(conf, label_font_size),
            rel_font_family,
            fast_metrics,
            config.metrics.as_deref(),
        );
        let techn_layout = rel.techn.as_ref().map(|t| {
            c4_text_layout(
                t,
                label_font_size,
                0.0,
                conf.wrap,
                estimate_text_width(
                    t,
                    label_font_size,
                    rel_font_family,
                    fast_metrics,
                    config.metrics.as_deref(),
                ),
                c4_text_line_height(conf, label_font_size),
                rel_font_family,
                fast_metrics,
                config.metrics.as_deref(),
            )
        });
        let waypoints = c4_route_around_shapes(start, end, &rel.from, &rel.to, &shapes_out);
        rels_out.push(C4RelLayout {
            kind: rel.kind,
            from: rel.from.clone(),
            to: rel.to.clone(),
            label: label_layout,
            techn: techn_layout,
            start,
            end,
            waypoints,
            offset_x: rel.offset_x,
            offset_y: rel.offset_y,
            line_color: rel.line_color.clone(),
            text_color: rel.text_color.clone(),
        });
    }
    resolve_c4_rel_label_offsets(&mut rels_out, &shapes_out, conf);

    let mut nodes: BTreeMap<String, NodeLayout> = BTreeMap::new();
    for shape in &shapes_out {
        nodes.insert(
            shape.id.clone(),
            NodeLayout {
                id: shape.id.clone(),
                x: shape.x,
                y: shape.y,
                width: shape.width,
                height: shape.height,
                label: TextBlock {
                    lines: shape.label.lines.clone(),
                    width: shape.label.width,
                    height: shape.label.height,
                },
                shape: crate::ir::NodeShape::Rectangle,
                style: crate::ir::NodeStyle::default(),
                link: None,
                anchor_subgraph: None,
                hidden: false,
                icon: None,
            },
        );
    }
    let mut edges: Vec<EdgeLayout> = Vec::new();
    for rel in &rels_out {
        edges.push(EdgeLayout {
            from: rel.from.clone(),
            to: rel.to.clone(),
            label: None,
            start_label: None,
            end_label: None,
            label_anchor: None,
            start_label_anchor: None,
            end_label_anchor: None,
            points: rel.waypoints.clone(),
            directed: rel.kind != crate::ir::C4RelKind::BiRel,
            arrow_start: false,
            arrow_end: rel.kind != crate::ir::C4RelKind::BiRel,
            arrow_start_kind: None,
            arrow_end_kind: None,
            start_decoration: None,
            end_decoration: None,
            style: crate::ir::EdgeStyle::Solid,
            override_style: crate::ir::EdgeStyleOverride::default(),
        });
    }

    let width = (global_max_x - conf.diagram_margin_x + 2.0 * conf.diagram_margin_x).max(1.0);
    let height = (global_max_y - conf.diagram_margin_y + 2.0 * conf.diagram_margin_y).max(1.0);
    let viewbox_x = 0.0;
    let viewbox_y = -conf.diagram_margin_y;
    let viewbox_width = width;
    let viewbox_height = height;

    Layout {
        kind: graph.kind,
        nodes,
        edges,
        subgraphs: Vec::new(),
        width,
        height,
        diagram: DiagramData::C4(C4Layout {
            shapes: shapes_out,
            boundaries: boundaries_out,
            rels: rels_out,
            viewbox_x,
            viewbox_y,
            viewbox_width,
            viewbox_height,
            use_max_width: conf.use_max_width,
        }),
    }
}

#[derive(Debug, Clone)]
struct C4BoundsData {
    startx: f32,
    stopx: f32,
    starty: f32,
    stopy: f32,
    width_limit: f32,
}

#[derive(Debug, Clone)]
struct C4BoundsNext {
    startx: f32,
    stopx: f32,
    starty: f32,
    stopy: f32,
    cnt: usize,
}

#[derive(Debug, Clone)]
struct C4Bounds {
    data: C4BoundsData,
    next: C4BoundsNext,
    conf: crate::config::C4Config,
}

impl C4Bounds {
    fn new(conf: &crate::config::C4Config) -> Self {
        Self {
            data: C4BoundsData {
                startx: 0.0,
                stopx: 0.0,
                starty: 0.0,
                stopy: 0.0,
                width_limit: 0.0,
            },
            next: C4BoundsNext {
                startx: 0.0,
                stopx: 0.0,
                starty: 0.0,
                stopy: 0.0,
                cnt: 0,
            },
            conf: conf.clone(),
        }
    }

    fn set_data(&mut self, startx: f32, stopx: f32, starty: f32, stopy: f32, width_limit: f32) {
        self.data.startx = startx;
        self.data.stopx = stopx;
        self.data.starty = starty;
        self.data.stopy = stopy;
        self.data.width_limit = width_limit;
        self.next.startx = startx;
        self.next.stopx = stopx;
        self.next.starty = starty;
        self.next.stopy = stopy;
        self.next.cnt = 0;
    }

    fn bump_last_margin(&mut self, margin: f32) {
        self.data.stopx += margin;
        self.data.stopy += margin;
    }

    fn insert(&mut self, width: f32, height: f32, margin: f32) -> (f32, f32) {
        self.next.cnt += 1;
        let mut startx = if (self.next.startx - self.next.stopx).abs() < f32::EPSILON {
            self.next.stopx + margin
        } else {
            self.next.stopx + margin * 2.0
        };
        let mut stopx = startx + width;
        let mut starty = self.next.starty + margin * 2.0;
        let mut stopy = starty + height;

        if startx >= self.data.width_limit
            || stopx >= self.data.width_limit
            || self.next.cnt > self.conf.c4_shape_in_row
        {
            startx = self.next.startx + margin + self.conf.next_line_padding_x;
            starty = self.next.stopy + margin * 2.0;
            stopx = startx + width;
            stopy = starty + height;
            self.next.starty = self.next.stopy;
            self.next.stopy = stopy;
            self.next.stopx = stopx;
            self.next.cnt = 1;
        }

        self.data.startx = if self.data.startx == 0.0 {
            startx
        } else {
            self.data.startx.min(startx)
        };
        self.data.starty = if self.data.starty == 0.0 {
            starty
        } else {
            self.data.starty.min(starty)
        };
        self.data.stopx = self.data.stopx.max(stopx);
        self.data.stopy = self.data.stopy.max(stopy);

        self.next.startx = self.next.startx.min(startx);
        self.next.starty = self.next.starty.min(starty);
        self.next.stopx = self.next.stopx.max(stopx);
        self.next.stopy = self.next.stopy.max(stopy);

        (startx, starty)
    }
}

#[allow(clippy::too_many_arguments)]
fn layout_c4_boundaries(
    parent_bounds: &mut C4Bounds,
    boundary_ids: &[String],
    shapes_out: &mut Vec<C4ShapeLayout>,
    boundaries_out: &mut Vec<C4BoundaryLayout>,
    global_max_x: &mut f32,
    global_max_y: &mut f32,
    shapes_by_boundary: &std::collections::HashMap<String, Vec<String>>,
    shape_map: &std::collections::HashMap<String, &crate::ir::C4Shape>,
    boundaries_by_parent: &std::collections::HashMap<String, Vec<String>>,
    boundary_map: &std::collections::HashMap<String, &crate::ir::C4Boundary>,
    conf: &crate::config::C4Config,
    fast_metrics: bool,
    metrics: Option<&dyn TextMetrics>,
) {
    if boundary_ids.is_empty() {
        return;
    }
    let mut current_bounds = C4Bounds::new(conf);
    let limit_div = conf.c4_boundary_in_row.max(1).min(boundary_ids.len());
    current_bounds.data.width_limit = parent_bounds.data.width_limit / limit_div as f32;

    for (idx, boundary_id) in boundary_ids.iter().enumerate() {
        let Some(boundary) = boundary_map.get(boundary_id) else {
            continue;
        };
        let mut y = 0.0;
        let boundary_text_wrap = conf.wrap;
        let label_font_size = conf.boundary_font_size + 2.0;
        let boundary_font_family = conf.boundary_font_family.as_str();
        let label_layout = c4_text_layout(
            &boundary.label,
            label_font_size,
            y + 8.0,
            boundary_text_wrap,
            current_bounds.data.width_limit,
            c4_text_line_height(conf, label_font_size),
            boundary_font_family,
            fast_metrics,
            metrics,
        );
        y = label_layout.y + label_layout.height;
        let mut boundary_type_layout = None;
        if !boundary.boundary_type.is_empty() {
            let type_text = format!("[{}]", boundary.boundary_type);
            let type_layout = c4_text_layout(
                &type_text,
                conf.boundary_font_size,
                y + 5.0,
                boundary_text_wrap,
                current_bounds.data.width_limit,
                c4_text_line_height(conf, conf.boundary_font_size),
                boundary_font_family,
                fast_metrics,
                metrics,
            );
            y = type_layout.y + type_layout.height;
            boundary_type_layout = Some(type_layout);
        }
        let mut boundary_descr_layout = None;
        if let Some(descr) = &boundary.descr {
            let descr_layout = c4_text_layout(
                descr,
                (conf.boundary_font_size - 2.0).max(1.0),
                y + 20.0,
                boundary_text_wrap,
                current_bounds.data.width_limit,
                c4_text_line_height(conf, (conf.boundary_font_size - 2.0).max(1.0)),
                boundary_font_family,
                fast_metrics,
                metrics,
            );
            y = descr_layout.y + descr_layout.height;
            boundary_descr_layout = Some(descr_layout);
        }

        if idx == 0 || idx % conf.c4_boundary_in_row == 0 {
            let startx = parent_bounds.data.startx + conf.diagram_margin_x;
            let starty = parent_bounds.data.stopy + conf.diagram_margin_y + y;
            current_bounds.set_data(
                startx,
                startx,
                starty,
                starty,
                current_bounds.data.width_limit,
            );
        } else {
            let startx =
                if (current_bounds.data.stopx - current_bounds.data.startx).abs() > f32::EPSILON {
                    current_bounds.data.stopx + conf.diagram_margin_x
                } else {
                    current_bounds.data.startx
                };
            let starty = current_bounds.data.starty;
            current_bounds.set_data(
                startx,
                startx,
                starty,
                starty,
                current_bounds.data.width_limit,
            );
        }

        if let Some(shape_ids) = shapes_by_boundary.get(boundary_id) {
            layout_c4_shapes(
                &mut current_bounds,
                shape_ids,
                shapes_out,
                shape_map,
                conf,
                fast_metrics,
                metrics,
            );
        }

        if let Some(child_boundaries) = boundaries_by_parent.get(boundary_id) {
            layout_c4_boundaries(
                &mut current_bounds,
                child_boundaries,
                shapes_out,
                boundaries_out,
                global_max_x,
                global_max_y,
                shapes_by_boundary,
                shape_map,
                boundaries_by_parent,
                boundary_map,
                conf,
                fast_metrics,
                metrics,
            );
        }

        if boundary_id != "global" {
            let boundary_layout = C4BoundaryLayout {
                id: boundary_id.clone(),
                label: label_layout,
                boundary_type: boundary_type_layout,
                descr: boundary_descr_layout,
                bg_color: boundary.bg_color.clone(),
                border_color: boundary.border_color.clone(),
                font_color: boundary.font_color.clone(),
                x: current_bounds.data.startx,
                y: current_bounds.data.starty,
                width: current_bounds.data.stopx - current_bounds.data.startx,
                height: current_bounds.data.stopy - current_bounds.data.starty,
            };
            boundaries_out.push(boundary_layout);
        }

        parent_bounds.data.stopy = parent_bounds
            .data
            .stopy
            .max(current_bounds.data.stopy + conf.c4_shape_margin);
        parent_bounds.data.stopx = parent_bounds
            .data
            .stopx
            .max(current_bounds.data.stopx + conf.c4_shape_margin);
        *global_max_x = (*global_max_x).max(parent_bounds.data.stopx);
        *global_max_y = (*global_max_y).max(parent_bounds.data.stopy);
    }
}

fn layout_c4_shapes(
    bounds: &mut C4Bounds,
    shape_ids: &[String],
    shapes_out: &mut Vec<C4ShapeLayout>,
    shape_map: &std::collections::HashMap<String, &crate::ir::C4Shape>,
    conf: &crate::config::C4Config,
    fast_metrics: bool,
    metrics: Option<&dyn TextMetrics>,
) {
    for shape_id in shape_ids {
        let Some(shape) = shape_map.get(shape_id) else {
            continue;
        };
        let type_font_size = (c4_shape_font_size(conf, shape.kind) - 2.0).max(1.0);
        let type_font_family = c4_shape_font_family(conf, shape.kind);
        let type_label_text = format!("<<{}>>", shape.kind.as_str());
        let type_width = estimate_text_width(
            &type_label_text,
            type_font_size,
            type_font_family,
            fast_metrics,
            metrics,
        );
        let type_height = type_font_size + 2.0;
        let type_layout = C4TextLayout {
            text: type_label_text.clone(),
            lines: vec![type_label_text],
            width: type_width,
            height: type_height,
            y: conf.c4_shape_padding,
        };
        let mut y = type_layout.y + type_layout.height - 4.0;

        let mut image_y = None;
        if matches!(
            shape.kind,
            crate::ir::C4ShapeKind::Person | crate::ir::C4ShapeKind::ExternalPerson
        ) {
            image_y = Some(y);
            y += conf.person_icon_size;
        } else if shape.sprite.is_some() {
            image_y = Some(y);
            y += conf.person_icon_size;
        }

        let label_font_size = c4_shape_font_size(conf, shape.kind) + 2.0;
        let label_font_family = c4_shape_font_family(conf, shape.kind);
        let text_limit_width = conf.width - conf.c4_shape_padding * 2.0;
        let label_layout = c4_text_layout(
            &shape.label,
            label_font_size,
            y + 8.0,
            conf.wrap,
            text_limit_width,
            c4_text_line_height(conf, label_font_size),
            label_font_family,
            fast_metrics,
            metrics,
        );
        y = label_layout.y + label_layout.height;

        let mut type_or_techn_layout = None;
        let type_or_techn_text = shape
            .techn
            .as_ref()
            .or(shape.type_label.as_ref())
            .map(|t| format!("[{}]", t));
        if let Some(text) = type_or_techn_text {
            let font_size = c4_shape_font_size(conf, shape.kind);
            let font_family = c4_shape_font_family(conf, shape.kind);
            let layout = c4_text_layout(
                &text,
                font_size,
                y + 5.0,
                conf.wrap,
                text_limit_width,
                c4_text_line_height(conf, font_size),
                font_family,
                fast_metrics,
                metrics,
            );
            y = layout.y + layout.height;
            type_or_techn_layout = Some(layout);
        }

        let mut descr_layout = None;
        let mut rect_height = y;
        let mut rect_width = label_layout.width;
        if let Some(descr) = &shape.descr {
            let font_size = c4_shape_font_size(conf, shape.kind);
            let font_family = c4_shape_font_family(conf, shape.kind);
            let layout = c4_text_layout(
                descr,
                font_size,
                y + 20.0,
                conf.wrap,
                text_limit_width,
                c4_text_line_height(conf, font_size),
                font_family,
                fast_metrics,
                metrics,
            );
            y = layout.y + layout.height;
            rect_width = rect_width.max(layout.width);
            let lines = layout.lines.len() as f32;
            rect_height = y - lines * 5.0;
            descr_layout = Some(layout);
        }
        rect_width += conf.c4_shape_padding;
        let width = conf.width.max(rect_width);
        let height = conf.height.max(rect_height);
        let margin = conf.c4_shape_margin;
        let (x, y_pos) = bounds.insert(width, height, margin);

        shapes_out.push(C4ShapeLayout {
            id: shape.id.clone(),
            kind: shape.kind,
            bg_color: shape.bg_color.clone(),
            border_color: shape.border_color.clone(),
            font_color: shape.font_color.clone(),
            x,
            y: y_pos,
            width,
            height,
            margin,
            type_label: type_layout,
            label: label_layout,
            type_or_techn: type_or_techn_layout,
            descr: descr_layout,
            image_y,
        });
    }
    bounds.bump_last_margin(conf.c4_shape_margin);
}

fn c4_shape_font_size(conf: &crate::config::C4Config, kind: crate::ir::C4ShapeKind) -> f32 {
    match kind {
        crate::ir::C4ShapeKind::Person => conf.person_font_size,
        crate::ir::C4ShapeKind::ExternalPerson => conf.external_person_font_size,
        crate::ir::C4ShapeKind::System => conf.system_font_size,
        crate::ir::C4ShapeKind::SystemDb => conf.system_db_font_size,
        crate::ir::C4ShapeKind::SystemQueue => conf.system_queue_font_size,
        crate::ir::C4ShapeKind::ExternalSystem => conf.external_system_font_size,
        crate::ir::C4ShapeKind::ExternalSystemDb => conf.external_system_db_font_size,
        crate::ir::C4ShapeKind::ExternalSystemQueue => conf.external_system_queue_font_size,
        crate::ir::C4ShapeKind::Container => conf.container_font_size,
        crate::ir::C4ShapeKind::ContainerDb => conf.container_db_font_size,
        crate::ir::C4ShapeKind::ContainerQueue => conf.container_queue_font_size,
        crate::ir::C4ShapeKind::ExternalContainer => conf.external_container_font_size,
        crate::ir::C4ShapeKind::ExternalContainerDb => conf.external_container_db_font_size,
        crate::ir::C4ShapeKind::ExternalContainerQueue => conf.external_container_queue_font_size,
        crate::ir::C4ShapeKind::Component => conf.component_font_size,
        crate::ir::C4ShapeKind::ComponentDb => conf.component_db_font_size,
        crate::ir::C4ShapeKind::ComponentQueue => conf.component_queue_font_size,
        crate::ir::C4ShapeKind::ExternalComponent => conf.external_component_font_size,
        crate::ir::C4ShapeKind::ExternalComponentDb => conf.external_component_db_font_size,
        crate::ir::C4ShapeKind::ExternalComponentQueue => conf.external_component_queue_font_size,
    }
}

fn c4_shape_font_family(conf: &crate::config::C4Config, kind: crate::ir::C4ShapeKind) -> &str {
    match kind {
        crate::ir::C4ShapeKind::Person => &conf.person_font_family,
        crate::ir::C4ShapeKind::ExternalPerson => &conf.external_person_font_family,
        crate::ir::C4ShapeKind::System => &conf.system_font_family,
        crate::ir::C4ShapeKind::SystemDb => &conf.system_db_font_family,
        crate::ir::C4ShapeKind::SystemQueue => &conf.system_queue_font_family,
        crate::ir::C4ShapeKind::ExternalSystem => &conf.external_system_font_family,
        crate::ir::C4ShapeKind::ExternalSystemDb => &conf.external_system_db_font_family,
        crate::ir::C4ShapeKind::ExternalSystemQueue => &conf.external_system_queue_font_family,
        crate::ir::C4ShapeKind::Container => &conf.container_font_family,
        crate::ir::C4ShapeKind::ContainerDb => &conf.container_db_font_family,
        crate::ir::C4ShapeKind::ContainerQueue => &conf.container_queue_font_family,
        crate::ir::C4ShapeKind::ExternalContainer => &conf.external_container_font_family,
        crate::ir::C4ShapeKind::ExternalContainerDb => &conf.external_container_db_font_family,
        crate::ir::C4ShapeKind::ExternalContainerQueue => {
            &conf.external_container_queue_font_family
        }
        crate::ir::C4ShapeKind::Component => &conf.component_font_family,
        crate::ir::C4ShapeKind::ComponentDb => &conf.component_db_font_family,
        crate::ir::C4ShapeKind::ComponentQueue => &conf.component_queue_font_family,
        crate::ir::C4ShapeKind::ExternalComponent => &conf.external_component_font_family,
        crate::ir::C4ShapeKind::ExternalComponentDb => &conf.external_component_db_font_family,
        crate::ir::C4ShapeKind::ExternalComponentQueue => {
            &conf.external_component_queue_font_family
        }
    }
}

fn c4_text_line_height(conf: &crate::config::C4Config, font_size: f32) -> f32 {
    let mut height = font_size + conf.text_line_height;
    if font_size <= conf.text_line_height_small_threshold {
        height += conf.text_line_height_small_add;
    }
    height.max(1.0)
}

fn c4_text_layout(
    text: &str,
    font_size: f32,
    y: f32,
    wrap: bool,
    max_width: f32,
    line_height: f32,
    font_family: &str,
    fast_metrics: bool,
    metrics: Option<&dyn TextMetrics>,
) -> C4TextLayout {
    let mut lines = Vec::new();
    for raw in split_lines(text) {
        if wrap {
            lines.extend(wrap_text_to_width(
                &raw,
                max_width,
                font_size,
                font_family,
                fast_metrics,
                metrics,
            ));
        } else {
            lines.push(raw);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    let width = lines
        .iter()
        .map(|line| estimate_text_width(line, font_size, font_family, fast_metrics, metrics))
        .fold(0.0, f32::max);
    let height = line_height * lines.len().max(1) as f32;
    C4TextLayout {
        text: text.to_string(),
        lines,
        width,
        height,
        y,
    }
}

fn wrap_text_to_width(
    text: &str,
    max_width: f32,
    font_size: f32,
    font_family: &str,
    fast_metrics: bool,
    metrics: Option<&dyn TextMetrics>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current, word)
        };
        if estimate_text_width(&candidate, font_size, font_family, fast_metrics, metrics)
            <= max_width
            || current.is_empty()
        {
            current = candidate;
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(text.to_string());
    }
    lines
}

fn estimate_text_width(
    text: &str,
    font_size: f32,
    font_family: &str,
    fast_metrics: bool,
    metrics: Option<&dyn TextMetrics>,
) -> f32 {
    if let Some(width) =
        metrics.and_then(|metrics| metrics.measure_text_width(text, font_size, font_family))
    {
        return width;
    }
    if fast_metrics && text.is_ascii() {
        return text.chars().map(c4_char_width_factor).sum::<f32>() * font_size;
    }
    text_metrics::measure_text_width(text, font_size, font_family)
        .unwrap_or_else(|| text.chars().map(c4_char_width_factor).sum::<f32>() * font_size)
}

fn c4_char_width_factor(ch: char) -> f32 {
    match ch {
        '<' | '>' => 0.247,
        '_' => 0.455,
        _ => char_width_factor(ch),
    }
}

fn c4_intersect_points(
    from_node: &C4ShapeLayout,
    to_node: &C4ShapeLayout,
) -> ((f32, f32), (f32, f32)) {
    let end_center = (
        to_node.x + to_node.width / 2.0,
        to_node.y + to_node.height / 2.0,
    );
    let start = c4_intersect_point(from_node, end_center);
    let start_center = (
        from_node.x + from_node.width / 2.0,
        from_node.y + from_node.height / 2.0,
    );
    let end = c4_intersect_point(to_node, start_center);
    (start, end)
}

/// Does the segment `a`-`b` pass through the axis-aligned rectangle `rect`,
/// shrunk by `pad` on every side so a connector merely grazing the boundary is
/// not treated as an intrusion?
fn c4_segment_hits_rect(a: (f32, f32), b: (f32, f32), rect: &C4ShapeLayout, pad: f32) -> bool {
    let min_x = rect.x + pad;
    let min_y = rect.y + pad;
    let max_x = rect.x + rect.width - pad;
    let max_y = rect.y + rect.height - pad;
    if max_x <= min_x || max_y <= min_y {
        return false;
    }
    // Liang-Barsky segment/AABB clip.
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let mut t0 = 0.0f32;
    let mut t1 = 1.0f32;
    let edges = [
        (-dx, a.0 - min_x),
        (dx, max_x - a.0),
        (-dy, a.1 - min_y),
        (dy, max_y - a.1),
    ];
    for (p, q) in edges {
        if p.abs() < 1e-6 {
            if q < 0.0 {
                return false;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                if r > t1 {
                    return false;
                }
                if r > t0 {
                    t0 = r;
                }
            } else {
                if r < t0 {
                    return false;
                }
                if r < t1 {
                    t1 = r;
                }
            }
        }
    }
    t0 <= t1
}

/// Route a C4 relationship connector from `start` to `end`, detouring around
/// any unrelated shape the straight line would cut through. The detour is a
/// single vertical jog above or below the obstacle band, whichever is shorter
/// and itself clear, falling back to the straight line when no clear detour
/// exists (keeping behaviour stable on dense diagrams).
fn c4_route_around_shapes(
    start: (f32, f32),
    end: (f32, f32),
    from_id: &str,
    to_id: &str,
    shapes: &[C4ShapeLayout],
) -> Vec<(f32, f32)> {
    let pad = 4.0;
    let blockers: Vec<&C4ShapeLayout> = shapes
        .iter()
        .filter(|s| s.id != from_id && s.id != to_id)
        .filter(|s| c4_segment_hits_rect(start, end, s, pad))
        .collect();
    if blockers.is_empty() {
        return vec![start, end];
    }

    // Vertical extent of the obstacle band the straight line crosses.
    let band_top = blockers.iter().map(|s| s.y).fold(f32::INFINITY, f32::min);
    let band_bottom = blockers
        .iter()
        .map(|s| s.y + s.height)
        .fold(f32::NEG_INFINITY, f32::max);
    let clearance = 16.0;

    for detour_y in [band_top - clearance, band_bottom + clearance] {
        let candidate = vec![start, (start.0, detour_y), (end.0, detour_y), end];
        // Accept the detour only if none of its legs re-enter a blocker.
        let clear = candidate.windows(2).all(|seg| {
            !shapes
                .iter()
                .filter(|s| s.id != from_id && s.id != to_id)
                .any(|s| c4_segment_hits_rect(seg[0], seg[1], s, pad))
        });
        if clear {
            return candidate;
        }
    }
    // No clear orthogonal detour found; keep the direct line.
    vec![start, end]
}

fn c4_intersect_point(node: &C4ShapeLayout, end: (f32, f32)) -> (f32, f32) {
    let (x1, y1) = (node.x, node.y);
    let (x2, y2) = end;
    let from_center_x = x1 + node.width / 2.0;
    let from_center_y = y1 + node.height / 2.0;
    let dx = (x1 - x2).abs();
    let dy = (y1 - y2).abs();
    let tan_dyx = if dx.abs() < f32::EPSILON {
        0.0
    } else {
        dy / dx
    };
    let from_dyx = node.height / node.width;
    if (y1 - y2).abs() < f32::EPSILON && x1 < x2 {
        return (x1 + node.width, from_center_y);
    }
    if (y1 - y2).abs() < f32::EPSILON && x1 > x2 {
        return (x1, from_center_y);
    }
    if (x1 - x2).abs() < f32::EPSILON && y1 < y2 {
        return (from_center_x, y1 + node.height);
    }
    if (x1 - x2).abs() < f32::EPSILON && y1 > y2 {
        return (from_center_x, y1);
    }
    if x1 > x2 && y1 < y2 {
        if from_dyx >= tan_dyx {
            (x1, from_center_y + tan_dyx * node.width / 2.0)
        } else {
            (
                from_center_x - dx / dy * node.height / 2.0,
                y1 + node.height,
            )
        }
    } else if x1 < x2 && y1 < y2 {
        if from_dyx >= tan_dyx {
            (x1 + node.width, from_center_y + tan_dyx * node.width / 2.0)
        } else {
            (
                from_center_x + dx / dy * node.height / 2.0,
                y1 + node.height,
            )
        }
    } else if x1 < x2 && y1 > y2 {
        if from_dyx >= tan_dyx {
            (x1 + node.width, from_center_y - tan_dyx * node.width / 2.0)
        } else {
            (from_center_x + node.height / 2.0 * dx / dy, y1)
        }
    } else if x1 > x2 && y1 > y2 {
        if from_dyx >= tan_dyx {
            (x1, from_center_y - node.width / 2.0 * tan_dyx)
        } else {
            (from_center_x - node.height / 2.0 * dx / dy, y1)
        }
    } else {
        (from_center_x, from_center_y)
    }
}

#[derive(Clone, Copy)]
struct C4Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

fn resolve_c4_rel_label_offsets(
    rels: &mut [C4RelLayout],
    shapes: &[C4ShapeLayout],
    conf: &crate::config::C4Config,
) {
    if rels.is_empty() {
        return;
    }
    let shape_obstacles: Vec<C4Rect> = shapes
        .iter()
        .map(|shape| C4Rect {
            x: shape.x,
            y: shape.y,
            width: shape.width,
            height: shape.height,
        })
        .collect();
    let mut placed_labels: Vec<C4Rect> = Vec::with_capacity(rels.len());
    let step = (conf.message_font_size * 1.2).max(10.0);

    for rel in rels.iter_mut() {
        let dx = rel.end.0 - rel.start.0;
        let dy = rel.end.1 - rel.start.1;
        let len = (dx * dx + dy * dy).sqrt();
        let (tangent_x, tangent_y, normal_x, normal_y) = if len > 1e-3 {
            let tx = dx / len;
            let ty = dy / len;
            (tx, ty, -ty, tx)
        } else {
            (1.0, 0.0, 0.0, -1.0)
        };

        let mut candidates = Vec::with_capacity(64);
        candidates.push((0.0, 0.0));
        for ring in 1..=6 {
            let dist = step * ring as f32;
            for normal_sign in [-1.0f32, 1.0f32] {
                candidates.push((normal_x * dist * normal_sign, normal_y * dist * normal_sign));
                if ring <= 3 {
                    for tangent_sign in [-1.0f32, 1.0f32] {
                        let tangent_dist = dist * 0.75 * tangent_sign;
                        candidates.push((
                            normal_x * dist * normal_sign + tangent_x * tangent_dist,
                            normal_y * dist * normal_sign + tangent_y * tangent_dist,
                        ));
                    }
                }
            }
        }

        let mut best_delta = (0.0f32, 0.0f32);
        let mut best_rect = c4_rel_label_rect(rel, conf, (0.0, 0.0));
        let mut best_score = c4_rel_label_score(&best_rect, &shape_obstacles, &placed_labels, 0.0);

        for delta in candidates.into_iter().skip(1) {
            let rect = c4_rel_label_rect(rel, conf, delta);
            let displacement = (delta.0 * delta.0 + delta.1 * delta.1).sqrt();
            let score = c4_rel_label_score(&rect, &shape_obstacles, &placed_labels, displacement);
            if score < best_score {
                best_score = score;
                best_delta = delta;
                best_rect = rect;
                if best_score < 1e-3 {
                    break;
                }
            }
        }

        rel.offset_x += best_delta.0;
        rel.offset_y += best_delta.1;
        placed_labels.push(best_rect);
    }
}

fn c4_rel_label_rect(
    rel: &C4RelLayout,
    conf: &crate::config::C4Config,
    delta: (f32, f32),
) -> C4Rect {
    let center_x =
        rel.start.0.min(rel.end.0) + (rel.start.0 - rel.end.0).abs() / 2.0 + rel.offset_x + delta.0;
    let center_y =
        rel.start.1.min(rel.end.1) + (rel.start.1 - rel.end.1).abs() / 2.0 + rel.offset_y + delta.1;
    let primary_height = rel.label.height.max(conf.message_font_size);
    let secondary_height = rel
        .techn
        .as_ref()
        .map(|layout| layout.height.max(conf.message_font_size))
        .unwrap_or(0.0);
    let secondary_center_y = center_y + conf.message_font_size + 5.0;
    let top = if secondary_height > 0.0 {
        (center_y - primary_height / 2.0).min(secondary_center_y - secondary_height / 2.0)
    } else {
        center_y - primary_height / 2.0
    };
    let bottom = if secondary_height > 0.0 {
        (center_y + primary_height / 2.0).max(secondary_center_y + secondary_height / 2.0)
    } else {
        center_y + primary_height / 2.0
    };
    let width = rel
        .techn
        .as_ref()
        .map(|layout| layout.width)
        .unwrap_or(0.0)
        .max(rel.label.width)
        .max(conf.message_font_size * 1.2);

    C4Rect {
        x: center_x - width / 2.0,
        y: top,
        width,
        height: (bottom - top).max(primary_height),
    }
}

fn c4_rel_label_score(
    rect: &C4Rect,
    shape_obstacles: &[C4Rect],
    placed_labels: &[C4Rect],
    displacement: f32,
) -> f32 {
    let shape_overlap: f32 = shape_obstacles
        .iter()
        .map(|obstacle| c4_rect_overlap_area(*rect, *obstacle))
        .sum();
    let label_overlap: f32 = placed_labels
        .iter()
        .map(|placed| c4_rect_overlap_area(*rect, *placed))
        .sum();
    shape_overlap * 6.0 + label_overlap * 9.0 + displacement * 0.015
}

fn c4_rect_overlap_area(a: C4Rect, b: C4Rect) -> f32 {
    let ax2 = a.x + a.width;
    let ay2 = a.y + a.height;
    let bx2 = b.x + b.width;
    let by2 = b.y + b.height;
    let ix = ax2.min(bx2) - a.x.max(b.x);
    let iy = ay2.min(by2) - a.y.max(b.y);
    if ix <= 0.0 || iy <= 0.0 {
        return 0.0;
    }
    ix * iy
}
