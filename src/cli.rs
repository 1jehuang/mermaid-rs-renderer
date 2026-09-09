use crate::config::{
    FlowchartLayoutEngine, load_config_with_theme, merge_init_config, parse_aspect_ratio_value,
};
use crate::layout::{compute_layout_with_metrics, write_layered_layout_dump};
use crate::layout_dump::write_layout_dump;
use crate::parser::parse_mermaid;
#[cfg(feature = "png")]
use crate::render::write_output_png;
use crate::render::{measure_svg_dimensions, render_svg_with_dimensions, write_output_svg};
use anyhow::Result;
use clap::{Parser, ValueEnum};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "mmdr",
    version,
    about = "Fast Mermaid diagram renderer in pure Rust"
)]
pub struct Args {
    /// Input file (.mmd) or '-' for stdin
    #[arg(short = 'i', long = "input")]
    pub input: Option<PathBuf>,

    /// Output file (svg/png). Defaults to stdout for SVG if omitted.
    #[arg(short = 'o', long = "output")]
    pub output: Option<PathBuf>,

    /// Output format. Defaults to the output file's extension, else svg.
    #[arg(short = 'e', long = "outputFormat", value_enum)]
    pub output_format: Option<OutputFormat>,

    /// Config JSON file (Mermaid-like themeVariables)
    #[arg(short = 'c', long = "configFile")]
    pub config: Option<PathBuf>,

    /// Built-in theme preset (default, dark, forest, neutral, modern).
    /// Applied before config-file/init themeVariables overrides.
    #[arg(short = 't', long = "theme")]
    pub theme: Option<String>,

    /// Width (defaults to the diagram's natural size; used for PNG rasterization fallback)
    #[arg(short = 'w', long = "width")]
    pub width: Option<f32>,

    /// Height (defaults to the diagram's natural size; used for PNG rasterization fallback)
    #[arg(short = 'H', long = "height")]
    pub height: Option<f32>,

    /// Preferred output aspect ratio (`width:height`, `width/height`, or decimal)
    #[arg(long = "preferredAspectRatio", value_parser = parse_aspect_ratio_value)]
    pub preferred_aspect_ratio: Option<f32>,

    /// Node spacing
    #[arg(long = "nodeSpacing")]
    pub node_spacing: Option<f32>,

    /// Rank spacing
    #[arg(long = "rankSpacing")]
    pub rank_spacing: Option<f32>,

    /// Dump computed layout JSON (file or directory for markdown input)
    #[arg(long = "dumpLayout")]
    pub dump_layout: Option<PathBuf>,

    /// Layered flowchart placement engine (`current`, `dagre`, or guarded `auto`)
    #[arg(long = "layoutEngine", value_enum)]
    pub layout_engine: Option<LayoutEngineArg>,

    /// Dump the deterministic layered placement stage before routing/cleanup
    #[arg(long = "dumpLayeredLayout")]
    pub dump_layered_layout: Option<PathBuf>,

    /// Output timing information as JSON to stderr
    #[arg(long = "timing")]
    pub timing: bool,

    /// Print computed image size metadata as JSON and exit
    #[arg(long = "size")]
    pub size: bool,

    /// Use fast text metrics (approximate widths) for speed
    #[arg(long = "fastText")]
    pub fast_text_metrics: bool,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum OutputFormat {
    Svg,
    Png,
}

impl Args {
    /// Format to write: an explicit `-e` wins, else the output file's
    /// extension, else SVG. Without the extension fallback `-o diagram.png`
    /// silently wrote SVG bytes into a `.png`.
    pub fn resolved_output_format(&self) -> OutputFormat {
        if let Some(format) = self.output_format {
            return format;
        }
        match self.output.as_deref().and_then(Path::extension) {
            Some(ext) if ext.eq_ignore_ascii_case("png") => OutputFormat::Png,
            _ => OutputFormat::Svg,
        }
    }
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum LayoutEngineArg {
    Current,
    Dagre,
    Auto,
}

impl From<LayoutEngineArg> for FlowchartLayoutEngine {
    fn from(value: LayoutEngineArg) -> Self {
        match value {
            LayoutEngineArg::Current => Self::Current,
            LayoutEngineArg::Dagre => Self::Dagre,
            LayoutEngineArg::Auto => Self::Auto,
        }
    }
}

/// Fallback dimensions used when only one of --width/--height is given and
/// as the usvg default size for PNG rasterization of size-less SVGs.
const DEFAULT_WIDTH: f32 = 1200.0;
const DEFAULT_HEIGHT: f32 = 800.0;

pub fn run() -> Result<()> {
    let args = Args::parse();
    let mut base_config = load_config_with_theme(args.config.as_deref(), args.theme.as_deref())?;
    // Explicit output dimensions are only forced when the user passed
    // --width/--height; otherwise the diagram keeps its natural size (#83).
    let explicit_dimensions = match (args.width, args.height) {
        (None, None) => None,
        (w, h) => Some((w.unwrap_or(DEFAULT_WIDTH), h.unwrap_or(DEFAULT_HEIGHT))),
    };
    base_config.render.width = args.width.unwrap_or(DEFAULT_WIDTH);
    base_config.render.height = args.height.unwrap_or(DEFAULT_HEIGHT);
    if let Some(ratio) = args.preferred_aspect_ratio {
        base_config.layout.preferred_aspect_ratio = Some(ratio);
    }
    if let Some(spacing) = args.node_spacing {
        base_config.layout.node_spacing = spacing;
    }
    if let Some(spacing) = args.rank_spacing {
        base_config.layout.rank_spacing = spacing;
    }
    if args.fast_text_metrics {
        base_config.layout.fast_text_metrics = true;
    }
    if let Some(engine) = args.layout_engine {
        base_config.layout.flowchart.engine = engine.into();
    }

    let (input, is_markdown) = read_input(args.input.as_deref())?;
    let diagrams = if is_markdown {
        extract_mermaid_blocks(&input)
    } else {
        vec![input]
    };

    if diagrams.is_empty() {
        return Err(anyhow::anyhow!("No Mermaid diagrams found in input"));
    }

    let layout_outputs = if args.dump_layout.is_some() {
        Some(resolve_layout_outputs(
            args.dump_layout.as_deref(),
            diagrams.len(),
        )?)
    } else {
        None
    };
    let layered_layout_outputs = if args.dump_layered_layout.is_some() {
        Some(resolve_layout_outputs(
            args.dump_layered_layout.as_deref(),
            diagrams.len(),
        )?)
    } else {
        None
    };

    if diagrams.len() == 1 {
        let t_parse_start = std::time::Instant::now();
        let parsed = parse_mermaid(&diagrams[0])?;
        let parse_us = t_parse_start.elapsed().as_micros();

        let mut config = base_config.clone();
        if let Some(init_cfg) = parsed.init_config {
            config = merge_init_config(config, init_cfg);
        }
        if let Some(engine) = args.layout_engine {
            config.layout.flowchart.engine = engine.into();
        }

        let t_layout_start = std::time::Instant::now();
        let (layout, layout_stages) =
            compute_layout_with_metrics(&parsed.graph, &config.theme, &config.layout);
        let layout_us = t_layout_start.elapsed().as_micros();

        if let Some(outputs) = layout_outputs.as_ref()
            && let Some(path) = outputs.first()
        {
            write_layout_dump(path, &layout, &parsed.graph)?;
        }
        if let Some(outputs) = layered_layout_outputs.as_ref()
            && let Some(path) = outputs.first()
        {
            let snapshot = layout_stages.layered_layout.as_ref().ok_or_else(|| {
                anyhow::anyhow!("--dumpLayeredLayout is only available for layered graph layouts")
            })?;
            write_layered_layout_dump(path, snapshot)?;
        }

        let dimensions = measure_svg_dimensions(&layout, &config.layout, explicit_dimensions);
        if args.size {
            println!("{}", serde_json::to_string_pretty(&dimensions)?);
            return Ok(());
        }

        let t_render_start = std::time::Instant::now();
        let svg =
            render_svg_with_dimensions(&layout, &config.theme, &config.layout, explicit_dimensions);
        let render_us = t_render_start.elapsed().as_micros();

        match args.resolved_output_format() {
            OutputFormat::Svg => {
                write_output_svg(&svg, args.output.as_deref())?;
            }
            #[cfg(feature = "png")]
            OutputFormat::Png => {
                let output = ensure_output(&args.output, "png")?;
                write_output_png(&svg, &output, &config.render, &config.theme)?;
            }
            #[cfg(not(feature = "png"))]
            OutputFormat::Png => {
                return Err(anyhow::anyhow!(
                    "PNG output requires the 'png' feature. Rebuild with: cargo build --features png"
                ));
            }
        }

        if args.timing {
            let total_us = parse_us + layout_us + render_us;
            let payload = serde_json::json!({
                "parse_us": parse_us,
                "layout_us": layout_us,
                "render_us": render_us,
                "total_us": total_us,
                "layout_stage_us": {
                    "layered_layout_us": layout_stages.layered_layout_us,
                    "port_assignment_us": layout_stages.port_assignment_us,
                    "edge_routing_us": layout_stages.edge_routing_us,
                    "label_placement_us": layout_stages.label_placement_us,
                    "total_us": layout_stages.total_us(),
                }
            });
            eprintln!("{payload}");
        }
        return Ok(());
    }

    // Multiple diagrams (Markdown input)
    if args.size {
        let mut sizes = Vec::new();
        for (idx, diagram) in diagrams.iter().enumerate() {
            let parsed = parse_mermaid(diagram)?;
            let mut config = base_config.clone();
            if let Some(init_cfg) = parsed.init_config.clone() {
                config = merge_init_config(config, init_cfg);
            }
            if let Some(engine) = args.layout_engine {
                config.layout.flowchart.engine = engine.into();
            }
            let (layout, layout_stages) =
                compute_layout_with_metrics(&parsed.graph, &config.theme, &config.layout);
            if let Some(outputs) = layered_layout_outputs.as_ref()
                && let Some(path) = outputs.get(idx)
            {
                let snapshot = layout_stages.layered_layout.as_ref().ok_or_else(|| {
                    anyhow::anyhow!(
                        "--dumpLayeredLayout is only available for layered graph layouts"
                    )
                })?;
                write_layered_layout_dump(path, snapshot)?;
            }
            let dimensions = measure_svg_dimensions(&layout, &config.layout, explicit_dimensions);
            sizes.push(serde_json::json!({
                "index": idx,
                "dimensions": dimensions,
            }));
        }
        println!("{}", serde_json::to_string_pretty(&sizes)?);
        return Ok(());
    }

    let outputs = resolve_multi_outputs(
        args.output.as_deref(),
        args.resolved_output_format(),
        diagrams.len(),
    )?;
    for (idx, diagram) in diagrams.iter().enumerate() {
        let parsed = parse_mermaid(diagram)?;
        let mut config = base_config.clone();
        if let Some(init_cfg) = parsed.init_config.clone() {
            config = merge_init_config(config, init_cfg);
        }
        if let Some(engine) = args.layout_engine {
            config.layout.flowchart.engine = engine.into();
        }
        let (layout, layout_stages) =
            compute_layout_with_metrics(&parsed.graph, &config.theme, &config.layout);
        if let Some(outputs) = layout_outputs.as_ref()
            && let Some(path) = outputs.get(idx)
        {
            write_layout_dump(path, &layout, &parsed.graph)?;
        }
        if let Some(outputs) = layered_layout_outputs.as_ref()
            && let Some(path) = outputs.get(idx)
        {
            let snapshot = layout_stages.layered_layout.as_ref().ok_or_else(|| {
                anyhow::anyhow!("--dumpLayeredLayout is only available for layered graph layouts")
            })?;
            write_layered_layout_dump(path, snapshot)?;
        }
        let svg =
            render_svg_with_dimensions(&layout, &config.theme, &config.layout, explicit_dimensions);
        match args.resolved_output_format() {
            OutputFormat::Svg => {
                write_output_svg(&svg, Some(&outputs[idx]))?;
            }
            #[cfg(feature = "png")]
            OutputFormat::Png => {
                write_output_png(&svg, &outputs[idx], &config.render, &config.theme)?;
            }
            #[cfg(not(feature = "png"))]
            OutputFormat::Png => {
                return Err(anyhow::anyhow!(
                    "PNG output requires the 'png' feature. Rebuild with: cargo build --features png"
                ));
            }
        }
    }

    Ok(())
}

fn read_input(path: Option<&Path>) -> Result<(String, bool)> {
    if let Some(path) = path {
        if path == Path::new("-") {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            return Ok((buf, false));
        }
        let content = std::fs::read_to_string(path)?;
        let is_md = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|ext| {
                let ext = ext.to_ascii_lowercase();
                matches!(ext.as_str(), "md" | "markdown")
            })
            .unwrap_or(false);
        return Ok((content, is_md));
    }

    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    Ok((buf, false))
}

#[cfg(feature = "png")]
fn ensure_output(output: &Option<PathBuf>, ext: &str) -> Result<PathBuf> {
    if let Some(path) = output {
        return Ok(path.clone());
    }
    Err(anyhow::anyhow!("Output path required for {} output", ext))
}

fn extract_mermaid_blocks(input: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut current = Vec::new();
    let mut fence = String::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if !in_block {
            if let Some(start_fence) = detect_mermaid_fence(trimmed) {
                in_block = true;
                fence = start_fence;
                continue;
            }
        } else if is_fence_end(trimmed, &fence) {
            in_block = false;
            blocks.push(current.join("\n"));
            current.clear();
            continue;
        }

        if in_block {
            current.push(line.to_string());
        }
    }

    blocks
}

fn detect_mermaid_fence(line: &str) -> Option<String> {
    if line.starts_with("```") {
        let rest = line.trim_start_matches('`').trim();
        if rest.starts_with("mermaid") {
            return Some("```".to_string());
        }
    }
    if line.starts_with("~~~") {
        let rest = line.trim_start_matches('~').trim();
        if rest.starts_with("mermaid") {
            return Some("~~~".to_string());
        }
    }
    if line.starts_with(":::") {
        let rest = line.trim_start_matches(':').trim();
        if rest.starts_with("mermaid") {
            return Some(":::".to_string());
        }
    }
    None
}

fn is_fence_end(line: &str, fence: &str) -> bool {
    if !line.starts_with(fence) {
        return false;
    }
    line[fence.len()..].trim().is_empty()
}

fn resolve_multi_outputs(
    output: Option<&Path>,
    format: OutputFormat,
    count: usize,
) -> Result<Vec<PathBuf>> {
    let ext = match format {
        OutputFormat::Svg => "svg",
        OutputFormat::Png => "png",
    };
    let base = output.ok_or_else(|| anyhow::anyhow!("Output path required for markdown input"))?;
    if is_directory_output_path(base) {
        std::fs::create_dir_all(base)?;
        let mut outputs = Vec::new();
        for idx in 0..count {
            outputs.push(base.join(format!("diagram-{}.{}", idx + 1, ext)));
        }
        return Ok(outputs);
    }
    let stem = base
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("diagram");
    let parent = base.parent().unwrap_or_else(|| Path::new("."));
    let mut outputs = Vec::new();
    for idx in 0..count {
        outputs.push(parent.join(format!("{}-{}.{}", stem, idx + 1, ext)));
    }
    Ok(outputs)
}

fn resolve_layout_outputs(output: Option<&Path>, count: usize) -> Result<Vec<PathBuf>> {
    let base = output.ok_or_else(|| anyhow::anyhow!("Dump layout path required"))?;
    if is_directory_output_path(base) {
        std::fs::create_dir_all(base)?;
        let mut outputs = Vec::new();
        for idx in 0..count {
            outputs.push(base.join(format!("diagram-{}.layout.json", idx + 1)));
        }
        return Ok(outputs);
    }
    if count == 1 {
        return Ok(vec![base.to_path_buf()]);
    }
    let stem = base
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("diagram");
    let parent = base.parent().unwrap_or_else(|| Path::new("."));
    let mut outputs = Vec::new();
    for idx in 0..count {
        outputs.push(parent.join(format!("{}-{}.layout.json", stem, idx + 1)));
    }
    Ok(outputs)
}

fn is_directory_output_path(path: &Path) -> bool {
    if path.is_dir() {
        return true;
    }
    let raw = path.as_os_str().to_string_lossy();
    raw.ends_with('/') || raw.ends_with('\\')
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use crate::config::Config;
    use serde_json::json;

    #[test]
    fn extracts_mermaid_blocks() {
        let input = r#"
text
``` mermaid
flowchart LR
  A --> B
```
more
~~~mermaid
flowchart TD
  X --> Y
~~~
::: mermaid
sequenceDiagram
  A->>B: hi
:::
"#;
        let blocks = extract_mermaid_blocks(input);
        assert_eq!(blocks.len(), 3);
        assert!(blocks[0].contains("flowchart"));
        assert!(blocks[1].contains("flowchart"));
        assert!(blocks[2].contains("sequenceDiagram"));
    }

    #[test]
    fn merge_init_config_updates_layout() {
        let config = Config::default();
        let init = json!({
            "flowchart": {
                "nodeSpacing": 55,
                "rankSpacing": 90,
                "engine": "dagre"
            }
        });
        let merged = merge_init_config(config, init);
        assert_eq!(merged.layout.node_spacing, 55.0);
        assert_eq!(merged.layout.rank_spacing, 90.0);
        assert_eq!(merged.layout.flowchart.engine, FlowchartLayoutEngine::Dagre);
    }

    #[test]
    fn parses_layered_layout_engine_and_dump_path() {
        let args = Args::try_parse_from([
            "mmdr",
            "--layoutEngine",
            "dagre",
            "--dumpLayeredLayout",
            "stage.json",
        ])
        .unwrap();
        assert!(matches!(args.layout_engine, Some(LayoutEngineArg::Dagre)));
        assert_eq!(
            args.dump_layered_layout.as_deref(),
            Some(Path::new("stage.json"))
        );
    }

    #[test]
    fn parses_guarded_auto_layout_engine() {
        let args = Args::try_parse_from(["mmdr", "--layoutEngine", "auto"]).unwrap();
        assert!(matches!(args.layout_engine, Some(LayoutEngineArg::Auto)));
    }

    #[test]
    fn merge_init_config_theme_variables() {
        let config = Config::default();
        let init = json!({
            "themeVariables": {
                "secondaryColor": "#ff00ff",
                "tertiaryColor": "#00ffff",
                "edgeLabelBackground": "#222222",
                "clusterBkg": "#333333",
                "clusterBorder": "#444444",
                "background": "#101010"
            }
        });
        let merged = merge_init_config(config, init);
        assert_eq!(merged.theme.secondary_color, "#ff00ff");
        assert_eq!(merged.theme.tertiary_color, "#00ffff");
        assert_eq!(merged.theme.edge_label_background, "#222222");
        assert_eq!(merged.theme.cluster_background, "#333333");
        assert_eq!(merged.theme.cluster_border, "#444444");
        assert_eq!(merged.theme.background, "#101010");
        assert_eq!(merged.render.background, "#101010");
    }

    #[test]
    fn parse_aspect_ratio_accepts_common_formats() {
        assert_eq!(parse_aspect_ratio_value("16:9").unwrap(), 16.0 / 9.0);
        assert_eq!(parse_aspect_ratio_value("4/3").unwrap(), 4.0 / 3.0);
        assert_eq!(parse_aspect_ratio_value("1.5").unwrap(), 1.5);
    }

    #[test]
    fn parses_size_flag() {
        let args = Args::try_parse_from(["mmdr", "--size"]).unwrap();
        assert!(args.size);
    }

    #[test]
    fn merge_init_config_updates_preferred_aspect_ratio() {
        let config = Config::default();
        let init = json!({
            "preferredAspectRatio": "16:9"
        });
        let merged = merge_init_config(config, init);
        assert_eq!(merged.layout.preferred_aspect_ratio, Some(16.0 / 9.0));
    }

    #[test]
    fn merge_init_config_updates_timeline_default_direction() {
        let config = Config::default();
        let init = json!({
            "timeline": {
                "defaultDirection": "TD"
            }
        });
        let merged = merge_init_config(config, init);
        assert_eq!(merged.layout.timeline.direction, "TD");
    }
}
