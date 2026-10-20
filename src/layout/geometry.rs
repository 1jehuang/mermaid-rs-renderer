use crate::ir::NodeShape;

use super::types::NodeLayout;

const POINT_EPS: f32 = 0.5;
const GEOM_EPS: f32 = 1e-6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum EdgeSide {
    Left,
    Right,
    Top,
    Bottom,
}

pub(super) fn node_center(node: &NodeLayout) -> (f32, f32) {
    (node.x + node.width * 0.5, node.y + node.height * 0.5)
}

pub(super) fn endpoint_side_for_point(node: &NodeLayout, point: (f32, f32)) -> EdgeSide {
    let left = (point.0 - node.x).abs();
    let right = (point.0 - (node.x + node.width)).abs();
    let top = (point.1 - node.y).abs();
    let bottom = (point.1 - (node.y + node.height)).abs();
    let mut best = (left, EdgeSide::Left);
    for candidate in [
        (right, EdgeSide::Right),
        (top, EdgeSide::Top),
        (bottom, EdgeSide::Bottom),
    ] {
        if candidate.0 < best.0 {
            best = candidate;
        }
    }
    best.1
}

pub(super) fn side_points_outward(
    side: EdgeSide,
    endpoint: (f32, f32),
    outside: (f32, f32),
) -> bool {
    match side {
        EdgeSide::Left => outside.0 <= endpoint.0 + POINT_EPS,
        EdgeSide::Right => outside.0 >= endpoint.0 - POINT_EPS,
        EdgeSide::Top => outside.1 <= endpoint.1 + POINT_EPS,
        EdgeSide::Bottom => outside.1 >= endpoint.1 - POINT_EPS,
    }
}

pub(super) fn source_exits_outward(side: EdgeSide, start: (f32, f32), next: (f32, f32)) -> bool {
    side_points_outward(side, start, next)
}

pub(super) fn target_enters_from_outside(
    side: EdgeSide,
    prev: (f32, f32),
    end: (f32, f32),
) -> bool {
    side_points_outward(side, end, prev)
}

pub(super) fn segment_intrudes_endpoint_rect(
    side: EdgeSide,
    outside: (f32, f32),
    endpoint: (f32, f32),
    node: &NodeLayout,
) -> bool {
    let within_y =
        endpoint.1 >= node.y - POINT_EPS && endpoint.1 <= node.y + node.height + POINT_EPS;
    let within_x =
        endpoint.0 >= node.x - POINT_EPS && endpoint.0 <= node.x + node.width + POINT_EPS;
    match side {
        EdgeSide::Left => within_y && outside.0 > endpoint.0 + POINT_EPS,
        EdgeSide::Right => within_y && outside.0 < endpoint.0 - POINT_EPS,
        EdgeSide::Top => within_x && outside.1 > endpoint.1 + POINT_EPS,
        EdgeSide::Bottom => within_x && outside.1 < endpoint.1 - POINT_EPS,
    }
}

pub(super) fn shape_polygon_points(node: &NodeLayout) -> Option<Vec<(f32, f32)>> {
    let x = node.x;
    let y = node.y;
    let w = node.width;
    let h = node.height;
    match node.shape {
        NodeShape::Rectangle
        | NodeShape::ForkJoin
        | NodeShape::RoundRect
        | NodeShape::ActorBox
        | NodeShape::Stadium
        | NodeShape::Subroutine
        | NodeShape::Text
        | NodeShape::MindmapDefault => Some(vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h)]),
        NodeShape::Diamond => {
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            Some(vec![(cx, y), (x + w, cy), (cx, y + h), (x, cy)])
        }
        NodeShape::Hexagon => {
            let x1 = x + w * 0.25;
            let x2 = x + w * 0.75;
            let y_mid = y + h / 2.0;
            Some(vec![
                (x1, y),
                (x2, y),
                (x + w, y_mid),
                (x2, y + h),
                (x1, y + h),
                (x, y_mid),
            ])
        }
        NodeShape::Parallelogram | NodeShape::ParallelogramAlt => {
            let offset = w * 0.18;
            let points = if node.shape == NodeShape::Parallelogram {
                vec![
                    (x + offset, y),
                    (x + w, y),
                    (x + w - offset, y + h),
                    (x, y + h),
                ]
            } else {
                vec![
                    (x, y),
                    (x + w - offset, y),
                    (x + w, y + h),
                    (x + offset, y + h),
                ]
            };
            Some(points)
        }
        NodeShape::Trapezoid | NodeShape::TrapezoidAlt => {
            let offset = w * 0.18;
            let points = if node.shape == NodeShape::Trapezoid {
                vec![
                    (x + offset, y),
                    (x + w - offset, y),
                    (x + w, y + h),
                    (x, y + h),
                ]
            } else {
                vec![
                    (x, y),
                    (x + w, y),
                    (x + w - offset, y + h),
                    (x + offset, y + h),
                ]
            };
            Some(points)
        }
        NodeShape::Asymmetric => {
            let slant = w * 0.22;
            Some(vec![
                (x, y),
                (x + w - slant, y),
                (x + w, y + h / 2.0),
                (x + w - slant, y + h),
                (x, y + h),
            ])
        }
        NodeShape::Circle | NodeShape::DoubleCircle | NodeShape::Cylinder => None,
    }
}

pub(super) fn ray_polygon_intersection(
    origin: (f32, f32),
    dir: (f32, f32),
    poly: &[(f32, f32)],
) -> Option<(f32, f32)> {
    let mut best_t = None;
    let ox = origin.0;
    let oy = origin.1;
    let rx = dir.0;
    let ry = dir.1;
    if poly.len() < 2 {
        return None;
    }
    for i in 0..poly.len() {
        let (x1, y1) = poly[i];
        let (x2, y2) = poly[(i + 1) % poly.len()];
        let sx = x2 - x1;
        let sy = y2 - y1;
        let qx = x1 - ox;
        let qy = y1 - oy;
        let denom = rx * sy - ry * sx;
        if denom.abs() < GEOM_EPS {
            continue;
        }
        let t = (qx * sy - qy * sx) / denom;
        let u = (qx * ry - qy * rx) / denom;
        if t >= 0.0 && (0.0..=1.0).contains(&u) {
            match best_t {
                Some(best) if t >= best => {}
                _ => best_t = Some(t),
            }
        }
    }
    best_t.map(|t| (ox + rx * t, oy + ry * t))
}

pub(super) fn ray_ellipse_intersection(
    origin: (f32, f32),
    dir: (f32, f32),
    center: (f32, f32),
    rx: f32,
    ry: f32,
) -> Option<(f32, f32)> {
    let (ox, oy) = origin;
    let (dx, dy) = dir;
    let (cx, cy) = center;
    let ox = ox - cx;
    let oy = oy - cy;
    let a = (dx * dx) / (rx * rx) + (dy * dy) / (ry * ry);
    let b = 2.0 * ((ox * dx) / (rx * rx) + (oy * dy) / (ry * ry));
    let c = (ox * ox) / (rx * rx) + (oy * oy) / (ry * ry) - 1.0;
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 || a.abs() < GEOM_EPS {
        return None;
    }
    let sqrt_disc = disc.sqrt();
    let t1 = (-b - sqrt_disc) / (2.0 * a);
    let t2 = (-b + sqrt_disc) / (2.0 * a);
    let t = if t1 >= 0.0 {
        t1
    } else if t2 >= 0.0 {
        t2
    } else {
        return None;
    };
    Some((origin.0 + dx * t, origin.1 + dy * t))
}

pub(super) fn point_in_polygon_strict(point: (f32, f32), polygon: &[(f32, f32)]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    if polygon
        .iter()
        .copied()
        .zip(polygon.iter().copied().cycle().skip(1))
        .take(polygon.len())
        .any(|(a, b)| point_near_segment(point, a, b, POINT_EPS))
    {
        return false;
    }
    let mut inside = false;
    let (px, py) = point;
    let mut prev = polygon[polygon.len() - 1];
    for &curr in polygon {
        if (curr.1 > py) != (prev.1 > py) {
            let denom = prev.1 - curr.1;
            if denom.abs() > GEOM_EPS {
                let x_at_y = (prev.0 - curr.0) * (py - curr.1) / denom + curr.0;
                if px < x_at_y {
                    inside = !inside;
                }
            }
        }
        prev = curr;
    }
    inside
}

pub(super) fn point_inside_node_shape_strict(node: &NodeLayout, point: (f32, f32)) -> bool {
    point_inside_node_shape_strict_with(node, node_shape_polygon(node).as_deref(), point)
}

/// The polygon `point_inside_node_shape_strict` tests against, or `None` for ellipses and
/// shapes that fall back to the node bounds.
fn node_shape_polygon(node: &NodeLayout) -> Option<Vec<(f32, f32)>> {
    match node.shape {
        NodeShape::Circle | NodeShape::DoubleCircle => None,
        _ => shape_polygon_points(node),
    }
}

fn point_inside_node_shape_strict_with(
    node: &NodeLayout,
    polygon: Option<&[(f32, f32)]>,
    point: (f32, f32),
) -> bool {
    match node.shape {
        NodeShape::Circle | NodeShape::DoubleCircle => {
            let (cx, cy) = node_center(node);
            let rx = (node.width * 0.5 - POINT_EPS).max(1.0);
            let ry = (node.height * 0.5 - POINT_EPS).max(1.0);
            let nx = (point.0 - cx) / rx;
            let ny = (point.1 - cy) / ry;
            nx * nx + ny * ny < 1.0
        }
        _ => polygon
            .map(|polygon| point_in_polygon_strict(point, polygon))
            .unwrap_or_else(|| point_inside_node_bounds_strict(node, point)),
    }
}

pub(super) fn segment_hits_node_shape_interior(
    a: (f32, f32),
    b: (f32, f32),
    node: &NodeLayout,
) -> bool {
    // Every node shape lies within the node's bounds, so no sample of a segment whose bounding
    // box misses them can be strictly inside. The slack absorbs interpolation rounding.
    const SLACK: f32 = 1.0;
    let (nx0, nx1) = (
        node.x.min(node.x + node.width),
        node.x.max(node.x + node.width),
    );
    let (ny0, ny1) = (
        node.y.min(node.y + node.height),
        node.y.max(node.y + node.height),
    );
    if a.0.max(b.0) < nx0 - SLACK
        || a.0.min(b.0) > nx1 + SLACK
        || a.1.max(b.1) < ny0 - SLACK
        || a.1.min(b.1) > ny1 + SLACK
    {
        return false;
    }
    let polygon = node_shape_polygon(node);
    let steps = (((b.0 - a.0).hypot(b.1 - a.1) / 4.0).ceil() as usize).max(2);
    (1..steps).any(|i| {
        let t = i as f32 / steps as f32;
        point_inside_node_shape_strict_with(
            node,
            polygon.as_deref(),
            (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t),
        )
    })
}

pub(super) fn path_length(points: &[(f32, f32)]) -> f32 {
    points
        .windows(2)
        .map(|segment| (segment[1].0 - segment[0].0).hypot(segment[1].1 - segment[0].1))
        .sum()
}

pub(super) fn path_point_at_progress(points: &[(f32, f32)], progress: f32) -> Option<(f32, f32)> {
    if points.len() < 2 {
        return None;
    }
    let total = path_length(points);
    if !total.is_finite() || total <= GEOM_EPS {
        return Some(points[0]);
    }
    let mut remain = total * progress.clamp(0.0, 1.0);
    for segment in points.windows(2) {
        let a = segment[0];
        let b = segment[1];
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        let seg_len = (dx * dx + dy * dy).sqrt();
        if seg_len <= GEOM_EPS {
            continue;
        }
        if remain <= seg_len {
            let t = remain / seg_len;
            return Some((a.0 + dx * t, a.1 + dy * t));
        }
        remain -= seg_len;
    }
    points.last().copied()
}

pub(super) fn path_bend_count(points: &[(f32, f32)]) -> usize {
    if points.len() < 3 {
        return 0;
    }
    let mut bends = 0usize;
    for idx in 1..points.len() - 1 {
        let p0 = points[idx - 1];
        let p1 = points[idx];
        let p2 = points[idx + 1];
        let dx1 = p1.0 - p0.0;
        let dy1 = p1.1 - p0.1;
        let dx2 = p2.0 - p1.0;
        let dy2 = p2.1 - p1.1;
        if (dx1.abs() <= 1e-4 && dy1.abs() <= 1e-4) || (dx2.abs() <= 1e-4 && dy2.abs() <= 1e-4) {
            continue;
        }
        let cross = dx1 * dy2 - dy1 * dx2;
        if cross.abs() > 1e-4 {
            bends += 1;
        }
    }
    bends
}

pub(super) fn path_intersects_rect_bounds(
    points: &[(f32, f32)],
    rect: (f32, f32, f32, f32),
) -> bool {
    points
        .windows(2)
        .any(|segment| segment_intersects_rect_bounds(segment[0], segment[1], rect))
}

pub(super) fn segment_intersects_rect_bounds(
    a: (f32, f32),
    b: (f32, f32),
    rect: (f32, f32, f32, f32),
) -> bool {
    let (rx, ry, rw, rh) = rect;
    if rw <= 0.0 || rh <= 0.0 {
        return false;
    }
    let min_x = a.0.min(b.0);
    let max_x = a.0.max(b.0);
    let min_y = a.1.min(b.1);
    let max_y = a.1.max(b.1);
    if max_x < rx || min_x > rx + rw || max_y < ry || min_y > ry + rh {
        return false;
    }
    if point_in_rect(a, rect) || point_in_rect(b, rect) {
        return true;
    }
    let corners = [(rx, ry), (rx + rw, ry), (rx + rw, ry + rh), (rx, ry + rh)];
    let edges = [
        (corners[0], corners[1]),
        (corners[1], corners[2]),
        (corners[2], corners[3]),
        (corners[3], corners[0]),
    ];
    edges
        .into_iter()
        .any(|(c, d)| segments_intersect(a, b, c, d))
}

pub(super) fn segments_share_endpoint(
    a1: (f32, f32),
    a2: (f32, f32),
    b1: (f32, f32),
    b2: (f32, f32),
) -> bool {
    points_near(a1, b1) || points_near(a1, b2) || points_near(a2, b1) || points_near(a2, b2)
}

pub(super) fn segments_intersect(
    a: (f32, f32),
    b: (f32, f32),
    c: (f32, f32),
    d: (f32, f32),
) -> bool {
    let o1 = orient(a, b, c);
    let o2 = orient(a, b, d);
    let o3 = orient(c, d, a);
    let o4 = orient(c, d, b);
    if ((o1 > 0.0 && o2 < 0.0) || (o1 < 0.0 && o2 > 0.0))
        && ((o3 > 0.0 && o4 < 0.0) || (o3 < 0.0 && o4 > 0.0))
    {
        return true;
    }
    if o1.abs() <= GEOM_EPS && on_segment(a, b, c) {
        return true;
    }
    if o2.abs() <= GEOM_EPS && on_segment(a, b, d) {
        return true;
    }
    if o3.abs() <= GEOM_EPS && on_segment(c, d, a) {
        return true;
    }
    if o4.abs() <= GEOM_EPS && on_segment(c, d, b) {
        return true;
    }
    false
}

fn point_inside_node_bounds_strict(node: &NodeLayout, point: (f32, f32)) -> bool {
    point.0 > node.x + POINT_EPS
        && point.0 < node.x + node.width - POINT_EPS
        && point.1 > node.y + POINT_EPS
        && point.1 < node.y + node.height - POINT_EPS
}

fn point_in_rect(point: (f32, f32), rect: (f32, f32, f32, f32)) -> bool {
    let (rx, ry, rw, rh) = rect;
    point.0 >= rx && point.0 <= rx + rw && point.1 >= ry && point.1 <= ry + rh
}

fn point_near_segment(point: (f32, f32), a: (f32, f32), b: (f32, f32), eps: f32) -> bool {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let len2 = dx * dx + dy * dy;
    if len2 <= GEOM_EPS {
        return points_near(point, a);
    }
    let t = ((point.0 - a.0) * dx + (point.1 - a.1) * dy) / len2;
    if !(-GEOM_EPS..=1.0 + GEOM_EPS).contains(&t) {
        return false;
    }
    let clamped_t = t.clamp(0.0, 1.0);
    let proj = (a.0 + dx * clamped_t, a.1 + dy * clamped_t);
    (point.0 - proj.0).hypot(point.1 - proj.1) <= eps
}

fn points_near(a: (f32, f32), b: (f32, f32)) -> bool {
    (a.0 - b.0).abs() <= POINT_EPS && (a.1 - b.1).abs() <= POINT_EPS
}

fn orient(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

/// Whether point `p` lies within the bounding box of segment `a`-`b`.
///
/// Callers pair this with a collinearity (`orient`) check, so the bounding box
/// test is sufficient for "point on segment". The epsilon must stay tiny:
/// a loose epsilon makes distinct collinear segments on a shared axis line
/// (such as straight chain edges) register as intersecting, which makes
/// crossing-reduction passes bend perfectly straight routes.
fn on_segment(a: (f32, f32), b: (f32, f32), p: (f32, f32)) -> bool {
    p.0 >= a.0.min(b.0) - GEOM_EPS
        && p.0 <= a.0.max(b.0) + GEOM_EPS
        && p.1 >= a.1.min(b.1) - GEOM_EPS
        && p.1 <= a.1.max(b.1) + GEOM_EPS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{NodeShape, NodeStyle};
    use crate::layout::TextBlock;

    fn node(shape: NodeShape) -> NodeLayout {
        NodeLayout {
            id: "n".to_string(),
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 60.0,
            label: TextBlock {
                lines: Vec::new(),
                width: 0.0,
                height: 0.0,
            },
            shape,
            style: NodeStyle::default(),
            link: None,
            anchor_subgraph: None,
            hidden: false,
            icon: None,
        }
    }

    #[test]
    fn diamond_interior_uses_actual_shape_not_bounding_box() {
        let diamond = node(NodeShape::Diamond);
        assert!(point_inside_node_shape_strict(&diamond, (60.0, 50.0)));
        assert!(!point_inside_node_shape_strict(&diamond, (15.0, 25.0)));
        assert!(!point_inside_node_shape_strict(&diamond, (60.0, 20.0)));
        assert!(segment_hits_node_shape_interior(
            (60.0, 10.0),
            (60.0, 90.0),
            &diamond
        ));
    }

    #[test]
    fn rectangle_boundary_is_not_strict_interior() {
        let rect = node(NodeShape::Rectangle);
        assert!(!point_inside_node_shape_strict(&rect, (60.0, 20.0)));
        assert!(!segment_hits_node_shape_interior(
            (0.0, 20.0),
            (120.0, 20.0),
            &rect
        ));
    }

    #[test]
    fn endpoint_side_and_outward_checks_are_side_aware() {
        let rect = node(NodeShape::Rectangle);
        let side = endpoint_side_for_point(&rect, (110.0, 50.0));
        assert_eq!(side, EdgeSide::Right);
        assert!(source_exits_outward(side, (110.0, 50.0), (120.0, 50.0)));
        assert!(!source_exits_outward(side, (110.0, 50.0), (100.0, 50.0)));
    }
}
