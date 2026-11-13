// SPDX-License-Identifier: AGPL-3.0-only
//! Plain-text readers: encoding detection (UTF-8, UTF-16, legacy code pages), plain text, CSV,
//! TSV and Markdown (headings, lists, tables, links, inline HTML). Never a panic, a hang or
//! unbounded memory.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() <= 1 << 20 {
        arca_preview::fuzzing::plain_text(data);
    }
});
