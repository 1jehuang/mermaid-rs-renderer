//! Venn diagram layout (`venn-beta`), after mermaid.js, which lays out with
//! venn.js.
//!
//! - Each set is a circle whose area is its size.
//! - Two sets overlap by the size of the `union` naming both. A union of three
//!   or more sets implies its pairwise overlaps, which are added as mermaid.js
//!   does (`ensurePairwiseSubsets`): a quarter of the smaller set.
//! - The distance between two circles is solved from their overlap area; sets
//!   with no union are kept apart.
//! - Placement: each cluster of overlapping sets starts greedy (each circle
//!   placed where it best meets its targets against those already placed) and
//!   is refined with Nelder–Mead on the pairwise area error. Clusters sit side
//!   by side. The first set is on the left, the second to its right, and the
//!   rest above them, as mermaid.js draws them.
//! - Each region's label goes to the point deepest inside the region (inside
//!   its sets, outside all others) that does not overlap a label already
//!   placed, the smallest regions first. mermaid.js centres every label and
//!   lets them collide: its own three-set docs example overlaps two. The grid
//!   of a region's `text` items is centred on that region's label.
//!
//! Unlike venn.js, the loss is pairwise only; regions of three or more sets
//! are not fitted to their own sizes, only given room by their pairwise
//! overlaps.
use std::collections::BTreeMap;
use std::f32::consts::PI;

use crate::config::LayoutConfig;
use crate::ir::{Graph, VennData};
use crate::theme::Theme;

use super::text::{measure_label_with_font_size, wrap_line};
use super::types::{VennCircleLayout, VennLabelLayout, VennTextLayout};
use super::{DiagramData, Layout, TextBlock};

/// mermaid.js's default viewBox (`venn.width`, `venn.height`, `venn.padding`).
pub(crate) const WIDTH: f32 = 800.0;
pub(crate) const HEIGHT: f32 = 450.0;
const PADDING: f32 = 15.0;
/// mermaid.js scales its type from a 1600 px reference width.
const SCALE: f32 = WIDTH / 1600.0;
pub(crate) const TITLE_HEIGHT: f32 = 48.0 * SCALE;
/// The title's size as mermaid.js draws it: its stylesheet's `.venn-title
/// { font-size: 32px }` outranks the scaled size set on the element.
pub(crate) const TITLE_FONT_SIZE: f32 = 32.0;
pub(crate) const LABEL_FONT_SIZE: f32 = 48.0 * SCALE;
pub(crate) const TEXT_FONT_SIZE: f32 = 40.0 * SCALE;
pub(crate) const STROKE_WIDTH: f32 = 5.0 * SCALE;

/// The weight of an overlap only implied by a union of three or more sets,
/// against 1 for a declared one. When not every overlap can be met — four
/// sets that must all touch cannot all overlap equally — the implied ones give
/// way first: Ikigai's declared neighbours stay a ring and its implied
/// diagonals take the misfit, as mermaid.js draws it. Tuned with
/// `REGION_WEIGHT` on the fixtures; the tests hold the result (every region
/// labelled inside itself, no label on another).
const IMPLIED_WEIGHT: f32 = 0.75;

/// The weight of a region of three or more sets falling short of its area,
/// against 1 for a pair. At 0 Ikigai's centre came out empty (mermaid.js
/// then drops its label); much above this the four circles pile into one
/// another and the ring is lost.
const REGION_WEIGHT: f32 = 2.0;

/// What the layout fits: the overlap area per pair of sets and its weight,
/// and each region of three or more sets with the area it should have.
struct Targets {
    overlap: Vec<Vec<f32>>,
    weight: Vec<Vec<f32>>,
    regions: Vec<(Vec<usize>, f32)>,
}

pub(super) fn compute_venn_layout(graph: &Graph, theme: &Theme, config: &LayoutConfig) -> Layout {
    let venn = &graph.venn;
    let title_h = if venn.title.is_some() {
        TITLE_HEIGHT
    } else {
        0.0
    };

    // The sets, in declared order, with their sizes.
    let sets: Vec<(String, f32)> = venn
        .subsets
        .iter()
        .filter(|s| s.sets.len() == 1)
        .map(|s| (s.sets[0].clone(), s.size))
        .collect();
    let index = |id: &str| sets.iter().position(|(s, _)| s == id);
    let radii: Vec<f32> = sets.iter().map(|(_, size)| (size / PI).sqrt()).collect();
    let n = sets.len();

    // Target overlap area per pair; absent means disjoint.
    let mut overlap = vec![vec![0.0f32; n]; n];
    for subset in venn.subsets.iter().filter(|s| s.sets.len() == 2) {
        if let (Some(a), Some(b)) = (index(&subset.sets[0]), index(&subset.sets[1])) {
            overlap[a][b] = subset.size;
            overlap[b][a] = subset.size;
        }
    }
    // Declared overlaps count fully; implied ones, a guess, give way first.
    let mut weight = vec![vec![1.0f32; n]; n];
    for subset in venn.subsets.iter().filter(|s| s.sets.len() >= 3) {
        let members: Vec<usize> = subset.sets.iter().filter_map(|id| index(id)).collect();
        for (k, &a) in members.iter().enumerate() {
            for &b in &members[k + 1..] {
                if overlap[a][b] <= 0.0 {
                    let size = sets[a].1.min(sets[b].1) / 4.0;
                    overlap[a][b] = size;
                    overlap[b][a] = size;
                    weight[a][b] = IMPLIED_WEIGHT;
                    weight[b][a] = IMPLIED_WEIGHT;
                }
            }
        }
    }
    // Regions of three or more sets, each with the area it should have.
    let regions: Vec<(Vec<usize>, f32)> = venn
        .subsets
        .iter()
        .filter(|s| s.sets.len() >= 3)
        .map(|s| (s.sets.iter().filter_map(|id| index(id)).collect(), s.size))
        .collect();
    let targets = Targets {
        overlap,
        weight,
        regions,
    };

    let mut centres = place(&radii, &targets);
    let scale = fit(&mut centres, &radii, title_h);

    let circles: Vec<VennCircleLayout> = sets
        .iter()
        .enumerate()
        .map(|(k, (id, _))| VennCircleLayout {
            set: id.clone(),
            x: centres[k].0,
            y: centres[k].1,
            radius: radii[k] * scale,
            color_index: k,
        })
        .collect();

    // Labels: a set shows its label or its name; a union only its label.
    // Placed most constrained first — regions of more sets are smaller — each
    // clear of those already placed where its region leaves room.
    let mut order: Vec<&crate::ir::VennSubset> = venn.subsets.iter().collect();
    order.sort_by_key(|s| std::cmp::Reverse(s.sets.len()));
    let mut labels = Vec::new();
    let mut boxes: Vec<(f32, f32, f32, f32)> = Vec::new();
    for subset in order {
        let text = match (&subset.label, subset.sets.len()) {
            (Some(label), _) => label.clone(),
            (None, 1) => subset.sets[0].clone(),
            (None, _) => continue,
        };
        let block =
            measure_label_with_font_size(&text, LABEL_FONT_SIZE, config, false, &theme.font_family);
        let (w, h) = (block.width, LABEL_FONT_SIZE * 1.2);
        let Some((x, y)) = place_label(&circles, &subset.sets, (w, h), &boxes) else {
            continue;
        };
        boxes.push((x - w / 2.0, y - h / 2.0, w, h));
        labels.push(VennLabelLayout {
            sets: subset.sets.clone(),
            text,
            x,
            y,
        });
    }

    let texts = text_items(venn, &circles, &labels, theme, config);

    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        width: WIDTH,
        height: HEIGHT,
        diagram: DiagramData::Venn(super::VennLayout {
            title: venn.title.clone(),
            circles,
            labels,
            texts,
            unions: venn
                .subsets
                .iter()
                .filter(|s| s.sets.len() >= 2)
                .map(|s| s.sets.clone())
                .collect(),
            styles: venn.styles.clone(),
        }),
    }
}

/// Area of the intersection of two circles `d` apart.
fn overlap_area(r1: f32, r2: f32, d: f32) -> f32 {
    let (r1, r2, d) = (r1 as f64, r2 as f64, d as f64);
    if d >= r1 + r2 {
        return 0.0;
    }
    if d <= (r1 - r2).abs() {
        let r = r1.min(r2);
        return (std::f64::consts::PI * r * r) as f32;
    }
    let a = r1
        * r1
        * ((d * d + r1 * r1 - r2 * r2) / (2.0 * d * r1))
            .clamp(-1.0, 1.0)
            .acos();
    let b = r2
        * r2
        * ((d * d + r2 * r2 - r1 * r1) / (2.0 * d * r2))
            .clamp(-1.0, 1.0)
            .acos();
    let c = 0.5
        * ((-d + r1 + r2) * (d + r1 - r2) * (d - r1 + r2) * (d + r1 + r2))
            .max(0.0)
            .sqrt();
    (a + b - c) as f32
}

/// The distance at which two circles overlap by `area`.
fn distance_for(r1: f32, r2: f32, area: f32) -> f32 {
    if area <= 0.0 {
        return r1 + r2;
    }
    if area >= PI * r1.min(r2).powi(2) {
        return (r1 - r2).abs();
    }
    let (mut lo, mut hi) = ((r1 - r2).abs(), r1 + r2);
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if overlap_area(r1, r2, mid) > area {
            lo = mid
        } else {
            hi = mid
        }
    }
    0.5 * (lo + hi)
}

/// Squared error of the pairwise overlaps; disjoint pairs only count when
/// they overlap.
///
/// ⚠ A PAIR THAT SHOULD OVERLAP BUT IS APART STILL PULLS. The overlap area is
/// flat at zero once two circles part, so the optimiser had no way back: four
/// sets that cannot all overlap equally (Ikigai) settled with one pair wholly
/// apart and the others exact. Past touching, the error grows with the gap
/// (continuous at touching), and the optimiser spreads the misfit instead.
fn loss(pos: &[(f32, f32)], radii: &[f32], t: &Targets, members: &[usize]) -> f32 {
    let mut total = 0.0;
    for (a, &i) in members.iter().enumerate() {
        for &j in &members[a + 1..] {
            let d = ((pos[i].0 - pos[j].0).powi(2) + (pos[i].1 - pos[j].1).powi(2)).sqrt();
            let (want, w) = (t.overlap[i][j], t.weight[i][j]);
            let apart = d - (radii[i] + radii[j]);
            if want > 0.0 && apart > 0.0 {
                total += w * (want * want + want * apart * apart);
                continue;
            }
            let actual = overlap_area(radii[i], radii[j], d);
            total += w * (actual - want).powi(2);
        }
    }
    // ⚠ A REGION OF THREE OR MORE MUST EXIST. Pairwise targets alone let
    // Ikigai's four circles meet only in pairs, leaving the region its centre
    // label names empty (mermaid.js then drops the label). Its area is
    // estimated by the circle as deep as the point at the centres' centroid —
    // signed, so an empty region still pulls — and only a shortfall counts.
    for (sets, want) in &t.regions {
        if !sets.iter().all(|i| members.contains(i)) {
            continue;
        }
        let k = sets.len() as f32;
        let cx = sets.iter().map(|&i| pos[i].0).sum::<f32>() / k;
        let cy = sets.iter().map(|&i| pos[i].1).sum::<f32>() / k;
        let depth = sets
            .iter()
            .map(|&i| radii[i] - ((cx - pos[i].0).powi(2) + (cy - pos[i].1).powi(2)).sqrt())
            .fold(f32::INFINITY, f32::min);
        let area = PI * depth * depth.abs();
        if area < *want {
            total += REGION_WEIGHT * (want - area).powi(2);
        }
    }
    total
}

/// Circle centres, before scaling: each cluster laid out, then the clusters
/// side by side.
fn place(radii: &[f32], t: &Targets) -> Vec<(f32, f32)> {
    let overlap = &t.overlap;
    let n = radii.len();
    let mut pos = vec![(0.0f32, 0.0f32); n];
    let mut cluster_of = vec![usize::MAX; n];
    let mut clusters: Vec<Vec<usize>> = Vec::new();
    for start in 0..n {
        if cluster_of[start] != usize::MAX {
            continue;
        }
        let mut members = vec![start];
        cluster_of[start] = clusters.len();
        let mut k = 0;
        while k < members.len() {
            let i = members[k];
            for j in 0..n {
                if cluster_of[j] == usize::MAX && overlap[i][j] > 0.0 {
                    cluster_of[j] = clusters.len();
                    members.push(j);
                }
            }
            k += 1;
        }
        members.sort();
        clusters.push(members);
    }

    let mut x_offset = 0.0f32;
    let gap = radii.iter().copied().fold(0.0f32, f32::max) * 0.2;
    for members in &clusters {
        place_cluster(&mut pos, radii, t, members);
        orient(&mut pos, members);
        let min_x = members
            .iter()
            .map(|&i| pos[i].0 - radii[i])
            .fold(f32::INFINITY, f32::min);
        let max_x = members
            .iter()
            .map(|&i| pos[i].0 + radii[i])
            .fold(f32::NEG_INFINITY, f32::max);
        for &i in members {
            pos[i].0 += x_offset - min_x;
        }
        x_offset += (max_x - min_x) + gap;
    }
    pos
}

fn place_cluster(pos: &mut [(f32, f32)], radii: &[f32], t: &Targets, members: &[usize]) {
    let overlap = &t.overlap;
    let first = members[0];
    pos[first] = (0.0, 0.0);
    let mut placed = vec![first];
    // Greedy: the next circle is the unplaced one with the most overlap to
    // those placed, put where it best meets its targets against them.
    while placed.len() < members.len() {
        let next = members
            .iter()
            .copied()
            .filter(|i| !placed.contains(i))
            .max_by(|&a, &b| {
                let score = |i: usize| placed.iter().map(|&p| overlap[i][p]).sum::<f32>();
                score(a).total_cmp(&score(b)).then(b.cmp(&a))
            })
            .unwrap_or(first);
        let mut best = (f32::INFINITY, (0.0, 0.0));
        for &anchor in &placed {
            let d = distance_for(radii[next], radii[anchor], overlap[next][anchor]);
            for step in 0..72 {
                let angle = step as f32 * PI / 36.0;
                let candidate = (
                    pos[anchor].0 + d * angle.cos(),
                    pos[anchor].1 + d * angle.sin(),
                );
                pos[next] = candidate;
                let mut with: Vec<usize> = placed.clone();
                with.push(next);
                let l = loss(pos, radii, t, &with);
                if l < best.0 - 1e-6 {
                    best = (l, candidate);
                }
            }
        }
        pos[next] = best.1;
        placed.push(next);
    }
    if members.len() > 2 {
        nelder_mead(pos, radii, t, members);
        // A second start, from classical MDS on the target distances (as
        // venn.js tries both): the greedy start can settle in a local minimum
        // — a ring of four sets came out with one pair not touching.
        let greedy: Vec<(f32, f32)> = members.iter().map(|&i| pos[i]).collect();
        let greedy_loss = loss(pos, radii, t, members);
        if let Some(start) = mds(radii, overlap, members) {
            let origin = start[0];
            for (k, &i) in members.iter().enumerate() {
                pos[i] = (start[k].0 - origin.0, start[k].1 - origin.1);
            }
            nelder_mead(pos, radii, t, members);
            if loss(pos, radii, t, members) >= greedy_loss {
                for (k, &i) in members.iter().enumerate() {
                    pos[i] = greedy[k];
                }
            }
        }
    }
}

/// Classical multidimensional scaling of the target distances into the plane.
fn mds(radii: &[f32], overlap: &[Vec<f32>], members: &[usize]) -> Option<Vec<(f32, f32)>> {
    let n = members.len();
    let d2: Vec<Vec<f64>> = members
        .iter()
        .map(|&i| {
            members
                .iter()
                .map(|&j| {
                    let d = if i == j {
                        0.0
                    } else {
                        distance_for(radii[i], radii[j], overlap[i][j]) as f64
                    };
                    d * d
                })
                .collect()
        })
        .collect();
    // B = -1/2 J D² J (double centring).
    let row: Vec<f64> = d2
        .iter()
        .map(|r| r.iter().sum::<f64>() / n as f64)
        .collect();
    let all = row.iter().sum::<f64>() / n as f64;
    let mut b: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| -0.5 * (d2[i][j] - row[i] - row[j] + all))
                .collect()
        })
        .collect();
    // The two largest eigenpairs, by power iteration with deflation.
    let mut coords = vec![(0.0f32, 0.0f32); n];
    for axis in 0..2 {
        let mut v: Vec<f64> = (0..n)
            .map(|k| 1.0 + k as f64 * (0.37 + axis as f64 * 0.21))
            .collect();
        let mut lambda = 0.0;
        for _ in 0..500 {
            let w: Vec<f64> = (0..n)
                .map(|i| (0..n).map(|j| b[i][j] * v[j]).sum())
                .collect();
            let norm = w.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm < 1e-12 {
                break;
            }
            lambda = norm;
            v = w.iter().map(|x| x / norm).collect();
        }
        let scale = lambda.max(0.0).sqrt();
        for k in 0..n {
            if axis == 0 {
                coords[k].0 = (v[k] * scale) as f32
            } else {
                coords[k].1 = (v[k] * scale) as f32
            }
        }
        for i in 0..n {
            for j in 0..n {
                b[i][j] -= lambda * v[i] * v[j];
            }
        }
    }
    coords
        .iter()
        .all(|c| c.0.is_finite() && c.1.is_finite())
        .then_some(coords)
}

/// Refines a cluster's centres (all but the first, which stays at the origin).
fn nelder_mead(pos: &mut [(f32, f32)], radii: &[f32], t: &Targets, members: &[usize]) {
    let free: Vec<usize> = members[1..].to_vec();
    let dim = free.len() * 2;
    let pack = |pos: &[(f32, f32)]| -> Vec<f32> {
        free.iter().flat_map(|&i| [pos[i].0, pos[i].1]).collect()
    };
    let mut work = pos.to_vec();
    let eval = |x: &[f32], work: &mut Vec<(f32, f32)>| -> f32 {
        for (k, &i) in free.iter().enumerate() {
            work[i] = (x[2 * k], x[2 * k + 1]);
        }
        loss(work, radii, t, members)
    };
    let step = radii.iter().copied().fold(0.0f32, f32::max) * 0.25;
    let x0 = pack(pos);
    let mut simplex: Vec<(Vec<f32>, f32)> = Vec::with_capacity(dim + 1);
    let f0 = eval(&x0, &mut work);
    simplex.push((x0.clone(), f0));
    for k in 0..dim {
        let mut x = x0.clone();
        x[k] += step;
        let f = eval(&x, &mut work);
        simplex.push((x, f));
    }
    for _ in 0..(400 * dim) {
        simplex.sort_by(|a, b| a.1.total_cmp(&b.1));
        if simplex[dim].1 - simplex[0].1 < 1e-9 {
            break;
        }
        let centroid: Vec<f32> = (0..dim)
            .map(|k| simplex[..dim].iter().map(|(x, _)| x[k]).sum::<f32>() / dim as f32)
            .collect();
        let towards = |t: f32, x: &[f32]| -> Vec<f32> {
            centroid
                .iter()
                .zip(x)
                .map(|(c, xi)| c + t * (xi - c))
                .collect()
        };
        let worst = simplex[dim].0.clone();
        let reflected = towards(-1.0, &worst);
        let fr = eval(&reflected, &mut work);
        if fr < simplex[0].1 {
            let expanded = towards(-2.0, &worst);
            let fe = eval(&expanded, &mut work);
            simplex[dim] = if fe < fr {
                (expanded, fe)
            } else {
                (reflected, fr)
            };
        } else if fr < simplex[dim - 1].1 {
            simplex[dim] = (reflected, fr);
        } else {
            let contracted = towards(0.5, &worst);
            let fc = eval(&contracted, &mut work);
            if fc < simplex[dim].1 {
                simplex[dim] = (contracted, fc);
            } else {
                let best = simplex[0].0.clone();
                for entry in simplex.iter_mut().skip(1) {
                    let x: Vec<f32> = best
                        .iter()
                        .zip(&entry.0)
                        .map(|(b, xi)| b + 0.5 * (xi - b))
                        .collect();
                    let f = eval(&x, &mut work);
                    *entry = (x, f);
                }
            }
        }
    }
    simplex.sort_by(|a, b| a.1.total_cmp(&b.1));
    for (k, &i) in free.iter().enumerate() {
        pos[i] = (simplex[0].0[2 * k], simplex[0].0[2 * k + 1]);
    }
}

/// First set on the left, second to its right, third above them.
fn orient(pos: &mut [(f32, f32)], members: &[usize]) {
    if members.len() < 2 {
        return;
    }
    let (a, b) = (pos[members[0]], pos[members[1]]);
    let angle = (b.1 - a.1).atan2(b.0 - a.0);
    let (sin, cos) = (-angle).sin_cos();
    for &i in members {
        let (x, y) = (pos[i].0 - a.0, pos[i].1 - a.1);
        pos[i] = (x * cos - y * sin, x * sin + y * cos);
    }
    if members.len() >= 3 && pos[members[2]].1 > 0.0 {
        for &i in members {
            pos[i].1 = -pos[i].1;
        }
    }
}

/// Scales and centres the circles into the canvas below the title; returns
/// the scale, for the radii.
fn fit(pos: &mut [(f32, f32)], radii: &[f32], title_h: f32) -> f32 {
    if pos.is_empty() {
        return 1.0;
    }
    let (min_x, max_x, min_y, max_y) = pos.iter().zip(radii).fold(
        (
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ),
        |(x0, x1, y0, y1), (p, r)| {
            (
                x0.min(p.0 - r),
                x1.max(p.0 + r),
                y0.min(p.1 - r),
                y1.max(p.1 + r),
            )
        },
    );
    let (w, h) = ((max_x - min_x).max(1e-6), (max_y - min_y).max(1e-6));
    let avail_w = WIDTH - 2.0 * PADDING;
    let avail_h = HEIGHT - title_h - 2.0 * PADDING;
    let scale = (avail_w / w).min(avail_h / h);
    let off_x = (WIDTH - w * scale) / 2.0;
    let off_y = title_h + (HEIGHT - title_h - h * scale) / 2.0;
    for p in pos.iter_mut() {
        *p = (off_x + (p.0 - min_x) * scale, off_y + (p.1 - min_y) * scale);
    }
    scale
}

/// The region of `sets`: inside each of their circles, outside every other.
struct Region<'a> {
    inside: Vec<&'a VennCircleLayout>,
    outside: Vec<&'a VennCircleLayout>,
}

impl<'a> Region<'a> {
    fn new(circles: &'a [VennCircleLayout], sets: &[String]) -> Self {
        let (inside, outside) = circles.iter().partition(|c| sets.contains(&c.set));
        Self { inside, outside }
    }

    /// How deep `p` lies in the inner circles and, if `exclusive`, clear of
    /// the others too. Negative outside the region.
    fn depth(&self, p: (f32, f32), exclusive: bool) -> f32 {
        let dist = |c: &VennCircleLayout| ((p.0 - c.x).powi(2) + (p.1 - c.y).powi(2)).sqrt();
        let m = self
            .inside
            .iter()
            .map(|c| c.radius - dist(c))
            .fold(f32::INFINITY, f32::min);
        if !exclusive {
            return m;
        }
        self.outside
            .iter()
            .map(|c| dist(c) - c.radius)
            .fold(m, f32::min)
    }

    /// The bounding box `(x0, x1, y0, y1)` of the inner circles' intersection.
    fn bounds(&self) -> Option<(f32, f32, f32, f32)> {
        if self.inside.is_empty() {
            return None;
        }
        let edge = |f: fn(&VennCircleLayout) -> f32, max: bool| {
            let values = self.inside.iter().map(|c| f(c));
            if max {
                values.fold(f32::NEG_INFINITY, f32::max)
            } else {
                values.fold(f32::INFINITY, f32::min)
            }
        };
        let x0 = edge(|c| c.x - c.radius, true);
        let x1 = edge(|c| c.x + c.radius, false);
        let y0 = edge(|c| c.y - c.radius, true);
        let y1 = edge(|c| c.y + c.radius, false);
        (x1 >= x0 && y1 >= y0).then_some((x0, x1, y0, y1))
    }
}

/// `steps + 1` × `steps + 1` points spanning the box `(x0, x1, y0, y1)`.
fn grid((x0, x1, y0, y1): (f32, f32, f32, f32), steps: usize) -> impl Iterator<Item = (f32, f32)> {
    (0..=steps).flat_map(move |i| {
        (0..=steps).map(move |j| {
            (
                x0 + (x1 - x0) * i as f32 / steps as f32,
                y0 + (y1 - y0) * j as f32 / steps as f32,
            )
        })
    })
}

/// The point deepest inside the region of `sets`, found on a grid refined
/// four times around the best point. Falls back to the point deepest inside
/// `sets` alone when the region is empty.
fn region_centre(circles: &[VennCircleLayout], sets: &[String]) -> Option<(f32, f32)> {
    let region = Region::new(circles, sets);
    let (x0, x1, y0, y1) = region.bounds()?;
    for exclusive in [true, false] {
        let (mut best, mut best_m) = ((0.5 * (x0 + x1), 0.5 * (y0 + y1)), f32::NEG_INFINITY);
        let mut area = (x0, x1, y0, y1);
        for _ in 0..4 {
            for p in grid(area, 32) {
                let m = region.depth(p, exclusive);
                if m > best_m + 1e-4 {
                    (best, best_m) = (p, m);
                }
            }
            let (hw, hh) = ((area.1 - area.0) / 8.0, (area.3 - area.2) / 8.0);
            area = (best.0 - hw, best.0 + hw, best.1 - hh, best.1 + hh);
        }
        if best_m > 0.0 || !exclusive {
            return Some(best);
        }
    }
    None
}

/// Where a label `size` (width, height) goes in the region of `sets`: of the
/// points inside the region, the one whose box overlaps the `placed` boxes
/// least, the deepest among equals; the region's centre when none beats it.
fn place_label(
    circles: &[VennCircleLayout],
    sets: &[String],
    size: (f32, f32),
    placed: &[(f32, f32, f32, f32)],
) -> Option<(f32, f32)> {
    let centre = region_centre(circles, sets)?;
    if placed.is_empty() {
        return Some(centre);
    }
    let region = Region::new(circles, sets);
    let overlap = |p: (f32, f32)| -> f32 {
        let (x0, y0) = (p.0 - size.0 / 2.0, p.1 - size.1 / 2.0);
        placed
            .iter()
            .map(|b| {
                let w = (x0 + size.0).min(b.0 + b.2) - x0.max(b.0);
                let h = (y0 + size.1).min(b.1 + b.3) - y0.max(b.1);
                w.max(0.0) * h.max(0.0)
            })
            .sum()
    };
    let mut best = (overlap(centre), -region.depth(centre, true), centre);
    for p in grid(region.bounds()?, 48) {
        let m = region.depth(p, true);
        if m <= 0.0 {
            continue;
        }
        let candidate = (overlap(p), -m, p);
        if candidate.0 < best.0 - 0.5 || (candidate.0 <= best.0 + 0.5 && candidate.1 < best.1) {
            best = candidate;
        }
    }
    Some(best.2)
}

/// `text` items: grouped by region and set in a grid around the region's
/// label, as mermaid.js does (below the region's label when it has one).
fn text_items(
    venn: &VennData,
    circles: &[VennCircleLayout],
    labels: &[VennLabelLayout],
    theme: &Theme,
    config: &LayoutConfig,
) -> Vec<VennTextLayout> {
    let mut by_region: Vec<(Vec<String>, Vec<&crate::ir::VennText>)> = Vec::new();
    for text in &venn.texts {
        match by_region.iter_mut().find(|(sets, _)| *sets == text.sets) {
            Some((_, items)) => items.push(text),
            None => by_region.push((text.sets.clone(), vec![text])),
        }
    }
    let mut out = Vec::new();
    for (sets, items) in by_region {
        let at = labels.iter().find(|l| l.sets == sets).map(|l| (l.x, l.y));
        let Some((cx, cy)) = at.or_else(|| region_centre(circles, &sets)) else {
            continue;
        };
        let region = Region::new(circles, &sets);
        let min_r = region
            .inside
            .iter()
            .map(|c| c.radius)
            .fold(f32::INFINITY, f32::min);
        let mut inner = region.depth((cx, cy), false).max(0.0);
        if inner == 0.0 && min_r.is_finite() {
            inner = min_r * 0.6;
        }
        let inner_w = (80.0 * SCALE).max(inner * 2.0 * 0.95);
        let inner_h = (60.0 * SCALE).max(inner * 2.0 * 0.95);
        let label_offset = if at.is_some() {
            (32.0 * SCALE).min(inner * 0.25)
        } else {
            0.0
        } + if items.len() <= 2 { 30.0 * SCALE } else { 0.0 };
        let cols = (items.len() as f32).sqrt().ceil().max(1.0) as usize;
        let rows = items.len().div_ceil(cols).max(1);
        let (cell_w, cell_h) = (inner_w / cols as f32, inner_h / rows as f32);
        let (start_x, start_y) = (cx - inner_w / 2.0, cy - inner_h / 2.0 + label_offset);
        for (i, item) in items.iter().enumerate() {
            let text = item.label.clone().unwrap_or_else(|| item.id.clone());
            let width = cell_w * 0.9;
            let lines = wrap_line(
                &text,
                width,
                TEXT_FONT_SIZE,
                &theme.font_family,
                config.fast_text_metrics,
            );
            let block = measure_label_with_font_size(
                &lines.join("\n"),
                TEXT_FONT_SIZE,
                config,
                false,
                &theme.font_family,
            );
            out.push(VennTextLayout {
                id: item.id.clone(),
                sets: sets.clone(),
                lines: TextBlock { lines, ..block },
                x: start_x + cell_w * ((i % cols) as f32 + 0.5),
                y: start_y + cell_h * ((i / cols) as f32 + 0.5),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlap_area_is_exact_at_the_limits() {
        assert!((overlap_area(1.0, 1.0, 2.0)).abs() < 1e-6);
        assert!((overlap_area(1.0, 2.0, 0.5) - PI).abs() < 1e-4);
    }

    fn layout_of(input: &str) -> super::super::VennLayout {
        let parsed = crate::parser::parse_mermaid(input).unwrap();
        let theme = Theme::modern();
        let config = LayoutConfig {
            fast_text_metrics: true,
            ..LayoutConfig::default()
        };
        let layout = super::super::compute_layout(&parsed.graph, &theme, &config);
        match layout.diagram {
            DiagramData::Venn(v) => v,
            _ => panic!("not a venn layout"),
        }
    }

    const CONWAY: &str = "venn-beta\n  title The Data Science Venn Diagram\n  set Hacking[\"Hacking Skills\"]\n  set Math[\"Math & Statistics\"]\n  set Domain[\"Substantive Expertise\"]\n  union Hacking,Math[\"Machine Learning\"]\n  union Math,Domain[\"Traditional Research\"]\n  union Hacking,Domain[\"Danger Zone!\"]\n  union Hacking,Math,Domain[\"Data Science\"]\n";
    const IKIGAI: &str = "venn-beta\n  set Love\n  set Good\n  set Needs\n  set Paid\n  union Love,Good[\"Passion\"]\n  union Love,Needs[\"Mission\"]\n  union Needs,Paid[\"Vocation\"]\n  union Good,Paid[\"Profession\"]\n  union Love,Good,Needs,Paid[\"Ikigai\"]\n";

    /// Every label is placed, inside the canvas, and no two overlap; every
    /// union's label lies inside each of its circles.
    fn assert_labels_clean(input: &str, expected: usize) {
        let venn = layout_of(input);
        assert_eq!(venn.labels.len(), expected, "labels placed");
        let config = LayoutConfig {
            fast_text_metrics: true,
            ..LayoutConfig::default()
        };
        let theme = Theme::modern();
        let boxes: Vec<(f32, f32, f32, f32)> = venn
            .labels
            .iter()
            .map(|l| {
                let w = measure_label_with_font_size(
                    &l.text,
                    LABEL_FONT_SIZE,
                    &config,
                    false,
                    &theme.font_family,
                )
                .width;
                (
                    l.x - w / 2.0,
                    l.y - LABEL_FONT_SIZE * 0.6,
                    w,
                    LABEL_FONT_SIZE * 1.2,
                )
            })
            .collect();
        for (i, a) in boxes.iter().enumerate() {
            assert!(
                a.0 >= 0.0 && a.1 >= 0.0 && a.0 + a.2 <= WIDTH && a.1 + a.3 <= HEIGHT,
                "{:?} off canvas",
                venn.labels[i].text
            );
            for (j, b) in boxes.iter().enumerate().skip(i + 1) {
                let overlap =
                    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3;
                assert!(
                    !overlap,
                    "{:?} overlaps {:?}",
                    venn.labels[i].text, venn.labels[j].text
                );
            }
        }
        for label in venn.labels.iter().filter(|l| l.sets.len() >= 2) {
            for c in venn.circles.iter().filter(|c| label.sets.contains(&c.set)) {
                let d = ((label.x - c.x).powi(2) + (label.y - c.y).powi(2)).sqrt();
                assert!(d < c.radius, "{:?} outside {}", label.text, c.set);
            }
        }
        for c in &venn.circles {
            assert!(
                c.x - c.radius >= 0.0 && c.x + c.radius <= WIDTH && c.y + c.radius <= HEIGHT,
                "{} off canvas",
                c.set
            );
        }
    }

    #[test]
    fn famous_diagrams_label_every_region_without_collisions() {
        // Three sets, three pairs and the centre: mermaid.js overlaps two of
        // these labels in its own docs example.
        assert_labels_clean(CONWAY, 7);
        // Four sets, four declared pairs and the centre.
        assert_labels_clean(IKIGAI, 9);
    }

    #[test]
    fn two_sets_overlap_by_their_union() {
        let venn = layout_of("venn-beta\n  set A:20\n  set B:12\n  union A,B:3\n");
        let (a, b) = (&venn.circles[0], &venn.circles[1]);
        let d = ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt();
        // Areas scale with the square of the radius: compare in the sets' units.
        let k = a.radius / (20.0f32 / PI).sqrt();
        let area = overlap_area(a.radius / k, b.radius / k, d / k);
        assert!((area - 3.0).abs() < 0.05, "overlap {area}");
        assert!(a.x < b.x, "first set on the left");
    }

    #[test]
    fn a_ring_keeps_its_declared_neighbours_touching() {
        let venn = layout_of(IKIGAI);
        let c = |id: &str| venn.circles.iter().find(|c| c.set == id).unwrap();
        for (x, y) in [
            ("Love", "Good"),
            ("Love", "Needs"),
            ("Needs", "Paid"),
            ("Good", "Paid"),
        ] {
            let (a, b) = (c(x), c(y));
            let d = ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt();
            assert!(d < a.radius + b.radius, "{x} and {y} should overlap");
        }
    }

    #[test]
    fn sets_without_a_union_stay_apart() {
        let venn = layout_of("venn-beta\n  set Cats\n  set Dogs\n  set Fish\n  union Cats,Dogs\n");
        let c = |id: &str| venn.circles.iter().find(|c| c.set == id).unwrap();
        let (fish, dogs) = (c("Fish"), c("Dogs"));
        let d = ((fish.x - dogs.x).powi(2) + (fish.y - dogs.y).powi(2)).sqrt();
        assert!(d >= fish.radius + dogs.radius, "Fish overlaps Dogs");
    }

    #[test]
    fn distance_inverts_the_area() {
        let (r1, r2) = (1.784, 1.2);
        let d = distance_for(r1, r2, 1.5);
        assert!((overlap_area(r1, r2, d) - 1.5).abs() < 1e-3);
    }
}
