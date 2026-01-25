#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::manual_strip)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::redundant_locals)]
#![allow(clippy::manual_clamp)]
#![allow(clippy::question_mark)]

//! # mmdr - Fast Mermaid Diagram Renderer
//!
//! A pure Rust implementation of Mermaid diagram rendering, providing 500-1000x
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
//! - **Class Diagrams** (`classDiagram`)
//! - **State Diagrams** (`stateDiagram-v2`)
//! - **Sequence Diagrams** (`sequenceDiagram`)
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
//! mermaid-rs-renderer = { version = "0.1", default-features = false }
//! ```

#[cfg(feature = "cli")]
pub mod cli;
pub mod config;
pub mod ir;
pub mod layout;
pub mod layout_dump;
pub mod parser;
pub mod render;
pub mod theme;

// Re-export commonly used types at crate root for ergonomic library usage
pub use config::{Config, LayoutConfig, RenderConfig};
pub use ir::{
    DiagramKind, Direction, Edge, EdgeArrowhead, EdgeDecoration, EdgeStyle, Graph, Node, NodeShape,
    NodeLink, SequenceActivation, SequenceActivationKind, SequenceBox, StateNote, StateNotePosition,
    Subgraph,
};
pub use layout::{EdgeLayout, Layout, NodeLayout, SubgraphLayout, compute_layout};
pub use parser::{ParseOutput, parse_mermaid};
#[cfg(feature = "png")]
pub use render::write_output_png;
pub use render::{render_svg, write_output_svg};
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
    let parsed = parse_mermaid(input)?;
    let layout = compute_layout(&parsed.graph, &options.theme, &options.layout);
    let svg = render_svg(&layout, &options.theme, &options.layout);
    Ok(svg)
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
    use std::time::Instant;

    let t0 = Instant::now();
    let parsed = parse_mermaid(input)?;
    let parse_us = t0.elapsed().as_micros();

    let t1 = Instant::now();
    let layout = compute_layout(&parsed.graph, &options.theme, &options.layout);
    let layout_us = t1.elapsed().as_micros();

    let t2 = Instant::now();
    let svg = render_svg(&layout, &options.theme, &options.layout);
    let render_us = t2.elapsed().as_micros();

    Ok(RenderResult {
        svg,
        parse_us,
        layout_us,
        render_us,
    })
}

// Re-export cli::run for the binary
#[cfg(feature = "cli")]
pub use cli::run;

#[cfg(test)]
mod tests {
    use super::*;

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
}
