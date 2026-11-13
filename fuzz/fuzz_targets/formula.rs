// SPDX-License-Identifier: AGPL-3.0-only
//! The spreadsheet formula engine: tokens, references across sheets, functions, errors and
//! circular references, and the shifting of shared formulas. Any formula text must be
//! calculated or refused in bounded time and memory — never a panic.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = core::str::from_utf8(data) {
        if text.len() <= 8192 {
            arca_preview::fuzzing::formula(text);
        }
    }
});
