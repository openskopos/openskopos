use ruzstd::encoding::{compress_to_vec, CompressionLevel};
use serde_json::json;
use skopos_core::search::{search_all, search_archive, search_lines, Search, Source};

fn doc(id: &str, version: u64, items: &str) -> String {
    format!(r#"{{"id":"{id}","version":{version},"kind":"spec","items":[{items}]}}"#)
}

#[test]
fn finds_each_item_once_from_its_newest_version() {
    let lines = [
        doc(
            "pa_0000000000000001",
            1,
            r#"{"id":"D1","type":"decision","status":"proposed","text":"Count seats nightly."}"#,
        ),
        doc(
            "pa_0000000000000001",
            3,
            r#"{"id":"D1","type":"decision","status":"accepted","text":"Count SEATS at the plan change."}"#,
        ),
        doc(
            "pa_0000000000000001",
            2,
            r#"{"id":"D1","type":"decision","status":"proposed","text":"Count seats twice."},{"id":"R1","type":"risk","text":"Late.","mitigation":"Recount seats."}"#,
        ),
        doc(
            "pa_0000000000000002",
            1,
            r#"{"id":"G1","type":"goal","text":"Invoices are right."}"#,
        ),
    ]
    .join("\n");
    let archive = compress_to_vec(lines.as_bytes(), CompressionLevel::Fastest);
    let r = search_archive(archive.as_slice(), "Seats").unwrap();
    assert_eq!(r.documents, 4);
    assert_eq!(r.documents_parsed, 3);
    let hits: Vec<_> = r
        .hits
        .iter()
        .map(|h| {
            (
                h.reference.as_str(),
                h.status.as_str(),
                h.source,
                h.matched_in_latest,
                h.removed_in_version,
                h.versions_matched,
            )
        })
        .collect();
    assert_eq!(
        hits,
        [
            (
                "pa_0000000000000001@3#D1",
                "accepted",
                Source::Archive,
                true,
                None,
                3
            ),
            (
                "pa_0000000000000001@2#R1",
                "",
                Source::Archive,
                false,
                Some(3),
                1
            )
        ]
    );
}

#[test]
fn searches_both_current_and_archive_and_highlights_source_and_staleness() {
    let archived = [
        doc(
            "pa_0000000000000001",
            1,
            r#"{"id":"D1","type":"decision","status":"accepted","text":"Run nightly batch."}"#,
        ),
        doc(
            "pa_0000000000000002",
            1,
            r#"{"id":"D1","type":"decision","status":"accepted","text":"Archived nightly rule."}"#,
        ),
    ]
    .join("\n");
    let archive = compress_to_vec(archived.as_bytes(), CompressionLevel::Fastest);

    let current = vec![
        // pa_1 v2 rejects D1 and rewrites text so "nightly" no longer appears.
        json!({
            "id": "pa_0000000000000001",
            "version": 2,
            "kind": "spec",
            "items": [
                {"id": "D1", "type": "decision", "status": "rejected", "text": "Run on demand instead."}
            ]
        }),
        // pa_3 exists only in current docs.
        json!({
            "id": "pa_0000000000000003",
            "version": 1,
            "kind": "spec",
            "items": [
                {"id": "D1", "type": "decision", "status": "accepted", "text": "Current nightly check."}
            ]
        }),
    ];

    let r = search_all(&current, [archive.as_slice()], "nightly").unwrap();
    assert_eq!(r.hits.len(), 3);
    // Current hits sort before archive-only hits.
    assert_eq!(r.hits[0].artifact, "pa_0000000000000001");
    assert_eq!(r.hits[0].source, Source::Current);
    assert_eq!(r.hits[0].version, 2);
    assert_eq!(r.hits[0].status, "rejected");
    assert_eq!(r.hits[0].matched_version, 1);
    assert_eq!(r.hits[0].matched_source, Source::Archive);
    assert!(!r.hits[0].matched_in_latest);

    assert_eq!(r.hits[1].artifact, "pa_0000000000000003");
    assert_eq!(r.hits[1].source, Source::Current);
    assert_eq!(r.hits[1].matched_source, Source::Current);
    assert!(r.hits[1].matched_in_latest);

    assert_eq!(r.hits[2].artifact, "pa_0000000000000002");
    assert_eq!(r.hits[2].source, Source::Archive);
    assert_eq!(r.hits[2].matched_source, Source::Archive);
    assert!(r.hits[2].matched_in_latest);
}

#[test]
fn reads_concatenated_zstd_frames_and_escaped_unicode() {
    let f1 = compress_to_vec(
        doc(
            "pa_0000000000000001",
            1,
            r#"{"id":"D1","type":"decision","status":"accepted","text":"Visit the caf\u00e9."}"#,
        )
        .as_bytes(),
        CompressionLevel::Fastest,
    );
    let f2 = compress_to_vec(
        format!(
            "\n{}",
            doc(
                "pa_0000000000000002",
                1,
                r#"{"id":"D1","type":"decision","status":"accepted","text":"Another café rule."}"#,
            )
        )
        .as_bytes(),
        CompressionLevel::Fastest,
    );
    let mut multi = f1;
    multi.extend_from_slice(&f2);

    let r = search_archive(multi.as_slice(), "café").unwrap();
    assert_eq!(r.documents, 2);
    assert_eq!(r.hits.len(), 2);
}

#[test]
fn rejects_oversized_zstd_window_and_handles_empty_query() {
    // Minimal Zstandard frame header with window descriptor 0x88 (exp = 17 =>
    // 1 << 27 = 128 MiB > 8 MiB MAX_WINDOW_SIZE).
    let oversized_frame: [u8; 8] = [0x28, 0xB5, 0x2F, 0xFD, 0x00, 0x88, 0x00, 0x00];
    let err = search_archive(oversized_frame.as_slice(), "seat").unwrap_err();
    assert!(err.contains("window size"), "unexpected error: {err}");

    let lines = doc(
        "pa_0000000000000001",
        1,
        r#"{"id":"D1","type":"decision","status":"accepted","text":"Hello."}"#,
    );
    let empty = search_lines(lines.as_bytes(), "   ").unwrap();
    assert!(empty.hits.is_empty());
    assert_eq!(empty.documents, 0);
}

#[test]
fn searches_list_and_nested_fields_and_updates_stale_archive_hits() {
    let lines = [
        doc(
            "pa_0000000000000001",
            1,
            r#"{"id":"T1","type":"term","status":"accepted","text":"Seat","definition":"User.","aliases":["License"],"code_names":["seat_count"]}"#,
        ),
        doc(
            "pa_0000000000000001",
            2,
            r#"{"id":"T1","type":"term","status":"rejected","text":"Seat","definition":"User."}"#,
        ),
    ]
    .join("\n");
    let mut s = Search::new("license");
    s.add_lines(lines.as_bytes()).unwrap();
    let r = s.finish();
    assert_eq!(r.hits.len(), 1);
    assert_eq!(r.hits[0].version, 2);
    assert_eq!(r.hits[0].status, "rejected");
    assert_eq!(r.hits[0].matched_version, 1);
    assert!(!r.hits[0].matched_in_latest);
}
