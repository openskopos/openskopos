//! Runs the conformance cases in `conformance/` through skopos-core
//! (specification section 13).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use skopos_core::{check, check_rules, check_schema, model::Document, parse, rules, write, Format};

fn conformance() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}

fn format_of(path: &Path) -> Format {
    match path.extension().and_then(|e| e.to_str()) {
        Some("yaml") | Some("yml") => Format::Yaml,
        _ => Format::Json,
    }
}

fn read(path: &Path) -> Value {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    parse(&text, format_of(path)).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn cases(dir: &str) -> Vec<Value> {
    read(&conformance().join(dir).join("index.json"))["cases"]
        .as_array()
        .expect("index has cases")
        .clone()
}

/// Every JSON or YAML document in the conformance folder.
fn all_documents() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if !path.ends_with("runner") {
                    walk(&path, out);
                }
            } else if matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("json" | "yaml" | "yml")
            ) && !path.ends_with("index.json")
            {
                out.push(path);
            }
        }
    }
    let mut out = vec![];
    walk(&conformance(), &mut out);
    out.sort();
    assert!(out.len() > 50, "found only {} documents", out.len());
    out
}

#[test]
fn valid_documents_have_no_problems() {
    for case in cases("valid") {
        let file = case["file"].as_str().unwrap();
        let problems = check(&read(&conformance().join("valid").join(file)));
        assert!(problems.is_empty(), "valid/{file}: {problems:#?}");
    }
}

#[test]
fn invalid_documents_fail_for_the_named_rule() {
    let mut semantic = BTreeSet::new();
    for case in cases("invalid") {
        let file = case["file"].as_str().unwrap();
        let rule = case["rule"].as_str().unwrap();
        let doc = read(&conformance().join("invalid").join(file));
        let problems = check(&doc);
        match case["check"].as_str().unwrap() {
            "schema" => {
                assert!(
                    problems.iter().any(|p| p.rule.is_none()),
                    "invalid/{file} must fail the schema ({rule}): {problems:#?}"
                );
                // One fault gives clear messages, with no noise about the
                // other fields of the item.
                for p in &problems {
                    assert!(!p.message.contains("Unevaluated"), "invalid/{file}: {p}");
                    assert!(p.message.len() < 400, "invalid/{file}: {p}");
                }
            }
            "semantic" => {
                assert!(
                    rules::ALL.contains(&rule),
                    "invalid/{file}: unknown rule {rule}"
                );
                semantic.insert(rule.to_string());
                let names: BTreeSet<_> = problems.iter().map(|p| p.rule.as_deref()).collect();
                assert_eq!(
                    names,
                    BTreeSet::from([Some(rule)]),
                    "invalid/{file} must break only {rule}: {problems:#?}"
                );
            }
            other => panic!("invalid/{file}: unknown check {other}"),
        }
    }
    assert_eq!(
        semantic,
        rules::ALL.map(String::from).into(),
        "each rule of section 9 has a case"
    );
}

#[test]
fn markdown_case_documents_are_valid() {
    for path in all_documents() {
        if path.components().any(|c| c.as_os_str() == "markdown") {
            let problems = check(&read(&path));
            assert!(problems.is_empty(), "{}: {problems:#?}", path.display());
        }
    }
}

#[test]
fn schema_and_rule_problems_are_separate() {
    for path in all_documents() {
        let doc = read(&path);
        assert!(check_schema(&doc).iter().all(|p| p.rule.is_none()));
        assert!(check_rules(&doc).iter().all(|p| p.rule.is_some()));
    }
}

#[test]
fn json_and_yaml_conversion_keeps_every_value() {
    for path in all_documents() {
        let doc = read(&path);
        let yaml = write(&doc, Format::Yaml);
        let from_yaml = parse(&yaml, Format::Yaml).unwrap();
        assert_eq!(from_yaml, doc, "{} through YAML:\n{yaml}", path.display());
        let json = write(&from_yaml, Format::Json);
        assert_eq!(
            parse(&json, Format::Json).unwrap(),
            doc,
            "{} through JSON",
            path.display()
        );
    }
}

#[test]
fn canonical_text_does_not_change_when_written_again() {
    for path in all_documents() {
        let doc = read(&path);
        for format in [Format::Json, Format::Yaml] {
            let once = write(&doc, format);
            let twice = write(&parse(&once, format).unwrap(), format);
            assert_eq!(once, twice, "{} as {format:?}", path.display());
        }
    }
}

#[test]
fn typed_model_keeps_every_valid_document() {
    for path in all_documents() {
        let doc = read(&path);
        if !check(&doc).is_empty() {
            continue;
        }
        let typed =
            Document::from_value(&doc).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(typed.to_value(), doc, "{}", path.display());
    }
}

#[test]
fn embedded_schema_is_the_published_file() {
    let file = fs::read_to_string(conformance().join("../schema/artifact/0.1.json")).unwrap();
    assert_eq!(
        skopos_core::SCHEMA,
        file,
        "copy schema/artifact/0.1.json to crates/skopos-core/schema/0.1.json"
    );
    let schema: Value = serde_json::from_str(&file).unwrap();
    assert_eq!(schema["$id"], skopos_core::SCHEMA_URL);
}

#[test]
fn rules_read_whole_number_floats_like_the_schema() {
    // JSON Schema reads 0.0 and 3.0 as integers, so the rules must too.
    let mut doc = read(&conformance().join("invalid/counter-below-highest-id.json"));
    let counter = doc["id_counters"]["S"].as_u64().unwrap();
    doc["id_counters"]["S"] = serde_json::json!(counter as f64);
    assert!(check_schema(&doc).is_empty());
    let names: Vec<_> = check(&doc).into_iter().filter_map(|p| p.rule).collect();
    assert_eq!(names, [rules::COUNTERS_COVER_ITEMS]);

    let mut doc = read(&conformance().join("invalid/previous-version-skips.json"));
    let version = doc["version"].as_u64().unwrap();
    doc["version"] = serde_json::json!(version as f64);
    assert!(check_schema(&doc).is_empty());
    let names: Vec<_> = check(&doc).into_iter().filter_map(|p| p.rule).collect();
    assert_eq!(names, [rules::PREVIOUS_VERSION_ONE_LESS]);
}

#[test]
fn parents_message_follows_the_schema_branch() {
    let mut doc = read(&conformance().join("invalid/v2-without-previous-version.json"));
    doc["version"] = serde_json::json!(2.0);
    let messages: Vec<_> = check_schema(&doc).into_iter().map(|p| p.message).collect();
    assert_eq!(
        messages,
        ["version 2 must have exactly one previous_version parent"]
    );
}
