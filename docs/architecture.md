# Architecture

## Scope
- Supports **12+ diagram types**: flowchart, sequence, class, state, ER, pie, gantt, journey, timeline, mindmap, gitGraph, xychart, quadrant.
- Support Mermaid init directives: `%%{init}%%` + per-diagram config.
- Deterministic output for stable diffs.

## Core pipeline decisions
- Parser: use a Rust parser generator (candidate: `pest` or `nom`).
- IR: typed nodes/edges/subgraphs, with explicit label + style objects.
- Layout: layered DAG layout (longest-path + crossing minimization).
- Rendering: generate SVG directly; rasterize PNG/PDF via `resvg`/`tiny-skia`.

## CLI decisions
- `mmdc`-compatible flags where possible: `-i`, `-o`, `-t`, `-c`, `-w`, `-H`.
- Config merge order: CLI flags > init directives > config file defaults.
- Zero network access by default.

## Theming decisions
- Map Mermaid theme variables into a Rust `Theme` struct.
- Emit CSS variables into the SVG for easy overrides.

## Extensibility decisions
- Add a diagram-type registry for future expansions.
- Keep parsing + layout isolated per diagram type.

## Diagrams
- `docs/diagrams/architecture.mmd`
- `docs/diagrams/pipeline.mmd`
