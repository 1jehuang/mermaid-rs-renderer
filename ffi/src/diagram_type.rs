//! Wrapper-side diagram-type pre-dispatch.
//!
//! This is the PRIMARY status-5 (`ErrUnsupported`) gate for the native renderer
//! (PRD R3-C / §4.3): the FFI layer detects the diagram type from the source
//! and refuses to invoke the renderer for anything outside the MVP allowlist,
//! returning `ErrUnsupported` instead. This is cheap (no parse), keeps
//! unsupported types from reaching renderer code paths that may panic, and is
//! independent of the fork (which gains its own `Unsupported` variant only in
//! the gated Phase 10 upstreaming).
//!
//! The detection mirrors the fork's own `parser::detect_diagram_kind` (header
//! keyword after front-matter/comment stripping, case-insensitive, with a
//! flowchart-edge fallback) so the gate never diverges from what the renderer
//! actually accepts.

/// The 13 documented diagram types plus an `Unknown` catch-all for every other
/// keyword the fork recognizes (sankey, c4*, requirementDiagram, block, …) and
/// for unrecognized input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagramType {
    // --- MVP (render path) ---
    Flowchart, // includes `graph`
    Sequence,
    Class,
    StateDiagram, // includes `stateDiagram-v2`
    ErDiagram,
    Pie,
    Gantt,
    // --- documented, non-MVP (status 5 until the post-MVP expansion) ---
    XyChart, // includes `xychart-beta`
    QuadrantChart,
    Timeline,
    Journey,
    Mindmap,
    GitGraph,
    // --- everything else (status 5) ---
    Unknown(String),
}

impl DiagramType {
    /// The 7 MVP types that route to the render path.
    pub const MVP_TYPES: &'static [DiagramType] = &[
        DiagramType::Flowchart,
        DiagramType::Sequence,
        DiagramType::Class,
        DiagramType::StateDiagram,
        DiagramType::ErDiagram,
        DiagramType::Pie,
        DiagramType::Gantt,
    ];
}

/// `true` if `dtype` is one of the 7 MVP types (route to render); `false`
/// means the FFI layer returns `ErrUnsupported` (status 5).
pub fn is_mvp_supported(dtype: &DiagramType) -> bool {
    DiagramType::MVP_TYPES.iter().any(|t| t == dtype)
}

/// Detect the diagram type of a Mermaid source.
///
/// Mirrors the fork's `detect_diagram_kind`: skips blank lines, YAML
/// front-matter (`---` … `---`), and `%%` comment / `%%{ init }%%` directive
/// lines, strips a trailing `%%` comment, then matches the first significant
/// line's leading header keyword case-insensitively. A keyword-less line that
/// looks like flowchart edge syntax (`A --> B`) is treated as a flowchart, as
/// the renderer does.
pub fn detect_diagram_type(source: &str) -> DiagramType {
    let mut in_frontmatter = false;
    for raw_line in source.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "---" {
            in_frontmatter = !in_frontmatter;
            continue;
        }
        if in_frontmatter {
            continue;
        }
        // `%%{ ... }%%` init directives and `%%` comments.
        if trimmed.starts_with("%%") {
            continue;
        }
        let without_comment = strip_trailing_comment(trimmed);
        if without_comment.is_empty() {
            continue;
        }
        return classify_header(&without_comment.to_ascii_lowercase(), without_comment);
    }
    DiagramType::Unknown(String::new())
}

/// Classify a significant line (already comment-stripped). `lower` is the
/// ASCII-lowercased form; `original` is used to preserve the keyword in
/// `Unknown`.
fn classify_header(lower: &str, original: &str) -> DiagramType {
    // Order follows the fork so aliasing/prefix collisions resolve identically.
    if starts_with_header(lower, "sequencediagram") {
        return DiagramType::Sequence;
    }
    if starts_with_header(lower, "classdiagram") {
        return DiagramType::Class;
    }
    if starts_with_header(lower, "statediagram") {
        // covers `stateDiagram` and `stateDiagram-v2`
        return DiagramType::StateDiagram;
    }
    if starts_with_header(lower, "erdiagram") {
        return DiagramType::ErDiagram;
    }
    if starts_with_header(lower, "pie") {
        return DiagramType::Pie;
    }
    if starts_with_header(lower, "gantt") {
        return DiagramType::Gantt;
    }
    if starts_with_header(lower, "mindmap") {
        return DiagramType::Mindmap;
    }
    if starts_with_header(lower, "journey") {
        return DiagramType::Journey;
    }
    if starts_with_header(lower, "timeline") {
        return DiagramType::Timeline;
    }
    if starts_with_header(lower, "gitgraph") {
        return DiagramType::GitGraph;
    }
    if starts_with_header(lower, "quadrantchart") {
        return DiagramType::QuadrantChart;
    }
    if starts_with_header(lower, "xychart") {
        // covers `xychart-beta`
        return DiagramType::XyChart;
    }
    if starts_with_header(lower, "flowchart") || starts_with_header(lower, "graph") {
        return DiagramType::Flowchart;
    }
    if looks_like_flowchart_edge(lower) {
        return DiagramType::Flowchart;
    }
    // Recognized-but-unsupported (sankey, c4*, requirementDiagram, block, …)
    // and truly unknown keywords both gate to status 5.
    DiagramType::Unknown(first_token(original).to_string())
}

/// Mirror of the fork's `starts_with_diagram_header`: `line` begins with
/// `keyword` and the following char (if any) is not alphanumeric — so `pie`
/// matches `pie title …` but not `pies`.
fn starts_with_header(line: &str, keyword: &str) -> bool {
    match line.strip_prefix(keyword) {
        None => false,
        Some(rest) => rest
            .chars()
            .next()
            .is_none_or(|ch| !ch.is_ascii_alphanumeric()),
    }
}

/// Cheap flowchart-edge heuristic for keyword-less input (e.g. `A --> B`). The
/// fork uses a regex over bracket-masked text; for the allowlist a conservative
/// scan for the common arrow tokens is sufficient and avoids false rejects of
/// renderable flowcharts.
fn looks_like_flowchart_edge(lower: &str) -> bool {
    const ARROWS: &[&str] = &["-->", "---", "-.-", "==>", "===", "--x", "--o", "<-->"];
    ARROWS.iter().any(|a| lower.contains(a))
}

/// Strip a trailing `%%` comment from a significant line. The header keyword
/// always precedes any inline comment, so a plain cut at the first `%%` is
/// sufficient for detection.
fn strip_trailing_comment(line: &str) -> &str {
    match line.find("%%") {
        Some(idx) => line[..idx].trim_end(),
        None => line,
    }
}

/// First whitespace-delimited token of a line.
fn first_token(line: &str) -> &str {
    line.split_whitespace().next().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detect(src: &str) -> DiagramType {
        detect_diagram_type(src)
    }

    // (a) all 13 documented keywords detected correctly
    #[test]
    fn detects_all_documented_keywords() {
        assert_eq!(detect("flowchart LR\nA-->B"), DiagramType::Flowchart);
        assert_eq!(detect("graph TD\nA-->B"), DiagramType::Flowchart);
        assert_eq!(detect("sequenceDiagram\nA->>B: hi"), DiagramType::Sequence);
        assert_eq!(detect("classDiagram\nA <|-- B"), DiagramType::Class);
        assert_eq!(detect("stateDiagram\n[*] --> S"), DiagramType::StateDiagram);
        assert_eq!(
            detect("stateDiagram-v2\n[*] --> S"),
            DiagramType::StateDiagram
        );
        assert_eq!(detect("erDiagram\nA ||--o{ B : x"), DiagramType::ErDiagram);
        assert_eq!(detect("pie title P\n\"a\" : 1"), DiagramType::Pie);
        assert_eq!(detect("gantt\ntitle G"), DiagramType::Gantt);
        assert_eq!(detect("xychart-beta\n"), DiagramType::XyChart);
        assert_eq!(detect("quadrantChart\n"), DiagramType::QuadrantChart);
        assert_eq!(detect("timeline\ntitle T"), DiagramType::Timeline);
        assert_eq!(detect("journey\ntitle J"), DiagramType::Journey);
        assert_eq!(detect("mindmap\nroot"), DiagramType::Mindmap);
        assert_eq!(detect("gitGraph\ncommit"), DiagramType::GitGraph);
    }

    // (b) 7 MVP keywords route to the render path
    #[test]
    fn mvp_types_are_supported() {
        for src in [
            "flowchart LR\nA-->B",
            "sequenceDiagram\nA->>B: hi",
            "classDiagram\nA <|-- B",
            "stateDiagram-v2\n[*] --> S",
            "erDiagram\nA ||--o{ B : x",
            "pie\n\"a\" : 1",
            "gantt\ntitle G",
        ] {
            assert!(is_mvp_supported(&detect(src)), "should be MVP: {src:?}");
        }
    }

    // (c) 6 non-MVP keywords gate to status 5
    #[test]
    fn non_mvp_documented_types_are_unsupported() {
        for src in [
            "xychart-beta\n",
            "quadrantChart\n",
            "timeline\ntitle T",
            "journey\ntitle J",
            "mindmap\nroot",
            "gitGraph\ncommit",
        ] {
            let t = detect(src);
            assert!(
                !is_mvp_supported(&t),
                "should be unsupported: {src:?} -> {t:?}"
            );
        }
    }

    // (d) unsupported keywords (sankey, C4Context) gate to status 5 as Unknown
    #[test]
    fn unsupported_keywords_are_unknown() {
        assert!(matches!(detect("sankey-beta\n"), DiagramType::Unknown(_)));
        assert!(matches!(
            detect("C4Context\ntitle C"),
            DiagramType::Unknown(_)
        ));
        assert!(matches!(
            detect("requirementDiagram\n"),
            DiagramType::Unknown(_)
        ));
        assert!(matches!(detect("block-beta\n"), DiagramType::Unknown(_)));
        assert!(matches!(
            detect("totally-not-a-diagram\n"),
            DiagramType::Unknown(_)
        ));
        assert!(!is_mvp_supported(&detect("sankey-beta\n")));
        assert!(!is_mvp_supported(&detect("C4Context\n")));
    }

    // (e) %%{init}%% and front-matter prefixed inputs detect correctly
    #[test]
    fn handles_init_directive_and_frontmatter() {
        assert_eq!(
            detect("%%{init: {\"theme\":\"dark\"}}%%\nflowchart LR\nA-->B"),
            DiagramType::Flowchart
        );
        assert_eq!(
            detect(
                "---\ntitle: My Diagram\nconfig:\n  theme: forest\n---\nsequenceDiagram\nA->>B: hi"
            ),
            DiagramType::Sequence
        );
        // front-matter containing `---`-like keys must not be mistaken for content
        assert_eq!(
            detect("---\ntitle: pie of life\n---\ngantt\ntitle G"),
            DiagramType::Gantt
        );
        // comment lines before the header
        assert_eq!(
            detect("%% a leading comment\n%% another\nclassDiagram\nA <|-- B"),
            DiagramType::Class
        );
    }

    // (f) aliases resolve correctly
    #[test]
    fn aliases_resolve() {
        assert_eq!(detect("graph TD\nA-->B"), detect("flowchart TD\nA-->B"));
        assert_eq!(
            detect("stateDiagram\n[*]-->S"),
            detect("stateDiagram-v2\n[*]-->S")
        );
    }

    // (g) leading whitespace tolerated (+ case-insensitive)
    #[test]
    fn leading_whitespace_and_case_tolerated() {
        assert_eq!(
            detect("   \n\t flowchart LR\n A-->B"),
            DiagramType::Flowchart
        );
        assert_eq!(detect("SEQUENCEDIAGRAM\nA->>B: hi"), DiagramType::Sequence);
        assert_eq!(
            detect("  ErDiagram\nA ||--o{ B : x"),
            DiagramType::ErDiagram
        );
    }

    // keyword-less flowchart edge syntax is treated as a flowchart (renderer parity)
    #[test]
    fn keyword_less_edge_syntax_is_flowchart() {
        assert_eq!(detect("A --> B\nB --> C"), DiagramType::Flowchart);
        assert!(is_mvp_supported(&detect("A --> B")));
    }

    // empty / whitespace-only input is Unknown (not MVP)
    #[test]
    fn empty_input_is_unknown() {
        assert!(matches!(detect(""), DiagramType::Unknown(_)));
        assert!(matches!(detect("   \n\n\t"), DiagramType::Unknown(_)));
        assert!(!is_mvp_supported(&detect("")));
    }

    // `pie` prefix must not swallow a different word
    #[test]
    fn header_prefix_is_word_bounded() {
        // not a real type, but proves `pie` doesn't match `pieces`
        assert!(matches!(
            detect("pieces of eight\n"),
            DiagramType::Unknown(_)
        ));
    }
}
