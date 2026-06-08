use super::*;

/// Reorder each boundary's directly-contained containers so that connected
/// containers land in adjacent grid slots. The boundary packs in row-major
/// order with `c4ShapeInRow` columns, so a container's slot is determined by
/// its index in the order; this picks the order (a permutation) minimizing the
/// total grid-distance of relationships between same-boundary containers, plus
/// a pull toward the boundary edge for containers that talk to externals.
/// Hill-climbs over adjacent swaps from the declaration order (kept on ties so
/// the result stays close to what the author wrote). Deterministic.
fn reorder_boundary_containers(
    shapes_by_boundary: &mut std::collections::HashMap<String, Vec<String>>,
    c4: &crate::ir::C4Data,
    conf: &crate::config::C4Config,
) {
    let in_row = c4
        .c4_shape_in_row_override
        .unwrap_or(conf.c4_shape_in_row)
        .max(1);

    for (bid, ids) in shapes_by_boundary.iter_mut() {
        let n = ids.len();
        if n < 3 {
            continue; // nothing to reorder meaningfully
        }
        // Slot (row, col) for index i in the order.
        let slot = |i: usize| -> (f32, f32) {
            ((i / in_row) as f32, (i % in_row) as f32)
        };
        // Cost of an ordering. The dominant term is BETWEENNESS: a straight
        // line between two connected containers clips a box when a third
        // container lies between them on the same grid row or column. Penalize
        // each such case heavily — that's what produces the visible
        // "line through a box". Add a small grid-distance term as a tie-break
        // (shorter lines) and a pull of externally-connected containers toward
        // the boundary perimeter.
        let cost = |order: &[String]| -> f32 {
            let pos: std::collections::HashMap<&str, usize> =
                order.iter().enumerate().map(|(i, s)| (s.as_str(), i)).collect();
            let rows = n.div_ceil(in_row) as f32;
            let mut total = 0.0f32;
            for rel in &c4.rels {
                let fi = pos.get(rel.from.as_str());
                let ti = pos.get(rel.to.as_str());
                match (fi, ti) {
                    (Some(&a), Some(&b)) => {
                        let (ar, ac) = slot(a);
                        let (br, bc) = slot(b);
                        total += (ar - br).abs() + (ac - bc).abs();
                        // betweenness: any other container k strictly between
                        // a and b along a shared row or column.
                        for k in 0..n {
                            if k == a || k == b {
                                continue;
                            }
                            let (kr, kc) = slot(k);
                            let on_col =
                                ac == bc && kc == ac && (kr - ar) * (kr - br) < 0.0;
                            let on_row =
                                ar == br && kr == ar && (kc - ac) * (kc - bc) < 0.0;
                            if on_col || on_row {
                                total += 20.0;
                            }
                        }
                    }
                    (Some(&a), None) | (None, Some(&a)) => {
                        let (r, c) = slot(a);
                        let edge_r = r.min(rows - 1.0 - r);
                        let edge_c = c.min(in_row as f32 - 1.0 - c);
                        total += edge_r.min(edge_c);
                    }
                    (None, None) => {}
                }
            }
            total
        };

        // Hill-climb over adjacent swaps; keep improvements only.
        let mut best = ids.clone();
        let mut best_cost = cost(&best);
        let mut improved = true;
        let mut guard = 0;
        while improved && guard < 200 {
            guard += 1;
            improved = false;
            for i in 0..n {
                for j in (i + 1)..n {
                    let mut cand = best.clone();
                    cand.swap(i, j);
                    let c = cost(&cand);
                    if c + 1e-3 < best_cost {
                        best = cand;
                        best_cost = c;
                        improved = true;
                    }
                }
            }
        }
        let _ = bid;
        *ids = best;
    }
}

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

    // Reorder each boundary's containers so connected ones sit adjacent (no
    // container stranded between two it connects to). Runs before the layout;
    // deterministic; only when `optimize` is on.
    if conf.optimize {
        reorder_boundary_containers(&mut shapes_by_boundary, c4, conf);
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
    );

    // Placement refinement: reposition free externals (and, for the grid
    // engine, also the boundary's exterior arrangement) to reduce relationship
    // crossings. `"grid"` (default) runs a crossing-aware grid + local search;
    // `"force"` runs the continuous force-directed fallback; `"none"` keeps the
    // raw declaration-order packing. `force_layout = false` forces `"none"`.
    let placement = if conf.force_layout {
        conf.placement.as_str()
    } else {
        "none"
    };
    if placement != "none" {
        // shape id -> its direct parent boundary id (from the IR; the layout
        // structs don't retain this).
        let shape_parent: std::collections::HashMap<&str, &str> = c4
            .shapes
            .iter()
            .map(|s| (s.id.as_str(), s.parent_boundary.as_str()))
            .collect();
        match placement {
            "force" => force_refine_c4_layout(
                &mut shapes_out,
                &mut boundaries_out,
                &c4.rels,
                &boundary_map,
                &shape_parent,
                conf,
                &mut global_max_x,
                &mut global_max_y,
            ),
            _ => grid_refine_c4_layout(
                &mut shapes_out,
                &mut boundaries_out,
                &c4.rels,
                &boundary_map,
                &shape_parent,
                conf,
                &mut global_max_x,
                &mut global_max_y,
            ),
        }
    }

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
            estimate_text_width(&rel.label, label_font_size, rel_font_family, fast_metrics),
            c4_text_line_height(conf, label_font_size),
            rel_font_family,
            fast_metrics,
        );
        let techn_layout = rel.techn.as_ref().map(|t| {
            c4_text_layout(
                t,
                label_font_size,
                0.0,
                conf.wrap,
                estimate_text_width(t, label_font_size, rel_font_family, fast_metrics),
                c4_text_line_height(conf, label_font_size),
                rel_font_family,
                fast_metrics,
            )
        });
        rels_out.push(C4RelLayout {
            kind: rel.kind,
            from: rel.from.clone(),
            to: rel.to.clone(),
            label: label_layout,
            techn: techn_layout,
            start,
            end,
            points: vec![start, end],
            label_base: (
                (start.0 + end.0) / 2.0,
                (start.1 + end.1) / 2.0,
            ),
            curved: false,
            bow: 0.0,
            offset_x: rel.offset_x,
            offset_y: rel.offset_y,
            line_color: rel.line_color.clone(),
            text_color: rel.text_color.clone(),
        });
    }
    // Joint refinement: optionally run simulated annealing over the placement
    // of free external shapes, routing and scoring each candidate so placement
    // is routing-aware (moves blockers out of edge paths, pulls externals in).
    if conf.optimize && conf.force_layout {
        let shape_parent: std::collections::HashMap<&str, &str> = c4
            .shapes
            .iter()
            .map(|s| (s.id.as_str(), s.parent_boundary.as_str()))
            .collect();
        anneal_c4_placement(
            &mut shapes_out,
            &mut boundaries_out,
            &rels_out,
            &boundary_map,
            &shape_parent,
            conf,
            &mut global_max_x,
            &mut global_max_y,
        );
    }

    // Final routing pass on the (possibly annealed) placement.
    let cw = (global_max_x - conf.diagram_margin_x + 2.0 * conf.diagram_margin_x).max(1.0);
    let ch = (global_max_y - conf.diagram_margin_y + 2.0 * conf.diagram_margin_y).max(1.0);
    rels_out = route_and_score_c4(&shapes_out, &rels_out, conf, cw, ch).0;

    // Final port-optimization pass: with node placement fixed, search port
    // orderings that further lower the unified fitness (kills same-hub T-junction
    // crossings the placement search can't reach). Only when optimizing.
    if conf.optimize {
        optimize_c4_ports(&mut rels_out, &shapes_out, conf, cw, ch);
    }

    resolve_c4_rel_label_offsets(&mut rels_out, &shapes_out, &boundaries_out, conf);

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
            points: if rel.points.len() >= 2 {
                rel.points.clone()
            } else {
                vec![rel.start, rel.end]
            },
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

    // Content bounds must include EDGE geometry (and relationship labels), not
    // just shapes/boundaries — routed edges can detour above/left of the
    // topmost shape (e.g. an external line that goes up over the diagram). If
    // we sized the canvas to shapes only, those segments and their arrowheads
    // would be clipped at the top/left edge.
    let mut min_x = conf.diagram_margin_x;
    let mut min_y = 0.0f32;
    let mut max_x = global_max_x;
    let mut max_y = global_max_y;
    for shape in &shapes_out {
        min_x = min_x.min(shape.x);
        min_y = min_y.min(shape.y);
        max_x = max_x.max(shape.x + shape.width);
        max_y = max_y.max(shape.y + shape.height);
    }
    // Boundary rectangles, plus headroom for the boundary's title text which is
    // drawn above the rectangle's top edge.
    for b in &boundaries_out {
        let label_h = b.label.height + b.boundary_type.as_ref().map(|t| t.height).unwrap_or(0.0);
        min_x = min_x.min(b.x);
        min_y = min_y.min(b.y - label_h);
        max_x = max_x.max(b.x + b.width);
        max_y = max_y.max(b.y + b.height);
    }
    for e in &edges {
        for &(px, py) in &e.points {
            min_x = min_x.min(px);
            min_y = min_y.min(py);
            max_x = max_x.max(px);
            max_y = max_y.max(py);
        }
    }
    // Relationship labels sit at label_base + offset; include a rough box.
    for rel in &rels_out {
        let lx = rel.label_base.0 + rel.offset_x;
        let ly = rel.label_base.1 + rel.offset_y;
        let lw = rel.label.width.max(rel.techn.as_ref().map(|t| t.width).unwrap_or(0.0));
        let lh = rel.label.height + rel.techn.as_ref().map(|t| t.height).unwrap_or(0.0);
        min_x = min_x.min(lx - lw / 2.0);
        min_y = min_y.min(ly - lh / 2.0);
        max_x = max_x.max(lx + lw / 2.0);
        max_y = max_y.max(ly + lh / 2.0);
    }
    let pad = conf.diagram_margin_y;
    min_x -= pad;
    min_y -= pad;
    max_x += pad;
    max_y += pad;
    let width = (max_x - min_x).max(1.0);
    let height = (max_y - min_y).max(1.0);
    // viewBox starts at the true content min so nothing is clipped.
    let viewbox_x = min_x;
    let viewbox_y = min_y;
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

/// A rigid cluster of shapes that move together during force refinement.
/// Either all shapes belonging to one top-level boundary (so the boundary
/// rectangle and its internal layout are preserved), or a single free shape.
struct ForceGroup {
    /// Indices into `shapes_out`.
    members: Vec<usize>,
    /// Current top-left of the group's bounding box (of its member shapes).
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    /// Extra drawn extent beyond the member shapes, for groups whose rendered
    /// rectangle is larger than its contents (boundary header + padding).
    /// Used so neighbours keep clear of the *drawn* box, not just the shapes.
    pad_top: f32,
    pad_other: f32,
    /// Whether this group is a (heavy, near-anchored) boundary rather than a
    /// free shape. Multi-shape boundaries should stay put while light
    /// externals orbit them.
    is_boundary: bool,
    /// Inverse mass: scales how far this group moves per iteration. Heavy
    /// boundaries get a small value so they barely drift; free externals get
    /// 1.0 so they relocate freely.
    inv_mass: f32,
    /// Accumulated displacement for the current iteration.
    dx: f32,
    dy: f32,
}

impl ForceGroup {
    fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Half-width of the drawn box (shapes + side padding).
    fn half_w(&self) -> f32 {
        self.width / 2.0 + self.pad_other
    }

    /// Half-height of the drawn box (shapes + header above + padding below).
    fn half_h(&self) -> f32 {
        (self.height + self.pad_top + self.pad_other) / 2.0
    }

    /// Center of the drawn box, which sits below the shape center by half the
    /// header so the header is accounted for above the shapes.
    fn drawn_center(&self) -> (f32, f32) {
        let (cx, cy) = self.center();
        (cx, cy - self.pad_top / 2.0 + self.pad_other / 2.0)
    }
}

/// Map every shape id to the id of its TOP-LEVEL ancestor boundary (empty
/// string if the shape lives directly at the diagram root). Walks the
/// boundary parent chain so nested boundaries collapse into their root.
fn top_level_boundary_of(
    boundary_id: &str,
    boundary_map: &std::collections::HashMap<String, &crate::ir::C4Boundary>,
) -> String {
    // The parser wraps everything in a synthetic "global" root boundary, so
    // treat both "" and "global" as the diagram root: a shape parented
    // directly under them is free-floating, not part of a rigid group.
    if boundary_id.is_empty() || is_c4_root_boundary(boundary_id) {
        return String::new();
    }
    let mut current = boundary_id.to_string();
    // Walk up to the highest ancestor that is still a real (non-root)
    // boundary — that body is what moves rigidly.
    loop {
        match boundary_map.get(current.as_str()) {
            Some(boundary)
                if !boundary.parent_boundary.is_empty()
                    && !is_c4_root_boundary(&boundary.parent_boundary) =>
            {
                current = boundary.parent_boundary.clone();
            }
            _ => return current,
        }
    }
}

fn is_c4_root_boundary(id: &str) -> bool {
    id == "global"
}

/// Force-directed refinement of the initial row-packed C4 placement.
///
/// Treats each top-level boundary as a single rigid body (its contents and
/// rectangle never deform) and each free-floating shape as its own body.
/// Relationships act as attractive springs between the bodies owning their
/// endpoints; overlapping bodies repel. The result clusters external systems
/// around whatever they connect to instead of stranding them in declaration
/// order, shortening arrows without rearranging anything inside a boundary.
#[allow(clippy::too_many_arguments)]
fn force_refine_c4_layout(
    shapes_out: &mut [C4ShapeLayout],
    boundaries_out: &mut [C4BoundaryLayout],
    rels: &[crate::ir::C4Rel],
    boundary_map: &std::collections::HashMap<String, &crate::ir::C4Boundary>,
    shape_parent: &std::collections::HashMap<&str, &str>,
    conf: &crate::config::C4Config,
    global_max_x: &mut f32,
    global_max_y: &mut f32,
) {
    if shapes_out.len() < 2 || rels.is_empty() {
        return;
    }

    // shape id -> index in shapes_out
    let mut shape_index: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::with_capacity(shapes_out.len());
    for (idx, shape) in shapes_out.iter().enumerate() {
        shape_index.insert(shape.id.as_str(), idx);
    }

    // Drawn header height of each top-level boundary (label + type + pad),
    // so neighbours keep clear of the boundary rectangle's title strip, not
    // just its contained shapes.
    let pad = conf.c4_shape_margin.max(16.0);
    let mut boundary_header: std::collections::HashMap<&str, f32> =
        std::collections::HashMap::new();
    for b in boundaries_out.iter() {
        let header = b.label.height + b.boundary_type.as_ref().map(|t| t.height).unwrap_or(0.0) + pad;
        boundary_header.insert(b.id.as_str(), header);
    }

    // Partition shapes into rigid groups keyed by their top-level boundary.
    // Free shapes (no boundary) each get a unique synthetic key so they stay
    // independent bodies.
    let mut group_of_shape: Vec<usize> = vec![0; shapes_out.len()];
    let mut group_key_to_idx: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut groups: Vec<ForceGroup> = Vec::new();
    for (idx, shape) in shapes_out.iter().enumerate() {
        let parent = shape_parent.get(shape.id.as_str()).copied().unwrap_or("");
        let root = top_level_boundary_of(parent, boundary_map);
        let (key, pad_top, pad_other, is_boundary) = if root.is_empty() {
            // Unique per free shape; drawn box == shape box.
            (format!("__free__{idx}"), 0.0, 0.0, false)
        } else {
            let header = boundary_header.get(root.as_str()).copied().unwrap_or(pad);
            (format!("__b__{root}"), header, pad, true)
        };
        let group_idx = *group_key_to_idx.entry(key).or_insert_with(|| {
            groups.push(ForceGroup {
                members: Vec::new(),
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
                pad_top,
                pad_other,
                is_boundary,
                inv_mass: 1.0,
                dx: 0.0,
                dy: 0.0,
            });
            groups.len() - 1
        });
        groups[group_idx].members.push(idx);
        group_of_shape[idx] = group_idx;
    }

    if groups.len() < 2 {
        return; // Nothing to move relative to anything else.
    }

    // Compute each group's bounding box from its members, padding boundary
    // groups so their drawn rectangle (which extends beyond the shapes) is
    // accounted for in repulsion.
    let recompute_group_bounds = |groups: &mut Vec<ForceGroup>, shapes: &[C4ShapeLayout]| {
        for group in groups.iter_mut() {
            let mut min_x = f32::MAX;
            let mut min_y = f32::MAX;
            let mut max_x = f32::MIN;
            let mut max_y = f32::MIN;
            for &m in &group.members {
                let s = &shapes[m];
                min_x = min_x.min(s.x);
                min_y = min_y.min(s.y);
                max_x = max_x.max(s.x + s.width);
                max_y = max_y.max(s.y + s.height);
            }
            group.x = min_x;
            group.y = min_y;
            group.width = (max_x - min_x).max(1.0);
            group.height = (max_y - min_y).max(1.0);
        }
    };
    recompute_group_bounds(&mut groups, shapes_out);

    // Inverse mass: a boundary is a heavy anchor (barely drifts) so light
    // externals orbit it rather than dragging it around. Free shapes all share
    // unit mass.
    for g in groups.iter_mut() {
        g.inv_mass = if g.is_boundary { 0.15 } else { 1.0 };
    }

    // Edges between groups (skip self-edges within one rigid body). Each edge
    // records, for both endpoints, the connection point's offset from its
    // group's origin (top-left). That lets the spring pull on the *actual*
    // connection point — so an external attached to a shape deep inside a
    // boundary is drawn toward that shape's edge of the boundary, letting it
    // settle alongside the boundary instead of merely above it.
    struct Edge {
        ga: usize,
        gb: usize,
        a_off: (f32, f32),
        b_off: (f32, f32),
    }
    let group_origin = |gi: usize, groups: &[ForceGroup]| (groups[gi].x, groups[gi].y);
    let mut edges: Vec<Edge> = Vec::new();
    for rel in rels {
        let (Some(&fi), Some(&ti)) =
            (shape_index.get(rel.from.as_str()), shape_index.get(rel.to.as_str()))
        else {
            continue;
        };
        let (ga, gb) = (group_of_shape[fi], group_of_shape[ti]);
        if ga == gb {
            continue;
        }
        let (gax, gay) = group_origin(ga, &groups);
        let (gbx, gby) = group_origin(gb, &groups);
        let fa = &shapes_out[fi];
        let tb = &shapes_out[ti];
        edges.push(Edge {
            ga,
            gb,
            a_off: (
                fa.x + fa.width / 2.0 - gax,
                fa.y + fa.height / 2.0 - gay,
            ),
            b_off: (
                tb.x + tb.width / 2.0 - gbx,
                tb.y + tb.height / 2.0 - gby,
            ),
        });
    }
    if edges.is_empty() {
        return;
    }

    // Snapshot the initial group origins and the initial inter-group
    // relationship length (measured between connection points), so we can
    // refuse a refinement that doesn't actually help.
    let initial_origins: Vec<(f32, f32)> =
        groups.iter().map(|g| (g.x, g.y)).collect();
    let endpoints = |e: &Edge, groups: &[ForceGroup]| -> ((f32, f32), (f32, f32)) {
        let a = (groups[e.ga].x + e.a_off.0, groups[e.ga].y + e.a_off.1);
        let b = (groups[e.gb].x + e.b_off.0, groups[e.gb].y + e.b_off.1);
        (a, b)
    };
    let rel_length = |groups: &[ForceGroup], edges: &[Edge]| -> f32 {
        edges
            .iter()
            .map(|e| {
                let ((ax, ay), (bx, by)) = endpoints(e, groups);
                ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt()
            })
            .sum()
    };
    let initial_length = rel_length(&groups, &edges);

    // Force constants. Attraction is strong enough to actually relocate free
    // externals around the boundary (not just nudge them); repulsion keeps
    // bodies from overlapping. Both run for many cooled iterations.
    let margin = conf.c4_shape_margin.max(16.0);
    let iterations = 600usize;
    let attract_k = 0.08f32;
    let repulse_pad = margin;

    for iter in 0..iterations {
        let cooling = 1.0 - (iter as f32 / iterations as f32);
        for g in groups.iter_mut() {
            g.dx = 0.0;
            g.dy = 0.0;
        }

        // Attraction along relationships: pull the two *connection points*
        // together. Because the target is the endpoint shape (not the group
        // center), an external attached to a shape on one side of a boundary
        // is drawn to that side and can settle beside the boundary rather than
        // piling up above it. The desired separation is just a small margin,
        // so endpoints end up adjacent (repulsion stops actual overlap).
        for e in &edges {
            let ((ax, ay), (bx, by)) = endpoints(e, &groups);
            let dx = bx - ax;
            let dy = by - ay;
            let dist = (dx * dx + dy * dy).sqrt().max(1.0);
            let force = attract_k * (dist - margin);
            let ux = dx / dist;
            let uy = dy / dist;
            groups[e.ga].dx += ux * force;
            groups[e.ga].dy += uy * force;
            groups[e.gb].dx -= ux * force;
            groups[e.gb].dy -= uy * force;
        }

        // Repulsion: any two groups whose padded *drawn* AABBs overlap push
        // apart along the axis of least penetration (keeps separation
        // orthogonal, which suits the boxy C4 aesthetic). Using the drawn box
        // — which for a boundary includes its header strip — stops free
        // shapes from settling on top of a boundary's title.
        for i in 0..groups.len() {
            for j in (i + 1)..groups.len() {
                let (ax, ay) = groups[i].drawn_center();
                let (bx, by) = groups[j].drawn_center();
                let overlap_x =
                    groups[i].half_w() + groups[j].half_w() + repulse_pad - (ax - bx).abs();
                let overlap_y =
                    groups[i].half_h() + groups[j].half_h() + repulse_pad - (ay - by).abs();
                if overlap_x > 0.0 && overlap_y > 0.0 {
                    if overlap_x < overlap_y {
                        let push = overlap_x * 0.5;
                        let dir = if ax <= bx { -1.0 } else { 1.0 };
                        groups[i].dx += dir * push;
                        groups[j].dx -= dir * push;
                    } else {
                        let push = overlap_y * 0.5;
                        let dir = if ay <= by { -1.0 } else { 1.0 };
                        groups[i].dy += dir * push;
                        groups[j].dy -= dir * push;
                    }
                }
            }
        }

        // Apply with cooling and per-group inverse mass, then refresh centers
        // for the next iteration.
        for g in groups.iter_mut() {
            let max_step = 60.0 * cooling + 4.0;
            let dx = (g.dx * g.inv_mass).clamp(-max_step, max_step);
            let dy = (g.dy * g.inv_mass).clamp(-max_step, max_step);
            g.x += dx;
            g.y += dy;
        }
    }

    // Refuse the refinement unless it meaningfully shortens relationships.
    // This keeps the force pass from disturbing diagrams whose row-packed
    // placement was already compact (small graphs, or shapes already declared
    // next to what they connect to). 2% guards against churn-for-nothing.
    let final_length = rel_length(&groups, &edges);
    if final_length >= initial_length * 0.98 {
        for (g, &(ox, oy)) in groups.iter_mut().zip(&initial_origins) {
            g.x = ox;
            g.y = oy;
        }
        return;
    }

    // Translate every shape rigidly by its group's net displacement. Each
    // group's `x`/`y` is its post-sim origin; the members still sit at their
    // original origin, so the shift is the difference.
    for group in &groups {
        let mut orig_x = f32::MAX;
        let mut orig_y = f32::MAX;
        for &m in &group.members {
            orig_x = orig_x.min(shapes_out[m].x);
            orig_y = orig_y.min(shapes_out[m].y);
        }
        let shift_x = group.x - orig_x;
        let shift_y = group.y - orig_y;
        for &m in &group.members {
            shapes_out[m].x += shift_x;
            shapes_out[m].y += shift_y;
        }
    }

    // Renormalize so the diagram starts at the configured margin (groups may
    // have drifted negative), and recompute boundary rectangles + bounds.
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    for s in shapes_out.iter() {
        min_x = min_x.min(s.x);
        min_y = min_y.min(s.y);
    }
    let norm_x = conf.diagram_margin_x - min_x;
    let norm_y = conf.diagram_margin_y - min_y;
    for s in shapes_out.iter_mut() {
        s.x += norm_x;
        s.y += norm_y;
    }

    recompute_c4_boundary_rects(shapes_out, boundaries_out, boundary_map, shape_parent, conf);

    let mut max_x = conf.diagram_margin_x;
    let mut max_y = conf.diagram_margin_y;
    for s in shapes_out.iter() {
        max_x = max_x.max(s.x + s.width);
        max_y = max_y.max(s.y + s.height);
    }
    for b in boundaries_out.iter() {
        max_x = max_x.max(b.x + b.width);
        max_y = max_y.max(b.y + b.height);
    }
    *global_max_x = max_x;
    *global_max_y = max_y;
}

/// Recompute each boundary's rectangle to tightly wrap the shapes it contains
/// (transitively) plus padding for its label and nested boundaries, after the
/// force pass has moved bodies around. A boundary that itself sits inside
/// another boundary is wrapped first so the parent encloses it.
fn recompute_c4_boundary_rects(
    shapes_out: &[C4ShapeLayout],
    boundaries_out: &mut [C4BoundaryLayout],
    boundary_map: &std::collections::HashMap<String, &crate::ir::C4Boundary>,
    shape_parent: &std::collections::HashMap<&str, &str>,
    conf: &crate::config::C4Config,
) {
    // For each boundary, collect every shape whose top-level-or-intermediate
    // chain passes through it.
    let pad = conf.c4_shape_margin.max(16.0);

    // Order boundaries innermost-first (deepest nesting) so a parent sees its
    // children's freshly computed rects. Depth = parent-chain length.
    let depth_of = |id: &str| -> usize {
        let mut current = id.to_string();
        let mut d = 0;
        while let Some(b) = boundary_map.get(current.as_str()) {
            if b.parent_boundary.is_empty() {
                break;
            }
            current = b.parent_boundary.clone();
            d += 1;
        }
        d
    };
    let mut order: Vec<usize> = (0..boundaries_out.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(depth_of(boundaries_out[i].id.as_str())));

    // Precompute, per boundary id, the set of shapes contained transitively.
    let contains = |boundary_id: &str, shape: &C4ShapeLayout| -> bool {
        let mut cur = shape_parent.get(shape.id.as_str()).copied().unwrap_or("");
        loop {
            if cur == boundary_id {
                return true;
            }
            match boundary_map.get(cur) {
                Some(b) if !b.parent_boundary.is_empty() => cur = b.parent_boundary.as_str(),
                _ => return cur == boundary_id,
            }
        }
    };

    // We also need child-boundary rects for the parent wrap; gather current
    // rects keyed by id as we go.
    let mut rect_by_id: std::collections::HashMap<String, (f32, f32, f32, f32)> =
        std::collections::HashMap::new();

    for &bi in &order {
        let bid = boundaries_out[bi].id.clone();
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for s in shapes_out.iter() {
            if contains(&bid, s) {
                min_x = min_x.min(s.x);
                min_y = min_y.min(s.y);
                max_x = max_x.max(s.x + s.width);
                max_y = max_y.max(s.y + s.height);
            }
        }
        // Enclose any already-computed child boundary rects.
        for (cid, &(cx, cy, cw, ch)) in &rect_by_id {
            if boundary_map
                .get(cid.as_str())
                .is_some_and(|cb| cb.parent_boundary == bid)
            {
                min_x = min_x.min(cx);
                min_y = min_y.min(cy);
                max_x = max_x.max(cx + cw);
                max_y = max_y.max(cy + ch);
            }
        }
        if min_x > max_x {
            continue; // empty boundary; leave as-is
        }
        // Header space above for the label/type lines.
        let header = boundaries_out[bi].label.height
            + boundaries_out[bi]
                .boundary_type
                .as_ref()
                .map(|t| t.height)
                .unwrap_or(0.0)
            + pad;
        let x = min_x - pad;
        let y = min_y - header;
        let width = (max_x - min_x) + pad * 2.0;
        let height = (max_y - min_y) + header + pad;
        boundaries_out[bi].x = x;
        boundaries_out[bi].y = y;
        boundaries_out[bi].width = width;
        boundaries_out[bi].height = height;
        rect_by_id.insert(bid, (x, y, width, height));
    }
}

// ---------------------------------------------------------------------------
// Grid + local-search placement
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
struct Pt {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy)]
struct Box {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Box {
    fn center(&self) -> Pt {
        Pt {
            x: self.x + self.w / 2.0,
            y: self.y + self.h / 2.0,
        }
    }
}

/// True if segments p1-p2 and p3-p4 properly cross (interiors intersect).
fn segments_cross(p1: Pt, p2: Pt, p3: Pt, p4: Pt) -> bool {
    let d = |a: Pt, b: Pt, c: Pt| (c.y - a.y) * (b.x - a.x) - (b.y - a.y) * (c.x - a.x);
    let d1 = d(p3, p4, p1);
    let d2 = d(p3, p4, p2);
    let d3 = d(p1, p2, p3);
    let d4 = d(p1, p2, p4);
    ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
}

/// True if segments p1-p2 and p3-p4 intersect AT ALL — proper crossing, a
/// T-junction (one segment's endpoint/elbow lands on the other), or a collinear
/// overlap. Used for visual quality: a line whose corner touches another line
/// reads as a crossing even though it isn't a "proper" interior intersection.
fn segments_touch(p1: Pt, p2: Pt, p3: Pt, p4: Pt) -> bool {
    let o = |a: Pt, b: Pt, c: Pt| -> i32 {
        let v = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
        if v.abs() < 1e-4 {
            0
        } else if v > 0.0 {
            1
        } else {
            -1
        }
    };
    let on = |a: Pt, b: Pt, p: Pt| -> bool {
        p.x >= a.x.min(b.x) - 0.5
            && p.x <= a.x.max(b.x) + 0.5
            && p.y >= a.y.min(b.y) - 0.5
            && p.y <= a.y.max(b.y) + 0.5
    };
    let d1 = o(p3, p4, p1);
    let d2 = o(p3, p4, p2);
    let d3 = o(p1, p2, p3);
    let d4 = o(p1, p2, p4);
    if d1 != d2 && d3 != d4 {
        return true;
    }
    (d1 == 0 && on(p3, p4, p1))
        || (d2 == 0 && on(p3, p4, p2))
        || (d3 == 0 && on(p1, p2, p3))
        || (d4 == 0 && on(p1, p2, p4))
}

/// Length of the portion of segment a-b that lies inside rectangle `r`
/// (Liang-Barsky clip), or `None` if it doesn't enter. Used to penalize edges
/// that cut across the boundary rectangle.
fn segment_box_overlap_len(a: Pt, b: Pt, r: Box) -> Option<f32> {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let p = [-dx, dx, -dy, dy];
    let q = [a.x - r.x, r.x + r.w - a.x, a.y - r.y, r.y + r.h - a.y];
    let mut u1 = 0.0f32;
    let mut u2 = 1.0f32;
    for k in 0..4 {
        if p[k].abs() < 1e-6 {
            if q[k] < 0.0 {
                return None;
            }
        } else {
            let t = q[k] / p[k];
            if p[k] < 0.0 {
                if t > u2 {
                    return None;
                }
                if t > u1 {
                    u1 = t;
                }
            } else {
                if t < u1 {
                    return None;
                }
                if t < u2 {
                    u2 = t;
                }
            }
        }
    }
    if u2 < u1 {
        return None;
    }
    let len = ((dx * dx + dy * dy).sqrt()) * (u2 - u1);
    if len > 1.0 { Some(len) } else { None }
}

/// True if segment a-b passes through the interior of rectangle `r`
/// (shrunk slightly so a line merely grazing an edge or terminating at the
/// box border doesn't count).
fn segment_hits_box(a: Pt, b: Pt, r: Box) -> bool {
    let inset = 1.0f32;
    let rx = r.x + inset;
    let ry = r.y + inset;
    let rw = (r.w - 2.0 * inset).max(0.0);
    let rh = (r.h - 2.0 * inset).max(0.0);
    if rw <= 0.0 || rh <= 0.0 {
        return false;
    }
    // Endpoint inside?
    let inside = |p: Pt| p.x > rx && p.x < rx + rw && p.y > ry && p.y < ry + rh;
    if inside(a) || inside(b) {
        return true;
    }
    let corners = [
        Pt { x: rx, y: ry },
        Pt { x: rx + rw, y: ry },
        Pt {
            x: rx + rw,
            y: ry + rh,
        },
        Pt { x: rx, y: ry + rh },
    ];
    for i in 0..4 {
        if segments_cross(a, b, corners[i], corners[(i + 1) % 4]) {
            return true;
        }
    }
    false
}

/// Where the straight line toward `target` leaves the border of box `r`.
fn box_border_point(r: Box, target: Pt) -> Pt {
    let c = r.center();
    let dx = target.x - c.x;
    let dy = target.y - c.y;
    if dx.abs() < 1e-3 && dy.abs() < 1e-3 {
        return c;
    }
    let hw = r.w / 2.0;
    let hh = r.h / 2.0;
    let scale_x = if dx.abs() > 1e-3 {
        hw / dx.abs()
    } else {
        f32::INFINITY
    };
    let scale_y = if dy.abs() > 1e-3 {
        hh / dy.abs()
    } else {
        f32::INFINITY
    };
    let t = scale_x.min(scale_y);
    Pt {
        x: c.x + dx * t,
        y: c.y + dy * t,
    }
}

/// A movable unit during grid placement: a single free shape (`shape_idx`) or,
/// conceptually, an anchored boundary block (`shape_idx == None`).
struct Unit {
    /// Indices into shapes_out (one for a free shape; many for a boundary).
    members: Vec<usize>,
    /// Movable footprint (the union box of members).
    bbox: Box,
    movable: bool,
}

/// Crossing-aware grid + local-search placement. Free externals are assigned to
/// cells on a ring around the anchored boundary block; a hill-climb over cell
/// assignments minimizes (edge crossings, edge-through-box, length). Overlaps
/// are impossible because each cell holds at most one unit.
#[allow(clippy::too_many_arguments)]
fn grid_refine_c4_layout(
    shapes_out: &mut [C4ShapeLayout],
    boundaries_out: &mut [C4BoundaryLayout],
    rels: &[crate::ir::C4Rel],
    boundary_map: &std::collections::HashMap<String, &crate::ir::C4Boundary>,
    shape_parent: &std::collections::HashMap<&str, &str>,
    conf: &crate::config::C4Config,
    global_max_x: &mut f32,
    global_max_y: &mut f32,
) {
    if shapes_out.len() < 2 || rels.is_empty() {
        return;
    }

    let shape_index: std::collections::HashMap<&str, usize> = shapes_out
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.as_str(), i))
        .collect();

    // Partition shapes into units: one per top-level boundary (anchored,
    // rigid) + one per free shape (movable).
    let mut unit_of_shape: Vec<usize> = vec![usize::MAX; shapes_out.len()];
    let mut key_to_unit: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut units: Vec<Unit> = Vec::new();
    for (idx, shape) in shapes_out.iter().enumerate() {
        let parent = shape_parent.get(shape.id.as_str()).copied().unwrap_or("");
        let root = top_level_boundary_of(parent, boundary_map);
        let (key, movable) = if root.is_empty() {
            (format!("__free__{idx}"), true)
        } else {
            (format!("__b__{root}"), false)
        };
        let unit_idx = *key_to_unit.entry(key).or_insert_with(|| {
            units.push(Unit {
                members: Vec::new(),
                bbox: Box {
                    x: 0.0,
                    y: 0.0,
                    w: 0.0,
                    h: 0.0,
                },
                movable,
            });
            units.len() - 1
        });
        units[unit_idx].members.push(idx);
        unit_of_shape[idx] = unit_idx;
    }

    let movable_units: Vec<usize> = (0..units.len()).filter(|&u| units[u].movable).collect();
    let anchored_units: Vec<usize> = (0..units.len()).filter(|&u| !units[u].movable).collect();
    // Need at least one anchored block and some movable externals to do useful
    // ring placement; otherwise leave the packing alone.
    if movable_units.is_empty() || anchored_units.is_empty() {
        return;
    }

    // Compute unit bounding boxes from member shapes.
    let unit_bbox = |members: &[usize], shapes: &[C4ShapeLayout]| -> Box {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for &m in members {
            let s = &shapes[m];
            min_x = min_x.min(s.x);
            min_y = min_y.min(s.y);
            max_x = max_x.max(s.x + s.width);
            max_y = max_y.max(s.y + s.height);
        }
        Box {
            x: min_x,
            y: min_y,
            w: max_x - min_x,
            h: max_y - min_y,
        }
    };
    for u in 0..units.len() {
        units[u].bbox = unit_bbox(&units[u].members, shapes_out);
    }

    // Spacing between grid cells (and the ring's gap from the boundary). The
    // base shape margin alone reads as crowded, so scale it up by the
    // configurable `grid_gap` factor for breathing room.
    let pad = (conf.c4_shape_margin.max(24.0)) * conf.grid_gap.max(1.0);
    let mut anchor = units[anchored_units[0]].bbox;
    for &u in &anchored_units[1..] {
        let b = units[u].bbox;
        let x0 = anchor.x.min(b.x);
        let y0 = anchor.y.min(b.y);
        let x1 = (anchor.x + anchor.w).max(b.x + b.w);
        let y1 = (anchor.y + anchor.h).max(b.y + b.h);
        anchor = Box {
            x: x0,
            y: y0,
            w: x1 - x0,
            h: y1 - y0,
        };
    }

    // Cell size: large enough for the biggest movable external + margin.
    let mut cell_w = 0.0f32;
    let mut cell_h = 0.0f32;
    for &u in &movable_units {
        cell_w = cell_w.max(units[u].bbox.w);
        cell_h = cell_h.max(units[u].bbox.h);
    }
    cell_w += pad;
    cell_h += pad;

    // Build a ring of candidate cells around the anchor: a configurable number
    // of rows above & below (spanning the anchor width + one cell each side)
    // and columns left & right (spanning the anchor height). Enough cells for
    // all externals even in the worst case.
    let cols_across = ((anchor.w / cell_w).ceil() as i32).max(1) + 2;
    let rows_down = ((anchor.h / cell_h).ceil() as i32).max(1);
    let mut cells: Vec<Pt> = Vec::new();
    // Rows above and below.
    for band in 0..3i32 {
        // up to 3 stacked rows each side if needed
        for side in [-1i32, 1i32] {
            let cy = if side < 0 {
                anchor.y - pad - cell_h / 2.0 - band as f32 * cell_h
            } else {
                anchor.y + anchor.h + pad + cell_h / 2.0 + band as f32 * cell_h
            };
            let start_x = anchor.x + anchor.w / 2.0 - (cols_across as f32 - 1.0) * cell_w / 2.0;
            for c in 0..cols_across {
                cells.push(Pt {
                    x: start_x + c as f32 * cell_w,
                    y: cy,
                });
            }
        }
    }
    // Columns left and right.
    for band in 0..2i32 {
        for side in [-1i32, 1i32] {
            let cx = if side < 0 {
                anchor.x - pad - cell_w / 2.0 - band as f32 * cell_w
            } else {
                anchor.x + anchor.w + pad + cell_w / 2.0 + band as f32 * cell_w
            };
            let start_y = anchor.y + anchor.h / 2.0 - (rows_down as f32 - 1.0) * cell_h / 2.0;
            for r in 0..rows_down {
                cells.push(Pt {
                    x: cx,
                    y: start_y + r as f32 * cell_h,
                });
            }
        }
    }
    if cells.len() < movable_units.len() {
        return; // Not enough ring cells; bail rather than overlap.
    }

    // Edges as (unit_a, unit_b, from_shape_idx, to_shape_idx), skipping
    // intra-unit relationships.
    struct GEdge {
        ua: usize,
        ub: usize,
        fa: usize,
        tb: usize,
    }
    let mut gedges: Vec<GEdge> = Vec::new();
    for rel in rels {
        let (Some(&fi), Some(&ti)) = (
            shape_index.get(rel.from.as_str()),
            shape_index.get(rel.to.as_str()),
        ) else {
            continue;
        };
        let (ua, ub) = (unit_of_shape[fi], unit_of_shape[ti]);
        if ua != ub {
            gedges.push(GEdge {
                ua,
                ub,
                fa: fi,
                tb: ti,
            });
        }
    }
    if gedges.is_empty() {
        return;
    }

    // assignment[movable_index] = cell index (into `cells`), or usize::MAX.
    // Each movable unit's shape members translate so the unit bbox centers on
    // the chosen cell. We work in terms of a per-unit translation we can apply
    // for cost evaluation without mutating shapes_out until the end.
    let n_mov = movable_units.len();

    // For cost evaluation we need, per shape, its current center given the
    // candidate translations of its (movable) unit. Anchored shapes never move.
    let shape_box = |idx: usize, translate: (f32, f32)| -> Box {
        let s = &shapes_out[idx];
        Box {
            x: s.x + translate.0,
            y: s.y + translate.1,
            w: s.width,
            h: s.height,
        }
    };

    // translation for a movable unit given its assigned cell.
    let unit_translation = |u: usize, cell: usize, cells: &[Pt]| -> (f32, f32) {
        let bb = units[u].bbox;
        let cx = bb.x + bb.w / 2.0;
        let cy = bb.y + bb.h / 2.0;
        (cells[cell].x - cx, cells[cell].y - cy)
    };

    // Cost weights. Structural defects (crossings, lines through boxes)
    // dominate, but `w_len` is high enough that — among the many layouts that
    // tie at zero crossings — the search prefers the COMPACT one that pulls
    // connected externals close (otherwise it strands e.g. Keycloak far away in
    // dead space). `w_span` penalizes the overall bounding box so the diagram
    // doesn't sprawl. Both stay well below `w_cross` so the search never trades
    // a crossing for shorter lines.
    let w_cross = 100.0f32;
    let w_thru = 80.0f32;
    let w_len = 0.3f32;
    let w_span = 0.05f32;
    // Per-pixel penalty for an external->container edge running through the
    // boundary rectangle. Tuned so cutting ~250px across the boundary costs
    // about as much as a crossing, steering externals to the right side.
    let w_boundary = 0.4f32;
    let eval_cost = |assign: &[usize], cells: &[Pt]| -> f32 {
        // Resolve each unit's translation.
        let mut trans = vec![(0.0f32, 0.0f32); units.len()];
        for (mi, &u) in movable_units.iter().enumerate() {
            if assign[mi] != usize::MAX {
                trans[u] = unit_translation(u, assign[mi], cells);
            }
        }
        // Build edge segments (border-to-border straight lines).
        let mut segs: Vec<(Pt, Pt)> = Vec::with_capacity(gedges.len());
        for e in &gedges {
            let ba = shape_box(e.fa, trans[e.ua]);
            let bb = shape_box(e.tb, trans[e.ub]);
            let p1 = box_border_point(ba, bb.center());
            let p2 = box_border_point(bb, ba.center());
            segs.push((p1, p2));
        }
        let mut cost = 0.0f32;
        // length
        for (a, b) in &segs {
            cost += w_len * ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
        }
        // compactness: half-perimeter of the bounding box over all shapes
        // (anchored + movable in their candidate positions).
        {
            let mut min_x = f32::MAX;
            let mut min_y = f32::MAX;
            let mut max_x = f32::MIN;
            let mut max_y = f32::MIN;
            for idx in 0..shapes_out.len() {
                let b = shape_box(idx, trans[unit_of_shape[idx]]);
                min_x = min_x.min(b.x);
                min_y = min_y.min(b.y);
                max_x = max_x.max(b.x + b.w);
                max_y = max_y.max(b.y + b.h);
            }
            cost += w_span * ((max_x - min_x) + (max_y - min_y));
        }
        // edge-edge crossings
        for i in 0..segs.len() {
            for j in (i + 1)..segs.len() {
                let share_endpoint = gedges[i].fa == gedges[j].fa
                    || gedges[i].fa == gedges[j].tb
                    || gedges[i].tb == gedges[j].fa
                    || gedges[i].tb == gedges[j].tb;
                if share_endpoint {
                    // Two edges meeting at a common shape can't "cross" as
                    // center-lines (they share a point), but they still tangle
                    // if they leave/enter that shape on the SAME side. Penalize
                    // that so placement spreads a hub's neighbours across
                    // different sides instead of bunching them onto one edge —
                    // e.g. moving a db from a crowded top side to a free right
                    // side. side is derived from the border point on the common
                    // shape (an endpoint shared by both segments).
                    let common = if gedges[i].fa == gedges[j].fa || gedges[i].fa == gedges[j].tb {
                        gedges[i].fa
                    } else {
                        gedges[i].tb
                    };
                    let cbox = shape_box(common, trans[unit_of_shape[common]]);
                    let pi = if gedges[i].fa == common { segs[i].0 } else { segs[i].1 };
                    let pj = if gedges[j].fa == common { segs[j].0 } else { segs[j].1 };
                    if box_side_of_point(cbox, pi) == box_side_of_point(cbox, pj) {
                        cost += w_cross * 0.5;
                    }
                } else if segments_cross(segs[i].0, segs[i].1, segs[j].0, segs[j].1) {
                    cost += w_cross;
                }
            }
        }
        // edge-through-box crossings
        for (i, (a, b)) in segs.iter().enumerate() {
            for idx in 0..shapes_out.len() {
                // skip the edge's own endpoint shapes
                if idx == gedges[i].fa || idx == gedges[i].tb {
                    continue;
                }
                let u = unit_of_shape[idx];
                let bx = shape_box(idx, trans[u]);
                if segment_hits_box(*a, *b, bx) {
                    cost += w_thru;
                }
            }
        }
        // Boundary-crossing penalty: an external connected to a container deep
        // inside the boundary should approach from the side facing that
        // container, not cut across the whole boundary rectangle. Penalize the
        // length of each edge that lies INSIDE the anchor (boundary) box but
        // isn't its endpoint's own approach — this pushes externals to the
        // boundary side nearest what they connect to. (The boundary itself
        // isn't in `shapes_out`, so without this the search can't see it.)
        for (i, (a, b)) in segs.iter().enumerate() {
            // only for edges with exactly one endpoint inside the boundary
            // (external -> container): if both/none are inside, skip.
            let a_in = a.x > anchor.x && a.x < anchor.x + anchor.w && a.y > anchor.y && a.y < anchor.y + anchor.h;
            let b_in = b.x > anchor.x && b.x < anchor.x + anchor.w && b.y > anchor.y && b.y < anchor.y + anchor.h;
            if a_in == b_in {
                continue;
            }
            let _ = i;
            // length of the segment portion inside the boundary box.
            if let Some(inside_len) = segment_box_overlap_len(*a, *b, anchor) {
                cost += w_boundary * inside_len;
            }
        }
        cost
    };

    // Greedy initial assignment: each movable unit to the free cell nearest its
    // primary connection target, processed by descending degree.
    let mut degree = vec![0usize; units.len()];
    for e in &gedges {
        degree[e.ua] += 1;
        degree[e.ub] += 1;
    }
    let mut order: Vec<usize> = (0..n_mov).collect();
    order.sort_by_key(|&mi| std::cmp::Reverse(degree[movable_units[mi]]));

    let mut assign = vec![usize::MAX; n_mov];
    let mut cell_used = vec![false; cells.len()];
    for &mi in &order {
        let u = movable_units[mi];
        // target = centroid of this unit's neighbours' current anchored centers
        let mut tx = 0.0;
        let mut ty = 0.0;
        let mut cnt = 0.0;
        for e in &gedges {
            let other = if e.ua == u {
                Some(e.tb)
            } else if e.ub == u {
                Some(e.fa)
            } else {
                None
            };
            if let Some(o) = other {
                let ob = units[unit_of_shape[o]].bbox;
                tx += ob.x + ob.w / 2.0;
                ty += ob.y + ob.h / 2.0;
                cnt += 1.0;
            }
        }
        let target = if cnt > 0.0 {
            Pt {
                x: tx / cnt,
                y: ty / cnt,
            }
        } else {
            anchor.center()
        };
        // nearest free cell
        let mut best = usize::MAX;
        let mut best_d = f32::MAX;
        for (ci, c) in cells.iter().enumerate() {
            if cell_used[ci] {
                continue;
            }
            let d = (c.x - target.x).powi(2) + (c.y - target.y).powi(2);
            if d < best_d {
                best_d = d;
                best = ci;
            }
        }
        if best != usize::MAX {
            assign[mi] = best;
            cell_used[best] = true;
        }
    }

    // Hill-climb: repeatedly try moving a unit to a free cell or swapping two
    // units' cells; keep any change that lowers cost. Deterministic sweep with
    // a fixed iteration cap.
    let mut cur_cost = eval_cost(&assign, &cells);
    let max_rounds = 40usize;
    for _ in 0..max_rounds {
        let mut improved = false;
        // moves
        for mi in 0..n_mov {
            let old = assign[mi];
            for ci in 0..cells.len() {
                if cell_used[ci] {
                    continue;
                }
                assign[mi] = ci;
                let c = eval_cost(&assign, &cells);
                if c + 1e-3 < cur_cost {
                    cell_used[old] = false;
                    cell_used[ci] = true;
                    cur_cost = c;
                    improved = true;
                    break;
                } else {
                    assign[mi] = old;
                }
            }
        }
        // swaps
        for a in 0..n_mov {
            for b in (a + 1)..n_mov {
                assign.swap(a, b);
                let c = eval_cost(&assign, &cells);
                if c + 1e-3 < cur_cost {
                    cur_cost = c;
                    improved = true;
                } else {
                    assign.swap(a, b);
                }
            }
        }
        if !improved {
            break;
        }
    }

    // Apply the assignment: translate each movable unit's shapes to its cell.
    for (mi, &u) in movable_units.iter().enumerate() {
        if assign[mi] == usize::MAX {
            continue;
        }
        let (tx, ty) = unit_translation(u, assign[mi], &cells);
        for &m in &units[u].members {
            shapes_out[m].x += tx;
            shapes_out[m].y += ty;
        }
    }

    // Renormalize to the configured top-left margin and recompute bounds/rects.
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    for s in shapes_out.iter() {
        min_x = min_x.min(s.x);
        min_y = min_y.min(s.y);
    }
    let nx = conf.diagram_margin_x - min_x;
    let ny = conf.diagram_margin_y - min_y;
    for s in shapes_out.iter_mut() {
        s.x += nx;
        s.y += ny;
    }
    recompute_c4_boundary_rects(shapes_out, boundaries_out, boundary_map, shape_parent, conf);

    let mut max_x = conf.diagram_margin_x;
    let mut max_y = conf.diagram_margin_y;
    for s in shapes_out.iter() {
        max_x = max_x.max(s.x + s.width);
        max_y = max_y.max(s.y + s.height);
    }
    for b in boundaries_out.iter() {
        max_x = max_x.max(b.x + b.width);
        max_y = max_y.max(b.y + b.height);
    }
    *global_max_x = max_x;
    *global_max_y = max_y;
}

/// Simulated annealing over the placement of free external shapes, scoring each
/// candidate by its fully-routed quality (`route_and_score_c4`). Because the
/// score reflects the actual routed render, placement becomes routing-aware:
/// it moves blockers out of edge paths and pulls connected externals close,
/// minimizing the final crossings/box-hits/length rather than a proxy.
///
/// Moves are restricted to swapping two free externals' positions and small
/// axis jitters, so boundaries and their contents never deform. Determinism is
/// preserved with a seeded LCG (no wall-clock/RNG).
#[allow(clippy::too_many_arguments)]
fn anneal_c4_placement(
    shapes_out: &mut [C4ShapeLayout],
    boundaries_out: &mut [C4BoundaryLayout],
    base_rels: &[C4RelLayout],
    boundary_map: &std::collections::HashMap<String, &crate::ir::C4Boundary>,
    shape_parent: &std::collections::HashMap<&str, &str>,
    conf: &crate::config::C4Config,
    global_max_x: &mut f32,
    global_max_y: &mut f32,
) {
    // Every shape is movable, but a shape may only swap positions with another
    // in the SAME group: free externals (group "") swap among themselves, and
    // each boundary's containers swap among themselves (so containers stay
    // inside their boundary and the rect recomputes around them). This lets the
    // search fix an internal blocker — e.g. a container sitting between two
    // others that an edge must cross — by reordering the row.
    let group_of: Vec<String> = (0..shapes_out.len())
        .map(|i| {
            let parent = shape_parent.get(shapes_out[i].id.as_str()).copied().unwrap_or("");
            top_level_boundary_of(parent, boundary_map)
        })
        .collect();
    let movable: Vec<usize> = (0..shapes_out.len()).collect();
    if movable.len() < 2 || base_rels.is_empty() {
        return;
    }
    // Per group, the list of movable indices that may swap with each other.
    // Build with a BTreeMap (sorted, deterministic) so the search is
    // reproducible run-to-run — HashMap iteration order is randomized per
    // process and would make the annealing non-deterministic.
    let mut groups: std::collections::BTreeMap<String, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (k, &i) in movable.iter().enumerate() {
        groups.entry(group_of[i].clone()).or_default().push(k);
    }
    let swap_groups: Vec<Vec<usize>> =
        groups.into_values().filter(|g| g.len() >= 2).collect();
    if swap_groups.is_empty() {
        return;
    }

    let margin_x = conf.diagram_margin_x;
    let margin_y = conf.diagram_margin_y;
    // Score the current placement (positions read straight from shapes_out).
    let score_now = |shapes: &[C4ShapeLayout]| -> f32 {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for s in shapes {
            min_x = min_x.min(s.x);
            min_y = min_y.min(s.y);
            max_x = max_x.max(s.x + s.width);
            max_y = max_y.max(s.y + s.height);
        }
        let cw = (max_x - min_x + 2.0 * margin_x).max(1.0);
        let ch = (max_y - min_y + 2.0 * margin_y).max(1.0);
        route_and_score_c4(shapes, base_rels, conf, cw, ch).1
    };

    // Snapshot of just the movable shapes' positions (centers), so a move can
    // place an external at another external's slot.
    let pos_of = |shapes: &[C4ShapeLayout], i: usize| (shapes[i].x, shapes[i].y);

    let initial: Vec<(f32, f32)> = movable.iter().map(|&i| pos_of(shapes_out, i)).collect();
    let initial_score = score_now(shapes_out);
    let mut best = initial.clone();
    let mut best_score = initial_score;

    // Seeded LCG for reproducible "randomness".
    let mut rng: u64 = 0x9E3779B97F4A7C15 ^ (movable.len() as u64);
    let mut next = move || {
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((rng >> 33) as u32) as f32 / (u32::MAX as f32)
    };

    let apply = |shapes: &mut [C4ShapeLayout], positions: &[(f32, f32)]| {
        for (k, &i) in movable.iter().enumerate() {
            shapes[i].x = positions[k].0;
            shapes[i].y = positions[k].1;
        }
    };

    // Several annealing restarts (each from the initial placement, full
    // cool-down) keep the search from getting stuck in one local optimum; the
    // global best across restarts is committed. Iterations are split across
    // restarts.
    let total = conf.optimize_iterations.max(1);
    let restarts = 6usize.min(total);
    let iters = (total / restarts).max(1);
    let t0 = 1500.0f32;
    for _restart in 0..restarts {
        let mut cur = initial.clone();
        let mut cur_score = initial_score;
        apply(shapes_out, &cur);
        for it in 0..iters {
            let t = t0 * (1.0 - it as f32 / iters as f32);
            let mut cand = cur.clone();
            // Move: 80% swap two shapes within one group (reorders a boundary
            // row, or permutes externals); 20% jitter a free external. Swaps
            // keep every shape inside its own group.
            let g =
                &swap_groups[(next() * swap_groups.len() as f32) as usize % swap_groups.len()];
            if next() < 0.8 || g.len() < 2 {
                let a = g[(next() * g.len() as f32) as usize % g.len()];
                let mut bi = (next() * g.len() as f32) as usize % g.len();
                if g[bi] == a {
                    bi = (bi + 1) % g.len();
                }
                cand.swap(a, g[bi]);
            } else {
                let a = g[(next() * g.len() as f32) as usize % g.len()];
                if group_of[movable[a]].is_empty() {
                    let step = 40.0;
                    cand[a].0 += (next() - 0.5) * 2.0 * step;
                    cand[a].1 += (next() - 0.5) * 2.0 * step;
                } else {
                    let a2 = g[(next() * g.len() as f32) as usize % g.len()];
                    if a2 != a {
                        cand.swap(a, a2);
                    }
                }
            }
            apply(shapes_out, &cand);
            let s = score_now(shapes_out);
            let accept = s < cur_score || (t > 1e-3 && next() < (-(s - cur_score) / t).exp());
            if accept {
                cur = cand;
                cur_score = s;
                if s < best_score {
                    best_score = s;
                    best = cur.clone();
                }
            } else {
                apply(shapes_out, &cur); // revert
            }
        }
    }

    // Commit the best placement found.
    apply(shapes_out, &best);

    // Renormalize to the margin and recompute boundary rects + bounds.
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    for s in shapes_out.iter() {
        min_x = min_x.min(s.x);
        min_y = min_y.min(s.y);
    }
    let nx = margin_x - min_x;
    let ny = margin_y - min_y;
    for s in shapes_out.iter_mut() {
        s.x += nx;
        s.y += ny;
    }
    recompute_c4_boundary_rects(shapes_out, boundaries_out, boundary_map, shape_parent, conf);
    let mut max_x = margin_x;
    let mut max_y = margin_y;
    for s in shapes_out.iter() {
        max_x = max_x.max(s.x + s.width);
        max_y = max_y.max(s.y + s.height);
    }
    for b in boundaries_out.iter() {
        max_x = max_x.max(b.x + b.width);
        max_y = max_y.max(b.y + b.height);
    }
    *global_max_x = max_x;
    *global_max_y = max_y;
}

// ---------------------------------------------------------------------------
// Layout quality scoring
// ---------------------------------------------------------------------------

/// The component metrics of a C4 layout's visual quality, plus a single
/// weighted `score` (lower is better). This is the inner-loop cost both the
/// placement search and any future routing-mode auto-selection optimize.
#[derive(Debug, Clone, Copy, Default)]
pub struct C4Quality {
    /// Edge segments that properly cross another edge's segment (includes
    /// edges sharing an endpoint shape — those still read as a tangle).
    pub crossings: usize,
    /// Edge segments passing through a non-endpoint shape box.
    pub box_hits: usize,
    /// Pairs of collinear edge segments overlapping in the same channel.
    pub overlaps: usize,
    /// Total drawn edge length.
    pub length: f32,
    /// Total number of bends (interior vertices) across all edges.
    pub bends: usize,
    /// Canvas area (width × height).
    pub area: f32,
    /// Weighted total — lower is better.
    pub score: f32,
}

/// Score a C4 layout's relationship geometry against its shapes. Weights are
/// chosen so structural defects (crossings, lines through boxes, overlaps)
/// dominate, with length/bends/area as gentle tie-breakers favouring compact,
/// simple diagrams.
pub fn c4_layout_quality(shapes: &[C4ShapeLayout], rels: &[C4RelLayout], width: f32, height: f32) -> C4Quality {
    let boxes: Vec<(usize, Box)> = shapes
        .iter()
        .enumerate()
        .map(|(i, s)| (i, Box { x: s.x, y: s.y, w: s.width, h: s.height }))
        .collect();
    let id_to_idx: std::collections::HashMap<&str, usize> =
        shapes.iter().enumerate().map(|(i, s)| (s.id.as_str(), i)).collect();

    // Flatten edges into (rel_index, from_idx, to_idx, segment) records.
    struct Seg {
        ri: usize,
        a: Pt,
        b: Pt,
    }
    let mut segs: Vec<Seg> = Vec::new();
    let mut length = 0.0f32;
    let mut bends = 0usize;
    for (ri, rel) in rels.iter().enumerate() {
        if rel.points.len() < 2 {
            continue;
        }
        if rel.curved && rel.points.len() == 2 {
            // Curved arcs store only the chord endpoints; reconstruct the same
            // quadratic bow the renderer draws and sample it into segments, so
            // crossing/box checks reflect the actual drawn curve (which bows
            // away from — and often clears — boxes the chord would cross).
            let start = Pt { x: rel.points[0].0, y: rel.points[0].1 };
            let end = Pt { x: rel.points[1].0, y: rel.points[1].1 };
            for w in sample_c4_arc(start, end, rel.bow).windows(2) {
                length += ((w[1].x - w[0].x).powi(2) + (w[1].y - w[0].y).powi(2)).sqrt();
                segs.push(Seg { ri, a: w[0], b: w[1] });
            }
            continue;
        }
        bends += rel.points.len().saturating_sub(2);
        for w in rel.points.windows(2) {
            let a = Pt { x: w[0].0, y: w[0].1 };
            let b = Pt { x: w[1].0, y: w[1].1 };
            length += ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
            segs.push(Seg { ri, a, b });
        }
    }

    let endpoints = |ri: usize| -> (Option<usize>, Option<usize>) {
        (
            id_to_idx.get(rels[ri].from.as_str()).copied(),
            id_to_idx.get(rels[ri].to.as_str()).copied(),
        )
    };

    // Count CROSSINGS as edge-pairs that intersect anywhere (proper crossing,
    // T-junction, or overlap) — this matches what's visually a crossing. Two
    // edges sharing a connection node legitimately meet at their ports, so an
    // intersection that happens only at (or right next to) a shared port is not
    // counted; a touch elsewhere (e.g. one edge's elbow landing on another's
    // line) is.
    let port_pt = |ri: usize, start: bool| -> Pt {
        let p = &rels[ri].points;
        if start {
            Pt { x: p[0].0, y: p[0].1 }
        } else {
            Pt {
                x: p[p.len() - 1].0,
                y: p[p.len() - 1].1,
            }
        }
    };
    let near = |a: Pt, b: Pt| (a.x - b.x).abs() < 2.0 && (a.y - b.y).abs() < 2.0;
    let mut crossings = 0usize;
    for ri in 0..rels.len() {
        if rels[ri].points.len() < 2 {
            continue;
        }
        for rj in (ri + 1)..rels.len() {
            if rels[rj].points.len() < 2 {
                continue;
            }
            // does any segment of ri touch any segment of rj?
            let mut touch = false;
            'outer: for wi in rels[ri].points.windows(2) {
                let a = Pt { x: wi[0].0, y: wi[0].1 };
                let b = Pt { x: wi[1].0, y: wi[1].1 };
                for wj in rels[rj].points.windows(2) {
                    let c = Pt { x: wj[0].0, y: wj[0].1 };
                    let d = Pt { x: wj[1].0, y: wj[1].1 };
                    if segments_touch(a, b, c, d) {
                        touch = true;
                        break 'outer;
                    }
                }
            }
            if !touch {
                continue;
            }
            // If the two edges share a connection port (same start/end point),
            // the touch is legitimate — only count it if they ALSO cross
            // properly somewhere (a real tangle, not just meeting at the port).
            let ports_i = [port_pt(ri, true), port_pt(ri, false)];
            let ports_j = [port_pt(rj, true), port_pt(rj, false)];
            let shares_port = ports_i
                .iter()
                .any(|pi| ports_j.iter().any(|pj| near(*pi, *pj)));
            if shares_port {
                let mut proper = false;
                'o2: for wi in rels[ri].points.windows(2) {
                    let a = Pt { x: wi[0].0, y: wi[0].1 };
                    let b = Pt { x: wi[1].0, y: wi[1].1 };
                    for wj in rels[rj].points.windows(2) {
                        let c = Pt { x: wj[0].0, y: wj[0].1 };
                        let d = Pt { x: wj[1].0, y: wj[1].1 };
                        if segments_cross(a, b, c, d) {
                            proper = true;
                            break 'o2;
                        }
                    }
                }
                if proper {
                    crossings += 1;
                }
            } else {
                crossings += 1;
            }
        }
    }

    let mut box_hits = 0usize;
    for s in &segs {
        let (fi, ti) = endpoints(s.ri);
        for &(bi, bx) in &boxes {
            if Some(bi) == fi || Some(bi) == ti {
                continue;
            }
            if segment_hits_box(s.a, s.b, bx) {
                box_hits += 1;
            }
        }
    }

    let mut overlaps = 0usize;
    let tol = 3.0f32;
    for i in 0..segs.len() {
        for j in (i + 1)..segs.len() {
            if segs[i].ri == segs[j].ri {
                continue;
            }
            let (a, b, c, d) = (segs[i].a, segs[i].b, segs[j].a, segs[j].b);
            let h1 = (a.y - b.y).abs() < 0.5;
            let h2 = (c.y - d.y).abs() < 0.5;
            let v1 = (a.x - b.x).abs() < 0.5;
            let v2 = (c.x - d.x).abs() < 0.5;
            if h1 && h2 && (a.y - c.y).abs() < tol {
                let lo = a.x.min(b.x).max(c.x.min(d.x));
                let hi = a.x.max(b.x).min(c.x.max(d.x));
                if hi - lo > 2.0 {
                    overlaps += 1;
                }
            } else if v1 && v2 && (a.x - c.x).abs() < tol {
                let lo = a.y.min(b.y).max(c.y.min(d.y));
                let hi = a.y.max(b.y).min(c.y.max(d.y));
                if hi - lo > 2.0 {
                    overlaps += 1;
                }
            }
        }
    }

    let area = width * height;
    // Lexicographic fitness (each level dominates all lower levels combined):
    //   1. node crosses  — a line through/over a box (worst)
    //   2. line crosses  — edges intersecting/touching each other
    //   3. line elbows   — total bends
    //   4. line length
    // Weights are spaced so one higher-priority defect always outranks any
    // amount of lower-priority cost a real diagram could accumulate.
    let score = box_hits as f32 * 1_000_000.0
        + overlaps as f32 * 200_000.0
        + crossings as f32 * 10_000.0
        + bends as f32 * 10.0
        + length * 0.02
        + area * 0.00002;

    C4Quality {
        crossings,
        box_hits,
        overlaps,
        length,
        bends,
        area,
        score,
    }
}

/// The quadratic-bezier control point for a curved arc between two ports,
/// bowed perpendicular to the chord by `bow` (or a length-based default when
/// `bow <= 0`). Shared by the renderer and the quality scorer so they agree on
/// the drawn geometry. Note: the drawn curve only reaches `bow/2` from the
/// chord (a quadratic bezier passes through the control point's midpoint), so
/// callers sizing a bow to clear an obstacle must double the clearance.
pub fn c4_arc_control(start: (f32, f32), end: (f32, f32), bow: f32) -> (f32, f32) {
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let bow = if bow > 0.0 {
        bow
    } else {
        (len * 0.12).clamp(12.0, 60.0)
    };
    let mid_x = (start.0 + end.0) / 2.0;
    let mid_y = (start.1 + end.1) / 2.0;
    (mid_x - dy / len * bow, mid_y + dx / len * bow)
}

/// Sample a curved arc (quadratic bezier) into a short polyline for geometric
/// scoring.
fn sample_c4_arc(start: Pt, end: Pt, bow: f32) -> Vec<Pt> {
    let (cx, cy) = c4_arc_control((start.x, start.y), (end.x, end.y), bow);
    let steps = 8;
    (0..=steps)
        .map(|i| {
            let t = i as f32 / steps as f32;
            let mt = 1.0 - t;
            Pt {
                x: mt * mt * start.x + 2.0 * mt * t * cx + t * t * end.x,
                y: mt * mt * start.y + 2.0 * mt * t * cy + t * t * end.y,
            }
        })
        .collect()
}

/// Final port-optimization pass. With node placement fixed, search port
/// assignments to minimize the unified fitness — specifically targeting
/// same-hub crossings (two edges leaving one shape whose ports are ordered so
/// their lines tangle) that the placement search can't fix. Holds the routed
/// `rels` and tries swapping the start (or end) ports of every same-side
/// same-shape edge pair; a swap is kept only if the FULL-diagram fitness
/// (c4_layout_quality, which now counts T-junction touches) improves. Greedy,
/// deterministic, repeated until stable.
fn optimize_c4_ports(
    rels: &mut [C4RelLayout],
    shapes: &[C4ShapeLayout],
    conf: &crate::config::C4Config,
    cw: f32,
    ch: f32,
) {
    if rels.len() < 2 {
        return;
    }
    let id_to_idx: std::collections::HashMap<&str, usize> = shapes
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.as_str(), i))
        .collect();
    let boxes: Vec<(usize, Box)> = shapes
        .iter()
        .enumerate()
        .map(|(i, s)| (i, Box { x: s.x, y: s.y, w: s.width, h: s.height }))
        .collect();
    let clearance = (conf.c4_shape_margin * 0.5).max(8.0);

    // Which box side a point sits on (0 left,1 right,2 top,3 bottom).
    let side_of = |p: (f32, f32), b: Box| -> usize {
        box_side_of_point(b, Pt { x: p.0, y: p.1 })
    };

    let mut best_score = c4_layout_quality(shapes, rels, cw, ch).score;

    // Pass A: for each edge, try re-routing it out of every side of each
    // endpoint (all-sides search) and keep the side choice that lowers the full
    // fitness. This lets an edge stuck on a crowded side (where it tangles with
    // a sibling) escape to a freer side.
    for _ in 0..3 {
        let mut improved = false;
        for i in 0..rels.len() {
            let (Some(&fi), Some(&ti)) = (
                id_to_idx.get(rels[i].from.as_str()),
                id_to_idx.get(rels[i].to.as_str()),
            ) else {
                continue;
            };
            let others: Vec<Vec<Pt>> = (0..rels.len())
                .filter(|&k| k != i)
                .map(|k| rels[k].points.iter().map(|&(x, y)| Pt { x, y }).collect())
                .collect();
            let base = make_route_params(&rels[i], fi, ti, &boxes);
            // try every (start_side, end_side) combination, and a few start
            // lanes per side (the lane staggers the turn corridor, so a sibling
            // can nest its corridor inside/outside another instead of crossing).
            let mut best_local: Option<(f32, Vec<Pt>)> = None;
            for ss in 0..4 {
                for es in 0..4 {
                    for s_lane in 0..4usize {
                        let mut rp = base.clone();
                        rp.sp = side_midpoint(boxes[fi].1, ss);
                        rp.s_side = ss;
                        rp.s_lane = s_lane;
                        rp.ep = side_midpoint(boxes[ti].1, es);
                        rp.e_side = es;
                        rp.e_lane = 0;
                        let path = route_best_for(&rp, &others, &boxes, clearance);
                        if path.len() < 2 {
                            continue;
                        }
                        let saved = rels[i].clone();
                        apply_path(&mut rels[i], &path);
                        let s = c4_layout_quality(shapes, rels, cw, ch).score;
                        rels[i] = saved;
                        if best_local.as_ref().is_none_or(|(bs, _)| s < *bs) {
                            best_local = Some((s, path));
                        }
                    }
                }
            }
            if let Some((s, path)) = best_local
                && s + 0.5 < best_score
            {
                apply_path(&mut rels[i], &path);
                best_score = s;
                improved = true;
            }
        }
        if !improved {
            break;
        }
    }

    // Pass B: same-hub port swaps (kept for the cases where two siblings need
    // to exchange ports on the SAME side).
    for _ in 0..4 {
        let mut improved = false;
        for i in 0..rels.len() {
            for j in (i + 1)..rels.len() {
                // Need both endpoints + side membership. Consider swapping the
                // start ports (if both START at the same shape+side) and the
                // end ports (if both END at the same shape+side).
                let (Some(&fi_i), Some(&ti_i)) = (
                    id_to_idx.get(rels[i].from.as_str()),
                    id_to_idx.get(rels[i].to.as_str()),
                ) else {
                    continue;
                };
                let (Some(&fi_j), Some(&ti_j)) = (
                    id_to_idx.get(rels[j].from.as_str()),
                    id_to_idx.get(rels[j].to.as_str()),
                ) else {
                    continue;
                };
                for which_start in [true, false] {
                    // shape + side the two edges share at this end
                    let (shape_i, shape_j) = if which_start {
                        (fi_i, fi_j)
                    } else {
                        (ti_i, ti_j)
                    };
                    if shape_i != shape_j {
                        continue;
                    }
                    let pi = if which_start {
                        rels[i].start
                    } else {
                        rels[i].end
                    };
                    let pj = if which_start {
                        rels[j].start
                    } else {
                        rels[j].end
                    };
                    if side_of(pi, boxes[shape_i].1) != side_of(pj, boxes[shape_j].1) {
                        continue;
                    }
                    // Re-route edge i and j after swapping their shared-end
                    // ports, with all OTHER rels' current paths as obstacles.
                    let others: Vec<Vec<Pt>> = (0..rels.len())
                        .filter(|&k| k != i && k != j)
                        .map(|k| rels[k].points.iter().map(|&(x, y)| Pt { x, y }).collect())
                        .collect();
                    let mut ri = make_route_params(&rels[i], fi_i, ti_i, &boxes);
                    let mut rj = make_route_params(&rels[j], fi_j, ti_j, &boxes);
                    if which_start {
                        std::mem::swap(&mut ri.sp, &mut rj.sp);
                        std::mem::swap(&mut ri.s_side, &mut rj.s_side);
                    } else {
                        std::mem::swap(&mut ri.ep, &mut rj.ep);
                        std::mem::swap(&mut ri.e_side, &mut rj.e_side);
                    }
                    let pa = route_best_for(&ri, &others, &boxes, clearance);
                    let mut others2 = others.clone();
                    others2.push(pa.clone());
                    let pb = route_best_for(&rj, &others2, &boxes, clearance);
                    if pa.len() < 2 || pb.len() < 2 {
                        continue;
                    }
                    // Trial: apply, score, keep if better.
                    let saved_i = rels[i].clone();
                    let saved_j = rels[j].clone();
                    apply_path(&mut rels[i], &pa);
                    apply_path(&mut rels[j], &pb);
                    let s = c4_layout_quality(shapes, rels, cw, ch).score;
                    if s + 0.5 < best_score {
                        best_score = s;
                        improved = true;
                    } else {
                        rels[i] = saved_i;
                        rels[j] = saved_j;
                    }
                }
            }
        }
        if !improved {
            break;
        }
    }
}

/// Midpoint of a box side (0 left,1 right,2 top,3 bottom).
fn side_midpoint(b: Box, side: usize) -> Pt {
    match side {
        0 => Pt { x: b.x, y: b.y + b.h / 2.0 },
        1 => Pt { x: b.x + b.w, y: b.y + b.h / 2.0 },
        2 => Pt { x: b.x + b.w / 2.0, y: b.y },
        _ => Pt { x: b.x + b.w / 2.0, y: b.y + b.h },
    }
}

/// Build RouteParams from a routed rel (its current ports become the params).
fn make_route_params(
    rel: &C4RelLayout,
    fi: usize,
    ti: usize,
    boxes: &[(usize, Box)],
) -> RouteParams {
    let sp = Pt { x: rel.start.0, y: rel.start.1 };
    let ep = Pt { x: rel.end.0, y: rel.end.1 };
    RouteParams {
        sp,
        s_side: box_side_of_point(boxes[fi].1, sp),
        s_lane: 0,
        ep,
        e_side: box_side_of_point(boxes[ti].1, ep),
        e_lane: 0,
        fi,
        ti,
    }
}

/// Replace a rel's geometry with a routed polyline.
fn apply_path(rel: &mut C4RelLayout, path: &[Pt]) {
    rel.start = (path[0].x, path[0].y);
    rel.end = (path[path.len() - 1].x, path[path.len() - 1].y);
    rel.points = path.iter().map(|p| (p.x, p.y)).collect();
    rel.curved = false;
    rel.label_base = (
        (path[0].x + path[path.len() - 1].x) / 2.0,
        (path[0].y + path[path.len() - 1].y) / 2.0,
    );
}

/// Route `base_rels` (un-routed, 2-point center lines) over `shapes` using the
/// configured mode — for `"auto"`, try ortho/arc/straight and keep the lowest
/// quality score. Returns the routed rels and their score. Reused by the
/// annealing loop to score candidate placements.
fn route_and_score_c4(
    shapes: &[C4ShapeLayout],
    base_rels: &[C4RelLayout],
    conf: &crate::config::C4Config,
    cw: f32,
    ch: f32,
) -> (Vec<C4RelLayout>, f32) {
    let apply_mode = |mode: &str| -> Vec<C4RelLayout> {
        let mut r = base_rels.to_vec();
        match mode {
            "none" => {}
            "arc" => route_c4_rels_arc(&mut r, shapes, conf, true),
            "straight" => route_c4_rels_arc(&mut r, shapes, conf, false),
            _ => route_c4_rels(&mut r, shapes, conf),
        }
        r
    };
    if conf.rel_routing == "auto" {
        let tie = 0.5f32;
        let mut best: Option<(f32, Vec<C4RelLayout>)> = None;
        for mode in ["arc", "straight", "ortho"] {
            let rels = apply_mode(mode);
            let s = c4_layout_quality(shapes, &rels, cw, ch).score;
            if best.as_ref().is_none_or(|(bs, _)| s < *bs - tie) {
                best = Some((s, rels));
            }
        }
        best.map(|(s, r)| (r, s))
            .unwrap_or_else(|| (base_rels.to_vec(), f32::MAX))
    } else {
        let rels = apply_mode(conf.rel_routing.as_str());
        let s = c4_layout_quality(shapes, &rels, cw, ch).score;
        (rels, s)
    }
}

/// Compute the quality score for a fully-built C4 `Layout`, or `None` if it
/// isn't a C4 diagram. Convenience wrapper used by the CLI's `--timing` output.
pub fn c4_quality_for_layout(layout: &Layout) -> Option<C4Quality> {
    if let DiagramData::C4(c4) = &layout.diagram {
        Some(c4_layout_quality(&c4.shapes, &c4.rels, layout.width, layout.height))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Relationship routing
// ---------------------------------------------------------------------------

/// Route each relationship as an orthogonal polyline that avoids intervening
/// shapes, replacing the straight `start`→`end` line. Strategy per edge:
/// generate a handful of candidate paths (direct, two L-shapes, two Z-shapes
/// that detour around the midpoint) and keep the one that crosses the fewest
/// shape boxes, breaking ties by fewest bends then shortest length. If even the
/// best candidate is no better than the straight line, the straight line is
/// kept. Endpoints are re-projected onto the box borders along the chosen
/// outgoing direction so arrows meet shapes cleanly.
fn route_c4_rels(rels: &mut [C4RelLayout], shapes: &[C4ShapeLayout], conf: &crate::config::C4Config) {
    if rels.is_empty() {
        return;
    }
    let boxes: Vec<(usize, Box)> = shapes
        .iter()
        .enumerate()
        .map(|(i, s)| {
            (
                i,
                Box {
                    x: s.x,
                    y: s.y,
                    w: s.width,
                    h: s.height,
                },
            )
        })
        .collect();
    let id_to_idx: std::collections::HashMap<&str, usize> = shapes
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.as_str(), i))
        .collect();

    let clearance = (conf.c4_shape_margin * 0.5).max(8.0);

    // Count how many shape boxes (excluding the two endpoints) a polyline hits.
    let path_box_hits = |pts: &[Pt], from_idx: usize, to_idx: usize| -> usize {
        let mut hits = 0;
        for w in pts.windows(2) {
            for &(bi, b) in &boxes {
                if bi == from_idx || bi == to_idx {
                    continue;
                }
                if segment_hits_box(w[0], w[1], b) {
                    hits += 1;
                }
            }
        }
        hits
    };

    // Already-routed paths, so we can also discourage crossing earlier edges.
    let mut placed: Vec<Vec<Pt>> = Vec::with_capacity(rels.len());
    let path_edge_crossings = |pts: &[Pt], placed: &[Vec<Pt>]| -> usize {
        let mut n = 0;
        for w in pts.windows(2) {
            for prev in placed {
                for pw in prev.windows(2) {
                    if segments_cross(w[0], w[1], pw[0], pw[1]) {
                        n += 1;
                    }
                }
            }
        }
        n
    };

    for rel in rels.iter_mut() {
        let (Some(&fi), Some(&ti)) = (
            id_to_idx.get(rel.from.as_str()),
            id_to_idx.get(rel.to.as_str()),
        ) else {
            continue;
        };
        let fb = boxes[fi].1;
        let tb = boxes[ti].1;
        let fc = fb.center();
        let tc = tb.center();

        // Candidate orthogonal paths between the two box centers. The router
        // clips the first/last segment to the box borders afterwards.
        let mut candidates: Vec<Vec<Pt>> = Vec::new();
        // Direct straight (diagonal) — keep as a baseline candidate.
        candidates.push(vec![fc, tc]);
        // Two L-shapes (horizontal-first and vertical-first).
        candidates.push(vec![fc, Pt { x: tc.x, y: fc.y }, tc]);
        candidates.push(vec![fc, Pt { x: fc.x, y: tc.y }, tc]);
        // Two Z-shapes bending at the mid x / mid y (route through the gap
        // between the boxes).
        let mx = (fc.x + tc.x) / 2.0;
        let my = (fc.y + tc.y) / 2.0;
        candidates.push(vec![
            fc,
            Pt { x: mx, y: fc.y },
            Pt { x: mx, y: tc.y },
            tc,
        ]);
        candidates.push(vec![
            fc,
            Pt { x: fc.x, y: my },
            Pt { x: tc.x, y: my },
            tc,
        ]);
        // Detour candidates: go out past the far side of obstacles. Route
        // around the top/bottom or left/right extreme of the two boxes plus
        // clearance, which clears boxes stacked directly between endpoints.
        let top = fb.y.min(tb.y) - clearance;
        let bot = (fb.y + fb.h).max(tb.y + tb.h) + clearance;
        let left = fb.x.min(tb.x) - clearance;
        let right = (fb.x + fb.w).max(tb.x + tb.w) + clearance;
        candidates.push(vec![
            fc,
            Pt { x: fc.x, y: top },
            Pt { x: tc.x, y: top },
            tc,
        ]);
        candidates.push(vec![
            fc,
            Pt { x: fc.x, y: bot },
            Pt { x: tc.x, y: bot },
            tc,
        ]);
        candidates.push(vec![
            fc,
            Pt { x: left, y: fc.y },
            Pt { x: left, y: tc.y },
            tc,
        ]);
        candidates.push(vec![
            fc,
            Pt { x: right, y: fc.y },
            Pt { x: right, y: tc.y },
            tc,
        ]);

        // Score each candidate: (box hits, edge crossings, bends, length).
        let bends = |pts: &[Pt]| pts.len().saturating_sub(2);
        let length = |pts: &[Pt]| {
            pts.windows(2)
                .map(|w| ((w[1].x - w[0].x).powi(2) + (w[1].y - w[0].y).powi(2)).sqrt())
                .sum::<f32>()
        };

        let mut best: Option<(usize, usize, usize, f32, Vec<Pt>)> = None;
        for cand in candidates {
            let hits = path_box_hits(&cand, fi, ti);
            let crossings = path_edge_crossings(&cand, &placed);
            let b = bends(&cand);
            let l = length(&cand);
            let key = (hits, crossings, b, l);
            if best
                .as_ref()
                .is_none_or(|(h, c, bb, ll, _)| key < (*h, *c, *bb, *ll))
            {
                best = Some((hits, crossings, b, l, cand));
            }
        }

        let Some((_, _, _, _, chosen)) = best else {
            placed.push(vec![
                Pt {
                    x: rel.start.0,
                    y: rel.start.1,
                },
                Pt {
                    x: rel.end.0,
                    y: rel.end.1,
                },
            ]);
            continue;
        };

        // Clip the first and last segments to the box borders so the line
        // starts/ends exactly on the shape edges (not at the centers).
        let mut pts = chosen;
        let first_dir = pts[1];
        let start = box_border_point(fb, first_dir);
        pts[0] = start;
        let n = pts.len();
        let last_dir = pts[n - 2];
        let end = box_border_point(tb, last_dir);
        pts[n - 1] = end;

        rel.start = (start.x, start.y);
        rel.end = (end.x, end.y);
        rel.points = pts.iter().map(|p| (p.x, p.y)).collect();
        placed.push(pts);
    }

    // Phase B: distribute endpoints that share a component side. Multiple lines
    // hitting the same box currently land on the same border point; spread them
    // evenly along the side they attach to so each arrow is distinct, then
    // re-route each line orthogonally between its new ports (avoiding boxes).
    distribute_c4_ports(rels, &boxes, &id_to_idx, clearance);
}

/// Assign distinct, evenly-spaced ports per component side (the same side-
/// facing + fan-out logic as the orthogonal router) but connect them with a
/// direct 2-point line — drawn as a smooth curve when `curved`, otherwise
/// straight. Curves naturally fan sibling lines apart and avoid the elbow
/// detours that can tangle on simple diagrams, at the cost of not routing
/// around obstacles.
fn route_c4_rels_arc(
    rels: &mut [C4RelLayout],
    shapes: &[C4ShapeLayout],
    _conf: &crate::config::C4Config,
    curved: bool,
) {
    if rels.is_empty() {
        return;
    }
    let boxes: Vec<(usize, Box)> = shapes
        .iter()
        .enumerate()
        .map(|(i, s)| {
            (
                i,
                Box {
                    x: s.x,
                    y: s.y,
                    w: s.width,
                    h: s.height,
                },
            )
        })
        .collect();
    let id_to_idx: std::collections::HashMap<&str, usize> = shapes
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.as_str(), i))
        .collect();

    // Group edge endpoints by (shape, side), facing the other endpoint.
    // BTreeMap (sorted) keeps port assignment deterministic across runs.
    let mut groups: std::collections::BTreeMap<(usize, usize), Vec<(usize, bool, f32)>> =
        std::collections::BTreeMap::new();
    for (ri, rel) in rels.iter().enumerate() {
        let (Some(&fi), Some(&ti)) = (
            id_to_idx.get(rel.from.as_str()),
            id_to_idx.get(rel.to.as_str()),
        ) else {
            continue;
        };
        let fb = boxes[fi].1;
        let tb = boxes[ti].1;
        let s_side = side_facing(fb, tb.center(), 1);
        let e_side = side_facing(tb, fb.center(), 0);
        let s_key = if s_side <= 1 { tb.center().y } else { tb.center().x };
        let e_key = if e_side <= 1 { fb.center().y } else { fb.center().x };
        groups.entry((fi, s_side)).or_default().push((ri, true, s_key));
        groups.entry((ti, e_side)).or_default().push((ri, false, e_key));
    }

    // start/end port + side per edge.
    let mut sport: Vec<Option<(Pt, usize)>> = vec![None; rels.len()];
    let mut eport: Vec<Option<(Pt, usize)>> = vec![None; rels.len()];
    for ((shape_idx, side), mut members) in groups {
        let b = boxes[shape_idx].1;
        let n = members.len();
        members.sort_by(|a, c| a.2.partial_cmp(&c.2).unwrap_or(std::cmp::Ordering::Equal));
        let inset = (if side <= 1 { b.h } else { b.w } * 0.12).min(12.0);
        for (k, (ri, is_start, _)) in members.iter().enumerate() {
            let t = (k as f32 + 1.0) / (n as f32 + 1.0);
            let port = match side {
                0 => Pt { x: b.x, y: b.y + inset + (b.h - 2.0 * inset) * t },
                1 => Pt { x: b.x + b.w, y: b.y + inset + (b.h - 2.0 * inset) * t },
                2 => Pt { x: b.x + inset + (b.w - 2.0 * inset) * t, y: b.y },
                _ => Pt { x: b.x + inset + (b.w - 2.0 * inset) * t, y: b.y + b.h },
            };
            if *is_start {
                sport[*ri] = Some((port, side));
            } else {
                eport[*ri] = Some((port, side));
            }
        }
    }

    for (ri, rel) in rels.iter_mut().enumerate() {
        if let (Some((s, _)), Some((e, _))) = (sport[ri], eport[ri]) {
            rel.start = (s.x, s.y);
            rel.end = (e.x, e.y);
            rel.points = vec![(s.x, s.y), (e.x, e.y)];
            rel.curved = curved;
            rel.label_base = ((s.x + e.x) / 2.0, (s.y + e.y) / 2.0);
        }
    }
}

/// Which side of box `b` the point `p` lies on (it sits on the border after
/// clipping). Chooses the side whose edge `p` is closest to.
fn box_side_of_point(b: Box, p: Pt) -> usize {
    // 0=left 1=right 2=top 3=bottom
    let dl = (p.x - b.x).abs();
    let dr = (p.x - (b.x + b.w)).abs();
    let dt = (p.y - b.y).abs();
    let db = (p.y - (b.y + b.h)).abs();
    let m = dl.min(dr).min(dt).min(db);
    if m == dl {
        0
    } else if m == dr {
        1
    } else if m == dt {
        2
    } else {
        3
    }
}

/// The side of box `b` (0=left 1=right 2=top 3=bottom) that best faces
/// `target`. Picks the dominant axis of the direction from b's center to the
/// target; `fallback` is returned only when the target is essentially at the
/// center.
fn side_facing(b: Box, target: Pt, fallback: usize) -> usize {
    let c = b.center();
    let dx = target.x - c.x;
    let dy = target.y - c.y;
    if dx.abs() < 1.0 && dy.abs() < 1.0 {
        return fallback;
    }
    if dx.abs() >= dy.abs() {
        if dx < 0.0 { 0 } else { 1 }
    } else if dy < 0.0 {
        2
    } else {
        3
    }
}

/// Resolved routing endpoints for one relationship.
#[derive(Clone)]
struct RouteParams {
    sp: Pt,
    s_side: usize,
    s_lane: usize,
    ep: Pt,
    e_side: usize,
    e_lane: usize,
    fi: usize,
    ti: usize,
}

impl RouteParams {
    fn clone_rp(&self) -> RouteParams {
        self.clone()
    }
}

/// Route one edge trying all four sides for each endpoint, picking the
/// lowest-cost (box-avoiding, fewest-crossing) path against `others`. Free
/// function mirror of the `route_best` closure, for the uncross pass.
fn route_best_for(
    p: &RouteParams,
    others: &[Vec<Pt>],
    boxes: &[(usize, Box)],
    clearance: f32,
) -> Vec<Pt> {
    let side_midport = |bi: usize, side: usize| -> Pt {
        let b = boxes[bi].1;
        match side {
            0 => Pt { x: b.x, y: b.y + b.h / 2.0 },
            1 => Pt { x: b.x + b.w, y: b.y + b.h / 2.0 },
            2 => Pt { x: b.x + b.w / 2.0, y: b.y },
            _ => Pt { x: b.x + b.w / 2.0, y: b.y + b.h },
        }
    };
    let start_sides: Vec<(Pt, usize, usize)> = (0..4)
        .map(|s| {
            if s == p.s_side {
                (p.sp, p.s_side, p.s_lane)
            } else {
                (side_midport(p.fi, s), s, 0)
            }
        })
        .collect();
    let end_sides: Vec<(Pt, usize, usize)> = (0..4)
        .map(|s| {
            if s == p.e_side {
                (p.ep, p.e_side, p.e_lane)
            } else {
                (side_midport(p.ti, s), s, 0)
            }
        })
        .collect();
    let mut best: Option<(RouteCost, Vec<Pt>)> = None;
    for &(sp, ss, sl) in &start_sides {
        for &(ep, es, el) in &end_sides {
            let (mut cost, path) =
                route_between_ports(sp, ss, sl, ep, es, el, p.fi, p.ti, boxes, others, clearance);
            if ss != p.s_side || es != p.e_side {
                cost.6 += 1.0;
            }
            if best.as_ref().is_none_or(|(bk, _)| cost < *bk) {
                best = Some((cost, path));
            }
        }
    }
    best.map(|(_, path)| path).unwrap_or_default()
}

/// Evenly space the attachment points of every edge that shares a (shape, side)
/// so multiple arrows on one component are visually separated, then re-route
/// each edge orthogonally between its assigned ports. For each side with N
/// edges, the side is split into N+1 intervals and the N ports placed at the
/// interior boundaries, ordered by the opposite endpoint's position so lines
/// don't cross near the box.
fn distribute_c4_ports(
    rels: &mut [C4RelLayout],
    boxes: &[(usize, Box)],
    id_to_idx: &std::collections::HashMap<&str, usize>,
    clearance: f32,
) {
    // Per (shape_idx, side): list of (rel_idx, is_start, sort_key, tie_key).
    // BTreeMap (sorted) keeps port assignment deterministic across runs.
    let mut groups: std::collections::BTreeMap<(usize, usize), Vec<(usize, bool, f32, f32)>> =
        std::collections::BTreeMap::new();
    // Resolved per-edge ports: (port, side, lane). `lane` is the endpoint's
    // index among the edges sharing that box side, used to stagger turn
    // distances so adjacent lines don't share a corridor.
    let mut ports: Vec<Option<(Pt, usize, usize)>> = vec![None; rels.len()]; // start
    let mut end_ports: Vec<Option<(Pt, usize, usize)>> = vec![None; rels.len()];

    for (ri, rel) in rels.iter().enumerate() {
        if rel.points.len() < 2 {
            continue;
        }
        let (Some(&fi), Some(&ti)) = (
            id_to_idx.get(rel.from.as_str()),
            id_to_idx.get(rel.to.as_str()),
        ) else {
            continue;
        };
        let fb = boxes[fi].1;
        let tb = boxes[ti].1;
        let start = Pt {
            x: rel.points[0].0,
            y: rel.points[0].1,
        };
        let end = Pt {
            x: rel.points[rel.points.len() - 1].0,
            y: rel.points[rel.points.len() - 1].1,
        };
        // Attach each end to the box side that faces the other endpoint, so a
        // line never leaves the wrong side and loops around (which causes
        // sibling edges to cross). Ties keep the side the initial route chose.
        let s_side = side_facing(fb, tb.center(), box_side_of_point(fb, start));
        let e_side = side_facing(tb, fb.center(), box_side_of_point(tb, end));
        // Sort key orders ports along the side by the other endpoint's
        // coordinate on the relevant axis (left→right for top/bottom,
        // top→bottom for left/right) so adjacent targets get adjacent ports.
        // The secondary key is distance along the side normal (the travel
        // direction), so siblings whose targets sit at the same cross-axis
        // position (a row/column of targets) still get a stable nesting order:
        // the nearest target takes the inner port, the farthest the outer.
        let (s_key, s_key2) = if s_side <= 1 {
            (tb.center().y, (tb.center().x - fb.center().x).abs())
        } else {
            (tb.center().x, (tb.center().y - fb.center().y).abs())
        };
        let (e_key, e_key2) = if e_side <= 1 {
            (fb.center().y, (fb.center().x - tb.center().x).abs())
        } else {
            (fb.center().x, (fb.center().y - tb.center().y).abs())
        };
        groups
            .entry((fi, s_side))
            .or_default()
            .push((ri, true, s_key, s_key2));
        groups
            .entry((ti, e_side))
            .or_default()
            .push((ri, false, e_key, e_key2));
    }

    // Assign evenly-spaced ports per side.
    for ((shape_idx, side), mut members) in groups {
        let b = boxes[shape_idx].1;
        let n = members.len();
        if n == 0 {
            continue;
        }
        members.sort_by(|a, b| {
            a.2.partial_cmp(&b.2)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal))
        });
        let inset = (if side <= 1 { b.h } else { b.w } * 0.12).min(12.0);
        for (k, (ri, is_start, _, _)) in members.iter().enumerate() {
            let t = (k as f32 + 1.0) / (n as f32 + 1.0);
            let port = match side {
                0 => Pt {
                    x: b.x,
                    y: b.y + inset + (b.h - 2.0 * inset) * t,
                },
                1 => Pt {
                    x: b.x + b.w,
                    y: b.y + inset + (b.h - 2.0 * inset) * t,
                },
                2 => Pt {
                    x: b.x + inset + (b.w - 2.0 * inset) * t,
                    y: b.y,
                },
                _ => Pt {
                    x: b.x + inset + (b.w - 2.0 * inset) * t,
                    y: b.y + b.h,
                },
            };
            // Lane: distance of this port from the side's center, so the
            // outermost edges turn furthest out and the corridors fan apart
            // symmetrically instead of stacking.
            let lane = (k as i32 - (n as i32 - 1) / 2).unsigned_abs() as usize;
            if *is_start {
                ports[*ri] = Some((port, side, lane));
            } else {
                end_ports[*ri] = Some((port, side, lane));
            }
        }
    }

    // Per-edge routing parameters, gathered so we can route, then refine.
    let mut params: Vec<Option<RouteParams>> = Vec::with_capacity(rels.len());
    for ri in 0..rels.len() {
        let p = match (ports[ri], end_ports[ri]) {
            (Some((sp, s_side, s_lane)), Some((ep, e_side, e_lane))) => {
                match (
                    id_to_idx.get(rels[ri].from.as_str()),
                    id_to_idx.get(rels[ri].to.as_str()),
                ) {
                    (Some(&fi), Some(&ti)) => Some(RouteParams {
                        sp,
                        s_side,
                        s_lane,
                        ep,
                        e_side,
                        e_lane,
                        fi,
                        ti,
                    }),
                    _ => None,
                }
            }
            _ => None,
        };
        params.push(p);
    }

    // Port at the midpoint of a box side (used when trying an alternative
    // entry/exit side that wasn't the one the distribution assigned).
    let side_midport = |bi: usize, side: usize| -> Pt {
        let b = boxes[bi].1;
        match side {
            0 => Pt { x: b.x, y: b.y + b.h / 2.0 },
            1 => Pt { x: b.x + b.w, y: b.y + b.h / 2.0 },
            2 => Pt { x: b.x + b.w / 2.0, y: b.y },
            _ => Pt { x: b.x + b.w / 2.0, y: b.y + b.h },
        }
    };

    // Route an edge trying its assigned (start,end) sides plus alternatives
    // where either endpoint instead leaves the orthogonal side that also faces
    // the other box. Picks the lowest-cost route, so an edge blocked on its
    // natural side (e.g. spa→api with WASM in between) can enter via the top
    // and avoid the elbows/crossing. The assigned side keeps its even-spaced
    // lane port; alternative sides use the side midpoint.
    let route_best = |p: &RouteParams, others: &[Vec<Pt>]| -> Vec<Pt> {
        // Candidate sides for each endpoint: the distribution-assigned side
        // (keeps its even-spaced lane port) plus every other side at the side
        // midpoint. The crossing/bend-aware cost then picks the combination
        // that reads best — e.g. an edge can enter a box from the top instead
        // of a blocked left side to avoid an elbow or a sibling crossing.
        let start_sides: Vec<(Pt, usize, usize)> = (0..4)
            .map(|s| {
                if s == p.s_side {
                    (p.sp, p.s_side, p.s_lane)
                } else {
                    (side_midport(p.fi, s), s, 0)
                }
            })
            .collect();
        let end_sides: Vec<(Pt, usize, usize)> = (0..4)
            .map(|s| {
                if s == p.e_side {
                    (p.ep, p.e_side, p.e_lane)
                } else {
                    (side_midport(p.ti, s), s, 0)
                }
            })
            .collect();
        let mut best: Option<(RouteCost, Vec<Pt>)> = None;
        for &(sp, ss, sl) in &start_sides {
            for &(ep, es, el) in &end_sides {
                let (mut cost, path) = route_between_ports(
                    sp, ss, sl, ep, es, el, p.fi, p.ti, boxes, others, clearance,
                );
                // Mild bias toward the originally-assigned sides so we only
                // switch sides when it genuinely improves the route: bump the
                // length term when either side differs from the assignment.
                if ss != p.s_side || es != p.e_side {
                    cost.6 += 1.0;
                }
                if best.as_ref().is_none_or(|(bk, _)| cost < *bk) {
                    best = Some((cost, path));
                }
            }
        }
        best.map(|(_, path)| path).unwrap_or_default()
    };

    // Initial routing pass: each edge avoids boxes and the edges placed so far.
    let mut placed: Vec<Vec<Pt>> = vec![Vec::new(); rels.len()];
    for ri in 0..rels.len() {
        let Some(p) = &params[ri] else {
            if rels[ri].points.len() >= 2 {
                placed[ri] = rels[ri].points.iter().map(|&(x, y)| Pt { x, y }).collect();
            }
            continue;
        };
        let others: Vec<Vec<Pt>> = placed[..ri].to_vec();
        placed[ri] = route_best(p, &others);
    }

    // Refinement sweeps: re-route each edge against ALL the others' final
    // paths, so an edge forced to detour early (and now crossing a sibling) can
    // pick a better route once every neighbour is known. Repeat until stable.
    for _ in 0..4 {
        let mut changed = false;
        for ri in 0..rels.len() {
            let Some(p) = &params[ri] else { continue };
            let others: Vec<Vec<Pt>> = placed
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != ri)
                .map(|(_, v)| v.clone())
                .collect();
            let np = route_best(p, &others);
            if np != placed[ri] {
                placed[ri] = np;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // Same-hub uncross: two edges leaving (or entering) the same shape on the
    // same side cross when their ports are ordered opposite to their targets
    // — e.g. the edge to the higher target took the lower port. The refinement
    // above can't fix this (it moves one edge at a time; the fix needs both to
    // swap ports together). Detect such crossing pairs and swap their start (or
    // end) ports if it removes the crossing, then re-route both. This is the
    // "if those two exit ports were swapped they wouldn't cross" case.
    let paths_cross = |a: &[Pt], b: &[Pt]| -> bool {
        for wa in a.windows(2) {
            for wb in b.windows(2) {
                if segments_cross(wa[0], wa[1], wb[0], wb[1]) {
                    return true;
                }
            }
        }
        false
    };
    for _ in 0..3 {
        let mut swapped = false;
        for i in 0..rels.len() {
            for j in (i + 1)..rels.len() {
                if placed[i].len() < 2 || placed[j].len() < 2 {
                    continue;
                }
                if !paths_cross(&placed[i], &placed[j]) {
                    continue;
                }
                // candidate swaps where the two edges share a side+shape
                let (share_start, share_end) =
                    match (params[i].as_ref(), params[j].as_ref()) {
                        (Some(pi), Some(pj)) => (
                            pi.fi == pj.fi && pi.s_side == pj.s_side,
                            pi.ti == pj.ti && pi.e_side == pj.e_side,
                        ),
                        _ => continue,
                    };
                for which_start in [true, false] {
                    if which_start && !share_start {
                        continue;
                    }
                    if !which_start && !share_end {
                        continue;
                    }
                    let mut a = params[i].as_ref().unwrap().clone_rp();
                    let mut b = params[j].as_ref().unwrap().clone_rp();
                    if which_start {
                        std::mem::swap(&mut a.sp, &mut b.sp);
                        std::mem::swap(&mut a.s_lane, &mut b.s_lane);
                    } else {
                        std::mem::swap(&mut a.ep, &mut b.ep);
                        std::mem::swap(&mut a.e_lane, &mut b.e_lane);
                    }
                    let others: Vec<Vec<Pt>> = (0..rels.len())
                        .filter(|&k| k != i && k != j)
                        .map(|k| placed[k].clone())
                        .collect();
                    let pa = route_best_for(&a, &others, boxes, clearance);
                    let mut others_j = others;
                    others_j.push(pa.clone());
                    let pb = route_best_for(&b, &others_j, boxes, clearance);
                    if !paths_cross(&pa, &pb) {
                        params[i] = Some(a);
                        params[j] = Some(b);
                        placed[i] = pa;
                        placed[j] = pb;
                        swapped = true;
                        break;
                    }
                }
            }
        }
        if !swapped {
            break;
        }
    }

    for ri in 0..rels.len() {
        if placed[ri].len() >= 2 {
            rels[ri].start = (placed[ri][0].x, placed[ri][0].y);
            rels[ri].end = (placed[ri][placed[ri].len() - 1].x, placed[ri][placed[ri].len() - 1].y);
            rels[ri].points = placed[ri].iter().map(|p| (p.x, p.y)).collect();
        }
    }
}


/// Minimum straight run leaving a component before the line is allowed to
/// turn. The C4 arrowhead is ~10px, and a turn closer than ~3× that looks
/// squashed, so hold a 30px stub.
const C4_MIN_STUB: f32 = 30.0;

/// Extra stub length per lane, so edges leaving the same box side turn at
/// staggered distances and don't share a corridor.
const C4_LANE_STEP: f32 = 16.0;

/// Build an orthogonal path between two ports, each leaving its box via a
/// perpendicular stub whose length is staggered by the port's lane (so
/// neighbouring lines turn at different distances), choosing the axis-aligned
/// connector that avoids shape boxes and earlier paths with the fewest bends.
#[allow(clippy::too_many_arguments)]
/// Cost of a routed path, lexicographically ordered (lower is better):
/// diagonals, box hits, collinear overlaps, crossings, double-backs, bends,
/// length. Returned so callers can compare alternative side choices.
type RouteCost = (usize, usize, usize, usize, usize, usize, f32);

#[allow(clippy::too_many_arguments)]
fn route_between_ports(
    sp: Pt,
    s_side: usize,
    s_lane: usize,
    ep: Pt,
    e_side: usize,
    e_lane: usize,
    fi: usize,
    ti: usize,
    boxes: &[(usize, Box)],
    placed: &[Vec<Pt>],
    clearance: f32,
) -> (RouteCost, Vec<Pt>) {
    let out = |p: Pt, side: usize, lane: usize| {
        let stub = C4_MIN_STUB + lane as f32 * C4_LANE_STEP;
        match side {
            0 => Pt { x: p.x - stub, y: p.y },
            1 => Pt { x: p.x + stub, y: p.y },
            2 => Pt { x: p.x, y: p.y - stub },
            _ => Pt { x: p.x, y: p.y + stub },
        }
    };
    let s_stub = out(sp, s_side, s_lane);
    let e_stub = out(ep, e_side, e_lane);
    let s_horiz = s_side <= 1; // stub leaves horizontally
    let e_horiz = e_side <= 1;

    // Orthogonal connectors between the two stub points. Every candidate must
    // join `s_stub`→…→`e_stub` with only axis-aligned segments AND turn in a
    // direction consistent with how each stub left its box (so the elbow at the
    // stub stays a clean 90°, never a diagonal). The first turn from a
    // horizontal stub must be vertical and vice-versa.
    let a = s_stub;
    let b = e_stub;
    let mx = (a.x + b.x) / 2.0;
    let my = (a.y + b.y) / 2.0;
    let mut mids: Vec<Vec<Pt>> = Vec::new();
    // L-shapes: corner respects the leaving axis of the start stub.
    if s_horiz {
        // start goes horizontal first → corner at (b.x, a.y) keeps a-segment horizontal
        mids.push(vec![a, Pt { x: b.x, y: a.y }, b]);
    } else {
        mids.push(vec![a, Pt { x: a.x, y: b.y }, b]);
    }
    if e_horiz {
        mids.push(vec![a, Pt { x: a.x, y: b.y }, b]);
    } else {
        mids.push(vec![a, Pt { x: b.x, y: a.y }, b]);
    }
    // Z-shapes that turn at the STAGGERED stub coordinate (a.x / a.y), so
    // siblings leaving the same side run their long leg in distinct lanes
    // instead of collapsing onto the target's coordinate. These usually beat
    // the L-shapes once the overlap penalty applies.
    if s_horiz {
        // vertical corridor at the staggered a.x, then across at b.y
        mids.push(vec![a, Pt { x: a.x, y: b.y }, b]);
    } else {
        // horizontal corridor at the staggered a.y, then up/down at b.x
        mids.push(vec![a, Pt { x: b.x, y: a.y }, b]);
    }
    // Z-shapes (mid corridor) — both orientations.
    mids.push(vec![a, Pt { x: mx, y: a.y }, Pt { x: mx, y: b.y }, b]);
    mids.push(vec![a, Pt { x: a.x, y: my }, Pt { x: b.x, y: my }, b]);

    // Detour candidates: route the cross-leg along a corridor just outside any
    // box that lies between the two ports, so a blocker directly in the path
    // (e.g. a sibling container between source and target) is gone around
    // rather than driven through. We probe corridors a clearance beyond the
    // union of all non-endpoint boxes overlapping the bounding span.
    let span_lo_x = a.x.min(b.x);
    let span_hi_x = a.x.max(b.x);
    let span_lo_y = a.y.min(b.y);
    let span_hi_y = a.y.max(b.y);
    let mut obs_top = f32::MAX;
    let mut obs_bot = f32::MIN;
    let mut obs_left = f32::MAX;
    let mut obs_right = f32::MIN;
    for &(bi, bx) in boxes {
        if bi == fi || bi == ti {
            continue;
        }
        // box overlaps the span rectangle?
        if bx.x <= span_hi_x && bx.x + bx.w >= span_lo_x && bx.y <= span_hi_y && bx.y + bx.h >= span_lo_y
        {
            obs_top = obs_top.min(bx.y);
            obs_bot = obs_bot.max(bx.y + bx.h);
            obs_left = obs_left.min(bx.x);
            obs_right = obs_right.max(bx.x + bx.w);
        }
    }
    let pad = clearance.max(16.0);
    if obs_bot > obs_top {
        // horizontal-corridor detours above / below the obstacle band
        let above = obs_top - pad;
        let below = obs_bot + pad;
        mids.push(vec![a, Pt { x: a.x, y: above }, Pt { x: b.x, y: above }, b]);
        mids.push(vec![a, Pt { x: a.x, y: below }, Pt { x: b.x, y: below }, b]);
        // vertical-corridor detours left / right of the obstacle band
        let leftc = obs_left - pad;
        let rightc = obs_right + pad;
        mids.push(vec![a, Pt { x: leftc, y: a.y }, Pt { x: leftc, y: b.y }, b]);
        mids.push(vec![a, Pt { x: rightc, y: a.y }, Pt { x: rightc, y: b.y }, b]);
    }

    // Box hits use a clearance margin so a line that grazes a component's
    // outskirts (running right along its edge) counts as a hit and is avoided.
    let margin = (clearance * 0.4).max(6.0);
    let hits = |pts: &[Pt]| -> usize {
        let mut h = 0;
        for w in pts.windows(2) {
            for &(bi, bx) in boxes {
                if bi == fi || bi == ti {
                    continue;
                }
                let grown = Box {
                    x: bx.x - margin,
                    y: bx.y - margin,
                    w: bx.w + 2.0 * margin,
                    h: bx.h + 2.0 * margin,
                };
                if segment_hits_box(w[0], w[1], grown) {
                    h += 1;
                }
            }
        }
        h
    };
    // A path doubles back if two consecutive segments on the same axis reverse
    // direction (go out then come back), which reads as an ugly hook.
    let doublebacks = |pts: &[Pt]| -> usize {
        let mut n = 0;
        for w in pts.windows(3) {
            let d1x = w[1].x - w[0].x;
            let d1y = w[1].y - w[0].y;
            let d2x = w[2].x - w[1].x;
            let d2y = w[2].y - w[1].y;
            if (d1x * d2x < -0.01) || (d1y * d2y < -0.01) {
                n += 1;
            }
        }
        n
    };
    let crossings = |pts: &[Pt]| -> usize {
        let mut n = 0;
        for w in pts.windows(2) {
            for prev in placed {
                for pw in prev.windows(2) {
                    if segments_cross(w[0], w[1], pw[0], pw[1]) {
                        n += 1;
                    }
                }
            }
        }
        n
    };
    // Collinear overlap with an already-placed segment: two lines running along
    // the same channel and overlapping read as a single line. Penalize heavily.
    let overlaps = |pts: &[Pt]| -> usize {
        let tol = 3.0f32;
        let mut n = 0;
        for w in pts.windows(2) {
            let h = (w[0].y - w[1].y).abs() < 0.5;
            let v = (w[0].x - w[1].x).abs() < 0.5;
            for prev in placed {
                for pw in prev.windows(2) {
                    let ph = (pw[0].y - pw[1].y).abs() < 0.5;
                    let pv = (pw[0].x - pw[1].x).abs() < 0.5;
                    if h && ph && (w[0].y - pw[0].y).abs() < tol {
                        let lo = w[0].x.min(w[1].x).max(pw[0].x.min(pw[1].x));
                        let hi = w[0].x.max(w[1].x).min(pw[0].x.max(pw[1].x));
                        if hi - lo > 2.0 {
                            n += 1;
                        }
                    } else if v && pv && (w[0].x - pw[0].x).abs() < tol {
                        let lo = w[0].y.min(w[1].y).max(pw[0].y.min(pw[1].y));
                        let hi = w[0].y.max(w[1].y).min(pw[0].y.max(pw[1].y));
                        if hi - lo > 2.0 {
                            n += 1;
                        }
                    }
                }
            }
        }
        n
    };
    let length = |pts: &[Pt]| {
        pts.windows(2)
            .map(|w| ((w[1].x - w[0].x).powi(2) + (w[1].y - w[0].y).powi(2)).sqrt())
            .sum::<f32>()
    };
    // Count segments that are neither horizontal nor vertical — should always
    // be zero for our candidates, but score it so a stray diagonal never wins.
    let diagonals = |pts: &[Pt]| -> usize {
        pts.windows(2)
            .filter(|w| (w[0].x - w[1].x).abs() > 0.5 && (w[0].y - w[1].y).abs() > 0.5)
            .count()
    };
    // Scoring key, lexicographic (lower better): no diagonals, fewest box hits,
    // fewest collinear overlaps, fewest crossings, no double-backs, fewest
    // bends, shortest.
    let mut best: Option<(RouteCost, Vec<Pt>)> = None;
    for mid in mids {
        // full path = port -> stub -> ...mid... -> stub -> port, with
        // consecutive duplicate points removed (a stub may coincide with the
        // next corner when the ports already align).
        let mut full = Vec::with_capacity(mid.len() + 2);
        full.push(sp);
        full.extend(mid.iter().copied());
        full.push(ep);
        dedup_points(&mut full);
        let key: RouteCost = (
            diagonals(&full),
            hits(&full),
            overlaps(&full),
            crossings(&full),
            doublebacks(&full),
            full.len().saturating_sub(2),
            length(&full),
        );
        if best.as_ref().is_none_or(|(bk, _)| key < *bk) {
            best = Some((key, full));
        }
    }
    best.unwrap_or_else(|| {
        let fallback = vec![sp, s_stub, e_stub, ep];
        ((9, 9, 9, 9, 9, 9, f32::MAX), fallback)
    })
}

/// Remove consecutive points that are (near) identical.
fn dedup_points(pts: &mut Vec<Pt>) {
    pts.dedup_by(|a, b| (a.x - b.x).abs() < 0.5 && (a.y - b.y).abs() < 0.5);
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
            );
            y = layout.y + layout.height;
            rect_width = rect_width.max(layout.width);
            // Box must fit the full wrapped description plus bottom padding.
            // (The previous `y - lines*5` shrank tall boxes below their text,
            // causing long descriptions to overflow into the next block.)
            rect_height = y + conf.c4_shape_padding;
            descr_layout = Some(layout);
        }
        rect_width += conf.c4_shape_padding * 2.0;
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
        .map(|line| estimate_text_width(line, font_size, font_family, fast_metrics))
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
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current, word)
        };
        if estimate_text_width(&candidate, font_size, font_family, fast_metrics) <= max_width
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

fn estimate_text_width(text: &str, font_size: f32, font_family: &str, fast_metrics: bool) -> f32 {
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

/// Place each relationship's label to minimize collisions with the edge LINES
/// (its own and every other), the node/boundary boxes, and other labels. For
/// each label we generate candidates that slide ALONG its own routed line (so
/// it can find an open stretch) crossed with small perpendicular offsets, score
/// each against all obstacles, and keep the best. A few refinement sweeps then
/// re-place every label against the others' final positions to undo the
/// order-dependence of a single greedy pass.
fn resolve_c4_rel_label_offsets(
    rels: &mut [C4RelLayout],
    shapes: &[C4ShapeLayout],
    boundaries: &[C4BoundaryLayout],
    conf: &crate::config::C4Config,
) {
    if rels.is_empty() {
        return;
    }
    // Node obstacles, grown by a small clearance so a label touching an edge of
    // a box counts as overlapping and is pushed clear.
    let clr = 6.0f32;
    let mut shape_obstacles: Vec<C4Rect> = shapes
        .iter()
        .map(|shape| C4Rect {
            x: shape.x - clr,
            y: shape.y - clr,
            width: shape.width + 2.0 * clr,
            height: shape.height + 2.0 * clr,
        })
        .collect();
    for b in boundaries {
        let header = b.label.height
            + b.boundary_type.as_ref().map(|t| t.height).unwrap_or(0.0)
            + conf.message_font_size;
        shape_obstacles.push(C4Rect {
            x: b.x,
            y: b.y,
            width: b.width,
            height: header.min(b.height),
        });
    }

    // All edge line segments (every relationship's routed polyline). Tagged
    // with the owning rel index so a label can be told to ignore the central
    // stretch of its OWN line (it's expected to sit on its own line).
    struct Seg {
        ri: usize,
        a: (f32, f32),
        b: (f32, f32),
    }
    let segments: Vec<Seg> = rels
        .iter()
        .enumerate()
        .flat_map(|(ri, rel)| {
            rel.points
                .windows(2)
                .map(move |w| Seg { ri, a: w[0], b: w[1] })
                .collect::<Vec<_>>()
        })
        .collect();

    // Per-rel: anchor point + tangent of its routed line, and its total length,
    // so candidates can slide along it.
    let anchors: Vec<((f32, f32), f32, f32, f32)> = rels
        .iter()
        .map(|rel| {
            let (base, tx, ty) = c4_path_label_anchor(&rel.points, rel.start, rel.end);
            let len: f32 = rel
                .points
                .windows(2)
                .map(|w| ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt())
                .sum();
            (base, tx, ty, len)
        })
        .collect();

    let step = (conf.message_font_size * 1.2).max(10.0);

    // Score a candidate label rect for rel `ri`: heavy penalty for crossing any
    // line (other than the immediate centre of its own line), node overlap,
    // label-label overlap, plus a small pull toward its own line (displacement).
    let score = |ri: usize,
                 rect: &C4Rect,
                 displacement: f32,
                 placed: &[Option<C4Rect>],
                 segments: &[Seg]|
     -> f32 {
        let node_overlap: f32 = shape_obstacles
            .iter()
            .map(|o| c4_rect_overlap_area(*rect, *o))
            .sum();
        let label_overlap: f32 = placed
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != ri)
            .filter_map(|(_, r)| r.as_ref())
            .map(|r| c4_rect_overlap_area(*rect, *r))
            .sum();
        // Count line crossings: how many edge segments pass through the rect.
        // Skip the owning rel's own segments (a label is allowed to overlay its
        // own line) — collisions with OTHER lines are what we minimize.
        let mut line_hits = 0.0f32;
        for s in segments {
            if s.ri == ri {
                continue;
            }
            if segment_intersects_rect(s.a, s.b, *rect) {
                line_hits += 1.0;
            }
        }
        // Node overlap is the worst (a label buried in a box is unreadable),
        // then crossing other lines, then overlapping another label. The pull
        // toward its own line is gentle so a label will travel a long way to
        // reach open space if it must.
        node_overlap * 30.0 + line_hits * 120.0 + label_overlap * 9.0 + displacement * 0.01
    };

    // Generate candidate (offset_from_anchor) deltas for rel ri: slide along the
    // line (tangent) over its usable length, each crossed with perpendicular
    // offsets on both sides. The perpendicular reach is large enough to escape
    // past an adjacent component when the label's own line runs between two.
    let candidates_for = |ri: usize| -> Vec<(f32, f32)> {
        let (_, tx, ty, len) = anchors[ri];
        let (nx, ny) = (-ty, tx);
        let mut cands = vec![(0.0f32, 0.0f32)];
        let reach = (len * 0.4).min(160.0);
        let along_steps = 6;
        for s in -along_steps..=along_steps {
            let t = (s as f32 / along_steps as f32) * reach;
            for ring in 0..=12 {
                let dist = step * ring as f32;
                for sign in [-1.0f32, 1.0f32] {
                    cands.push((tx * t + nx * dist * sign, ty * t + ny * dist * sign));
                    if ring == 0 {
                        break; // only one zero-offset per along position
                    }
                }
            }
        }
        cands
    };

    let n = rels.len();
    let mut placed: Vec<Option<C4Rect>> = vec![None; n];

    // Set anchors first (render reads label_base).
    for (ri, rel) in rels.iter_mut().enumerate() {
        rel.label_base = anchors[ri].0;
    }

    // Initial greedy placement.
    for ri in 0..n {
        let cands = candidates_for(ri);
        let mut best_delta = (0.0f32, 0.0f32);
        let mut best_rect = c4_rel_label_rect(&rels[ri], conf, (0.0, 0.0));
        let mut best_score =
            score(ri, &best_rect, 0.0, &placed, &segments);
        for d in cands.into_iter().skip(1) {
            let rect = c4_rel_label_rect(&rels[ri], conf, d);
            let disp = (d.0 * d.0 + d.1 * d.1).sqrt();
            let s = score(ri, &rect, disp, &placed, &segments);
            if s < best_score {
                best_score = s;
                best_delta = d;
                best_rect = rect;
                if best_score < 1e-3 {
                    break;
                }
            }
        }
        rels[ri].offset_x = best_delta.0;
        rels[ri].offset_y = best_delta.1;
        placed[ri] = Some(best_rect);
    }

    // Refinement sweeps: re-place each label against everyone else's final
    // positions, so a label placed early (against few neighbours) can improve
    // once the full picture is known.
    for _ in 0..3 {
        let mut changed = false;
        for ri in 0..n {
            let cands = candidates_for(ri);
            let mut best_delta = (rels[ri].offset_x, rels[ri].offset_y);
            let cur_rect = c4_rel_label_rect(&rels[ri], conf, (0.0, 0.0));
            let cur_disp = (best_delta.0 * best_delta.0 + best_delta.1 * best_delta.1).sqrt();
            let mut best_score = score(ri, &cur_rect, cur_disp, &placed, &segments);
            let mut best_rect = cur_rect;
            for d in cands {
                // d is relative to the anchor; the rect uses offset = d here.
                let rect = c4_rel_label_rect_at(&rels[ri], conf, d);
                let disp = (d.0 * d.0 + d.1 * d.1).sqrt();
                let s = score(ri, &rect, disp, &placed, &segments);
                if s + 1e-3 < best_score {
                    best_score = s;
                    best_delta = d;
                    best_rect = rect;
                }
            }
            if (best_delta.0 - rels[ri].offset_x).abs() > 0.5
                || (best_delta.1 - rels[ri].offset_y).abs() > 0.5
            {
                rels[ri].offset_x = best_delta.0;
                rels[ri].offset_y = best_delta.1;
                placed[ri] = Some(best_rect);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

/// Label rect with the offset set to an absolute value (not added to the
/// existing offset) — for the refinement sweep which recomputes from scratch.
fn c4_rel_label_rect_at(
    rel: &C4RelLayout,
    conf: &crate::config::C4Config,
    offset: (f32, f32),
) -> C4Rect {
    let center_x = rel.label_base.0 + offset.0;
    let center_y = rel.label_base.1 + offset.1;
    c4_label_rect_at_center(rel, conf, center_x, center_y)
}

/// Midpoint of a routed polyline (by arc length) plus the unit tangent of the
/// segment it falls on. Falls back to the straight start→end line.
fn c4_path_label_anchor(
    points: &[(f32, f32)],
    start: (f32, f32),
    end: (f32, f32),
) -> ((f32, f32), f32, f32) {
    let pts: &[(f32, f32)] = if points.len() >= 2 {
        points
    } else {
        &[start, end]
    };
    let total: f32 = pts
        .windows(2)
        .map(|w| ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt())
        .sum();
    if total < 1e-3 {
        return (start, 1.0, 0.0);
    }
    let half = total / 2.0;
    let mut acc = 0.0;
    for w in pts.windows(2) {
        let seg = ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt();
        if acc + seg >= half {
            let t = if seg > 1e-3 { (half - acc) / seg } else { 0.0 };
            let px = w[0].0 + (w[1].0 - w[0].0) * t;
            let py = w[0].1 + (w[1].1 - w[0].1) * t;
            let (tx, ty) = if seg > 1e-3 {
                ((w[1].0 - w[0].0) / seg, (w[1].1 - w[0].1) / seg)
            } else {
                (1.0, 0.0)
            };
            return ((px, py), tx, ty);
        }
        acc += seg;
    }
    (end, 1.0, 0.0)
}

fn c4_rel_label_rect(
    rel: &C4RelLayout,
    conf: &crate::config::C4Config,
    delta: (f32, f32),
) -> C4Rect {
    let center_x = rel.label_base.0 + rel.offset_x + delta.0;
    let center_y = rel.label_base.1 + rel.offset_y + delta.1;
    c4_label_rect_at_center(rel, conf, center_x, center_y)
}

/// The bounding rect of a relationship's label+techn text centred at a point.
fn c4_label_rect_at_center(
    rel: &C4RelLayout,
    conf: &crate::config::C4Config,
    center_x: f32,
    center_y: f32,
) -> C4Rect {
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

/// True if segment a-b intersects (or lies within) rectangle `r`. Used to
/// detect a label box sitting on top of an edge line.
fn segment_intersects_rect(a: (f32, f32), b: (f32, f32), r: C4Rect) -> bool {
    let (rx, ry, rw, rh) = (r.x, r.y, r.width, r.height);
    // endpoint inside?
    let inside = |p: (f32, f32)| p.0 >= rx && p.0 <= rx + rw && p.1 >= ry && p.1 <= ry + rh;
    if inside(a) || inside(b) {
        return true;
    }
    // segment vs the four rect edges
    let pa = Pt { x: a.0, y: a.1 };
    let pb = Pt { x: b.0, y: b.1 };
    let corners = [
        Pt { x: rx, y: ry },
        Pt { x: rx + rw, y: ry },
        Pt { x: rx + rw, y: ry + rh },
        Pt { x: rx, y: ry + rh },
    ];
    for i in 0..4 {
        if segments_touch(pa, pb, corners[i], corners[(i + 1) % 4]) {
            return true;
        }
    }
    false
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
