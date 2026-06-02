//! `MmdrStatus` — the stable, additive C status enum (PRD §4.3).
//!
//! Discriminants mirror the fork's `ParseError` shapes (1–4) plus the
//! FFI-introduced sentinels: the diagram-type allowlist gate (`5`), the two
//! FFI-only programmer-error guards (`6`/`7`), the forward-compat version guard
//! (`8`), the DoS bound (`9`), a RESERVED timeout slot (`10`, never emitted in
//! MVP), and the `catch_unwind` panic sentinel (`99`). The set is append-only:
//! existing values never change, so a cached Swift header stays valid against a
//! newer binary.

/// C status code returned in [`crate::types::MmdrResult::status`] (as `i32`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MmdrStatus {
    /// Render succeeded; `data_ptr`/`data_len` carry the payload.
    Ok = 0,
    /// `ParseError::UnexpectedToken` — also the generic parser-error fallback.
    ErrUnexpectedToken = 1,
    /// `ParseError::UnknownParticipant`.
    ErrUnknownParticipant = 2,
    /// `ParseError::UnclosedSubgraph`.
    ErrUnclosedSubgraph = 3,
    /// `ParseError::InvalidDirective`.
    ErrInvalidDirective = 4,
    /// Diagram type not in the FFI-wrapper MVP allowlist (PRIMARY gate, Phase
    /// 3). Set BEFORE the renderer runs — the lenient parser is never invoked.
    ErrUnsupported = 5,
    /// FFI-only guard: input bytes were not valid UTF-8. Never produced by the
    /// renderer.
    ErrInvalidUtf8 = 6,
    /// FFI-only guard: a null pointer was passed where a non-null one is
    /// required. Never produced by the renderer.
    ErrNullArg = 7,
    /// `opts.struct_size` / `opts.abi_version` disagree with the binary
    /// (forward-compat guard).
    ErrVersionMismatch = 8,
    /// A DoS bound tripped (source-size or node cap).
    ErrTooLarge = 9,
    /// RESERVED for a future pure-Rust internal deadline (§13-D10). NOT emitted
    /// by the wrapper in MVP; the MVP timeout is Swift-side perceived-latency
    /// only. Kept so wiring it later needs no ABI bump.
    ErrTimeout = 10,
    /// `catch_unwind` tripped — an internal renderer bug, NOT user input. The
    /// payload is redacted to a fixed sentinel (no internal paths leak).
    ErrPanic = 99,
}

impl MmdrStatus {
    /// The status as the `i32` stored in `MmdrResult.status`.
    #[inline]
    pub const fn as_i32(self) -> i32 {
        self as i32
    }
}
