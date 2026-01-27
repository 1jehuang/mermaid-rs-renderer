use crate::ir::{DiagramKind, Direction, Graph, NodeStyle, Subgraph};
use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{HashMap, VecDeque};

type NodeTokenParts = (
    String,
    Option<String>,
    Option<crate::ir::NodeShape>,
    Vec<String>,
);

static HEADER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(flowchart|graph)\s+(\w+)").unwrap());
static SUBGRAPH_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^subgraph\s+(.*)$").unwrap());
static INIT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^%%\{\s*init\s*:\s*(\{.*\})\s*\}%%").unwrap());
static PIPE_LABEL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^(?P<left>.+?)\s*(?P<arrow><[-.=ox]*[-=]+[-.=ox]*>|<[-.=ox]*[-=]+|[-.=ox]*[-=]+>|[-.=ox]*[-=]+)\|(?P<label>.+?)\|\s*(?P<right>.+)$",
    )
    .unwrap()
});
static LABEL_ARROW_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^(?P<left>.+?)\s*(?P<start><)?(?P<dash1>[-.=ox]*[-=]+[-.=ox]*)\s+(?P<label>[^<>=]+?)\s+(?P<dash2>[-.=ox]*[-=]+[-.=ox]*)(?P<end>>)?\s*(?P<right>.+)$",
    )
    .unwrap()
});
static ARROW_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^(?P<left>.+?)\s*(?P<arrow><[-.=ox]*[-=]+[-.=ox]*>|<[-.=ox]*[-=]+|[-.=ox]*[-=]+>|[-.=ox]*[-=]+)\s*(?P<right>.+)$",
    )
    .unwrap()
});
static ARROW_TOKEN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<[-.=ox]*[-=]+[-.=ox]*>|<[-.=ox]*[-=]+|[-.=ox]*[-=]+>|[-.=ox]*[-=]+").unwrap()
});

#[derive(Debug, Default)]
pub struct ParseOutput {
    pub graph: Graph,
    pub init_config: Option<serde_json::Value>,
}

pub fn parse_mermaid(input: &str) -> Result<ParseOutput> {
    match detect_diagram_kind(input) {
        DiagramKind::Class => parse_class_diagram(input),
        DiagramKind::State => parse_state_diagram(input),
        DiagramKind::Sequence => parse_sequence_diagram(input),
        DiagramKind::Er => parse_er_diagram(input),
        DiagramKind::Pie => parse_pie_diagram(input),
        DiagramKind::Mindmap => parse_mindmap_diagram(input),
        DiagramKind::Journey => parse_journey_diagram(input),
        DiagramKind::Timeline => parse_timeline_diagram(input),
        DiagramKind::Gantt => parse_gantt_diagram(input),
        DiagramKind::Requirement => parse_requirement_diagram(input),
        DiagramKind::GitGraph => parse_gitgraph_diagram(input),
        DiagramKind::C4 => parse_c4_diagram(input),
        DiagramKind::Sankey => parse_sankey_diagram(input),
        DiagramKind::Quadrant => parse_quadrant_diagram(input),
        DiagramKind::ZenUML => parse_zenuml_diagram(input),
        DiagramKind::Block => parse_block_diagram(input),
        DiagramKind::Packet => parse_packet_diagram(input),
        DiagramKind::Kanban => parse_kanban_diagram(input),
        DiagramKind::Architecture => parse_architecture_diagram(input),
        DiagramKind::Radar => parse_radar_diagram(input),
        DiagramKind::Treemap => parse_treemap_diagram(input),
        DiagramKind::XYChart => parse_xy_chart_diagram(input),
        DiagramKind::Flowchart => parse_flowchart(input),
    }
}

fn detect_diagram_kind(input: &str) -> DiagramKind {
    for raw_line in input.lines() {
        let trimmed_line = raw_line.trim();
        if trimmed_line.is_empty() {
            continue;
        }
        if trimmed_line.starts_with("%%") {
            continue;
        }
        if trimmed_line.starts_with("%%{") {
            continue;
        }
        let without_comment = strip_trailing_comment(trimmed_line);
        if without_comment.is_empty() {
            continue;
        }
        let lower = without_comment.to_ascii_lowercase();
        if lower.starts_with("sequencediagram") {
            return DiagramKind::Sequence;
        }
        if lower.starts_with("classdiagram") {
            return DiagramKind::Class;
        }
        if lower.starts_with("statediagram") {
            return DiagramKind::State;
        }
        if lower.starts_with("erdiagram") {
            return DiagramKind::Er;
        }
        if lower.starts_with("pie") {
            return DiagramKind::Pie;
        }
        if lower.starts_with("mindmap") {
            return DiagramKind::Mindmap;
        }
        if lower.starts_with("journey") {
            return DiagramKind::Journey;
        }
        if lower.starts_with("timeline") {
            return DiagramKind::Timeline;
        }
        if lower.starts_with("gantt") {
            return DiagramKind::Gantt;
        }
        if lower.starts_with("requirementdiagram") {
            return DiagramKind::Requirement;
        }
        if lower.starts_with("gitgraph") {
            return DiagramKind::GitGraph;
        }
        if lower.starts_with("c4") {
            return DiagramKind::C4;
        }
        if lower.starts_with("sankey") {
            return DiagramKind::Sankey;
        }
        if lower.starts_with("quadrantchart") {
            return DiagramKind::Quadrant;
        }
        if lower.starts_with("zenuml") {
            return DiagramKind::ZenUML;
        }
        if lower.starts_with("block") {
            return DiagramKind::Block;
        }
        if lower.starts_with("packet") {
            return DiagramKind::Packet;
        }
        if lower.starts_with("kanban") {
            return DiagramKind::Kanban;
        }
        if lower.starts_with("architecture") {
            return DiagramKind::Architecture;
        }
        if lower.starts_with("radar") {
            return DiagramKind::Radar;
        }
        if lower.starts_with("treemap") {
            return DiagramKind::Treemap;
        }
        if lower.starts_with("xychart") {
            return DiagramKind::XYChart;
        }
        if lower.starts_with("flowchart") || lower.starts_with("graph") {
            return DiagramKind::Flowchart;
        }
    }
    DiagramKind::Flowchart
}

fn preprocess_input(input: &str) -> Result<(Vec<String>, Option<serde_json::Value>)> {
    let mut init_config: Option<serde_json::Value> = None;
    let mut lines = Vec::new();

    for raw_line in input.lines() {
        let trimmed_line = raw_line.trim();
        if trimmed_line.is_empty() {
            continue;
        }
        if let Some(caps) = INIT_RE.captures(trimmed_line) {
            if let Some(json_str) = caps.get(1).map(|m| m.as_str()) {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                    init_config = Some(value);
                } else if let Ok(value) = json5::from_str::<serde_json::Value>(json_str) {
                    init_config = Some(value);
                }
            }
            continue;
        }
        if trimmed_line.starts_with("%%") {
            continue;
        }
        let without_comment = strip_trailing_comment(trimmed_line);
        if without_comment.is_empty() {
            continue;
        }
        lines.push(without_comment.to_string());
    }

    Ok((lines, init_config))
}

fn preprocess_input_keep_indent(input: &str) -> Result<(Vec<String>, Option<serde_json::Value>)> {
    let mut init_config: Option<serde_json::Value> = None;
    let mut lines = Vec::new();

    for raw_line in input.lines() {
        let trimmed_line = raw_line.trim();
        if trimmed_line.is_empty() {
            continue;
        }
        if let Some(caps) = INIT_RE.captures(trimmed_line) {
            if let Some(json_str) = caps.get(1).map(|m| m.as_str()) {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                    init_config = Some(value);
                } else if let Ok(value) = json5::from_str::<serde_json::Value>(json_str) {
                    init_config = Some(value);
                }
            }
            continue;
        }
        if trimmed_line.starts_with("%%") {
            continue;
        }
        let without_comment = strip_trailing_comment_keep_indent(raw_line);
        if without_comment.trim().is_empty() {
            continue;
        }
        lines.push(without_comment);
    }

    Ok((lines, init_config))
}

fn parse_flowchart(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Flowchart;
    let mut subgraph_stack: Vec<usize> = Vec::new();

    let (lines, init_config) = preprocess_input(input)?;

    for raw_line in lines {
        for line in split_statements(&raw_line) {
            if line.is_empty() {
                continue;
            }

            if let Some(caps) = HEADER_RE.captures(&line) {
                if let Some(dir) = caps.get(2).and_then(|m| Direction::from_token(m.as_str())) {
                    graph.direction = dir;
                }
                continue;
            }

            if line == "end" {
                subgraph_stack.pop();
                continue;
            }

            if let Some(caps) = SUBGRAPH_RE.captures(&line) {
                let rest = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let (id, label, classes) = parse_subgraph_header(rest);
                graph.subgraphs.push(Subgraph {
                    id: id.clone(),
                    label,
                    nodes: Vec::new(),
                    direction: None,
                });
                subgraph_stack.push(graph.subgraphs.len() - 1);
                if let Some(id) = id {
                    apply_subgraph_classes(&mut graph, &id, &classes);
                }
                continue;
            }

            if let Some(direction) = parse_direction_line(&line) {
                if let Some(idx) = subgraph_stack.last().copied() {
                    if let Some(sub) = graph.subgraphs.get_mut(idx) {
                        sub.direction = Some(direction);
                    }
                } else {
                    graph.direction = direction;
                }
                continue;
            }

            if line.starts_with("classDef") {
                parse_class_def(&line, &mut graph);
                continue;
            }

            if line.starts_with("class ") {
                parse_class_line(&line, &mut graph);
                continue;
            }

            if line.starts_with("style ") {
                parse_style_line(&line, &mut graph);
                continue;
            }

            if line.starts_with("linkStyle") {
                parse_link_style_line(&line, &mut graph);
                continue;
            }

            if let Some((id, link)) = parse_click_line(&line) {
                graph.node_links.insert(id, link);
                continue;
            }

            if line.starts_with("accTitle")
                || line.starts_with("accDescr")
                || line.starts_with("title ")
            {
                continue;
            }

            if let Some(chain_lines) = split_edge_chain(&line) {
                let mut added = false;
                for edge_line in chain_lines {
                    added |= add_flowchart_edge(&edge_line, &mut graph, &subgraph_stack);
                }
                if added {
                    continue;
                }
            }

            if add_flowchart_edge(&line, &mut graph, &subgraph_stack) {
                continue;
            }

            if let Some((node_id, node_label, node_shape, node_classes)) = parse_node_only(&line) {
                graph.ensure_node(&node_id, node_label, node_shape);
                apply_node_classes(&mut graph, &node_id, &node_classes);
                add_node_to_subgraphs(&mut graph, &subgraph_stack, &node_id);
            }
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn add_flowchart_edge(line: &str, graph: &mut Graph, subgraph_stack: &[usize]) -> bool {
    let Some((left, label, right, edge_meta)) = parse_edge_line(line) else {
        return false;
    };

    let sources: Vec<&str> = left
        .split('&')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect();
    let targets: Vec<&str> = right
        .split('&')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect();

    let mut source_ids = Vec::new();
    for source in sources {
        let (left_id, left_label, left_shape, left_classes) = parse_node_token(source);
        graph.ensure_node(&left_id, left_label, left_shape);
        apply_node_classes(graph, &left_id, &left_classes);
        add_node_to_subgraphs(graph, subgraph_stack, &left_id);
        source_ids.push(left_id);
    }

    let mut target_ids = Vec::new();
    for target in targets {
        let (right_id, right_label, right_shape, right_classes) = parse_node_token(target);
        graph.ensure_node(&right_id, right_label, right_shape);
        apply_node_classes(graph, &right_id, &right_classes);
        add_node_to_subgraphs(graph, subgraph_stack, &right_id);
        target_ids.push(right_id);
    }

    for left_id in &source_ids {
        for right_id in &target_ids {
            graph.edges.push(crate::ir::Edge {
                from: left_id.clone(),
                to: right_id.clone(),
                label: label.clone(),
                start_label: None,
                end_label: None,
                directed: edge_meta.directed,
                arrow_start: edge_meta.arrow_start,
                arrow_end: edge_meta.arrow_end,
                arrow_start_kind: edge_meta.arrow_start_kind,
                arrow_end_kind: edge_meta.arrow_end_kind,
                start_decoration: edge_meta.start_decoration,
                end_decoration: edge_meta.end_decoration,
                style: edge_meta.style,
            });
        }
    }

    true
}

fn split_trailing_quoted(input: &str) -> Option<(&str, &str)> {
    let trimmed = input.trim_end();
    let quote = trimmed.chars().last()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let mut iter = trimmed.char_indices().rev();
    let _ = iter.next();
    for (idx, ch) in iter {
        if ch == quote {
            let before = &trimmed[..idx];
            let value = &trimmed[idx + 1..trimmed.len() - 1];
            return Some((before, value));
        }
    }
    None
}

fn split_leading_quoted(input: &str) -> Option<(&str, &str)> {
    let trimmed = input.trim_start();
    let mut iter = trimmed.char_indices();
    let Some((_, quote)) = iter.next() else {
        return None;
    };
    if quote != '"' && quote != '\'' {
        return None;
    }
    for (idx, ch) in iter {
        if ch == quote {
            let value = &trimmed[1..idx];
            let rest = &trimmed[idx + 1..];
            return Some((value, rest));
        }
    }
    None
}

fn split_multiplicity_left(input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return (String::new(), None);
    }
    if let Some((before, value)) = split_trailing_quoted(trimmed) {
        let before = before.trim();
        if !before.is_empty() && !value.is_empty() {
            return (before.to_string(), Some(value.to_string()));
        }
    }
    (trimmed.to_string(), None)
}

fn split_multiplicity_right(input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return (String::new(), None);
    }
    if let Some((value, rest)) = split_leading_quoted(trimmed) {
        let rest = rest.trim();
        if !rest.is_empty() && !value.is_empty() {
            return (rest.to_string(), Some(value.to_string()));
        }
    }
    (trimmed.to_string(), None)
}

fn parse_class_relation_line(
    line: &str,
) -> Option<(
    String,
    String,
    EdgeMeta,
    Option<String>,
    Option<String>,
    Option<String>,
)> {
    let tokens = [
        "<|..", "..|>", "<|--", "--|>", "*--", "--*", "o--", "--o", "<..", "..>", "<--", "-->",
        "..", "--",
    ];

    for token in tokens {
        if let Some(pos) = line.find(token) {
            let left = line[..pos].trim();
            let right_part = line[pos + token.len()..].trim();
            if left.is_empty() || right_part.is_empty() {
                continue;
            }
            let (right, label) = split_label(right_part);
            let (left, start_label) = split_multiplicity_left(left);
            let (right, end_label) = split_multiplicity_right(&right);
            let meta = edge_meta_from_class_token(token);
            return Some((left, right, meta, label, start_label, end_label));
        }
    }
    None
}

fn edge_meta_from_class_token(token: &str) -> EdgeMeta {
    let arrow_start = token.contains('<');
    let arrow_end = token.contains('>');
    let directed = arrow_start || arrow_end;
    let style = if token.contains("..") {
        crate::ir::EdgeStyle::Dotted
    } else {
        crate::ir::EdgeStyle::Solid
    };

    let mut start_decoration = None;
    let mut end_decoration = None;
    if token.starts_with('*') {
        start_decoration = Some(crate::ir::EdgeDecoration::DiamondFilled);
    }
    if token.ends_with('*') {
        end_decoration = Some(crate::ir::EdgeDecoration::DiamondFilled);
    }
    if token.starts_with('o') {
        start_decoration = Some(crate::ir::EdgeDecoration::Diamond);
    }
    if token.ends_with('o') {
        end_decoration = Some(crate::ir::EdgeDecoration::Diamond);
    }

    let mut arrow_start_kind = None;
    let mut arrow_end_kind = None;
    if token.contains('|') {
        if arrow_start {
            arrow_start_kind = Some(crate::ir::EdgeArrowhead::OpenTriangle);
        }
        if arrow_end {
            arrow_end_kind = Some(crate::ir::EdgeArrowhead::OpenTriangle);
        }
    } else {
        if arrow_start {
            arrow_start_kind = Some(crate::ir::EdgeArrowhead::ClassDependency);
        }
        if arrow_end {
            arrow_end_kind = Some(crate::ir::EdgeArrowhead::ClassDependency);
        }
    }

    EdgeMeta {
        directed,
        arrow_start,
        arrow_end,
        arrow_start_kind,
        arrow_end_kind,
        start_decoration,
        end_decoration,
        style,
    }
}

fn parse_class_declaration(input: &str) -> Option<(String, Option<String>, Option<String>, bool)> {
    let mut rest = input.trim();
    if rest.is_empty() {
        return None;
    }

    let mut body: Option<String> = None;
    let mut open_body = false;
    if let Some(open_idx) = rest.find('{') {
        let header = rest[..open_idx].trim();
        let tail = rest[open_idx + 1..].trim();
        if let Some(close_idx) = tail.find('}') {
            let body_str = tail[..close_idx].trim();
            if !body_str.is_empty() {
                body = Some(body_str.to_string());
            }
        } else {
            open_body = true;
        }
        rest = header;
    }

    let lower = rest.to_ascii_lowercase();
    if let Some(as_idx) = lower.find(" as ") {
        let label_part = rest[..as_idx].trim();
        let id_part = rest[as_idx + 4..].trim();
        if !id_part.is_empty() {
            let label = strip_quotes(label_part);
            return Some((id_part.to_string(), Some(label), body, open_body));
        }
    }

    if rest.starts_with('"') && rest.ends_with('"') {
        let label = strip_quotes(rest);
        return Some((label.clone(), Some(label), body, open_body));
    }

    let id = strip_quotes(rest);
    Some((id, None, body, open_body))
}

fn split_class_body(body: &str) -> Vec<String> {
    let mut entries = Vec::new();
    for part in body.split(';') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        for line in trimmed.lines() {
            let line_trim = line.trim();
            if !line_trim.is_empty() {
                entries.push(line_trim.to_string());
            }
        }
    }
    entries
}

fn normalize_class_method_signature(entry: &str) -> String {
    let trimmed = entry.trim();
    let Some(close_idx) = trimmed.find(')') else {
        return trimmed.to_string();
    };
    let (sig, rest) = trimmed.split_at(close_idx + 1);
    let rest = rest.trim();
    if rest.is_empty() {
        return trimmed.to_string();
    }
    if rest.starts_with(':') {
        return format!("{} {}", sig, rest);
    }
    if trimmed.contains("):") || trimmed.contains(") :") {
        return trimmed.to_string();
    }
    format!("{} : {}", sig, rest)
}

fn parse_class_member_line(line: &str) -> Option<(String, String)> {
    let (left, right) = line.split_once(':')?;
    let id = left.trim();
    let member = right.trim();
    if id.is_empty() || member.is_empty() {
        return None;
    }
    if id.contains(' ') {
        return None;
    }
    Some((id.to_string(), member.to_string()))
}

fn normalize_class_id(token: &str) -> (String, Option<String>) {
    let trimmed = token.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') {
        let label = strip_quotes(trimmed);
        return (label.clone(), Some(label));
    }
    (trimmed.to_string(), None)
}

fn parse_state_alias_line(line: &str) -> Option<(String, String, Vec<String>)> {
    let trimmed = line.trim();
    if !trimmed.starts_with("state ") {
        return None;
    }
    if trimmed.contains('{') {
        return None;
    }
    let rest = trimmed.trim_start_matches("state ").trim();
    if !rest.starts_with('"') {
        return None;
    }
    let end_quote = rest[1..].find('"')? + 1;
    let label = rest[1..end_quote].to_string();
    let remaining = rest[end_quote + 1..].trim();
    if !remaining.to_ascii_lowercase().starts_with("as ") {
        return None;
    }
    let id = remaining[3..].trim();
    let (id, classes) = parse_state_id_with_classes(id);
    if id.is_empty() {
        return None;
    }
    Some((id, label, classes))
}

fn parse_state_stereotype(line: &str) -> (String, Option<crate::ir::NodeShape>, Option<String>) {
    let trimmed = line.trim();
    if !trimmed.starts_with("state ") {
        return (trimmed.to_string(), None, None);
    }
    let Some(start) = trimmed.find("<<") else {
        return (trimmed.to_string(), None, None);
    };
    let Some(end) = trimmed[start + 2..].find(">>") else {
        return (trimmed.to_string(), None, None);
    };
    let stereo_raw = &trimmed[start + 2..start + 2 + end];
    let stereo = stereo_raw.trim().to_ascii_lowercase();

    let before = trimmed[..start].trim_end();
    let after = trimmed[start + 2 + end + 2..].trim_start();
    let cleaned = if after.is_empty() {
        before.to_string()
    } else if before.is_empty() {
        after.to_string()
    } else {
        format!("{before} {after}")
    };

    let (shape, label_override) = match stereo.as_str() {
        "choice" => (Some(crate::ir::NodeShape::Diamond), None),
        "fork" | "join" => (Some(crate::ir::NodeShape::ForkJoin), Some(String::new())),
        _ => (None, None),
    };

    (cleaned, shape, label_override)
}

fn parse_state_description_line(line: &str) -> Option<(String, String, Vec<String>)> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.to_ascii_lowercase().starts_with("note ") {
        return None;
    }
    let rest = if trimmed.starts_with("state ") {
        trimmed[6..].trim()
    } else {
        trimmed
    };
    if rest.to_ascii_lowercase().contains(" as ") {
        return None;
    }
    let mut sep = None;
    let bytes = rest.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        if bytes[idx] == b':' {
            if idx + 2 < bytes.len() && bytes[idx + 1] == b':' && bytes[idx + 2] == b':' {
                idx += 3;
                continue;
            }
            sep = Some(idx);
            break;
        }
        idx += 1;
    }
    let sep = sep?;
    let (id_part, desc_part) = rest.split_at(sep);
    let desc_part = desc_part.get(1..).unwrap_or("");
    let (id, classes) = parse_state_id_with_classes(id_part.trim());
    let desc = strip_quotes(desc_part.trim());
    if id.is_empty() || desc.is_empty() {
        return None;
    }
    Some((id, desc, classes))
}

fn parse_state_id_with_classes(input: &str) -> (String, Vec<String>) {
    let (base, classes) = split_inline_classes(input);
    (strip_quotes(base.trim()), classes)
}

fn parse_state_note(line: &str) -> Option<(crate::ir::StateNotePosition, String, String)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("note ") {
        return None;
    }
    let rest = trimmed[4..].trim();
    let lower_rest = rest.to_ascii_lowercase();
    let (position, targets_part) = if lower_rest.starts_with("right of ") {
        (crate::ir::StateNotePosition::RightOf, rest[9..].trim())
    } else if lower_rest.starts_with("left of ") {
        (crate::ir::StateNotePosition::LeftOf, rest[8..].trim())
    } else {
        return None;
    };
    let (target, label) = targets_part.split_once(':')?;
    let target = target.trim();
    let label = label.trim();
    if target.is_empty() || label.is_empty() {
        return None;
    }
    Some((position, target.to_string(), label.to_string()))
}

fn parse_state_transition(line: &str) -> Option<(String, EdgeMeta, String, Option<String>)> {
    let tokens = ["<-->", "<--", "-->", "<->", "->", "<-", "..>", "<.."];
    for token in tokens {
        if let Some(pos) = line.find(token) {
            let left = line[..pos].trim();
            let right_part = line[pos + token.len()..].trim();
            if left.is_empty() || right_part.is_empty() {
                continue;
            }
            let (right, label) = split_label(right_part);
            let meta = edge_meta_from_state_token(token);
            return Some((left.to_string(), meta, right.to_string(), label));
        }
    }
    None
}

fn edge_meta_from_state_token(token: &str) -> EdgeMeta {
    let arrow_start = token.contains('<');
    let arrow_end = token.contains('>');
    let directed = arrow_start || arrow_end;
    let style = if token.contains("..") {
        crate::ir::EdgeStyle::Dotted
    } else {
        crate::ir::EdgeStyle::Solid
    };
    EdgeMeta {
        directed,
        arrow_start,
        arrow_end,
        arrow_start_kind: None,
        arrow_end_kind: None,
        start_decoration: None,
        end_decoration: None,
        style,
    }
}

fn normalize_state_token(
    token: &str,
    is_start: bool,
    start_counter: &mut usize,
    end_states: &mut HashMap<String, String>,
    scope: &str,
) -> (String, crate::ir::NodeShape, Option<String>) {
    let trimmed = token.trim();
    if trimmed == "[*]" || trimmed == "*" {
        let (id, shape) = if is_start {
            // Start states are unique - each [*] --> X creates a new start node
            let id = format!("__start_{}__", *start_counter);
            *start_counter += 1;
            (id, crate::ir::NodeShape::Circle)
        } else {
            // End states are shared per scope - all X --> [*] in same scope go to same node
            let id = end_states
                .entry(scope.to_string())
                .or_insert_with(|| format!("__end_{}__", scope))
                .clone();
            (id, crate::ir::NodeShape::DoubleCircle)
        };
        return (id, shape, Some(String::new()));
    }
    (strip_quotes(trimmed), crate::ir::NodeShape::RoundRect, None)
}

fn parse_state_simple(line: &str) -> Option<(String, Vec<String>)> {
    let trimmed = line.trim();
    if !trimmed.starts_with("state ") {
        return None;
    }
    if trimmed.contains('{') {
        return None;
    }
    let mut rest = trimmed.trim_start_matches("state ").trim();
    if rest.to_ascii_lowercase().contains(" as ") {
        return None;
    }
    if let Some(idx) = rest.find('{') {
        rest = rest[..idx].trim();
    }
    if rest.is_empty() {
        return None;
    }
    let (id, classes) = parse_state_id_with_classes(rest);
    if id.is_empty() {
        return None;
    }
    Some((id, classes))
}

fn parse_state_container_header(line: &str) -> Option<(Option<String>, String, String)> {
    let trimmed = line.trim();
    if !trimmed.starts_with("state ") {
        return None;
    }
    let brace_idx = trimmed.find('{')?;
    let head = trimmed[..brace_idx].trim();
    let tail = trimmed[brace_idx + 1..].trim().to_string();

    let rest = head.trim_start_matches("state ").trim();
    if rest.is_empty() {
        return None;
    }

    if rest.starts_with('"') {
        let end_quote = rest[1..].find('"')? + 1;
        let label = rest[1..end_quote].to_string();
        let remaining = rest[end_quote + 1..].trim();
        if remaining.to_ascii_lowercase().starts_with("as ") {
            let id = remaining[3..].trim();
            if id.is_empty() {
                return None;
            }
            return Some((Some(id.to_string()), label, tail));
        }
        return Some((None, label, tail));
    }

    let lower = rest.to_ascii_lowercase();
    if let Some(as_idx) = lower.find(" as ") {
        let id_part = rest[..as_idx].trim();
        let label_part = rest[as_idx + 4..].trim();
        if id_part.is_empty() || label_part.is_empty() {
            return None;
        }
        let id = strip_quotes(id_part);
        let label = strip_quotes(label_part);
        return Some((Some(id), label, tail));
    }

    let id = strip_quotes(rest);
    Some((Some(id.clone()), id, tail))
}

fn parse_sequence_participant(
    line: &str,
) -> Option<(String, Option<String>, crate::ir::NodeShape)> {
    let lowered = line.to_ascii_lowercase();
    let keywords = [
        ("participant ", crate::ir::NodeShape::ActorBox),
        ("actor ", crate::ir::NodeShape::ActorBox),
        ("boundary ", crate::ir::NodeShape::ActorBox),
        ("control ", crate::ir::NodeShape::ActorBox),
        ("entity ", crate::ir::NodeShape::ActorBox),
        ("database ", crate::ir::NodeShape::Cylinder),
    ];
    let mut rest = None;
    let mut shape = crate::ir::NodeShape::ActorBox;
    for (keyword, keyword_shape) in keywords {
        if lowered.starts_with(keyword) {
            rest = Some(line[keyword.len()..].trim());
            shape = keyword_shape;
            break;
        }
    }
    let rest = rest?;
    if rest.is_empty() {
        return None;
    }

    let lower_rest = rest.to_ascii_lowercase();
    if let Some(as_idx) = lower_rest.find(" as ") {
        let label_part = rest[..as_idx].trim();
        let id_part = rest[as_idx + 4..].trim();
        if id_part.is_empty() {
            return None;
        }
        let label = strip_quotes(label_part);
        return Some((id_part.to_string(), Some(label), shape));
    }

    if rest.starts_with('"') && rest.ends_with('"') {
        let label = strip_quotes(rest);
        return Some((label.clone(), Some(label), shape));
    }

    Some((strip_quotes(rest), None, shape))
}

fn is_color_token(token: &str) -> bool {
    let lower = token.trim().to_ascii_lowercase();
    lower == "transparent"
        || lower.starts_with('#')
        || lower.starts_with("rgb(")
        || lower.starts_with("rgba(")
        || lower.starts_with("hsl(")
        || lower.starts_with("hsla(")
}

fn parse_sequence_box_line(line: &str) -> Option<(Option<String>, Option<String>)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("box") {
        return None;
    }
    let rest = trimmed[3..].trim();
    if rest.is_empty() {
        return Some((None, None));
    }
    let tokens = tokenize_quoted(rest);
    if tokens.is_empty() {
        return Some((None, None));
    }
    let first = tokens[0].clone();
    if first.eq_ignore_ascii_case("transparent") {
        let label = tokens[1..].join(" ");
        let label = if label.trim().is_empty() {
            None
        } else {
            Some(label)
        };
        return Some((None, label));
    }
    let color = if tokens.len() > 1 {
        Some(first.clone())
    } else if is_color_token(&first) {
        Some(first.clone())
    } else {
        None
    };
    let label = if tokens.len() > 1 {
        Some(tokens[1..].join(" "))
    } else if color.is_none() {
        Some(first.clone())
    } else {
        None
    };
    let label = label.filter(|value| !value.trim().is_empty());
    let color = color.filter(|value| value.to_ascii_lowercase() != "transparent");
    Some((color, label))
}

fn ensure_sequence_node(
    graph: &mut Graph,
    labels: &HashMap<String, String>,
    id: &str,
    shape: Option<crate::ir::NodeShape>,
) {
    let label = labels.get(id).cloned();
    if let Some(shape) = shape {
        graph.ensure_node(id, label, Some(shape));
        return;
    }
    if graph.nodes.contains_key(id) {
        graph.ensure_node(id, label, None);
    } else {
        graph.ensure_node(id, label, Some(crate::ir::NodeShape::ActorBox));
    }
}

fn parse_sequence_message(
    line: &str,
) -> Option<(
    String,
    String,
    Option<String>,
    crate::ir::EdgeStyle,
    Option<crate::ir::SequenceActivationKind>,
)> {
    let tokens = [
        "-->>+", "->>+", "-->+", "->+", "-->>-", "->>-", "-->-", "->-", "<--+", "<-+", "<--", "<-",
        "-->>", "->>", "-->", "->",
    ];
    for token in tokens {
        if let Some(pos) = line.find(token) {
            let left = line[..pos].trim();
            let right_part = line[pos + token.len()..].trim();
            if left.is_empty() || right_part.is_empty() {
                continue;
            }
            let (right, label) = split_label(right_part);
            let mut from = left.to_string();
            let mut to = right.to_string();
            if token.starts_with('<') {
                std::mem::swap(&mut from, &mut to);
            }
            let trimmed = token.trim_start_matches('<').trim_end_matches(['+', '-']);
            let style = if trimmed.starts_with("--") {
                crate::ir::EdgeStyle::Dotted
            } else {
                crate::ir::EdgeStyle::Solid
            };
            let activation = if token.ends_with('+') {
                Some(crate::ir::SequenceActivationKind::Activate)
            } else if token.ends_with('-') {
                Some(crate::ir::SequenceActivationKind::Deactivate)
            } else {
                None
            };
            return Some((from, to, label, style, activation));
        }
    }
    None
}

fn parse_sequence_note(
    line: &str,
) -> Option<(crate::ir::SequenceNotePosition, Vec<String>, String)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("note ") {
        return None;
    }
    let rest = trimmed[4..].trim();
    let lower_rest = rest.to_ascii_lowercase();
    let (position, targets_part) = if lower_rest.starts_with("left of ") {
        (crate::ir::SequenceNotePosition::LeftOf, rest[8..].trim())
    } else if lower_rest.starts_with("right of ") {
        (crate::ir::SequenceNotePosition::RightOf, rest[9..].trim())
    } else if lower_rest.starts_with("over ") {
        (crate::ir::SequenceNotePosition::Over, rest[5..].trim())
    } else {
        return None;
    };

    let (targets, label) = targets_part.split_once(':')?;
    let label = label.trim();
    if label.is_empty() {
        return None;
    }
    let participants = targets
        .split(',')
        .map(|part| strip_quotes(part.trim()))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if participants.is_empty() {
        return None;
    }

    Some((position, participants, label.to_string()))
}

fn split_label(input: &str) -> (String, Option<String>) {
    if let Some((left, right)) = input.split_once(':') {
        let label = right.trim();
        let target = left.trim();
        if !label.is_empty() {
            return (target.to_string(), Some(label.to_string()));
        }
        return (target.to_string(), None);
    }
    (input.trim().to_string(), None)
}

fn parse_class_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Class;
    graph.direction = Direction::TopDown;
    let (lines, init_config) = preprocess_input(input)?;

    let mut members: HashMap<String, Vec<String>> = HashMap::new();
    let mut labels: HashMap<String, String> = HashMap::new();
    let mut current_class: Option<String> = None;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("classdiagram") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 1
                && let Some(dir) = Direction::from_token(parts[1])
            {
                graph.direction = dir;
            }
            continue;
        }

        if let Some(direction) = parse_direction_line(line) {
            graph.direction = direction;
            continue;
        }

        if let Some(active) = current_class.clone() {
            if let Some(end_idx) = line.find('}') {
                let fragment = line[..end_idx].trim();
                if !fragment.is_empty() {
                    members
                        .entry(active.clone())
                        .or_default()
                        .push(fragment.to_string());
                }
                current_class = None;
            } else {
                members
                    .entry(active.clone())
                    .or_default()
                    .push(line.to_string());
            }
            continue;
        }

        if let Some((left, right, meta, label, start_label, end_label)) =
            parse_class_relation_line(line)
        {
            let (left_id, left_label) = normalize_class_id(&left);
            let (right_id, right_label) = normalize_class_id(&right);
            if let Some(label) = left_label {
                labels.insert(left_id.clone(), label);
            }
            if let Some(label) = right_label {
                labels.insert(right_id.clone(), label);
            }
            graph.ensure_node(
                &left_id,
                labels.get(&left_id).cloned(),
                Some(crate::ir::NodeShape::Rectangle),
            );
            graph.ensure_node(
                &right_id,
                labels.get(&right_id).cloned(),
                Some(crate::ir::NodeShape::Rectangle),
            );
            graph.edges.push(crate::ir::Edge {
                from: left_id,
                to: right_id,
                label,
                start_label,
                end_label,
                directed: meta.directed,
                arrow_start: meta.arrow_start,
                arrow_end: meta.arrow_end,
                arrow_start_kind: meta.arrow_start_kind,
                arrow_end_kind: meta.arrow_end_kind,
                start_decoration: meta.start_decoration,
                end_decoration: meta.end_decoration,
                style: meta.style,
            });
            continue;
        }

        if line.starts_with("class ") {
            let rest = line.trim_start_matches("class ").trim();
            if let Some((id, label, body, open_body)) = parse_class_declaration(rest) {
                if let Some(label) = label.clone() {
                    labels.insert(id.clone(), label);
                }
                graph.ensure_node(
                    &id,
                    labels.get(&id).cloned(),
                    Some(crate::ir::NodeShape::Rectangle),
                );
                if let Some(body) = body {
                    for entry in split_class_body(&body) {
                        if !entry.is_empty() {
                            members.entry(id.clone()).or_default().push(entry);
                        }
                    }
                }
                if open_body {
                    current_class = Some(id.clone());
                }
                continue;
            }
        }

        if let Some((id, member)) = parse_class_member_line(line) {
            members.entry(id).or_default().push(member);
            continue;
        }
    }

    for (id, node) in graph.nodes.iter_mut() {
        let class_name = labels
            .get(id)
            .cloned()
            .unwrap_or_else(|| node.label.clone());
        let mut lines = Vec::new();
        lines.push(class_name.clone());
        if let Some(items) = members.get(id)
            && !items.is_empty()
        {
            let mut attrs = Vec::new();
            let mut methods = Vec::new();
            for entry in items {
                let trimmed = entry.trim();
                if trimmed.contains('(') && trimmed.contains(')') {
                    methods.push(normalize_class_method_signature(trimmed));
                } else {
                    attrs.push(trimmed.to_string());
                }
            }
            lines.push("---".to_string());
            if !attrs.is_empty() {
                lines.extend(attrs);
                if !methods.is_empty() {
                    lines.push("---".to_string());
                    lines.extend(methods);
                }
            } else {
                lines.extend(methods);
            }
        }
        node.label = lines.join("\n");
    }

    Ok(ParseOutput { graph, init_config })
}

fn is_er_card_char(ch: char) -> bool {
    matches!(ch, '|' | 'o' | '{' | '}')
}

fn split_er_cardinality_left(input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return (String::new(), None);
    }
    let chars: Vec<char> = trimmed.chars().collect();
    let len = chars.len();
    if len >= 2 {
        let last_two = &chars[len - 2..];
        if last_two.iter().all(|ch| is_er_card_char(*ch)) {
            let entity = chars[..len - 2].iter().collect::<String>();
            let token = last_two.iter().collect::<String>();
            return (entity.trim().to_string(), Some(token));
        }
    }
    if let Some(&last) = chars.last()
        && is_er_card_char(last)
    {
        let entity = chars[..len - 1].iter().collect::<String>();
        return (entity.trim().to_string(), Some(last.to_string()));
    }
    (trimmed.to_string(), None)
}

fn split_er_cardinality_right(input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return (String::new(), None);
    }
    let chars: Vec<char> = trimmed.chars().collect();
    let len = chars.len();
    if len >= 2 {
        let first_two = &chars[..2];
        if first_two.iter().all(|ch| is_er_card_char(*ch)) {
            let entity = chars[2..].iter().collect::<String>();
            let token = first_two.iter().collect::<String>();
            return (entity.trim().to_string(), Some(token));
        }
    }
    if is_er_card_char(chars[0]) {
        let entity = chars[1..].iter().collect::<String>();
        return (entity.trim().to_string(), Some(chars[0].to_string()));
    }
    (trimmed.to_string(), None)
}

fn normalize_er_cardinality(token: &str) -> (String, Option<crate::ir::EdgeDecoration>) {
    let trimmed = token.trim();
    match trimmed {
        "||" | "|" => (
            "1".to_string(),
            Some(crate::ir::EdgeDecoration::CrowsFootOne),
        ),
        "o|" | "|o" | "o" => (
            "0..1".to_string(),
            Some(crate::ir::EdgeDecoration::CrowsFootZeroOne),
        ),
        "|{" | "}|" => (
            "1..*".to_string(),
            Some(crate::ir::EdgeDecoration::CrowsFootMany),
        ),
        "o{" | "}o" | "}" | "{" => (
            "0..*".to_string(),
            Some(crate::ir::EdgeDecoration::CrowsFootZeroMany),
        ),
        _ => (trimmed.to_string(), None),
    }
}

fn parse_er_relation_line(
    line: &str,
) -> Option<(
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<crate::ir::EdgeDecoration>,
    Option<crate::ir::EdgeDecoration>,
    crate::ir::EdgeStyle,
)> {
    let (relation_part, label) = if let Some((before, after)) = line.split_once(':') {
        let label = after.trim();
        let label = if label.is_empty() {
            None
        } else {
            Some(label.to_string())
        };
        (before.trim(), label)
    } else {
        (line.trim(), None)
    };

    let (sep, style) = if let Some(idx) = relation_part.find("--") {
        (idx, crate::ir::EdgeStyle::Solid)
    } else if let Some(idx) = relation_part.find("..") {
        (idx, crate::ir::EdgeStyle::Dotted)
    } else {
        return None;
    };
    let left_part = relation_part[..sep].trim();
    let right_part = relation_part[sep + 2..].trim();
    if left_part.is_empty() || right_part.is_empty() {
        return None;
    }
    let (left_entity, left_card) = split_er_cardinality_left(left_part);
    let (right_entity, right_card) = split_er_cardinality_right(right_part);
    if left_entity.is_empty() || right_entity.is_empty() {
        return None;
    }
    let left_id = strip_quotes(left_entity.trim());
    let right_id = strip_quotes(right_entity.trim());
    if left_id.is_empty() || right_id.is_empty() {
        return None;
    }
    let (left_label, left_decoration) = left_card
        .map(|token| normalize_er_cardinality(&token))
        .map(|(label, dec)| (Some(label), dec))
        .unwrap_or((None, None));
    let (right_label, right_decoration) = right_card
        .map(|token| normalize_er_cardinality(&token))
        .map(|(label, dec)| (Some(label), dec))
        .unwrap_or((None, None));
    Some((
        left_id,
        right_id,
        label,
        left_label,
        right_label,
        left_decoration,
        right_decoration,
        style,
    ))
}

fn parse_er_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Er;
    graph.direction = Direction::TopDown;
    let (lines, init_config) = preprocess_input(input)?;

    let mut members: HashMap<String, Vec<String>> = HashMap::new();
    let mut current_entity: Option<String> = None;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("erdiagram") {
            continue;
        }
        if let Some(direction) = parse_direction_line(line) {
            graph.direction = direction;
            continue;
        }

        if let Some(active) = current_entity.clone() {
            if let Some(end_idx) = line.find('}') {
                let fragment = line[..end_idx].trim();
                if !fragment.is_empty() {
                    members
                        .entry(active.clone())
                        .or_default()
                        .push(fragment.to_string());
                }
                current_entity = None;
            } else {
                members
                    .entry(active.clone())
                    .or_default()
                    .push(line.to_string());
            }
            continue;
        }

        if let Some((
            left,
            right,
            label,
            _left_label,
            _right_label,
            left_decoration,
            right_decoration,
            style,
        )) = parse_er_relation_line(line)
        {
            graph.ensure_node(&left, None, Some(crate::ir::NodeShape::Rectangle));
            graph.ensure_node(&right, None, Some(crate::ir::NodeShape::Rectangle));
            // Don't use start_label/end_label for ER diagrams - crow's foot symbols convey cardinality
            graph.edges.push(crate::ir::Edge {
                from: left,
                to: right,
                label,
                start_label: None,
                end_label: None,
                directed: false,
                arrow_start: false,
                arrow_end: false,
                arrow_start_kind: None,
                arrow_end_kind: None,
                start_decoration: left_decoration,
                end_decoration: right_decoration,
                style,
            });
            continue;
        }

        if let Some(open_idx) = line.find('{') {
            let name = line[..open_idx].trim();
            let name = strip_quotes(name);
            if !name.is_empty() {
                graph.ensure_node(&name, None, Some(crate::ir::NodeShape::Rectangle));
                current_entity = Some(name.clone());
                let tail = line[open_idx + 1..].trim();
                if let Some(close_idx) = tail.find('}') {
                    let fragment = tail[..close_idx].trim();
                    if !fragment.is_empty() {
                        members.entry(name).or_default().push(fragment.to_string());
                    }
                    current_entity = None;
                } else if !tail.is_empty() {
                    members.entry(name).or_default().push(tail.to_string());
                }
            }
            continue;
        }

        let entity = strip_quotes(line);
        if !entity.is_empty() {
            graph.ensure_node(&entity, None, Some(crate::ir::NodeShape::Rectangle));
        }
    }

    for (id, node) in graph.nodes.iter_mut() {
        let mut lines = Vec::new();
        lines.push(node.label.clone());
        if let Some(attrs) = members.get(id)
            && !attrs.is_empty()
        {
            lines.push("---".to_string());
            lines.extend(attrs.clone());
        }
        node.label = lines.join("\n");
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_pie_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Pie;
    let (lines, init_config) = preprocess_input(input)?;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("pie") {
            if lower.contains("showdata") {
                graph.pie_show_data = true;
            }
            // Check for title on the same line: "pie title My Title"
            if let Some(title_pos) = lower.find("title") {
                let title_start = title_pos + 5; // len("title")
                if let Some(title) = line.get(title_start..) {
                    let title = title.trim();
                    if !title.is_empty() {
                        graph.pie_title = Some(title.to_string());
                    }
                }
            }
            continue;
        }
        if lower.starts_with("showdata") {
            graph.pie_show_data = true;
            continue;
        }
        if lower.starts_with("title") {
            let title = line.get(5..).unwrap_or("").trim();
            if !title.is_empty() {
                graph.pie_title = Some(title.to_string());
            }
            continue;
        }
        if let Some((label, value)) = parse_pie_slice_line(line) {
            graph.pie_slices.push(crate::ir::PieSlice { label, value });
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_pie_slice_line(line: &str) -> Option<(String, f32)> {
    let (label_part, value_part) = line.split_once(':')?;
    let label = strip_quotes(label_part.trim());
    if label.is_empty() {
        return None;
    }
    let value_str = value_part.trim();
    if value_str.is_empty() {
        return None;
    }
    let value = value_str.parse::<f32>().ok()?;
    Some((label, value))
}

fn parse_mindmap_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Mindmap;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input_keep_indent(input)?;
    let mut stack: Vec<String> = Vec::new();
    let mut base_indent: Option<usize> = None;
    let mut node_index: HashMap<String, usize> = HashMap::new();

    for raw_line in lines {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("mindmap") {
            continue;
        }

        let indent = count_indent(&raw_line);
        let base = *base_indent.get_or_insert(indent);
        let rel_indent = indent.saturating_sub(base);
        let mut level = rel_indent / 2;
        if level > stack.len() {
            level = stack.len();
        }

        let (raw_id, label, node_type, classes) = parse_mindmap_node_token(trimmed);
        let mut id = raw_id;
        if id.is_empty() {
            id = sanitize_id(&label);
        }
        if id.is_empty() {
            id = format!("mindmap_{}", graph.mindmap.nodes.len());
        }
        if graph.nodes.contains_key(&id) {
            id = format!("{}_{}", id, graph.nodes.len());
        }

        let shape = match node_type {
            crate::ir::MindmapNodeType::Circle => crate::ir::NodeShape::Circle,
            crate::ir::MindmapNodeType::RoundedRect => crate::ir::NodeShape::RoundRect,
            crate::ir::MindmapNodeType::Rect => crate::ir::NodeShape::Rectangle,
            crate::ir::MindmapNodeType::Hexagon => crate::ir::NodeShape::Hexagon,
            crate::ir::MindmapNodeType::Cloud | crate::ir::MindmapNodeType::Bang => {
                crate::ir::NodeShape::RoundRect
            }
            crate::ir::MindmapNodeType::Default => crate::ir::NodeShape::MindmapDefault,
        };

        graph.ensure_node(&id, Some(label.clone()), Some(shape));
        if !classes.is_empty() {
            apply_node_classes(&mut graph, &id, &classes);
        }

        if graph.mindmap.root_id.is_none() {
            graph.mindmap.root_id = Some(id.clone());
        }

        if level > 0 && stack.len() > level {
            stack.truncate(level);
        }

        let parent_id = if level > 0 {
            stack.last().cloned()
        } else {
            None
        };

        let section = if level == 0 {
            None
        } else if let Some(parent_id) = parent_id.as_ref() {
            let parent_idx = node_index.get(parent_id).copied();
            if let Some(parent_idx) = parent_idx {
                let parent = &graph.mindmap.nodes[parent_idx];
                if parent.level == 0 {
                    Some(parent.children.len())
                } else {
                    parent.section
                }
            } else {
                None
            }
        } else {
            None
        };

        let node = crate::ir::MindmapNode {
            id: id.clone(),
            label: label.clone(),
            level,
            section,
            node_type,
            icon: None,
            class: None,
            children: Vec::new(),
        };

        let idx = graph.mindmap.nodes.len();
        graph.mindmap.nodes.push(node);
        node_index.insert(id.clone(), idx);

        if let Some(parent_id) = parent_id {
            if let Some(parent_idx) = node_index.get(&parent_id).copied() {
                graph.mindmap.nodes[parent_idx].children.push(id.clone());
            }
            graph.edges.push(crate::ir::Edge {
                from: parent_id,
                to: id.clone(),
                label: None,
                start_label: None,
                end_label: None,
                directed: false,
                arrow_start: false,
                arrow_end: false,
                arrow_start_kind: None,
                arrow_end_kind: None,
                start_decoration: None,
                end_decoration: None,
                style: crate::ir::EdgeStyle::Solid,
            });
        } else {
            stack.clear();
        }

        stack.push(id);
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_mindmap_node_token(
    token: &str,
) -> (String, String, crate::ir::MindmapNodeType, Vec<String>) {
    let (base, classes) = split_inline_classes(token);
    let trimmed = base.trim();
    if trimmed.is_empty() {
        return (
            String::new(),
            String::new(),
            crate::ir::MindmapNodeType::Default,
            classes,
        );
    }

    let mut id = String::new();
    let mut label = trimmed.to_string();
    let mut node_type = crate::ir::MindmapNodeType::Default;

    let shape_start = trimmed
        .find(|ch: char| ch == '[' || ch == '(' || ch == '{')
        .unwrap_or(0);
    if shape_start > 0 && !trimmed[..shape_start].contains(' ') {
        id = trimmed[..shape_start].trim().to_string();
        let raw = trimmed[shape_start..].trim();
        if let Some((shape_label, shape_type)) = parse_mindmap_shape(raw) {
            label = shape_label;
            node_type = shape_type;
        }
    } else if let Some((shape_label, shape_type)) = parse_mindmap_shape(trimmed) {
        label = shape_label;
        node_type = shape_type;
    }

    if id.is_empty() {
        id = sanitize_id(&label);
    }

    (id, label, node_type, classes)
}

fn parse_mindmap_shape(raw: &str) -> Option<(String, crate::ir::MindmapNodeType)> {
    let trimmed = raw.trim();
    if trimmed.starts_with("((") && trimmed.ends_with("))") {
        return Some((
            strip_quotes(&trimmed[2..trimmed.len() - 2]),
            crate::ir::MindmapNodeType::Circle,
        ));
    }
    if trimmed.starts_with("{{") && trimmed.ends_with("}}") {
        return Some((
            strip_quotes(&trimmed[2..trimmed.len() - 2]),
            crate::ir::MindmapNodeType::Hexagon,
        ));
    }
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        return Some((
            strip_quotes(&trimmed[1..trimmed.len() - 1]),
            crate::ir::MindmapNodeType::Rect,
        ));
    }
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        return Some((
            strip_quotes(&trimmed[1..trimmed.len() - 1]),
            crate::ir::MindmapNodeType::RoundedRect,
        ));
    }
    None
}

fn sanitize_id(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch.is_alphanumeric() {
            out.push(ch);
        } else if ch.is_whitespace() || ch == '-' || ch == '_' {
            if !out.ends_with('_') {
                out.push('_');
            }
        }
    }
    out.trim_matches('_').to_string()
}

fn parse_journey_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Journey;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;

    let mut current_section: Option<usize> = None;
    let mut last_task: Option<String> = None;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("journey") {
            continue;
        }
        if lower.starts_with("title") {
            continue;
        }
        if lower.starts_with("section") {
            let label = line.get(7..).unwrap_or("").trim();
            let id = format!("section_{}", graph.subgraphs.len());
            graph.subgraphs.push(Subgraph {
                id: Some(id),
                label: label.to_string(),
                nodes: Vec::new(),
                direction: None,
            });
            current_section = Some(graph.subgraphs.len() - 1);
            last_task = None;
            continue;
        }

        if let Some((label, score, actors)) = parse_journey_task_line(line) {
            let node_id = format!("journey_{}", graph.nodes.len());
            let mut node_label = label;
            if let Some(score) = score {
                node_label.push_str(&format!("\nscore: {}", score));
            }
            if !actors.is_empty() {
                node_label.push_str(&format!("\n{}", actors.join(", ")));
            }
            graph.ensure_node(
                &node_id,
                Some(node_label),
                Some(crate::ir::NodeShape::Rectangle),
            );
            if let Some(idx) = current_section {
                if let Some(subgraph) = graph.subgraphs.get_mut(idx) {
                    subgraph.nodes.push(node_id.clone());
                }
            }
            if let Some(prev) = last_task.take() {
                graph.edges.push(crate::ir::Edge {
                    from: prev,
                    to: node_id.clone(),
                    label: None,
                    start_label: None,
                    end_label: None,
                    directed: false,
                    arrow_start: false,
                    arrow_end: false,
                    arrow_start_kind: None,
                    arrow_end_kind: None,
                    start_decoration: None,
                    end_decoration: None,
                    style: crate::ir::EdgeStyle::Solid,
                });
            }
            last_task = Some(node_id);
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_journey_task_line(line: &str) -> Option<(String, Option<String>, Vec<String>)> {
    let mut parts = line.split(':').map(|part| part.trim()).collect::<Vec<_>>();
    if parts.len() < 2 {
        return None;
    }
    let label = parts.remove(0).to_string();
    if label.is_empty() {
        return None;
    }
    let score = parts
        .get(0)
        .map(|value| value.to_string())
        .filter(|value| !value.is_empty());
    let actors = if parts.len() >= 2 {
        parts[1]
            .split(',')
            .map(|actor| actor.trim().to_string())
            .filter(|actor| !actor.is_empty())
            .collect()
    } else {
        Vec::new()
    };
    Some((label, score, actors))
}

fn parse_timeline_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Timeline;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;

    let mut current_section: Option<String> = None;
    let mut pending_time: Option<String> = None;
    let mut pending_events: Vec<String> = Vec::new();

    let flush_pending = |graph: &mut Graph,
                         pending_time: &mut Option<String>,
                         pending_events: &mut Vec<String>,
                         current_section: &Option<String>| {
        if let Some(time) = pending_time.take() {
            graph.timeline.events.push(crate::ir::TimelineEvent {
                time,
                events: std::mem::take(pending_events),
                section: current_section.clone(),
            });
        }
        pending_events.clear();
    };

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("timeline") {
            continue;
        }
        if lower.starts_with("title") {
            let rest = line.get(5..).unwrap_or("").trim();
            if !rest.is_empty() {
                graph.timeline.title = Some(strip_quotes(rest));
            }
            continue;
        }
        if lower.starts_with("section") {
            // Flush any pending event before starting new section
            flush_pending(
                &mut graph,
                &mut pending_time,
                &mut pending_events,
                &current_section,
            );

            let label = line.get(7..).unwrap_or("").trim();
            graph.timeline.sections.push(label.to_string());
            current_section = Some(label.to_string());
            continue;
        }

        // Parse timeline event line: "time : event" or "time : event1 : event2"
        if let Some(colon_idx) = line.find(':') {
            let time_part = line[..colon_idx].trim();
            let events_part = line[colon_idx + 1..].trim();

            if !time_part.is_empty() {
                // New time entry - flush any previous
                flush_pending(
                    &mut graph,
                    &mut pending_time,
                    &mut pending_events,
                    &current_section,
                );
                pending_time = Some(time_part.to_string());

                // Parse events (can be multiple separated by :)
                for event in events_part.split(':') {
                    let event = event.trim();
                    if !event.is_empty() {
                        pending_events.push(event.to_string());
                    }
                }
            }
        }
    }

    // Flush any remaining pending event
    flush_pending(
        &mut graph,
        &mut pending_time,
        &mut pending_events,
        &current_section,
    );

    Ok(ParseOutput { graph, init_config })
}

fn parse_gantt_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Gantt;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;

    let mut current_section: Option<usize> = None;
    let mut current_section_name: Option<String> = None;
    let mut last_task: Option<String> = None;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("gantt") {
            continue;
        }
        if lower.starts_with("title") {
            let title = line.get(5..).unwrap_or("").trim();
            if !title.is_empty() {
                graph.gantt_title = Some(title.to_string());
            }
            continue;
        }
        if lower.starts_with("dateformat")
            || lower.starts_with("axisformat")
            || lower.starts_with("tickinterval")
            || lower.starts_with("todaymarker")
            || lower.starts_with("excludes")
            || lower.starts_with("includes")
        {
            continue;
        }
        if lower.starts_with("section") {
            let label = line.get(7..).unwrap_or("").trim();
            let id = format!("section_{}", graph.subgraphs.len());
            graph.subgraphs.push(Subgraph {
                id: Some(id),
                label: label.to_string(),
                nodes: Vec::new(),
                direction: None,
            });
            current_section = Some(graph.subgraphs.len() - 1);
            current_section_name = Some(label.to_string());
            graph.gantt_sections.push(label.to_string());
            last_task = None;
            continue;
        }

        if let Some((task_label, meta)) = line.split_once(':') {
            let label = task_label.trim();
            if label.is_empty() {
                continue;
            }
            let (id, details, after) = parse_gantt_task_meta(meta);
            let node_id = id
                .clone()
                .unwrap_or_else(|| format!("gantt_{}", graph.nodes.len()));
            let mut node_label = label.to_string();
            if !details.is_empty() {
                node_label.push_str(&format!("\n{}", details.join(" | ")));
            }

            // Add to gantt_tasks
            let (start, duration) = extract_gantt_timing(&details);
            graph.gantt_tasks.push(crate::ir::GanttTask {
                id: node_id.clone(),
                label: label.to_string(),
                start,
                duration,
                after: after.clone(),
                section: current_section_name.clone(),
            });

            graph.ensure_node(
                &node_id,
                Some(node_label),
                Some(crate::ir::NodeShape::Rectangle),
            );
            if let Some(idx) = current_section {
                if let Some(subgraph) = graph.subgraphs.get_mut(idx) {
                    subgraph.nodes.push(node_id.clone());
                }
            }

            if let Some(after_id) = after {
                graph.ensure_node(&after_id, None, Some(crate::ir::NodeShape::Rectangle));
                graph.edges.push(crate::ir::Edge {
                    from: after_id,
                    to: node_id.clone(),
                    label: None,
                    start_label: None,
                    end_label: None,
                    directed: true,
                    arrow_start: false,
                    arrow_end: true,
                    arrow_start_kind: None,
                    arrow_end_kind: None,
                    start_decoration: None,
                    end_decoration: None,
                    style: crate::ir::EdgeStyle::Solid,
                });
            } else if let Some(prev) = last_task.take() {
                graph.edges.push(crate::ir::Edge {
                    from: prev,
                    to: node_id.clone(),
                    label: None,
                    start_label: None,
                    end_label: None,
                    directed: false,
                    arrow_start: false,
                    arrow_end: false,
                    arrow_start_kind: None,
                    arrow_end_kind: None,
                    start_decoration: None,
                    end_decoration: None,
                    style: crate::ir::EdgeStyle::Solid,
                });
            }

            last_task = Some(node_id);
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_gantt_task_meta(meta: &str) -> (Option<String>, Vec<String>, Option<String>) {
    let mut id: Option<String> = None;
    let mut details: Vec<String> = Vec::new();
    let mut after: Option<String> = None;

    for raw_token in meta.split(',') {
        let token = raw_token.trim();
        if token.is_empty() {
            continue;
        }
        let lower = token.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("after ") {
            let dep = rest.trim().to_string();
            if !dep.is_empty() {
                after = Some(dep);
            }
            continue;
        }
        if is_gantt_status_token(&lower) {
            details.push(token.to_string());
            continue;
        }
        if looks_like_date(token) || looks_like_duration(token) {
            details.push(token.to_string());
            continue;
        }
        if id.is_none() {
            id = Some(token.to_string());
        } else {
            details.push(token.to_string());
        }
    }

    (id, details, after)
}

fn is_gantt_status_token(token: &str) -> bool {
    matches!(token, "done" | "active" | "crit" | "milestone")
}

fn looks_like_date(token: &str) -> bool {
    token.contains('-') || token.contains('/') || token.contains('.')
}

fn looks_like_duration(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    matches!(
        lower.chars().last(),
        Some('d') | Some('h') | Some('w') | Some('m') | Some('y')
    )
}

fn extract_gantt_timing(details: &[String]) -> (Option<String>, Option<String>) {
    let mut start: Option<String> = None;
    let mut duration: Option<String> = None;
    for detail in details {
        if looks_like_date(detail) && start.is_none() {
            start = Some(detail.clone());
        } else if looks_like_duration(detail) && duration.is_none() {
            duration = Some(detail.clone());
        }
    }
    (start, duration)
}

fn requirement_kind_label(kind: &str) -> String {
    let lower = kind.trim().to_ascii_lowercase();
    let mut chars = lower.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::new();
    out.push(first.to_ascii_uppercase());
    out.extend(chars);
    out
}

fn requirement_title_case(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let lower = trimmed.to_ascii_lowercase();
    let mut chars = lower.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::new();
    out.push(first.to_ascii_uppercase());
    out.extend(chars);
    out
}

fn normalize_requirement_attr(line: &str) -> String {
    let Some((key_raw, value_raw)) = line.split_once(':') else {
        return line.trim().to_string();
    };
    let key = key_raw.trim().to_ascii_lowercase();
    let value = value_raw.trim();
    let pretty_key = match key.as_str() {
        "id" => "ID".to_string(),
        "text" => "Text".to_string(),
        "risk" => "Risk".to_string(),
        "verifymethod" | "verification" => "Verification".to_string(),
        other => requirement_kind_label(other),
    };
    let pretty_value = match key.as_str() {
        "risk" | "verifymethod" | "verification" => requirement_title_case(value),
        _ => value.to_string(),
    };
    if pretty_value.is_empty() {
        pretty_key
    } else {
        format!("{pretty_key}: {pretty_value}")
    }
}

fn parse_requirement_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Requirement;
    graph.direction = Direction::TopDown;
    let (lines, init_config) = preprocess_input(input)?;

    let mut attributes: HashMap<String, Vec<String>> = HashMap::new();
    let mut current_id: Option<String> = None;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("requirementdiagram") {
            continue;
        }

        if let Some(active) = current_id.clone() {
            if let Some(end_idx) = line.find('}') {
                let fragment = line[..end_idx].trim();
                if !fragment.is_empty() {
                    attributes
                        .entry(active.clone())
                        .or_default()
                        .push(fragment.to_string());
                }
                current_id = None;
            } else {
                attributes
                    .entry(active.clone())
                    .or_default()
                    .push(line.to_string());
            }
            continue;
        }

        if let Some((from, rel, to)) = parse_requirement_relation_line(line) {
            graph.ensure_node(&from, None, Some(crate::ir::NodeShape::Rectangle));
            graph.ensure_node(&to, None, Some(crate::ir::NodeShape::Rectangle));
            graph.edges.push(crate::ir::Edge {
                from,
                to,
                label: Some(rel),
                start_label: None,
                end_label: None,
                directed: true,
                arrow_start: false,
                arrow_end: true,
                arrow_start_kind: None,
                arrow_end_kind: None,
                start_decoration: None,
                end_decoration: None,
                style: crate::ir::EdgeStyle::Solid,
            });
            continue;
        }

        if let Some(open_idx) = line.find('{') {
            let header = line[..open_idx].trim();
            let mut parts = header.split_whitespace();
            let kind = parts.next().unwrap_or("").to_string();
            let id = parts.next().unwrap_or("").to_string();
            if !id.is_empty() {
                let label = if kind.is_empty() {
                    id.clone()
                } else {
                    let kind_label = requirement_kind_label(&kind);
                    format!("<<{}>>\n{}", kind_label, id)
                };
                graph.ensure_node(&id, Some(label), Some(crate::ir::NodeShape::Rectangle));
                current_id = Some(id.clone());
                let tail = line[open_idx + 1..].trim();
                if let Some(close_idx) = tail.find('}') {
                    let fragment = tail[..close_idx].trim();
                    if !fragment.is_empty() {
                        attributes.entry(id).or_default().push(fragment.to_string());
                    }
                    current_id = None;
                } else if !tail.is_empty() {
                    attributes.entry(id).or_default().push(tail.to_string());
                }
            }
            continue;
        }

        let mut parts = line.split_whitespace();
        let kind = parts.next().unwrap_or("");
        let id = parts.next().unwrap_or("");
        if !id.is_empty() {
            let label = if kind.is_empty() {
                id.to_string()
            } else {
                let kind_label = requirement_kind_label(kind);
                format!("<<{}>>\n{}", kind_label, id)
            };
            graph.ensure_node(id, Some(label), Some(crate::ir::NodeShape::Rectangle));
        }
    }

    for (id, node) in graph.nodes.iter_mut() {
        if let Some(attrs) = attributes.get(id)
            && !attrs.is_empty()
        {
            let mut lines = Vec::new();
            lines.push(node.label.clone());
            lines.extend(attrs.iter().map(|attr| normalize_requirement_attr(attr)));
            node.label = lines.join("\n");
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_requirement_relation_line(line: &str) -> Option<(String, String, String)> {
    let (left, right) = line.split_once("->")?;
    let to = right.trim();
    if to.is_empty() {
        return None;
    }
    let left = left.trim();
    let (from_part, rel_part) = left.split_once('-')?;
    let from = from_part.trim();
    let rel = rel_part.trim().trim_matches('-').trim();
    let rel_clean = rel.trim_start_matches('<').trim_end_matches('>').trim();
    if from.is_empty() || rel_clean.is_empty() {
        return None;
    }
    Some((from.to_string(), rel_clean.to_string(), to.to_string()))
}

fn parse_gitgraph_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::GitGraph;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;

    let mut branch_heads: HashMap<String, Option<String>> = HashMap::new();
    let mut branch_insertion: HashMap<String, usize> = HashMap::new();

    let main_branch = "main".to_string();
    graph.gitgraph.main_branch = main_branch.clone();
    branch_heads.insert(main_branch.clone(), None);
    branch_insertion.insert(main_branch.clone(), 0);
    graph.gitgraph.branches.push(crate::ir::GitGraphBranch {
        name: main_branch.clone(),
        order: Some(0.0),
        insertion_index: 0,
    });

    let mut current_branch = main_branch;
    let mut commit_seq: usize = 0;
    let mut rng = GitGraphIdRng::new(hash_seed(input));

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("gitgraph") {
            continue;
        }
        if let Some(direction) = parse_gitgraph_direction(line) {
            graph.direction = direction;
            continue;
        }
        if lower.starts_with("branch ") {
            let name = line.get(7..).unwrap_or("").trim();
            if !name.is_empty() {
                let order = extract_gitgraph_order(line);
                let head = branch_heads.get(&current_branch).cloned().unwrap_or(None);
                branch_heads.insert(name.to_string(), head);
                if !branch_insertion.contains_key(name) {
                    let idx = graph.gitgraph.branches.len();
                    branch_insertion.insert(name.to_string(), idx);
                    graph.gitgraph.branches.push(crate::ir::GitGraphBranch {
                        name: name.to_string(),
                        order,
                        insertion_index: idx,
                    });
                }
                current_branch = name.to_string();
            }
            continue;
        }
        if lower.starts_with("checkout ") || lower.starts_with("switch ") {
            let name = if lower.starts_with("checkout ") {
                line.get(9..).unwrap_or("").trim()
            } else {
                line.get(7..).unwrap_or("").trim()
            };
            if !name.is_empty() {
                current_branch = name.to_string();
                branch_heads.entry(current_branch.clone()).or_insert(None);
                if !branch_insertion.contains_key(name) {
                    let idx = graph.gitgraph.branches.len();
                    branch_insertion.insert(name.to_string(), idx);
                    graph.gitgraph.branches.push(crate::ir::GitGraphBranch {
                        name: name.to_string(),
                        order: None,
                        insertion_index: idx,
                    });
                }
            }
            continue;
        }
        if lower.starts_with("merge ") {
            let from_branch = line.get(6..).unwrap_or("").trim();
            if from_branch.is_empty() {
                continue;
            }
            let from_head = branch_heads.get(from_branch).cloned().unwrap_or(None);
            let current_head = branch_heads.get(&current_branch).cloned().unwrap_or(None);
            if from_head.is_none() && current_head.is_none() {
                continue;
            }
            let mut parents = Vec::new();
            if let Some(parent) = current_head.clone() {
                parents.push(parent);
            }
            if let Some(parent) = from_head.clone() {
                parents.push(parent);
            }

            let (id, custom_id) = extract_gitgraph_id(line)
                .map(|value| (value, true))
                .unwrap_or_else(|| {
                    let hex = rng.next_hex(7);
                    (format!("{commit_seq}-{hex}"), false)
                });
            let tags = extract_gitgraph_tags(line);
            let custom_type = extract_gitgraph_commit_type(line);
            let commit = crate::ir::GitGraphCommit {
                id: id.clone(),
                message: Some(format!(
                    "merged branch {} into {}",
                    from_branch, current_branch
                )),
                seq: commit_seq,
                commit_type: crate::ir::GitGraphCommitType::Merge,
                custom_type,
                tags,
                parents,
                branch: current_branch.clone(),
                custom_id,
            };
            commit_seq += 1;
            graph.gitgraph.commits.push(commit);
            branch_heads.insert(current_branch.clone(), Some(id));
            continue;
        }
        if lower.starts_with("commit") {
            let (id, custom_id) = extract_gitgraph_id(line)
                .map(|value| (value, true))
                .unwrap_or_else(|| {
                    let hex = rng.next_hex(7);
                    (format!("{commit_seq}-{hex}"), false)
                });
            let tags = extract_gitgraph_tags(line);
            let commit_type =
                extract_gitgraph_commit_type(line).unwrap_or(crate::ir::GitGraphCommitType::Normal);
            let parents = branch_heads
                .get(&current_branch)
                .cloned()
                .unwrap_or(None)
                .map(|parent| vec![parent])
                .unwrap_or_default();
            let message = extract_gitgraph_message(line);
            let commit = crate::ir::GitGraphCommit {
                id: id.clone(),
                message,
                seq: commit_seq,
                commit_type,
                custom_type: None,
                tags,
                parents,
                branch: current_branch.clone(),
                custom_id,
            };
            commit_seq += 1;
            graph.gitgraph.commits.push(commit);
            branch_heads.insert(current_branch.clone(), Some(id));
            continue;
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_gitgraph_direction(line: &str) -> Option<Direction> {
    let trimmed = line.trim();
    if trimmed.eq_ignore_ascii_case("LR") {
        return Some(Direction::LeftRight);
    }
    if trimmed.eq_ignore_ascii_case("TB") {
        return Some(Direction::TopDown);
    }
    if trimmed.eq_ignore_ascii_case("BT") {
        return Some(Direction::BottomTop);
    }
    let lower = trimmed.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("direction") {
        let token = rest.trim();
        if token.eq_ignore_ascii_case("lr") {
            return Some(Direction::LeftRight);
        }
        if token.eq_ignore_ascii_case("tb") {
            return Some(Direction::TopDown);
        }
        if token.eq_ignore_ascii_case("bt") {
            return Some(Direction::BottomTop);
        }
    }
    None
}

fn extract_gitgraph_id(line: &str) -> Option<String> {
    extract_gitgraph_attr(line, "id")
}

fn extract_gitgraph_message(line: &str) -> Option<String> {
    extract_gitgraph_attr(line, "msg")
}

fn extract_gitgraph_commit_type(line: &str) -> Option<crate::ir::GitGraphCommitType> {
    let raw = extract_gitgraph_attr(line, "type")?;
    match raw.to_ascii_uppercase().as_str() {
        "NORMAL" => Some(crate::ir::GitGraphCommitType::Normal),
        "REVERSE" => Some(crate::ir::GitGraphCommitType::Reverse),
        "HIGHLIGHT" => Some(crate::ir::GitGraphCommitType::Highlight),
        _ => None,
    }
}

fn extract_gitgraph_order(line: &str) -> Option<f32> {
    let raw = extract_gitgraph_attr(line, "order")?;
    raw.parse::<f32>().ok()
}

fn extract_gitgraph_tags(line: &str) -> Vec<String> {
    extract_gitgraph_attrs(line, "tag")
}

fn extract_gitgraph_attrs(line: &str, key: &str) -> Vec<String> {
    let mut values = Vec::new();
    let lower = line.to_ascii_lowercase();
    let needle = format!("{}:", key);
    let mut start = 0;
    while let Some(idx) = lower[start..].find(&needle) {
        let offset = start + idx;
        if let Some((value, next)) = extract_gitgraph_attr_at(line, offset + needle.len()) {
            values.push(value);
            start = next;
        } else {
            break;
        }
    }
    values
}

fn extract_gitgraph_attr_at(line: &str, start: usize) -> Option<(String, usize)> {
    let bytes = line.as_bytes();
    let mut idx = start;
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }
    if idx >= bytes.len() {
        return None;
    }
    let first = bytes[idx] as char;
    if first == '"' || first == '\'' {
        idx += 1;
        let begin = idx;
        while idx < bytes.len() && bytes[idx] as char != first {
            idx += 1;
        }
        let value = String::from_utf8_lossy(&bytes[begin..idx]).to_string();
        let next = (idx + 1).min(bytes.len());
        return Some((value, next));
    }
    let begin = idx;
    while idx < bytes.len() && !bytes[idx].is_ascii_whitespace() && bytes[idx] != b',' {
        idx += 1;
    }
    let value = String::from_utf8_lossy(&bytes[begin..idx]).to_string();
    Some((value, idx))
}

fn hash_seed(input: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

struct GitGraphIdRng {
    state: u64,
}

impl GitGraphIdRng {
    fn new(seed: u64) -> Self {
        let seed = if seed == 0 {
            0xA5A5_A5A5_5A5A_5A5A
        } else {
            seed
        };
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        // xorshift64*
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        (x >> 32) as u32
    }

    fn next_hex(&mut self, len: usize) -> String {
        let mut out = String::with_capacity(len);
        for _ in 0..len {
            let val = (self.next_u32() & 0xF) as u8;
            out.push(std::char::from_digit(val as u32, 16).unwrap_or('0'));
        }
        out
    }
}

fn extract_gitgraph_attr(line: &str, key: &str) -> Option<String> {
    let needle = format!("{}:", key);
    let idx = line.find(&needle)?;
    let mut rest = line[idx + needle.len()..].trim_start();
    if rest.is_empty() {
        return None;
    }
    let first = rest.chars().next()?;
    if first == '"' || first == '\'' {
        rest = &rest[1..];
        if let Some(end) = rest.find(first) {
            return Some(rest[..end].to_string());
        }
        return Some(rest.to_string());
    }
    let end = rest
        .find(|ch: char| ch.is_whitespace() || ch == ',')
        .unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

fn parse_c4_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::C4;
    graph.direction = Direction::LeftRight;
    graph.c4 = crate::ir::C4Data::default();
    graph.c4.boundaries.push(crate::ir::C4Boundary {
        id: "global".to_string(),
        label: "global".to_string(),
        boundary_type: "global".to_string(),
        descr: None,
        sprite: None,
        tags: None,
        link: None,
        parent_boundary: String::new(),
        bg_color: None,
        border_color: None,
        font_color: None,
    });
    let (lines, init_config) = preprocess_input(input)?;
    let mut boundary_stack: Vec<String> = vec!["global".to_string()];

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("c4") {
            graph.c4.c4_type = Some(line.trim().to_string());
            continue;
        }
        if line == "}" || lower == "end" {
            if boundary_stack.len() > 1 {
                boundary_stack.pop();
            }
            continue;
        }

        if let Some(brace_idx) = line.find('{') {
            let before = line[..brace_idx].trim();
            let after = line[brace_idx + 1..].trim();
            if !before.is_empty() {
                process_c4_line(before, &mut graph.c4, &mut boundary_stack);
            }
            if !after.is_empty() {
                let closes = after.ends_with('}');
                let after_trimmed = after.trim_end_matches('}').trim();
                if !after_trimmed.is_empty() {
                    process_c4_line(after_trimmed, &mut graph.c4, &mut boundary_stack);
                }
                if closes {
                    if boundary_stack.len() > 1 {
                        boundary_stack.pop();
                    }
                }
            }
            continue;
        }

        process_c4_line(line, &mut graph.c4, &mut boundary_stack);
    }

    Ok(ParseOutput { graph, init_config })
}

fn process_c4_line(line: &str, c4: &mut crate::ir::C4Data, boundary_stack: &mut Vec<String>) {
    if let Some((func, args)) = parse_function_call(line) {
        let func_lower = func.to_ascii_lowercase();
        let (positional, kv) = parse_c4_args(&args);
        if is_c4_boundary(&func_lower) {
            let id = positional
                .get(0)
                .cloned()
                .unwrap_or_else(|| format!("boundary_{}", c4.boundaries.len()));
            let label = positional.get(1).cloned().unwrap_or_else(|| id.clone());
            let mut boundary_type = positional.get(2).cloned();
            if let Some(value) = kv.get("type") {
                boundary_type = Some(value.clone());
            }
            let boundary_type =
                boundary_type.unwrap_or_else(|| c4_boundary_default_type(&func_lower));
            let descr = kv.get("descr").or_else(|| kv.get("description")).cloned();
            let sprite = kv.get("sprite").cloned();
            let tags = kv.get("tags").cloned();
            let link = kv.get("link").cloned();
            let parent_boundary = boundary_stack.last().cloned().unwrap_or_default();
            c4.boundaries.push(crate::ir::C4Boundary {
                id: id.clone(),
                label,
                boundary_type,
                descr,
                sprite,
                tags,
                link,
                parent_boundary,
                bg_color: None,
                border_color: None,
                font_color: None,
            });
            boundary_stack.push(id);
            return;
        }
        if let Some(rel_kind) = c4_rel_kind_for(&func_lower) {
            let mut rel_args = positional;
            if func_lower.starts_with("relindex") && rel_args.len() > 1 {
                rel_args.remove(0);
            }
            if rel_args.len() >= 3 {
                let from = rel_args[0].clone();
                let to = rel_args[1].clone();
                let label = rel_args[2].clone();
                let techn = rel_args
                    .get(3)
                    .cloned()
                    .or_else(|| kv.get("techn").cloned());
                let descr = rel_args
                    .get(4)
                    .cloned()
                    .or_else(|| kv.get("descr").cloned());
                let sprite = rel_args
                    .get(5)
                    .cloned()
                    .or_else(|| kv.get("sprite").cloned());
                let tags = rel_args.get(6).cloned().or_else(|| kv.get("tags").cloned());
                let link = rel_args.get(7).cloned().or_else(|| kv.get("link").cloned());
                c4.rels.push(crate::ir::C4Rel {
                    kind: rel_kind,
                    from,
                    to,
                    label,
                    techn,
                    descr,
                    sprite,
                    tags,
                    link,
                    offset_x: 0.0,
                    offset_y: 0.0,
                    line_color: None,
                    text_color: None,
                });
            }
            return;
        }

        if func_lower == "updateelementstyle"
            || func_lower == "update_el_style"
            || func_lower == "updateelstyle"
        {
            let element = positional
                .get(0)
                .cloned()
                .or_else(|| get_c4_kv(&kv, "element"));
            if let Some(element) = element {
                let bg_color = get_c4_kv(&kv, "bgColor").or_else(|| positional.get(1).cloned());
                let font_color = get_c4_kv(&kv, "fontColor").or_else(|| positional.get(2).cloned());
                let border_color =
                    get_c4_kv(&kv, "borderColor").or_else(|| positional.get(3).cloned());
                let sprite = get_c4_kv(&kv, "sprite").or_else(|| positional.get(6).cloned());
                let techn = get_c4_kv(&kv, "techn").or_else(|| positional.get(7).cloned());

                if let Some(shape) = c4.shapes.iter_mut().find(|s| s.id == element) {
                    if let Some(val) = bg_color {
                        shape.bg_color = Some(val);
                    }
                    if let Some(val) = font_color {
                        shape.font_color = Some(val);
                    }
                    if let Some(val) = border_color {
                        shape.border_color = Some(val);
                    }
                    if let Some(val) = sprite {
                        shape.sprite = Some(val);
                    }
                    if let Some(val) = techn {
                        shape.techn = Some(val);
                    }
                } else if let Some(boundary) = c4.boundaries.iter_mut().find(|b| b.id == element) {
                    if let Some(val) = bg_color {
                        boundary.bg_color = Some(val);
                    }
                    if let Some(val) = font_color {
                        boundary.font_color = Some(val);
                    }
                    if let Some(val) = border_color {
                        boundary.border_color = Some(val);
                    }
                    if let Some(val) = sprite {
                        boundary.sprite = Some(val);
                    }
                }
            }
            return;
        }

        if func_lower == "updaterelstyle" || func_lower == "update_rel_style" {
            let from = positional
                .get(0)
                .cloned()
                .or_else(|| get_c4_kv(&kv, "from"));
            let to = positional.get(1).cloned().or_else(|| get_c4_kv(&kv, "to"));
            if let (Some(from), Some(to)) = (from, to) {
                if let Some(rel) = c4.rels.iter_mut().find(|r| r.from == from && r.to == to) {
                    let text_color =
                        get_c4_kv(&kv, "textColor").or_else(|| positional.get(2).cloned());
                    let line_color =
                        get_c4_kv(&kv, "lineColor").or_else(|| positional.get(3).cloned());
                    let offset_x = get_c4_kv(&kv, "offsetX").or_else(|| positional.get(4).cloned());
                    let offset_y = get_c4_kv(&kv, "offsetY").or_else(|| positional.get(5).cloned());
                    if let Some(val) = text_color {
                        rel.text_color = Some(val);
                    }
                    if let Some(val) = line_color {
                        rel.line_color = Some(val);
                    }
                    if let Some(val) = offset_x {
                        if let Ok(num) = val.trim().parse::<f32>() {
                            rel.offset_x = num;
                        }
                    }
                    if let Some(val) = offset_y {
                        if let Ok(num) = val.trim().parse::<f32>() {
                            rel.offset_y = num;
                        }
                    }
                }
            }
            return;
        }

        if func_lower == "updatelayoutconfig" || func_lower == "update_layout_config" {
            let shape_in_row =
                get_c4_kv(&kv, "c4ShapeInRow").or_else(|| positional.get(0).cloned());
            let boundary_in_row =
                get_c4_kv(&kv, "c4BoundaryInRow").or_else(|| positional.get(1).cloned());
            if let Some(val) = shape_in_row {
                if let Ok(num) = val.trim().parse::<usize>() {
                    if num >= 1 {
                        c4.c4_shape_in_row_override = Some(num);
                    }
                }
            }
            if let Some(val) = boundary_in_row {
                if let Ok(num) = val.trim().parse::<usize>() {
                    if num >= 1 {
                        c4.c4_boundary_in_row_override = Some(num);
                    }
                }
            }
            return;
        }

        if let Some(kind) = c4_shape_kind_for(&func_lower) {
            if let Some(id) = positional.get(0).cloned() {
                let label = positional.get(1).cloned().unwrap_or_else(|| id.clone());
                let mut type_label: Option<String> = None;
                let mut techn: Option<String> = None;
                let mut descr: Option<String> = None;
                let mut sprite: Option<String> = None;
                let mut tags: Option<String> = None;
                let mut link: Option<String> = None;
                if let Some(value) = kv.get("type") {
                    type_label = Some(value.clone());
                }
                if let Some(value) = kv.get("techn").or_else(|| kv.get("technology")) {
                    techn = Some(value.clone());
                }
                if let Some(value) = kv.get("descr").or_else(|| kv.get("description")) {
                    descr = Some(value.clone());
                }
                if let Some(value) = kv.get("sprite") {
                    sprite = Some(value.clone());
                }
                if let Some(value) = kv.get("tags") {
                    tags = Some(value.clone());
                }
                if let Some(value) = kv.get("link") {
                    link = Some(value.clone());
                }
                if kind_uses_techn(kind) {
                    if techn.is_none() {
                        techn = positional.get(2).cloned();
                    }
                    if descr.is_none() {
                        descr = positional.get(3).cloned();
                    }
                    if sprite.is_none() {
                        sprite = positional.get(4).cloned();
                    }
                    if tags.is_none() {
                        tags = positional.get(5).cloned();
                    }
                    if link.is_none() {
                        link = positional.get(6).cloned();
                    }
                } else {
                    if type_label.is_none() {
                        type_label = positional.get(2).cloned();
                    }
                    if descr.is_none() {
                        descr = positional.get(3).cloned();
                    }
                    if sprite.is_none() {
                        sprite = positional.get(4).cloned();
                    }
                    if tags.is_none() {
                        tags = positional.get(5).cloned();
                    }
                    if link.is_none() {
                        link = positional.get(6).cloned();
                    }
                }
                let parent_boundary = boundary_stack.last().cloned().unwrap_or_default();
                c4.shapes.push(crate::ir::C4Shape {
                    id,
                    label,
                    type_label,
                    techn,
                    descr,
                    sprite,
                    tags,
                    link,
                    parent_boundary,
                    kind,
                    bg_color: None,
                    border_color: None,
                    font_color: None,
                });
            }
        }
    }
}

fn parse_function_call(line: &str) -> Option<(String, Vec<String>)> {
    let trimmed = line.trim();
    let open = trimmed.find('(')?;
    let close = trimmed.rfind(')')?;
    if close <= open {
        return None;
    }
    let func = trimmed[..open].trim();
    let args_str = &trimmed[open + 1..close];
    let args = split_args(args_str)
        .into_iter()
        .map(|arg| strip_quotes(arg.trim()))
        .collect();
    if func.is_empty() {
        None
    } else {
        Some((func.to_string(), args))
    }
}

fn split_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for ch in input.chars() {
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            current.push(ch);
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            current.push(ch);
            continue;
        }
        if ch == ',' {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                args.push(trimmed.to_string());
            }
            current.clear();
            continue;
        }
        current.push(ch);
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        args.push(trimmed.to_string());
    }
    args
}

fn parse_c4_args(args: &[String]) -> (Vec<String>, std::collections::HashMap<String, String>) {
    let mut positional = Vec::new();
    let mut kv = std::collections::HashMap::new();
    for arg in args {
        let trimmed = arg.trim();
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim().trim_start_matches('$');
            let value = value.trim();
            if !key.is_empty() {
                kv.insert(key.to_string(), value.to_string());
                continue;
            }
        }
        if !trimmed.is_empty() {
            positional.push(trimmed.to_string());
        }
    }
    (positional, kv)
}

fn normalize_c4_key(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    for ch in key.chars() {
        if ch == '_' || ch == '-' {
            continue;
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}

fn get_c4_kv(kv: &std::collections::HashMap<String, String>, key: &str) -> Option<String> {
    let target = normalize_c4_key(key);
    kv.iter()
        .find(|(k, _)| normalize_c4_key(k) == target)
        .map(|(_, v)| v.clone())
}

fn is_c4_boundary(func_lower: &str) -> bool {
    func_lower.contains("boundary")
        || func_lower.starts_with("deployment_node")
        || func_lower == "node"
        || func_lower == "node_l"
        || func_lower == "node_r"
}

fn c4_boundary_default_type(func_lower: &str) -> String {
    if func_lower.contains("enterprise") {
        "enterprise".to_string()
    } else if func_lower.contains("container") {
        "container".to_string()
    } else if func_lower.contains("system") {
        "system".to_string()
    } else if func_lower.contains("node") {
        "node".to_string()
    } else {
        "system".to_string()
    }
}

fn c4_shape_kind_for(func_lower: &str) -> Option<crate::ir::C4ShapeKind> {
    let f = func_lower.replace('-', "_");
    let is_ext = f.contains("ext");
    if f.contains("person") {
        return Some(if is_ext {
            crate::ir::C4ShapeKind::ExternalPerson
        } else {
            crate::ir::C4ShapeKind::Person
        });
    }
    if f.contains("system") {
        let is_db = f.contains("db");
        let is_queue = f.contains("queue");
        return Some(match (is_ext, is_db, is_queue) {
            (true, true, _) => crate::ir::C4ShapeKind::ExternalSystemDb,
            (true, _, true) => crate::ir::C4ShapeKind::ExternalSystemQueue,
            (true, _, _) => crate::ir::C4ShapeKind::ExternalSystem,
            (false, true, _) => crate::ir::C4ShapeKind::SystemDb,
            (false, _, true) => crate::ir::C4ShapeKind::SystemQueue,
            (false, _, _) => crate::ir::C4ShapeKind::System,
        });
    }
    if f.contains("container") {
        let is_db = f.contains("db");
        let is_queue = f.contains("queue");
        return Some(match (is_ext, is_db, is_queue) {
            (true, true, _) => crate::ir::C4ShapeKind::ExternalContainerDb,
            (true, _, true) => crate::ir::C4ShapeKind::ExternalContainerQueue,
            (true, _, _) => crate::ir::C4ShapeKind::ExternalContainer,
            (false, true, _) => crate::ir::C4ShapeKind::ContainerDb,
            (false, _, true) => crate::ir::C4ShapeKind::ContainerQueue,
            (false, _, _) => crate::ir::C4ShapeKind::Container,
        });
    }
    if f.contains("component") {
        let is_db = f.contains("db");
        let is_queue = f.contains("queue");
        return Some(match (is_ext, is_db, is_queue) {
            (true, true, _) => crate::ir::C4ShapeKind::ExternalComponentDb,
            (true, _, true) => crate::ir::C4ShapeKind::ExternalComponentQueue,
            (true, _, _) => crate::ir::C4ShapeKind::ExternalComponent,
            (false, true, _) => crate::ir::C4ShapeKind::ComponentDb,
            (false, _, true) => crate::ir::C4ShapeKind::ComponentQueue,
            (false, _, _) => crate::ir::C4ShapeKind::Component,
        });
    }
    None
}

fn kind_uses_techn(kind: crate::ir::C4ShapeKind) -> bool {
    matches!(
        kind,
        crate::ir::C4ShapeKind::Container
            | crate::ir::C4ShapeKind::ContainerDb
            | crate::ir::C4ShapeKind::ContainerQueue
            | crate::ir::C4ShapeKind::ExternalContainer
            | crate::ir::C4ShapeKind::ExternalContainerDb
            | crate::ir::C4ShapeKind::ExternalContainerQueue
            | crate::ir::C4ShapeKind::Component
            | crate::ir::C4ShapeKind::ComponentDb
            | crate::ir::C4ShapeKind::ComponentQueue
            | crate::ir::C4ShapeKind::ExternalComponent
            | crate::ir::C4ShapeKind::ExternalComponentDb
            | crate::ir::C4ShapeKind::ExternalComponentQueue
    )
}

fn c4_rel_kind_for(func_lower: &str) -> Option<crate::ir::C4RelKind> {
    let f = func_lower.replace('-', "_");
    if f.starts_with("birel") {
        return Some(crate::ir::C4RelKind::BiRel);
    }
    if f.starts_with("rel_u") || f.starts_with("rel_up") {
        return Some(crate::ir::C4RelKind::RelUp);
    }
    if f.starts_with("rel_d") || f.starts_with("rel_down") {
        return Some(crate::ir::C4RelKind::RelDown);
    }
    if f.starts_with("rel_l") || f.starts_with("rel_left") {
        return Some(crate::ir::C4RelKind::RelLeft);
    }
    if f.starts_with("rel_r") || f.starts_with("rel_right") {
        return Some(crate::ir::C4RelKind::RelRight);
    }
    if f.starts_with("rel_b") || f.starts_with("rel_back") {
        return Some(crate::ir::C4RelKind::RelBack);
    }
    if f.starts_with("rel") || f.starts_with("relindex") {
        return Some(crate::ir::C4RelKind::Rel);
    }
    None
}

fn parse_sankey_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Sankey;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("sankey") {
            continue;
        }
        let parts = split_args(line);
        if parts.len() < 3 {
            continue;
        }
        let from = strip_quotes(parts[0].trim());
        let to = strip_quotes(parts[1].trim());
        let value = parts[2].trim();
        if from.is_empty() || to.is_empty() {
            continue;
        }
        graph.ensure_node(&from, None, Some(crate::ir::NodeShape::Rectangle));
        graph.ensure_node(&to, None, Some(crate::ir::NodeShape::Rectangle));
        let label = if value.is_empty() {
            None
        } else {
            Some(value.to_string())
        };
        graph.edges.push(crate::ir::Edge {
            from,
            to,
            label,
            start_label: None,
            end_label: None,
            directed: true,
            arrow_start: false,
            arrow_end: true,
            arrow_start_kind: None,
            arrow_end_kind: None,
            start_decoration: None,
            end_decoration: None,
            style: crate::ir::EdgeStyle::Solid,
        });
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_quadrant_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Quadrant;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("quadrantchart") {
            continue;
        }
        if lower.starts_with("title") {
            let title = line.get(5..).unwrap_or("").trim();
            if !title.is_empty() {
                graph.quadrant.title = Some(title.to_string());
            }
            continue;
        }
        if lower.starts_with("x-axis") {
            // Format: x-axis Low Reach --> High Reach
            let rest = line.get(6..).unwrap_or("").trim();
            if let Some((left, right)) = rest.split_once("-->") {
                graph.quadrant.x_axis_left = Some(left.trim().to_string());
                graph.quadrant.x_axis_right = Some(right.trim().to_string());
            }
            continue;
        }
        if lower.starts_with("y-axis") {
            // Format: y-axis Low Engagement --> High Engagement
            let rest = line.get(6..).unwrap_or("").trim();
            if let Some((bottom, top)) = rest.split_once("-->") {
                graph.quadrant.y_axis_bottom = Some(bottom.trim().to_string());
                graph.quadrant.y_axis_top = Some(top.trim().to_string());
            }
            continue;
        }
        if lower.starts_with("quadrant-") {
            // Format: quadrant-1 We should expand
            if let Some(rest) = line.get(10..) {
                let label = rest.trim().to_string();
                if lower.starts_with("quadrant-1") {
                    graph.quadrant.quadrant_labels[0] = Some(label);
                } else if lower.starts_with("quadrant-2") {
                    graph.quadrant.quadrant_labels[1] = Some(label);
                } else if lower.starts_with("quadrant-3") {
                    graph.quadrant.quadrant_labels[2] = Some(label);
                } else if lower.starts_with("quadrant-4") {
                    graph.quadrant.quadrant_labels[3] = Some(label);
                }
            }
            continue;
        }
        // Parse data points: Campaign A: [0.3, 0.6]
        if let Some((label, x, y)) = parse_quadrant_point_coords(line) {
            let node_id = format!("quadrant_{}", graph.nodes.len());
            graph.ensure_node(
                &node_id,
                Some(label.clone()),
                Some(crate::ir::NodeShape::Rectangle),
            );
            graph
                .quadrant
                .points
                .push(crate::ir::QuadrantPoint { label, x, y });
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_quadrant_point_coords(line: &str) -> Option<(String, f32, f32)> {
    let (left, right) = line.split_once(':')?;
    let label = left.trim().to_string();
    if label.is_empty() {
        return None;
    }
    let coords = right
        .trim()
        .trim_matches(|ch| ch == '[' || ch == ']' || ch == '(' || ch == ')');
    let mut parts = coords.split(',').map(|p| p.trim());
    let x: f32 = parts.next()?.parse().ok()?;
    let y: f32 = parts.next()?.parse().ok()?;
    Some((label, x, y))
}

fn parse_zenuml_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::ZenUML;
    let (lines, init_config) = preprocess_input(input)?;
    let mut order: Vec<String> = Vec::new();
    let labels: HashMap<String, String> = HashMap::new();

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("zenuml") || lower.starts_with("title") {
            continue;
        }
        if let Some((from, to, label, style)) = parse_zenuml_message_line(line) {
            ensure_sequence_node(&mut graph, &labels, &from, None);
            ensure_sequence_node(&mut graph, &labels, &to, None);
            if !order.contains(&from) {
                order.push(from.clone());
            }
            if !order.contains(&to) {
                order.push(to.clone());
            }
            graph.edges.push(crate::ir::Edge {
                from,
                to,
                label,
                start_label: None,
                end_label: None,
                directed: true,
                arrow_start: false,
                arrow_end: true,
                arrow_start_kind: None,
                arrow_end_kind: None,
                start_decoration: None,
                end_decoration: None,
                style,
            });
        }
    }

    graph.sequence_participants = order;

    Ok(ParseOutput { graph, init_config })
}

fn parse_zenuml_message_line(
    line: &str,
) -> Option<(String, String, Option<String>, crate::ir::EdgeStyle)> {
    let arrows = ["-->>", "->>", "-->", "->", "==>", "=>"];
    let mut found = None;
    for arrow in &arrows {
        if let Some(idx) = line.find(arrow) {
            found = Some((idx, *arrow));
            break;
        }
    }
    let (idx, arrow) = found?;
    let left = line[..idx].trim();
    let rest = line[idx + arrow.len()..].trim();
    if left.is_empty() || rest.is_empty() {
        return None;
    }
    let (right, label) = if let Some((r, l)) = rest.split_once(':') {
        let lbl = l.trim();
        let lbl = if lbl.is_empty() {
            None
        } else {
            Some(lbl.to_string())
        };
        (r.trim(), lbl)
    } else {
        (rest, None)
    };
    if right.is_empty() {
        return None;
    }
    let style = if arrow.contains("--") {
        crate::ir::EdgeStyle::Dotted
    } else {
        crate::ir::EdgeStyle::Solid
    };
    Some((left.to_string(), right.to_string(), label, style))
}

fn parse_block_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Block;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("block") {
            continue;
        }
        if let Some((left, label, right, edge_meta)) = parse_edge_line(line) {
            let sources: Vec<&str> = left
                .split('&')
                .map(|part| part.trim())
                .filter(|part| !part.is_empty())
                .collect();
            let targets: Vec<&str> = right
                .split('&')
                .map(|part| part.trim())
                .filter(|part| !part.is_empty())
                .collect();

            for source in &sources {
                let (source_id, source_label, source_shape, source_classes) =
                    parse_node_token(source);
                graph.ensure_node(&source_id, source_label, source_shape);
                if !source_classes.is_empty() {
                    apply_node_classes(&mut graph, &source_id, &source_classes);
                }
            }
            for target in &targets {
                let (target_id, target_label, target_shape, target_classes) =
                    parse_node_token(target);
                graph.ensure_node(&target_id, target_label, target_shape);
                if !target_classes.is_empty() {
                    apply_node_classes(&mut graph, &target_id, &target_classes);
                }
            }

            for source in &sources {
                let (source_id, _, _, _) = parse_node_token(source);
                for target in &targets {
                    let (target_id, _, _, _) = parse_node_token(target);
                    graph.edges.push(crate::ir::Edge {
                        from: source_id.clone(),
                        to: target_id.clone(),
                        label: label.clone(),
                        start_label: None,
                        end_label: None,
                        directed: edge_meta.directed,
                        arrow_start: edge_meta.arrow_start,
                        arrow_end: edge_meta.arrow_end,
                        arrow_start_kind: edge_meta.arrow_start_kind,
                        arrow_end_kind: edge_meta.arrow_end_kind,
                        start_decoration: edge_meta.start_decoration,
                        end_decoration: edge_meta.end_decoration,
                        style: edge_meta.style,
                    });
                }
            }
            continue;
        }

        let (id, label, shape, classes) = parse_node_token(line);
        graph.ensure_node(&id, label, shape);
        if !classes.is_empty() {
            apply_node_classes(&mut graph, &id, &classes);
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_packet_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Packet;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;
    let mut last_node: Option<String> = None;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("packet") || lower.starts_with("title") {
            continue;
        }
        if let Some((range, label)) = line.split_once(':') {
            let range = range.trim();
            let label = strip_quotes(label.trim());
            if range.is_empty() {
                continue;
            }
            let node_id = format!("packet_{}", graph.nodes.len());
            let node_label = if label.is_empty() {
                range.to_string()
            } else {
                format!("{}\n{}", range, label)
            };
            graph.ensure_node(
                &node_id,
                Some(node_label),
                Some(crate::ir::NodeShape::Rectangle),
            );
            if let Some(prev) = last_node.take() {
                graph.edges.push(crate::ir::Edge {
                    from: prev,
                    to: node_id.clone(),
                    label: None,
                    start_label: None,
                    end_label: None,
                    directed: false,
                    arrow_start: false,
                    arrow_end: false,
                    arrow_start_kind: None,
                    arrow_end_kind: None,
                    start_decoration: None,
                    end_decoration: None,
                    style: crate::ir::EdgeStyle::Solid,
                });
            }
            last_node = Some(node_id);
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_kanban_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Kanban;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input_keep_indent(input)?;
    let mut current_section: Option<usize> = None;
    let mut base_indent: Option<usize> = None;

    for raw_line in lines {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("kanban") {
            continue;
        }
        let indent = count_indent(&raw_line);
        let base = *base_indent.get_or_insert(indent);
        if indent <= base {
            let (id, label, _shape, _classes) = parse_node_token(trimmed);
            let col_label = label.unwrap_or_else(|| id.clone());
            graph.subgraphs.push(Subgraph {
                id: Some(id),
                label: col_label,
                nodes: Vec::new(),
                direction: None,
            });
            current_section = Some(graph.subgraphs.len() - 1);
            continue;
        }

        let (task_part, meta) = if let Some((left, right)) = trimmed.split_once("@{") {
            let meta = right.trim_end_matches('}').trim();
            (left.trim(), Some(meta.to_string()))
        } else {
            (trimmed, None)
        };
        let (mut id, label, _shape, _classes) = parse_node_token(task_part);
        if graph.nodes.contains_key(&id) {
            id = format!("{}_{}", id, graph.nodes.len());
        }
        let mut node_label = label.unwrap_or_else(|| id.clone());
        if let Some(meta) = meta {
            if !meta.is_empty() {
                node_label.push_str(&format!("\n{}", meta));
            }
        }
        graph.ensure_node(&id, Some(node_label), Some(crate::ir::NodeShape::Rectangle));
        if let Some(idx) = current_section {
            if let Some(subgraph) = graph.subgraphs.get_mut(idx) {
                subgraph.nodes.push(id);
            }
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_architecture_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Architecture;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;
    let mut groups: HashMap<String, usize> = HashMap::new();

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("architecture") {
            continue;
        }
        if lower.starts_with("group ") || lower.starts_with("service ") {
            if let Some((kind, id, label, parent)) = parse_architecture_node(line) {
                if kind == "group" {
                    graph.subgraphs.push(Subgraph {
                        id: Some(id.clone()),
                        label: label.clone(),
                        nodes: Vec::new(),
                        direction: None,
                    });
                    groups.insert(id, graph.subgraphs.len() - 1);
                } else {
                    graph.ensure_node(&id, Some(label), Some(crate::ir::NodeShape::Rectangle));
                    if let Some(parent_id) = parent {
                        if let Some(idx) = groups.get(&parent_id).copied() {
                            if let Some(subgraph) = graph.subgraphs.get_mut(idx) {
                                subgraph.nodes.push(id.clone());
                            }
                        }
                    }
                }
            }
            continue;
        }
        if let Some((from, to)) = parse_architecture_edge(line) {
            graph.ensure_node(&from, None, Some(crate::ir::NodeShape::Rectangle));
            graph.ensure_node(&to, None, Some(crate::ir::NodeShape::Rectangle));
            graph.edges.push(crate::ir::Edge {
                from,
                to,
                label: None,
                start_label: None,
                end_label: None,
                directed: true,
                arrow_start: false,
                arrow_end: true,
                arrow_start_kind: None,
                arrow_end_kind: None,
                start_decoration: None,
                end_decoration: None,
                style: crate::ir::EdgeStyle::Solid,
            });
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_architecture_node(line: &str) -> Option<(String, String, String, Option<String>)> {
    let mut parts = line.splitn(2, ' ');
    let kind = parts.next()?.trim().to_ascii_lowercase();
    let rest = parts.next()?.trim();
    let (node_part, parent) = if let Some((left, right)) = rest.split_once(" in ") {
        (left.trim(), Some(right.trim().to_string()))
    } else {
        (rest, None)
    };
    let label = if let Some(start) = node_part.find('[') {
        if let Some(end) = node_part.rfind(']') {
            strip_quotes(node_part[start + 1..end].trim())
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    let id_part = node_part.split('[').next().unwrap_or(node_part).trim();
    let id = id_part
        .split('(')
        .next()
        .unwrap_or(id_part)
        .trim()
        .to_string();
    if id.is_empty() {
        return None;
    }
    let label = if label.is_empty() { id.clone() } else { label };
    Some((kind, id, label, parent))
}

fn parse_architecture_edge(line: &str) -> Option<(String, String)> {
    let arrows = ["-->", "--", "->"];
    for arrow in &arrows {
        if let Some(idx) = line.find(arrow) {
            let left = line[..idx].trim();
            let right = line[idx + arrow.len()..].trim();
            let from = strip_arch_port(left);
            let to = strip_arch_port(right);
            if from.is_empty() || to.is_empty() {
                return None;
            }
            return Some((from.to_string(), to.to_string()));
        }
    }
    None
}

fn strip_arch_port(token: &str) -> &str {
    token.split(':').last().unwrap_or(token).trim()
}

fn parse_radar_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Radar;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;
    let mut axes: Vec<String> = Vec::new();

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("radar") || lower.starts_with("title") {
            continue;
        }
        if lower.starts_with("axis") {
            let rest = line.get(4..).unwrap_or("").trim();
            axes = split_args(rest)
                .into_iter()
                .map(|value| strip_quotes(value.trim()))
                .filter(|value| !value.is_empty())
                .collect();
            continue;
        }
        if lower.starts_with("curve") {
            if let Some((name, values)) = parse_radar_curve(line) {
                let node_id = format!("radar_{}", graph.nodes.len());
                let mut label_lines = Vec::new();
                label_lines.push(name);
                if !values.is_empty() {
                    for (idx, value) in values.iter().enumerate() {
                        if let Some(axis) = axes.get(idx) {
                            label_lines.push(format!("{}: {}", axis, value));
                        } else {
                            label_lines.push(value.to_string());
                        }
                    }
                }
                graph.ensure_node(
                    &node_id,
                    Some(label_lines.join("\n")),
                    Some(crate::ir::NodeShape::Circle),
                );
            }
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_radar_curve(line: &str) -> Option<(String, Vec<String>)> {
    let rest = line.get(5..).unwrap_or("").trim();
    let (name_part, values_part) = rest.split_once('{')?;
    let name = strip_quotes(name_part.trim());
    let values_raw = values_part.split_once('}')?.0;
    let values = split_args(values_raw)
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if name.is_empty() {
        return None;
    }
    Some((name, values))
}

fn parse_treemap_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Treemap;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input_keep_indent(input)?;
    let mut stack: Vec<String> = Vec::new();
    let mut base_indent: Option<usize> = None;

    for raw_line in lines {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("treemap") {
            continue;
        }
        let indent = count_indent(&raw_line);
        let base = *base_indent.get_or_insert(indent);
        let rel_indent = indent.saturating_sub(base);
        let mut level = rel_indent / 2;
        if level > stack.len() {
            level = stack.len();
        }

        let (label, value) = parse_treemap_item(trimmed);
        let node_id = format!("treemap_{}", graph.nodes.len());
        let node_label = if let Some(value) = value {
            format!("{}\n{}", label, value)
        } else {
            label.clone()
        };
        graph.ensure_node(
            &node_id,
            Some(node_label),
            Some(crate::ir::NodeShape::Rectangle),
        );

        if level > 0 {
            if stack.len() > level {
                stack.truncate(level);
            }
            if let Some(parent) = stack.last().cloned() {
                graph.edges.push(crate::ir::Edge {
                    from: parent,
                    to: node_id.clone(),
                    label: None,
                    start_label: None,
                    end_label: None,
                    directed: false,
                    arrow_start: false,
                    arrow_end: false,
                    arrow_start_kind: None,
                    arrow_end_kind: None,
                    start_decoration: None,
                    end_decoration: None,
                    style: crate::ir::EdgeStyle::Solid,
                });
            }
        } else {
            stack.clear();
        }
        stack.push(node_id);
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_treemap_item(line: &str) -> (String, Option<String>) {
    if let Some((left, right)) = line.split_once(':') {
        let label = strip_quotes(left.trim());
        let value = right.trim();
        let value = if value.is_empty() {
            None
        } else {
            Some(value.to_string())
        };
        return (
            if label.is_empty() {
                left.trim().to_string()
            } else {
                label
            },
            value,
        );
    }
    (strip_quotes(line.trim()), None)
}

fn parse_xy_chart_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::XYChart;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("xychart") {
            continue;
        }
        if lower.starts_with("title") {
            let rest = line.get(5..).unwrap_or("").trim();
            if !rest.is_empty() {
                graph.xychart.title = Some(strip_quotes(rest));
            }
            continue;
        }
        if lower.starts_with("x-axis") {
            let rest = line.get(6..).unwrap_or("").trim();
            // Check if it's a label followed by categories or just categories
            if let Some(bracket_idx) = rest.find('[') {
                let label_part = rest[..bracket_idx].trim();
                if !label_part.is_empty() {
                    graph.xychart.x_axis_label = Some(strip_quotes(label_part));
                }
                graph.xychart.x_axis_categories = parse_xy_axis_categories(&rest[bracket_idx..]);
            } else {
                // Just categories without brackets or a label
                graph.xychart.x_axis_categories = parse_xy_axis_categories(rest);
            }
            continue;
        }
        if lower.starts_with("y-axis") {
            let rest = line.get(6..).unwrap_or("").trim();
            if !rest.is_empty() {
                // Parse y-axis which can have label and/or range
                // Format: y-axis "Label" min --> max  OR  y-axis min --> max  OR  y-axis "Label"
                let rest_lower = rest.to_ascii_lowercase();
                if let Some(arrow_idx) = rest_lower.find("-->") {
                    // Has range
                    let before_arrow = rest[..arrow_idx].trim();
                    let after_arrow = rest[arrow_idx + 3..].trim();

                    // Parse min value (might have label before it)
                    let min_str = before_arrow.split_whitespace().last().unwrap_or("0");
                    if let Ok(min) = min_str.parse::<f32>() {
                        graph.xychart.y_axis_min = Some(min);
                    }
                    if let Ok(max) = after_arrow.parse::<f32>() {
                        graph.xychart.y_axis_max = Some(max);
                    }
                    // Check for label before the min value
                    let label_part = before_arrow.trim_end_matches(min_str).trim();
                    if !label_part.is_empty() {
                        graph.xychart.y_axis_label = Some(strip_quotes(label_part));
                    }
                } else {
                    graph.xychart.y_axis_label = Some(strip_quotes(rest));
                }
            }
            continue;
        }
        if let Some((series_kind, label, values)) = parse_xy_series_line_v2(line) {
            graph.xychart.series.push(crate::ir::XYSeries {
                kind: series_kind,
                label,
                values,
            });
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_xy_series_line_v2(
    line: &str,
) -> Option<(crate::ir::XYSeriesKind, Option<String>, Vec<f32>)> {
    let lower = line.to_ascii_lowercase();
    let (kind, rest) = if lower.starts_with("bar") {
        (
            crate::ir::XYSeriesKind::Bar,
            line.get(3..).unwrap_or("").trim(),
        )
    } else if lower.starts_with("line") {
        (
            crate::ir::XYSeriesKind::Line,
            line.get(4..).unwrap_or("").trim(),
        )
    } else {
        return None;
    };

    // Parse optional label and values: [1, 2, 3] or "Label" [1, 2, 3]
    let (label, values_str) = if let Some(bracket_idx) = rest.find('[') {
        let label_part = rest[..bracket_idx].trim();
        let label = if label_part.is_empty() {
            None
        } else {
            Some(strip_quotes(label_part))
        };
        (label, &rest[bracket_idx..])
    } else {
        (None, rest)
    };

    let values: Vec<f32> = values_str
        .trim_matches(|ch| ch == '[' || ch == ']')
        .split(',')
        .filter_map(|s| s.trim().parse::<f32>().ok())
        .collect();

    if values.is_empty() {
        None
    } else {
        Some((kind, label, values))
    }
}

fn parse_xy_axis_categories(rest: &str) -> Vec<String> {
    let trimmed = rest.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let categories = if let Some(open) = trimmed.find('[') {
        if let Some(close) = trimmed.rfind(']') {
            if close > open {
                &trimmed[open + 1..close]
            } else {
                trimmed
            }
        } else {
            trimmed
        }
    } else {
        trimmed
    };

    split_args(categories)
        .into_iter()
        .map(|value| {
            let cleaned = value
                .trim()
                .trim_matches(|ch| ch == '[' || ch == ']')
                .trim();
            strip_quotes(cleaned)
        })
        .filter(|value| !value.is_empty())
        .collect()
}

fn parse_xy_series_line(line: &str) -> Option<(String, Vec<String>)> {
    let mut parts = line.splitn(2, ' ');
    let series = parts.next()?.trim().to_string();
    let rest = parts.next()?.trim();
    let values = rest
        .trim_matches(|ch| ch == '[' || ch == ']')
        .split(',')
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if series.is_empty() {
        None
    } else {
        Some((series, values))
    }
}

fn parse_state_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::State;
    let (lines, init_config) = preprocess_input(input)?;

    let mut labels: HashMap<String, String> = HashMap::new();
    let mut start_counter: usize = 0;
    let mut end_states: HashMap<String, String> = HashMap::new();
    let mut subgraph_stack: Vec<usize> = Vec::new();
    let mut region_counter: usize = 0;

    #[derive(Debug)]
    struct CompositeContext {
        subgraph_idx: usize,
        regions: Vec<Vec<String>>,
        current_region: usize,
        has_separator: bool,
    }

    let mut composite_stack: Vec<CompositeContext> = Vec::new();
    let mut pending: VecDeque<String> = lines.into();

    let record_region_node = |stack: &mut [CompositeContext], node_id: &str| {
        for ctx in stack.iter_mut() {
            if ctx
                .regions
                .iter()
                .any(|region| region.iter().any(|id| id == node_id))
            {
                continue;
            }
            let region = &mut ctx.regions[ctx.current_region];
            region.push(node_id.to_string());
        }
    };

    let finalize_regions =
        |ctx: CompositeContext, graph: &mut Graph, region_counter: &mut usize| {
            if !ctx.has_separator {
                return;
            }
            let mut regions: Vec<Vec<String>> = ctx
                .regions
                .into_iter()
                .filter(|region| !region.is_empty())
                .collect();
            if regions.len() <= 1 {
                return;
            }
            for region_nodes in regions.drain(..) {
                let id = format!("__region_{}__", *region_counter);
                *region_counter += 1;
                graph.subgraphs.push(Subgraph {
                    id: Some(id.clone()),
                    label: String::new(),
                    nodes: region_nodes,
                    direction: None,
                });
                graph.subgraph_styles.insert(
                    id,
                    NodeStyle {
                        fill: Some("none".to_string()),
                        stroke: Some("none".to_string()),
                        text_color: None,
                        stroke_width: Some(0.0),
                        stroke_dasharray: None,
                        line_color: None,
                    },
                );
            }
        };
    while let Some(raw_line) = pending.pop_front() {
        for raw_statement in split_statements(&raw_line) {
            let raw_line = raw_statement.trim();
            if raw_line.is_empty() {
                continue;
            }
            let (line, state_shape, label_override) = parse_state_stereotype(raw_line);
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let lower = line.to_ascii_lowercase();
            if lower.starts_with("statediagram") {
                continue;
            }

            if let Some(direction) = parse_direction_line(line) {
                graph.direction = direction;
                continue;
            }

            if line.starts_with("classDef") {
                parse_class_def(line, &mut graph);
                continue;
            }

            if line.starts_with("class ") {
                parse_class_line(line, &mut graph);
                continue;
            }

            if line.starts_with("style ") {
                parse_style_line(line, &mut graph);
                continue;
            }

            if line == "}" {
                if let Some(ctx) = composite_stack.pop() {
                    if let Some(idx) = subgraph_stack.pop()
                        && idx != ctx.subgraph_idx
                    {
                        subgraph_stack.push(idx);
                    }
                    finalize_regions(ctx, &mut graph, &mut region_counter);
                }
                continue;
            }

            if line == "--" {
                if let Some(ctx) = composite_stack.last_mut() {
                    ctx.has_separator = true;
                    ctx.regions.push(Vec::new());
                    ctx.current_region = ctx.regions.len().saturating_sub(1);
                }
                continue;
            }

            if let Some((id, label, tail)) = parse_state_container_header(line) {
                if let Some(id) = id.clone() {
                    labels.insert(id.clone(), label.clone());
                }
                graph.subgraphs.push(Subgraph {
                    id: id.clone(),
                    label: label.clone(),
                    nodes: Vec::new(),
                    direction: None,
                });
                subgraph_stack.push(graph.subgraphs.len() - 1);
                composite_stack.push(CompositeContext {
                    subgraph_idx: graph.subgraphs.len() - 1,
                    regions: vec![Vec::new()],
                    current_region: 0,
                    has_separator: false,
                });

                if !tail.is_empty() {
                    if let Some(close_idx) = tail.find('}') {
                        let body = tail[..close_idx].trim();
                        let after = tail[close_idx + 1..].trim();
                        if !after.is_empty() {
                            pending.push_front(after.to_string());
                        }
                        pending.push_front("}".to_string());
                        if !body.is_empty() {
                            pending.push_front(body.to_string());
                        }
                    } else {
                        pending.push_front(tail);
                    }
                }
                continue;
            }

            if let Some((id, label, classes)) = parse_state_alias_line(line) {
                let label = label_override.clone().unwrap_or(label);
                labels.insert(id.clone(), label);
                graph.ensure_node(
                    &id,
                    labels.get(&id).cloned(),
                    state_shape.or(Some(crate::ir::NodeShape::RoundRect)),
                );
                apply_node_classes(&mut graph, &id, &classes);
                add_node_to_subgraphs(&mut graph, &subgraph_stack, &id);
                record_region_node(&mut composite_stack, &id);
                continue;
            }

            if let Some((left, meta, right, label)) = parse_state_transition(line) {
                // Determine current scope for start/end state tracking
                let scope = subgraph_stack
                    .last()
                    .and_then(|&idx| graph.subgraphs.get(idx))
                    .and_then(|sub| sub.id.clone())
                    .unwrap_or_else(|| "root".to_string());
                let (left_token, left_classes) = split_inline_classes(&left);
                let (right_token, right_classes) = split_inline_classes(&right);
                let (left_id, left_shape, left_label_override) = normalize_state_token(
                    &left_token,
                    true,
                    &mut start_counter,
                    &mut end_states,
                    &scope,
                );
                let (right_id, right_shape, right_label_override) = normalize_state_token(
                    &right_token,
                    false,
                    &mut start_counter,
                    &mut end_states,
                    &scope,
                );

                let left_label = left_label_override.or_else(|| labels.get(&left_id).cloned());
                let right_label = right_label_override.or_else(|| labels.get(&right_id).cloned());
                let left_shape = if left_shape == crate::ir::NodeShape::RoundRect
                    && graph.nodes.contains_key(&left_id)
                {
                    None
                } else {
                    Some(left_shape)
                };
                let right_shape = if right_shape == crate::ir::NodeShape::RoundRect
                    && graph.nodes.contains_key(&right_id)
                {
                    None
                } else {
                    Some(right_shape)
                };
                graph.ensure_node(&left_id, left_label, left_shape);
                graph.ensure_node(&right_id, right_label, right_shape);
                apply_node_classes(&mut graph, &left_id, &left_classes);
                apply_node_classes(&mut graph, &right_id, &right_classes);
                add_node_to_subgraphs(&mut graph, &subgraph_stack, &left_id);
                add_node_to_subgraphs(&mut graph, &subgraph_stack, &right_id);
                record_region_node(&mut composite_stack, &left_id);
                record_region_node(&mut composite_stack, &right_id);
                graph.edges.push(crate::ir::Edge {
                    from: left_id,
                    to: right_id,
                    label,
                    start_label: None,
                    end_label: None,
                    directed: meta.directed,
                    arrow_start: meta.arrow_start,
                    arrow_end: meta.arrow_end,
                    arrow_start_kind: meta.arrow_start_kind,
                    arrow_end_kind: meta.arrow_end_kind,
                    start_decoration: meta.start_decoration,
                    end_decoration: meta.end_decoration,
                    style: meta.style,
                });
                continue;
            }

            if let Some((id, label, classes)) = parse_state_description_line(line) {
                let label = label_override.clone().unwrap_or(label);
                labels.insert(id.clone(), label);
                graph.ensure_node(
                    &id,
                    labels.get(&id).cloned(),
                    state_shape.or(Some(crate::ir::NodeShape::RoundRect)),
                );
                apply_node_classes(&mut graph, &id, &classes);
                add_node_to_subgraphs(&mut graph, &subgraph_stack, &id);
                record_region_node(&mut composite_stack, &id);
                continue;
            }

            if let Some((position, target_raw, label)) = parse_state_note(line) {
                let (target, classes) = parse_state_id_with_classes(&target_raw);
                if target.is_empty() {
                    continue;
                }
                let shape = if graph.nodes.contains_key(&target) {
                    None
                } else {
                    Some(crate::ir::NodeShape::RoundRect)
                };
                graph.ensure_node(&target, labels.get(&target).cloned(), shape);
                apply_node_classes(&mut graph, &target, &classes);
                graph.state_notes.push(crate::ir::StateNote {
                    position,
                    target: target.clone(),
                    label,
                });
                add_node_to_subgraphs(&mut graph, &subgraph_stack, &target);
                record_region_node(&mut composite_stack, &target);
                continue;
            }

            if let Some((id, classes)) = parse_state_simple(line) {
                if let Some(label) = label_override.clone() {
                    labels.insert(id.clone(), label);
                }
                graph.ensure_node(
                    &id,
                    labels.get(&id).cloned(),
                    state_shape.or(Some(crate::ir::NodeShape::RoundRect)),
                );
                apply_node_classes(&mut graph, &id, &classes);
                add_node_to_subgraphs(&mut graph, &subgraph_stack, &id);
                record_region_node(&mut composite_stack, &id);
                continue;
            }
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_sequence_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Sequence;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;

    let mut labels: HashMap<String, String> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut open_frames: Vec<crate::ir::SequenceFrame> = Vec::new();
    let mut frames: Vec<crate::ir::SequenceFrame> = Vec::new();
    let mut open_boxes: Vec<crate::ir::SequenceBox> = Vec::new();

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("sequencediagram") {
            continue;
        }
        if let Some((id, label, shape)) = parse_sequence_participant(line) {
            if !order.contains(&id) {
                order.push(id.clone());
            }
            if let Some(label) = label.clone() {
                labels.insert(id.clone(), label);
            }
            ensure_sequence_node(&mut graph, &labels, &id, Some(shape));
            if let Some(box_ctx) = open_boxes.last_mut()
                && !box_ctx.participants.contains(&id)
            {
                box_ctx.participants.push(id.clone());
            }
            continue;
        }

        if let Some((color, label)) = parse_sequence_box_line(line) {
            open_boxes.push(crate::ir::SequenceBox {
                label,
                color,
                participants: Vec::new(),
            });
            continue;
        }

        if lower == "alt"
            || lower.starts_with("alt ")
            || lower == "opt"
            || lower.starts_with("opt ")
            || lower == "loop"
            || lower.starts_with("loop ")
            || lower == "par"
            || lower.starts_with("par ")
            || lower == "rect"
            || lower.starts_with("rect ")
            || lower == "critical"
            || lower.starts_with("critical ")
            || lower == "break"
            || lower.starts_with("break ")
        {
            let (kind, offset) = if lower.starts_with("opt") {
                (crate::ir::SequenceFrameKind::Opt, 3)
            } else if lower.starts_with("loop") {
                (crate::ir::SequenceFrameKind::Loop, 4)
            } else if lower.starts_with("par") {
                (crate::ir::SequenceFrameKind::Par, 3)
            } else if lower.starts_with("rect") {
                (crate::ir::SequenceFrameKind::Rect, 4)
            } else if lower.starts_with("critical") {
                (crate::ir::SequenceFrameKind::Critical, 8)
            } else if lower.starts_with("break") {
                (crate::ir::SequenceFrameKind::Break, 5)
            } else {
                (crate::ir::SequenceFrameKind::Alt, 3)
            };
            let label = line.get(offset..).map(str::trim).unwrap_or_default();
            let label = if label.is_empty() {
                None
            } else {
                Some(strip_quotes(label))
            };
            let start_idx = graph.edges.len();
            open_frames.push(crate::ir::SequenceFrame {
                kind,
                sections: vec![crate::ir::SequenceFrameSection {
                    label,
                    start_idx,
                    end_idx: start_idx,
                }],
                start_idx,
                end_idx: start_idx,
            });
            continue;
        }

        if lower == "else" || lower.starts_with("else ") {
            if let Some(frame) = open_frames.last_mut() {
                let split_idx = graph.edges.len();
                if let Some(last) = frame.sections.last_mut() {
                    last.end_idx = split_idx;
                }
                let label = line.get(4..).map(str::trim).unwrap_or_default();
                let label = if label.is_empty() {
                    None
                } else {
                    Some(strip_quotes(label))
                };
                frame.sections.push(crate::ir::SequenceFrameSection {
                    label,
                    start_idx: split_idx,
                    end_idx: split_idx,
                });
            }
            continue;
        }

        if lower == "and" || lower.starts_with("and ") {
            if let Some(frame) = open_frames.last_mut()
                && frame.kind == crate::ir::SequenceFrameKind::Par
            {
                let split_idx = graph.edges.len();
                if let Some(last) = frame.sections.last_mut() {
                    last.end_idx = split_idx;
                }
                let label = line.get(3..).map(str::trim).unwrap_or_default();
                let label = if label.is_empty() {
                    None
                } else {
                    Some(strip_quotes(label))
                };
                frame.sections.push(crate::ir::SequenceFrameSection {
                    label,
                    start_idx: split_idx,
                    end_idx: split_idx,
                });
            }
            continue;
        }

        if lower == "option" || lower.starts_with("option ") {
            if let Some(frame) = open_frames.last_mut()
                && frame.kind == crate::ir::SequenceFrameKind::Critical
            {
                let split_idx = graph.edges.len();
                if let Some(last) = frame.sections.last_mut() {
                    last.end_idx = split_idx;
                }
                let label = line.get(6..).map(str::trim).unwrap_or_default();
                let label = if label.is_empty() {
                    None
                } else {
                    Some(strip_quotes(label))
                };
                frame.sections.push(crate::ir::SequenceFrameSection {
                    label,
                    start_idx: split_idx,
                    end_idx: split_idx,
                });
            }
            continue;
        }

        if lower == "end" {
            if let Some(mut frame) = open_frames.pop() {
                let end_idx = graph.edges.len();
                if let Some(last) = frame.sections.last_mut() {
                    last.end_idx = end_idx;
                }
                frame.end_idx = end_idx;
                frames.push(frame);
            } else if let Some(seq_box) = open_boxes.pop() {
                graph.sequence_boxes.push(seq_box);
            }
            continue;
        }

        if let Some((position, participants, label)) = parse_sequence_note(line) {
            for id in &participants {
                if !order.contains(id) {
                    order.push(id.clone());
                }
                ensure_sequence_node(&mut graph, &labels, id, None);
            }
            graph.sequence_notes.push(crate::ir::SequenceNote {
                position,
                participants,
                label,
                index: graph.edges.len(),
            });
            continue;
        }

        if lower.starts_with("activate ") {
            let id = line[9..].trim();
            if !id.is_empty() {
                let id = strip_quotes(id);
                if !order.contains(&id) {
                    order.push(id.clone());
                }
                ensure_sequence_node(&mut graph, &labels, &id, None);
                graph
                    .sequence_activations
                    .push(crate::ir::SequenceActivation {
                        participant: id,
                        index: graph.edges.len(),
                        kind: crate::ir::SequenceActivationKind::Activate,
                    });
            }
            continue;
        }
        if lower.starts_with("deactivate ") {
            let id = line[11..].trim();
            if !id.is_empty() {
                let id = strip_quotes(id);
                if !order.contains(&id) {
                    order.push(id.clone());
                }
                ensure_sequence_node(&mut graph, &labels, &id, None);
                graph
                    .sequence_activations
                    .push(crate::ir::SequenceActivation {
                        participant: id,
                        index: graph.edges.len(),
                        kind: crate::ir::SequenceActivationKind::Deactivate,
                    });
            }
            continue;
        }
        if lower.starts_with("autonumber") {
            let parts = line.split_whitespace().collect::<Vec<_>>();
            if parts.len() >= 2 {
                let token = parts[1].to_ascii_lowercase();
                if token == "off" || token == "stop" || token == "disable" {
                    graph.sequence_autonumber = None;
                } else if let Ok(start) = parts[1].parse::<usize>() {
                    graph.sequence_autonumber = Some(start);
                } else {
                    graph.sequence_autonumber = Some(1);
                }
            } else {
                graph.sequence_autonumber = Some(1);
            }
            continue;
        }

        if let Some((from, to, label, style, activation)) = parse_sequence_message(line) {
            if !order.contains(&from) {
                order.push(from.clone());
            }
            if !order.contains(&to) {
                order.push(to.clone());
            }
            ensure_sequence_node(&mut graph, &labels, &from, None);
            ensure_sequence_node(&mut graph, &labels, &to, None);
            graph.edges.push(crate::ir::Edge {
                from,
                to,
                label,
                start_label: None,
                end_label: None,
                directed: true,
                arrow_start: false,
                arrow_end: true,
                arrow_start_kind: None,
                arrow_end_kind: None,
                start_decoration: None,
                end_decoration: None,
                style,
            });
            if let Some(kind) = activation
                && let Some(last) = graph.edges.len().checked_sub(1)
            {
                let participant = graph.edges[last].to.clone();
                graph
                    .sequence_activations
                    .push(crate::ir::SequenceActivation {
                        participant,
                        index: last,
                        kind,
                    });
            }
        }
    }

    while let Some(mut frame) = open_frames.pop() {
        let end_idx = graph.edges.len();
        if let Some(last) = frame.sections.last_mut() {
            last.end_idx = end_idx;
        }
        frame.end_idx = end_idx;
        frames.push(frame);
    }
    while let Some(seq_box) = open_boxes.pop() {
        graph.sequence_boxes.push(seq_box);
    }

    graph.sequence_participants = order;
    graph.sequence_frames = frames;
    Ok(ParseOutput { graph, init_config })
}

fn add_node_to_subgraph(graph: &mut Graph, idx: usize, node_id: &str) {
    if let Some(subgraph) = graph.subgraphs.get_mut(idx)
        && !subgraph.nodes.contains(&node_id.to_string())
    {
        subgraph.nodes.push(node_id.to_string());
    }
}

fn add_node_to_subgraphs(graph: &mut Graph, subgraph_stack: &[usize], node_id: &str) {
    for idx in subgraph_stack {
        add_node_to_subgraph(graph, *idx, node_id);
    }
}

fn split_statements(line: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for ch in line.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' {
            current.push(ch);
            escaped = true;
            continue;
        }

        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            current.push(ch);
            continue;
        }

        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            current.push(ch);
            continue;
        }

        match ch {
            '[' | '(' | '{' => {
                depth += 1;
                current.push(ch);
            }
            ']' | ')' | '}' => {
                if depth > 0 {
                    depth -= 1;
                }
                current.push(ch);
            }
            ';' if depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        parts.push(trimmed.to_string());
    }
    parts
}

fn strip_trailing_comment(line: &str) -> String {
    let mut quote: Option<char> = None;
    let mut chars = line.chars().peekable();
    let mut out = String::new();
    while let Some(ch) = chars.next() {
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            out.push(ch);
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            out.push(ch);
            continue;
        }
        if ch == '%'
            && let Some('%') = chars.peek().copied()
        {
            break;
        }
        out.push(ch);
    }
    out.trim().to_string()
}

fn strip_trailing_comment_keep_indent(line: &str) -> String {
    let mut quote: Option<char> = None;
    let mut chars = line.chars().peekable();
    let mut out = String::new();
    while let Some(ch) = chars.next() {
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            out.push(ch);
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            out.push(ch);
            continue;
        }
        if ch == '%'
            && let Some('%') = chars.peek().copied()
        {
            break;
        }
        out.push(ch);
    }
    out.trim_end().to_string()
}

fn extract_leading_decoration(right: &str) -> Option<(char, String)> {
    let mut chars = right.chars();
    let first = chars.next()?;
    if first != 'o' && first != 'x' {
        return None;
    }
    let rest: String = chars.collect();
    if rest.is_empty() {
        return None;
    }
    if rest
        .chars()
        .next()
        .map(|c| c.is_whitespace())
        .unwrap_or(false)
    {
        return Some((first, rest.trim_start().to_string()));
    }
    None
}

fn parse_subgraph_header(input: &str) -> (Option<String>, String, Vec<String>) {
    let (base, classes) = split_inline_classes(input);
    let trimmed = base.trim();
    if trimmed.is_empty() {
        return (None, "Subgraph".to_string(), classes);
    }

    if let Some((id, label, _shape)) = split_id_label(trimmed) {
        return (Some(id.to_string()), label, classes);
    }

    if !trimmed.contains(['"', '\'']) {
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() == 1 {
            let token = parts[0];
            return (Some(token.to_string()), token.to_string(), classes);
        }
    }

    (None, strip_quotes(trimmed), classes)
}

fn parse_node_only(line: &str) -> Option<NodeTokenParts> {
    if line.contains("--") {
        return None;
    }
    let (id, label, shape, classes) = parse_node_token(line);
    if id.is_empty() {
        None
    } else {
        Some((id, label, shape, classes))
    }
}

/// Mask content inside brackets to prevent edge detection from matching dashes in labels.
/// Returns a string where characters inside [...], (...), {...}, "...", '...' are replaced with spaces.
fn mask_bracket_content(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut depth_square = 0;
    let mut depth_paren = 0;
    let mut depth_curly = 0;
    let mut in_double_quote = false;
    let mut in_single_quote = false;
    let mut prev_char = '\0';

    for ch in line.chars() {
        let in_bracket = depth_square > 0 || depth_paren > 0 || depth_curly > 0;
        let in_quote = in_double_quote || in_single_quote;

        match ch {
            '[' if !in_quote => {
                depth_square += 1;
                result.push(ch);
            }
            ']' if !in_quote && depth_square > 0 => {
                depth_square -= 1;
                result.push(ch);
            }
            '(' if !in_quote && !in_bracket => {
                depth_paren += 1;
                result.push(ch);
            }
            ')' if !in_quote && depth_paren > 0 => {
                depth_paren -= 1;
                result.push(ch);
            }
            '{' if !in_quote && !in_bracket => {
                depth_curly += 1;
                result.push(ch);
            }
            '}' if !in_quote && depth_curly > 0 => {
                depth_curly -= 1;
                result.push(ch);
            }
            '"' if prev_char != '\\' => {
                in_double_quote = !in_double_quote;
                if in_bracket || in_quote {
                    result.push(' ');
                } else {
                    result.push(ch);
                }
            }
            '\'' if prev_char != '\\' => {
                in_single_quote = !in_single_quote;
                if in_bracket || in_quote {
                    result.push(' ');
                } else {
                    result.push(ch);
                }
            }
            _ => {
                if in_bracket || in_quote {
                    result.push(' ');
                } else {
                    result.push(ch);
                }
            }
        }
        prev_char = ch;
    }
    result
}

fn split_edge_chain(line: &str) -> Option<Vec<String>> {
    let masked = mask_bracket_content(line);
    if LABEL_ARROW_RE.is_match(&masked) {
        return None;
    }

    let matches: Vec<regex::Match> = ARROW_TOKEN_RE.find_iter(&masked).collect();
    if matches.len() < 2 {
        return None;
    }

    let mut nodes: Vec<String> = Vec::with_capacity(matches.len() + 1);
    let mut arrows: Vec<String> = Vec::with_capacity(matches.len());
    let mut last_idx = 0usize;

    for m in matches {
        nodes.push(line[last_idx..m.start()].trim().to_string());
        arrows.push(line[m.start()..m.end()].trim().to_string());
        last_idx = m.end();
    }
    nodes.push(line[last_idx..].trim().to_string());

    if nodes.len() != arrows.len() + 1 {
        return None;
    }

    // Attach leading pipe labels to the preceding arrow and strip them from the node token.
    for i in 1..nodes.len() {
        let trimmed = nodes[i].trim_start();
        if let Some(stripped) = trimmed.strip_prefix('|')
            && let Some(end_idx) = stripped.find('|')
        {
            let label_len = end_idx + 2;
            let label = &trimmed[..label_len];
            let rest = trimmed[label_len..].trim_start();
            arrows[i - 1].push_str(label);
            nodes[i] = rest.to_string();
        }
    }

    if nodes.iter().any(|node| node.is_empty()) {
        return None;
    }

    let mut statements = Vec::with_capacity(arrows.len());
    for i in 0..arrows.len() {
        statements.push(format!("{} {} {}", nodes[i], arrows[i], nodes[i + 1]));
    }
    Some(statements)
}

fn parse_edge_line(line: &str) -> Option<(String, Option<String>, String, EdgeMeta)> {
    // Mask bracket content to prevent matching dashes inside labels like A[wi-fi]
    let masked = mask_bracket_content(line);

    // Helper to extract from original line using match positions from masked line
    let extract = |m: regex::Match| -> &str { &line[m.start()..m.end()] };

    if let Some(caps) = PIPE_LABEL_RE.captures(&masked) {
        let left_match = caps.name("left")?;
        let right_match = caps.name("right")?;
        let label_match = caps.name("label")?;
        let arrow_match = caps.name("arrow")?;
        let left = extract(left_match).trim();
        let right = extract(right_match).trim();
        let label_clean = extract(label_match).trim();
        if !label_clean.is_empty() && !left.is_empty() && !right.is_empty() {
            let arrow = extract(arrow_match).trim();
            let edge_meta = parse_edge_meta(arrow);
            return Some((
                left.to_string(),
                Some(label_clean.to_string()),
                right.to_string(),
                edge_meta,
            ));
        }
    }

    if let Some(caps) = LABEL_ARROW_RE.captures(&masked) {
        let left_match = caps.name("left")?;
        let right_match = caps.name("right")?;
        let label_match = caps.name("label")?;
        let left = extract(left_match).trim();
        let right = extract(right_match).trim();
        let label_raw = extract(label_match).trim();
        let label_clean = label_raw.trim_matches('|').trim();
        if !label_clean.is_empty() && !left.is_empty() && !right.is_empty() {
            let start = caps.name("start").map(|m| m.as_str()).unwrap_or("");
            let dash1 = caps.name("dash1")?.as_str();
            let dash2 = caps.name("dash2")?.as_str();
            let end = caps.name("end").map(|m| m.as_str()).unwrap_or("");
            let arrow = format!("{}{}{}{}", start, dash1, dash2, end);
            let edge_meta = parse_edge_meta(&arrow);
            return Some((
                left.to_string(),
                Some(label_clean.to_string()),
                right.to_string(),
                edge_meta,
            ));
        }
    }

    let caps = ARROW_RE.captures(&masked)?;
    let left_match = caps.name("left")?;
    let right_match = caps.name("right")?;
    let left = extract(left_match).trim();
    let mut arrow = caps.name("arrow")?.as_str().trim().to_string();
    let mut right = extract(right_match).trim().to_string();

    if let Some((dec, rest)) = extract_leading_decoration(&right) {
        arrow.push(dec);
        right = rest;
    }

    if left.is_empty() || right.is_empty() || arrow.is_empty() {
        return None;
    }

    let (label, right_token) = if let Some(stripped) = right.strip_prefix('|') {
        if let Some(end) = stripped.find('|') {
            let label = stripped[..end].trim().to_string();
            let rest = stripped[end + 1..].trim();
            (Some(label), rest)
        } else {
            (None, right.as_str())
        }
    } else {
        (None, right.as_str())
    };

    if right_token.is_empty() {
        return None;
    }

    let edge_meta = parse_edge_meta(&arrow);
    Some((left.to_string(), label, right_token.to_string(), edge_meta))
}

#[derive(Debug, Clone, Copy)]
struct EdgeMeta {
    directed: bool,
    arrow_start: bool,
    arrow_end: bool,
    arrow_start_kind: Option<crate::ir::EdgeArrowhead>,
    arrow_end_kind: Option<crate::ir::EdgeArrowhead>,
    start_decoration: Option<crate::ir::EdgeDecoration>,
    end_decoration: Option<crate::ir::EdgeDecoration>,
    style: crate::ir::EdgeStyle,
}

fn parse_edge_meta(arrow: &str) -> EdgeMeta {
    let mut trimmed = arrow.trim().to_string();
    let mut start_decoration = None;
    let mut end_decoration = None;

    if trimmed.starts_with('o') {
        start_decoration = Some(crate::ir::EdgeDecoration::Circle);
        trimmed.remove(0);
    } else if trimmed.starts_with('x') {
        start_decoration = Some(crate::ir::EdgeDecoration::Cross);
        trimmed.remove(0);
    }

    if trimmed.ends_with('o') {
        end_decoration = Some(crate::ir::EdgeDecoration::Circle);
        trimmed.pop();
    } else if trimmed.ends_with('x') {
        end_decoration = Some(crate::ir::EdgeDecoration::Cross);
        trimmed.pop();
    }

    let arrow_start = trimmed.starts_with('<');
    let arrow_end = trimmed.ends_with('>');

    let style = if trimmed.contains('=') {
        crate::ir::EdgeStyle::Thick
    } else if trimmed.contains('.') {
        crate::ir::EdgeStyle::Dotted
    } else {
        crate::ir::EdgeStyle::Solid
    };

    let directed = arrow_start || arrow_end;

    EdgeMeta {
        directed,
        arrow_start,
        arrow_end,
        arrow_start_kind: None,
        arrow_end_kind: None,
        start_decoration,
        end_decoration,
        style,
    }
}

fn parse_direction_line(line: &str) -> Option<Direction> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() == 2 && parts[0] == "direction" {
        return Direction::from_token(parts[1]);
    }
    None
}

fn parse_class_def(line: &str, graph: &mut Graph) {
    let trimmed = line.trim();
    let mut parts = trimmed.splitn(3, char::is_whitespace);
    let _ = parts.next();
    let class_name = parts.next().unwrap_or("").trim();
    let rest = parts.next().unwrap_or("").trim();
    if class_name.is_empty() || rest.is_empty() {
        return;
    }
    let style = parse_node_style(rest);
    graph.class_defs.insert(class_name.to_string(), style);
}

fn parse_class_line(line: &str, graph: &mut Graph) {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return;
    }
    let class_name = parts.last().unwrap().to_string();
    let class_names: Vec<String> = class_name
        .split(',')
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect();
    if class_names.is_empty() {
        return;
    }
    let nodes_raw = parts[1..parts.len() - 1].join(" ");
    for node_id in nodes_raw.split(',') {
        let id = node_id.trim();
        if id.is_empty() {
            continue;
        }
        for class_name in &class_names {
            graph
                .node_classes
                .entry(id.to_string())
                .or_default()
                .push(class_name.clone());
            graph
                .subgraph_classes
                .entry(id.to_string())
                .or_default()
                .push(class_name.clone());
        }
    }
}

fn apply_node_classes(graph: &mut Graph, node_id: &str, classes: &[String]) {
    for class_name in classes {
        if class_name.is_empty() {
            continue;
        }
        graph
            .node_classes
            .entry(node_id.to_string())
            .or_default()
            .push(class_name.clone());
    }
}

fn apply_subgraph_classes(graph: &mut Graph, subgraph_id: &str, classes: &[String]) {
    for class_name in classes {
        if class_name.is_empty() {
            continue;
        }
        graph
            .subgraph_classes
            .entry(subgraph_id.to_string())
            .or_default()
            .push(class_name.clone());
    }
}

fn parse_style_line(line: &str, graph: &mut Graph) {
    let mut parts = line.splitn(3, ' ');
    let _ = parts.next();
    let node_id = parts.next().unwrap_or("").trim();
    let rest = parts.next().unwrap_or("").trim();
    if node_id.is_empty() || rest.is_empty() {
        return;
    }
    let style = parse_node_style(rest);
    for raw in node_id.split(',') {
        let id = raw.trim();
        if id.is_empty() {
            continue;
        }
        graph.node_styles.insert(id.to_string(), style.clone());
        graph.subgraph_styles.insert(id.to_string(), style.clone());
    }
}

fn parse_link_style_line(line: &str, graph: &mut Graph) {
    let trimmed = line.trim();
    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    if tokens.len() < 3 {
        return;
    }

    let mut style_idx = None;
    for (idx, token) in tokens.iter().enumerate().skip(1) {
        if token.contains(':') {
            style_idx = Some(idx);
            break;
        }
    }
    let Some(style_idx) = style_idx else {
        return;
    };
    let index_tokens = &tokens[1..style_idx];
    let style_str = tokens[style_idx..].join(" ");
    if style_str.is_empty() {
        return;
    }

    let style = parse_edge_style(&style_str);
    if index_tokens.len() == 1 && index_tokens[0] == "default" {
        graph.edge_style_default = Some(style);
        return;
    }

    for raw in index_tokens.iter().flat_map(|token| token.split(',')) {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }
        if let Ok(index) = token.parse::<usize>() {
            graph.edge_styles.insert(index, style.clone());
        }
    }
}

fn tokenize_quoted(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            continue;
        }
        if ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(current);
                current = String::new();
            }
            continue;
        }
        current.push(ch);
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn parse_click_line(line: &str) -> Option<(String, crate::ir::NodeLink)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let keyword_len = if lower.starts_with("click ") {
        5
    } else if lower.starts_with("link ") {
        4
    } else {
        return None;
    };
    let rest = trimmed[keyword_len..].trim();
    let tokens = tokenize_quoted(rest);
    if tokens.len() < 2 {
        return None;
    }
    let id = tokens[0].clone();
    let mut idx = 1usize;
    if tokens[idx].eq_ignore_ascii_case("call") {
        return None;
    }
    if tokens[idx].eq_ignore_ascii_case("href") {
        idx += 1;
    }
    let url = tokens.get(idx)?.clone();
    idx += 1;
    let mut title = None;
    let mut target = None;
    if let Some(token) = tokens.get(idx) {
        if token.starts_with('_') {
            target = Some(token.clone());
            idx += 1;
        } else {
            title = Some(token.clone());
            idx += 1;
        }
    }
    if target.is_none() {
        if let Some(token) = tokens.get(idx) {
            if token.starts_with('_') {
                target = Some(token.clone());
            }
        }
    }

    Some((id, crate::ir::NodeLink { url, title, target }))
}

fn parse_node_style(input: &str) -> crate::ir::NodeStyle {
    let mut style = crate::ir::NodeStyle::default();
    for part in input.split(',') {
        let mut kv = part.splitn(2, ':');
        let key = kv.next().unwrap_or("").trim();
        let value = kv.next().unwrap_or("").trim();
        if key.is_empty() || value.is_empty() {
            continue;
        }
        match key {
            "fill" => style.fill = Some(value.to_string()),
            "stroke" => style.stroke = Some(value.to_string()),
            "stroke-width" => {
                let width = value.trim_end_matches("px").parse::<f32>().ok();
                style.stroke_width = width;
            }
            "stroke-dasharray" => style.stroke_dasharray = Some(value.to_string()),
            "color" => style.text_color = Some(value.to_string()),
            _ => {}
        }
    }
    style
}

fn parse_edge_style(input: &str) -> crate::ir::EdgeStyleOverride {
    let mut style = crate::ir::EdgeStyleOverride::default();
    for part in input.split(',') {
        let mut kv = part.splitn(2, ':');
        let key = kv.next().unwrap_or("").trim();
        let value = kv.next().unwrap_or("").trim();
        if key.is_empty() || value.is_empty() {
            continue;
        }
        match key {
            "stroke" => style.stroke = Some(value.to_string()),
            "stroke-width" => {
                style.stroke_width = value.trim_end_matches("px").parse::<f32>().ok();
            }
            "stroke-dasharray" => style.dasharray = Some(value.to_string()),
            "color" => style.label_color = Some(value.to_string()),
            _ => {}
        }
    }
    style
}

fn parse_node_token(
    token: &str,
) -> (
    String,
    Option<String>,
    Option<crate::ir::NodeShape>,
    Vec<String>,
) {
    let (base, classes) = split_inline_classes(token);
    let trimmed = base.trim();
    if let Some((id, label, shape)) = split_asymmetric_label(trimmed) {
        return (id, Some(label), Some(shape), classes);
    }
    if let Some((id, label, shape)) = split_id_label(trimmed) {
        return (id.to_string(), Some(label), Some(shape), classes);
    }

    let id = trimmed.split_whitespace().next().unwrap_or("").to_string();
    (id, None, None, classes)
}

fn split_asymmetric_label(token: &str) -> Option<(String, String, crate::ir::NodeShape)> {
    let trimmed = token.trim();
    if trimmed.contains('[') {
        return None;
    }
    let Some(pos) = trimmed.find('>') else {
        return None;
    };
    if !trimmed.ends_with(']') {
        return None;
    }
    let id = trimmed[..pos].trim();
    if id.is_empty() {
        return None;
    }
    let label = trimmed[pos + 1..trimmed.len() - 1].trim();
    if label.is_empty() {
        return None;
    }
    Some((
        id.to_string(),
        strip_quotes(label),
        crate::ir::NodeShape::Asymmetric,
    ))
}

fn split_inline_classes(token: &str) -> (String, Vec<String>) {
    let mut parts = token.split(":::");
    let base = parts.next().unwrap_or("").trim().to_string();
    let classes = parts
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    (base, classes)
}

fn split_id_label(token: &str) -> Option<(&str, String, crate::ir::NodeShape)> {
    if let Some(start) = token.find('[')
        && token.ends_with(']')
    {
        let id = token[..start].trim();
        if !id.is_empty() {
            let raw = &token[start..];
            let (label, shape) = parse_shape_from_brackets(raw);
            return Some((id, label, shape));
        }
    }

    if let Some(start) = token.find('(')
        && token.ends_with(')')
    {
        let id = token[..start].trim();
        if !id.is_empty() {
            let raw = &token[start..];
            let (label, shape) = parse_shape_from_parens(raw);
            return Some((id, label, shape));
        }
    }

    if let Some(start) = token.find('{')
        && token.ends_with('}')
    {
        let id = token[..start].trim();
        if !id.is_empty() {
            let raw = &token[start..];
            let (label, shape) = parse_shape_from_braces(raw);
            return Some((id, label, shape));
        }
    }

    None
}

fn parse_shape_from_brackets(raw: &str) -> (String, crate::ir::NodeShape) {
    let trimmed = raw.trim();
    if trimmed.starts_with("[/") && trimmed.ends_with("/]") {
        return (
            strip_quotes(&trimmed[2..trimmed.len() - 2]),
            crate::ir::NodeShape::Parallelogram,
        );
    }
    if trimmed.starts_with("[\\") && trimmed.ends_with("\\]") {
        return (
            strip_quotes(&trimmed[2..trimmed.len() - 2]),
            crate::ir::NodeShape::ParallelogramAlt,
        );
    }
    if trimmed.starts_with("[/") && trimmed.ends_with("\\]") {
        return (
            strip_quotes(&trimmed[2..trimmed.len() - 2]),
            crate::ir::NodeShape::Trapezoid,
        );
    }
    if trimmed.starts_with("[\\") && trimmed.ends_with("/]") {
        return (
            strip_quotes(&trimmed[2..trimmed.len() - 2]),
            crate::ir::NodeShape::TrapezoidAlt,
        );
    }
    if trimmed.starts_with("[[") && trimmed.ends_with("]]") {
        return (
            strip_quotes(&trimmed[2..trimmed.len() - 2]),
            crate::ir::NodeShape::Subroutine,
        );
    }
    if trimmed.starts_with("[(") && trimmed.ends_with(")]") {
        return (
            strip_quotes(&trimmed[2..trimmed.len() - 2]),
            crate::ir::NodeShape::Cylinder,
        );
    }
    if trimmed.starts_with("[") && trimmed.ends_with("]") {
        let inner = &trimmed[1..trimmed.len() - 1];
        if inner.starts_with('(') && inner.ends_with(')') {
            return (
                strip_quotes(&inner[1..inner.len() - 1]),
                crate::ir::NodeShape::Stadium,
            );
        }
        return (strip_quotes(inner), crate::ir::NodeShape::Rectangle);
    }
    (strip_quotes(trimmed), crate::ir::NodeShape::Rectangle)
}

fn parse_shape_from_parens(raw: &str) -> (String, crate::ir::NodeShape) {
    let trimmed = raw.trim();
    if trimmed.starts_with("(((") && trimmed.ends_with(")))") {
        return (
            strip_quotes(&trimmed[3..trimmed.len() - 3]),
            crate::ir::NodeShape::DoubleCircle,
        );
    }
    if trimmed.starts_with("((") && trimmed.ends_with("))") {
        return (
            strip_quotes(&trimmed[2..trimmed.len() - 2]),
            crate::ir::NodeShape::DoubleCircle,
        );
    }
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        let inner = &trimmed[1..trimmed.len() - 1];
        if inner.starts_with('[') && inner.ends_with(']') {
            return (
                strip_quotes(&inner[1..inner.len() - 1]),
                crate::ir::NodeShape::Stadium,
            );
        }
        return (strip_quotes(inner), crate::ir::NodeShape::RoundRect);
    }
    (strip_quotes(trimmed), crate::ir::NodeShape::RoundRect)
}

fn parse_shape_from_braces(raw: &str) -> (String, crate::ir::NodeShape) {
    let trimmed = raw.trim();
    if trimmed.starts_with("{{") && trimmed.ends_with("}}") {
        return (
            strip_quotes(&trimmed[2..trimmed.len() - 2]),
            crate::ir::NodeShape::Hexagon,
        );
    }
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return (
            strip_quotes(&trimmed[1..trimmed.len() - 1]),
            crate::ir::NodeShape::Diamond,
        );
    }
    (strip_quotes(trimmed), crate::ir::NodeShape::Diamond)
}

fn strip_quotes(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        trimmed[1..trimmed.len() - 1].to_string()
    } else if trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2 {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn count_indent(line: &str) -> usize {
    let mut count = 0;
    for ch in line.chars() {
        match ch {
            ' ' => count += 1,
            '\t' => count += 2,
            _ => break,
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::DiagramKind;

    #[test]
    fn parse_simple_flowchart() {
        let input = "flowchart lr\nA[Start] -->|go| B(End)";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.nodes.len(), 2);
        assert_eq!(parsed.graph.edges.len(), 1);
        assert_eq!(parsed.graph.edges[0].label.as_deref(), Some("go"));
        assert_eq!(parsed.graph.direction, Direction::LeftRight);
        assert_eq!(
            parsed.graph.nodes.get("B").unwrap().shape,
            crate::ir::NodeShape::RoundRect
        );
    }

    #[test]
    fn parse_subgraph() {
        let input = "flowchart TD\nsubgraph Group[\"My Group\"]\nA --> B\nend";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.subgraphs.len(), 1);
        let sg = &parsed.graph.subgraphs[0];
        assert_eq!(sg.label, "My Group");
        assert_eq!(sg.nodes.len(), 2);
    }

    #[test]
    fn parse_nested_subgraphs() {
        let input = "flowchart LR\nsubgraph Outer\n  subgraph Inner\n    A --> B\n  end\nend";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.subgraphs.len(), 2);
        let outer = &parsed.graph.subgraphs[0];
        let inner = &parsed.graph.subgraphs[1];
        assert!(outer.nodes.contains(&"A".to_string()));
        assert!(outer.nodes.contains(&"B".to_string()));
        assert!(inner.nodes.contains(&"A".to_string()));
        assert!(inner.nodes.contains(&"B".to_string()));
    }

    #[test]
    fn parse_edge_styles() {
        let input = "flowchart LR\nA -.-> B\nC ==> D\nE <--> F\nG --- H\nlinkStyle 0 stroke:#0ff,stroke-width:2,color:#f00";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 4);
        assert_eq!(parsed.graph.edges[0].style, crate::ir::EdgeStyle::Dotted);
        assert_eq!(parsed.graph.edges[1].style, crate::ir::EdgeStyle::Thick);
        assert_eq!(parsed.graph.edges[2].arrow_start, true);
        assert_eq!(parsed.graph.edges[2].arrow_end, true);
        assert_eq!(parsed.graph.edges[3].directed, false);
        let style = parsed.graph.edge_styles.get(&0).unwrap();
        assert_eq!(style.label_color.as_deref(), Some("#f00"));
    }

    #[test]
    fn parse_class_and_styles() {
        let input = "flowchart LR\nclassDef hot fill:#f00,stroke:#000,color:#fff,stroke-width:2\nA[One]\nclass A hot\nstyle A fill:#0f0,stroke:#00f,stroke-width:3,color:#111\nA --> B\nlinkStyle 0 stroke:#0ff,stroke-width:4,stroke-dasharray:5 5";
        let parsed = parse_mermaid(input).unwrap();
        assert!(parsed.graph.class_defs.contains_key("hot"));
        assert!(parsed.graph.node_classes.contains_key("A"));
        assert!(parsed.graph.node_styles.contains_key("A"));
        assert!(parsed.graph.edge_styles.contains_key(&0));
        let edge_style = parsed.graph.edge_styles.get(&0).unwrap();
        assert_eq!(edge_style.stroke.as_deref(), Some("#0ff"));
    }

    #[test]
    fn parse_inline_class_and_linkstyle_default() {
        let input = "flowchart LR\nclassDef hot fill:#f00\nA[Alpha]:::hot --> B\nB --> C\nlinkStyle default stroke:#0ff,stroke-width:3\nlinkStyle 1 stroke:#00f";
        let parsed = parse_mermaid(input).unwrap();
        let classes = parsed
            .graph
            .node_classes
            .get("A")
            .cloned()
            .unwrap_or_default();
        assert!(classes.iter().any(|c| c == "hot"));
        assert!(parsed.graph.edge_style_default.is_some());
        let edge_style = parsed.graph.edge_styles.get(&1).unwrap();
        assert_eq!(edge_style.stroke.as_deref(), Some("#00f"));
    }

    #[test]
    fn parse_edge_label_in_arrow() {
        let input = "flowchart LR\nA -- needs review --> B\nC --|ship it|--> D";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 2);
        assert_eq!(parsed.graph.edges[0].label.as_deref(), Some("needs review"));
        assert_eq!(parsed.graph.edges[1].label.as_deref(), Some("ship it"));
    }

    #[test]
    fn parse_pipe_edge_label() {
        let input = "flowchart LR\nA -->|yes| B";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 1);
        assert_eq!(parsed.graph.edges[0].label.as_deref(), Some("yes"));
    }

    #[test]
    fn parse_multi_target_edges() {
        let input = "flowchart LR\nA --> B & C";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 2);
        assert!(parsed.graph.nodes.contains_key("B"));
        assert!(parsed.graph.nodes.contains_key("C"));
    }

    #[test]
    fn parse_multi_source_edges() {
        let input = "flowchart LR\nA & B --> C";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 2);
        assert!(parsed.graph.nodes.contains_key("A"));
        assert!(parsed.graph.nodes.contains_key("B"));
        assert!(parsed.graph.nodes.contains_key("C"));
    }

    #[test]
    fn parse_subgraph_style() {
        let input = "flowchart LR\nclassDef hot fill:#f00,stroke:#0f0\nsubgraph SG[Group]:::hot\nA --> B\nend\nclass SG hot\nstyle SG fill:#faf,stroke:#111";
        let parsed = parse_mermaid(input).unwrap();
        let style = parsed.graph.subgraph_styles.get("SG").unwrap();
        assert_eq!(style.fill.as_deref(), Some("#faf"));
        assert_eq!(style.stroke.as_deref(), Some("#111"));
        let classes = parsed.graph.subgraph_classes.get("SG").unwrap();
        assert!(classes.iter().any(|c| c == "hot"));
    }

    #[test]
    fn parse_semicolon_statements() {
        let input = "flowchart LR; A --> B; B --> C";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 2);
    }

    #[test]
    fn parse_subgraph_single_token_id() {
        let input = "flowchart LR\nsubgraph Alpha\nA --> B\nend\nstyle Alpha fill:#fff";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.subgraphs.len(), 1);
        assert_eq!(parsed.graph.subgraphs[0].id.as_deref(), Some("Alpha"));
        assert!(parsed.graph.subgraph_styles.contains_key("Alpha"));
    }

    #[test]
    fn parse_style_multiple_nodes() {
        let input = "flowchart LR\nA-->B\nstyle A,B fill:#f00";
        let parsed = parse_mermaid(input).unwrap();
        assert!(parsed.graph.node_styles.contains_key("A"));
        assert!(parsed.graph.node_styles.contains_key("B"));
    }

    #[test]
    fn parse_edge_decorations() {
        let input = "flowchart LR\nA o--o B\nC x--> D";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 2);
        assert_eq!(
            parsed.graph.edges[0].start_decoration,
            Some(crate::ir::EdgeDecoration::Circle)
        );
        assert_eq!(
            parsed.graph.edges[0].end_decoration,
            Some(crate::ir::EdgeDecoration::Circle)
        );
        assert_eq!(
            parsed.graph.edges[1].start_decoration,
            Some(crate::ir::EdgeDecoration::Cross)
        );
        assert!(parsed.graph.edges[1].arrow_end);
    }

    #[test]
    fn parse_class_diagram_basic() {
        let input = "classDiagram\nclass Animal {\n+String name\n+eat()\n}\nclass Dog\nAnimal <|-- Dog : inherits";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Class);
        assert!(parsed.graph.nodes.contains_key("Animal"));
        assert!(parsed.graph.nodes.contains_key("Dog"));
        assert_eq!(parsed.graph.edges.len(), 1);
        assert_eq!(parsed.graph.edges[0].label.as_deref(), Some("inherits"));
        let label = &parsed.graph.nodes.get("Animal").unwrap().label;
        assert!(label.contains("Animal"));
        assert!(label.contains("name"));
    }

    #[test]
    fn parse_class_relation_multiplicity() {
        let input = "classDiagram\nClass01 \"1\" *-- \"many\" Class02 : contains";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 1);
        let edge = &parsed.graph.edges[0];
        assert_eq!(edge.start_label.as_deref(), Some("1"));
        assert_eq!(edge.end_label.as_deref(), Some("many"));
        assert_eq!(edge.label.as_deref(), Some("contains"));
    }

    #[test]
    fn parse_er_diagram_basic() {
        let input =
            "erDiagram\nCUSTOMER ||--o{ ORDER : places\nCUSTOMER {\nstring id\nstring name\n}";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Er);
        assert_eq!(parsed.graph.edges.len(), 1);
        let edge = &parsed.graph.edges[0];
        assert_eq!(edge.label.as_deref(), Some("places"));
        // ER diagrams use crow's foot decorations, not text labels
        assert_eq!(edge.start_label, None);
        assert_eq!(edge.end_label, None);
        assert_eq!(
            edge.start_decoration,
            Some(crate::ir::EdgeDecoration::CrowsFootOne)
        );
        assert_eq!(
            edge.end_decoration,
            Some(crate::ir::EdgeDecoration::CrowsFootZeroMany)
        );
        let customer = parsed.graph.nodes.get("CUSTOMER").unwrap();
        assert!(customer.label.contains("CUSTOMER"));
        assert!(customer.label.contains("string id"));
    }

    #[test]
    fn parse_pie_diagram_basic() {
        let input = "pie showData\n  title Pets\n  \"Dogs\" : 10\n  Cats : 5";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Pie);
        assert!(parsed.graph.pie_show_data);
        assert_eq!(parsed.graph.pie_title.as_deref(), Some("Pets"));
        assert_eq!(parsed.graph.pie_slices.len(), 2);
        assert_eq!(parsed.graph.pie_slices[0].label, "Dogs");
        assert_eq!(parsed.graph.pie_slices[0].value, 10.0);
    }

    #[test]
    fn parse_mindmap_basic() {
        let input = "mindmap\n  root((Root))\n    Child A\n    Child B\n      Grandchild";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Mindmap);
        assert!(parsed.graph.nodes.len() >= 4);
        assert_eq!(parsed.graph.edges.len(), 3);
    }

    #[test]
    fn parse_journey_basic() {
        let input = "journey\n  title My Journey\n  section Start\n    Step one: 5: Alice\n    Step two: 3: Alice, Bob";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Journey);
        assert_eq!(parsed.graph.subgraphs.len(), 1);
        assert_eq!(parsed.graph.nodes.len(), 2);
        assert_eq!(parsed.graph.edges.len(), 1);
    }

    #[test]
    fn parse_timeline_basic() {
        let input = "timeline\n  title History\n  2020 : Launch\n  2021 : Growth";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Timeline);
        assert_eq!(parsed.graph.timeline.events.len(), 2);
        assert_eq!(parsed.graph.timeline.title.as_deref(), Some("History"));
        assert_eq!(parsed.graph.timeline.events[0].time, "2020");
        assert_eq!(parsed.graph.timeline.events[0].events, vec!["Launch"]);
    }

    #[test]
    fn parse_gantt_basic() {
        let input = "gantt\n  title Plan\n  section Alpha\n  Task A : done, a1, 2020-01-01, 5d\n  Task B : after a1, 3d";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Gantt);
        assert!(parsed.graph.nodes.len() >= 2);
        assert_eq!(parsed.graph.edges.len(), 1);
    }

    #[test]
    fn parse_requirement_basic() {
        let input = "requirementDiagram\n  requirement req1 {\n    id: 1\n    text: Login\n  }\n  requirement req2\n  req1 - satisfies -> req2";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Requirement);
        assert_eq!(parsed.graph.nodes.len(), 2);
        assert_eq!(parsed.graph.edges.len(), 1);
        assert_eq!(parsed.graph.edges[0].label.as_deref(), Some("satisfies"));
    }

    #[test]
    fn parse_gitgraph_basic() {
        let input = "gitGraph\n  commit\n  branch feature\n  checkout feature\n  commit id:\"F1\"\n  checkout main\n  merge feature";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::GitGraph);
        assert!(parsed.graph.gitgraph.commits.len() >= 3);
        assert!(parsed.graph.gitgraph.branches.len() >= 2);
    }

    #[test]
    fn parse_c4_basic() {
        let input = "C4Context\n  Person(admin, \"Admin\")\n  System(sys, \"System\")\n  Rel(admin, sys, \"Uses\")\n  Boundary(b0, \"Boundary\") { SystemDb(db, \"DB\") }";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::C4);
        assert!(parsed.graph.c4.shapes.len() >= 3);
        assert_eq!(parsed.graph.c4.rels.len(), 1);
        assert!(parsed.graph.c4.boundaries.len() >= 2);
    }

    #[test]
    fn parse_sankey_basic() {
        let input = "sankey\n  A, B, 10\n  B, C, 5";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Sankey);
        assert_eq!(parsed.graph.edges.len(), 2);
    }

    #[test]
    fn parse_quadrant_basic() {
        let input = "quadrantChart\n  title Sample\n  A : [0.2, 0.8]\n  B : [0.7, 0.3]";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Quadrant);
        assert_eq!(parsed.graph.nodes.len(), 2);
    }

    #[test]
    fn parse_zenuml_basic() {
        let input = "zenuml\n  Alice->Bob: Hello\n  Bob-->Alice: Reply";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::ZenUML);
        assert_eq!(parsed.graph.sequence_participants.len(), 2);
        assert_eq!(parsed.graph.edges.len(), 2);
    }

    #[test]
    fn parse_block_basic() {
        let input = "block\n  A --> B";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Block);
        assert_eq!(parsed.graph.edges.len(), 1);
    }

    #[test]
    fn parse_packet_basic() {
        let input = "packet\n  0-7: \"Type\"\n  8-15: \"Len\"";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Packet);
        assert_eq!(parsed.graph.nodes.len(), 2);
        assert_eq!(parsed.graph.edges.len(), 1);
    }

    #[test]
    fn parse_kanban_basic() {
        let input = "kanban\n  todo[To Do]\n    t1[Task 1]\n  done[Done]\n    t2[Task 2]";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Kanban);
        assert_eq!(parsed.graph.subgraphs.len(), 2);
        assert_eq!(parsed.graph.nodes.len(), 2);
    }

    #[test]
    fn parse_architecture_basic() {
        let input = "architecture-beta\n  group api(icon)[API]\n  service web(icon)[Web] in api\n  service db(icon)[DB] in api\n  web:R --> L:db";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Architecture);
        assert_eq!(parsed.graph.subgraphs.len(), 1);
        assert_eq!(parsed.graph.edges.len(), 1);
    }

    #[test]
    fn parse_radar_basic() {
        let input = "radar-beta\n  axis A, B, C\n  curve Alpha {1,2,3}";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Radar);
        assert_eq!(parsed.graph.nodes.len(), 1);
    }

    #[test]
    fn parse_treemap_basic() {
        let input = "treemap-beta\n  Root: 100\n    Child: 40";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Treemap);
        assert_eq!(parsed.graph.nodes.len(), 2);
        assert_eq!(parsed.graph.edges.len(), 1);
    }

    #[test]
    fn parse_xy_chart_basic() {
        let input = "xychart-beta\n  x-axis Q1, Q2\n  y-axis Units\n  bar [10, 20]";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::XYChart);
        let xychart = &parsed.graph.xychart;
        assert_eq!(xychart.x_axis_categories, vec!["Q1", "Q2"]);
        assert_eq!(xychart.y_axis_label.as_deref(), Some("Units"));
        assert_eq!(xychart.series.len(), 1);
    }

    #[test]
    fn parse_state_diagram_basic() {
        let input = "stateDiagram-v2\n[*] --> Idle\nIdle --> Active : start\nstate \"Waiting\" as Wait\nWait --> Active";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::State);
        assert!(parsed.graph.nodes.contains_key("Idle"));
        assert!(parsed.graph.nodes.contains_key("Active"));
        assert!(parsed.graph.nodes.contains_key("Wait"));
        let wait_label = &parsed.graph.nodes.get("Wait").unwrap().label;
        assert_eq!(wait_label, "Waiting");
        assert!(parsed.graph.edges.len() >= 2);
    }

    #[test]
    fn parse_state_description_line() {
        let input = "stateDiagram-v2\nstate Idle : Waiting\nIdle --> Done";
        let parsed = parse_mermaid(input).unwrap();
        let node = parsed.graph.nodes.get("Idle").unwrap();
        assert_eq!(node.label, "Waiting");
    }

    #[test]
    fn parse_state_choice_stereotype() {
        let input = "stateDiagram-v2\nstate Decide <<choice>>\n[*] --> Decide";
        let parsed = parse_mermaid(input).unwrap();
        let node = parsed.graph.nodes.get("Decide").unwrap();
        assert_eq!(node.shape, crate::ir::NodeShape::Diamond);
    }

    #[test]
    fn parse_state_fork_stereotype() {
        let input = "stateDiagram-v2\nstate Fork <<fork>>\n[*] --> Fork";
        let parsed = parse_mermaid(input).unwrap();
        let node = parsed.graph.nodes.get("Fork").unwrap();
        assert_eq!(node.shape, crate::ir::NodeShape::ForkJoin);
        assert!(node.label.trim().is_empty());
    }

    #[test]
    fn parse_state_inline_class() {
        let input = "stateDiagram-v2\nclassDef hot fill:#f00\nstate Idle:::hot";
        let parsed = parse_mermaid(input).unwrap();
        let classes = parsed.graph.node_classes.get("Idle").unwrap();
        assert!(classes.iter().any(|c| c == "hot"));
    }

    #[test]
    fn parse_state_note() {
        let input = "stateDiagram-v2\nstate Idle\nnote right of Idle: waiting";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.state_notes.len(), 1);
        let note = &parsed.graph.state_notes[0];
        assert_eq!(note.target, "Idle");
        assert_eq!(note.label, "waiting");
        assert_eq!(note.position, crate::ir::StateNotePosition::RightOf);
    }

    #[test]
    fn parse_sequence_diagram_basic() {
        let input = "sequenceDiagram\nparticipant Alice as A\nparticipant Bob\nA->>Bob: Hello\nBob-->>A: Hi";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Sequence);
        assert_eq!(parsed.graph.sequence_participants.len(), 2);
        assert_eq!(parsed.graph.sequence_participants[0], "A");
        assert_eq!(parsed.graph.sequence_participants[1], "Bob");
        assert_eq!(parsed.graph.edges.len(), 2);
        assert_eq!(parsed.graph.edges[1].style, crate::ir::EdgeStyle::Dotted);
    }

    #[test]
    fn parse_sequence_database_participant() {
        let input = "sequenceDiagram\ndatabase DB\nDB->>DB: ping";
        let parsed = parse_mermaid(input).unwrap();
        let node = parsed.graph.nodes.get("DB").unwrap();
        assert_eq!(node.shape, crate::ir::NodeShape::Cylinder);
    }

    #[test]
    fn parse_sequence_autonumber_off() {
        let input = "sequenceDiagram\nautonumber off\nA->>B: ping";
        let parsed = parse_mermaid(input).unwrap();
        assert!(parsed.graph.sequence_autonumber.is_none());
    }

    #[test]
    fn parse_sequence_alt_sections() {
        let input = "sequenceDiagram\nA->>B: req\nalt ok\nB-->>A: yes\nelse bad\nB-->>A: no\nend";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Sequence);
        assert_eq!(parsed.graph.edges.len(), 3);
        assert_eq!(parsed.graph.sequence_frames.len(), 1);
        let frame = &parsed.graph.sequence_frames[0];
        assert_eq!(frame.sections.len(), 2);
        assert_eq!(frame.sections[0].label.as_deref(), Some("ok"));
        assert_eq!(frame.sections[0].start_idx, 1);
        assert_eq!(frame.sections[0].end_idx, 2);
        assert_eq!(frame.sections[1].label.as_deref(), Some("bad"));
        assert_eq!(frame.sections[1].start_idx, 2);
        assert_eq!(frame.sections[1].end_idx, 3);
    }

    #[test]
    fn parse_sequence_par_sections() {
        let input =
            "sequenceDiagram\nA->>B: req\npar first\nB-->>A: yes\nand second\nB-->>A: no\nend";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.sequence_frames.len(), 1);
        let frame = &parsed.graph.sequence_frames[0];
        assert_eq!(frame.kind, crate::ir::SequenceFrameKind::Par);
        assert_eq!(frame.sections.len(), 2);
        assert_eq!(frame.sections[0].label.as_deref(), Some("first"));
        assert_eq!(frame.sections[1].label.as_deref(), Some("second"));
    }

    #[test]
    fn parse_sequence_critical_sections() {
        let input =
            "sequenceDiagram\nA->>B: req\ncritical ok\nB-->>A: yes\noption fail\nB-->>A: no\nend";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.sequence_frames.len(), 1);
        let frame = &parsed.graph.sequence_frames[0];
        assert_eq!(frame.kind, crate::ir::SequenceFrameKind::Critical);
        assert_eq!(frame.sections.len(), 2);
        assert_eq!(frame.sections[0].label.as_deref(), Some("ok"));
        assert_eq!(frame.sections[1].label.as_deref(), Some("fail"));
    }

    #[test]
    fn parse_sequence_box() {
        let input = "sequenceDiagram\nbox Aqua Group\nparticipant A\nparticipant B\nend";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.sequence_boxes.len(), 1);
        let seq_box = &parsed.graph.sequence_boxes[0];
        assert_eq!(seq_box.color.as_deref(), Some("Aqua"));
        assert_eq!(seq_box.label.as_deref(), Some("Group"));
        assert_eq!(seq_box.participants.len(), 2);
        assert!(seq_box.participants.iter().any(|id| id == "A"));
        assert!(seq_box.participants.iter().any(|id| id == "B"));
    }

    #[test]
    fn parse_sequence_notes() {
        let input = "sequenceDiagram\nparticipant Alice\nparticipant Bob\nAlice->>Bob: Hello\nNote over Alice,Bob: ping\nBob-->>Alice: Hi\nNote right of Bob: done";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.sequence_notes.len(), 2);
        let first = &parsed.graph.sequence_notes[0];
        assert_eq!(first.index, 1);
        assert_eq!(first.label, "ping");
        assert_eq!(first.position, crate::ir::SequenceNotePosition::Over);
        let second = &parsed.graph.sequence_notes[1];
        assert_eq!(second.index, 2);
        assert_eq!(second.label, "done");
        assert_eq!(second.position, crate::ir::SequenceNotePosition::RightOf);
    }

    #[test]
    fn parse_multiple_classes() {
        let input =
            "flowchart LR\nclassDef hot fill:#f00\nclassDef cold fill:#00f\nA\nclass A hot,cold";
        let parsed = parse_mermaid(input).unwrap();
        let classes = parsed.graph.node_classes.get("A").unwrap();
        assert!(classes.iter().any(|c| c == "hot"));
        assert!(classes.iter().any(|c| c == "cold"));
    }

    #[test]
    fn parse_node_id_with_dot() {
        let input = "flowchart LR\nsvc.api[Service] --> db.main[(DB)]";
        let parsed = parse_mermaid(input).unwrap();
        assert!(parsed.graph.nodes.contains_key("svc.api"));
        assert!(parsed.graph.nodes.contains_key("db.main"));
    }

    #[test]
    fn parse_init_with_single_quotes() {
        let input = "%%{init: {'themeVariables': {'primaryColor': '#fff'}}}%%\nflowchart LR\nA-->B";
        let parsed = parse_mermaid(input).unwrap();
        assert!(parsed.init_config.is_some());
    }

    #[test]
    fn parses_click_directive() {
        let input = "flowchart LR\nA-->B\nclick A \"https://example.com\"";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.nodes.len(), 2);
        assert_eq!(parsed.graph.edges.len(), 1);
        let link = parsed.graph.node_links.get("A").unwrap();
        assert_eq!(link.url, "https://example.com");
        assert!(link.title.is_none());
        assert!(link.target.is_none());
    }

    #[test]
    fn strips_inline_comments() {
        let input = "flowchart LR\nA-->B %% comment\nB-->C";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 2);
    }

    #[test]
    fn parse_link_style_whitespace_indexes() {
        let input = "flowchart LR\nA-->B\nB-->C\nlinkStyle 0 1 stroke:#0f0";
        let parsed = parse_mermaid(input).unwrap();
        assert!(parsed.graph.edge_styles.contains_key(&0));
        assert!(parsed.graph.edge_styles.contains_key(&1));
    }
}
