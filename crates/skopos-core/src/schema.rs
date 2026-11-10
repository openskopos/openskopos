//! Validation against the JSON Schema of format 0.1.

use std::collections::HashSet;
use std::sync::OnceLock;

use jsonschema::{error::ValidationErrorKind, ValidationError, Validator};
use serde_json::Value;

use crate::model::ItemType;
use crate::{parse_item_id, whole_number, Problem};

/// The JSON Schema of format 0.1, as text. It is the same file as
/// `schema/artifact/0.1.json` in the repository.
pub const SCHEMA: &str = include_str!("../schema/0.1.json");

/// The `$schema` URL of format 0.1.
pub const SCHEMA_URL: &str = "https://openskopos.org/schema/artifact/0.1.json";

fn validator() -> &'static Validator {
    static VALIDATOR: OnceLock<Validator> = OnceLock::new();
    VALIDATOR.get_or_init(|| {
        let schema: Value = serde_json::from_str(SCHEMA).expect("the embedded schema is JSON");
        jsonschema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .should_validate_formats(true)
            .build(&schema)
            .expect("the embedded schema compiles")
    })
}

/// Finds the problems that the JSON Schema finds. Each problem has no rule
/// name.
///
/// Where the message of the validator is hard to read, the problem has a
/// clearer message. One wrong field of an item does not give a second problem
/// about the other fields of the item.
pub fn check_schema(doc: &Value) -> Vec<Problem> {
    let mut seen = HashSet::new();
    validator()
        .iter_errors(doc)
        .filter_map(|e| explain(doc, &e))
        .filter(|p| seen.insert(p.clone()))
        .collect()
}

const ITEM_COMMON: [&str; 9] = [
    "id",
    "type",
    "text",
    "status",
    "refs",
    "evidence",
    "proposed_by",
    "ratification",
    "detail",
];
const LONGEST_MESSAGE: usize = 300;

/// A problem for one validation error, or `None` if another error says the
/// same thing more clearly.
fn explain(doc: &Value, e: &ValidationError) -> Option<Problem> {
    let path = e.instance_path.to_string();
    let segments: Vec<&str> = path.split('/').skip(1).collect();
    let item = match segments.as_slice() {
        ["items", i, ..] => i
            .parse::<usize>()
            .ok()
            .and_then(|i| doc.get("items")?.get(i)),
        _ => None,
    };
    let item_type = item
        .and_then(|i| i.get("type"))
        .and_then(Value::as_str)
        .and_then(|t| t.parse::<ItemType>().ok());

    let message = match (&e.kind, segments.as_slice()) {
        (ValidationErrorKind::UnevaluatedProperties { unexpected }, ["items", _]) => {
            // When a field of the item's own type is wrong, the validator also
            // reports the fields of that type as unexpected. Report only keys
            // that the type does not have.
            let item_type = item_type?;
            let extra: Vec<&String> = unexpected
                .iter()
                .filter(|k| !ITEM_COMMON.contains(&k.as_str()) && !item_type.fields().contains(&k.as_str()))
                .collect();
            if extra.is_empty() {
                return None;
            }
            let hint = if extra.iter().any(|k| !ItemType::ALL.iter().any(|t| t.fields().contains(&k.as_str()))) {
                "; keys of your own must start with x-"
            } else {
                ""
            };
            format!("a {item_type} item cannot have {}{hint}", keys(&extra))
        }
        (ValidationErrorKind::AdditionalProperties { unexpected }, []) => {
            let extra: Vec<&String> = unexpected.iter().collect();
            format!("the document cannot have {}; keys of your own must start with x-", keys(&extra))
        }
        (ValidationErrorKind::AdditionalProperties { unexpected }, _) => {
            let extra: Vec<&String> = unexpected.iter().collect();
            format!("this object cannot have {}", keys(&extra))
        }
        (ValidationErrorKind::Not { .. }, ["items", _]) if item_type == Some(ItemType::Constraint) => {
            "only an ask_first constraint can have approver or bring".to_string()
        }
        // The schema checks the parents of version 1 with `not contains`, and
        // the parents of a later version with `contains`.
        (ValidationErrorKind::Not { .. }, ["parents"]) => {
            "version 1 cannot have a previous_version parent".to_string()
        }
        (ValidationErrorKind::Contains, ["parents"]) => match doc.get("version").and_then(whole_number) {
            Some(v) => format!("version {v} must have exactly one previous_version parent"),
            None => "a version after 1 must have exactly one previous_version parent".to_string(),
        },
        (
            ValidationErrorKind::OneOfNotValid { .. } | ValidationErrorKind::OneOfMultipleValid { .. },
            ["parents", _],
        ) => "a parent must be a version of an artifact (rel, artifact and version) or external material \
              (rel input and source)"
            .to_string(),
        (ValidationErrorKind::Pattern { .. }, ["items", _, "id"]) => {
            let id = e.instance.as_str().unwrap_or_default();
            match (parse_item_id(id), item_type) {
                (Some(_), Some(t)) => format!(
                    "the ID {id} does not fit a {t} item: the ID of a {t} item starts with {}, for example {}1",
                    t.prefix(),
                    t.prefix()
                ),
                _ => format!(
                    "{id:?} is not an item ID: an item ID is a type prefix and a positive integer with no \
                     leading zero, for example FR3"
                ),
            }
        }
        _ => e.to_string(),
    };
    Some(Problem::new(path, shorten(message)))
}

/// `the key a` or `the keys a and b`.
fn keys(names: &[&String]) -> String {
    match names {
        [one] => format!("the key {one}"),
        [rest @ .., last] => {
            let rest: Vec<&str> = rest.iter().map(|s| s.as_str()).collect();
            format!("the keys {} and {last}", rest.join(", "))
        }
        [] => "these keys".to_string(),
    }
}

/// Cuts a long message, because some messages contain a whole object.
fn shorten(message: String) -> String {
    if message.chars().count() <= LONGEST_MESSAGE {
        return message;
    }
    let mut short: String = message.chars().take(LONGEST_MESSAGE).collect();
    short.push_str("...");
    short
}
