use super::*;

type Rect = (f32, f32, f32, f32);

/// Min/max summaries compose without rescanning the contents of nested frames.
#[derive(Clone, Copy, Debug, PartialEq)]
struct FrameBounds {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
}

impl FrameBounds {
    const EMPTY: Self = Self {
        min_x: f32::INFINITY,
        max_x: f32::NEG_INFINITY,
        min_y: f32::INFINITY,
        max_y: f32::NEG_INFINITY,
    };

    /// Return the union of two bounds; EMPTY is the identity for this operation.
    fn merge(self, other: Self) -> Self {
        Self {
            min_x: self.min_x.min(other.min_x),
            max_x: self.max_x.max(other.max_x),
            min_y: self.min_y.min(other.min_y),
            max_y: self.max_y.max(other.max_y),
        }
    }
}

/// O(n) storage/build and O(log n) queries, including arbitrary public-IR ranges.
struct FrameBoundsIndex {
    tree: Vec<FrameBounds>,
    len: usize,
}

impl FrameBoundsIndex {
    /// Build a linear-space range index over bounds in message or note order.
    fn new(bounds: impl ExactSizeIterator<Item = FrameBounds>) -> Self {
        let len = bounds.len();
        let mut tree = vec![FrameBounds::EMPTY; len * 2];
        for (index, bounds) in bounds.enumerate() {
            tree[len + index] = bounds;
        }
        for index in (1..len).rev() {
            tree[index] = tree[index * 2].merge(tree[index * 2 + 1]);
        }
        Self { tree, len }
    }

    /// Return bounds for a half-open range, clamping endpoints to the indexed length.
    /// Empty or reversed ranges return EMPTY, including queries on an empty index.
    fn query(&self, range: std::ops::Range<usize>) -> FrameBounds {
        let mut result = FrameBounds::EMPTY;
        let mut start = range.start.min(self.len) + self.len;
        let mut end = range.end.min(self.len) + self.len;
        while start < end {
            if start % 2 == 1 {
                result = result.merge(self.tree[start]);
                start += 1;
            }
            if end % 2 == 1 {
                end -= 1;
                result = result.merge(self.tree[end]);
            }
            start /= 2;
            end /= 2;
        }
        result
    }
}

const SEQUENCE_LABEL_PAD_X: f32 = 3.0;
const SEQUENCE_LABEL_PAD_Y: f32 = 2.0;
const SEQUENCE_ENDPOINT_LABEL_PAD_X: f32 = 2.5;
const SEQUENCE_ENDPOINT_LABEL_PAD_Y: f32 = 1.5;
const SEQUENCE_LABEL_TOUCH_EPS: f32 = 0.5;
const SEQUENCE_CENTER_LABEL_GAP_MIN: f32 = 1.8;
const SEQUENCE_CENTER_LABEL_GAP_MAX: f32 = 7.0;
const SEQUENCE_CENTER_LABEL_FAR_GAP: f32 = 10.5;
const SEQUENCE_ENDPOINT_LABEL_GAP_TARGET: f32 = 2.5;
const SEQUENCE_ENDPOINT_LABEL_GAP_MIN: f32 = 1.0;
const SEQUENCE_ENDPOINT_LABEL_GAP_MAX: f32 = 6.0;
const SEQUENCE_ENDPOINT_LABEL_FAR_GAP: f32 = 10.0;
const SEQUENCE_CENTER_LABEL_TANGENT_LINEAR_WEIGHT: f32 = 0.22;
const SEQUENCE_CENTER_LABEL_TANGENT_QUAD_WEIGHT: f32 = 0.95;
const SEQUENCE_CENTER_LABEL_TANGENT_SOFT_LIMIT: f32 = 1.2;
const SEQUENCE_CENTER_LABEL_TANGENT_FAR_WEIGHT: f32 = 3.2;

#[derive(Clone, Copy)]
enum SequenceLabelPlacementMode {
    Center,
    Endpoint,
}

#[derive(Clone, Copy)]
struct SequenceGeometry {
    actor_min_width: f32,
    actor_min_height: f32,
    actor_pad_y: f32,
    lane_pitch: f32,
    min_lane_gap: f32,
    message_step: f32,
    note_gap_y: f32,
    note_gap_x: f32,
    note_padding_x: f32,
    note_padding_y: f32,
    lane_side_pad_x: f32,
    footbox_gap: f32,
}

impl SequenceGeometry {
    fn from_theme(theme: &Theme) -> Self {
        let font = theme.font_size.max(1.0);
        Self {
            actor_min_width: (font * 9.375).max(150.0),
            actor_min_height: (font * 4.0625).max(65.0),
            actor_pad_y: (font * 0.75).max(12.0),
            lane_pitch: (font * 12.5).max(200.0),
            min_lane_gap: (font * 3.125).max(50.0),
            message_step: (font * 2.875).max(46.0),
            note_gap_y: (font * 0.625).max(10.0),
            note_gap_x: (font * 1.5625).max(25.0),
            note_padding_x: (font * 1.0).max(15.0),
            note_padding_y: (font * 0.55).max(6.0),
            lane_side_pad_x: (font * 1.5625).max(25.0),
            footbox_gap: (font * 1.375).max(22.0),
        }
    }
}

fn measure_sequence_text(text: &str, theme: &Theme, config: &LayoutConfig) -> TextBlock {
    let mut sequence_config = config.clone();
    sequence_config.max_label_width_chars = sequence_config.max_label_width_chars.min(14);
    measure_label_with_font_size(
        text,
        theme.font_size.max(16.0),
        &sequence_config,
        true,
        theme.font_family.as_str(),
    )
}

fn sequence_lane_center(node: &NodeLayout) -> f32 {
    node.x + node.width / 2.0
}

fn compute_sequence_lane_centers(
    participants: &[String],
    participant_widths: &HashMap<String, f32>,
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
    geometry: SequenceGeometry,
) -> Vec<f32> {
    if participants.is_empty() {
        return Vec::new();
    }

    let participant_indices: HashMap<&str, usize> = participants
        .iter()
        .enumerate()
        .map(|(idx, id)| (id.as_str(), idx))
        .collect();

    let mut pitches = participants
        .windows(2)
        .map(|pair| {
            let left_w = participant_widths
                .get(&pair[0])
                .copied()
                .unwrap_or(geometry.actor_min_width);
            let right_w = participant_widths
                .get(&pair[1])
                .copied()
                .unwrap_or(geometry.actor_min_width);
            geometry
                .lane_pitch
                .max((left_w + right_w) * 0.5 + geometry.min_lane_gap)
        })
        .collect::<Vec<_>>();

    for edge in &graph.edges {
        let (Some(&from_idx), Some(&to_idx)) = (
            participant_indices.get(edge.from.as_str()),
            participant_indices.get(edge.to.as_str()),
        ) else {
            continue;
        };

        let left_idx = from_idx.min(to_idx);
        let right_idx = from_idx.max(to_idx);
        if right_idx != left_idx + 1 {
            continue;
        }

        if let Some(label) = edge.label.as_ref() {
            let block = measure_sequence_text(label, theme, config);
            let base_center = if left_idx == 0 {
                geometry.actor_min_width * 0.5
            } else {
                geometry.actor_min_width * 0.5 + pitches.iter().take(left_idx).sum::<f32>()
            };
            let default_mid_x = base_center + geometry.lane_pitch * 0.5;
            let max_label_x = default_mid_x + block.width * 0.5;
            let right_min_center = max_label_x + geometry.note_gap_x;
            let current_right_center = base_center + pitches[left_idx];
            pitches[left_idx] = pitches[left_idx].max(right_min_center - base_center);
            pitches[left_idx] = pitches[left_idx].max(current_right_center - base_center);
        }
    }

    let first_width = participant_widths
        .get(&participants[0])
        .copied()
        .unwrap_or(geometry.actor_min_width);
    let mut centers = Vec::with_capacity(participants.len());
    centers.push(first_width * 0.5);
    for pitch in pitches {
        let prev = *centers.last().unwrap_or(&0.0);
        centers.push(prev + pitch);
    }
    centers
}

/// Lay out participants, messages, notes, and frames, using message and note
/// ranges to include only the content owned by each frame.
pub(super) fn compute_sequence_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> Layout {
    let mut nodes = BTreeMap::new();
    let mut edges = Vec::new();
    let subgraphs = Vec::new();

    let mut participants = graph.sequence_participants.clone();
    for id in graph.nodes.keys() {
        if !participants.contains(id) {
            participants.push(id.clone());
        }
    }

    let geometry = SequenceGeometry::from_theme(theme);
    let mut label_blocks: HashMap<String, TextBlock> = HashMap::new();
    let mut participant_widths: HashMap<String, f32> = HashMap::new();
    let mut max_label_height = 0.0f32;

    for id in &participants {
        let node = graph.nodes.get(id).expect("participant missing");
        let label = measure_sequence_text(&node.label, theme, config);
        max_label_height = max_label_height.max(label.height);
        let width = geometry.actor_min_width;
        participant_widths.insert(id.clone(), width);
        label_blocks.insert(id.clone(), label);
    }

    let actor_height =
        (max_label_height + geometry.actor_pad_y * 2.0).max(geometry.actor_min_height);
    let lane_centers = compute_sequence_lane_centers(
        &participants,
        &participant_widths,
        graph,
        theme,
        config,
        geometry,
    );

    for (idx, id) in participants.iter().enumerate() {
        let node = graph.nodes.get(id).expect("participant missing");
        let actor_width = participant_widths
            .get(id)
            .copied()
            .unwrap_or(geometry.actor_min_width);
        let label = label_blocks.get(id).cloned().unwrap_or_else(|| TextBlock {
            lines: vec![id.clone()],
            width: 0.0,
            height: 0.0,
        });
        let center_x = lane_centers.get(idx).copied().unwrap_or(actor_width * 0.5);
        nodes.insert(
            id.clone(),
            NodeLayout {
                id: id.clone(),
                x: center_x - actor_width / 2.0,
                y: 0.0,
                width: actor_width,
                height: actor_height,
                label,
                shape: node.shape,
                style: resolve_node_style(id.as_str(), graph),
                link: graph.node_links.get(id).cloned(),
                anchor_subgraph: None,
                hidden: false,
                icon: None,
            },
        );
    }

    let base_spacing = geometry.message_step.max(18.0);
    let message_row_spacing: Vec<f32> = graph
        .edges
        .iter()
        .map(|edge| {
            let mut row_h = 0.0f32;
            if let Some(label) = &edge.label {
                row_h = row_h.max(measure_sequence_text(label, theme, config).height);
            }
            if let Some(label) = &edge.start_label {
                row_h = row_h.max(measure_sequence_text(label, theme, config).height);
            }
            if let Some(label) = &edge.end_label {
                row_h = row_h.max(measure_sequence_text(label, theme, config).height);
            }
            base_spacing.max(row_h + theme.font_size * 1.25)
        })
        .collect();

    let mut extra_before = vec![0.0; graph.edges.len()];
    let frame_end_pad = (theme.font_size * 0.85).max(12.0);
    let mut frame_end_padding: BTreeMap<(usize, usize), f32> = BTreeMap::new();
    for frame in &graph.sequence_frames {
        if frame.start_idx < extra_before.len() {
            extra_before[frame.start_idx] += base_spacing;
        }
        for section in frame.sections.iter().skip(1) {
            if section.start_idx < extra_before.len() {
                extra_before[section.start_idx] += base_spacing;
            }
        }
        *frame_end_padding
            .entry((frame.end_idx, frame.note_range.end))
            .or_default() += frame_end_pad;
    }

    let mut notes_by_index = vec![Vec::new(); graph.edges.len().saturating_add(1)];
    for note in &graph.sequence_notes {
        let idx = note.index.min(graph.edges.len());
        notes_by_index[idx].push(note);
    }

    let mut message_cursor = actor_height;
    let mut message_ys = Vec::new();
    let mut sequence_notes = Vec::new();
    for idx in 0..=graph.edges.len() {
        if let Some(bucket) = notes_by_index.get(idx) {
            for note in bucket {
                message_cursor += frame_end_padding
                    .remove(&(idx, sequence_notes.len()))
                    .unwrap_or(0.0);
                message_cursor += geometry.note_gap_y;
                let label = measure_sequence_text(&note.label, theme, config);
                let mut width =
                    (label.width + geometry.note_padding_x * 2.0).max(geometry.actor_min_width);
                let height = label.height + geometry.note_padding_y * 2.0;
                let mut lifeline_xs = note
                    .participants
                    .iter()
                    .filter_map(|id| nodes.get(id))
                    .map(sequence_lane_center)
                    .collect::<Vec<_>>();
                if lifeline_xs.is_empty() {
                    lifeline_xs.push(0.0);
                }
                let base_x = lifeline_xs[0];
                let min_x = lifeline_xs.iter().copied().fold(f32::INFINITY, f32::min);
                let max_x = lifeline_xs
                    .iter()
                    .copied()
                    .fold(f32::NEG_INFINITY, f32::max);
                if note.position == crate::ir::SequenceNotePosition::Over
                    && note.participants.len() > 1
                {
                    let span = (max_x - min_x).abs();
                    width = width.max(span + geometry.note_gap_x * 2.0);
                }
                let x = match note.position {
                    crate::ir::SequenceNotePosition::LeftOf => base_x - geometry.note_gap_x - width,
                    crate::ir::SequenceNotePosition::RightOf => base_x + geometry.note_gap_x,
                    crate::ir::SequenceNotePosition::Over => (min_x + max_x) / 2.0 - width / 2.0,
                };
                sequence_notes.push(SequenceNoteLayout {
                    x,
                    y: message_cursor,
                    width,
                    height,
                    label,
                    position: note.position,
                    participants: note.participants.clone(),
                    index: note.index,
                });
                message_cursor += height;
            }
        }
        message_cursor += frame_end_padding
            .remove(&(idx, sequence_notes.len()))
            .unwrap_or(0.0);
        if idx < graph.edges.len() {
            message_cursor += extra_before[idx] + message_row_spacing[idx];
            message_ys.push(message_cursor);
            if graph.edges[idx].from == graph.edges[idx].to {
                message_cursor += geometry.note_gap_x * 1.4;
            }
        }
    }

    for (idx, edge) in graph.edges.iter().enumerate() {
        let from = nodes.get(&edge.from).expect("from node missing");
        let to = nodes.get(&edge.to).expect("to node missing");
        let y = message_ys.get(idx).copied().unwrap_or(message_cursor);
        let label = edge
            .label
            .as_ref()
            .map(|l| measure_sequence_text(l, theme, config));
        let start_label = edge
            .start_label
            .as_ref()
            .map(|l| measure_sequence_text(l, theme, config));
        let end_label = edge
            .end_label
            .as_ref()
            .map(|l| measure_sequence_text(l, theme, config));

        let points = if edge.from == edge.to {
            let pad = geometry.note_gap_x * 1.4;
            let x = sequence_lane_center(from);
            vec![(x, y), (x + pad, y), (x + pad, y + pad), (x, y + pad)]
        } else {
            vec![
                (sequence_lane_center(from), y),
                (sequence_lane_center(to), y),
            ]
        };

        let mut override_style = resolve_edge_style(idx, graph);
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
            points,
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

    let mut sequence_frames = Vec::new();
    if !graph.sequence_frames.is_empty() && !message_ys.is_empty() {
        let message_bounds = FrameBoundsIndex::new(edges.iter().map(|edge| {
            let mut bounds = FrameBounds::EMPTY;
            for id in [&edge.from, &edge.to] {
                if let Some(node) = nodes.get(id) {
                    let center = sequence_lane_center(node);
                    bounds.min_x = bounds.min_x.min(center);
                    bounds.max_x = bounds.max_x.max(center);
                }
            }
            for &(x, y) in &edge.points {
                bounds.min_x = bounds.min_x.min(x);
                bounds.max_x = bounds.max_x.max(x);
                bounds.max_y = bounds.max_y.max(y);
            }
            if let (Some(label), Some(first), Some(last)) =
                (&edge.label, edge.points.first(), edge.points.last())
            {
                let center = (first.0 + last.0) * 0.5;
                bounds.min_x = bounds.min_x.min(center - label.width * 0.5);
                bounds.max_x = bounds.max_x.max(center + label.width * 0.5);
            }
            bounds
        }));
        let note_bounds = FrameBoundsIndex::new(sequence_notes.iter().map(|note| FrameBounds {
            min_x: note.x,
            max_x: note.x + note.width,
            min_y: note.y,
            max_y: note.y + note.height,
        }));
        let mut frames: Vec<_> = graph.sequence_frames.iter().collect();
        frames.sort_by(|a, b| {
            a.start_idx
                .cmp(&b.start_idx)
                .then_with(|| b.end_idx.cmp(&a.end_idx))
        });
        for frame in frames {
            if frame.start_idx >= frame.end_idx || frame.start_idx >= message_ys.len() {
                continue;
            }

            let messages = message_bounds.query(frame.start_idx..frame.end_idx);
            let notes = note_bounds.query(frame.note_range.clone());
            let min_center_x = messages.min_x.min(notes.min_x);
            let max_center_x = messages.max_x.max(notes.max_x);
            if !min_center_x.is_finite() || !max_center_x.is_finite() {
                continue;
            }
            let frame_pad_x = (theme.font_size * 0.7).max(11.0);
            let frame_x = min_center_x - frame_pad_x;
            let mut frame_width =
                (max_center_x - min_center_x) + frame_pad_x + theme.font_size * 1.05;

            let first_y = message_ys
                .get(frame.start_idx)
                .copied()
                .unwrap_or(message_cursor);
            let last_y = message_ys
                .get(frame.end_idx.saturating_sub(1))
                .copied()
                .unwrap_or(first_y);
            let min_y = first_y.min(notes.min_y);
            let max_y = last_y.max(messages.max_y).max(notes.max_y);
            let top_offset = (base_spacing * 1.8).max(theme.font_size * 3.9);
            let bottom_offset = (theme.font_size * 0.85).max(12.0);
            let frame_y = min_y - top_offset;
            let frame_height = (max_y - min_y).max(0.0) + top_offset + bottom_offset;

            let frame_label_text = match frame.kind {
                crate::ir::SequenceFrameKind::Alt => "alt",
                crate::ir::SequenceFrameKind::Opt => "opt",
                crate::ir::SequenceFrameKind::Loop => "loop",
                crate::ir::SequenceFrameKind::Par => "par",
                crate::ir::SequenceFrameKind::Rect => "rect",
                crate::ir::SequenceFrameKind::Critical => "critical",
                crate::ir::SequenceFrameKind::Break => "break",
            };
            let label_block = measure_sequence_text(frame_label_text, theme, config);
            let label_box_w =
                (label_block.width + theme.font_size * 1.2).max(theme.font_size * 3.1);
            let label_box_h = (theme.font_size * 1.25).max(20.0);
            let section_blocks: Vec<_> = frame
                .sections
                .iter()
                .map(|section| {
                    section
                        .label
                        .as_ref()
                        .map(|label| measure_sequence_text(&format!("[{label}]"), theme, config))
                })
                .collect();
            for block in section_blocks.iter().flatten() {
                frame_width = frame_width.max(label_box_w + block.width + theme.font_size * 1.5);
            }
            let label_box_x = frame_x;
            let label_box_y = frame_y;
            let label = SequenceLabel {
                x: label_box_x + label_box_w / 2.0,
                y: label_box_y + label_box_h / 2.0,
                text: label_block,
            };

            let mut dividers = Vec::new();
            let divider_offset = theme.font_size * 0.9;
            for window in frame.sections.windows(2) {
                let prev_end = window[0].end_idx;
                let base_y = message_ys
                    .get(prev_end.saturating_sub(1))
                    .copied()
                    .unwrap_or(first_y);
                dividers.push(base_y + divider_offset);
            }

            let mut section_labels = Vec::new();
            let label_offset = theme.font_size * 0.7;
            for (section_idx, block) in section_blocks.into_iter().enumerate() {
                if let Some(block) = block {
                    let label_y = if section_idx == 0 {
                        frame_y + label_box_h.max(block.height) * 0.5 + theme.font_size * 0.25
                    } else {
                        dividers
                            .get(section_idx - 1)
                            .copied()
                            .unwrap_or(frame_y + label_offset)
                            + block.height * 0.5
                            + theme.font_size * 0.3
                    };
                    let side_pad = theme.font_size * 0.45;
                    let frame_center_x = frame_x + frame_width / 2.0;
                    let clamp_or_midpoint = |preferred: f32, min_x: f32, max_x: f32| {
                        if min_x <= max_x {
                            preferred.clamp(min_x, max_x)
                        } else {
                            (min_x + max_x) / 2.0
                        }
                    };
                    let label_x = if section_idx == 0 {
                        let preferred =
                            frame_x + label_box_w + theme.font_size * 3.0 + block.width / 2.0;
                        let min_x =
                            frame_x + label_box_w + block.width / 2.0 + theme.font_size * 0.4;
                        let max_x =
                            frame_x + frame_width - block.width / 2.0 - theme.font_size * 0.4;
                        clamp_or_midpoint(preferred, min_x, max_x)
                    } else {
                        let preferred = frame_center_x;
                        let min_x = frame_x + block.width / 2.0 + side_pad;
                        let max_x = frame_x + frame_width - block.width / 2.0 - side_pad;
                        clamp_or_midpoint(preferred, min_x, max_x)
                    };
                    section_labels.push(SequenceLabel {
                        x: label_x,
                        y: label_y,
                        text: block,
                    });
                }
            }

            sequence_frames.push(SequenceFrameLayout {
                kind: frame.kind,
                x: frame_x,
                y: frame_y,
                width: frame_width,
                height: frame_height,
                label_box: (label_box_x, label_box_y, label_box_w, label_box_h),
                label,
                section_labels,
                dividers,
            });
        }
    }

    let lifeline_start = actor_height;
    let mut last_message_y = message_ys
        .last()
        .copied()
        .unwrap_or(lifeline_start + base_spacing);
    for note in &sequence_notes {
        last_message_y = last_message_y.max(note.y + note.height);
    }
    last_message_y = last_message_y.max(message_cursor);
    for frame in &sequence_frames {
        last_message_y = last_message_y.max(frame.y + frame.height);
    }
    let lifeline_end = last_message_y + geometry.footbox_gap;
    let lifelines = participants
        .iter()
        .filter_map(|id| nodes.get(id))
        .map(|node| Lifeline {
            id: node.id.clone(),
            x: sequence_lane_center(node),
            y1: lifeline_start,
            y2: lifeline_end,
        })
        .collect::<Vec<_>>();

    let sequence_footboxes = participants
        .iter()
        .filter_map(|id| nodes.get(id))
        .map(|node| {
            let mut foot = node.clone();
            foot.y = lifeline_end;
            foot
        })
        .collect::<Vec<_>>();

    let mut sequence_boxes = Vec::new();
    if !graph.sequence_boxes.is_empty() {
        let pad_x = geometry.lane_side_pad_x;
        let pad_y = theme.font_size * 0.6;
        let bottom = sequence_footboxes
            .iter()
            .map(|foot| foot.y + foot.height)
            .fold(lifeline_end, f32::max);
        for seq_box in &graph.sequence_boxes {
            let mut min_center_x = f32::INFINITY;
            let mut max_center_x = f32::NEG_INFINITY;
            for participant in &seq_box.participants {
                if let Some(node) = nodes.get(participant) {
                    let center = sequence_lane_center(node);
                    min_center_x = min_center_x.min(center);
                    max_center_x = max_center_x.max(center);
                }
            }
            if !min_center_x.is_finite() || !max_center_x.is_finite() {
                continue;
            }
            let x = min_center_x - pad_x;
            let width = (max_center_x - min_center_x) + pad_x * 2.0;
            let label = seq_box
                .label
                .as_ref()
                .map(|text| measure_sequence_text(text, theme, config));
            sequence_boxes.push(SequenceBoxLayout {
                x,
                y: 0.0,
                width,
                height: bottom + pad_y,
                label,
                color: seq_box.color.clone(),
            });
        }
    }

    let activation_width = (theme.font_size * 0.625).max(10.0);
    let activation_offset = (activation_width * 0.6).max(4.0);
    let activation_end_default = message_ys
        .last()
        .copied()
        .unwrap_or(lifeline_start + base_spacing * 0.5)
        + base_spacing * 0.6;
    let mut sequence_activations = Vec::new();
    let mut activation_stacks: HashMap<String, Vec<(f32, usize)>> = HashMap::new();
    let mut events = graph
        .sequence_activations
        .iter()
        .cloned()
        .enumerate()
        .map(|(order, event)| (event.index, order, event))
        .collect::<Vec<_>>();
    events.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let activation_y_for = |idx: usize| {
        if idx < message_ys.len() {
            message_ys[idx]
        } else {
            activation_end_default
        }
    };
    for (_, _, event) in events {
        let y = activation_y_for(event.index);
        let stack = activation_stacks
            .entry(event.participant.clone())
            .or_default();
        match event.kind {
            crate::ir::SequenceActivationKind::Activate => {
                let depth = stack.len();
                stack.push((y, depth));
            }
            crate::ir::SequenceActivationKind::Deactivate => {
                if let Some((start_y, depth)) = stack.pop()
                    && let Some(node) = nodes.get(&event.participant)
                {
                    let base_x = sequence_lane_center(node) - activation_width / 2.0;
                    let x = base_x + depth as f32 * activation_offset;
                    let mut y0 = start_y.min(y);
                    let mut height = (y - start_y).abs();
                    if height < base_spacing * 0.6 {
                        height = base_spacing * 0.6;
                    }
                    if y0 < lifeline_start {
                        y0 = lifeline_start;
                    }
                    sequence_activations.push(SequenceActivationLayout {
                        x,
                        y: y0,
                        width: activation_width,
                        height,
                        participant: event.participant.clone(),
                        depth,
                    });
                }
            }
        }
    }
    for (participant, stack) in activation_stacks {
        for (start_y, depth) in stack {
            if let Some(node) = nodes.get(&participant) {
                let base_x = sequence_lane_center(node) - activation_width / 2.0;
                let x = base_x + depth as f32 * activation_offset;
                let mut y0 = start_y.min(activation_end_default);
                let mut height = (activation_end_default - start_y).abs();
                if height < base_spacing * 0.6 {
                    height = base_spacing * 0.6;
                }
                if y0 < lifeline_start {
                    y0 = lifeline_start;
                }
                sequence_activations.push(SequenceActivationLayout {
                    x,
                    y: y0,
                    width: activation_width,
                    height,
                    participant: participant.clone(),
                    depth,
                });
            }
        }
    }

    let mut sequence_numbers = Vec::new();
    if let Some(start) = graph.sequence_autonumber {
        let mut value = start;
        for (idx, edge) in graph.edges.iter().enumerate() {
            if let (Some(from), Some(y)) = (nodes.get(&edge.from), message_ys.get(idx).copied()) {
                let from_x = sequence_lane_center(from);
                let to_x = nodes
                    .get(&edge.to)
                    .map(sequence_lane_center)
                    .unwrap_or(from_x);
                let offset = if to_x >= from_x { 16.0 } else { -16.0 };
                let number_y = y - (theme.font_size * 0.85).max(10.0);
                sequence_numbers.push(SequenceNumberLayout {
                    x: from_x + offset,
                    y: number_y,
                    value,
                });
                value += 1;
            }
        }
    }

    let mut layout = Layout {
        kind: graph.kind,
        nodes,
        edges,
        subgraphs,
        width: 1.0,
        height: 1.0,
        diagram: DiagramData::Sequence(SequenceData {
            lifelines,
            footboxes: sequence_footboxes,
            boxes: sequence_boxes,
            frames: sequence_frames,
            notes: sequence_notes,
            activations: sequence_activations,
            numbers: sequence_numbers,
        }),
    };
    finalize_sequence_layout_bounds(&mut layout);
    layout
}

pub(super) fn resolve_sequence_label_positions(layout: &mut Layout, theme: &Theme) {
    let Layout {
        nodes,
        edges,
        diagram,
        ..
    } = layout;
    let DiagramData::Sequence(seq) = diagram else {
        return;
    };
    place_sequence_label_anchors(
        edges,
        nodes,
        &seq.lifelines,
        &seq.footboxes,
        &seq.frames,
        &seq.notes,
        &seq.activations,
        &seq.numbers,
        theme,
    );
}

fn place_sequence_label_anchors(
    edges: &mut [EdgeLayout],
    nodes: &BTreeMap<String, NodeLayout>,
    lifelines: &[Lifeline],
    footboxes: &[NodeLayout],
    frames: &[SequenceFrameLayout],
    notes: &[SequenceNoteLayout],
    activations: &[SequenceActivationLayout],
    numbers: &[SequenceNumberLayout],
    theme: &Theme,
) {
    if edges.is_empty() {
        return;
    }

    let mut occupied: Vec<Rect> = Vec::new();
    for node in nodes.values() {
        occupied.push((node.x, node.y, node.width, node.height));
    }
    for lifeline in lifelines {
        occupied.push((
            lifeline.x - 1.5,
            lifeline.y1,
            3.0,
            (lifeline.y2 - lifeline.y1).max(0.0),
        ));
    }
    for footbox in footboxes {
        occupied.push((footbox.x, footbox.y, footbox.width, footbox.height));
    }
    for frame in frames {
        occupied.push(frame.label_box);
        let line_pad = 1.5;
        occupied.push((
            frame.x - line_pad,
            frame.y - line_pad,
            frame.width + line_pad * 2.0,
            line_pad * 2.0,
        ));
        occupied.push((
            frame.x - line_pad,
            frame.y + frame.height - line_pad,
            frame.width + line_pad * 2.0,
            line_pad * 2.0,
        ));
        occupied.push((
            frame.x - line_pad,
            frame.y - line_pad,
            line_pad * 2.0,
            frame.height + line_pad * 2.0,
        ));
        occupied.push((
            frame.x + frame.width - line_pad,
            frame.y - line_pad,
            line_pad * 2.0,
            frame.height + line_pad * 2.0,
        ));
        let section_pad_x = (theme.font_size * 0.18).max(1.5);
        let section_pad_y = (theme.font_size * 0.15).max(1.2);
        for label in &frame.section_labels {
            occupied.push((
                label.x - label.text.width / 2.0 - section_pad_x,
                label.y - label.text.height / 2.0 - section_pad_y,
                label.text.width + section_pad_x * 2.0,
                label.text.height + section_pad_y * 2.0,
            ));
        }
        for divider in &frame.dividers {
            occupied.push((frame.x, *divider - line_pad, frame.width, line_pad * 2.0));
        }
    }
    for note in notes {
        occupied.push((note.x, note.y, note.width, note.height));
    }
    for activation in activations {
        occupied.push((
            activation.x,
            activation.y,
            activation.width,
            activation.height,
        ));
    }
    let number_r = (theme.font_size * 0.45).max(6.0);
    for number in numbers {
        occupied.push((
            number.x - number_r,
            number.y - number_r,
            number_r * 2.0,
            number_r * 2.0,
        ));
    }

    let edge_paths: Vec<Vec<(f32, f32)>> = edges.iter().map(|edge| edge.points.clone()).collect();
    for idx in 0..edges.len() {
        if let Some(label) = edges[idx].label.clone() {
            let anchor = choose_sequence_center_label_anchor(
                &edge_paths[idx],
                &label,
                &occupied,
                &edge_paths,
                idx,
                theme,
            );
            edges[idx].label_anchor = Some(anchor);
            occupied.push(label_rect(
                anchor,
                &label,
                SEQUENCE_LABEL_PAD_X,
                SEQUENCE_LABEL_PAD_Y,
            ));
        }

        if let Some(label) = edges[idx].start_label.clone() {
            let anchor = choose_sequence_endpoint_label_anchor(
                &edge_paths[idx],
                &label,
                true,
                &occupied,
                &edge_paths,
                idx,
                theme,
            );
            edges[idx].start_label_anchor = anchor;
            if let Some(center) = anchor {
                occupied.push(label_rect(
                    center,
                    &label,
                    SEQUENCE_ENDPOINT_LABEL_PAD_X,
                    SEQUENCE_ENDPOINT_LABEL_PAD_Y,
                ));
            }
        }

        if let Some(label) = edges[idx].end_label.clone() {
            let anchor = choose_sequence_endpoint_label_anchor(
                &edge_paths[idx],
                &label,
                false,
                &occupied,
                &edge_paths,
                idx,
                theme,
            );
            edges[idx].end_label_anchor = anchor;
            if let Some(center) = anchor {
                occupied.push(label_rect(
                    center,
                    &label,
                    SEQUENCE_ENDPOINT_LABEL_PAD_X,
                    SEQUENCE_ENDPOINT_LABEL_PAD_Y,
                ));
            }
        }
    }
}

fn choose_sequence_center_label_anchor(
    points: &[(f32, f32)],
    label: &TextBlock,
    occupied: &[Rect],
    edge_paths: &[Vec<(f32, f32)>],
    edge_idx: usize,
    theme: &Theme,
) -> (f32, f32) {
    let (anchor, dir) = edge_midpoint_with_direction(points);
    let normal = (-dir.1, dir.0);
    let normal_step = (label.height * 0.5 + SEQUENCE_LABEL_PAD_Y).max(6.0);
    let tangent_step = (label.width + theme.font_size * 0.35).max(10.0) * 0.24;
    // Path-first search: keep center labels on their own message path and slide
    // along the path before moving off-path.
    let tangent_offsets_primary = [
        0.0, -0.25, 0.25, -0.55, 0.55, -0.95, 0.95, -1.45, 1.45, -2.1, 2.1, -2.9, 2.9, -3.8, 3.8,
        -4.9, 4.9, -6.2, 6.2,
    ];
    let tangent_offsets_wide = [
        0.0, -0.35, 0.35, -0.75, 0.75, -1.3, 1.3, -2.0, 2.0, -2.9, 2.9, -4.0, 4.0, -5.3, 5.3, -6.8,
        6.8, -8.4, 8.4,
    ];
    let normal_offsets_optimal = [-1.2, 1.2, -1.35, 1.35, -1.5, 1.5];
    let normal_offsets_near_optimal = [-1.05, 1.05, -1.8, 1.8, -2.25, 2.25];
    let normal_offsets_fallback = [
        -0.9, 0.9, -2.7, 2.7, -3.4, 3.4, -4.4, 4.4, -5.8, 5.8, -7.4, 7.4,
    ];
    let mut best = anchor;
    let mut best_score = f32::INFINITY;

    let mut evaluate_band = |tangent_offsets: &[f32], normal_offsets: &[f32]| {
        for t in tangent_offsets {
            for n in normal_offsets {
                let center = (
                    anchor.0 + dir.0 * tangent_step * *t + normal.0 * normal_step * *n,
                    anchor.1 + dir.1 * tangent_step * *t + normal.1 * normal_step * *n,
                );
                let rect = label_rect(center, label, SEQUENCE_LABEL_PAD_X, SEQUENCE_LABEL_PAD_Y);
                let mut score = sequence_label_penalty(
                    rect,
                    center,
                    anchor,
                    points,
                    label.height,
                    occupied,
                    SequenceLabelPlacementMode::Center,
                );
                score += sequence_edge_overlap_penalty(rect, edge_paths, edge_idx);
                let own_dist = point_to_polyline_distance(center, points);
                score += own_dist * 0.045;
                // Keep center labels near message midpoint. We still allow drift
                // when required to resolve overlaps, but large tangent shifts are
                // strongly discouraged versus vertical escape.
                let tangent_abs = t.abs();
                score += tangent_abs * SEQUENCE_CENTER_LABEL_TANGENT_LINEAR_WEIGHT;
                score += tangent_abs * tangent_abs * SEQUENCE_CENTER_LABEL_TANGENT_QUAD_WEIGHT;
                if tangent_abs > SEQUENCE_CENTER_LABEL_TANGENT_SOFT_LIMIT {
                    score += (tangent_abs - SEQUENCE_CENTER_LABEL_TANGENT_SOFT_LIMIT)
                        * SEQUENCE_CENTER_LABEL_TANGENT_FAR_WEIGHT;
                }
                if dir.0.abs() > dir.1.abs() && center.1 > anchor.1 {
                    // Keep horizontal message labels out of the row below.
                    score += 0.3;
                }
                if score < best_score {
                    best_score = score;
                    best = center;
                }
            }
        }
    };

    evaluate_band(&tangent_offsets_primary, &normal_offsets_optimal);
    evaluate_band(&tangent_offsets_primary, &normal_offsets_near_optimal);
    evaluate_band(&tangent_offsets_wide, &normal_offsets_fallback);

    best
}

fn choose_sequence_endpoint_label_anchor(
    points: &[(f32, f32)],
    label: &TextBlock,
    start: bool,
    occupied: &[Rect],
    edge_paths: &[Vec<(f32, f32)>],
    edge_idx: usize,
    theme: &Theme,
) -> Option<(f32, f32)> {
    let ((anchor_x, anchor_y), dir) = sequence_endpoint_base(points, start, theme)?;
    let normal = (-dir.1, dir.0);
    let base_step = (theme.font_size * 0.45).max(6.0);
    let tangent_offsets = [0.0, 0.6, -0.6, 1.2, -1.2, 2.0, -2.0, 2.9, -2.9];
    let normal_offsets = [0.35, -0.35, 0.75, -0.75, 1.1, -1.1, 1.45, -1.45, 1.8, -1.8];
    let anchor = (anchor_x, anchor_y);
    let mut best = anchor;
    let mut best_score = f32::INFINITY;

    for t in tangent_offsets {
        for n in normal_offsets {
            let center = (
                anchor.0 + dir.0 * base_step * t + normal.0 * base_step * n,
                anchor.1 + dir.1 * base_step * t + normal.1 * base_step * n,
            );
            let rect = label_rect(
                center,
                label,
                SEQUENCE_ENDPOINT_LABEL_PAD_X,
                SEQUENCE_ENDPOINT_LABEL_PAD_Y,
            );
            let mut score = sequence_label_penalty(
                rect,
                center,
                anchor,
                points,
                label.height,
                occupied,
                SequenceLabelPlacementMode::Endpoint,
            );
            score += sequence_edge_overlap_penalty(rect, edge_paths, edge_idx);
            score += distance(center, anchor) * 0.05;
            if score < best_score {
                best_score = score;
                best = center;
            }
        }
    }

    Some(best)
}

fn sequence_endpoint_base(
    points: &[(f32, f32)],
    start: bool,
    theme: &Theme,
) -> Option<((f32, f32), (f32, f32))> {
    if points.len() < 2 {
        return None;
    }
    let (p0, p1) = if start {
        (points[0], points[1])
    } else {
        (points[points.len() - 1], points[points.len() - 2])
    };
    let dx = p1.0 - p0.0;
    let dy = p1.1 - p0.1;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= f32::EPSILON {
        return None;
    }
    let dir = (dx / len, dy / len);
    let offset = (theme.font_size * 0.45).max(6.0);
    let anchor = (p0.0 + dir.0 * offset * 1.4, p0.1 + dir.1 * offset * 1.4);
    Some((anchor, dir))
}

fn edge_midpoint_with_direction(points: &[(f32, f32)]) -> ((f32, f32), (f32, f32)) {
    if points.len() < 2 {
        let point = points.first().copied().unwrap_or((0.0, 0.0));
        return (point, (1.0, 0.0));
    }
    let mut lengths = Vec::with_capacity(points.len().saturating_sub(1));
    let mut total = 0.0f32;
    for segment in points.windows(2) {
        let len = distance(segment[0], segment[1]);
        lengths.push(len);
        total += len;
    }
    if total <= f32::EPSILON {
        let dx = points[1].0 - points[0].0;
        let dy = points[1].1 - points[0].1;
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        return (points[0], (dx / len, dy / len));
    }
    let target = total * 0.5;
    let mut acc = 0.0f32;
    for (idx, len) in lengths.iter().copied().enumerate() {
        if acc + len >= target {
            let seg = (points[idx], points[idx + 1]);
            let local_t = ((target - acc) / len.max(1e-6)).clamp(0.0, 1.0);
            let point = (
                seg.0.0 + (seg.1.0 - seg.0.0) * local_t,
                seg.0.1 + (seg.1.1 - seg.0.1) * local_t,
            );
            let dx = seg.1.0 - seg.0.0;
            let dy = seg.1.1 - seg.0.1;
            let dlen = (dx * dx + dy * dy).sqrt().max(1e-6);
            return (point, (dx / dlen, dy / dlen));
        }
        acc += len;
    }
    let last = points[points.len() - 1];
    let prev = points[points.len() - 2];
    let dx = last.0 - prev.0;
    let dy = last.1 - prev.1;
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    (last, (dx / len, dy / len))
}

fn sequence_label_penalty(
    rect: Rect,
    center: (f32, f32),
    anchor: (f32, f32),
    own_points: &[(f32, f32)],
    label_height: f32,
    occupied: &[Rect],
    mode: SequenceLabelPlacementMode,
) -> f32 {
    let mut overlap_area_sum = 0.0f32;
    let mut overlap_count = 0usize;
    for obstacle in occupied {
        let area = rect_overlap_area(rect, *obstacle);
        if area > 0.0 {
            overlap_count += 1;
            overlap_area_sum += area;
        }
    }
    let own_gap = polyline_rect_gap(own_points, rect);
    let gap_penalty = match mode {
        SequenceLabelPlacementMode::Center => {
            let (target_gap, too_close_limit) = sequence_center_gap_profile(label_height);
            if !own_gap.is_finite() {
                150.0
            } else if own_gap <= too_close_limit {
                140.0 + (too_close_limit - own_gap).max(0.0) * 28.0
            } else if own_gap < SEQUENCE_CENTER_LABEL_GAP_MIN {
                let delta =
                    (SEQUENCE_CENTER_LABEL_GAP_MIN - own_gap) / SEQUENCE_CENTER_LABEL_GAP_MIN;
                delta * delta * 22.0
            } else if own_gap <= SEQUENCE_CENTER_LABEL_GAP_MAX {
                let delta = (own_gap - target_gap) / target_gap.max(1e-3);
                delta * delta * 0.85
            } else {
                let far = own_gap - SEQUENCE_CENTER_LABEL_GAP_MAX;
                let mut penalty = far * far * 1.7 + far * 0.35;
                if own_gap > SEQUENCE_CENTER_LABEL_FAR_GAP {
                    penalty += (own_gap - SEQUENCE_CENTER_LABEL_FAR_GAP) * 0.7;
                }
                penalty
            }
        }
        SequenceLabelPlacementMode::Endpoint => {
            let mut penalty = 0.0f32;
            if own_gap <= SEQUENCE_LABEL_TOUCH_EPS {
                penalty += 120.0 + (SEQUENCE_LABEL_TOUCH_EPS - own_gap).max(0.0) * 30.0;
            } else if own_gap < SEQUENCE_ENDPOINT_LABEL_GAP_MIN {
                let delta = (SEQUENCE_ENDPOINT_LABEL_GAP_MIN - own_gap)
                    / SEQUENCE_ENDPOINT_LABEL_GAP_MIN.max(1e-3);
                penalty += delta * delta * 14.0;
            } else if own_gap <= SEQUENCE_ENDPOINT_LABEL_GAP_MAX {
                let delta = (own_gap - SEQUENCE_ENDPOINT_LABEL_GAP_TARGET)
                    / SEQUENCE_ENDPOINT_LABEL_GAP_TARGET.max(1e-3);
                penalty += delta * delta * 0.9;
            } else {
                let far = own_gap - SEQUENCE_ENDPOINT_LABEL_GAP_MAX;
                penalty += far * far * 2.4 + far * 0.4;
                if own_gap > SEQUENCE_ENDPOINT_LABEL_FAR_GAP {
                    penalty += (own_gap - SEQUENCE_ENDPOINT_LABEL_FAR_GAP) * 0.9;
                }
            }
            penalty
        }
    };
    let anchor_weight = match mode {
        SequenceLabelPlacementMode::Center => 0.018,
        SequenceLabelPlacementMode::Endpoint => 0.025,
    };
    let overlap_penalty = if overlap_count == 0 {
        0.0
    } else {
        10_000.0 + overlap_count as f32 * 1_000.0 + overlap_area_sum * 4.0
    };
    overlap_penalty + gap_penalty + distance(center, anchor) * anchor_weight
}

fn sequence_center_gap_profile(label_height: f32) -> (f32, f32) {
    let h = label_height.max(1.0);
    let target = (h * 0.16).clamp(2.8, 4.8);
    let too_close = (h * 0.10).clamp(1.2, 2.6);
    (target, too_close)
}

fn sequence_edge_overlap_penalty(
    rect: Rect,
    edge_paths: &[Vec<(f32, f32)>],
    edge_idx: usize,
) -> f32 {
    let mut hits = 0usize;
    for (idx, points) in edge_paths.iter().enumerate() {
        if idx == edge_idx || points.len() < 2 {
            continue;
        }
        if points
            .windows(2)
            .any(|segment| segment_intersects_rect(segment[0], segment[1], rect))
        {
            hits += 1;
        }
    }
    if hits == 0 {
        0.0
    } else {
        1.0 + hits as f32 * 4.0
    }
}

fn label_rect(center: (f32, f32), label: &TextBlock, pad_x: f32, pad_y: f32) -> Rect {
    (
        center.0 - label.width / 2.0 - pad_x,
        center.1 - label.height / 2.0 - pad_y,
        label.width + pad_x * 2.0,
        label.height + pad_y * 2.0,
    )
}

fn rect_overlap_area(a: Rect, b: Rect) -> f32 {
    let x1 = a.0.max(b.0);
    let y1 = a.1.max(b.1);
    let x2 = (a.0 + a.2).min(b.0 + b.2);
    let y2 = (a.1 + a.3).min(b.1 + b.3);
    if x2 <= x1 || y2 <= y1 {
        return 0.0;
    }
    (x2 - x1) * (y2 - y1)
}

fn point_to_polyline_distance(point: (f32, f32), points: &[(f32, f32)]) -> f32 {
    if points.is_empty() {
        return 0.0;
    }
    if points.len() == 1 {
        return distance(point, points[0]);
    }
    points
        .windows(2)
        .map(|segment| point_to_segment_distance(point, segment[0], segment[1]))
        .fold(f32::INFINITY, f32::min)
}

fn point_rect_distance(point: (f32, f32), rect: Rect) -> f32 {
    let min_x = rect.0;
    let min_y = rect.1;
    let max_x = rect.0 + rect.2;
    let max_y = rect.1 + rect.3;
    let dx = if point.0 < min_x {
        min_x - point.0
    } else if point.0 > max_x {
        point.0 - max_x
    } else {
        0.0
    };
    let dy = if point.1 < min_y {
        min_y - point.1
    } else if point.1 > max_y {
        point.1 - max_y
    } else {
        0.0
    };
    (dx * dx + dy * dy).sqrt()
}

fn segment_rect_distance(a: (f32, f32), b: (f32, f32), rect: Rect) -> f32 {
    if segment_intersects_rect(a, b, rect) {
        return 0.0;
    }
    let mut best = point_rect_distance(a, rect).min(point_rect_distance(b, rect));
    let corners = [
        (rect.0, rect.1),
        (rect.0 + rect.2, rect.1),
        (rect.0 + rect.2, rect.1 + rect.3),
        (rect.0, rect.1 + rect.3),
    ];
    for corner in corners {
        best = best.min(point_to_segment_distance(corner, a, b));
    }
    best
}

fn polyline_rect_gap(points: &[(f32, f32)], rect: Rect) -> f32 {
    if points.len() < 2 {
        return f32::INFINITY;
    }
    points
        .windows(2)
        .map(|segment| segment_rect_distance(segment[0], segment[1], rect))
        .fold(f32::INFINITY, f32::min)
}

fn point_to_segment_distance(point: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let ab = (b.0 - a.0, b.1 - a.1);
    let len_sq = ab.0 * ab.0 + ab.1 * ab.1;
    if len_sq <= f32::EPSILON {
        return distance(point, a);
    }
    let ap = (point.0 - a.0, point.1 - a.1);
    let t = ((ap.0 * ab.0 + ap.1 * ab.1) / len_sq).clamp(0.0, 1.0);
    let proj = (a.0 + ab.0 * t, a.1 + ab.1 * t);
    distance(point, proj)
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx * dx + dy * dy).sqrt()
}

fn segment_intersects_rect(a: (f32, f32), b: (f32, f32), rect: Rect) -> bool {
    let (x, y, w, h) = rect;
    let min_x = a.0.min(b.0);
    let max_x = a.0.max(b.0);
    let min_y = a.1.min(b.1);
    let max_y = a.1.max(b.1);
    if max_x < x || min_x > x + w || max_y < y || min_y > y + h {
        return false;
    }
    if point_in_rect(a, rect) || point_in_rect(b, rect) {
        return true;
    }
    let corners = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
    for i in 0..4 {
        let c = corners[i];
        let d = corners[(i + 1) % 4];
        if segments_intersect(a, b, c, d) {
            return true;
        }
    }
    false
}

fn point_in_rect(point: (f32, f32), rect: Rect) -> bool {
    point.0 >= rect.0
        && point.0 <= rect.0 + rect.2
        && point.1 >= rect.1
        && point.1 <= rect.1 + rect.3
}

fn segments_intersect(a: (f32, f32), b: (f32, f32), c: (f32, f32), d: (f32, f32)) -> bool {
    const EPS: f32 = 1e-6;
    let o1 = orient(a, b, c);
    let o2 = orient(a, b, d);
    let o3 = orient(c, d, a);
    let o4 = orient(c, d, b);

    if o1.abs() < EPS && on_segment(a, b, c) {
        return true;
    }
    if o2.abs() < EPS && on_segment(a, b, d) {
        return true;
    }
    if o3.abs() < EPS && on_segment(c, d, a) {
        return true;
    }
    if o4.abs() < EPS && on_segment(c, d, b) {
        return true;
    }
    (o1 > 0.0) != (o2 > 0.0) && (o3 > 0.0) != (o4 > 0.0)
}

fn orient(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

fn on_segment(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    const EPS: f32 = 1e-6;
    c.0 >= a.0.min(b.0) - EPS
        && c.0 <= a.0.max(b.0) + EPS
        && c.1 >= a.1.min(b.1) - EPS
        && c.1 <= a.1.max(b.1) + EPS
}

fn extend_bounds(
    min_x: &mut f32,
    min_y: &mut f32,
    max_x: &mut f32,
    max_y: &mut f32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) {
    *min_x = (*min_x).min(x);
    *min_y = (*min_y).min(y);
    *max_x = (*max_x).max(x + w);
    *max_y = (*max_y).max(y + h);
}

/// Recompute sequence canvas bounds after label placement and translate the
/// diagram as needed to keep its geometry within the measured canvas.
pub(super) fn finalize_sequence_layout_bounds(layout: &mut Layout) {
    let DiagramData::Sequence(seq) = &mut layout.diagram else {
        return;
    };

    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for node in layout.nodes.values() {
        extend_bounds(
            &mut min_x,
            &mut min_y,
            &mut max_x,
            &mut max_y,
            node.x,
            node.y,
            node.width,
            node.height,
        );
    }
    for footbox in &seq.footboxes {
        extend_bounds(
            &mut min_x,
            &mut min_y,
            &mut max_x,
            &mut max_y,
            footbox.x,
            footbox.y,
            footbox.width,
            footbox.height,
        );
    }
    for seq_box in &seq.boxes {
        extend_bounds(
            &mut min_x,
            &mut min_y,
            &mut max_x,
            &mut max_y,
            seq_box.x,
            seq_box.y,
            seq_box.width,
            seq_box.height,
        );
    }
    for frame in &seq.frames {
        extend_bounds(
            &mut min_x,
            &mut min_y,
            &mut max_x,
            &mut max_y,
            frame.x,
            frame.y,
            frame.width,
            frame.height,
        );
    }
    for note in &seq.notes {
        extend_bounds(
            &mut min_x,
            &mut min_y,
            &mut max_x,
            &mut max_y,
            note.x,
            note.y,
            note.width,
            note.height,
        );
    }
    for activation in &seq.activations {
        extend_bounds(
            &mut min_x,
            &mut min_y,
            &mut max_x,
            &mut max_y,
            activation.x,
            activation.y,
            activation.width,
            activation.height,
        );
    }
    for number in &seq.numbers {
        extend_bounds(
            &mut min_x, &mut min_y, &mut max_x, &mut max_y, number.x, number.y, 0.0, 0.0,
        );
    }
    for edge in &layout.edges {
        for point in &edge.points {
            extend_bounds(
                &mut min_x, &mut min_y, &mut max_x, &mut max_y, point.0, point.1, 0.0, 0.0,
            );
        }
        if let (Some(label), Some((x, y))) = (&edge.label, edge.label_anchor) {
            extend_bounds(
                &mut min_x,
                &mut min_y,
                &mut max_x,
                &mut max_y,
                x - label.width / 2.0 - SEQUENCE_LABEL_PAD_X,
                y - label.height / 2.0 - SEQUENCE_LABEL_PAD_Y,
                label.width + 2.0 * SEQUENCE_LABEL_PAD_X,
                label.height + 2.0 * SEQUENCE_LABEL_PAD_Y,
            );
        }
        if let (Some(label), Some((x, y))) = (&edge.start_label, edge.start_label_anchor) {
            extend_bounds(
                &mut min_x,
                &mut min_y,
                &mut max_x,
                &mut max_y,
                x - label.width / 2.0 - SEQUENCE_ENDPOINT_LABEL_PAD_X,
                y - label.height / 2.0 - SEQUENCE_ENDPOINT_LABEL_PAD_Y,
                label.width + 2.0 * SEQUENCE_ENDPOINT_LABEL_PAD_X,
                label.height + 2.0 * SEQUENCE_ENDPOINT_LABEL_PAD_Y,
            );
        }
        if let (Some(label), Some((x, y))) = (&edge.end_label, edge.end_label_anchor) {
            extend_bounds(
                &mut min_x,
                &mut min_y,
                &mut max_x,
                &mut max_y,
                x - label.width / 2.0 - SEQUENCE_ENDPOINT_LABEL_PAD_X,
                y - label.height / 2.0 - SEQUENCE_ENDPOINT_LABEL_PAD_Y,
                label.width + 2.0 * SEQUENCE_ENDPOINT_LABEL_PAD_X,
                label.height + 2.0 * SEQUENCE_ENDPOINT_LABEL_PAD_Y,
            );
        }
    }

    if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
        layout.width = 1.0;
        layout.height = 1.0;
        return;
    }

    let content_margin = 0.0;
    let shift_x = content_margin - min_x;
    let shift_y = content_margin - min_y;
    if shift_x.abs() > 1e-3 || shift_y.abs() > 1e-3 {
        for node in layout.nodes.values_mut() {
            node.x += shift_x;
            node.y += shift_y;
        }
        for edge in &mut layout.edges {
            for point in &mut edge.points {
                point.0 += shift_x;
                point.1 += shift_y;
            }
            if let Some((x, y)) = edge.label_anchor {
                edge.label_anchor = Some((x + shift_x, y + shift_y));
            }
            if let Some((x, y)) = edge.start_label_anchor {
                edge.start_label_anchor = Some((x + shift_x, y + shift_y));
            }
            if let Some((x, y)) = edge.end_label_anchor {
                edge.end_label_anchor = Some((x + shift_x, y + shift_y));
            }
        }
        for lifeline in &mut seq.lifelines {
            lifeline.x += shift_x;
            lifeline.y1 += shift_y;
            lifeline.y2 += shift_y;
        }
        for footbox in &mut seq.footboxes {
            footbox.x += shift_x;
            footbox.y += shift_y;
        }
        for seq_box in &mut seq.boxes {
            seq_box.x += shift_x;
            seq_box.y += shift_y;
        }
        for frame in &mut seq.frames {
            frame.x += shift_x;
            frame.y += shift_y;
            frame.label_box.0 += shift_x;
            frame.label_box.1 += shift_y;
            frame.label.x += shift_x;
            frame.label.y += shift_y;
            for label in &mut frame.section_labels {
                label.x += shift_x;
                label.y += shift_y;
            }
            for divider in &mut frame.dividers {
                *divider += shift_y;
            }
        }
        for note in &mut seq.notes {
            note.x += shift_x;
            note.y += shift_y;
        }
        for activation in &mut seq.activations {
            activation.x += shift_x;
            activation.y += shift_y;
        }
        for number in &mut seq.numbers {
            number.x += shift_x;
            number.y += shift_y;
        }
        min_x += shift_x;
        min_y += shift_y;
        max_x += shift_x;
        max_y += shift_y;
    }

    layout.width = (max_x - min_x + content_margin * 2.0).max(1.0);
    layout.height = (max_y - min_y + content_margin * 2.0).max(1.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_bounds_queries_match_direct_scan_for_nested_and_invalid_ranges() {
        for count in [0, 1, 7, 32, 101] {
            let bounds: Vec<_> = (0..count)
                .map(|i| FrameBounds {
                    min_x: (i * 17 % 23) as f32,
                    max_x: (i * 11 % 29) as f32 + 30.0,
                    min_y: i as f32 * 50.0,
                    max_y: i as f32 * 50.0 + 40.0,
                })
                .collect();
            let index = FrameBoundsIndex::new(bounds.iter().copied());
            assert_eq!(index.tree.len(), count * 2);
            for start in 0..count + 3 {
                for end in 0..count + 3 {
                    let expected = bounds
                        .iter()
                        .copied()
                        .enumerate()
                        .filter(|(i, _)| (start..end).contains(i))
                        .fold(FrameBounds::EMPTY, |acc, (_, b)| acc.merge(b));
                    assert_eq!(index.query(start..end), expected);
                }
            }
            assert_eq!(index.query(usize::MAX..usize::MAX), FrameBounds::EMPTY);
        }
    }

    #[test]
    #[ignore = "manual scaling benchmark; run optimized with --nocapture"]
    fn performance_scaling_nested_frames() {
        use std::hint::black_box;
        for count in [128, 512, 2048] {
            let mut source = String::from("sequenceDiagram\nparticipant A\nparticipant B\n");
            for _ in 0..count {
                source.push_str("loop condition\nnote right of A: note\nA->>B: message\n");
            }
            for _ in 0..count {
                source.push_str("end\n");
            }
            let parsed = crate::parser::parse_mermaid(&source).unwrap();
            let config = LayoutConfig {
                fast_text_metrics: true,
                ..LayoutConfig::default()
            };
            let theme = Theme::modern();
            let mut samples = Vec::new();
            for _ in 0..3 {
                let start = std::time::Instant::now();
                black_box(compute_sequence_layout(
                    black_box(&parsed.graph),
                    &theme,
                    &config,
                ));
                samples.push(start.elapsed());
            }
            samples.sort();
            println!(
                "nested_frames,{count},{:.3}",
                samples[1].as_secs_f64() * 1000.0
            );
        }
    }

    #[test]
    fn sequence_center_label_prefers_optimal_gap_band() {
        let points = vec![(0.0, 0.0), (140.0, 0.0)];
        let label = TextBlock {
            lines: vec!["msg".to_string()],
            width: 36.0,
            height: 14.0,
        };
        let theme = Theme::mermaid_default();
        let anchor = choose_sequence_center_label_anchor(
            &points,
            &label,
            &[],
            std::slice::from_ref(&points),
            0,
            &theme,
        );
        let rect = label_rect(anchor, &label, SEQUENCE_LABEL_PAD_X, SEQUENCE_LABEL_PAD_Y);
        let gap = polyline_rect_gap(&points, rect);
        let (target_gap, too_close_limit) = sequence_center_gap_profile(label.height);
        assert!(
            gap > too_close_limit,
            "expected center label to keep positive clearance (gap={gap:.3}, too-close={too_close_limit:.3})",
        );
        assert!(
            (gap - target_gap).abs() <= 2.0,
            "expected center label gap near target (gap={gap:.3}, target={target_gap:.3})",
        );
        assert!(
            gap <= SEQUENCE_CENTER_LABEL_GAP_MAX + 1.0,
            "expected center label to stay visually attached, got gap {:.3}",
            gap
        );
    }

    #[test]
    fn sequence_center_label_moves_off_path_when_path_is_blocked() {
        let points = vec![(0.0, 0.0), (140.0, 0.0)];
        let label = TextBlock {
            lines: vec!["msg".to_string()],
            width: 36.0,
            height: 14.0,
        };
        let theme = Theme::mermaid_default();
        let occupied = vec![(-20.0, -10.0, 180.0, 20.0)];
        let anchor = choose_sequence_center_label_anchor(
            &points,
            &label,
            &occupied,
            std::slice::from_ref(&points),
            0,
            &theme,
        );
        assert!(
            anchor.1.abs() > 4.0,
            "expected off-path fallback for blocked corridor, got y={:.2}",
            anchor.1
        );
        assert!(
            (anchor.0 - 70.0).abs() <= 8.0,
            "expected blocked fallback to stay near midpoint, got x={:.2}",
            anchor.0
        );
    }
}
