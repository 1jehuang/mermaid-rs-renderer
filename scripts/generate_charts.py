#!/usr/bin/env python3
"""Generate benchmark SVG charts from data."""

# Benchmark data
COMPARISON_DATA = [
    {"name": "Flowchart", "mmdr": 2.75, "cli": 2636},
    {"name": "Class", "mmdr": 3.19, "cli": 2381},
    {"name": "State", "mmdr": 2.45, "cli": 2647},
    {"name": "Sequence", "mmdr": 2.47, "cli": 2444},
]

BREAKDOWN_DATA = [
    {"name": "Flowchart", "parse": 1.04, "layout": 0.05, "render": 0.05},
    {"name": "Class", "parse": 0.36, "layout": 0.23, "render": 0.15},
    {"name": "State", "parse": 0.39, "layout": 0.18, "render": 0.10},
    {"name": "Sequence", "parse": 0.37, "layout": 0.03, "render": 0.07},
]

# Colors
GREEN = "#10b981"
ORANGE = "#f97316"
BLUE = "#3b82f6"
PURPLE = "#8b5cf6"


def generate_comparison_chart():
    """Generate the mmdr vs mermaid-cli comparison chart."""
    width, height = 600, 320
    bar_width = 50
    spacing = 125
    start_x = 55
    chart_top = 105
    chart_bottom = 285
    chart_height = chart_bottom - chart_top

    # Max value for scaling (mermaid-cli times)
    max_val = max(d["cli"] for d in COMPARISON_DATA)

    svg = f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}">
  <style>
    .title {{ font: bold 18px system-ui, sans-serif; fill: #000; }}
    .label {{ font: bold 14px system-ui, sans-serif; fill: #000; }}
    .value {{ font: bold 11px system-ui, sans-serif; fill: white; }}
    .speedup {{ font: bold 14px system-ui, sans-serif; fill: #047857; }}
    .legend {{ font: 13px system-ui, sans-serif; fill: #000; }}
  </style>

  <text x="{width/2}" y="28" text-anchor="middle" class="title">Render Time Comparison (ms)</text>

  <!-- Legend -->
  <rect x="200" y="45" width="14" height="14" rx="2" fill="{GREEN}"/>
  <text x="218" y="57" class="legend">mmdr</text>
  <rect x="290" y="45" width="14" height="14" rx="2" fill="{ORANGE}"/>
  <text x="308" y="57" class="legend">mermaid-cli</text>
'''

    for i, d in enumerate(COMPARISON_DATA):
        x = start_x + i * spacing
        center_x = x + bar_width

        # mmdr bar (tiny, fixed small height since values are ~2-3ms)
        mmdr_height = 24
        mmdr_y = chart_bottom - mmdr_height

        # cli bar (scaled to chart height)
        cli_height = (d["cli"] / max_val) * chart_height
        cli_y = chart_bottom - cli_height

        speedup = int(d["cli"] / d["mmdr"])

        svg += f'''
  <!-- {d["name"]} -->
  <text x="{center_x}" y="95" text-anchor="middle" class="label">{d["name"]}</text>
  <rect x="{x}" y="{mmdr_y}" width="{bar_width}" height="{mmdr_height}" rx="4" fill="{GREEN}"/>
  <text x="{x + bar_width/2}" y="{mmdr_y + 16}" text-anchor="middle" class="value">{d["mmdr"]:.1f}</text>
  <rect x="{x + bar_width + 10}" y="{cli_y}" width="{bar_width}" height="{cli_height}" rx="4" fill="{ORANGE}"/>
  <text x="{x + bar_width + 10 + bar_width/2}" y="{cli_y + 20}" text-anchor="middle" class="value">{d["cli"]}</text>
  <text x="{center_x}" y="305" text-anchor="middle" class="speedup">{speedup}×</text>
'''

    svg += '</svg>\n'
    return svg


def generate_breakdown_chart():
    """Generate the pipeline breakdown stacked bar chart."""
    width, height = 500, 280
    bar_width = 55
    spacing = 100
    start_x = 55
    baseline = 238

    # Scale: pixels per ms
    max_total = max(d["parse"] + d["layout"] + d["render"] for d in BREAKDOWN_DATA)
    scale = 160 / max_total  # 160px for max bar

    svg = f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}">
  <style>
    .title {{ font: bold 18px system-ui, sans-serif; fill: #000; }}
    .label {{ font: bold 14px system-ui, sans-serif; fill: #000; }}
    .value {{ font: bold 10px system-ui, sans-serif; fill: white; }}
    .axis {{ font: 12px system-ui, sans-serif; fill: #000; }}
    .legend {{ font: bold 12px system-ui, sans-serif; fill: #000; }}
  </style>

  <text x="{width/2}" y="28" text-anchor="middle" class="title">mmdr Pipeline Breakdown (ms)</text>

  <!-- Legend -->
  <rect x="130" y="46" width="14" height="14" rx="2" fill="{GREEN}"/>
  <text x="148" y="58" class="legend">Parse</text>
  <rect x="210" y="46" width="14" height="14" rx="2" fill="{BLUE}"/>
  <text x="228" y="58" class="legend">Layout</text>
  <rect x="295" y="46" width="14" height="14" rx="2" fill="{PURPLE}"/>
  <text x="313" y="58" class="legend">Render</text>

  <!-- Y-axis -->
  <text x="38" y="82" text-anchor="end" class="axis">1.2</text>
  <text x="38" y="122" text-anchor="end" class="axis">0.9</text>
  <text x="38" y="162" text-anchor="end" class="axis">0.6</text>
  <text x="38" y="202" text-anchor="end" class="axis">0.3</text>
  <text x="38" y="242" text-anchor="end" class="axis">0</text>

  <!-- Grid lines -->
  <line x1="45" y1="78" x2="460" y2="78" stroke="#d1d5db" stroke-width="1"/>
  <line x1="45" y1="118" x2="460" y2="118" stroke="#d1d5db" stroke-width="1"/>
  <line x1="45" y1="158" x2="460" y2="158" stroke="#d1d5db" stroke-width="1"/>
  <line x1="45" y1="198" x2="460" y2="198" stroke="#d1d5db" stroke-width="1"/>
  <line x1="45" y1="238" x2="460" y2="238" stroke="#666" stroke-width="1"/>
'''

    # Scale for y-axis (1.2ms = 160px)
    px_per_ms = 160 / 1.2

    for i, d in enumerate(BREAKDOWN_DATA):
        x = start_x + i * spacing
        center_x = x + bar_width / 2

        parse_h = d["parse"] * px_per_ms
        layout_h = d["layout"] * px_per_ms
        render_h = d["render"] * px_per_ms

        # Stack from bottom: parse, then layout, then render on top
        parse_y = baseline - parse_h
        layout_y = parse_y - layout_h
        render_y = layout_y - render_h

        svg += f'''
  <!-- {d["name"]} -->
  <rect x="{x}" y="{parse_y:.0f}" width="{bar_width}" height="{parse_h:.0f}" fill="{GREEN}"/>
  <text x="{center_x}" y="{parse_y + parse_h/2 + 4:.0f}" text-anchor="middle" class="value">{d["parse"]}</text>
  <rect x="{x}" y="{layout_y:.0f}" width="{bar_width}" height="{layout_h:.0f}" fill="{BLUE}"/>'''

        # Only show layout value if bar is tall enough
        if layout_h > 15:
            svg += f'''
  <text x="{center_x}" y="{layout_y + layout_h/2 + 4:.0f}" text-anchor="middle" class="value">{d["layout"]}</text>'''

        svg += f'''
  <rect x="{x}" y="{render_y:.0f}" width="{bar_width}" height="{render_h:.0f}" rx="3 3 0 0" fill="{PURPLE}"/>
  <text x="{center_x}" y="258" text-anchor="middle" class="label">{d["name"]}</text>
'''

    svg += '</svg>\n'
    return svg


if __name__ == "__main__":
    import os

    script_dir = os.path.dirname(os.path.abspath(__file__))
    docs_dir = os.path.join(script_dir, "..", "docs", "benchmarks")

    # Generate comparison chart
    comparison_svg = generate_comparison_chart()
    with open(os.path.join(docs_dir, "comparison.svg"), "w") as f:
        f.write(comparison_svg)
    print("Generated comparison.svg")

    # Generate breakdown chart
    breakdown_svg = generate_breakdown_chart()
    with open(os.path.join(docs_dir, "breakdown.svg"), "w") as f:
        f.write(breakdown_svg)
    print("Generated breakdown.svg")
