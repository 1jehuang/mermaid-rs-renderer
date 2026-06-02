# ``MermaidRenderKit``

A headless Mermaid diagram renderer for Apple platforms.

## Overview

MermaidRenderKit renders Mermaid diagram source to **SVG** (vector) or **PNG**
(raster) on iOS and macOS. The renderer is a prebuilt Rust binary (the
``mermaid-rs-renderer`` core, wrapped in a C ABI) — there is **no WebView, no
JavaScript engine, and no network I/O**. Rendering is pure computation against
a compile-time embedded font, so output is deterministic across devices,
including sandboxed iOS.

```swift
import MermaidRenderKit

let result = MermaidNativeRenderer.render(
    code: "flowchart LR\n  A --> B",
    format: .vectorSVG
)

switch result {
case .success(let payload, _):
    if case .svg(let svg) = payload { /* display the SVG string */ }
case .unsupported, .parseError, .renderError:
    break
}
```

The kit is **headless**: it produces an SVG `String` or PNG `Data` and does not
depend on SwiftUI, UIKit, or AppKit. Display and theming resolution belong to
the calling application.

## Topics

### Rendering

- ``MermaidNativeRenderer``
- ``MermaidRenderGuard``
- <doc:GettingStarted>

### Results

- ``MermaidRenderResult``
- ``MermaidRenderPayload``
- ``MermaidDisplayFormat``

### Theming

- ``MermaidThemeOptions``

### Caching

- ``MermaidRenderCache``
- ``MermaidCacheKey``

### Accessibility

- ``MermaidAccessibility``
- ``MermaidDiagramType``

### SVG Utilities & Security

- <doc:SecurityConsiderations>
- ``SVGSanitizer``
- ``SVGSizing``
- ``SVGIntrinsicSize``
