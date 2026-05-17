/// WASM-compatible time shim.
///
/// On native targets this is a transparent re-export of `std::time::Instant`.
/// On `wasm32-unknown-unknown` (which has no system clock) it is a zero-cost
/// dummy that always reports zero elapsed time.  This lets the rest of the
/// codebase use `Instant::now()` / `.elapsed()` without any `#[cfg]` guards.

#[cfg(not(target_arch = "wasm32"))]
pub use std::time::Instant;

#[cfg(target_arch = "wasm32")]
pub struct Instant;

#[cfg(target_arch = "wasm32")]
impl Instant {
    #[inline(always)]
    pub fn now() -> Self { Instant }
    #[inline(always)]
    pub fn elapsed(&self) -> std::time::Duration { std::time::Duration::ZERO }
}
