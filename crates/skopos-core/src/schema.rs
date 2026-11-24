//! Validation against the JSON Schema of format 0.1.

use std::collections::HashSet;
use std::sync::OnceLock;

use jsonschema::{error::ValidationErrorKind, ValidationError, Validator};
use serde_json::Value;

use crate::fix::{either, item_label, new_id_fix, Fix, Op};
use crate::model::ItemType;
use crate::{parse_item_id, pointer, whole_number, Problem};

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
        .filter(|p| seen.insert((p.path.clone(), p.message.clone())))
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
    let (hint, fix) = advise(doc, e, &segments, item_type);
    let problem = Problem::new(path, shorten(message));
    Some(
        match hint {
            Some(h) => problem.with_hint(h),
            None => problem,
        }
        .with_fix(fix),
    )
}

/// A path in words: `refs/0` is `ref 1`, `session` is `session`.
fn place(segments: &[&str]) -> String {
    let mut parts: Vec<String> = Vec::new();
    for s in segments {
        match (s.parse::<usize>(), parts.last_mut()) {
            (Ok(n), Some(last)) => {
                let one = match last.as_str() {
                    "refs" | "parents" | "participants" | "examples" | "items" | "options"
                    | "scope" => last.trim_end_matches('s').to_string(),
                    other => other.to_string(),
                };
                *last = format!("{one} {}", n + 1);
            }
            _ => parts.push(s.replace('_', " ")),
        }
    }
    parts.join(", ")
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map_or_else(String::new, |f| f.to_uppercase().chain(c).collect())
}

/// The values of a field with a fixed list, for a hint.
fn values_of(field: &str) -> Vec<String> {
    if field == "type" {
        return ItemType::ALL
            .iter()
            .map(|t| t.as_str().to_string())
            .collect();
    }
    crate::markdown::enum_values(field).to_vec()
}

/// The hint and the fix for a validation error. `None` for the hint keeps
/// the message as the hint.
fn advise(
    doc: &Value,
    e: &ValidationError,
    segments: &[&str],
    item_type: Option<ItemType>,
) -> (Option<String>, Option<Fix>) {
    let item = match segments {
        ["items", i, ..] => i.parse::<usize>().ok(),
        _ => None,
    };
    // Who has the problem: `The document`, `FR3`, `Ref 1 of FR3`,
    // `Parent 2` or `The session`.
    let owner = |rest: &[&str]| -> String {
        let (inner, of) = match (rest, item) {
            (["items", _, inner @ ..], Some(i)) => (inner, Some(item_label(doc, i))),
            _ => (rest, None),
        };
        let name = place(inner);
        match (name.is_empty(), of) {
            (true, Some(label)) => label,
            (true, None) => "The document".to_string(),
            (false, Some(label)) if name.contains(char::is_numeric) => {
                format!("{} of {label}", capital(&name))
            }
            (false, Some(label)) => format!("The {name} of {label}"),
            (false, None) if inner.len() == 1 => format!("The {name}"),
            (false, None) => capital(&name),
        }
    };
    // `Item 4` and `The document` in the middle of a sentence. IDs stay.
    let mid = |who: &str| match who.split_once(' ') {
        Some((first @ ("Item" | "The"), rest)) => format!("{} {rest}", first.to_lowercase()),
        _ => who.to_string(),
    };
    let fix = |label: String, op: Op| {
        Some(Fix {
            label,
            patch: vec![op],
        })
    };
    let here: String = segments.iter().map(|s| format!("/{s}")).collect();
    let at = |field: &str| pointer(&here, field);
    match &e.kind {
        ValidationErrorKind::Required { property } => {
            let field = property.as_str().unwrap_or_default();
            let who = owner(segments);
            let values = match (segments, field) {
                ([], "status") => vec!["draft".to_string(), "final".to_string()],
                _ => values_of(field),
            };
            let hint = if segments.is_empty() && field == "finalized_at" {
                "A final version needs finalized_at: the time it became final.".to_string()
            } else if values.is_empty() || field == "kind" {
                format!("{who} has no {field}.")
            } else {
                format!("{who} needs a {field}: {}.", either(&values))
            };
            let fix = match (segments, field) {
                ([], "$schema") => {
                    fix("Add $schema".into(), Op::add("/$schema", SCHEMA_URL.into()))
                }
                ([], "status") => fix(
                    "Set the status to draft".into(),
                    Op::add("/status", "draft".into()),
                ),
                ([], "items") => fix(
                    "Add an empty list of items".into(),
                    Op::add("/items", Value::Array(vec![])),
                ),
                (["items", _], "status") => fix(
                    format!("Set the status of {who} to proposed"),
                    Op::add(at("status"), "proposed".into()),
                ),
                (["items", _], "id") => match (item, item_type) {
                    (Some(i), Some(t)) => Some(new_id_fix(doc, i, t.prefix(), |id| {
                        format!("Give {} the ID {id}", mid(&who))
                    })),
                    _ => None,
                },
                _ => None,
            };
            (Some(hint), fix)
        }
        ValidationErrorKind::Enum { options } => {
            let options: Vec<String> = options
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|o| o.as_str().map(str::to_string))
                .collect();
            let field = segments.last().copied().unwrap_or_default();
            let who = owner(&segments[..segments.len().saturating_sub(1)]);
            let value = match e.instance.as_ref() {
                Value::String(v) => v.clone(),
                other => other.to_string(),
            };
            let hint = format!("{who} has the {field} {value:?}. Use {}.", either(&options));
            // "Accepted" or "ask first" means one value for sure.
            let fold = |s: &str| s.trim().to_lowercase().replace([' ', '-'], "_");
            let fix = e.instance.as_str().and_then(|v| {
                let same = options.iter().find(|o| fold(o) == fold(v))?;
                fix(
                    format!("Change the {field} of {} to {same}", mid(&who)),
                    Op::replace(here.clone(), same.as_str().into()),
                )
            });
            (Some(hint), fix)
        }
        ValidationErrorKind::Format { format } if format == "date" || format == "date-time" => {
            let field = segments.last().copied().unwrap_or_default();
            let example = if format == "date" {
                "a date such as 2026-12-01"
            } else {
                "a date and time in UTC such as 2026-11-24T09:00:00Z"
            };
            (
                Some(format!(
                    "{} has the {field} {}. Use {example}.",
                    owner(&segments[..segments.len().saturating_sub(1)]),
                    e.instance
                )),
                None,
            )
        }
        ValidationErrorKind::Pattern { .. } => {
            let field = segments.last().copied().unwrap_or_default();
            let value = e.instance.as_str().unwrap_or_default();
            let hint = match segments {
                [] | ["$schema"] => format!(
                    "This tool reads format 0.1. If the document is format 0.1, set $schema to {SCHEMA_URL}."
                ),
                ["items", _, "id"] => match (parse_item_id(value), item_type) {
                    (Some(_), Some(t)) => format!(
                        "The ID {value} does not fit a {t} item. The ID of a {t} item starts with {}, such as {}1.",
                        t.prefix(),
                        t.prefix()
                    ),
                    _ => format!(
                        "The ID {value:?} is not an item ID. An item ID is a type prefix and a number with no leading zero, such as FR3."
                    ),
                },
                _ if field == "id" || field == "artifact" => format!(
                    "{} has the {field} {value:?}. An artifact ID is pa_ and 16 to 32 letters and digits, such as pa_7Hk2mQ9xR4tV8wZ1.",
                    owner(&segments[..segments.len() - 1])
                ),
                _ => format!(
                    "{} has the {field} {value:?}, which is not in the right form.",
                    owner(&segments[..segments.len().saturating_sub(1)])
                ),
            };
            (Some(hint), None)
        }
        ValidationErrorKind::PropertyNames { .. } if segments == ["id_counters"] => {
            let prefixes: Vec<String> = ItemType::ALL
                .iter()
                .map(|t| t.prefix().to_string())
                .collect();
            let unknown: Vec<String> = doc["id_counters"]
                .as_object()
                .into_iter()
                .flat_map(|m| m.keys())
                .filter(|k| !prefixes.contains(k))
                .cloned()
                .collect();
            (
                Some(format!(
                    "There is a counter for {} in id_counters, but only item types have counters. The prefixes are {}.",
                    either(&unknown),
                    either(&prefixes)
                )),
                None,
            )
        }
        ValidationErrorKind::Minimum { limit } if segments.first() == Some(&"id_counters") => (
            Some(format!(
                "The counter for {} is {}. It must be {limit} or more.",
                segments.last().copied().unwrap_or_default(),
                e.instance
            )),
            None,
        ),
        ValidationErrorKind::AdditionalProperties { unexpected }
            if (!segments.is_empty() && item.is_none()) || segments.len() > 2 =>
        {
            let extra: Vec<&String> = unexpected.iter().collect();
            (
                Some(format!("{} cannot have {}.", owner(segments), keys(&extra))),
                None,
            )
        }
        ValidationErrorKind::Not { .. } if item_type == Some(ItemType::Constraint) => (
            Some(format!(
                "{} is not an ask_first constraint, so it cannot have approver or bring.",
                owner(segments)
            )),
            None,
        ),
        ValidationErrorKind::Minimum { limit } => (
            Some(format!(
                "{} has the {} {}. It must be {limit} or more.",
                owner(&segments[..segments.len().saturating_sub(1)]),
                segments.last().copied().unwrap_or("value"),
                e.instance
            )),
            None,
        ),
        ValidationErrorKind::MinItems { limit } => (
            Some(format!(
                "{} needs at least {limit} {}.",
                owner(&segments[..segments.len().saturating_sub(1)]),
                segments.last().copied().unwrap_or("value")
            )),
            None,
        ),
        ValidationErrorKind::MinLength { .. } if segments.last() == Some(&"text") => (
            Some(format!(
                "{} has no text. Write one plain statement.",
                owner(&segments[..segments.len() - 1])
            )),
            None,
        ),
        ValidationErrorKind::Format { format } if format == "date-time" => (
            Some(format!(
                "{} must be a date and time in UTC, such as 2026-11-24T09:00:00Z.",
                segments.last().copied().unwrap_or("the value")
            )),
            None,
        ),
        _ => (None, None),
    }
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
