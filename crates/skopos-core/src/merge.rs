//! Merge two documents into one, item by item, with conflicts written in the
//! format itself.
//!
//! An artifact has no branches (specification 8.1), so two drafts are two
//! documents. [`merge`] takes the document that the team picks as main and
//! the other document, and makes the next version of main:
//!
//! - It has a `previous_version` parent to main and an `input` parent to the
//!   other document.
//! - Each item of the other document that is new in the result gets a new ID
//!   from the counters of main, so IDs never collide, and a `derived_from`
//!   ref to where it came from. Refs to renamed items are rewritten.
//! - A conflict keeps both items, as `proposed`. The item of main keeps its
//!   ID. The item of the other document gets a new ID, a `derived_from` ref
//!   and a `contradicts` ref to the item of main. To resolve the conflict,
//!   the team accepts one item and rejects the other ([`resolve`]). The
//!   rejected item stays in the record.
//!
//! The function pairs the items of the two documents in one of three ways:
//!
//! - **With a base**, the common earlier version, an item of main and an
//!   item of the other document are the same item when both have an ID of
//!   the base. A field that only one side changed merges by itself. A field
//!   that both sides changed in different ways is a conflict. An item that
//!   one side changed and the other side removed is a conflict too.
//! - **With pairs** that people confirmed, as `(main ID, other ID)`.
//! - **Without either**, for example for two drafts that two sessions made
//!   from the same input, it pairs items of the same type with similar text,
//!   and returns the pairs so that people can confirm or split them and merge
//!   again with the pairs.
//!
//! Envelope fields such as `title` and `summary` keep the value of main. The
//! result names the fields where the other document is different.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as _};

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::diff::{different_keys, items_by_id, same_field, NOT_CONTENT};
use crate::model::ItemType;
use crate::words::{word_diff, Span};
use crate::{assign_ids, parse_item_id, Problem};

/// The least similarity, in percent, for two items to pair by text.
const PAIR_BY_TEXT: u32 = 50;

/// Options for [`merge`].
#[derive(Debug, Clone, Default)]
pub struct Options<'a> {
    /// The common earlier version of the two documents.
    pub base: Option<&'a Value>,
    /// Pairs of items that are the same item, as (ID in main, ID in other).
    /// When given, no other items pair.
    pub pairs: Option<Vec<(String, String)>>,
    /// The time for `created_at` of the result, for example
    /// `2026-11-24T12:00:00Z`. Without it, the result keeps the time of main.
    pub now: Option<String>,
}

/// How two items were paired.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PairedBy {
    /// The same ID in the base and in both documents.
    Id,
    /// The same type and similar text.
    Text,
    /// Given in [`Options::pairs`].
    Given,
}

/// An item of main and an item of the other document that are the same item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pair {
    /// The ID in main and in the result.
    pub main: String,
    /// The ID in the other document.
    pub other: String,
    pub by: PairedBy,
    /// For a pair by text: how similar the texts are, from 0 to 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub similarity: Option<u32>,
}

/// The kind of a conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictKind {
    /// Both documents changed the item in different ways.
    Changed,
    /// Main removed the item, and the other document changed it.
    RemovedInMain,
    /// The other document removed the item, and main changed it.
    RemovedInOther,
}

/// A conflict in the result. Its items are `proposed` until the team
/// decides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conflict {
    pub kind: ConflictKind,
    /// The ID in the result of the item of main, if there is one: for
    /// `removed_in_main`, the item of main that supersedes the removed item.
    pub main: Option<String>,
    /// The ID in the result of the item of the other document, if there is
    /// one: for `removed_in_other`, the item of the other document that
    /// supersedes the removed item.
    pub other: Option<String>,
    /// For `changed`: the fields that are different, in alphabetical order.
    pub fields: Vec<String>,
    /// If both items are in the result: the text of the item of main to the
    /// text of the item of the other document, word by word.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_spans: Option<Vec<Span>>,
    /// In a merge with a base: the text of the item in the base.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_text: Option<String>,
}

/// An item of the other document that has a new ID in the result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Renamed {
    /// The ID in the other document.
    pub from: String,
    /// The ID in the result.
    pub to: String,
}

/// The result of [`merge`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Merge {
    /// The next version of main, as a draft.
    pub doc: Value,
    /// The items that were paired, in the order of main.
    pub pairs: Vec<Pair>,
    /// In the order of the result.
    pub conflicts: Vec<Conflict>,
    /// In the order of the other document.
    pub renamed: Vec<Renamed>,
    /// Envelope fields where the other document is different. The result
    /// keeps the values of main.
    pub envelope: Vec<String>,
}

type Item = Map<String, Value>;

fn str_of<'a>(item: &'a Item, key: &str) -> &'a str {
    item.get(key).and_then(Value::as_str).unwrap_or("")
}

fn doc_ref(doc: &Value, what: &str) -> Result<(String, u64), Problem> {
    let id = doc
        .get("id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty());
    let version = doc.get("version").and_then(crate::whole_number);
    match (id, version) {
        (Some(id), Some(v)) => Ok((id.to_string(), v)),
        _ => Err(Problem::new(
            "",
            format!("the {what} document has no artifact ID or no version"),
        )),
    }
}

/// The words of a text, in lower case.
fn words(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// How similar two texts are, from 0 to 100: the shared words, twice, as a
/// part of all words (the Dice coefficient).
fn similarity(a: &str, b: &str) -> u32 {
    let (a, b) = (words(a), words(b));
    if a.is_empty() && b.is_empty() {
        return 100;
    }
    let shared = a.intersection(&b).count();
    (200 * shared / (a.len() + b.len())) as u32
}

fn has_ref(item: &Item, rel: &str, id: &str) -> bool {
    item.get("refs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|r| r.get("rel").and_then(Value::as_str) == Some(rel) && r["item"] == id)
}

fn push_ref(item: &mut Item, r: Value) {
    let refs = item
        .entry("refs")
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Value::Array(list) = refs {
        if !list.contains(&r) {
            list.push(r);
        }
    }
}

/// The items of a document that supersede `id` and are not rejected.
fn superseders<'a>(items: &[(&'a str, &Item)], id: &str) -> Vec<&'a str> {
    items
        .iter()
        .filter(|(_, i)| str_of(i, "status") != "rejected" && has_ref(i, "supersedes", id))
        .filter(|(_, i)| {
            // A local ref only.
            i["refs"].as_array().into_iter().flatten().any(|r| {
                r["item"] == id
                    && r.get("rel").and_then(Value::as_str) == Some("supersedes")
                    && r.get("artifact").is_none()
            })
        })
        .map(|(sid, _)| *sid)
        .collect()
}

/// Merges two documents. See the [module](self) documentation.
///
/// The function does not check the documents: use [`crate::check`] first.
/// It returns a problem if a document has no artifact ID or version, if an
/// item that needs a new ID has no known type, or if a given pair names an
/// item that does not exist or an item twice.
///
/// ```
/// use serde_json::json;
/// use skopos_core::merge::{merge, Options};
/// let main = json!({"id": "pa_1", "version": 1, "status": "final", "items": [
///     {"id": "FR1", "type": "requirement", "text": "Block the plan change.", "status": "accepted"}
/// ], "id_counters": {"FR": 1}});
/// let other = json!({"id": "pa_2", "version": 1, "status": "final", "items": [
///     {"id": "FR1", "type": "requirement", "text": "Email the admins after a downgrade.", "status": "accepted"}
/// ], "id_counters": {"FR": 1}});
/// let m = merge(&main, &other, &Options::default()).unwrap();
/// // Different texts do not pair, so the other item is new, as FR2.
/// assert_eq!(m.renamed[0].to, "FR2");
/// assert_eq!(m.doc["version"], 2);
/// ```
pub fn merge(main: &Value, other: &Value, options: &Options) -> Result<Merge, Problem> {
    let (main_artifact, main_version) = doc_ref(main, "main")?;
    let (other_artifact, other_version) = doc_ref(other, "other")?;
    let m_items = items_by_id(main);
    let o_items = items_by_id(other);
    let b_items = options.base.map(items_by_id).unwrap_or_default();
    let m_by: BTreeMap<&str, &Item> = m_items.iter().copied().collect();
    let o_by: BTreeMap<&str, &Item> = o_items.iter().copied().collect();
    let b_by: BTreeMap<&str, &Item> = b_items.iter().copied().collect();
    let three_way = options.base.is_some() && options.pairs.is_none();

    // Pairs, as other ID -> main ID.
    let mut pairs: Vec<Pair> = Vec::new();
    if let Some(given) = &options.pairs {
        let mut seen = BTreeSet::new();
        for (m, o) in given {
            if !m_by.contains_key(m.as_str()) {
                return Err(Problem::new("", format!("main has no item {m}")));
            }
            if !o_by.contains_key(o.as_str()) {
                return Err(Problem::new(
                    "",
                    format!("the other document has no item {o}"),
                ));
            }
            if !seen.insert(format!("m{m}")) || !seen.insert(format!("o{o}")) {
                return Err(Problem::new(
                    "",
                    format!("the pair {m} and {o} names an item that is in another pair"),
                ));
            }
            pairs.push(Pair {
                main: m.clone(),
                other: o.clone(),
                by: PairedBy::Given,
                similarity: None,
            });
        }
    } else if three_way {
        for (id, _) in &m_items {
            if b_by.contains_key(id) && o_by.contains_key(id) {
                pairs.push(Pair {
                    main: id.to_string(),
                    other: id.to_string(),
                    by: PairedBy::Id,
                    similarity: None,
                });
            }
        }
    } else {
        let mut candidates: Vec<(u32, usize, usize)> = Vec::new();
        for (mi, (_, m)) in m_items.iter().enumerate() {
            for (oi, (_, o)) in o_items.iter().enumerate() {
                if str_of(m, "type") != str_of(o, "type") {
                    continue;
                }
                let s = similarity(str_of(m, "text"), str_of(o, "text"));
                if s >= PAIR_BY_TEXT {
                    candidates.push((s, mi, oi));
                }
            }
        }
        // Best first. Ties go in the order of main, then of other.
        candidates.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        let (mut used_m, mut used_o) = (BTreeSet::new(), BTreeSet::new());
        let mut chosen = Vec::new();
        for (s, mi, oi) in candidates {
            if used_m.insert(mi) {
                if used_o.insert(oi) {
                    chosen.push((mi, oi, s));
                } else {
                    used_m.remove(&mi);
                }
            }
        }
        chosen.sort();
        for (mi, oi, s) in chosen {
            pairs.push(Pair {
                main: m_items[mi].0.to_string(),
                other: o_items[oi].0.to_string(),
                by: PairedBy::Text,
                similarity: Some(s),
            });
        }
    }
    let main_of: BTreeMap<&str, &str> = pairs
        .iter()
        .map(|p| (p.other.as_str(), p.main.as_str()))
        .collect();
    let other_of: BTreeMap<&str, &str> = pairs
        .iter()
        .map(|p| (p.main.as_str(), p.other.as_str()))
        .collect();

    // Whether an item changed since the base.
    let changed_since_base = |item: &Item, id: &str| {
        b_by.get(id)
            .is_some_and(|b| !different_keys(b, item, &[]).is_empty())
    };

    // The ID counters of main, raised to cover its items.
    let mut counters: BTreeMap<String, u64> = BTreeMap::new();
    if let Some(Value::Object(c)) = main.get("id_counters") {
        for (k, v) in c {
            if let Some(n) = crate::whole_number(v) {
                counters.insert(k.clone(), n);
            }
        }
    }
    for (id, _) in &m_items {
        if let Some((t, n)) = parse_item_id(id) {
            let c = counters.entry(t.prefix().to_string()).or_insert(0);
            *c = (*c).max(n);
        }
    }

    // Items of the other document that come into the result with their own
    // ID: with a base, an item that main removed and the other changed.
    let readded: BTreeSet<&str> = if three_way {
        o_items
            .iter()
            .filter(|(id, o)| !m_by.contains_key(id) && changed_since_base(o, id))
            .map(|(id, _)| *id)
            .collect()
    } else {
        BTreeSet::new()
    };

    // New IDs: first for the items that only the other document has, then
    // for the other item of each conflict. A ref never needs the second
    // kind: a ref to a paired item goes to the item of main.
    let mut new_ids: BTreeMap<String, String> = BTreeMap::new();
    let mut give_id = |id: &str, new_ids: &mut BTreeMap<String, String>| -> Result<(), Problem> {
        let o = o_by[id];
        let t: ItemType = str_of(o, "type").parse().map_err(|_| {
            Problem::new(
                "",
                format!(
                    "item {id} of the other document has no known type, so it cannot get an ID"
                ),
            )
        })?;
        let c = counters.entry(t.prefix().to_string()).or_insert(0);
        *c += 1;
        new_ids.insert(id.to_string(), format!("{}{c}", t.prefix()));
        Ok(())
    };
    for (id, _) in &o_items {
        let kept = main_of.contains_key(id)
            || readded.contains(id)
            || (three_way && b_by.contains_key(id));
        if !kept {
            give_id(id, &mut new_ids)?;
        }
    }

    // The ID in the result for a ref to an item of the other document.
    let result_id = |id: &str, new_ids: &BTreeMap<String, String>| -> Option<String> {
        if let Some(m) = main_of.get(id) {
            return Some(m.to_string());
        }
        if readded.contains(id) || (three_way && b_by.contains_key(id) && m_by.contains_key(id)) {
            return Some(id.to_string());
        }
        new_ids.get(id).cloned()
    };
    let other_ref = |id: &str, rel: &str| json!({"item": id, "artifact": other_artifact, "version": other_version, "rel": rel});
    // An item of the other document with its refs rewritten for the result.
    // A ref to an item that is not in the result points into the other
    // document.
    let rewrite = |item: &Item, new_ids: &BTreeMap<String, String>| -> Item {
        let mut item = item.clone();
        if let Some(Value::Array(refs)) = item.get_mut("refs") {
            for r in refs.iter_mut() {
                if !crate::diff::local_ref(r, &other_artifact) {
                    continue;
                }
                let Some(target) = r.get("item").and_then(Value::as_str).map(str::to_string) else {
                    continue;
                };
                let Some(obj) = r.as_object_mut() else {
                    continue;
                };
                match result_id(&target, new_ids) {
                    Some(new) => {
                        obj.insert("item".into(), Value::String(new));
                        obj.remove("artifact");
                        obj.remove("version");
                    }
                    None => {
                        obj.insert("artifact".into(), Value::String(other_artifact.clone()));
                        obj.insert("version".into(), Value::from(other_version));
                    }
                }
            }
        }
        item
    };

    // Each paired item: the merged item and the fields in conflict.
    let mut merged: BTreeMap<&str, (Item, Vec<String>)> = BTreeMap::new();
    for p in &pairs {
        let m = m_by[p.main.as_str()];
        let o = rewrite(o_by[p.other.as_str()], &new_ids);
        let mut item = m.clone();
        let fields = match three_way.then(|| b_by[p.main.as_str()]) {
            Some(b) => {
                let keys: BTreeSet<&String> = b.keys().chain(m.keys()).chain(o.keys()).collect();
                let mut fields = Vec::new();
                for k in keys.into_iter().filter(|k| *k != "id") {
                    let (bv, mv, ov) = (b.get(k), m.get(k), o.get(k));
                    if same_field(k, mv, ov) || same_field(k, bv, ov) {
                        continue;
                    }
                    if same_field(k, bv, mv) {
                        // Only the other document changed the field.
                        match ov {
                            Some(v) => item.insert(k.clone(), v.clone()),
                            None => item.remove(k),
                        };
                    } else {
                        fields.push(k.clone());
                    }
                }
                fields
            }
            None => different_keys(m, &o, &["id"]),
        };
        if !fields.is_empty() {
            give_id(&p.other, &mut new_ids)?;
        }
        merged.insert(p.main.as_str(), (item, fields));
    }
    let mut renamed = Vec::new();
    // The other items that supersede an item that main changed and the
    // other removed, as other ID -> main ID.
    let mut removed_in_other: Vec<(&str, Option<String>)> = Vec::new();
    let mut contradicts: BTreeMap<String, String> = BTreeMap::new();
    if three_way {
        for (id, m) in &m_items {
            if b_by.contains_key(id) && !o_by.contains_key(id) && changed_since_base(m, id) {
                let by = superseders(&o_items, id)
                    .first()
                    .and_then(|s| result_id(s, &new_ids));
                if let Some(by) = &by {
                    contradicts.insert(by.clone(), id.to_string());
                }
                removed_in_other.push((id, by));
            }
        }
    }

    // Build the items of the result.
    let mut out: Vec<Item> = Vec::new();
    let mut conflicts = Vec::new();
    let removed_main: BTreeSet<&str> = removed_in_other.iter().map(|(id, _)| *id).collect();
    for (id, m) in &m_items {
        if let Some((item, fields)) = merged.get(id) {
            let o_id = other_of[id];
            if fields.is_empty() {
                out.push(item.clone());
                continue;
            }
            let mut mine = item.clone();
            mine.insert("status".into(), "proposed".into());
            let mut theirs = rewrite(o_by[o_id], &new_ids);
            let new = new_ids[o_id].clone();
            theirs.insert("id".into(), Value::String(new.clone()));
            theirs.insert("status".into(), "proposed".into());
            push_ref(&mut theirs, other_ref(o_id, "derived_from"));
            push_ref(&mut theirs, json!({"item": id, "rel": "contradicts"}));
            out.push(mine);
            out.push(theirs);
            renamed.push(Renamed {
                from: o_id.to_string(),
                to: new.clone(),
            });
            conflicts.push(Conflict {
                kind: ConflictKind::Changed,
                main: Some(id.to_string()),
                other: Some(new),
                fields: fields.clone(),
                text_spans: None,
                base_text: None,
            });
            continue;
        }
        if three_way && b_by.contains_key(id) && !o_by.contains_key(id) {
            if !removed_main.contains(id) {
                // The other document removed it, and main did not change it.
                continue;
            }
            let mut mine = (*m).clone();
            mine.insert("status".into(), "proposed".into());
            out.push(mine);
            let by = removed_in_other
                .iter()
                .find(|(r, _)| r == id)
                .and_then(|(_, by)| by.clone());
            conflicts.push(Conflict {
                kind: ConflictKind::RemovedInOther,
                main: Some(id.to_string()),
                other: by,
                fields: Vec::new(),
                text_spans: None,
                base_text: None,
            });
            continue;
        }
        out.push((*m).clone());
    }

    // Items of the other document that are new in the result.
    for (id, o) in &o_items {
        if main_of.contains_key(id) {
            continue;
        }
        if readded.contains(id) {
            let mut theirs = rewrite(o, &new_ids);
            theirs.insert("status".into(), "proposed".into());
            push_ref(&mut theirs, other_ref(id, "derived_from"));
            let by = superseders(&m_items, id).first().map(|s| s.to_string());
            if let Some(by) = &by {
                push_ref(&mut theirs, json!({"item": by, "rel": "contradicts"}));
                if let Some(y) = out.iter_mut().find(|i| str_of(i, "id") == by) {
                    y.insert("status".into(), "proposed".into());
                }
            }
            out.push(theirs);
            conflicts.push(Conflict {
                kind: ConflictKind::RemovedInMain,
                main: by,
                other: Some(id.to_string()),
                fields: Vec::new(),
                text_spans: None,
                base_text: None,
            });
            continue;
        }
        let Some(new) = new_ids.get(*id) else {
            continue;
        };
        let mut theirs = rewrite(o, &new_ids);
        theirs.insert("id".into(), Value::String(new.clone()));
        push_ref(&mut theirs, other_ref(id, "derived_from"));
        if let Some(main_id) = contradicts.get(new) {
            theirs.insert("status".into(), "proposed".into());
            push_ref(&mut theirs, json!({"item": main_id, "rel": "contradicts"}));
        }
        out.push(theirs);
        renamed.push(Renamed {
            from: id.to_string(),
            to: new.clone(),
        });
    }
    renamed.sort_by_key(|r| o_items.iter().position(|(id, _)| *id == r.from));
    let text_in_out = |id: &Option<String>| {
        let id = id.as_deref()?;
        out.iter()
            .find(|i| str_of(i, "id") == id)
            .map(|i| str_of(i, "text").to_string())
    };
    for c in &mut conflicts {
        if let (Some(m), Some(o)) = (text_in_out(&c.main), text_in_out(&c.other)) {
            c.text_spans = Some(word_diff(&m, &o));
        }
        if three_way {
            let base_id = match c.kind {
                ConflictKind::RemovedInMain => &c.other,
                _ => &c.main,
            };
            c.base_text = base_id
                .as_deref()
                .and_then(|id| b_by.get(id))
                .map(|b| str_of(b, "text").to_string());
        }
    }
    conflicts.sort_by_key(|c| {
        let key = c
            .main
            .as_ref()
            .or(c.other.as_ref())
            .cloned()
            .unwrap_or_default();
        out.iter().position(|i| str_of(i, "id") == key)
    });

    // The envelope.
    let mut doc = main.clone();
    let obj = doc
        .as_object_mut()
        .expect("main has an ID, so it is an object");
    let envelope = match other.as_object() {
        Some(o) => {
            let mut skip: Vec<&str> = NOT_CONTENT.to_vec();
            skip.push("id");
            different_keys(main.as_object().expect("an object"), o, &skip)
        }
        None => Vec::new(),
    };
    obj.insert("version".into(), Value::from(main_version + 1));
    obj.insert("status".into(), "draft".into());
    obj.remove("finalized_at");
    if let Some(now) = &options.now {
        obj.insert("created_at".into(), Value::String(now.clone()));
    }
    let mut parents = vec![json!({
        "rel": "previous_version", "artifact": main_artifact, "version": main_version
    })];
    let other_parent = json!({
        "rel": "input", "artifact": other_artifact, "version": other_version
    });
    for p in obj
        .get("parents")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let same_input = p.get("rel").and_then(Value::as_str) == Some("input")
            && p.get("artifact") == other_parent.get("artifact")
            && p.get("version").and_then(crate::whole_number) == Some(other_version);
        if p.get("rel").and_then(Value::as_str) != Some("previous_version") && !same_input {
            parents.push(p.clone());
        }
    }
    let mut input = other_parent;
    for key in ["kind", "title"] {
        if let Some(v) = other.get(key).and_then(Value::as_str) {
            input[key] = Value::String(v.to_string());
        }
    }
    parents.push(input);
    obj.insert("parents".into(), Value::Array(parents));
    obj.insert(
        "items".into(),
        Value::Array(out.into_iter().map(Value::Object).collect()),
    );
    let mut new_counters: Map<String, Value> = match obj.get("id_counters") {
        Some(Value::Object(c)) => c.clone(),
        _ => Map::new(),
    };
    for t in ItemType::ALL {
        if let Some(&c) = counters.get(t.prefix()) {
            let old = new_counters.get(t.prefix()).and_then(crate::whole_number);
            if c > 0 && old != Some(c) {
                new_counters.insert(t.prefix().to_string(), Value::from(c));
            }
        }
    }
    if !new_counters.is_empty() {
        obj.insert("id_counters".into(), Value::Object(new_counters));
    }
    // Raise each counter that a re-added item needs.
    assign_ids(&mut doc).map_err(|mut p| p.remove(0))?;

    Ok(Merge {
        doc,
        pairs,
        conflicts,
        renamed,
        envelope,
    })
}

impl Merge {
    /// The merge as text for people.
    ///
    /// ```text
    /// pa_7Hk2mQ9xR4tV8wZ1@2 + pa_3Fq8Lp2Wd6Ks9Nb4@1 -> pa_7Hk2mQ9xR4tV8wZ1@3 (draft)
    /// paired     FR1 = FR2 (text, 80%)
    /// renamed    FR1 -> FR4
    /// envelope   summary, title (main kept)
    /// 1 conflict:
    ///   FR1 or FR4: text
    /// ```
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        self.write_text(&mut out)
            .expect("writing to a String cannot fail");
        out
    }

    fn write_text(&self, out: &mut String) -> fmt::Result {
        let label = |doc: &Value| {
            crate::version_label(
                doc.get("id").and_then(Value::as_str).unwrap_or(""),
                doc.get("version").and_then(crate::whole_number),
            )
        };
        let parents = self.doc["parents"].as_array().cloned().unwrap_or_default();
        let parent = |rel: &str| {
            parents
                .iter()
                .rev()
                .find(|p| p["rel"] == rel)
                .map(|p| {
                    crate::version_label(
                        p["artifact"].as_str().unwrap_or(""),
                        crate::whole_number(&p["version"]),
                    )
                })
                .unwrap_or_default()
        };
        writeln!(
            out,
            "{} + {} -> {} (draft)",
            parent("previous_version"),
            parent("input"),
            label(&self.doc)
        )?;
        for p in &self.pairs {
            let how = match (p.by, p.similarity) {
                (PairedBy::Text, Some(s)) => format!("text, {s}%"),
                (PairedBy::Id, _) => "id".to_string(),
                (PairedBy::Given, _) => "given".to_string(),
                (PairedBy::Text, None) => "text".to_string(),
            };
            if p.main != p.other || p.by != PairedBy::Id {
                writeln!(out, "paired     {} = {} ({how})", p.main, p.other)?;
            }
        }
        for r in &self.renamed {
            writeln!(
                out,
                "new        {}  from {} of the other document",
                r.to, r.from
            )?;
        }
        if !self.envelope.is_empty() {
            writeln!(out, "envelope   {} (main kept)", self.envelope.join(", "))?;
        }
        if self.conflicts.is_empty() {
            return writeln!(out, "no conflicts");
        }
        writeln!(out, "{}:", plural(self.conflicts.len(), "conflict"))?;
        for c in &self.conflicts {
            writeln!(out, "  {}", conflict_line(c))?;
        }
        Ok(())
    }
}

fn conflict_line(c: &Conflict) -> String {
    let main = c.main.as_deref();
    let other = c.other.as_deref();
    let ids = match (main, other) {
        (Some(m), Some(o)) => format!("{m} or {o}"),
        (Some(id), None) | (None, Some(id)) => id.to_string(),
        (None, None) => String::new(),
    };
    match c.kind {
        ConflictKind::Changed => format!("{ids}: {}", c.fields.join(", ")),
        ConflictKind::RemovedInMain => {
            format!("{ids}: removed in main, changed in the other document")
        }
        ConflictKind::RemovedInOther => {
            format!("{ids}: changed in main, removed in the other document")
        }
    }
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}

/// A conflict that is still open in a merged document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenConflict {
    /// The item that the other item contradicts.
    pub main: String,
    /// The item that came from the other document, with the `contradicts`
    /// ref.
    pub other: String,
    /// The fields that are different, apart from `id`, `status` and `refs`.
    pub fields: Vec<String>,
}

impl fmt::Display for OpenConflict {
    /// `FR5 or FR9: text`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} or {}", self.main, self.other)?;
        if !self.fields.is_empty() {
            write!(f, ": {}", self.fields.join(", "))?;
        }
        Ok(())
    }
}

/// The conflicts of a merged document that are still open: two `proposed`
/// items, one with a `contradicts` ref to the other and a `derived_from` ref
/// into another artifact. This is how [`merge`] writes a conflict. In the
/// order of the items.
pub fn conflicts(doc: &Value) -> Vec<OpenConflict> {
    let items = items_by_id(doc);
    let by: BTreeMap<&str, &Item> = items.iter().copied().collect();
    let mut out = Vec::new();
    for (id, item) in &items {
        if str_of(item, "status") != "proposed" {
            continue;
        }
        let refs: Vec<&Value> = item
            .get("refs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .collect();
        // A derived_from ref with an artifact points into another version.
        let from_other = refs.iter().any(|r| {
            r.get("rel").and_then(Value::as_str) == Some("derived_from")
                && r.get("artifact").is_some()
        });
        if !from_other {
            continue;
        }
        for r in &refs {
            if r.get("rel").and_then(Value::as_str) != Some("contradicts")
                || r.get("artifact").is_some()
            {
                continue;
            }
            let Some(target) = r.get("item").and_then(Value::as_str) else {
                continue;
            };
            let Some(main) = by.get(target) else { continue };
            if str_of(main, "status") != "proposed" {
                continue;
            }
            out.push(OpenConflict {
                main: target.to_string(),
                other: id.to_string(),
                fields: different_keys(main, item, &["id", "status", "refs"]),
            });
        }
    }
    out
}

/// What [`resolve`] changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolved {
    pub accepted: String,
    pub rejected: String,
}

/// Resolves an open conflict: accepts the item with this ID and rejects the
/// other item of its pair. Returns an error if the item is not in an open
/// conflict ([`conflicts`]).
pub fn resolve(doc: &mut Value, id: &str) -> Result<Resolved, Problem> {
    let open = conflicts(doc);
    let Some(c) = open.iter().find(|c| c.main == id || c.other == id) else {
        return Err(Problem::new("", format!("{id} is not in an open conflict")));
    };
    let rejected = if c.main == id { &c.other } else { &c.main }.clone();
    if let Some(items) = doc.get_mut("items").and_then(Value::as_array_mut) {
        for item in items {
            let status = match item.get("id").and_then(Value::as_str) {
                Some(i) if i == id => "accepted",
                Some(i) if i == rejected => "rejected",
                _ => continue,
            };
            item["status"] = Value::String(status.to_string());
        }
    }
    Ok(Resolved {
        accepted: id.to_string(),
        rejected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::merge;

    fn doc(id: &str, version: u64, items: Value) -> Value {
        json!({"id": id, "version": version, "status": "draft", "items": items})
    }

    fn req(id: &str, text: &str) -> Value {
        json!({"id": id, "type": "requirement", "text": text, "status": "accepted"})
    }

    #[test]
    fn given_pairs_replace_pairs_by_text() {
        let main = doc(
            "pa_1",
            1,
            json!([req("FR1", "Block it."), req("FR2", "Warn the admin.")]),
        );
        let other = doc(
            "pa_2",
            1,
            json!([req("FR1", "Warn the admin."), req("FR2", "Stop it.")]),
        );
        let options = Options {
            pairs: Some(vec![("FR1".into(), "FR2".into())]),
            ..Default::default()
        };
        let m = merge(&main, &other, &options).unwrap();
        assert_eq!(m.pairs.len(), 1);
        assert_eq!(m.pairs[0].by, PairedBy::Given);
        // FR1 = FR2 conflicts on text. The other FR1 is not paired, so it is new.
        assert_eq!(m.conflicts[0].main.as_deref(), Some("FR1"));
        let ids: Vec<&str> = m.doc["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, ["FR1", "FR4", "FR2", "FR3"]);
        assert_eq!(m.doc["id_counters"]["FR"], 4);

        let bad = Options {
            pairs: Some(vec![("FR1".into(), "FR9".into())]),
            ..Default::default()
        };
        assert!(merge(&main, &other, &bad)
            .unwrap_err()
            .message
            .contains("FR9"));
        let twice = Options {
            pairs: Some(vec![
                ("FR1".into(), "FR1".into()),
                ("FR1".into(), "FR2".into()),
            ]),
            ..Default::default()
        };
        assert!(merge(&main, &other, &twice).is_err());
    }

    #[test]
    fn an_item_that_main_removed_and_the_other_changed_comes_back() {
        let base = doc("pa_1", 1, json!([req("FR1", "Old.")]));
        let main = doc(
            "pa_1",
            2,
            json!([{"id": "FR2", "type": "requirement", "text": "New.", "status": "accepted",
                    "refs": [{"item": "FR1", "rel": "supersedes"}]}]),
        );
        let other = doc("pa_2", 1, json!([req("FR1", "Old, but changed.")]));
        let options = Options {
            base: Some(&base),
            ..Default::default()
        };
        let m = merge(&main, &other, &options).unwrap();
        assert_eq!(m.conflicts[0].kind, ConflictKind::RemovedInMain);
        assert_eq!(m.conflicts[0].main.as_deref(), Some("FR2"));
        assert_eq!(m.conflicts[0].other.as_deref(), Some("FR1"));
        let items = m.doc["items"].as_array().unwrap();
        assert_eq!(items[0]["status"], "proposed");
        assert_eq!(items[1]["id"], "FR1");
        assert_eq!(merge::conflicts(&m.doc)[0].main, "FR2");
        // Main removed an item that the other did not change: it stays removed.
        let other = doc("pa_2", 1, json!([req("FR1", "Old.")]));
        let m = merge(&main, &other, &options).unwrap();
        assert!(m.conflicts.is_empty());
        assert_eq!(m.doc["items"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn a_ref_to_an_item_left_out_points_into_the_other_document() {
        let base = doc("pa_1", 1, json!([req("FR1", "Old.")]));
        let main = doc("pa_1", 2, json!([]));
        let other = doc(
            "pa_2",
            3,
            json!([req("FR1", "Old."), {"id": "S1", "type": "success_signal", "text": "x",
                   "status": "accepted", "refs": [{"item": "FR1", "rel": "verifies"}]}]),
        );
        let options = Options {
            base: Some(&base),
            ..Default::default()
        };
        let m = merge(&main, &other, &options).unwrap();
        let s1 = &m.doc["items"][0];
        assert_eq!(
            s1["refs"][0],
            json!({"item": "FR1", "artifact": "pa_2", "version": 3, "rel": "verifies"})
        );
    }

    #[test]
    fn resolve_accepts_one_and_rejects_the_other() {
        let main = doc("pa_1", 1, json!([req("FR1", "Block the plan change.")]));
        let other = doc("pa_2", 1, json!([req("FR1", "Block the plan change now.")]));
        let mut d = merge(&main, &other, &Options::default()).unwrap().doc;
        assert_eq!(merge::conflicts(&d).len(), 1);
        assert!(resolve(&mut d, "FR7").is_err());
        let r = resolve(&mut d, "FR1").unwrap();
        assert_eq!(r.rejected, "FR2");
        assert!(merge::conflicts(&d).is_empty());
        assert_eq!(d["items"][1]["status"], "rejected");
    }

    #[test]
    fn a_conflict_has_its_words_and_the_base_text() {
        let base = doc("pa_1", 1, json!([req("FR1", "Warn the admin.")]));
        let main = doc("pa_1", 2, json!([req("FR1", "Block the change.")]));
        let other = doc("pa_2", 1, json!([req("FR1", "Warn the admin by email.")]));
        let options = Options {
            base: Some(&base),
            ..Default::default()
        };
        let c = &merge(&main, &other, &options).unwrap().conflicts[0];
        assert_eq!(c.base_text.as_deref(), Some("Warn the admin."));
        let spans: Vec<(crate::words::Op, &str)> = c
            .text_spans
            .as_ref()
            .unwrap()
            .iter()
            .map(|s| (s.op, s.text.as_str()))
            .collect();
        use crate::words::Op::*;
        assert_eq!(
            spans,
            [
                (Del, "Block"),
                (Ins, "Warn"),
                (Same, "the"),
                (Del, "change."),
                (Ins, "admin by email.")
            ]
        );
        // Without a base there is no base text.
        let c = &merge(&main, &other, &Options::default()).unwrap();
        assert!(c.conflicts.iter().all(|c| c.base_text.is_none()));
    }

    #[test]
    fn similarity_counts_shared_words() {
        assert_eq!(similarity("Block the change.", "block the CHANGE"), 100);
        assert_eq!(similarity("a b", "c d"), 0);
        assert_eq!(similarity("a b c d", "a b x y"), 50);
    }
}
