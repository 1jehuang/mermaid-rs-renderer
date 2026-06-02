//! Egress-locked, zero-filesystem SVG→PNG rasterization (PRD §4.7b / v7-MF1–3).
//!
//! The wrapper constructs `usvg::Options` itself — never `Options::default()`
//! alone — so it can apply ALL THREE locks the PNG path requires:
//!
//! 1. **data:-only href resolver** (F1-AC5 PNG path / CWE-918): `resolve_string`
//!    refuses every `http(s)://`/`file://`/relative href (returns `None`); only
//!    inline `data:` URIs (via the default data resolver) survive.
//! 2. **`resources_dir = None`** (v7-MF3 / CWE-918): usvg resolves a RELATIVE
//!    `<image href="logo.png">` by joining `resources_dir` and `std::fs::read`
//!    BEFORE the href-resolver closure is consulted — leaving it set is an
//!    independent filesystem-egress surface the resolver does not cover.
//! 3. **embedded-font `fontdb`** (v7-MF2 / CWE-732): usvg lays out the SVG's
//!    `<text>`/`<tspan>` with ITS OWN fontdb. We inject the SAME compile-time
//!    embedded face the fork measures with (`embedded_font::get_font_database`),
//!    and NEVER call `load_system_fonts()` — otherwise iOS reintroduces system
//!    font enumeration + `~/.cache` writes AND the PNG text is laid out with the
//!    wrong advances vs the SVG's `<text>` positions (silent degradation).

use std::sync::{Arc, OnceLock};

use mermaid_rs_renderer::embedded_font;
use mermaid_rs_renderer::Theme;

/// A rasterization failure. Carries no detail across the FFI boundary — `abi`
/// maps it to the redacted internal-error path.
#[derive(Debug)]
pub enum RasterError {
    /// usvg failed to parse our own SVG (an internal renderer bug).
    Parse,
    /// The target pixmap could not be allocated (zero size or OOM).
    Pixmap,
    /// PNG encoding failed.
    Encode,
}

/// The wrapper's own embedded `fontdb`, seeded once from the SAME embedded face
/// the fork registers (`get_font_database`), wrapped in an `Arc` for
/// `usvg::Options.fontdb`. Built once; cloning the `Arc` is cheap. No
/// `load_system_fonts()` is ever called on it.
fn embedded_fontdb() -> Arc<fontdb::Database> {
    static DB: OnceLock<Arc<fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| Arc::new(embedded_font::get_font_database().clone()))
        .clone()
}

/// Force the embedded-font database `OnceLock` to evaluate (latency hint for
/// `mmdr_init`). Idempotent; cheap after the first call.
pub(crate) fn warm() {
    let _ = embedded_fontdb();
}

/// Build the locked `usvg::Options` (all three locks applied).
fn locked_options() -> usvg::Options<'static> {
    let mut opt = usvg::Options {
        // Lock 2: no filesystem base dir for relative hrefs.
        resources_dir: None,
        // All generic CSS families already resolve to the embedded face; set
        // the primary too so a bare `font-family` with no generic also lands
        // on it.
        font_family: embedded_font::EMBEDDED_FAMILY.to_string(),
        // Lock 1: only inline `data:` URIs resolve; every external/relative
        // string href is refused.
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: usvg::ImageHrefResolver::default_data_resolver(),
            resolve_string: Box::new(|_href, _opts| None),
        },
        ..Default::default()
    };
    // Lock 3: the embedded-font database (NOT system fonts).
    opt.fontdb = embedded_fontdb();
    opt
}

/// Parse a `#RGB` / `#RRGGBB` / `#RRGGBBAA` CSS hex into a tiny-skia color for
/// the background fill. Mirrors the fork's `parse_hex_color`; non-hex (named /
/// `rgb()` / `hsl()`) colors yield `None` and leave the canvas transparent.
fn parse_hex_color(input: &str) -> Option<resvg::tiny_skia::Color> {
    let hex = input.trim().strip_prefix('#')?;
    if !hex.is_ascii() {
        return None;
    }
    let (r, g, b, a) = match hex.len() {
        3 => (
            u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?,
            u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?,
            u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?,
            255,
        ),
        6 => (
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
            255,
        ),
        8 => (
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
            u8::from_str_radix(&hex[6..8], 16).ok()?,
        ),
        _ => return None,
    };
    resvg::tiny_skia::Color::from_rgba8(r, g, b, a).into()
}

/// Rasterize the hand-rolled SVG to PNG bytes with all three locks applied.
/// Returns binary PNG bytes on success.
pub fn rasterize_png(svg: &str, theme: &Theme) -> Result<Vec<u8>, RasterError> {
    let opt = locked_options();

    let tree = usvg::Tree::from_str(svg, &opt).map_err(|_| RasterError::Parse)?;
    let size = tree.size().to_int_size();
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(size.width(), size.height()).ok_or(RasterError::Pixmap)?;

    if let Some(color) = parse_hex_color(&theme.background) {
        pixmap.fill(color);
    }

    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );

    pixmap.encode_png().map_err(|_| RasterError::Encode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mermaid_rs_renderer::{render_strict, RenderOptions};

    fn svg_of(src: &str) -> String {
        render_strict(src, RenderOptions::default()).unwrap()
    }

    #[test]
    fn rasterizes_valid_svg_to_png_magic() {
        let svg = svg_of("flowchart LR\nA-->B");
        let png = rasterize_png(&svg, &Theme::modern()).unwrap();
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    }

    #[test]
    fn external_string_href_is_refused() {
        // An <image> with an http href must NOT be resolved (egress lock 1).
        // usvg drops the unresolved image; rasterization still succeeds.
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40">
            <image href="http://evil.example/x.png" width="40" height="40"/>
        </svg>"#;
        let png = rasterize_png(svg, &Theme::modern()).unwrap();
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    }

    #[test]
    fn relative_href_is_refused_no_filesystem_read() {
        // resources_dir = None (lock 2): a relative href cannot be joined to a
        // filesystem dir, so no std::fs::read occurs; render still succeeds.
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40">
            <image href="logo.png" width="40" height="40"/>
        </svg>"#;
        let png = rasterize_png(svg, &Theme::modern()).unwrap();
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    }

    #[test]
    fn hex_color_parses_all_forms() {
        assert!(parse_hex_color("#fff").is_some());
        assert!(parse_hex_color("#112233").is_some());
        assert!(parse_hex_color("#11223344").is_some());
        assert!(parse_hex_color("rebeccapurple").is_none());
        assert!(parse_hex_color("rgb(1,2,3)").is_none());
    }

    // Serializes the process-global `$HOME`/`$XDG_CACHE_HOME` mutation below so
    // it cannot race other parallel tests. (No other test in this crate reads
    // those vars — the fontdb filesystem cache was removed in Phase 2b.)
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn text_render_under_nonwritable_home_is_unaffected_zero_font_io() {
        // T2b(3) / v7-MF2: a text-bearing diagram rasterizes correctly with
        // `$HOME`/`$XDG_CACHE_HOME` pointed at a non-writable sentinel — proving
        // the injected embedded `fontdb` lays out text with ZERO filesystem font
        // I/O (`load_system_fonts()` never fires; no `~/.cache` read/write).
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // Force the embedded fontdb to initialize BEFORE the sentinel $HOME is
        // set, so warming is order-independent (not reliant on svg_of warming it).
        warm();
        let svg = svg_of("flowchart LR\nAlpha-->Beta"); // node labels = text
        let baseline = rasterize_png(&svg, &Theme::modern());

        let old_home = std::env::var_os("HOME");
        let old_xdg = std::env::var_os("XDG_CACHE_HOME");
        std::env::set_var("HOME", "/nonexistent/mmdr-sentinel");
        std::env::set_var("XDG_CACHE_HOME", "/nonexistent/mmdr-sentinel/cache");
        // Capture the Result WITHOUT unwrapping, then ALWAYS restore the env
        // before asserting — a panic here must not leave the process polluted.
        let under_sentinel = rasterize_png(&svg, &Theme::modern());
        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_xdg {
            Some(v) => std::env::set_var("XDG_CACHE_HOME", v),
            None => std::env::remove_var("XDG_CACHE_HOME"),
        }

        let baseline = baseline.unwrap();
        let under_sentinel = under_sentinel.unwrap();
        assert_eq!(
            &under_sentinel[..8],
            &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
        );
        // Byte-identical to the baseline ⇒ the rasterizer's text layout does not
        // depend on any filesystem path the sentinel `$HOME` would have broken.
        assert_eq!(
            baseline, under_sentinel,
            "PNG text layout must be independent of $HOME (embedded fontdb, no fs font I/O)"
        );
    }
}
