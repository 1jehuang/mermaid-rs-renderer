# Architecture

## Scope
- Supports **12+ diagram types**: flowchart, sequence, class, state, ER, pie, gantt, journey, timeline, mindmap, gitGraph, xychart, quadrant.
- Support Mermaid init directives: `%%{init}%%` + per-diagram config.
- Deterministic output for stable diffs.

## Core pipeline decisions
- Parser: use a Rust parser generator (candidate: `pest` or `nom`).
- IR: typed nodes/edges/subgraphs, with explicit label + style objects.
- Layout: layered DAG layout (ranking + crossing minimization) with a dedicated routing phase.
- Routing: obstacle-aware orthogonal routing with a grid A* router + heuristic fallback, using
  a shared occupancy grid to reduce edge-edge overlap.
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

## Flowchart routing architecture
- Detailed design and validation notes: [`docs/flowchart-routing-architecture.md`](flowchart-routing-architecture.md).
- **Port assignment**: per-node side ordering based on target alignment; offsets can snap to
  the routing grid to stabilize edge paths.
- **Port stubs**: edges exit nodes with a short orthogonal stub to normalize entry angles
  and reduce immediate overlaps near ports.
- **Rank ordering**: median-based ordering per layer with damped barycentric placement to
  reduce crossings and keep node structure stable.
- **Obstacles**: nodes + visible subgraphs expanded by padding; edges avoid these regions.
- **Global routing**: a grid A* router searches orthogonal paths with turn penalties and
  occupancy cost; if no grid path is found (or the grid is too large), heuristic
  candidates are used.
- **Occupancy**: routed edges mark a shared grid to discourage later overlaps while keeping
  routing deterministic.
- **Presentation**: arrowheads render in an overlay layer so they are not obscured by nodes.

## Gantt layout architecture
- **Timeline model**: tasks normalize to start/duration pairs from date + duration tokens.
- **Grid**: day-level ticks + section bands provide consistent visual rhythm.
- **Status styling**: done/active/crit/milestone map to distinct fill colors.

## Diagrams
- `docs/diagrams/architecture.mmd`
- `docs/diagrams/pipeline.mmd`

## Sequence frame note ownership

`SequenceFrame::start_idx..end_idx` identifies the frame's messages, while
`SequenceFrame::note_range` is a half-open range into `Graph::sequence_notes`.
The parser records note counts when opening and closing a frame. This distinguishes
notes inside a frame from notes immediately before or after it, even when they
share the same message index. Nested frames can have overlapping note ranges.

Code constructing a `SequenceFrame` literal must provide `note_range`; use an empty
range such as `0..0` when it owns no notes. The new public field is a source-level
API change for downstream struct literals and must be considered when releasing.
Layout uses the range for frame bounds and space before a following note. Empty
message frames and complex nested-frame layout remain separate limitations.
