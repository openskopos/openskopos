//! Read, check and write OpenSkopos documents.
//!
//! OpenSkopos is a file format for the decisions that a team makes before an
//! AI agent builds the work. This crate is the reference implementation of format 0.1.
//! See the specification in `spec/0.1/README.md` of the repository.
//!
//! All functions are pure: they take strings or [`serde_json::Value`] and
//! return strings or values. The crate reads no files and uses no network, so
//! it builds for `wasm32-unknown-unknown`.
//!
//! The main functions:
//!
//! - [`check`] finds every problem in a document: schema problems and the
//!   rules of specification section 9.
//! - [`parse`] reads JSON or YAML text into a [`Value`].
//! - [`write`] writes a [`Value`] as JSON or YAML in the canonical layout.
//!   JSON to YAML and back gives the same value.
//! - [`assign_ids`] gives an ID to each item that has none and updates
//!   `id_counters`.
//!
//! The [`markdown`] module reads and writes Markdown (specification section
//! 10). The [`model`] module has serde types for a valid document.
//!
//! ```
//! let text = r#"{
//!   "$schema": "https://openskopos.org/schema/artifact/0.1.json",
//!   "id": "pa_7Hk2mQ9xR4tV8wZ1",
//!   "version": 1,
//!   "kind": "intent",
//!   "title": "Self-serve plan downgrades",
//!   "status": "draft",
//!   "items": [],
//!   "created_at": "2026-11-05T09:00:00Z"
//! }"#;
//! let doc = skopos_core::parse(text, skopos_core::Format::Json).unwrap();
//! assert!(skopos_core::check(&doc).is_empty());
//! let yaml = skopos_core::write(&doc, skopos_core::Format::Yaml);
//! assert!(yaml.starts_with("$schema: https://openskopos.org/schema/artifact/0.1.json\n"));
//! ```

use std::fmt;

use serde::{Deserialize, Serialize};
pub use serde_json::Value;

pub mod diff;
pub mod export;
mod ids;
pub mod markdown;
pub mod model;
mod order;
pub mod rules;
mod schema;
mod text;
mod yaml;

pub use ids::{assign_ids, parse_item_id, AssignedId, IdReport};
pub use rules::check_rules;
pub use schema::{check_schema, SCHEMA, SCHEMA_URL};
pub use text::{parse, to_json, to_yaml, write, Format};

/// One problem in a document.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Problem {
    /// Where the problem is, as a JSON Pointer (RFC 6901). An empty string is
    /// the whole document. `/items/3/id` is the `id` of the fourth item.
    pub path: String,
    /// For a problem with a rule of specification section 9, the name of the
    /// rule, for example `unique-item-ids`. `None` for a schema problem or a
    /// problem in the text.
    pub rule: Option<String>,
    /// What is wrong, in words for people.
    pub message: String,
}

impl Problem {
    /// A problem with no rule name.
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Problem {
            path: path.into(),
            rule: None,
            message: message.into(),
        }
    }

    /// A problem that breaks a rule of specification section 9.
    pub fn rule(path: impl Into<String>, rule: &str, message: impl Into<String>) -> Self {
        Problem {
            path: path.into(),
            rule: Some(rule.to_string()),
            message: message.into(),
        }
    }
}

impl fmt::Display for Problem {
    /// `<path>: <message> [<rule>]`. The path and the rule are left out when
    /// they are empty.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.path.is_empty() {
            write!(f, "{}: ", self.path)?;
        }
        f.write_str(&self.message)?;
        if let Some(rule) = &self.rule {
            write!(f, " [{rule}]")?;
        }
        Ok(())
    }
}

impl std::error::Error for Problem {}

/// Finds every problem in a document.
///
/// The result has the schema problems first, then the problems with the
/// rules of specification section 9. An empty list means that the document is
/// valid.
///
/// One rule needs the earlier versions of the artifact: an ID that an earlier
/// version used for one item is never used for a different item. This
/// function sees one version only, so it cannot check that rule.
pub fn check(doc: &Value) -> Vec<Problem> {
    let mut problems = check_schema(doc);
    problems.extend(check_rules(doc));
    problems
}

/// A whole number of 0 or more. JSON Schema reads `3.0` as the integer 3, so
/// this function does too.
pub(crate) fn whole_number(value: &Value) -> Option<u64> {
    if let Some(n) = value.as_u64() {
        return Some(n);
    }
    let f = value.as_f64()?;
    // 2^64 as f64. Every whole f64 below it fits in a u64.
    (f >= 0.0 && f.fract() == 0.0 && f < 18_446_744_073_709_551_616.0).then_some(f as u64)
}

/// JSON Pointer of a key under a parent pointer.
pub(crate) fn pointer(parent: &str, key: impl fmt::Display) -> String {
    let key = key.to_string().replace('~', "~0").replace('/', "~1");
    format!("{parent}/{key}")
}

/// Text on one line: line breaks and runs of spaces become one space.
pub(crate) fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// An artifact and version, for example `pa_2Xc5Rv8Tn1Mb4Kz7@3`, or the
/// artifact ID alone if there is no version.
pub(crate) fn version_label(artifact: &str, version: Option<u64>) -> String {
    match version {
        Some(v) => format!("{artifact}@{v}"),
        None => artifact.to_string(),
    }
}

/// A reference string (specification section 6), for example
/// `pa_2Xc5Rv8Tn1Mb4Kz7@3#FR4`.
pub(crate) fn reference_string(artifact: &str, version: Option<u64>, item: &str) -> String {
    format!("{}#{item}", version_label(artifact, version))
}

/// The date part of an RFC 3339 date and time, for example `2026-11-08`.
pub(crate) fn date_of(timestamp: &str) -> &str {
    timestamp.get(..10).unwrap_or(timestamp)
}
