use serde_json::{json, Value};
use skopos_core::compact::{compact, Compaction, Options, Reason};
use skopos_core::history::{history, ItemState};

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

fn run(docs: &[Value]) -> Compaction {
    let mut n = 0;
    let options = Options {
        now: Some("2026-11-27T00:00:00Z".into()),
    };
    let c = compact(docs, &options, &mut || {
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
    assert_eq!(c.unchanged, ["pa_b000000000000001@4"]);
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
    let inputs: Vec<&str> = spec["parents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["artifact"].as_str().unwrap())
        .collect();
    assert_eq!(inputs, ["pa_a000000000000001", "pa_b000000000000001"]);

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
    // it lives now, and the superseded item stays superseded.
    let mut all = docs.to_vec();
    all.extend(c.drafts.iter().map(|d| d.doc.clone()));
    let h = history(&all);
    let b_d1 = h.item("pa_b000000000000001#D1").unwrap();
    assert_eq!(b_d1.derived_into, ["pa_new00000000000001@1#D2"]);
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
fn refs_into_unchanged_artifacts_stay_and_open_critiques_stay() {
    let intent = doc(
        "pa_i000000000000001",
        1,
        "intent",
        json!([
            {"id": "G1", "type": "goal", "text": "Fair bills.", "status": "accepted"}
        ]),
    );
    let spec = doc(
        "pa_a000000000000001",
        1,
        "spec",
        json!([
            {"id": "FR1", "type": "requirement", "text": "Count seats.", "status": "accepted",
             "refs": [{"item": "G1", "artifact": "pa_i000000000000001", "version": 1, "rel": "derived_from"}]},
            {"id": "FR2", "type": "requirement", "text": "Old.", "status": "rejected"},
            {"id": "CR1", "type": "critique", "text": "Counts can lag.", "status": "proposed",
             "severity": "major", "category": "gap", "evidence": [{"kind": "quote", "text": "lag"}], "resolution": {"outcome": "open"}},
            {"id": "Q1", "type": "open_question", "text": "Which plans?", "status": "accepted", "answer": " "}
        ]),
    );
    let c = run(&[intent, spec]);
    let d = &c.drafts[0].doc;
    assert_eq!(
        ids(d),
        ["FR1", "CR1", "Q1"],
        "an open critique and a blank answer are not settled"
    );
    assert_eq!(
        d["items"][0]["refs"],
        json!([
            {"item": "G1", "artifact": "pa_i000000000000001", "version": 1, "rel": "derived_from"}
        ]),
        "a ref into the unchanged intent stays"
    );
    assert_eq!(c.unchanged, ["pa_i000000000000001@1"]);
}

#[test]
fn the_glossary_keeps_its_ids_and_only_terms_are_paired() {
    let rules = doc(
        "pa_a000000000000001",
        1,
        "agent_rules",
        json!([
            {"id": "T1", "type": "term", "text": "Seat", "definition": "A user who can sign in.", "status": "accepted"},
            {"id": "T2", "type": "term", "text": "Plan", "definition": "A price list.", "status": "accepted"}
        ]),
    );
    let mut glossary = doc(
        "pa_g000000000000001",
        1,
        "glossary",
        json!([
            {"id": "T1", "type": "term", "text": "Seat", "definition": "A user who can sign in.", "status": "accepted"},
            {"id": "T2", "type": "term", "text": "Plan", "definition": "What the customer pays for.", "status": "accepted"},
            {"id": "D1", "type": "decision", "text": "Seat", "status": "accepted"}
        ]),
    );
    glossary["id_counters"] = json!({"T": 2, "D": 1});
    let c = run(&[rules, glossary]);
    let g = &c.drafts.iter().find(|d| d.kind == "glossary").unwrap().doc;
    assert_eq!(g["id"], "pa_g000000000000001");
    assert_eq!(
        ids(g),
        ["T1", "T2", "D1", "T3"],
        "the glossary's own terms keep their IDs"
    );
    assert_eq!(
        g["items"][2]["status"], "accepted",
        "a decision is never a term conflict"
    );
    assert_eq!(
        g["items"][3]["refs"][1],
        json!({"item": "T2", "rel": "contradicts"})
    );
    // merge::conflicts reads the pair, so skopos conflicts and resolve work on it.
    let open = skopos_core::merge::conflicts(g);
    assert_eq!(open.len(), 1, "{open:?}");
    let dup = c
        .left_out
        .iter()
        .find(|l| l.reason == Reason::Duplicate)
        .unwrap();
    assert_eq!(dup.reference, "pa_a000000000000001@1#T1");
}

#[test]
fn a_new_glossary_without_now_is_valid() {
    let spec = doc(
        "pa_a000000000000001",
        1,
        "spec",
        json!([
            {"id": "T1", "type": "term", "text": "Seat", "definition": "A user.", "status": "accepted"}
        ]),
    );
    let mut n = 0;
    let c = compact(&[spec], &Options::default(), &mut || {
        n += 1;
        format!("pa_new000000000000{n:02}")
    })
    .unwrap();
    for d in &c.drafts {
        assert_eq!(skopos_core::check(&d.doc), vec![], "{} draft", d.kind);
    }
    let g = c.drafts.iter().find(|d| d.kind == "glossary").unwrap();
    assert_eq!(g.doc["title"], "Glossary");
    assert_eq!(g.doc["created_at"], "2026-11-01T00:00:00Z");
}
