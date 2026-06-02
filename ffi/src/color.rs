//! Color decoding + theme override application (PRD §4.3 Arbitration B).
//!
//! The options struct carries explicit `0xRRGGBBAA` channels gated by a
//! `color_override_mask` bitfield; the fork's `Theme` stores colors as CSS
//! strings. We map each set bit onto the corresponding `Theme` slot, formatting
//! the RGBA as an 8-digit `#RRGGBBAA` CSS hex (which usvg/resvg parse — the
//! fork's own `parse_hex_color` handles the 8-digit form), so a fully
//! transparent override (`Color.clear`, `0x00000000`) is expressible.

use mermaid_rs_renderer::Theme;

/// `color_override_mask` bit: background slot.
pub const MASK_BACKGROUND: u32 = 0b001;
/// `color_override_mask` bit: foreground (text + line) slot.
pub const MASK_FOREGROUND: u32 = 0b010;
/// `color_override_mask` bit: accent (primary) slot.
pub const MASK_ACCENT: u32 = 0b100;

/// Format an `0xRRGGBBAA` value as a CSS `#RRGGBBAA` hex string.
#[inline]
pub fn rgba_to_css(rgba: u32) -> String {
    format!("#{rgba:08X}")
}

/// Build the starting [`Theme`] for a `base_theme` selector (`0` = modern,
/// anything else = mermaid_default), then apply each color override whose mask
/// bit is set. Slots whose bit is unset keep the base-theme value.
///
/// Mask → slot mapping (PRD §4.3):
/// - `MASK_BACKGROUND` → `background`
/// - `MASK_FOREGROUND` → `text_color` AND `line_color` (foreground = text/line)
/// - `MASK_ACCENT` → `primary_color`
pub fn build_theme(
    base_theme: i32,
    mask: u32,
    background_rgba: u32,
    foreground_rgba: u32,
    accent_rgba: u32,
) -> Theme {
    let mut theme = if base_theme == 0 {
        Theme::modern()
    } else {
        Theme::mermaid_default()
    };

    if mask & MASK_BACKGROUND != 0 {
        theme.background = rgba_to_css(background_rgba);
    }
    if mask & MASK_FOREGROUND != 0 {
        let fg = rgba_to_css(foreground_rgba);
        theme.text_color = fg.clone();
        theme.line_color = fg;
    }
    if mask & MASK_ACCENT != 0 {
        theme.primary_color = rgba_to_css(accent_rgba);
    }

    theme
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba_formats_as_eight_digit_css_hex() {
        assert_eq!(rgba_to_css(0xFF5733FF), "#FF5733FF");
        assert_eq!(rgba_to_css(0x00000000), "#00000000"); // Color.clear is legal
        assert_eq!(rgba_to_css(0x123456AB), "#123456AB");
    }

    #[test]
    fn unset_mask_keeps_base_theme_colors() {
        let base = Theme::modern();
        let built = build_theme(0, 0, 0xAABBCCDD, 0x11223344, 0x55667788);
        assert_eq!(built.background, base.background);
        assert_eq!(built.text_color, base.text_color);
        assert_eq!(built.line_color, base.line_color);
        assert_eq!(built.primary_color, base.primary_color);
    }

    #[test]
    fn background_bit_overrides_only_background() {
        let base = Theme::modern();
        let built = build_theme(0, MASK_BACKGROUND, 0x010203FF, 0, 0);
        assert_eq!(built.background, "#010203FF");
        assert_eq!(built.text_color, base.text_color);
        assert_eq!(built.primary_color, base.primary_color);
    }

    #[test]
    fn foreground_bit_overrides_text_and_line() {
        let built = build_theme(0, MASK_FOREGROUND, 0, 0x0A0B0C0D, 0);
        assert_eq!(built.text_color, "#0A0B0C0D");
        assert_eq!(built.line_color, "#0A0B0C0D");
    }

    #[test]
    fn accent_bit_overrides_primary() {
        let built = build_theme(0, MASK_ACCENT, 0, 0, 0x99887766);
        assert_eq!(built.primary_color, "#99887766");
    }

    #[test]
    fn transparent_override_is_applied_not_ignored() {
        // The whole point of the mask (vs an alpha==0 sentinel): a fully
        // transparent color is a real override when its bit is set.
        let built = build_theme(0, MASK_BACKGROUND, 0x00000000, 0, 0);
        assert_eq!(built.background, "#00000000");
    }

    #[test]
    fn all_bits_override_all_slots() {
        let built = build_theme(
            0,
            MASK_BACKGROUND | MASK_FOREGROUND | MASK_ACCENT,
            0x11111111,
            0x22222222,
            0x33333333,
        );
        assert_eq!(built.background, "#11111111");
        assert_eq!(built.text_color, "#22222222");
        assert_eq!(built.line_color, "#22222222");
        assert_eq!(built.primary_color, "#33333333");
    }

    #[test]
    fn reserved_high_bits_are_ignored_not_rejected() {
        // Bits 3–31 set must not change behaviour vs the low 3 bits alone.
        let with_reserved = build_theme(0, 0xFFFF_FFFF, 0x010101FF, 0x020202FF, 0x030303FF);
        let low_only = build_theme(0, 0b111, 0x010101FF, 0x020202FF, 0x030303FF);
        assert_eq!(with_reserved.background, low_only.background);
        assert_eq!(with_reserved.text_color, low_only.text_color);
        assert_eq!(with_reserved.primary_color, low_only.primary_color);
    }

    #[test]
    fn base_theme_selector_picks_palette() {
        let modern = build_theme(0, 0, 0, 0, 0);
        let classic = build_theme(1, 0, 0, 0, 0);
        // The two base palettes differ somewhere; background is a safe witness.
        assert_eq!(modern.background, Theme::modern().background);
        assert_eq!(classic.background, Theme::mermaid_default().background);
    }
}
