//! Golden tests for the exporters and the diff.
//!
//! Each case compares the output with the files in `tests/golden/`. To write
//! the files again after a deliberate change, run the tests with
//! `UPDATE_GOLDEN=1` and review the change in git.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use skopos_core::diff::diff;
use skopos_core::export::{export, Options, Tool};
use skopos_core::{check, parse, Format};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn read(path: &Path) -> Value {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let doc = parse(&text, Format::Json).unwrap();
    assert!(
        check(&doc).is_empty(),
        "{}: {:?}",
        path.display(),
        check(&doc)
    );
    doc
}

fn conformance(file: &str) -> Value {
    read(&root().join("../../conformance/valid").join(file))
}

fn update() -> bool {
    std::env::var_os("UPDATE_GOLDEN").is_some()
}

/// All files under a folder, as paths relative to it with `/`.
fn files_under(dir: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = fs::read_dir(&d) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let rel = path.strip_prefix(dir).unwrap();
                out.insert(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    out
}

fn compare_file(path: &Path, actual: &str) {
    if update() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, actual).unwrap();
        return;
    }
    let expected = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{}: {e}. Run with UPDATE_GOLDEN=1.", path.display()));
    assert_eq!(actual, expected, "{} is different", path.display());
}

const CASES: [&str; 3] = [
    "spec-all-types-v3.json",
    "intent-lock-intent-v2.json",
    "minimal-draft.json",
];

fn golden_export(tool: Tool) {
    for case in CASES {
        let doc = conformance(case);
        let out = export(&doc, tool, &Options::default());
        let dir = root()
            .join("tests/golden")
            .join(tool.as_str())
            .join(case.trim_end_matches(".json"));
        if update() {
            let _ = fs::remove_dir_all(&dir);
        }
        let mut paths = BTreeSet::new();
        for file in &out.files {
            assert!(
                file.path.starts_with(&format!("{}/", out.folder)),
                "{} is outside {}",
                file.path,
                out.folder
            );
            compare_file(&dir.join(&file.path), &file.text);
            paths.insert(file.path.clone());
        }
        assert_eq!(paths, files_under(&dir), "{tool} {case}: the set of files");
        assert_every_item_is_placed(&doc, &out);
    }
}

/// Every item is either in the files with its reference string or in the
/// "Not exported" list, which is also in the files.
fn assert_every_item_is_placed(doc: &Value, out: &skopos_core::export::Export) {
    let all: String = out.files.iter().map(|f| f.text.as_str()).collect();
    let artifact = doc["id"].as_str().unwrap();
    let version = doc["version"].as_u64().unwrap();
    for item in doc["items"].as_array().unwrap() {
        let id = item["id"].as_str().unwrap();
        let reference = format!("{artifact}@{version}#{id}");
        let skipped = out.not_exported.iter().any(|n| n.id == id);
        assert!(
            all.contains(&format!("`{reference}`")),
            "{}: {reference} is not in the files",
            out.tool
        );
        assert_eq!(
            skipped,
            item["status"] != "accepted"
                || item["type"] == "critique"
                || !placed_type(out.tool, item),
            "{}: {reference} skipped = {skipped}",
            out.tool
        );
    }
}

fn placed_type(tool: Tool, item: &Value) -> bool {
    let t = item["type"].as_str().unwrap();
    match tool {
        Tool::OpenSpec => t != "term",
        Tool::Kiro => !["assumption", "non_goal", "risk", "open_question"].contains(&t),
        Tool::SpecKit => t != "risk",
    }
}

#[test]
fn openspec() {
    golden_export(Tool::OpenSpec);
}

#[test]
fn kiro() {
    golden_export(Tool::Kiro);
}

#[test]
fn speckit() {
    golden_export(Tool::SpecKit);
}

#[test]
fn export_is_deterministic_and_takes_a_name_and_number() {
    let doc = conformance("spec-all-types-v3.json");
    let options = Options {
        name: Some("Seat Limits".to_string()),
        number: Some(12),
    };
    assert_eq!(
        export(&doc, Tool::SpecKit, &options),
        export(&doc, Tool::SpecKit, &options)
    );
    assert_eq!(
        export(&doc, Tool::SpecKit, &options).folder,
        "specs/012-seat-limits"
    );
    assert_eq!(
        export(&doc, Tool::OpenSpec, &options).folder,
        "openspec/changes/seat-limits"
    );
}

#[test]
fn diff_of_two_versions() {
    let v2 = read(&root().join("tests/fixtures/spec-v2.json"));
    let v3 = conformance("spec-all-types-v3.json");
    let d = diff(&v2, &v3);
    let dir = root().join("tests/golden/diff");
    compare_file(&dir.join("spec-v2-to-v3.txt"), &d.to_text());
    let mut json = serde_json::to_string_pretty(&d).unwrap();
    json.push('\n');
    compare_file(&dir.join("spec-v2-to-v3.json"), &json);

    let ids = |list: Vec<&str>| list.into_iter().map(str::to_string).collect::<Vec<_>>();
    assert_eq!(
        d.added.iter().map(|i| i.id.clone()).collect::<Vec<_>>(),
        ids(vec!["FR4", "C4", "A2", "R1", "D1", "CR1"])
    );
    assert_eq!(d.superseded.len(), 1);
    assert_eq!(d.superseded[0].id, "FR2");
    assert_eq!(d.superseded[0].by, ids(vec!["FR4"]));
    assert!(d.dropped.is_empty());

    // Backwards, the new items are dropped and nothing is superseded.
    let back = diff(&v3, &v2);
    assert_eq!(back.dropped.len(), 6);
    assert_eq!(back.added[0].id, "FR2");
    assert!(back.superseded.is_empty());
}
