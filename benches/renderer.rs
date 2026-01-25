use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use mermaid_rs_renderer::config::LayoutConfig;
use mermaid_rs_renderer::layout::compute_layout;
use mermaid_rs_renderer::parser::parse_mermaid;
use mermaid_rs_renderer::render::render_svg;
use mermaid_rs_renderer::theme::Theme;

fn fixture(name: &str) -> &'static str {
    match name {
        "flowchart_small" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/flowchart_small.mmd"
        )),
        "flowchart_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/flowchart_medium.mmd"
        )),
        "flowchart_large" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/flowchart_large.mmd"
        )),
        "class_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/class_medium.mmd"
        )),
        "state_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/state_medium.mmd"
        )),
        "sequence_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/sequence_medium.mmd"
        )),
        "er_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/er_medium.mmd"
        )),
        "pie_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/pie_medium.mmd"
        )),
        "mindmap_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/mindmap_medium.mmd"
        )),
        "journey_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/journey_medium.mmd"
        )),
        "timeline_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/timeline_medium.mmd"
        )),
        "gantt_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/gantt_medium.mmd"
        )),
        "requirement_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/requirement_medium.mmd"
        )),
        "gitgraph_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/gitgraph_medium.mmd"
        )),
        "c4_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/c4_medium.mmd"
        )),
        "sankey_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/sankey_medium.mmd"
        )),
        "quadrant_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/quadrant_medium.mmd"
        )),
        "zenuml_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/zenuml_medium.mmd"
        )),
        "block_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/block_medium.mmd"
        )),
        "packet_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/packet_medium.mmd"
        )),
        "kanban_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/kanban_medium.mmd"
        )),
        "architecture_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/architecture_medium.mmd"
        )),
        "radar_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/radar_medium.mmd"
        )),
        "treemap_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/treemap_medium.mmd"
        )),
        "xychart_medium" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/fixtures/xychart_medium.mmd"
        )),
        _ => panic!("unknown fixture"),
    }
}

fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");
    for name in [
        "flowchart_small",
        "flowchart_medium",
        "flowchart_large",
        "class_medium",
        "state_medium",
        "sequence_medium",
        "er_medium",
        "pie_medium",
        "mindmap_medium",
        "journey_medium",
        "timeline_medium",
        "gantt_medium",
        "requirement_medium",
        "gitgraph_medium",
        "c4_medium",
        "sankey_medium",
        "quadrant_medium",
        "zenuml_medium",
        "block_medium",
        "packet_medium",
        "kanban_medium",
        "architecture_medium",
        "radar_medium",
        "treemap_medium",
        "xychart_medium",
    ] {
        let input = fixture(name);
        group.bench_with_input(BenchmarkId::from_parameter(name), input, |b, data| {
            b.iter(|| {
                let parsed = parse_mermaid(black_box(data)).expect("parse failed");
                black_box(parsed.graph.nodes.len());
            });
        });
    }
    group.finish();
}

fn bench_layout(c: &mut Criterion) {
    let mut group = c.benchmark_group("layout");
    let theme = Theme::modern();
    let config = LayoutConfig::default();
    for name in [
        "flowchart_medium",
        "flowchart_large",
        "class_medium",
        "state_medium",
        "sequence_medium",
        "er_medium",
        "pie_medium",
        "mindmap_medium",
        "journey_medium",
        "timeline_medium",
        "gantt_medium",
        "requirement_medium",
        "gitgraph_medium",
        "c4_medium",
        "sankey_medium",
        "quadrant_medium",
        "zenuml_medium",
        "block_medium",
        "packet_medium",
        "kanban_medium",
        "architecture_medium",
        "radar_medium",
        "treemap_medium",
        "xychart_medium",
    ] {
        let parsed = parse_mermaid(fixture(name)).expect("parse failed");
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &parsed.graph,
            |b, graph| {
                b.iter(|| {
                    let layout = compute_layout(black_box(graph), &theme, &config);
                    black_box(layout.nodes.len());
                });
            },
        );
    }
    group.finish();
}

fn bench_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_svg");
    let theme = Theme::modern();
    let config = LayoutConfig::default();
    for name in [
        "flowchart_medium",
        "flowchart_large",
        "class_medium",
        "state_medium",
        "sequence_medium",
        "er_medium",
        "pie_medium",
        "mindmap_medium",
        "journey_medium",
        "timeline_medium",
        "gantt_medium",
        "requirement_medium",
        "gitgraph_medium",
        "c4_medium",
        "sankey_medium",
        "quadrant_medium",
        "zenuml_medium",
        "block_medium",
        "packet_medium",
        "kanban_medium",
        "architecture_medium",
        "radar_medium",
        "treemap_medium",
        "xychart_medium",
    ] {
        let parsed = parse_mermaid(fixture(name)).expect("parse failed");
        let layout = compute_layout(&parsed.graph, &theme, &config);
        group.bench_with_input(BenchmarkId::from_parameter(name), &layout, |b, data| {
            b.iter(|| {
                let svg = render_svg(black_box(data), &theme, &config);
                black_box(svg.len());
            });
        });
    }
    group.finish();
}

fn bench_end_to_end(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end");
    let theme = Theme::modern();
    let config = LayoutConfig::default();
    for name in [
        "flowchart_small",
        "flowchart_medium",
        "class_medium",
        "state_medium",
        "sequence_medium",
        "er_medium",
        "pie_medium",
        "mindmap_medium",
        "journey_medium",
        "timeline_medium",
        "gantt_medium",
        "requirement_medium",
        "gitgraph_medium",
        "c4_medium",
        "sankey_medium",
        "quadrant_medium",
        "zenuml_medium",
        "block_medium",
        "packet_medium",
        "kanban_medium",
        "architecture_medium",
        "radar_medium",
        "treemap_medium",
        "xychart_medium",
    ] {
        let input = fixture(name);
        group.bench_with_input(BenchmarkId::from_parameter(name), input, |b, data| {
            b.iter(|| {
                let parsed = parse_mermaid(black_box(data)).expect("parse failed");
                let layout = compute_layout(&parsed.graph, &theme, &config);
                let svg = render_svg(&layout, &theme, &config);
                black_box(svg.len());
            });
        });
    }
    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = bench_parse, bench_layout, bench_render, bench_end_to_end
);
criterion_main!(benches);
