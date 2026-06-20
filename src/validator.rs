//! Preflight validator for mermaid source.
//!
//! Runs as a single-pass scan over the input lines before the
//! kind-specific parser takes over. Detects the highest-frequency
//! malformed-input shapes and reports them as typed [`ParseError`]
//! variants.
//!
//! Coverage is deliberately narrow (see f160b acceptance criteria
//! in `docs/error_tracking.md`): five starter detection paths on
//! the common authoring mistakes. The full per-diagram-type
//! validation surface is follow-up work.
//!
//! The validator is additive: a successful `validate` does not
//! guarantee a successful parse, it only rules out the five
//! specific failure modes below. Existing successful inputs parse
//! byte-identically after validation.

use crate::error::ParseError;

/// Run the preflight validation pass over `input` and return
/// `Ok(())` when none of the detected failure modes apply.
///
/// The five detection paths implemented here are:
///
/// 1. `%%{init: ... }%%` with unparseable JSON → [`ParseError::InvalidDirective`]
/// 2. Flowchart `subgraph` without a matching `end` → [`ParseError::UnclosedSubgraph`]
/// 3. Flowchart `end` with no open `subgraph` → [`ParseError::UnexpectedToken`]
/// 4. Any line beginning with an arrow operator → [`ParseError::UnexpectedToken`]
/// 5. `click NodeId "url"` with unbalanced quotes → [`ParseError::UnexpectedToken`]
///
/// Returns the first error encountered; does not attempt to
/// collect multiple.
///
/// # Errors
///
/// Returns [`ParseError`] on the first detected failure mode.
pub fn validate(input: &str) -> Result<(), ParseError> {
    let lines: Vec<&str> = input.lines().collect();

    // 1. %%{init: ...}%% directive JSON well-formedness.
    check_init_directive(&lines)?;

    // 2-3. Subgraph / end balance.
    if is_flowchart_diagram(&lines) {
        check_subgraph_balance(&lines)?;
    }

    // 4. Lines beginning with an arrow.
    check_leading_arrow(&lines)?;

    // 5. click directives with unbalanced quotes.
    check_click_quotes(&lines)?;

    // 6. Sequence-diagram arrows that reference a participant
    //    name never declared (only when the diagram declares at
    //    least one participant explicitly — otherwise mermaid's
    //    auto-creation semantics apply and no error is raised).
    check_sequence_participants(&lines)?;

    Ok(())
}

/// Path 1: validate `%%{init: {...}}%%` directive JSON.
///
/// The original parser silently drops directives that fail the
/// regex match; this check explicitly rejects a directive whose
/// opening `%%{` fence is present but whose JSON payload does
/// not parse, so authors get a diagnostic rather than a
/// mysteriously-ignored directive.
fn check_init_directive(lines: &[&str]) -> Result<(), ParseError> {
    for (idx, raw) in lines.iter().enumerate() {
        let line_no = u32_from_index(idx);
        let trimmed = raw.trim_start();
        // Character-column of the first non-whitespace, 1-based.
        let col = col_of_first_nonws(raw);

        let Some(rest) = trimmed.strip_prefix("%%{") else {
            continue;
        };
        // Must end with }%%; anything else is ill-formed.
        let Some(inside) = rest.trim_end().strip_suffix("}%%") else {
            return Err(ParseError::InvalidDirective {
                line: line_no,
                col,
                directive: "unknown".to_string(),
                reason: "missing closing '}%%' fence".to_string(),
            });
        };
        // Split directive name (e.g. "init") from its JSON body.
        let Some(colon) = inside.find(':') else {
            return Err(ParseError::InvalidDirective {
                line: line_no,
                col,
                directive: "unknown".to_string(),
                reason: "missing ':' between directive name and body".to_string(),
            });
        };
        let name = inside[..colon].trim();
        let body = inside[colon + 1..].trim();
        if name != "init" {
            // Unknown directive names are tolerated (parser may
            // ignore them silently). Only `init` is validated.
            continue;
        }
        if body.is_empty() {
            return Err(ParseError::InvalidDirective {
                line: line_no,
                col,
                directive: name.to_string(),
                reason: "empty body".to_string(),
            });
        }
        // Body should parse as JSON (mmdr's regex uses json5,
        // so we prefer json5 here too for consistency).
        if let Err(e) = json5::from_str::<serde_json::Value>(body) {
            return Err(ParseError::InvalidDirective {
                line: line_no,
                col,
                directive: name.to_string(),
                reason: format!("JSON parse error: {e}"),
            });
        }
    }
    Ok(())
}

/// Paths 2-3: subgraph / end balance.
///
/// Tracks a stack of `subgraph` opening-line numbers. An `end`
/// with an empty stack yields [`ParseError::UnexpectedToken`];
/// a non-empty stack at EOF yields [`ParseError::UnclosedSubgraph`]
fn is_flowchart_diagram(lines: &[&str]) -> bool {
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        return lower.starts_with("flowchart") || lower.starts_with("graph ");
    }
    false
}

/// with the line of the outermost unclosed opening.
fn check_subgraph_balance(lines: &[&str]) -> Result<(), ParseError> {
    let mut open_stack: Vec<u32> = Vec::new();
    for (idx, raw) in lines.iter().enumerate() {
        let line_no = u32_from_index(idx);
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }
        if is_subgraph_open(trimmed) {
            open_stack.push(line_no);
        } else if is_subgraph_close(trimmed) && open_stack.pop().is_none() {
            let col = col_of_first_nonws(raw);
            return Err(ParseError::UnexpectedToken {
                line: line_no,
                col,
                found: "end".to_string(),
                expected: "matching subgraph".to_string(),
            });
        }
    }
    if let Some(opened_at) = open_stack.first() {
        return Err(ParseError::UnclosedSubgraph {
            opened_at: *opened_at,
        });
    }
    Ok(())
}

/// Path 4: lines that begin with an arrow operator.
///
/// `--> X`, `---> X`, `==> X`, etc., with no source node before
/// the arrow, are illegal in every mmdr-supported diagram kind.
/// This catches accidental pastes or omitted source identifiers.
fn check_leading_arrow(lines: &[&str]) -> Result<(), ParseError> {
    for (idx, raw) in lines.iter().enumerate() {
        let line_no = u32_from_index(idx);
        let trimmed = raw.trim_start();
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }
        if starts_with_arrow(trimmed) {
            let col = col_of_first_nonws(raw);
            let found_token: String = trimmed.chars().take_while(|c| !c.is_whitespace()).collect();
            return Err(ParseError::UnexpectedToken {
                line: line_no,
                col,
                found: found_token,
                expected: "node identifier".to_string(),
            });
        }
    }
    Ok(())
}

/// Path 5: `click NodeId "url" ["tooltip"]` with unbalanced
/// double quotes.
fn check_click_quotes(lines: &[&str]) -> Result<(), ParseError> {
    for (idx, raw) in lines.iter().enumerate() {
        let line_no = u32_from_index(idx);
        let trimmed = raw.trim_start();
        if !trimmed.starts_with("click ") && !trimmed.starts_with("click\t") {
            continue;
        }
        let quote_count = trimmed.chars().filter(|c| *c == '"').count();
        if quote_count % 2 == 1 {
            // Column of the first unmatched quote is a
            // reasonable anchor.
            let leading_ws = raw.len() - trimmed.len();
            let quote_byte = trimmed.find('"').unwrap_or(0);
            let col = col_of_char_offset(raw, leading_ws + quote_byte)
                .unwrap_or_else(|| col_of_first_nonws(raw));
            return Err(ParseError::UnexpectedToken {
                line: line_no,
                col,
                found: "\"".to_string(),
                expected: "matching double quote".to_string(),
            });
        }
    }
    Ok(())
}

/// Path 6: sequence-diagram arrows that reference an undeclared
/// participant.
///
/// Mermaid normally auto-creates participants on first use. This
/// check only activates when the author has explicitly declared
/// at least one `participant` (or `actor`) in the diagram: in
/// that case it is very likely a typo when a subsequent arrow
/// references a name outside the declared set, and surfacing the
/// typo as [`ParseError::UnknownParticipant`] is far more useful
/// than silently auto-creating a second actor with the wrong
/// name.
fn check_sequence_participants(lines: &[&str]) -> Result<(), ParseError> {
    if !looks_like_sequence_diagram(lines) {
        return Ok(());
    }
    let declared = collect_declared_participants(lines);
    if declared.is_empty() {
        // No explicit declarations: auto-creation applies, no
        // error surface.
        return Ok(());
    }
    for (idx, raw) in lines.iter().enumerate() {
        let line_no = u32_from_index(idx);
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }
        if let Some((left, right)) = split_sequence_arrow(trimmed) {
            for name in [left.trim(), right.trim()] {
                // Skip empty (defensive) and metadata tokens.
                if name.is_empty() {
                    continue;
                }
                if !declared.iter().any(|d| d == name) {
                    let candidates = nearest_candidates(name, &declared);
                    return Err(ParseError::UnknownParticipant {
                        name: name.to_string(),
                        line: line_no,
                        candidates,
                    });
                }
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------

/// Convert a 0-based line index (as produced by `enumerate()` on
/// `input.lines()`) to the 1-based line number this module uses.
fn u32_from_index(idx: usize) -> u32 {
    u32::try_from(idx + 1).unwrap_or(u32::MAX)
}

/// 1-based character column of the first non-whitespace
/// character in `raw`, or `1` when the line is all whitespace
/// or empty.
fn col_of_first_nonws(raw: &str) -> u32 {
    let col = raw
        .char_indices()
        .find(|(_, c)| !c.is_whitespace())
        .map_or(0, |(i, _)| raw[..i].chars().count());
    u32::try_from(col + 1).unwrap_or(u32::MAX)
}

/// 1-based character column of the byte-offset `byte_offset`
/// within `raw`. Returns `None` when the offset does not fall on
/// a character boundary.
fn col_of_char_offset(raw: &str, byte_offset: usize) -> Option<u32> {
    if !raw.is_char_boundary(byte_offset) {
        return None;
    }
    let col = raw[..byte_offset].chars().count();
    Some(u32::try_from(col + 1).unwrap_or(u32::MAX))
}

/// True if the trimmed line opens a subgraph block
/// (case-insensitive `subgraph` keyword at line start).
fn is_subgraph_open(trimmed: &str) -> bool {
    let lower = trimmed.to_ascii_lowercase();
    lower == "subgraph" || lower.starts_with("subgraph ") || lower.starts_with("subgraph\t")
}

/// True if the trimmed line is exactly the `end` keyword
/// (closing a subgraph / alt / opt / loop block).
fn is_subgraph_close(trimmed: &str) -> bool {
    let lower = trimmed.to_ascii_lowercase();
    // The `end` keyword can be followed by a comment (`end %% close`).
    lower == "end"
        || lower.starts_with("end ")
        || lower.starts_with("end\t")
        || lower.starts_with("end%%")
}

/// True if `trimmed` begins with any arrow operator mmdr
/// recognises. Matches the pattern used by the library's own
/// `FLOW_EDGE_PATTERN` at a line start.
fn starts_with_arrow(trimmed: &str) -> bool {
    let bytes = trimmed.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    match bytes[0] {
        b'-' | b'=' | b'~' | b'.' | b'<' => {
            // Consume leading dashes / equals / tildes / dots,
            // and require something resembling an arrow head
            // within the run.
            bytes
                .iter()
                .copied()
                .take_while(|&b| matches!(b, b'-' | b'=' | b'~' | b'.' | b'<' | b'>' | b'o' | b'x'))
                .any(|b| matches!(b, b'>' | b'<' | b'-' | b'='))
                && bytes
                    .iter()
                    .copied()
                    .take(16)
                    .any(|b| matches!(b, b'>' | b'<'))
        }
        _ => false,
    }
}

/// True when the input's first non-empty, non-comment line
/// begins with `sequenceDiagram` (case-insensitive).
fn looks_like_sequence_diagram(lines: &[&str]) -> bool {
    for raw in lines {
        let t = raw.trim();
        if t.is_empty() || t.starts_with("%%") {
            continue;
        }
        return t.to_ascii_lowercase().starts_with("sequencediagram");
    }
    false
}

/// Extract the set of names declared via `participant NAME`,
/// `participant NAME as ALIAS`, or `actor NAME ...`.
///
/// Both the raw name and the alias (if present) are added, so an
/// arrow can refer to either form.
fn collect_declared_participants(lines: &[&str]) -> Vec<String> {
    let mut declared: Vec<String> = Vec::new();
    for raw in lines {
        let t = raw.trim();
        let (keyword, rest) = if let Some(r) = t.strip_prefix("participant ") {
            ("participant", r)
        } else if let Some(r) = t.strip_prefix("actor ") {
            ("actor", r)
        } else {
            continue;
        };
        let _ = keyword;
        // Supported shapes: "NAME", "NAME as ALIAS".
        let rest = rest.trim();
        if let Some((name, alias_part)) = rest.split_once(" as ") {
            let name = name.trim().to_string();
            let alias = alias_part.trim().to_string();
            if !name.is_empty() {
                declared.push(name);
            }
            if !alias.is_empty() {
                declared.push(alias);
            }
        } else if !rest.is_empty() {
            declared.push(rest.to_string());
        }
    }
    declared
}

/// If `trimmed` is a sequence-diagram arrow line, return the
/// source and target names. Otherwise `None`.
///
/// Matches the common arrow shapes: `->`, `-->`, `->>`, `-x`,
/// `--x`, `-)`, `--)`. Message text after a `:` is stripped.
fn split_sequence_arrow(trimmed: &str) -> Option<(&str, &str)> {
    // Strip any trailing `: message` payload.
    let before_colon = trimmed.split_once(':').map_or(trimmed, |(a, _)| a);
    // Longer patterns first so `-->` wins over `->`.
    const PATTERNS: &[&str] = &["-->>", "--x", "--)", "-->", "->>", "->", "-x", "-)"];
    for pat in PATTERNS {
        if let Some((lhs, rhs)) = before_colon.split_once(pat) {
            return Some((lhs, rhs));
        }
    }
    None
}

/// Return up to three declared names whose lowercase form shares
/// a prefix with `target` or differs only by case, as
/// "did-you-mean" suggestions. Deterministic order (source
/// order).
fn nearest_candidates(target: &str, declared: &[String]) -> Vec<String> {
    let target_lower = target.to_ascii_lowercase();
    declared
        .iter()
        .filter(|d| {
            let dl = d.to_ascii_lowercase();
            dl == target_lower || dl.starts_with(&target_lower) || target_lower.starts_with(&dl)
        })
        .take(3)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Path 1: init directive ---------------------------------------

    #[test]
    fn init_directive_valid_json_passes() {
        let input = r#"%%{init: {"theme": "dark"}}%%
flowchart LR
A-->B"#;
        assert!(validate(input).is_ok());
    }

    #[test]
    fn init_directive_invalid_json_is_reported() {
        let input = r#"%%{init: {theme dark}}%%
flowchart LR"#;
        let err = validate(input).unwrap_err();
        assert!(
            matches!(err, ParseError::InvalidDirective { directive, .. } if directive == "init")
        );
    }

    #[test]
    fn init_directive_missing_colon_is_reported() {
        let input = r#"%%{init}%%
flowchart LR"#;
        let err = validate(input).unwrap_err();
        assert!(matches!(err, ParseError::InvalidDirective { .. }));
    }

    #[test]
    fn init_directive_unknown_name_is_tolerated() {
        // Only `init` is validated; unknown directive names pass
        // through (original parser semantics).
        let input = r#"%%{customdirective: {"x": 1}}%%
flowchart LR"#;
        assert!(validate(input).is_ok());
    }

    // --- Paths 2-3: subgraph balance ---------------------------------

    #[test]
    fn subgraph_unclosed_is_reported() {
        let input = "flowchart LR\nsubgraph S\n  A --> B\n";
        let err = validate(input).unwrap_err();
        assert!(
            matches!(err, ParseError::UnclosedSubgraph { opened_at: 2 }),
            "got {err:?}"
        );
    }

    #[test]
    fn subgraph_balanced_passes() {
        let input = "flowchart LR\nsubgraph S\n  A --> B\nend\n";
        assert!(validate(input).is_ok());
    }

    #[test]
    fn nested_subgraphs_balanced_pass() {
        let input = "flowchart LR\nsubgraph O\n  subgraph I\n    A --> B\n  end\nend\n";
        assert!(validate(input).is_ok());
    }

    #[test]
    fn nested_subgraphs_inner_unclosed_is_reported() {
        let input = "flowchart LR\nsubgraph O\n  subgraph I\n    A --> B\nend\n";
        let err = validate(input).unwrap_err();
        assert!(matches!(err, ParseError::UnclosedSubgraph { .. }));
    }

    #[test]
    fn stray_end_without_open_is_reported() {
        let input = "flowchart LR\nA --> B\nend\n";
        let err = validate(input).unwrap_err();
        assert!(matches!(
            err,
            ParseError::UnexpectedToken { found, expected, .. }
                if found == "end" && expected == "matching subgraph"
        ));
    }

    // --- Path 4: leading arrow ----------------------------------------

    #[test]
    fn leading_arrow_is_reported() {
        let input = "flowchart LR\n--> B\n";
        let err = validate(input).unwrap_err();
        assert!(matches!(
            err,
            ParseError::UnexpectedToken { expected, .. }
                if expected == "node identifier"
        ));
    }

    #[test]
    fn leading_thick_arrow_is_reported() {
        let input = "flowchart LR\n==> B\n";
        let err = validate(input).unwrap_err();
        assert!(matches!(err, ParseError::UnexpectedToken { .. }));
    }

    #[test]
    fn regular_edge_passes() {
        let input = "flowchart LR\nA --> B\n";
        assert!(validate(input).is_ok());
    }

    // --- Path 5: click directive quoting ------------------------------

    #[test]
    fn click_unbalanced_quote_is_reported() {
        let input = "flowchart LR\nA --> B\nclick A \"https://example.com\n";
        let err = validate(input).unwrap_err();
        assert!(matches!(
            err,
            ParseError::UnexpectedToken { expected, .. }
                if expected == "matching double quote"
        ));
    }

    #[test]
    fn click_balanced_passes() {
        let input = "flowchart LR\nA --> B\nclick A \"https://example.com\"\n";
        assert!(validate(input).is_ok());
    }

    // --- Path 6: sequence unknown participant -------------------------

    #[test]
    fn sequence_without_declarations_passes() {
        // Auto-creation applies -- no error.
        let input = "sequenceDiagram\nAlice->>Bob: hi\n";
        assert!(validate(input).is_ok());
    }

    #[test]
    fn sequence_declared_participants_match_passes() {
        let input = "sequenceDiagram\nparticipant Alice\nparticipant Bob\nAlice->>Bob: hi\n";
        assert!(validate(input).is_ok());
    }

    #[test]
    fn sequence_unknown_participant_on_right_is_reported() {
        let input = "sequenceDiagram\nparticipant Alice\nparticipant Bob\nAlice->>Carol: hi\n";
        let err = validate(input).unwrap_err();
        assert!(
            matches!(err, ParseError::UnknownParticipant { ref name, line: 4, .. }
                if name == "Carol"),
            "got {err:?}"
        );
    }

    #[test]
    fn sequence_unknown_participant_on_left_is_reported() {
        let input = "sequenceDiagram\nparticipant Alice\nparticipant Bob\nCarol->>Bob: hi\n";
        let err = validate(input).unwrap_err();
        assert!(matches!(err, ParseError::UnknownParticipant { .. }));
    }

    #[test]
    fn sequence_participant_as_alias_is_honored() {
        // Either the raw name OR the alias satisfies the reference.
        let input = "sequenceDiagram\nparticipant A as Alice\nA->>A: hi\n";
        assert!(validate(input).is_ok());
    }

    // --- Edge cases ---------------------------------------------------

    #[test]
    fn empty_input_passes() {
        assert!(validate("").is_ok());
    }

    #[test]
    fn comment_only_input_passes() {
        assert!(validate("%% just a comment\n%% and another\n").is_ok());
    }
}
