//! The internal render pipeline (PRD §4.3 panic-safety + DoS node cap).
//!
//! We replicate the fork's `render_strict` pipeline
//! (`parse_mermaid_strict` → `compute_layout` → `render_svg`) rather than call
//! `render_strict` directly, because the DoS **node cap must be enforced AFTER
//! parse but BEFORE `compute_layout`** (PRD §4.3 DoS item 2 — "without entering
//! `compute_layout`"). `render_strict` is a single opaque call that gives no
//! pre-layout seam. The whole of this function is invoked inside the
//! `catch_unwind` closure in [`crate::abi`], so any layout/render/raster panic
//! still maps to status 99.

use mermaid_rs_renderer::{
    compute_layout, parse_mermaid_strict, render_svg, LayoutConfig, ParseError, RenderOptions,
};

use crate::status::MmdrStatus;
use crate::types::MmdrOptions;

/// Output format selector — realized at the ABI as two entry points, threaded
/// here so the rasterization step lives inside the same `catch_unwind` closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Svg,
    Png,
}

/// The result of one render attempt. `abi` turns this into an `MmdrResult`.
pub enum Outcome {
    /// Success — payload bytes (UTF-8 SVG, or binary PNG).
    Ok(Vec<u8>),
    /// A typed parser error (maps to status 1–4, with location).
    Parse(ParseError),
    /// A DoS bound tripped (node cap) — maps to status 9.
    TooLarge,
    /// A clean internal failure (e.g. rasterizing our own SVG failed). Maps to
    /// the same internal-error UX path as a caught panic (status 99) with a
    /// redacted payload — it is "internal error, not user input".
    Internal,
}

/// Decoded location/status for a `ParseError`, ready for `MmdrResult`.
pub struct MappedError {
    pub status: MmdrStatus,
    pub err_line: u32,
    pub err_col: u32,
    pub loc_valid: u32,
    pub message: String,
}

/// Map a fork `ParseError` onto the C status + the explicit `loc_valid` flag
/// (PRD §4.3 error model). A line-only error reports `err_col = 0` (column not
/// reported) with `loc_valid = 1`; the `#[non_exhaustive]` catch-all is the
/// generic parser-error fallback → status 1, `loc_valid = 0`.
pub fn map_parse_error(e: &ParseError) -> MappedError {
    let message = e.to_string();
    match e {
        ParseError::UnexpectedToken { line, col, .. } => MappedError {
            status: MmdrStatus::ErrUnexpectedToken,
            err_line: *line,
            err_col: *col,
            loc_valid: 1,
            message,
        },
        ParseError::UnknownParticipant { line, .. } => MappedError {
            status: MmdrStatus::ErrUnknownParticipant,
            err_line: *line,
            err_col: 0, // column not reported for this shape
            loc_valid: 1,
            message,
        },
        ParseError::UnclosedSubgraph { opened_at } => MappedError {
            status: MmdrStatus::ErrUnclosedSubgraph,
            err_line: *opened_at,
            err_col: 0,
            loc_valid: 1,
            message,
        },
        ParseError::InvalidDirective { line, col, .. } => MappedError {
            status: MmdrStatus::ErrInvalidDirective,
            err_line: *line,
            err_col: *col,
            loc_valid: 1,
            message,
        },
        // `ParseError` is `#[non_exhaustive]`: a future variant maps to the
        // generic parser-error fallback with no location (loc_valid = 0).
        _ => MappedError {
            status: MmdrStatus::ErrUnexpectedToken,
            err_line: 0,
            err_col: 0,
            loc_valid: 0,
            message,
        },
    }
}

/// Build the fork `RenderOptions` from the decoded C options.
fn render_options(opts: &MmdrOptions) -> RenderOptions {
    let theme = crate::color::build_theme(
        opts.base_theme,
        opts.color_override_mask,
        opts.background_rgba,
        opts.foreground_rgba,
        opts.accent_rgba,
    );
    let mut ropts = RenderOptions {
        theme,
        layout: LayoutConfig::default(),
    };
    if opts.node_spacing > 0.0 {
        ropts = ropts.with_node_spacing(opts.node_spacing);
    }
    if opts.rank_spacing > 0.0 {
        ropts = ropts.with_rank_spacing(opts.rank_spacing);
    }
    if opts.preferred_aspect_w > 0.0 && opts.preferred_aspect_h > 0.0 {
        ropts = ropts
            .with_preferred_aspect_ratio_parts(opts.preferred_aspect_w, opts.preferred_aspect_h);
    }
    ropts
}

/// Run the full pipeline for one format. MUST be called inside `catch_unwind`.
///
/// Order (PRD §4.3): parse → **node cap (pre-layout)** → layout → render →
/// (PNG: rasterize). The diagram-type allowlist + source-size cap are enforced
/// by `abi` BEFORE this is reached.
pub fn run(input: &str, opts: &MmdrOptions, format: Format) -> Outcome {
    let ropts = render_options(opts);

    let parsed = match parse_mermaid_strict(input) {
        Ok(p) => p,
        Err(e) => return Outcome::Parse(e),
    };

    // DoS node cap — counted after parse, before layout (item 2).
    if parsed.graph.nodes.len() > opts.effective_max_nodes() {
        return Outcome::TooLarge;
    }

    let layout = compute_layout(&parsed.graph, &ropts.theme, &ropts.layout);
    let svg = render_svg(&layout, &ropts.theme, &ropts.layout);

    match format {
        Format::Svg => Outcome::Ok(svg.into_bytes()),
        Format::Png => match crate::raster::rasterize_png(&svg, &ropts.theme) {
            Ok(bytes) => Outcome::Ok(bytes),
            Err(_) => Outcome::Internal,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> MmdrOptions {
        MmdrOptions::default()
    }

    #[test]
    fn valid_flowchart_renders_svg_bytes() {
        match run("flowchart LR\nA-->B", &opts(), Format::Svg) {
            Outcome::Ok(bytes) => {
                let s = String::from_utf8(bytes).unwrap();
                assert!(s.contains("<svg"));
            }
            other => panic!("expected Ok, got {:?}", OutcomeDbg(&other)),
        }
    }

    #[test]
    fn malformed_source_is_parse_error() {
        // An arrow with no source node is an UnexpectedToken.
        match run("flowchart LR\n--> B", &opts(), Format::Svg) {
            Outcome::Parse(e) => {
                let m = map_parse_error(&e);
                assert_eq!(m.status, MmdrStatus::ErrUnexpectedToken);
            }
            other => panic!("expected Parse, got {:?}", OutcomeDbg(&other)),
        }
    }

    #[test]
    fn node_cap_trips_before_layout() {
        let mut o = opts();
        o.max_nodes = 2;
        let src = "flowchart LR\nA-->B\nB-->C\nC-->D"; // 4 nodes > cap 2
        assert!(matches!(run(src, &o, Format::Svg), Outcome::TooLarge));
    }

    #[test]
    fn node_cap_default_allows_small_graphs() {
        // default cap = 5000; a 2-node graph passes.
        assert!(matches!(
            run("flowchart LR\nA-->B", &opts(), Format::Svg),
            Outcome::Ok(_)
        ));
    }

    #[test]
    fn valid_flowchart_renders_png_bytes() {
        match run("flowchart LR\nA-->B", &opts(), Format::Png) {
            Outcome::Ok(bytes) => {
                // PNG 8-byte magic.
                assert_eq!(
                    &bytes[..8],
                    &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
                );
            }
            other => panic!("expected Ok PNG, got {:?}", OutcomeDbg(&other)),
        }
    }

    #[test]
    fn unclosed_subgraph_maps_to_status_3() {
        let src = "flowchart LR\nsubgraph S\nA-->B";
        match run(src, &opts(), Format::Svg) {
            Outcome::Parse(e) => {
                let m = map_parse_error(&e);
                assert_eq!(m.status, MmdrStatus::ErrUnclosedSubgraph);
                assert_eq!(m.loc_valid, 1);
                assert!(m.err_line >= 1);
            }
            other => panic!("expected Parse, got {:?}", OutcomeDbg(&other)),
        }
    }

    // Small helper so panic messages can name the outcome variant.
    struct OutcomeDbg<'a>(&'a Outcome);
    impl std::fmt::Debug for OutcomeDbg<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self.0 {
                Outcome::Ok(b) => write!(f, "Ok({} bytes)", b.len()),
                Outcome::Parse(e) => write!(f, "Parse({e})"),
                Outcome::TooLarge => write!(f, "TooLarge"),
                Outcome::Internal => write!(f, "Internal"),
            }
        }
    }
}
