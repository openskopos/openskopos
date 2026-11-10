//! Checks the OpenSkopos conformance cases.
//!
//! - Each document in `valid/` passes the schema and the rules in the
//!   specification that the schema cannot express.
//! - Each `schema` case in `invalid/` fails the schema.
//! - Each `semantic` case in `invalid/` passes the schema and breaks exactly
//!   the rule that its index entry names.
//! - The JSON files of the Markdown cases are valid, and each Markdown file
//!   of a stored document names it in the front matter. Reading and writing
//!   Markdown is for an implementation to test. This runner does not do it.
//!
//! Usage: `cargo run --manifest-path conformance/runner/Cargo.toml [repo root]`

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use jsonschema::Validator;
use serde_json::Value;

const SCHEMA_ID: &str = "https://openskopos.org/schema/artifact/0.1.json";
const SEMANTIC_RULES: [&str; 4] = [
    "unique-item-ids",
    "counters-cover-items",
    "previous-version-same-artifact",
    "previous-version-one-less",
];

struct Run {
    failures: Vec<String>,
    passed: usize,
}

impl Run {
    fn check(&mut self, ok: bool, what: String) {
        if ok {
            self.passed += 1;
        } else {
            println!("FAIL {what}");
            self.failures.push(what);
        }
    }
}

fn main() -> ExitCode {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."));
    match run(&root) {
        Ok(run) if run.failures.is_empty() => {
            println!("All {} checks passed.", run.passed);
            ExitCode::SUCCESS
        }
        Ok(run) => {
            println!("{} checks passed, {} failed.", run.passed, run.failures.len());
            ExitCode::FAILURE
        }
        Err(e) => {
            println!("ERROR {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(root: &Path) -> Result<Run, String> {
    let schema = read_json(&root.join("schema/artifact/0.1.json"))?;
    let mut run = Run { failures: vec![], passed: 0 };

    run.check(
        jsonschema::meta::is_valid(&schema),
        "schema/artifact/0.1.json is a valid JSON Schema".into(),
    );
    run.check(
        schema["$id"] == SCHEMA_ID,
        format!("schema $id is {SCHEMA_ID}"),
    );

    let validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|e| format!("cannot compile the schema: {e}"))?;

    let conf = root.join("conformance");
    check_valid(&mut run, &validator, &conf.join("valid"))?;
    check_invalid(&mut run, &validator, &conf.join("invalid"))?;
    check_markdown(&mut run, &validator, &conf.join("markdown"))?;
    Ok(run)
}

fn check_valid(run: &mut Run, v: &Validator, dir: &Path) -> Result<(), String> {
    let cases = index_cases(dir, "cases")?;
    let mut listed = BTreeSet::new();
    for case in &cases {
        let file = str_field(case, "file")?;
        listed.insert(file.to_string());
        let doc = read_doc(&dir.join(file))?;
        let schema_errors = schema_errors(v, &doc);
        let rules = semantic_errors(&doc);
        if !schema_errors.is_empty() {
            println!("  valid/{file}: {}", schema_errors.join("; "));
        }
        if !rules.is_empty() {
            println!("  valid/{file}: breaks {}", rules.join(", "));
        }
        run.check(
            schema_errors.is_empty() && rules.is_empty(),
            format!("valid/{file} is valid"),
        );
    }
    check_listed(run, dir, &listed, "valid")
}

fn check_invalid(run: &mut Run, v: &Validator, dir: &Path) -> Result<(), String> {
    let cases = index_cases(dir, "cases")?;
    let mut listed = BTreeSet::new();
    for case in &cases {
        let file = str_field(case, "file")?;
        let check = str_field(case, "check")?;
        let rule = str_field(case, "rule")?;
        str_field(case, "reason")?;
        listed.insert(file.to_string());
        let doc = read_doc(&dir.join(file))?;
        let schema_errors = schema_errors(v, &doc);
        match check {
            "schema" => run.check(
                !schema_errors.is_empty(),
                format!("invalid/{file} fails the schema ({rule})"),
            ),
            "semantic" => {
                run.check(
                    SEMANTIC_RULES.contains(&rule),
                    format!("invalid/{file} names a known rule ({rule})"),
                );
                if !schema_errors.is_empty() {
                    println!("  invalid/{file}: {}", schema_errors.join("; "));
                }
                let broken = semantic_errors(&doc);
                run.check(
                    schema_errors.is_empty() && broken == vec![rule.to_string()],
                    format!("invalid/{file} passes the schema and breaks only {rule} (breaks {broken:?})"),
                );
            }
            other => run.check(false, format!("invalid/{file}: unknown check {other:?}")),
        }
    }
    check_listed(run, dir, &listed, "invalid")
}

fn check_markdown(run: &mut Run, v: &Validator, dir: &Path) -> Result<(), String> {
    let index = read_json(&dir.join("index.json"))?;
    let valid = |j: &Value| schema_errors(v, j).is_empty() && semantic_errors(j).is_empty();

    // round-trip: document.json and its Markdown, document.md.
    let mut listed = BTreeSet::new();
    for case in array(&index, "round_trip")? {
        let name = str_field(case, "case")?;
        str_field(case, "description")?;
        listed.insert(name.to_string());
        let d = dir.join("round-trip").join(name);
        let doc = read_doc(&d.join("document.json"))?;
        run.check(valid(&doc), format!("markdown/round-trip/{name}/document.json is valid"));
        if d.join("inputs").is_dir() {
            for f in sorted_files(&d.join("inputs"))? {
                let input = read_doc(&d.join("inputs").join(&f))?;
                run.check(valid(&input), format!("markdown/round-trip/{name}/inputs/{f} is valid"));
            }
        }
        let fm = front_matter(&d.join("document.md"))?;
        run.check(
            same_version(&fm, &doc),
            format!("markdown/round-trip/{name}/document.md front matter names document.json"),
        );
    }
    check_listed_dirs(run, &dir.join("round-trip"), &listed, "markdown/round-trip")?;

    // generous: a hand-written file and the document that a reader makes from it.
    let mut listed = BTreeSet::new();
    for case in array(&index, "generous")? {
        let name = str_field(case, "case")?;
        let file = str_field(case, "file")?;
        str_field(case, "description")?;
        listed.insert(name.to_string());
        let d = dir.join("generous").join(name);
        run.check(d.join(file).is_file(), format!("markdown/generous/{name}/{file} exists"));
        let expected = read_doc(&d.join("expected.json"))?;
        run.check(valid(&expected), format!("markdown/generous/{name}/expected.json is valid"));
    }
    check_listed_dirs(run, &dir.join("generous"), &listed, "markdown/generous")?;

    // next-version: a stored version, an edited file, and what the tool stores.
    let mut listed = BTreeSet::new();
    for case in array(&index, "next_version")? {
        let name = str_field(case, "case")?;
        str_field(case, "description")?;
        listed.insert(name.to_string());
        let d = dir.join("next-version").join(name);
        let stored = read_doc(&d.join("stored.json"))?;
        let expected = read_doc(&d.join("expected.json"))?;
        run.check(valid(&stored), format!("markdown/next-version/{name}/stored.json is valid"));
        run.check(valid(&expected), format!("markdown/next-version/{name}/expected.json is valid"));
        let fm = front_matter(&d.join("edited.md"))?;
        run.check(
            same_version(&fm, &stored),
            format!("markdown/next-version/{name}/edited.md front matter names stored.json"),
        );
        let n = stored["version"].as_u64().unwrap_or(0);
        let next = expected["version"].as_u64().unwrap_or(0);
        let ok = expected["id"] == stored["id"]
            && if next == n + 1 {
                stored["status"] == "final" && expected["status"] == "draft"
            } else {
                next == n
            };
        run.check(
            ok,
            format!("markdown/next-version/{name}/expected.json is version {n}, or draft {} after a final version", n + 1),
        );
    }
    check_listed_dirs(run, &dir.join("next-version"), &listed, "markdown/next-version")?;

    // invalid: files that a reader must reject.
    let mut listed = BTreeSet::new();
    for case in array(&index, "invalid")? {
        let name = str_field(case, "case")?;
        let file = str_field(case, "file")?;
        str_field(case, "reason")?;
        listed.insert(name.to_string());
        let path = dir.join("invalid").join(name).join(file);
        run.check(path.is_file(), format!("markdown/invalid/{name}/{file} exists"));
    }
    check_listed_dirs(run, &dir.join("invalid"), &listed, "markdown/invalid")
}

/// The rules of the specification that JSON Schema cannot express, for one document.
fn semantic_errors(doc: &Value) -> Vec<String> {
    let mut broken = vec![];
    let items = doc["items"].as_array().cloned().unwrap_or_default();

    let mut seen = BTreeSet::new();
    let mut highest: BTreeMap<String, u64> = BTreeMap::new();
    let mut duplicate = false;
    for item in &items {
        let Some(id) = item["id"].as_str() else { continue };
        if !seen.insert(id.to_string()) {
            duplicate = true;
        }
        let split = id.find(|c: char| c.is_ascii_digit()).unwrap_or(id.len());
        let (prefix, number) = id.split_at(split);
        if let Ok(n) = number.parse::<u64>() {
            let h = highest.entry(prefix.to_string()).or_insert(0);
            *h = (*h).max(n);
        }
    }
    if duplicate {
        broken.push("unique-item-ids".to_string());
    }

    let counters = &doc["id_counters"];
    if highest
        .iter()
        .any(|(prefix, n)| counters[prefix].as_u64().unwrap_or(0) < *n)
    {
        broken.push("counters-cover-items".to_string());
    }

    for parent in doc["parents"].as_array().cloned().unwrap_or_default() {
        if parent["rel"] != "previous_version" {
            continue;
        }
        if parent["artifact"] != doc["id"] {
            broken.push("previous-version-same-artifact".to_string());
        }
        let version = doc["version"].as_u64().unwrap_or(0);
        if parent["version"].as_u64() != version.checked_sub(1) {
            broken.push("previous-version-one-less".to_string());
        }
    }
    broken
}

fn schema_errors(v: &Validator, doc: &Value) -> Vec<String> {
    v.iter_errors(doc)
        .map(|e| format!("{}: {}", e.instance_path, e))
        .collect()
}

fn same_version(fm: &Value, doc: &Value) -> bool {
    fm["schema"] == doc["$schema"]
        && fm["id"] == doc["id"]
        && fm["version"] == doc["version"]
        && fm["kind"] == doc["kind"]
}

fn front_matter(path: &Path) -> Result<Value, String> {
    let text = read(path)?;
    let rest = text
        .strip_prefix("---\n")
        .ok_or_else(|| format!("{}: no front matter", path.display()))?;
    let end = rest
        .find("\n---\n")
        .ok_or_else(|| format!("{}: front matter does not end", path.display()))?;
    serde_yaml_ng::from_str(&rest[..end]).map_err(|e| format!("{}: {e}", path.display()))
}

fn read_doc(path: &Path) -> Result<Value, String> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("yaml") | Some("yml") => {
            serde_yaml_ng::from_str(&read(path)?).map_err(|e| format!("{}: {e}", path.display()))
        }
        _ => read_json(path),
    }
}

fn read_json(path: &Path) -> Result<Value, String> {
    serde_json::from_str(&read(path)?).map_err(|e| format!("{}: {e}", path.display()))
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn index_cases(dir: &Path, key: &str) -> Result<Vec<Value>, String> {
    let index = read_json(&dir.join("index.json"))?;
    Ok(array(&index, key)?.to_vec())
}

fn array<'a>(v: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    v[key].as_array().ok_or_else(|| format!("index has no {key:?} list"))
}

fn str_field<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v[key]
        .as_str()
        .ok_or_else(|| format!("index entry has no {key:?}: {v}"))
}

fn sorted_files(dir: &Path) -> Result<Vec<String>, String> {
    let mut names = vec![];
    for entry in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.path().is_file() {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    names.sort();
    Ok(names)
}

fn check_listed(run: &mut Run, dir: &Path, listed: &BTreeSet<String>, label: &str) -> Result<(), String> {
    let on_disk: BTreeSet<String> = sorted_files(dir)?
        .into_iter()
        .filter(|f| f != "index.json")
        .collect();
    for f in on_disk.symmetric_difference(listed) {
        run.check(false, format!("{label}/{f} is on disk and in index.json"));
    }
    Ok(())
}

fn check_listed_dirs(run: &mut Run, dir: &Path, listed: &BTreeSet<String>, label: &str) -> Result<(), String> {
    let mut on_disk = BTreeSet::new();
    for entry in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.path().is_dir() {
            on_disk.insert(entry.file_name().to_string_lossy().into_owned());
        }
    }
    for f in on_disk.symmetric_difference(listed) {
        run.check(false, format!("{label}/{f} is on disk and in index.json"));
    }
    Ok(())
}
