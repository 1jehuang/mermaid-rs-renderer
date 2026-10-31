use mermaid_rs_renderer::{LayoutConfig, Theme, compute_layout, parse_mermaid, render_svg};

fn render(source: &str, theme: &Theme) -> String {
    let graph = parse_mermaid(source).unwrap().graph;
    let config = LayoutConfig::default();
    render_svg(&compute_layout(&graph, theme, &config), theme, &config)
}

fn text_element<'a>(svg: &'a str, label: &str) -> &'a str {
    svg.split("<text ")
        .filter_map(|part| part.split_once("</text>"))
        .map(|(element, _)| element)
        .find(|element| {
            element.contains(&format!(">{label}<")) || element.ends_with(&format!(">{label}"))
        })
        .unwrap_or_else(|| panic!("missing text: {label}"))
}

#[test]
fn er_attributes_preserve_all_key_badges() {
    for theme in [Theme::modern(), Theme::dark()] {
        let svg = render("erDiagram\n BOOK {\n int identifier PK,FK,UK\n }", &theme);
        for label in ["PK", "FK", "UK"] {
            text_element(&svg, label);
        }
        assert!(svg.contains("identifier"));
    }
}
