// SPDX-License-Identifier: AGPL-3.0-only
// Additional terms under AGPLv3 section 7: see LICENSE_ADDITIONAL_TERMS.md in the repository root.

//! Unit tests of the safe API layer.

use arca_engine::Status;

use crate::api::*;

#[test]
fn uninitialized_state_returns_not_ready() {
    if !is_init() && !has_guest() {
        assert_eq!(init(0, 0, 0, 0), Status::NotReady.code());
        assert_eq!(doc_new(10), -i64::from(Status::NotReady.code()));
        assert!(info().is_none());
    }
}

#[test]
fn invalid_handles_produce_status_bad_handle() {
    // An initialised host (of this thread) answers for the handle, not for readiness.
    test_host::install();
    assert_eq!(doc_len(9999), -i64::from(Status::BadHandle.code()));
    assert_eq!(outcome_status(9999), -Status::BadHandle.code());
    assert_eq!(report(9999), Err(Status::BadHandle));
    assert_eq!(preview_meta(9999), Err(Status::BadHandle));
    assert_eq!(sanitized_bytes(9999), Err(Status::BadHandle));
}

#[test]
fn about_and_attribution_screens_produce_valid_json() {
    let json_ru = String::from_utf8(about(0)).expect("valid utf-8");
    assert!(json_ru.contains("\"appName\""));
    assert!(json_ru.contains("\"fingerprint\""));
    assert!(json_ru.contains("\"cards\""));

    let json_en = String::from_utf8(about(1)).expect("valid utf-8");
    assert!(json_en.contains("\"appName\""));

    let lic = String::from_utf8(license_text(0).expect("agpl text")).expect("valid utf-8");
    assert!(lic.contains("GNU AFFERO GENERAL PUBLIC LICENSE"));
}

#[test]
fn passport_flow() {
    // Fresh record
    let json = String::from_utf8(passport(0, &[], 100)).expect("valid utf-8");
    assert!(json.contains("\"title\""));
    assert!(json.contains("\"documents\":0"));

    // Update record
    let updated_record = passport_update(&[], 100, 2, 1);
    assert_eq!(updated_record.len(), 36);

    // Feed updated record
    let json_after = String::from_utf8(passport(0, &updated_record, 100)).expect("valid utf-8");
    assert!(json_after.contains("\"documents\":1"));
    assert!(json_after.contains("\"trackers\":2"));
    assert!(json_after.contains("\"active\":1"));
}

#[test]
fn document_and_job_lifecycle() {
    test_host::install();
    let doc = doc_new(1024 * 1024);
    assert!(doc > 0);
    let chunk = b"Hello, Arca!";
    assert_eq!(doc_append(doc, chunk), Status::Ok.code());
    assert_eq!(doc_len(doc), i64::try_from(chunk.len()).unwrap_or(-1));

    let job = job_new();
    assert!(job > 0);
    job_cancel(job);
    job_free(job);

    doc_free(doc);
    assert_eq!(doc_len(doc), -i64::from(Status::BadHandle.code()));
}

#[test]
fn non_positive_handles_are_bad_handles() {
    test_host::install();
    assert_eq!(doc_len(0), -i64::from(Status::BadHandle.code()));
    assert_eq!(doc_len(-5), -i64::from(Status::BadHandle.code()));
    assert_eq!(doc_append(i64::MIN, b"x"), Status::BadHandle.code());
}

#[test]
fn statistics_and_reminder_rules_work_without_a_guest() {
    // Feed three outcomes and read the flat totals; there is no day anywhere in the interface.
    let mut blob = stats_record_scan(&[], 0, 0, 0);
    blob = stats_record_scan(&blob, 2, 1, 4);
    blob = stats_record_event(&blob, 1);
    let json = String::from_utf8(stats_summary(&blob)).expect("utf-8");
    assert!(
        json.contains("\"all\":{\"opened\":2,\"clean\":1,\"trackers\":1,\"active\":4"),
        "{json}"
    );
    assert!(json.contains("\"sterile\":1"), "{json}");
    assert!(!json.contains("week") && !json.contains("firstDay"), "{json}");
    // Hostile blobs are empty statistics, never a failure.
    let junk = stats_summary(&[0xff; 300]);
    assert!(String::from_utf8(junk).expect("utf-8").contains("\"empty\":true"));
    let pass = String::from_utf8(stats_passport(1, &blob, 20_000, 20_001)).expect("utf-8");
    assert!(pass.contains("\"documents\":2"), "{pass}");

    // Reminder rules.
    let now = 1_772_323_200_000_i64;
    let t = reminder_plan(now, 3_600_000, 7);
    assert!(t > now + 6 * 86_400_000);
    assert_eq!(reminder_refine(t, 3_600_000, 3_600_000), t);
    let fire =
        String::from_utf8(reminder_on_fire(now + 13 * 3_600_000, 0, 0, 1, 1, 0)).expect("utf-8");
    assert!(fire.starts_with("{\"action\":\"show\""), "{fire}");
    let start = String::from_utf8(reminder_on_start(0, now, 0, 1, 0)).expect("utf-8");
    assert!(start.contains("\"plan\""), "{start}");
    assert_eq!(reminder_item_count(), 30);
    assert_eq!(reminder_pick(31), 1);
}

// ---- the whole path the iOS app takes, over real fixtures and the real guest ----------------
//
// These need the guest embedded at build time (`ARCA_GUEST_WASM`); CI sets it together with
// `ARCA_REQUIRE_E2E=1`, and without it they say so and return.

fn ready() -> bool {
    if !has_guest() {
        assert!(
            std::env::var_os("ARCA_REQUIRE_E2E").is_none_or(|v| v.is_empty() || v == "0"),
            "ARCA_REQUIRE_E2E is set but the library was built without ARCA_GUEST_WASM"
        );
        #[allow(clippy::print_stderr)] // a skipped test says so
        {
            eprintln!("skipped: built without ARCA_GUEST_WASM");
        }
        return false;
    }
    assert_eq!(init(0, 0, 0, 0), 0);
    true
}

fn json(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).expect("utf-8")
}

/// The numbers of `outcome_info`: the sandbox must report real, non-zero resource use.
fn assert_real_metrics(outcome: i64, what: &str) {
    let info = json(outcome_info(outcome).expect("outcome info"));
    let num = |key: &str| -> u64 {
        let tail = info
            .split(&format!("\"{key}\":"))
            .nth(1)
            .unwrap_or_else(|| panic!("{what}: no {key} in {info}"));
        let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
        digits.parse().unwrap_or_else(|_| panic!("{what}: bad {key} in {info}"))
    };
    assert!(num("fuel") > 0, "{what}: fuel is zero: {info}");
    assert!(num("peakMemory") > 0, "{what}: peak memory is zero: {info}");
    assert!(num("fuelLimit") >= num("fuel"), "{what}: {info}");
}

/// Opens the bytes the way `arca_doc_append` does, runs `op` and returns the outcome handle.
fn run_op(bytes: &[u8], op: i32, flags: i32) -> (i64, i64) {
    let doc = doc_new(64 << 20);
    assert!(doc > 0, "{doc}");
    assert_eq!(doc_append(doc, bytes), 0);
    let job = job_new();
    assert!(job > 0);
    let outcome = run(job, doc, op, flags);
    job_free(job);
    assert!(outcome > 0, "op {op} returned {outcome}");
    assert_eq!(outcome_status(outcome), 0, "op {op}");
    (doc, outcome)
}

#[test]
fn preview_slices_carry_real_content_for_every_family() {
    if !ready() {
        return;
    }
    let fixtures: [(&str, &[u8], &str); 5] = [
        ("report.docx", include_bytes!("../../../tests/fixtures/report.docx"), "\"k\":\"p\""),
        ("report.odt", include_bytes!("../../../tests/fixtures/report.odt"), "\"k\":\"p\""),
        ("slides.pptx", include_bytes!("../../../tests/fixtures/slides.pptx"), "\"k\":\"p\""),
        ("sheet.xlsx", include_bytes!("../../../tests/fixtures/sheet.xlsx"), "\"k\":\"g"),
        ("report.pdf", include_bytes!("../../../tests/fixtures/report.pdf"), "\"k\":\""),
    ];
    for (name, bytes, marker) in fixtures {
        // arca.h: ARCA_OP_PREVIEW = 3, ARCA_FLAG_REVEAL_HIDDEN = 1.
        let (doc, outcome) = run_op(bytes, 3, 1);
        assert_real_metrics(outcome, name);
        let meta = json(preview_meta(outcome).expect("meta"));
        assert!(meta.contains("\"sections\""), "{name}: {meta}");
        assert!(!meta.contains("\"sections\":[]"), "{name}: no sections: {meta}");
        assert!(
            !meta.contains("\"items\":0,") || meta.matches("\"items\":").count() > 1,
            "{name}: {meta}"
        );

        // Slice the first section the way the app does: 64 at a time, until it is exhausted.
        let first = json(preview_items(outcome, 0, 0, 64).expect("first slice"));
        assert!(first.starts_with('[') && first != "[]", "{name}: empty first slice: {first}");
        assert!(first.contains(marker), "{name}: no {marker} item in {first}");
        let again = json(preview_items(outcome, 0, 0, 64).expect("same slice"));
        assert_eq!(first, again, "{name}: a slice must not depend on how often it is asked for");
        // Past the end is an empty array, never an error.
        let beyond = json(preview_items(outcome, 0, 1 << 40, 64).expect("beyond the end"));
        assert_eq!(beyond, "[]", "{name}");
        outcome_free(outcome);
        doc_free(doc);
    }
}

#[test]
fn scan_render_and_image_report_real_metrics() {
    if !ready() {
        return;
    }
    let pdf: &[u8] = include_bytes!("../../../tests/fixtures/report.pdf");
    let (doc, scan) = run_op(pdf, 1, 0);
    assert_real_metrics(scan, "scan");
    assert!(json(report(scan).expect("report")).contains("\"verdict\""));
    outcome_free(scan);

    // arca.h: ARCA_OP_RENDER = 4, the flags carry the page number.
    let job = job_new();
    let render = run(job, doc, 4, 0);
    job_free(job);
    assert!(render > 0, "{render}");
    assert_eq!(outcome_status(render), 0);
    assert_real_metrics(render, "render");
    let drawing = json(page(render).expect("page"));
    assert!(drawing.contains("\"p\":[[") && drawing.contains("\"w\":"), "{drawing}");
    assert!(drawing.contains("\"h\":"), "{drawing}");
    outcome_free(render);
    doc_free(doc);
}
