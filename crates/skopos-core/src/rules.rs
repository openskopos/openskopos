//! The rules of specification section 9, which JSON Schema cannot express.

use std::collections::{BTreeMap, HashMap};

use serde_json::Value;

use crate::fix::{counter_op, new_id_fix, Fix, Op};
use crate::model::ItemType;
use crate::{parse_item_id, pointer, whole_number, Problem};

/// No two items in a version have the same ID.
pub const UNIQUE_ITEM_IDS: &str = "unique-item-ids";
/// For each prefix in `items`, the counter in `id_counters` is equal to or
/// more than the highest number of that prefix. A missing counter is 0.
pub const COUNTERS_COVER_ITEMS: &str = "counters-cover-items";
/// A `previous_version` parent has the same artifact ID as the document.
pub const PREVIOUS_VERSION_SAME_ARTIFACT: &str = "previous-version-same-artifact";
/// A `previous_version` parent has the version of the document minus 1.
pub const PREVIOUS_VERSION_ONE_LESS: &str = "previous-version-one-less";

/// The names of all rules, in the order of specification section 9.
pub const ALL: [&str; 4] = [
    UNIQUE_ITEM_IDS,
    COUNTERS_COVER_ITEMS,
    PREVIOUS_VERSION_SAME_ARTIFACT,
    PREVIOUS_VERSION_ONE_LESS,
];

/// Finds the problems with the rules of specification section 9.
///
/// Each problem has the name of its rule. The function looks only at the
/// parts of the document that have the right shape. The schema reports the
/// other parts.
pub fn check_rules(doc: &Value) -> Vec<Problem> {
    let mut problems = vec![];
    let items = doc.get("items").and_then(Value::as_array);

    // unique-item-ids
    let mut first: HashMap<&str, usize> = HashMap::new();
    for (i, item) in items.into_iter().flatten().enumerate() {
        let Some(id) = item.get("id").and_then(Value::as_str) else {
            continue;
        };
        if let Some(j) = first.get(id) {
            // The prefix of the item's type, or else of the ID.
            let prefix = item
                .get("type")
                .and_then(Value::as_str)
                .and_then(|t| t.parse::<ItemType>().ok())
                .or_else(|| parse_item_id(id).map(|(t, _)| t))
                .map(ItemType::prefix);
            let fix = prefix.map(|p| {
                new_id_fix(doc, i, p, |new| {
                    format!("Give the second {id} the ID {new}")
                })
            });
            let hint = match &fix {
                Some(f) => format!(
                    "Two items have the ID {id}. Give the second one a new ID, such as {}.",
                    f.label.rsplit(' ').next().unwrap_or_default()
                ),
                None => format!("Two items have the ID {id}. Give the second one a new ID."),
            };
            problems.push(
                Problem::rule(
                    pointer(&pointer("/items", i), "id"),
                    UNIQUE_ITEM_IDS,
                    format!("the item ID {id} is also the ID of /items/{j}"),
                )
                .with_hint(hint)
                .with_fix(fix),
            );
        } else {
            first.insert(id, i);
        }
    }

    // counters-cover-items
    let mut highest: BTreeMap<&'static str, (u64, usize)> = BTreeMap::new();
    for (i, item) in items.into_iter().flatten().enumerate() {
        let Some((item_type, n)) = item
            .get("id")
            .and_then(Value::as_str)
            .and_then(parse_item_id)
        else {
            continue;
        };
        let entry = highest.entry(item_type.prefix()).or_insert((n, i));
        if n > entry.0 {
            *entry = (n, i);
        }
    }
    let counters = doc.get("id_counters").and_then(Value::as_object);
    // With no id_counters, one fix adds a counter for every prefix.
    let add_all = Fix {
        label: "Add id_counters for the IDs in use".to_string(),
        patch: vec![Op::add(
            "/id_counters",
            highest
                .iter()
                .map(|(p, (n, _))| (p.to_string(), Value::from(*n)))
                .collect::<serde_json::Map<_, _>>()
                .into(),
        )],
    };
    if doc.get("id_counters").is_none() || counters.is_some() {
        for (prefix, (n, i)) in highest.clone() {
            let counter = counters.and_then(|c| c.get(prefix));
            let value = match counter {
                None => 0,
                Some(v) => match whole_number(v) {
                    Some(v) => v,
                    // The schema reports a counter that is not a whole number.
                    None => continue,
                },
            };
            if value < n {
                let path = if counter.is_some() {
                    pointer("/id_counters", prefix)
                } else {
                    "/id_counters".to_string()
                };
                let fix = if counters.is_none() {
                    Some(add_all.clone())
                } else {
                    counter_op(doc, prefix, n).map(|op| Fix {
                        label: format!("Set the counter for {prefix} to {n}"),
                        patch: vec![op],
                    })
                };
                problems.push(
                    Problem::rule(
                        path,
                        COUNTERS_COVER_ITEMS,
                        format!(
                            "the counter for {prefix} is {value}, but /items/{i} has the ID {prefix}{n}; the counter must be {n} or more"
                        ),
                    )
                    .with_hint(format!(
                        "The counter for {prefix} in id_counters must be {n} or more, because {prefix}{n} is in use."
                    ))
                    .with_fix(fix),
                );
            }
        }
    }

    // previous-version-same-artifact and previous-version-one-less
    let parents = doc.get("parents").and_then(Value::as_array);
    for (k, parent) in parents.into_iter().flatten().enumerate() {
        if parent.get("rel").and_then(Value::as_str) != Some("previous_version") {
            continue;
        }
        let at = pointer("/parents", k);
        if let (Some(artifact), Some(id)) = (
            parent.get("artifact").and_then(Value::as_str),
            doc.get("id").and_then(Value::as_str),
        ) {
            if artifact != id {
                problems.push(
                    Problem::rule(
                        pointer(&at, "artifact"),
                        PREVIOUS_VERSION_SAME_ARTIFACT,
                        format!("the previous version is of the artifact {artifact}, but this document is {id}"),
                    )
                    .with_hint(format!(
                        "The previous_version parent must be a version of this artifact, {id}. To build on another artifact, make it an input parent."
                    )),
                );
            }
        }
        // Version 1 has no previous version. The schema reports that.
        if let (Some(parent_version), Some(version)) = (
            parent.get("version").and_then(whole_number),
            doc.get("version")
                .and_then(whole_number)
                .filter(|v| *v >= 2),
        ) {
            if parent_version != version - 1 {
                problems.push(
                    Problem::rule(
                        pointer(&at, "version"),
                        PREVIOUS_VERSION_ONE_LESS,
                        format!(
                            "the previous version is {parent_version}, but this document is version {version}; the previous version must be {}",
                            version - 1
                        ),
                    )
                    .with_hint(format!(
                        "Version {version} must follow version {}. Change the version of the previous_version parent, or the version of the document.",
                        version - 1
                    )),
                );
            }
        }
    }

    problems
}
