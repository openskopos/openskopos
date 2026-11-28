use std::collections::BTreeMap;

use serde_json::{json, Value};
use skopos_core::compact::{compact, Compaction, Options, Reason, Unchanged};
use skopos_core::history::{history, ItemState};
use skopos_core::merge;

const SCHEMA: &str = "https://openskopos.org/schema/artifact/0.2.json";

fn doc(id: &str, version: u64, kind: &str, items: Value) -> Value {
    let mut d = json!({
        "$schema": SCHEMA, "id": id, "version": version, "kind": kind, "title": format!("{kind} {id}"),
        "status": "final", "created_at": "2026-11-01T00:00:00Z", "finalized_at": "2026-11-01T00:00:00Z",
        "items": items,
    });
    if version > 1 {
        d["parents"] = json!([{"rel": "previous_version", "artifact": id, "version": version - 1}]);
    }
    d
}

fn run_with(docs: &[Value], options: &Options<'_>) -> Compaction {
    let mut n = 0;
    let c = compact(docs, options, &mut || {
        n += 1;
        format!("pa_new000000000000{n:02}")
    })
    .unwrap();
    for d in &c.drafts {
        assert_eq!(
            skopos_core::check(&d.doc),
            vec![],
            "{} draft is valid",
            d.kind
        );
    }
    c
}

fn run(docs: &[Value]) -> Compaction {
    let options = Options {
        now: Some("2026-11-27T00:00:00Z".into()),
        ..Default::default()
    };
    run_with(docs, &options)
}

fn ids(doc: &Value) -> Vec<&str> {
    doc["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap())
        .collect()
}

fn reasons(c: &Compaction) -> Vec<(&str, Reason)> {
    c.left_out
        .iter()
        .map(|l| (l.reference.as_str(), l.reason))
        .collect()
}

#[test]
fn one_artifact_continues_with_its_ids() {
    let v1 = doc(
        "pa_a000000000000001",
        1,
        "spec",
        json!([
            {"id": "FR1", "type": "requirement", "text": "Count nightly.", "status": "accepted"},
            {"id": "FR2", "type": "requirement", "text": "Warn admins.", "status": "accepted"}
        ]),
    );
    let mut v2 = doc(
        "pa_a000000000000001",
        2,
        "spec",
        json!([
            {"id": "FR2", "type": "requirement", "text": "Warn admins.", "status": "accepted",
             "refs": [{"item": "D1", "rel": "depends_on"}]},
            {"id": "FR3", "type": "requirement", "text": "Count at the change.", "status": "accepted",
             "refs": [{"item": "FR1", "rel": "supersedes"}]},
            {"id": "D1", "type": "decision", "text": "Use email.", "status": "rejected"},
            {"id": "Q1", "type": "open_question", "text": "Which plans?", "status": "accepted", "answer": "All."},
            {"id": "Q2", "type": "open_question", "text": "Which regions?", "status": "proposed"}
        ]),
    );
    v2["id_counters"] = json!({"FR": 3, "D": 1, "Q": 2});
    v2["method"] = json!({"id": "elaboration-mob", "version": "1.0"});
    v2["session"] = json!({"id": "sess-1", "title": "Old session"});
    let intent = doc(
        "pa_b000000000000001",
        4,
        "intent",
        json!([
            {"id": "G1", "type": "goal", "text": "Fair bills.", "status": "accepted"}
        ]),
    );
    let c = run(&[v2, v1, intent]);

    assert_eq!(c.drafts.len(), 1);
    let d = &c.drafts[0].doc;
    assert!(!c.drafts[0].new_artifact);
    assert_eq!(
        (d["id"].as_str(), d["version"].as_u64()),
        (Some("pa_a000000000000001"), Some(3))
    );
    assert_eq!(
        d["parents"][0],
        json!({"rel": "previous_version", "artifact": "pa_a000000000000001", "version": 2})
    );
    assert_eq!(
        d["parents"][1],
        json!({"rel": "replaces", "artifact": "pa_a000000000000001", "version": 2})
    );
    assert_eq!(d["$schema"], skopos_core::DRAFT_0_3_SCHEMA_URL);
    assert_eq!(d.get("method"), None, "stale method is cleared");
    assert_eq!(d.get("session"), None, "stale session is cleared");
    assert_eq!(
        d["id_counters"],
        json!({"FR": 3, "D": 1, "Q": 2}),
        "no ID is used again"
    );
    assert_eq!(ids(d), ["FR2", "FR3", "Q2"]);
    assert_eq!(
        d["items"][0].get("refs"),
        None,
        "the ref to the rejected D1 is removed"
    );
    assert_eq!(
        d["items"][1]["refs"],
        json!([{"item": "FR1", "rel": "supersedes"}]),
        "supersedes stays"
    );
    assert_eq!(
        c.unchanged,
        [Unchanged {
            kind: "intent".into(),
            artifact: "pa_b000000000000001".into(),
            version: 4,
        }]
    );
    assert_eq!(
        reasons(&c),
        [
            ("pa_a000000000000001@2#D1", Reason::Rejected),
            ("pa_a000000000000001@2#Q1", Reason::Answered)
        ]
    );
}

#[test]
fn several_artifacts_fold_into_a_new_one_that_history_links() {
    let a = doc(
        "pa_a000000000000001",
        1,
        "spec",
        json!([
            {"id": "D1", "type": "decision", "text": "Count at the change.", "status": "accepted"},
            {"id": "D2", "type": "decision", "text": "Count nightly.", "status": "accepted"},
            {"id": "T1", "type": "term", "text": "Seat", "definition": "A user who can sign in.", "status": "accepted"}
        ]),
    );
    let mut b = doc(
        "pa_b000000000000001",
        1,
        "spec",
        json!([
            {"id": "D1", "type": "decision", "text": "Recount inside the change.", "status": "accepted",
             "refs": [{"item": "D2", "artifact": "pa_a000000000000001", "version": 1, "rel": "supersedes"}]},
            {"id": "FR1", "type": "requirement", "text": "Use the count of D1.", "status": "accepted",
             "refs": [{"item": "D1", "artifact": "pa_a000000000000001", "rel": "derived_from"},
                      {"item": "D1", "rel": "refines"}]},
            {"id": "T1", "type": "term", "text": "seat ", "definition": "A user who can sign in.", "status": "accepted"},
            {"id": "T2", "type": "term", "text": "Plan", "definition": "What the customer pays for.", "status": "accepted"}
        ]),
    );
    b["parents"] = json!([{"rel": "input", "artifact": "pa_a000000000000001", "version": 1}]);
    let glossary = doc(
        "pa_g000000000000001",
        2,
        "glossary",
        json!([
            {"id": "T4", "type": "term", "text": "Plan", "definition": "A price list.", "status": "accepted"}
        ]),
    );
    let docs = [a.clone(), b.clone(), glossary];
    let c = run(&docs);

    let spec = &c.drafts.iter().find(|d| d.kind == "spec").unwrap().doc;
    assert_eq!(spec["id"], "pa_new00000000000001");
    assert_eq!(ids(spec), ["D1", "D2", "FR1"]);
    assert_eq!(
        spec["items"][0]["refs"],
        json!([
            {"item": "D1", "artifact": "pa_a000000000000001", "version": 1, "rel": "derived_from"}
        ])
    );
    assert_eq!(
        spec["items"][1]["refs"],
        json!([
            {"item": "D1", "artifact": "pa_b000000000000001", "version": 1, "rel": "derived_from"},
            {"item": "D2", "artifact": "pa_a000000000000001", "version": 1, "rel": "supersedes"}
        ])
    );
    assert_eq!(
        spec["items"][2]["refs"],
        json!([
            {"item": "FR1", "artifact": "pa_b000000000000001", "version": 1, "rel": "derived_from"},
            {"item": "D1", "rel": "derived_from"},
            {"item": "D2", "rel": "refines"}
        ]),
        "refs follow the items to their new IDs"
    );
    let parent_rels: Vec<(&str, &str)> = spec["parents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["rel"].as_str().unwrap(), p["artifact"].as_str().unwrap()))
        .collect();
    assert_eq!(
        parent_rels,
        [
            ("replaces", "pa_a000000000000001"),
            ("replaces", "pa_b000000000000001")
        ]
    );

    let g = &c.drafts.iter().find(|d| d.kind == "glossary").unwrap();
    assert!(!g.new_artifact, "the one glossary continues");
    assert_eq!(ids(&g.doc), ["T4", "T5", "T6"]);
    let texts: Vec<(&str, &str)> = g.doc["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| (i["text"].as_str().unwrap(), i["status"].as_str().unwrap()))
        .collect();
    assert_eq!(
        texts,
        [
            ("Plan", "proposed"),
            ("Seat", "accepted"),
            ("Plan", "proposed")
        ]
    );
    assert_eq!(
        g.doc["items"][2]["refs"][1],
        json!({"item": "T4", "rel": "contradicts"})
    );
    assert_eq!(c.conflicts.len(), 1);
    assert_eq!(c.conflicts[0].items, ["T4", "T6"]);
    assert!(reasons(&c).contains(&("pa_a000000000000001@1#D2", Reason::Superseded)));
    let dup = c
        .left_out
        .iter()
        .find(|l| l.reason == Reason::Duplicate)
        .unwrap();
    assert_eq!(
        (dup.reference.as_str(), dup.kept.as_deref()),
        ("pa_b000000000000001@1#T1", Some("pa_a000000000000001#T1"))
    );

    // History over the sources and the drafts: each old item points to where
    // it lives now (including deduplicated terms), and superseded stays superseded.
    let mut all = docs.to_vec();
    all.extend(c.drafts.iter().map(|d| d.doc.clone()));
    let h = history(&all);
    let b_d1 = h.item("pa_b000000000000001#D1").unwrap();
    assert_eq!(b_d1.state, ItemState::Carried);
    assert_eq!(b_d1.derived_into, ["pa_new00000000000001@1#D2"]);
    let b_t1 = h.item("pa_b000000000000001#T1").unwrap();
    assert_eq!(b_t1.state, ItemState::Carried);
    assert_eq!(
        b_t1.derived_into,
        ["pa_g000000000000001@3#T5"],
        "deduplicated term keeps traceability via derived_from"
    );
    assert_eq!(
        h.item("pa_a000000000000001#D2").unwrap().state,
        ItemState::Superseded
    );
}

#[test]
fn a_draft_stops_the_compaction() {
    let mut d = doc("pa_a000000000000001", 1, "spec", json!([]));
    d["status"] = "draft".into();
    let e = compact(&[d], &Options::default(), &mut || unreachable!()).unwrap_err();
    assert!(e.message.contains("is a draft"), "{}", e.message);
}

#[test]
fn refs_into_unchanged_artifacts_are_preserved() {
    let intent = doc(
        "pa_i000000000000001",
        1,
        "intent",
        json!([{"id": "G1", "type": "goal", "text": "Fair bills.", "status": "accepted"}]),
    );
    let spec = doc(
        "pa_s000000000000001",
        1,
        "spec",
        json!([
            {"id": "FR1", "type": "requirement", "text": "Charge seats.", "status": "accepted",
             "refs": [{"item": "G1", "artifact": "pa_i000000000000001", "version": 1, "rel": "derived_from"}]},
            {"id": "D1", "type": "decision", "text": "Drop me.", "status": "rejected"}
        ]),
    );
    let c = run(&[intent, spec]);
    assert_eq!(c.unchanged.len(), 1);
    assert_eq!(c.unchanged[0].kind, "intent");
    let fr1 = &c.drafts[0].doc["items"][0];
    assert_eq!(
        fr1["refs"],
        json!([{"item": "G1", "artifact": "pa_i000000000000001", "rel": "derived_from"}]),
        "ref to unchanged intent goal must survive compaction"
    );
}

#[test]
fn open_critiques_and_blank_answers_stay_current() {
    let mut cr = doc(
        "pa_c000000000000001",
        1,
        "critique",
        json!([
            {"id": "CR1", "type": "critique", "text": "Missing auth check.", "status": "accepted",
             "severity": "blocker", "category": "gap",
             "evidence": [{"kind": "quote", "text": "No auth."}],
             "resolution": {"outcome": "open", "note": "Still looking."}},
            {"id": "CR2", "type": "critique", "text": "Old typo.", "status": "accepted",
             "severity": "minor", "category": "ambiguity",
             "evidence": [{"kind": "quote", "text": "Typo."}],
             "resolution": {"outcome": "fix_spec"}},
            {"id": "Q1", "type": "open_question", "text": "Blank answer?", "status": "accepted", "answer": "   "}
        ]),
    );
    cr["id_counters"] = json!({"CR": 2, "Q": 1});
    let c = run(&[cr]);
    assert_eq!(ids(&c.drafts[0].doc), ["CR1", "Q1"]);
    assert_eq!(
        reasons(&c),
        [("pa_c000000000000001@1#CR2", Reason::Resolved)]
    );
}

#[test]
fn glossary_keeps_its_ids_and_non_terms_even_with_agent_rules() {
    // `agent_rules` sorts alphabetically before `glossary`; the existing
    // glossary must still win on duplicates and be the main item in conflicts
    // so `merge::conflicts` and `merge::resolve` detect them.
    let mut ar = doc(
        "pa_a000000000000001",
        1,
        "agent_rules",
        json!([
            {"id": "T1", "type": "term", "text": "Seat", "definition": "A user.", "status": "accepted"},
            {"id": "T2", "type": "term", "text": "Plan", "definition": "From agent_rules.", "status": "accepted"}
        ]),
    );
    ar["id_counters"] = json!({"T": 2});
    let mut gl = doc(
        "pa_g000000000000001",
        1,
        "glossary",
        json!([
            {"id": "D1", "type": "decision", "text": "Seat", "status": "accepted"},
            {"id": "D2", "type": "decision", "text": "Seat", "status": "accepted"},
            {"id": "T1", "type": "term", "text": "Seat", "definition": "A user.", "status": "accepted"},
            {"id": "T2", "type": "term", "text": "Plan", "definition": "From glossary.", "status": "accepted"}
        ]),
    );
    gl["id_counters"] = json!({"D": 2, "T": 2});

    let c = run(&[ar, gl]);
    let g = c.drafts.iter().find(|d| d.kind == "glossary").unwrap();
    // Non-term decisions in glossary are untouched (not deduplicated or conflicted with terms).
    assert_eq!(ids(&g.doc), ["D1", "D2", "T1", "T2", "T3"]);
    assert_eq!(g.doc["items"][0]["status"], "accepted");
    assert_eq!(g.doc["items"][1]["status"], "accepted");
    // Glossary's T1 kept its ID (not replaced by agent_rules's T1).
    assert_eq!(g.doc["items"][2]["id"], "T1");
    assert_eq!(g.doc["items"][2]["status"], "accepted");
    // Conflict is detected by `merge::conflicts` and resolvable by `merge::resolve`.
    let open = merge::conflicts(&g.doc);
    assert_eq!(open.len(), 1);
    assert_eq!(
        (open[0].main.as_str(), open[0].other.as_str()),
        ("T2", "T3")
    );
}

#[test]
fn proposed_item_does_not_drop_the_accepted_item_it_supersedes() {
    let mut s = doc(
        "pa_p000000000000001",
        1,
        "spec",
        json!([
            {"id": "D1", "type": "decision", "text": "Current rule.", "status": "accepted"},
            {"id": "D2", "type": "decision", "text": "Proposed replacement.", "status": "proposed",
             "refs": [{"item": "D1", "rel": "supersedes"}]}
        ]),
    );
    s["id_counters"] = json!({"D": 2});
    let c = run(&[s]);
    assert!(c.left_out.is_empty(), "D1 must stay until D2 is accepted");
    assert!(c.drafts.is_empty());
    assert_eq!(c.unchanged.len(), 1);
}

#[test]
fn duplicate_terms_merge_additive_fields_and_conflict_on_differing_detail() {
    let a = doc(
        "pa_a000000000000001",
        1,
        "spec",
        json!([
            {"id": "T1", "type": "term", "text": "Seat", "definition": "A user.", "status": "proposed",
             "aliases": ["User"], "code_names": ["seat_id"]},
            {"id": "T2", "type": "term", "text": "Tier", "definition": "A plan level.", "status": "accepted",
             "detail": "Monthly billing only."}
        ]),
    );
    let b = doc(
        "pa_b000000000000001",
        1,
        "spec",
        json!([
            {"id": "T1", "type": "term", "text": "seat", "definition": "A user.", "status": "accepted",
             "aliases": ["License", "User"], "code_names": ["account_seat"],
             "evidence": [{"kind": "quote", "text": "One seat per user."}]},
            {"id": "T2", "type": "term", "text": "Tier", "definition": "A plan level.", "status": "accepted",
             "detail": "Annual billing only."}
        ]),
    );
    let c = run(&[a, b]);
    let g = c.drafts.iter().find(|d| d.kind == "glossary").unwrap();
    let seat = &g.doc["items"][0];
    assert_eq!(seat["status"], "accepted", "accepted wins over proposed");
    assert_eq!(seat["aliases"], json!(["User", "License"]));
    assert_eq!(seat["code_names"], json!(["seat_id", "account_seat"]));
    assert_eq!(seat["evidence"].as_array().unwrap().len(), 1);
    assert_eq!(
        seat["refs"].as_array().unwrap().len(),
        2,
        "derived_from links to both source terms"
    );
    assert_eq!(c.conflicts.len(), 1);
    assert_eq!(c.conflicts[0].term, "tier");
}

#[test]
fn no_op_glossary_when_duplicate_term_adds_nothing_new() {
    let mut gl = doc(
        "pa_g000000000000001",
        1,
        "glossary",
        json!([
            {"id": "T1", "type": "term", "text": "Seat", "definition": "A user.", "status": "accepted"}
        ]),
    );
    gl["id_counters"] = json!({"T": 1});
    let mut sp = doc(
        "pa_s000000000000001",
        1,
        "spec",
        json!([
            {"id": "T1", "type": "term", "text": "Seat", "definition": "A user.", "status": "accepted"},
            {"id": "FR1", "type": "requirement", "text": "Use Seat.", "status": "accepted",
             "refs": [{"item": "T1", "rel": "relates_to"}]}
        ]),
    );
    sp["id_counters"] = json!({"FR": 1, "T": 1});
    let c = run(&[gl, sp]);
    // Spec changes (term removed, ref points to glossary); glossary gets a
    // derived_from ref to the duplicate term so the old ref stays traceable.
    let spec = c.drafts.iter().find(|d| d.kind == "spec").unwrap();
    assert_eq!(
        spec.doc["items"][0]["refs"],
        json!([{"item": "T1", "artifact": "pa_g000000000000001", "rel": "relates_to"}])
    );
    let g = c.drafts.iter().find(|d| d.kind == "glossary").unwrap();
    assert_eq!(
        g.doc["items"][0]["refs"],
        json!([{"item": "T1", "artifact": "pa_s000000000000001", "version": 1, "rel": "derived_from"}])
    );
}

#[test]
fn participants_are_merged_and_remapped_without_collisions() {
    let mut a = doc(
        "pa_a000000000000001",
        1,
        "spec",
        json!([
            {"id": "D1", "type": "decision", "text": "Choice A.", "status": "accepted",
             "proposed_by": "P1", "decided_by": "P1",
             "evidence": [{"kind": "discussion", "text": "Alice approved.", "speaker": "P1"}]}
        ]),
    );
    a["participants"] = json!([{"id": "P1", "name": "Alice", "kind": "human"}]);
    a["id_counters"] = json!({"D": 1});

    let mut b = doc(
        "pa_b000000000000001",
        1,
        "spec",
        json!([
            {"id": "D1", "type": "decision", "text": "Choice B.", "status": "accepted",
             "proposed_by": "P1", "decided_by": "P2",
             "evidence": [{"kind": "discussion", "text": "Bob approved.", "speaker": "P1"}]}
        ]),
    );
    b["participants"] = json!([
        {"id": "P1", "name": "Bob", "kind": "human"},
        {"id": "P2", "name": "Alice", "kind": "human"}
    ]);
    b["id_counters"] = json!({"D": 1});

    let c = run(&[a, b]);
    let spec = &c.drafts[0].doc;
    let participants = spec["participants"].as_array().unwrap();
    assert_eq!(
        participants.len(),
        2,
        "Alice deduplicated, Bob gets P2: {participants:?}"
    );
    assert_eq!(
        participants[0],
        json!({"id": "P1", "name": "Alice", "kind": "human"})
    );
    assert_eq!(
        participants[1],
        json!({"id": "P2", "name": "Bob", "kind": "human"})
    );
    // Choice B: Bob was P1 -> P2, Alice was P2 -> P1.
    assert_eq!(spec["items"][1]["proposed_by"], "P2");
    assert_eq!(spec["items"][1]["decided_by"], "P1");
    assert_eq!(spec["items"][1]["evidence"][0]["speaker"], "P2");
}

#[test]
fn synthesized_glossary_without_now_is_valid_and_supports_custom_titles() {
    let sp = doc(
        "pa_s000000000000001",
        1,
        "spec",
        json!([
            {"id": "T1", "type": "term", "text": "Seat", "definition": "A user.", "status": "accepted"}
        ]),
    );
    let mut titles = BTreeMap::new();
    titles.insert("glossary".into(), "Billing glossary".into());
    let options = Options {
        now: None,
        context: &[],
        titles,
    };
    let c = run_with(&[sp], &options);
    let g = c.drafts.iter().find(|d| d.kind == "glossary").unwrap();
    assert_eq!(g.doc["created_at"], "2026-11-01T00:00:00Z");
    assert_eq!(g.doc["title"], "Billing glossary");

    // Also verify default fallback title when options.titles is not set.
    let sp2 = doc(
        "pa_s000000000000002",
        1,
        "spec",
        json!([
            {"id": "T1", "type": "term", "text": "Seat", "definition": "A user.", "status": "accepted"}
        ]),
    );
    let c2 = run_with(&[sp2], &Options::default());
    let g2 = c2.drafts.iter().find(|d| d.kind == "glossary").unwrap();
    assert_eq!(g2.doc["title"], "Glossary");
    assert_eq!(g2.doc["created_at"], "2026-11-01T00:00:00Z");
}

#[test]
fn context_docs_detect_cross_area_supersession() {
    let area_a = doc(
        "pa_a000000000000001",
        1,
        "spec",
        json!([
            {"id": "D1", "type": "decision", "text": "Old cross-area rule.", "status": "accepted"},
            {"id": "D2", "type": "decision", "text": "Still current.", "status": "accepted"}
        ]),
    );
    let area_b = doc(
        "pa_b000000000000001",
        1,
        "spec",
        json!([
            {"id": "D1", "type": "decision", "text": "Replacement in area B.", "status": "accepted",
             "refs": [{"item": "D1", "artifact": "pa_a000000000000001", "version": 1, "rel": "supersedes"}]}
        ]),
    );
    let context = [area_b];
    let options = Options {
        now: Some("2026-11-27T00:00:00Z".into()),
        context: &context,
        titles: BTreeMap::new(),
    };
    let c = run_with(&[area_a], &options);
    assert_eq!(c.drafts.len(), 1);
    assert!(
        !c.drafts[0].new_artifact,
        "area_b is context only, not compacted"
    );
    assert_eq!(ids(&c.drafts[0].doc), ["D2"]);
    assert_eq!(
        reasons(&c),
        [("pa_a000000000000001@1#D1", Reason::Superseded)]
    );
}

#[test]
fn earlier_id_counters_are_respected_and_self_refs_are_dropped() {
    let mut v1 = doc(
        "pa_g000000000000001",
        1,
        "glossary",
        json!([
            {"id": "T1", "type": "term", "text": "Seat", "definition": "A user.", "status": "accepted"}
        ]),
    );
    v1["id_counters"] = json!({"T": 10});
    let mut v2 = doc(
        "pa_g000000000000001",
        2,
        "glossary",
        json!([
            {"id": "T1", "type": "term", "text": "Seat", "definition": "A user.", "status": "accepted",
             "refs": [{"item": "T1", "artifact": "pa_s000000000000001", "rel": "relates_to"}]}
        ]),
    );
    v2["id_counters"] = json!({"T": 1});
    let sp = doc(
        "pa_s000000000000001",
        1,
        "spec",
        json!([
            {"id": "T1", "type": "term", "text": "seat", "definition": "A user.", "status": "accepted"},
            {"id": "T2", "type": "term", "text": "Plan", "definition": "A tier.", "status": "accepted"}
        ]),
    );
    let c = run(&[v1, v2, sp]);
    let g = c.drafts.iter().find(|d| d.kind == "glossary").unwrap();
    assert_eq!(ids(&g.doc), ["T1", "T11"]);
    assert_eq!(g.doc["id_counters"]["T"], 11);
    // T1's ref to duplicate pa_s#T1 would become a self-ref to T1 and must be omitted.
    let t1_refs = g.doc["items"][0]["refs"].as_array().unwrap();
    assert!(
        !t1_refs
            .iter()
            .any(|r| r.get("rel").and_then(Value::as_str) == Some("relates_to")),
        "self-ref must not be emitted: {t1_refs:?}"
    );
}

#[test]
fn critique_target_is_rewritten_when_targeted_artifact_folds() {
    let a = doc(
        "pa_a000000000000001",
        1,
        "spec",
        json!([{"id": "D1", "type": "decision", "text": "One.", "status": "accepted"}]),
    );
    let b = doc(
        "pa_b000000000000001",
        1,
        "spec",
        json!([{"id": "D1", "type": "decision", "text": "Two.", "status": "accepted"}]),
    );
    let mut cr = doc(
        "pa_c000000000000001",
        1,
        "critique",
        json!([
            {"id": "CR1", "type": "critique", "text": "Open issue.", "status": "accepted",
             "severity": "major", "category": "gap",
             "evidence": [{"kind": "quote", "text": "Two."}],
             "target": {"artifact": "pa_b000000000000001", "version": 1, "item": "D1"}},
            {"id": "CR2", "type": "critique", "text": "Done.", "status": "accepted",
             "severity": "minor", "category": "other",
             "evidence": [{"kind": "quote", "text": "Done."}],
             "resolution": {"outcome": "fix_spec"}}
        ]),
    );
    cr["id_counters"] = json!({"CR": 2});
    let c = run(&[a, b, cr]);
    let spec_id = c.drafts.iter().find(|d| d.kind == "spec").unwrap().doc["id"]
        .as_str()
        .unwrap();
    let crit = c.drafts.iter().find(|d| d.kind == "critique").unwrap();
    assert_eq!(
        crit.doc["items"][0]["target"],
        json!({"artifact": spec_id, "item": "D2"})
    );
}

#[test]
fn continued_compaction_preserves_states_and_repeated_compaction_propagates_supersession() {
    // 1. Continued artifact: rejected, settled, and moved-term items do not
    // become `dropped` in history.
    let s1 = doc(
        "pa_s000000000000001",
        1,
        "spec",
        json!([
            {"id": "D1", "type": "decision", "text": "Bad idea.", "status": "rejected"},
            {"id": "D2", "type": "decision", "text": "Good idea.", "status": "accepted"},
            {"id": "Q1", "type": "open_question", "text": "Which?", "status": "accepted", "answer": "This."},
            {"id": "A1", "type": "assumption", "text": "Sky is green.", "status": "accepted", "confirmation": "refuted"},
            {"id": "T1", "type": "term", "text": "Seat", "definition": "A user.", "status": "accepted"}
        ]),
    );
    let c1 = run(std::slice::from_ref(&s1));
    let mut all1 = vec![s1];
    all1.extend(c1.drafts.iter().map(|d| d.doc.clone()));
    let h1 = history(&all1);
    assert_eq!(
        h1.item("pa_s000000000000001#D1").unwrap().state,
        ItemState::Rejected
    );
    assert_eq!(
        h1.item("pa_s000000000000001#D2").unwrap().state,
        ItemState::InForce
    );
    assert_eq!(
        h1.item("pa_s000000000000001#Q1").unwrap().state,
        ItemState::InForce
    );
    assert_eq!(
        h1.item("pa_s000000000000001#A1").unwrap().state,
        ItemState::InForce
    );
    assert_eq!(
        h1.item("pa_s000000000000001#T1").unwrap().state,
        ItemState::Carried
    );

    // 2. Repeated compaction: A+B -> C (replaces A, B), then D supersedes
    // C#D1, then C+D -> E (replaces C, D). History over all five must show
    // A#D1 as Superseded, A#D2 and C#D2 as Carried, and E does not accumulate
    // duplicate derived_from refs back to A.
    let a = doc(
        "pa_a000000000000020",
        1,
        "spec",
        json!([
            {"id": "D1", "type": "decision", "text": "Decision 1.", "status": "accepted"},
            {"id": "D2", "type": "decision", "text": "Decision 2.", "status": "accepted"}
        ]),
    );
    let b = doc(
        "pa_b000000000000020",
        1,
        "spec",
        json!([
            {"id": "D1", "type": "decision", "text": "Decision 3.", "status": "accepted"}
        ]),
    );
    let comp1 = run(&[a.clone(), b.clone()]);
    let mut c = comp1.drafts[0].doc.clone();
    c["status"] = json!("final");
    c["finalized_at"] = json!("2026-11-27T01:00:00Z");
    let c_id = c["id"].as_str().unwrap().to_string();

    let d = doc(
        "pa_d000000000000020",
        1,
        "spec",
        json!([
            {"id": "D1", "type": "decision", "text": "Decision 1 replaced.", "status": "accepted",
             "refs": [{"item": "D1", "artifact": c_id, "version": 1, "rel": "supersedes"}]}
        ]),
    );
    let mut n = 10;
    let comp2 = compact(
        &[c.clone(), d.clone()],
        &Options {
            now: Some("2026-11-27T02:00:00Z".into()),
            ..Default::default()
        },
        &mut || {
            n += 1;
            format!("pa_new000000000000{n:02}")
        },
    )
    .unwrap();
    let e = comp2.drafts[0].doc.clone();
    assert_eq!(skopos_core::check(&e), vec![]);

    // Carried item in E (originally A#D2 -> C#D2 -> E#D2) links only to C@1#D2,
    // not also to A@1#D2.
    let e_d2_derived: Vec<&str> = e["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["text"] == "Decision 2.")
        .unwrap()["refs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["rel"] == "derived_from")
        .map(|r| r["artifact"].as_str().unwrap())
        .collect();
    assert_eq!(e_d2_derived, [c_id.as_str()]);

    let h = history(&[a, b, c, d, e]);
    let a_d1 = h.item("pa_a000000000000020#D1").unwrap();
    assert_eq!(a_d1.state, ItemState::Superseded);
    assert!(
        a_d1.superseded_by
            .iter()
            .any(|s| s == "pa_d000000000000020@1#D1"),
        "A#D1 must inherit superseder from C#D1: {:?}",
        a_d1.superseded_by
    );
    assert_eq!(
        h.item("pa_a000000000000020#D2").unwrap().state,
        ItemState::Carried
    );
    assert_eq!(
        h.item(&format!("{c_id}#D1")).unwrap().state,
        ItemState::Superseded
    );
    assert_eq!(
        h.item(&format!("{c_id}#D2")).unwrap().state,
        ItemState::Carried
    );
}
