//! Compile-time embedded font with zero filesystem I/O.
//!
//! The renderer must run on sandboxed platforms (iOS) where system-font
//! enumeration and on-disk caches are unavailable, and must cross-compile
//! without a fontconfig C library. This module embeds a subsetted DejaVu Sans
//! face at compile time and exposes a single shared [`fontdb::Database`] whose
//! generic-family slots all resolve to that face, so every CSS family name an
//! SVG references (`Inter`, `trebuchet ms`, `arial`, `sans-serif`, …) maps to
//! the one embedded font. No `fs::read`/`fs::write`, no `load_system_fonts`.

/// Subsetted DejaVu Sans (Latin + punctuation coverage).
///
/// Licensed under the Bitstream Vera / DejaVu license — see
/// `LICENSES/embedded-font/`. The subset is 23.5 KB, well under the 400 KB cap.
pub const EMBEDDED_FONT: &[u8] = include_bytes!("../assets/DejaVuSans-subset.ttf");

/// Family name stored in the embedded face's `name` table.
pub const EMBEDDED_FAMILY: &str = "DejaVu Sans";

/// Shared font database used by the PNG raster path.
///
/// Loaded exactly once. Contains only the embedded face, with every generic
/// family (`serif`/`sans-serif`/`monospace`/`cursive`/`fantasy`) forced to it
/// so font resolution can never reach the filesystem.
#[cfg(feature = "png")]
pub fn get_font_database() -> &'static fontdb::Database {
    use std::sync::OnceLock;
    static FONT_DB: OnceLock<fontdb::Database> = OnceLock::new();
    FONT_DB.get_or_init(|| {
        let mut db = fontdb::Database::new();
        db.load_font_data(EMBEDDED_FONT.to_vec());
        db.set_sans_serif_family(EMBEDDED_FAMILY);
        db.set_serif_family(EMBEDDED_FAMILY);
        db.set_monospace_family(EMBEDDED_FAMILY);
        db.set_cursive_family(EMBEDDED_FAMILY);
        db.set_fantasy_family(EMBEDDED_FAMILY);
        db
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ttf_parser::Face;

    #[test]
    fn embedded_font_parses_and_reports_dejavu_family() {
        let face = Face::parse(EMBEDDED_FONT, 0).expect("embedded face must parse");
        assert_eq!(face.units_per_em(), 2048);
        let has_family = face
            .names()
            .into_iter()
            .filter_map(|n| n.to_string())
            .any(|s| s == EMBEDDED_FAMILY);
        assert!(
            has_family,
            "embedded face must advertise '{EMBEDDED_FAMILY}'"
        );
    }

    #[test]
    fn embedded_blob_under_400kb() {
        assert!(
            EMBEDDED_FONT.len() <= 400 * 1024,
            "embedded font {} bytes exceeds 400 KB cap",
            EMBEDDED_FONT.len()
        );
    }

    #[cfg(feature = "png")]
    #[test]
    fn font_database_has_single_embedded_face() {
        let db = get_font_database();
        assert_eq!(db.len(), 1, "database must hold only the embedded face");
    }
}
