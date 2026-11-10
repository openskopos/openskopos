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

const DOCUMENT: &[&str] = &[
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
    let known: Vec<&str> = match shape {
        Shape::Document => DOCUMENT.to_vec(),
        Shape::Method => METHOD.to_vec(),
        Shape::Session => SESSION.to_vec(),
        Shape::Participant => PARTICIPANT.to_vec(),
        Shape::Parent => PARENT.to_vec(),
        Shape::Source => SOURCE.to_vec(),
        Shape::IdCounters => ID_COUNTERS.to_vec(),
        Shape::Item => {
            let fields = map
                .get("type")
                .and_then(Value::as_str)
                .and_then(|t| t.parse::<ItemType>().ok())
                .map(ItemType::fields)
                .unwrap_or_default();
            [ITEM_HEAD, fields, ITEM_TAIL].concat()
        }
        Shape::Example => EXAMPLE.to_vec(),
        Shape::Evidence => EVIDENCE.to_vec(),
        Shape::QuoteSource => QUOTE_SOURCE.to_vec(),
        Shape::Ref => REF.to_vec(),
        Shape::Target => TARGET.to_vec(),
        Shape::Resolution => RESOLUTION.to_vec(),
        Shape::Ratification => RATIFICATION.to_vec(),
        Shape::Free => vec![],
    };
    let mut out: Vec<(&String, &Value, Shape)> = known
        .iter()
        .filter_map(|k| map.get_key_value(*k))
        .map(|(k, v)| (k, v, shape.child(k)))
        .collect();
    let rest = map.iter().filter(|(k, _)| !known.contains(&k.as_str()));
    let (ext, other): (Vec<_>, Vec<_>) = rest.partition(|(k, _)| k.starts_with("x-"));
    out.extend(
        ext.into_iter()
            .chain(other)
            .map(|(k, v)| (k, v, shape.child(k))),
    );
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
