//! Certain fixes for problems.
//!
//! [`crate::check`] gives some problems a [`Fix`]: a change that makes the
//! problem go away and that needs no choice from a person. For example, a
//! counter in `id_counters` that is too low gets the highest number of its
//! items. A fix is a list of JSON Patch operations (RFC 6902) on the
//! document.
//!
//! - [`apply`] applies one fix to a document.
//! - [`fix_all`] applies fixes until no problem has one, and gives the fixes
//!   that it applied.
//!
//! ```
//! let doc = serde_json::json!({
//!   "$schema": "https://openskopos.org/schema/artifact/0.1.json",
//!   "id": "pa_7Hk2mQ9xR4tV8wZ1", "version": 1, "kind": "intent",
//!   "title": "Faster invoices", "status": "draft", "created_at": "2026-11-05T09:00:00Z",
//!   "id_counters": {"G": 0},
//!   "items": [{"id": "G1", "type": "goal", "text": "Invoices go out on day one.", "status": "accepted"}],
//! });
//! let problems = skopos_core::check(&doc);
//! let fix = problems[0].fix.as_ref().unwrap();
//! assert_eq!(fix.label, "Set the counter for G to 1");
//! let fixed = skopos_core::fix::apply(&doc, fix).unwrap();
//! assert!(skopos_core::check(&fixed).is_empty());
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{check, Problem};

/// A change that fixes a problem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fix {
    /// What the fix does, in words for people, for example
    /// `Set the counter for FR to 4`.
    pub label: String,
    /// The JSON Patch operations (RFC 6902) that make the change.
    pub patch: Vec<Op>,
}

/// One JSON Patch operation: `add`, `replace` or `remove`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Op {
    pub op: String,
    /// A JSON Pointer.
    pub path: String,
    /// The new value. Not used by `remove`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
}

impl Op {
    /// An `add` operation. On an object key, it sets the value whether the
    /// key exists or not.
    pub fn add(path: impl Into<String>, value: Value) -> Self {
        Op {
            op: "add".into(),
            path: path.into(),
            value: Some(value),
        }
    }

    /// A `replace` operation: the value at the path must exist.
    pub fn replace(path: impl Into<String>, value: Value) -> Self {
        Op {
            op: "replace".into(),
            path: path.into(),
            value: Some(value),
        }
    }
}

/// The keys of a JSON Pointer, unescaped.
fn tokens(path: &str) -> Result<Vec<String>, String> {
    if path.is_empty() {
        return Ok(vec![]);
    }
    let rest = path
        .strip_prefix('/')
        .ok_or_else(|| format!("{path:?} is not a JSON Pointer"))?;
    Ok(rest
        .split('/')
        .map(|t| t.replace("~1", "/").replace("~0", "~"))
        .collect())
}

fn apply_op(doc: &mut Value, op: &Op) -> Result<(), String> {
    let mut keys = tokens(&op.path)?;
    let Some(last) = keys.pop() else {
        return match (op.op.as_str(), &op.value) {
            ("add" | "replace", Some(v)) => {
                *doc = v.clone();
                Ok(())
            }
            _ => Err("cannot remove the whole document".into()),
        };
    };
    let mut parent = &mut *doc;
    for k in &keys {
        parent = match parent {
            Value::Object(m) => m.get_mut(k),
            Value::Array(a) => k.parse::<usize>().ok().and_then(|i| a.get_mut(i)),
            _ => None,
        }
        .ok_or_else(|| format!("{} does not exist", op.path))?;
    }
    let missing = || format!("{} does not exist", op.path);
    match (op.op.as_str(), parent) {
        ("add", Value::Object(m)) => {
            m.insert(last, op.value.clone().ok_or("add needs a value")?);
        }
        ("replace", Value::Object(m)) => {
            let slot = m.get_mut(&last).ok_or_else(missing)?;
            *slot = op.value.clone().ok_or("replace needs a value")?;
        }
        ("remove", Value::Object(m)) => {
            m.remove(&last).ok_or_else(missing)?;
        }
        ("add", Value::Array(a)) => {
            let v = op.value.clone().ok_or("add needs a value")?;
            let i = if last == "-" {
                a.len()
            } else {
                last.parse::<usize>()
                    .ok()
                    .filter(|i| *i <= a.len())
                    .ok_or_else(missing)?
            };
            a.insert(i, v);
        }
        ("replace", Value::Array(a)) => {
            let slot = last
                .parse::<usize>()
                .ok()
                .and_then(|i| a.get_mut(i))
                .ok_or_else(missing)?;
            *slot = op.value.clone().ok_or("replace needs a value")?;
        }
        ("remove", Value::Array(a)) => {
            let i = last
                .parse::<usize>()
                .ok()
                .filter(|i| *i < a.len())
                .ok_or_else(missing)?;
            a.remove(i);
        }
        ("add" | "replace" | "remove", _) => return Err(missing()),
        (other, _) => return Err(format!("the operation {other:?} is not supported")),
    }
    Ok(())
}

/// Applies a fix to a document and gives the new document. The fix applies
/// in full or not at all. An error means that the fix does not fit the
/// document, for example because the document changed after the check.
pub fn apply(doc: &Value, fix: &Fix) -> Result<Value, Problem> {
    let mut out = doc.clone();
    for op in &fix.patch {
        apply_op(&mut out, op)
            .map_err(|e| Problem::new(op.path.clone(), format!("cannot apply the fix: {e}")))?;
    }
    Ok(out)
}

/// Applies the fixes of the problems that have one, until no problem has a
/// fix. Gives the new document and the fixes that it applied, in order.
///
/// A fix can change other problems, so the function checks the document
/// again after each fix. The problems that are left need a person.
pub fn fix_all(doc: &Value) -> (Value, Vec<Fix>) {
    let mut doc = doc.clone();
    let mut applied = Vec::new();
    // Each fix removes at least one problem, so this is a guard only.
    for _ in 0..1000 {
        let Some(fix) = check(&doc).into_iter().find_map(|p| p.fix.map(|f| *f)) else {
            break;
        };
        match apply(&doc, &fix) {
            Ok(next) if next != doc => {
                doc = next;
                applied.push(fix);
            }
            _ => break,
        }
    }
    (doc, applied)
}

// ------------------------------------------------------------------ helpers

/// An item for people: its ID, or `Item 4` if it has none.
pub(crate) fn item_label(doc: &Value, index: usize) -> String {
    doc.get("items")
        .and_then(|i| i.get(index))
        .and_then(|i| i.get("id"))
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .map_or_else(|| format!("Item {}", index + 1), str::to_string)
}

/// `a`, `a or b`, or `a, b or c`.
pub(crate) fn either(values: &[String]) -> String {
    match values {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}

/// The highest number of the items with this prefix.
fn highest(doc: &Value, prefix: &str) -> u64 {
    doc.get("items")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|i| crate::parse_item_id(i.get("id")?.as_str()?))
        .filter(|(t, _)| t.prefix() == prefix)
        .map(|(_, n)| n)
        .max()
        .unwrap_or(0)
}

/// The operation that sets the counter of a prefix, or `None` if
/// `id_counters` is there but is not an object.
pub(crate) fn counter_op(doc: &Value, prefix: &str, n: u64) -> Option<Op> {
    match doc.get("id_counters") {
        None => Some(Op::add("/id_counters", serde_json::json!({ prefix: n }))),
        Some(Value::Object(_)) => Some(Op::add(crate::pointer("/id_counters", prefix), n.into())),
        Some(_) => None,
    }
}

/// A fix that gives the item at `index` the next free ID of its prefix and
/// raises the counter, as `assign_ids` does.
pub(crate) fn new_id_fix(
    doc: &Value,
    index: usize,
    prefix: &str,
    label: impl FnOnce(&str) -> String,
) -> Fix {
    let counter = doc
        .get("id_counters")
        .and_then(|c| c.get(prefix))
        .and_then(crate::whole_number)
        .unwrap_or(0);
    let n = counter.max(highest(doc, prefix)) + 1;
    let id = format!("{prefix}{n}");
    let mut patch = vec![Op::add(
        crate::pointer(&crate::pointer("/items", index), "id"),
        id.clone().into(),
    )];
    patch.extend(counter_op(doc, prefix, n));
    Fix {
        label: label(&id),
        patch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn applies_patch_operations() {
        let doc = json!({"a": {"b~/c": 1}, "l": [1, 3]});
        let fix = Fix {
            label: String::new(),
            patch: vec![
                Op::replace("/a/b~0~1c", json!(2)),
                Op::add("/l/1", json!(2)),
                Op::add("/l/-", json!(4)),
                Op::add("/n", json!(null)),
                Op {
                    op: "remove".into(),
                    path: "/l/0".into(),
                    value: None,
                },
            ],
        };
        assert_eq!(
            apply(&doc, &fix).unwrap(),
            json!({"a": {"b~/c": 2}, "l": [2, 3, 4], "n": null})
        );
        // All or nothing.
        let bad = Fix {
            label: String::new(),
            patch: vec![Op::add("/x", json!(1)), Op::replace("/missing", json!(1))],
        };
        assert!(apply(&doc, &bad).is_err());
    }
}
