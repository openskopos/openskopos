//! The difference between two versions of an artifact, by item ID.
//!
//! An item ID keeps its meaning in every version (specification section 6),
//! so two versions compare item by item:
//!
//! - **Added**: the ID is in the new version only.
//! - **Changed**: the ID is in both versions, and a field of the item is
//!   different. A change of display order is not a change.
//! - **Superseded**: the ID is in the old version only, and an item of the new
//!   version that is not rejected links to it with `supersedes`.
//! - **Dropped**: the ID is in the old version only, and no item supersedes it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as _};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The envelope fields that a diff does not compare. They describe how a
/// version was made, not what it says (specification 10.8).
/// `id_counters` changes with every new item, so it is left out too.
const NOT_CONTENT: [&str; 9] = [
    "version",
    "id_counters",
    "status",
    "parents",
    "created_at",
    "finalized_at",
    "method",
    "session",
    "items",
];

/// One version of an artifact: its ID and version number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionRef {
    pub artifact: String,
    pub version: Option<u64>,
}

/// An item that is in one version only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemSummary {
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub text: String,
}

/// An item that is in both versions with different fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangedItem {
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub text: String,
    /// The names of the fields that are different, in alphabetical order. A
    /// field that is in one version only is in the list.
    pub fields: Vec<String>,
    /// The old `status`, if the status changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_from: Option<String>,
    /// The new `status`, if the status changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_to: Option<String>,
}

/// An item of the old version that the new version replaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupersededItem {
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub text: String,
    /// The IDs of the new items that supersede it.
    pub by: Vec<String>,
}

/// The difference between two documents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diff {
    pub from: VersionRef,
    pub to: VersionRef,
    /// True if both documents have the same artifact ID. Item IDs mean the
    /// same thing only inside one artifact.
    pub same_artifact: bool,
    /// Envelope fields that are different, such as `title` or `summary`. The
    /// fields that describe how a version was made are not compared.
    pub envelope: Vec<String>,
    /// In the order of the new version.
    pub added: Vec<ItemSummary>,
    /// In the order of the new version.
    pub changed: Vec<ChangedItem>,
    /// In the order of the old version.
    pub dropped: Vec<ItemSummary>,
    /// In the order of the old version.
    pub superseded: Vec<SupersededItem>,
}

impl Diff {
    /// True if the two documents have the same content.
    pub fn is_empty(&self) -> bool {
        self.envelope.is_empty()
            && self.added.is_empty()
            && self.changed.is_empty()
            && self.dropped.is_empty()
            && self.superseded.is_empty()
    }

    /// The diff as text for people, one line for each item.
    ///
    /// ```text
    /// pa_2Xc5Rv8Tn1Mb4Kz7@2 -> pa_2Xc5Rv8Tn1Mb4Kz7@3
    /// added      FR4  A plan change that leaves more seats ...
    /// changed    A1   text, confirmation
    /// superseded FR2  by FR4
    /// dropped    Q3   Should guests count as seats?
    /// ```
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        self.write_text(&mut out)
            .expect("writing to a String cannot fail");
        out
    }

    fn write_text(&self, out: &mut String) -> fmt::Result {
        writeln!(
            out,
            "{} -> {}",
            crate::version_label(&self.from.artifact, self.from.version),
            crate::version_label(&self.to.artifact, self.to.version)
        )?;
        if !self.same_artifact {
            writeln!(
                out,
                "note: the documents are different artifacts, so the same ID can mean different items"
            )?;
        }
        if self.is_empty() {
            return writeln!(out, "no changes");
        }
        if !self.envelope.is_empty() {
            writeln!(out, "envelope   {}", self.envelope.join(", "))?;
        }
        let width = self
            .added
            .iter()
            .chain(&self.dropped)
            .map(|i| i.id.len())
            .chain(self.changed.iter().map(|i| i.id.len()))
            .chain(self.superseded.iter().map(|i| i.id.len()))
            .max()
            .unwrap_or(0);
        for item in &self.added {
            let text = crate::one_line(&item.text);
            writeln!(out, "added      {:width$}  {text}", item.id)?;
        }
        for item in &self.changed {
            write!(
                out,
                "changed    {:width$}  {}",
                item.id,
                item.fields.join(", ")
            )?;
            if let (Some(from), Some(to)) = (&item.status_from, &item.status_to) {
                write!(out, " (status {from} -> {to})")?;
            }
            writeln!(out)?;
        }
        for item in &self.superseded {
            writeln!(
                out,
                "superseded {:width$}  by {}",
                item.id,
                item.by.join(", ")
            )?;
        }
        for item in &self.dropped {
            let text = crate::one_line(&item.text);
            writeln!(out, "dropped    {:width$}  {text}", item.id)?;
        }
        Ok(())
    }
}

fn version_ref(doc: &Value) -> VersionRef {
    VersionRef {
        artifact: str_field(doc, "id").to_string(),
        version: doc.get("version").and_then(crate::whole_number),
    }
}

fn str_field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

/// The items of a document by ID, in document order. An item with no string
/// ID is left out. If two items have the same ID, the first one counts.
fn items_by_id(doc: &Value) -> Vec<(&str, &Map<String, Value>)> {
    let mut seen = BTreeSet::new();
    doc.get("items")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let map = item.as_object()?;
            let id = map.get("id")?.as_str().filter(|id| !id.is_empty())?;
            seen.insert(id).then_some((id, map))
        })
        .collect()
}

fn summary(id: &str, item: &Map<String, Value>) -> ItemSummary {
    ItemSummary {
        id: id.to_string(),
        item_type: item
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        text: item
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    }
}

/// Keys whose values are different in two objects, in alphabetical order.
/// The order of `refs` does not matter: a change of order is not a change.
fn different_keys(a: &Map<String, Value>, b: &Map<String, Value>, skip: &[&str]) -> Vec<String> {
    let keys: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
    keys.into_iter()
        .filter(|k| !skip.contains(&k.as_str()))
        .filter(|k| match (k.as_str(), a.get(*k), b.get(*k)) {
            ("refs", Some(Value::Array(x)), Some(Value::Array(y))) => !same_set(x, y),
            (_, x, y) => !same(x, y),
        })
        .cloned()
        .collect()
}

/// Two lists hold the same values, in any order.
fn same_set(x: &[Value], y: &[Value]) -> bool {
    let mut unmatched: Vec<&Value> = y.iter().collect();
    x.len() == y.len()
        && x.iter().all(
            |v| match unmatched.iter().position(|w| same(Some(v), Some(w))) {
                Some(i) => {
                    unmatched.swap_remove(i);
                    true
                }
                None => false,
            },
        )
}

/// JSON values are equal. `3` and `3.0` are equal, as in JSON Schema. Two
/// integers are compared exactly.
fn same(a: Option<&Value>, b: Option<&Value>) -> bool {
    match (a, b) {
        (Some(Value::Number(x)), Some(Value::Number(y))) => {
            x == y || ((x.is_f64() || y.is_f64()) && x.as_f64() == y.as_f64())
        }
        (Some(Value::Array(x)), Some(Value::Array(y))) => {
            x.len() == y.len() && x.iter().zip(y).all(|(x, y)| same(Some(x), Some(y)))
        }
        (Some(Value::Object(x)), Some(Value::Object(y))) => {
            x.len() == y.len() && x.iter().all(|(k, v)| same(Some(v), y.get(k)))
        }
        (a, b) => a == b,
    }
}

/// True if a ref points to an item of this artifact. A ref with no
/// `artifact` points into its own document.
fn local_ref(r: &Value, artifact: &str) -> bool {
    match r.get("artifact").and_then(Value::as_str) {
        None => true,
        Some(a) => a == artifact,
    }
}

/// Compares two documents by item ID. `a` is the old version and `b` the new
/// one.
///
/// The function works on any JSON value. It does not check the documents:
/// use [`crate::check`] first. Items with no ID are left out.
///
/// ```
/// use serde_json::json;
/// let a = json!({"id": "pa_7Hk2mQ9xR4tV8wZ1", "version": 1, "items": [
///     {"id": "FR1", "type": "requirement", "text": "Old rule.", "status": "accepted"}
/// ]});
/// let b = json!({"id": "pa_7Hk2mQ9xR4tV8wZ1", "version": 2, "items": [
///     {"id": "FR2", "type": "requirement", "text": "New rule.", "status": "accepted",
///      "refs": [{"item": "FR1", "rel": "supersedes"}]}
/// ]});
/// let d = skopos_core::diff::diff(&a, &b);
/// assert_eq!(d.added[0].id, "FR2");
/// assert_eq!(d.superseded[0].id, "FR1");
/// assert_eq!(d.superseded[0].by, vec!["FR2"]);
/// assert!(d.dropped.is_empty());
/// ```
pub fn diff(a: &Value, b: &Value) -> Diff {
    let from = version_ref(a);
    let to = version_ref(b);
    let old = items_by_id(a);
    let new = items_by_id(b);
    let old_ids: BTreeMap<&str, &Map<String, Value>> = old.iter().copied().collect();
    let new_ids: BTreeMap<&str, &Map<String, Value>> = new.iter().copied().collect();

    let envelope = match (a.as_object(), b.as_object()) {
        (Some(a), Some(b)) => different_keys(a, b, &NOT_CONTENT),
        _ => Vec::new(),
    };

    let mut added = Vec::new();
    let mut changed = Vec::new();
    // For each old ID, the new items that supersede it.
    let mut superseders: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for (id, item) in &new {
        // A rejected item replaces nothing: the team decided against it.
        let rejected = item.get("status").and_then(Value::as_str) == Some("rejected");
        for r in item
            .get("refs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if !rejected
                && r.get("rel").and_then(Value::as_str) == Some("supersedes")
                && local_ref(r, &to.artifact)
            {
                if let Some(target) = r.get("item").and_then(Value::as_str) {
                    let by = superseders.entry(target).or_default();
                    if !by.iter().any(|x| x == id) {
                        by.push(id.to_string());
                    }
                }
            }
        }
        match old_ids.get(id) {
            None => added.push(summary(id, item)),
            Some(old_item) => {
                let fields = different_keys(old_item, item, &[]);
                if fields.is_empty() {
                    continue;
                }
                let status = |m: &Map<String, Value>| {
                    m.get("status").and_then(Value::as_str).map(str::to_string)
                };
                let (status_from, status_to) = if fields.iter().any(|f| f == "status") {
                    (status(old_item), status(item))
                } else {
                    (None, None)
                };
                let s = summary(id, item);
                changed.push(ChangedItem {
                    id: s.id,
                    item_type: s.item_type,
                    text: s.text,
                    fields,
                    status_from,
                    status_to,
                });
            }
        }
    }

    let mut dropped = Vec::new();
    let mut superseded = Vec::new();
    for (id, item) in &old {
        if new_ids.contains_key(id) {
            continue;
        }
        let s = summary(id, item);
        match superseders.get(id) {
            Some(by) => superseded.push(SupersededItem {
                id: s.id,
                item_type: s.item_type,
                text: s.text,
                by: by.clone(),
            }),
            None => dropped.push(s),
        }
    }

    Diff {
        same_artifact: from.artifact == to.artifact,
        from,
        to,
        envelope,
        added,
        changed,
        dropped,
        superseded,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn same_document_has_no_changes() {
        let doc = json!({"id": "pa_1", "version": 1, "title": "T", "items": [
            {"id": "G1", "type": "goal", "text": "Why.", "status": "accepted"}
        ]});
        let d = diff(&doc, &doc);
        assert!(d.is_empty());
        assert!(d.to_text().ends_with("no changes\n"));
    }

    #[test]
    fn order_and_process_fields_are_not_changes() {
        let a = json!({"id": "pa_1", "version": 1, "status": "final", "created_at": "x", "items": [
            {"id": "G1", "type": "goal", "text": "One.", "status": "accepted"},
            {"id": "G2", "type": "goal", "text": "Two.", "status": "accepted"}
        ]});
        let b = json!({"id": "pa_1", "version": 2, "status": "draft", "created_at": "y",
            "parents": [{"rel": "previous_version", "artifact": "pa_1", "version": 1}],
            "items": [
            {"id": "G2", "type": "goal", "text": "Two.", "status": "accepted"},
            {"id": "G1", "type": "goal", "text": "One.", "status": "accepted"}
        ]});
        assert!(diff(&a, &b).is_empty());
    }

    #[test]
    fn whole_number_floats_are_equal_to_integers() {
        let a = json!({"id": "pa_1", "items": [
            {"id": "CR1", "type": "critique", "text": "x", "status": "accepted",
             "ratification": {"yes": 2, "no": 0, "eligible": 2}}
        ]});
        let b = json!({"id": "pa_1", "items": [
            {"id": "CR1", "type": "critique", "text": "x", "status": "accepted",
             "ratification": {"yes": 2.0, "no": 0, "eligible": 2}}
        ]});
        assert!(diff(&a, &b).is_empty());
    }

    #[test]
    fn status_change_names_both_statuses() {
        let a = json!({"id": "pa_1", "items": [
            {"id": "Q1", "type": "open_question", "text": "Why?", "status": "proposed"}
        ]});
        let b = json!({"id": "pa_1", "items": [
            {"id": "Q1", "type": "open_question", "text": "Why?", "status": "rejected"}
        ]});
        let d = diff(&a, &b);
        assert_eq!(d.changed[0].fields, vec!["status"]);
        assert_eq!(d.changed[0].status_from.as_deref(), Some("proposed"));
        assert_eq!(d.changed[0].status_to.as_deref(), Some("rejected"));
    }

    #[test]
    fn a_ref_into_another_artifact_does_not_supersede() {
        let a = json!({"id": "pa_1", "items": [
            {"id": "FR1", "type": "requirement", "text": "Old.", "status": "accepted"}
        ]});
        let b = json!({"id": "pa_1", "items": [
            {"id": "FR2", "type": "requirement", "text": "New.", "status": "accepted",
             "refs": [{"item": "FR1", "artifact": "pa_2", "version": 1, "rel": "supersedes"}]}
        ]});
        let d = diff(&a, &b);
        assert_eq!(d.dropped[0].id, "FR1");
        assert!(d.superseded.is_empty());
    }

    #[test]
    fn a_rejected_item_does_not_supersede() {
        let a = json!({"id": "pa_1", "items": [
            {"id": "FR1", "type": "requirement", "text": "Old.", "status": "accepted"}
        ]});
        let b = json!({"id": "pa_1", "items": [
            {"id": "FR2", "type": "requirement", "text": "New.", "status": "rejected",
             "refs": [{"item": "FR1", "rel": "supersedes"}]}
        ]});
        let d = diff(&a, &b);
        assert_eq!(d.dropped[0].id, "FR1");
        assert!(d.superseded.is_empty());
    }

    #[test]
    fn ref_order_is_not_a_change_but_a_new_ref_is() {
        let refs = |r: Value| {
            json!({"id": "pa_1", "items": [
                {"id": "D1", "type": "decision", "text": "x", "status": "accepted", "refs": r}
            ]})
        };
        let a =
            refs(json!([{"item": "Q1", "rel": "resolves"}, {"item": "A1", "rel": "depends_on"}]));
        let b =
            refs(json!([{"item": "A1", "rel": "depends_on"}, {"item": "Q1", "rel": "resolves"}]));
        let c =
            refs(json!([{"item": "A1", "rel": "depends_on"}, {"item": "A1", "rel": "depends_on"}]));
        assert!(diff(&a, &b).is_empty());
        assert_eq!(diff(&a, &c).changed[0].fields, vec!["refs"]);
    }

    #[test]
    fn large_integers_are_compared_exactly() {
        let n = |v: Value| json!({"id": "pa_1", "x-n": v, "items": []});
        let d = diff(
            &n(json!(9_007_199_254_740_993_u64)),
            &n(json!(9_007_199_254_740_992_u64)),
        );
        assert_eq!(d.envelope, vec!["x-n"]);
    }

    #[test]
    fn text_lines_stay_on_one_line() {
        let a = json!({"id": "pa_1", "items": []});
        let b = json!({"id": "pa_1", "items": [
            {"id": "FR1", "type": "requirement", "text": "Block it.\nShow members.", "status": "accepted"}
        ]});
        assert!(diff(&a, &b)
            .to_text()
            .contains("added      FR1  Block it. Show members.\n"));
    }

    #[test]
    fn different_artifacts_are_flagged() {
        let a = json!({"id": "pa_1", "version": 1, "items": []});
        let b = json!({"id": "pa_2", "version": 1, "items": []});
        let d = diff(&a, &b);
        assert!(!d.same_artifact);
        assert_eq!(d.envelope, vec!["id"]);
        assert!(d.to_text().contains("different artifacts"));
    }
}
