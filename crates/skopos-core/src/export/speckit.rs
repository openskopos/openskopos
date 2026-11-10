//! Spec Kit: one feature folder, `specs/<NNN>-<name>/`.
//!
//! Checked against Spec Kit 1.3.3: its `spec-template.md`,
//! `plan-template.md` and scripts.

use std::fmt::{self, Write as _};

use super::{
    constraint, decision, example_label, goal, indent, not_exported_note, render, term, Export,
    File, Item, NotExported, Source, Tool,
};
use crate::ids::parse_item_id;
use crate::model::ItemType;

pub(super) fn export(source: &Source, name: &str, number: u32) -> Export {
    let feature = format!("{number:03}-{name}");
    let folder = format!("specs/{feature}");
    let not_exported = source.skipped(Tool::SpecKit, &[ItemType::Risk]);
    let spec_path = format!("{folder}/spec.md");
    let plan_path = format!("{folder}/plan.md");
    let research_path = format!("{folder}/research.md");

    let mut files = vec![File {
        path: spec_path.clone(),
        text: render(|out| spec(out, source, &feature, &not_exported)),
    }];
    if let Some(text) = plan(source, &feature) {
        files.push(File {
            path: plan_path.clone(),
            text,
        });
    }
    let decisions = source.of(ItemType::Decision);
    if !decisions.is_empty() {
        files.push(File {
            path: research_path.clone(),
            text: render(|out| {
                writeln!(out, "# Research: {}", source.title())?;
                for d in &decisions {
                    writeln!(out)?;
                    decision(out, source, d, "##")?;
                }
                Ok(())
            }),
        });
    }
    Export {
        tool: Tool::SpecKit,
        folder,
        files,
        not_exported,
        owned: vec![spec_path, plan_path, research_path],
        warnings: Vec::new(),
    }
}

/// `FR-004` for `FR4`: the Spec Kit number keeps the OpenSkopos number.
fn numbered(prefix: &str, id: &str) -> String {
    match parse_item_id(id) {
        Some((_, n)) => format!("{prefix}-{n:03}"),
        None => id.to_string(),
    }
}

/// Text for inside `[NEEDS CLARIFICATION: ...]`: square brackets become
/// round brackets, so that the marker ends where the question ends.
fn marker_text(text: &str) -> String {
    text.replace('[', "(").replace(']', ")")
}

/// The priority of a story, from the requirement that it refines, and why.
fn priority(source: &Source, story: &Item) -> (u8, Option<String>) {
    let best = story
        .links("refines", source.artifact)
        .into_iter()
        .filter_map(|id| source.get(id))
        .filter(|i| i.item_type == Some(ItemType::Requirement))
        .filter_map(|r| {
            let p = match r.str("priority") {
                "must" => 1,
                "should" => 2,
                "could" => 3,
                _ => return None,
            };
            Some((p, r))
        })
        .min_by_key(|(p, _)| *p);
    match best {
        Some((p, r)) => (
            p,
            Some(format!(
                "it refines {}, a \"{}\" requirement.",
                r.id,
                r.str("priority")
            )),
        ),
        None => (2, None),
    }
}

fn spec(
    out: &mut String,
    source: &Source,
    feature: &str,
    not_exported: &[NotExported],
) -> fmt::Result {
    let date = source.date();
    writeln!(out, "# Feature Specification: {}\n", source.title())?;
    writeln!(out, "**Feature Branch**: `{feature}`\n")?;
    writeln!(out, "**Created**: {date}\n")?;
    writeln!(out, "**Status**: Draft\n")?;
    writeln!(
        out,
        "**Input**: OpenSkopos document `{}`. Each line ends with the reference string of its item.",
        source.label()
    )?;
    if let Some(summary) = source.summary() {
        writeln!(out, "\n{summary}")?;
    }

    let questions = source.of(ItemType::OpenQuestion);
    let (answered, open): (Vec<&Item>, Vec<&Item>) =
        questions.iter().partition(|q| q.field("answer").is_some());
    if !answered.is_empty() {
        writeln!(out, "\n## Clarifications\n\n### Session {date}\n")?;
        for q in answered {
            writeln!(
                out,
                "- Q: {} → A: {} {}",
                q.text(),
                q.field("answer").unwrap_or_default(),
                source.tag(q.id)
            )?;
        }
    }

    writeln!(out, "\n## User Scenarios & Testing *(mandatory)*")?;
    let mut edges: Vec<String> = Vec::new();
    for (n, st) in source.of(ItemType::Story).iter().enumerate() {
        let (p, why) = priority(source, st);
        writeln!(
            out,
            "\n### User Story {} - {} (Priority: P{p})\n",
            n + 1,
            st.short()
        )?;
        writeln!(out, "{} {}", st.text(), source.tag(st.id))?;
        if let Some(d) = st.detail() {
            writeln!(out, "\n{d}")?;
        }
        if let Some(why) = why {
            writeln!(out, "\n**Why this priority**: {why}")?;
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
            writeln!(out, "\n**Acceptance Scenarios**:\n")?;
            for (i, s) in scenarios.iter().enumerate() {
                writeln!(out, "{}. {s}", i + 1)?;
            }
        }
    }

    let requirements = source.of(ItemType::Requirement);
    let mut fr_lines = String::new();
    for r in &requirements {
        writeln!(
            fr_lines,
            "- **{}**: {} {}",
            numbered("FR", r.id),
            r.text(),
            source.tag(r.id)
        )?;
        if let Some(p) = r.field("priority") {
            writeln!(fr_lines, "  - Priority: {p}")?;
        }
        for (kind, text) in r.examples() {
            if kind == "edge" {
                edges.push(format!("{text} {}", source.tag(r.id)));
            } else {
                writeln!(fr_lines, "  - {}: {text}", example_label(kind))?;
            }
        }
        if let Some(d) = r.detail() {
            writeln!(fr_lines, "\n{}", indent(&d, "  "))?;
        }
    }
    if !edges.is_empty() {
        writeln!(out, "\n### Edge Cases\n")?;
        for e in &edges {
            writeln!(out, "- {e}")?;
        }
    }

    writeln!(
        out,
        "\n## Requirements *(mandatory)*\n\n### Functional Requirements\n"
    )?;
    out.push_str(&fr_lines);
    for q in open {
        writeln!(
            out,
            "- [NEEDS CLARIFICATION: {}] {}",
            marker_text(&q.text()),
            source.tag(q.id)
        )?;
        if let Some(options) = q.joined("options", "; ") {
            writeln!(out, "  - Options: {options}")?;
        }
    }

    let terms = source.of(ItemType::Term);
    if !terms.is_empty() {
        writeln!(out, "\n### Key Entities\n")?;
        for t in &terms {
            term(out, source, t)?;
        }
    }

    let signals = source.of(ItemType::SuccessSignal);
    if !signals.is_empty() {
        writeln!(
            out,
            "\n## Success Criteria *(mandatory)*\n\n### Measurable Outcomes\n"
        )?;
        for s in &signals {
            writeln!(
                out,
                "- **{}**: {} {}",
                numbered("SC", s.id),
                s.text(),
                source.tag(s.id)
            )?;
            if let Some(v) = s.field("verification") {
                writeln!(out, "  - Verification: {v}")?;
            }
        }
    }

    let assumptions = source.of(ItemType::Assumption);
    let non_goals = source.of(ItemType::NonGoal);
    if !assumptions.is_empty() || !non_goals.is_empty() {
        writeln!(out, "\n## Assumptions\n")?;
        for a in &assumptions {
            let confirmation = a
                .field("confirmation")
                .unwrap_or_else(|| "unconfirmed".to_string());
            writeln!(out, "- {} ({confirmation}) {}", a.text(), source.tag(a.id))?;
        }
        for n in &non_goals {
            write!(out, "- Out of scope: {}", n.text())?;
            if let Some(r) = n.field("reason") {
                write!(out, " Reason: {r}")?;
            }
            if n.bool("deferred") {
                write!(out, " (deferred: later, not never)")?;
            }
            writeln!(out, " {}", source.tag(n.id))?;
        }
    }
    not_exported_note(out, source, not_exported, "##")
}

fn plan(source: &Source, feature: &str) -> Option<String> {
    let goals = source.of(ItemType::Goal);
    let constraints = source.of(ItemType::Constraint);
    if goals.is_empty() && constraints.is_empty() {
        return None;
    }
    Some(render(|out| {
        writeln!(out, "# Implementation Plan: {}\n", source.title())?;
        writeln!(
            out,
            "**Branch**: `{feature}` | **Date**: {} | **Spec**: [spec.md](spec.md)",
            source.date()
        )?;
        if !goals.is_empty() {
            writeln!(out, "\n## Summary\n")?;
            for g in &goals {
                goal(out, source, g)?;
            }
        }
        if !constraints.is_empty() {
            writeln!(out, "\n## Technical Context\n\n**Constraints**:\n")?;
            for c in &constraints {
                constraint(out, source, c)?;
            }
        }
        Ok(())
    }))
}
