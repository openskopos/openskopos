//! OpenSpec: one change folder, `openspec/changes/<name>/`.
//!
//! Checked against OpenSpec 1.17.0: the templates of its `spec-driven`
//! schema, and its parser and validator.

use std::fmt::{self, Write as _};

use super::{
    constraint, decision, example_label, goal, indent, not_exported_note, plural, render, signal,
    Export, File, Item, NotExported, Source, Tool,
};
use crate::model::ItemType;

pub(super) fn export(source: &Source, name: &str) -> Export {
    let folder = format!("openspec/changes/{name}");
    let requirements = source.of(ItemType::Requirement);
    let not_exported = source.skipped(Tool::OpenSpec, &[ItemType::Term]);

    // Success signals that verify a requirement become its scenarios. The
    // others go under "Why", next to the goals.
    let req_ids: Vec<&str> = requirements.iter().map(|r| r.id).collect();
    let scenario_signals = source.signals_for(&req_ids);
    let other_signals: Vec<Item> = source
        .of(ItemType::SuccessSignal)
        .into_iter()
        .filter(|s| !scenario_signals.iter().any(|x| x.id == s.id))
        .collect();

    let meta = format!("{folder}/.openspec.yaml");
    let proposal_path = format!("{folder}/proposal.md");
    let design_path = format!("{folder}/design.md");
    let spec_path = format!("{folder}/specs/{name}/spec.md");

    let mut files = vec![
        File {
            path: meta.clone(),
            text: render(|out| metadata(out, source, requirements.is_empty())),
        },
        File {
            path: proposal_path.clone(),
            text: render(|out| {
                proposal(
                    out,
                    source,
                    name,
                    &requirements,
                    &other_signals,
                    &not_exported,
                )
            }),
        },
    ];
    if let Some(text) = design(source) {
        files.push(File {
            path: design_path.clone(),
            text,
        });
    }
    if !requirements.is_empty() {
        files.push(File {
            path: spec_path.clone(),
            text: render(|out| spec(out, source, &requirements)),
        });
    }
    Export {
        tool: Tool::OpenSpec,
        folder,
        files,
        not_exported,
        owned: vec![meta, proposal_path, design_path, spec_path],
    }
}

fn metadata(out: &mut String, source: &Source, no_requirements: bool) -> fmt::Result {
    writeln!(out, "schema: spec-driven")?;
    writeln!(out, "created: \"{}\"", source.date())?;
    if no_requirements {
        // `openspec validate` accepts a change with no spec delta only with
        // this marker.
        writeln!(out, "skip_specs: true")?;
    }
    Ok(())
}

fn proposal(
    out: &mut String,
    source: &Source,
    name: &str,
    requirements: &[Item],
    signals: &[Item],
    not_exported: &[NotExported],
) -> fmt::Result {
    writeln!(out, "# Proposal: {}\n", source.title())?;
    writeln!(
        out,
        "Exported from the OpenSkopos document `{}`. Each item ends with its reference string.\n",
        source.label()
    )?;

    writeln!(out, "## Why\n")?;
    if let Some(summary) = source.summary() {
        writeln!(out, "{summary}\n")?;
    }
    let goals = source.of(ItemType::Goal);
    if !goals.is_empty() {
        writeln!(out, "Goals:\n")?;
        for g in &goals {
            goal(out, source, g)?;
        }
        writeln!(out)?;
    }
    if !signals.is_empty() {
        writeln!(out, "Success signals:\n")?;
        for s in signals {
            signal(out, source, s)?;
        }
        writeln!(out)?;
    }

    writeln!(out, "## What Changes\n")?;
    if requirements.is_empty() {
        writeln!(out, "- No requirements. This change has no spec delta.")?;
    } else {
        let ids: Vec<&str> = requirements.iter().map(|r| r.id).collect();
        writeln!(
            out,
            "- Adds {} to `specs/{name}/spec.md`: {}.",
            plural(requirements.len(), "requirement"),
            ids.join(", ")
        )?;
    }
    for st in source.of(ItemType::Story) {
        story(out, source, &st)?;
    }
    writeln!(out)?;

    writeln!(out, "## Capabilities\n\n### New Capabilities\n")?;
    writeln!(out, "- `{name}`: {}\n", source.title())?;

    writeln!(out, "## Impact\n")?;
    writeln!(
        out,
        "- Source: OpenSkopos `{}`. A reference string `<artifact>@<version>#<item>` names the item that a line comes from.",
        source.label()
    )?;
    not_exported_note(out, source, not_exported, "##")
}

fn story(out: &mut String, source: &Source, st: &Item) -> fmt::Result {
    writeln!(out, "- Story: {} {}", st.text(), source.tag(st.id))?;
    let refines = st.links("refines", source.artifact);
    if !refines.is_empty() {
        writeln!(out, "  - Refines: {}", refines.join(", "))?;
    }
    for ac in st.list("acceptance_criteria") {
        writeln!(out, "  - Acceptance: {ac}")?;
    }
    for (kind, text) in st.examples() {
        writeln!(out, "  - {}: {text}", example_label(kind))?;
    }
    if let Some(d) = st.detail() {
        writeln!(out, "\n{}", indent(&d, "  "))?;
    }
    Ok(())
}

fn design(source: &Source) -> Option<String> {
    let constraints = source.of(ItemType::Constraint);
    let assumptions = source.of(ItemType::Assumption);
    let non_goals = source.of(ItemType::NonGoal);
    let decisions = source.of(ItemType::Decision);
    let risks = source.of(ItemType::Risk);
    let questions = source.of(ItemType::OpenQuestion);
    if [
        &constraints,
        &assumptions,
        &non_goals,
        &decisions,
        &risks,
        &questions,
    ]
    .iter()
    .all(|list| list.is_empty())
    {
        return None;
    }

    let mut text = render(|out| {
        writeln!(out, "# Design: {}\n", source.title())?;
        if !constraints.is_empty() || !assumptions.is_empty() {
            writeln!(out, "## Context\n")?;
            if !constraints.is_empty() {
                writeln!(out, "Constraints:\n")?;
                for c in &constraints {
                    constraint(out, source, c)?;
                }
                writeln!(out)?;
            }
            if !assumptions.is_empty() {
                writeln!(out, "Assumptions:\n")?;
                for a in &assumptions {
                    assumption(out, source, a)?;
                }
                writeln!(out)?;
            }
        }

        writeln!(
            out,
            "## Goals / Non-Goals\n\n**Goals:**\n\n- See \"Why\" in proposal.md.\n"
        )?;
        if !non_goals.is_empty() {
            writeln!(out, "**Non-Goals:**\n")?;
            for n in &non_goals {
                writeln!(out, "- {} {}", n.text(), source.tag(n.id))?;
                if let Some(r) = n.field("reason") {
                    writeln!(out, "  - Reason: {r}")?;
                }
                if n.bool("deferred") {
                    writeln!(out, "  - Deferred: later, not never.")?;
                }
            }
            writeln!(out)?;
        }

        if !decisions.is_empty() {
            writeln!(out, "## Decisions\n")?;
            for d in &decisions {
                decision(out, source, d, "###")?;
                writeln!(out)?;
            }
        }

        if !risks.is_empty() {
            writeln!(out, "## Risks / Trade-offs\n")?;
            for r in &risks {
                risk(out, source, r)?;
            }
            writeln!(out)?;
        }

        if !questions.is_empty() {
            writeln!(out, "## Open Questions\n")?;
            for q in &questions {
                question(out, source, q)?;
            }
        }
        Ok(())
    });
    text.truncate(text.trim_end().len());
    text.push('\n');
    Some(text)
}

fn assumption(out: &mut String, source: &Source, a: &Item) -> fmt::Result {
    let confirmation = a
        .field("confirmation")
        .unwrap_or_else(|| "unconfirmed".to_string());
    writeln!(out, "- {} ({confirmation}) {}", a.text(), source.tag(a.id))?;
    if let Some(by) = a.field("confirm_by") {
        writeln!(out, "  - Confirm by: {by}")?;
    }
    Ok(())
}

fn risk(out: &mut String, source: &Source, r: &Item) -> fmt::Result {
    let mitigation = r
        .field("mitigation")
        .unwrap_or_else(|| "no mitigation yet".to_string());
    writeln!(
        out,
        "- [Risk] {} → {mitigation} {}",
        r.text(),
        source.tag(r.id)
    )?;
    let rating: Vec<String> = ["likelihood", "impact"]
        .into_iter()
        .filter_map(|key| Some(format!("{key} {}", r.field(key)?)))
        .collect();
    if !rating.is_empty() {
        writeln!(out, "  - {}", rating.join(", "))?;
    }
    Ok(())
}

fn question(out: &mut String, source: &Source, q: &Item) -> fmt::Result {
    writeln!(out, "- {} {}", q.text(), source.tag(q.id))?;
    if q.bool("blocking") {
        writeln!(
            out,
            "  - Blocking: the work cannot continue without an answer."
        )?;
    }
    if let Some(options) = q.joined("options", "; ") {
        writeln!(out, "  - Options: {options}")?;
    }
    if let Some(answer) = q.field("answer") {
        writeln!(out, "  - Answer: {answer}")?;
    }
    Ok(())
}

fn spec(out: &mut String, source: &Source, requirements: &[Item]) -> fmt::Result {
    writeln!(out, "# Spec Delta\n\n## Purpose\n")?;
    // OpenSpec wants a purpose of 50 characters or more.
    let purpose = source
        .summary()
        .and_then(|s| s.split("\n\n").next().map(crate::one_line))
        .filter(|p| p.chars().count() >= 50)
        .unwrap_or_else(|| {
            format!(
                "The requirements of the OpenSkopos document \"{}\" ({}).",
                source.title(),
                source.label()
            )
        });
    writeln!(out, "{purpose}\n\n## ADDED Requirements")?;

    for r in requirements {
        // The ID keeps the names unique: OpenSpec rejects two ADDED
        // requirements with the same name.
        writeln!(out, "\n### Requirement: {} - {}\n", r.id, r.short())?;
        writeln!(out, "{}\n", r.text())?;
        write!(out, "Reference: {}", source.tag(r.id))?;
        if let Some(p) = r.field("priority") {
            write!(out, ". Priority: {p}")?;
        }
        writeln!(out)?;
        if let Some(d) = r.detail() {
            writeln!(out, "\n{d}")?;
        }

        let mut scenarios = 0;
        for (n, (kind, text)) in r.examples().into_iter().enumerate() {
            let then = match kind {
                "forbidden" => "this MUST NOT happen",
                _ => "the system behaves as the example says",
            };
            writeln!(
                out,
                "\n#### Scenario: {} {}\n\n- **WHEN** {text}\n- **THEN** {then}",
                example_label(kind),
                n + 1
            )?;
            scenarios += 1;
        }
        for s in source.signals_for(&[r.id]) {
            let when = s
                .field("verification")
                .unwrap_or_else(|| "the team checks the change".to_string());
            writeln!(
                out,
                "\n#### Scenario: Success signal {}\n\n- **WHEN** {when}\n- **THEN** {} {}",
                s.id,
                s.text(),
                source.tag(s.id)
            )?;
            scenarios += 1;
        }
        if scenarios == 0 {
            // OpenSpec rejects an ADDED requirement with no scenario.
            writeln!(
                out,
                "\n#### Scenario: {} holds\n\n- **WHEN** the change is done\n- **THEN** {}",
                r.id,
                r.text()
            )?;
        }
    }
    Ok(())
}
