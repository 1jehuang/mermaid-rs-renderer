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
        r"^(?P<left>.+?)\s*(?P<arrow1><?[-.=ox]*[-=]+[-.=ox]*)\|(?P<label>.+?)\|(?P<arrow2>[-.=ox]*[-=]+[-.=ox]*>?)\s*(?P<right>.+)$",
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

            if line.starts_with("click ")
                || line.starts_with("link ")
                || line.starts_with("accTitle")
                || line.starts_with("accDescr")
                || line.starts_with("title ")
            {
                continue;
            }

            if let Some((left, label, right, edge_meta)) = parse_edge_line(&line) {
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
                    apply_node_classes(&mut graph, &left_id, &left_classes);
                    add_node_to_subgraphs(&mut graph, &subgraph_stack, &left_id);
                    source_ids.push(left_id);
                }

                let mut target_ids = Vec::new();
                for target in targets {
                    let (right_id, right_label, right_shape, right_classes) =
                        parse_node_token(target);
                    graph.ensure_node(&right_id, right_label, right_shape);
                    apply_node_classes(&mut graph, &right_id, &right_classes);
                    add_node_to_subgraphs(&mut graph, &subgraph_stack, &right_id);
                    target_ids.push(right_id);
                }

                for left_id in &source_ids {
                    for right_id in &target_ids {
                        graph.edges.push(crate::ir::Edge {
                            from: left_id.clone(),
                            to: right_id.clone(),
                            label: label.clone(),
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

            if let Some((node_id, node_label, node_shape, node_classes)) = parse_node_only(&line) {
                graph.ensure_node(&node_id, node_label, node_shape);
                apply_node_classes(&mut graph, &node_id, &node_classes);
                add_node_to_subgraphs(&mut graph, &subgraph_stack, &node_id);
            }
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_class_relation_line(line: &str) -> Option<(String, String, EdgeMeta, Option<String>)> {
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
            let meta = edge_meta_from_class_token(token);
            return Some((left.to_string(), right.to_string(), meta, label));
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

fn parse_state_alias_line(line: &str) -> Option<(String, String)> {
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
    if id.is_empty() {
        return None;
    }
    Some((id.to_string(), label))
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

fn parse_state_simple(line: &str) -> Option<String> {
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
    Some(strip_quotes(rest))
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

fn parse_sequence_participant(line: &str) -> Option<(String, Option<String>)> {
    let lowered = line.to_ascii_lowercase();
    let keywords = ["participant ", "actor ", "entity "];
    let mut rest = None;
    for keyword in keywords {
        if lowered.starts_with(keyword) {
            rest = Some(line[keyword.len()..].trim());
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
        return Some((id_part.to_string(), Some(label)));
    }

    if rest.starts_with('"') && rest.ends_with('"') {
        let label = strip_quotes(rest);
        return Some((label.clone(), Some(label)));
    }

    Some((strip_quotes(rest), None))
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

        if let Some((left, right, meta, label)) = parse_class_relation_line(line) {
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
                    },
                );
            }
        };
    while let Some(raw_line) = pending.pop_front() {
        for raw_statement in split_statements(&raw_line) {
            let line = raw_statement.trim();
            if line.is_empty() {
                continue;
            }
            let lower = line.to_ascii_lowercase();
            if lower.starts_with("statediagram") {
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

            if let Some((id, label)) = parse_state_alias_line(line) {
                labels.insert(id.clone(), label);
                graph.ensure_node(
                    &id,
                    labels.get(&id).cloned(),
                    Some(crate::ir::NodeShape::RoundRect),
                );
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
                let (left_id, left_shape, left_label_override) =
                    normalize_state_token(&left, true, &mut start_counter, &mut end_states, &scope);
                let (right_id, right_shape, right_label_override) = normalize_state_token(
                    &right,
                    false,
                    &mut start_counter,
                    &mut end_states,
                    &scope,
                );

                let left_label = left_label_override.or_else(|| labels.get(&left_id).cloned());
                let right_label = right_label_override.or_else(|| labels.get(&right_id).cloned());
                graph.ensure_node(&left_id, left_label, Some(left_shape));
                graph.ensure_node(&right_id, right_label, Some(right_shape));
                add_node_to_subgraphs(&mut graph, &subgraph_stack, &left_id);
                add_node_to_subgraphs(&mut graph, &subgraph_stack, &right_id);
                record_region_node(&mut composite_stack, &left_id);
                record_region_node(&mut composite_stack, &right_id);
                graph.edges.push(crate::ir::Edge {
                    from: left_id,
                    to: right_id,
                    label,
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

            if let Some(id) = parse_state_simple(line) {
                graph.ensure_node(
                    &id,
                    labels.get(&id).cloned(),
                    Some(crate::ir::NodeShape::RoundRect),
                );
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

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("sequencediagram") {
            continue;
        }
        if let Some((id, label)) = parse_sequence_participant(line) {
            if !order.contains(&id) {
                order.push(id.clone());
            }
            if let Some(label) = label.clone() {
                labels.insert(id.clone(), label);
            }
            graph.ensure_node(
                &id,
                labels.get(&id).cloned(),
                Some(crate::ir::NodeShape::RoundRect),
            );
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
        {
            let (kind, offset) = if lower.starts_with("opt") {
                (crate::ir::SequenceFrameKind::Opt, 3)
            } else if lower.starts_with("loop") {
                (crate::ir::SequenceFrameKind::Loop, 4)
            } else if lower.starts_with("par") {
                (crate::ir::SequenceFrameKind::Par, 3)
            } else if lower.starts_with("rect") {
                (crate::ir::SequenceFrameKind::Rect, 4)
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

        if lower == "end" {
            if let Some(mut frame) = open_frames.pop() {
                let end_idx = graph.edges.len();
                if let Some(last) = frame.sections.last_mut() {
                    last.end_idx = end_idx;
                }
                frame.end_idx = end_idx;
                frames.push(frame);
            }
            continue;
        }

        if let Some((position, participants, label)) = parse_sequence_note(line) {
            for id in &participants {
                if !order.contains(id) {
                    order.push(id.clone());
                }
                graph.ensure_node(
                    id,
                    labels.get(id).cloned(),
                    Some(crate::ir::NodeShape::RoundRect),
                );
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
                graph.ensure_node(
                    &id,
                    labels.get(&id).cloned(),
                    Some(crate::ir::NodeShape::RoundRect),
                );
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
                graph.ensure_node(
                    &id,
                    labels.get(&id).cloned(),
                    Some(crate::ir::NodeShape::RoundRect),
                );
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
                if let Ok(start) = parts[1].parse::<usize>() {
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
            graph.ensure_node(
                &from,
                labels.get(&from).cloned(),
                Some(crate::ir::NodeShape::RoundRect),
            );
            graph.ensure_node(
                &to,
                labels.get(&to).cloned(),
                Some(crate::ir::NodeShape::RoundRect),
            );
            graph.edges.push(crate::ir::Edge {
                from,
                to,
                label,
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

fn parse_edge_line(line: &str) -> Option<(String, Option<String>, String, EdgeMeta)> {
    // Mask bracket content to prevent matching dashes inside labels like A[wi-fi]
    let masked = mask_bracket_content(line);

    // Helper to extract from original line using match positions from masked line
    let extract = |m: regex::Match| -> &str {
        &line[m.start()..m.end()]
    };

    if let Some(caps) = PIPE_LABEL_RE.captures(&masked) {
        let left_match = caps.name("left")?;
        let right_match = caps.name("right")?;
        let label_match = caps.name("label")?;
        let left = extract(left_match).trim();
        let right = extract(right_match).trim();
        let label_clean = extract(label_match).trim();
        if !label_clean.is_empty() && !left.is_empty() && !right.is_empty() {
            let arrow1 = caps.name("arrow1")?.as_str();
            let arrow2 = caps.name("arrow2")?.as_str();
            let arrow = format!("{}{}", arrow1, arrow2);
            let edge_meta = parse_edge_meta(&arrow);
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
    } else {
        trimmed.to_string()
    }
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
    fn ignores_click_directive() {
        let input = "flowchart LR\nA-->B\nclick A \"https://example.com\"";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.nodes.len(), 2);
        assert_eq!(parsed.graph.edges.len(), 1);
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
