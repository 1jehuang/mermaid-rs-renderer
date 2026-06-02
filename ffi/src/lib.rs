//! C-ABI FFI wrapper around `mermaid-rs-renderer`.
//!
//! This crate is built as a `staticlib` and linked into `MermaidFFI.xcframework`
//! (Phases 4/5). It keeps the fork submodule pristine for independent
//! per-PR upstreaming (PRD §4.5 / D3): all FFI-specific glue lives here, not in
//! the fork.
//!
//! # ABI surface (PRD §4.3)
//! - [`types::MmdrResult`] / [`types::MmdrOptions`] — transparent, versioned
//!   `#[repr(C)]` structs (40 / 56 bytes) whose byte layout is a documented ABI
//!   commitment, pinned by compile-time size asserts and the cbindgen
//!   header-drift gate.
//! - [`status::MmdrStatus`] — the stable, additive status enum (0–10, 99).
//! - Render entry points `mmdr_render_svg` / `mmdr_render_png`, the deallocator
//!   `mmdr_result_free`, the field accessors, `mmdr_version`, and the
//!   `mmdr_init` latency-hint no-op (see [`abi`]).
//! - [`diagram_type`]: wrapper-side keyword pre-dispatch — the PRIMARY status-5
//!   `ErrUnsupported` gate for non-MVP diagram types.
//!
//! # Safety model
//! Every render call applies five pre-dispatch guards (null-arg, version/size,
//! source-size cap, UTF-8, diagram-type allowlist) BEFORE the renderer runs,
//! then wraps the ENTIRE parse→node-cap→layout→render→[rasterize] pipeline in
//! `catch_unwind` so any panic maps to status 99 with a redacted payload. The
//! crate's release profile keeps `panic = "unwind"` (see `Cargo.toml`) — a
//! `panic = "abort"` build would silently defeat `catch_unwind`.

pub mod abi;
pub mod color;
pub mod diagram_type;
pub mod raster;
pub mod render;
pub mod status;
pub mod types;

/// Default DoS source-size cap: 256 KB (PRD §4.3 DoS item 1). Used when
/// `MmdrOptions.max_source_bytes == 0`.
pub const DEFAULT_MAX_SOURCE_BYTES: usize = 262_144;

/// Default DoS node cap: 5000 nodes (PRD §4.3 DoS item 2) — the TRUE compute
/// bound (enforced pre-layout). Used when `MmdrOptions.max_nodes == 0`.
pub const DEFAULT_MAX_NODES: usize = 5_000;

// Re-export the ABI surface at the crate root for ergonomic Rust-side tests and
// for cbindgen's symbol discovery.
pub use status::MmdrStatus;
pub use types::{MmdrOptions, MmdrResult, MMDR_ABI_VERSION};
