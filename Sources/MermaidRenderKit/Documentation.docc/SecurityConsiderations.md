# Security Considerations

How MermaidRenderKit bounds untrusted input and sanitizes output.

## No ambient capabilities

The renderer performs **no network I/O**, runs **no JavaScript**, and reads
**no files** — text metrics and PNG rasterization use a compile-time embedded
font, so there is no system-font enumeration and no on-disk font cache. This
holds on sandboxed iOS where `$HOME` and system fonts are unavailable.

## Denial-of-service bounds

``MermaidRenderGuard`` applies caps before and during rendering:

- **Source-size cap** — input larger than the configured byte limit is
  rejected before parsing.
- **Node cap** — diagrams exceeding the node limit are rejected pre-layout.
- **Timeout** — rendering runs under a deadline and is cancelled if exceeded.

The underlying C ABI also runs every render inside a `catch_unwind` boundary,
so a panic in the renderer maps to a render error rather than crashing the host.

## SVG output sanitization

SVG is an active document format. ``SVGSanitizer`` strips egress vectors from
rendered SVG before it reaches a display layer — script elements, event-handler
attributes, and external references — while preserving inert content such as
`data:` images and local fragment references. Always treat rendered SVG as
untrusted until sanitized.

## Sizing

``SVGSizing`` extracts an intrinsic ``SVGIntrinsicSize`` from the rendered SVG
(via `viewBox` or width/height) for layout, with a sensible floor so degenerate
diagrams remain visible.
