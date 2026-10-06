use mermaid_rs_renderer::{RenderOptions, render_with_options};

fn render(input: &str) -> String {
    render_with_options(input, RenderOptions::mermaid_default()).expect("valid diagram")
}

#[test]
fn upstream_configuration_and_link_fixes_are_preserved() {
    let svg = render(include_str!("inputs/render_regressions/18-init.mmd"));
    assert!(svg.contains("#123456"));
    assert!(svg.contains("#333333"));
    let svg = render(include_str!(
        "inputs/render_regressions/17-active-content.mmd"
    ));
    assert!(!svg.contains("href=\"javascript:"));
    assert!(!svg.contains("<script>"));
    assert!(!svg.contains("<image"));
}
