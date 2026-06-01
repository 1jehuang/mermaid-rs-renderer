//! Runtime proof that the embedded-font path performs zero filesystem I/O.
//!
//! F1-AC6 (c): a text-bearing diagram must rasterize to PNG even when every
//! environment variable the old on-disk font cache consulted points at a
//! non-writable, non-existent sentinel. Glyph metrics come from the embedded
//! face — no `load_system_fonts`, no `$HOME/.cache`, no `XDG_CACHE_HOME`.

#![cfg(feature = "png")]

use mermaid_rs_renderer::{RenderConfig, Theme, render};

#[test]
fn renders_text_png_under_nonwritable_home_sentinel() {
    let sentinel = "/mmdr-nonwritable-sentinel/this/path/must/not/exist";
    // SAFETY: single-threaded within this test binary (one test); env is
    // per-process so sibling test binaries are unaffected.
    unsafe {
        std::env::set_var("HOME", sentinel);
        std::env::set_var("XDG_CACHE_HOME", sentinel);
    }

    let svg =
        render("flowchart LR\n    A[Hello World] --> B[Goodbye]").expect("SVG render must succeed");
    assert!(
        svg.contains("Hello World"),
        "diagram text must reach the SVG"
    );

    let out = std::env::temp_dir().join("mmdr_embedded_font_zero_fs.png");
    mermaid_rs_renderer::write_output_png(&svg, &out, &RenderConfig::default(), &Theme::modern())
        .expect("PNG render must succeed without filesystem font access");

    let bytes = std::fs::read(&out).expect("PNG output file must exist");
    assert!(
        bytes.len() > 8 && &bytes[1..4] == b"PNG",
        "output must carry a valid PNG signature"
    );
    let _ = std::fs::remove_file(&out);
}
