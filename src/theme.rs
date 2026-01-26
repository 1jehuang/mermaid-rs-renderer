use serde::{Deserialize, Serialize};

const MERMAID_GIT_COLORS: [&str; 8] = [
    "hsl(240, 100%, 46.2745098039%)",
    "hsl(60, 100%, 43.5294117647%)",
    "hsl(80, 100%, 46.2745098039%)",
    "hsl(210, 100%, 46.2745098039%)",
    "hsl(180, 100%, 46.2745098039%)",
    "hsl(150, 100%, 46.2745098039%)",
    "hsl(300, 100%, 46.2745098039%)",
    "hsl(0, 100%, 46.2745098039%)",
];

const MERMAID_GIT_INV_COLORS: [&str; 8] = [
    "hsl(60, 100%, 3.7254901961%)",
    "rgb(0, 0, 160.5)",
    "rgb(48.8333333334, 0, 146.5000000001)",
    "rgb(146.5000000001, 73.2500000001, 0)",
    "rgb(146.5000000001, 0, 0)",
    "rgb(146.5000000001, 0, 73.2500000001)",
    "rgb(0, 146.5000000001, 0)",
    "rgb(0, 146.5000000001, 146.5000000001)",
];

const MERMAID_GIT_BRANCH_LABEL_COLORS: [&str; 8] = [
    "#ffffff", "black", "black", "#ffffff", "black", "black", "black", "black",
];

const MERMAID_GIT_COMMIT_LABEL_COLOR: &str = "#000021";
const MERMAID_GIT_COMMIT_LABEL_BG: &str = "#ffffde";
const MERMAID_GIT_TAG_LABEL_COLOR: &str = "#131300";
const MERMAID_GIT_TAG_LABEL_BG: &str = "#ECECFF";
const MERMAID_GIT_TAG_LABEL_BORDER: &str = "hsl(240, 60%, 86.2745098039%)";
const MERMAID_TEXT_COLOR: &str = "#333";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub font_family: String,
    pub font_size: f32,
    pub primary_color: String,
    pub primary_text_color: String,
    pub primary_border_color: String,
    pub line_color: String,
    pub secondary_color: String,
    pub tertiary_color: String,
    pub edge_label_background: String,
    pub cluster_background: String,
    pub cluster_border: String,
    pub background: String,
    pub sequence_actor_fill: String,
    pub sequence_actor_border: String,
    pub sequence_actor_line: String,
    pub sequence_note_fill: String,
    pub sequence_note_border: String,
    pub sequence_activation_fill: String,
    pub sequence_activation_border: String,
    pub text_color: String,
    pub git_colors: [String; 8],
    pub git_inv_colors: [String; 8],
    pub git_branch_label_colors: [String; 8],
    pub git_commit_label_color: String,
    pub git_commit_label_background: String,
    pub git_tag_label_color: String,
    pub git_tag_label_background: String,
    pub git_tag_label_border: String,
    pub pie_colors: [String; 12],
    pub pie_title_text_size: f32,
    pub pie_title_text_color: String,
    pub pie_section_text_size: f32,
    pub pie_section_text_color: String,
    pub pie_legend_text_size: f32,
    pub pie_legend_text_color: String,
    pub pie_stroke_color: String,
    pub pie_stroke_width: f32,
    pub pie_outer_stroke_width: f32,
    pub pie_outer_stroke_color: String,
    pub pie_opacity: f32,
}

impl Theme {
    pub fn mermaid_default() -> Self {
        let primary_color = "#ECECFF".to_string();
        let secondary_color = "#FFFFDE".to_string();
        let tertiary_color = "#ECECFF".to_string();
        let pie_colors = default_pie_colors(&primary_color, &secondary_color, &tertiary_color);
        Self {
            font_family: "'trebuchet ms', verdana, arial, sans-serif".to_string(),
            font_size: 16.0,
            primary_color,
            primary_text_color: "#333333".to_string(),
            primary_border_color: "#9370DB".to_string(),
            line_color: "#333333".to_string(),
            secondary_color,
            tertiary_color,
            edge_label_background: "#FFFFFF".to_string(),
            cluster_background: "#FFFFDE".to_string(),
            cluster_border: "#AAAA33".to_string(),
            background: "#FFFFFF".to_string(),
            sequence_actor_fill: "#EAEAEA".to_string(),
            sequence_actor_border: "#666666".to_string(),
            sequence_actor_line: "#999999".to_string(),
            sequence_note_fill: "#FFF5AD".to_string(),
            sequence_note_border: "#AAAA33".to_string(),
            sequence_activation_fill: "#F4F4F4".to_string(),
            sequence_activation_border: "#666666".to_string(),
            text_color: MERMAID_TEXT_COLOR.to_string(),
            git_colors: MERMAID_GIT_COLORS.map(|value| value.to_string()),
            git_inv_colors: MERMAID_GIT_INV_COLORS.map(|value| value.to_string()),
            git_branch_label_colors: MERMAID_GIT_BRANCH_LABEL_COLORS
                .map(|value| value.to_string()),
            git_commit_label_color: MERMAID_GIT_COMMIT_LABEL_COLOR.to_string(),
            git_commit_label_background: MERMAID_GIT_COMMIT_LABEL_BG.to_string(),
            git_tag_label_color: MERMAID_GIT_TAG_LABEL_COLOR.to_string(),
            git_tag_label_background: MERMAID_GIT_TAG_LABEL_BG.to_string(),
            git_tag_label_border: MERMAID_GIT_TAG_LABEL_BORDER.to_string(),
            pie_colors,
            pie_title_text_size: 25.0,
            pie_title_text_color: MERMAID_TEXT_COLOR.to_string(),
            pie_section_text_size: 17.0,
            pie_section_text_color: MERMAID_TEXT_COLOR.to_string(),
            pie_legend_text_size: 17.0,
            pie_legend_text_color: MERMAID_TEXT_COLOR.to_string(),
            pie_stroke_color: "#000000".to_string(),
            pie_stroke_width: 2.0,
            pie_outer_stroke_width: 2.0,
            pie_outer_stroke_color: "#000000".to_string(),
            pie_opacity: 0.7,
        }
    }

    pub fn modern() -> Self {
        let primary_color = "#F8FAFF".to_string();
        let secondary_color = "#EEF2F8".to_string();
        let tertiary_color = "#FFFFFF".to_string();
        let pie_colors = default_pie_colors(&primary_color, &secondary_color, &tertiary_color);
        Self {
            font_family: "Inter, Segoe UI, system-ui, -apple-system, sans-serif".to_string(),
            font_size: 13.0,
            primary_color,
            primary_text_color: "#1C2430".to_string(),
            primary_border_color: "#C7D2E5".to_string(),
            line_color: "#7A8AA6".to_string(),
            secondary_color,
            tertiary_color,
            edge_label_background: "#FFFFFF".to_string(),
            cluster_background: "#F7FAFF".to_string(),
            cluster_border: "#D7E0F0".to_string(),
            background: "#FFFFFF".to_string(),
            sequence_actor_fill: "#F8FAFF".to_string(),
            sequence_actor_border: "#C7D2E5".to_string(),
            sequence_actor_line: "#7A8AA6".to_string(),
            sequence_note_fill: "#F7FAFF".to_string(),
            sequence_note_border: "#D7E0F0".to_string(),
            sequence_activation_fill: "#EEF2F8".to_string(),
            sequence_activation_border: "#7A8AA6".to_string(),
            text_color: "#1C2430".to_string(),
            git_colors: MERMAID_GIT_COLORS.map(|value| value.to_string()),
            git_inv_colors: MERMAID_GIT_INV_COLORS.map(|value| value.to_string()),
            git_branch_label_colors: MERMAID_GIT_BRANCH_LABEL_COLORS
                .map(|value| value.to_string()),
            git_commit_label_color: MERMAID_GIT_COMMIT_LABEL_COLOR.to_string(),
            git_commit_label_background: MERMAID_GIT_COMMIT_LABEL_BG.to_string(),
            git_tag_label_color: MERMAID_GIT_TAG_LABEL_COLOR.to_string(),
            git_tag_label_background: MERMAID_GIT_TAG_LABEL_BG.to_string(),
            git_tag_label_border: MERMAID_GIT_TAG_LABEL_BORDER.to_string(),
            pie_colors,
            pie_title_text_size: 25.0,
            pie_title_text_color: "#1C2430".to_string(),
            pie_section_text_size: 17.0,
            pie_section_text_color: "#1C2430".to_string(),
            pie_legend_text_size: 17.0,
            pie_legend_text_color: "#1C2430".to_string(),
            pie_stroke_color: "#000000".to_string(),
            pie_stroke_width: 2.0,
            pie_outer_stroke_width: 2.0,
            pie_outer_stroke_color: "#000000".to_string(),
            pie_opacity: 0.7,
        }
    }
}

fn default_pie_colors(primary: &str, secondary: &str, tertiary: &str) -> [String; 12] {
    [
        primary.to_string(),
        secondary.to_string(),
        tertiary.to_string(),
        adjust_color(primary, 0.0, 0.0, -10.0),
        adjust_color(secondary, 0.0, 0.0, -10.0),
        adjust_color(tertiary, 0.0, 0.0, -10.0),
        adjust_color(primary, 60.0, 0.0, -10.0),
        adjust_color(primary, -60.0, 0.0, -10.0),
        adjust_color(primary, 120.0, 0.0, 0.0),
        adjust_color(primary, 60.0, 0.0, -20.0),
        adjust_color(primary, -60.0, 0.0, -20.0),
        adjust_color(primary, 120.0, 0.0, -10.0),
    ]
}

fn adjust_color(color: &str, delta_h: f32, delta_s: f32, delta_l: f32) -> String {
    let Some((h, s, l)) = parse_color_to_hsl(color) else {
        return color.to_string();
    };
    let mut h = h + delta_h;
    if h < 0.0 {
        h = (h % 360.0) + 360.0;
    } else if h >= 360.0 {
        h = h % 360.0;
    }
    let s = (s + delta_s).clamp(0.0, 100.0);
    let l = (l + delta_l).clamp(0.0, 100.0);
    format!("hsl({:.10}, {:.10}%, {:.10}%)", h, s, l)
}

fn parse_color_to_hsl(color: &str) -> Option<(f32, f32, f32)> {
    let color = color.trim();
    if let Some(hsl) = parse_hsl(color) {
        return Some(hsl);
    }
    let rgb = parse_hex(color)?;
    Some(rgb_to_hsl(rgb.0, rgb.1, rgb.2))
}

fn parse_hsl(value: &str) -> Option<(f32, f32, f32)> {
    let value = value.trim();
    let open = value.find('(')?;
    let close = value.rfind(')')?;
    let prefix = value[..open].trim().to_ascii_lowercase();
    if prefix != "hsl" && prefix != "hsla" {
        return None;
    }
    let inner = &value[open + 1..close];
    let parts: Vec<&str> = inner.split(',').collect();
    if parts.len() < 3 {
        return None;
    }
    let h = parts[0].trim().parse::<f32>().ok()?;
    let s = parts[1].trim().trim_end_matches('%').parse::<f32>().ok()?;
    let l = parts[2].trim().trim_end_matches('%').parse::<f32>().ok()?;
    Some((h, s, l))
}

fn parse_hex(value: &str) -> Option<(f32, f32, f32)> {
    let hex = value.strip_prefix('#')?;
    let digits = match hex.len() {
        3 => {
            let mut expanded = String::new();
            for ch in hex.chars() {
                expanded.push(ch);
                expanded.push(ch);
            }
            expanded
        }
        6 => hex.to_string(),
        8 => hex[..6].to_string(),
        _ => return None,
    };
    let r = u8::from_str_radix(&digits[0..2], 16).ok()?;
    let g = u8::from_str_radix(&digits[2..4], 16).ok()?;
    let b = u8::from_str_radix(&digits[4..6], 16).ok()?;
    Some((r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0))
}

fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g.max(b));
    let min = r.min(g.min(b));
    let mut h = 0.0;
    let l = (max + min) / 2.0;
    let d = max - min;
    let s = if d == 0.0 {
        0.0
    } else {
        d / (1.0 - (2.0 * l - 1.0).abs())
    };
    if d != 0.0 {
        if max == r {
            h = ((g - b) / d) % 6.0;
        } else if max == g {
            h = (b - r) / d + 2.0;
        } else {
            h = (r - g) / d + 4.0;
        }
        h *= 60.0;
        if h < 0.0 {
            h += 360.0;
        }
    }
    (h, s * 100.0, l * 100.0)
}
