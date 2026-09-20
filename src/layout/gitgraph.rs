use super::*;

pub(super) fn compute_gitgraph_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> Layout {
    let gg = &config.gitgraph;
    let is_vertical = matches!(graph.direction, Direction::TopDown | Direction::BottomTop);
    let is_bottom_top = graph.direction == Direction::BottomTop;
    let mut branches = graph.gitgraph.branches.clone();
    if branches.is_empty() {
        branches.push(crate::ir::GitGraphBranch {
            name: gg.main_branch_name.clone(),
            order: Some(gg.main_branch_order),
            insertion_index: 0,
        });
    }

    let mut branch_entries: Vec<(crate::ir::GitGraphBranch, f32)> = branches
        .into_iter()
        .map(|branch| {
            let order = branch
                .order
                .unwrap_or_else(|| default_branch_order(branch.insertion_index));
            (branch, order)
        })
        .collect();
    branch_entries.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));

    let mut branch_pos: HashMap<String, (f32, usize, f32, f32)> = HashMap::new();
    let mut branch_layouts = Vec::new();
    let mut pos = 0.0f32;
    for (index, (branch, _order)) in branch_entries.iter().enumerate() {
        let measure_font_size = if gg.branch_label_font_size > 0.0 {
            gg.branch_label_font_size
        } else {
            theme.font_size
        };
        let (label_width, label_height) = measure_gitgraph_text(
            &branch.name,
            measure_font_size,
            gg.branch_label_line_height,
            gg.text_width_scale,
            theme.font_family.as_str(),
            config.fast_text_metrics,
            config.metrics.as_deref(),
        );
        let spacing_rotate_extra = if gg.rotate_commit_label {
            gg.branch_spacing_rotate_extra
        } else {
            0.0
        };
        let label_rotate_extra = if gg.rotate_commit_label {
            gg.branch_label_rotate_extra
        } else {
            0.0
        };
        let bg_width = label_width + gg.branch_label_bg_pad_x;
        let bg_height = label_height + gg.branch_label_bg_pad_y;
        let (bg_final_x, bg_final_y, text_x, text_y) = if is_vertical {
            let bg_x = pos - label_width / 2.0 - gg.branch_label_tb_bg_offset_x;
            let text_x = pos - label_width / 2.0 - gg.branch_label_tb_text_offset_x;
            let base_y = if is_bottom_top {
                0.0
            } else {
                gg.branch_label_tb_offset_y
            };
            (bg_x, base_y, text_x, base_y)
        } else {
            let bg_x = -label_width - gg.branch_label_bg_offset_x - label_rotate_extra;
            let bg_y = -label_height / 2.0 + gg.branch_label_bg_offset_y;
            let bg_final_x = bg_x + gg.branch_label_translate_x;
            let bg_final_y = bg_y + (pos - label_height / 2.0);
            let text_x = -label_width - gg.branch_label_text_offset_x - label_rotate_extra;
            let text_y = pos - label_height / 2.0 + gg.branch_label_text_offset_y;
            (bg_final_x, bg_final_y, text_x, text_y)
        };
        let label = GitGraphBranchLabelLayout {
            bg_x: bg_final_x,
            bg_y: bg_final_y,
            bg_width,
            bg_height,
            text_x,
            text_y,
            text_width: label_width,
            text_height: label_height,
        };
        branch_layouts.push(GitGraphBranchLayout {
            name: branch.name.clone(),
            index,
            pos,
            label,
        });
        branch_pos.insert(branch.name.clone(), (pos, index, label_width, label_height));
        let width_extra = if is_vertical { label_width / 2.0 } else { 0.0 };
        pos += gg.branch_spacing + spacing_rotate_extra + width_extra;
    }

    let mut commits = graph.gitgraph.commits.clone();
    commits.sort_by_key(|commit| commit.seq);
    let mut commit_layouts = Vec::new();
    let mut commit_pos: HashMap<String, (f32, f32)> = HashMap::new();
    let mut pos = if is_vertical { gg.default_pos } else { 0.0 };
    let mut max_pos = pos;
    let is_parallel = gg.parallel_commits;
    let mut commit_order: Vec<&crate::ir::GitGraphCommit> = commits.iter().collect();
    if is_bottom_top && is_parallel {
        gitgraph_set_parallel_bt_pos(
            &commit_order,
            gg.default_pos,
            gg.commit_step,
            gg.layout_offset,
            &branch_pos,
            &mut commit_pos,
        );
    }
    if is_bottom_top {
        commit_order.reverse();
    }

    for commit in commit_order {
        if is_parallel {
            pos = gitgraph_calculate_position(
                commit,
                graph.direction,
                gg.default_pos,
                gg.commit_step,
                &commit_pos,
            );
        }
        let (x, y, pos_with_offset) = gitgraph_commit_position(
            commit,
            pos,
            is_parallel,
            graph.direction,
            gg.layout_offset,
            &branch_pos,
        );
        let axis_pos = pos;
        let (_branch_axis_pos, branch_index, _bw, _bh) = branch_pos
            .get(&commit.branch)
            .cloned()
            .unwrap_or((0.0, 0, 0.0, 0.0));

        let show_label = gg.show_commit_label
            && commit.commit_type != crate::ir::GitGraphCommitType::CherryPick
            && (commit.commit_type != crate::ir::GitGraphCommitType::Merge || commit.custom_id);
        let label = if show_label {
            let (label_width, label_height) = measure_gitgraph_text(
                &commit.id,
                gg.commit_label_font_size,
                gg.commit_label_line_height,
                gg.text_width_scale,
                theme.font_family.as_str(),
                config.fast_text_metrics,
                config.metrics.as_deref(),
            );
            let (text_x, text_y, bg_x, bg_y, transform) = if is_vertical {
                let text_x = x - (label_width + gg.commit_label_tb_text_extra);
                let text_y = y + label_height + gg.commit_label_tb_text_offset_y;
                let bg_x = x - (label_width + gg.commit_label_tb_bg_extra);
                let bg_y = y + gg.commit_label_tb_bg_offset_y;
                let transform = if gg.rotate_commit_label {
                    Some(GitGraphTransform {
                        translate_x: 0.0,
                        translate_y: 0.0,
                        rotate_deg: gg.commit_label_rotate_angle,
                        rotate_cx: x,
                        rotate_cy: y,
                    })
                } else {
                    None
                };
                (text_x, text_y, bg_x, bg_y, transform)
            } else {
                let text_x = pos_with_offset - label_width / 2.0;
                let text_y = y + gg.commit_label_offset_y;
                let bg_x = pos_with_offset - label_width / 2.0 - gg.commit_label_padding;
                let bg_y = y + gg.commit_label_bg_offset_y;
                let transform = if gg.rotate_commit_label {
                    let rotate_x = gg.commit_label_rotate_translate_x_base
                        - (label_width + gg.commit_label_rotate_translate_x_width_offset)
                            * gg.commit_label_rotate_translate_x_scale;
                    let rotate_y = gg.commit_label_rotate_translate_y_base
                        + label_width * gg.commit_label_rotate_translate_y_scale;
                    Some(GitGraphTransform {
                        translate_x: rotate_x,
                        translate_y: rotate_y,
                        rotate_deg: gg.commit_label_rotate_angle,
                        rotate_cx: axis_pos,
                        rotate_cy: y,
                    })
                } else {
                    None
                };
                (text_x, text_y, bg_x, bg_y, transform)
            };
            let bg_width = label_width + 2.0 * gg.commit_label_padding;
            let bg_height = label_height + 2.0 * gg.commit_label_padding;
            Some(GitGraphCommitLabelLayout {
                text: commit.id.clone(),
                text_x,
                text_y,
                bg_x,
                bg_y,
                bg_width,
                bg_height,
                transform,
            })
        } else {
            None
        };

        let mut tag_layouts = Vec::new();
        if !commit.tags.is_empty() {
            let mut max_width = 0.0f32;
            let mut max_height = 0.0f32;
            let mut tag_defs = Vec::new();
            let mut y_offset = 0.0f32;
            for tag_value in commit.tags.iter().rev() {
                let (w, h) = measure_gitgraph_text(
                    tag_value,
                    gg.tag_label_font_size,
                    gg.tag_label_line_height,
                    gg.text_width_scale,
                    theme.font_family.as_str(),
                    config.fast_text_metrics,
                    config.metrics.as_deref(),
                );
                max_width = max_width.max(w);
                max_height = max_height.max(h);
                tag_defs.push((tag_value.clone(), w, y_offset));
                y_offset += gg.tag_spacing_y;
            }
            let half_h = max_height / 2.0;
            for (text, text_width, tag_offset) in tag_defs {
                if is_vertical {
                    let y_origin = axis_pos + tag_offset;
                    let px = gg.tag_padding_x;
                    let py = gg.tag_padding_y;
                    let text_translate_delta =
                        gg.tag_text_rotate_translate - gg.tag_rotate_translate;
                    let text_x = x + gg.tag_text_offset_x_tb + text_translate_delta;
                    let text_y = y_origin + gg.tag_text_offset_y_tb + text_translate_delta;
                    let points = vec![
                        (x, y_origin + py),
                        (x, y_origin - py),
                        (x + gg.layout_offset, y_origin - half_h - py),
                        (
                            x + gg.layout_offset + max_width + px,
                            y_origin - half_h - py,
                        ),
                        (
                            x + gg.layout_offset + max_width + px,
                            y_origin + half_h + py,
                        ),
                        (x + gg.layout_offset, y_origin + half_h + py),
                    ];
                    let hole_x = x + px / 2.0;
                    let hole_y = y_origin;
                    tag_layouts.push(GitGraphTagLayout {
                        text,
                        text_x,
                        text_y,
                        points,
                        hole_x,
                        hole_y,
                        transform: Some(GitGraphTransform {
                            translate_x: gg.tag_rotate_translate,
                            translate_y: gg.tag_rotate_translate,
                            rotate_deg: gg.tag_rotate_angle,
                            rotate_cx: x,
                            rotate_cy: axis_pos,
                        }),
                    });
                } else {
                    let text_x = pos_with_offset - text_width / 2.0;
                    let text_y = y - gg.tag_text_offset_y - tag_offset;
                    let ly = y - gg.tag_polygon_offset_y - tag_offset;
                    let px = gg.tag_padding_x;
                    let py = gg.tag_padding_y;
                    let points = vec![
                        (axis_pos - max_width / 2.0 - px / 2.0, ly + py),
                        (axis_pos - max_width / 2.0 - px / 2.0, ly - py),
                        (pos_with_offset - max_width / 2.0 - px, ly - half_h - py),
                        (pos_with_offset + max_width / 2.0 + px, ly - half_h - py),
                        (pos_with_offset + max_width / 2.0 + px, ly + half_h + py),
                        (pos_with_offset - max_width / 2.0 - px, ly + half_h + py),
                    ];
                    let hole_x = axis_pos - max_width / 2.0 + px / 2.0;
                    let hole_y = ly;
                    tag_layouts.push(GitGraphTagLayout {
                        text,
                        text_x,
                        text_y,
                        points,
                        hole_x,
                        hole_y,
                        transform: None,
                    });
                }
            }
        }

        commit_layouts.push(GitGraphCommitLayout {
            id: commit.id.clone(),
            seq: commit.seq,
            branch_index,
            x,
            y,
            axis_pos,
            commit_type: commit.commit_type,
            custom_type: commit.custom_type,
            tags: tag_layouts,
            label,
        });

        if is_vertical {
            commit_pos.insert(commit.id.clone(), (x, pos_with_offset));
        } else {
            commit_pos.insert(commit.id.clone(), (pos_with_offset, y));
        }
        pos = if is_bottom_top && is_parallel {
            pos + gg.commit_step
        } else {
            pos + gg.commit_step + gg.layout_offset
        };
        if pos > max_pos {
            max_pos = pos;
        }
    }

    if is_bottom_top {
        for branch in &mut branch_layouts {
            branch.label.bg_y = max_pos + gg.branch_label_bt_offset_y;
            branch.label.text_y = max_pos + gg.branch_label_bt_offset_y;
        }
    }

    let mut arrows = Vec::new();
    let mut lanes = Vec::new();
    for commit in &graph.gitgraph.commits {
        if commit.parents.is_empty() {
            continue;
        }
        for parent in &commit.parents {
            if let (Some((p1x, p1y)), Some((p2x, p2y))) =
                (commit_pos.get(parent), commit_pos.get(&commit.id))
            {
                let commit_a = commit_by_id(&graph.gitgraph.commits, parent);
                let commit_b = commit_by_id(&graph.gitgraph.commits, &commit.id);
                if let (Some(commit_a), Some(commit_b)) = (commit_a, commit_b) {
                    let path = gitgraph_arrow_path(
                        graph.direction,
                        commit_a,
                        commit_b,
                        (*p1x, *p1y),
                        (*p2x, *p2y),
                        &graph.gitgraph.commits,
                        gg,
                        &mut lanes,
                    );
                    let mut color_index =
                        branch_pos.get(&commit_b.branch).map(|v| v.1).unwrap_or(0);
                    if commit_b.commit_type == crate::ir::GitGraphCommitType::Merge
                        && commit_a.id != commit_b.parents.first().cloned().unwrap_or_default()
                    {
                        color_index = branch_pos
                            .get(&commit_a.branch)
                            .map(|v| v.1)
                            .unwrap_or(color_index);
                    }
                    arrows.push(GitGraphArrowLayout { path, color_index });
                }
            }
        }
    }

    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for branch in &branch_layouts {
        let (x1, y1, x2, y2) = if is_vertical {
            let start = if is_bottom_top {
                max_pos
            } else {
                gg.default_pos
            };
            let end = if is_bottom_top {
                gg.default_pos
            } else {
                max_pos
            };
            (branch.pos, start, branch.pos, end)
        } else {
            (0.0, branch.pos, max_pos, branch.pos)
        };
        update_bounds_line(
            &mut min_x, &mut min_y, &mut max_x, &mut max_y, x1, y1, x2, y2,
        );
        update_bounds_rect(
            &mut min_x,
            &mut min_y,
            &mut max_x,
            &mut max_y,
            branch.label.bg_x,
            branch.label.bg_y,
            branch.label.bg_width,
            branch.label.bg_height,
            None,
        );
    }

    for commit in &commit_layouts {
        let radius = if commit.commit_type == crate::ir::GitGraphCommitType::Merge {
            gg.merge_radius_outer
        } else {
            gg.commit_radius
        };
        update_bounds_rect(
            &mut min_x,
            &mut min_y,
            &mut max_x,
            &mut max_y,
            commit.x - radius,
            commit.y - radius,
            radius * 2.0,
            radius * 2.0,
            None,
        );
        if let Some(label) = &commit.label {
            update_bounds_rect(
                &mut min_x,
                &mut min_y,
                &mut max_x,
                &mut max_y,
                label.bg_x,
                label.bg_y,
                label.bg_width,
                label.bg_height,
                label.transform.as_ref(),
            );
        }
        for tag in &commit.tags {
            update_bounds_points(
                &mut min_x,
                &mut min_y,
                &mut max_x,
                &mut max_y,
                &tag.points,
                tag.transform.as_ref(),
            );
        }
    }

    if !min_x.is_finite() {
        min_x = 0.0;
        min_y = 0.0;
        max_x = 1.0;
        max_y = 1.0;
    }

    min_x -= gg.diagram_padding;
    min_y -= gg.diagram_padding;
    max_x += gg.diagram_padding;
    max_y += gg.diagram_padding;

    let width = (max_x - min_x).max(1.0);
    let height = (max_y - min_y).max(1.0);

    let mut nodes = BTreeMap::new();
    nodes.insert(
        "__gitgraph_metrics_content".to_string(),
        NodeLayout {
            id: "__gitgraph_metrics_content".to_string(),
            x: gg.diagram_padding,
            y: gg.diagram_padding,
            width: (width - gg.diagram_padding * 2.0).max(1.0),
            height: (height - gg.diagram_padding * 2.0).max(1.0),
            label: TextBlock {
                lines: vec![String::new()],
                width: 0.0,
                height: 0.0,
            },
            shape: crate::ir::NodeShape::Rectangle,
            style: crate::ir::NodeStyle::default(),
            link: None,
            anchor_subgraph: None,
            hidden: false,
            icon: None,
        },
    );

    Layout {
        kind: graph.kind,
        nodes,
        edges: Vec::new(),
        subgraphs: Vec::new(),
        diagram: DiagramData::GitGraph(GitGraphLayout {
            branches: branch_layouts,
            commits: commit_layouts,
            arrows,
            width,
            height,
            offset_x: -min_x,
            offset_y: -min_y,
            max_pos,
            direction: graph.direction,
        }),
        width,
        height,
    }
}

fn default_branch_order(index: usize) -> f32 {
    if index == 0 {
        return 0.0;
    }
    let mut denom = 1.0f32;
    let mut value = index;
    while value > 0 {
        denom *= 10.0;
        value /= 10;
    }
    (index as f32) / denom
}

fn measure_gitgraph_text(
    text: &str,
    font_size: f32,
    line_height: f32,
    width_scale: f32,
    font_family: &str,
    fast_metrics: bool,
    metrics: Option<&dyn TextMetrics>,
) -> (f32, f32) {
    let lines = split_lines(text);
    let max_width = lines
        .iter()
        .map(|line| text_width(line, font_size, font_family, fast_metrics, metrics))
        .fold(0.0, f32::max);
    let width = max_width * width_scale;
    let height = lines.len() as f32 * font_size * line_height;
    (width, height)
}

fn commit_by_id<'a>(
    commits: &'a [crate::ir::GitGraphCommit],
    id: &str,
) -> Option<&'a crate::ir::GitGraphCommit> {
    commits.iter().find(|commit| commit.id == id)
}

fn gitgraph_find_closest_parent(
    parents: &[String],
    commit_pos: &HashMap<String, (f32, f32)>,
    dir: Direction,
) -> Option<String> {
    let mut chosen: Option<String> = None;
    let mut target = if dir == Direction::BottomTop {
        f32::INFINITY
    } else {
        0.0
    };
    for parent in parents {
        if let Some((x, y)) = commit_pos.get(parent) {
            let pos = if matches!(dir, Direction::TopDown | Direction::BottomTop) {
                *y
            } else {
                *x
            };
            let accept = if dir == Direction::BottomTop {
                pos <= target
            } else {
                pos >= target
            };
            if accept {
                target = pos;
                chosen = Some(parent.clone());
            }
        }
    }
    chosen
}

fn gitgraph_find_closest_parent_bt(
    parents: &[String],
    commit_pos: &HashMap<String, (f32, f32)>,
) -> Option<String> {
    let mut chosen: Option<String> = None;
    let mut max_pos = f32::INFINITY;
    for parent in parents {
        if let Some((_x, y)) = commit_pos.get(parent)
            && *y <= max_pos
        {
            max_pos = *y;
            chosen = Some(parent.clone());
        }
    }
    chosen
}

fn gitgraph_find_closest_parent_pos(
    commit: &crate::ir::GitGraphCommit,
    commit_pos: &HashMap<String, (f32, f32)>,
) -> Option<f32> {
    let closest_parent =
        gitgraph_find_closest_parent(&commit.parents, commit_pos, Direction::BottomTop)?;
    commit_pos.get(&closest_parent).map(|(_x, y)| *y)
}

fn gitgraph_calculate_commit_position(
    commit: &crate::ir::GitGraphCommit,
    commit_step: f32,
    commit_pos: &HashMap<String, (f32, f32)>,
) -> f32 {
    let closest_parent_pos = gitgraph_find_closest_parent_pos(commit, commit_pos).unwrap_or(0.0);
    closest_parent_pos + commit_step
}

fn gitgraph_set_commit_position(
    commit: &crate::ir::GitGraphCommit,
    cur_pos: f32,
    layout_offset: f32,
    branch_pos: &HashMap<String, (f32, usize, f32, f32)>,
    commit_pos: &mut HashMap<String, (f32, f32)>,
) -> (f32, f32) {
    let x = branch_pos
        .get(&commit.branch)
        .map(|value| value.0)
        .unwrap_or(0.0);
    let y = cur_pos + layout_offset;
    commit_pos.insert(commit.id.clone(), (x, y));
    (x, y)
}

fn gitgraph_set_root_position(
    commit: &crate::ir::GitGraphCommit,
    cur_pos: f32,
    default_pos: f32,
    branch_pos: &HashMap<String, (f32, usize, f32, f32)>,
    commit_pos: &mut HashMap<String, (f32, f32)>,
) {
    let x = branch_pos
        .get(&commit.branch)
        .map(|value| value.0)
        .unwrap_or(0.0);
    let y = cur_pos + default_pos;
    commit_pos.insert(commit.id.clone(), (x, y));
}

fn gitgraph_set_parallel_bt_pos(
    commits: &[&crate::ir::GitGraphCommit],
    default_pos: f32,
    commit_step: f32,
    layout_offset: f32,
    branch_pos: &HashMap<String, (f32, usize, f32, f32)>,
    commit_pos: &mut HashMap<String, (f32, f32)>,
) {
    let mut cur_pos = default_pos;
    let mut max_position = default_pos;
    let mut roots = Vec::new();
    for commit in commits {
        if !commit.parents.is_empty() {
            cur_pos = gitgraph_calculate_commit_position(commit, commit_step, commit_pos);
            max_position = max_position.max(cur_pos);
        } else {
            roots.push(*commit);
        }
        gitgraph_set_commit_position(commit, cur_pos, layout_offset, branch_pos, commit_pos);
    }
    cur_pos = max_position;
    for commit in roots {
        gitgraph_set_root_position(commit, cur_pos, default_pos, branch_pos, commit_pos);
    }
    for commit in commits {
        if !commit.parents.is_empty()
            && let Some(closest_parent) =
                gitgraph_find_closest_parent_bt(&commit.parents, commit_pos)
            && let Some((_x, y)) = commit_pos.get(&closest_parent)
        {
            cur_pos = *y - commit_step;
            if cur_pos <= max_position {
                max_position = cur_pos;
            }
            let x = branch_pos
                .get(&commit.branch)
                .map(|value| value.0)
                .unwrap_or(0.0);
            let y = cur_pos - layout_offset;
            commit_pos.insert(commit.id.clone(), (x, y));
        }
    }
}

fn gitgraph_calculate_position(
    commit: &crate::ir::GitGraphCommit,
    dir: Direction,
    default_pos: f32,
    commit_step: f32,
    commit_pos: &HashMap<String, (f32, f32)>,
) -> f32 {
    let default_commit_pos = (0.0, 0.0);
    if !commit.parents.is_empty() {
        if let Some(parent) = gitgraph_find_closest_parent(&commit.parents, commit_pos, dir) {
            let parent_pos = commit_pos
                .get(&parent)
                .cloned()
                .unwrap_or(default_commit_pos);
            if dir == Direction::TopDown {
                return parent_pos.1 + commit_step;
            } else if dir == Direction::BottomTop {
                let current = commit_pos
                    .get(&commit.id)
                    .cloned()
                    .unwrap_or(default_commit_pos);
                return current.1 - commit_step;
            } else {
                return parent_pos.0 + commit_step;
            }
        }
    } else if dir == Direction::TopDown {
        return default_pos;
    } else if dir == Direction::BottomTop {
        let current = commit_pos
            .get(&commit.id)
            .cloned()
            .unwrap_or(default_commit_pos);
        return current.1 - commit_step;
    } else {
        return 0.0;
    }
    0.0
}

fn gitgraph_commit_position(
    commit: &crate::ir::GitGraphCommit,
    pos: f32,
    is_parallel: bool,
    dir: Direction,
    layout_offset: f32,
    branch_pos: &HashMap<String, (f32, usize, f32, f32)>,
) -> (f32, f32, f32) {
    let pos_with_offset = if dir == Direction::BottomTop && is_parallel {
        pos
    } else {
        pos + layout_offset
    };
    let branch_axis_pos = branch_pos
        .get(&commit.branch)
        .map(|value| value.0)
        .unwrap_or(0.0);
    let (x, y) = if matches!(dir, Direction::TopDown | Direction::BottomTop) {
        (branch_axis_pos, pos_with_offset)
    } else {
        (pos_with_offset, branch_axis_pos)
    };
    (x, y, pos_with_offset)
}

fn update_bounds_line(
    min_x: &mut f32,
    min_y: &mut f32,
    max_x: &mut f32,
    max_y: &mut f32,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
) {
    *min_x = min_x.min(x1.min(x2));
    *min_y = min_y.min(y1.min(y2));
    *max_x = max_x.max(x1.max(x2));
    *max_y = max_y.max(y1.max(y2));
}

fn update_bounds_rect(
    min_x: &mut f32,
    min_y: &mut f32,
    max_x: &mut f32,
    max_y: &mut f32,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    transform: Option<&GitGraphTransform>,
) {
    let corners = [
        (x, y),
        (x + width, y),
        (x + width, y + height),
        (x, y + height),
    ];
    update_bounds_points(min_x, min_y, max_x, max_y, &corners, transform);
}

fn update_bounds_points(
    min_x: &mut f32,
    min_y: &mut f32,
    max_x: &mut f32,
    max_y: &mut f32,
    points: &[(f32, f32)],
    transform: Option<&GitGraphTransform>,
) {
    for (x, y) in points {
        let (px, py) = apply_transform_point(*x, *y, transform);
        *min_x = min_x.min(px);
        *min_y = min_y.min(py);
        *max_x = max_x.max(px);
        *max_y = max_y.max(py);
    }
}

fn apply_transform_point(x: f32, y: f32, transform: Option<&GitGraphTransform>) -> (f32, f32) {
    if let Some(transform) = transform {
        let mut px = x + transform.translate_x;
        let mut py = y + transform.translate_y;
        if transform.rotate_deg.abs() > f32::EPSILON {
            let angle = transform.rotate_deg.to_radians();
            let cos = angle.cos();
            let sin = angle.sin();
            let dx = px - transform.rotate_cx;
            let dy = py - transform.rotate_cy;
            px = transform.rotate_cx + dx * cos - dy * sin;
            py = transform.rotate_cy + dx * sin + dy * cos;
        }
        (px, py)
    } else {
        (x, y)
    }
}

fn gitgraph_arrow_path(
    dir: Direction,
    commit_a: &crate::ir::GitGraphCommit,
    commit_b: &crate::ir::GitGraphCommit,
    p1: (f32, f32),
    p2: (f32, f32),
    commits: &[crate::ir::GitGraphCommit],
    config: &crate::config::GitGraphConfig,
    lanes: &mut Vec<f32>,
) -> String {
    let (p1x, p1y) = p1;
    let (p2x, p2y) = p2;
    let arrow_needs_reroute = should_reroute_arrow(dir, commit_a, commit_b, p1, p2, commits);
    let (arc, arc2, radius, offset) = if arrow_needs_reroute {
        let radius = config.arrow_reroute_radius;
        (
            format!("A {radius} {radius}, 0, 0, 0,"),
            format!("A {radius} {radius}, 0, 0, 1,"),
            radius,
            radius,
        )
    } else {
        let radius = config.arrow_radius;
        (
            format!("A {radius} {radius}, 0, 0, 0,"),
            format!("A {radius} {radius}, 0, 0, 1,"),
            radius,
            radius,
        )
    };

    let mut line_def = String::new();
    if arrow_needs_reroute {
        let line_y = if p1y < p2y {
            find_lane(p1y, p2y, lanes, config, 0)
        } else {
            find_lane(p2y, p1y, lanes, config, 0)
        };
        let line_x = if p1x < p2x {
            find_lane(p1x, p2x, lanes, config, 0)
        } else {
            find_lane(p2x, p1x, lanes, config, 0)
        };
        match dir {
            Direction::TopDown => {
                if p1x < p2x {
                    line_def = format!(
                        "M {p1x} {p1y} L {x1} {p1y} {arc2} {line_x} {y1} L {line_x} {y2} {arc} {x2} {p2y} L {p2x} {p2y}",
                        x1 = line_x - radius,
                        y1 = p1y + offset,
                        y2 = p2y - radius,
                        x2 = line_x + offset
                    );
                } else {
                    line_def = format!(
                        "M {p1x} {p1y} L {x1} {p1y} {arc} {line_x} {y1} L {line_x} {y2} {arc2} {x2} {p2y} L {p2x} {p2y}",
                        x1 = line_x + radius,
                        y1 = p1y + offset,
                        y2 = p2y - radius,
                        x2 = line_x - offset
                    );
                }
            }
            Direction::BottomTop => {
                if p1x < p2x {
                    line_def = format!(
                        "M {p1x} {p1y} L {x1} {p1y} {arc} {line_x} {y1} L {line_x} {y2} {arc2} {x2} {p2y} L {p2x} {p2y}",
                        x1 = line_x - radius,
                        y1 = p1y - offset,
                        y2 = p2y + radius,
                        x2 = line_x + offset
                    );
                } else {
                    line_def = format!(
                        "M {p1x} {p1y} L {x1} {p1y} {arc2} {line_x} {y1} L {line_x} {y2} {arc} {x2} {p2y} L {p2x} {p2y}",
                        x1 = line_x + radius,
                        y1 = p1y - offset,
                        y2 = p2y + radius,
                        x2 = line_x - offset
                    );
                }
            }
            _ => {
                if p1y < p2y {
                    line_def = format!(
                        "M {p1x} {p1y} L {p1x} {y1} {arc} {x1} {line_y} L {x2} {line_y} {arc2} {p2x} {y2} L {p2x} {p2y}",
                        y1 = line_y - radius,
                        x1 = p1x + offset,
                        x2 = p2x - radius,
                        y2 = line_y + offset
                    );
                } else {
                    line_def = format!(
                        "M {p1x} {p1y} L {p1x} {y1} {arc2} {x1} {line_y} L {x2} {line_y} {arc} {p2x} {y2} L {p2x} {p2y}",
                        y1 = line_y + radius,
                        x1 = p1x + offset,
                        x2 = p2x - radius,
                        y2 = line_y - offset
                    );
                }
            }
        }
        return line_def;
    }

    match dir {
        Direction::TopDown => {
            if p1x < p2x {
                if commit_b.commit_type == crate::ir::GitGraphCommitType::Merge
                    && commit_a.id
                        != commit_b
                            .parents
                            .first()
                            .cloned()
                            .unwrap_or_else(String::new)
                {
                    line_def = format!(
                        "M {p1x} {p1y} L {p1x} {y1} {arc} {x1} {p2y} L {p2x} {p2y}",
                        y1 = p2y - radius,
                        x1 = p1x + offset
                    );
                } else {
                    line_def = format!(
                        "M {p1x} {p1y} L {x1} {p1y} {arc2} {p2x} {y1} L {p2x} {p2y}",
                        x1 = p2x - radius,
                        y1 = p1y + offset
                    );
                }
            }
            if p1x > p2x {
                if commit_b.commit_type == crate::ir::GitGraphCommitType::Merge
                    && commit_a.id
                        != commit_b
                            .parents
                            .first()
                            .cloned()
                            .unwrap_or_else(String::new)
                {
                    line_def = format!(
                        "M {p1x} {p1y} L {p1x} {y1} {arc2} {x1} {p2y} L {p2x} {p2y}",
                        y1 = p2y - radius,
                        x1 = p1x - offset
                    );
                } else {
                    line_def = format!(
                        "M {p1x} {p1y} L {x1} {p1y} {arc} {p2x} {y1} L {p2x} {p2y}",
                        x1 = p2x + radius,
                        y1 = p1y + offset
                    );
                }
            }
            if (p1x - p2x).abs() < f32::EPSILON {
                line_def = format!("M {p1x} {p1y} L {p2x} {p2y}");
            }
        }
        Direction::BottomTop => {
            if p1x < p2x {
                if commit_b.commit_type == crate::ir::GitGraphCommitType::Merge
                    && commit_a.id
                        != commit_b
                            .parents
                            .first()
                            .cloned()
                            .unwrap_or_else(String::new)
                {
                    line_def = format!(
                        "M {p1x} {p1y} L {p1x} {y1} {arc2} {x1} {p2y} L {p2x} {p2y}",
                        y1 = p2y + radius,
                        x1 = p1x + offset
                    );
                } else {
                    line_def = format!(
                        "M {p1x} {p1y} L {x1} {p1y} {arc} {p2x} {y1} L {p2x} {p2y}",
                        x1 = p2x - radius,
                        y1 = p1y - offset
                    );
                }
            }
            if p1x > p2x {
                if commit_b.commit_type == crate::ir::GitGraphCommitType::Merge
                    && commit_a.id
                        != commit_b
                            .parents
                            .first()
                            .cloned()
                            .unwrap_or_else(String::new)
                {
                    line_def = format!(
                        "M {p1x} {p1y} L {p1x} {y1} {arc} {x1} {p2y} L {p2x} {p2y}",
                        y1 = p2y + radius,
                        x1 = p1x - offset
                    );
                } else {
                    line_def = format!(
                        "M {p1x} {p1y} L {x1} {p1y} {arc} {p2x} {y1} L {p2x} {p2y}",
                        x1 = p2x - radius,
                        y1 = p1y - offset
                    );
                }
            }
            if (p1x - p2x).abs() < f32::EPSILON {
                line_def = format!("M {p1x} {p1y} L {p2x} {p2y}");
            }
        }
        _ => {
            if p1y < p2y {
                if commit_b.commit_type == crate::ir::GitGraphCommitType::Merge
                    && commit_a.id
                        != commit_b
                            .parents
                            .first()
                            .cloned()
                            .unwrap_or_else(String::new)
                {
                    line_def = format!(
                        "M {p1x} {p1y} L {x1} {p1y} {arc2} {p2x} {y1} L {p2x} {p2y}",
                        x1 = p2x - radius,
                        y1 = p1y + offset
                    );
                } else {
                    line_def = format!(
                        "M {p1x} {p1y} L {p1x} {y1} {arc} {x1} {p2y} L {p2x} {p2y}",
                        y1 = p2y - radius,
                        x1 = p1x + offset
                    );
                }
            }
            if p1y > p2y {
                if commit_b.commit_type == crate::ir::GitGraphCommitType::Merge
                    && commit_a.id
                        != commit_b
                            .parents
                            .first()
                            .cloned()
                            .unwrap_or_else(String::new)
                {
                    line_def = format!(
                        "M {p1x} {p1y} L {x1} {p1y} {arc} {p2x} {y1} L {p2x} {p2y}",
                        x1 = p2x - radius,
                        y1 = p1y - offset
                    );
                } else {
                    line_def = format!(
                        "M {p1x} {p1y} L {p1x} {y1} {arc2} {x1} {p2y} L {p2x} {p2y}",
                        y1 = p2y + radius,
                        x1 = p1x + offset
                    );
                }
            }
            if (p1y - p2y).abs() < f32::EPSILON {
                line_def = format!("M {p1x} {p1y} L {p2x} {p2y}");
            }
        }
    }

    if line_def.is_empty() {
        line_def = format!("M {p1x} {p1y} L {p2x} {p2y}");
    }
    line_def
}

fn should_reroute_arrow(
    dir: Direction,
    commit_a: &crate::ir::GitGraphCommit,
    commit_b: &crate::ir::GitGraphCommit,
    p1: (f32, f32),
    p2: (f32, f32),
    commits: &[crate::ir::GitGraphCommit],
) -> bool {
    let commit_b_is_furthest = match dir {
        Direction::TopDown | Direction::BottomTop => p1.0 < p2.0,
        _ => p1.1 < p2.1,
    };
    let branch_to_get_curve = if commit_b_is_furthest {
        &commit_b.branch
    } else {
        &commit_a.branch
    };
    commits.iter().any(|commit| {
        commit.seq > commit_a.seq
            && commit.seq < commit_b.seq
            && &commit.branch == branch_to_get_curve
    })
}

fn find_lane(
    y1: f32,
    y2: f32,
    lanes: &mut Vec<f32>,
    config: &crate::config::GitGraphConfig,
    depth: usize,
) -> f32 {
    let candidate = y1 + (y2 - y1).abs() / 2.0;
    if depth > config.lane_max_depth {
        return candidate;
    }
    let ok = lanes
        .iter()
        .all(|lane| (lane - candidate).abs() >= config.lane_spacing);
    if ok {
        lanes.push(candidate);
        return candidate;
    }
    let diff = (y1 - y2).abs();
    find_lane(y1, y2 - diff / 5.0, lanes, config, depth + 1)
}
