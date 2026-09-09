use fontdb::{Database, Family, Query, Stretch, Style, Weight};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Mutex;
use ttf_parser::{Face, GlyphId};

use crate::unicode_width::{Cluster, consume_cluster, is_cjk_wide_char};

static TEXT_MEASURER: Lazy<Mutex<TextMeasurer>> = Lazy::new(|| Mutex::new(TextMeasurer::new()));
const FONT_CACHE_VERSION: &str = "v2-font-family-case";

pub fn measure_text_width(text: &str, font_size: f32, font_family: &str) -> Option<f32> {
    if text.is_empty() || font_size <= 0.0 {
        return Some(0.0);
    }
    // Ignore-poison idiom: a panic elsewhere while the measurer lock was held
    // poisons the mutex, but the measurer's only state is a glyph-advance cache
    // with no cross-render invariant, so recovering the inner value is sound.
    // Using `.ok()?` here instead would permanently degrade every subsequent
    // measurement to `None` (the ~0.56em heuristic fallback) for the process
    // lifetime after a single transient panic.
    let mut guard = TEXT_MEASURER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
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

struct TextMeasurer {
    db: Database,
    loaded_system_fonts: bool,
    cache: HashMap<String, Option<FontFace>>,
}

impl TextMeasurer {
    fn new() -> Self {
        let db = Database::new();
        Self {
            db,
            loaded_system_fonts: false,
            cache: HashMap::new(),
        }
    }

    fn measure(&mut self, text: &str, font_size: f32, font_family: &str) -> Option<f32> {
        let family_key = normalize_family_key(font_family);
        let face = if self.cache.contains_key(&family_key) {
            self.cache
                .get_mut(&family_key)
                .and_then(|face| face.as_mut())
        } else {
            let face = self.load_face(font_family);
            self.cache.insert(family_key.clone(), face);
            self.cache
                .get_mut(&family_key)
                .and_then(|face| face.as_mut())
        }?;
        let normalized = text.replace('\t', "    ");
        face.measure_width(&normalized, font_size)
    }

    fn load_face(&mut self, font_family: &str) -> Option<FontFace> {
        let family_key = normalize_family_key(font_family);
        if let Some(face) = load_cached_face(&family_key) {
            return Some(face);
        }
        if !self.loaded_system_fonts {
            self.db.load_system_fonts();
            self.loaded_system_fonts = true;
        }
        #[derive(Clone, Copy)]
        enum FamilyToken {
            Generic(fontdb::Family<'static>),
            Name(usize),
        }

        let mut names: Vec<String> = Vec::new();
        let mut order: Vec<FamilyToken> = Vec::new();
        for part in font_family.split(',') {
            let raw = part.trim().trim_matches('"').trim_matches('\'');
            if raw.is_empty() {
                continue;
            }
            let lower = raw.to_ascii_lowercase();
            match lower.as_str() {
                "serif" => order.push(FamilyToken::Generic(Family::Serif)),
                "sans-serif" => order.push(FamilyToken::Generic(Family::SansSerif)),
                "monospace" => order.push(FamilyToken::Generic(Family::Monospace)),
                "cursive" => order.push(FamilyToken::Generic(Family::Cursive)),
                "fantasy" => order.push(FamilyToken::Generic(Family::Fantasy)),
                "system-ui" | "-apple-system" | "ui-sans-serif" => {
                    order.push(FamilyToken::Generic(Family::SansSerif))
                }
                "ui-monospace" => order.push(FamilyToken::Generic(Family::Monospace)),
                _ => {
                    let idx = names.len();
                    names.push(
                        canonical_family_name(&self.db, raw).unwrap_or_else(|| raw.to_string()),
                    );
                    order.push(FamilyToken::Name(idx));
                }
            }
        }
        if order.is_empty() {
            order.push(FamilyToken::Generic(Family::SansSerif));
        }

        let mut families: Vec<Family<'_>> = Vec::with_capacity(order.len());
        for token in order {
            match token {
                FamilyToken::Generic(family) => families.push(family),
                FamilyToken::Name(idx) => families.push(Family::Name(names[idx].as_str())),
            }
        }

        let query = Query {
            families: &families,
            weight: Weight::NORMAL,
            stretch: Stretch::Normal,
            style: Style::Normal,
        };
        let id = self.db.query(&query)?;
        let mut loaded: Option<FontFace> = None;
        self.db.with_face_data(id, |data, index| {
            let bytes = data.to_vec();
            if let Ok(face) = Face::parse(&bytes, index) {
                let units_per_em = face.units_per_em().max(1);
                if let Some((font_path, meta_path)) = cache_paths(&family_key)
                    && !font_path.exists()
                {
                    if let Some(parent) = font_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let _ = fs::write(&font_path, &bytes);
                    let _ = fs::write(&meta_path, index.to_string());
                }
                loaded = Some(FontFace::new(bytes, index, units_per_em));
            }
        });
        loaded
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

fn canonical_family_name(db: &Database, raw: &str) -> Option<String> {
    db.faces()
        .flat_map(|face| face.families.iter().map(|(family, _)| family))
        .find(|family| family.eq_ignore_ascii_case(raw))
        .cloned()
}

fn normalize_family_key(font_family: &str) -> String {
    let trimmed = font_family.trim();
    let family = if trimmed.is_empty() {
        "sans-serif"
    } else {
        trimmed
    };
    format!("{FONT_CACHE_VERSION}:{family}")
}

fn cache_paths(family_key: &str) -> Option<(PathBuf, PathBuf)> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    family_key.hash(&mut hasher);
    let hash = hasher.finish();
    let dir = base.join("mmdr").join("font-cache");
    let font_path = dir.join(format!("{hash:x}.font"));
    let meta_path = dir.join(format!("{hash:x}.meta"));
    Some((font_path, meta_path))
}

fn load_cached_face(family_key: &str) -> Option<FontFace> {
    let (font_path, meta_path) = cache_paths(family_key)?;
    if !font_path.exists() || !meta_path.exists() {
        return None;
    }
    let bytes = fs::read(font_path).ok()?;
    let index: u32 = fs::read_to_string(meta_path).ok()?.trim().parse().ok()?;
    let face = Face::parse(&bytes, index).ok()?;
    let units_per_em = face.units_per_em().max(1);
    Some(FontFace::new(bytes, index, units_per_em))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fontdb::{FaceInfo, ID, Language, Source};
    use std::sync::Arc;

    fn push_family(db: &mut Database, family: &str) {
        db.push_face_info(FaceInfo {
            id: ID::dummy(),
            source: Source::Binary(Arc::new(Vec::<u8>::new())),
            index: 0,
            families: vec![(family.to_string(), Language::English_UnitedStates)],
            post_script_name: family.replace(' ', ""),
            style: Style::Normal,
            weight: Weight::NORMAL,
            stretch: Stretch::Normal,
            monospaced: false,
        });
    }

    #[test]
    fn measure_empty_text_is_zero() {
        assert_eq!(measure_text_width("", 16.0, "sans-serif"), Some(0.0));
    }

    #[test]
    fn canonical_family_name_matches_case_insensitively() {
        let mut db = Database::new();
        push_family(&mut db, "Trebuchet MS");

        assert_eq!(
            canonical_family_name(&db, "trebuchet ms").as_deref(),
            Some("Trebuchet MS")
        );
    }

    #[test]
    fn normalize_family_key_uses_versioned_cache_namespace() {
        assert_eq!(
            normalize_family_key("trebuchet ms"),
            "v2-font-family-case:trebuchet ms"
        );
    }

    /// NF-03: a panic that poisons the shared measurer mutex must NOT
    /// permanently degrade subsequent measurements. The ignore-poison idiom in
    /// `measure_text_width` recovers the inner measurer, so a measurement after
    /// a poisoning panic returns the SAME value as before it (rather than
    /// `None`/heuristic for the process lifetime, as `.lock().ok()?` would).
    #[test]
    fn measurer_recovers_after_lock_poison() {
        // Baseline (also forces the lazy measurer init).
        let baseline = measure_text_width("Hello", 16.0, "sans-serif");
        assert!(baseline.is_some(), "baseline measurement must succeed");

        // Poison the mutex: panic while holding the lock.
        let _ = std::thread::spawn(|| {
            let _guard = TEXT_MEASURER.lock().expect("acquire lock to poison");
            panic!("intentional poison for NF-03 recovery test");
        })
        .join();
        assert!(
            TEXT_MEASURER.is_poisoned(),
            "mutex should be poisoned after the panic"
        );

        // After poison, measurement must still succeed and match the baseline,
        // proving the inner measurer was recovered rather than abandoned.
        let after = measure_text_width("Hello", 16.0, "sans-serif");
        assert_eq!(after, baseline, "measurer must recover from poison");
    }
}
