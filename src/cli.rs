use crate::config::{Config, load_config};
use crate::layout::compute_layout;
use crate::layout_dump::write_layout_dump;
use crate::parser::parse_mermaid;
#[cfg(feature = "png")]
use crate::render::write_output_png;
use crate::render::{render_svg, write_output_svg};
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

    /// Output format
    #[arg(short = 'e', long = "outputFormat", value_enum, default_value = "svg")]
    pub output_format: OutputFormat,

    /// Config JSON file (Mermaid-like themeVariables)
    #[arg(short = 'c', long = "configFile")]
    pub config: Option<PathBuf>,

    /// Width
    #[arg(short = 'w', long = "width", default_value_t = 1200.0)]
    pub width: f32,

    /// Height
    #[arg(short = 'H', long = "height", default_value_t = 800.0)]
    pub height: f32,

    /// Node spacing
    #[arg(long = "nodeSpacing")]
    pub node_spacing: Option<f32>,

    /// Rank spacing
    #[arg(long = "rankSpacing")]
    pub rank_spacing: Option<f32>,

    /// Dump computed layout JSON (file or directory for markdown input)
    #[arg(long = "dumpLayout")]
    pub dump_layout: Option<PathBuf>,

    /// Output timing information as JSON to stderr
    #[arg(long = "timing")]
    pub timing: bool,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum OutputFormat {
    Svg,
    Png,
}

pub fn run() -> Result<()> {
    let args = Args::parse();
    let mut base_config = load_config(args.config.as_deref())?;
    base_config.render.width = args.width;
    base_config.render.height = args.height;
    if let Some(spacing) = args.node_spacing {
        base_config.layout.node_spacing = spacing;
    }
    if let Some(spacing) = args.rank_spacing {
        base_config.layout.rank_spacing = spacing;
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

    if diagrams.len() == 1 {
        let t_parse_start = std::time::Instant::now();
        let parsed = parse_mermaid(&diagrams[0])?;
        let parse_us = t_parse_start.elapsed().as_micros();

        let mut config = base_config.clone();
        if let Some(init_cfg) = parsed.init_config {
            config = merge_init_config(config, init_cfg);
        }

        let t_layout_start = std::time::Instant::now();
        let layout = compute_layout(&parsed.graph, &config.theme, &config.layout);
        let layout_us = t_layout_start.elapsed().as_micros();

        if let Some(outputs) = layout_outputs.as_ref()
            && let Some(path) = outputs.first()
        {
            write_layout_dump(path, &layout, &parsed.graph)?;
        }

        let t_render_start = std::time::Instant::now();
        let svg = render_svg(&layout, &config.theme, &config.layout);
        let render_us = t_render_start.elapsed().as_micros();

        match args.output_format {
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
            eprintln!(
                r#"{{"parse_us":{},"layout_us":{},"render_us":{},"total_us":{}}}"#,
                parse_us, layout_us, render_us, total_us
            );
        }
        return Ok(());
    }

    // Multiple diagrams (Markdown input)
    let outputs =
        resolve_multi_outputs(args.output.as_deref(), args.output_format, diagrams.len())?;
    for (idx, diagram) in diagrams.iter().enumerate() {
        let parsed = parse_mermaid(diagram)?;
        let mut config = base_config.clone();
        if let Some(init_cfg) = parsed.init_config.clone() {
            config = merge_init_config(config, init_cfg);
        }
        let layout = compute_layout(&parsed.graph, &config.theme, &config.layout);
        if let Some(outputs) = layout_outputs.as_ref()
            && let Some(path) = outputs.get(idx)
        {
            write_layout_dump(path, &layout, &parsed.graph)?;
        }
        let svg = render_svg(&layout, &config.theme, &config.layout);
        match args.output_format {
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
    if base.is_dir() {
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
    if base.is_dir() {
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

#[cfg(test)]
mod tests {
    use super::*;
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
                "rankSpacing": 90
            }
        });
        let merged = merge_init_config(config, init);
        assert_eq!(merged.layout.node_spacing, 55.0);
        assert_eq!(merged.layout.rank_spacing, 90.0);
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
}

fn merge_init_config(mut config: Config, init: serde_json::Value) -> Config {
    if let Some(theme_name) = init.get("theme").and_then(|v| v.as_str()) {
        if theme_name == "modern" {
            config.theme = crate::theme::Theme::modern();
        } else if theme_name == "base" || theme_name == "default" || theme_name == "mermaid" {
            config.theme = crate::theme::Theme::mermaid_default();
        }
    }
    if let Some(theme_vars) = init.get("themeVariables") {
        let tag_label_border_explicit = theme_vars
            .get("tagLabelBorder")
            .and_then(|v| v.as_str())
            .is_some();
        let primary_border_override = theme_vars
            .get("primaryBorderColor")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());
        if let Some(val) = theme_vars.get("primaryColor").and_then(|v| v.as_str()) {
            config.theme.primary_color = val.to_string();
        }
        if let Some(val) = theme_vars.get("primaryTextColor").and_then(|v| v.as_str()) {
            config.theme.primary_text_color = val.to_string();
        }
        if let Some(val) = theme_vars
            .get("primaryBorderColor")
            .and_then(|v| v.as_str())
        {
            config.theme.primary_border_color = val.to_string();
        }
        if let Some(val) = theme_vars.get("lineColor").and_then(|v| v.as_str()) {
            config.theme.line_color = val.to_string();
        }
        if let Some(val) = theme_vars.get("secondaryColor").and_then(|v| v.as_str()) {
            config.theme.secondary_color = val.to_string();
        }
        if let Some(val) = theme_vars.get("tertiaryColor").and_then(|v| v.as_str()) {
            config.theme.tertiary_color = val.to_string();
        }
        if let Some(val) = theme_vars.get("textColor").and_then(|v| v.as_str()) {
            config.theme.text_color = val.to_string();
        }
        if let Some(val) = theme_vars
            .get("edgeLabelBackground")
            .and_then(|v| v.as_str())
        {
            config.theme.edge_label_background = val.to_string();
        }
        if let Some(val) = theme_vars.get("clusterBkg").and_then(|v| v.as_str()) {
            config.theme.cluster_background = val.to_string();
        }
        if let Some(val) = theme_vars.get("clusterBorder").and_then(|v| v.as_str()) {
            config.theme.cluster_border = val.to_string();
        }
        if let Some(val) = theme_vars.get("background").and_then(|v| v.as_str()) {
            config.theme.background = val.to_string();
        }
        if let Some(val) = theme_vars.get("actorBkg").and_then(|v| v.as_str()) {
            config.theme.sequence_actor_fill = val.to_string();
        }
        if let Some(val) = theme_vars.get("actorBorder").and_then(|v| v.as_str()) {
            config.theme.sequence_actor_border = val.to_string();
        }
        if let Some(val) = theme_vars.get("actorLine").and_then(|v| v.as_str()) {
            config.theme.sequence_actor_line = val.to_string();
        }
        if let Some(val) = theme_vars.get("noteBkg").and_then(|v| v.as_str()) {
            config.theme.sequence_note_fill = val.to_string();
        }
        if let Some(val) = theme_vars
            .get("noteBorderColor")
            .and_then(|v| v.as_str())
        {
            config.theme.sequence_note_border = val.to_string();
        }
        if let Some(val) = theme_vars
            .get("activationBkgColor")
            .and_then(|v| v.as_str())
        {
            config.theme.sequence_activation_fill = val.to_string();
        }
        if let Some(val) = theme_vars
            .get("activationBorderColor")
            .and_then(|v| v.as_str())
        {
            config.theme.sequence_activation_border = val.to_string();
        }
        if let Some(val) = theme_vars.get("git0").and_then(|v| v.as_str()) {
            config.theme.git_colors[0] = val.to_string();
        }
        if let Some(val) = theme_vars.get("git1").and_then(|v| v.as_str()) {
            config.theme.git_colors[1] = val.to_string();
        }
        if let Some(val) = theme_vars.get("git2").and_then(|v| v.as_str()) {
            config.theme.git_colors[2] = val.to_string();
        }
        if let Some(val) = theme_vars.get("git3").and_then(|v| v.as_str()) {
            config.theme.git_colors[3] = val.to_string();
        }
        if let Some(val) = theme_vars.get("git4").and_then(|v| v.as_str()) {
            config.theme.git_colors[4] = val.to_string();
        }
        if let Some(val) = theme_vars.get("git5").and_then(|v| v.as_str()) {
            config.theme.git_colors[5] = val.to_string();
        }
        if let Some(val) = theme_vars.get("git6").and_then(|v| v.as_str()) {
            config.theme.git_colors[6] = val.to_string();
        }
        if let Some(val) = theme_vars.get("git7").and_then(|v| v.as_str()) {
            config.theme.git_colors[7] = val.to_string();
        }
        if let Some(val) = theme_vars.get("gitInv0").and_then(|v| v.as_str()) {
            config.theme.git_inv_colors[0] = val.to_string();
        }
        if let Some(val) = theme_vars.get("gitInv1").and_then(|v| v.as_str()) {
            config.theme.git_inv_colors[1] = val.to_string();
        }
        if let Some(val) = theme_vars.get("gitInv2").and_then(|v| v.as_str()) {
            config.theme.git_inv_colors[2] = val.to_string();
        }
        if let Some(val) = theme_vars.get("gitInv3").and_then(|v| v.as_str()) {
            config.theme.git_inv_colors[3] = val.to_string();
        }
        if let Some(val) = theme_vars.get("gitInv4").and_then(|v| v.as_str()) {
            config.theme.git_inv_colors[4] = val.to_string();
        }
        if let Some(val) = theme_vars.get("gitInv5").and_then(|v| v.as_str()) {
            config.theme.git_inv_colors[5] = val.to_string();
        }
        if let Some(val) = theme_vars.get("gitInv6").and_then(|v| v.as_str()) {
            config.theme.git_inv_colors[6] = val.to_string();
        }
        if let Some(val) = theme_vars.get("gitInv7").and_then(|v| v.as_str()) {
            config.theme.git_inv_colors[7] = val.to_string();
        }
        if let Some(val) = theme_vars.get("gitBranchLabel0").and_then(|v| v.as_str()) {
            config.theme.git_branch_label_colors[0] = val.to_string();
        }
        if let Some(val) = theme_vars.get("gitBranchLabel1").and_then(|v| v.as_str()) {
            config.theme.git_branch_label_colors[1] = val.to_string();
        }
        if let Some(val) = theme_vars.get("gitBranchLabel2").and_then(|v| v.as_str()) {
            config.theme.git_branch_label_colors[2] = val.to_string();
        }
        if let Some(val) = theme_vars.get("gitBranchLabel3").and_then(|v| v.as_str()) {
            config.theme.git_branch_label_colors[3] = val.to_string();
        }
        if let Some(val) = theme_vars.get("gitBranchLabel4").and_then(|v| v.as_str()) {
            config.theme.git_branch_label_colors[4] = val.to_string();
        }
        if let Some(val) = theme_vars.get("gitBranchLabel5").and_then(|v| v.as_str()) {
            config.theme.git_branch_label_colors[5] = val.to_string();
        }
        if let Some(val) = theme_vars.get("gitBranchLabel6").and_then(|v| v.as_str()) {
            config.theme.git_branch_label_colors[6] = val.to_string();
        }
        if let Some(val) = theme_vars.get("gitBranchLabel7").and_then(|v| v.as_str()) {
            config.theme.git_branch_label_colors[7] = val.to_string();
        }
        if let Some(val) = theme_vars.get("commitLabelColor").and_then(|v| v.as_str()) {
            config.theme.git_commit_label_color = val.to_string();
        }
        if let Some(val) = theme_vars
            .get("commitLabelBackground")
            .and_then(|v| v.as_str())
        {
            config.theme.git_commit_label_background = val.to_string();
        }
        if let Some(val) = theme_vars.get("tagLabelColor").and_then(|v| v.as_str()) {
            config.theme.git_tag_label_color = val.to_string();
        }
        if let Some(val) = theme_vars
            .get("tagLabelBackground")
            .and_then(|v| v.as_str())
        {
            config.theme.git_tag_label_background = val.to_string();
        }
        if let Some(val) = theme_vars.get("tagLabelBorder").and_then(|v| v.as_str()) {
            config.theme.git_tag_label_border = val.to_string();
        }
        if !tag_label_border_explicit && primary_border_override.is_some() {
            config.theme.git_tag_label_border = config.theme.primary_border_color.clone();
        }
        if let Some(val) = theme_vars.get("fontFamily").and_then(|v| v.as_str()) {
            config.theme.font_family = val.to_string();
        }
        if let Some(val) = theme_vars.get("fontSize").and_then(|v| v.as_f64()) {
            config.theme.font_size = val as f32;
        }
    }
    if let Some(flowchart) = init.get("flowchart") {
        if let Some(val) = flowchart.get("nodeSpacing").and_then(|v| v.as_f64()) {
            config.layout.node_spacing = val as f32;
        }
        if let Some(val) = flowchart.get("rankSpacing").and_then(|v| v.as_f64()) {
            config.layout.rank_spacing = val as f32;
        }
    }
    if let Some(gitgraph) = init.get("gitGraph") {
        let mut commit_step_set = false;
        if let Some(val) = gitgraph.get("diagramPadding").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.diagram_padding = val as f32;
        }
        if let Some(val) = gitgraph.get("titleTopMargin").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.title_top_margin = val as f32;
        }
        if let Some(val) = gitgraph.get("useMaxWidth").and_then(|v| v.as_bool()) {
            config.layout.gitgraph.use_max_width = val;
        }
        if let Some(val) = gitgraph.get("mainBranchName").and_then(|v| v.as_str()) {
            config.layout.gitgraph.main_branch_name = val.to_string();
        }
        if let Some(val) = gitgraph.get("mainBranchOrder").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.main_branch_order = val as f32;
        }
        if let Some(val) = gitgraph.get("showCommitLabel").and_then(|v| v.as_bool()) {
            config.layout.gitgraph.show_commit_label = val;
        }
        if let Some(val) = gitgraph.get("showBranches").and_then(|v| v.as_bool()) {
            config.layout.gitgraph.show_branches = val;
        }
        if let Some(val) = gitgraph.get("rotateCommitLabel").and_then(|v| v.as_bool()) {
            config.layout.gitgraph.rotate_commit_label = val;
        }
        if let Some(val) = gitgraph.get("parallelCommits").and_then(|v| v.as_bool()) {
            config.layout.gitgraph.parallel_commits = val;
        }
        if let Some(val) = gitgraph.get("commitStep").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.commit_step = val as f32;
            commit_step_set = true;
        }
        if let Some(val) = gitgraph.get("layoutOffset").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.layout_offset = val as f32;
        }
        if let Some(val) = gitgraph.get("defaultPos").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.default_pos = val as f32;
        }
        if let Some(val) = gitgraph.get("branchSpacing").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.branch_spacing = val as f32;
        }
        if let Some(val) = gitgraph
            .get("branchSpacingRotateExtra")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.branch_spacing_rotate_extra = val as f32;
        }
        if let Some(val) = gitgraph
            .get("branchLabelRotateExtra")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.branch_label_rotate_extra = val as f32;
        }
        if let Some(val) = gitgraph
            .get("branchLabelTranslateX")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.branch_label_translate_x = val as f32;
        }
        if let Some(val) = gitgraph
            .get("branchLabelBgOffsetX")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.branch_label_bg_offset_x = val as f32;
        }
        if let Some(val) = gitgraph
            .get("branchLabelBgOffsetY")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.branch_label_bg_offset_y = val as f32;
        }
        if let Some(val) = gitgraph
            .get("branchLabelBgPadX")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.branch_label_bg_pad_x = val as f32;
        }
        if let Some(val) = gitgraph
            .get("branchLabelBgPadY")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.branch_label_bg_pad_y = val as f32;
        }
        if let Some(val) = gitgraph
            .get("branchLabelTextOffsetX")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.branch_label_text_offset_x = val as f32;
        }
        if let Some(val) = gitgraph
            .get("branchLabelTextOffsetY")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.branch_label_text_offset_y = val as f32;
        }
        if let Some(val) = gitgraph
            .get("branchLabelTbBgOffsetX")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.branch_label_tb_bg_offset_x = val as f32;
        }
        if let Some(val) = gitgraph
            .get("branchLabelTbTextOffsetX")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.branch_label_tb_text_offset_x = val as f32;
        }
        if let Some(val) = gitgraph
            .get("branchLabelTbOffsetY")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.branch_label_tb_offset_y = val as f32;
        }
        if let Some(val) = gitgraph
            .get("branchLabelBtOffsetY")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.branch_label_bt_offset_y = val as f32;
        }
        if let Some(val) = gitgraph
            .get("branchLabelCornerRadius")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.branch_label_corner_radius = val as f32;
        }
        if let Some(val) = gitgraph
            .get("branchLabelFontSize")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.branch_label_font_size = val as f32;
        }
        if let Some(val) = gitgraph
            .get("branchLabelLineHeight")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.branch_label_line_height = val as f32;
        }
        if let Some(val) = gitgraph.get("textWidthScale").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.text_width_scale = val as f32;
        }
        if let Some(val) = gitgraph
            .get("commitLabelFontSize")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.commit_label_font_size = val as f32;
        }
        if let Some(val) = gitgraph
            .get("commitLabelLineHeight")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.commit_label_line_height = val as f32;
        }
        if let Some(val) = gitgraph
            .get("commitLabelOffsetY")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.commit_label_offset_y = val as f32;
        }
        if let Some(val) = gitgraph
            .get("commitLabelBgOffsetY")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.commit_label_bg_offset_y = val as f32;
        }
        if let Some(val) = gitgraph
            .get("commitLabelPadding")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.commit_label_padding = val as f32;
        }
        if let Some(val) = gitgraph
            .get("commitLabelBgOpacity")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.commit_label_bg_opacity = val as f32;
        }
        if let Some(val) = gitgraph
            .get("commitLabelRotateAngle")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.commit_label_rotate_angle = val as f32;
        }
        if let Some(val) = gitgraph
            .get("commitLabelRotateTranslateXBase")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.commit_label_rotate_translate_x_base = val as f32;
        }
        if let Some(val) = gitgraph
            .get("commitLabelRotateTranslateXScale")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.commit_label_rotate_translate_x_scale = val as f32;
        }
        if let Some(val) = gitgraph
            .get("commitLabelRotateTranslateXWidthOffset")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.commit_label_rotate_translate_x_width_offset = val as f32;
        }
        if let Some(val) = gitgraph
            .get("commitLabelRotateTranslateYBase")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.commit_label_rotate_translate_y_base = val as f32;
        }
        if let Some(val) = gitgraph
            .get("commitLabelRotateTranslateYScale")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.commit_label_rotate_translate_y_scale = val as f32;
        }
        if let Some(val) = gitgraph
            .get("commitLabelTbTextExtra")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.commit_label_tb_text_extra = val as f32;
        }
        if let Some(val) = gitgraph
            .get("commitLabelTbBgExtra")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.commit_label_tb_bg_extra = val as f32;
        }
        if let Some(val) = gitgraph
            .get("commitLabelTbTextOffsetY")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.commit_label_tb_text_offset_y = val as f32;
        }
        if let Some(val) = gitgraph
            .get("commitLabelTbBgOffsetY")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.commit_label_tb_bg_offset_y = val as f32;
        }
        if let Some(val) = gitgraph
            .get("tagLabelFontSize")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.tag_label_font_size = val as f32;
        }
        if let Some(val) = gitgraph
            .get("tagLabelLineHeight")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.tag_label_line_height = val as f32;
        }
        if let Some(val) = gitgraph.get("tagTextOffsetY").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.tag_text_offset_y = val as f32;
        }
        if let Some(val) = gitgraph
            .get("tagPolygonOffsetY")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.tag_polygon_offset_y = val as f32;
        }
        if let Some(val) = gitgraph.get("tagSpacingY").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.tag_spacing_y = val as f32;
        }
        if let Some(val) = gitgraph.get("tagPaddingX").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.tag_padding_x = val as f32;
        }
        if let Some(val) = gitgraph.get("tagPaddingY").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.tag_padding_y = val as f32;
        }
        if let Some(val) = gitgraph.get("tagHoleRadius").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.tag_hole_radius = val as f32;
        }
        if let Some(val) = gitgraph
            .get("tagRotateTranslate")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.tag_rotate_translate = val as f32;
        }
        if let Some(val) = gitgraph
            .get("tagTextRotateTranslate")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.tag_text_rotate_translate = val as f32;
        }
        if let Some(val) = gitgraph.get("tagRotateAngle").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.tag_rotate_angle = val as f32;
        }
        if let Some(val) = gitgraph
            .get("tagTextOffsetXTb")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.tag_text_offset_x_tb = val as f32;
        }
        if let Some(val) = gitgraph
            .get("tagTextOffsetYTb")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.tag_text_offset_y_tb = val as f32;
        }
        if let Some(val) = gitgraph
            .get("arrowRerouteRadius")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.arrow_reroute_radius = val as f32;
        }
        if let Some(val) = gitgraph.get("arrowRadius").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.arrow_radius = val as f32;
        }
        if let Some(val) = gitgraph.get("laneSpacing").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.lane_spacing = val as f32;
        }
        if let Some(val) = gitgraph.get("laneMaxDepth").and_then(|v| v.as_u64()) {
            config.layout.gitgraph.lane_max_depth = val as usize;
        }
        if let Some(val) = gitgraph.get("commitRadius").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.commit_radius = val as f32;
        }
        if let Some(val) = gitgraph.get("mergeRadiusOuter").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.merge_radius_outer = val as f32;
        }
        if let Some(val) = gitgraph.get("mergeRadiusInner").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.merge_radius_inner = val as f32;
        }
        if let Some(val) = gitgraph.get("highlightOuterSize").and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.highlight_outer_size = val as f32;
        }
        if let Some(val) = gitgraph.get("highlightInnerSize").and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.highlight_inner_size = val as f32;
        }
        if let Some(val) = gitgraph.get("reverseCrossSize").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.reverse_cross_size = val as f32;
        }
        if let Some(val) = gitgraph.get("reverseStrokeWidth").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.reverse_stroke_width = val as f32;
        }
        if let Some(val) = gitgraph
            .get("cherryPickDotRadius")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.cherry_pick_dot_radius = val as f32;
        }
        if let Some(val) = gitgraph
            .get("cherryPickDotOffsetX")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.cherry_pick_dot_offset_x = val as f32;
        }
        if let Some(val) = gitgraph
            .get("cherryPickDotOffsetY")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.cherry_pick_dot_offset_y = val as f32;
        }
        if let Some(val) = gitgraph
            .get("cherryPickStemStartOffsetY")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.cherry_pick_stem_start_offset_y = val as f32;
        }
        if let Some(val) = gitgraph
            .get("cherryPickStemEndOffsetY")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.cherry_pick_stem_end_offset_y = val as f32;
        }
        if let Some(val) = gitgraph
            .get("cherryPickStemStrokeWidth")
            .and_then(|v| v.as_f64())
        {
            config.layout.gitgraph.cherry_pick_stem_stroke_width = val as f32;
        }
        if let Some(val) = gitgraph
            .get("cherryPickAccentColor")
            .and_then(|v| v.as_str())
        {
            config.layout.gitgraph.cherry_pick_accent_color = val.to_string();
        }
        if let Some(val) = gitgraph.get("arrowStrokeWidth").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.arrow_stroke_width = val as f32;
        }
        if let Some(val) = gitgraph.get("branchStrokeWidth").and_then(|v| v.as_f64()) {
            config.layout.gitgraph.branch_stroke_width = val as f32;
        }
        if let Some(val) = gitgraph.get("branchDasharray").and_then(|v| v.as_str()) {
            config.layout.gitgraph.branch_dasharray = val.to_string();
        }
        if let Some(val) = gitgraph.get("commitSpacing").and_then(|v| v.as_f64()) {
            if !commit_step_set {
                let step = (val as f32 - config.layout.gitgraph.layout_offset).max(1.0);
                config.layout.gitgraph.commit_step = step;
            }
        }
    }
    if let Some(c4) = init.get("c4").and_then(|v| v.as_object()) {
        let get_f32 =
            |map: &serde_json::Map<String, serde_json::Value>, key: &str| -> Option<f32> {
                map.get(key).and_then(|val| match val {
                    serde_json::Value::Number(num) => num.as_f64().map(|v| v as f32),
                    serde_json::Value::String(text) => text.trim().parse::<f32>().ok(),
                    _ => None,
                })
            };
        let get_usize =
            |map: &serde_json::Map<String, serde_json::Value>, key: &str| -> Option<usize> {
                map.get(key).and_then(|val| match val {
                    serde_json::Value::Number(num) => num.as_u64().map(|v| v as usize),
                    serde_json::Value::String(text) => text.trim().parse::<usize>().ok(),
                    _ => None,
                })
            };
        let get_bool =
            |map: &serde_json::Map<String, serde_json::Value>, key: &str| -> Option<bool> {
                map.get(key).and_then(|val| val.as_bool())
            };
        let get_string =
            |map: &serde_json::Map<String, serde_json::Value>, key: &str| -> Option<String> {
                map.get(key)
                    .and_then(|val| val.as_str())
                    .map(|val| val.to_string())
            };
        let get_num_or_string_f32 =
            |map: &serde_json::Map<String, serde_json::Value>, key: &str| -> Option<f32> {
                map.get(key).and_then(|val| match val {
                    serde_json::Value::Number(num) => num.as_f64().map(|v| v as f32),
                    serde_json::Value::String(text) => text.trim().parse::<f32>().ok(),
                    _ => None,
                })
            };
        let get_num_or_string_string =
            |map: &serde_json::Map<String, serde_json::Value>, key: &str| -> Option<String> {
                map.get(key).and_then(|val| match val {
                    serde_json::Value::String(text) => Some(text.to_string()),
                    serde_json::Value::Number(num) => num.as_f64().map(|v| v.to_string()),
                    _ => None,
                })
            };

        if let Some(val) = get_bool(c4, "useMaxWidth") {
            config.layout.c4.use_max_width = val;
        }
        if let Some(val) = get_f32(c4, "diagramMarginX") {
            config.layout.c4.diagram_margin_x = val;
        }
        if let Some(val) = get_f32(c4, "diagramMarginY") {
            config.layout.c4.diagram_margin_y = val;
        }
        if let Some(val) = get_f32(c4, "c4ShapeMargin") {
            config.layout.c4.c4_shape_margin = val;
        }
        if let Some(val) = get_f32(c4, "c4ShapePadding") {
            config.layout.c4.c4_shape_padding = val;
        }
        if let Some(val) = get_f32(c4, "width") {
            config.layout.c4.width = val;
        }
        if let Some(val) = get_f32(c4, "height") {
            config.layout.c4.height = val;
        }
        if let Some(val) = get_f32(c4, "boxMargin") {
            config.layout.c4.box_margin = val;
        }
        if let Some(val) = get_usize(c4, "c4ShapeInRow") {
            config.layout.c4.c4_shape_in_row = val;
        }
        if let Some(val) = get_f32(c4, "nextLinePaddingX") {
            config.layout.c4.next_line_padding_x = val;
        }
        if let Some(val) = get_usize(c4, "c4BoundaryInRow") {
            config.layout.c4.c4_boundary_in_row = val;
        }
        if let Some(val) = get_bool(c4, "wrap") {
            config.layout.c4.wrap = val;
        }
        if let Some(val) = get_f32(c4, "wrapPadding") {
            config.layout.c4.wrap_padding = val;
        }
        if let Some(val) = get_f32(c4, "textLineHeight") {
            config.layout.c4.text_line_height = val;
        }
        if let Some(val) = get_f32(c4, "textLineHeightSmallAdd") {
            config.layout.c4.text_line_height_small_add = val;
        }
        if let Some(val) = get_f32(c4, "textLineHeightSmallThreshold") {
            config.layout.c4.text_line_height_small_threshold = val;
        }
        if let Some(val) = get_f32(c4, "shapeCornerRadius") {
            config.layout.c4.shape_corner_radius = val;
        }
        if let Some(val) = get_f32(c4, "shapeStrokeWidth") {
            config.layout.c4.shape_stroke_width = val;
        }
        if let Some(val) = get_f32(c4, "boundaryCornerRadius") {
            config.layout.c4.boundary_corner_radius = val;
        }
        if let Some(val) = get_f32(c4, "personIconSize") {
            config.layout.c4.person_icon_size = val;
        }
        if let Some(val) = get_f32(c4, "dbEllipseHeight") {
            config.layout.c4.db_ellipse_height = val;
        }
        if let Some(val) = get_f32(c4, "queueCurveRadius") {
            config.layout.c4.queue_curve_radius = val;
        }
        if let Some(val) = get_string(c4, "boundaryStroke") {
            config.layout.c4.boundary_stroke = val;
        }
        if let Some(val) = get_string(c4, "boundaryDasharray") {
            config.layout.c4.boundary_dasharray = val;
        }
        if let Some(val) = get_f32(c4, "boundaryStrokeWidth") {
            config.layout.c4.boundary_stroke_width = val;
        }
        if let Some(val) = get_string(c4, "boundaryFill") {
            config.layout.c4.boundary_fill = val;
        }
        if let Some(val) = get_f32(c4, "boundaryFillOpacity") {
            config.layout.c4.boundary_fill_opacity = val;
        }
        if let Some(val) = get_num_or_string_f32(c4, "personFontSize") {
            config.layout.c4.person_font_size = val;
        }
        if let Some(val) = get_string(c4, "personFontFamily") {
            config.layout.c4.person_font_family = val;
        }
        if let Some(val) = get_num_or_string_string(c4, "personFontWeight") {
            config.layout.c4.person_font_weight = val;
        }
        if let Some(val) = get_num_or_string_f32(c4, "externalPersonFontSize") {
            config.layout.c4.external_person_font_size = val;
        }
        if let Some(val) = get_string(c4, "externalPersonFontFamily") {
            config.layout.c4.external_person_font_family = val;
        }
        if let Some(val) = get_num_or_string_string(c4, "externalPersonFontWeight") {
            config.layout.c4.external_person_font_weight = val;
        }
        if let Some(val) = get_num_or_string_f32(c4, "systemFontSize") {
            config.layout.c4.system_font_size = val;
        }
        if let Some(val) = get_string(c4, "systemFontFamily") {
            config.layout.c4.system_font_family = val;
        }
        if let Some(val) = get_num_or_string_string(c4, "systemFontWeight") {
            config.layout.c4.system_font_weight = val;
        }
        if let Some(val) = get_num_or_string_f32(c4, "externalSystemFontSize") {
            config.layout.c4.external_system_font_size = val;
        }
        if let Some(val) = get_string(c4, "externalSystemFontFamily") {
            config.layout.c4.external_system_font_family = val;
        }
        if let Some(val) = get_num_or_string_string(c4, "externalSystemFontWeight") {
            config.layout.c4.external_system_font_weight = val;
        }
        if let Some(val) = get_num_or_string_f32(c4, "systemDbFontSize") {
            config.layout.c4.system_db_font_size = val;
        }
        if let Some(val) = get_string(c4, "systemDbFontFamily") {
            config.layout.c4.system_db_font_family = val;
        }
        if let Some(val) = get_num_or_string_string(c4, "systemDbFontWeight") {
            config.layout.c4.system_db_font_weight = val;
        }
        if let Some(val) = get_num_or_string_f32(c4, "externalSystemDbFontSize") {
            config.layout.c4.external_system_db_font_size = val;
        }
        if let Some(val) = get_string(c4, "externalSystemDbFontFamily") {
            config.layout.c4.external_system_db_font_family = val;
        }
        if let Some(val) = get_num_or_string_string(c4, "externalSystemDbFontWeight") {
            config.layout.c4.external_system_db_font_weight = val;
        }
        if let Some(val) = get_num_or_string_f32(c4, "systemQueueFontSize") {
            config.layout.c4.system_queue_font_size = val;
        }
        if let Some(val) = get_string(c4, "systemQueueFontFamily") {
            config.layout.c4.system_queue_font_family = val;
        }
        if let Some(val) = get_num_or_string_string(c4, "systemQueueFontWeight") {
            config.layout.c4.system_queue_font_weight = val;
        }
        if let Some(val) = get_num_or_string_f32(c4, "externalSystemQueueFontSize") {
            config.layout.c4.external_system_queue_font_size = val;
        }
        if let Some(val) = get_string(c4, "externalSystemQueueFontFamily") {
            config.layout.c4.external_system_queue_font_family = val;
        }
        if let Some(val) = get_num_or_string_string(c4, "externalSystemQueueFontWeight") {
            config.layout.c4.external_system_queue_font_weight = val;
        }
        if let Some(val) = get_num_or_string_f32(c4, "boundaryFontSize") {
            config.layout.c4.boundary_font_size = val;
        }
        if let Some(val) = get_string(c4, "boundaryFontFamily") {
            config.layout.c4.boundary_font_family = val;
        }
        if let Some(val) = get_num_or_string_string(c4, "boundaryFontWeight") {
            config.layout.c4.boundary_font_weight = val;
        }
        if let Some(val) = get_num_or_string_f32(c4, "messageFontSize") {
            config.layout.c4.message_font_size = val;
        }
        if let Some(val) = get_string(c4, "messageFontFamily") {
            config.layout.c4.message_font_family = val;
        }
        if let Some(val) = get_num_or_string_string(c4, "messageFontWeight") {
            config.layout.c4.message_font_weight = val;
        }
        if let Some(val) = get_num_or_string_f32(c4, "containerFontSize") {
            config.layout.c4.container_font_size = val;
        }
        if let Some(val) = get_string(c4, "containerFontFamily") {
            config.layout.c4.container_font_family = val;
        }
        if let Some(val) = get_num_or_string_string(c4, "containerFontWeight") {
            config.layout.c4.container_font_weight = val;
        }
        if let Some(val) = get_num_or_string_f32(c4, "externalContainerFontSize") {
            config.layout.c4.external_container_font_size = val;
        }
        if let Some(val) = get_string(c4, "externalContainerFontFamily") {
            config.layout.c4.external_container_font_family = val;
        }
        if let Some(val) = get_num_or_string_string(c4, "externalContainerFontWeight") {
            config.layout.c4.external_container_font_weight = val;
        }
        if let Some(val) = get_num_or_string_f32(c4, "containerDbFontSize") {
            config.layout.c4.container_db_font_size = val;
        }
        if let Some(val) = get_string(c4, "containerDbFontFamily") {
            config.layout.c4.container_db_font_family = val;
        }
        if let Some(val) = get_num_or_string_string(c4, "containerDbFontWeight") {
            config.layout.c4.container_db_font_weight = val;
        }
        if let Some(val) = get_num_or_string_f32(c4, "externalContainerDbFontSize") {
            config.layout.c4.external_container_db_font_size = val;
        }
        if let Some(val) = get_string(c4, "externalContainerDbFontFamily") {
            config.layout.c4.external_container_db_font_family = val;
        }
        if let Some(val) = get_num_or_string_string(c4, "externalContainerDbFontWeight") {
            config.layout.c4.external_container_db_font_weight = val;
        }
        if let Some(val) = get_num_or_string_f32(c4, "containerQueueFontSize") {
            config.layout.c4.container_queue_font_size = val;
        }
        if let Some(val) = get_string(c4, "containerQueueFontFamily") {
            config.layout.c4.container_queue_font_family = val;
        }
        if let Some(val) = get_num_or_string_string(c4, "containerQueueFontWeight") {
            config.layout.c4.container_queue_font_weight = val;
        }
        if let Some(val) = get_num_or_string_f32(c4, "externalContainerQueueFontSize") {
            config.layout.c4.external_container_queue_font_size = val;
        }
        if let Some(val) = get_string(c4, "externalContainerQueueFontFamily") {
            config.layout.c4.external_container_queue_font_family = val;
        }
        if let Some(val) = get_num_or_string_string(c4, "externalContainerQueueFontWeight") {
            config.layout.c4.external_container_queue_font_weight = val;
        }
        if let Some(val) = get_num_or_string_f32(c4, "componentFontSize") {
            config.layout.c4.component_font_size = val;
        }
        if let Some(val) = get_string(c4, "componentFontFamily") {
            config.layout.c4.component_font_family = val;
        }
        if let Some(val) = get_num_or_string_string(c4, "componentFontWeight") {
            config.layout.c4.component_font_weight = val;
        }
        if let Some(val) = get_num_or_string_f32(c4, "externalComponentFontSize") {
            config.layout.c4.external_component_font_size = val;
        }
        if let Some(val) = get_string(c4, "externalComponentFontFamily") {
            config.layout.c4.external_component_font_family = val;
        }
        if let Some(val) = get_num_or_string_string(c4, "externalComponentFontWeight") {
            config.layout.c4.external_component_font_weight = val;
        }
        if let Some(val) = get_num_or_string_f32(c4, "componentDbFontSize") {
            config.layout.c4.component_db_font_size = val;
        }
        if let Some(val) = get_string(c4, "componentDbFontFamily") {
            config.layout.c4.component_db_font_family = val;
        }
        if let Some(val) = get_num_or_string_string(c4, "componentDbFontWeight") {
            config.layout.c4.component_db_font_weight = val;
        }
        if let Some(val) = get_num_or_string_f32(c4, "externalComponentDbFontSize") {
            config.layout.c4.external_component_db_font_size = val;
        }
        if let Some(val) = get_string(c4, "externalComponentDbFontFamily") {
            config.layout.c4.external_component_db_font_family = val;
        }
        if let Some(val) = get_num_or_string_string(c4, "externalComponentDbFontWeight") {
            config.layout.c4.external_component_db_font_weight = val;
        }
        if let Some(val) = get_num_or_string_f32(c4, "componentQueueFontSize") {
            config.layout.c4.component_queue_font_size = val;
        }
        if let Some(val) = get_string(c4, "componentQueueFontFamily") {
            config.layout.c4.component_queue_font_family = val;
        }
        if let Some(val) = get_num_or_string_string(c4, "componentQueueFontWeight") {
            config.layout.c4.component_queue_font_weight = val;
        }
        if let Some(val) = get_num_or_string_f32(c4, "externalComponentQueueFontSize") {
            config.layout.c4.external_component_queue_font_size = val;
        }
        if let Some(val) = get_string(c4, "externalComponentQueueFontFamily") {
            config.layout.c4.external_component_queue_font_family = val;
        }
        if let Some(val) = get_num_or_string_string(c4, "externalComponentQueueFontWeight") {
            config.layout.c4.external_component_queue_font_weight = val;
        }
        if let Some(val) = get_string(c4, "personBgColor") {
            config.layout.c4.person_bg_color = val;
        }
        if let Some(val) = get_string(c4, "personBorderColor") {
            config.layout.c4.person_border_color = val;
        }
        if let Some(val) = get_string(c4, "externalPersonBgColor") {
            config.layout.c4.external_person_bg_color = val;
        }
        if let Some(val) = get_string(c4, "externalPersonBorderColor") {
            config.layout.c4.external_person_border_color = val;
        }
        if let Some(val) = get_string(c4, "systemBgColor") {
            config.layout.c4.system_bg_color = val;
        }
        if let Some(val) = get_string(c4, "systemBorderColor") {
            config.layout.c4.system_border_color = val;
        }
        if let Some(val) = get_string(c4, "systemDbBgColor") {
            config.layout.c4.system_db_bg_color = val;
        }
        if let Some(val) = get_string(c4, "systemDbBorderColor") {
            config.layout.c4.system_db_border_color = val;
        }
        if let Some(val) = get_string(c4, "systemQueueBgColor") {
            config.layout.c4.system_queue_bg_color = val;
        }
        if let Some(val) = get_string(c4, "systemQueueBorderColor") {
            config.layout.c4.system_queue_border_color = val;
        }
        if let Some(val) = get_string(c4, "externalSystemBgColor") {
            config.layout.c4.external_system_bg_color = val;
        }
        if let Some(val) = get_string(c4, "externalSystemBorderColor") {
            config.layout.c4.external_system_border_color = val;
        }
        if let Some(val) = get_string(c4, "externalSystemDbBgColor") {
            config.layout.c4.external_system_db_bg_color = val;
        }
        if let Some(val) = get_string(c4, "externalSystemDbBorderColor") {
            config.layout.c4.external_system_db_border_color = val;
        }
        if let Some(val) = get_string(c4, "externalSystemQueueBgColor") {
            config.layout.c4.external_system_queue_bg_color = val;
        }
        if let Some(val) = get_string(c4, "externalSystemQueueBorderColor") {
            config.layout.c4.external_system_queue_border_color = val;
        }
        if let Some(val) = get_string(c4, "containerBgColor") {
            config.layout.c4.container_bg_color = val;
        }
        if let Some(val) = get_string(c4, "containerBorderColor") {
            config.layout.c4.container_border_color = val;
        }
        if let Some(val) = get_string(c4, "containerDbBgColor") {
            config.layout.c4.container_db_bg_color = val;
        }
        if let Some(val) = get_string(c4, "containerDbBorderColor") {
            config.layout.c4.container_db_border_color = val;
        }
        if let Some(val) = get_string(c4, "containerQueueBgColor") {
            config.layout.c4.container_queue_bg_color = val;
        }
        if let Some(val) = get_string(c4, "containerQueueBorderColor") {
            config.layout.c4.container_queue_border_color = val;
        }
        if let Some(val) = get_string(c4, "externalContainerBgColor") {
            config.layout.c4.external_container_bg_color = val;
        }
        if let Some(val) = get_string(c4, "externalContainerBorderColor") {
            config.layout.c4.external_container_border_color = val;
        }
        if let Some(val) = get_string(c4, "externalContainerDbBgColor") {
            config.layout.c4.external_container_db_bg_color = val;
        }
        if let Some(val) = get_string(c4, "externalContainerDbBorderColor") {
            config.layout.c4.external_container_db_border_color = val;
        }
        if let Some(val) = get_string(c4, "externalContainerQueueBgColor") {
            config.layout.c4.external_container_queue_bg_color = val;
        }
        if let Some(val) = get_string(c4, "externalContainerQueueBorderColor") {
            config.layout.c4.external_container_queue_border_color = val;
        }
        if let Some(val) = get_string(c4, "componentBgColor") {
            config.layout.c4.component_bg_color = val;
        }
        if let Some(val) = get_string(c4, "componentBorderColor") {
            config.layout.c4.component_border_color = val;
        }
        if let Some(val) = get_string(c4, "componentDbBgColor") {
            config.layout.c4.component_db_bg_color = val;
        }
        if let Some(val) = get_string(c4, "componentDbBorderColor") {
            config.layout.c4.component_db_border_color = val;
        }
        if let Some(val) = get_string(c4, "componentQueueBgColor") {
            config.layout.c4.component_queue_bg_color = val;
        }
        if let Some(val) = get_string(c4, "componentQueueBorderColor") {
            config.layout.c4.component_queue_border_color = val;
        }
        if let Some(val) = get_string(c4, "externalComponentBgColor") {
            config.layout.c4.external_component_bg_color = val;
        }
        if let Some(val) = get_string(c4, "externalComponentBorderColor") {
            config.layout.c4.external_component_border_color = val;
        }
        if let Some(val) = get_string(c4, "externalComponentDbBgColor") {
            config.layout.c4.external_component_db_bg_color = val;
        }
        if let Some(val) = get_string(c4, "externalComponentDbBorderColor") {
            config.layout.c4.external_component_db_border_color = val;
        }
        if let Some(val) = get_string(c4, "externalComponentQueueBgColor") {
            config.layout.c4.external_component_queue_bg_color = val;
        }
        if let Some(val) = get_string(c4, "externalComponentQueueBorderColor") {
            config.layout.c4.external_component_queue_border_color = val;
        }
    }
    config.render.background = config.theme.background.clone();
    config
}
