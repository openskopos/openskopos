use ruzstd::encoding::{compress_to_vec, CompressionLevel};
use skopos_core::search::search_archive;

fn doc(id: &str, version: u64, items: &str) -> String {
    format!(r#"{{"id":"{id}","version":{version},"kind":"spec","items":[{items}]}}"#)
}

#[test]
fn finds_each_item_once_from_its_newest_version() {
    let lines = [
        doc("pa_0000000000000001", 1, r#"{"id":"D1","type":"decision","status":"proposed","text":"Count seats nightly."}"#),
        doc("pa_0000000000000001", 3, r#"{"id":"D1","type":"decision","status":"accepted","text":"Count SEATS at the plan change."}"#),
        doc("pa_0000000000000001", 2, r#"{"id":"D1","type":"decision","status":"proposed","text":"Count seats twice."},{"id":"R1","type":"risk","text":"Late.","mitigation":"Recount seats."}"#),
        doc("pa_0000000000000002", 1, r#"{"id":"G1","type":"goal","text":"Invoices are right."}"#),
    ]
    .join("\n");
    let archive = compress_to_vec(lines.as_bytes(), CompressionLevel::Fastest);
    let r = search_archive(archive.as_slice(), "Seats").unwrap();
    assert_eq!(r.documents, 4);
    assert_eq!(r.documents_parsed, 3);
    let hits: Vec<_> = r
        .hits
        .iter()
        .map(|h| (h.reference.as_str(), h.status.as_str(), h.versions_matched))
        .collect();
    assert_eq!(
        hits,
        [
            ("pa_0000000000000001@3#D1", "accepted", 3),
            ("pa_0000000000000001@2#R1", "", 1)
        ]
    );
}
