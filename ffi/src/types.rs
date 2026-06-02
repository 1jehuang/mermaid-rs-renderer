//! Transparent `#[repr(C)]` ABI structs (PRD §4.3, Arbitrations B + E).
//!
//! Both structs are a documented, versioned byte-layout commitment — cbindgen
//! emits the concrete fields into `mmdr.h` and Swift reads them read-only. The
//! byte tables are pinned by the compile-time `const _` size asserts below and
//! by the cbindgen header-drift gate.

use std::ffi::c_void;

/// Current ABI version. Bumped to `2` for v0.6 to reflect the additive second
/// render entry point (`mmdr_render_png`); the bump is purely additive — the
/// `MmdrResult`/`MmdrOptions` byte layouts are byte-identical to v0.5, so a v0.5
/// caller that only uses `mmdr_render_svg` keeps working (see the version-check
/// operator in [`crate::abi`]).
pub const MMDR_ABI_VERSION: u32 = 2;

/// Transparent, versioned render result. Layout is part of the public ABI
/// (additive only). Total size = **40 bytes**, alignment = 8, no internal
/// padding (4-byte block precedes the 8-byte block).
#[repr(C)]
pub struct MmdrResult {
    /// `== MMDR_ABI_VERSION`; lets Swift assert header/binary agreement.
    pub abi_version: u32,
    /// `== size_of::<MmdrResult>()` at build; forward-compat guard.
    pub struct_size: u32,
    /// [`crate::status::MmdrStatus`] as `i32`.
    pub status: i32,
    /// 1-based line; valid only when `loc_valid != 0`. `0` = column/line not
    /// reported for this error shape.
    pub err_line: u32,
    /// 1-based column; valid only when `loc_valid != 0`. `0` = column not
    /// reported (e.g. line-only errors).
    pub err_col: u32,
    /// `0` = `err_line`/`err_col` are unknown/unpopulated; nonzero = at least
    /// `err_line` is accurate (replaces overloading `(1,1)` as a sentinel).
    pub loc_valid: u32,
    /// Payload byte length, no NUL terminator. FIXED-WIDTH `u64` (NOT `usize`)
    /// so the ABI is pointer-width-independent.
    pub data_len: u64,
    /// FORMAT-DEPENDENT payload (v7-MF5): UTF-8 SVG bytes (`mmdr_render_svg`,
    /// status Ok) / raw BINARY PNG bytes (`mmdr_render_png`, status Ok) /
    /// UTF-8 (redacted) error-message bytes (any status != Ok); may be null.
    /// NOTE: PNG bytes are binary, NOT UTF-8 — a Swift consumer MUST wrap them
    /// in `Data` for `UIImage`/`NSImage`, never `String(bytes:encoding:.utf8)`
    /// (which returns nil).
    pub data_ptr: *mut u8,
}

/// Forward-compatible POD options. Total size = **56 bytes**, alignment = 4, no
/// padding (every field is 4-byte). `abi_version` precedes `struct_size`,
/// IDENTICAL to `MmdrResult`'s guard-field order.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MmdrOptions {
    /// `== MMDR_ABI_VERSION` (FIRST, matching `MmdrResult`'s guard order).
    pub abi_version: u32,
    /// Caller sets `size_of::<MmdrOptions>()`; Rust validates exactly.
    pub struct_size: u32,
    /// Starting palette: `0` = modern, `1` = mermaid_default (additive).
    pub base_theme: i32,
    /// bit0 = background set, bit1 = foreground set, bit2 = accent set. A `0`
    /// bit => use `base_theme` for that slot (NOT an alpha sentinel). Bits 3–31
    /// RESERVED, MUST be zero; the wrapper IGNORES unrecognized high bits
    /// (forward-compat — no `ErrVersionMismatch` on a set reserved bit).
    pub color_override_mask: u32,
    /// `0xRRGGBBAA`; applied verbatim iff `(mask & 0b001)`. `0x00000000`
    /// (`Color.clear`) is a LEGAL value when its bit is set.
    pub background_rgba: u32,
    /// Text/line color; applied iff `(mask & 0b010)`.
    pub foreground_rgba: u32,
    /// Primary/accent; applied iff `(mask & 0b100)`.
    pub accent_rgba: u32,
    /// `<= 0` => default.
    pub node_spacing: f32,
    /// `<= 0` => default.
    pub rank_spacing: f32,
    /// `0` => unset.
    pub preferred_aspect_w: f32,
    /// `0` => unset.
    pub preferred_aspect_h: f32,
    /// DoS source-size cap; `0` => wrapper default (262144).
    pub max_source_bytes: u32,
    /// DoS node cap; `0` => wrapper default (5000).
    pub max_nodes: u32,
    /// ACCEPTED but IGNORED by the Rust wrapper in MVP — carries the Swift
    /// adapter's perceived-latency deadline for symmetry/forward-compat; the
    /// wrapper does NOT enforce a pure-Rust deadline in MVP. `0` => default.
    pub timeout_ms: u32,
}

// --- Compile-time ABI byte-layout pins (PRD §4.3 byte tables) -------------
// These fail the build if the layout ever drifts, independent of the cbindgen
// header-drift gate and the runtime `#[test]` assertions.
const _: () = assert!(std::mem::size_of::<MmdrResult>() == 40);
const _: () = assert!(std::mem::align_of::<MmdrResult>() == 8);
const _: () = assert!(std::mem::size_of::<MmdrOptions>() == 56);
const _: () = assert!(std::mem::align_of::<MmdrOptions>() == 4);

impl MmdrOptions {
    /// Effective source-size cap in bytes (`0` => the 256 KB default).
    #[inline]
    pub fn effective_max_source_bytes(&self) -> usize {
        if self.max_source_bytes == 0 {
            crate::DEFAULT_MAX_SOURCE_BYTES
        } else {
            self.max_source_bytes as usize
        }
    }

    /// Effective node cap (`0` => the 5000 default).
    #[inline]
    pub fn effective_max_nodes(&self) -> usize {
        if self.max_nodes == 0 {
            crate::DEFAULT_MAX_NODES
        } else {
            self.max_nodes as usize
        }
    }
}

/// A defaulted options value used when the caller passes a null `opts_ptr`.
/// All-zero except the version/size guard fields, so every "`0` => default"
/// rule applies and no color override is active.
impl Default for MmdrOptions {
    fn default() -> Self {
        Self {
            abi_version: MMDR_ABI_VERSION,
            struct_size: std::mem::size_of::<MmdrOptions>() as u32,
            base_theme: 0,
            color_override_mask: 0,
            background_rgba: 0,
            foreground_rgba: 0,
            accent_rgba: 0,
            node_spacing: 0.0,
            rank_spacing: 0.0,
            preferred_aspect_w: 0.0,
            preferred_aspect_h: 0.0,
            max_source_bytes: 0,
            max_nodes: 0,
            timeout_ms: 0,
        }
    }
}

/// cbindgen marker so the generated header is anchored to a known type; not
/// used by Rust callers. (`c_void` keeps the import meaningful across configs.)
#[allow(dead_code)]
pub(crate) type Opaque = c_void;

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, size_of};

    #[test]
    fn mmdr_result_layout_is_40_bytes() {
        assert_eq!(size_of::<MmdrResult>(), 40);
        assert_eq!(align_of::<MmdrResult>(), 8);
    }

    #[test]
    fn mmdr_options_layout_is_56_bytes() {
        assert_eq!(size_of::<MmdrOptions>(), 56);
        assert_eq!(align_of::<MmdrOptions>(), 4);
    }

    #[test]
    fn mmdr_result_field_offsets_match_byte_table() {
        // PRD §4.3 MmdrResult v1 byte-offset table.
        let base = std::ptr::null::<MmdrResult>() as usize;
        let probe = std::mem::MaybeUninit::<MmdrResult>::uninit();
        let p = probe.as_ptr();
        let off = |addr: usize| addr - p as usize;
        unsafe {
            assert_eq!(off(std::ptr::addr_of!((*p).abi_version) as usize), 0);
            assert_eq!(off(std::ptr::addr_of!((*p).struct_size) as usize), 4);
            assert_eq!(off(std::ptr::addr_of!((*p).status) as usize), 8);
            assert_eq!(off(std::ptr::addr_of!((*p).err_line) as usize), 12);
            assert_eq!(off(std::ptr::addr_of!((*p).err_col) as usize), 16);
            assert_eq!(off(std::ptr::addr_of!((*p).loc_valid) as usize), 20);
            assert_eq!(off(std::ptr::addr_of!((*p).data_len) as usize), 24);
            assert_eq!(off(std::ptr::addr_of!((*p).data_ptr) as usize), 32);
        }
        let _ = base;
    }

    #[test]
    fn mmdr_options_field_offsets_match_byte_table() {
        // PRD §4.3 MmdrOptions v1 byte-offset table.
        let probe = std::mem::MaybeUninit::<MmdrOptions>::uninit();
        let p = probe.as_ptr();
        let off = |addr: usize| addr - p as usize;
        unsafe {
            assert_eq!(off(std::ptr::addr_of!((*p).abi_version) as usize), 0);
            assert_eq!(off(std::ptr::addr_of!((*p).struct_size) as usize), 4);
            assert_eq!(off(std::ptr::addr_of!((*p).base_theme) as usize), 8);
            assert_eq!(
                off(std::ptr::addr_of!((*p).color_override_mask) as usize),
                12
            );
            assert_eq!(off(std::ptr::addr_of!((*p).background_rgba) as usize), 16);
            assert_eq!(off(std::ptr::addr_of!((*p).foreground_rgba) as usize), 20);
            assert_eq!(off(std::ptr::addr_of!((*p).accent_rgba) as usize), 24);
            assert_eq!(off(std::ptr::addr_of!((*p).node_spacing) as usize), 28);
            assert_eq!(off(std::ptr::addr_of!((*p).rank_spacing) as usize), 32);
            assert_eq!(
                off(std::ptr::addr_of!((*p).preferred_aspect_w) as usize),
                36
            );
            assert_eq!(
                off(std::ptr::addr_of!((*p).preferred_aspect_h) as usize),
                40
            );
            assert_eq!(off(std::ptr::addr_of!((*p).max_source_bytes) as usize), 44);
            assert_eq!(off(std::ptr::addr_of!((*p).max_nodes) as usize), 48);
            assert_eq!(off(std::ptr::addr_of!((*p).timeout_ms) as usize), 52);
        }
    }
}
