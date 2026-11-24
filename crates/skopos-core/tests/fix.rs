//! Hints and fixes for problems, on the invalid conformance cases.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use skopos_core::fix::{apply, fix_all};
use skopos_core::{check, parse, Format};

fn invalid() -> Vec<(String, Value)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/invalid");
    let mut out: Vec<(String, Value)> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p: &PathBuf| {
            p.extension().is_some_and(|e| e == "json") && !p.ends_with("index.json")
        })
        .map(|p| {
            let text = fs::read_to_string(&p).unwrap();
            (
                p.file_name().unwrap().to_string_lossy().to_string(),
                parse(&text, Format::Json).unwrap(),
            )
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

#[test]
fn every_problem_has_a_hint_and_every_fix_removes_its_problem() {
    let mut fixes = 0;
    let mut bad = vec![];
    for (name, doc) in invalid() {
        for p in check(&doc) {
            if !(p.hint.ends_with('.') && p.hint.chars().next().unwrap().is_uppercase()) {
                bad.push(format!("{name}: {}", p.hint));
            }
            let Some(fix) = &p.fix else { continue };
            fixes += 1;
            let fixed = apply(&doc, fix).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(
                !check(&fixed)
                    .iter()
                    .any(|q| q.path == p.path && q.message == p.message),
                "{name}: {} did not fix {p}",
                fix.label
            );
        }
        // After fix_all, no problem with a fix is left.
        let (fixed, _) = fix_all(&doc);
        assert!(check(&fixed).iter().all(|p| p.fix.is_none()), "{name}");
    }
    assert!(
        bad.is_empty(),
        "hints that are not sentences:\n{}",
        bad.join("\n")
    );
    assert!(fixes >= 5, "only {fixes} fixes");
}

#[test]
fn certain_fixes() {
    let doc = json!({
        "id": "pa_7Hk2mQ9xR4tV8wZ1", "version": 1, "kind": "spec", "title": "T",
        "created_at": "2026-11-05T09:00:00Z",
        "id_counters": {"FR": 2},
        "items": [
            {"id": "FR2", "type": "requirement", "text": "a", "status": "Accepted"},
            {"id": "FR2", "type": "requirement", "text": "b", "status": "accepted"},
            {"id": "C1", "type": "constraint", "text": "c", "status": "accepted", "modality": "always"},
        ],
    });
    let (fixed, applied) = fix_all(&doc);
    assert!(check(&fixed).is_empty(), "{:?}", check(&fixed));
    let labels: Vec<&str> = applied.iter().map(|f| f.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Change the status of FR2 to accepted",
            "Add $schema",
            "Set the status to draft",
            "Give the second FR2 the ID FR3",
            "Set the counter for C to 1",
        ]
    );
    assert_eq!(fixed["items"][0]["id"], "FR2");
    assert_eq!(fixed["items"][1]["id"], "FR3");
    assert_eq!(fixed["id_counters"], json!({"FR": 3, "C": 1}));
}

#[test]
fn no_fix_when_a_person_must_choose() {
    let doc = json!({
        "$schema": "https://openskopos.org/schema/artifact/0.2.json",
        "id": "pa_7Hk2mQ9xR4tV8wZ1", "version": 1, "kind": "spec", "title": "T",
        "status": "draft", "created_at": "2026-11-05T09:00:00Z", "id_counters": {"C": 1, "FR": 1},
        "items": [
            {"id": "C1", "type": "constraint", "text": "c", "status": "accepted"},
            {"id": "FR1", "type": "requirement", "text": "r", "status": "accepted", "priority": "high"},
        ],
    });
    let problems = check(&doc);
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(problems.iter().all(|p| p.fix.is_none()));
    assert_eq!(
        problems[0].hint,
        "C1 needs a modality: always, never, ask_first, prefer or avoid."
    );
    assert_eq!(
        problems[1].hint,
        "FR1 has the priority \"high\". Use must, should or could."
    );
}
