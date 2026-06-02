// swift-tools-version: 5.9
import PackageDescription

// MermaidRenderKit — a headless Swift wrapper over the `mermaid-rs-renderer`
// C-ABI (the `Cmmdr` module), rendering Mermaid source to SVG or PNG with no
// WebView, no JavaScript, and no network I/O.
//
// The binary target's `url`/`checksum` below are PLACEHOLDERS. They are stamped
// with the real Release-hosted xcframework URL and checksum by
// `.github/workflows/swift-release.yml` at release time (the "release-driven
// tag" flow): CI builds the xcframework, computes the checksum, rewrites these
// two fields, commits, and tags that commit — so the tag that SwiftPM / the
// Swift Package Index resolves always carries a valid, stamped manifest. The
// default-branch HEAD keeps the placeholders.
let package = Package(
    name: "MermaidRenderKit",
    platforms: [
        .iOS(.v16),
        .macOS(.v13)
    ],
    products: [
        .library(name: "MermaidRenderKit", targets: ["MermaidRenderKit"])
    ],
    targets: [
        .binaryTarget(
            name: "CMermaidFFI",
            url: "{{XCFRAMEWORK_URL}}",
            checksum: "{{XCFRAMEWORK_SHA}}"
        ),
        .target(
            name: "MermaidRenderKit",
            dependencies: ["CMermaidFFI"]
        ),
        .testTarget(
            name: "MermaidRenderKitTests",
            dependencies: ["MermaidRenderKit"],
            // The Rust crate already owns the lowercase `tests/` directory, and
            // macOS's case-insensitive filesystem cannot host both `tests/` and
            // the SwiftPM-default `Tests/`. Use a distinct, collision-free path.
            path: "MermaidRenderKitTests"
        )
    ]
)
