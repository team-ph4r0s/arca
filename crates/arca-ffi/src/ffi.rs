// SAFETY: the caller passes `len` readable bytes at `password` (or null with 0).
// SAFETY: the caller passes `len` readable bytes at `data` (or null with 0).
// SPDX-License-Identifier: AGPL-3.0-only
// Additional terms under AGPLv3 section 7: see LICENSE_ADDITIONAL_TERMS.md in the repository root.

//! The C ABI entry points: the one file of arca-ffi that contains `unsafe` code.
//!
//! Every function here does the same three things:
//! 1. unwrap and validate raw C arguments;
//! 2. call safe functions of [`crate::api`] inside `catch_unwind`;
//! 3. convert results into standard C types or [`ArcaBuffer`].
//!
//! Memory management:
//! Whenever an [`ArcaBuffer`] is populated by an `arca_*` call, its memory is allocated on
//! the Rust heap. The foreign caller (Swift, Objective-C, C) must free it by calling
//! [`arca_buffer_free`]. When freed, the buffer is securely wiped (`zeroize`) before release.

#![allow(unsafe_code, reason = "this file is the C ABI boundary")]
#![warn(clippy::undocumented_unsafe_blocks, clippy::missing_safety_doc)]

use std::fs::File;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr::null_mut;

use arca_engine::Status;
use zeroize::Zeroize;

use crate::api;

/// A contiguous byte buffer returned by Arca to C/Swift.
///
/// When `data` is non-null, the caller owns the memory and must release it
/// with [`arca_buffer_free`].
#[repr(C)]
#[derive(Debug)]
pub struct ArcaBuffer {
    /// Pointer to the byte array, or null if empty.
    pub data: *mut u8,
    /// Length of the byte array in bytes.
    pub len: usize,
}

impl ArcaBuffer {
    /// An empty, null buffer.
    #[must_use]
    pub const fn empty() -> Self {
        Self { data: null_mut(), len: 0 }
    }

    /// Creates an `ArcaBuffer` from a `Vec<u8>`.
    #[must_use]
    pub fn from_vec(v: Vec<u8>) -> Self {
        let boxed: Box<[u8]> = v.into_boxed_slice();
        let len = boxed.len();
        if len == 0 {
            Self::empty()
        } else {
            let ptr = Box::into_raw(boxed) as *mut u8;
            Self { data: ptr, len }
        }
    }
}

/// Frees a buffer previously allocated and returned by Arca.
///
/// Memory is securely wiped (`zeroize`) before being returned to the allocator.
///
/// # Safety
///
/// `buf` must be a buffer this library returned and that was not freed before (or an empty
/// one); the caller must not use its pointer afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_buffer_free(buf: ArcaBuffer) {
    if !buf.data.is_null() && buf.len > 0 {
        let slice = core::ptr::slice_from_raw_parts_mut(buf.data, buf.len);
        // SAFETY: buf.data was allocated via Box::into_raw with length buf.len (the caller
        // guarantees it came from this library and was not freed before).
        let mut boxed: Box<[u8]> = unsafe { Box::from_raw(slice) };
        boxed.zeroize();
        drop(boxed);
    }
}

/// Clears `*out` (when it can be written) and answers [`Status::BadArgument`].
fn reject(out: *mut ArcaBuffer) -> i32 {
    if !out.is_null() {
        // SAFETY: `out` is not null and, per the header, points to a writable `ArcaBuffer`.
        unsafe {
            *out = ArcaBuffer::empty();
        }
    }
    Status::BadArgument.code()
}

fn guard<T>(fallback: T, f: impl FnOnce() -> T) -> T {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or(fallback)
}

fn buffer_result(out: *mut ArcaBuffer, f: impl FnOnce() -> Result<Vec<u8>, Status>) -> i32 {
    if out.is_null() {
        return Status::BadArgument.code();
    }
    // SAFETY: out was checked for null.
    unsafe {
        *out = ArcaBuffer::empty();
    }
    guard(Status::Fault.code(), || match f() {
        Ok(v) => {
            // SAFETY: out was checked for null.
            unsafe {
                *out = ArcaBuffer::from_vec(v);
            }
            Status::Ok.code()
        }
        Err(s) => s.code(),
    })
}

fn buffer_result_direct(out: *mut ArcaBuffer, f: impl FnOnce() -> Vec<u8>) -> i32 {
    if out.is_null() {
        return Status::BadArgument.code();
    }
    // SAFETY: out was checked for null.
    unsafe {
        *out = ArcaBuffer::empty();
    }
    guard(Status::Fault.code(), || {
        let v = f();
        // SAFETY: out was checked for null.
        unsafe {
            *out = ArcaBuffer::from_vec(v);
        }
        Status::Ok.code()
    })
}

fn optional_buffer_result(out: *mut ArcaBuffer, f: impl FnOnce() -> Option<Vec<u8>>) -> i32 {
    if out.is_null() {
        return Status::BadArgument.code();
    }
    // SAFETY: out was checked for null.
    unsafe {
        *out = ArcaBuffer::empty();
    }
    guard(Status::Fault.code(), || match f() {
        Some(v) => {
            // SAFETY: out was checked for null.
            unsafe {
                *out = ArcaBuffer::from_vec(v);
            }
            Status::Ok.code()
        }
        None => Status::NotReady.code(),
    })
}

/// The bytes `data[..len]`, or `None` for a null pointer with a non-zero length or a length no
/// slice can have (more than `isize::MAX` bytes).
///
/// # Safety
///
/// When `data` is not null it must point to `len` readable bytes that stay valid and unchanged
/// for the lifetime `'a`.
unsafe fn slice_from_raw<'a>(data: *const u8, len: usize) -> Option<&'a [u8]> {
    if len == 0 {
        Some(&[])
    } else if data.is_null() || isize::try_from(len).is_err() {
        None
    } else {
        // SAFETY: caller guarantees data points to len valid bytes.
        Some(unsafe { core::slice::from_raw_parts(data, len) })
    }
}

/// Takes ownership of `fd`.
///
/// # Safety
///
/// `fd` must be negative or an open descriptor that nothing else owns or closes afterwards.
#[cfg(unix)]
unsafe fn file_from_fd(fd: i32) -> Option<File> {
    use std::os::fd::FromRawFd;
    if fd < 0 {
        return None;
    }
    // SAFETY: the caller hands over the only ownership of the open descriptor.
    Some(unsafe { File::from_raw_fd(fd) })
}

#[cfg(not(unix))]
unsafe fn file_from_fd(_fd: i32) -> Option<File> {
    None
}

// ---------------------------------------------------------------------------
// Lifecycle & Engine Configuration
// ---------------------------------------------------------------------------

/// Initialises the engine with device-appropriate limits.
///
/// Returns 0 (`Status::Ok`) or a status error code.
#[unsafe(no_mangle)]
pub extern "C" fn arca_init(
    max_input_mib: i32,
    max_memory_mib: i32,
    scan_secs: i32,
    preview_secs: i32,
) -> i32 {
    guard(Status::Fault.code(), || {
        api::init(max_input_mib, max_memory_mib, scan_secs, preview_secs)
    })
}

/// The version of the guest ABI this library was built with (`ARCA_ABI_VERSION` in arca.h): an
/// app built against another header must refuse to start the engine.
#[unsafe(no_mangle)]
pub extern "C" fn arca_abi_version() -> i32 {
    arca_engine::ABI_VERSION
}

/// Returns 1 if a WebAssembly guest was embedded at build time, 0 otherwise.
#[unsafe(no_mangle)]
pub extern "C" fn arca_has_guest() -> i32 {
    guard(0, || if api::has_guest() { 1 } else { 0 })
}

/// Returns 1 if the engine has been initialized, 0 otherwise.
#[unsafe(no_mangle)]
pub extern "C" fn arca_is_init() -> i32 {
    guard(0, || if api::is_init() { 1 } else { 0 })
}

/// Writes engine and guest metadata JSON into `out`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_info(out: *mut ArcaBuffer) -> i32 {
    optional_buffer_result(out, api::info)
}

/// Runs the built-in self-test and writes its JSON report into `out`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_self_test(out: *mut ArcaBuffer) -> i32 {
    optional_buffer_result(out, api::self_test)
}

// ---------------------------------------------------------------------------
// Document Management
// ---------------------------------------------------------------------------

/// Starts an empty document accepting at most `max_bytes`.
///
/// Returns a positive document handle, or a negative `-status` code.
#[unsafe(no_mangle)]
pub extern "C" fn arca_doc_new(max_bytes: i64) -> i64 {
    guard(-i64::from(Status::Fault.code()), || api::doc_new(max_bytes))
}

/// Appends a byte chunk to the document.
///
/// Returns 0 on success or a status error code.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_doc_append(doc: i64, data: *const u8, len: usize) -> i32 {
    guard(Status::Fault.code(), || {
        // SAFETY: the caller passes `len` readable bytes at `data` (or null with 0).
        let Some(slice) = (unsafe { slice_from_raw(data, len) }) else {
            return Status::BadArgument.code();
        };
        api::doc_append(doc, slice)
    })
}

/// Reads a POSIX file descriptor to EOF into the document and closes `fd`.
///
/// Returns the number of bytes read or a negative `-status` code.
///
/// # Safety
///
/// `fd` must be negative or an open descriptor owned by the caller, who must not use or close
/// it after the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_doc_read_fd(doc: i64, fd: i32) -> i64 {
    guard(-i64::from(Status::Fault.code()), || {
        // SAFETY: the caller hands over ownership of `fd` (see the function's contract).
        let Some(mut file) = (unsafe { file_from_fd(fd) }) else {
            return -i64::from(Status::BadArgument.code());
        };
        api::doc_read(doc, &mut file)
    })
}

/// Remembers the password (UTF-8) for subsequent runs over the document.
///
/// Passing `len = 0` clears the password.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_doc_set_password(doc: i64, password: *const u8, len: usize) -> i32 {
    guard(Status::Fault.code(), || {
        // SAFETY: the caller passes `len` readable bytes at `password` (or null with 0).
        let Some(slice) = (unsafe { slice_from_raw(password, len) }) else {
            return Status::BadArgument.code();
        };
        api::doc_set_password(doc, slice)
    })
}

/// Returns the byte length of the document so far, or a negative `-status` code.
#[unsafe(no_mangle)]
pub extern "C" fn arca_doc_len(doc: i64) -> i64 {
    guard(-i64::from(Status::Fault.code()), || api::doc_len(doc))
}

/// Releases the document handle and wipes its bytes.
#[unsafe(no_mangle)]
pub extern "C" fn arca_doc_free(doc: i64) {
    guard((), || api::doc_free(doc));
}

// ---------------------------------------------------------------------------
// Jobs & Cancellation
// ---------------------------------------------------------------------------

/// Creates a new cancellation token job.
///
/// Returns a positive job handle, or a negative `-status` code.
#[unsafe(no_mangle)]
pub extern "C" fn arca_job_new() -> i64 {
    guard(-i64::from(Status::Fault.code()), api::job_new)
}

/// Asks an ongoing run linked to `job` to cancel promptly.
///
/// Thread-safe: can be called from any thread while `arca_run` executes.
#[unsafe(no_mangle)]
pub extern "C" fn arca_job_cancel(job: i64) {
    guard((), || api::job_cancel(job));
}

/// Releases the job handle.
#[unsafe(no_mangle)]
pub extern "C" fn arca_job_free(job: i64) {
    guard((), || api::job_free(job));
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

/// Runs an operation synchronously (blocking).
///
/// `op`: 1 = Scan, 2 = Sanitize, 3 = Preview, 4 = Render, 5 = Image.
/// Returns a positive outcome handle, or a negative `-status` code.
#[unsafe(no_mangle)]
pub extern "C" fn arca_run(job: i64, doc: i64, op: i32, flags: i32) -> i64 {
    guard(-i64::from(Status::Fault.code()), || api::run(job, doc, op, flags))
}

// ---------------------------------------------------------------------------
// Outcomes & Inspection
// ---------------------------------------------------------------------------

/// Returns the status code of a finished outcome (e.g. 0 for Ok), or negative for bad handle.
#[unsafe(no_mangle)]
pub extern "C" fn arca_outcome_status(outcome: i64) -> i32 {
    guard(-Status::Fault.code(), || api::outcome_status(outcome))
}

/// Writes outcome diagnostics JSON into `out`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_outcome_info(outcome: i64, out: *mut ArcaBuffer) -> i32 {
    buffer_result(out, || api::outcome_info(outcome))
}

/// Writes the threat X-ray report JSON into `out`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_outcome_report(outcome: i64, out: *mut ArcaBuffer) -> i32 {
    buffer_result(out, || api::report(outcome))
}

/// Writes preview metadata JSON (sections, counts, notices) into `out`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_outcome_preview_meta(outcome: i64, out: *mut ArcaBuffer) -> i32 {
    buffer_result(out, || api::preview_meta(outcome))
}

/// Writes a page of preview items JSON into `out`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_outcome_preview_items(
    outcome: i64,
    section: i32,
    from: i64,
    count: i32,
    out: *mut ArcaBuffer,
) -> i32 {
    buffer_result(out, || api::preview_items(outcome, section, from, count))
}

/// Writes rendered page primitives JSON into `out`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_outcome_page(outcome: i64, out: *mut ArcaBuffer) -> i32 {
    buffer_result(out, || api::page(outcome))
}

/// Writes picture metadata facts JSON into `out`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_outcome_image_info(outcome: i64, out: *mut ArcaBuffer) -> i32 {
    buffer_result(out, || api::image_info(outcome))
}

/// Writes premultiplied RGBA pixel bytes (4 bytes per pixel) into `out`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_outcome_image_pixels(outcome: i64, out: *mut ArcaBuffer) -> i32 {
    buffer_result(out, || api::image_pixels(outcome))
}

/// Writes description JSON of a sterile copy into `out`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_outcome_sanitized_info(outcome: i64, out: *mut ArcaBuffer) -> i32 {
    buffer_result(out, || api::sanitized_info(outcome))
}

/// Returns the byte length of the sterile copy, or a negative `-status` code.
#[unsafe(no_mangle)]
pub extern "C" fn arca_outcome_sanitized_len(outcome: i64) -> i64 {
    guard(-i64::from(Status::Fault.code()), || match api::sanitized_len(outcome) {
        Ok(n) => i64::try_from(n).unwrap_or(-i64::from(Status::Fault.code())),
        Err(s) => -i64::from(s.code()),
    })
}

/// Writes the full byte contents of the sterile copy into `out`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_outcome_sanitized_bytes(outcome: i64, out: *mut ArcaBuffer) -> i32 {
    buffer_result(out, || api::sanitized_bytes(outcome))
}

/// Writes the sterile copy to a POSIX file descriptor and closes `fd`.
///
/// Returns byte count written or a negative `-status` code.
///
/// # Safety
///
/// `fd` must be negative or an open descriptor owned by the caller, who must not use or close
/// it after the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_outcome_sanitized_write_fd(outcome: i64, fd: i32) -> i64 {
    guard(-i64::from(Status::Fault.code()), || {
        // SAFETY: the caller hands over ownership of `fd` (see the function's contract).
        let Some(mut file) = (unsafe { file_from_fd(fd) }) else {
            return -i64::from(Status::BadArgument.code());
        };
        api::sanitized_write(outcome, &mut file)
    })
}

/// Releases the outcome handle.
#[unsafe(no_mangle)]
pub extern "C" fn arca_outcome_free(outcome: i64) {
    guard((), || api::outcome_free(outcome));
}

// ---------------------------------------------------------------------------
// Attribution, Passport & About
// ---------------------------------------------------------------------------

/// Writes the "About & Support" screen JSON into `out` (`lang`: 0 = Ru, 1 = En).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_about(lang: i32, out: *mut ArcaBuffer) -> i32 {
    buffer_result_direct(out, || api::about(lang))
}

/// Writes the Patron Passport JSON into `out` for a given 36-byte record.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_passport(
    lang: i32,
    record: *const u8,
    record_len: usize,
    today: i32,
    out: *mut ArcaBuffer,
) -> i32 {
    // SAFETY: the caller passes `record_len` readable bytes at `record` (or null with 0), see the header.
    let Some(rec) = (unsafe { slice_from_raw(record, record_len) }) else {
        return reject(out);
    };
    buffer_result_direct(out, || api::passport(lang, rec, today))
}

/// Updates the 36-byte patron record after one document and writes it into `out`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_passport_update(
    record: *const u8,
    record_len: usize,
    today: i32,
    trackers: i32,
    active: i32,
    out: *mut ArcaBuffer,
) -> i32 {
    // SAFETY: the caller passes `record_len` readable bytes at `record` (or null with 0), see the header.
    let Some(rec) = (unsafe { slice_from_raw(record, record_len) }) else {
        return reject(out);
    };
    buffer_result_direct(out, || api::passport_update(rec, today, trackers, active))
}

/// Local statistics: the blob after one scan (verdict = wire tag 0..4), written into `out`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_stats_record_scan(
    blob: *const u8,
    blob_len: usize,
    verdict: i32,
    trackers: i32,
    active: i32,
    out: *mut ArcaBuffer,
) -> i32 {
    // SAFETY: the caller passes `blob_len` readable bytes at `blob` (or null with 0), see the header.
    let Some(b) = (unsafe { slice_from_raw(blob, blob_len) }) else {
        return reject(out);
    };
    buffer_result_direct(out, || api::stats_record_scan(b, verdict, trackers, active))
}

/// Local statistics: the blob after a sterile copy (1) or an opened picture (2).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_stats_record_event(
    blob: *const u8,
    blob_len: usize,
    event: i32,
    out: *mut ArcaBuffer,
) -> i32 {
    // SAFETY: the caller passes `blob_len` readable bytes at `blob` (or null with 0), see the header.
    let Some(b) = (unsafe { slice_from_raw(blob, blob_len) }) else {
        return reject(out);
    };
    buffer_result_direct(out, || api::stats_record_event(b, event))
}

/// Local statistics: the flat all-time totals as JSON.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_stats_summary(
    blob: *const u8,
    blob_len: usize,
    out: *mut ArcaBuffer,
) -> i32 {
    // SAFETY: the caller passes `blob_len` readable bytes at `blob` (or null with 0), see the header.
    let Some(b) = (unsafe { slice_from_raw(blob, blob_len) }) else {
        return reject(out);
    };
    buffer_result_direct(out, || api::stats_summary(b))
}

/// Local statistics: the Patron Passport JSON built from the statistics blob.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_stats_passport(
    lang: i32,
    blob: *const u8,
    blob_len: usize,
    first_use_day: i32,
    today: i32,
    out: *mut ArcaBuffer,
) -> i32 {
    // SAFETY: the caller passes `blob_len` readable bytes at `blob` (or null with 0), see the header.
    let Some(b) = (unsafe { slice_from_raw(blob, blob_len) }) else {
        return reject(out);
    };
    buffer_result_direct(out, || api::stats_passport(lang, b, first_use_day, today))
}

/// Reminder rules: the planned time of the next reminder (epoch milliseconds).
#[unsafe(no_mangle)]
pub extern "C" fn arca_reminder_plan(now_ms: i64, offset_ms: i32, rnd: i32) -> i64 {
    guard(0, || api::reminder_plan(now_ms, offset_ms, rnd))
}

/// Reminder rules: a planned time corrected for the UTC offset at that time.
#[unsafe(no_mangle)]
pub extern "C" fn arca_reminder_refine(
    target_ms: i64,
    offset_used_ms: i32,
    offset_at_target_ms: i32,
) -> i64 {
    guard(target_ms, || api::reminder_refine(target_ms, offset_used_ms, offset_at_target_ms))
}

/// Reminder rules: what to do when the reminder fires, as JSON.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_reminder_on_fire(
    now_ms: i64,
    offset_ms: i32,
    last_shown_ms: i64,
    enabled: i32,
    can_post: i32,
    rnd: i32,
    out: *mut ArcaBuffer,
) -> i32 {
    buffer_result_direct(out, || {
        api::reminder_on_fire(now_ms, offset_ms, last_shown_ms, enabled, can_post, rnd)
    })
}

/// Reminder rules: what to do with the stored time when the app starts, as JSON.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_reminder_on_start(
    next_ms: i64,
    now_ms: i64,
    offset_ms: i32,
    enabled: i32,
    rnd: i32,
    out: *mut ArcaBuffer,
) -> i32 {
    buffer_result_direct(out, || api::reminder_on_start(next_ms, now_ms, offset_ms, enabled, rnd))
}

/// Reminder rules: the index of the text to show for the stored position.
#[unsafe(no_mangle)]
pub extern "C" fn arca_reminder_pick(seq: i32) -> i32 {
    guard(0, || api::reminder_pick(seq))
}

/// Reminder rules: the number of reminder texts the app must have.
#[unsafe(no_mangle)]
pub extern "C" fn arca_reminder_item_count() -> i32 {
    guard(0, api::reminder_item_count)
}

/// Writes the extra support addresses (wallets, Boosty) as JSON into `out`; the list is empty
/// until the owner configures it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_extra_support(out: *mut ArcaBuffer) -> i32 {
    buffer_result_direct(out, api::extra_support)
}

/// Writes license text into `out` (`0` = AGPLv3, `1` = Additional Terms Ru, `2` = En).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_license_text(which: i32, out: *mut ArcaBuffer) -> i32 {
    optional_buffer_result(out, || api::license_text(which))
}

/// Writes release readiness JSON into `out`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arca_release_info(out: *mut ArcaBuffer) -> i32 {
    buffer_result_direct(out, api::release_info)
}

#[cfg(test)]
#[allow(
    clippy::undocumented_unsafe_blocks,
    reason = "each test calls the exports with exactly the arguments its name describes"
)]
mod tests {
    use super::*;

    #[test]
    fn buffer_allocation_and_free() {
        let original = b"Arca Zero Trust Document Engine".to_vec();
        let original_len = original.len();
        let buf = ArcaBuffer::from_vec(original);
        assert_eq!(buf.len, original_len);
        assert!(!buf.data.is_null());

        let slice = unsafe { core::slice::from_raw_parts(buf.data, buf.len) };
        assert_eq!(slice, b"Arca Zero Trust Document Engine");

        unsafe {
            arca_buffer_free(buf);
        }
    }

    #[test]
    fn buffer_free_null_or_empty_is_noop() {
        unsafe {
            arca_buffer_free(ArcaBuffer::empty());
            arca_buffer_free(ArcaBuffer { data: null_mut(), len: 100 });
        }
    }

    #[test]
    fn null_pointers_are_safely_rejected() {
        unsafe {
            assert_eq!(arca_info(null_mut()), Status::BadArgument.code());
            assert_eq!(arca_self_test(null_mut()), Status::BadArgument.code());
            assert_eq!(arca_doc_append(1, null_mut(), 10), Status::BadArgument.code());
            assert_eq!(arca_doc_set_password(1, null_mut(), 10), Status::BadArgument.code());
            assert_eq!(arca_about(1, null_mut()), Status::BadArgument.code());
            assert_eq!(arca_passport(1, null_mut(), 10, 0, null_mut()), Status::BadArgument.code());
        }
    }

    #[test]
    fn a_null_record_with_a_length_is_a_bad_argument_not_an_empty_one() {
        let mut out = ArcaBuffer::empty();
        unsafe {
            assert_eq!(
                arca_passport(1, core::ptr::null(), 36, 0, &raw mut out),
                Status::BadArgument.code()
            );
            assert!(out.data.is_null());
            assert_eq!(
                arca_stats_summary(core::ptr::null(), 8, &raw mut out),
                Status::BadArgument.code()
            );
            assert_eq!(
                arca_stats_record_event(core::ptr::null(), 1, 1, &raw mut out),
                Status::BadArgument.code()
            );
            // An empty record is still fine.
            assert_eq!(arca_stats_summary(core::ptr::null(), 0, &raw mut out), Status::Ok.code());
            arca_buffer_free(core::mem::replace(&mut out, ArcaBuffer::empty()));
        }
    }

    #[test]
    fn a_length_no_slice_can_have_is_refused() {
        let byte = 0u8;
        unsafe {
            assert_eq!(arca_doc_append(1, &raw const byte, usize::MAX), Status::BadArgument.code());
            assert_eq!(
                arca_doc_set_password(1, &raw const byte, usize::MAX),
                Status::BadArgument.code()
            );
        }
    }

    #[test]
    fn the_abi_version_is_the_engine_s() {
        assert_eq!(arca_abi_version(), arca_engine::ABI_VERSION);
    }
}

#[cfg(test)]
mod header_tests {
    /// The C header states the ABI version this library speaks, and the copy of the header in
    /// the Swift package is the same file.
    #[test]
    fn the_header_names_the_abi_version_of_the_library() {
        let header = include_str!("../../../include/arca.h");
        let want = format!("#define ARCA_ABI_VERSION {}\n", arca_engine::ABI_VERSION);
        assert!(header.contains(&want), "include/arca.h must say {want:?}");
        assert_eq!(header, include_str!("../../../app/Sources/ArcaCore/include/arca.h"));
        for gone in ["arca_doc_read_file", "arca_outcome_sanitized_write_file"] {
            assert!(!header.contains(gone), "{gone} is no longer exported");
        }
    }
}
