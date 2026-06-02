//! The `extern "C"` ABI surface (PRD §4.3 function surface + ownership + panic
//! safety). Every entry point is `#[no_mangle] pub extern "C"`; cbindgen emits
//! the matching C declarations into `mmdr.h`.
//!
//! Guard order for a render call (PRD §4.3): null-arg → version/size → source
//! cap → UTF-8 → diagram-type allowlist → `catch_unwind`(parse → node cap →
//! layout → render → [rasterize]). The first five guards run BEFORE the
//! renderer is ever invoked; the sixth wraps the ENTIRE pipeline so any
//! layout/render/raster panic maps to status 99.

use std::ffi::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::{ptr, slice};

use crate::diagram_type::{detect_diagram_type, is_mvp_supported};
use crate::render::{self, map_parse_error, Format, Outcome};
use crate::status::MmdrStatus;
use crate::types::{MmdrOptions, MmdrResult, MMDR_ABI_VERSION};

/// Redacted payload for the internal-error / panic path — no internal paths or
/// source fragments ever cross the boundary (security NF-01).
const INTERNAL_ERROR: &[u8] = b"internal error";

/// Allocate an owned `MmdrResult` on the heap (PRD Arbitration E ownership
/// contract). The payload is published as a `Box<[u8]>` (`len == capacity`, no
/// capacity tracking); an EMPTY payload is published as a null `data_ptr` with
/// `data_len == 0` (the documented null-buffer case that `mmdr_result_free`
/// guards). Returns a pointer Swift owns and MUST free via `mmdr_result_free`.
fn alloc_result(
    status: MmdrStatus,
    err_line: u32,
    err_col: u32,
    loc_valid: u32,
    payload: Vec<u8>,
) -> *mut MmdrResult {
    let (data_ptr, data_len) = if payload.is_empty() {
        (ptr::null_mut::<u8>(), 0u64)
    } else {
        let boxed: Box<[u8]> = payload.into_boxed_slice();
        let len = boxed.len() as u64;
        // Fat `*mut [u8]` → thin `*mut u8` keeps the data pointer; the length
        // is recorded in `data_len` and reapplied on free.
        let ptr = Box::into_raw(boxed) as *mut u8;
        (ptr, len)
    };

    let res = Box::new(MmdrResult {
        abi_version: MMDR_ABI_VERSION,
        struct_size: std::mem::size_of::<MmdrResult>() as u32,
        status: status.as_i32(),
        err_line,
        err_col,
        loc_valid,
        data_len,
        data_ptr,
    });
    Box::into_raw(res)
}

/// Decode + validate the options pointer. `None` return means a version/size
/// mismatch (the caller has already been handed an `ErrVersionMismatch`
/// result). A null `opts_ptr` yields defaults.
///
/// SAFETY: reads only the two leading guard fields (`abi_version`,
/// `struct_size`) — identical offset/order on every ABI version — BEFORE
/// trusting `struct_size` to read the full struct, so an under-sized caller
/// struct can never cause an out-of-bounds read.
unsafe fn decode_options(opts_ptr: *const MmdrOptions) -> Result<MmdrOptions, *mut MmdrResult> {
    if opts_ptr.is_null() {
        return Ok(MmdrOptions::default());
    }
    let guard = opts_ptr as *const u32;
    let abi_version = ptr::read(guard); // offset 0
    let struct_size = ptr::read(guard.add(1)); // offset 4
    if abi_version > MMDR_ABI_VERSION || struct_size as usize != std::mem::size_of::<MmdrOptions>()
    {
        return Err(alloc_result(
            MmdrStatus::ErrVersionMismatch,
            0,
            0,
            0,
            b"options version/size mismatch".to_vec(),
        ));
    }
    Ok(ptr::read(opts_ptr))
}

/// Shared body for both render entry points.
///
/// SAFETY: `src_ptr` must be null or valid for `src_len` bytes; `opts_ptr` must
/// be null or point to a valid `MmdrOptions`. The returned pointer is owned by
/// the caller and freed with `mmdr_result_free`.
unsafe fn render_common(
    src_ptr: *const u8,
    src_len: usize,
    opts_ptr: *const MmdrOptions,
    format: Format,
) -> *mut MmdrResult {
    // 1. Null-arg guard (FFI-only programmer error).
    if src_ptr.is_null() {
        return alloc_result(
            MmdrStatus::ErrNullArg,
            0,
            0,
            0,
            b"null source pointer".to_vec(),
        );
    }

    // 2. Version/size guard (also yields defaults for a null opts_ptr).
    let opts = match decode_options(opts_ptr) {
        Ok(o) => o,
        Err(res) => return res,
    };

    // 3. Source-size DoS cap (defense in depth — the Swift adapter also caps).
    if src_len > opts.effective_max_source_bytes() {
        return alloc_result(
            MmdrStatus::ErrTooLarge,
            0,
            0,
            0,
            b"diagram source too large".to_vec(),
        );
    }

    // 4. UTF-8 guard (FFI-only programmer error).
    let bytes = slice::from_raw_parts(src_ptr, src_len);
    let src = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => {
            return alloc_result(
                MmdrStatus::ErrInvalidUtf8,
                0,
                0,
                0,
                b"source is not valid UTF-8".to_vec(),
            )
        }
    };

    // 5. Diagram-type allowlist — the PRIMARY status-5 gate. The renderer is
    //    NEVER invoked for a non-MVP type.
    let dtype = detect_diagram_type(src);
    if !is_mvp_supported(&dtype) {
        return alloc_result(
            MmdrStatus::ErrUnsupported,
            0,
            0,
            0,
            b"diagram type not yet supported".to_vec(),
        );
    }

    // 6. The whole pipeline under catch_unwind (a layout/render/raster panic →
    //    status 99 with a redacted payload).
    let outcome = catch_unwind(AssertUnwindSafe(|| render::run(src, &opts, format)));
    finish_outcome(outcome)
}

/// Map a caught pipeline outcome onto an owned `MmdrResult`. An `Err` (a caught
/// panic) and an `Outcome::Internal` both yield status 99 with the redacted
/// `"internal error"` payload — no internal paths or source fragments leak.
fn finish_outcome(outcome: std::thread::Result<Outcome>) -> *mut MmdrResult {
    match outcome {
        Ok(Outcome::Ok(bytes)) => alloc_result(MmdrStatus::Ok, 0, 0, 0, bytes),
        Ok(Outcome::Parse(e)) => {
            let m = map_parse_error(&e);
            alloc_result(
                m.status,
                m.err_line,
                m.err_col,
                m.loc_valid,
                m.message.into_bytes(),
            )
        }
        Ok(Outcome::TooLarge) => alloc_result(
            MmdrStatus::ErrTooLarge,
            0,
            0,
            0,
            b"diagram too large".to_vec(),
        ),
        Ok(Outcome::Internal) | Err(_) => {
            // Internal failure or caught panic — redacted, no detail leaks.
            alloc_result(MmdrStatus::ErrPanic, 0, 0, 0, INTERNAL_ERROR.to_vec())
        }
    }
}

/// Render Mermaid source to **vector SVG** (`data_ptr` = UTF-8 SVG bytes).
/// `opts_ptr` may be null (defaults). Returns an owned handle freed with
/// `mmdr_result_free`; returns null only on allocation failure.
///
/// # Safety
/// `src_ptr` must be null or valid for `src_len` bytes; `opts_ptr` must be null
/// or point to a valid [`MmdrOptions`].
#[no_mangle]
pub unsafe extern "C" fn mmdr_render_svg(
    src_ptr: *const u8,
    src_len: usize,
    opts_ptr: *const MmdrOptions,
) -> *mut MmdrResult {
    render_common(src_ptr, src_len, opts_ptr, Format::Svg)
}

/// Render Mermaid source to **raster PNG** (`data_ptr` = binary PNG bytes —
/// wrap in `Data`, NEVER decode as UTF-8). Same ownership / status / panic
/// contract as `mmdr_render_svg`. The rasterization runs with an egress-locked,
/// zero-filesystem `usvg::Options` (PRD §4.7b) and is INSIDE the `catch_unwind`.
///
/// # Safety
/// `src_ptr` must be null or valid for `src_len` bytes; `opts_ptr` must be null
/// or point to a valid [`MmdrOptions`].
#[no_mangle]
pub unsafe extern "C" fn mmdr_render_png(
    src_ptr: *const u8,
    src_len: usize,
    opts_ptr: *const MmdrOptions,
) -> *mut MmdrResult {
    render_common(src_ptr, src_len, opts_ptr, Format::Png)
}

/// Free a result. No-op on null. The **only** correct deallocator (PRD
/// Arbitration E). Reconstitutes the `Box<[u8]>` payload (null-guarded) and the
/// `Box<MmdrResult>`, dropping both.
///
/// # Safety
/// `res` must be null or a pointer returned by `mmdr_render_svg`/`_png` that has
/// not already been freed.
#[no_mangle]
pub unsafe extern "C" fn mmdr_result_free(res: *mut MmdrResult) {
    if res.is_null() {
        return;
    }
    let boxed = Box::from_raw(res);
    if !boxed.data_ptr.is_null() {
        // Reapply the recorded length to rebuild the original Box<[u8]>.
        drop(Box::from_raw(ptr::slice_from_raw_parts_mut(
            boxed.data_ptr,
            boxed.data_len as usize,
        )));
    } else {
        debug_assert_eq!(boxed.data_len, 0);
    }
    // `boxed` (the MmdrResult allocation) drops at end of scope.
}

/// Payload pointer accessor (fields are also directly readable — transparent
/// ABI). Null on a null `res`.
///
/// # Safety
/// `res` must be null or a valid `MmdrResult` pointer.
#[no_mangle]
pub unsafe extern "C" fn mmdr_result_data(res: *const MmdrResult) -> *const u8 {
    if res.is_null() {
        return ptr::null();
    }
    (*res).data_ptr
}

/// Payload length accessor (bytes). `0` on a null `res`.
///
/// # Safety
/// `res` must be null or a valid `MmdrResult` pointer.
#[no_mangle]
pub unsafe extern "C" fn mmdr_result_len(res: *const MmdrResult) -> u64 {
    if res.is_null() {
        return 0;
    }
    (*res).data_len
}

/// Status accessor. `ErrNullArg` (7) on a null `res`.
///
/// # Safety
/// `res` must be null or a valid `MmdrResult` pointer.
#[no_mangle]
pub unsafe extern "C" fn mmdr_result_status(res: *const MmdrResult) -> i32 {
    if res.is_null() {
        return MmdrStatus::ErrNullArg.as_i32();
    }
    (*res).status
}

/// `err_line` accessor. `0` on a null `res`.
///
/// # Safety
/// `res` must be null or a valid `MmdrResult` pointer.
#[no_mangle]
pub unsafe extern "C" fn mmdr_result_err_line(res: *const MmdrResult) -> u32 {
    if res.is_null() {
        return 0;
    }
    (*res).err_line
}

/// `err_col` accessor. `0` on a null `res`.
///
/// # Safety
/// `res` must be null or a valid `MmdrResult` pointer.
#[no_mangle]
pub unsafe extern "C" fn mmdr_result_err_col(res: *const MmdrResult) -> u32 {
    if res.is_null() {
        return 0;
    }
    (*res).err_col
}

/// `loc_valid` accessor. `0` on a null `res`.
///
/// # Safety
/// `res` must be null or a valid `MmdrResult` pointer.
#[no_mangle]
pub unsafe extern "C" fn mmdr_result_loc_valid(res: *const MmdrResult) -> u32 {
    if res.is_null() {
        return 0;
    }
    (*res).loc_valid
}

/// A `'static`, NUL-terminated version string. Never freed.
#[no_mangle]
pub extern "C" fn mmdr_version() -> *const c_char {
    c"mermaid-ffi 0.1.0 (abi 2)".as_ptr()
}

/// Test-only: deliberately panic inside the SAME `catch_unwind` + `finish`
/// path the render entry points use, so a linked harness can prove the SHIPPED
/// staticlib (release + LTO + strip, `panic = "unwind"`) maps a renderer panic
/// to status 99 with a redacted payload and WITHOUT aborting the process — the
/// PRD §4.3 N5 on-target guarantee. Gated behind the non-default `panic_probe`
/// feature so it never enters a shipped build or the cbindgen header. (No input
/// panics the hardened renderer, so this is the deterministic substitute for
/// "feed a deliberately-panicking input".)
#[cfg(feature = "panic_probe")]
#[no_mangle]
pub extern "C" fn mmdr_panic_probe() -> *mut MmdrResult {
    let outcome = catch_unwind(AssertUnwindSafe(|| -> Outcome {
        panic!("panic_probe: deliberate panic for the N5 on-target guarantee");
    }));
    finish_outcome(outcome)
}

/// No-op latency hint (PRD Arbitration A). The embedded font is seeded into its
/// `OnceLock` database at construction with ZERO filesystem I/O, so the first
/// render is correct with no prior init. This call merely forces the font
/// `OnceLock`s to evaluate early (off the render hot path). Idempotent,
/// thread-safe, and panic-isolated; guarantees nothing about correctness.
#[no_mangle]
pub extern "C" fn mmdr_init() {
    let _ = catch_unwind(crate::raster::warm);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MMDR_ABI_VERSION;

    /// Render via the C ABI and return the owned result pointer.
    unsafe fn render_svg(src: &str, opts: Option<&MmdrOptions>) -> *mut MmdrResult {
        let optr = opts.map_or(ptr::null(), |o| o as *const MmdrOptions);
        mmdr_render_svg(src.as_ptr(), src.len(), optr)
    }

    unsafe fn status_of(res: *const MmdrResult) -> i32 {
        mmdr_result_status(res)
    }

    unsafe fn payload(res: *const MmdrResult) -> Vec<u8> {
        let p = mmdr_result_data(res);
        let n = mmdr_result_len(res) as usize;
        if p.is_null() {
            Vec::new()
        } else {
            slice::from_raw_parts(p, n).to_vec()
        }
    }

    #[test]
    fn valid_render_is_status_0_with_svg() {
        unsafe {
            let res = render_svg("flowchart LR\nA-->B", None);
            assert_eq!(status_of(res), 0);
            let body = String::from_utf8(payload(res)).unwrap();
            assert!(body.contains("<svg"));
            mmdr_result_free(res);
        }
    }

    #[test]
    fn null_source_is_status_7() {
        unsafe {
            let res = mmdr_render_svg(ptr::null(), 0, ptr::null());
            assert_eq!(status_of(res), MmdrStatus::ErrNullArg.as_i32());
            mmdr_result_free(res);
        }
    }

    #[test]
    fn invalid_utf8_is_status_6() {
        unsafe {
            let bad = [0xFFu8, 0xFE, 0x00, 0x9F];
            let res = mmdr_render_svg(bad.as_ptr(), bad.len(), ptr::null());
            assert_eq!(status_of(res), MmdrStatus::ErrInvalidUtf8.as_i32());
            mmdr_result_free(res);
        }
    }

    #[test]
    fn non_mvp_type_is_status_5() {
        unsafe {
            let res = render_svg("mindmap\nroot", None);
            assert_eq!(status_of(res), MmdrStatus::ErrUnsupported.as_i32());
            mmdr_result_free(res);
        }
    }

    #[test]
    fn unknown_keyword_is_status_5() {
        unsafe {
            let res = render_svg("sankey-beta\n", None);
            assert_eq!(status_of(res), MmdrStatus::ErrUnsupported.as_i32());
            mmdr_result_free(res);
        }
    }

    #[test]
    fn oversize_source_is_status_9() {
        unsafe {
            let o = MmdrOptions {
                max_source_bytes: 16,
                ..Default::default()
            };
            let big = "flowchart LR\nA-->B\nB-->C\nC-->D\nD-->E";
            let res = render_svg(big, Some(&o));
            assert_eq!(status_of(res), MmdrStatus::ErrTooLarge.as_i32());
            mmdr_result_free(res);
        }
    }

    #[test]
    fn node_cap_is_status_9() {
        unsafe {
            let o = MmdrOptions {
                max_nodes: 1,
                ..Default::default()
            };
            let res = render_svg("flowchart LR\nA-->B\nB-->C", Some(&o));
            assert_eq!(status_of(res), MmdrStatus::ErrTooLarge.as_i32());
            mmdr_result_free(res);
        }
    }

    #[test]
    fn version_mismatch_is_status_8() {
        unsafe {
            // Caller claims a newer ABI than the binary.
            let o = MmdrOptions {
                abi_version: MMDR_ABI_VERSION + 1,
                ..Default::default()
            };
            let res = render_svg("flowchart LR\nA-->B", Some(&o));
            assert_eq!(status_of(res), MmdrStatus::ErrVersionMismatch.as_i32());
            mmdr_result_free(res);
        }
    }

    #[test]
    fn wrong_struct_size_is_status_8() {
        unsafe {
            let o = MmdrOptions {
                struct_size: 40, // not 56
                ..Default::default()
            };
            let res = render_svg("flowchart LR\nA-->B", Some(&o));
            assert_eq!(status_of(res), MmdrStatus::ErrVersionMismatch.as_i32());
            mmdr_result_free(res);
        }
    }

    #[test]
    fn older_caller_abi_still_works() {
        unsafe {
            // v7-N1: caller_version < binary AND struct_size matches → OK.
            let o = MmdrOptions {
                abi_version: 1, // older than current (2)
                ..Default::default()
            };
            let res = render_svg("flowchart LR\nA-->B", Some(&o));
            assert_eq!(status_of(res), 0);
            mmdr_result_free(res);
        }
    }

    #[test]
    fn parse_error_populates_location() {
        unsafe {
            let res = render_svg("flowchart LR\n--> B", None);
            let s = status_of(res);
            assert_eq!(s, MmdrStatus::ErrUnexpectedToken.as_i32());
            assert_eq!(mmdr_result_loc_valid(res), 1);
            assert!(mmdr_result_err_line(res) >= 1);
            mmdr_result_free(res);
        }
    }

    #[test]
    fn png_render_is_status_0_with_png_magic() {
        unsafe {
            let res = mmdr_render_png("flowchart LR\nA-->B".as_ptr(), 18, ptr::null());
            assert_eq!(status_of(res), 0);
            let body = payload(res);
            assert_eq!(
                &body[..8],
                &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
            );
            mmdr_result_free(res);
        }
    }

    #[test]
    fn free_is_noop_on_null() {
        unsafe {
            mmdr_result_free(ptr::null_mut());
        }
    }

    #[test]
    fn accessors_are_safe_on_null() {
        unsafe {
            assert!(mmdr_result_data(ptr::null()).is_null());
            assert_eq!(mmdr_result_len(ptr::null()), 0);
            assert_eq!(
                mmdr_result_status(ptr::null()),
                MmdrStatus::ErrNullArg.as_i32()
            );
            assert_eq!(mmdr_result_loc_valid(ptr::null()), 0);
        }
    }

    #[test]
    fn version_is_nul_terminated_cstr() {
        unsafe {
            let p = mmdr_version();
            let s = std::ffi::CStr::from_ptr(p).to_str().unwrap();
            assert!(s.contains("mermaid-ffi"));
        }
    }

    #[test]
    fn init_is_idempotent_and_safe() {
        mmdr_init();
        mmdr_init();
    }

    #[test]
    fn result_carries_abi_version_and_struct_size() {
        unsafe {
            let res = render_svg("flowchart LR\nA-->B", None);
            assert_eq!((*res).abi_version, MMDR_ABI_VERSION);
            assert_eq!((*res).struct_size, std::mem::size_of::<MmdrResult>() as u32);
            mmdr_result_free(res);
        }
    }

    // --- Panic safety (PRD §4.3) ------------------------------------------

    #[test]
    fn catch_unwind_works_proving_panic_unwind_not_abort() {
        // If this crate were compiled `panic = "abort"`, `catch_unwind` would
        // be a no-op and the process would abort instead of returning Err.
        let caught = catch_unwind(AssertUnwindSafe(|| panic!("intentional")));
        assert!(caught.is_err());
    }

    #[test]
    fn caught_panic_maps_to_status_99_redacted() {
        // A pipeline panic must surface as status 99 with the fixed redacted
        // payload — never an internal path or source fragment.
        let outcome = catch_unwind(AssertUnwindSafe(|| -> Outcome {
            panic!("src/render.rs:412 secret user fragment");
        }));
        let res = finish_outcome(outcome);
        unsafe {
            assert_eq!(status_of(res), MmdrStatus::ErrPanic.as_i32());
            assert_eq!(payload(res), INTERNAL_ERROR.to_vec());
            mmdr_result_free(res);
        }
    }

    #[test]
    fn internal_outcome_maps_to_status_99_redacted() {
        let res = finish_outcome(Ok(Outcome::Internal));
        unsafe {
            assert_eq!(status_of(res), MmdrStatus::ErrPanic.as_i32());
            assert_eq!(payload(res), INTERNAL_ERROR.to_vec());
            mmdr_result_free(res);
        }
    }

    // --- Ownership round-trip (Miri-clean — no renderer involved) ----------
    // Run under Miri with: `cargo +nightly miri test miri_`. These exercise
    // `alloc_result` + `mmdr_result_free` + accessors ONLY (the Box<[u8]>
    // round-trip and the null-data-buffer guard), which is the soundness
    // surface the PRD's mandatory Miri test targets — the resvg/usvg render
    // path is not Miri-executable and is covered by the on-target T3 build.

    #[test]
    fn miri_round_trip_nonempty_payload() {
        unsafe {
            let res = alloc_result(MmdrStatus::Ok, 0, 0, 0, b"hello world".to_vec());
            assert_eq!(status_of(res), 0);
            assert_eq!(mmdr_result_len(res), 11);
            assert_eq!(payload(res), b"hello world".to_vec());
            mmdr_result_free(res); // must not leak or UB
        }
    }

    #[test]
    fn miri_round_trip_empty_payload_null_data_branch() {
        unsafe {
            // Empty payload => null data_ptr, len 0 (the documented null branch
            // that `mmdr_result_free` must guard against — Box::from_raw on a
            // null slice would be UB without the guard).
            let res = alloc_result(MmdrStatus::Ok, 0, 0, 0, Vec::new());
            assert!(mmdr_result_data(res).is_null());
            assert_eq!(mmdr_result_len(res), 0);
            mmdr_result_free(res);
        }
    }

    #[test]
    fn miri_round_trip_error_message_payload() {
        unsafe {
            let res = alloc_result(
                MmdrStatus::ErrUnexpectedToken,
                3,
                7,
                1,
                b"unexpected token".to_vec(),
            );
            assert_eq!(status_of(res), MmdrStatus::ErrUnexpectedToken.as_i32());
            assert_eq!(mmdr_result_err_line(res), 3);
            assert_eq!(mmdr_result_err_col(res), 7);
            assert_eq!(mmdr_result_loc_valid(res), 1);
            mmdr_result_free(res);
        }
    }

    #[test]
    fn miri_double_free_is_avoided_single_free_only() {
        // A single free of a freshly-allocated result is sound; we never free
        // twice (the contract). This documents the single-owner discipline.
        unsafe {
            let res = alloc_result(MmdrStatus::Ok, 0, 0, 0, b"x".to_vec());
            mmdr_result_free(res);
        }
    }

    // --- Reentrancy (PRD §4.3 thread-safety; run with --test-threads > 1) ---

    #[test]
    fn concurrent_renders_are_reentrant() {
        use std::thread;
        let handles: Vec<_> = (0..8)
            .map(|i| {
                thread::spawn(move || unsafe {
                    let src = format!("flowchart LR\nA{i}-->B{i}");
                    let res = mmdr_render_svg(src.as_ptr(), src.len(), ptr::null());
                    let ok = mmdr_result_status(res) == 0;
                    mmdr_result_free(res);
                    ok
                })
            })
            .collect();
        for h in handles {
            assert!(h.join().unwrap());
        }
    }
}
