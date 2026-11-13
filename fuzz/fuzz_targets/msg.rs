// SPDX-License-Identifier: AGPL-3.0-only
//! The reading view of Outlook messages (property streams, attachments). The same entry point as the `preview` target, with a corpus
//! seeded only from this format and its own dictionary, so that the fuzzer's effort goes into
//! this parser. Hostile input must give an error or a bounded, deterministic preview — never a
//! panic, a hang or unbounded memory.
#![no_main]

use arca_preview::{PreviewPolicy, preview};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let policy = PreviewPolicy { max_input_bytes: 8 << 20, ..PreviewPolicy::default() };
    if let Ok(p) = preview(data, &policy) {
        assert_eq!(preview(data, &policy).as_ref(), Ok(&p), "not deterministic");
    }
});
