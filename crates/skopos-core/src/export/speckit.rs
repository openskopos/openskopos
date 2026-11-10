//! Spec Kit: one feature folder, `specs/<NNN>-<name>/`.
//!
//! Checked against Spec Kit 1.3.3 and its `spec-template.md` and
//! `plan-template.md`.

use super::{
    constraint, decision, example_label, goal, not_exported_note, term, Export, File, Item, Source,
    Tool,
};
use crate::ids::parse_item_id;
use crate::model::ItemType;

pub(super) fn export(source: &Source, name: &str, number: u32) -> Export {
    let feature = format!("{number:03}-{name}");
    let folder = format!("specs/{feature}");
    let not_exported = source.skipped(Tool::SpecKit, &[ItemType::Risk, ItemType::Critique]);
    let created = source.str("created_at");
    let date = created.get(..10).unwrap_or(created);

    let mut files = vec![File {
        path: format!("{folder}/spec.md"),
        text: spec(source, &feature, date, &not_exported),
    }];
    if let Some(text) = plan(source, &feature, date) {
        files.push(File {
            path: format!("{folder}/plan.md"),
            text,
        });
    }
    let decisions = source.of(ItemType::Decision);
    if !decisions.is_empty() {
        let mut text = format!("# Research: {}\n", source.title());
        for d in &decisions {
            text.push('\n');
            text.push_str(&decision(source, d, "##"));
        }
        files.push(File {
            path: format!("{folder}/research.md"),
            text,
        });
    }
    Export {
        tool: Tool::SpecKit,
        folder,
        files,
        not_exported,
    }
}

/// `FR-004` for `FR4`: the Spec Kit number keeps the OpenSkopos number.
fn numbered(prefix: &str, id: &str) -> String {
    match parse_item_id(id) {
        Some((_, n)) => format!("{prefix}-{n:03}"),
        None => id.to_string(),
    }
}

fn priority(source: &Source, story: &Item) -> (String, Option<String>) {
    let refined: Vec<Item> = story
        .links("refines", source.artifact)
        .into_iter()
        .filter_map(|id| source.get(id))
        .filter(|i| i.item_type == Some(ItemType::Requirement))
        .collect();
    let best = refined
        .iter()
        .filter_map(|r| match r.str("priority") {
            "must" => Some((1, r)),
            "should" => Some((2, r)),
            "could" => Some((3, r)),
            _ => None,
        })
        .min_by_key(|(p, _)| *p);
    match best {
        Some((p, r)) => (
            format!("P{p}"),
            Some(format!(
                "it refines {}, a \"{}\" requirement.",
                r.id,
                r.str("priority")
            )),
        ),
        None => ("P2".to_string(), None),
    }
}

fn spec(source: &Source, feature: &str, date: &str, not_exported: &[super::NotExported]) -> String {
    let mut out = format!("# Feature Specification: {}\n\n", source.title());
    out.push_str(&format!("**Feature Branch**: `{feature}`  \n"));
    out.push_str(&format!("**Created**: {date}  \n"));
    out.push_str("**Status**: Draft  \n");
    out.push_str(&format!(
        "**Input**: OpenSkopos document `{}`. Each line ends with the reference string of its item.\n",
        source.label()
    ));
    if let Some(summary) = source.summary() {
        out.push_str(&format!("\n{summary}\n"));
    }

    let questions = source.of(ItemType::OpenQuestion);
    let answered: Vec<&Item> = questions
        .iter()
        .filter(|q| q.field("answer").is_some())
        .collect();
    if !answered.is_empty() {
        out.push_str(&format!("\n## Clarifications\n\n### Session {date}\n\n"));
        for q in answered {
            out.push_str(&format!(
                "- Q: {} → A: {} {}\n",
                q.text(),
                q.field("answer").unwrap_or_default(),
                source.tag(q.id)
            ));
        }
    }

    out.push_str("\n## User Scenarios & Testing *(mandatory)*\n");
    let mut edges: Vec<String> = Vec::new();
    for (n, st) in source.of(ItemType::Story).iter().enumerate() {
        let (p, why) = priority(source, st);
        out.push_str(&format!(
            "\n### User Story {} - {} (Priority: {p})\n\n",
            n + 1,
            st.short()
        ));
        out.push_str(&format!("{} {}\n", st.text(), source.tag(st.id)));
        if let Some(d) = st.detail() {
            out.push_str(&format!("\n{d}\n"));
        }
        if let Some(why) = why {
            out.push_str(&format!("\n**Why this priority**: {why}\n"));
        }
        let mut scenarios: Vec<String> = st
            .list("acceptance_criteria")
            .into_iter()
            .map(|ac| format!("{ac} {}", source.tag(st.id)))
            .collect();
        for (kind, text) in st.examples() {
            let line = format!("{}: {text} {}", example_label(kind), source.tag(st.id));
            if kind == "edge" {
                edges.push(line);
            } else {
                scenarios.push(line);
            }
        }
        if !scenarios.is_empty() {
            out.push_str("\n**Acceptance Scenarios**:\n\n");
            for (i, s) in scenarios.iter().enumerate() {
                out.push_str(&format!("{}. {s}\n", i + 1));
            }
        }
    }

    let requirements = source.of(ItemType::Requirement);
    let mut fr_lines = String::new();
    for r in &requirements {
        fr_lines.push_str(&format!(
            "- **{}**: {} {}\n",
            numbered("FR", r.id),
            r.text(),
            source.tag(r.id)
        ));
        if let Some(p) = r.field("priority") {
            fr_lines.push_str(&format!("  - Priority: {p}\n"));
        }
        for (kind, text) in r.examples() {
            if kind == "edge" {
                edges.push(format!("{text} {}", source.tag(r.id)));
            } else {
                fr_lines.push_str(&format!("  - {}: {text}\n", example_label(kind)));
            }
        }
        if let Some(d) = r.detail() {
            fr_lines.push_str(&format!("  - {}\n", super::one_line(d)));
        }
    }
    if !edges.is_empty() {
        out.push_str("\n### Edge Cases\n\n");
        for e in &edges {
            out.push_str(&format!("- {e}\n"));
        }
    }

    out.push_str("\n## Requirements *(mandatory)*\n\n### Functional Requirements\n\n");
    out.push_str(&fr_lines);
    for q in questions.iter().filter(|q| q.field("answer").is_none()) {
        out.push_str(&format!(
            "- [NEEDS CLARIFICATION: {}] {}\n",
            q.text(),
            source.tag(q.id)
        ));
        let options = q.list("options");
        if !options.is_empty() {
            out.push_str(&format!("  - Options: {}\n", options.join("; ")));
        }
    }

    let terms = source.of(ItemType::Term);
    if !terms.is_empty() {
        out.push_str("\n### Key Entities\n\n");
        for t in &terms {
            out.push_str(&term(source, t));
        }
    }

    let signals = source.of(ItemType::SuccessSignal);
    if !signals.is_empty() {
        out.push_str("\n## Success Criteria *(mandatory)*\n\n### Measurable Outcomes\n\n");
        for s in &signals {
            out.push_str(&format!(
                "- **{}**: {} {}\n",
                numbered("SC", s.id),
                s.text(),
                source.tag(s.id)
            ));
            if let Some(v) = s.field("verification") {
                out.push_str(&format!("  - Verification: {v}\n"));
            }
        }
    }

    let assumptions = source.of(ItemType::Assumption);
    let non_goals = source.of(ItemType::NonGoal);
    if !assumptions.is_empty() || !non_goals.is_empty() {
        out.push_str("\n## Assumptions\n\n");
        for a in &assumptions {
            let confirmation = a
                .field("confirmation")
                .unwrap_or_else(|| "unconfirmed".to_string());
            out.push_str(&format!(
                "- {} ({confirmation}) {}\n",
                a.text(),
                source.tag(a.id)
            ));
        }
        for n in &non_goals {
            let mut line = format!("- Out of scope: {}", n.text());
            if let Some(r) = n.field("reason") {
                line.push_str(&format!(" Reason: {r}"));
            }
            if n.bool("deferred") {
                line.push_str(" (deferred: later, not never)");
            }
            out.push_str(&format!("{line} {}\n", source.tag(n.id)));
        }
    }
    out.push_str(&not_exported_note(source, not_exported, "##"));
    out
}

fn plan(source: &Source, feature: &str, date: &str) -> Option<String> {
    let goals = source.of(ItemType::Goal);
    let constraints = source.of(ItemType::Constraint);
    if goals.is_empty() && constraints.is_empty() {
        return None;
    }
    let mut out = format!("# Implementation Plan: {}\n\n", source.title());
    out.push_str(&format!(
        "**Branch**: `{feature}` | **Date**: {date} | **Spec**: [spec.md](spec.md)\n"
    ));
    if !goals.is_empty() {
        out.push_str("\n## Summary\n\n");
        for g in &goals {
            out.push_str(&goal(source, g));
        }
    }
    if !constraints.is_empty() {
        out.push_str("\n## Technical Context\n\n**Constraints**:\n\n");
        for c in &constraints {
            out.push_str(&constraint(source, c));
        }
    }
    Some(out)
}
