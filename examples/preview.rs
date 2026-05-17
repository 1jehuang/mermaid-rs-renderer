use mermaid_rs_renderer as mmdr;

const TD_SIMPLE: &str = "flowchart TD
  A([Start]) --> B{Working?}
  B -->|Yes| C[Ship it]
  B -->|No| D[Debug]
  D --> E[Fix bug]
  E --> B
  C --> F([Done])";

const RL_CALVIN: &str = "flowchart RL
  Glucose([Glucose]) --> G3P[G3P]
  G3P --> RuBP[RuBP]
  RuBP --> CO2((CO2))
  ATP([ATP]) --> G3P
  NADPH([NADPH]) --> G3P
  LR[Light Reactions] --> ATP
  LR --> NADPH
  LR --> O2((O2))
  H2O((H2O)) --> LR
  Photons((Photons)) --> LR
  RuBP --> Rubisco{{Rubisco}}
  Rubisco --> CO2";

const TD_MULTI: &str = "flowchart TD
  A[Input] --> P[Processor]
  B[Config] --> P
  C[Cache] --> P
  P --> R1[Result 1]
  P --> R2[Result 2]
  R1 --> O((Output))
  R2 --> O
  P --> E{Error?}
  E -->|Yes| ER[Error Handler]
  E -->|No| O
  ER --> A";

const LR_SHAPES: &str = "flowchart LR
  S([Start]) --> D{Decision}
  D -->|A| H{{Hexagon}}
  D -->|B| C((Circle))
  H --> R[Rectangle]
  C --> R
  R --> E([End])
  E --> D";

fn render(input: &str) -> String {
    let config = mmdr::LayoutConfig::default();
    let theme = mmdr::Theme::modern();
    let parsed = mmdr::parse_mermaid(input).expect("parse failed");
    let layout = mmdr::compute_layout(&parsed.graph, &theme, &config);
    mmdr::render_svg(&layout, &theme, &config)
}

fn panel(title: &str, svg: &str) -> String {
    format!(
        r#"<div class="panel">
  <div class="label">{title}</div>
  <div class="svg-wrap">{svg}</div>
</div>"#
    )
}

fn main() {
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Ariel Renderer Preview</title>
<style>
* {{ box-sizing: border-box; margin: 0; padding: 0; }}
body {{ font-family: system-ui, sans-serif; background: #f8fafc; padding: 32px; }}
h1 {{ font-size: 18px; font-weight: 700; margin-bottom: 24px; color: #0f172a; }}
.grid {{ display: grid; grid-template-columns: 1fr 1fr; gap: 24px; }}
.panel {{ background: #fff; border: 1px solid #e2e8f0; border-radius: 10px; padding: 20px; }}
.label {{ font-size: 11px; font-weight: 600; letter-spacing: 0.08em; text-transform: uppercase; color: #94a3b8; margin-bottom: 14px; }}
svg {{ max-width: 100%; height: auto; display: block; }}
</style>
</head>
<body>
<h1>Ariel Renderer Preview</h1>
<div class="grid">
  {p1}
  {p2}
  {p3}
  {p4}
</div>
</body>
</html>"#,
        p1 = panel("TD — back-edge loop", &render(TD_SIMPLE)),
        p2 = panel("RL — Calvin cycle (multi-edge)", &render(RL_CALVIN)),
        p3 = panel("TD — multiple inputs &amp; outputs", &render(TD_MULTI)),
        p4 = panel("LR — mixed shapes", &render(LR_SHAPES)),
    );

    std::fs::write("preview.html", &html).unwrap();
    println!("Written: preview.html");
}
