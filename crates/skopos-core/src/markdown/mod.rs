//! Markdown (specification section 10).
//!
//! A Markdown file contains every field of a document, so a document can go to
//! Markdown and back with no loss. People can also write a file by hand, with
//! no other file, and the reader makes a document from it.
//!
//! - [`write`] writes a document as Markdown (10.1 to 10.4). It can copy
//!   material from other artifacts into the file (10.7).
//! - [`read`] reads a Markdown file into a document (10.5). It is generous: it
//!   accepts many ways to write the same thing. It returns warnings for things
//!   it skips, or errors if it cannot make a valid document (10.6).
//! - [`next_version`] gives the document that a tool stores when a person
//!   edits the Markdown of a stored version (10.8).
//!
//! ```
//! let md = "# Faster invoices\n\n## Goals\n\n- Finance sends invoices on day one.\n";
//! let (doc, warnings) = skopos_core::markdown::read(md, Some("INTENT.md")).unwrap();
//! assert!(warnings.is_empty());
//! assert_eq!(doc["kind"], "intent");
//! assert_eq!(doc["items"][0]["id"], "G1");
//! let text = skopos_core::markdown::write(&doc, &[]);
//! assert!(text.contains("- G1: Finance sends invoices on day one.\n"));
//! ```

use std::collections::HashMap;
use std::fmt;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::model::{
    Category, Confirmation, ItemStatus, ItemType, Level, Modality, Outcome, Priority, RefRel,
    Severity,
};
use crate::Problem;

mod read;
mod version;
mod write;

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub use read::read;
pub use read::read_with;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub use version::next_version;
pub use version::{next_version_at, same_content};
pub use write::write;

/// A case where the reader skips or changes something and continues (10.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Warning {
    /// The line in the file, counted from 1.
    pub line: usize,
    /// What the reader found and what it did.
    pub message: String,
}

impl fmt::Display for Warning {
    /// `line <n>: <message>`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

/// A case where the reader cannot make a valid document (10.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Error {
    /// The line in the file, counted from 1. `None` if the error is about the
    /// whole file.
    pub line: Option<usize>,
    /// What is wrong and what to write.
    pub message: String,
    /// For an error from the schema or the rules of section 9, the problem
    /// that [`crate::check`] found in the document. `message` is the message
    /// of the problem.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<Problem>,
}

impl Error {
    pub(crate) fn at(line: usize, message: impl Into<String>) -> Self {
        Error {
            line: Some(line),
            message: message.into(),
            problem: None,
        }
    }

    pub(crate) fn file(message: impl Into<String>) -> Self {
        Error {
            line: None,
            message: message.into(),
            problem: None,
        }
    }
}

impl fmt::Display for Error {
    /// `line <n>: <message>`. An error from [`crate::check`] also has the
    /// JSON Pointer and the rule of its problem.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(line) = self.line {
            write!(f, "line {line}: ")?;
        }
        match &self.problem {
            Some(p) => write!(f, "{p}"),
            None => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for Error {}

/// The values that the reader fills when the file does not give them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fill {
    /// The time for `created_at`, for example `2026-11-10T12:00:00Z`.
    pub now: String,
    /// The artifact ID for a file with no `id`, for example
    /// `pa_7Hk2mQ9xR4tV8wZ1`.
    pub artifact_id: String,
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
impl Fill {
    /// The current time and a new random artifact ID.
    ///
    /// This function is not in the WebAssembly build, which has no clock. Use
    /// [`read_with`] and [`next_version_at`] there.
    pub fn new() -> Self {
        Fill {
            now: clock::now(),
            artifact_id: clock::new_artifact_id(),
        }
    }
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
impl Default for Fill {
    fn default() -> Self {
        Fill::new()
    }
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
mod clock {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    /// The current time in UTC, to the second: `2026-11-10T12:00:00Z`.
    pub(super) fn now() -> String {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let (days, rest) = (secs / 86_400, secs % 86_400);
        let (y, m, d) = civil_from_days(days as i64);
        format!(
            "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
            rest / 3600,
            rest % 3600 / 60,
            rest % 60
        )
    }

    /// The date of a day after 1970-01-01 (Howard Hinnant's algorithm).
    fn civil_from_days(z: i64) -> (i64, u32, u32) {
        let z = z + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let y = yoe + era * 400 + i64::from(m <= 2);
        (y, m, d)
    }

    /// `pa_` and 16 random letters and digits. The standard library seeds
    /// each `RandomState` from the operating system.
    pub(super) fn new_artifact_id() -> String {
        const CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
        let mut id = String::from("pa_");
        let mut bits = 0u64;
        for n in 0..16 {
            if n % 8 == 0 {
                let mut h = RandomState::new().build_hasher();
                h.write_u128(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_or(0, |d| d.as_nanos()),
                );
                bits = h.finish();
            }
            id.push(CHARS[(bits % 62) as usize] as char);
            bits /= 62;
        }
        id
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn dates() {
            assert_eq!(super::civil_from_days(0), (1970, 1, 1));
            assert_eq!(super::civil_from_days(20_767), (2026, 11, 10));
            assert_eq!(super::civil_from_days(11_016), (2000, 2, 29));
        }

        #[test]
        fn artifact_ids_match_the_schema() {
            let id = super::new_artifact_id();
            assert_eq!(id.len(), 19);
            assert!(id[3..].chars().all(|c| c.is_ascii_alphanumeric()));
            assert_ne!(id, super::new_artifact_id());
        }
    }
}

// ------------------------------------------------------------------ tables

pub(crate) const SCHEMA_KEY: &str = "$schema";

/// The H2 that contains material from other artifacts (10.7).
pub(crate) const OTHER: &str = "From other artifacts";

/// Envelope keys in the order of specification section 3.
pub(crate) const ENVELOPE: &[&str] = crate::order::DOCUMENT;

/// How a field of an item type is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Text,
    Enum,
    Bool,
    List,
    Examples,
    Target,
    Resolution,
}

pub(crate) fn kind(field: &str) -> Kind {
    match field {
        "priority" | "modality" | "confirmation" | "likelihood" | "impact" | "severity"
        | "category" => Kind::Enum,
        "deferred" | "blocking" => Kind::Bool,
        "acceptance_criteria"
        | "scope"
        | "bring"
        | "options"
        | "alternatives"
        | "consequences"
        | "aliases"
        | "code_names" => Kind::List,
        "examples" => Kind::Examples,
        "target" => Kind::Target,
        "resolution" => Kind::Resolution,
        _ => Kind::Text,
    }
}

/// The key of a list field in Markdown: one line for each value (10.3).
pub(crate) fn list_key(field: &str) -> &str {
    match field {
        "acceptance_criteria" => "ac",
        "options" => "option",
        "alternatives" => "alternative",
        "consequences" => "consequence",
        "aliases" => "alias",
        "code_names" => "code_name",
        other => other,
    }
}

/// The names of enum values, as serde writes them, for example `ask_first`.
fn names<T: Serialize>(all: &[T]) -> Vec<String> {
    all.iter()
        .filter_map(|v| serde_json::to_value(v).ok()?.as_str().map(str::to_string))
        .collect()
}

/// The values of each field with a fixed list, from the types in `model`.
static ENUMS: LazyLock<HashMap<&'static str, Vec<String>>> = LazyLock::new(|| {
    HashMap::from([
        ("priority", names(Priority::ALL)),
        ("modality", names(Modality::ALL)),
        ("confirmation", names(Confirmation::ALL)),
        ("likelihood", names(Level::ALL)),
        ("impact", names(Level::ALL)),
        ("severity", names(Severity::ALL)),
        ("category", names(Category::ALL)),
        ("status", names(ItemStatus::ALL)),
        ("outcome", names(Outcome::ALL)),
        ("rel", names(RefRel::ALL)),
    ])
});

/// The values of a field with a fixed list. `rel` gives the values of `rel`
/// in a ref (specification section 5).
pub(crate) fn enum_values(field: &str) -> &'static [String] {
    ENUMS.get(field).map_or(&[], Vec::as_slice)
}

/// Whether a key is the `rel` of a ref, for example `refines`.
pub(crate) fn is_rel(key: &str) -> bool {
    enum_values("rel").iter().any(|r| r == key)
}

/// The main field of each evidence kind: the value on the `- <kind>:` line.
pub(crate) fn evidence_main(kind: &str) -> Option<&'static str> {
    match kind {
        "quote" | "discussion" => Some("text"),
        "code" => Some("path"),
        "link" => Some("uri"),
        _ => None,
    }
}

/// The type of an item, from its `type` value.
pub(crate) fn item_type(item: &Value) -> Option<ItemType> {
    item.get("type")?.as_str()?.parse().ok()
}

/// The position of a type in the order of the sections.
pub(crate) fn type_rank(item: &Value) -> usize {
    item_type(item)
        .and_then(|t| ItemType::ALL.iter().position(|x| *x == t))
        .unwrap_or(ItemType::ALL.len())
}
