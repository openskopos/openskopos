//! Compact a set of documents into their current state.
//!
//! [`compact`] takes the documents of one area: every version that the team
//! has of each artifact. It keeps the items that are current and writes one
//! draft per kind of artifact:
//!
//! - An item is current when its state ([`crate::history`]) is in force or
//!   proposed. Rejected, superseded and dropped items are left out, and so
//!   are answered questions, critiques with a resolution and refuted
//!   assumptions.
//! - When a kind has one artifact, the draft is its next version. Items keep
//!   their IDs, so reference strings keep working.
//! - When a kind has several artifacts, the draft is a new artifact. Its
//!   items get new IDs and a `derived_from` ref to the item they come from,
//!   and each source artifact is an `input` parent.
//! - Terms of every artifact go to the glossary draft. Two terms with the
//!   same name and different definitions both stay, as `proposed`, with a
//!   `contradicts` ref, like a conflict of [`crate::merge`].
//! - Refs between current items are rewritten to the new IDs. A
//!   `supersedes` ref keeps pointing to the old item. Other refs to items
//!   that are left out are removed.
//!
//! A kind whose artifact loses no items and gets none stays unchanged and
//! has no draft. The function is pure: the caller gives the time and the
//! IDs of new artifacts. It does not move files; that is the job of a tool.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::history::{history, ItemState, Warning};
use crate::Problem;

/// Options for [`compact`].
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// The time for `created_at` of each draft, for example
    /// `2026-11-27T12:00:00Z`. Without it, a next version keeps the time of
    /// the version before it, and a new artifact uses the newest time of its
    /// sources.
    pub now: Option<String>,
}

/// Why an item is not in the drafts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    Rejected,
    Superseded,
    /// An answered open question.
    Answered,
    /// A critique with a resolution.
    Resolved,
    /// An assumption that the team refuted.
    Refuted,
    /// A term with the same name and definition as a term that is kept.
    Duplicate,
}

/// An item that is not in the drafts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeftOut {
    /// The reference string in the latest version, such as `pa_1@3#FR2`.
    pub reference: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub reason: Reason,
    /// For a duplicate term: the reference string of the term that is kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kept: Option<String>,
}

/// One draft that the compaction writes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Draft {
    pub kind: String,
    /// True if the draft is a new artifact, false if it is the next version
    /// of its only source.
    pub new_artifact: bool,
    /// The latest version of each source, as `<artifact>@<version>`.
    pub sources: Vec<String>,
    /// The number of items in the draft.
    pub items: usize,
    pub doc: Value,
}

/// Two terms with the same name and different definitions. Both stay as
/// `proposed` until the team rejects one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermConflict {
    pub term: String,
    /// IDs in the glossary draft.
    pub items: Vec<String>,
}

/// The result of [`compact`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Compaction {
    /// One draft per kind that changes, in the order of the kinds' names.
    pub drafts: Vec<Draft>,
    /// Kinds that need no draft, as `<artifact>@<version>`.
    pub unchanged: Vec<String>,
    pub left_out: Vec<LeftOut>,
    pub conflicts: Vec<TermConflict>,
    /// The warnings of the history, such as a missing version.
    pub warnings: Vec<Warning>,
}

fn s<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

fn version_of(doc: &Value) -> u64 {
    doc.get("version")
        .and_then(crate::whole_number)
        .unwrap_or(0)
}

fn items(doc: &Value) -> &[Value] {
    doc.get("items")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn prefix(id: &str) -> &str {
    id.trim_end_matches(|c: char| c.is_ascii_digit())
}

fn reason(item: &Value, state: ItemState) -> Option<Reason> {
    match state {
        ItemState::Rejected => return Some(Reason::Rejected),
        ItemState::Superseded => return Some(Reason::Superseded),
        // A dropped item is in no latest version, so it never gets here.
        ItemState::Dropped => return Some(Reason::Superseded),
        ItemState::InForce | ItemState::Proposed => {}
    }
    match s(item, "type") {
        "open_question" if item.get("answer").is_some() && s(item, "status") == "accepted" => {
            Some(Reason::Answered)
        }
        "critique" if item.get("resolution").is_some() => Some(Reason::Resolved),
        "assumption" if s(item, "confirmation") == "refuted" => Some(Reason::Refuted),
        _ => None,
    }
}

/// Where a carried item goes: the index of its draft and its ID there.
type Dest = (usize, String);

/// Compacts documents. See the [module](self) documentation.
///
/// `new_id` gives the ID of each new artifact; the CLI and the WebAssembly
/// build give random IDs. Returns a problem if the latest version of an
/// artifact is a draft: finish or discard it first.
///
/// ```
/// use serde_json::json;
/// let v1 = json!({"id": "pa_1", "version": 1, "kind": "spec", "status": "final", "items": [
///     {"id": "D1", "type": "decision", "text": "Count nightly.", "status": "rejected"},
///     {"id": "D2", "type": "decision", "text": "Count at the change.", "status": "accepted"}
/// ]});
/// let c = skopos_core::compact::compact(&[v1], &Default::default(), &mut || unreachable!()).unwrap();
/// assert_eq!(c.drafts[0].doc["version"], 2);
/// assert_eq!(c.drafts[0].doc["items"][0]["id"], "D2");
/// assert_eq!(c.left_out[0].reference, "pa_1@1#D1");
/// ```
pub fn compact(
    docs: &[Value],
    options: &Options,
    new_id: &mut dyn FnMut() -> String,
) -> Result<Compaction, Problem> {
    let h = history(docs);
    let state: BTreeMap<&str, ItemState> =
        h.items.iter().map(|i| (i.key.as_str(), i.state)).collect();

    // The latest version of each artifact, by kind.
    let mut latest: BTreeMap<&str, &Value> = BTreeMap::new();
    for doc in docs {
        let id = s(doc, "id");
        if id.is_empty() {
            continue;
        }
        if latest
            .get(id)
            .is_none_or(|d| version_of(d) < version_of(doc))
        {
            latest.insert(id, doc);
        }
    }
    for (id, doc) in &latest {
        if s(doc, "status") != "final" {
            return Err(Problem::new(
                "",
                format!(
                    "version {} of {id} is a draft. Finalize or discard it before compacting",
                    version_of(doc)
                ),
            ));
        }
    }
    let mut kinds: BTreeMap<&str, Vec<&Value>> = BTreeMap::new();
    for doc in latest.values() {
        kinds.entry(s(doc, "kind")).or_default().push(doc);
    }

    // Phase 1: choose what each draft holds and give each carried item its
    // ID in the draft.
    let mut left_out = Vec::new();
    let mut members: BTreeMap<&str, Vec<(&Value, &Value)>> = BTreeMap::new();
    let mut changed: BTreeSet<&str> = BTreeSet::new();
    for (kind, sources) in &kinds {
        for doc in sources {
            for item in items(doc) {
                let key = format!("{}#{}", s(doc, "id"), s(item, "id"));
                let st = state
                    .get(key.as_str())
                    .copied()
                    .unwrap_or(ItemState::InForce);
                if let Some(r) = reason(item, st) {
                    left_out.push(LeftOut {
                        reference: format!(
                            "{}@{}#{}",
                            s(doc, "id"),
                            version_of(doc),
                            s(item, "id")
                        ),
                        item_type: s(item, "type").to_string(),
                        reason: r,
                        kept: None,
                    });
                    changed.insert(kind);
                    continue;
                }
                let target = if s(item, "type") == "term" {
                    "glossary"
                } else {
                    kind
                };
                if target != *kind {
                    changed.insert(kind);
                    changed.insert(target);
                }
                members.entry(target).or_default().push((doc, item));
            }
        }
    }

    // Same term, same definition: keep the first. Different definitions: a
    // conflict, settled later in the draft.
    let mut conflict_names: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut duplicate_of: BTreeMap<(String, String), (String, String)> = BTreeMap::new();
    if let Some(terms) = members.get_mut("glossary") {
        let mut first: BTreeMap<(String, String), (String, String)> = BTreeMap::new();
        let mut kept = Vec::new();
        for (doc, item) in terms.drain(..) {
            let name = s(item, "text").trim().to_lowercase();
            let def = s(item, "definition").trim().to_string();
            let me = (s(doc, "id").to_string(), s(item, "id").to_string());
            match first.get(&(name.clone(), def.clone())) {
                Some(k) => {
                    left_out.push(LeftOut {
                        reference: format!("{}@{}#{}", me.0, version_of(doc), me.1),
                        item_type: "term".into(),
                        reason: Reason::Duplicate,
                        kept: Some(format!("{}#{}", k.0, k.1)),
                    });
                    duplicate_of.insert(me, k.clone());
                    changed.insert("glossary");
                }
                None => {
                    first.insert((name.clone(), def), me);
                    conflict_names.entry(name).or_default().push(kept.len());
                    kept.push((doc, item));
                }
            }
        }
        *terms = kept;
    }

    let mut plans: Vec<Plan> = Vec::new();
    let mut unchanged = Vec::new();
    let mut dest: BTreeMap<(String, String), Dest> = BTreeMap::new();
    let all_kinds: BTreeSet<&str> = kinds
        .keys()
        .copied()
        .chain(members.keys().copied())
        .collect();
    for kind in all_kinds {
        let sources: Vec<&Value> = kinds.get(kind).cloned().unwrap_or_default();
        let carried = members.remove(kind).unwrap_or_default();
        if sources.len() == 1 && !changed.contains(kind) {
            unchanged.push(format!(
                "{}@{}",
                s(sources[0], "id"),
                version_of(sources[0])
            ));
            continue;
        }
        if carried.is_empty() && sources.is_empty() {
            continue;
        }
        let continues = (sources.len() == 1).then(|| sources[0]);
        let artifact = match continues {
            Some(doc) => s(doc, "id").to_string(),
            None => new_id(),
        };
        // Counters of a continued artifact: its id_counters, raised to the
        // highest ID in any version given, so no ID is used again.
        let mut counters: BTreeMap<String, u64> = BTreeMap::new();
        if let Some(c) = continues {
            if let Some(Value::Object(m)) = c.get("id_counters") {
                counters.extend(
                    m.iter()
                        .filter_map(|(k, v)| Some((k.clone(), crate::whole_number(v)?))),
                );
            }
            for d in docs.iter().filter(|d| s(d, "id") == artifact) {
                for item in items(d) {
                    if let Some((t, n)) = crate::parse_item_id(s(item, "id")) {
                        let e = counters.entry(t.prefix().to_string()).or_insert(0);
                        *e = (*e).max(n);
                    }
                }
            }
        }
        let index = plans.len();
        let mut ids = Vec::new();
        for (doc, item) in &carried {
            let old = s(item, "id");
            let keep = continues.is_some_and(|c| s(c, "id") == s(doc, "id"));
            let id = if keep {
                old.to_string()
            } else {
                let c = counters.entry(prefix(old).to_string()).or_insert(0);
                *c += 1;
                format!("{}{c}", prefix(old))
            };
            dest.insert(
                (s(doc, "id").to_string(), old.to_string()),
                (index, id.clone()),
            );
            ids.push(id);
        }
        plans.push(Plan {
            kind: kind.to_string(),
            artifact,
            continues,
            sources,
            carried,
            ids,
            counters,
        });
    }
    for (dup, kept) in duplicate_of {
        if let Some(d) = dest.get(&kept).cloned() {
            dest.insert(dup, d);
        }
    }
    let in_set: BTreeSet<&str> = latest.keys().copied().collect();

    // Phase 2: write the drafts.
    let mut drafts = Vec::new();
    let mut conflicts = Vec::new();
    for (index, plan) in plans.iter().enumerate() {
        let mut out: Vec<Value> = Vec::new();
        for ((doc, item), id) in plan.carried.iter().zip(&plan.ids) {
            let src = s(doc, "id");
            let mut obj: Map<String, Value> = item.as_object().cloned().unwrap_or_default();
            obj.insert("id".into(), Value::String(id.clone()));
            let mut refs: Vec<Value> = Vec::new();
            let moved = src != plan.artifact;
            if moved {
                refs.push(json!({"item": s(item, "id"), "artifact": src, "version": version_of(doc), "rel": "derived_from"}));
            }
            for r in item
                .get("refs")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let target_artifact = r.get("artifact").and_then(Value::as_str).unwrap_or(src);
                let target = (target_artifact.to_string(), s(r, "item").to_string());
                let mut nr = r.as_object().cloned().unwrap_or_default();
                if let Some((di, did)) = dest.get(&target) {
                    nr.insert("item".into(), Value::String(did.clone()));
                    nr.remove("version");
                    if *di == index {
                        nr.remove("artifact");
                    } else {
                        nr.insert(
                            "artifact".into(),
                            Value::String(plans[*di].artifact.clone()),
                        );
                    }
                } else if s(r, "rel") == "supersedes" || !in_set.contains(target_artifact) {
                    // The old item stays in the record; point to it where it is.
                    if target_artifact != plan.artifact {
                        nr.insert(
                            "artifact".into(),
                            Value::String(target_artifact.to_string()),
                        );
                    }
                } else {
                    continue;
                }
                let nr = Value::Object(nr);
                if !refs.contains(&nr) {
                    refs.push(nr);
                }
            }
            if refs.is_empty() {
                obj.remove("refs");
            } else {
                obj.insert("refs".into(), Value::Array(refs));
            }
            out.push(Value::Object(obj));
        }
        if plan.kind == "glossary" {
            for (name, positions) in &conflict_names {
                if positions.len() < 2 {
                    continue;
                }
                let first = plan.ids[positions[0]].clone();
                for &p in positions {
                    out[p]["status"] = "proposed".into();
                    if p != positions[0] {
                        let refs = out[p]
                            .as_object_mut()
                            .expect("an item")
                            .entry("refs")
                            .or_insert(json!([]));
                        refs.as_array_mut()
                            .expect("refs")
                            .push(json!({"item": first, "rel": "contradicts"}));
                    }
                }
                conflicts.push(TermConflict {
                    term: name.clone(),
                    items: positions.iter().map(|&p| plan.ids[p].clone()).collect(),
                });
            }
        }
        drafts.push(write_draft(plan, out, options));
    }

    left_out.sort_by(|a, b| a.reference.cmp(&b.reference));
    Ok(Compaction {
        drafts,
        unchanged,
        left_out,
        conflicts,
        warnings: h.warnings,
    })
}

/// One draft before it is written.
struct Plan<'a> {
    kind: String,
    artifact: String,
    /// The only source, when the draft is its next version.
    continues: Option<&'a Value>,
    sources: Vec<&'a Value>,
    carried: Vec<(&'a Value, &'a Value)>,
    ids: Vec<String>,
    counters: BTreeMap<String, u64>,
}

fn write_draft(plan: &Plan, items: Vec<Value>, options: &Options) -> Draft {
    let mut doc = match plan.continues {
        Some(prev) => {
            let mut d = prev.clone();
            let o = d.as_object_mut().expect("a document is an object");
            o.insert("version".into(), Value::from(version_of(prev) + 1));
            o.remove("finalized_at");
            let mut parents = vec![
                json!({"rel": "previous_version", "artifact": plan.artifact, "version": version_of(prev)}),
            ];
            for p in prev
                .get("parents")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if s(p, "rel") != "previous_version" {
                    parents.push(p.clone());
                }
            }
            o.insert("parents".into(), Value::Array(parents));
            d
        }
        None => {
            // The title of the source with the most carried items.
            let mut count: BTreeMap<&str, usize> = BTreeMap::new();
            for (d, _) in &plan.carried {
                *count.entry(s(d, "id")).or_default() += 1;
            }
            let main = plan
                .sources
                .iter()
                .max_by_key(|d| {
                    (
                        count.get(s(d, "id")).copied().unwrap_or(0),
                        std::cmp::Reverse(s(d, "id")),
                    )
                })
                .copied();
            let title = main
                .map(|d| s(d, "title"))
                .filter(|t| !t.is_empty())
                .unwrap_or(&plan.kind);
            let created = plan
                .sources
                .iter()
                .map(|d| s(d, "created_at"))
                .max()
                .unwrap_or("");
            json!({
                "$schema": main.map(|d| s(d, "$schema")).filter(|x| !x.is_empty()).unwrap_or(crate::SCHEMA_URL),
                "id": plan.artifact,
                "version": 1,
                "kind": plan.kind,
                "title": title,
                "created_at": created,
            })
        }
    };
    let o = doc.as_object_mut().expect("a document is an object");
    o.insert("status".into(), "draft".into());
    if let Some(now) = &options.now {
        o.insert("created_at".into(), Value::String(now.clone()));
    }
    // An input parent for each source that is not the version before.
    let mut inputs: Vec<(&str, u64, &str, &str)> = Vec::new();
    for d in &plan.sources {
        if Some(s(d, "id")) != plan.continues.map(|c| s(c, "id")) {
            inputs.push((s(d, "id"), version_of(d), s(d, "kind"), s(d, "title")));
        }
    }
    for (d, _) in &plan.carried {
        let i = (s(d, "id"), version_of(d), s(d, "kind"), s(d, "title"));
        if i.0 != plan.artifact && !inputs.contains(&i) {
            inputs.push(i);
        }
    }
    let parents = o
        .entry("parents")
        .or_insert(json!([]))
        .as_array_mut()
        .expect("parents is a list");
    for (artifact, version, kind, title) in inputs {
        let mut p = json!({"rel": "input", "artifact": artifact, "version": version, "kind": kind});
        if !title.is_empty() {
            p["title"] = title.into();
        }
        let exists = parents.iter().any(|q| {
            s(q, "rel") == "input"
                && s(q, "artifact") == artifact
                && q.get("version") == p.get("version")
        });
        if !exists {
            parents.push(p);
        }
    }
    if parents.is_empty() {
        o.remove("parents");
    }
    if !plan.counters.is_empty() {
        o.insert(
            "id_counters".into(),
            Value::Object(
                plan.counters
                    .iter()
                    .map(|(k, v)| (k.clone(), Value::from(*v)))
                    .collect(),
            ),
        );
    }
    let n = items.len();
    o.insert("items".into(), Value::Array(items));
    Draft {
        kind: plan.kind.clone(),
        new_artifact: plan.continues.is_none(),
        sources: plan
            .sources
            .iter()
            .map(|d| format!("{}@{}", s(d, "id"), version_of(d)))
            .collect(),
        items: n,
        doc,
    }
}

impl Compaction {
    /// The result for people, as `skopos compact` writes it.
    pub fn to_text(&self) -> String {
        use std::fmt::Write as _;
        let mut t = String::new();
        for d in &self.drafts {
            let what = if d.new_artifact {
                "new artifact"
            } else {
                "next version"
            };
            let from = match d.sources.len() {
                0 => "the terms of the other kinds".to_string(),
                1..=4 => d.sources.join(", "),
                n => format!("{n} artifacts"),
            };
            let _ = writeln!(
                t,
                "{}: {what} {}@{} (draft), {} items, from {from}",
                d.kind,
                s(&d.doc, "id"),
                version_of(&d.doc),
                d.items,
            );
        }
        for u in &self.unchanged {
            let _ = writeln!(t, "unchanged: {u}");
        }
        let mut by: BTreeMap<Reason, usize> = BTreeMap::new();
        for l in &self.left_out {
            *by.entry(l.reason).or_default() += 1;
        }
        if !by.is_empty() {
            let parts: Vec<String> = by
                .iter()
                .map(|(r, n)| {
                    format!(
                        "{} {n}",
                        serde_json::to_value(r)
                            .ok()
                            .and_then(|v| v.as_str().map(String::from))
                            .unwrap_or_default()
                    )
                })
                .collect();
            let _ = writeln!(
                t,
                "left out: {} ({})",
                self.left_out.len(),
                parts.join(", ")
            );
        }
        for c in &self.conflicts {
            let _ = writeln!(
                t,
                "term conflict: \"{}\" {}: reject one before you finalize",
                c.term,
                c.items.join(", ")
            );
        }
        for w in &self.warnings {
            let _ = writeln!(t, "warning: {w}");
        }
        t
    }
}
