//! wasm-safe monotonic timer shim.
//!
//! `std::time::Instant::now()` panics on `wasm32-unknown-unknown`
//! ("time not implemented on this platform"). The layout profiler uses
//! `Instant` only to populate discarded `stage_metrics.*_us` counters, so on
//! wasm we substitute a zero-cost stub that reports a zero elapsed duration.
//! Native builds re-export the real `std::time::Instant` unchanged.

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use std::time::Instant;

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
pub(crate) struct Instant;

#[cfg(target_arch = "wasm32")]
impl Instant {
    #[inline]
    pub(crate) fn now() -> Self {
        Instant
    }
    #[inline]
    pub(crate) fn elapsed(&self) -> std::time::Duration {
        std::time::Duration::ZERO
    }
}
