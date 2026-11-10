//! Runs the Markdown cases in `conformance/markdown/` through skopos-core
//! (specification sections 10 and 13), and a Markdown round trip of every
//! valid document.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use skopos_core::markdown::{next_version_at, read_with, write, Fill};
use skopos_core::{model::ItemType, parse, Format};

const NOW: &str = "2026-11-10T12:00:00Z";

fn fill() -> Fill {
    Fill {
        now: NOW.to_string(),
        artifact_id: "pa_0000000000000000".to_string(),
    }
}

fn conformance() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}

fn text(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn doc(path: &Path) -> Value {
    let format = match path.extension().and_then(|e| e.to_str()) {
        Some("yaml" | "yml") => Format::Yaml,
        _ => Format::Json,
    };
    parse(&text(path), format).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn cases(group: &str) -> Vec<Value> {
    doc(&conformance().join("markdown/index.json"))[group]
        .as_array()
        .unwrap_or_else(|| panic!("index has {group}"))
        .clone()
}

/// The document with its items in the order of the sections (section 13).
fn normal(mut doc: Value) -> Value {
    let rank = |i: &Value| {
        let t = i["type"].as_str().and_then(|t| t.parse::<ItemType>().ok());
        t.and_then(|t| ItemType::ALL.iter().position(|x| *x == t))
    };
    if let Some(items) = doc["items"].as_array_mut() {
        items.sort_by_key(rank);
    }
    doc
}

fn assert_same(got: &Value, want: &Value, what: &str) {
    let (got, want) = (normal(got.clone()), normal(want.clone()));
    assert!(
        got == want,
        "{what}\n--- got:\n{}\n--- want:\n{}",
        serde_json::to_string_pretty(&got).unwrap(),
        serde_json::to_string_pretty(&want).unwrap()
    );
}

fn read_ok(md: &str, filename: &str, what: &str) -> (Value, usize) {
    match read_with(md, Some(filename), &fill()) {
        Ok((doc, warnings)) => (doc, warnings.len()),
        Err(errors) => panic!("{what}: {errors:#?}"),
    }
}

#[test]
fn round_trip() {
    for case in cases("round_trip") {
        let name = case["case"].as_str().unwrap();
        let dir = conformance().join("markdown/round-trip").join(name);
        let want = doc(&dir.join("document.json"));

        // Reading document.md gives document.json.
        let (got, warnings) = read_ok(&text(&dir.join("document.md")), "document.md", name);
        assert_same(&got, &want, &format!("{name}: reading document.md"));
        assert_eq!(warnings, 0, "{name}: warnings");

        // Writing document.json and reading the result gives document.json.
        let mut inputs = vec![];
        if let Ok(entries) = fs::read_dir(dir.join("inputs")) {
            for e in entries {
                inputs.push(doc(&e.unwrap().path()));
            }
        }
        let md = write(&want, &inputs);
        let (back, warnings) = read_ok(&md, "document.md", name);
        assert_same(
            &back,
            &want,
            &format!("{name}: written and read again:\n{md}"),
        );
        assert_eq!(warnings, 0, "{name}: warnings");

        // The writer makes the same layout as document.md, also the copied
        // material, below the front matter.
        let body = |s: &str| s.split_once("\n---\n").map(|(_, body)| body.to_string());
        assert_eq!(
            body(&md),
            body(&text(&dir.join("document.md"))),
            "{name}: the body of the written file"
        );
    }
}

#[test]
fn generous() {
    for case in cases("generous") {
        let name = case["case"].as_str().unwrap();
        let file = case["file"].as_str().unwrap();
        let dir = conformance().join("markdown/generous").join(name);
        let md = text(&dir.join(file));
        let mut want = doc(&dir.join("expected.json"));
        let (got, warnings) = read_ok(&md, file, name);
        // If the file has no id or created_at, the comparison ignores it.
        for key in ["id", "created_at"] {
            if !md.lines().any(|l| l.starts_with(&format!("{key}:"))) {
                want[key] = got[key].clone();
            }
        }
        assert_same(&got, &want, name);
        assert_eq!(
            warnings,
            case["warnings"].as_u64().unwrap() as usize,
            "{name}: number of warnings"
        );
    }
}

#[test]
fn next_version() {
    for case in cases("next_version") {
        let name = case["case"].as_str().unwrap();
        let dir = conformance().join("markdown/next-version").join(name);
        let stored = doc(&dir.join("stored.json"));
        let mut want = doc(&dir.join("expected.json"));
        let (read, _) = read_ok(&text(&dir.join("edited.md")), "edited.md", name);
        let got = next_version_at(&stored, &read, NOW);
        // If the version number changes, the comparison ignores created_at.
        if want["version"] != stored["version"] {
            want["created_at"] = got["created_at"].clone();
        }
        assert_same(&got, &want, name);
        assert!(
            skopos_core::check(&got).is_empty(),
            "{name}: the result is valid"
        );
    }
}

#[test]
fn invalid() {
    for case in cases("invalid") {
        let name = case["case"].as_str().unwrap();
        let file = case["file"].as_str().unwrap();
        let md = text(&conformance().join("markdown/invalid").join(name).join(file));
        match read_with(&md, Some(file), &fill()) {
            Ok((doc, _)) => panic!("{name}: the reader made a document: {doc:#}"),
            Err(errors) => {
                assert!(!errors.is_empty(), "{name}");
                for e in &errors {
                    assert!(!e.message.is_empty(), "{name}: {e:?}");
                }
            }
        }
    }
}

/// Errors name the line where they can.
#[test]
fn errors_name_their_line() {
    let lines = |file: &str, name: &str| {
        let md = text(&conformance().join("markdown/invalid").join(name).join(file));
        read_with(&md, Some(file), &fill())
            .unwrap_err()
            .iter()
            .map(|e| e.line)
            .collect::<Vec<_>>()
    };
    assert_eq!(lines("SPEC.md", "bad-enum-value"), [Some(6)]);
    assert_eq!(lines("INTENT.md", "bad-boolean"), [Some(6)]);
    assert_eq!(lines("SPEC.md", "bad-ref"), [Some(6)]);
    assert_eq!(lines("INTENT.md", "broken-front-matter"), [Some(2)]);
    assert_eq!(lines("INTENT.md", "incomplete-votes"), [Some(6)]);
    // A schema problem has the line of its item.
    assert!(lines("INTENT.md", "constraint-without-modality")
        .iter()
        .all(|l| *l == Some(5)));
    assert_eq!(lines("INTENT.md", "no-title"), [None]);
}

/// Every valid document goes to Markdown and back with no loss.
#[test]
fn valid_documents_round_trip() {
    let index = doc(&conformance().join("valid/index.json"));
    let files = index["cases"].as_array().unwrap();
    assert!(!files.is_empty());
    for case in files {
        let file = case["file"].as_str().unwrap();
        let want = doc(&conformance().join("valid").join(file));
        let md = write(&want, &[]);
        let (got, warnings) = read_ok(&md, file, file);
        assert_same(&got, &want, &format!("valid/{file}:\n{md}"));
        assert_eq!(warnings, 0, "valid/{file}: warnings");
        // Writing again gives the same text.
        assert_eq!(write(&got, &[]), md, "valid/{file}: written again");
    }
}

fn errors(md: &str) -> Vec<String> {
    read_with(md, Some("INTENT.md"), &fill())
        .unwrap_err()
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn a_field_with_one_value_appears_once() {
    let md = "# I\n\n## Risks\n\n- R1: Late.\n  - impact: low\n  - impact: high\n";
    assert_eq!(
        errors(md),
        ["line 7: impact appears twice in this item: keep one impact line"]
    );
}

#[test]
fn unknown_keys_in_structured_fields_are_errors() {
    let md = "# C\n\n## Critiques\n\n- CR1: Gap.\n  - severity: minor\n  - category: gap\n  - evidence:\n    - said: No limit.\n      - colour: blue\n    - rumour: Heard it.\n";
    let got = errors(md);
    assert_eq!(got.len(), 2, "{got:?}");
    assert!(got[0].starts_with("line 10: discussion evidence has no field 'colour'"));
    assert!(got[1].starts_with("line 11: evidence is quote, code, discussion or link"));
}

/// A detail with a list and a code block, and `x-` values of each JSON type,
/// survive a round trip.
#[test]
fn details_and_extension_values_round_trip() {
    let doc = serde_json::json!({
        "$schema": "https://openskopos.org/schema/artifact/0.1.json",
        "id": "pa_7Hk2mQ9xR4tV8wZ1",
        "version": 1,
        "kind": "intent",
        "title": "Edge cases",
        "summary": "Two paragraphs.\n\nThe second one.",
        "status": "draft",
        "id_counters": {"G": 2},
        "items": [
            {
                "id": "G1", "type": "goal", "text": "One.", "status": "accepted",
                "detail": "A list:\n\n- first\n- second\n\n```\n## not a heading\n- not an item\n```",
                "x-number": 3, "x-text": "12", "x-lines": "a\nb", "x-map": {"a": [1, true, null]}
            },
            {"id": "G2", "type": "goal", "text": "Two.", "status": "proposed"}
        ],
        "created_at": "2026-11-05T09:00:00Z"
    });
    let md = write(&doc, &[]);
    let (back, warnings) = read_ok(&md, "INTENT.md", &md);
    assert_same(&back, &doc, &md);
    assert_eq!(warnings, 0);
}

/// Values that look like Markdown or like IDs survive a round trip: a title
/// that ends in `#`, a target section and a quote title that look like item
/// IDs, a code path that ends in `:<number>`, and a detail with a code fence
/// that has no closing line.
#[test]
fn values_that_look_like_markup_round_trip() {
    let doc = serde_json::json!({
        "$schema": "https://openskopos.org/schema/artifact/0.1.json",
        "id": "pa_3fQ8nV2kLm7Xc1Rt",
        "version": 1,
        "kind": "critique",
        "title": "Support for C#",
        "status": "draft",
        "id_counters": {"CR": 2},
        "items": [
            {
                "id": "CR1", "type": "critique", "text": "Q1 is vague.", "status": "accepted",
                "severity": "minor", "category": "ambiguity",
                "target": {"section": "Q1"},
                "evidence": [
                    {"kind": "quote", "text": "Ship in Q1.", "from": {"title": "FR2"}},
                    {"kind": "code", "path": "config/app.yml:2", "line_start": 10}
                ],
                "detail": "Start of a block:\n\n```\nno end"
            },
            {
                "id": "CR2", "type": "critique", "text": "Second.", "status": "accepted",
                "severity": "note", "category": "gap",
                "target": {"section": "Retry policy"},
                "evidence": [{"kind": "discussion", "text": "Said so."}]
            }
        ],
        "created_at": "2026-11-05T09:00:00Z"
    });
    let md = write(&doc, &[]);
    let (back, warnings) = read_ok(&md, "CRITIQUE.md", &md);
    assert_same(&back, &doc, &md);
    assert_eq!(warnings, 0);
}

/// An indented line under a field continues the value of that field.
#[test]
fn a_wrapped_field_value_continues_the_field() {
    let md = "# I\n\n## Decisions\n\n- D1: Use the queue.\n  - rationale: The queue keeps\n    jobs in order.\n  - decided_by: P1\n";
    let (doc, warnings) = read_ok(md, "INTENT.md", "wrapped");
    assert_eq!(warnings, 0);
    let item = &doc["items"][0];
    assert_eq!(item["rationale"], "The queue keeps jobs in order.");
    assert_eq!(item["decided_by"], "P1");
    assert!(item.get("detail").is_none(), "{item:#}");
}

/// A number that is too large is an error, not a silent drop.
#[test]
fn numbers_that_are_too_large_are_errors() {
    let big = "99999999999999999999";
    let md = format!(
        "# C\n\n## Critiques\n\n- CR1: Gap.\n  - severity: minor\n  - category: gap\n  - evidence:\n    - code: lib/x.ex:{big}\n  - votes: {big} yes, 0 no, 2 eligible\n"
    );
    let got = errors(&md);
    assert_eq!(
        got,
        [
            format!("line 9: {big} is too large for a number"),
            format!("line 10: {big} is too large for a number"),
        ]
    );
}
