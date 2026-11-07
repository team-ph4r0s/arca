// SPDX-License-Identifier: AGPL-3.0-only
// Additional terms under AGPLv3 section 7: see LICENSE_ADDITIONAL_TERMS.md in the repository root.

//! The properties of the `wire` fuzz target (fuzz/fuzz_targets/wire.rs), run on stable Rust over
//! inputs the fuzzer once found: whatever the host decodes must re-encode to something that
//! decodes to the same value, and a larger budget never changes what a smaller one accepted.
//! Every input that ever failed the fuzzer is kept here, so that the failure is a plain test.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test")]

use arca_abi::envelope::{PayloadKind, open};
use arca_abi::wire::{self, DecodeLimits};
use arca_image::Image;
use arca_preview::Preview;
use arca_preview::page::PageModel;
use arca_xray::XRayReport;
use arca_xray::sanitize::SanitizedDocument;

/// Two values are the same when they encode to the same bytes: that is bit-exact, so a NaN is the
/// same NaN (`NaN != NaN` made `assert_eq!` fail on a page whose size the guest left undefined).
fn same<T: wire::Wire>(a: &T, b: &T) -> bool {
    wire::to_bytes(a).ok() == wire::to_bytes(b).ok()
}

fn roundtrip<T: wire::Wire + PartialEq + core::fmt::Debug>(payload: &[u8]) {
    let limits = DecodeLimits::default();
    if let Ok(value) = wire::from_bytes::<T>(payload, limits) {
        let bytes = wire::to_bytes(&value).expect("a decoded value re-encodes");
        let back: T = wire::from_bytes(&bytes, limits).expect("re-encoded bytes decode");
        assert!(same(&back, &value), "a decoded value must survive its own re-encoding");
    }
}

fn decode_with<T: wire::Wire>(
    payload: &[u8],
    limits: DecodeLimits,
    budget: usize,
) -> Result<T, wire::WireError> {
    let mut r = wire::Reader::new(payload, limits).with_total_budget(budget);
    let v = T::decode(&mut r)?;
    r.finish()?;
    Ok(v)
}

fn budgeted<T: wire::Wire + PartialEq + core::fmt::Debug>(payload: &[u8]) {
    let full = DecodeLimits::default();
    let mut first_ok: Option<T> = None;
    for budget in [64, 1 << 10, 1 << 16, 1 << 22, wire::DEFAULT_TOTAL_BYTES] {
        let r = decode_with::<T>(payload, full, budget);
        if let Some(smaller) = &first_ok {
            assert!(r.as_ref().ok().is_some_and(|v| same(v, smaller)));
        } else if let Ok(v) = r {
            first_ok = Some(v);
        }
    }
    let tight = DecodeLimits { max_string_bytes: 256, max_items: 16, max_depth: 4 };
    if let Ok(v) = decode_with::<T>(payload, tight, wire::DEFAULT_TOTAL_BYTES) {
        let wide = decode_with::<T>(payload, full, wire::DEFAULT_TOTAL_BYTES).ok();
        assert!(wide.is_some_and(|w| same(&w, &v)));
    }
}

fn both<T: wire::Wire + PartialEq + core::fmt::Debug>(payload: &[u8]) {
    roundtrip::<T>(payload);
    budgeted::<T>(payload);
}

/// Everything the fuzz target does with one input.
fn target(data: &[u8]) {
    both::<XRayReport>(data);
    both::<SanitizedDocument>(data);
    both::<Preview>(data);
    both::<PageModel>(data);
    both::<Image>(data);
    both::<String>(data);
    if let Ok((kind, payload)) = open(data) {
        match kind {
            PayloadKind::XRayReport => both::<XRayReport>(payload),
            PayloadKind::SanitizedDocument => both::<SanitizedDocument>(payload),
            PayloadKind::Preview => both::<Preview>(payload),
            PayloadKind::Page => both::<PageModel>(payload),
            PayloadKind::Image => both::<Image>(payload),
            PayloadKind::Error => both::<String>(payload),
        }
    }
}

/// The input that failed the fuzzer in CI (run 42244878717, "deadly signal"), as a byte string.
const CRASH_FROM_CI: [u8; 22] =
    [0, 8, 246, 255, 4, 0, 255, 255, 0, 12, 54, 130, 1, 146, 22, 0, 0, 0, 0, 0, 0, 0];

#[test]
fn the_input_that_failed_the_fuzzer_in_ci() {
    // The fuzzer's main thread has 8 MiB of stack; so does this one.
    std::thread::Builder::new()
        .stack_size(8 << 20)
        .spawn(|| target(&CRASH_FROM_CI))
        .expect("a thread")
        .join()
        .expect("the wire properties hold for the CI crash input");
}

/// A hostile guest states `u32::MAX × u32::MAX` pixels: `width × height × 4` overflows `u64`.
/// That used to panic in `Image::decode` (found by the `wire` fuzzer in CI); it must be an error.
#[test]
fn an_image_that_states_an_overflowing_size_is_refused_not_a_panic() {
    let png = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../arca-image/tests/fixtures/img-411-opt.ref.png"
    ))
    .expect("a fixture picture");
    let image = arca_image::decode(&png, &arca_image::Limits::default()).expect("a readable PNG");
    let (w, h) = (image.width, image.height);
    let mut bytes = wire::to_bytes(&image).expect("an image encodes");
    // The sizes are two little-endian u32 in a row, the last such pair before the pixel buffer
    // (the facts before them state the size of the original too).
    let stated: Vec<u8> = [w.to_le_bytes(), h.to_le_bytes()].concat();
    let head = bytes.len() - image.rgba.len();
    let at = bytes[..head]
        .windows(stated.len())
        .rposition(|pair| pair == stated.as_slice())
        .expect("the stated size is in the encoding");
    bytes[at..at + 8].copy_from_slice(&[0xFF; 8]);
    let decoded = wire::from_bytes::<Image>(&bytes, DecodeLimits::default());
    assert!(decoded.is_err(), "an overflowing size must be refused");
    // And the whole fuzz target survives it.
    target(&bytes);
}
