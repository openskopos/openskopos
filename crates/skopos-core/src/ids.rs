//! Item IDs and ID assignment (specification sections 6 and 7).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::model::ItemType;
use crate::{pointer, whole_number, Problem};

/// Reads an item ID such as `CR12` into its type and number.
///
/// Returns `None` if the ID does not have the form of specification section
/// 6: a known prefix and a positive integer with no leading zero.
pub fn parse_item_id(id: &str) -> Option<(ItemType, u64)> {
    let split = id.find(|c: char| c.is_ascii_digit())?;
    let (prefix, number) = id.split_at(split);
    if number.starts_with('0') || !number.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((ItemType::from_prefix(prefix)?, number.parse().ok()?))
}

/// An ID that [`assign_ids`] gave to an item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssignedId {
    /// The position of the item in `items`.
    pub index: usize,
    /// The new ID.
    pub id: String,
}

/// What [`assign_ids`] changed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdReport {
    /// The new IDs, in the order of `items`.
    pub assigned: Vec<AssignedId>,
    /// True if `id_counters` changed.
    pub counters_changed: bool,
}

impl IdReport {
    /// True if the document changed.
    pub fn changed(&self) -> bool {
        self.counters_changed || !self.assigned.is_empty()
    }
}

/// Gives an ID to each item that has none, and updates `id_counters`
/// (specification section 7).
///
/// An item has no ID if `id` is absent, `null` or an empty string. Its prefix
/// comes from its `type`. The next number for a prefix is one more than the
/// higher of its counter and the highest ID of that prefix in `items`. Items
/// get IDs in the order of `items`.
///
/// The function also raises each counter that is less than the highest ID of
/// its prefix. It does not change other IDs, also IDs that are not valid.
/// [`crate::check`] reports those.
///
/// If an item that needs an ID has no known `type`, the function returns the
/// problems and does not change the document.
pub fn assign_ids(doc: &mut Value) -> Result<IdReport, Vec<Problem>> {
    let Some(obj) = doc.as_object() else {
        return Err(vec![Problem::new("", "the document is not an object")]);
    };
    let items = match obj.get("items") {
        None => return Ok(IdReport::default()),
        Some(Value::Array(items)) => items,
        Some(_) => return Err(vec![Problem::new("/items", "items is not a list")]),
    };

    let mut problems = vec![];
    let mut counters: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut new_items: Vec<(usize, ItemType)> = vec![];
    for (i, item) in items.iter().enumerate() {
        let at = pointer("/items", i);
        let Some(item) = item.as_object() else {
            problems.push(Problem::new(at, "the item is not an object"));
            continue;
        };
        match item.get("id") {
            None | Some(Value::Null) => {}
            Some(Value::String(s)) if s.is_empty() => {}
            Some(id) => {
                if let Some((t, n)) = id.as_str().and_then(parse_item_id) {
                    let c = counters.entry(t.prefix()).or_insert(0);
                    *c = (*c).max(n);
                }
                continue;
            }
        }
        match item
            .get("type")
            .and_then(Value::as_str)
            .map(str::parse::<ItemType>)
        {
            Some(Ok(t)) => new_items.push((i, t)),
            _ => problems.push(Problem::new(
                pointer(&at, "type"),
                "the item has no ID and no known type, so it cannot get an ID",
            )),
        }
    }

    let old_counters = match obj.get("id_counters") {
        None => None,
        Some(Value::Object(map)) => Some(map),
        Some(_) => {
            problems.push(Problem::new("/id_counters", "id_counters is not an object"));
            None
        }
    };
    for (prefix, value) in old_counters.into_iter().flatten() {
        let Some(t) = ItemType::from_prefix(prefix) else {
            continue;
        };
        match whole_number(value) {
            Some(v) => {
                let c = counters.entry(t.prefix()).or_insert(0);
                *c = (*c).max(v);
            }
            None => problems.push(Problem::new(
                pointer("/id_counters", prefix),
                "the counter is not a whole number of 0 or more",
            )),
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }

    let mut assigned = vec![];
    for (i, t) in new_items {
        let c = counters.entry(t.prefix()).or_insert(0);
        let Some(next) = c.checked_add(1) else {
            return Err(vec![Problem::new(
                pointer("/id_counters", t.prefix()),
                "the counter is too large",
            )]);
        };
        *c = next;
        assigned.push(AssignedId {
            index: i,
            id: format!("{}{next}", t.prefix()),
        });
    }

    // Keep each counter that is there, raised if needed, and add a counter
    // for each other prefix that is in use.
    let mut new_counters: Map<String, Value> = old_counters.cloned().unwrap_or_default();
    for t in ItemType::ALL {
        if let Some(&c) = counters.get(t.prefix()) {
            // Write a counter only if it is new or its number changes, so
            // that a counter such as 3.0 stays as it is.
            let old = new_counters.get(t.prefix()).and_then(whole_number);
            if (c > 0 || old.is_some()) && old != Some(c) {
                new_counters.insert(t.prefix().to_string(), Value::from(c));
            }
        }
    }
    let counters_changed = match old_counters {
        Some(old) => *old != new_counters,
        None => !new_counters.is_empty(),
    };

    let obj = doc.as_object_mut().expect("checked above");
    if counters_changed {
        obj.insert("id_counters".to_string(), Value::Object(new_counters));
    }
    let items = obj
        .get_mut("items")
        .and_then(Value::as_array_mut)
        .expect("checked above");
    for a in &assigned {
        if let Some(item) = items[a.index].as_object_mut() {
            item.insert("id".to_string(), Value::String(a.id.clone()));
        }
    }
    Ok(IdReport {
        assigned,
        counters_changed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_item_ids() {
        assert_eq!(parse_item_id("CR12"), Some((ItemType::Critique, 12)));
        assert_eq!(parse_item_id("ST1"), Some((ItemType::Story, 1)));
        assert_eq!(parse_item_id("S1"), Some((ItemType::SuccessSignal, 1)));
        assert_eq!(parse_item_id("G01"), None);
        assert_eq!(parse_item_id("G0"), None);
        assert_eq!(parse_item_id("X1"), None);
        assert_eq!(parse_item_id("G"), None);
        assert_eq!(parse_item_id("G1a"), None);
        assert_eq!(parse_item_id("g1"), None);
    }

    #[test]
    fn assigns_ids_after_the_counter_and_the_highest_id() {
        let mut doc = json!({
            "id_counters": {"FR": 4, "C": 1},
            "items": [
                {"id": "FR2", "type": "requirement"},
                {"type": "requirement"},
                {"id": "C3", "type": "constraint"},
                {"id": null, "type": "constraint"},
                {"id": "", "type": "goal"},
                {"type": "requirement"}
            ]
        });
        let report = assign_ids(&mut doc).unwrap();
        let ids: Vec<_> = report
            .assigned
            .iter()
            .map(|a| (a.index, a.id.as_str()))
            .collect();
        assert_eq!(ids, [(1, "FR5"), (3, "C4"), (4, "G1"), (5, "FR6")]);
        assert!(report.counters_changed);
        assert_eq!(doc["id_counters"], json!({"FR": 6, "C": 4, "G": 1}));
        assert_eq!(doc["items"][1]["id"], "FR5");
        assert_eq!(doc["items"][5]["id"], "FR6");
    }

    #[test]
    fn changes_nothing_when_every_item_has_an_id() {
        let mut doc = json!({
            "id_counters": {"G": 3},
            "items": [{"id": "G2", "type": "goal"}]
        });
        let before = doc.clone();
        let report = assign_ids(&mut doc).unwrap();
        assert!(!report.changed());
        assert_eq!(doc, before);
    }

    #[test]
    fn raises_a_counter_below_the_highest_id() {
        let mut doc = json!({"items": [{"id": "G2", "type": "goal"}]});
        let report = assign_ids(&mut doc).unwrap();
        assert!(report.assigned.is_empty());
        assert!(report.counters_changed);
        assert_eq!(doc["id_counters"], json!({"G": 2}));
    }

    #[test]
    fn rejects_an_item_with_no_known_type() {
        let mut doc = json!({"items": [{"type": "wish"}, {"text": "x"}, {"type": "goal"}]});
        let before = doc.clone();
        let problems = assign_ids(&mut doc).unwrap_err();
        let paths: Vec<_> = problems.iter().map(|p| p.path.as_str()).collect();
        assert_eq!(paths, ["/items/0/type", "/items/1/type"]);
        assert_eq!(doc, before);
    }

    #[test]
    fn reads_whole_number_floats_as_counters() {
        let mut doc = json!({
            "id_counters": {"G": 3.0},
            "items": [{"id": "G2", "type": "goal"}, {"type": "goal"}]
        });
        let report = assign_ids(&mut doc).unwrap();
        assert_eq!(report.assigned[0].id, "G4");
        assert_eq!(doc["id_counters"], json!({"G": 4}));

        let mut doc = json!({"id_counters": {"G": 3.0}, "items": [{"id": "G2", "type": "goal"}]});
        assert!(!assign_ids(&mut doc).unwrap().changed());
        assert_eq!(doc["id_counters"], json!({"G": 3.0}));
    }

    #[test]
    fn keeps_a_document_with_no_items_unchanged() {
        let mut doc = json!({"items": []});
        assert!(!assign_ids(&mut doc).unwrap().changed());
        assert_eq!(doc, json!({"items": []}));
    }
}
