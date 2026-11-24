//! Readable views of a document, for people.
//!
//! JSON, YAML and Markdown are the formats that tools read and write
//! ([`crate::write`] and [`crate::markdown`]). This module makes views that
//! people read:
//!
//! - [`html()`] gives semantic HTML with stable `sk-` class names and an
//!   optional default stylesheet ([`STYLESHEET`]) that uses CSS custom
//!   properties.
//! - [`terminal()`] gives text for a terminal, with colors or as plain text.
//!
//! Both views show the same things in the same order: the title and summary,
//! one line per item with its ID, the items by type in the order of the
//! specification, and the fields of each item as details. With a previous
//! version, they also mark the items that were added and changed, show the
//! changed words of each text, and list the items that were removed.
//!
//! The functions render any value, also a document that is not valid. They
//! leave out what they cannot show.
//!
//! ```
//! use skopos_core::render::{html, HtmlOptions};
//! let doc = serde_json::json!({
//!   "id": "pa_7Hk2mQ9xR4tV8wZ1", "version": 1, "kind": "intent",
//!   "title": "Faster invoices", "status": "draft",
//!   "items": [{"id": "G1", "type": "goal", "text": "Invoices go out on day one.", "status": "accepted"}],
//! });
//! let page = html(&doc, &HtmlOptions { fragment: true, ..Default::default() });
//! assert!(page.starts_with("<article class=\"sk-doc\""));
//! assert!(page.contains("<li class=\"sk-item sk-item--accepted\" id=\"G1\""));
//! ```

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use crate::diff;
use crate::markdown::{item_type, kind, ref_string, Kind};
use crate::model::ItemType;
use crate::words::{word_diff, Op, Span};
use crate::{one_line, whole_number};

mod html;
mod terminal;

pub use html::{html, HtmlOptions, STYLESHEET};
pub use terminal::{terminal, TerminalOptions};

// ------------------------------------------------------------------ view

/// What both renderers show: the document, ready to lay out.
pub(crate) struct View {
    pub artifact: String,
    pub version: Option<u64>,
    pub kind: String,
    pub status: String,
    pub title: String,
    /// Paragraphs.
    pub summary: Vec<String>,
    /// `17 items: 12 accepted, 4 proposed, 1 rejected`.
    pub overview: String,
    pub sections: Vec<Section>,
    /// Facts about the version, such as the participants and the parents.
    pub meta: Vec<Field>,
    /// Set when the view compares with a previous version.
    pub since: Option<Since>,
}

pub(crate) struct Since {
    /// `Since version 2: 6 added, 5 changed, 1 superseded.`
    pub line: String,
    /// `version 2` or `the previous version`.
    pub what: String,
    /// The counts in `line` that are not 0, such as `(6, "added")`.
    pub counts: Vec<(usize, &'static str)>,
    /// A note when the two documents are different artifacts.
    pub note: Option<String>,
    /// The envelope fields that changed, such as `summary`.
    pub envelope: Vec<String>,
    /// Items of the previous version that this version does not have.
    pub removed: Vec<Removed>,
    /// The version number of the previous version.
    pub version: Option<u64>,
}

pub(crate) struct Removed {
    pub id: String,
    pub item_type: String,
    pub text: String,
    /// Empty for a dropped item.
    pub by: Vec<String>,
}

pub(crate) struct Section {
    /// `requirement`, or empty for items with no known type.
    pub item_type: String,
    pub name: String,
    pub items: Vec<ItemView>,
}

pub(crate) struct ItemView {
    pub id: String,
    pub item_type: String,
    pub status: String,
    pub text: Vec<Span>,
    /// Short values shown on the item line, such as the priority.
    pub badges: Vec<Badge>,
    pub fields: Vec<Field>,
    /// Paragraphs of the `detail` field.
    pub detail: Vec<String>,
    /// IDs of the items of this document that supersede this item.
    pub superseded_by: Vec<String>,
    pub change: Option<Change>,
}

impl ItemView {
    pub fn has_details(&self) -> bool {
        !self.fields.is_empty() || !self.detail.is_empty()
    }
}

/// How an item changed since the previous version.
pub(crate) enum Change {
    Added,
    Changed {
        /// The changed fields other than `text`.
        fields: Vec<String>,
        /// True if the text changed.
        text: bool,
        status_from: Option<String>,
    },
}

impl Change {
    pub fn name(&self) -> &'static str {
        match self {
            Change::Added => "added",
            Change::Changed { .. } => "changed",
        }
    }
}

pub(crate) struct Badge {
    /// The field, such as `priority`, or `status`.
    pub field: &'static str,
    /// The value as in the document, such as `ask_first`.
    pub value: String,
    /// The value for people, such as `ask first`.
    pub label: String,
}

/// A field and its values. A field with one value shows it on the same line.
pub(crate) struct Field {
    pub name: String,
    pub values: Vec<Line>,
    /// True if the values are a list, also when there is one value.
    pub list: bool,
}

pub(crate) type Line = Vec<Part>;

pub(crate) enum Part {
    Text(String),
    /// A path, a line number or a snippet of code.
    Code(String),
    /// A web link. The renderers link only `http`, `https` and `mailto`.
    Link {
        text: String,
        href: String,
    },
    /// A reference string. `anchor` is the item ID when the item is in this
    /// document.
    Ref {
        text: String,
        anchor: Option<String>,
    },
}

// ------------------------------------------------------------------ build

fn str_of<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

/// A value as text: a string as it is, a whole number without `.0`, and
/// anything else as JSON.
fn text_of(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(_) => whole_number(v).map_or_else(|| v.to_string(), |n| n.to_string()),
        other => other.to_string(),
    }
}

/// An enum value or a field name for people: `ask_first` is `ask first`.
pub(crate) fn words(name: &str) -> String {
    name.replace('_', " ")
}

/// The first letter in upper case.
pub(crate) fn capital(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// Paragraphs of a text: blocks between blank lines, each on one line.
fn paragraphs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur: Vec<&str> = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            if !cur.is_empty() {
                out.push(one_line(&cur.join(" ")));
                cur.clear();
            }
        } else {
            cur.push(line);
        }
    }
    if !cur.is_empty() {
        out.push(one_line(&cur.join(" ")));
    }
    out
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}

/// The fields that are shown on the item line, not in the details.
const BADGES: [&str; 6] = [
    "priority",
    "modality",
    "severity",
    "confirmation",
    "blocking",
    "deferred",
];

struct Builder<'a> {
    /// The artifact ID of the document.
    artifact: &'a str,
    /// Participant names by ID.
    people: BTreeMap<String, String>,
    /// The IDs that have an element in the view.
    anchors: BTreeSet<String>,
}

impl Builder<'_> {
    /// A participant ID as the name, or the value as it is.
    fn person(&self, v: &Value) -> String {
        let id = text_of(v);
        self.people.get(&id).cloned().unwrap_or(id)
    }

    /// A ref object as a part that links to the item when it is in this
    /// document.
    fn reference(&self, r: &Value) -> Part {
        let text = ref_string(r);
        let item = r.get("item").and_then(Value::as_str);
        let here = r
            .get("artifact")
            .and_then(Value::as_str)
            .is_none_or(|a| a == self.artifact);
        let anchor = item
            .filter(|i| here && self.anchors.contains(*i))
            .map(str::to_string);
        Part::Ref { text, anchor }
    }

    fn value_line(&self, field: &str, v: &Value) -> Line {
        match field {
            "approver" | "owner" | "decided_by" | "proposed_by" => {
                vec![Part::Text(self.person(v))]
            }
            _ => vec![Part::Text(one_line(&text_of(v)))],
        }
    }

    fn fields(&self, item: &Value) -> Vec<Field> {
        let mut out = Vec::new();
        let get = |k: &str| item.get(k);
        let type_fields = item_type(item).map_or(&[][..], ItemType::fields);
        for &field in type_fields {
            if BADGES.contains(&field) {
                continue;
            }
            let Some(v) = get(field) else { continue };
            let one = |line: Line| Field {
                name: words(field),
                values: vec![line],
                list: false,
            };
            match kind(field) {
                Kind::Text => out.push(one(self.value_line(field, v))),
                Kind::Enum | Kind::Bool => out.push(one(vec![Part::Text(words(&text_of(v)))])),
                Kind::List => out.push(Field {
                    name: words(field),
                    values: v
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|x| vec![Part::Text(one_line(&text_of(x)))])
                        .collect(),
                    list: true,
                }),
                Kind::Examples => out.push(Field {
                    name: "examples".into(),
                    values: v
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|ex| {
                            let mut line = Vec::new();
                            if let Some(k) = ex.get("kind").and_then(Value::as_str) {
                                line.push(Part::Text(format!("{}: ", words(k))));
                            }
                            line.push(Part::Text(one_line(str_of(ex, "text"))));
                            line
                        })
                        .collect(),
                    list: true,
                }),
                Kind::Target => {
                    let mut line = Vec::new();
                    if v.get("item").is_some() || v.get("artifact").is_some() {
                        line.push(self.reference(v));
                    }
                    if let Some(s) = v.get("section").and_then(Value::as_str) {
                        if !line.is_empty() {
                            line.push(Part::Text(", ".into()));
                        }
                        line.push(Part::Text(format!("section {}", one_line(s))));
                    }
                    out.push(one(line));
                }
                Kind::Resolution => {
                    let mut s = words(str_of(v, "outcome"));
                    if let Some(note) = v.get("note").and_then(Value::as_str) {
                        s.push_str(": ");
                        s.push_str(&one_line(note));
                    }
                    if let Some(by) = v.get("decided_by") {
                        s.push_str(&format!(" (decided by {})", self.person(by)));
                    }
                    out.push(one(vec![Part::Text(s)]));
                }
            }
        }
        // Refs, one field for each relation, in the order of first use.
        let mut by_rel: Vec<(String, Vec<Line>)> = Vec::new();
        for r in get("refs").and_then(Value::as_array).into_iter().flatten() {
            let rel = words(r.get("rel").and_then(Value::as_str).unwrap_or("refers to"));
            let line = vec![self.reference(r)];
            match by_rel.iter_mut().find(|(n, _)| *n == rel) {
                Some((_, lines)) => lines.push(line),
                None => by_rel.push((rel, vec![line])),
            }
        }
        for (name, values) in by_rel {
            out.push(Field {
                name,
                values,
                list: false,
            });
        }
        let evidence: Vec<Line> = get("evidence")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|ev| self.evidence(ev))
            .collect();
        if !evidence.is_empty() {
            out.push(Field {
                name: "evidence".into(),
                values: evidence,
                list: true,
            });
        }
        if let Some(p) = get("proposed_by") {
            out.push(Field {
                name: "proposed by".into(),
                values: vec![vec![Part::Text(self.person(p))]],
                list: false,
            });
        }
        if let Some(r) = get("ratification") {
            let n = |k: &str| r.get(k).map(text_of).unwrap_or_default();
            out.push(Field {
                name: "votes".into(),
                values: vec![vec![Part::Text(format!(
                    "{} yes, {} no, {} eligible",
                    n("yes"),
                    n("no"),
                    n("eligible")
                ))]],
                list: false,
            });
        }
        if let Some(map) = item.as_object() {
            out.extend(extensions(map));
        }
        out
    }

    fn evidence(&self, ev: &Value) -> Line {
        let get = |k: &str| ev.get(k).and_then(Value::as_str);
        let mut line = Vec::new();
        match get("kind").unwrap_or("") {
            "quote" => {
                line.push(Part::Text(format!(
                    "\u{201c}{}\u{201d}",
                    one_line(get("text").unwrap_or(""))
                )));
                let from = ev.get("from");
                let source =
                    from.filter(|f| f.get("artifact").is_some() || f.get("item").is_some());
                let title = from.and_then(|f| f.get("title")).and_then(Value::as_str);
                if source.is_some() || title.is_some() {
                    line.push(Part::Text(" from ".into()));
                    if let Some(f) = source {
                        line.push(self.reference(f));
                        if let Some(t) = title {
                            line.push(Part::Text(format!(" ({})", one_line(t))));
                        }
                    } else if let Some(t) = title {
                        line.push(Part::Text(one_line(t)));
                    }
                }
                if let Some(loc) = get("location") {
                    line.push(Part::Text(format!(", {}", one_line(loc))));
                }
            }
            "discussion" => {
                if let Some(s) = ev.get("speaker") {
                    line.push(Part::Text(format!("{}: ", self.person(s))));
                }
                line.push(Part::Text(format!(
                    "\u{201c}{}\u{201d}",
                    one_line(get("text").unwrap_or(""))
                )));
            }
            "code" => {
                let mut path = get("path").unwrap_or("").to_string();
                let n = |k: &str| ev.get(k).map(text_of);
                match (n("line_start"), n("line_end")) {
                    (Some(s), Some(e)) => path.push_str(&format!(":{s}-{e}")),
                    (Some(s), None) => path.push_str(&format!(":{s}")),
                    _ => {}
                }
                line.push(Part::Code(path));
                if let Some(repo) = get("repo") {
                    line.push(Part::Text(format!(" in {repo}")));
                }
                if let Some(c) = get("commit") {
                    line.push(Part::Text(" at ".into()));
                    line.push(Part::Code(c.to_string()));
                }
                if let Some(s) = get("snippet") {
                    line.push(Part::Text(": ".into()));
                    line.push(Part::Code(one_line(s)));
                }
            }
            "link" => {
                let uri = get("uri").unwrap_or("").to_string();
                let text = get("title").map_or_else(|| uri.clone(), one_line);
                line.push(Part::Link { text, href: uri });
            }
            other => line.push(Part::Text(format!("{other}: {}", text_of(ev)))),
        }
        line
    }

    fn badges(item: &Value) -> Vec<Badge> {
        let mut out = Vec::new();
        let status = str_of(item, "status");
        if !status.is_empty() && status != "accepted" {
            out.push(Badge {
                field: "status",
                value: status.into(),
                label: words(status),
            });
        }
        for field in BADGES {
            let Some(v) = item.get(field) else { continue };
            let label = match (field, v) {
                (_, Value::Bool(true)) => words(field),
                (_, Value::Bool(false)) => continue,
                ("severity", _) => format!("{} severity", words(&text_of(v))),
                _ => words(&text_of(v)),
            };
            out.push(Badge {
                field,
                value: text_of(v),
                label,
            });
        }
        out
    }

    /// Facts about the version, at most five rows: reference, created,
    /// people, session and sources. `x-` keys are for tools, so the view
    /// leaves them out.
    fn meta(&self, map: &Map<String, Value>) -> Vec<Field> {
        let mut out = Vec::new();
        let mut add = |name: &str, line: Line| {
            out.push(Field {
                name: name.into(),
                values: vec![line],
                list: false,
            })
        };
        if let Some(id) = map.get("id").and_then(Value::as_str) {
            let version = map.get("version").and_then(whole_number);
            add(
                "reference",
                vec![Part::Code(crate::version_label(id, version))],
            );
        }
        if let Some(t) = map.get("created_at").and_then(Value::as_str) {
            let mut s = day(t).to_string();
            if let Some(f) = map.get("finalized_at").and_then(Value::as_str) {
                s.push_str(&format!(", finalized {}", day(f)));
            }
            add("created", vec![Part::Text(s)]);
        }
        let decider = map.get("decider").and_then(Value::as_str);
        let mut people: Vec<String> = Vec::new();
        if let Some(d) = map.get("decider") {
            people.push(format!("{} (decider)", self.person(d)));
        }
        for p in map
            .get("participants")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if decider.is_some() && p.get("id").and_then(Value::as_str) == decider {
                continue;
            }
            let name = one_line(str_of(p, "name"));
            people.push(if str_of(p, "kind") == "ai" {
                format!("{name} (AI)")
            } else {
                name
            });
        }
        if !people.is_empty() {
            add("people", vec![Part::Text(people.join(", "))]);
        }
        let mut session: Line = Vec::new();
        if let Some(s) = map.get("session") {
            let title = one_line(
                s.get("title")
                    .and_then(Value::as_str)
                    .unwrap_or(str_of(s, "id")),
            );
            session.push(match s.get("url").and_then(Value::as_str) {
                Some(url) => Part::Link {
                    text: title,
                    href: url.to_string(),
                },
                None => Part::Text(title),
            });
        }
        if let Some(m) = map.get("method") {
            let mut s = str_of(m, "id").to_string();
            if let Some(v) = m.get("version").and_then(Value::as_str) {
                s.push_str(&format!(" {v}"));
            }
            if !session.is_empty() {
                s.insert_str(0, " \u{b7} ");
            }
            session.push(Part::Text(s));
        }
        if !session.is_empty() {
            add("session", session);
        }
        let parents: Vec<Line> = map
            .get("parents")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|p| self.parent(p))
            .collect();
        if !parents.is_empty() {
            out.push(Field {
                name: "sources".into(),
                values: parents,
                list: true,
            });
        }
        out
    }

    fn parent(&self, p: &Value) -> Line {
        let rel = capital(&words(str_of(p, "rel")));
        let mut line = vec![Part::Text(format!("{rel}: "))];
        if let Some(src) = p.get("source") {
            let title = one_line(str_of(src, "title"));
            match src.get("uri").and_then(Value::as_str) {
                Some(uri) => line.push(Part::Link {
                    text: title,
                    href: uri.to_string(),
                }),
                None => line.push(Part::Text(title)),
            }
            let mut place = String::new();
            if let Some(repo) = src.get("repo").and_then(Value::as_str) {
                place.push_str(repo);
            }
            if let Some(path) = src.get("path").and_then(Value::as_str) {
                if !place.is_empty() {
                    place.push(' ');
                }
                place.push_str(path);
            }
            if !place.is_empty() {
                line.push(Part::Text(" ".into()));
                line.push(Part::Code(place));
            }
        } else {
            if let Some(t) = p.get("title").and_then(Value::as_str) {
                line.push(Part::Text(format!("{} ", one_line(t))));
            }
            let id = str_of(p, "artifact");
            let version = p.get("version").and_then(whole_number);
            line.push(Part::Code(crate::version_label(id, version)));
        }
        line
    }
}

/// The date of a date and time, such as `2026-11-08`, or the value as it is.
fn day(t: &str) -> &str {
    match t.get(..10) {
        Some(d) if d.as_bytes()[4] == b'-' && d.as_bytes()[7] == b'-' => d,
        _ => t,
    }
}

/// The `x-` keys of an object as fields.
fn extensions(map: &Map<String, Value>) -> Vec<Field> {
    map.iter()
        .filter(|(k, _)| k.starts_with("x-"))
        .map(|(k, v)| Field {
            name: k.clone(),
            values: vec![vec![Part::Text(one_line(&text_of(v)))]],
            list: false,
        })
        .collect()
}

/// The view of a document, compared with a previous version if there is one.
pub(crate) fn view(doc: &Value, previous: Option<&Value>) -> View {
    let empty = Map::new();
    let map = doc.as_object().unwrap_or(&empty);
    let artifact = str_of(doc, "id");
    let items: Vec<&Value> = map
        .get("items")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|i| i.is_object())
        .collect();

    let d = previous.map(|p| diff::diff(p, doc));
    let removed: Vec<Removed> = d.as_ref().map_or_else(Vec::new, |d| {
        let sup = d.superseded.iter().map(|s| Removed {
            id: s.id.clone(),
            item_type: s.item_type.clone(),
            text: one_line(&s.text),
            by: s.by.clone(),
        });
        let dropped = d.dropped.iter().map(|s| Removed {
            id: s.id.clone(),
            item_type: s.item_type.clone(),
            text: one_line(&s.text),
            by: Vec::new(),
        });
        sup.chain(dropped).collect()
    });

    let mut anchors: BTreeSet<String> = items
        .iter()
        .map(|i| str_of(i, "id").to_string())
        .filter(|id| !id.is_empty())
        .collect();
    anchors.extend(removed.iter().map(|r| r.id.clone()));
    let people = map
        .get("participants")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|p| (str_of(p, "id").to_string(), one_line(str_of(p, "name"))))
        .filter(|(id, name)| !id.is_empty() && !name.is_empty())
        .collect();
    let b = Builder {
        artifact,
        people,
        anchors,
    };

    // Items of this document that an item that is not rejected supersedes.
    let mut superseded: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for item in &items {
        if str_of(item, "status") == "rejected" {
            continue;
        }
        for r in item
            .get("refs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let here = r
                .get("artifact")
                .and_then(Value::as_str)
                .is_none_or(|a| a == artifact);
            if here && str_of(r, "rel") == "supersedes" {
                superseded
                    .entry(str_of(r, "item").to_string())
                    .or_default()
                    .push(str_of(item, "id").to_string());
            }
        }
    }

    let old_items: BTreeMap<&str, &Value> = previous
        .and_then(|p| p.get("items"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|i| Some((i.get("id")?.as_str()?, i)))
        .collect();

    let item_view = |item: &Value| -> ItemView {
        let id = str_of(item, "id").to_string();
        let text = one_line(str_of(item, "text"));
        let change = d.as_ref().and_then(|d| {
            if d.added.iter().any(|a| a.id == id) {
                return Some(Change::Added);
            }
            d.changed
                .iter()
                .find(|c| c.id == id)
                .map(|c| Change::Changed {
                    // The changed words of the text show in the text itself.
                    fields: c.fields.iter().filter(|f| *f != "text").cloned().collect(),
                    text: c.fields.iter().any(|f| f == "text"),
                    status_from: c.status_from.clone(),
                })
        });
        let spans = match &change {
            Some(Change::Added) => vec![Span { op: Op::Ins, text }],
            Some(Change::Changed { text: true, .. }) => {
                let old = old_items
                    .get(id.as_str())
                    .map(|o| one_line(str_of(o, "text")))
                    .unwrap_or_default();
                word_diff(&old, &text)
            }
            _ => vec![Span { op: Op::Same, text }],
        };
        ItemView {
            item_type: str_of(item, "type").to_string(),
            status: str_of(item, "status").to_string(),
            text: spans,
            badges: Builder::badges(item),
            fields: b.fields(item),
            detail: paragraphs(str_of(item, "detail")),
            superseded_by: superseded.get(&id).cloned().unwrap_or_default(),
            change,
            id,
        }
    };

    let mut sections = Vec::new();
    for t in ItemType::ALL {
        let of_type: Vec<ItemView> = items
            .iter()
            .filter(|i| item_type(i) == Some(t))
            .map(|i| item_view(i))
            .collect();
        if !of_type.is_empty() {
            sections.push(Section {
                item_type: t.as_str().into(),
                name: t.section().into(),
                items: of_type,
            });
        }
    }
    let other: Vec<ItemView> = items
        .iter()
        .filter(|i| item_type(i).is_none())
        .map(|i| item_view(i))
        .collect();
    if !other.is_empty() {
        sections.push(Section {
            item_type: String::new(),
            name: "Other items".into(),
            items: other,
        });
    }

    let mut by_status: BTreeMap<&str, usize> = BTreeMap::new();
    for i in &items {
        *by_status.entry(str_of(i, "status")).or_default() += 1;
    }
    let counts: Vec<String> = ["accepted", "proposed", "rejected"]
        .iter()
        .filter_map(|s| by_status.get(s).map(|n| format!("{n} {s}")))
        .collect();
    let overview = if items.is_empty() {
        "No items yet.".to_string()
    } else if counts.is_empty() {
        format!("{}.", plural(items.len(), "item"))
    } else {
        format!("{}: {}.", plural(items.len(), "item"), counts.join(", "))
    };

    let since = d.map(|d| {
        let n_sup = d.superseded.len();
        let counts: Vec<(usize, &'static str)> = [
            (d.added.len(), "added"),
            (d.changed.len(), "changed"),
            (n_sup, "superseded"),
            (d.dropped.len(), "dropped"),
        ]
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .collect();
        let parts: Vec<String> = counts.iter().map(|(n, w)| format!("{n} {w}")).collect();
        let what = match d.from.version {
            Some(v) => format!("version {v}"),
            None => "the previous version".to_string(),
        };
        let line = if d.is_empty() {
            format!("No changes since {what}.")
        } else if parts.is_empty() {
            format!("Since {what}: no item changed.")
        } else {
            format!("Since {what}: {}.", parts.join(", "))
        };
        Since {
            line,
            what,
            counts,
            note: (!d.same_artifact).then(|| {
                "The two documents are different artifacts, so the same ID can mean different items."
                    .to_string()
            }),
            // `x-` keys are for tools, not for people.
            envelope: d
                .envelope
                .iter()
                .filter(|k| !k.starts_with("x-"))
                .cloned()
                .collect(),
            removed,
            version: d.from.version,
        }
    });

    View {
        artifact: artifact.to_string(),
        version: map.get("version").and_then(whole_number),
        kind: str_of(doc, "kind").to_string(),
        status: str_of(doc, "status").to_string(),
        title: one_line(str_of(doc, "title")),
        summary: paragraphs(str_of(doc, "summary")),
        overview,
        sections,
        meta: b.meta(map),
        since,
    }
}
