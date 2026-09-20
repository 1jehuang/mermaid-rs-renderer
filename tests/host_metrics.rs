//! A host can measure labels with its own faces.
//!
//! The provider is optional and changes nothing when it declines a request,
//! so these guards cover both directions: an installed provider is consulted
//! and moves the layout, and one that answers `None` leaves the layout
//! exactly as the system font database measured it.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use mermaid_rs_renderer::config::LayoutConfig;
use mermaid_rs_renderer::layout::{Layout, compute_layout};
use mermaid_rs_renderer::metrics::TextMetrics;
use mermaid_rs_renderer::parser::parse_mermaid;
use mermaid_rs_renderer::render::render_svg;
use mermaid_rs_renderer::theme::Theme;

/// The width of the `PK` key badge's rectangle, which carries its own fill.
fn pk_badge_width(svg: &str) -> f32 {
    svg.split("<rect")
        .find(|element| element.contains("fill=\"#1D4ED8\""))
        .and_then(|element| element.split("width=\"").nth(1))
        .and_then(|width| width.split('"').next())
        .and_then(|width| width.parse().ok())
        .expect("a PK badge rectangle")
}

/// Measures every label four times as wide as a font database would.
#[derive(Debug, Default)]
struct Wider {
    calls: AtomicUsize,
}

impl TextMetrics for Wider {
    fn measure_text_width(&self, text: &str, font_size: f32, _font_family: &str) -> Option<f32> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Some(text.chars().count() as f32 * font_size * 4.0)
    }
}

/// Declines every request, so the renderer measures it itself.
#[derive(Debug, Default)]
struct Declines;

impl TextMetrics for Declines {
    fn measure_text_width(&self, _text: &str, _font_size: f32, _font_family: &str) -> Option<f32> {
        None
    }
}

fn layout(input: &str, config: &LayoutConfig) -> Layout {
    let parsed = parse_mermaid(input).expect("parse");
    compute_layout(&parsed.graph, &Theme::modern(), config)
}

fn node_widths(layout: &Layout) -> Vec<f32> {
    layout.nodes.values().map(|node| node.width).collect()
}

const CHART: &str = "flowchart TD\n    A[Start here] --> B[A longer label]\n";

#[test]
fn an_installed_provider_measures_the_labels() {
    let default = layout(CHART, &LayoutConfig::default());
    let wider = Arc::new(Wider::default());
    let config = LayoutConfig {
        metrics: Some(wider.clone()),
        ..LayoutConfig::default()
    };
    let measured = layout(CHART, &config);
    assert!(wider.calls.load(Ordering::Relaxed) > 0, "provider unused");
    assert_ne!(node_widths(&default), node_widths(&measured));
    assert!(
        node_widths(&measured)
            .iter()
            .zip(node_widths(&default))
            .all(|(measured, default)| *measured >= default),
        "a wider measurement must not shrink a node"
    );
}

#[test]
fn fast_metrics_still_ask_the_provider_first() {
    // `fast_text_metrics` is the renderer's own shortcut, so it only applies
    // where the host has nothing to say.
    let wider = Arc::new(Wider::default());
    let config = LayoutConfig {
        fast_text_metrics: true,
        metrics: Some(wider.clone()),
        ..LayoutConfig::default()
    };
    let measured = layout(CHART, &config);
    assert!(wider.calls.load(Ordering::Relaxed) > 0, "provider unused");
    let fast = layout(
        CHART,
        &LayoutConfig {
            fast_text_metrics: true,
            ..LayoutConfig::default()
        },
    );
    assert_ne!(node_widths(&fast), node_widths(&measured));
}

#[test]
fn a_declining_provider_leaves_the_default_measurement_alone() {
    for fast in [false, true] {
        let default = layout(
            CHART,
            &LayoutConfig {
                fast_text_metrics: fast,
                ..LayoutConfig::default()
            },
        );
        let declined = layout(
            CHART,
            &LayoutConfig {
                fast_text_metrics: fast,
                metrics: Some(Arc::new(Declines)),
                ..LayoutConfig::default()
            },
        );
        assert_eq!(node_widths(&default), node_widths(&declined), "fast={fast}");
        assert_eq!(default.width, declined.width, "fast={fast}");
        assert_eq!(default.height, declined.height, "fast={fast}");
    }
}

#[test]
fn er_key_badges_are_drawn_with_the_host_metrics() {
    // The badge's rectangle reserves space with the host's measurement, so it
    // must be drawn with the same one, or a wider face overflows its box.
    const ER: &str =
        "erDiagram\n    CUSTOMER {\n        string name PK\n        string mail UK\n    }\n";
    let theme = Theme::modern();
    let parsed = parse_mermaid(ER).expect("parse");
    let plain = LayoutConfig::default();
    let layout = compute_layout(&parsed.graph, &theme, &plain);
    let without = pk_badge_width(&render_svg(&layout, &theme, &plain));
    let config = LayoutConfig {
        metrics: Some(Arc::new(Wider::default())),
        ..LayoutConfig::default()
    };
    let layout = compute_layout(&parsed.graph, &theme, &config);
    let with = pk_badge_width(&render_svg(&layout, &theme, &config));
    assert!(with > without * 2.0, "badges: {without} vs {with}");
}

#[test]
fn the_provider_is_not_part_of_the_json_config() {
    let json = serde_json::to_string(&LayoutConfig {
        metrics: Some(Arc::new(Declines)),
        ..LayoutConfig::default()
    })
    .expect("serialize");
    // `fast_text_metrics` is a config field of its own; the provider is not.
    assert!(!json.contains("\"metrics\":"), "{json}");
    let parsed: LayoutConfig = serde_json::from_str(&json).expect("deserialize");
    assert!(parsed.metrics.is_none());
}
