use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap, HashMap, HashSet};

use crate::config::LayoutConfig;
use crate::ir::Direction;

use super::{NodeLayout, SubgraphLayout};

// ── Edge side selection ──────────────────────────────────────────────
/// Aspect-ratio threshold for preferring horizontal vs vertical edge sides.
const DIRECTION_PREF_RATIO: f32 = 1.35;
/// Maximum detour penalty before a non-primary side is rejected.
const ROUTE_DETOUR_THRESHOLD: f32 = 120.0;
/// Soft side load cap used to encourage side diversification on dense hubs.
const SIDE_LOAD_SOFT_CAP: f32 = 6.0;
/// For hub->leaf edges, keep main-axis forcing unless geometric cross-axis
/// separation is clearly stronger and the forced sides are already saturated.
const HUB_DIVERSIFY_GEOM_RATIO: f32 = 1.35;
const HUB_DIVERSIFY_LOAD_SUM: usize = 14;
const LOW_DEGREE_BALANCE_MIN_PRIMARY_LOAD: usize = 4;

// ── Port stub sizing ────────────────────────────────────────────────
/// Ratio of node_spacing used as base port stub length.
const PORT_STUB_RATIO: f32 = 0.35;
/// Ratio of smallest node dimension used to cap stub length.
const PORT_STUB_SIZE_CAP_RATIO: f32 = 0.35;
/// Default max stub length when node size cap is invalid.
const PORT_STUB_DEFAULT_MAX: f32 = 18.0;
/// Hard clamp range for port stub length.
const PORT_STUB_MIN: f32 = 6.0;
const PORT_STUB_MAX: f32 = 22.0;

// ── Routing grid ────────────────────────────────────────────────────
/// Default routing cell size as a ratio of node_spacing.
const ROUTING_CELL_RATIO: f32 = 0.35;
/// Minimum routing cell size.
const ROUTING_CELL_MIN: f32 = 8.0;
/// Minimum node spacing used to compute grid margin.
const GRID_MARGIN_MIN_SPACING: f32 = 24.0;

// ── A* cost scaling ─────────────────────────────────────────────────
/// Integer cost multiplier so A* can use u32 costs with fractional cell sizes.
const ASTAR_COST_SCALE: f32 = 1000.0;

// ── Self-loop / orthogonal routing pad ──────────────────────────────
/// Ratio of node_spacing used for self-loop padding and routing step.
const ROUTING_PAD_RATIO: f32 = 0.6;
/// Minimum node spacing for self-loop / routing pad computations.
const ROUTING_PAD_MIN_SPACING: f32 = 20.0;
/// Minimum node spacing used in orthogonal routing step fallback.
const ORTHO_STEP_MIN_SPACING: f32 = 16.0;
/// Fraction of step used as channel candidate threshold.
const CHANNEL_CANDIDATE_RATIO: f32 = 0.75;

// ── Obstacle construction ───────────────────────────────────────────
/// Ratio of node_spacing used for obstacle padding around nodes/subgraphs.
const OBSTACLE_PAD_RATIO: f32 = 0.35;
/// Minimum obstacle padding.
const OBSTACLE_PAD_MIN: f32 = 6.0;

// ── Occupancy overlap detection ─────────────────────────────────────
/// Fraction of path-length-in-cells used to trigger occupancy detour.
const OVERLAP_TRIGGER_RATIO: f32 = 0.35;
/// Minimum overlap cell count to trigger detour.
const OVERLAP_TRIGGER_MIN: f32 = 4.0;
/// Minimum collinear overlap length (px) that should trigger extra detour search.
const OVERLAP_DETOUR_MIN: f32 = 3.0;
/// Path-length epsilon used when preferring shorter routes in tie-breaks.
const ROUTE_LENGTH_TIE_EPS: f32 = 2.0;
/// Tie-break epsilon for path distance to the preferred label center.
const ROUTE_VIA_TIE_EPS: f32 = 0.4;

// ── Label obstacle padding ──────────────────────────────────────────
/// Padding around node labels when building label obstacles.
const LABEL_OBSTACLE_NODE_PAD: f32 = 2.0;
/// Padding around subgraph labels when building label obstacles.
const LABEL_OBSTACLE_SUB_PAD: f32 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum EdgeSide {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct EdgePortInfo {
    pub(super) start_side: EdgeSide,
    pub(super) end_side: EdgeSide,
    pub(super) start_offset: f32,
    pub(super) end_offset: f32,
}

#[derive(Debug, Clone)]
pub(super) struct PortCandidate {
    pub(super) edge_idx: usize,
    pub(super) is_start: bool,
    pub(super) other_pos: f32,
}

#[derive(Debug, Clone)]
pub(super) struct Obstacle {
    pub(super) id: String,
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) width: f32,
    pub(super) height: f32,
    pub(super) members: Option<HashSet<String>>,
}

pub(super) fn is_horizontal(direction: Direction) -> bool {
    matches!(direction, Direction::LeftRight | Direction::RightLeft)
}

pub(super) fn side_is_vertical(side: EdgeSide) -> bool {
    matches!(side, EdgeSide::Left | EdgeSide::Right)
}

pub(super) fn edge_sides(
    from: &NodeLayout,
    to: &NodeLayout,
    direction: Direction,
) -> (EdgeSide, EdgeSide, bool) {
    let from_cx = from.x + from.width / 2.0;
    let from_cy = from.y + from.height / 2.0;
    let to_cx = to.x + to.width / 2.0;
    let to_cy = to.y + to.height / 2.0;
    let dx = to_cx - from_cx;
    let dy = to_cy - from_cy;
    let x_overlap = (from.x.max(to.x) - (from.x + from.width).min(to.x + to.width)).abs() < 1e-3
        || from.x < to.x + to.width && to.x < from.x + from.width;
    let y_overlap = (from.y.max(to.y) - (from.y + from.height).min(to.y + to.height)).abs() < 1e-3
        || from.y < to.y + to.height && to.y < from.y + from.height;

    let ratio = dx.abs() / (dy.abs().max(1e-3));
    let horiz_pref = ratio > DIRECTION_PREF_RATIO || (y_overlap && ratio > 0.9);
    let vert_pref = ratio < (1.0 / DIRECTION_PREF_RATIO) || (x_overlap && ratio < 1.1);
    let use_horizontal = if horiz_pref && !vert_pref {
        true
    } else if vert_pref && !horiz_pref {
        false
    } else {
        is_horizontal(direction)
    };

    if use_horizontal {
        let is_backward = to.x + to.width < from.x;
        if dx >= 0.0 {
            (EdgeSide::Right, EdgeSide::Left, is_backward)
        } else {
            (EdgeSide::Left, EdgeSide::Right, is_backward)
        }
    } else {
        let is_backward = to.y + to.height < from.y;
        if dy >= 0.0 {
            (EdgeSide::Bottom, EdgeSide::Top, is_backward)
        } else {
            (EdgeSide::Top, EdgeSide::Bottom, is_backward)
        }
    }
}

pub(super) fn edge_axis_is_horizontal(side: EdgeSide) -> bool {
    side_is_vertical(side)
}

pub(super) fn side_slot(side: EdgeSide) -> usize {
    match side {
        EdgeSide::Left => 0,
        EdgeSide::Right => 1,
        EdgeSide::Top => 2,
        EdgeSide::Bottom => 3,
    }
}

pub(super) fn side_load_for_node(
    side_loads: &HashMap<String, [usize; 4]>,
    node_id: &str,
    side: EdgeSide,
) -> usize {
    side_loads
        .get(node_id)
        .map(|slots| slots[side_slot(side)])
        .unwrap_or(0)
}

pub(super) fn bump_side_load(
    side_loads: &mut HashMap<String, [usize; 4]>,
    node_id: &str,
    side: EdgeSide,
) {
    let slots = side_loads.entry(node_id.to_string()).or_insert([0; 4]);
    slots[side_slot(side)] += 1;
}

pub(super) fn edge_sides_balanced(
    from_id: &str,
    to_id: &str,
    from: &NodeLayout,
    to: &NodeLayout,
    allow_low_degree_balancing: bool,
    direction: Direction,
    node_degrees: &HashMap<String, usize>,
    side_loads: &HashMap<String, [usize; 4]>,
) -> (EdgeSide, EdgeSide, bool) {
    let primary = edge_sides(from, to, direction);
    let from_degree = node_degrees.get(from_id).copied().unwrap_or(0);
    let to_degree = node_degrees.get(to_id).copied().unwrap_or(0);
    if from_degree < 6 && to_degree < 6 {
        if !allow_low_degree_balancing {
            return primary;
        }
        let primary_load = side_load_for_node(side_loads, from_id, primary.0)
            + side_load_for_node(side_loads, to_id, primary.1);
        if primary_load < LOW_DEGREE_BALANCE_MIN_PRIMARY_LOAD {
            return primary;
        }
    }

    let from_cx = from.x + from.width / 2.0;
    let from_cy = from.y + from.height / 2.0;
    let to_cx = to.x + to.width / 2.0;
    let to_cy = to.y + to.height / 2.0;
    let dx = to_cx - from_cx;
    let dy = to_cy - from_cy;

    // For hub-to-leaf edges, side balancing can over-disperse ports and
    // introduce fan crossing. Prefer the diagram's main direction axis.
    if (from_degree >= 10 && to_degree <= 4) || (to_degree >= 10 && from_degree <= 4) {
        let forced = if is_horizontal(direction) {
            let is_backward = to.x + to.width < from.x;
            if dx >= 0.0 {
                (EdgeSide::Right, EdgeSide::Left, is_backward)
            } else {
                (EdgeSide::Left, EdgeSide::Right, is_backward)
            }
        } else {
            let is_backward = to.y + to.height < from.y;
            if dy >= 0.0 {
                (EdgeSide::Bottom, EdgeSide::Top, is_backward)
            } else {
                (EdgeSide::Top, EdgeSide::Bottom, is_backward)
            }
        };
        let forced_load = side_load_for_node(side_loads, from_id, forced.0)
            + side_load_for_node(side_loads, to_id, forced.1);
        let main_axis = if is_horizontal(direction) {
            dx.abs()
        } else {
            dy.abs()
        };
        let cross_axis = if is_horizontal(direction) {
            dy.abs()
        } else {
            dx.abs()
        };
        let can_diversify = forced_load >= HUB_DIVERSIFY_LOAD_SUM
            && cross_axis > main_axis * HUB_DIVERSIFY_GEOM_RATIO;
        if !can_diversify {
            return forced;
        }
    }

    let horizontal = if dx >= 0.0 {
        (EdgeSide::Right, EdgeSide::Left, to.x + to.width < from.x)
    } else {
        (EdgeSide::Left, EdgeSide::Right, to.x > from.x + from.width)
    };
    let vertical = if dy >= 0.0 {
        (EdgeSide::Bottom, EdgeSide::Top, to.y + to.height < from.y)
    } else {
        (EdgeSide::Top, EdgeSide::Bottom, to.y > from.y + from.height)
    };

    let mut options = vec![primary];
    if !options
        .iter()
        .any(|(start, end, _)| *start == horizontal.0 && *end == horizontal.1)
    {
        options.push(horizontal);
    }
    if !options
        .iter()
        .any(|(start, end, _)| *start == vertical.0 && *end == vertical.1)
    {
        options.push(vertical);
    }

    let primary_axis = edge_axis_is_horizontal(primary.0);
    let primary_from_anchor = anchor_point_for_node(from, primary.0, 0.0);
    let primary_to_anchor = anchor_point_for_node(to, primary.1, 0.0);
    let primary_manhattan = (primary_to_anchor.0 - primary_from_anchor.0).abs()
        + (primary_to_anchor.1 - primary_from_anchor.1).abs();
    let mut best = primary;
    let mut best_score = f32::MAX;
    let mut best_tiebreak = f32::MAX;
    for (start_side, end_side, is_backward) in options {
        let from_load = side_load_for_node(side_loads, from_id, start_side) as f32;
        let to_load = side_load_for_node(side_loads, to_id, end_side) as f32;
        let load_score = from_load * from_load + to_load * to_load + (from_load + to_load) * 0.5;
        let overload =
            (from_load - SIDE_LOAD_SOFT_CAP).max(0.0) + (to_load - SIDE_LOAD_SOFT_CAP).max(0.0);
        let overload_penalty = overload * overload * 6.0;
        let from_anchor = anchor_point_for_node(from, start_side, 0.0);
        let to_anchor = anchor_point_for_node(to, end_side, 0.0);
        let manhattan = (to_anchor.0 - from_anchor.0).abs() + (to_anchor.1 - from_anchor.1).abs();
        if !(start_side == primary.0 && end_side == primary.1)
            && manhattan > primary_manhattan * DIRECTION_PREF_RATIO + ROUTE_DETOUR_THRESHOLD
        {
            continue;
        }
        let axis_penalty = if edge_axis_is_horizontal(start_side) == primary_axis {
            0.0
        } else {
            5.0
        };
        let primary_penalty = if start_side == primary.0 && end_side == primary.1 {
            0.0
        } else {
            2.0
        };
        let backward_penalty = if is_backward && !primary.2 { 4.0 } else { 0.0 };
        let score = load_score * 9.0
            + overload_penalty
            + manhattan * 0.22
            + axis_penalty
            + primary_penalty
            + backward_penalty;
        let tiebreak = manhattan + from_load + to_load;
        if score < best_score || ((score - best_score).abs() < 1e-4 && tiebreak < best_tiebreak) {
            best = (start_side, end_side, is_backward);
            best_score = score;
            best_tiebreak = tiebreak;
        }
    }

    best
}

pub(super) struct RouteContext<'a> {
    pub(super) from_id: &'a str,
    pub(super) to_id: &'a str,
    pub(super) from: &'a NodeLayout,
    pub(super) to: &'a NodeLayout,
    pub(super) direction: Direction,
    pub(super) config: &'a LayoutConfig,
    pub(super) obstacles: &'a [Obstacle],
    pub(super) label_obstacles: &'a [Obstacle],
    pub(super) fast_route: bool,
    pub(super) base_offset: f32,
    pub(super) start_side: EdgeSide,
    pub(super) end_side: EdgeSide,
    pub(super) start_offset: f32,
    pub(super) end_offset: f32,
    pub(super) stub_len: f32,
    pub(super) start_inset: f32,
    pub(super) end_inset: f32,
    pub(super) prefer_shorter_ties: bool,
    pub(super) preferred_label_id: Option<&'a str>,
    pub(super) preferred_label_center: Option<(f32, f32)>,
}

#[derive(Debug, Clone)]
pub(super) struct EdgeOccupancy {
    cell: f32,
    weights: HashMap<(i32, i32), u16>,
}

impl EdgeOccupancy {
    pub(super) fn new(cell: f32) -> Self {
        let cell = cell.max(8.0);
        Self {
            cell,
            weights: HashMap::new(),
        }
    }

    pub(super) fn cell_index(&self, x: f32, y: f32) -> (i32, i32) {
        (
            (x / self.cell).floor() as i32,
            (y / self.cell).floor() as i32,
        )
    }

    pub(super) fn score_path(&self, points: &[(f32, f32)]) -> u32 {
        let mut score = 0u32;
        for segment in points.windows(2) {
            let (x1, y1) = segment[0];
            let (x2, y2) = segment[1];
            let dx = x2 - x1;
            let dy = y2 - y1;
            let len = (dx * dx + dy * dy).sqrt();
            let steps = ((len / self.cell).ceil() as usize).max(1);
            let stride = if steps > 32 { (steps / 32).max(1) } else { 1 };
            for i in (0..=steps).step_by(stride) {
                let t = i as f32 / steps as f32;
                let x = x1 + dx * t;
                let y = y1 + dy * t;
                if let Some(weight) = self.weights.get(&self.cell_index(x, y)) {
                    score += *weight as u32;
                }
            }
        }
        score
    }

    pub(super) fn overlap_count(&self, points: &[(f32, f32)]) -> u32 {
        let mut count = 0u32;
        for segment in points.windows(2) {
            let (x1, y1) = segment[0];
            let (x2, y2) = segment[1];
            let dx = x2 - x1;
            let dy = y2 - y1;
            let len = (dx * dx + dy * dy).sqrt();
            let steps = ((len / self.cell).ceil() as usize).max(1);
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                let x = x1 + dx * t;
                let y = y1 + dy * t;
                if let Some(weight) = self.weights.get(&self.cell_index(x, y))
                    && *weight > 0
                {
                    count = count.saturating_add(1);
                }
            }
        }
        count
    }

    pub(super) fn add_path(&mut self, points: &[(f32, f32)]) {
        for segment in points.windows(2) {
            let (x1, y1) = segment[0];
            let (x2, y2) = segment[1];
            let dx = x2 - x1;
            let dy = y2 - y1;
            let len = (dx * dx + dy * dy).sqrt();
            let steps = ((len / self.cell).ceil() as usize).max(1);
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                let x = x1 + dx * t;
                let y = y1 + dy * t;
                let (ix, iy) = self.cell_index(x, y);
                for dx_cell in -1i32..=1 {
                    for dy_cell in -1i32..=1 {
                        let weight = match (dx_cell.abs(), dy_cell.abs()) {
                            (0, 0) => 3u16,
                            (1, 0) | (0, 1) => 2u16,
                            _ => 1u16,
                        };
                        let idx = (ix + dx_cell, iy + dy_cell);
                        let entry = self.weights.entry(idx).or_insert(0);
                        *entry = entry.saturating_add(weight);
                    }
                }
            }
        }
    }

    pub(super) fn weight_at(&self, x: f32, y: f32) -> u16 {
        self.weights
            .get(&self.cell_index(x, y))
            .copied()
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone)]
pub(super) struct RoutingGrid {
    cell: f32,
    min_x: f32,
    min_y: f32,
    cols: i32,
    rows: i32,
    cell_obstacles: Vec<Vec<usize>>,
}

impl RoutingGrid {
    fn new(obstacles: &[Obstacle], cell: f32, margin: f32, max_cells: usize) -> Option<Self> {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for obs in obstacles {
            min_x = min_x.min(obs.x);
            min_y = min_y.min(obs.y);
            max_x = max_x.max(obs.x + obs.width);
            max_y = max_y.max(obs.y + obs.height);
        }
        if min_x == f32::MAX {
            return None;
        }
        min_x -= margin;
        min_y -= margin;
        max_x += margin;
        max_y += margin;
        let cell = cell.max(6.0);
        let cols = ((max_x - min_x) / cell).ceil() as i32 + 1;
        let rows = ((max_y - min_y) / cell).ceil() as i32 + 1;
        if cols <= 1 || rows <= 1 {
            return None;
        }
        let total_cells = (cols as usize).saturating_mul(rows as usize);
        if total_cells > max_cells {
            return None;
        }
        let mut cell_obstacles = vec![Vec::new(); (cols * rows) as usize];
        for (idx, obs) in obstacles.iter().enumerate() {
            let start_x = ((obs.x - min_x) / cell).floor().max(0.0) as i32;
            let end_x = ((obs.x + obs.width - min_x) / cell)
                .floor()
                .min((cols - 1) as f32) as i32;
            let start_y = ((obs.y - min_y) / cell).floor().max(0.0) as i32;
            let end_y = ((obs.y + obs.height - min_y) / cell)
                .floor()
                .min((rows - 1) as f32) as i32;
            for iy in start_y..=end_y {
                for ix in start_x..=end_x {
                    let cell_idx = (iy * cols + ix) as usize;
                    cell_obstacles[cell_idx].push(idx);
                }
            }
        }
        Some(Self {
            cell,
            min_x,
            min_y,
            cols,
            rows,
            cell_obstacles,
        })
    }

    fn index(&self, ix: i32, iy: i32) -> usize {
        (iy * self.cols + ix) as usize
    }

    fn cell_for_point(&self, x: f32, y: f32) -> Option<(i32, i32)> {
        let ix = ((x - self.min_x) / self.cell).floor() as i32;
        let iy = ((y - self.min_y) / self.cell).floor() as i32;
        if ix < 0 || iy < 0 || ix >= self.cols || iy >= self.rows {
            return None;
        }
        Some((ix, iy))
    }

    fn cell_center(&self, ix: i32, iy: i32) -> (f32, f32) {
        (
            self.min_x + (ix as f32 + 0.5) * self.cell,
            self.min_y + (iy as f32 + 0.5) * self.cell,
        )
    }

    fn cell_obstacle_indices(&self, ix: i32, iy: i32) -> &[usize] {
        &self.cell_obstacles[self.index(ix, iy)]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct GridState {
    x: i32,
    y: i32,
    dir: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct GridEntry {
    est: u32,
    cost: u32,
    state: GridState,
}

impl Ord for GridEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .est
            .cmp(&self.est)
            .then_with(|| other.cost.cmp(&self.cost))
            .then_with(|| self.state.y.cmp(&other.state.y))
            .then_with(|| self.state.x.cmp(&other.state.x))
            .then_with(|| self.state.dir.cmp(&other.state.dir))
    }
}

impl PartialOrd for GridEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub(super) fn apply_port_offset(point: (f32, f32), side: EdgeSide, offset: f32) -> (f32, f32) {
    match side {
        EdgeSide::Left | EdgeSide::Right => (point.0, point.1 + offset),
        EdgeSide::Top | EdgeSide::Bottom => (point.0 + offset, point.1),
    }
}

pub(super) fn port_stub_length(config: &LayoutConfig, from: &NodeLayout, to: &NodeLayout) -> f32 {
    let base = config.node_spacing * PORT_STUB_RATIO;
    let size_cap =
        from.width.min(from.height).min(to.width.min(to.height)) * PORT_STUB_SIZE_CAP_RATIO;
    let max_len = if size_cap.is_finite() && size_cap > 0.0 {
        size_cap
    } else {
        PORT_STUB_DEFAULT_MAX
    };
    base.min(max_len).clamp(PORT_STUB_MIN, PORT_STUB_MAX)
}

pub(super) fn port_stub_point(point: (f32, f32), side: EdgeSide, length: f32) -> (f32, f32) {
    match side {
        EdgeSide::Left => (point.0 - length, point.1),
        EdgeSide::Right => (point.0 + length, point.1),
        EdgeSide::Top => (point.0, point.1 - length),
        EdgeSide::Bottom => (point.0, point.1 + length),
    }
}

pub(super) fn shape_polygon_points(node: &NodeLayout) -> Option<Vec<(f32, f32)>> {
    let x = node.x;
    let y = node.y;
    let w = node.width;
    let h = node.height;
    match node.shape {
        crate::ir::NodeShape::Rectangle
        | crate::ir::NodeShape::RoundRect
        | crate::ir::NodeShape::ActorBox
        | crate::ir::NodeShape::Stadium
        | crate::ir::NodeShape::Subroutine
        | crate::ir::NodeShape::Text => Some(vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h)]),
        crate::ir::NodeShape::Diamond => {
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            Some(vec![(cx, y), (x + w, cy), (cx, y + h), (x, cy)])
        }
        crate::ir::NodeShape::Hexagon => {
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
        crate::ir::NodeShape::Parallelogram | crate::ir::NodeShape::ParallelogramAlt => {
            let offset = w * 0.18;
            let points = if node.shape == crate::ir::NodeShape::Parallelogram {
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
        crate::ir::NodeShape::Trapezoid | crate::ir::NodeShape::TrapezoidAlt => {
            let offset = w * 0.18;
            let points = if node.shape == crate::ir::NodeShape::Trapezoid {
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
        crate::ir::NodeShape::Asymmetric => {
            let slant = w * 0.22;
            Some(vec![
                (x, y),
                (x + w - slant, y),
                (x + w, y + h / 2.0),
                (x + w - slant, y + h),
                (x, y + h),
            ])
        }
        _ => None,
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
        if denom.abs() < 1e-6 {
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
    if disc < 0.0 || a.abs() < 1e-6 {
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

/// Compute where a straight line from `remote` to `node`'s centre would
/// cross `node`'s boundary on `side`.  Returns the coordinate along the
/// side's axis (x for Top/Bottom, y for Left/Right) – i.e. the ideal
/// port position if the edge could travel in a straight line.
pub(super) fn ideal_port_pos(remote: (f32, f32), node: &NodeLayout, side: EdgeSide) -> f32 {
    let cx = node.x + node.width / 2.0;
    let cy = node.y + node.height / 2.0;
    if side_is_vertical(side) {
        // Left / Right – port distributed along y-axis
        let edge_x = if matches!(side, EdgeSide::Left) {
            node.x
        } else {
            node.x + node.width
        };
        let dx = cx - remote.0;
        if dx.abs() < 1.0 {
            return cy;
        }
        let t = (edge_x - remote.0) / dx;
        remote.1 + t * (cy - remote.1)
    } else {
        // Top / Bottom – port distributed along x-axis
        let edge_y = if matches!(side, EdgeSide::Top) {
            node.y
        } else {
            node.y + node.height
        };
        let dy = cy - remote.1;
        if dy.abs() < 1.0 {
            return cx;
        }
        let t = (edge_y - remote.1) / dy;
        remote.0 + t * (cx - remote.0)
    }
}

pub(super) fn anchor_point_for_node(node: &NodeLayout, side: EdgeSide, offset: f32) -> (f32, f32) {
    let cx = node.x + node.width / 2.0;
    let cy = node.y + node.height / 2.0;
    let (dir, perp, max_offset) = match side {
        EdgeSide::Left => ((-1.0, 0.0), (0.0, 1.0), node.height / 2.0 - 1.0),
        EdgeSide::Right => ((1.0, 0.0), (0.0, 1.0), node.height / 2.0 - 1.0),
        EdgeSide::Top => ((0.0, -1.0), (1.0, 0.0), node.width / 2.0 - 1.0),
        EdgeSide::Bottom => ((0.0, 1.0), (1.0, 0.0), node.width / 2.0 - 1.0),
    };
    let clamp = if max_offset > 0.0 {
        offset.clamp(-max_offset, max_offset)
    } else {
        0.0
    };
    let origin = (cx + perp.0 * clamp, cy + perp.1 * clamp);

    match node.shape {
        crate::ir::NodeShape::Circle | crate::ir::NodeShape::DoubleCircle => {
            let rx = node.width / 2.0;
            let ry = node.height / 2.0;
            if let Some(point) = ray_ellipse_intersection(origin, dir, (cx, cy), rx, ry) {
                return point;
            }
        }
        _ => {}
    }

    if let Some(poly) = shape_polygon_points(node)
        && let Some(point) = ray_polygon_intersection(origin, dir, &poly)
    {
        return point;
    }

    // Fallback to bounding box anchor.
    let base = match side {
        EdgeSide::Left => (node.x, cy),
        EdgeSide::Right => (node.x + node.width, cy),
        EdgeSide::Top => (cx, node.y),
        EdgeSide::Bottom => (cx, node.y + node.height),
    };
    apply_port_offset(base, side, clamp)
}

pub(super) fn routing_cell_size(config: &LayoutConfig) -> f32 {
    let mut cell = config.flowchart.routing.grid_cell;
    if cell <= 0.0 {
        cell = config.node_spacing * ROUTING_CELL_RATIO;
    }
    cell.max(ROUTING_CELL_MIN)
}

pub(super) fn build_routing_grid(
    obstacles: &[Obstacle],
    config: &LayoutConfig,
) -> Option<RoutingGrid> {
    let cell = routing_cell_size(config);
    let margin = config.node_spacing.max(GRID_MARGIN_MIN_SPACING) * 2.0;
    let max_cells = (config.flowchart.routing.max_steps / 16).max(3000);
    RoutingGrid::new(obstacles, cell, margin, max_cells)
}

pub(super) fn cell_blocked(
    grid: &RoutingGrid,
    obstacles: &[Obstacle],
    ix: i32,
    iy: i32,
    ctx: &RouteContext<'_>,
) -> bool {
    let (cx, cy) = grid.cell_center(ix, iy);
    for &obs_idx in grid.cell_obstacle_indices(ix, iy) {
        let obstacle = &obstacles[obs_idx];
        if obstacle.id == ctx.from_id || obstacle.id == ctx.to_id {
            continue;
        }
        if let Some(members) = &obstacle.members
            && (members.contains(ctx.from_id) || members.contains(ctx.to_id))
        {
            continue;
        }
        if cx >= obstacle.x
            && cx <= obstacle.x + obstacle.width
            && cy >= obstacle.y
            && cy <= obstacle.y + obstacle.height
        {
            return true;
        }
    }
    false
}

/// Insert a label dummy center as a via-point into an edge's routed path.
/// Finds the segment where the via-point falls (by main-axis coordinate)
/// and inserts the point there so the edge bends through the label position.
pub(super) fn insert_label_via_point(
    points: &mut Vec<(f32, f32)>,
    via: (f32, f32),
    _direction: Direction,
) {
    if points.len() < 2 {
        return;
    }
    if polyline_point_distance(points, via) <= 0.6 {
        return;
    }
    // Insert on the segment that minimizes extra path length, which keeps
    // the routed path stable and avoids large detours from axis-only matching.
    let mut best_idx = None;
    let mut best_delta = f32::INFINITY;
    for i in 1..points.len() {
        let a = points[i - 1];
        let b = points[i];
        let base_len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        if base_len <= 1e-4 {
            continue;
        }
        let via_len_a = ((via.0 - a.0).powi(2) + (via.1 - a.1).powi(2)).sqrt();
        let via_len_b = ((via.0 - b.0).powi(2) + (via.1 - b.1).powi(2)).sqrt();
        let delta = (via_len_a + via_len_b - base_len).max(0.0);
        if delta < best_delta {
            best_delta = delta;
            best_idx = Some(i);
        }
    }

    if let Some(i) = best_idx {
        let dist_a = ((via.0 - points[i - 1].0).powi(2) + (via.1 - points[i - 1].1).powi(2)).sqrt();
        let dist_b = ((via.0 - points[i].0).powi(2) + (via.1 - points[i].1).powi(2)).sqrt();
        if dist_a > 2.0 && dist_b > 2.0 {
            points.insert(i, via);
        }
        return;
    }

    let mid = points.len() / 2;
    points.insert(mid, via);
}

pub(super) fn compress_path(points: &[(f32, f32)]) -> Vec<(f32, f32)> {
    if points.len() <= 2 {
        return points.to_vec();
    }
    let mut out: Vec<(f32, f32)> = Vec::with_capacity(points.len());
    out.push(points[0]);
    for idx in 1..points.len() - 1 {
        let prev = out[out.len() - 1];
        let curr = points[idx];
        if (curr.0 - prev.0).abs() <= 1e-4 && (curr.1 - prev.1).abs() <= 1e-4 {
            continue;
        }
        if idx == 1 || idx == points.len() - 2 {
            out.push(curr);
            continue;
        }
        let next = points[idx + 1];
        let dx1 = curr.0 - prev.0;
        let dy1 = curr.1 - prev.1;
        let dx2 = next.0 - curr.0;
        let dy2 = next.1 - curr.1;
        if (dx1.abs() <= 1e-4 && dx2.abs() <= 1e-4) || (dy1.abs() <= 1e-4 && dy2.abs() <= 1e-4) {
            // Keep explicit reversal points (U-turns). They are needed when
            // forcing a route through a reserved label center.
            let dot = dx1 * dx2 + dy1 * dy2;
            if dot >= 0.0 {
                continue;
            }
        }
        out.push(curr);
    }
    let last = points[points.len() - 1];
    if (last.0 - out[out.len() - 1].0).abs() > 1e-4 || (last.1 - out[out.len() - 1].1).abs() > 1e-4
    {
        out.push(last);
    }
    out
}

pub(super) fn route_edge_with_grid(
    ctx: &RouteContext<'_>,
    grid: &RoutingGrid,
    occupancy: Option<&EdgeOccupancy>,
    start: (f32, f32),
    end: (f32, f32),
) -> Option<Vec<(f32, f32)>> {
    if !ctx.config.flowchart.routing.enable_grid_router {
        return None;
    }

    let (start_ix, start_iy) = grid.cell_for_point(start.0, start.1)?;
    let (end_ix, end_iy) = grid.cell_for_point(end.0, end.1)?;
    if start_ix == end_ix && start_iy == end_iy {
        return Some(vec![start, end]);
    }

    let dirs: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];
    let step_cost = (grid.cell * ASTAR_COST_SCALE).round() as u32;
    let turn_penalty =
        (ctx.config.flowchart.routing.turn_penalty * grid.cell * ASTAR_COST_SCALE).round() as u32;
    let occupancy_weight =
        (ctx.config.flowchart.routing.occupancy_weight * grid.cell * ASTAR_COST_SCALE).round()
            as u32;
    let max_steps = ctx.config.flowchart.routing.max_steps.max(10_000);

    let cols = grid.cols;
    let rows = grid.rows;
    let states = (cols * rows * 4) as usize;
    let mut best_cost = vec![u32::MAX; states];
    let mut prev: Vec<Option<GridState>> = vec![None; states];
    let mut heap = BinaryHeap::new();

    for dir in 0..4u8 {
        let idx = ((start_iy * cols + start_ix) as usize) * 4 + dir as usize;
        best_cost[idx] = 0;
        heap.push(GridEntry {
            est: 0,
            cost: 0,
            state: GridState {
                x: start_ix,
                y: start_iy,
                dir,
            },
        });
    }

    let mut end_state: Option<GridState> = None;
    let mut steps = 0usize;

    while let Some(entry) = heap.pop() {
        steps += 1;
        if steps > max_steps {
            break;
        }
        let GridEntry { cost, state, .. } = entry;
        let state_idx = ((state.y * cols + state.x) as usize) * 4 + state.dir as usize;
        if cost != best_cost[state_idx] {
            continue;
        }
        if state.x == end_ix && state.y == end_iy {
            end_state = Some(state);
            break;
        }
        for (dir_idx, (dx, dy)) in dirs.iter().enumerate() {
            let nx = state.x + dx;
            let ny = state.y + dy;
            if nx < 0 || ny < 0 || nx >= cols || ny >= rows {
                continue;
            }
            if (nx != end_ix || ny != end_iy)
                && (nx != start_ix || ny != start_iy)
                && cell_blocked(grid, ctx.obstacles, nx, ny, ctx)
            {
                continue;
            }
            let mut next_cost = cost.saturating_add(step_cost);
            if state.dir != dir_idx as u8 {
                next_cost = next_cost.saturating_add(turn_penalty);
            }
            if let Some(occ) = occupancy {
                let (cx, cy) = grid.cell_center(nx, ny);
                let weight = occ.weight_at(cx, cy) as u32;
                if weight > 0 {
                    next_cost = next_cost.saturating_add(weight.saturating_mul(occupancy_weight));
                }
            }
            let next_idx = ((ny * cols + nx) as usize) * 4 + dir_idx;
            if next_cost >= best_cost[next_idx] {
                continue;
            }
            best_cost[next_idx] = next_cost;
            prev[next_idx] = Some(state);
            let manhattan = (nx - end_ix).unsigned_abs() + (ny - end_iy).unsigned_abs();
            let est = next_cost.saturating_add(manhattan.saturating_mul(step_cost));
            heap.push(GridEntry {
                est,
                cost: next_cost,
                state: GridState {
                    x: nx,
                    y: ny,
                    dir: dir_idx as u8,
                },
            });
        }
    }

    let end_state = end_state?;
    let mut cells: Vec<(i32, i32)> = Vec::new();
    let mut cur = end_state;
    loop {
        cells.push((cur.x, cur.y));
        let cur_idx = ((cur.y * cols + cur.x) as usize) * 4 + cur.dir as usize;
        if let Some(prev_state) = prev[cur_idx] {
            cur = prev_state;
        } else {
            break;
        }
    }
    cells.reverse();
    if cells.is_empty() {
        return None;
    }

    let mut points: Vec<(f32, f32)> = Vec::with_capacity(cells.len() + 4);
    points.push(start);
    if let Some((ix, iy)) = cells.first() {
        let (cx, cy) = grid.cell_center(*ix, *iy);
        match ctx.start_side {
            EdgeSide::Left | EdgeSide::Right => points.push((cx, start.1)),
            EdgeSide::Top | EdgeSide::Bottom => points.push((start.0, cy)),
        }
        points.push((cx, cy));
    }
    for &(ix, iy) in cells.iter().skip(1) {
        points.push(grid.cell_center(ix, iy));
    }
    if let Some((ix, iy)) = cells.last() {
        let (cx, cy) = grid.cell_center(*ix, *iy);
        match ctx.end_side {
            EdgeSide::Left | EdgeSide::Right => points.push((cx, end.1)),
            EdgeSide::Top | EdgeSide::Bottom => points.push((end.0, cy)),
        }
    }
    points.push(end);
    Some(compress_path(&points))
}

pub(super) fn push_route_candidate_metrics(
    points: Vec<(f32, f32)>,
    ctx: &RouteContext<'_>,
    existing_segments: &[((f32, f32), (f32, f32))],
    use_existing: bool,
    candidates: &mut Vec<Vec<(f32, f32)>>,
    intersections: &mut Vec<usize>,
    crossings: &mut Vec<usize>,
    label_hits: &mut Vec<usize>,
    overlaps: &mut Vec<f32>,
    via_distances: &mut Vec<f32>,
) {
    if points.len() < 2 || !path_coords_reasonable(&points) {
        return;
    }
    let hits = path_obstacle_intersections(&points, ctx.obstacles, ctx.from_id, ctx.to_id);
    let labels = path_label_intersections(&points, ctx.label_obstacles, ctx.preferred_label_id);
    let via_dist = ctx
        .preferred_label_center
        .map(|center| polyline_point_distance(&points, center))
        .unwrap_or(0.0);
    let (cross, overlap) = if use_existing {
        edge_crossings_with_existing(&points, existing_segments)
    } else {
        (0, 0.0)
    };
    candidates.push(points);
    intersections.push(hits);
    crossings.push(cross);
    label_hits.push(labels);
    overlaps.push(overlap);
    via_distances.push(via_dist);
}

pub(super) fn path_coords_reasonable(points: &[(f32, f32)]) -> bool {
    const LIMIT: f32 = 100_000.0;
    points
        .iter()
        .all(|(x, y)| x.is_finite() && y.is_finite() && x.abs() <= LIMIT && y.abs() <= LIMIT)
}

fn apply_endpoint_insets(
    mut path: Vec<(f32, f32)>,
    start_inset: f32,
    end_inset: f32,
) -> Vec<(f32, f32)> {
    if start_inset > 0.0 && path.len() >= 2 {
        let (sx, sy) = path[0];
        let (nx, ny) = path[1];
        let dx = sx - nx;
        let dy = sy - ny;
        let len = (dx * dx + dy * dy).sqrt();
        if len > start_inset {
            let r = start_inset / len;
            path[0] = (sx - dx * r, sy - dy * r);
        }
    }
    if end_inset > 0.0 && path.len() >= 2 {
        let n = path.len();
        let (px, py) = path[n - 2];
        let (ex, ey) = path[n - 1];
        let dx = ex - px;
        let dy = ey - py;
        let len = (dx * dx + dy * dy).sqrt();
        if len > end_inset {
            let r = end_inset / len;
            path[n - 1] = (ex - dx * r, ey - dy * r);
        }
    }
    path
}

fn point_segment_distance(a: (f32, f32), b: (f32, f32), p: (f32, f32)) -> f32 {
    let vx = b.0 - a.0;
    let vy = b.1 - a.1;
    let wx = p.0 - a.0;
    let wy = p.1 - a.1;
    let vv = vx * vx + vy * vy;
    if vv <= 1e-6 {
        let dx = p.0 - a.0;
        let dy = p.1 - a.1;
        return (dx * dx + dy * dy).sqrt();
    }
    let t = ((wx * vx + wy * vy) / vv).clamp(0.0, 1.0);
    let proj_x = a.0 + t * vx;
    let proj_y = a.1 + t * vy;
    let dx = p.0 - proj_x;
    let dy = p.1 - proj_y;
    (dx * dx + dy * dy).sqrt()
}

pub(super) fn polyline_point_distance(points: &[(f32, f32)], point: (f32, f32)) -> f32 {
    if points.is_empty() {
        return f32::INFINITY;
    }
    if points.len() == 1 {
        let dx = points[0].0 - point.0;
        let dy = points[0].1 - point.1;
        return (dx * dx + dy * dy).sqrt();
    }
    let mut best = f32::INFINITY;
    for segment in points.windows(2) {
        best = best.min(point_segment_distance(segment[0], segment[1], point));
    }
    best
}

fn enforce_preferred_label_via(points: &mut Vec<(f32, f32)>, ctx: &RouteContext<'_>) {
    let Some(via) = ctx.preferred_label_center else {
        return;
    };
    if points.len() < 2 {
        return;
    }
    if polyline_point_distance(points, via) <= 0.6 {
        return;
    }
    insert_label_via_point(points, via, ctx.direction);
}

pub(super) fn route_edge_with_avoidance(
    ctx: &RouteContext<'_>,
    occupancy: Option<&EdgeOccupancy>,
    grid: Option<&RoutingGrid>,
    existing: Option<&[Segment]>,
) -> Vec<(f32, f32)> {
    if ctx.from_id == ctx.to_id {
        let existing_segments = existing.unwrap_or(&[]);
        let use_existing = !existing_segments.is_empty();
        let mut candidates: Vec<Vec<(f32, f32)>> = Vec::new();
        let mut intersections: Vec<usize> = Vec::new();
        let mut crossings: Vec<usize> = Vec::new();
        let mut label_hits: Vec<usize> = Vec::new();
        let mut overlaps: Vec<f32> = Vec::new();
        let mut via_distances: Vec<f32> = Vec::new();

        let pad = ctx.config.node_spacing.max(ROUTING_PAD_MIN_SPACING) * ROUTING_PAD_RATIO;
        for points in route_self_loop_candidates(ctx.from, pad) {
            push_route_candidate_metrics(
                points,
                ctx,
                existing_segments,
                use_existing,
                &mut candidates,
                &mut intersections,
                &mut crossings,
                &mut label_hits,
                &mut overlaps,
                &mut via_distances,
            );
        }
        push_route_candidate_metrics(
            route_self_loop(ctx.from, ctx.direction, ctx.config),
            ctx,
            existing_segments,
            use_existing,
            &mut candidates,
            &mut intersections,
            &mut crossings,
            &mut label_hits,
            &mut overlaps,
            &mut via_distances,
        );

        if candidates.is_empty() {
            return route_self_loop(ctx.from, ctx.direction, ctx.config);
        }

        let mut best_idx = 0usize;
        let mut best_hits = usize::MAX;
        let mut best_cross = usize::MAX;
        let mut best_label_hits = usize::MAX;
        let mut best_overlap = f32::MAX;
        let mut best_via_dist = f32::MAX;
        let mut best_bends = usize::MAX;
        let mut best_len = f32::MAX;
        let mut best_score = u32::MAX;

        for (idx, points) in candidates.iter().enumerate() {
            let hits = intersections.get(idx).copied().unwrap_or(0);
            let cross = crossings.get(idx).copied().unwrap_or(0);
            let label = label_hits.get(idx).copied().unwrap_or(0);
            let overlap = overlaps.get(idx).copied().unwrap_or(0.0);
            let via_dist = via_distances.get(idx).copied().unwrap_or(f32::MAX);
            let bends = path_bend_count(points);
            let len = path_length(points);
            let score = occupancy.map(|grid| grid.score_path(points)).unwrap_or(0);
            let better = if ctx.prefer_shorter_ties {
                hits < best_hits
                    || (hits == best_hits && cross < best_cross)
                    || (hits == best_hits && cross == best_cross && label < best_label_hits)
                    || (hits == best_hits
                        && cross == best_cross
                        && label == best_label_hits
                        && overlap < best_overlap)
                    || (hits == best_hits
                        && cross == best_cross
                        && label == best_label_hits
                        && (overlap - best_overlap).abs() <= 1e-4
                        && via_dist + ROUTE_VIA_TIE_EPS < best_via_dist)
                    || (hits == best_hits
                        && cross == best_cross
                        && label == best_label_hits
                        && (overlap - best_overlap).abs() <= 1e-4
                        && (via_dist - best_via_dist).abs() <= ROUTE_VIA_TIE_EPS
                        && len + ROUTE_LENGTH_TIE_EPS < best_len)
                    || (hits == best_hits
                        && cross == best_cross
                        && label == best_label_hits
                        && (overlap - best_overlap).abs() <= 1e-4
                        && (via_dist - best_via_dist).abs() <= ROUTE_VIA_TIE_EPS
                        && (len - best_len).abs() <= ROUTE_LENGTH_TIE_EPS
                        && occupancy.is_some()
                        && score < best_score)
                    || (hits == best_hits
                        && cross == best_cross
                        && label == best_label_hits
                        && (overlap - best_overlap).abs() <= 1e-4
                        && (via_dist - best_via_dist).abs() <= ROUTE_VIA_TIE_EPS
                        && (len - best_len).abs() <= ROUTE_LENGTH_TIE_EPS
                        && (!occupancy.is_some() || score == best_score)
                        && bends < best_bends)
            } else {
                hits < best_hits
                    || (hits == best_hits && cross < best_cross)
                    || (hits == best_hits && cross == best_cross && label < best_label_hits)
                    || (hits == best_hits
                        && cross == best_cross
                        && label == best_label_hits
                        && overlap < best_overlap)
                    || (hits == best_hits
                        && cross == best_cross
                        && label == best_label_hits
                        && (overlap - best_overlap).abs() <= 1e-4
                        && via_dist + ROUTE_VIA_TIE_EPS < best_via_dist)
                    || (hits == best_hits
                        && cross == best_cross
                        && label == best_label_hits
                        && (overlap - best_overlap).abs() <= 1e-4
                        && (via_dist - best_via_dist).abs() <= ROUTE_VIA_TIE_EPS
                        && bends < best_bends)
                    || (hits == best_hits
                        && cross == best_cross
                        && label == best_label_hits
                        && (overlap - best_overlap).abs() <= 1e-4
                        && (via_dist - best_via_dist).abs() <= ROUTE_VIA_TIE_EPS
                        && bends == best_bends
                        && occupancy.is_some()
                        && score < best_score)
                    || (hits == best_hits
                        && cross == best_cross
                        && label == best_label_hits
                        && (overlap - best_overlap).abs() <= 1e-4
                        && (via_dist - best_via_dist).abs() <= ROUTE_VIA_TIE_EPS
                        && bends == best_bends
                        && (!occupancy.is_some() || score == best_score)
                        && len < best_len)
            };
            if better {
                best_hits = hits;
                best_cross = cross;
                best_label_hits = label;
                best_overlap = overlap;
                best_via_dist = via_dist;
                best_bends = bends;
                best_len = len;
                best_score = score;
                best_idx = idx;
            }
        }

        let mut best = compress_path(&candidates.swap_remove(best_idx));
        enforce_preferred_label_via(&mut best, ctx);
        return apply_endpoint_insets(compress_path(&best), ctx.start_inset, ctx.end_inset);
    }

    let (_, _, is_backward) = edge_sides(ctx.from, ctx.to, ctx.direction);

    // Anchor edges using resolved port offsets to reduce overlap
    let start = anchor_point_for_node(ctx.from, ctx.start_side, ctx.start_offset);
    let end = anchor_point_for_node(ctx.to, ctx.end_side, ctx.end_offset);
    let stub_len = ctx.stub_len;
    let mut route_start = port_stub_point(start, ctx.start_side, stub_len);
    let mut route_end = port_stub_point(end, ctx.end_side, stub_len);
    let stub_hits_node = |a: (f32, f32), b: (f32, f32)| {
        ctx.obstacles.iter().any(|obstacle| {
            if obstacle.members.is_some() {
                return false;
            }
            if obstacle.id == ctx.from_id || obstacle.id == ctx.to_id {
                return false;
            }
            segment_intersects_rect(a, b, obstacle)
        })
    };
    if ctx.obstacles.len() <= 10 {
        if stub_hits_node(start, route_start) {
            route_start = start;
        }
        if stub_hits_node(route_end, end) {
            route_end = end;
        }
    }
    if ctx.fast_route {
        let mut fast = compress_path(&[start, route_start, route_end, end]);
        enforce_preferred_label_via(&mut fast, ctx);
        return apply_endpoint_insets(compress_path(&fast), ctx.start_inset, ctx.end_inset);
    }
    let mut candidates: Vec<Vec<(f32, f32)>> = Vec::new();
    let mut intersections: Vec<usize> = Vec::new();
    let mut crossings: Vec<usize> = Vec::new();
    let mut label_hits: Vec<usize> = Vec::new();
    let mut overlaps: Vec<f32> = Vec::new();
    let mut via_distances: Vec<f32> = Vec::new();
    let existing_segments = existing.unwrap_or(&[]);
    let use_existing = !existing_segments.is_empty();

    // For backward edges, try routing around obstacles (both left and right)
    if is_backward {
        let pad = ctx.config.node_spacing.max(30.0);
        // Find the extents of any obstacle that blocks the direct path
        let mut min_left = f32::MAX;
        let mut max_right = 0.0f32;
        let mut min_top = f32::MAX;
        let mut max_bottom = 0.0f32;
        for obstacle in ctx.obstacles {
            if obstacle.id == ctx.from_id || obstacle.id == ctx.to_id {
                continue;
            }
            if let Some(members) = &obstacle.members
                && (members.contains(ctx.from_id) || members.contains(ctx.to_id))
            {
                continue;
            }
            // Check if obstacle vertically overlaps the edge path
            let obs_top = obstacle.y;
            let obs_bottom = obstacle.y + obstacle.height;
            let path_top = end.1.min(start.1);
            let path_bottom = start.1.max(end.1);
            if obs_top < path_bottom && obs_bottom > path_top {
                min_left = min_left.min(obstacle.x);
                max_right = max_right.max(obstacle.x + obstacle.width);
            }
            // Check if obstacle horizontally overlaps the edge span
            let obs_left = obstacle.x;
            let obs_right = obstacle.x + obstacle.width;
            let path_left = start.0.min(end.0);
            let path_right = start.0.max(end.0);
            if obs_left < path_right && obs_right > path_left {
                min_top = min_top.min(obs_top);
                max_bottom = max_bottom.max(obs_bottom);
            }
        }

        // Try routing around the right side first
        if max_right > 0.0 {
            let route_x = max_right + pad;
            let points = vec![
                route_start,
                (route_x, route_start.1),
                (route_x, route_end.1),
                route_end,
            ];
            push_route_candidate_metrics(
                points,
                ctx,
                existing_segments,
                use_existing,
                &mut candidates,
                &mut intersections,
                &mut crossings,
                &mut label_hits,
                &mut overlaps,
                &mut via_distances,
            );
        }

        // Try routing under all blocking obstacles
        if max_bottom > 0.0 {
            let route_y = max_bottom + pad;
            let points = vec![
                route_start,
                (route_start.0, route_y),
                (route_end.0, route_y),
                route_end,
            ];
            push_route_candidate_metrics(
                points,
                ctx,
                existing_segments,
                use_existing,
                &mut candidates,
                &mut intersections,
                &mut crossings,
                &mut label_hits,
                &mut overlaps,
                &mut via_distances,
            );
        }

        // Try routing above all blocking obstacles
        if min_top < f32::MAX {
            let route_y = min_top - pad;
            let points = vec![
                route_start,
                (route_start.0, route_y),
                (route_end.0, route_y),
                route_end,
            ];
            push_route_candidate_metrics(
                points,
                ctx,
                existing_segments,
                use_existing,
                &mut candidates,
                &mut intersections,
                &mut crossings,
                &mut label_hits,
                &mut overlaps,
                &mut via_distances,
            );
        }

        // Try routing around the left side
        if min_left < f32::MAX {
            let route_x = min_left - pad;
            let points = vec![
                route_start,
                (route_x, route_start.1),
                (route_x, route_end.1),
                route_end,
            ];
            push_route_candidate_metrics(
                points,
                ctx,
                existing_segments,
                use_existing,
                &mut candidates,
                &mut intersections,
                &mut crossings,
                &mut label_hits,
                &mut overlaps,
                &mut via_distances,
            );
        }
    }

    // Check if a direct line is possible (no obstacles in the way)
    let direct_path = vec![route_start, route_end];
    push_route_candidate_metrics(
        direct_path,
        ctx,
        existing_segments,
        use_existing,
        &mut candidates,
        &mut intersections,
        &mut crossings,
        &mut label_hits,
        &mut overlaps,
        &mut via_distances,
    );

    if let Some(via) = ctx.preferred_label_center {
        let via_mid_x = via.0;
        let via_mid_y = via.1;
        let through_vertical = vec![
            route_start,
            (via_mid_x, route_start.1),
            (via_mid_x, via_mid_y),
            (via_mid_x, route_end.1),
            route_end,
        ];
        push_route_candidate_metrics(
            through_vertical,
            ctx,
            existing_segments,
            use_existing,
            &mut candidates,
            &mut intersections,
            &mut crossings,
            &mut label_hits,
            &mut overlaps,
            &mut via_distances,
        );

        let through_horizontal = vec![
            route_start,
            (route_start.0, via_mid_y),
            (via_mid_x, via_mid_y),
            (route_end.0, via_mid_y),
            route_end,
        ];
        push_route_candidate_metrics(
            through_horizontal,
            ctx,
            existing_segments,
            use_existing,
            &mut candidates,
            &mut intersections,
            &mut crossings,
            &mut label_hits,
            &mut overlaps,
            &mut via_distances,
        );
    }

    // Fall back to orthogonal routing with control points
    let step = ctx.config.node_spacing.max(ORTHO_STEP_MIN_SPACING) * ROUTING_PAD_RATIO;
    let mut offsets = vec![ctx.base_offset];
    for i in 1..=6 {
        let delta = step * i as f32;
        offsets.push(ctx.base_offset + delta);
        offsets.push(ctx.base_offset - delta);
    }

    let cross_axis_delta = if is_horizontal(ctx.direction) {
        (route_end.1 - route_start.1).abs()
    } else {
        (route_end.0 - route_start.0).abs()
    };
    let use_channel_candidates = (cross_axis_delta > step * CHANNEL_CANDIDATE_RATIO
        && ctx.obstacles.len() > 10)
        || is_backward
        || (ctx.start_side == ctx.end_side && ctx.obstacles.len() > 4);

    for (offset_rank, offset) in offsets.iter().copied().enumerate() {
        if is_horizontal(ctx.direction) {
            let mid_x = (route_start.0 + route_end.0) / 2.0 + offset;
            let points = vec![
                route_start,
                (mid_x, route_start.1),
                (mid_x, route_end.1),
                route_end,
            ];
            push_route_candidate_metrics(
                points,
                ctx,
                existing_segments,
                use_existing,
                &mut candidates,
                &mut intersections,
                &mut crossings,
                &mut label_hits,
                &mut overlaps,
                &mut via_distances,
            );

            let mid_y = (route_start.1 + route_end.1) / 2.0 + offset;
            let alt = vec![
                route_start,
                (route_start.0, mid_y),
                (route_end.0, mid_y),
                route_end,
            ];
            push_route_candidate_metrics(
                alt,
                ctx,
                existing_segments,
                use_existing,
                &mut candidates,
                &mut intersections,
                &mut crossings,
                &mut label_hits,
                &mut overlaps,
                &mut via_distances,
            );

            if use_channel_candidates && offset_rank <= 3 {
                let near_start_x = route_start.0 + offset;
                let near_start = vec![
                    route_start,
                    (near_start_x, route_start.1),
                    (near_start_x, route_end.1),
                    route_end,
                ];
                push_route_candidate_metrics(
                    near_start,
                    ctx,
                    existing_segments,
                    use_existing,
                    &mut candidates,
                    &mut intersections,
                    &mut crossings,
                    &mut label_hits,
                    &mut overlaps,
                    &mut via_distances,
                );

                let near_end_x = route_end.0 + offset;
                let near_end = vec![
                    route_start,
                    (near_end_x, route_start.1),
                    (near_end_x, route_end.1),
                    route_end,
                ];
                push_route_candidate_metrics(
                    near_end,
                    ctx,
                    existing_segments,
                    use_existing,
                    &mut candidates,
                    &mut intersections,
                    &mut crossings,
                    &mut label_hits,
                    &mut overlaps,
                    &mut via_distances,
                );
            }
        } else {
            let mid_y = (route_start.1 + route_end.1) / 2.0 + offset;
            let points = vec![
                route_start,
                (route_start.0, mid_y),
                (route_end.0, mid_y),
                route_end,
            ];
            push_route_candidate_metrics(
                points,
                ctx,
                existing_segments,
                use_existing,
                &mut candidates,
                &mut intersections,
                &mut crossings,
                &mut label_hits,
                &mut overlaps,
                &mut via_distances,
            );

            let mid_x = (route_start.0 + route_end.0) / 2.0 + offset;
            let alt = vec![
                route_start,
                (mid_x, route_start.1),
                (mid_x, route_end.1),
                route_end,
            ];
            push_route_candidate_metrics(
                alt,
                ctx,
                existing_segments,
                use_existing,
                &mut candidates,
                &mut intersections,
                &mut crossings,
                &mut label_hits,
                &mut overlaps,
                &mut via_distances,
            );

            if use_channel_candidates && offset_rank <= 3 {
                let near_start_y = route_start.1 + offset;
                let near_start = vec![
                    route_start,
                    (route_start.0, near_start_y),
                    (route_end.0, near_start_y),
                    route_end,
                ];
                push_route_candidate_metrics(
                    near_start,
                    ctx,
                    existing_segments,
                    use_existing,
                    &mut candidates,
                    &mut intersections,
                    &mut crossings,
                    &mut label_hits,
                    &mut overlaps,
                    &mut via_distances,
                );

                let near_end_y = route_end.1 + offset;
                let near_end = vec![
                    route_start,
                    (route_start.0, near_end_y),
                    (route_end.0, near_end_y),
                    route_end,
                ];
                push_route_candidate_metrics(
                    near_end,
                    ctx,
                    existing_segments,
                    use_existing,
                    &mut candidates,
                    &mut intersections,
                    &mut crossings,
                    &mut label_hits,
                    &mut overlaps,
                    &mut via_distances,
                );
            }
        }
    }

    let min_hits = intersections.iter().copied().min().unwrap_or(0);
    let min_crossings = crossings.iter().copied().min().unwrap_or(0);
    let min_label_hits = label_hits.iter().copied().min().unwrap_or(0);
    let min_overlap = overlaps.iter().copied().fold(f32::INFINITY, f32::min);
    let mut needs_detour = min_crossings > 0
        || min_label_hits > 0
        || (min_overlap.is_finite() && min_overlap >= OVERLAP_DETOUR_MIN);
    if min_hits == 0
        && let Some(occ) = occupancy
    {
        let mut best_idx = 0usize;
        let mut best_score = u32::MAX;
        let mut best_bends = usize::MAX;
        let mut best_len = f32::MAX;
        for (idx, points) in candidates.iter().enumerate() {
            let score = occ.score_path(points);
            let bends = path_bend_count(points);
            let len = path_length(points);
            let better = if ctx.prefer_shorter_ties {
                score < best_score
                    || (score == best_score && len + ROUTE_LENGTH_TIE_EPS < best_len)
                    || (score == best_score
                        && (len - best_len).abs() <= ROUTE_LENGTH_TIE_EPS
                        && bends < best_bends)
            } else {
                score < best_score
                    || (score == best_score && bends < best_bends)
                    || (score == best_score && bends == best_bends && len < best_len)
            };
            if better {
                best_score = score;
                best_bends = bends;
                best_len = len;
                best_idx = idx;
            }
        }
        if let Some(points) = candidates.get(best_idx) {
            let overlap = occ.overlap_count(points);
            let path_len = path_length(points);
            let overlap_trigger = ((path_len / occ.cell) * OVERLAP_TRIGGER_RATIO)
                .max(OVERLAP_TRIGGER_MIN)
                .ceil() as u32;
            if overlap >= overlap_trigger {
                needs_detour = true;
            }
        }
    }

    if min_hits > 0 || needs_detour {
        for i in 7..=9 {
            let delta = step * i as f32;
            for sign in [1.0, -1.0] {
                let offset = ctx.base_offset + sign * delta;
                let points = if is_horizontal(ctx.direction) {
                    let mid_x = (route_start.0 + route_end.0) / 2.0 + offset;
                    vec![
                        route_start,
                        (mid_x, route_start.1),
                        (mid_x, route_end.1),
                        route_end,
                    ]
                } else {
                    let mid_y = (route_start.1 + route_end.1) / 2.0 + offset;
                    vec![
                        route_start,
                        (route_start.0, mid_y),
                        (route_end.0, mid_y),
                        route_end,
                    ]
                };
                push_route_candidate_metrics(
                    points,
                    ctx,
                    existing_segments,
                    use_existing,
                    &mut candidates,
                    &mut intersections,
                    &mut crossings,
                    &mut label_hits,
                    &mut overlaps,
                    &mut via_distances,
                );
            }
        }
    }

    let min_hits = intersections.iter().copied().min().unwrap_or(0);
    if (min_hits > 0 || needs_detour)
        && let Some(grid) = grid
        && let Some(points) = route_edge_with_grid(ctx, grid, occupancy, route_start, route_end)
    {
        push_route_candidate_metrics(
            points,
            ctx,
            existing_segments,
            use_existing,
            &mut candidates,
            &mut intersections,
            &mut crossings,
            &mut label_hits,
            &mut overlaps,
            &mut via_distances,
        );
    }

    if let Some(grid) = occupancy {
        let mut best_idx = 0usize;
        let mut best_hits = usize::MAX;
        let mut best_cross = usize::MAX;
        let mut best_label_hits = usize::MAX;
        let mut best_overlap = f32::MAX;
        let mut best_via_dist = f32::MAX;
        let mut best_bends = usize::MAX;
        let mut best_score = u32::MAX;
        let mut best_len = f32::MAX;
        for (idx, points) in candidates.iter().enumerate() {
            let hits = intersections.get(idx).copied().unwrap_or(0);
            let cross = crossings.get(idx).copied().unwrap_or(0);
            let label = label_hits.get(idx).copied().unwrap_or(0);
            let overlap = overlaps.get(idx).copied().unwrap_or(0.0);
            let via_dist = via_distances.get(idx).copied().unwrap_or(f32::MAX);
            let bends = path_bend_count(points);
            let score = grid.score_path(points);
            let len = path_length(points);
            let better = if ctx.prefer_shorter_ties {
                hits < best_hits
                    || (hits == best_hits && cross < best_cross)
                    || (hits == best_hits && cross == best_cross && label < best_label_hits)
                    || (hits == best_hits
                        && cross == best_cross
                        && label == best_label_hits
                        && overlap < best_overlap)
                    || (hits == best_hits
                        && cross == best_cross
                        && label == best_label_hits
                        && (overlap - best_overlap).abs() <= 1e-4
                        && via_dist + ROUTE_VIA_TIE_EPS < best_via_dist)
                    || (hits == best_hits
                        && cross == best_cross
                        && label == best_label_hits
                        && (overlap - best_overlap).abs() <= 1e-4
                        && (via_dist - best_via_dist).abs() <= ROUTE_VIA_TIE_EPS
                        && len + ROUTE_LENGTH_TIE_EPS < best_len)
                    || (hits == best_hits
                        && cross == best_cross
                        && label == best_label_hits
                        && (overlap - best_overlap).abs() <= 1e-4
                        && (via_dist - best_via_dist).abs() <= ROUTE_VIA_TIE_EPS
                        && (len - best_len).abs() <= ROUTE_LENGTH_TIE_EPS
                        && score < best_score)
                    || (hits == best_hits
                        && cross == best_cross
                        && label == best_label_hits
                        && (overlap - best_overlap).abs() <= 1e-4
                        && (via_dist - best_via_dist).abs() <= ROUTE_VIA_TIE_EPS
                        && (len - best_len).abs() <= ROUTE_LENGTH_TIE_EPS
                        && score == best_score
                        && bends < best_bends)
            } else {
                hits < best_hits
                    || (hits == best_hits && cross < best_cross)
                    || (hits == best_hits && cross == best_cross && label < best_label_hits)
                    || (hits == best_hits
                        && cross == best_cross
                        && label == best_label_hits
                        && overlap < best_overlap)
                    || (hits == best_hits
                        && cross == best_cross
                        && label == best_label_hits
                        && (overlap - best_overlap).abs() <= 1e-4
                        && via_dist + ROUTE_VIA_TIE_EPS < best_via_dist)
                    || (hits == best_hits
                        && cross == best_cross
                        && label == best_label_hits
                        && (overlap - best_overlap).abs() <= 1e-4
                        && (via_dist - best_via_dist).abs() <= ROUTE_VIA_TIE_EPS
                        && bends < best_bends)
                    || (hits == best_hits
                        && cross == best_cross
                        && label == best_label_hits
                        && (overlap - best_overlap).abs() <= 1e-4
                        && (via_dist - best_via_dist).abs() <= ROUTE_VIA_TIE_EPS
                        && bends == best_bends
                        && score < best_score)
                    || (hits == best_hits
                        && cross == best_cross
                        && label == best_label_hits
                        && (overlap - best_overlap).abs() <= 1e-4
                        && (via_dist - best_via_dist).abs() <= ROUTE_VIA_TIE_EPS
                        && bends == best_bends
                        && score == best_score
                        && len < best_len)
            };
            if better {
                best_hits = hits;
                best_cross = cross;
                best_label_hits = label;
                best_overlap = overlap;
                best_via_dist = via_dist;
                best_bends = bends;
                best_score = score;
                best_len = len;
                best_idx = idx;
            }
        }
        let mut combined = Vec::with_capacity(candidates[best_idx].len() + 2);
        combined.push(start);
        combined.extend(candidates.swap_remove(best_idx));
        combined.push(end);
        enforce_preferred_label_via(&mut combined, ctx);
        return apply_endpoint_insets(compress_path(&combined), ctx.start_inset, ctx.end_inset);
    }

    let mut best_idx = 0usize;
    let mut best_hits = usize::MAX;
    let mut best_cross = usize::MAX;
    let mut best_label_hits = usize::MAX;
    let mut best_overlap = f32::MAX;
    let mut best_via_dist = f32::MAX;
    let mut best_bends = usize::MAX;
    let mut best_len = f32::MAX;
    for (idx, points) in candidates.iter().enumerate() {
        let hits = intersections.get(idx).copied().unwrap_or(0);
        let cross = crossings.get(idx).copied().unwrap_or(0);
        let label = label_hits.get(idx).copied().unwrap_or(0);
        let overlap = overlaps.get(idx).copied().unwrap_or(0.0);
        let via_dist = via_distances.get(idx).copied().unwrap_or(f32::MAX);
        let bends = path_bend_count(points);
        let len = path_length(points);
        let better = if ctx.prefer_shorter_ties {
            hits < best_hits
                || (hits == best_hits && cross < best_cross)
                || (hits == best_hits && cross == best_cross && label < best_label_hits)
                || (hits == best_hits
                    && cross == best_cross
                    && label == best_label_hits
                    && overlap < best_overlap)
                || (hits == best_hits
                    && cross == best_cross
                    && label == best_label_hits
                    && (overlap - best_overlap).abs() <= 1e-4
                    && via_dist + ROUTE_VIA_TIE_EPS < best_via_dist)
                || (hits == best_hits
                    && cross == best_cross
                    && label == best_label_hits
                    && (overlap - best_overlap).abs() <= 1e-4
                    && (via_dist - best_via_dist).abs() <= ROUTE_VIA_TIE_EPS
                    && len + ROUTE_LENGTH_TIE_EPS < best_len)
                || (hits == best_hits
                    && cross == best_cross
                    && label == best_label_hits
                    && (overlap - best_overlap).abs() <= 1e-4
                    && (via_dist - best_via_dist).abs() <= ROUTE_VIA_TIE_EPS
                    && (len - best_len).abs() <= ROUTE_LENGTH_TIE_EPS
                    && bends < best_bends)
        } else {
            hits < best_hits
                || (hits == best_hits && cross < best_cross)
                || (hits == best_hits && cross == best_cross && label < best_label_hits)
                || (hits == best_hits
                    && cross == best_cross
                    && label == best_label_hits
                    && overlap < best_overlap)
                || (hits == best_hits
                    && cross == best_cross
                    && label == best_label_hits
                    && (overlap - best_overlap).abs() <= 1e-4
                    && via_dist + ROUTE_VIA_TIE_EPS < best_via_dist)
                || (hits == best_hits
                    && cross == best_cross
                    && label == best_label_hits
                    && (overlap - best_overlap).abs() <= 1e-4
                    && (via_dist - best_via_dist).abs() <= ROUTE_VIA_TIE_EPS
                    && bends < best_bends)
                || (hits == best_hits
                    && cross == best_cross
                    && label == best_label_hits
                    && (overlap - best_overlap).abs() <= 1e-4
                    && (via_dist - best_via_dist).abs() <= ROUTE_VIA_TIE_EPS
                    && bends == best_bends
                    && len < best_len)
        };
        if better {
            best_hits = hits;
            best_cross = cross;
            best_label_hits = label;
            best_overlap = overlap;
            best_via_dist = via_dist;
            best_bends = bends;
            best_len = len;
            best_idx = idx;
        }
    }
    let mut combined = Vec::with_capacity(candidates[best_idx].len() + 2);
    combined.push(start);
    combined.extend(candidates.swap_remove(best_idx));
    combined.push(end);
    enforce_preferred_label_via(&mut combined, ctx);
    apply_endpoint_insets(compress_path(&combined), ctx.start_inset, ctx.end_inset)
}

pub(super) fn path_obstacle_intersections(
    points: &[(f32, f32)],
    obstacles: &[Obstacle],
    from_id: &str,
    to_id: &str,
) -> usize {
    if points.len() < 2 {
        return 0;
    }
    let mut count = 0usize;
    for segment in points.windows(2) {
        let (a, b) = (segment[0], segment[1]);
        for obstacle in obstacles {
            if obstacle.id == from_id || obstacle.id == to_id {
                continue;
            }
            if let Some(members) = &obstacle.members
                && (members.contains(from_id) || members.contains(to_id))
            {
                continue;
            }
            if segment_intersects_rect(a, b, obstacle) {
                count += 1;
            }
        }
    }
    count
}

pub(super) fn path_label_intersections(
    points: &[(f32, f32)],
    label_obstacles: &[Obstacle],
    ignore_label_id: Option<&str>,
) -> usize {
    if points.len() < 2 || label_obstacles.is_empty() {
        return 0;
    }
    let mut count = 0usize;
    for segment in points.windows(2) {
        let (a, b) = (segment[0], segment[1]);
        for obstacle in label_obstacles {
            if ignore_label_id.is_some_and(|id| id == obstacle.id) {
                continue;
            }
            if segment_intersects_rect(a, b, obstacle) {
                count += 1;
            }
        }
    }
    count
}

pub(super) fn path_length(points: &[(f32, f32)]) -> f32 {
    let mut length = 0.0;
    for segment in points.windows(2) {
        let dx = segment[1].0 - segment[0].0;
        let dy = segment[1].1 - segment[0].1;
        length += (dx * dx + dy * dy).sqrt();
    }
    length
}

pub(super) fn path_point_at_progress(points: &[(f32, f32)], progress: f32) -> Option<(f32, f32)> {
    if points.len() < 2 {
        return None;
    }
    let total = path_length(points);
    if !total.is_finite() || total <= 1e-6 {
        return Some(points[0]);
    }
    let mut remain = total * progress.clamp(0.0, 1.0);
    for segment in points.windows(2) {
        let a = segment[0];
        let b = segment[1];
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        let seg_len = (dx * dx + dy * dy).sqrt();
        if seg_len <= 1e-6 {
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

pub(super) fn edge_label_anchor_from_points(points: &[(f32, f32)]) -> Option<(f32, f32)> {
    // Center labels should stay on the geometric midpoint of the routed path
    // (arc-length progress 0.5), not merely the midpoint of the longest run.
    path_point_at_progress(points, 0.5)
}

pub(super) fn route_self_loop(
    node: &NodeLayout,
    direction: Direction,
    config: &LayoutConfig,
) -> Vec<(f32, f32)> {
    let pad = config.node_spacing.max(ROUTING_PAD_MIN_SPACING) * ROUTING_PAD_RATIO;
    if is_horizontal(direction) {
        let start = (node.x + node.width, node.y + node.height / 2.0);
        let p1 = (node.x + node.width + pad, node.y + node.height / 2.0);
        let p2 = (node.x + node.width + pad, node.y - pad);
        let p3 = (node.x + node.width / 2.0, node.y - pad);
        let end = (node.x + node.width / 2.0, node.y);
        vec![start, p1, p2, p3, end]
    } else {
        let start = (node.x + node.width / 2.0, node.y + node.height);
        let p1 = (node.x + node.width / 2.0, node.y + node.height + pad);
        let p2 = (node.x + node.width + pad, node.y + node.height + pad);
        let p3 = (node.x + node.width + pad, node.y + node.height / 2.0);
        let end = (node.x + node.width, node.y + node.height / 2.0);
        vec![start, p1, p2, p3, end]
    }
}

pub(super) fn route_self_loop_candidates(node: &NodeLayout, pad: f32) -> Vec<Vec<(f32, f32)>> {
    let x = node.x;
    let y = node.y;
    let w = node.width;
    let h = node.height;
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let left = (x, cy);
    let right = (x + w, cy);
    let top = (cx, y);
    let bottom = (cx, y + h);
    let left_x = x - pad;
    let right_x = x + w + pad;
    let top_y = y - pad;
    let bottom_y = y + h + pad;

    vec![
        // Right-side loops
        vec![right, (right_x, cy), (right_x, top_y), (cx, top_y), top],
        vec![
            right,
            (right_x, cy),
            (right_x, bottom_y),
            (cx, bottom_y),
            bottom,
        ],
        // Left-side loops
        vec![left, (left_x, cy), (left_x, top_y), (cx, top_y), top],
        vec![
            left,
            (left_x, cy),
            (left_x, bottom_y),
            (cx, bottom_y),
            bottom,
        ],
        // Top-side loops
        vec![top, (cx, top_y), (right_x, top_y), (right_x, cy), right],
        vec![top, (cx, top_y), (left_x, top_y), (left_x, cy), left],
        // Bottom-side loops
        vec![
            bottom,
            (cx, bottom_y),
            (right_x, bottom_y),
            (right_x, cy),
            right,
        ],
        vec![
            bottom,
            (cx, bottom_y),
            (left_x, bottom_y),
            (left_x, cy),
            left,
        ],
    ]
}

pub(super) fn build_obstacles(
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
    config: &LayoutConfig,
) -> Vec<Obstacle> {
    let mut obstacles = Vec::new();
    let pad = (config.node_spacing * OBSTACLE_PAD_RATIO).max(OBSTACLE_PAD_MIN);
    for node in nodes.values() {
        if node.hidden {
            continue;
        }
        if node.anchor_subgraph.is_some() {
            continue;
        }
        obstacles.push(Obstacle {
            id: node.id.clone(),
            x: node.x - pad,
            y: node.y - pad,
            width: node.width + pad * 2.0,
            height: node.height + pad * 2.0,
            members: None,
        });
    }

    for (idx, sub) in subgraphs.iter().enumerate() {
        let invisible_region = sub.label.trim().is_empty()
            && sub.style.stroke.as_deref() == Some("none")
            && sub.style.fill.as_deref() == Some("none");
        if invisible_region {
            continue;
        }
        let mut members: HashSet<String> = sub.nodes.iter().cloned().collect();
        for node in nodes.values() {
            if node.anchor_subgraph == Some(idx) {
                members.insert(node.id.clone());
            }
        }
        obstacles.push(Obstacle {
            id: format!("subgraph:{}", sub.label),
            x: sub.x - pad,
            y: sub.y - pad,
            width: sub.width + pad * 2.0,
            height: sub.height + pad * 2.0,
            members: Some(members),
        });
    }
    obstacles
}

pub(super) fn build_label_obstacles_for_routing(
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
) -> Vec<Obstacle> {
    let mut obstacles = Vec::new();

    let node_pad = LABEL_OBSTACLE_NODE_PAD;
    for node in nodes.values() {
        if node.hidden || node.anchor_subgraph.is_some() {
            continue;
        }
        if node.label.width <= 0.0
            || node.label.height <= 0.0
            || node.label.lines.iter().all(|line| line.trim().is_empty())
        {
            continue;
        }
        let x = node.x + (node.width - node.label.width) / 2.0 - node_pad;
        let y = node.y + (node.height - node.label.height) / 2.0 - node_pad;
        obstacles.push(Obstacle {
            id: format!("node-label:{}", node.id),
            x,
            y,
            width: node.label.width + node_pad * 2.0,
            height: node.label.height + node_pad * 2.0,
            members: None,
        });
    }

    let sub_pad = LABEL_OBSTACLE_SUB_PAD;
    for sub in subgraphs {
        if sub.label.trim().is_empty()
            || sub.label_block.width <= 0.0
            || sub.label_block.height <= 0.0
        {
            continue;
        }
        // Approximate the header label box as rendered in flowchart/subgraph mode.
        let x = sub.x + 12.0 - sub_pad;
        let y = sub.y + 6.0 - sub_pad;
        obstacles.push(Obstacle {
            id: format!("subgraph-label:{}", sub.label),
            x,
            y,
            width: sub.label_block.width + sub_pad * 2.0,
            height: sub.label_block.height + sub_pad * 2.0,
            members: None,
        });
    }

    obstacles
}

pub(super) fn edge_pair_key(edge: &crate::ir::Edge) -> (String, String) {
    if edge.from <= edge.to {
        (edge.from.clone(), edge.to.clone())
    } else {
        (edge.to.clone(), edge.from.clone())
    }
}

pub(super) fn build_edge_pair_counts(
    edges: &[crate::ir::Edge],
) -> HashMap<(String, String), usize> {
    let mut counts: HashMap<(String, String), usize> = HashMap::new();
    for edge in edges {
        let key = edge_pair_key(edge);
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
}

pub(super) fn segment_intersects_rect(a: (f32, f32), b: (f32, f32), rect: &Obstacle) -> bool {
    let (x1, y1) = a;
    let (x2, y2) = b;
    let min_x = x1.min(x2);
    let max_x = x1.max(x2);
    let min_y = y1.min(y2);
    let max_y = y1.max(y2);
    if max_x < rect.x
        || min_x > rect.x + rect.width
        || max_y < rect.y
        || min_y > rect.y + rect.height
    {
        return false;
    }
    if x1 >= rect.x && x1 <= rect.x + rect.width && y1 >= rect.y && y1 <= rect.y + rect.height {
        return true;
    }
    if x2 >= rect.x && x2 <= rect.x + rect.width && y2 >= rect.y && y2 <= rect.y + rect.height {
        return true;
    }
    let corners = [
        (rect.x, rect.y),
        (rect.x + rect.width, rect.y),
        (rect.x + rect.width, rect.y + rect.height),
        (rect.x, rect.y + rect.height),
    ];
    let edges = [
        (corners[0], corners[1]),
        (corners[1], corners[2]),
        (corners[2], corners[3]),
        (corners[3], corners[0]),
    ];
    for (c, d) in edges {
        if segments_intersect(a, b, c, d) {
            return true;
        }
    }
    false
}

pub(super) type Segment = ((f32, f32), (f32, f32));

pub(super) fn segments_intersect(
    a: (f32, f32),
    b: (f32, f32),
    c: (f32, f32),
    d: (f32, f32),
) -> bool {
    fn orient(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
        (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
    }
    fn on_segment(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
        let min_x = a.0.min(b.0);
        let max_x = a.0.max(b.0);
        let min_y = a.1.min(b.1);
        let max_y = a.1.max(b.1);
        c.0 >= min_x - 1e-6 && c.0 <= max_x + 1e-6 && c.1 >= min_y - 1e-6 && c.1 <= max_y + 1e-6
    }
    let o1 = orient(a, b, c);
    let o2 = orient(a, b, d);
    let o3 = orient(c, d, a);
    let o4 = orient(c, d, b);
    if (o1 > 0.0 && o2 < 0.0 || o1 < 0.0 && o2 > 0.0)
        && (o3 > 0.0 && o4 < 0.0 || o3 < 0.0 && o4 > 0.0)
    {
        return true;
    }
    if o1.abs() <= 1e-6 && on_segment(a, b, c) {
        return true;
    }
    if o2.abs() <= 1e-6 && on_segment(a, b, d) {
        return true;
    }
    if o3.abs() <= 1e-6 && on_segment(c, d, a) {
        return true;
    }
    if o4.abs() <= 1e-6 && on_segment(c, d, b) {
        return true;
    }
    false
}

pub(super) fn collinear_overlap_length(
    a: (f32, f32),
    b: (f32, f32),
    c: (f32, f32),
    d: (f32, f32),
) -> f32 {
    let cross1 = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
    let cross2 = (b.0 - a.0) * (d.1 - a.1) - (b.1 - a.1) * (d.0 - a.0);
    if cross1.abs() > 1e-6 || cross2.abs() > 1e-6 {
        return 0.0;
    }
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let seg_len_sq = dx * dx + dy * dy;
    if seg_len_sq < 1e-6 {
        return 0.0;
    }
    let proj = |p: (f32, f32)| ((p.0 - a.0) * dx + (p.1 - a.1) * dy) / seg_len_sq;
    let t1 = proj(c);
    let t2 = proj(d);
    let tmin = t1.min(t2);
    let tmax = t1.max(t2);
    let overlap = (tmax.min(1.0) - tmin.max(0.0)).max(0.0);
    overlap * seg_len_sq.sqrt()
}

pub(super) fn edge_crossings_with_existing(
    points: &[(f32, f32)],
    existing: &[Segment],
) -> (usize, f32) {
    if points.len() < 2 || existing.is_empty() {
        return (0, 0.0);
    }
    let mut crossings = 0usize;
    let mut overlap = 0.0f32;
    for segment in points.windows(2) {
        let a1 = segment[0];
        let a2 = segment[1];
        for &(b1, b2) in existing {
            if (a1.0 - b1.0).abs() < 1e-6 && (a1.1 - b1.1).abs() < 1e-6
                || (a1.0 - b2.0).abs() < 1e-6 && (a1.1 - b2.1).abs() < 1e-6
                || (a2.0 - b1.0).abs() < 1e-6 && (a2.1 - b1.1).abs() < 1e-6
                || (a2.0 - b2.0).abs() < 1e-6 && (a2.1 - b2.1).abs() < 1e-6
            {
                continue;
            }
            overlap += collinear_overlap_length(a1, a2, b1, b2);
            if segments_intersect(a1, a2, b1, b2) {
                crossings += 1;
            }
        }
    }
    (crossings, overlap)
}
