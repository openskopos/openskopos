//! JSON and YAML text.

use serde_json::Value;

use crate::order::{Ordered, Shape};
use crate::Problem;

/// A text format of a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    Json,
    Yaml,
}

/// Reads a document from JSON or YAML text.
///
/// The result is the data of the document. Comments in YAML, and the order and
/// layout of the text, are not kept. A YAML file must use only the data types
/// of JSON (specification 2.2). A byte order mark at the start is ignored.
pub fn parse(text: &str, format: Format) -> Result<Value, Problem> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    match format {
        Format::Json => serde_json::from_str(text)
            .map_err(|e| Problem::new("", format!("the text is not valid JSON: {e}"))),
        Format::Yaml => serde_yaml_ng::from_str(text).map_err(|e| {
            Problem::new(
                "",
                format!("the text is not valid YAML for a document: {e}"),
            )
        }),
    }
}

/// Writes a document as JSON or YAML in the canonical layout.
///
/// Keys are in the order of the specification: the fields of the document in
/// the order of section 3, the fields of each item in the order of section 4,
/// then `x-` keys. JSON has an indent of two spaces. YAML puts dates, times,
/// `"yes"` and `"no"` in quotes (specification 2.2). The text ends with a line
/// break.
///
/// The function writes any value, also a document that is not valid. Reading
/// the text with [`parse`] gives the same value again.
pub fn write(doc: &Value, format: Format) -> String {
    match format {
        Format::Json => to_json(doc),
        Format::Yaml => to_yaml(doc),
    }
}

/// Writes a document as JSON in the canonical layout. See [`write`].
pub fn to_json(doc: &Value) -> String {
    let mut text = serde_json::to_string_pretty(&Ordered {
        value: doc,
        shape: Shape::Document,
    })
    .expect("a JSON value always serializes");
    text.push('\n');
    text
}

/// Writes a document as YAML in the canonical layout. See [`write`].
pub fn to_yaml(doc: &Value) -> String {
    crate::yaml::write(doc)
}
