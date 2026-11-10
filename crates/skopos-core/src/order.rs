//! The canonical order of keys.
//!
//! Each object in a document has a shape, for example an item or a ref. Each
//! shape has a list of known keys. The canonical order is: the known keys in
//! the order of the list, then the `x-` keys, then any other keys. The last
//! two groups are in the order of the map, which is sorted by key unless
//! `serde_json` keeps the order of insertion.

use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};
use serde_json::{Map, Value};

use crate::model::ItemType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shape {
    Document,
    Method,
    Session,
    Participant,
    Parent,
    Source,
    IdCounters,
    Item,
    Example,
    Evidence,
    QuoteSource,
    Ref,
    Target,
    Resolution,
    Ratification,
    /// An object of unknown shape, for example the value of an `x-` key.
    Free,
}

pub(crate) const DOCUMENT: &[&str] = &[
    "$schema",
    "id",
    "version",
    "kind",
    "title",
    "summary",
    "status",
    "method",
    "session",
    "participants",
    "decider",
    "parents",
    "id_counters",
    "items",
    "created_at",
    "finalized_at",
];
const METHOD: &[&str] = &["id", "version"];
const SESSION: &[&str] = &["id", "title", "url", "ended_at"];
const PARTICIPANT: &[&str] = &["id", "name", "kind"];
const PARENT: &[&str] = &["rel", "artifact", "version", "kind", "title", "source"];
const SOURCE: &[&str] = &["kind", "title", "uri", "repo", "path", "commit", "digest"];
const ID_COUNTERS: &[&str] = &[
    "G", "S", "FR", "ST", "C", "A", "N", "R", "Q", "D", "T", "CR",
];
const ITEM_HEAD: &[&str] = &["id", "type", "text", "status"];
const ITEM_TAIL: &[&str] = &["refs", "evidence", "proposed_by", "ratification", "detail"];
const EXAMPLE: &[&str] = &["text", "kind"];
const EVIDENCE: &[&str] = &[
    "kind",
    "text",
    "from",
    "location",
    "repo",
    "path",
    "line_start",
    "line_end",
    "commit",
    "snippet",
    "speaker",
    "uri",
    "title",
];
const QUOTE_SOURCE: &[&str] = &["artifact", "version", "item", "title"];
const REF: &[&str] = &["item", "artifact", "version", "rel"];
const TARGET: &[&str] = &["artifact", "version", "item", "section"];
const RESOLUTION: &[&str] = &["outcome", "note", "decided_by"];
const RATIFICATION: &[&str] = &["yes", "no", "eligible"];

impl Shape {
    /// The shape of the value of a key, or of the elements of a list value.
    fn child(self, key: &str) -> Shape {
        match (self, key) {
            (Shape::Document, "method") => Shape::Method,
            (Shape::Document, "session") => Shape::Session,
            (Shape::Document, "participants") => Shape::Participant,
            (Shape::Document, "parents") => Shape::Parent,
            (Shape::Document, "id_counters") => Shape::IdCounters,
            (Shape::Document, "items") => Shape::Item,
            (Shape::Parent, "source") => Shape::Source,
            (Shape::Item, "examples") => Shape::Example,
            (Shape::Item, "evidence") => Shape::Evidence,
            (Shape::Item, "refs") => Shape::Ref,
            (Shape::Item, "target") => Shape::Target,
            (Shape::Item, "resolution") => Shape::Resolution,
            (Shape::Item, "ratification") => Shape::Ratification,
            (Shape::Evidence, "from") => Shape::QuoteSource,
            _ => Shape::Free,
        }
    }
}

/// The entries of an object in canonical order, each with the shape of its
/// value.
pub(crate) fn entries(map: &Map<String, Value>, shape: Shape) -> Vec<(&String, &Value, Shape)> {
    let list: &[&str] = match shape {
        Shape::Document => DOCUMENT,
        Shape::Method => METHOD,
        Shape::Session => SESSION,
        Shape::Participant => PARTICIPANT,
        Shape::Parent => PARENT,
        Shape::Source => SOURCE,
        Shape::IdCounters => ID_COUNTERS,
        Shape::Item => ITEM_HEAD,
        Shape::Example => EXAMPLE,
        Shape::Evidence => EVIDENCE,
        Shape::QuoteSource => QUOTE_SOURCE,
        Shape::Ref => REF,
        Shape::Target => TARGET,
        Shape::Resolution => RESOLUTION,
        Shape::Ratification => RATIFICATION,
        Shape::Free => &[],
    };
    // An item has its head, the fields of its type, and its tail.
    let (fields, tail): (&[&str], &[&str]) = match shape {
        Shape::Item => (
            map.get("type")
                .and_then(Value::as_str)
                .and_then(|t| t.parse::<ItemType>().ok())
                .map_or(&[], ItemType::fields),
            ITEM_TAIL,
        ),
        _ => (&[], &[]),
    };
    let known = [list, fields, tail];
    let is_known = |k: &str| known.iter().any(|part| part.contains(&k));

    let mut out = Vec::with_capacity(map.len());
    for k in known.into_iter().flatten() {
        if let Some((k, v)) = map.get_key_value(*k) {
            out.push((k, v, shape.child(k)));
        }
    }
    let rest = || map.iter().filter(|(k, _)| !is_known(k));
    let ext = rest().filter(|(k, _)| k.starts_with("x-"));
    let other = rest().filter(|(k, _)| !k.starts_with("x-"));
    out.extend(ext.chain(other).map(|(k, v)| (k, v, shape.child(k))));
    out
}

/// A value that serializes its objects in canonical key order.
pub(crate) struct Ordered<'a> {
    pub value: &'a Value,
    pub shape: Shape,
}

impl Serialize for Ordered<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.value {
            Value::Object(map) => {
                let mut m = serializer.serialize_map(Some(map.len()))?;
                for (k, v, shape) in entries(map, self.shape) {
                    m.serialize_entry(k, &Ordered { value: v, shape })?;
                }
                m.end()
            }
            Value::Array(list) => {
                let mut s = serializer.serialize_seq(Some(list.len()))?;
                for v in list {
                    s.serialize_element(&Ordered {
                        value: v,
                        shape: self.shape,
                    })?;
                }
                s.end()
            }
            other => other.serialize(serializer),
        }
    }
}
