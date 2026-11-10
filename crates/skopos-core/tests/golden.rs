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
use skopos_core::export::{export, File, Options, Tool};
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
        let out = export(&doc, tool, &Options::default()).unwrap();
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
        ..Options::default()
    };
    assert_eq!(
        export(&doc, Tool::SpecKit, &options).unwrap(),
        export(&doc, Tool::SpecKit, &options).unwrap()
    );
    assert_eq!(
        export(&doc, Tool::SpecKit, &options).unwrap().folder,
        "specs/012-seat-limits"
    );
    assert_eq!(
        export(&doc, Tool::OpenSpec, &options).unwrap().folder,
        "openspec/changes/seat-limits-v3"
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

/// A document whose free text holds Markdown that could break the files of a
/// tool: headings in `detail` and `summary`, and brackets in a question.
fn tricky() -> Value {
    serde_json::json!({
        "$schema": "https://openskopos.org/schema/artifact/0.1.json",
        "id": "pa_7Hk2mQ9xR4tV8wZ1", "version": 1, "kind": "spec",
        "title": "Tricky text", "status": "draft",
        "summary": "# Overview\n\n## Background\nWhy we do it.",
        "created_at": "2026-11-05T09:00:00Z",
        "id_counters": {"FR": 1, "Q": 1, "T": 1},
        "items": [
            {"id": "FR1", "type": "requirement", "status": "accepted",
             "text": "Support API v2.5 for all clients.",
             "detail": "## Background\n\n1. First.\n\n```md\n## kept in the fence\n```",
             "examples": [{"text": "A v2.5 client connects."}]},
            {"id": "Q1", "type": "open_question", "status": "accepted",
             "text": "Should limits apply to [legacy] plans?"},
            {"id": "T1", "type": "term", "status": "accepted",
             "text": "Seat*", "definition": "A user."}
        ]
    })
}

#[test]
fn free_text_cannot_add_sections_to_the_files() {
    let doc = tricky();
    assert!(check(&doc).is_empty(), "{:?}", check(&doc));
    for tool in Tool::ALL {
        let out = export(&doc, tool, &Options::default()).unwrap();
        for file in &out.files {
            let mut fenced = false;
            for line in file.text.lines() {
                if line.starts_with("```") {
                    fenced = !fenced;
                }
                assert!(
                    fenced
                        || !(line.starts_with("# Overview") || line.starts_with("## Background")),
                    "{tool} {}: {line}",
                    file.path
                );
            }
        }
    }

    // OpenSpec: the scenario stays inside the requirement, after the detail.
    let out = export(&doc, Tool::OpenSpec, &Options::default()).unwrap();
    let spec = &out
        .files
        .iter()
        .find(|f| f.path.ends_with("spec.md"))
        .unwrap()
        .text;
    let added = spec.split("## ADDED Requirements").nth(1).unwrap();
    let outside_fences: Vec<&str> = added
        .split("```")
        .step_by(2)
        .flat_map(str::lines)
        .filter(|l| l.starts_with("## "))
        .collect();
    assert!(outside_fences.is_empty(), "{outside_fences:?}");
    assert!(added.contains("**Background**"));
    assert!(added.find("**Background**") < added.find("#### Scenario: Example 1"));
    assert!(spec.contains("### Requirement: FR1 - Support API v2.5 for all clients\n"));

    // Spec Kit: the marker ends where the question ends.
    let out = export(&doc, Tool::SpecKit, &Options::default()).unwrap();
    assert!(out.files[0]
        .text
        .contains("- [NEEDS CLARIFICATION: Should limits apply to (legacy) plans?] `"));
    assert!(out.files[0].text.contains("- **Seat\\***: A user."));

    // Kiro: the detail list does not continue the criteria list.
    let out = export(&doc, Tool::Kiro, &Options::default()).unwrap();
    let req = &out.files[0].text;
    assert!(
        req.find("1. First.") < req.find("#### Acceptance Criteria"),
        "{req}"
    );
}

#[test]
fn owned_paths_hold_every_written_file() {
    for case in CASES {
        let doc = conformance(case);
        for tool in Tool::ALL {
            let out = export(&doc, tool, &Options::default()).unwrap();
            for f in &out.files {
                let owned = out.owned.iter().any(|o| match o.strip_suffix('/') {
                    Some(folder) => f.path.starts_with(&format!("{folder}/")),
                    None => *o == f.path,
                });
                assert!(owned, "{tool}: {} is not owned", f.path);
            }
        }
    }
}

// OpenSpec: exports that read the specs of earlier exports.
//
// The fixtures in tests/fixtures/openspec/after-v<n>/ are the main specs
// that the real OpenSpec CLI wrote when it archived the export of version
// <n>. crates/skopos/tests/openspec_roundtrip.sh makes them, and CI runs it.

/// The files of a fixture project that `skopos export` reads: the main
/// specs and the Markdown files of open changes.
fn openspec_tree(name: &str) -> Vec<File> {
    let dir = root().join("tests/fixtures/openspec").join(name);
    files_under(&dir)
        .into_iter()
        .filter(|p| p.ends_with("/spec.md") || p.starts_with("openspec/changes/"))
        .map(|p| File {
            text: fs::read_to_string(dir.join(&p)).unwrap(),
            path: p,
        })
        .collect()
}

fn version_doc(v: u32) -> Value {
    if v == 3 {
        conformance("spec-all-types-v3.json")
    } else {
        read(&root().join(format!("tests/fixtures/spec-v{v}.json")))
    }
}

fn reexport(v: u32, existing: Vec<File>) -> skopos_core::export::Export {
    let options = Options {
        existing,
        ..Options::default()
    };
    export(&version_doc(v), Tool::OpenSpec, &options).unwrap()
}

fn text_of<'a>(out: &'a skopos_core::export::Export, suffix: &str) -> &'a str {
    &out.files
        .iter()
        .find(|f| f.path.ends_with(suffix))
        .unwrap_or_else(|| panic!("no file ends with {suffix}"))
        .text
}

#[test]
fn openspec_versions_match_the_real_round_trip() {
    for v in 2..=5 {
        let existing = if v == 2 {
            // Before the first export, the project has only the team's own
            // capability.
            openspec_tree("after-v2")
                .into_iter()
                .filter(|f| f.path.contains("/notifications/"))
                .collect()
        } else {
            openspec_tree(&format!("after-v{}", v - 1))
        };
        let options = Options {
            name: (v == 2).then(|| "seat-limits".to_string()),
            existing,
            ..Options::default()
        };
        let out = export(&version_doc(v), Tool::OpenSpec, &options).unwrap();
        let dir = root().join(format!("tests/golden/openspec-versions/v{v}"));
        let mut paths = BTreeSet::new();
        for f in &out.files {
            compare_file(&dir.join(&f.path), &f.text);
            paths.insert(f.path.clone());
        }
        assert_eq!(paths, files_under(&dir), "v{v}: the set of files");
        assert!(out.warnings.is_empty(), "v{v}: {:?}", out.warnings);
        assert!(
            out.files.iter().all(|f| !f.path.contains("notifications")),
            "v{v}: the team's own capability is left alone"
        );
    }
}

#[test]
fn openspec_deltas_name_what_changed() {
    // v3: FR4 supersedes FR2. FR3 did not change, so it has no delta.
    let v3 = reexport(3, openspec_tree("after-v2"));
    let delta = text_of(&v3, "specs/seat-limits/spec.md");
    assert!(
        !delta.contains("## Purpose"),
        "an existing capability has no purpose"
    );
    assert!(delta.contains("## ADDED Requirements\n\n### Requirement: FR4 - "));
    assert!(delta.contains("**Reason**: pa_2Xc5Rv8Tn1Mb4Kz7@3 replaces FR2 with FR4."));
    assert!(delta.contains("**Migration**: Follow FR4."));
    assert!(!delta.contains("FR3"), "{delta}");
    let proposal = text_of(&v3, "proposal.md");
    assert!(proposal.contains("### Modified Capabilities\n\n- `seat-limits`"));
    assert!(proposal.contains("- `seat-limits`: adds FR4; removes FR2."));

    // v4: FR3 changes (MODIFIED). FR4 loses a scenario, so it is replaced
    // under a new name. FR5 is new.
    let v4 = reexport(4, openspec_tree("after-v3"));
    let delta = text_of(&v4, "specs/seat-limits/spec.md");
    assert!(delta.contains(
        "## MODIFIED Requirements\n\n### Requirement: FR3 - The seat count never goes below zero\n"
    ));
    assert!(delta.contains(
        "### Requirement: FR4 - A plan change that leaves more seats in use than the new... (v4)"
    ));
    assert!(delta.contains("FR4 changed in pa_2Xc5Rv8Tn1Mb4Kz7@4 and lost a scenario."));

    // v5: FR3 is rejected and FR5 dropped. FR4 did not change.
    let v5 = reexport(5, openspec_tree("after-v4"));
    let delta = text_of(&v5, "specs/seat-limits/spec.md");
    assert!(delta.contains("**Reason**: FR3 is rejected in pa_2Xc5Rv8Tn1Mb4Kz7@5."));
    assert!(delta.contains("**Reason**: FR5 was dropped in pa_2Xc5Rv8Tn1Mb4Kz7@5."));
    assert!(!delta.contains("## MODIFIED"), "{delta}");
}

#[test]
fn openspec_export_with_no_changes_has_no_spec_delta() {
    let out = reexport(5, openspec_tree("after-v5"));
    assert!(out.files.iter().all(|f| !f.path.contains("/specs/")));
    assert!(text_of(&out, ".openspec.yaml").contains("skip_specs: true"));
    assert!(text_of(&out, "proposal.md").contains("- No requirement changes."));
}

#[test]
fn openspec_hand_edit_is_replaced_with_a_warning() {
    let mut tree = openspec_tree("after-v4");
    let spec = tree
        .iter_mut()
        .find(|f| f.path.ends_with("seat-limits/spec.md"))
        .unwrap();
    let edited = spec.text.replace(
        "- **WHEN** 12 seats in use, new limit 10. The change is blocked.",
        "- **WHEN** 12 seats in use, new limit 10. The change is blocked and logged.",
    );
    assert_ne!(edited, spec.text);
    spec.text = edited;
    let out = reexport(5, tree);
    assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
    assert!(out.warnings[0].contains("(FR4) changed after it was exported"));
    let delta = text_of(&out, "specs/seat-limits/spec.md");
    assert!(delta.contains("## MODIFIED Requirements\n\n### Requirement: FR4 - "));
}

#[test]
fn openspec_block_without_hash_is_updated_with_no_warning() {
    let mut tree = openspec_tree("after-v3");
    for f in &mut tree {
        f.text = f
            .text
            .lines()
            .filter(|l| !l.starts_with("<!-- skopos-export:"))
            .collect::<Vec<_>>()
            .join("\n");
    }
    let out = reexport(4, tree);
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    assert!(text_of(&out, "specs/seat-limits/spec.md").contains("## MODIFIED Requirements"));
}

#[test]
fn openspec_finds_a_renamed_and_nested_capability() {
    let mut tree = openspec_tree("after-v4");
    for f in &mut tree {
        f.path = f
            .path
            .replace("specs/seat-limits/", "specs/billing/seat-limits/");
    }
    let out = reexport(5, tree);
    let delta = text_of(&out, "/specs/billing/seat-limits/spec.md");
    assert!(delta.contains("## ADDED Requirements\n\n### Requirement: FR6 - "));
    assert!(!delta.contains("## Purpose"));
    assert!(text_of(&out, "proposal.md")
        .contains("### Modified Capabilities\n\n- `billing/seat-limits`"));
}

/// Moves the FR3 block of the after-v3 tree into a second capability, so
/// that each capability holds one requirement of the artifact.
fn split_tree() -> Vec<File> {
    let mut tree = openspec_tree("after-v3");
    let spec = tree
        .iter_mut()
        .find(|f| f.path.ends_with("seat-limits/spec.md"))
        .unwrap();
    let start = spec.text.find("### Requirement: FR3").unwrap();
    let end = spec.text[start..]
        .find("\n### Requirement:")
        .map_or(spec.text.len(), |e| start + e + 1);
    let block = spec.text[start..end].to_string();
    spec.text.replace_range(start..end, "");
    tree.push(File {
        path: "openspec/specs/seat-counts/spec.md".to_string(),
        text: format!("# seat-counts Specification\n\n## Purpose\nHow seats are counted, kept apart from the limits.\n\n## Requirements\n\n{block}"),
    });
    tree
}

#[test]
fn openspec_refuses_a_tie_between_capabilities() {
    let options = Options {
        existing: split_tree(),
        ..Options::default()
    };
    let err = export(&version_doc(4), Tool::OpenSpec, &options).unwrap_err();
    assert!(err.message.contains("seat-counts and seat-limits"), "{err}");
    assert!(err.message.contains("--name"), "{err}");
}

#[test]
fn openspec_name_breaks_a_tie_and_each_block_stays_in_its_capability() {
    let options = Options {
        name: Some("seat-limits".to_string()),
        existing: split_tree(),
        ..Options::default()
    };
    let out = export(&version_doc(4), Tool::OpenSpec, &options).unwrap();
    let counts = text_of(&out, "specs/seat-counts/spec.md");
    assert!(counts.contains("## MODIFIED Requirements\n\n### Requirement: FR3 - "));
    assert!(!counts.contains("FR5"));
    let limits = text_of(&out, "specs/seat-limits/spec.md");
    assert!(limits.contains("### Requirement: FR5 - "));
}

#[test]
fn openspec_refuses_while_an_earlier_export_is_open() {
    let mut tree = openspec_tree("after-v3");
    let golden = root().join("tests/golden/openspec-versions/v4");
    for p in files_under(&golden) {
        if p.ends_with(".md") {
            tree.push(File {
                text: fs::read_to_string(golden.join(&p)).unwrap(),
                path: p,
            });
        }
    }
    let options = Options {
        existing: tree.clone(),
        ..Options::default()
    };
    let err = export(&version_doc(5), Tool::OpenSpec, &options).unwrap_err();
    assert!(
        err.message.contains(
            "openspec/changes/spec-seat-limits-on-plan-changes-v4 holds an earlier export"
        ),
        "{err}"
    );
    assert!(err
        .message
        .contains("openspec archive spec-seat-limits-on-plan-changes-v4"));

    // Exporting the same version again writes over its own open change.
    let options = Options {
        existing: tree,
        ..Options::default()
    };
    assert!(export(&version_doc(4), Tool::OpenSpec, &options).is_ok());
}
