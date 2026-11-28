//! The history of a set of documents: what each version decided, and the life
//! of each item across versions and artifacts.
//!
//! [`history`] takes any number of documents, in any order, from one or more
//! artifacts. It does these steps:
//!
//! 1. It puts the documents of each artifact in a chain, in version order.
//!    The versions of one artifact form a straight line (specification 8.1).
//!    A missing version gives a warning, not an error: the versions on each
//!    side of the gap are compared directly.
//! 2. It orders the chains so that an artifact comes before the artifacts
//!    that have one of its versions as an `input` parent, for example when a
//!    second team takes over (specification 8.1.6).
//! 3. It compares each pair of consecutive versions with [`crate::diff::diff`]
//!    and folds the results into one timeline for each item. An item is the
//!    same item in every version of its artifact (specification section 6).
//! 4. It follows `supersedes` and `derived_from` refs, also into other
//!    artifacts, and gives the state of each item now.
//!
//! The result is plain data and is the same for the same documents in any
//! order.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as _};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::diff::{diff, items_by_id, Diff};

/// The history of a set of documents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct History {
    /// One chain for each artifact. An artifact comes before the artifacts
    /// that use one of its versions as an input. Otherwise the order is by
    /// artifact ID.
    pub chains: Vec<Chain>,
    /// The `input` parents of the documents that are versions of an
    /// artifact, in the order of the chains and versions.
    pub links: Vec<Link>,
    /// Every item seen in any version: the items of the first chain in the
    /// order in which they first appear, then those of the next chain.
    pub items: Vec<ItemHistory>,
    /// Missing versions and documents that were left out.
    pub warnings: Vec<Warning>,
}

/// The versions of one artifact that were given, in version order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chain {
    pub artifact: String,
    /// The `kind` of the latest version.
    pub kind: String,
    /// The `title` of the latest version.
    pub title: String,
    pub versions: Vec<VersionEntry>,
}

/// One version in a chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionEntry {
    pub version: u64,
    /// `draft` or `final`.
    pub status: String,
    pub title: String,
    pub created_at: String,
    pub finalized_at: Option<String>,
    /// The number of items.
    pub items: usize,
    /// What this version changed, compared with the version before it in
    /// the chain. `None` for the first version of the chain.
    pub diff: Option<Diff>,
}

/// An `input` parent that names a version of an OpenSkopos artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    /// The document that has the parent.
    pub artifact: String,
    pub version: u64,
    /// The version that it names.
    pub input_artifact: String,
    pub input_version: u64,
    /// True if that version is one of the documents.
    pub present: bool,
}

/// A thing that the history could not see or used in a special way.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Warning {
    /// The artifact, if the warning is about one.
    pub artifact: Option<String>,
    pub message: String,
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// The state of an item now, in the latest version of its artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemState {
    /// Accepted in the latest version, and no item replaces it.
    InForce,
    /// Proposed in the latest version: not decided yet.
    Proposed,
    /// Rejected in the latest version. It stays in the record.
    Rejected,
    /// An item that is not rejected replaces it with `supersedes`, in this
    /// artifact or in another one (directly or through a carried item).
    Superseded,
    /// Carried into another artifact by compaction via `derived_from` (0.3).
    Carried,
    /// Removed, and no item replaces or carries it.
    Dropped,
}

impl ItemState {
    /// The state in words, for example `in force`.
    pub fn words(self) -> &'static str {
        match self {
            ItemState::InForce => "in force",
            ItemState::Proposed => "proposed",
            ItemState::Rejected => "rejected",
            ItemState::Superseded => "superseded",
            ItemState::Carried => "carried",
            ItemState::Dropped => "dropped",
        }
    }
}

/// What a version did to an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Change {
    /// The item is in the first version of the chain that was given, and
    /// that version is not version 1, so it can be older.
    Seen,
    /// The item is new in this version.
    Added,
    /// A field of the item changed.
    Changed,
    /// The item is removed, and an item of this version replaces it.
    Superseded,
    /// The item is removed, and no item replaces it.
    Dropped,
}

impl Change {
    fn words(self) -> &'static str {
        match self {
            Change::Seen => "seen",
            Change::Added => "added",
            Change::Changed => "changed",
            Change::Superseded => "superseded",
            Change::Dropped => "dropped",
        }
    }
}

/// One step in the life of an item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub version: u64,
    pub change: Change,
    /// The `status` of the item after the step. Absent if the item was
    /// removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// For `changed`: the fields that changed, in alphabetical order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
    /// For `changed`: the old status, if the status changed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_from: Option<String>,
    /// For `changed`: the new status, if the status changed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_to: Option<String>,
    /// For `superseded`: the IDs of the items of this version that replace
    /// it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub by: Vec<String>,
    /// For `seen`, `added` and `changed`: the whole item after the step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<Value>,
}

/// The life of one item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemHistory {
    /// `<artifact id>#<item id>`: the same for the item in every version.
    pub key: String,
    pub artifact: String,
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    /// The text in the last version that has the item.
    pub text: String,
    /// The status in the last version that has the item.
    pub status: String,
    /// The reference string of the item in the last version that has it,
    /// for example `pa_2Xc5Rv8Tn1Mb4Kz7@4#FR2`.
    pub reference: String,
    /// The first version given that has the item.
    pub first_version: u64,
    /// The last version given that has the item.
    pub last_version: u64,
    pub state: ItemState,
    /// True if a field of the item changed in a version after the first.
    pub changed: bool,
    /// The changes, in version order. A version that did not change the
    /// item has no step.
    pub steps: Vec<Step>,
    /// Reference strings of the items that this item replaces.
    pub supersedes: Vec<String>,
    /// Reference strings of the items that replace this item.
    pub superseded_by: Vec<String>,
    /// Reference strings of the items in other artifacts that this item
    /// comes from.
    pub derived_from: Vec<String>,
    /// Reference strings of the items in other artifacts that come from this
    /// item.
    pub derived_into: Vec<String>,
}

fn str_of<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

fn refs(item: &Value) -> impl Iterator<Item = &Value> {
    item.get("refs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

fn push_new(list: &mut Vec<String>, value: String) {
    if !list.contains(&value) {
        list.push(value);
    }
}

/// The artifact versions that a document has as `input` or `replaces` parents.
fn input_parents(doc: &Value) -> Vec<(String, u64)> {
    doc.get("parents")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|p| matches!(str_of(p, "rel"), "input" | "replaces"))
        .filter_map(|p| {
            let artifact = p.get("artifact")?.as_str()?;
            let version = p.get("version").and_then(crate::whole_number)?;
            Some((artifact.to_string(), version))
        })
        .collect()
}

/// The artifact versions that a document has as `replaces` parents.
fn replaces_parents(doc: &Value) -> Vec<(String, u64)> {
    doc.get("parents")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|p| str_of(p, "rel") == "replaces")
        .filter_map(|p| {
            let artifact = p.get("artifact")?.as_str()?;
            let version = p.get("version").and_then(crate::whole_number)?;
            Some((artifact.to_string(), version))
        })
        .collect()
}

/// True if an item was already settled in its last version (an accepted
/// answered question, a critique with a non-open resolution, or a refuted
/// assumption), so removing it in a compacted next version is not a drop.
fn is_settled(item: &Value) -> bool {
    match str_of(item, "type") {
        "open_question" => {
            !str_of(item, "answer").trim().is_empty() && str_of(item, "status") == "accepted"
        }
        "critique" => item
            .get("resolution")
            .is_some_and(|r| !matches!(str_of(r, "outcome"), "" | "open")),
        "assumption" => str_of(item, "confirmation") == "refuted",
        _ => false,
    }
}

/// True if a `derived_from` edge from `source` to `target` carries the item
/// across a compaction (rather than an ordinary derivation such as a
/// requirement derived from a decision):
/// - `target`'s artifact has a `replaces` parent naming `source`'s artifact, or
/// - `source` is a `term` carried into another `term` (in a glossary) after
///   `source`'s artifact was replaced or compacted.
fn is_carrying_link(
    source: &ItemHistory,
    target: &ItemHistory,
    replaced_by: &BTreeMap<&str, BTreeSet<&str>>,
    removed_by_compaction: bool,
) -> bool {
    if replaced_by
        .get(source.artifact.as_str())
        .is_some_and(|replacers| replacers.contains(target.artifact.as_str()))
    {
        return true;
    }
    let source_compacted =
        removed_by_compaction || replaced_by.contains_key(source.artifact.as_str());
    source_compacted && source.item_type == "term" && target.item_type == "term"
}

fn versions_words(artifact: &str, from: u64, to: u64) -> String {
    if from == to {
        format!("version {from} of {artifact} is")
    } else {
        format!("versions {from} to {to} of {artifact} are")
    }
}

/// Reads documents into their history. See the [module](self) documentation.
///
/// The function works on any JSON values and does not check them: use
/// [`crate::check`] first. A document with no artifact ID or no version is
/// left out with a warning.
///
/// ```
/// use serde_json::json;
/// let v1 = json!({"id": "pa_1", "version": 1, "items": [
///     {"id": "FR1", "type": "requirement", "text": "Old rule.", "status": "accepted"}
/// ]});
/// let v2 = json!({"id": "pa_1", "version": 2, "items": [
///     {"id": "FR2", "type": "requirement", "text": "New rule.", "status": "accepted",
///      "refs": [{"item": "FR1", "rel": "supersedes"}]}
/// ]});
/// let h = skopos_core::history::history(&[v2, v1]);
/// let fr1 = h.item("pa_1#FR1").unwrap();
/// assert_eq!(fr1.state, skopos_core::history::ItemState::Superseded);
/// assert_eq!(fr1.superseded_by, vec!["pa_1@2#FR2"]);
/// ```
pub fn history(docs: &[Value]) -> History {
    let mut warnings = Vec::new();

    // The documents of each artifact, by version.
    let mut by_artifact: BTreeMap<String, BTreeMap<u64, &Value>> = BTreeMap::new();
    for (i, doc) in docs.iter().enumerate() {
        let artifact = doc
            .get("id")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty());
        let version = doc.get("version").and_then(crate::whole_number);
        let (Some(artifact), Some(version)) = (artifact, version) else {
            warnings.push(Warning {
                artifact: None,
                message: format!(
                    "document {} has no artifact ID or no version, so it is left out",
                    i + 1
                ),
            });
            continue;
        };
        let versions = by_artifact.entry(artifact.to_string()).or_default();
        match versions.get(&version) {
            None => {
                versions.insert(version, doc);
            }
            Some(first) if *first == doc => {}
            Some(_) => warnings.push(Warning {
                artifact: Some(artifact.to_string()),
                message: format!(
                    "two different documents are {}, so the first one is used",
                    crate::version_label(artifact, Some(version))
                ),
            }),
        }
    }

    // The order of the chains: inputs first.
    let inputs: BTreeMap<&str, BTreeSet<&str>> = by_artifact
        .iter()
        .map(|(artifact, versions)| {
            let set = versions
                .values()
                .flat_map(|doc| input_parents(doc))
                .filter_map(|(a, _)| by_artifact.get_key_value(&a).map(|(k, _)| k.as_str()))
                .filter(|a| a != artifact)
                .collect();
            (artifact.as_str(), set)
        })
        .collect();
    // Kahn's algorithm: each step takes the smallest artifact ID whose
    // inputs are all placed, as the loop it replaces did.
    let mut waiting: BTreeMap<&str, usize> =
        inputs.iter().map(|(a, ins)| (*a, ins.len())).collect();
    let mut users: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (a, ins) in &inputs {
        for i in ins {
            users.entry(*i).or_default().push(*a);
        }
    }
    let mut ready: BTreeSet<&str> = waiting
        .iter()
        .filter(|(_, n)| **n == 0)
        .map(|(a, _)| *a)
        .collect();
    let mut placed: BTreeSet<&str> = BTreeSet::new();
    let mut order: Vec<&str> = Vec::new();
    while order.len() < inputs.len() {
        let next = match ready.pop_first() {
            Some(a) => a,
            // A loop of inputs: take the first artifact that is left.
            None => *inputs
                .keys()
                .find(|a| !placed.contains(*a))
                .expect("an artifact is left"),
        };
        placed.insert(next);
        order.push(next);
        for u in users.get(next).into_iter().flatten() {
            let n = waiting.get_mut(u).expect("a user waits");
            *n = n.saturating_sub(1);
            if *n == 0 && !placed.contains(u) {
                ready.insert(*u);
            }
        }
    }

    let mut chains = Vec::new();
    let mut links = Vec::new();
    let mut items: Vec<ItemHistory> = Vec::new();
    let mut index: BTreeMap<String, usize> = BTreeMap::new();
    // The last value of each item, in the order of `items`.
    let mut latest: Vec<Value> = Vec::new();
    // The latest version of each artifact.
    let mut latest_version: BTreeMap<&str, u64> = BTreeMap::new();

    for &artifact in &order {
        let versions = &by_artifact[artifact];
        let first = *versions.keys().next().expect("a chain has a version");
        if first > 1 {
            warnings.push(Warning {
                artifact: Some(artifact.to_string()),
                message: format!(
                    "{} missing, so its history starts at version {first}",
                    versions_words(artifact, 1, first - 1)
                ),
            });
        }
        let mut entries = Vec::new();
        let mut prev: Option<(u64, &Value)> = None;
        for (&version, &doc) in versions {
            if let Some((p, _)) = prev {
                if version > p + 1 {
                    warnings.push(Warning {
                        artifact: Some(artifact.to_string()),
                        message: format!(
                            "{} missing, so version {version} is compared with version {p}",
                            versions_words(artifact, p + 1, version - 1)
                        ),
                    });
                }
            }
            for (input_artifact, input_version) in input_parents(doc) {
                let present = by_artifact
                    .get(&input_artifact)
                    .is_some_and(|v| v.contains_key(&input_version));
                links.push(Link {
                    artifact: artifact.to_string(),
                    version,
                    input_artifact,
                    input_version,
                    present,
                });
            }

            let doc_items = items_by_id(doc);
            let mut values: BTreeMap<&str, Value> = BTreeMap::new();
            for (id, map) in &doc_items {
                let key = format!("{artifact}#{id}");
                let value = Value::Object((*map).clone());
                let i = *index.entry(key.clone()).or_insert_with(|| {
                    items.push(ItemHistory {
                        key,
                        artifact: artifact.to_string(),
                        id: id.to_string(),
                        item_type: String::new(),
                        text: String::new(),
                        status: String::new(),
                        reference: String::new(),
                        first_version: version,
                        last_version: version,
                        state: ItemState::Dropped,
                        changed: false,
                        steps: Vec::new(),
                        supersedes: Vec::new(),
                        superseded_by: Vec::new(),
                        derived_from: Vec::new(),
                        derived_into: Vec::new(),
                    });
                    latest.push(Value::Null);
                    items.len() - 1
                });
                items[i].last_version = version;
                latest[i] = value.clone();
                values.insert(id, value);
            }
            let step = |change: Change, id: &str| Step {
                version,
                change,
                status: values.get(id).map(|v| str_of(v, "status").to_string()),
                fields: Vec::new(),
                status_from: None,
                status_to: None,
                by: Vec::new(),
                item: values.get(id).cloned(),
            };
            let mut steps: Vec<(String, Step)> = Vec::new();
            let d = prev.map(|(_, p)| diff(p, doc));
            match &d {
                None => {
                    let change = if version == 1 {
                        Change::Added
                    } else {
                        Change::Seen
                    };
                    for (id, _) in &doc_items {
                        steps.push((id.to_string(), step(change, id)));
                    }
                }
                Some(d) => {
                    for a in &d.added {
                        steps.push((a.id.clone(), step(Change::Added, &a.id)));
                    }
                    for c in &d.changed {
                        let mut s = step(Change::Changed, &c.id);
                        s.fields = c.fields.clone();
                        s.status_from = c.status_from.clone();
                        s.status_to = c.status_to.clone();
                        steps.push((c.id.clone(), s));
                    }
                    for s in &d.superseded {
                        let mut st = step(Change::Superseded, &s.id);
                        st.by = s.by.clone();
                        steps.push((s.id.clone(), st));
                    }
                    for s in &d.dropped {
                        steps.push((s.id.clone(), step(Change::Dropped, &s.id)));
                    }
                }
            }
            for (id, s) in steps {
                let i = index[&format!("{artifact}#{id}")];
                items[i].changed |= s.change == Change::Changed;
                items[i].steps.push(s);
            }
            entries.push(VersionEntry {
                version,
                status: str_of(doc, "status").to_string(),
                title: str_of(doc, "title").to_string(),
                created_at: str_of(doc, "created_at").to_string(),
                finalized_at: doc
                    .get("finalized_at")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                items: doc_items.len(),
                diff: d,
            });
            prev = Some((version, doc));
        }
        let (&last, last_doc) = versions.iter().next_back().expect("a chain has a version");
        latest_version.insert(artifact, last);
        chains.push(Chain {
            artifact: artifact.to_string(),
            kind: str_of(last_doc, "kind").to_string(),
            title: str_of(last_doc, "title").to_string(),
            versions: entries,
        });
    }

    for (item, value) in items.iter_mut().zip(&latest) {
        item.item_type = str_of(value, "type").to_string();
        item.text = str_of(value, "text").to_string();
        item.status = str_of(value, "status").to_string();
        item.reference = crate::reference_string(&item.artifact, Some(item.last_version), &item.id);
    }

    // The reference string of an item that a ref names: the one of the item
    // if it is in the history, or else the one that the ref gives.
    let target = |items: &[ItemHistory], artifact: &str, r: &Value| -> (Option<usize>, String) {
        let item = str_of(r, "item");
        let art = r
            .get("artifact")
            .and_then(Value::as_str)
            .unwrap_or(artifact);
        match index.get(&format!("{art}#{item}")) {
            Some(&i) => (Some(i), items[i].reference.clone()),
            None => {
                let version = r.get("version").and_then(crate::whole_number);
                (None, crate::reference_string(art, version, item))
            }
        }
    };

    // Track which artifacts are replaced by which other artifacts (via a
    // cross-artifact `replaces` parent), and which specific `(artifact, version)`
    // pairs were compacted into their own next version (a self-`replaces` parent).
    let mut replaced_by: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut compacted_versions: BTreeSet<(&str, u64)> = BTreeSet::new();
    for (artifact, versions) in &by_artifact {
        for (&version, &doc) in versions {
            for (rep_art, rep_ver) in replaces_parents(doc) {
                if rep_art == *artifact && rep_ver < version {
                    compacted_versions.insert((artifact.as_str(), version));
                } else if let Some((k, _)) = by_artifact.get_key_value(&rep_art) {
                    if rep_art != *artifact {
                        replaced_by
                            .entry(k.as_str())
                            .or_default()
                            .insert(artifact.as_str());
                    }
                }
            }
        }
    }

    // Track `derived_from` edges between artifacts so we can identify carried
    // items and propagate supersession backward across repeated compactions.
    let mut derived_edges: Vec<Vec<usize>> = vec![Vec::new(); items.len()];
    for i in 0..items.len() {
        let artifact = items[i].artifact.clone();
        let in_latest = items[i].last_version == latest_version[artifact.as_str()];
        let rejected = items[i].status == "rejected";
        for r in refs(&latest[i]) {
            let rel = str_of(r, "rel");
            let other_artifact = !crate::diff::local_ref(r, &artifact);
            if rel == "supersedes" && in_latest && !rejected {
                let (t, reference) = target(&items, &artifact, r);
                push_new(&mut items[i].supersedes, reference);
                if let Some(t) = t {
                    let by = items[i].reference.clone();
                    push_new(&mut items[t].superseded_by, by);
                }
            } else if rel == "derived_from" && other_artifact {
                let (t, reference) = target(&items, &artifact, r);
                push_new(&mut items[i].derived_from, reference);
                if let Some(t) = t {
                    let into = items[i].reference.clone();
                    push_new(&mut items[t].derived_into, into);
                    if !derived_edges[t].contains(&i) {
                        derived_edges[t].push(i);
                    }
                }
            }
        }
    }

    // Add same-artifact supersession recorded on removal steps before
    // propagating supersession backward across carried items.
    for i in 0..items.len() {
        let latest_v = latest_version[items[i].artifact.as_str()];
        let item = &items[i];
        let removed_by: Vec<String> = match item.steps.last() {
            Some(s) if item.last_version < latest_v && s.change == Change::Superseded => {
                let artifact = &item.artifact;
                s.by.iter()
                    .map(|id| match index.get(&format!("{artifact}#{id}")) {
                        Some(&j) => items[j].reference.clone(),
                        None => crate::reference_string(artifact, Some(s.version), id),
                    })
                    .collect()
            }
            _ => Vec::new(),
        };
        let item = &mut items[i];
        for by in removed_by {
            push_new(&mut item.superseded_by, by);
        }
    }

    // Precompute which items were removed by a compacted next version and
    // which outgoing `derived_from` edges actually carry an item across a
    // compaction (rather than an ordinary `derived_from` link such as FR1
    // derived from D1).
    let removed_by_compaction: Vec<bool> = (0..items.len())
        .map(|i| {
            let art = items[i].artifact.as_str();
            let latest_v = latest_version[art];
            items[i].last_version < latest_v
                && items[i].steps.last().is_some_and(|s| {
                    s.change == Change::Dropped && compacted_versions.contains(&(art, s.version))
                })
        })
        .collect();
    let carried_edges: Vec<Vec<usize>> = (0..items.len())
        .map(|i| {
            derived_edges[i]
                .iter()
                .copied()
                .filter(|&j| {
                    is_carrying_link(&items[i], &items[j], &replaced_by, removed_by_compaction[i])
                })
                .collect()
        })
        .collect();

    // Propagate supersession backward across repeated compactions: if an item
    // was carried into a downstream item that is superseded (directly or
    // transitively), record those same superseders on the carried item.
    loop {
        let mut changed = false;
        for i in 0..items.len() {
            if carried_edges[i].is_empty() {
                continue;
            }
            let downstream: Vec<String> = carried_edges[i]
                .iter()
                .flat_map(|&j| items[j].superseded_by.iter().cloned())
                .collect();
            for by in downstream {
                if !items[i].superseded_by.contains(&by) {
                    items[i].superseded_by.push(by);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }

    for i in 0..items.len() {
        let latest_v = latest_version[items[i].artifact.as_str()];
        let carried = !carried_edges[i].is_empty();
        let compacted = removed_by_compaction[i];
        let settled = is_settled(&latest[i]);
        let item = &mut items[i];
        item.state = if !item.superseded_by.is_empty() {
            ItemState::Superseded
        } else if carried {
            ItemState::Carried
        } else if item.last_version < latest_v {
            if compacted && item.status == "rejected" {
                ItemState::Rejected
            } else if compacted && settled {
                if item.status == "accepted" {
                    ItemState::InForce
                } else {
                    ItemState::Proposed
                }
            } else {
                ItemState::Dropped
            }
        } else if item.status == "rejected" {
            ItemState::Rejected
        } else if item.status == "accepted" {
            ItemState::InForce
        } else {
            ItemState::Proposed
        };
    }

    History {
        chains,
        links,
        items,
        warnings,
    }
}

impl History {
    /// The item with this key, for example `pa_2Xc5Rv8Tn1Mb4Kz7#FR2`.
    pub fn item(&self, key: &str) -> Option<&ItemHistory> {
        self.items.iter().find(|i| i.key == key)
    }

    /// The item that a reference string names, for example
    /// `pa_2Xc5Rv8Tn1Mb4Kz7@3#FR2`. The version is not needed: an item is the
    /// same item in every version. An item ID alone, such as `FR2`, works if
    /// only one artifact has it.
    pub fn find(&self, reference: &str) -> Result<&ItemHistory, String> {
        let (artifact, id) = match reference.split_once('#') {
            Some((left, id)) => {
                let artifact = left.split_once('@').map_or(left, |(a, _)| a);
                (Some(artifact).filter(|a| !a.is_empty()), id)
            }
            None => (None, reference),
        };
        let found: Vec<&ItemHistory> = self
            .items
            .iter()
            .filter(|i| i.id == id && artifact.is_none_or(|a| a == i.artifact))
            .collect();
        match found.as_slice() {
            [one] => Ok(one),
            [] => Err(format!("no document has the item {reference}")),
            many => Err(format!(
                "{id} is in more than one artifact: {}. Give the artifact ID too, for example {}#{id}",
                many.iter()
                    .map(|i| i.artifact.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                many[0].artifact
            )),
        }
    }

    /// The history as text for people: what each version changed, then the
    /// items in force now and the items that are still proposed. With
    /// `since`, the list of versions leaves out each version with that number
    /// or a lower one, in every artifact.
    ///
    /// ```text
    /// Seat limits on plan changes (spec pa_2Xc5Rv8Tn1Mb4Kz7)
    ///   version 2  final  12 items
    ///   version 3  final  14 items
    ///     added      FR4  A plan change that leaves more seats ...
    ///     superseded FR2  by FR4
    ///
    /// In force now
    ///   pa_2Xc5Rv8Tn1Mb4Kz7@3
    ///     G2   Teams can move to a smaller plan without support.
    /// ```
    pub fn to_text(&self, since: Option<u64>) -> String {
        let mut out = String::new();
        self.write_text(&mut out, since)
            .expect("writing to a String cannot fail");
        out
    }

    fn write_text(&self, out: &mut String, since: Option<u64>) -> fmt::Result {
        for w in &self.warnings {
            writeln!(out, "warning: {w}")?;
        }
        if !self.warnings.is_empty() {
            writeln!(out)?;
        }
        if self.chains.is_empty() {
            return writeln!(out, "no documents");
        }
        for chain in &self.chains {
            let kind = if chain.kind.is_empty() {
                String::new()
            } else {
                format!("{} ", chain.kind)
            };
            writeln!(
                out,
                "{} ({kind}{})",
                crate::one_line(&chain.title),
                chain.artifact
            )?;
            let mut shown = 0;
            for v in &chain.versions {
                if since.is_some_and(|s| v.version <= s) {
                    continue;
                }
                shown += 1;
                write!(
                    out,
                    "  version {}  {}  {}",
                    v.version,
                    v.status,
                    crate::date_of(v.finalized_at.as_deref().unwrap_or(&v.created_at))
                )?;
                writeln!(out, "  {}", plural(v.items, "item"))?;
                let inputs: Vec<String> = self
                    .links
                    .iter()
                    .filter(|l| l.artifact == chain.artifact && l.version == v.version && l.present)
                    .map(|l| crate::version_label(&l.input_artifact, Some(l.input_version)))
                    .collect();
                if !inputs.is_empty() {
                    writeln!(out, "    input      {}", inputs.join(", "))?;
                }
                if let Some(d) = &v.diff {
                    if d.is_empty() {
                        writeln!(out, "    no changes")?;
                    } else {
                        d.write_lines(out, "    ")?;
                    }
                }
                // Items that this version added and that replace items of
                // other artifacts.
                for i in self.items.iter().filter(|i| {
                    i.artifact == chain.artifact
                        && i.steps.iter().any(|s| {
                            s.version == v.version
                                && matches!(s.change, Change::Added | Change::Seen)
                        })
                }) {
                    for s in i
                        .supersedes
                        .iter()
                        .filter(|s| s.split(['@', '#']).next() != Some(&chain.artifact))
                    {
                        writeln!(out, "    superseded {s}  by {}", i.id)?;
                    }
                }
            }
            if shown == 0 {
                writeln!(out, "  no versions after version {}", since.unwrap_or(0))?;
            }
            writeln!(out)?;
        }
        self.write_state(out, ItemState::InForce, "In force now")?;
        self.write_state(out, ItemState::Proposed, "Still proposed")
    }

    fn write_state(&self, out: &mut String, state: ItemState, heading: &str) -> fmt::Result {
        if !self.items.iter().any(|i| i.state == state) {
            return Ok(());
        }
        if state != ItemState::InForce {
            writeln!(out)?;
        }
        writeln!(out, "{heading}")?;
        for chain in &self.chains {
            let list: Vec<&ItemHistory> = self
                .items
                .iter()
                .filter(|i| i.artifact == chain.artifact && i.state == state)
                .collect();
            let Some(last) = chain.versions.last() else {
                continue;
            };
            if list.is_empty() {
                continue;
            }
            writeln!(
                out,
                "  {}",
                crate::version_label(&chain.artifact, Some(last.version))
            )?;
            let width = list.iter().map(|i| i.id.len()).max().unwrap_or(0);
            for i in list {
                writeln!(out, "    {:width$}  {}", i.id, crate::one_line(&i.text))?;
            }
        }
        Ok(())
    }

    /// The life of one item as text for people: each step, the state now,
    /// and the items that replace it, with their reasons.
    ///
    /// ```text
    /// pa_2Xc5Rv8Tn1Mb4Kz7#FR2  requirement
    /// now: superseded by pa_2Xc5Rv8Tn1Mb4Kz7@5#FR4
    ///
    /// version 2  added       accepted  Block a plan change that ...
    /// version 4  superseded  by FR4
    ///
    /// pa_2Xc5Rv8Tn1Mb4Kz7@5#FR4 (accepted)
    ///   A plan change that leaves more seats ...
    ///   rationale: ...
    /// ```
    pub fn trace_text(&self, item: &ItemHistory) -> String {
        let mut out = String::new();
        self.write_trace(&mut out, item)
            .expect("writing to a String cannot fail");
        out
    }

    fn write_trace(&self, out: &mut String, item: &ItemHistory) -> fmt::Result {
        writeln!(out, "{}  {}", item.key, item.item_type)?;
        write!(out, "now: {}", item.state.words())?;
        if !item.superseded_by.is_empty() {
            write!(out, " by {}", item.superseded_by.join(", "))?;
        }
        writeln!(
            out,
            " (last in {})",
            crate::version_label(&item.artifact, Some(item.last_version))
        )?;
        for (words, list) in [
            ("supersedes", &item.supersedes),
            ("derived from", &item.derived_from),
            ("derived into", &item.derived_into),
        ] {
            if !list.is_empty() {
                writeln!(out, "{words}: {}", list.join(", "))?;
            }
        }
        writeln!(out)?;
        let mut last_text: Option<&str> = None;
        for s in &item.steps {
            write!(out, "version {}  {:10}", s.version, s.change.words())?;
            match s.change {
                Change::Superseded => write!(out, "  by {}", s.by.join(", "))?,
                Change::Dropped => {}
                Change::Changed => {
                    write!(out, "  {}", s.fields.join(", "))?;
                    if let (Some(from), Some(to)) = (&s.status_from, &s.status_to) {
                        write!(out, " (status {from} -> {to})")?;
                    }
                }
                Change::Seen | Change::Added => {
                    write!(out, "  {}", s.status.as_deref().unwrap_or(""))?;
                }
            }
            writeln!(out)?;
            let text = s.item.as_ref().map(|v| str_of(v, "text"));
            if let Some(text) = text.filter(|t| Some(*t) != last_text) {
                writeln!(out, "    {}", crate::one_line(text))?;
                last_text = Some(text);
            }
        }
        for by in &item.superseded_by {
            let Ok(other) = self.find(by) else {
                continue;
            };
            writeln!(out)?;
            writeln!(out, "{} ({})", other.reference, other.status)?;
            writeln!(out, "  {}", crate::one_line(&other.text))?;
            let value = other.steps.iter().rev().find_map(|s| s.item.as_ref());
            if let Some(value) = value {
                write_reasons(out, value)?;
            }
        }
        Ok(())
    }
}

/// The fields of an item that say why: `rationale`, `detail` and evidence.
fn write_reasons(out: &mut String, item: &Value) -> fmt::Result {
    for key in ["rationale", "reason", "detail"] {
        if let Some(text) = item.get(key).and_then(Value::as_str) {
            writeln!(out, "  {key}: {}", crate::one_line(text))?;
        }
    }
    for e in item
        .get("evidence")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let line = match str_of(e, "kind") {
            "quote" => format!("quote: \"{}\"", crate::one_line(str_of(e, "text"))),
            "discussion" => match e.get("speaker").and_then(Value::as_str) {
                Some(who) => format!("{who} said: \"{}\"", crate::one_line(str_of(e, "text"))),
                None => format!("discussion: \"{}\"", crate::one_line(str_of(e, "text"))),
            },
            "code" => {
                let start = e
                    .get("line_start")
                    .map(Value::to_string)
                    .unwrap_or_default();
                format!("code: {}:{start}", str_of(e, "path"))
            }
            "link" => format!("link: {}", str_of(e, "uri")),
            _ => continue,
        };
        writeln!(out, "  evidence: {line}")?;
    }
    Ok(())
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn doc(version: u64, items: Value) -> Value {
        json!({"id": "pa_1", "version": version, "kind": "spec", "title": "T",
               "status": "final", "created_at": "2026-11-0{version}T09:00:00Z", "items": items})
    }

    #[test]
    fn order_of_the_documents_does_not_matter() {
        let v1 = doc(
            1,
            json!([{"id": "G1", "type": "goal", "text": "A.", "status": "proposed"}]),
        );
        let v2 = doc(
            2,
            json!([{"id": "G1", "type": "goal", "text": "A.", "status": "accepted"}]),
        );
        let a = history(&[v1.clone(), v2.clone()]);
        let b = history(&[v2, v1]);
        assert_eq!(a, b);
        let g1 = a.item("pa_1#G1").unwrap();
        assert_eq!(g1.state, ItemState::InForce);
        assert_eq!(g1.steps.len(), 2);
        assert_eq!(g1.steps[1].status_from.as_deref(), Some("proposed"));
        assert!(g1.changed);
    }

    #[test]
    fn a_gap_is_a_warning_and_the_sides_are_compared() {
        let v1 = doc(
            1,
            json!([{"id": "G1", "type": "goal", "text": "A.", "status": "accepted"}]),
        );
        let v3 = doc(3, json!([]));
        let h = history(&[v1, v3]);
        assert_eq!(h.warnings.len(), 1);
        assert!(h.warnings[0]
            .message
            .contains("version 2 of pa_1 is missing"));
        let g1 = h.item("pa_1#G1").unwrap();
        assert_eq!(g1.state, ItemState::Dropped);
        assert_eq!(g1.steps[1].version, 3);
        assert_eq!(g1.steps[1].change, Change::Dropped);
    }

    #[test]
    fn a_chain_that_starts_late_has_seen_items() {
        let v2 = doc(
            2,
            json!([{"id": "G1", "type": "goal", "text": "A.", "status": "accepted"}]),
        );
        let h = history(&[v2]);
        assert!(h.warnings[0]
            .message
            .contains("version 1 of pa_1 is missing"));
        assert_eq!(h.items[0].steps[0].change, Change::Seen);
    }

    #[test]
    fn an_item_still_in_the_document_can_be_superseded() {
        let v1 = doc(
            1,
            json!([
                {"id": "D1", "type": "decision", "text": "Old.", "status": "accepted"},
                {"id": "D2", "type": "decision", "text": "New.", "status": "accepted",
                 "refs": [{"item": "D1", "rel": "supersedes"}]}
            ]),
        );
        let h = history(&[v1]);
        assert_eq!(h.item("pa_1#D1").unwrap().state, ItemState::Superseded);
        assert_eq!(h.item("pa_1#D2").unwrap().supersedes, vec!["pa_1@1#D1"]);
    }

    #[test]
    fn find_needs_the_artifact_when_two_have_the_id() {
        let a = doc(
            1,
            json!([{"id": "G1", "type": "goal", "text": "A.", "status": "accepted"}]),
        );
        let mut b = a.clone();
        b["id"] = json!("pa_2");
        let h = history(&[a, b]);
        assert!(h.find("G1").unwrap_err().contains("more than one artifact"));
        assert_eq!(h.find("pa_2@1#G1").unwrap().artifact, "pa_2");
        assert!(h.find("pa_3#G1").is_err());
    }

    #[test]
    fn duplicates_and_documents_without_an_id_are_reported() {
        let v1 = doc(1, json!([]));
        let mut other = v1.clone();
        other["title"] = json!("Other");
        let h = history(&[v1.clone(), v1.clone(), other, json!({"items": []})]);
        assert_eq!(h.warnings.len(), 2);
        assert_eq!(h.chains[0].title, "T");
    }

    #[test]
    fn chains_order_inputs_first_with_lexicographic_ties_and_cycles() {
        let make = |id: &str, inputs: &[&str]| {
            let parents: Vec<Value> = inputs
                .iter()
                .map(|a| json!({"rel": "input", "artifact": a, "version": 1}))
                .collect();
            json!({
                "id": id, "version": 1, "kind": "spec", "title": id,
                "status": "final", "created_at": "2026-11-01T09:00:00Z",
                "parents": parents, "items": []
            })
        };
        // Diamond: d depends on b and c; both depend on a.
        // Expected: a, b, c, d regardless of input order.
        let h = history(&[
            make("pa_d", &["pa_b", "pa_c"]),
            make("pa_c", &["pa_a"]),
            make("pa_b", &["pa_a"]),
            make("pa_a", &[]),
        ]);
        let order: Vec<&str> = h.chains.iter().map(|c| c.artifact.as_str()).collect();
        assert_eq!(order, ["pa_a", "pa_b", "pa_c", "pa_d"]);

        // Cycle between pa_b and pa_c, plus pa_d depending on pa_c, plus independent pa_a.
        // Expected: pa_a (ready), then cycle break picks smallest pa_b, which unlocks pa_c, then pa_d.
        let h = history(&[
            make("pa_d", &["pa_c"]),
            make("pa_c", &["pa_b"]),
            make("pa_b", &["pa_c"]),
            make("pa_a", &[]),
        ]);
        let order: Vec<&str> = h.chains.iter().map(|c| c.artifact.as_str()).collect();
        assert_eq!(order, ["pa_a", "pa_b", "pa_c", "pa_d"]);
    }
}
