use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use ttf_parser::{Face, GlyphId};

use crate::embedded_font::EMBEDDED_FONT;
use crate::unicode_width::{Cluster, consume_cluster, is_cjk_wide_char};

static TEXT_MEASURER: Lazy<Mutex<TextMeasurer>> = Lazy::new(|| Mutex::new(TextMeasurer::new()));

pub fn measure_text_width(text: &str, font_size: f32, font_family: &str) -> Option<f32> {
    if text.is_empty() || font_size <= 0.0 {
        return Some(0.0);
    }
    let mut guard = TEXT_MEASURER.lock().ok()?;
    guard.measure(text, font_size, font_family)
}

pub fn average_char_width(font_family: &str, font_size: f32) -> Option<f32> {
    if font_size <= 0.0 {
        return None;
    }
    let sample = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let width = measure_text_width(sample, font_size, font_family)?;
    let count = sample.chars().count().max(1) as f32;
    Some(width / count)
}

/// Text measurement against the compile-time embedded face.
///
/// All CSS family names resolve to the single embedded DejaVu Sans face: the
/// renderer performs no system-font enumeration and no on-disk caching, so
/// layout metrics are deterministic and identical across every platform
/// (including sandboxed iOS). The `font_family` argument is retained for API
/// compatibility but never changes which face is used.
struct TextMeasurer {
    face: Option<FontFace>,
    initialized: bool,
}

impl TextMeasurer {
    fn new() -> Self {
        Self {
            face: None,
            initialized: false,
        }
    }

    fn measure(&mut self, text: &str, font_size: f32, _font_family: &str) -> Option<f32> {
        if !self.initialized {
            self.face = FontFace::from_embedded();
            self.initialized = true;
        }
        let face = self.face.as_mut()?;
        let normalized = text.replace('\t', "    ");
        face.measure_width(&normalized, font_size)
    }
}

struct FontFace {
    data: Vec<u8>,
    index: u32,
    units_per_em: u16,
    ascii_advances: Option<[u16; 128]>,
    glyph_cache: HashMap<char, Option<u16>>,
    advance_cache: HashMap<u16, u16>,
}

impl FontFace {
    /// Build the face from the embedded font bytes. This is the only source of
    /// font data — there is no filesystem fallback.
    fn from_embedded() -> Option<FontFace> {
        let bytes = EMBEDDED_FONT.to_vec();
        let units_per_em = Face::parse(&bytes, 0).ok()?.units_per_em().max(1);
        Some(FontFace::new(bytes, 0, units_per_em))
    }

    fn new(data: Vec<u8>, index: u32, units_per_em: u16) -> Self {
        let ascii_advances = Face::parse(&data, index).ok().map(|parsed| {
            let mut advances = [0u16; 128];
            for byte in 0u8..=127 {
                let ch = byte as char;
                if let Some(glyph_id) = parsed.glyph_index(ch) {
                    advances[byte as usize] = parsed.glyph_hor_advance(glyph_id).unwrap_or(0);
                }
            }
            advances
        });
        Self {
            data,
            index,
            units_per_em,
            ascii_advances,
            glyph_cache: HashMap::new(),
            advance_cache: HashMap::new(),
        }
    }

    fn measure_width(&mut self, text: &str, font_size: f32) -> Option<f32> {
        let scale = font_size / self.units_per_em as f32;
        let fallback = font_size * 0.56;

        if text.is_ascii()
            && let Some(advances) = &self.ascii_advances
        {
            let mut width = 0.0f32;
            for byte in text.as_bytes() {
                if *byte == b'\n' {
                    continue;
                }
                let advance = advances[*byte as usize];
                if advance == 0 {
                    width += fallback;
                } else {
                    width += advance as f32 * scale;
                }
            }
            return Some(width.max(0.0));
        }

        let face = Face::parse(&self.data, self.index).ok()?;
        let scale = font_size / self.units_per_em as f32;
        let mut width = 0.0f32;
        let chars: Vec<char> = text.chars().collect();
        let mut idx = 0usize;

        while idx < chars.len() {
            let ch = chars[idx];
            if ch == '\n' {
                idx += 1;
                continue;
            }

            // Mirror fallback_text_width grapheme-cluster handling so both
            // measurement paths agree on widths for CJK ideographs, kana,
            // hangul, fullwidth chars, and emoji sequences. The OS will
            // render these via system font fallback (e.g. PingFang on iOS)
            // at roughly 1em even when the loaded face has no glyph for
            // them — using 0.56em here under-measures CJK by ~44%.
            if let Some((kind, new_idx)) = consume_cluster(&chars, idx) {
                width += match kind {
                    Cluster::Wide => font_size,
                    Cluster::ZeroWidth => 0.0,
                };
                idx = new_idx;
                continue;
            }

            let glyph = if let Some(cached) = self.glyph_cache.get(&ch) {
                *cached
            } else {
                let glyph = face.glyph_index(ch).map(|id| id.0);
                self.glyph_cache.insert(ch, glyph);
                glyph
            };

            if let Some(glyph_id) = glyph {
                let advance = if let Some(value) = self.advance_cache.get(&glyph_id) {
                    *value
                } else {
                    let value = face.glyph_hor_advance(GlyphId(glyph_id)).unwrap_or(0);
                    self.advance_cache.insert(glyph_id, value);
                    value
                };
                width += advance as f32 * scale;
            } else if is_cjk_wide_char(ch) {
                width += font_size;
            } else {
                width += fallback;
            }
            idx += 1;
        }

        Some(width.max(0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_empty_text_is_zero() {
        assert_eq!(measure_text_width("", 16.0, "sans-serif"), Some(0.0));
    }

    /// F1-AC6: the embedded face must produce a KNOWN advance, not merely a
    /// non-zero one. "Hello" sums to 5191 font units in DejaVu Sans
    /// (H=1540, e=1260, l=569, l=569, o=1253) at units_per_em=2048.
    #[test]
    fn measure_hello_matches_known_embedded_advance() {
        const HELLO_UNITS: f32 = 5191.0;
        const UPEM: f32 = 2048.0;
        for font_size in [12.0_f32, 16.0, 32.0, 2048.0] {
            let expected = HELLO_UNITS * font_size / UPEM;
            let actual = measure_text_width("Hello", font_size, "sans-serif")
                .expect("embedded face must measure");
            assert!(
                (actual - expected).abs() < 0.01,
                "size {font_size}: expected {expected} px, got {actual} px"
            );
        }
    }

    /// RMF1: every CSS family resolves to the single embedded face, so the
    /// measured width is identical regardless of the requested family name.
    #[test]
    fn all_families_resolve_to_embedded_face() {
        let reference = measure_text_width("Hello", 16.0, "sans-serif").unwrap();
        for family in [
            "Inter",
            "'trebuchet ms'",
            "verdana",
            "arial",
            "monospace",
            "serif",
            "Some Unknown Family",
            "",
        ] {
            let width = measure_text_width("Hello", 16.0, family).unwrap();
            assert!(
                (width - reference).abs() < 0.001,
                "family {family:?} measured {width}, expected {reference}"
            );
        }
    }
}
