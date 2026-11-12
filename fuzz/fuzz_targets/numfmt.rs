// SPDX-License-Identifier: AGPL-3.0-only
//! Spreadsheet number formats: sections, colours, conditions, dates and times, fractions,
//! scientific notation, literal text. The first 8 bytes are the value (any f64, NaN and
//! infinities included); the rest is the format code.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((head, code)) = data.split_first_chunk::<8>() else { return };
    let value = f64::from_le_bytes(*head);
    if let Ok(code) = core::str::from_utf8(code) {
        if code.len() <= 1024 {
            arca_preview::fuzzing::number_format(code, value, code);
        }
    }
});
