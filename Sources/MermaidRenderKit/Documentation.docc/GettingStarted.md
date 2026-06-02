# Getting Started

Render a Mermaid diagram to SVG or PNG, with optional theming and timeouts.

## Add the dependency

```swift
.package(url: "https://github.com/1jehuang/mermaid-rs-renderer.git", from: "1.0.0")
```

Then add `MermaidRenderKit` to your target's dependencies.

> Note: The Swift package is versioned independently of the Rust crate (which
> follows its own `0.x` line on crates.io). Swift releases start at `1.0.0`.

## Render to SVG

``MermaidNativeRenderer/render(code:format:options:)`` returns a
``MermaidRenderResult``:

```swift
let result = MermaidNativeRenderer.render(code: source, format: .vectorSVG)
guard case .success(let payload, let size) = result,
      case .svg(let svg) = payload else { return }
// `svg` is sanitized SVG text; `size` is the intrinsic size for layout.
```

## Render to PNG

```swift
let result = MermaidNativeRenderer.render(code: source, format: .rasterPNG)
guard case .success(let payload, _) = result,
      case .png(let data) = payload else { return }
// `data` is PNG bytes — wrap in `Data`/an image, never decode as text.
```

## Apply a theme

Resolve your app's colors to raw `0xRRGGBBAA` values first (the kit is
headless and does not resolve dynamic platform colors), then build options:

```swift
let theme = MermaidThemeOptions(
    backgroundRGBA: 0xFFFFFFFF,
    foregroundRGBA: 0x111111FF,
    accentRGBA:     0x2A6BF2FF
)
let result = MermaidNativeRenderer.render(
    code: source, format: .vectorSVG, options: theme.toMmdrOptions()
)
```

## Bound work with the guard

``MermaidRenderGuard`` adds a source-size cap, a node cap, a timeout, and a
bounded result cache around the renderer for async, DoS-resistant rendering:

```swift
let result = await MermaidRenderGuard.render(
    code: source, format: .vectorSVG, options: theme.toMmdrOptions()
)
```
