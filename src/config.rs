use crate::theme::Theme;
use serde::{Deserialize, Serialize};
use std::path::Path;

const MINDMAP_SECTION_COLORS: [&str; 12] = [
    "hsl(240, 100%, 76.2745098039%)",
    "hsl(60, 100%, 73.5294117647%)",
    "hsl(80, 100%, 76.2745098039%)",
    "hsl(270, 100%, 76.2745098039%)",
    "hsl(300, 100%, 76.2745098039%)",
    "hsl(330, 100%, 76.2745098039%)",
    "hsl(0, 100%, 76.2745098039%)",
    "hsl(30, 100%, 76.2745098039%)",
    "hsl(90, 100%, 76.2745098039%)",
    "hsl(150, 100%, 76.2745098039%)",
    "hsl(180, 100%, 76.2745098039%)",
    "hsl(210, 100%, 76.2745098039%)",
];

const MINDMAP_SECTION_LINE_COLORS: [&str; 12] = [
    "hsl(60, 100%, 86.2745098039%)",
    "hsl(240, 100%, 83.5294117647%)",
    "hsl(260, 100%, 86.2745098039%)",
    "hsl(90, 100%, 86.2745098039%)",
    "hsl(120, 100%, 86.2745098039%)",
    "hsl(150, 100%, 86.2745098039%)",
    "hsl(180, 100%, 86.2745098039%)",
    "hsl(210, 100%, 86.2745098039%)",
    "hsl(270, 100%, 86.2745098039%)",
    "hsl(330, 100%, 86.2745098039%)",
    "hsl(0, 100%, 86.2745098039%)",
    "hsl(30, 100%, 86.2745098039%)",
];

const MINDMAP_SECTION_LABEL_COLORS: [&str; 12] = [
    "#ffffff",
    "black",
    "black",
    "#ffffff",
    "black",
    "black",
    "black",
    "black",
    "black",
    "black",
    "black",
    "black",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementConfig {
    pub fill: String,
    pub box_stroke: String,
    pub box_stroke_width: f32,
    pub stroke: String,
    pub stroke_width: f32,
    pub label_color: String,
    pub divider_color: String,
    pub divider_width: f32,
    pub header_band_height: f32,
    pub header_line_gap: f32,
    pub label_padding_x: f32,
    pub label_padding_y: f32,
    pub edge_stroke: String,
    pub edge_dasharray: String,
    pub edge_stroke_width: f32,
    pub edge_label_color: String,
    pub edge_label_background: String,
    pub edge_label_padding_x: f32,
    pub edge_label_padding_y: f32,
    pub edge_label_brackets: bool,
    pub render_padding_x: f32,
    pub render_padding_y: f32,
}

impl Default for RequirementConfig {
    fn default() -> Self {
        Self {
            fill: "#ECECFF".to_string(),
            box_stroke: "#C7D2E5".to_string(),
            box_stroke_width: 1.0,
            stroke: "#9370DB".to_string(),
            stroke_width: 1.3,
            label_color: "#131300".to_string(),
            divider_color: "#9370DB".to_string(),
            divider_width: 1.3,
            header_band_height: 68.0,
            header_line_gap: 24.0,
            label_padding_x: 10.0,
            label_padding_y: 8.0,
            edge_stroke: "#333333".to_string(),
            edge_dasharray: "10,7".to_string(),
            edge_stroke_width: 1.0,
            edge_label_color: "#000000".to_string(),
            edge_label_background: "rgba(232,232,232, 0.8)".to_string(),
            edge_label_padding_x: 4.0,
            edge_label_padding_y: 2.0,
            edge_label_brackets: true,
            render_padding_x: 6.8,
            render_padding_y: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MindmapConfig {
    pub use_max_width: bool,
    pub padding: f32,
    pub max_node_width: f32,
    pub layout_algorithm: String,
    pub node_spacing: f32,
    pub rank_spacing: f32,
    pub node_spacing_multiplier: f32,
    pub rank_spacing_multiplier: f32,
    pub text_width_scale: f32,
    pub rounded_padding: f32,
    pub rect_padding: f32,
    pub circle_padding: f32,
    pub hexagon_padding_multiplier: f32,
    pub default_corner_radius: f32,
    pub edge_depth_base_width: f32,
    pub edge_depth_step: f32,
    pub divider_line_width: f32,
    pub section_colors: Vec<String>,
    pub section_label_colors: Vec<String>,
    pub section_line_colors: Vec<String>,
    pub root_fill: Option<String>,
    pub root_text: Option<String>,
}

impl Default for MindmapConfig {
    fn default() -> Self {
        Self {
            use_max_width: true,
            padding: 9.853,
            max_node_width: 200.0,
            layout_algorithm: "cose-bilkent".to_string(),
            node_spacing: 50.0,
            rank_spacing: 50.0,
            node_spacing_multiplier: 1.0,
            rank_spacing_multiplier: 1.07,
            text_width_scale: 1.0,
            rounded_padding: 15.0,
            rect_padding: 10.0,
            circle_padding: 10.271,
            hexagon_padding_multiplier: 2.0,
            default_corner_radius: 5.0,
            edge_depth_base_width: 17.0,
            edge_depth_step: -3.0,
            divider_line_width: 3.0,
            section_colors: MINDMAP_SECTION_COLORS
                .iter()
                .map(|value| value.to_string())
                .collect(),
            section_label_colors: MINDMAP_SECTION_LABEL_COLORS
                .iter()
                .map(|value| value.to_string())
                .collect(),
            section_line_colors: MINDMAP_SECTION_LINE_COLORS
                .iter()
                .map(|value| value.to_string())
                .collect(),
            root_fill: None,
            root_text: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitGraphConfig {
    pub diagram_padding: f32,
    pub title_top_margin: f32,
    pub use_max_width: bool,
    pub main_branch_name: String,
    pub main_branch_order: f32,
    pub show_commit_label: bool,
    pub show_branches: bool,
    pub rotate_commit_label: bool,
    pub parallel_commits: bool,
    pub commit_step: f32,
    pub layout_offset: f32,
    pub default_pos: f32,
    pub branch_spacing: f32,
    pub branch_spacing_rotate_extra: f32,
    pub branch_label_rotate_extra: f32,
    pub branch_label_translate_x: f32,
    pub branch_label_bg_offset_x: f32,
    pub branch_label_bg_offset_y: f32,
    pub branch_label_bg_pad_x: f32,
    pub branch_label_bg_pad_y: f32,
    pub branch_label_text_offset_x: f32,
    pub branch_label_text_offset_y: f32,
    pub branch_label_tb_bg_offset_x: f32,
    pub branch_label_tb_text_offset_x: f32,
    pub branch_label_tb_offset_y: f32,
    pub branch_label_bt_offset_y: f32,
    pub branch_label_corner_radius: f32,
    pub branch_label_font_size: f32,
    pub branch_label_line_height: f32,
    pub text_width_scale: f32,
    pub commit_label_font_size: f32,
    pub commit_label_line_height: f32,
    pub commit_label_offset_y: f32,
    pub commit_label_bg_offset_y: f32,
    pub commit_label_padding: f32,
    pub commit_label_bg_opacity: f32,
    pub commit_label_rotate_angle: f32,
    pub commit_label_rotate_translate_x_base: f32,
    pub commit_label_rotate_translate_x_scale: f32,
    pub commit_label_rotate_translate_x_width_offset: f32,
    pub commit_label_rotate_translate_y_base: f32,
    pub commit_label_rotate_translate_y_scale: f32,
    pub commit_label_tb_text_extra: f32,
    pub commit_label_tb_bg_extra: f32,
    pub commit_label_tb_text_offset_y: f32,
    pub commit_label_tb_bg_offset_y: f32,
    pub tag_label_font_size: f32,
    pub tag_label_line_height: f32,
    pub tag_text_offset_y: f32,
    pub tag_polygon_offset_y: f32,
    pub tag_spacing_y: f32,
    pub tag_padding_x: f32,
    pub tag_padding_y: f32,
    pub tag_hole_radius: f32,
    pub tag_rotate_translate: f32,
    pub tag_text_rotate_translate: f32,
    pub tag_rotate_angle: f32,
    pub tag_text_offset_x_tb: f32,
    pub tag_text_offset_y_tb: f32,
    pub arrow_reroute_radius: f32,
    pub arrow_radius: f32,
    pub lane_spacing: f32,
    pub lane_max_depth: usize,
    pub commit_radius: f32,
    pub merge_radius_outer: f32,
    pub merge_radius_inner: f32,
    pub highlight_outer_size: f32,
    pub highlight_inner_size: f32,
    pub reverse_cross_size: f32,
    pub reverse_stroke_width: f32,
    pub cherry_pick_dot_radius: f32,
    pub cherry_pick_dot_offset_x: f32,
    pub cherry_pick_dot_offset_y: f32,
    pub cherry_pick_stem_start_offset_y: f32,
    pub cherry_pick_stem_end_offset_y: f32,
    pub cherry_pick_stem_stroke_width: f32,
    pub cherry_pick_accent_color: String,
    pub arrow_stroke_width: f32,
    pub branch_stroke_width: f32,
    pub branch_dasharray: String,
}

impl Default for GitGraphConfig {
    fn default() -> Self {
        Self {
            diagram_padding: 8.0,
            title_top_margin: 25.0,
            use_max_width: true,
            main_branch_name: "main".to_string(),
            main_branch_order: 0.0,
            show_commit_label: true,
            show_branches: true,
            rotate_commit_label: true,
            parallel_commits: false,
            commit_step: 40.0,
            layout_offset: 10.0,
            default_pos: 30.0,
            branch_spacing: 50.0,
            branch_spacing_rotate_extra: 40.0,
            branch_label_rotate_extra: 30.0,
            branch_label_translate_x: -19.0,
            branch_label_bg_offset_x: 4.0,
            branch_label_bg_offset_y: 8.0,
            branch_label_bg_pad_x: 18.0,
            branch_label_bg_pad_y: 4.0,
            branch_label_text_offset_x: 14.0,
            branch_label_text_offset_y: -1.0,
            branch_label_tb_bg_offset_x: 10.0,
            branch_label_tb_text_offset_x: 5.0,
            branch_label_tb_offset_y: 0.0,
            branch_label_bt_offset_y: 0.0,
            branch_label_corner_radius: 4.0,
            branch_label_font_size: 0.0,
            branch_label_line_height: 1.54,
            text_width_scale: 1.0,
            commit_label_font_size: 10.0,
            commit_label_line_height: 1.2,
            commit_label_offset_y: 25.0,
            commit_label_bg_offset_y: 13.5,
            commit_label_padding: 2.0,
            commit_label_bg_opacity: 0.5,
            commit_label_rotate_angle: -45.0,
            commit_label_rotate_translate_x_base: -7.5,
            commit_label_rotate_translate_x_scale: 9.5 / 25.0,
            commit_label_rotate_translate_x_width_offset: 10.0,
            commit_label_rotate_translate_y_base: 10.0,
            commit_label_rotate_translate_y_scale: 8.5 / 25.0,
            commit_label_tb_text_extra: 16.0,
            commit_label_tb_bg_extra: 21.0,
            commit_label_tb_text_offset_y: -12.0,
            commit_label_tb_bg_offset_y: -12.0,
            tag_label_font_size: 10.0,
            tag_label_line_height: 1.2,
            tag_text_offset_y: 16.0,
            tag_polygon_offset_y: 19.2,
            tag_spacing_y: 20.0,
            tag_padding_x: 4.0,
            tag_padding_y: 2.0,
            tag_hole_radius: 1.5,
            tag_rotate_translate: 12.0,
            tag_text_rotate_translate: 14.0,
            tag_rotate_angle: 45.0,
            tag_text_offset_x_tb: 5.0,
            tag_text_offset_y_tb: 3.0,
            arrow_reroute_radius: 10.0,
            arrow_radius: 20.0,
            lane_spacing: 10.0,
            lane_max_depth: 5,
            commit_radius: 10.0,
            merge_radius_outer: 9.0,
            merge_radius_inner: 6.0,
            highlight_outer_size: 20.0,
            highlight_inner_size: 12.0,
            reverse_cross_size: 5.0,
            reverse_stroke_width: 3.0,
            cherry_pick_dot_radius: 2.75,
            cherry_pick_dot_offset_x: 3.0,
            cherry_pick_dot_offset_y: 2.0,
            cherry_pick_stem_start_offset_y: 1.0,
            cherry_pick_stem_end_offset_y: -5.0,
            cherry_pick_stem_stroke_width: 1.0,
            cherry_pick_accent_color: "#fff".to_string(),
            arrow_stroke_width: 8.0,
            branch_stroke_width: 1.0,
            branch_dasharray: "2".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C4Config {
    pub use_max_width: bool,
    pub diagram_margin_x: f32,
    pub diagram_margin_y: f32,
    pub c4_shape_margin: f32,
    pub c4_shape_padding: f32,
    pub width: f32,
    pub height: f32,
    pub box_margin: f32,
    pub c4_shape_in_row: usize,
    pub next_line_padding_x: f32,
    pub c4_boundary_in_row: usize,
    pub wrap: bool,
    pub wrap_padding: f32,
    pub text_line_height: f32,
    pub text_line_height_small_add: f32,
    pub text_line_height_small_threshold: f32,
    pub shape_corner_radius: f32,
    pub shape_stroke_width: f32,
    pub boundary_corner_radius: f32,
    pub person_icon_size: f32,
    pub db_ellipse_height: f32,
    pub queue_curve_radius: f32,
    pub boundary_stroke: String,
    pub boundary_dasharray: String,
    pub boundary_stroke_width: f32,
    pub boundary_fill: String,
    pub boundary_fill_opacity: f32,
    pub person_font_size: f32,
    pub person_font_family: String,
    pub person_font_weight: String,
    pub external_person_font_size: f32,
    pub external_person_font_family: String,
    pub external_person_font_weight: String,
    pub system_font_size: f32,
    pub system_font_family: String,
    pub system_font_weight: String,
    pub external_system_font_size: f32,
    pub external_system_font_family: String,
    pub external_system_font_weight: String,
    pub system_db_font_size: f32,
    pub system_db_font_family: String,
    pub system_db_font_weight: String,
    pub external_system_db_font_size: f32,
    pub external_system_db_font_family: String,
    pub external_system_db_font_weight: String,
    pub system_queue_font_size: f32,
    pub system_queue_font_family: String,
    pub system_queue_font_weight: String,
    pub external_system_queue_font_size: f32,
    pub external_system_queue_font_family: String,
    pub external_system_queue_font_weight: String,
    pub boundary_font_size: f32,
    pub boundary_font_family: String,
    pub boundary_font_weight: String,
    pub message_font_size: f32,
    pub message_font_family: String,
    pub message_font_weight: String,
    pub container_font_size: f32,
    pub container_font_family: String,
    pub container_font_weight: String,
    pub external_container_font_size: f32,
    pub external_container_font_family: String,
    pub external_container_font_weight: String,
    pub container_db_font_size: f32,
    pub container_db_font_family: String,
    pub container_db_font_weight: String,
    pub external_container_db_font_size: f32,
    pub external_container_db_font_family: String,
    pub external_container_db_font_weight: String,
    pub container_queue_font_size: f32,
    pub container_queue_font_family: String,
    pub container_queue_font_weight: String,
    pub external_container_queue_font_size: f32,
    pub external_container_queue_font_family: String,
    pub external_container_queue_font_weight: String,
    pub component_font_size: f32,
    pub component_font_family: String,
    pub component_font_weight: String,
    pub external_component_font_size: f32,
    pub external_component_font_family: String,
    pub external_component_font_weight: String,
    pub component_db_font_size: f32,
    pub component_db_font_family: String,
    pub component_db_font_weight: String,
    pub external_component_db_font_size: f32,
    pub external_component_db_font_family: String,
    pub external_component_db_font_weight: String,
    pub component_queue_font_size: f32,
    pub component_queue_font_family: String,
    pub component_queue_font_weight: String,
    pub external_component_queue_font_size: f32,
    pub external_component_queue_font_family: String,
    pub external_component_queue_font_weight: String,
    pub person_bg_color: String,
    pub person_border_color: String,
    pub external_person_bg_color: String,
    pub external_person_border_color: String,
    pub system_bg_color: String,
    pub system_border_color: String,
    pub system_db_bg_color: String,
    pub system_db_border_color: String,
    pub system_queue_bg_color: String,
    pub system_queue_border_color: String,
    pub external_system_bg_color: String,
    pub external_system_border_color: String,
    pub external_system_db_bg_color: String,
    pub external_system_db_border_color: String,
    pub external_system_queue_bg_color: String,
    pub external_system_queue_border_color: String,
    pub container_bg_color: String,
    pub container_border_color: String,
    pub container_db_bg_color: String,
    pub container_db_border_color: String,
    pub container_queue_bg_color: String,
    pub container_queue_border_color: String,
    pub external_container_bg_color: String,
    pub external_container_border_color: String,
    pub external_container_db_bg_color: String,
    pub external_container_db_border_color: String,
    pub external_container_queue_bg_color: String,
    pub external_container_queue_border_color: String,
    pub component_bg_color: String,
    pub component_border_color: String,
    pub component_db_bg_color: String,
    pub component_db_border_color: String,
    pub component_queue_bg_color: String,
    pub component_queue_border_color: String,
    pub external_component_bg_color: String,
    pub external_component_border_color: String,
    pub external_component_db_bg_color: String,
    pub external_component_db_border_color: String,
    pub external_component_queue_bg_color: String,
    pub external_component_queue_border_color: String,
}

impl Default for C4Config {
    fn default() -> Self {
        Self {
            use_max_width: true,
            diagram_margin_x: 50.0,
            diagram_margin_y: 10.0,
            c4_shape_margin: 50.0,
            c4_shape_padding: 20.0,
            width: 216.0,
            height: 60.0,
            box_margin: 10.0,
            c4_shape_in_row: 4,
            next_line_padding_x: 0.0,
            c4_boundary_in_row: 2,
            wrap: true,
            wrap_padding: 10.0,
            text_line_height: 1.0,
            text_line_height_small_add: 1.0,
            text_line_height_small_threshold: 14.0,
            shape_corner_radius: 2.5,
            shape_stroke_width: 0.5,
            boundary_corner_radius: 2.5,
            person_icon_size: 48.0,
            db_ellipse_height: 10.0,
            queue_curve_radius: 5.0,
            boundary_stroke: "#444444".to_string(),
            boundary_dasharray: "7.0,7.0".to_string(),
            boundary_stroke_width: 1.0,
            boundary_fill: "none".to_string(),
            boundary_fill_opacity: 0.0,
            person_font_size: 14.0,
            person_font_family: "\"Open Sans\", sans-serif".to_string(),
            person_font_weight: "normal".to_string(),
            external_person_font_size: 14.0,
            external_person_font_family: "\"Open Sans\", sans-serif".to_string(),
            external_person_font_weight: "normal".to_string(),
            system_font_size: 14.0,
            system_font_family: "\"Open Sans\", sans-serif".to_string(),
            system_font_weight: "normal".to_string(),
            external_system_font_size: 14.0,
            external_system_font_family: "\"Open Sans\", sans-serif".to_string(),
            external_system_font_weight: "normal".to_string(),
            system_db_font_size: 14.0,
            system_db_font_family: "\"Open Sans\", sans-serif".to_string(),
            system_db_font_weight: "normal".to_string(),
            external_system_db_font_size: 14.0,
            external_system_db_font_family: "\"Open Sans\", sans-serif".to_string(),
            external_system_db_font_weight: "normal".to_string(),
            system_queue_font_size: 14.0,
            system_queue_font_family: "\"Open Sans\", sans-serif".to_string(),
            system_queue_font_weight: "normal".to_string(),
            external_system_queue_font_size: 14.0,
            external_system_queue_font_family: "\"Open Sans\", sans-serif".to_string(),
            external_system_queue_font_weight: "normal".to_string(),
            boundary_font_size: 14.0,
            boundary_font_family: "\"Open Sans\", sans-serif".to_string(),
            boundary_font_weight: "normal".to_string(),
            message_font_size: 12.0,
            message_font_family: "\"Open Sans\", sans-serif".to_string(),
            message_font_weight: "normal".to_string(),
            container_font_size: 14.0,
            container_font_family: "\"Open Sans\", sans-serif".to_string(),
            container_font_weight: "normal".to_string(),
            external_container_font_size: 14.0,
            external_container_font_family: "\"Open Sans\", sans-serif".to_string(),
            external_container_font_weight: "normal".to_string(),
            container_db_font_size: 14.0,
            container_db_font_family: "\"Open Sans\", sans-serif".to_string(),
            container_db_font_weight: "normal".to_string(),
            external_container_db_font_size: 14.0,
            external_container_db_font_family: "\"Open Sans\", sans-serif".to_string(),
            external_container_db_font_weight: "normal".to_string(),
            container_queue_font_size: 14.0,
            container_queue_font_family: "\"Open Sans\", sans-serif".to_string(),
            container_queue_font_weight: "normal".to_string(),
            external_container_queue_font_size: 14.0,
            external_container_queue_font_family: "\"Open Sans\", sans-serif".to_string(),
            external_container_queue_font_weight: "normal".to_string(),
            component_font_size: 14.0,
            component_font_family: "\"Open Sans\", sans-serif".to_string(),
            component_font_weight: "normal".to_string(),
            external_component_font_size: 14.0,
            external_component_font_family: "\"Open Sans\", sans-serif".to_string(),
            external_component_font_weight: "normal".to_string(),
            component_db_font_size: 14.0,
            component_db_font_family: "\"Open Sans\", sans-serif".to_string(),
            component_db_font_weight: "normal".to_string(),
            external_component_db_font_size: 14.0,
            external_component_db_font_family: "\"Open Sans\", sans-serif".to_string(),
            external_component_db_font_weight: "normal".to_string(),
            component_queue_font_size: 14.0,
            component_queue_font_family: "\"Open Sans\", sans-serif".to_string(),
            component_queue_font_weight: "normal".to_string(),
            external_component_queue_font_size: 14.0,
            external_component_queue_font_family: "\"Open Sans\", sans-serif".to_string(),
            external_component_queue_font_weight: "normal".to_string(),
            person_bg_color: "#08427B".to_string(),
            person_border_color: "#073B6F".to_string(),
            external_person_bg_color: "#686868".to_string(),
            external_person_border_color: "#8A8A8A".to_string(),
            system_bg_color: "#1168BD".to_string(),
            system_border_color: "#3C7FC0".to_string(),
            system_db_bg_color: "#1168BD".to_string(),
            system_db_border_color: "#3C7FC0".to_string(),
            system_queue_bg_color: "#1168BD".to_string(),
            system_queue_border_color: "#3C7FC0".to_string(),
            external_system_bg_color: "#999999".to_string(),
            external_system_border_color: "#8A8A8A".to_string(),
            external_system_db_bg_color: "#999999".to_string(),
            external_system_db_border_color: "#8A8A8A".to_string(),
            external_system_queue_bg_color: "#999999".to_string(),
            external_system_queue_border_color: "#8A8A8A".to_string(),
            container_bg_color: "#438DD5".to_string(),
            container_border_color: "#3C7FC0".to_string(),
            container_db_bg_color: "#438DD5".to_string(),
            container_db_border_color: "#3C7FC0".to_string(),
            container_queue_bg_color: "#438DD5".to_string(),
            container_queue_border_color: "#3C7FC0".to_string(),
            external_container_bg_color: "#B3B3B3".to_string(),
            external_container_border_color: "#A6A6A6".to_string(),
            external_container_db_bg_color: "#B3B3B3".to_string(),
            external_container_db_border_color: "#A6A6A6".to_string(),
            external_container_queue_bg_color: "#B3B3B3".to_string(),
            external_container_queue_border_color: "#A6A6A6".to_string(),
            component_bg_color: "#85BBF0".to_string(),
            component_border_color: "#78A8D8".to_string(),
            component_db_bg_color: "#85BBF0".to_string(),
            component_db_border_color: "#78A8D8".to_string(),
            component_queue_bg_color: "#85BBF0".to_string(),
            component_queue_border_color: "#78A8D8".to_string(),
            external_component_bg_color: "#CCCCCC".to_string(),
            external_component_border_color: "#BFBFBF".to_string(),
            external_component_db_bg_color: "#CCCCCC".to_string(),
            external_component_db_border_color: "#BFBFBF".to_string(),
            external_component_queue_bg_color: "#CCCCCC".to_string(),
            external_component_queue_border_color: "#BFBFBF".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TreemapRenderMode {
    Error,
    Flowchart,
}

impl Default for TreemapRenderMode {
    fn default() -> Self {
        Self::Error
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PieRenderMode {
    Error,
    Chart,
}

impl Default for PieRenderMode {
    fn default() -> Self {
        Self::Error
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PieConfig {
    pub render_mode: PieRenderMode,
    pub use_max_width: bool,
    pub text_position: f32,
    pub height: f32,
    pub margin: f32,
    pub legend_rect_size: f32,
    pub legend_spacing: f32,
    pub legend_horizontal_multiplier: f32,
    pub min_percent: f32,
    pub error_message: String,
    pub error_version: String,
    pub error_viewbox_width: f32,
    pub error_viewbox_height: f32,
    pub error_render_width: f32,
    pub error_render_height: Option<f32>,
    pub error_text_x: f32,
    pub error_text_y: f32,
    pub error_text_size: f32,
    pub error_version_x: f32,
    pub error_version_y: f32,
    pub error_version_size: f32,
    pub icon_scale: f32,
    pub icon_tx: f32,
    pub icon_ty: f32,
}

impl Default for PieConfig {
    fn default() -> Self {
        Self {
            render_mode: PieRenderMode::Error,
            use_max_width: true,
            text_position: 0.75,
            height: 450.0,
            margin: 40.0,
            legend_rect_size: 18.0,
            legend_spacing: 4.0,
            legend_horizontal_multiplier: 12.0,
            min_percent: 1.0,
            error_message: "Syntax error in text".to_string(),
            error_version: "11.12.2".to_string(),
            error_viewbox_width: 2412.0,
            error_viewbox_height: 512.0,
            error_render_width: 512.0,
            error_render_height: None,
            error_text_x: 1440.0,
            error_text_y: 250.0,
            error_text_size: 150.0,
            error_version_x: 1250.0,
            error_version_y: 400.0,
            error_version_size: 100.0,
            icon_scale: 1.0,
            icon_tx: 0.0,
            icon_ty: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreemapConfig {
    pub render_mode: TreemapRenderMode,
    pub error_message: String,
    pub error_version: String,
    pub error_viewbox_width: f32,
    pub error_viewbox_height: f32,
    pub error_render_width: f32,
    pub error_render_height: Option<f32>,
    pub error_text_x: f32,
    pub error_text_y: f32,
    pub error_text_size: f32,
    pub error_version_x: f32,
    pub error_version_y: f32,
    pub error_version_size: f32,
    pub icon_scale: f32,
    pub icon_tx: f32,
    pub icon_ty: f32,
}

impl Default for TreemapConfig {
    fn default() -> Self {
        Self {
            render_mode: TreemapRenderMode::Error,
            error_message: "Syntax error in text".to_string(),
            error_version: "11.12.2".to_string(),
            error_viewbox_width: 2412.0,
            error_viewbox_height: 512.0,
            error_render_width: 512.0,
            error_render_height: None,
            error_text_x: 1440.0,
            error_text_y: 250.0,
            error_text_size: 150.0,
            error_version_x: 1250.0,
            error_version_y: 400.0,
            error_version_size: 100.0,
            icon_scale: 1.0,
            icon_tx: 0.0,
            icon_ty: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    pub node_spacing: f32,
    pub rank_spacing: f32,
    pub node_padding_x: f32,
    pub node_padding_y: f32,
    pub label_line_height: f32,
    pub max_label_width_chars: usize,
    pub requirement: RequirementConfig,
    pub mindmap: MindmapConfig,
    pub gitgraph: GitGraphConfig,
    pub c4: C4Config,
    pub pie: PieConfig,
    pub treemap: TreemapConfig,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            node_spacing: 50.0,
            rank_spacing: 50.0,
            node_padding_x: 30.0,
            node_padding_y: 15.0,
            label_line_height: 1.5,
            max_label_width_chars: 22,
            requirement: RequirementConfig::default(),
            mindmap: MindmapConfig::default(),
            gitgraph: GitGraphConfig::default(),
            c4: C4Config::default(),
            pie: PieConfig::default(),
            treemap: TreemapConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderConfig {
    pub width: f32,
    pub height: f32,
    pub background: String,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            width: 1200.0,
            height: 800.0,
            background: "#FFFFFF".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub theme: Theme,
    pub layout: LayoutConfig,
    pub render: RenderConfig,
}

impl Default for Config {
    fn default() -> Self {
        let theme = Theme::mermaid_default();
        let render = RenderConfig {
            background: theme.background.clone(),
            ..Default::default()
        };
        Self {
            theme,
            layout: LayoutConfig::default(),
            render,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThemeVariables {
    font_family: Option<String>,
    font_size: Option<f32>,
    primary_color: Option<String>,
    primary_text_color: Option<String>,
    primary_border_color: Option<String>,
    line_color: Option<String>,
    secondary_color: Option<String>,
    tertiary_color: Option<String>,
    text_color: Option<String>,
    edge_label_background: Option<String>,
    cluster_bkg: Option<String>,
    cluster_border: Option<String>,
    background: Option<String>,
    actor_bkg: Option<String>,
    actor_border: Option<String>,
    actor_line: Option<String>,
    note_bkg: Option<String>,
    note_border_color: Option<String>,
    activation_bkg_color: Option<String>,
    activation_border_color: Option<String>,
    git0: Option<String>,
    git1: Option<String>,
    git2: Option<String>,
    git3: Option<String>,
    git4: Option<String>,
    git5: Option<String>,
    git6: Option<String>,
    git7: Option<String>,
    git_inv0: Option<String>,
    git_inv1: Option<String>,
    git_inv2: Option<String>,
    git_inv3: Option<String>,
    git_inv4: Option<String>,
    git_inv5: Option<String>,
    git_inv6: Option<String>,
    git_inv7: Option<String>,
    git_branch_label0: Option<String>,
    git_branch_label1: Option<String>,
    git_branch_label2: Option<String>,
    git_branch_label3: Option<String>,
    git_branch_label4: Option<String>,
    git_branch_label5: Option<String>,
    git_branch_label6: Option<String>,
    git_branch_label7: Option<String>,
    commit_label_color: Option<String>,
    commit_label_background: Option<String>,
    tag_label_color: Option<String>,
    tag_label_background: Option<String>,
    tag_label_border: Option<String>,
    pie1: Option<String>,
    pie2: Option<String>,
    pie3: Option<String>,
    pie4: Option<String>,
    pie5: Option<String>,
    pie6: Option<String>,
    pie7: Option<String>,
    pie8: Option<String>,
    pie9: Option<String>,
    pie10: Option<String>,
    pie11: Option<String>,
    pie12: Option<String>,
    pie_title_text_size: Option<NumberOrString>,
    pie_title_text_color: Option<String>,
    pie_section_text_size: Option<NumberOrString>,
    pie_section_text_color: Option<String>,
    pie_legend_text_size: Option<NumberOrString>,
    pie_legend_text_color: Option<String>,
    pie_stroke_color: Option<String>,
    pie_stroke_width: Option<NumberOrString>,
    pie_outer_stroke_width: Option<NumberOrString>,
    pie_outer_stroke_color: Option<String>,
    pie_opacity: Option<NumberOrString>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum NumberOrString {
    Number(f32),
    String(String),
}

impl NumberOrString {
    fn as_f32(&self) -> Option<f32> {
        match self {
            NumberOrString::Number(val) => Some(*val),
            NumberOrString::String(val) => val.trim().parse::<f32>().ok(),
        }
    }

    fn as_string(&self) -> String {
        match self {
            NumberOrString::Number(val) => format!("{}", val),
            NumberOrString::String(val) => val.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FlowchartConfig {
    node_spacing: Option<f32>,
    rank_spacing: Option<f32>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PieConfigFile {
    render_mode: Option<PieRenderMode>,
    use_max_width: Option<bool>,
    text_position: Option<f32>,
    height: Option<f32>,
    margin: Option<f32>,
    legend_rect_size: Option<f32>,
    legend_spacing: Option<f32>,
    legend_horizontal_multiplier: Option<f32>,
    min_percent: Option<f32>,
    error_message: Option<String>,
    error_version: Option<String>,
    error_viewbox_width: Option<f32>,
    error_viewbox_height: Option<f32>,
    error_render_width: Option<f32>,
    error_render_height: Option<f32>,
    error_text_x: Option<f32>,
    error_text_y: Option<f32>,
    error_text_size: Option<f32>,
    error_version_x: Option<f32>,
    error_version_y: Option<f32>,
    error_version_size: Option<f32>,
    icon_scale: Option<f32>,
    icon_tx: Option<f32>,
    icon_ty: Option<f32>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RequirementConfigFile {
    fill: Option<String>,
    box_stroke: Option<String>,
    box_stroke_width: Option<f32>,
    stroke: Option<String>,
    stroke_width: Option<f32>,
    label_color: Option<String>,
    divider_color: Option<String>,
    divider_width: Option<f32>,
    header_band_height: Option<f32>,
    header_line_gap: Option<f32>,
    label_padding_x: Option<f32>,
    label_padding_y: Option<f32>,
    edge_stroke: Option<String>,
    edge_dasharray: Option<String>,
    edge_stroke_width: Option<f32>,
    edge_label_color: Option<String>,
    edge_label_background: Option<String>,
    edge_label_padding_x: Option<f32>,
    edge_label_padding_y: Option<f32>,
    edge_label_brackets: Option<bool>,
    render_padding_x: Option<f32>,
    render_padding_y: Option<f32>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct MindmapConfigFile {
    use_max_width: Option<bool>,
    padding: Option<f32>,
    max_node_width: Option<f32>,
    layout_algorithm: Option<String>,
    node_spacing: Option<f32>,
    rank_spacing: Option<f32>,
    node_spacing_multiplier: Option<f32>,
    rank_spacing_multiplier: Option<f32>,
    text_width_scale: Option<f32>,
    rounded_padding: Option<f32>,
    rect_padding: Option<f32>,
    circle_padding: Option<f32>,
    hexagon_padding_multiplier: Option<f32>,
    default_corner_radius: Option<f32>,
    edge_depth_base_width: Option<f32>,
    edge_depth_step: Option<f32>,
    divider_line_width: Option<f32>,
    section_colors: Option<Vec<String>>,
    section_label_colors: Option<Vec<String>>,
    section_line_colors: Option<Vec<String>>,
    root_fill: Option<String>,
    root_text: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GitGraphConfigFile {
    diagram_padding: Option<f32>,
    title_top_margin: Option<f32>,
    use_max_width: Option<bool>,
    main_branch_name: Option<String>,
    main_branch_order: Option<f32>,
    show_commit_label: Option<bool>,
    show_branches: Option<bool>,
    rotate_commit_label: Option<bool>,
    parallel_commits: Option<bool>,
    commit_step: Option<f32>,
    layout_offset: Option<f32>,
    default_pos: Option<f32>,
    branch_spacing: Option<f32>,
    branch_spacing_rotate_extra: Option<f32>,
    branch_label_rotate_extra: Option<f32>,
    branch_label_translate_x: Option<f32>,
    branch_label_bg_offset_x: Option<f32>,
    branch_label_bg_offset_y: Option<f32>,
    branch_label_bg_pad_x: Option<f32>,
    branch_label_bg_pad_y: Option<f32>,
    branch_label_text_offset_x: Option<f32>,
    branch_label_text_offset_y: Option<f32>,
    branch_label_tb_bg_offset_x: Option<f32>,
    branch_label_tb_text_offset_x: Option<f32>,
    branch_label_tb_offset_y: Option<f32>,
    branch_label_bt_offset_y: Option<f32>,
    branch_label_corner_radius: Option<f32>,
    branch_label_font_size: Option<f32>,
    branch_label_line_height: Option<f32>,
    text_width_scale: Option<f32>,
    commit_label_font_size: Option<f32>,
    commit_label_line_height: Option<f32>,
    commit_label_offset_y: Option<f32>,
    commit_label_bg_offset_y: Option<f32>,
    commit_label_padding: Option<f32>,
    commit_label_bg_opacity: Option<f32>,
    commit_label_rotate_angle: Option<f32>,
    commit_label_rotate_translate_x_base: Option<f32>,
    commit_label_rotate_translate_x_scale: Option<f32>,
    commit_label_rotate_translate_x_width_offset: Option<f32>,
    commit_label_rotate_translate_y_base: Option<f32>,
    commit_label_rotate_translate_y_scale: Option<f32>,
    commit_label_tb_text_extra: Option<f32>,
    commit_label_tb_bg_extra: Option<f32>,
    commit_label_tb_text_offset_y: Option<f32>,
    commit_label_tb_bg_offset_y: Option<f32>,
    tag_label_font_size: Option<f32>,
    tag_label_line_height: Option<f32>,
    tag_text_offset_y: Option<f32>,
    tag_polygon_offset_y: Option<f32>,
    tag_spacing_y: Option<f32>,
    tag_padding_x: Option<f32>,
    tag_padding_y: Option<f32>,
    tag_hole_radius: Option<f32>,
    tag_rotate_translate: Option<f32>,
    tag_text_rotate_translate: Option<f32>,
    tag_rotate_angle: Option<f32>,
    tag_text_offset_x_tb: Option<f32>,
    tag_text_offset_y_tb: Option<f32>,
    arrow_reroute_radius: Option<f32>,
    arrow_radius: Option<f32>,
    lane_spacing: Option<f32>,
    lane_max_depth: Option<usize>,
    commit_radius: Option<f32>,
    merge_radius_outer: Option<f32>,
    merge_radius_inner: Option<f32>,
    highlight_outer_size: Option<f32>,
    highlight_inner_size: Option<f32>,
    reverse_cross_size: Option<f32>,
    reverse_stroke_width: Option<f32>,
    cherry_pick_dot_radius: Option<f32>,
    cherry_pick_dot_offset_x: Option<f32>,
    cherry_pick_dot_offset_y: Option<f32>,
    cherry_pick_stem_start_offset_y: Option<f32>,
    cherry_pick_stem_end_offset_y: Option<f32>,
    cherry_pick_stem_stroke_width: Option<f32>,
    cherry_pick_accent_color: Option<String>,
    arrow_stroke_width: Option<f32>,
    branch_stroke_width: Option<f32>,
    branch_dasharray: Option<String>,
    commit_spacing: Option<f32>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct C4ConfigFile {
    use_max_width: Option<bool>,
    diagram_margin_x: Option<f32>,
    diagram_margin_y: Option<f32>,
    c4_shape_margin: Option<f32>,
    c4_shape_padding: Option<f32>,
    width: Option<f32>,
    height: Option<f32>,
    box_margin: Option<f32>,
    c4_shape_in_row: Option<usize>,
    next_line_padding_x: Option<f32>,
    c4_boundary_in_row: Option<usize>,
    wrap: Option<bool>,
    wrap_padding: Option<f32>,
    text_line_height: Option<f32>,
    text_line_height_small_add: Option<f32>,
    text_line_height_small_threshold: Option<f32>,
    shape_corner_radius: Option<f32>,
    shape_stroke_width: Option<f32>,
    boundary_corner_radius: Option<f32>,
    person_icon_size: Option<f32>,
    db_ellipse_height: Option<f32>,
    queue_curve_radius: Option<f32>,
    boundary_stroke: Option<String>,
    boundary_dasharray: Option<String>,
    boundary_stroke_width: Option<f32>,
    boundary_fill: Option<String>,
    boundary_fill_opacity: Option<f32>,
    person_font_size: Option<NumberOrString>,
    person_font_family: Option<String>,
    person_font_weight: Option<NumberOrString>,
    external_person_font_size: Option<NumberOrString>,
    external_person_font_family: Option<String>,
    external_person_font_weight: Option<NumberOrString>,
    system_font_size: Option<NumberOrString>,
    system_font_family: Option<String>,
    system_font_weight: Option<NumberOrString>,
    external_system_font_size: Option<NumberOrString>,
    external_system_font_family: Option<String>,
    external_system_font_weight: Option<NumberOrString>,
    system_db_font_size: Option<NumberOrString>,
    system_db_font_family: Option<String>,
    system_db_font_weight: Option<NumberOrString>,
    external_system_db_font_size: Option<NumberOrString>,
    external_system_db_font_family: Option<String>,
    external_system_db_font_weight: Option<NumberOrString>,
    system_queue_font_size: Option<NumberOrString>,
    system_queue_font_family: Option<String>,
    system_queue_font_weight: Option<NumberOrString>,
    external_system_queue_font_size: Option<NumberOrString>,
    external_system_queue_font_family: Option<String>,
    external_system_queue_font_weight: Option<NumberOrString>,
    boundary_font_size: Option<NumberOrString>,
    boundary_font_family: Option<String>,
    boundary_font_weight: Option<NumberOrString>,
    message_font_size: Option<NumberOrString>,
    message_font_family: Option<String>,
    message_font_weight: Option<NumberOrString>,
    container_font_size: Option<NumberOrString>,
    container_font_family: Option<String>,
    container_font_weight: Option<NumberOrString>,
    external_container_font_size: Option<NumberOrString>,
    external_container_font_family: Option<String>,
    external_container_font_weight: Option<NumberOrString>,
    container_db_font_size: Option<NumberOrString>,
    container_db_font_family: Option<String>,
    container_db_font_weight: Option<NumberOrString>,
    external_container_db_font_size: Option<NumberOrString>,
    external_container_db_font_family: Option<String>,
    external_container_db_font_weight: Option<NumberOrString>,
    container_queue_font_size: Option<NumberOrString>,
    container_queue_font_family: Option<String>,
    container_queue_font_weight: Option<NumberOrString>,
    external_container_queue_font_size: Option<NumberOrString>,
    external_container_queue_font_family: Option<String>,
    external_container_queue_font_weight: Option<NumberOrString>,
    component_font_size: Option<NumberOrString>,
    component_font_family: Option<String>,
    component_font_weight: Option<NumberOrString>,
    external_component_font_size: Option<NumberOrString>,
    external_component_font_family: Option<String>,
    external_component_font_weight: Option<NumberOrString>,
    component_db_font_size: Option<NumberOrString>,
    component_db_font_family: Option<String>,
    component_db_font_weight: Option<NumberOrString>,
    external_component_db_font_size: Option<NumberOrString>,
    external_component_db_font_family: Option<String>,
    external_component_db_font_weight: Option<NumberOrString>,
    component_queue_font_size: Option<NumberOrString>,
    component_queue_font_family: Option<String>,
    component_queue_font_weight: Option<NumberOrString>,
    external_component_queue_font_size: Option<NumberOrString>,
    external_component_queue_font_family: Option<String>,
    external_component_queue_font_weight: Option<NumberOrString>,
    person_bg_color: Option<String>,
    person_border_color: Option<String>,
    external_person_bg_color: Option<String>,
    external_person_border_color: Option<String>,
    system_bg_color: Option<String>,
    system_border_color: Option<String>,
    system_db_bg_color: Option<String>,
    system_db_border_color: Option<String>,
    system_queue_bg_color: Option<String>,
    system_queue_border_color: Option<String>,
    external_system_bg_color: Option<String>,
    external_system_border_color: Option<String>,
    external_system_db_bg_color: Option<String>,
    external_system_db_border_color: Option<String>,
    external_system_queue_bg_color: Option<String>,
    external_system_queue_border_color: Option<String>,
    container_bg_color: Option<String>,
    container_border_color: Option<String>,
    container_db_bg_color: Option<String>,
    container_db_border_color: Option<String>,
    container_queue_bg_color: Option<String>,
    container_queue_border_color: Option<String>,
    external_container_bg_color: Option<String>,
    external_container_border_color: Option<String>,
    external_container_db_bg_color: Option<String>,
    external_container_db_border_color: Option<String>,
    external_container_queue_bg_color: Option<String>,
    external_container_queue_border_color: Option<String>,
    component_bg_color: Option<String>,
    component_border_color: Option<String>,
    component_db_bg_color: Option<String>,
    component_db_border_color: Option<String>,
    component_queue_bg_color: Option<String>,
    component_queue_border_color: Option<String>,
    external_component_bg_color: Option<String>,
    external_component_border_color: Option<String>,
    external_component_db_bg_color: Option<String>,
    external_component_db_border_color: Option<String>,
    external_component_queue_bg_color: Option<String>,
    external_component_queue_border_color: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct TreemapConfigFile {
    render_mode: Option<TreemapRenderMode>,
    error_message: Option<String>,
    error_version: Option<String>,
    error_viewbox_width: Option<f32>,
    error_viewbox_height: Option<f32>,
    error_render_width: Option<f32>,
    error_render_height: Option<f32>,
    error_text_x: Option<f32>,
    error_text_y: Option<f32>,
    error_text_size: Option<f32>,
    error_version_x: Option<f32>,
    error_version_y: Option<f32>,
    error_version_size: Option<f32>,
    icon_scale: Option<f32>,
    icon_tx: Option<f32>,
    icon_ty: Option<f32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigFile {
    theme: Option<String>,
    theme_variables: Option<ThemeVariables>,
    flowchart: Option<FlowchartConfig>,
    pie: Option<PieConfigFile>,
    requirement: Option<RequirementConfigFile>,
    mindmap: Option<MindmapConfigFile>,
    #[serde(rename = "gitGraph")]
    gitgraph: Option<GitGraphConfigFile>,
    c4: Option<C4ConfigFile>,
    treemap: Option<TreemapConfigFile>,
}

pub fn load_config(path: Option<&Path>) -> anyhow::Result<Config> {
    let mut config = Config::default();
    let Some(path) = path else {
        return Ok(config);
    };

    let contents = std::fs::read_to_string(path)?;
    let parsed: ConfigFile = serde_json::from_str(&contents)?;

    if let Some(theme_name) = parsed.theme.as_deref() {
        if theme_name == "modern" {
            config.theme = Theme::modern();
        } else if theme_name == "base" || theme_name == "default" || theme_name == "mermaid" {
            config.theme = Theme::mermaid_default();
        }
    }

    if let Some(vars) = parsed.theme_variables {
        let tag_label_border_explicit = vars.tag_label_border.is_some();
        let primary_border_override = vars.primary_border_color.clone();
        if let Some(v) = vars.font_family {
            config.theme.font_family = v;
        }
        if let Some(v) = vars.font_size {
            config.theme.font_size = v;
        }
        if let Some(v) = vars.primary_color {
            config.theme.primary_color = v;
        }
        if let Some(v) = vars.primary_text_color {
            config.theme.primary_text_color = v;
        }
        if let Some(v) = vars.primary_border_color {
            config.theme.primary_border_color = v;
        }
        if let Some(v) = vars.line_color {
            config.theme.line_color = v;
        }
        if let Some(v) = vars.secondary_color {
            config.theme.secondary_color = v;
        }
        if let Some(v) = vars.tertiary_color {
            config.theme.tertiary_color = v;
        }
        if let Some(v) = vars.text_color {
            config.theme.text_color = v;
        }
        if let Some(v) = vars.edge_label_background {
            config.theme.edge_label_background = v;
        }
        if let Some(v) = vars.cluster_bkg {
            config.theme.cluster_background = v;
        }
        if let Some(v) = vars.cluster_border {
            config.theme.cluster_border = v;
        }
        if let Some(v) = vars.background {
            config.theme.background = v;
        }
        if let Some(v) = vars.actor_bkg {
            config.theme.sequence_actor_fill = v;
        }
        if let Some(v) = vars.actor_border {
            config.theme.sequence_actor_border = v;
        }
        if let Some(v) = vars.actor_line {
            config.theme.sequence_actor_line = v;
        }
        if let Some(v) = vars.note_bkg {
            config.theme.sequence_note_fill = v;
        }
        if let Some(v) = vars.note_border_color {
            config.theme.sequence_note_border = v;
        }
        if let Some(v) = vars.activation_bkg_color {
            config.theme.sequence_activation_fill = v;
        }
        if let Some(v) = vars.activation_border_color {
            config.theme.sequence_activation_border = v;
        }
        if let Some(v) = vars.git0 {
            config.theme.git_colors[0] = v;
        }
        if let Some(v) = vars.git1 {
            config.theme.git_colors[1] = v;
        }
        if let Some(v) = vars.git2 {
            config.theme.git_colors[2] = v;
        }
        if let Some(v) = vars.git3 {
            config.theme.git_colors[3] = v;
        }
        if let Some(v) = vars.git4 {
            config.theme.git_colors[4] = v;
        }
        if let Some(v) = vars.git5 {
            config.theme.git_colors[5] = v;
        }
        if let Some(v) = vars.git6 {
            config.theme.git_colors[6] = v;
        }
        if let Some(v) = vars.git7 {
            config.theme.git_colors[7] = v;
        }
        if let Some(v) = vars.git_inv0 {
            config.theme.git_inv_colors[0] = v;
        }
        if let Some(v) = vars.git_inv1 {
            config.theme.git_inv_colors[1] = v;
        }
        if let Some(v) = vars.git_inv2 {
            config.theme.git_inv_colors[2] = v;
        }
        if let Some(v) = vars.git_inv3 {
            config.theme.git_inv_colors[3] = v;
        }
        if let Some(v) = vars.git_inv4 {
            config.theme.git_inv_colors[4] = v;
        }
        if let Some(v) = vars.git_inv5 {
            config.theme.git_inv_colors[5] = v;
        }
        if let Some(v) = vars.git_inv6 {
            config.theme.git_inv_colors[6] = v;
        }
        if let Some(v) = vars.git_inv7 {
            config.theme.git_inv_colors[7] = v;
        }
        if let Some(v) = vars.git_branch_label0 {
            config.theme.git_branch_label_colors[0] = v;
        }
        if let Some(v) = vars.git_branch_label1 {
            config.theme.git_branch_label_colors[1] = v;
        }
        if let Some(v) = vars.git_branch_label2 {
            config.theme.git_branch_label_colors[2] = v;
        }
        if let Some(v) = vars.git_branch_label3 {
            config.theme.git_branch_label_colors[3] = v;
        }
        if let Some(v) = vars.git_branch_label4 {
            config.theme.git_branch_label_colors[4] = v;
        }
        if let Some(v) = vars.git_branch_label5 {
            config.theme.git_branch_label_colors[5] = v;
        }
        if let Some(v) = vars.git_branch_label6 {
            config.theme.git_branch_label_colors[6] = v;
        }
        if let Some(v) = vars.git_branch_label7 {
            config.theme.git_branch_label_colors[7] = v;
        }
        if let Some(v) = vars.commit_label_color {
            config.theme.git_commit_label_color = v;
        }
        if let Some(v) = vars.commit_label_background {
            config.theme.git_commit_label_background = v;
        }
        if let Some(v) = vars.tag_label_color {
            config.theme.git_tag_label_color = v;
        }
        if let Some(v) = vars.tag_label_background {
            config.theme.git_tag_label_background = v;
        }
        if let Some(v) = vars.tag_label_border {
            config.theme.git_tag_label_border = v;
        }
        if !tag_label_border_explicit && primary_border_override.is_some() {
            config.theme.git_tag_label_border = config.theme.primary_border_color.clone();
        }
        if let Some(v) = vars.pie1 {
            config.theme.pie_colors[0] = v;
        }
        if let Some(v) = vars.pie2 {
            config.theme.pie_colors[1] = v;
        }
        if let Some(v) = vars.pie3 {
            config.theme.pie_colors[2] = v;
        }
        if let Some(v) = vars.pie4 {
            config.theme.pie_colors[3] = v;
        }
        if let Some(v) = vars.pie5 {
            config.theme.pie_colors[4] = v;
        }
        if let Some(v) = vars.pie6 {
            config.theme.pie_colors[5] = v;
        }
        if let Some(v) = vars.pie7 {
            config.theme.pie_colors[6] = v;
        }
        if let Some(v) = vars.pie8 {
            config.theme.pie_colors[7] = v;
        }
        if let Some(v) = vars.pie9 {
            config.theme.pie_colors[8] = v;
        }
        if let Some(v) = vars.pie10 {
            config.theme.pie_colors[9] = v;
        }
        if let Some(v) = vars.pie11 {
            config.theme.pie_colors[10] = v;
        }
        if let Some(v) = vars.pie12 {
            config.theme.pie_colors[11] = v;
        }
        if let Some(v) = vars.pie_title_text_size {
            if let Some(size) = v.as_f32() {
                config.theme.pie_title_text_size = size;
            }
        }
        if let Some(v) = vars.pie_title_text_color {
            config.theme.pie_title_text_color = v;
        }
        if let Some(v) = vars.pie_section_text_size {
            if let Some(size) = v.as_f32() {
                config.theme.pie_section_text_size = size;
            }
        }
        if let Some(v) = vars.pie_section_text_color {
            config.theme.pie_section_text_color = v;
        }
        if let Some(v) = vars.pie_legend_text_size {
            if let Some(size) = v.as_f32() {
                config.theme.pie_legend_text_size = size;
            }
        }
        if let Some(v) = vars.pie_legend_text_color {
            config.theme.pie_legend_text_color = v;
        }
        if let Some(v) = vars.pie_stroke_color {
            config.theme.pie_stroke_color = v;
        }
        if let Some(v) = vars.pie_stroke_width {
            if let Some(width) = v.as_f32() {
                config.theme.pie_stroke_width = width;
            }
        }
        if let Some(v) = vars.pie_outer_stroke_width {
            if let Some(width) = v.as_f32() {
                config.theme.pie_outer_stroke_width = width;
            }
        }
        if let Some(v) = vars.pie_outer_stroke_color {
            config.theme.pie_outer_stroke_color = v;
        }
        if let Some(v) = vars.pie_opacity {
            if let Some(opacity) = v.as_f32() {
                config.theme.pie_opacity = opacity;
            }
        }
    }

    if let Some(flow) = parsed.flowchart {
        if let Some(v) = flow.node_spacing {
            config.layout.node_spacing = v;
        }
        if let Some(v) = flow.rank_spacing {
            config.layout.rank_spacing = v;
        }
    }

    if let Some(pie) = parsed.pie {
        if let Some(v) = pie.render_mode {
            config.layout.pie.render_mode = v;
        }
        if let Some(v) = pie.use_max_width {
            config.layout.pie.use_max_width = v;
        }
        if let Some(v) = pie.text_position {
            config.layout.pie.text_position = v;
        }
        if let Some(v) = pie.height {
            config.layout.pie.height = v;
        }
        if let Some(v) = pie.margin {
            config.layout.pie.margin = v;
        }
        if let Some(v) = pie.legend_rect_size {
            config.layout.pie.legend_rect_size = v;
        }
        if let Some(v) = pie.legend_spacing {
            config.layout.pie.legend_spacing = v;
        }
        if let Some(v) = pie.legend_horizontal_multiplier {
            config.layout.pie.legend_horizontal_multiplier = v;
        }
        if let Some(v) = pie.min_percent {
            config.layout.pie.min_percent = v;
        }
        if let Some(v) = pie.error_message {
            config.layout.pie.error_message = v;
        }
        if let Some(v) = pie.error_version {
            config.layout.pie.error_version = v;
        }
        if let Some(v) = pie.error_viewbox_width {
            config.layout.pie.error_viewbox_width = v;
        }
        if let Some(v) = pie.error_viewbox_height {
            config.layout.pie.error_viewbox_height = v;
        }
        if let Some(v) = pie.error_render_width {
            config.layout.pie.error_render_width = v;
        }
        if pie.error_render_height.is_some() {
            config.layout.pie.error_render_height = pie.error_render_height;
        }
        if let Some(v) = pie.error_text_x {
            config.layout.pie.error_text_x = v;
        }
        if let Some(v) = pie.error_text_y {
            config.layout.pie.error_text_y = v;
        }
        if let Some(v) = pie.error_text_size {
            config.layout.pie.error_text_size = v;
        }
        if let Some(v) = pie.error_version_x {
            config.layout.pie.error_version_x = v;
        }
        if let Some(v) = pie.error_version_y {
            config.layout.pie.error_version_y = v;
        }
        if let Some(v) = pie.error_version_size {
            config.layout.pie.error_version_size = v;
        }
        if let Some(v) = pie.icon_scale {
            config.layout.pie.icon_scale = v;
        }
        if let Some(v) = pie.icon_tx {
            config.layout.pie.icon_tx = v;
        }
        if let Some(v) = pie.icon_ty {
            config.layout.pie.icon_ty = v;
        }
    }

    if let Some(req) = parsed.requirement {
        if let Some(v) = req.fill {
            config.layout.requirement.fill = v;
        }
        if let Some(v) = req.box_stroke {
            config.layout.requirement.box_stroke = v;
        }
        if let Some(v) = req.box_stroke_width {
            config.layout.requirement.box_stroke_width = v;
        }
        if let Some(v) = req.stroke {
            config.layout.requirement.stroke = v;
        }
        if let Some(v) = req.stroke_width {
            config.layout.requirement.stroke_width = v;
        }
        if let Some(v) = req.label_color {
            config.layout.requirement.label_color = v;
        }
        if let Some(v) = req.divider_color {
            config.layout.requirement.divider_color = v;
        }
        if let Some(v) = req.divider_width {
            config.layout.requirement.divider_width = v;
        }
        if let Some(v) = req.header_band_height {
            config.layout.requirement.header_band_height = v;
        }
        if let Some(v) = req.header_line_gap {
            config.layout.requirement.header_line_gap = v;
        }
        if let Some(v) = req.label_padding_x {
            config.layout.requirement.label_padding_x = v;
        }
        if let Some(v) = req.label_padding_y {
            config.layout.requirement.label_padding_y = v;
        }
        if let Some(v) = req.edge_stroke {
            config.layout.requirement.edge_stroke = v;
        }
        if let Some(v) = req.edge_dasharray {
            config.layout.requirement.edge_dasharray = v;
        }
        if let Some(v) = req.edge_stroke_width {
            config.layout.requirement.edge_stroke_width = v;
        }
        if let Some(v) = req.edge_label_color {
            config.layout.requirement.edge_label_color = v;
        }
        if let Some(v) = req.edge_label_background {
            config.layout.requirement.edge_label_background = v;
        }
        if let Some(v) = req.edge_label_padding_x {
            config.layout.requirement.edge_label_padding_x = v;
        }
        if let Some(v) = req.edge_label_padding_y {
            config.layout.requirement.edge_label_padding_y = v;
        }
        if let Some(v) = req.edge_label_brackets {
            config.layout.requirement.edge_label_brackets = v;
        }
        if let Some(v) = req.render_padding_x {
            config.layout.requirement.render_padding_x = v;
        }
        if let Some(v) = req.render_padding_y {
            config.layout.requirement.render_padding_y = v;
        }
    }

    if let Some(mm) = parsed.mindmap {
        if let Some(v) = mm.use_max_width {
            config.layout.mindmap.use_max_width = v;
        }
        if let Some(v) = mm.padding {
            config.layout.mindmap.padding = v;
        }
        if let Some(v) = mm.max_node_width {
            config.layout.mindmap.max_node_width = v;
        }
        if let Some(v) = mm.layout_algorithm {
            config.layout.mindmap.layout_algorithm = v;
        }
        if let Some(v) = mm.node_spacing {
            config.layout.mindmap.node_spacing = v;
        }
        if let Some(v) = mm.rank_spacing {
            config.layout.mindmap.rank_spacing = v;
        }
        if let Some(v) = mm.node_spacing_multiplier {
            config.layout.mindmap.node_spacing_multiplier = v;
        }
        if let Some(v) = mm.rank_spacing_multiplier {
            config.layout.mindmap.rank_spacing_multiplier = v;
        }
        if let Some(v) = mm.text_width_scale {
            config.layout.mindmap.text_width_scale = v;
        }
        if let Some(v) = mm.rounded_padding {
            config.layout.mindmap.rounded_padding = v;
        }
        if let Some(v) = mm.rect_padding {
            config.layout.mindmap.rect_padding = v;
        }
        if let Some(v) = mm.circle_padding {
            config.layout.mindmap.circle_padding = v;
        }
        if let Some(v) = mm.hexagon_padding_multiplier {
            config.layout.mindmap.hexagon_padding_multiplier = v;
        }
        if let Some(v) = mm.default_corner_radius {
            config.layout.mindmap.default_corner_radius = v;
        }
        if let Some(v) = mm.edge_depth_base_width {
            config.layout.mindmap.edge_depth_base_width = v;
        }
        if let Some(v) = mm.edge_depth_step {
            config.layout.mindmap.edge_depth_step = v;
        }
        if let Some(v) = mm.divider_line_width {
            config.layout.mindmap.divider_line_width = v;
        }
        if let Some(v) = mm.section_colors {
            config.layout.mindmap.section_colors = v;
        }
        if let Some(v) = mm.section_label_colors {
            config.layout.mindmap.section_label_colors = v;
        }
        if let Some(v) = mm.section_line_colors {
            config.layout.mindmap.section_line_colors = v;
        }
        if let Some(v) = mm.root_fill {
            config.layout.mindmap.root_fill = Some(v);
        }
        if let Some(v) = mm.root_text {
            config.layout.mindmap.root_text = Some(v);
        }
    }

    if let Some(gg) = parsed.gitgraph {
        let mut commit_step_set = false;
        if let Some(v) = gg.diagram_padding {
            config.layout.gitgraph.diagram_padding = v;
        }
        if let Some(v) = gg.title_top_margin {
            config.layout.gitgraph.title_top_margin = v;
        }
        if let Some(v) = gg.use_max_width {
            config.layout.gitgraph.use_max_width = v;
        }
        if let Some(v) = gg.main_branch_name {
            config.layout.gitgraph.main_branch_name = v;
        }
        if let Some(v) = gg.main_branch_order {
            config.layout.gitgraph.main_branch_order = v;
        }
        if let Some(v) = gg.show_commit_label {
            config.layout.gitgraph.show_commit_label = v;
        }
        if let Some(v) = gg.show_branches {
            config.layout.gitgraph.show_branches = v;
        }
        if let Some(v) = gg.rotate_commit_label {
            config.layout.gitgraph.rotate_commit_label = v;
        }
        if let Some(v) = gg.parallel_commits {
            config.layout.gitgraph.parallel_commits = v;
        }
        if let Some(v) = gg.commit_step {
            config.layout.gitgraph.commit_step = v;
            commit_step_set = true;
        }
        if let Some(v) = gg.layout_offset {
            config.layout.gitgraph.layout_offset = v;
        }
        if let Some(v) = gg.default_pos {
            config.layout.gitgraph.default_pos = v;
        }
        if let Some(v) = gg.branch_spacing {
            config.layout.gitgraph.branch_spacing = v;
        }
        if let Some(v) = gg.branch_spacing_rotate_extra {
            config.layout.gitgraph.branch_spacing_rotate_extra = v;
        }
        if let Some(v) = gg.branch_label_rotate_extra {
            config.layout.gitgraph.branch_label_rotate_extra = v;
        }
        if let Some(v) = gg.branch_label_translate_x {
            config.layout.gitgraph.branch_label_translate_x = v;
        }
        if let Some(v) = gg.branch_label_bg_offset_x {
            config.layout.gitgraph.branch_label_bg_offset_x = v;
        }
        if let Some(v) = gg.branch_label_bg_offset_y {
            config.layout.gitgraph.branch_label_bg_offset_y = v;
        }
        if let Some(v) = gg.branch_label_bg_pad_x {
            config.layout.gitgraph.branch_label_bg_pad_x = v;
        }
        if let Some(v) = gg.branch_label_bg_pad_y {
            config.layout.gitgraph.branch_label_bg_pad_y = v;
        }
        if let Some(v) = gg.branch_label_text_offset_x {
            config.layout.gitgraph.branch_label_text_offset_x = v;
        }
        if let Some(v) = gg.branch_label_text_offset_y {
            config.layout.gitgraph.branch_label_text_offset_y = v;
        }
        if let Some(v) = gg.branch_label_tb_bg_offset_x {
            config.layout.gitgraph.branch_label_tb_bg_offset_x = v;
        }
        if let Some(v) = gg.branch_label_tb_text_offset_x {
            config.layout.gitgraph.branch_label_tb_text_offset_x = v;
        }
        if let Some(v) = gg.branch_label_tb_offset_y {
            config.layout.gitgraph.branch_label_tb_offset_y = v;
        }
        if let Some(v) = gg.branch_label_bt_offset_y {
            config.layout.gitgraph.branch_label_bt_offset_y = v;
        }
        if let Some(v) = gg.branch_label_corner_radius {
            config.layout.gitgraph.branch_label_corner_radius = v;
        }
        if let Some(v) = gg.branch_label_font_size {
            config.layout.gitgraph.branch_label_font_size = v;
        }
        if let Some(v) = gg.branch_label_line_height {
            config.layout.gitgraph.branch_label_line_height = v;
        }
        if let Some(v) = gg.text_width_scale {
            config.layout.gitgraph.text_width_scale = v;
        }
        if let Some(v) = gg.commit_label_font_size {
            config.layout.gitgraph.commit_label_font_size = v;
        }
        if let Some(v) = gg.commit_label_line_height {
            config.layout.gitgraph.commit_label_line_height = v;
        }
        if let Some(v) = gg.commit_label_offset_y {
            config.layout.gitgraph.commit_label_offset_y = v;
        }
        if let Some(v) = gg.commit_label_bg_offset_y {
            config.layout.gitgraph.commit_label_bg_offset_y = v;
        }
        if let Some(v) = gg.commit_label_padding {
            config.layout.gitgraph.commit_label_padding = v;
        }
        if let Some(v) = gg.commit_label_bg_opacity {
            config.layout.gitgraph.commit_label_bg_opacity = v;
        }
        if let Some(v) = gg.commit_label_rotate_angle {
            config.layout.gitgraph.commit_label_rotate_angle = v;
        }
        if let Some(v) = gg.commit_label_rotate_translate_x_base {
            config.layout.gitgraph.commit_label_rotate_translate_x_base = v;
        }
        if let Some(v) = gg.commit_label_rotate_translate_x_scale {
            config.layout.gitgraph.commit_label_rotate_translate_x_scale = v;
        }
        if let Some(v) = gg.commit_label_rotate_translate_x_width_offset {
            config.layout.gitgraph.commit_label_rotate_translate_x_width_offset = v;
        }
        if let Some(v) = gg.commit_label_rotate_translate_y_base {
            config.layout.gitgraph.commit_label_rotate_translate_y_base = v;
        }
        if let Some(v) = gg.commit_label_rotate_translate_y_scale {
            config.layout.gitgraph.commit_label_rotate_translate_y_scale = v;
        }
        if let Some(v) = gg.commit_label_tb_text_extra {
            config.layout.gitgraph.commit_label_tb_text_extra = v;
        }
        if let Some(v) = gg.commit_label_tb_bg_extra {
            config.layout.gitgraph.commit_label_tb_bg_extra = v;
        }
        if let Some(v) = gg.commit_label_tb_text_offset_y {
            config.layout.gitgraph.commit_label_tb_text_offset_y = v;
        }
        if let Some(v) = gg.commit_label_tb_bg_offset_y {
            config.layout.gitgraph.commit_label_tb_bg_offset_y = v;
        }
        if let Some(v) = gg.tag_label_font_size {
            config.layout.gitgraph.tag_label_font_size = v;
        }
        if let Some(v) = gg.tag_label_line_height {
            config.layout.gitgraph.tag_label_line_height = v;
        }
        if let Some(v) = gg.tag_text_offset_y {
            config.layout.gitgraph.tag_text_offset_y = v;
        }
        if let Some(v) = gg.tag_polygon_offset_y {
            config.layout.gitgraph.tag_polygon_offset_y = v;
        }
        if let Some(v) = gg.tag_spacing_y {
            config.layout.gitgraph.tag_spacing_y = v;
        }
        if let Some(v) = gg.tag_padding_x {
            config.layout.gitgraph.tag_padding_x = v;
        }
        if let Some(v) = gg.tag_padding_y {
            config.layout.gitgraph.tag_padding_y = v;
        }
        if let Some(v) = gg.tag_hole_radius {
            config.layout.gitgraph.tag_hole_radius = v;
        }
        if let Some(v) = gg.tag_rotate_translate {
            config.layout.gitgraph.tag_rotate_translate = v;
        }
        if let Some(v) = gg.tag_text_rotate_translate {
            config.layout.gitgraph.tag_text_rotate_translate = v;
        }
        if let Some(v) = gg.tag_rotate_angle {
            config.layout.gitgraph.tag_rotate_angle = v;
        }
        if let Some(v) = gg.tag_text_offset_x_tb {
            config.layout.gitgraph.tag_text_offset_x_tb = v;
        }
        if let Some(v) = gg.tag_text_offset_y_tb {
            config.layout.gitgraph.tag_text_offset_y_tb = v;
        }
        if let Some(v) = gg.arrow_reroute_radius {
            config.layout.gitgraph.arrow_reroute_radius = v;
        }
        if let Some(v) = gg.arrow_radius {
            config.layout.gitgraph.arrow_radius = v;
        }
        if let Some(v) = gg.lane_spacing {
            config.layout.gitgraph.lane_spacing = v;
        }
        if let Some(v) = gg.lane_max_depth {
            config.layout.gitgraph.lane_max_depth = v;
        }
        if let Some(v) = gg.commit_radius {
            config.layout.gitgraph.commit_radius = v;
        }
        if let Some(v) = gg.merge_radius_outer {
            config.layout.gitgraph.merge_radius_outer = v;
        }
        if let Some(v) = gg.merge_radius_inner {
            config.layout.gitgraph.merge_radius_inner = v;
        }
        if let Some(v) = gg.highlight_outer_size {
            config.layout.gitgraph.highlight_outer_size = v;
        }
        if let Some(v) = gg.highlight_inner_size {
            config.layout.gitgraph.highlight_inner_size = v;
        }
        if let Some(v) = gg.reverse_cross_size {
            config.layout.gitgraph.reverse_cross_size = v;
        }
        if let Some(v) = gg.reverse_stroke_width {
            config.layout.gitgraph.reverse_stroke_width = v;
        }
        if let Some(v) = gg.cherry_pick_dot_radius {
            config.layout.gitgraph.cherry_pick_dot_radius = v;
        }
        if let Some(v) = gg.cherry_pick_dot_offset_x {
            config.layout.gitgraph.cherry_pick_dot_offset_x = v;
        }
        if let Some(v) = gg.cherry_pick_dot_offset_y {
            config.layout.gitgraph.cherry_pick_dot_offset_y = v;
        }
        if let Some(v) = gg.cherry_pick_stem_start_offset_y {
            config.layout.gitgraph.cherry_pick_stem_start_offset_y = v;
        }
        if let Some(v) = gg.cherry_pick_stem_end_offset_y {
            config.layout.gitgraph.cherry_pick_stem_end_offset_y = v;
        }
        if let Some(v) = gg.cherry_pick_stem_stroke_width {
            config.layout.gitgraph.cherry_pick_stem_stroke_width = v;
        }
        if let Some(v) = gg.cherry_pick_accent_color {
            config.layout.gitgraph.cherry_pick_accent_color = v;
        }
        if let Some(v) = gg.arrow_stroke_width {
            config.layout.gitgraph.arrow_stroke_width = v;
        }
        if let Some(v) = gg.branch_stroke_width {
            config.layout.gitgraph.branch_stroke_width = v;
        }
        if let Some(v) = gg.branch_dasharray {
            config.layout.gitgraph.branch_dasharray = v;
        }
        if let Some(v) = gg.commit_spacing {
            if !commit_step_set {
                let step = (v - config.layout.gitgraph.layout_offset).max(1.0);
                config.layout.gitgraph.commit_step = step;
            }
        }
    }

    if let Some(c4) = parsed.c4 {
        if let Some(v) = c4.use_max_width {
            config.layout.c4.use_max_width = v;
        }
        if let Some(v) = c4.diagram_margin_x {
            config.layout.c4.diagram_margin_x = v;
        }
        if let Some(v) = c4.diagram_margin_y {
            config.layout.c4.diagram_margin_y = v;
        }
        if let Some(v) = c4.c4_shape_margin {
            config.layout.c4.c4_shape_margin = v;
        }
        if let Some(v) = c4.c4_shape_padding {
            config.layout.c4.c4_shape_padding = v;
        }
        if let Some(v) = c4.width {
            config.layout.c4.width = v;
        }
        if let Some(v) = c4.height {
            config.layout.c4.height = v;
        }
        if let Some(v) = c4.box_margin {
            config.layout.c4.box_margin = v;
        }
        if let Some(v) = c4.c4_shape_in_row {
            config.layout.c4.c4_shape_in_row = v;
        }
        if let Some(v) = c4.next_line_padding_x {
            config.layout.c4.next_line_padding_x = v;
        }
        if let Some(v) = c4.c4_boundary_in_row {
            config.layout.c4.c4_boundary_in_row = v;
        }
        if let Some(v) = c4.wrap {
            config.layout.c4.wrap = v;
        }
        if let Some(v) = c4.wrap_padding {
            config.layout.c4.wrap_padding = v;
        }
        if let Some(v) = c4.text_line_height {
            config.layout.c4.text_line_height = v;
        }
        if let Some(v) = c4.text_line_height_small_add {
            config.layout.c4.text_line_height_small_add = v;
        }
        if let Some(v) = c4.text_line_height_small_threshold {
            config.layout.c4.text_line_height_small_threshold = v;
        }
        if let Some(v) = c4.shape_corner_radius {
            config.layout.c4.shape_corner_radius = v;
        }
        if let Some(v) = c4.shape_stroke_width {
            config.layout.c4.shape_stroke_width = v;
        }
        if let Some(v) = c4.boundary_corner_radius {
            config.layout.c4.boundary_corner_radius = v;
        }
        if let Some(v) = c4.person_icon_size {
            config.layout.c4.person_icon_size = v;
        }
        if let Some(v) = c4.db_ellipse_height {
            config.layout.c4.db_ellipse_height = v;
        }
        if let Some(v) = c4.queue_curve_radius {
            config.layout.c4.queue_curve_radius = v;
        }
        if let Some(v) = c4.boundary_stroke {
            config.layout.c4.boundary_stroke = v;
        }
        if let Some(v) = c4.boundary_dasharray {
            config.layout.c4.boundary_dasharray = v;
        }
        if let Some(v) = c4.boundary_stroke_width {
            config.layout.c4.boundary_stroke_width = v;
        }
        if let Some(v) = c4.boundary_fill {
            config.layout.c4.boundary_fill = v;
        }
        if let Some(v) = c4.boundary_fill_opacity {
            config.layout.c4.boundary_fill_opacity = v;
        }
        if let Some(v) = c4.person_font_size.and_then(|v| v.as_f32()) {
            config.layout.c4.person_font_size = v;
        }
        if let Some(v) = c4.person_font_family {
            config.layout.c4.person_font_family = v;
        }
        if let Some(v) = c4.person_font_weight {
            config.layout.c4.person_font_weight = v.as_string();
        }
        if let Some(v) = c4.external_person_font_size.and_then(|v| v.as_f32()) {
            config.layout.c4.external_person_font_size = v;
        }
        if let Some(v) = c4.external_person_font_family {
            config.layout.c4.external_person_font_family = v;
        }
        if let Some(v) = c4.external_person_font_weight {
            config.layout.c4.external_person_font_weight = v.as_string();
        }
        if let Some(v) = c4.system_font_size.and_then(|v| v.as_f32()) {
            config.layout.c4.system_font_size = v;
        }
        if let Some(v) = c4.system_font_family {
            config.layout.c4.system_font_family = v;
        }
        if let Some(v) = c4.system_font_weight {
            config.layout.c4.system_font_weight = v.as_string();
        }
        if let Some(v) = c4.external_system_font_size.and_then(|v| v.as_f32()) {
            config.layout.c4.external_system_font_size = v;
        }
        if let Some(v) = c4.external_system_font_family {
            config.layout.c4.external_system_font_family = v;
        }
        if let Some(v) = c4.external_system_font_weight {
            config.layout.c4.external_system_font_weight = v.as_string();
        }
        if let Some(v) = c4.system_db_font_size.and_then(|v| v.as_f32()) {
            config.layout.c4.system_db_font_size = v;
        }
        if let Some(v) = c4.system_db_font_family {
            config.layout.c4.system_db_font_family = v;
        }
        if let Some(v) = c4.system_db_font_weight {
            config.layout.c4.system_db_font_weight = v.as_string();
        }
        if let Some(v) = c4.external_system_db_font_size.and_then(|v| v.as_f32()) {
            config.layout.c4.external_system_db_font_size = v;
        }
        if let Some(v) = c4.external_system_db_font_family {
            config.layout.c4.external_system_db_font_family = v;
        }
        if let Some(v) = c4.external_system_db_font_weight {
            config.layout.c4.external_system_db_font_weight = v.as_string();
        }
        if let Some(v) = c4.system_queue_font_size.and_then(|v| v.as_f32()) {
            config.layout.c4.system_queue_font_size = v;
        }
        if let Some(v) = c4.system_queue_font_family {
            config.layout.c4.system_queue_font_family = v;
        }
        if let Some(v) = c4.system_queue_font_weight {
            config.layout.c4.system_queue_font_weight = v.as_string();
        }
        if let Some(v) = c4.external_system_queue_font_size.and_then(|v| v.as_f32()) {
            config.layout.c4.external_system_queue_font_size = v;
        }
        if let Some(v) = c4.external_system_queue_font_family {
            config.layout.c4.external_system_queue_font_family = v;
        }
        if let Some(v) = c4.external_system_queue_font_weight {
            config.layout.c4.external_system_queue_font_weight = v.as_string();
        }
        if let Some(v) = c4.boundary_font_size.and_then(|v| v.as_f32()) {
            config.layout.c4.boundary_font_size = v;
        }
        if let Some(v) = c4.boundary_font_family {
            config.layout.c4.boundary_font_family = v;
        }
        if let Some(v) = c4.boundary_font_weight {
            config.layout.c4.boundary_font_weight = v.as_string();
        }
        if let Some(v) = c4.message_font_size.and_then(|v| v.as_f32()) {
            config.layout.c4.message_font_size = v;
        }
        if let Some(v) = c4.message_font_family {
            config.layout.c4.message_font_family = v;
        }
        if let Some(v) = c4.message_font_weight {
            config.layout.c4.message_font_weight = v.as_string();
        }
        if let Some(v) = c4.container_font_size.and_then(|v| v.as_f32()) {
            config.layout.c4.container_font_size = v;
        }
        if let Some(v) = c4.container_font_family {
            config.layout.c4.container_font_family = v;
        }
        if let Some(v) = c4.container_font_weight {
            config.layout.c4.container_font_weight = v.as_string();
        }
        if let Some(v) = c4.external_container_font_size.and_then(|v| v.as_f32()) {
            config.layout.c4.external_container_font_size = v;
        }
        if let Some(v) = c4.external_container_font_family {
            config.layout.c4.external_container_font_family = v;
        }
        if let Some(v) = c4.external_container_font_weight {
            config.layout.c4.external_container_font_weight = v.as_string();
        }
        if let Some(v) = c4.container_db_font_size.and_then(|v| v.as_f32()) {
            config.layout.c4.container_db_font_size = v;
        }
        if let Some(v) = c4.container_db_font_family {
            config.layout.c4.container_db_font_family = v;
        }
        if let Some(v) = c4.container_db_font_weight {
            config.layout.c4.container_db_font_weight = v.as_string();
        }
        if let Some(v) = c4.external_container_db_font_size.and_then(|v| v.as_f32()) {
            config.layout.c4.external_container_db_font_size = v;
        }
        if let Some(v) = c4.external_container_db_font_family {
            config.layout.c4.external_container_db_font_family = v;
        }
        if let Some(v) = c4.external_container_db_font_weight {
            config.layout.c4.external_container_db_font_weight = v.as_string();
        }
        if let Some(v) = c4.container_queue_font_size.and_then(|v| v.as_f32()) {
            config.layout.c4.container_queue_font_size = v;
        }
        if let Some(v) = c4.container_queue_font_family {
            config.layout.c4.container_queue_font_family = v;
        }
        if let Some(v) = c4.container_queue_font_weight {
            config.layout.c4.container_queue_font_weight = v.as_string();
        }
        if let Some(v) = c4.external_container_queue_font_size.and_then(|v| v.as_f32()) {
            config.layout.c4.external_container_queue_font_size = v;
        }
        if let Some(v) = c4.external_container_queue_font_family {
            config.layout.c4.external_container_queue_font_family = v;
        }
        if let Some(v) = c4.external_container_queue_font_weight {
            config.layout.c4.external_container_queue_font_weight = v.as_string();
        }
        if let Some(v) = c4.component_font_size.and_then(|v| v.as_f32()) {
            config.layout.c4.component_font_size = v;
        }
        if let Some(v) = c4.component_font_family {
            config.layout.c4.component_font_family = v;
        }
        if let Some(v) = c4.component_font_weight {
            config.layout.c4.component_font_weight = v.as_string();
        }
        if let Some(v) = c4.external_component_font_size.and_then(|v| v.as_f32()) {
            config.layout.c4.external_component_font_size = v;
        }
        if let Some(v) = c4.external_component_font_family {
            config.layout.c4.external_component_font_family = v;
        }
        if let Some(v) = c4.external_component_font_weight {
            config.layout.c4.external_component_font_weight = v.as_string();
        }
        if let Some(v) = c4.component_db_font_size.and_then(|v| v.as_f32()) {
            config.layout.c4.component_db_font_size = v;
        }
        if let Some(v) = c4.component_db_font_family {
            config.layout.c4.component_db_font_family = v;
        }
        if let Some(v) = c4.component_db_font_weight {
            config.layout.c4.component_db_font_weight = v.as_string();
        }
        if let Some(v) = c4.external_component_db_font_size.and_then(|v| v.as_f32()) {
            config.layout.c4.external_component_db_font_size = v;
        }
        if let Some(v) = c4.external_component_db_font_family {
            config.layout.c4.external_component_db_font_family = v;
        }
        if let Some(v) = c4.external_component_db_font_weight {
            config.layout.c4.external_component_db_font_weight = v.as_string();
        }
        if let Some(v) = c4.component_queue_font_size.and_then(|v| v.as_f32()) {
            config.layout.c4.component_queue_font_size = v;
        }
        if let Some(v) = c4.component_queue_font_family {
            config.layout.c4.component_queue_font_family = v;
        }
        if let Some(v) = c4.component_queue_font_weight {
            config.layout.c4.component_queue_font_weight = v.as_string();
        }
        if let Some(v) = c4.external_component_queue_font_size.and_then(|v| v.as_f32()) {
            config.layout.c4.external_component_queue_font_size = v;
        }
        if let Some(v) = c4.external_component_queue_font_family {
            config.layout.c4.external_component_queue_font_family = v;
        }
        if let Some(v) = c4.external_component_queue_font_weight {
            config.layout.c4.external_component_queue_font_weight = v.as_string();
        }
        if let Some(v) = c4.person_bg_color {
            config.layout.c4.person_bg_color = v;
        }
        if let Some(v) = c4.person_border_color {
            config.layout.c4.person_border_color = v;
        }
        if let Some(v) = c4.external_person_bg_color {
            config.layout.c4.external_person_bg_color = v;
        }
        if let Some(v) = c4.external_person_border_color {
            config.layout.c4.external_person_border_color = v;
        }
        if let Some(v) = c4.system_bg_color {
            config.layout.c4.system_bg_color = v;
        }
        if let Some(v) = c4.system_border_color {
            config.layout.c4.system_border_color = v;
        }
        if let Some(v) = c4.system_db_bg_color {
            config.layout.c4.system_db_bg_color = v;
        }
        if let Some(v) = c4.system_db_border_color {
            config.layout.c4.system_db_border_color = v;
        }
        if let Some(v) = c4.system_queue_bg_color {
            config.layout.c4.system_queue_bg_color = v;
        }
        if let Some(v) = c4.system_queue_border_color {
            config.layout.c4.system_queue_border_color = v;
        }
        if let Some(v) = c4.external_system_bg_color {
            config.layout.c4.external_system_bg_color = v;
        }
        if let Some(v) = c4.external_system_border_color {
            config.layout.c4.external_system_border_color = v;
        }
        if let Some(v) = c4.external_system_db_bg_color {
            config.layout.c4.external_system_db_bg_color = v;
        }
        if let Some(v) = c4.external_system_db_border_color {
            config.layout.c4.external_system_db_border_color = v;
        }
        if let Some(v) = c4.external_system_queue_bg_color {
            config.layout.c4.external_system_queue_bg_color = v;
        }
        if let Some(v) = c4.external_system_queue_border_color {
            config.layout.c4.external_system_queue_border_color = v;
        }
        if let Some(v) = c4.container_bg_color {
            config.layout.c4.container_bg_color = v;
        }
        if let Some(v) = c4.container_border_color {
            config.layout.c4.container_border_color = v;
        }
        if let Some(v) = c4.container_db_bg_color {
            config.layout.c4.container_db_bg_color = v;
        }
        if let Some(v) = c4.container_db_border_color {
            config.layout.c4.container_db_border_color = v;
        }
        if let Some(v) = c4.container_queue_bg_color {
            config.layout.c4.container_queue_bg_color = v;
        }
        if let Some(v) = c4.container_queue_border_color {
            config.layout.c4.container_queue_border_color = v;
        }
        if let Some(v) = c4.external_container_bg_color {
            config.layout.c4.external_container_bg_color = v;
        }
        if let Some(v) = c4.external_container_border_color {
            config.layout.c4.external_container_border_color = v;
        }
        if let Some(v) = c4.external_container_db_bg_color {
            config.layout.c4.external_container_db_bg_color = v;
        }
        if let Some(v) = c4.external_container_db_border_color {
            config.layout.c4.external_container_db_border_color = v;
        }
        if let Some(v) = c4.external_container_queue_bg_color {
            config.layout.c4.external_container_queue_bg_color = v;
        }
        if let Some(v) = c4.external_container_queue_border_color {
            config.layout.c4.external_container_queue_border_color = v;
        }
        if let Some(v) = c4.component_bg_color {
            config.layout.c4.component_bg_color = v;
        }
        if let Some(v) = c4.component_border_color {
            config.layout.c4.component_border_color = v;
        }
        if let Some(v) = c4.component_db_bg_color {
            config.layout.c4.component_db_bg_color = v;
        }
        if let Some(v) = c4.component_db_border_color {
            config.layout.c4.component_db_border_color = v;
        }
        if let Some(v) = c4.component_queue_bg_color {
            config.layout.c4.component_queue_bg_color = v;
        }
        if let Some(v) = c4.component_queue_border_color {
            config.layout.c4.component_queue_border_color = v;
        }
        if let Some(v) = c4.external_component_bg_color {
            config.layout.c4.external_component_bg_color = v;
        }
        if let Some(v) = c4.external_component_border_color {
            config.layout.c4.external_component_border_color = v;
        }
        if let Some(v) = c4.external_component_db_bg_color {
            config.layout.c4.external_component_db_bg_color = v;
        }
        if let Some(v) = c4.external_component_db_border_color {
            config.layout.c4.external_component_db_border_color = v;
        }
        if let Some(v) = c4.external_component_queue_bg_color {
            config.layout.c4.external_component_queue_bg_color = v;
        }
        if let Some(v) = c4.external_component_queue_border_color {
            config.layout.c4.external_component_queue_border_color = v;
        }
    }

    if let Some(treemap) = parsed.treemap {
        if let Some(v) = treemap.render_mode {
            config.layout.treemap.render_mode = v;
        }
        if let Some(v) = treemap.error_message {
            config.layout.treemap.error_message = v;
        }
        if let Some(v) = treemap.error_version {
            config.layout.treemap.error_version = v;
        }
        if let Some(v) = treemap.error_viewbox_width {
            config.layout.treemap.error_viewbox_width = v;
        }
        if let Some(v) = treemap.error_viewbox_height {
            config.layout.treemap.error_viewbox_height = v;
        }
        if let Some(v) = treemap.error_render_width {
            config.layout.treemap.error_render_width = v;
        }
        if treemap.error_render_height.is_some() {
            config.layout.treemap.error_render_height = treemap.error_render_height;
        }
        if let Some(v) = treemap.error_text_x {
            config.layout.treemap.error_text_x = v;
        }
        if let Some(v) = treemap.error_text_y {
            config.layout.treemap.error_text_y = v;
        }
        if let Some(v) = treemap.error_text_size {
            config.layout.treemap.error_text_size = v;
        }
        if let Some(v) = treemap.error_version_x {
            config.layout.treemap.error_version_x = v;
        }
        if let Some(v) = treemap.error_version_y {
            config.layout.treemap.error_version_y = v;
        }
        if let Some(v) = treemap.error_version_size {
            config.layout.treemap.error_version_size = v;
        }
        if let Some(v) = treemap.icon_scale {
            config.layout.treemap.icon_scale = v;
        }
        if let Some(v) = treemap.icon_tx {
            config.layout.treemap.icon_tx = v;
        }
        if let Some(v) = treemap.icon_ty {
            config.layout.treemap.icon_ty = v;
        }
    }

    config.render.background = config.theme.background.clone();

    Ok(config)
}
