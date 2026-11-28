//! Compact a set of documents into their current state.
//!
//! [`compact`] takes the documents of one area: every version that the team
//! has of each artifact. It keeps the items that are current and writes one
//! draft per kind of artifact:
//!
//! - An item is current when its state ([`crate::history`]) is in force or
//!   proposed. Rejected, superseded (by an accepted item) and dropped items
//!   are left out, and so are answered questions, critiques with a non-open
//!   resolution and refuted assumptions.
//! - When a kind has one artifact, the draft is its next version. Items keep
//!   their IDs, so reference strings keep working.
//! - When a kind has several artifacts, the draft is a new artifact. Its
//!   items get new IDs and a `derived_from` ref to the item they come from,
//!   and each source artifact is an `input` parent.
//! - Terms of every artifact go to the glossary draft. Two terms with the
//!   same name and definition collapse into one, keeping `accepted` over
//!   `proposed`, combining `aliases`, `code_names` and `evidence`, and
//!   keeping `derived_from` refs to each source. Two terms with the same name
//!   and different definitions (or different `detail`) both stay, as
//!   `proposed`, with a `contradicts` ref, like a conflict of
//!   [`crate::merge`].
//! - Refs between current items are rewritten to the new IDs. A
//!   `supersedes` ref keeps pointing to the old item. Refs to items in
//!   unchanged artifacts or outside the compacted set are preserved. Other
//!   refs to items that are left out are removed.
//! - Participant tables are merged and participant IDs on moved items are
//!   remapped so IDs never collide or dangle.
//!
//! A kind whose artifact loses no items, gets none, and has no item changes
//! stays unchanged and has no draft. The function is pure: the caller gives
//! the time, optional context documents from other areas, optional titles for
//! new artifacts, and the IDs of new artifacts. It does not move files; that
//! is the job of a tool.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::history::{history, ItemState, Warning};
use crate::Problem;

/// Options for [`compact`].
#[derive(Debug, Clone, Default)]
pub struct Options<'a> {
    /// The time for `created_at` of each draft, for example
    /// `2026-11-27T12:00:00Z`. Without it, a next version keeps the time of
    /// the version before it, and a new artifact uses the newest time of its
    /// sources (including artifacts whose terms it carries).
    pub now: Option<String>,
    /// Read-only documents from outside the compacted area, used to detect
    /// cross-area `supersedes` links without compacting those documents.
    pub context: &'a [Value],
    /// Optional titles for newly created artifacts, keyed by kind (for example
    /// `"spec"` or `"glossary"`). When absent, a new artifact takes the title
    /// of the source with the most carried items, or the kind name.
    pub titles: BTreeMap<String, String>,
}

/// Why an item is not in the drafts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    Rejected,
    Superseded,
    /// An answered open question.
    Answered,
    /// A critique with a non-open resolution.
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
    /// The latest version of each source of this kind, as
    /// `<artifact>@<version>`.
    pub sources: Vec<String>,
    /// The number of items in the draft.
    pub items: usize,
    pub doc: Value,
}

/// A kind that needed no draft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unchanged {
    pub kind: String,
    pub artifact: String,
    pub version: u64,
}

/// Two or more terms with the same name and conflicting definitions or detail.
/// All stay as `proposed` until the team rejects the others.
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
    /// Kinds that need no draft, in the order of the kinds' names.
    pub unchanged: Vec<Unchanged>,
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

fn is_non_empty_str(v: Option<&Value>) -> bool {
    v.and_then(Value::as_str)
        .is_some_and(|text| !text.trim().is_empty())
}

fn reason(item: &Value, state: ItemState, accepted_superseded: bool) -> Option<Reason> {
    match state {
        ItemState::Rejected => return Some(Reason::Rejected),
        ItemState::Superseded if accepted_superseded => return Some(Reason::Superseded),
        // A dropped item is in no latest version, so it never gets here.
        ItemState::Dropped => return Some(Reason::Superseded),
        ItemState::InForce | ItemState::Proposed | ItemState::Superseded => {}
    }
    match s(item, "type") {
        "open_question"
            if is_non_empty_str(item.get("answer")) && s(item, "status") == "accepted" =>
        {
            Some(Reason::Answered)
        }
        "critique"
            if item
                .get("resolution")
                .and_then(|r| r.get("outcome"))
                .and_then(Value::as_str)
                .is_some_and(|o| o != "open") =>
        {
            Some(Reason::Resolved)
        }
        "assumption" if s(item, "confirmation") == "refuted" => Some(Reason::Refuted),
        _ => None,
    }
}

/// Where a carried item goes: the target artifact ID and its item ID there.
#[derive(Clone)]
struct Dest {
    artifact: String,
    id: String,
}

/// One carried item before refs and participants are rewritten.
struct Carried<'a> {
    doc: &'a Value,
    item: Value,
    /// Additional `(source_doc, source_item_id)` origins merged into this item
    /// (for deduplicated terms).
    extra_sources: Vec<(&'a Value, String)>,
}

/// Merge string-array fields (such as `aliases` and `code_names`), keeping
/// order and skipping duplicates.
fn merge_string_list(target: &mut Map<String, Value>, source: &Map<String, Value>, field: &str) {
    let Some(src_list) = source.get(field).and_then(Value::as_array) else {
        return;
    };
    if src_list.is_empty() {
        return;
    }
    let dst = target
        .entry(field.to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Value::Array(dst_list) = dst {
        for v in src_list {
            if !dst_list.contains(v) {
                dst_list.push(v.clone());
            }
        }
    }
}

/// Merge `evidence` entries from `source` into `target`, skipping exact
/// duplicates and remapping `speaker` if needed later (carried with its source
/// artifact in `extra_evidence` if it has a speaker).
fn merge_evidence(
    target: &mut Map<String, Value>,
    source: &Map<String, Value>,
    source_doc: &Value,
    pmap: &mut ParticipantTable,
) {
    let Some(src_list) = source.get("evidence").and_then(Value::as_array) else {
        return;
    };
    if src_list.is_empty() {
        return;
    }
    let dst = target
        .entry("evidence".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Value::Array(dst_list) = dst {
        for entry in src_list {
            let mut remapped = entry.clone();
            if let Some(obj) = remapped.as_object_mut() {
                if let Some(spk) = obj.get("speaker").and_then(Value::as_str) {
                    if let Some(new_id) = pmap.remap(source_doc, spk) {
                        obj.insert("speaker".into(), Value::String(new_id));
                    }
                }
            }
            if !dst_list.contains(&remapped) {
                dst_list.push(remapped);
            }
        }
    }
}

/// Merged participant table for one draft. Maps `(source_artifact, old_pid)`
/// to the participant ID in the draft.
#[derive(Default)]
struct ParticipantTable {
    /// Draft participants, in order.
    entries: Vec<Value>,
    /// `(name_lower, kind)` -> draft participant ID, for deduplicating the
    /// same person across folded artifacts.
    by_identity: BTreeMap<(String, String), String>,
    /// Next participant number (`P<n>`).
    next_num: u64,
    /// `(source_artifact_id, old_pid)` -> new_pid.
    map: BTreeMap<(String, String), String>,
    /// Existing IDs already in `entries`.
    used_ids: BTreeSet<String>,
}

impl ParticipantTable {
    fn from_continued(doc: &Value) -> Self {
        let mut t = Self::default();
        let art = s(doc, "id");
        for p in doc
            .get("participants")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let id = s(p, "id");
            if id.is_empty() {
                continue;
            }
            if let Some(n) = id.strip_prefix('P').and_then(|s| s.parse::<u64>().ok()) {
                t.next_num = t.next_num.max(n);
            }
            t.used_ids.insert(id.to_string());
            let key = (s(p, "name").trim().to_lowercase(), s(p, "kind").to_string());
            t.by_identity.entry(key).or_insert_with(|| id.to_string());
            t.map
                .insert((art.to_string(), id.to_string()), id.to_string());
            t.entries.push(p.clone());
        }
        t
    }

    fn remap(&mut self, source_doc: &Value, old_pid: &str) -> Option<String> {
        let art = s(source_doc, "id");
        if let Some(mapped) = self.map.get(&(art.to_string(), old_pid.to_string())) {
            return Some(mapped.clone());
        }
        // Look up the participant definition in `source_doc`.
        let found = source_doc
            .get("participants")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .find(|p| s(p, "id") == old_pid);
        let p = found?;
        let key = (s(p, "name").trim().to_lowercase(), s(p, "kind").to_string());
        if let Some(existing) = self.by_identity.get(&key) {
            self.map
                .insert((art.to_string(), old_pid.to_string()), existing.clone());
            return Some(existing.clone());
        }
        let new_id = loop {
            self.next_num += 1;
            let cand = format!("P{}", self.next_num);
            if self.used_ids.insert(cand.clone()) {
                break cand;
            }
        };
        let mut entry = p.clone();
        if let Some(obj) = entry.as_object_mut() {
            obj.insert("id".into(), Value::String(new_id.clone()));
        }
        self.entries.push(entry);
        self.by_identity.insert(key, new_id.clone());
        self.map
            .insert((art.to_string(), old_pid.to_string()), new_id.clone());
        Some(new_id)
    }

    fn remap_item(&mut self, source_doc: &Value, obj: &mut Map<String, Value>) {
        // Direct participant ID fields on items.
        for field in ["proposed_by", "decided_by", "owner"] {
            if let Some(old) = obj.get(field).and_then(Value::as_str).map(str::to_string) {
                if let Some(new_id) = self.remap(source_doc, &old) {
                    obj.insert(field.into(), Value::String(new_id));
                }
            }
        }
        // `approver` on ask_first constraints can be a participant ID or a role.
        if let Some(old) = obj
            .get("approver")
            .and_then(Value::as_str)
            .filter(|v| is_participant_id(v))
            .map(str::to_string)
        {
            if let Some(new_id) = self.remap(source_doc, &old) {
                obj.insert("approver".into(), Value::String(new_id));
            }
        }
        // `resolution.decided_by` on critiques.
        if let Some(res) = obj.get_mut("resolution").and_then(Value::as_object_mut) {
            if let Some(old) = res
                .get("decided_by")
                .and_then(Value::as_str)
                .map(str::to_string)
            {
                if let Some(new_id) = self.remap(source_doc, &old) {
                    res.insert("decided_by".into(), Value::String(new_id));
                }
            }
        }
        // `evidence[].speaker`.
        if let Some(ev) = obj.get_mut("evidence").and_then(Value::as_array_mut) {
            for e in ev {
                if let Some(eobj) = e.as_object_mut() {
                    if let Some(old) = eobj
                        .get("speaker")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                    {
                        if let Some(new_id) = self.remap(source_doc, &old) {
                            eobj.insert("speaker".into(), Value::String(new_id));
                        }
                    }
                }
            }
        }
    }
}

fn is_participant_id(s: &str) -> bool {
    let Some(rest) = s.strip_prefix('P') else {
        return false;
    };
    !rest.is_empty() && !rest.starts_with('0') && rest.bytes().all(|b| b.is_ascii_digit())
}

/// Compacts documents. See the [module](self) documentation.
///
/// `new_id` gives the ID of each new artifact; the CLI and the WebAssembly
/// build give random IDs. Returns a problem if the latest version of an
/// artifact in `docs` is a draft: finish or discard it first.
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
    options: &Options<'_>,
    new_id: &mut dyn FnMut() -> String,
) -> Result<Compaction, Problem> {
    // Run history over the compacted area plus any read-only context documents
    // so cross-area `supersedes` links are respected.
    let h = if options.context.is_empty() {
        history(docs)
    } else {
        let mut combined = Vec::with_capacity(docs.len() + options.context.len());
        combined.extend_from_slice(docs);
        combined.extend_from_slice(options.context);
        history(&combined)
    };
    let state: BTreeMap<&str, ItemState> =
        h.items.iter().map(|i| (i.key.as_str(), i.state)).collect();
    // An item is superseded for compaction only if at least one *accepted*
    // item supersedes it (or it was already removed from its artifact). A
    // `proposed` superseder does not drop the accepted item until accepted.
    let item_status_by_ref: BTreeMap<&str, &str> = h
        .items
        .iter()
        .map(|i| (i.reference.as_str(), i.status.as_str()))
        .collect();
    let accepted_superseded: BTreeSet<&str> = h
        .items
        .iter()
        .filter(|i| {
            i.state == ItemState::Superseded
                && i.superseded_by.iter().any(|by| {
                    item_status_by_ref
                        .get(by.as_str())
                        .is_none_or(|st| *st == "accepted")
                })
        })
        .map(|i| i.key.as_str())
        .collect();

    // The latest version of each artifact in `docs`, by kind.
    let mut latest: BTreeMap<&str, &Value> = BTreeMap::new();
    for doc in docs {
        let id = s(doc, "id");
        if id.is_empty() || version_of(doc) == 0 {
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

    // Phase 1: choose what each kind carries.
    // Put existing `glossary` artifacts' items into `members["glossary"]`
    // before terms moved from other kinds (such as `agent_rules`) so the
    // glossary keeps its own IDs and acts as main in any term conflict.
    let mut left_out = Vec::new();
    let mut members: BTreeMap<&str, Vec<Carried<'_>>> = BTreeMap::new();
    let mut glossary_moved_in: Vec<Carried<'_>> = Vec::new();
    for (kind, sources) in &kinds {
        for doc in sources {
            for item in items(doc) {
                let key = format!("{}#{}", s(doc, "id"), s(item, "id"));
                let st = state
                    .get(key.as_str())
                    .copied()
                    .unwrap_or(ItemState::InForce);
                if let Some(r) = reason(item, st, accepted_superseded.contains(key.as_str())) {
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
                    continue;
                }
                let entry = Carried {
                    doc,
                    item: item.clone(),
                    extra_sources: Vec::new(),
                };
                if s(item, "type") == "term" && *kind != "glossary" {
                    glossary_moved_in.push(entry);
                } else {
                    members.entry(kind).or_default().push(entry);
                }
            }
        }
    }
    if !glossary_moved_in.is_empty() {
        members
            .entry("glossary")
            .or_default()
            .extend(glossary_moved_in);
    }

    // Deduplicate and detect conflicts among *term* items in the glossary
    // (leaving any non-term items that live in a glossary untouched).
    //
    // Same normalized name + same trimmed definition + compatible detail:
    // collapse into one term, preferring `accepted` over `proposed`, merging
    // `aliases` / `code_names` (and `evidence` during phase 2), and recording
    // all sources so every old reference gets a `derived_from` link.
    // Different definition or different non-empty detail: conflict.
    let mut conflict_names: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut duplicate_of: BTreeMap<(String, String), (String, String)> = BTreeMap::new();
    let mut merged_term_extras: BTreeMap<usize, Vec<(&Value, Value)>> = BTreeMap::new();
    let mut glossary_term_changed = false;
    if let Some(entries) = members.get_mut("glossary") {
        // Key: (normalized name, trimmed definition, trimmed detail) -> index in `kept`.
        let mut by_key: BTreeMap<(String, String, String), usize> = BTreeMap::new();
        // Also track (normalized name, trimmed definition) -> indices in `kept`
        // so an entry with no detail can fold into an entry with detail (or vice versa)
        // when the other side has no conflicting detail.
        let mut by_name_def: BTreeMap<(String, String), Vec<usize>> = BTreeMap::new();
        let mut kept: Vec<Carried<'_>> = Vec::new();

        for c in entries.drain(..) {
            if s(&c.item, "type") != "term" {
                kept.push(c);
                continue;
            }
            let name = s(&c.item, "text").trim().to_lowercase();
            let def = s(&c.item, "definition").trim().to_string();
            let detail = s(&c.item, "detail").trim().to_string();
            let me = (s(c.doc, "id").to_string(), s(&c.item, "id").to_string());

            // Find an existing kept term with the same name and definition
            // whose `detail` is either equal or empty on one side.
            let matching_idx = by_key
                .get(&(name.clone(), def.clone(), detail.clone()))
                .copied()
                .or_else(|| {
                    by_name_def
                        .get(&(name.clone(), def.clone()))
                        .and_then(|idxs| {
                            idxs.iter().copied().find(|&idx| {
                                let existing_detail = s(&kept[idx].item, "detail").trim();
                                existing_detail.is_empty() || detail.is_empty()
                            })
                        })
                });

            if let Some(idx) = matching_idx {
                let target_doc_id = s(kept[idx].doc, "id").to_string();
                let target_item_id = s(&kept[idx].item, "id").to_string();
                left_out.push(LeftOut {
                    reference: format!("{}@{}#{}", me.0, version_of(c.doc), me.1),
                    item_type: "term".into(),
                    reason: Reason::Duplicate,
                    kept: Some(format!("{target_doc_id}#{target_item_id}")),
                });
                duplicate_of.insert(me, (target_doc_id, target_item_id));
                // Merge additive fields and prefer `accepted` over `proposed`.
                let before = kept[idx].item.clone();
                if let (Some(dst), Some(src)) = (kept[idx].item.as_object_mut(), c.item.as_object())
                {
                    if s(&Value::Object(dst.clone()), "status") == "proposed"
                        && s(&c.item, "status") == "accepted"
                    {
                        dst.insert("status".into(), Value::String("accepted".into()));
                    }
                    if s(&Value::Object(dst.clone()), "detail").trim().is_empty()
                        && !detail.is_empty()
                    {
                        if let Some(d) = src.get("detail") {
                            dst.insert("detail".into(), d.clone());
                        }
                    }
                    merge_string_list(dst, src, "aliases");
                    merge_string_list(dst, src, "code_names");
                }
                if kept[idx].item != before || c.item.get("evidence").is_some() {
                    glossary_term_changed = true;
                }
                kept[idx]
                    .extra_sources
                    .push((c.doc, s(&c.item, "id").to_string()));
                if c.item.get("evidence").is_some() {
                    merged_term_extras
                        .entry(idx)
                        .or_default()
                        .push((c.doc, c.item));
                }
                // Update detail index in case detail was filled in.
                let new_detail = s(&kept[idx].item, "detail").trim().to_string();
                by_key.insert((name, def, new_detail), idx);
            } else {
                let idx = kept.len();
                by_key.insert((name.clone(), def.clone(), detail), idx);
                by_name_def
                    .entry((name.clone(), def))
                    .or_default()
                    .push(idx);
                conflict_names.entry(name).or_default().push(idx);
                kept.push(c);
            }
        }
        *entries = kept;
    }

    // Track already-used artifact IDs so `new_id` can never collide with an
    // existing artifact or another draft.
    let mut used_artifacts: BTreeSet<String> = latest.keys().map(|k| (*k).to_string()).collect();

    let mut plans: Vec<Plan> = Vec::new();
    let mut unchanged = Vec::new();
    let mut dest: BTreeMap<(String, String), Dest> = BTreeMap::new();
    let all_kinds: BTreeSet<&str> = kinds
        .keys()
        .copied()
        .chain(members.keys().copied())
        .collect();
    let has_glossary_conflicts = conflict_names.values().any(|pos| pos.len() >= 2);
    for kind in all_kinds {
        let sources: Vec<&Value> = kinds.get(kind).cloned().unwrap_or_default();
        let carried = members.remove(kind).unwrap_or_default();
        if carried.is_empty() && sources.is_empty() {
            continue;
        }
        let is_unchanged = if sources.len() == 1 {
            let src = sources[0];
            let src_id = s(src, "id");
            let same_items = carried.len() == items(src).len()
                && carried
                    .iter()
                    .all(|c| s(c.doc, "id") == src_id && c.extra_sources.is_empty());
            same_items && !(kind == "glossary" && (glossary_term_changed || has_glossary_conflicts))
        } else {
            false
        };
        if is_unchanged {
            let src = sources[0];
            let src_id = s(src, "id").to_string();
            for c in &carried {
                let item_id = s(&c.item, "id").to_string();
                dest.insert(
                    (src_id.clone(), item_id.clone()),
                    Dest {
                        artifact: src_id.clone(),
                        id: item_id,
                    },
                );
            }
            unchanged.push(Unchanged {
                kind: kind.to_string(),
                artifact: src_id,
                version: version_of(src),
            });
            continue;
        }

        let continues = (sources.len() == 1).then(|| sources[0]);
        let artifact = match continues {
            Some(doc) => s(doc, "id").to_string(),
            None => {
                let mut candidate = String::new();
                for _ in 0..64 {
                    let id = new_id();
                    if !id.is_empty() && used_artifacts.insert(id.clone()) {
                        candidate = id;
                        break;
                    }
                }
                if candidate.is_empty() {
                    return Err(Problem::new(
                        "",
                        "could not get a unique artifact ID for the compacted draft",
                    ));
                }
                candidate
            }
        };
        // Counters of a continued artifact: the max of `id_counters` and all
        // item IDs across every version given in `docs`, so no ID is reused.
        let mut counters: BTreeMap<String, u64> = BTreeMap::new();
        if continues.is_some() {
            for d in docs.iter().filter(|d| s(d, "id") == artifact) {
                if let Some(Value::Object(m)) = d.get("id_counters") {
                    for (k, v) in m {
                        if let Some(n) = crate::whole_number(v) {
                            let e = counters.entry(k.clone()).or_insert(0);
                            *e = (*e).max(n);
                        }
                    }
                }
                for item in items(d) {
                    if let Some((t, n)) = crate::parse_item_id(s(item, "id")) {
                        let e = counters.entry(t.prefix().to_string()).or_insert(0);
                        *e = (*e).max(n);
                    }
                }
            }
        }
        let mut ids = Vec::new();
        for c in &carried {
            let old = s(&c.item, "id");
            let keep = continues.is_some_and(|cont| s(cont, "id") == s(c.doc, "id"));
            let id = if keep {
                old.to_string()
            } else {
                let p = prefix(old);
                let cnt = counters.entry(p.to_string()).or_insert(0);
                *cnt += 1;
                format!("{p}{cnt}")
            };
            dest.insert(
                (s(c.doc, "id").to_string(), old.to_string()),
                Dest {
                    artifact: artifact.clone(),
                    id: id.clone(),
                },
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
    for plan in &plans {
        let mut ptable = match plan.continues {
            Some(prev) => ParticipantTable::from_continued(prev),
            None => ParticipantTable::default(),
        };
        let mut out: Vec<Value> = Vec::new();
        for (pos, (carried, id)) in plan.carried.iter().zip(&plan.ids).enumerate() {
            let doc = carried.doc;
            let item = &carried.item;
            let src = s(doc, "id");
            let mut obj: Map<String, Value> = item.as_object().cloned().unwrap_or_default();
            obj.insert("id".into(), Value::String(id.clone()));

            // Remap participant IDs on items (both moved and folded).
            ptable.remap_item(doc, &mut obj);

            // Merge evidence from any deduplicated terms that collapsed here.
            if plan.kind == "glossary" {
                if let Some(extras) = merged_term_extras.get(&pos) {
                    for (ed, ei) in extras {
                        if let Some(eobj) = ei.as_object() {
                            merge_evidence(&mut obj, eobj, ed, &mut ptable);
                        }
                    }
                }
            }

            // Rewrite `critique.target` if it points to a carried item.
            if s(item, "type") == "critique" {
                if let Some(target_obj) = obj.get_mut("target").and_then(Value::as_object_mut) {
                    if let Some(t_item) = target_obj
                        .get("item")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                    {
                        let t_art = target_obj
                            .get("artifact")
                            .and_then(Value::as_str)
                            .unwrap_or(src);
                        if let Some(d) = dest.get(&(t_art.to_string(), t_item)) {
                            target_obj.insert("item".into(), Value::String(d.id.clone()));
                            target_obj.remove("version");
                            if d.artifact == plan.artifact {
                                target_obj.remove("artifact");
                            } else {
                                target_obj
                                    .insert("artifact".into(), Value::String(d.artifact.clone()));
                            }
                        }
                    }
                }
            }

            let mut refs: Vec<Value> = Vec::new();
            let moved = src != plan.artifact;
            if moved {
                refs.push(json!({"item": s(item, "id"), "artifact": src, "version": version_of(doc), "rel": "derived_from"}));
            }
            for (extra_doc, extra_id) in &carried.extra_sources {
                let extra_art = s(extra_doc, "id");
                if extra_art != plan.artifact || extra_id != id {
                    let r = json!({
                        "item": extra_id,
                        "artifact": extra_art,
                        "version": version_of(extra_doc),
                        "rel": "derived_from"
                    });
                    if !refs.contains(&r) {
                        refs.push(r);
                    }
                }
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
                if let Some(d) = dest.get(&target) {
                    // Never rewrite a ref into a self-reference (for example
                    // when a duplicate term collapses into the referencing term).
                    if d.artifact == plan.artifact && d.id == *id {
                        continue;
                    }
                    nr.insert("item".into(), Value::String(d.id.clone()));
                    nr.remove("version");
                    if d.artifact == plan.artifact {
                        nr.remove("artifact");
                    } else {
                        nr.insert("artifact".into(), Value::String(d.artifact.clone()));
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
                        let r = json!({"item": first, "rel": "contradicts"});
                        let arr = refs.as_array_mut().expect("refs");
                        if !arr.contains(&r) {
                            arr.push(r);
                        }
                    }
                }
                conflicts.push(TermConflict {
                    term: name.clone(),
                    items: positions.iter().map(|&p| plan.ids[p].clone()).collect(),
                });
            }
        }
        drafts.push(write_draft(plan, out, ptable.entries, options));
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
    carried: Vec<Carried<'a>>,
    ids: Vec<String>,
    counters: BTreeMap<String, u64>,
}

fn write_draft(
    plan: &Plan<'_>,
    items: Vec<Value>,
    participants: Vec<Value>,
    options: &Options<'_>,
) -> Draft {
    let mut doc = match plan.continues {
        Some(prev) => {
            let mut d = prev.clone();
            let o = d.as_object_mut().expect("a document is an object");
            o.insert("version".into(), Value::from(version_of(prev) + 1));
            o.remove("finalized_at");
            o.remove("method");
            o.remove("session");
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
            // The title of the source with the most carried items, unless
            // overridden in `options.titles`.
            let mut count: BTreeMap<&str, usize> = BTreeMap::new();
            for c in &plan.carried {
                *count.entry(s(c.doc, "id")).or_default() += 1;
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
            let title = match options
                .titles
                .get(&plan.kind)
                .map(String::as_str)
                .filter(|t| !t.is_empty())
                .or_else(|| main.map(|d| s(d, "title")).filter(|t| !t.is_empty()))
            {
                Some(t) => t.to_string(),
                None => {
                    let mut chars: Vec<char> = plan.kind.replace('_', " ").chars().collect();
                    if let Some(first) = chars.first_mut() {
                        *first = first.to_ascii_uppercase();
                    }
                    chars.into_iter().collect()
                }
            };
            let all_origin_docs = || {
                plan.sources
                    .iter()
                    .copied()
                    .chain(plan.carried.iter().map(|c| c.doc))
            };
            let created = all_origin_docs()
                .map(|d| s(d, "created_at"))
                .filter(|t| !t.is_empty())
                .max()
                .unwrap_or("");
            let schema_url = main
                .map(|d| s(d, "$schema"))
                .filter(|x| !x.is_empty())
                .or_else(|| {
                    all_origin_docs()
                        .map(|d| s(d, "$schema"))
                        .find(|x| !x.is_empty())
                })
                .unwrap_or(crate::SCHEMA_URL);
            json!({
                "$schema": schema_url,
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
    if !participants.is_empty() {
        o.insert("participants".into(), Value::Array(participants));
    }
    // An input parent for each source that is not the version before.
    let mut inputs: Vec<(&str, u64, &str, &str)> = Vec::new();
    for d in &plan.sources {
        if Some(s(d, "id")) != plan.continues.map(|c| s(c, "id")) {
            inputs.push((s(d, "id"), version_of(d), s(d, "kind"), s(d, "title")));
        }
    }
    for c in &plan.carried {
        let i = (
            s(c.doc, "id"),
            version_of(c.doc),
            s(c.doc, "kind"),
            s(c.doc, "title"),
        );
        if i.0 != plan.artifact && !inputs.contains(&i) {
            inputs.push(i);
        }
        for (ed, _) in &c.extra_sources {
            let ei = (s(ed, "id"), version_of(ed), s(ed, "kind"), s(ed, "title"));
            if ei.0 != plan.artifact && !inputs.contains(&ei) {
                inputs.push(ei);
            }
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
            let _ = writeln!(t, "unchanged: {} {}@{}", u.kind, u.artifact, u.version);
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
