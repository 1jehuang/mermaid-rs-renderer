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
    /// mermaid.js's `venn1`…`venn8`. Empty, as in every built-in theme, derives
    /// them from the primary, secondary and tertiary colours.
    #[serde(default)]
    pub venn_colors: Vec<String>,
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
        // mermaid-js theme-default.js: tertiaryColor = adjust(primaryColor, { h: -160 })
        let tertiary_color = adjust_color(&primary_color, -160.0, 0.0, 0.0);
        let pie_colors = default_pie_colors(&primary_color, &secondary_color, &tertiary_color);
        Self {
            font_family: "'trebuchet ms', verdana, arial, \"DejaVu Sans\", \"Liberation Sans\", sans-serif, \"Noto Color Emoji\", \"Apple Color Emoji\", \"Segoe UI Emoji\"".to_string(),
            font_size: 16.0,
            primary_color,
            primary_text_color: "#333333".to_string(),
            primary_border_color: "#7B88A8".to_string(),
            line_color: "#2F3B4D".to_string(),
            secondary_color,
            tertiary_color,
            edge_label_background: "rgba(248,250,252, 0.92)".to_string(),
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
            git_branch_label_colors: MERMAID_GIT_BRANCH_LABEL_COLORS.map(|value| value.to_string()),
            git_commit_label_color: MERMAID_GIT_COMMIT_LABEL_COLOR.to_string(),
            git_commit_label_background: MERMAID_GIT_COMMIT_LABEL_BG.to_string(),
            git_tag_label_color: MERMAID_GIT_TAG_LABEL_COLOR.to_string(),
            git_tag_label_background: MERMAID_GIT_TAG_LABEL_BG.to_string(),
            git_tag_label_border: MERMAID_GIT_TAG_LABEL_BORDER.to_string(),
            pie_colors,
            venn_colors: Vec::new(),
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
        let primary_color = "#F8FAFC".to_string();
        let secondary_color = "#E2E8F0".to_string();
        let tertiary_color = "#FFFFFF".to_string();
        let pie_colors = default_pie_colors(&primary_color, &secondary_color, &tertiary_color);
        Self {
            font_family: "Inter, ui-sans-serif, system-ui, -apple-system, \"Segoe UI\", \"DejaVu Sans\", \"Liberation Sans\", sans-serif, \"Noto Color Emoji\", \"Apple Color Emoji\", \"Segoe UI Emoji\""
                .to_string(),
            font_size: 14.0,
            primary_color,
            primary_text_color: "#0F172A".to_string(),
            primary_border_color: "#94A3B8".to_string(),
            line_color: "#64748B".to_string(),
            secondary_color,
            tertiary_color,
            edge_label_background: "#FFFFFF".to_string(),
            cluster_background: "#F1F5F9".to_string(),
            cluster_border: "#CBD5E1".to_string(),
            background: "#FFFFFF".to_string(),
            sequence_actor_fill: "#F8FAFC".to_string(),
            sequence_actor_border: "#94A3B8".to_string(),
            sequence_actor_line: "#64748B".to_string(),
            sequence_note_fill: "#FFF7ED".to_string(),
            sequence_note_border: "#FDBA74".to_string(),
            sequence_activation_fill: "#E2E8F0".to_string(),
            sequence_activation_border: "#94A3B8".to_string(),
            text_color: "#0F172A".to_string(),
            git_colors: MERMAID_GIT_COLORS.map(|value| value.to_string()),
            git_inv_colors: MERMAID_GIT_INV_COLORS.map(|value| value.to_string()),
            git_branch_label_colors: MERMAID_GIT_BRANCH_LABEL_COLORS.map(|value| value.to_string()),
            git_commit_label_color: MERMAID_GIT_COMMIT_LABEL_COLOR.to_string(),
            git_commit_label_background: MERMAID_GIT_COMMIT_LABEL_BG.to_string(),
            git_tag_label_color: MERMAID_GIT_TAG_LABEL_COLOR.to_string(),
            git_tag_label_background: MERMAID_GIT_TAG_LABEL_BG.to_string(),
            git_tag_label_border: MERMAID_GIT_TAG_LABEL_BORDER.to_string(),
            pie_colors,
            venn_colors: Vec::new(),
            pie_title_text_size: 25.0,
            pie_title_text_color: "#0F172A".to_string(),
            pie_section_text_size: 17.0,
            pie_section_text_color: "#0F172A".to_string(),
            pie_legend_text_size: 17.0,
            pie_legend_text_color: "#0F172A".to_string(),
            pie_stroke_color: "#334155".to_string(),
            pie_stroke_width: 1.6,
            pie_outer_stroke_width: 1.6,
            pie_outer_stroke_color: "#CBD5E1".to_string(),
            pie_opacity: 0.85,
        }
    }

    /// Mermaid `dark` theme, ported from mermaid-js `theme-dark.js`.
    /// Base colors: background #333, primaryColor #1f2020, with derived
    /// values precomputed (khroma lighten/invert/adjust equivalents).
    pub fn dark() -> Self {
        let primary_color = "#1f2020".to_string();
        // lighten(primary, 16)
        let secondary_color = "#474949".to_string();
        // adjust(primary, { h: -160 })
        let tertiary_color = "#201f1f".to_string();
        // theme-dark.js overrides cScale1..12 with a fixed dark palette and
        // maps pie1..pie12 straight onto it.
        let pie_colors = [
            "#0b0000", "#4d1037", "#3f5258", "#4f2f1b", "#6e0a0a", "#3b0048", "#995a01", "#154706",
            "#161722", "#00296f", "#01629c", "#010029",
        ]
        .map(|value| value.to_string());
        let git_colors = [
            // git0 = lighten(secondary, 20); git1..7 = lighten(cScale2.., 10-20)
            "#797d7d", "#a12273", "#6a8993", "#9b5c35", "#cc1212", "#65007b", "#cc7801", "#31a50e",
        ]
        .map(|value| value.to_string());
        let git_inv_colors = [
            "#868282", "#5edd8c", "#95766c", "#64a3ca", "#33eded", "#9aff84", "#3387fe", "#ce5af1",
        ]
        .map(|value| value.to_string());
        Self {
            font_family: "'trebuchet ms', verdana, arial, \"DejaVu Sans\", \"Liberation Sans\", sans-serif, \"Noto Color Emoji\", \"Apple Color Emoji\", \"Segoe UI Emoji\"".to_string(),
            font_size: 16.0,
            primary_color,
            // invert(primaryColor)
            primary_text_color: "#e0dfdf".to_string(),
            // invert(background)
            primary_border_color: "#cccccc".to_string(),
            // mainContrastColor
            line_color: "lightgrey".to_string(),
            secondary_color: secondary_color.clone(),
            tertiary_color,
            // lighten(labelBackground #181818, 25)
            edge_label_background: "#585858".to_string(),
            cluster_background: "#302f3d".to_string(),
            // border2 = rgba(255,255,255,0.25)
            cluster_border: "rgba(255, 255, 255, 0.25)".to_string(),
            background: "#333333".to_string(),
            sequence_actor_fill: "#1f2020".to_string(),
            sequence_actor_border: "#cccccc".to_string(),
            sequence_actor_line: "#cccccc".to_string(),
            // noteBkgColor = secondBkg
            sequence_note_fill: secondary_color.clone(),
            // mkBorder(secondaryColor, dark)
            sequence_note_border: "#626262".to_string(),
            sequence_activation_fill: secondary_color,
            sequence_activation_border: "#cccccc".to_string(),
            text_color: "#ccc".to_string(),
            git_colors,
            git_inv_colors,
            git_branch_label_colors: [
                "#2c2c2c", "lightgrey", "lightgrey", "#2c2c2c", "lightgrey", "lightgrey",
                "lightgrey", "lightgrey",
            ]
            .map(|value| value.to_string()),
            // invert(secondaryColor)
            git_commit_label_color: "#b8b6b6".to_string(),
            git_commit_label_background: "#474949".to_string(),
            // tagLabelColor = primaryTextColor, background = primaryColor
            git_tag_label_color: "#e0dfdf".to_string(),
            git_tag_label_background: "#1f2020".to_string(),
            git_tag_label_border: "#cccccc".to_string(),
            pie_colors,
            venn_colors: Vec::new(),
            pie_title_text_size: 25.0,
            pie_title_text_color: "lightgrey".to_string(),
            pie_section_text_size: 17.0,
            pie_section_text_color: "#ccc".to_string(),
            pie_legend_text_size: 17.0,
            pie_legend_text_color: "lightgrey".to_string(),
            pie_stroke_color: "#000000".to_string(),
            pie_stroke_width: 2.0,
            pie_outer_stroke_width: 2.0,
            pie_outer_stroke_color: "#000000".to_string(),
            pie_opacity: 0.7,
        }
    }

    /// Mermaid `forest` theme, ported from mermaid-js `theme-forest.js`.
    /// Base colors: primaryColor #cde498, secondaryColor #cdffb2.
    pub fn forest() -> Self {
        let primary_color = "#cde498".to_string();
        let secondary_color = "#cdffb2".to_string();
        // lighten(primary, 10)
        let tertiary_color = "#e1efc0".to_string();
        // theme-forest.js pie1..pie12 derivation (adjust h/l on base colors).
        let pie_colors = [
            "#cde498", "#cdffb2", "#e1efc0", "#8cb42f", "#6aff19", "#33b52e", "#70d990", "#d99070",
            "#98cde4", "#1a6330", "#63301a", "#1a4d63",
        ]
        .map(|value| value.to_string());
        // git0..7 = darken(adjusted primary/secondary/tertiary, 25)
        let git_colors = [
            "#9bc834", "#7aff33", "#b1d55a", "#c8ab34", "#c86134", "#c83452", "#34c861", "#349bc8",
        ]
        .map(|value| value.to_string());
        let git_inv_colors = [
            "#6437cb", "#8500cc", "#4e2aa5", "#3754cb", "#379ecb", "#37cbad", "#cb379e", "#cb6437",
        ]
        .map(|value| value.to_string());
        Self {
            font_family: "'trebuchet ms', verdana, arial, \"DejaVu Sans\", \"Liberation Sans\", sans-serif, \"Noto Color Emoji\", \"Apple Color Emoji\", \"Segoe UI Emoji\"".to_string(),
            font_size: 16.0,
            primary_color: primary_color.clone(),
            // invert(primaryColor)
            primary_text_color: "#321b67".to_string(),
            // border1
            primary_border_color: "#13540c".to_string(),
            line_color: "green".to_string(),
            secondary_color,
            tertiary_color,
            edge_label_background: "#e8e8e8".to_string(),
            // clusterBkg = secondBkg
            cluster_background: "#cdffb2".to_string(),
            // border2
            cluster_border: "#6eaa49".to_string(),
            background: "#ffffff".to_string(),
            sequence_actor_fill: primary_color,
            // darken(mainBkg, 20)
            sequence_actor_border: "#a6cf47".to_string(),
            sequence_actor_line: "#a6cf47".to_string(),
            sequence_note_fill: "#fff5ad".to_string(),
            sequence_note_border: "#6eaa49".to_string(),
            sequence_activation_fill: "#f4f4f4".to_string(),
            sequence_activation_border: "#666666".to_string(),
            text_color: MERMAID_TEXT_COLOR.to_string(),
            git_colors,
            git_inv_colors,
            git_branch_label_colors: [
                "black", "black", "black", "black", "black", "black", "black", "black",
            ]
            .map(|value| value.to_string()),
            // invert(secondaryColor)
            git_commit_label_color: "#32004d".to_string(),
            git_commit_label_background: "#cdffb2".to_string(),
            // invert(primaryColor), primaryColor, mkBorder(primary)
            git_tag_label_color: "#321b67".to_string(),
            git_tag_label_background: "#cde498".to_string(),
            git_tag_label_border: "#abb594".to_string(),
            pie_colors,
            venn_colors: Vec::new(),
            pie_title_text_size: 25.0,
            pie_title_text_color: "black".to_string(),
            pie_section_text_size: 17.0,
            pie_section_text_color: MERMAID_TEXT_COLOR.to_string(),
            pie_legend_text_size: 17.0,
            pie_legend_text_color: "black".to_string(),
            pie_stroke_color: "#000000".to_string(),
            pie_stroke_width: 2.0,
            pie_outer_stroke_width: 2.0,
            pie_outer_stroke_color: "#000000".to_string(),
            pie_opacity: 0.7,
        }
    }

    /// Mermaid `neutral` theme, ported from mermaid-js `theme-neutral.js`.
    /// Greyscale palette suited for black-and-white printing.
    pub fn neutral() -> Self {
        let primary_color = "#eeeeee".to_string();
        // lighten(contrast #707070, 55)
        let secondary_color = "#fcfcfc".to_string();
        // adjust(primary, { h: -160 }) — grey, hue shift is a no-op
        let tertiary_color = "#eeeeee".to_string();
        // theme-neutral.js maps pie1..pie11 onto its fixed grey cScale1..11
        // and wraps pie12 back to cScale0 (#555).
        let pie_colors = [
            "#f4f4f4", "#555555", "#bbbbbb", "#777777", "#999999", "#dddddd", "#ffffff", "#dddddd",
            "#bbbbbb", "#999999", "#777777", "#555555",
        ]
        .map(|value| value.to_string());
        // git0 = darken(pie1, 25), git1..7 = pie2..8
        let git_colors = [
            "#b4b4b4", "#555555", "#bbbbbb", "#777777", "#999999", "#dddddd", "#ffffff", "#dddddd",
        ]
        .map(|value| value.to_string());
        let git_inv_colors = [
            "#4b4b4b", "#aaaaaa", "#444444", "#888888", "#666666", "#222222", "#000000", "#222222",
        ]
        .map(|value| value.to_string());
        Self {
            font_family: "'trebuchet ms', verdana, arial, \"DejaVu Sans\", \"Liberation Sans\", sans-serif, \"Noto Color Emoji\", \"Apple Color Emoji\", \"Segoe UI Emoji\"".to_string(),
            font_size: 16.0,
            primary_color: primary_color.clone(),
            // invert(primaryColor)
            primary_text_color: "#111111".to_string(),
            // mkBorder(primaryColor)
            primary_border_color: "#d4d4d4".to_string(),
            line_color: "#666666".to_string(),
            secondary_color: secondary_color.clone(),
            tertiary_color,
            edge_label_background: "white".to_string(),
            // clusterBkg = secondBkg = lighten(contrast, 55)
            cluster_background: "#fcfcfc".to_string(),
            // border2 = contrast
            cluster_border: "#707070".to_string(),
            background: "#ffffff".to_string(),
            sequence_actor_fill: primary_color,
            // lighten(border1 #999, 23)
            sequence_actor_border: "#d4d4d4".to_string(),
            sequence_actor_line: "#d4d4d4".to_string(),
            // note = #ffa in the source, but sequence notes render as #666/#fff
            sequence_note_fill: "#666666".to_string(),
            sequence_note_border: "#999999".to_string(),
            sequence_activation_fill: "#f4f4f4".to_string(),
            sequence_activation_border: "#666666".to_string(),
            text_color: MERMAID_TEXT_COLOR.to_string(),
            git_colors,
            git_inv_colors,
            git_branch_label_colors: [
                "black", "white", "black", "white", "black", "black", "black", "black",
            ]
            .map(|value| value.to_string()),
            // invert(secondaryColor)
            git_commit_label_color: "#030303".to_string(),
            git_commit_label_background: "#fcfcfc".to_string(),
            git_tag_label_color: "#111111".to_string(),
            git_tag_label_background: "#eeeeee".to_string(),
            git_tag_label_border: "#d4d4d4".to_string(),
            pie_colors,
            venn_colors: Vec::new(),
            pie_title_text_size: 25.0,
            // taskTextDarkColor = text = #333
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

    /// Resolve a named built-in theme preset. Accepted names match the
    /// mermaid `theme` values plus this renderer's own presets.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "modern" => Some(Self::modern()),
            "base" | "default" | "mermaid" => Some(Self::mermaid_default()),
            "dark" => Some(Self::dark()),
            "forest" => Some(Self::forest()),
            "neutral" => Some(Self::neutral()),
            _ => None,
        }
    }
}

/// Derive the 12-color pie palette the same way mermaid-js `theme-default.js`
/// derives `pie1..pie12` from the base colors. Unlike the previous
/// implementation this never yields duplicate slice colors when the tertiary
/// color equals the primary color (issue #69: "Dogs" and "Rats" rendered with
/// the same fill).
fn default_pie_colors(primary: &str, secondary: &str, tertiary: &str) -> [String; 12] {
    [
        primary.to_string(),
        secondary.to_string(),
        adjust_color(tertiary, 0.0, 0.0, -40.0),
        adjust_color(primary, 0.0, 0.0, -10.0),
        adjust_color(secondary, 0.0, 0.0, -30.0),
        adjust_color(tertiary, 0.0, 0.0, -20.0),
        adjust_color(primary, 60.0, 0.0, -20.0),
        adjust_color(primary, -60.0, 0.0, -40.0),
        adjust_color(primary, 120.0, 0.0, -40.0),
        adjust_color(primary, 60.0, 0.0, -40.0),
        adjust_color(primary, -90.0, 0.0, -40.0),
        adjust_color(primary, 120.0, 0.0, -30.0),
    ]
}

pub(crate) fn adjust_color(color: &str, delta_h: f32, delta_s: f32, delta_l: f32) -> String {
    let Some((h, s, l)) = parse_color_to_hsl(color) else {
        return color.to_string();
    };
    let mut h = h + delta_h;
    if h < 0.0 {
        h = (h % 360.0) + 360.0;
    } else if h >= 360.0 {
        h %= 360.0;
    }
    let s = (s + delta_s).clamp(0.0, 100.0);
    let l = (l + delta_l).clamp(0.0, 100.0);
    format!("hsl({:.10}, {:.10}%, {:.10}%)", h, s, l)
}

pub(crate) fn parse_color_to_hsl(color: &str) -> Option<(f32, f32, f32)> {
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
    if !hex.is_ascii() {
        return None;
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_rejects_multibyte_utf8() {
        // 3-byte char
        assert_eq!(parse_hex("#\u{1000}"), None);
        // 2-byte char inside a 6-byte string
        assert_eq!(parse_hex("#a\u{00FF}bcd"), None);
        // 2-byte char inside an 8-byte string
        assert_eq!(parse_hex("#abcde\u{0100}f"), None);
    }

    #[test]
    fn parse_hex_valid_colors() {
        assert_eq!(parse_hex("#fff"), Some((1.0, 1.0, 1.0)));
        assert_eq!(parse_hex("#ff0000"), Some((1.0, 0.0, 0.0)));
        assert_eq!(parse_hex("#00ff0080"), Some((0.0, 1.0, 0.0)));
    }

    #[test]
    fn theme_from_name_resolves_builtin_presets() {
        assert_eq!(Theme::from_name("dark").unwrap().background, "#333333");
        assert_eq!(Theme::from_name("forest").unwrap().primary_color, "#cde498");
        assert_eq!(
            Theme::from_name("neutral").unwrap().primary_color,
            "#eeeeee"
        );
        assert_eq!(
            Theme::from_name("default").unwrap().primary_color,
            "#ECECFF"
        );
        assert_eq!(
            Theme::from_name("DARK").unwrap().background,
            "#333333",
            "names should be case-insensitive"
        );
        assert!(Theme::from_name("no-such-theme").is_none());
    }

    #[test]
    fn builtin_theme_pie_palettes_have_no_adjacent_duplicates() {
        for theme in [
            Theme::mermaid_default(),
            Theme::modern(),
            Theme::dark(),
            Theme::forest(),
            Theme::neutral(),
        ] {
            for pair in theme.pie_colors.windows(2) {
                assert_ne!(
                    pair[0], pair[1],
                    "adjacent pie palette entries should differ: {:?}",
                    theme.pie_colors
                );
            }
            // The first three slices are the most common case; they must all
            // be distinct (issue #69).
            assert_ne!(theme.pie_colors[0], theme.pie_colors[2]);
            assert_ne!(theme.pie_colors[1], theme.pie_colors[2]);
        }
    }
}
