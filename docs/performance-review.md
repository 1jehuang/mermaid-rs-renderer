# Performance review of diagram semantics and geometry fixes

The review compares `e572e1f` with the follow-up optimizations on macOS ARM64,
Rust 1.90.0, using `release-fast` and fast text metrics. These are local synthetic
measurements, not latency guarantees for arbitrary diagrams.

## Findings and changes

- **Clamped labels:** the new release check scanned every node for each label.
  It now queries the existing obstacle grid and filters out non-node entries.
- **Attachment correction:** candidate scoring repeatedly scanned all nodes,
  labels, and edge paths. Static geometry is indexed once, live text bounds are
  updated after each move, and fallback candidates are generated only when
  needed. Segment queries count each intersected edge once. Floating-point
  overlap sums retain the original obstacle order. Dense queries use an ordered
  scan when sorting the candidate list would cost more.
- **Large coordinates:** grid insertion and queries could enumerate cells in
  proportion to rectangle area. Each operation now visits at most 256 cells;
  oversized obstacles are kept separately, and oversized queries scan the index.
  This bounds grid storage by obstacle count rather than canvas dimensions.
- **Subgraphs:** each group previously scanned all edges with linear membership
  checks, even when it had no local direction. A reverse membership index now
  visits only the groups containing an edge's endpoints. Shared memberships
  cancel out; duplicate node entries and links to group IDs retain their meaning.
- **Sequence frames:** nested frames repeatedly scanned enclosed edges and all
  notes. Two range indexes use linear storage/build time and logarithmic queries.
  Frame records are borrowed, and section labels are measured once.
- **Parser:** continuation validation borrows the preceding line rather than
  accumulating another statement copy. Labels without entities keep their
  existing string allocation. Pipe-label masking and quadrant bounds add only
  linear passes.

## Scaling measurements

Median milliseconds over three samples (five for subgraph normalization). Fixture
construction is outside the timer. Attachment probes include both correction
rounds and all index construction. Sequence probes measure sequence layout,
including text measurement, but exclude parsing and later global label placement.

| Probe | Items | Before (ms) | After (ms) |
| --- | ---: | ---: | ---: |
| Sparse attachment correction | 128 | 4.738 | 1.741 |
| Sparse attachment correction | 512 | 31.549 | 6.960 |
| Sparse attachment correction | 2,048 | 350.560 | 15.429 |
| Isolated directed subgraphs | 128 | 0.201 | 0.044 |
| Isolated directed subgraphs | 512 | 2.718 | 0.182 |
| Isolated directed subgraphs | 2,048 | 33.254 | 0.752 |
| Nested sequence frames | 128 | 3.305 | 3.118 |
| Nested sequence frames | 512 | 14.674 | 10.549 |
| Nested sequence frames | 2,048 | 86.708 | 42.302 |

Run the retained probes with:

```sh
cargo test --locked --profile release-fast --no-default-features --lib \
  performance_scaling -- --ignored --nocapture --test-threads=1
```

The parser probe additionally exercises up to 8,192 continued edges with Unicode,
entities, and arrow-looking pipe labels (70.780 ms at 8,192 edges in a separate
all-features run). A dense attachment probe deliberately overlays all geometry
to expose the residual dense-intersection cost: 1.268 / 15.985 / 258.439 ms
for 128 / 512 / 2,048 overlaid labels in the final all-features probe. This remains
quadratic as expected; it is not hidden by the sparse-case speedups. These tests are intentionally ignored in
normal CI; elapsed-time assertions would be unreliable across runners. Ordinary
unit tests compare indexed results with direct scans, verify sparse candidate
counts, and cover oversized coordinates, moves, nested ranges, and invalid ranges.

## Remaining limits

Spatial indexing removes scans of unrelated geometry; it cannot remove the cost
of actual dense intersections. If every obstacle occupies the same region, or many
obstacles exceed the grid cell budget, queries can still visit every obstacle and
the complete correction pass can remain quadratic. Fallback candidate counts and
correction rounds are bounded, and indexing never drops collision checks.

Subgraph work scales with the memberships of edge endpoints. Deeply overlapping
subgraphs can therefore still cost more than a flat graph. Range indexing removes
repeated frame-content scans, but does not redesign sequence label placement.

The existing routing, crossing minimization, and global label-placement passes
are outside this optimization. Complex dense diagrams can remain expensive in
those stages. This review does not establish a worst-case render-time bound;
applications rendering untrusted diagrams should retain their existing complexity
limits and process timeout.
