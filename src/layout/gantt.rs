use super::*;

fn gantt_palette(theme: &Theme) -> Vec<String> {
    vec![
        theme.primary_border_color.clone(),
        "#0ea5e9".to_string(), // sky-500
        "#10b981".to_string(), // emerald-500
        "#6366f1".to_string(), // indigo-500
        "#f97316".to_string(), // orange-500
    ]
}

fn hsl_color(h: f32, s: f32, l: f32) -> String {
    format!("hsl({:.10}, {:.10}%, {:.10}%)", h, s, l)
}

fn shift_color(color: &str, target_s: f32, target_l: f32, strength: f32) -> String {
    let Some((_h, s, l)) = parse_color_to_hsl(color) else {
        return color.to_string();
    };
    let delta_s = (target_s - s) * strength;
    let delta_l = (target_l - l) * strength;
    adjust_color(color, 0.0, delta_s, delta_l)
}

fn gantt_section_palette(theme: &Theme, sections: &[String]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if sections.is_empty() {
        return map;
    }
    let base = theme.primary_border_color.as_str();
    let step = 360.0 / sections.len().max(1) as f32;
    for (idx, name) in sections.iter().enumerate() {
        let hue_shift = step * idx as f32;
        let mut color = adjust_color(base, hue_shift, 0.0, 0.0);
        color = shift_color(&color, 60.0, 55.0, 0.4);
        map.insert(name.clone(), color);
    }
    map
}

fn gantt_task_color(status: Option<crate::ir::GanttStatus>, base: &str, fallback: &str) -> String {
    let base = if parse_color_to_hsl(base).is_some() {
        base.to_string()
    } else {
        fallback.to_string()
    };
    match status {
        Some(crate::ir::GanttStatus::Done) => shift_color(&base, 30.0, 80.0, 0.7),
        Some(crate::ir::GanttStatus::Active) => shift_color(&base, 70.0, 52.0, 0.6),
        Some(crate::ir::GanttStatus::Crit) => {
            if let Some((_, s, l)) = parse_color_to_hsl(&base) {
                hsl_color(0.0, s.max(65.0), l.clamp(45.0, 60.0))
            } else {
                "#ef4444".to_string()
            }
        }
        Some(crate::ir::GanttStatus::Milestone) => {
            if let Some((_, s, l)) = parse_color_to_hsl(&base) {
                hsl_color(45.0, s.max(65.0), l.clamp(50.0, 65.0))
            } else {
                "#f59e0b".to_string()
            }
        }
        None => base,
    }
}

fn parse_gantt_duration(value: &str) -> Option<f32> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let mut digits = String::new();
    let mut unit = None;
    for ch in value.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            digits.push(ch);
        } else if !ch.is_whitespace() {
            unit = Some(ch.to_ascii_lowercase());
        }
    }
    let number: f32 = digits.parse().ok()?;
    let mult = match unit {
        Some('d') => 1.0,
        Some('w') => 7.0,
        Some('h') => 1.0 / 24.0,
        Some('m') => 30.0,
        Some('y') => 365.0,
        _ => 1.0,
    };
    Some(number * mult)
}

fn parse_gantt_date(value: &str) -> Option<i32> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let parts: Vec<&str> = value
        .split(|ch| ch == '-' || ch == '/' || ch == '.')
        .collect();
    if parts.len() != 3 {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;
    if month == 0 || month > 12 || day == 0 || day > 31 {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i32 {
    let y = year - (month <= 2) as i32;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = month as i32;
    let d = day as i32;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(days: i32) -> (i32, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + (m <= 2) as i32;
    (year, m as u32, d as u32)
}

fn format_gantt_date(days: i32) -> String {
    let (year, month, day) = civil_from_days(days);
    format!("{:04}-{:02}-{:02}", year, month, day)
}

pub(super) fn compute_gantt_layout(graph: &Graph, theme: &Theme, config: &LayoutConfig) -> Layout {
    let padding = theme.font_size * 1.25;
    let row_height = (theme.font_size * 1.5).max(theme.font_size + 8.0);
    let label_gap = theme.font_size * 1.05;
    let default_duration = 3.0_f32;

    let title = graph
        .gantt_title
        .as_ref()
        .map(|t| measure_label(t, theme, config));
    let title_height = title.as_ref().map(|t| t.height + padding).unwrap_or(0.0);

    let mut task_label_width = 0.0_f32;
    let mut section_label_width = 0.0_f32;
    for task in &graph.gantt_tasks {
        let label = measure_label(&task.label, theme, config);
        task_label_width = task_label_width.max(label.width);
        if let Some(section) = task.section.as_ref() {
            let section_label = measure_label(section, theme, config);
            section_label_width = section_label_width.max(section_label.width);
        }
    }
    task_label_width = task_label_width.max(theme.font_size * 6.5);

    let label_x = padding;
    let section_task_gap = if section_label_width > 0.0 {
        theme.font_size * 0.8
    } else {
        0.0
    };
    let label_width = section_label_width + section_task_gap + task_label_width;
    let section_label_x = label_x;
    let task_label_x = label_x + section_label_width + section_task_gap;
    let chart_x = padding + label_width + label_gap;
    let chart_y = title_height + padding;
    let chart_width = theme.font_size * 26.0;

    let mut parsed_starts: HashMap<String, f32> = HashMap::new();
    let mut origin: Option<f32> = None;
    for task in &graph.gantt_tasks {
        if let Some(start) = task.start.as_deref().and_then(parse_gantt_date) {
            let start = start as f32;
            parsed_starts.insert(task.id.clone(), start);
            origin = Some(origin.map_or(start, |v| v.min(start)));
        }
    }
    let has_dates = origin.is_some();

    let mut timing: HashMap<String, (f32, f32)> = HashMap::new();
    let mut cursor = 0.0_f32;
    let mut time_start = f32::MAX;
    let mut time_end = f32::MIN;

    let mut computed: Vec<(
        String,
        f32,
        f32,
        Option<crate::ir::GanttStatus>,
        Option<String>,
    )> = Vec::with_capacity(graph.gantt_tasks.len());
    for task in &graph.gantt_tasks {
        let duration = task
            .duration
            .as_deref()
            .and_then(parse_gantt_duration)
            .unwrap_or(default_duration)
            .max(0.1);
        let mut start = parsed_starts.get(&task.id).copied();
        if start.is_none() {
            if let Some(after_id) = task.after.as_deref() {
                if let Some((_, end)) = timing.get(after_id) {
                    start = Some(*end);
                }
            }
        }
        let fallback_base = origin.unwrap_or(0.0);
        let start = start.unwrap_or(fallback_base + cursor);
        let end = start + duration;
        timing.insert(task.id.clone(), (start, end));
        cursor = cursor.max(end + 0.5);
        time_start = time_start.min(start);
        time_end = time_end.max(end);
        computed.push((
            task.label.clone(),
            start,
            duration,
            task.status,
            task.section.clone(),
        ));
    }
    if !time_start.is_finite() || !time_end.is_finite() {
        time_start = 0.0;
        time_end = 1.0;
    }
    if (time_end - time_start).abs() < 0.01 {
        time_end = time_start + 1.0;
    }
    let time_span = (time_end - time_start).max(1.0);
    let time_scale = chart_width / time_span;

    let mut ticks: Vec<GanttTick> = Vec::new();
    let tick_count = 4;
    for i in 0..=tick_count {
        let t = time_start + time_span * (i as f32) / (tick_count as f32);
        let x = chart_x + (t - time_start) * time_scale;
        let label = if has_dates {
            format_gantt_date(t.round() as i32)
        } else {
            format!("{:.0}", t - time_start)
        };
        ticks.push(GanttTick { x, label });
    }

    let compact = graph
        .gantt_display_mode
        .as_deref()
        .map(|m| m.eq_ignore_ascii_case("compact"))
        .unwrap_or(false);

    let palette = gantt_palette(theme);
    let section_palette = gantt_section_palette(theme, &graph.gantt_sections);
    let mut current_section: Option<String> = None;
    let mut current_section_idx: Option<usize> = None;
    let mut sections: Vec<GanttSectionLayout> = Vec::new();
    let mut tasks: Vec<GanttTaskLayout> = Vec::new();
    let mut y = chart_y;

    // Compact mode: pack non-overlapping tasks into the same row.
    // Each lane is y_position, end_time
    let mut lanes: Vec<(f32, f32)> = Vec::new();

    for (idx, (label, start, duration, status, section)) in computed.iter().enumerate() {
        if section != &current_section {
            if let Some(sec) = section.as_ref() {
                if let Some(prev_idx) = current_section_idx {
                    let height = (y - sections[prev_idx].y).max(row_height);
                    sections[prev_idx].height = height;
                }
                lanes.clear();
                let base_color = section_palette
                    .get(sec)
                    .cloned()
                    .unwrap_or_else(|| palette[idx % palette.len()].clone());
                let band_color = shift_color(&base_color, 20.0, 92.0, 0.7);
                sections.push(GanttSectionLayout {
                    label: measure_label(sec, theme, config),
                    y,
                    height: 0.0,
                    color: base_color,
                    band_color,
                });
                current_section_idx = Some(sections.len() - 1);
            } else if let Some(prev_idx) = current_section_idx {
                let height = (y - sections[prev_idx].y).max(row_height);
                sections[prev_idx].height = height;
                current_section_idx = None;
                lanes.clear();
            }
            current_section = section.clone();
        }

        let bar_x = chart_x + (start - time_start) * time_scale;
        let mut bar_width = duration * time_scale;
        let min_width = row_height * 0.5;
        if bar_width < min_width {
            bar_width = min_width;
        }
        let base_color = if let Some(sec) = section.as_ref() {
            section_palette
                .get(sec)
                .cloned()
                .unwrap_or_else(|| palette[idx % palette.len()].clone())
        } else {
            palette[idx % palette.len()].clone()
        };
        let color = gantt_task_color(*status, &base_color, &palette[0]);

        let task_end = start + duration;
        let task_y = if compact {
            if let Some(lane) = lanes.iter_mut().find(|(_, end)| *start >= *end) {
                let ly = lane.0;
                lane.1 = task_end;
                ly
            } else {
                let ly = y;
                lanes.push((ly, task_end));
                y += row_height;
                ly
            }
        } else {
            let ly = y;
            y += row_height;
            ly
        };

        tasks.push(GanttTaskLayout {
            label: measure_label(label, theme, config),
            x: bar_x,
            y: task_y,
            width: bar_width,
            height: row_height,
            color,
            start: *start,
            duration: *duration,
            status: *status,
        });
    }
    if let Some(prev_idx) = current_section_idx {
        let height = (y - sections[prev_idx].y).max(row_height);
        sections[prev_idx].height = height;
    }

    let tick_font = theme.font_size * 0.8;
    let max_tick_half_width = ticks
        .iter()
        .map(|tick| {
            measure_label_with_font_size(
                tick.label.as_str(),
                tick_font,
                config,
                false,
                theme.font_family.as_str(),
            )
            .width
                / 2.0
        })
        .fold(0.0_f32, f32::max);
    let axis_pad = row_height * 0.9 + theme.font_size;
    let height = y + padding + axis_pad;
    let label_overflow = if compact {
        padding + task_label_width
    } else {
        0.0
    };
    let right_padding = (max_tick_half_width + padding * 0.4)
        .max(label_overflow)
        .max(padding);
    let width = chart_x + chart_width + right_padding;

    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        diagram: DiagramData::Gantt(GanttLayout {
            title,
            sections,
            tasks,
            time_start,
            time_end,
            chart_x,
            chart_y,
            chart_width,
            chart_height: y - chart_y,
            row_height,
            label_x,
            label_width,
            section_label_x,
            section_label_width,
            task_label_x,
            task_label_width,
            title_y: chart_y - row_height * 0.6,
            ticks,
            compact,
        }),
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::{DiagramKind, GanttStatus, GanttTask, Graph};
    use crate::layout::LayoutConfig;
    use crate::layout::types::DiagramData;
    use crate::theme::Theme;

    use super::compute_gantt_layout;

    fn make_graph(display_mode: Option<&str>, tasks: Vec<GanttTask>) -> Graph {
        let mut graph = Graph::new();
        graph.kind = DiagramKind::Gantt;
        graph.gantt_display_mode = display_mode.map(|s| s.to_string());
        let mut sections = Vec::new();
        for t in &tasks {
            if let Some(sec) = &t.section {
                if !sections.contains(sec) {
                    sections.push(sec.clone());
                }
            }
        }
        graph.gantt_sections = sections;
        graph.gantt_tasks = tasks;
        graph
    }

    fn task(id: &str, section: &str, start: &str, dur: &str) -> GanttTask {
        GanttTask {
            id: id.to_string(),
            label: id.to_string(),
            start: Some(start.to_string()),
            duration: Some(dur.to_string()),
            after: None,
            section: Some(section.to_string()),
            status: None,
        }
    }

    fn milestone(id: &str, section: &str, start: &str) -> GanttTask {
        GanttTask {
            id: id.to_string(),
            label: id.to_string(),
            start: Some(start.to_string()),
            duration: Some("0d".to_string()),
            after: None,
            section: Some(section.to_string()),
            status: Some(GanttStatus::Milestone),
        }
    }

    fn extract_task_ys(graph: &Graph) -> Vec<f32> {
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let layout = compute_gantt_layout(graph, &theme, &config);
        match &layout.diagram {
            DiagramData::Gantt(g) => g.tasks.iter().map(|t| t.y).collect(),
            _ => panic!("expected Gantt layout"),
        }
    }

    #[test]
    fn compact_non_overlapping_tasks_share_row() {
        let graph = make_graph(
            Some("compact"),
            vec![
                milestone("m1", "MacOS", "2025-09-01"),
                milestone("m2", "MacOS", "2026-09-01"),
                milestone("m3", "MacOS", "2027-09-01"),
            ],
        );
        let ys = extract_task_ys(&graph);
        assert_eq!(ys[0], ys[1], "m1 and m2 should share a row");
        assert_eq!(ys[1], ys[2], "m2 and m3 should share a row");
    }

    #[test]
    fn compact_overlapping_tasks_get_separate_rows() {
        let graph = make_graph(
            Some("compact"),
            vec![
                task("hw", "support", "2025-09-01", "1y"),
                task("sw", "support", "2025-09-01", "2y"),
            ],
        );
        let ys = extract_task_ys(&graph);
        assert_ne!(ys[0], ys[1], "overlapping tasks must be on different rows");
    }

    #[test]
    fn non_compact_always_separate_rows() {
        let graph = make_graph(
            None,
            vec![
                milestone("m1", "MacOS", "2025-09-01"),
                milestone("m2", "MacOS", "2026-09-01"),
                milestone("m3", "MacOS", "2027-09-01"),
            ],
        );
        let ys = extract_task_ys(&graph);
        assert_ne!(ys[0], ys[1]);
        assert_ne!(ys[1], ys[2]);
    }
}
