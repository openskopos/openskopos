//! Runs the `skopos` binary: the conformance cases, and each command.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{json, Value};

fn skopos(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_skopos"))
        .args(args)
        .output()
        .expect("skopos runs")
}

fn conformance() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}

fn cases(dir: &str) -> Vec<Value> {
    let text = fs::read_to_string(conformance().join(dir).join("index.json")).unwrap();
    serde_json::from_str::<Value>(&text).unwrap()["cases"]
        .as_array()
        .unwrap()
        .clone()
}

/// A new empty folder for one test.
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("skopos-cli-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn stdout_json(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&out.stdout)))
}

#[test]
fn check_accepts_every_valid_case() {
    let files: Vec<String> = cases("valid")
        .iter()
        .map(|c| {
            conformance()
                .join("valid")
                .join(c["file"].as_str().unwrap())
                .display()
                .to_string()
        })
        .collect();
    let mut args = vec!["check", "--json"];
    args.extend(files.iter().map(String::as_str));
    let out = skopos(&args);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let results = stdout_json(&out);
    assert_eq!(results.as_array().unwrap().len(), files.len());
    for r in results.as_array().unwrap() {
        assert_eq!(r["problems"], json!([]), "{}", r["file"]);
    }
}

#[test]
fn check_rejects_every_invalid_case_for_its_rule() {
    // One run for all cases, so that the schema compiles one time.
    let cases = cases("invalid");
    let paths: Vec<String> = cases
        .iter()
        .map(|c| {
            conformance()
                .join("invalid")
                .join(c["file"].as_str().unwrap())
                .display()
                .to_string()
        })
        .collect();
    let mut args = vec!["check", "--json"];
    args.extend(paths.iter().map(String::as_str));
    let out = skopos(&args);
    assert_eq!(out.status.code(), Some(1));
    let results = stdout_json(&out);
    let results = results.as_array().unwrap();
    assert_eq!(results.len(), cases.len());
    for (case, result) in cases.iter().zip(results) {
        let file = case["file"].as_str().unwrap();
        let rule = case["rule"].as_str().unwrap();
        let problems = result["problems"].as_array().unwrap();
        let rules: BTreeSet<Option<&str>> = problems.iter().map(|p| p["rule"].as_str()).collect();
        match case["check"].as_str().unwrap() {
            "schema" => assert!(
                rules.contains(&None),
                "invalid/{file} must fail the schema: {problems:#?}"
            ),
            _ => assert_eq!(
                rules,
                BTreeSet::from([Some(rule)]),
                "invalid/{file} must break only {rule}"
            ),
        }
    }
}

#[test]
fn check_writes_one_line_for_each_problem() {
    let path = conformance().join("invalid/duplicate-item-id.json");
    let out = skopos(&["check", path.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("duplicate-item-id.json: /items/"),
        "{stdout}"
    );
    assert!(stdout.contains("[unique-item-ids]"), "{stdout}");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("1 problem in 1 of 1 file."), "{stderr}");
}

#[test]
fn check_reports_a_file_that_it_cannot_read() {
    let dir = temp_dir("unreadable");
    let bad = dir.join("broken.json");
    fs::write(&bad, "{ not json").unwrap();
    let txt = dir.join("intent.txt");
    fs::write(&txt, "# Intent\n").unwrap();
    let out = skopos(&[
        "check",
        "--json",
        bad.to_str().unwrap(),
        txt.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(1));
    let results = stdout_json(&out);
    assert!(results[0]["problems"][0]["message"]
        .as_str()
        .unwrap()
        .contains("not valid JSON"));
    assert!(results[1]["problems"][0]["message"]
        .as_str()
        .unwrap()
        .contains(".md"));
}

#[test]
fn convert_to_yaml_and_back_keeps_the_document() {
    let dir = temp_dir("convert");
    let source = conformance().join("valid/spec-all-types-v3.json");
    let yaml = dir.join("spec.skopos.yaml");
    let json_out = dir.join("spec.skopos.json");
    assert!(skopos(&[
        "convert",
        "--to",
        "yaml",
        source.to_str().unwrap(),
        "-o",
        yaml.to_str().unwrap()
    ])
    .status
    .success());
    let out = skopos(&["convert", "--to", "json", yaml.to_str().unwrap()]);
    assert!(out.status.success());
    fs::write(&json_out, &out.stdout).unwrap();
    let original: Value = serde_json::from_str(&fs::read_to_string(&source).unwrap()).unwrap();
    assert_eq!(stdout_json(&out), original);
    assert!(
        skopos(&["check", yaml.to_str().unwrap(), json_out.to_str().unwrap()])
            .status
            .success()
    );
}

#[test]
fn fmt_writes_the_canonical_layout() {
    let dir = temp_dir("fmt");
    let file = dir.join("doc.skopos.json");
    fs::write(
        &file,
        r#"{"items":[],"title":"Short","kind":"intent","version":1,"id":"pa_7Hk2mQ9xR4tV8wZ1","status":"draft","created_at":"2026-11-05T09:00:00Z","$schema":"https://openskopos.org/schema/artifact/0.1.json"}"#,
    )
    .unwrap();
    let path = file.to_str().unwrap();
    let out = skopos(&["fmt", "--check", path]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stdout).contains("doc.skopos.json"));

    assert!(skopos(&["fmt", path]).status.success());
    let text = fs::read_to_string(&file).unwrap();
    assert!(
        text.starts_with(
            "{\n  \"$schema\": \"https://openskopos.org/schema/artifact/0.1.json\",\n  \"id\": "
        ),
        "{text}"
    );
    assert!(
        text.ends_with("\"created_at\": \"2026-11-05T09:00:00Z\"\n}\n"),
        "{text}"
    );
    assert!(skopos(&["fmt", "--check", path]).status.success());
}

#[test]
fn ids_gives_new_items_ids_from_the_counters() {
    let dir = temp_dir("ids");
    let file = dir.join("intent.skopos.yaml");
    fs::write(
        &file,
        "$schema: https://openskopos.org/schema/artifact/0.1.json\n\
         id: pa_7Hk2mQ9xR4tV8wZ1\nversion: 1\nkind: intent\ntitle: Downgrades\nstatus: draft\n\
         id_counters:\n  G: 3\n\
         items:\n\
         - id: G2\n  type: goal\n  text: Customers can lower their bill.\n  status: accepted\n\
         - type: goal\n  text: Support spends less time on downgrades.\n  status: proposed\n\
         - type: constraint\n  text: Ask before adding a migration.\n  status: proposed\n  modality: ask_first\n  approver: Tech lead\n\
         created_at: '2026-11-05T09:00:00Z'\n",
    )
    .unwrap();
    let path = file.to_str().unwrap();

    let out = skopos(&["check", path]);
    assert_eq!(out.status.code(), Some(1), "items with no ID are not valid");
    let out = skopos(&["ids", "--check", path]);
    assert_eq!(out.status.code(), Some(1));

    let out = skopos(&["ids", path]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("/items/1 gets G4, /items/2 gets C1, id_counters changes"),
        "{stdout}"
    );
    let text = fs::read_to_string(&file).unwrap();
    assert!(text.contains("- id: G4\n  type: goal\n"), "{text}");
    assert!(text.contains("id_counters:\n  G: 4\n  C: 1\n"), "{text}");
    assert!(skopos(&["check", path]).status.success());
    assert!(skopos(&["ids", "--check", path]).status.success());
}

#[test]
fn ids_rejects_an_item_with_no_type() {
    let dir = temp_dir("ids-no-type");
    let file = dir.join("doc.json");
    let before = r#"{"items": [{"text": "No type"}]}"#;
    fs::write(&file, before).unwrap();
    let out = skopos(&["ids", file.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("/items/0/type"));
    assert_eq!(fs::read_to_string(&file).unwrap(), before);
}

/// The files of the Markdown cases in one group of `conformance/markdown/`.
fn markdown_files(group: &str) -> Vec<String> {
    let text = fs::read_to_string(conformance().join("markdown/index.json")).unwrap();
    let index: Value = serde_json::from_str(&text).unwrap();
    index[group]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            let dir = conformance()
                .join("markdown")
                .join(group)
                .join(c["case"].as_str().unwrap());
            dir.join(c["file"].as_str().unwrap()).display().to_string()
        })
        .collect()
}

#[test]
fn check_reads_markdown() {
    let files = markdown_files("generous");
    let mut args = vec!["check"];
    args.extend(files.iter().map(String::as_str));
    let out = skopos(&args);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    // The warnings go to standard error, with their lines.
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("SPEC.md: warning: line 16: "), "{stderr}");

    for file in markdown_files("invalid") {
        let out = skopos(&["check", "--json", &file]);
        assert_eq!(out.status.code(), Some(1), "{file}");
        let problems = &stdout_json(&out)[0]["problems"];
        assert!(!problems.as_array().unwrap().is_empty(), "{file}");
    }
}

#[test]
fn convert_to_markdown_and_back_keeps_the_document() {
    let dir = temp_dir("convert-md");
    let source = conformance().join("valid/spec-all-types-v3.json");
    let md = dir.join("SPEC.md");
    assert!(skopos(&[
        "convert",
        "--to",
        "md",
        source.to_str().unwrap(),
        "-o",
        md.to_str().unwrap()
    ])
    .status
    .success());
    let out = skopos(&["convert", "--to", "json", md.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let original: Value = serde_json::from_str(&fs::read_to_string(&source).unwrap()).unwrap();
    // Markdown keeps the order of items within a type, and every field.
    let by_id = |d: &Value| {
        let mut items = d["items"].as_array().unwrap().clone();
        items.sort_by_key(|i| i["id"].as_str().unwrap().to_string());
        items
    };
    let back = stdout_json(&out);
    assert_eq!(by_id(&back), by_id(&original));
    let mut a = back.clone();
    let mut b = original.clone();
    a.as_object_mut().unwrap().remove("items");
    b.as_object_mut().unwrap().remove("items");
    assert_eq!(a, b);
}

#[test]
fn convert_to_markdown_copies_material_from_inputs() {
    let case = conformance().join("markdown/round-trip/03-copied-material");
    let input = case.join("inputs/pa_3fQ8nV2kLm7Xc1Rt@1.json");
    let out = skopos(&[
        "convert",
        "--to",
        "markdown",
        case.join("document.json").to_str().unwrap(),
        "--input",
        input.to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("\n## From other artifacts\n"), "{text}");
    assert!(text.contains("\n- pa_3fQ8nV2kLm7Xc1Rt@1#CR1: "), "{text}");
}

#[test]
fn import_stores_the_next_version() {
    let dir = temp_dir("import");
    for (case, version, message) in [
        ("01-edit-final-version", 3, "new draft version 3"),
        ("02-unchanged-final-version", 2, "no change"),
        ("03-edit-draft", 3, "replaces draft version 3"),
    ] {
        let case_dir = conformance().join("markdown/next-version").join(case);
        let output = dir.join(format!("{case}.json"));
        let out = skopos(&[
            "import",
            case_dir.join("edited.md").to_str().unwrap(),
            "--stored",
            case_dir.join("stored.json").to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{case}: {stderr}");
        assert!(stderr.contains(message), "{case}: {stderr}");
        let got: Value = serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
        assert_eq!(got["version"], version, "{case}");
        assert!(skopos(&["check", output.to_str().unwrap()])
            .status
            .success());
    }
}

#[test]
fn fmt_and_ids_do_not_take_markdown() {
    let file = conformance().join("markdown/generous/01-start-from-scratch/INTENT.md");
    for command in ["fmt", "ids"] {
        let out = skopos(&[command, file.to_str().unwrap()]);
        assert_eq!(out.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&out.stderr).contains("skopos convert"));
    }
}
