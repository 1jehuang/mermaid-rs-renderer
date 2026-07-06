#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::manual_strip)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::redundant_locals)]
#![allow(clippy::manual_clamp)]
#![allow(clippy::question_mark)]
#![allow(clippy::if_same_then_else)]

//! # mmdr - Fast Mermaid Diagram Renderer
//!
//! A pure Rust implementation of Mermaid diagram rendering, providing 100-600x
//! faster rendering than mermaid-cli by eliminating browser dependencies.
//!
//! ## Quick Start
//!
//! ```rust
//! use mermaid_rs_renderer::{render, render_with_options, RenderOptions};
//!
//! let diagram = r#"
//! flowchart LR
//!     A[Start] --> B{Decision}
//!     B -->|Yes| C[OK]
//!     B -->|No| D[Cancel]
//! "#;
//!
//! // Simple one-liner
//! let svg = render(diagram).unwrap();
//!
//! // With options
//! let svg = render_with_options(diagram, RenderOptions::default()).unwrap();
//! ```
//!
//! ## Pipeline Control
//!
//! For more control over the rendering pipeline, use the individual stages:
//!
//! ```rust
//! use mermaid_rs_renderer::{parse_mermaid, compute_layout, render_svg};
//! use mermaid_rs_renderer::{Theme, LayoutConfig};
//!
//! let diagram = "flowchart LR; A-->B-->C";
//!
//! // Stage 1: Parse
//! let parsed = parse_mermaid(diagram).unwrap();
//!
//! // Stage 2: Layout
//! let theme = Theme::modern();
//! let config = LayoutConfig::default();
//! let layout = compute_layout(&parsed.graph, &theme, &config);
//!
//! // Stage 3: Render
//! let svg = render_svg(&layout, &theme, &config);
//! ```
//!
//! ## Supported Diagram Types
//!
//! - **Flowcharts** (`flowchart` / `graph`): TD, TB, LR, RL, BT directions
//! - **Sequence Diagrams** (`sequenceDiagram`)
//! - **Class Diagrams** (`classDiagram`)
//! - **State Diagrams** (`stateDiagram-v2`)
//! - **ER Diagrams** (`erDiagram`)
//! - **Pie Charts** (`pie`)
//! - **XY Charts** (`xychart`)
//! - **Quadrant Charts** (`quadrantChart`)
//! - **Gantt** (`gantt`)
//! - **Timeline** (`timeline`)
//! - **Journey** (`journey`)
//! - **Mindmap** (`mindmap`)
//! - **Git Graph** (`gitGraph`)
//!
//! ## Features
//!
//! - Pure Rust, no browser or Node.js required
//! - ~3ms cold start vs ~2500ms for mermaid-cli
//! - ~15MB memory vs ~300MB for mermaid-cli
//! - SVG and PNG output (PNG via resvg)
//! - Customizable themes and layout configuration
//!
//! ## Cargo Features
//!
//! - **`cli`** (default) - CLI binary support. Disable for library-only usage.
//! - **`png`** (default) - PNG output via resvg. Disable for SVG-only usage.
//!
//! For minimal dependencies (e.g., embedding in other tools like Zola):
//!
//! ```toml
//! [dependencies]
//! mermaid-rs-renderer = { version = "0.2.2", default-features = false }
//! ```

#[cfg(feature = "cli")]
pub mod cli;
pub mod config;
mod edge_geometry;
pub mod embedded_font;
pub mod error;
pub mod ir;
pub mod layout;
pub mod layout_dump;
pub mod parser;
pub mod render;
mod text_metrics;
pub mod theme;
pub(crate) mod unicode_width;
pub mod validator;

// Re-export commonly used types at crate root for ergonomic library usage
pub use config::{Config, LayoutConfig, RenderConfig};
pub use error::ParseError;
pub use ir::{
    DiagramKind, Direction, Edge, EdgeArrowhead, EdgeDecoration, EdgeStyle, Graph, Node, NodeLink,
    NodeShape, SequenceActivation, SequenceActivationKind, SequenceBox, StateNote,
    StateNotePosition, Subgraph,
};
pub use layout::{
    EdgeLayout, Layout, LayoutStageMetrics, NodeLayout, SubgraphLayout, compute_layout,
    compute_layout_with_metrics,
};
pub use parser::{ParseOutput, parse_mermaid};
#[cfg(feature = "png")]
pub use render::write_output_png;
pub use render::{SvgDimensions, measure_svg_dimensions, render_svg, write_output_svg};
pub use theme::Theme;

/// Options for the high-level `render` function.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Theme to use for colors and styling.
    pub theme: Theme,
    /// Layout configuration (spacing, etc.).
    pub layout: LayoutConfig,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            theme: Theme::modern(),
            layout: LayoutConfig::default(),
        }
    }
}

impl RenderOptions {
    /// Create options with the modern theme (default).
    pub fn modern() -> Self {
        Self::default()
    }

    /// Create options with the classic Mermaid theme.
    pub fn mermaid_default() -> Self {
        Self {
            theme: Theme::mermaid_default(),
            layout: LayoutConfig::default(),
        }
    }

    /// Set custom node spacing.
    pub fn with_node_spacing(mut self, spacing: f32) -> Self {
        self.layout.node_spacing = spacing;
        self
    }

    /// Set custom rank spacing (vertical/horizontal gap between ranks).
    pub fn with_rank_spacing(mut self, spacing: f32) -> Self {
        self.layout.rank_spacing = spacing;
        self
    }

    /// Hint the renderer to target a preferred output aspect ratio (`width / height`).
    ///
    /// Invalid values (non-finite or `<= 0`) are ignored.
    pub fn with_preferred_aspect_ratio(mut self, ratio: f32) -> Self {
        if ratio.is_finite() && ratio > 0.0 {
            self.layout.preferred_aspect_ratio = Some(ratio);
        }
        self
    }

    /// Hint the renderer to target a preferred output aspect ratio from
    /// explicit width and height parts (e.g. `16` and `9`).
    pub fn with_preferred_aspect_ratio_parts(mut self, width: f32, height: f32) -> Self {
        if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 {
            self.layout.preferred_aspect_ratio = Some(width / height);
        }
        self
    }
}

/// Render a Mermaid diagram to SVG with default options.
///
/// This is the simplest way to render a diagram. For more control,
/// use [`render_with_options`] or the individual pipeline functions.
///
/// # Example
///
/// ```rust
/// use mermaid_rs_renderer::render;
///
/// let svg = render("flowchart LR; A-->B-->C").unwrap();
/// assert!(svg.contains("<svg"));
/// ```
///
/// # Errors
///
/// Returns an error if the diagram syntax is invalid.
pub fn render(input: &str) -> anyhow::Result<String> {
    render_with_options(input, RenderOptions::default())
}

/// Render a Mermaid diagram to SVG with custom options.
///
/// # Example
///
/// ```rust
/// use mermaid_rs_renderer::{render_with_options, RenderOptions};
///
/// let opts = RenderOptions::mermaid_default()
///     .with_node_spacing(60.0)
///     .with_rank_spacing(80.0);
///
/// let svg = render_with_options("flowchart LR; A-->B", opts).unwrap();
/// ```
pub fn render_with_options(input: &str, options: RenderOptions) -> anyhow::Result<String> {
    let parsed = parse_mermaid_strict(input)?;
    let layout = compute_layout(&parsed.graph, &options.theme, &options.layout);
    let svg = render_svg(&layout, &options.theme, &options.layout);
    Ok(svg)
}

/// Measure the SVG dimensions Mermaid would render with the given options.
///
/// This runs parse and layout, but does not allocate or serialize the full SVG.
pub fn measure(input: &str, options: RenderOptions) -> anyhow::Result<SvgDimensions> {
    measure_with_dimensions(input, options, None)
}

/// Measure dimensions with an optional explicit output image size override.
///
/// Passing `Some((width, height))` mirrors [`render::render_svg_with_dimensions`]
/// and the `mmdr -w/-H` PNG path: `width`/`height` in the returned metadata are
/// the final output canvas, while `viewbox_*` still describe the natural diagram
/// coordinate system.
pub fn measure_with_dimensions(
    input: &str,
    options: RenderOptions,
    dimensions: Option<(f32, f32)>,
) -> anyhow::Result<SvgDimensions> {
    let parsed = parse_mermaid_strict(input)?;
    let layout = compute_layout(&parsed.graph, &options.theme, &options.layout);
    Ok(measure_svg_dimensions(&layout, &options.layout, dimensions))
}

/// Parse a Mermaid diagram with typed parser diagnostics.
///
/// This is the structured-error counterpart to [`parse_mermaid`]. It runs a
/// preflight validation pass that catches common malformed-input cases and
/// reports them as [`ParseError`] variants, then delegates to the existing
/// parser for valid input.
pub fn parse_mermaid_strict(input: &str) -> Result<ParseOutput, ParseError> {
    validator::validate(input)?;
    parse_mermaid(input).map_err(|err| ParseError::UnexpectedToken {
        line: 1,
        col: 1,
        found: input
            .lines()
            .find(|line| !line.trim().is_empty())
            .map(str::trim)
            .unwrap_or("<empty>")
            .to_string(),
        expected: err.to_string(),
    })
}

/// Render a Mermaid diagram to SVG with typed parser diagnostics.
pub fn render_strict(input: &str, options: RenderOptions) -> Result<String, ParseError> {
    let parsed = parse_mermaid_strict(input)?;
    let layout = compute_layout(&parsed.graph, &options.theme, &options.layout);
    Ok(render_svg(&layout, &options.theme, &options.layout))
}

/// Result of rendering with timing information.
#[derive(Debug, Clone)]
pub struct RenderResult {
    /// The rendered SVG string.
    pub svg: String,
    /// Time spent parsing (microseconds).
    pub parse_us: u128,
    /// Time spent computing layout (microseconds).
    pub layout_us: u128,
    /// Time spent rendering to SVG (microseconds).
    pub render_us: u128,
}

impl RenderResult {
    /// Total render time in microseconds.
    pub fn total_us(&self) -> u128 {
        self.parse_us + self.layout_us + self.render_us
    }

    /// Total render time in milliseconds.
    pub fn total_ms(&self) -> f64 {
        self.total_us() as f64 / 1000.0
    }
}

/// Result of rendering with detailed layout stage timing information.
#[derive(Debug, Clone)]
pub struct RenderDetailedResult {
    /// The rendered SVG string.
    pub svg: String,
    /// Time spent parsing (microseconds).
    pub parse_us: u128,
    /// Time spent computing layout (microseconds).
    pub layout_us: u128,
    /// Time spent rendering to SVG (microseconds).
    pub render_us: u128,
    /// Fine-grained layout stage timings (microseconds).
    pub layout_stages: LayoutStageMetrics,
}

impl RenderDetailedResult {
    /// Total render time in microseconds.
    pub fn total_us(&self) -> u128 {
        self.parse_us + self.layout_us + self.render_us
    }

    /// Total render time in milliseconds.
    pub fn total_ms(&self) -> f64 {
        self.total_us() as f64 / 1000.0
    }
}

/// Render a Mermaid diagram to SVG with timing information.
///
/// Useful for benchmarking and profiling.
///
/// # Example
///
/// ```rust
/// use mermaid_rs_renderer::{render_with_timing, RenderOptions};
///
/// let result = render_with_timing("flowchart LR; A-->B", RenderOptions::default()).unwrap();
/// println!("Rendered in {:.2}ms", result.total_ms());
/// println!("  Parse:  {}us", result.parse_us);
/// println!("  Layout: {}us", result.layout_us);
/// println!("  Render: {}us", result.render_us);
/// ```
pub fn render_with_timing(input: &str, options: RenderOptions) -> anyhow::Result<RenderResult> {
    let detailed = render_with_detailed_timing(input, options)?;
    Ok(RenderResult {
        svg: detailed.svg,
        parse_us: detailed.parse_us,
        layout_us: detailed.layout_us,
        render_us: detailed.render_us,
    })
}

/// Render a Mermaid diagram to SVG with detailed timing information.
///
/// Includes a stage-level timing breakdown for layout (port assignment, edge
/// routing, label placement) to support architecture-level profiling.
pub fn render_with_detailed_timing(
    input: &str,
    options: RenderOptions,
) -> anyhow::Result<RenderDetailedResult> {
    use std::time::Instant;

    let t0 = Instant::now();
    let parsed = parse_mermaid(input)?;
    let parse_us = t0.elapsed().as_micros();

    let t1 = Instant::now();
    let (layout, layout_stages) =
        compute_layout_with_metrics(&parsed.graph, &options.theme, &options.layout);
    let layout_us = t1.elapsed().as_micros();

    let t2 = Instant::now();
    let svg = render_svg(&layout, &options.theme, &options.layout);
    let render_us = t2.elapsed().as_micros();

    Ok(RenderDetailedResult {
        svg,
        parse_us,
        layout_us,
        render_us,
        layout_stages,
    })
}

// Re-export cli::run for the binary
#[cfg(feature = "cli")]
pub use cli::run;

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_svg_attr(svg: &str, attr: &str) -> Option<f32> {
        let marker = format!("{attr}=\"");
        let start = svg.find(&marker)? + marker.len();
        let end = svg[start..].find('"')? + start;
        svg[start..end].parse::<f32>().ok()
    }

    fn parse_viewbox_ratio(svg: &str) -> Option<f32> {
        let marker = "viewBox=\"";
        let start = svg.find(marker)? + marker.len();
        let end = svg[start..].find('"')? + start;
        let parts: Vec<&str> = svg[start..end]
            .split(|ch: char| ch.is_ascii_whitespace() || ch == ',')
            .filter(|part| !part.is_empty())
            .collect();
        if parts.len() < 4 {
            return None;
        }
        let width = parts[2].parse::<f32>().ok()?;
        let height = parts[3].parse::<f32>().ok()?;
        if width <= 0.0 || height <= 0.0 {
            return None;
        }
        Some(width / height)
    }

    #[test]
    fn test_render_simple() {
        let svg = render("flowchart LR; A-->B").unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_render_with_options() {
        let opts = RenderOptions::modern().with_node_spacing(100.0);
        let svg = render_with_options("flowchart TD; X-->Y", opts).unwrap();
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn test_measure_matches_rendered_svg_dimensions() {
        let input = "flowchart LR; A-->B-->C";
        let opts = RenderOptions::default().with_preferred_aspect_ratio_parts(16.0, 9.0);
        let measured = measure(input, opts.clone()).unwrap();
        let svg = render_with_options(input, opts).unwrap();
        let width = parse_svg_attr(&svg, "width").expect("width");
        let height = parse_svg_attr(&svg, "height").expect("height");

        assert!((measured.width - width).abs() < 0.001);
        assert!((measured.height - height).abs() < 0.001);
        assert!((measured.aspect_ratio() - (16.0 / 9.0)).abs() < 0.001);
    }

    #[test]
    fn test_measure_with_dimensions_reports_output_canvas_and_viewbox() {
        let measured = measure_with_dimensions(
            "flowchart TD; A-->B",
            RenderOptions::default(),
            Some((1400.0, 900.0)),
        )
        .unwrap();

        assert_eq!(measured.width, 1400.0);
        assert_eq!(measured.height, 900.0);
        assert!(measured.viewbox_width > 0.0);
        assert!(measured.viewbox_height > 0.0);
    }

    #[test]
    fn test_render_with_timing() {
        let result =
            render_with_timing("flowchart LR; A-->B-->C", RenderOptions::default()).unwrap();
        assert!(result.svg.contains("<svg"));
        assert!(result.total_us() > 0);
    }

    #[test]
    fn test_class_diagram() {
        let svg = render(
            r#"classDiagram
            Animal <|-- Duck
            Animal: +int age
            Duck: +swim()"#,
        )
        .unwrap();
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn test_sequence_diagram() {
        let svg = render(
            r#"sequenceDiagram
            Alice->>Bob: Hello
            Bob-->>Alice: Hi"#,
        )
        .unwrap();
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn test_state_diagram() {
        let svg = render(
            r#"stateDiagram-v2
            [*] --> Active
            Active --> [*]"#,
        )
        .unwrap();
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn test_pie_diagram() {
        let svg = render(
            r#"pie showData
            title Pets
            "Dogs" : 10
            Cats : 5"#,
        )
        .unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("Dogs"));
        assert!(!svg.contains("Syntax error in text"));
    }

    #[test]
    fn test_preferred_aspect_ratio_applies_to_svg_dimensions() {
        let opts = RenderOptions::default().with_preferred_aspect_ratio_parts(16.0, 9.0);
        let svg = render_with_options("flowchart LR; A-->B-->C", opts).unwrap();
        let width = parse_svg_attr(&svg, "width").expect("width");
        let height = parse_svg_attr(&svg, "height").expect("height");
        let ratio = width / height;
        assert!((ratio - (16.0 / 9.0)).abs() < 0.001);
    }

    #[test]
    fn test_preferred_aspect_ratio_rebalances_viewbox_layout() {
        let input = "flowchart LR; A-->B-->C-->D-->E";
        let base_svg = render(input).unwrap();
        let base_ratio = parse_viewbox_ratio(&base_svg).expect("base viewBox ratio");

        let target_ratio = 1.0;
        let opts = RenderOptions::default().with_preferred_aspect_ratio(target_ratio);
        let tuned_svg = render_with_options(input, opts).unwrap();
        let tuned_ratio = parse_viewbox_ratio(&tuned_svg).expect("tuned viewBox ratio");

        assert!(
            (tuned_ratio - target_ratio).abs() + 0.01 < (base_ratio - target_ratio).abs(),
            "expected preferred ratio to move viewBox ratio toward target (base={base_ratio:.3}, tuned={tuned_ratio:.3})"
        );
        assert!(
            (tuned_ratio - target_ratio).abs() < 0.05,
            "expected preferred ratio to closely match target for simple flowcharts (target={target_ratio:.3}, got={tuned_ratio:.3})"
        );
    }

    #[test]
    fn test_preferred_aspect_ratio_handles_tall_targets() {
        let input = "flowchart LR; A-->B-->C-->D-->E";
        let target_ratio = 9.0 / 16.0;
        let opts = RenderOptions::default().with_preferred_aspect_ratio(target_ratio);
        let tuned_svg = render_with_options(input, opts).unwrap();
        let tuned_ratio = parse_viewbox_ratio(&tuned_svg).expect("tuned viewBox ratio");
        assert!(
            (tuned_ratio - target_ratio).abs() < 0.05,
            "expected tall preferred ratio to be respected (target={target_ratio:.3}, got={tuned_ratio:.3})"
        );
    }
}
