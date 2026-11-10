//! Editing a stored version (specification 10.8).

use serde_json::{Map, Value};

use super::type_rank;
use crate::whole_number;

/// The fields that are not content (10.8).
const NOT_CONTENT: &[&str] = &[
    "version",
    "status",
    "parents",
    "created_at",
    "finalized_at",
    "method",
    "session",
];

/// The document that a tool stores after it reads an edited Markdown file of
/// a stored version.
///
/// `stored` is the stored version. `read_doc` is the document from the file,
/// from [`super::read`].
///
/// - If `stored` is final and the content did not change, the result is
///   `stored`.
/// - If `stored` is final and the content changed, the result is version n+1
///   as a draft. Its first parent is a `previous_version` parent for version
///   n. It has no `finalized_at`, `method` or `session`, and `created_at` is
///   the current time.
/// - If `stored` is a draft, the result is the document from the file, with
///   the version number of `stored`.
///
/// The result always has the artifact ID of `stored`, because it is a
/// version of that artifact. A file with no front matter gets a new ID from
/// the reader, and this function puts the stored ID back.
///
/// This function is not in the WebAssembly build. Use [`next_version_at`]
/// there.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn next_version(stored: &Value, read_doc: &Value) -> Value {
    next_version_at(stored, read_doc, &super::Fill::new().now)
}

/// [`next_version`], with the time to use for `created_at`, for example
/// `2026-11-10T12:00:00Z`.
pub fn next_version_at(stored: &Value, read_doc: &Value, now: &str) -> Value {
    let mut new = read_doc.clone();
    let Some(map) = new.as_object_mut() else {
        return new;
    };
    if let Some(id) = stored.get("id") {
        map.insert("id".into(), id.clone());
    }
    let version = stored.get("version").cloned().unwrap_or(Value::Null);
    if stored.get("status").and_then(Value::as_str) != Some("final") {
        map.insert("version".into(), version);
        map.insert("status".into(), "draft".into());
        return new;
    }
    if same_content(stored, &new) {
        return stored.clone();
    }
    let map = new.as_object_mut().expect("an object");
    let n = whole_number(&version).unwrap_or(0);
    map.insert("version".into(), (n + 1).into());
    map.insert("status".into(), "draft".into());
    let mut previous = Map::new();
    previous.insert("rel".into(), "previous_version".into());
    previous.insert(
        "artifact".into(),
        stored.get("id").cloned().unwrap_or(Value::Null),
    );
    previous.insert("version".into(), version);
    let mut parents = vec![Value::Object(previous)];
    if let Some(list) = map.get("parents").and_then(Value::as_array) {
        parents.extend(
            list.iter()
                .filter(|p| p.get("rel").and_then(Value::as_str) != Some("previous_version"))
                .cloned(),
        );
    }
    map.insert("parents".into(), Value::Array(parents));
    for k in ["finalized_at", "method", "session"] {
        map.remove(k);
    }
    map.insert("created_at".into(), now.into());
    new
}

/// Whether two documents have the same content (10.8): equal after the
/// fields that are not content, and the order of items between types, are
/// left out (specification section 13).
pub fn same_content(a: &Value, b: &Value) -> bool {
    content(a) == content(b)
}

fn content(doc: &Value) -> Value {
    let mut doc = doc.clone();
    if let Some(map) = doc.as_object_mut() {
        for k in NOT_CONTENT {
            map.remove(*k);
        }
    }
    normal_form(doc)
}

/// A document with its items in the order of the sections, so that two
/// documents that differ only in the order of items between types are
/// equal. The sort is stable, so items of one type keep their order.
pub(crate) fn normal_form(mut doc: Value) -> Value {
    if let Some(items) = doc.get_mut("items").and_then(Value::as_array_mut) {
        items.sort_by_key(type_rank);
    }
    doc
}
