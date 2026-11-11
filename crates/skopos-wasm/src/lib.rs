//! The WebAssembly build of `skopos-core` and of the `skopos` commands.
//!
//! The npm package `@openskopos/core` holds this build. Each public function
//! of `skopos-core` has an export here. Inputs and outputs are strings or
//! plain JavaScript objects. A document goes in and out as the object that
//! `JSON.parse` gives. Problems, warnings and errors are plain objects with
//! the field names of the Rust types, in snake_case.
//!
//! A function that cannot give a result throws a JavaScript `Error` with the
//! name `SkoposError`. The error has the problems (`problems`) or the
//! Markdown errors (`errors`) as plain objects.
//!
//! `runCli` runs the commands of the `skopos` tool. The caller gives a host
//! object that reads and writes files and the terminal. The npm package
//! `@openskopos/cli` gives a host for Node.
//!
//! If the Rust code panics, the call throws a JavaScript `Error` with the
//! panic message. The module is then in an unknown state: load it again
//! before the next call.
//!
//! The crate is empty on other targets, so that native builds of the
//! workspace do not compile it.

#![cfg(target_arch = "wasm32")]

use std::io;
use std::path::Path;

use js_sys::{Array, Reflect, Uint8Array};
use serde::Serialize;
use serde_json::Value;
use skopos_core::export::{self, Options, Tool};
use skopos_core::markdown::{self, Fill, Warning};
use skopos_core::{AssignedId, Format, Problem};
use wasm_bindgen::prelude::*;

/// Runs when the module loads. A panic then throws a JavaScript `Error` with
/// the panic message, not only `RuntimeError: unreachable`.
#[wasm_bindgen(start)]
fn start() {
    std::panic::set_hook(Box::new(|info| {
        wasm_bindgen::throw_str(&format!(
            "skopos panicked: {info}. Load the module again before the next call."
        ));
    }));
}

#[wasm_bindgen(typescript_custom_section)]
const TYPES: &str = r#"
/** A file format for `parse` and `write`. */
export type Format = "json" | "yaml";

/** A tool for `exportTo`. */
export type Tool = "openspec" | "kiro" | "speckit";

/** An item type (specification section 4). */
export type ItemType =
  | "goal" | "success_signal" | "requirement" | "story" | "constraint"
  | "assumption" | "non_goal" | "risk" | "open_question" | "decision"
  | "term" | "critique";

/** One item of a document. Other fields depend on the type. */
export interface Item {
  id?: string;
  type?: ItemType | (string & {});
  text?: string;
  status?: "proposed" | "accepted" | "rejected" | (string & {});
  [key: string]: unknown;
}

/**
 * A document, as `JSON.parse` gives it. The JSON Schema (`schema()`) and
 * `check()` say what a valid document is.
 */
export interface Document {
  $schema?: string;
  id?: string;
  version?: number;
  kind?: string;
  title?: string;
  summary?: string;
  status?: "draft" | "final" | (string & {});
  items?: Item[];
  id_counters?: Record<string, number>;
  created_at?: string;
  finalized_at?: string;
  [key: string]: unknown;
}

/** One problem in a document. */
export interface Problem {
  /** Where the problem is, as a JSON Pointer. "" is the whole document. */
  path: string;
  /** The name of the rule of specification section 9, or null for a schema problem. */
  rule: string | null;
  /** What is wrong, in words for people. */
  message: string;
}

/** A thing that the Markdown reader skipped or changed (specification 10.6). */
export interface Warning {
  line: number;
  message: string;
}

/** A reason why the Markdown reader cannot make a valid document (specification 10.6). */
export interface MarkdownError {
  /** The line in the file, or null if the error is about the whole file. */
  line: number | null;
  message: string;
  /** For an error from the schema or the rules of section 9: the problem. */
  problem?: Problem;
}

/**
 * The error that a function throws when it cannot give a result. `problems`
 * or `errors` has the details.
 */
export interface SkoposError extends Error {
  name: "SkoposError";
  problems?: Problem[];
  errors?: MarkdownError[];
}

/** An ID that `assignIds` gave to an item. */
export interface AssignedId {
  /** The position of the item in `items`. */
  index: number;
  id: string;
}

/** The result of `assignIds`. */
export interface AssignIdsResult {
  /** The document with the new IDs and counters. */
  doc: Document;
  assigned: AssignedId[];
  counters_changed: boolean;
}

/** Options for `markdownRead`. */
export interface MarkdownReadOptions {
  /** The name of the file. `INTENT.md` gives the kind `intent` if the front matter has none. */
  filename?: string;
  /** The time for `created_at` if the file has none. Default: now. */
  now?: string;
  /** The artifact ID if the file has none. Default: a new random ID. */
  artifact_id?: string;
}

/** The result of `markdownRead`. */
export interface MarkdownReadResult {
  doc: Document;
  warnings: Warning[];
}

/** One file that an exporter writes. */
export interface ExportFile {
  /** The path from the root of the project, with "/" between folders. */
  path: string;
  text: string;
}

/** Options for `exportTo` and `exportName`. */
export interface ExportOptions {
  /** The name of the change or feature. Default: a name from the title. */
  name?: string;
  /** Spec Kit only: the feature number. Default: 1. */
  number?: number;
  /**
   * OpenSpec only: files of the project that the exporter reads, with paths
   * from the root of the project: the main specs and the files of the open
   * changes.
   */
  existing?: ExportFile[];
}

/** An item that an export leaves out, and why. */
export interface NotExported {
  reference: string;
  id: string;
  type: string;
  reason: string;
}

/** The result of `exportTo`. */
export interface Export {
  tool: Tool;
  folder: string;
  files: ExportFile[];
  not_exported: NotExported[];
  owned: string[];
  warnings: string[];
}

export interface VersionRef { artifact: string; version: number | null; }
export interface ItemSummary { id: string; type: string; text: string; }
export interface ChangedItem extends ItemSummary {
  fields: string[];
  status_from?: string | null;
  status_to?: string | null;
}
export interface SupersededItem extends ItemSummary { by: string[]; }

/** The result of `diff`. */
export interface Diff {
  from: VersionRef;
  to: VersionRef;
  same_artifact: boolean;
  envelope: string[];
  added: ItemSummary[];
  changed: ChangedItem[];
  dropped: ItemSummary[];
  superseded: SupersededItem[];
}

/**
 * The file system and terminal for `runCli`. Each method does one simple
 * operation. A method that fails throws, as the `node:fs` functions do.
 */
export interface CliHost {
  readFile(path: string): Uint8Array;
  writeFile(path: string, text: string): void;
  exists(path: string): boolean;
  isDir(path: string): boolean;
  readDir(path: string): string[];
  createDirAll(path: string): void;
  removeFile(path: string): void;
  removeDir(path: string): void;
  stdout(text: string): void;
  stderr(text: string): void;
  /** True if text for standard error (true) or standard output (false) can have colors. */
  color(stderr: boolean): boolean;
  /**
   * The platform, as `process.platform` gives it, for example "linux" or
   * "darwin". On Linux and macOS, file errors get the same messages as in
   * the native tool. Without this method, they keep the message of the host.
   */
  platform?(): string;
}
"#;

// ------------------------------------------------------------------ values
//
// Values cross the boundary as JSON text: JSON.stringify and serde_json on
// the way in, serde_json and JSON.parse on the way out. A conversion value by
// value (serde-wasm-bindgen) makes a call across the boundary for each field,
// which is slower for documents with many items than the native JSON parsers
// of the engine. JSON also gives the rules that JavaScript users know: a key
// with the value `undefined` is left out, and `toJSON` applies.

/// A JavaScript value as JSON.
fn from_js(value: &JsValue, what: &str) -> Result<Value, JsValue> {
    if value.is_undefined() {
        return Err(type_error(&format!("{what} is undefined")));
    }
    let text = js_sys::JSON::stringify(value)?;
    let text = text
        .as_string()
        .ok_or_else(|| type_error(&format!("{what} cannot be written as JSON")))?;
    serde_json::from_str(&text).map_err(|e| type_error(&format!("{what}: {e}")))
}

/// An optional JavaScript value as JSON: `undefined` and `null` give `None`.
fn from_js_opt(value: &Option<JsValue>, what: &str) -> Result<Option<Value>, JsValue> {
    match value {
        Some(v) if !v.is_undefined() && !v.is_null() => from_js(v, what).map(Some),
        _ => Ok(None),
    }
}

/// A Rust value as a plain JavaScript object.
fn to_js<T: Serialize + ?Sized>(value: &T) -> JsValue {
    let text = serde_json::to_string(value).expect("values serialize");
    js_sys::JSON::parse(&text).expect("JSON from serde_json parses")
}

fn type_error(message: &str) -> JsValue {
    js_sys::TypeError::new(message).into()
}

/// A `SkoposError` with a list of details under `key`.
fn skopos_error<T: Serialize>(message: &str, key: &str, details: &[T]) -> JsValue {
    let e = js_sys::Error::new(message);
    e.set_name("SkoposError");
    let _ = Reflect::set(&e, &JsValue::from_str(key), &to_js(details));
    e.into()
}

fn problems_error(problems: &[Problem]) -> JsValue {
    let message = problems
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    skopos_error(&message, "problems", problems)
}

fn format_of(format: &str) -> Result<Format, JsValue> {
    match format {
        "json" => Ok(Format::Json),
        "yaml" => Ok(Format::Yaml),
        _ => Err(type_error(&format!(
            "unknown format {format:?}: use json or yaml"
        ))),
    }
}

/// The result of `assignIds`.
#[derive(Serialize)]
struct AssignIdsResult<'a> {
    doc: &'a Value,
    assigned: &'a [AssignedId],
    counters_changed: bool,
}

/// The result of `markdownRead`.
#[derive(Serialize)]
struct MarkdownReadResult<'a> {
    doc: &'a Value,
    warnings: &'a [Warning],
}

/// The result of `parseItemId`.
#[derive(Serialize)]
struct ItemId {
    #[serde(rename = "type")]
    item_type: &'static str,
    number: u64,
}

/// A string field of an options object, if it is there.
fn option_string(options: &Option<Value>, key: &str) -> Option<String> {
    options.as_ref()?.get(key)?.as_str().map(str::to_string)
}

// ------------------------------------------------------------------ fill

/// The current time in UTC, to the second: `2026-11-10T12:00:00Z`.
fn now() -> String {
    let iso = String::from(js_sys::Date::new_0().to_iso_string());
    // toISOString gives 2026-11-10T12:00:00.000Z.
    format!("{}Z", iso.get(..19).unwrap_or(&iso))
}

/// A new artifact ID: `pa_` and 16 random letters and digits.
#[wasm_bindgen(js_name = newArtifactId)]
pub fn new_artifact_id() -> String {
    const CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mut id = String::from("pa_");
    while id.len() < 19 {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).expect("the JavaScript host gives random numbers");
        // 248 is 4 × 62. Bytes above it would make some letters more likely.
        for b in bytes.into_iter().filter(|b| *b < 248) {
            if id.len() < 19 {
                id.push(CHARS[usize::from(b % 62)] as char);
            }
        }
    }
    id
}

fn default_fill() -> Fill {
    Fill {
        now: now(),
        artifact_id: new_artifact_id(),
    }
}

// ------------------------------------------------------------------ core

/// Finds every problem in a document: schema problems first, then the
/// problems with the rules of specification section 9. An empty list means
/// that the document is valid.
#[wasm_bindgen(unchecked_return_type = "Problem[]")]
pub fn check(
    #[wasm_bindgen(unchecked_param_type = "unknown")] doc: JsValue,
) -> Result<JsValue, JsValue> {
    Ok(to_js(&skopos_core::check(&from_js(&doc, "doc")?)))
}

/// Finds the problems of a document with the JSON Schema only.
#[wasm_bindgen(js_name = checkSchema, unchecked_return_type = "Problem[]")]
pub fn check_schema(
    #[wasm_bindgen(unchecked_param_type = "unknown")] doc: JsValue,
) -> Result<JsValue, JsValue> {
    Ok(to_js(&skopos_core::check_schema(&from_js(&doc, "doc")?)))
}

/// Finds the problems of a document with the rules of specification section
/// 9 only.
#[wasm_bindgen(js_name = checkRules, unchecked_return_type = "Problem[]")]
pub fn check_rules(
    #[wasm_bindgen(unchecked_param_type = "unknown")] doc: JsValue,
) -> Result<JsValue, JsValue> {
    Ok(to_js(&skopos_core::check_rules(&from_js(&doc, "doc")?)))
}

/// Reads JSON or YAML text into a document. Does not check the document.
/// Throws a `SkoposError` with one problem if the text is not JSON or YAML.
#[wasm_bindgen(unchecked_return_type = "Document")]
pub fn parse(
    text: &str,
    #[wasm_bindgen(unchecked_param_type = "Format")] format: &str,
) -> Result<JsValue, JsValue> {
    match skopos_core::parse(text, format_of(format)?) {
        Ok(doc) => Ok(to_js(&doc)),
        Err(problem) => Err(problems_error(&[problem])),
    }
}

/// Writes a document as JSON or YAML in the canonical key order and layout,
/// with a newline at the end.
#[wasm_bindgen]
pub fn write(
    #[wasm_bindgen(unchecked_param_type = "Document")] doc: JsValue,
    #[wasm_bindgen(unchecked_param_type = "Format")] format: &str,
) -> Result<String, JsValue> {
    let format = format_of(format)?;
    Ok(skopos_core::write(&from_js(&doc, "doc")?, format))
}

/// `write(doc, "json")`.
#[wasm_bindgen(js_name = toJson)]
pub fn to_json(
    #[wasm_bindgen(unchecked_param_type = "Document")] doc: JsValue,
) -> Result<String, JsValue> {
    Ok(skopos_core::to_json(&from_js(&doc, "doc")?))
}

/// `write(doc, "yaml")`.
#[wasm_bindgen(js_name = toYaml)]
pub fn to_yaml(
    #[wasm_bindgen(unchecked_param_type = "Document")] doc: JsValue,
) -> Result<String, JsValue> {
    Ok(skopos_core::to_yaml(&from_js(&doc, "doc")?))
}

/// Gives an ID to each item that has none, and updates `id_counters`
/// (specification section 7). Returns a new document and does not change the
/// one you give. Throws a `SkoposError` with the problems if an item that
/// needs an ID has no known type.
#[wasm_bindgen(js_name = assignIds, unchecked_return_type = "AssignIdsResult")]
pub fn assign_ids(
    #[wasm_bindgen(unchecked_param_type = "Document")] doc: JsValue,
) -> Result<JsValue, JsValue> {
    let mut doc = from_js(&doc, "doc")?;
    match skopos_core::assign_ids(&mut doc) {
        Ok(report) => Ok(to_js(&AssignIdsResult {
            doc: &doc,
            assigned: &report.assigned,
            counters_changed: report.counters_changed,
        })),
        Err(problems) => Err(problems_error(&problems)),
    }
}

/// The type and number of an item ID, for example `FR4` gives
/// `{ type: "requirement", number: 4 }`. `undefined` if the ID is not valid.
#[wasm_bindgen(
    js_name = parseItemId,
    unchecked_return_type = "{ type: ItemType; number: number } | undefined"
)]
pub fn parse_item_id(id: &str) -> JsValue {
    match skopos_core::parse_item_id(id) {
        Some((item_type, number)) => to_js(&ItemId {
            item_type: item_type.as_str(),
            number,
        }),
        None => JsValue::UNDEFINED,
    }
}

/// The JSON Schema of format 0.1, as text.
#[wasm_bindgen]
pub fn schema() -> String {
    skopos_core::SCHEMA.to_string()
}

/// The URL of the JSON Schema of format 0.1.
#[wasm_bindgen(js_name = schemaUrl)]
pub fn schema_url() -> String {
    skopos_core::SCHEMA_URL.to_string()
}

/// The names of the rules of specification section 9.
#[wasm_bindgen]
pub fn rules() -> Vec<String> {
    skopos_core::rules::ALL
        .iter()
        .map(|r| r.to_string())
        .collect()
}

/// The version of this package.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ------------------------------------------------------------------ Markdown

/// Writes a document as Markdown (specification 10.1 to 10.4). `inputs` are
/// other documents to copy material from, such as pinned glossaries (10.7).
#[wasm_bindgen(js_name = markdownWrite)]
pub fn markdown_write(
    #[wasm_bindgen(unchecked_param_type = "Document")] doc: JsValue,
    #[wasm_bindgen(unchecked_param_type = "Document[]")] inputs: Option<JsValue>,
) -> Result<String, JsValue> {
    let doc = from_js(&doc, "doc")?;
    let inputs = match from_js_opt(&inputs, "inputs")? {
        None => vec![],
        Some(Value::Array(list)) => list,
        Some(_) => return Err(type_error("inputs must be a list of documents")),
    };
    Ok(markdown::write(&doc, &inputs))
}

/// Reads a Markdown file into a document (specification 10.5 and 10.6). The
/// document is valid. Throws a `SkoposError` with the errors (`errors`) if
/// the reader cannot make a valid document.
#[wasm_bindgen(js_name = markdownRead, unchecked_return_type = "MarkdownReadResult")]
pub fn markdown_read(
    md: &str,
    #[wasm_bindgen(unchecked_param_type = "MarkdownReadOptions")] options: Option<JsValue>,
) -> Result<JsValue, JsValue> {
    let options = from_js_opt(&options, "options")?;
    let fill = Fill {
        now: option_string(&options, "now").unwrap_or_else(now),
        artifact_id: option_string(&options, "artifact_id").unwrap_or_else(new_artifact_id),
    };
    let filename = option_string(&options, "filename");
    match markdown::read_with(md, filename.as_deref(), &fill) {
        Ok((doc, warnings)) => Ok(to_js(&MarkdownReadResult {
            doc: &doc,
            warnings: &warnings,
        })),
        Err(errors) => {
            let message = errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n");
            Err(skopos_error(&message, "errors", &errors))
        }
    }
}

/// The document to store after a person edits the Markdown of a stored
/// version (specification 10.8). `readDoc` comes from `markdownRead`. `now`
/// is the time for `created_at` of a new version. Default: now.
#[wasm_bindgen(js_name = nextVersion, unchecked_return_type = "Document")]
pub fn next_version(
    #[wasm_bindgen(unchecked_param_type = "Document")] stored: JsValue,
    #[wasm_bindgen(js_name = readDoc, unchecked_param_type = "Document")] read_doc: JsValue,
    #[wasm_bindgen(js_name = now)] now_time: Option<String>,
) -> Result<JsValue, JsValue> {
    let stored = from_js(&stored, "stored")?;
    let read_doc = from_js(&read_doc, "readDoc")?;
    let time = now_time.unwrap_or_else(now);
    Ok(to_js(&markdown::next_version_at(&stored, &read_doc, &time)))
}

/// True if two documents have the same content in the sense of
/// specification 10.8.
#[wasm_bindgen(js_name = sameContent)]
pub fn same_content(
    #[wasm_bindgen(unchecked_param_type = "Document")] a: JsValue,
    #[wasm_bindgen(unchecked_param_type = "Document")] b: JsValue,
) -> Result<bool, JsValue> {
    Ok(markdown::same_content(
        &from_js(&a, "a")?,
        &from_js(&b, "b")?,
    ))
}

// ------------------------------------------------------------------ export and diff

fn export_options(options: &Option<JsValue>) -> Result<Options, JsValue> {
    let options = from_js_opt(options, "options")?;
    let number = match options.as_ref().and_then(|o| o.get("number")) {
        None | Some(Value::Null) => None,
        Some(n) => Some(
            n.as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| type_error("options.number must be a whole number"))?,
        ),
    };
    let existing = match options.as_ref().and_then(|o| o.get("existing")) {
        None | Some(Value::Null) => vec![],
        Some(files) => serde_json::from_value(files.clone()).map_err(|e| {
            type_error(&format!(
                "options.existing must be a list of {{ path, text }}: {e}"
            ))
        })?,
    };
    Ok(Options {
        name: option_string(&options, "name"),
        number,
        existing,
    })
}

/// Exports a document to the files of a spec-driven development tool. Does
/// not check the document: use `check` first. Throws a `SkoposError` if the
/// export cannot be made safely.
#[wasm_bindgen(js_name = exportTo, unchecked_return_type = "Export")]
pub fn export_to(
    #[wasm_bindgen(unchecked_param_type = "Document")] doc: JsValue,
    #[wasm_bindgen(unchecked_param_type = "Tool")] tool: &str,
    #[wasm_bindgen(unchecked_param_type = "ExportOptions")] options: Option<JsValue>,
) -> Result<JsValue, JsValue> {
    let doc = from_js(&doc, "doc")?;
    let tool: Tool = tool
        .parse()
        .map_err(|e: export::UnknownTool| type_error(&e.to_string()))?;
    let options = export_options(&options)?;
    match export::export(&doc, tool, &options) {
        Ok(out) => Ok(to_js(&out)),
        Err(e) => {
            let err = js_sys::Error::new(&e.message);
            err.set_name("SkoposError");
            Err(err.into())
        }
    }
}

/// The name that `exportTo` uses for the change or feature folder.
#[wasm_bindgen(js_name = exportName)]
pub fn export_name(
    #[wasm_bindgen(unchecked_param_type = "Document")] doc: JsValue,
    #[wasm_bindgen(unchecked_param_type = "ExportOptions")] options: Option<JsValue>,
) -> Result<String, JsValue> {
    Ok(export::name(
        &from_js(&doc, "doc")?,
        &export_options(&options)?,
    ))
}

/// The items that were added, changed, dropped and superseded between two
/// versions, by item ID.
#[wasm_bindgen(unchecked_return_type = "Diff")]
pub fn diff(
    #[wasm_bindgen(js_name = oldDoc, unchecked_param_type = "Document")] old_doc: JsValue,
    #[wasm_bindgen(js_name = newDoc, unchecked_param_type = "Document")] new_doc: JsValue,
) -> Result<JsValue, JsValue> {
    let d = skopos_core::diff::diff(&from_js(&old_doc, "oldDoc")?, &from_js(&new_doc, "newDoc")?);
    Ok(to_js(&d))
}

/// `diff(old, new)` as text for people, as `skopos diff` writes it.
#[wasm_bindgen(js_name = diffText)]
pub fn diff_text(
    #[wasm_bindgen(js_name = oldDoc, unchecked_param_type = "Document")] old_doc: JsValue,
    #[wasm_bindgen(js_name = newDoc, unchecked_param_type = "Document")] new_doc: JsValue,
) -> Result<String, JsValue> {
    let d = skopos_core::diff::diff(&from_js(&old_doc, "oldDoc")?, &from_js(&new_doc, "newDoc")?);
    Ok(d.to_text())
}

// ------------------------------------------------------------------ CLI

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "CliHost")]
    pub type CliHost;

    #[wasm_bindgen(method, catch, js_name = readFile)]
    fn read_file(this: &CliHost, path: &str) -> Result<Uint8Array, JsValue>;
    #[wasm_bindgen(method, catch, js_name = writeFile)]
    fn write_file(this: &CliHost, path: &str, text: &str) -> Result<(), JsValue>;
    #[wasm_bindgen(method, catch)]
    fn exists(this: &CliHost, path: &str) -> Result<bool, JsValue>;
    #[wasm_bindgen(method, catch, js_name = isDir)]
    fn is_dir(this: &CliHost, path: &str) -> Result<bool, JsValue>;
    #[wasm_bindgen(method, catch, js_name = readDir)]
    fn read_dir(this: &CliHost, path: &str) -> Result<Array, JsValue>;
    #[wasm_bindgen(method, catch, js_name = createDirAll)]
    fn create_dir_all(this: &CliHost, path: &str) -> Result<(), JsValue>;
    #[wasm_bindgen(method, catch, js_name = removeFile)]
    fn remove_file(this: &CliHost, path: &str) -> Result<(), JsValue>;
    #[wasm_bindgen(method, catch, js_name = removeDir)]
    fn remove_dir(this: &CliHost, path: &str) -> Result<(), JsValue>;
    #[wasm_bindgen(method, catch)]
    fn stdout(this: &CliHost, text: &str) -> Result<(), JsValue>;
    #[wasm_bindgen(method, catch)]
    fn stderr(this: &CliHost, text: &str) -> Result<(), JsValue>;
    #[wasm_bindgen(method, catch)]
    fn color(this: &CliHost, stderr: bool) -> Result<bool, JsValue>;
    #[wasm_bindgen(method, catch)]
    fn platform(this: &CliHost) -> Result<String, JsValue>;
}

/// The text of the common errors of the file system, as the C library of
/// Linux (`linux`) or macOS (`darwin`) gives it. The native tool shows these
/// texts, so the same error gives the same message in both tools.
fn strerror(code: &str, platform: &str) -> Option<&'static str> {
    Some(match (code, platform) {
        ("EPERM", _) => "Operation not permitted",
        ("ENOENT", _) => "No such file or directory",
        ("EIO", _) => "Input/output error",
        ("EACCES", _) => "Permission denied",
        ("EBUSY", "darwin") => "Resource busy",
        ("EBUSY", _) => "Device or resource busy",
        ("EEXIST", _) => "File exists",
        ("EXDEV", "darwin") => "Cross-device link",
        ("EXDEV", _) => "Invalid cross-device link",
        ("ENOTDIR", _) => "Not a directory",
        ("EISDIR", _) => "Is a directory",
        ("EINVAL", _) => "Invalid argument",
        ("ENFILE", _) => "Too many open files in system",
        ("EMFILE", _) => "Too many open files",
        ("ETXTBSY", _) => "Text file busy",
        ("EFBIG", _) => "File too large",
        ("ENOSPC", _) => "No space left on device",
        ("EROFS", _) => "Read-only file system",
        ("EPIPE", _) => "Broken pipe",
        ("ENAMETOOLONG", _) => "File name too long",
        ("ENOTEMPTY", _) => "Directory not empty",
        ("ELOOP", _) => "Too many levels of symbolic links",
        ("EDQUOT", "darwin") => "Disc quota exceeded",
        ("EDQUOT", _) => "Disk quota exceeded",
        _ => return None,
    })
}

/// A JavaScript error from a host method as an `io::Error`. On Linux and
/// macOS, a Node error gets the message that Rust gives for the same error,
/// for example `No such file or directory (os error 2)`. Other errors, and
/// all errors on other platforms, keep the message of Node.
fn io_error(err: JsValue, platform: &str) -> io::Error {
    let field = |key: &str| Reflect::get(&err, &JsValue::from_str(key)).ok();
    let code = field("code").and_then(|v| v.as_string());
    // Node gives the negative errno of the C library.
    let errno = field("errno").and_then(|v| v.as_f64()).map(f64::abs);
    let kind = match code.as_deref() {
        Some("ENOENT") => io::ErrorKind::NotFound,
        Some("EACCES" | "EPERM") => io::ErrorKind::PermissionDenied,
        Some("EEXIST") => io::ErrorKind::AlreadyExists,
        Some("EPIPE") => io::ErrorKind::BrokenPipe,
        _ => io::ErrorKind::Other,
    };
    let text = match platform {
        "linux" | "darwin" => code.as_deref().and_then(|c| strerror(c, platform)),
        _ => None,
    };
    let message = match (text, errno) {
        (Some(text), Some(n)) => format!("{text} (os error {n})"),
        _ => field("message")
            .and_then(|v| v.as_string())
            .or_else(|| err.as_string())
            .unwrap_or_else(|| "unknown error".to_string()),
    };
    io::Error::new(kind, message)
}

fn path_str(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// A host object from JavaScript, and its platform.
struct JsHost {
    js: CliHost,
    platform: String,
}

impl JsHost {
    fn error(&self, err: JsValue) -> io::Error {
        io_error(err, &self.platform)
    }
}

impl skopos::Host for JsHost {
    fn read(&mut self, path: &Path) -> io::Result<Vec<u8>> {
        match self.js.read_file(&path_str(path)) {
            Ok(bytes) => Ok(bytes.to_vec()),
            Err(e) => Err(self.error(e)),
        }
    }

    fn write(&mut self, path: &Path, text: &str) -> io::Result<()> {
        self.js
            .write_file(&path_str(path), text)
            .map_err(|e| self.error(e))
    }

    fn exists(&mut self, path: &Path) -> bool {
        self.js.exists(&path_str(path)).unwrap_or(false)
    }

    fn is_dir(&mut self, path: &Path) -> bool {
        self.js.is_dir(&path_str(path)).unwrap_or(false)
    }

    fn read_dir(&mut self, path: &Path) -> io::Result<Vec<String>> {
        match self.js.read_dir(&path_str(path)) {
            Ok(names) => Ok(names.iter().filter_map(|n| n.as_string()).collect()),
            Err(e) => Err(self.error(e)),
        }
    }

    fn create_dir_all(&mut self, path: &Path) -> io::Result<()> {
        self.js
            .create_dir_all(&path_str(path))
            .map_err(|e| self.error(e))
    }

    fn remove_file(&mut self, path: &Path) -> io::Result<()> {
        self.js
            .remove_file(&path_str(path))
            .map_err(|e| self.error(e))
    }

    fn remove_dir(&mut self, path: &Path) -> io::Result<()> {
        self.js
            .remove_dir(&path_str(path))
            .map_err(|e| self.error(e))
    }

    fn stdout(&mut self, text: &str) -> io::Result<()> {
        self.js.stdout(text).map_err(|e| self.error(e))
    }

    fn stderr(&mut self, text: &str) {
        let _ = self.js.stderr(text);
    }

    fn color(&mut self, stream: skopos::Stream) -> bool {
        self.js
            .color(stream == skopos::Stream::Stderr)
            .unwrap_or(false)
    }

    fn fill(&mut self) -> Fill {
        default_fill()
    }
}

/// Runs the `skopos` command line tool with its arguments, without the name
/// of the program. The output, the files that it writes and the exit code
/// are the same as those of the native tool. Returns the exit code: 0 if all
/// went well, 1 if a command found a problem, 2 if the arguments are wrong.
#[wasm_bindgen(js_name = runCli)]
pub fn run_cli(args: Vec<String>, host: CliHost) -> u8 {
    let platform = host.platform().unwrap_or_default();
    let mut host = JsHost { js: host, platform };
    let argv = std::iter::once("skopos".to_string()).chain(args);
    skopos::run(argv, &mut host)
}
