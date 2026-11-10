//! OpenSpec: one change folder, `openspec/changes/<name>/`.
//!
//! Checked against OpenSpec 1.17.0 and its `spec-driven` schema templates.

use super::{
    constraint, decision, example_label, goal, indent, not_exported_note, signal, signals_for,
    Export, File, Item, Source, Tool,
};
use crate::model::ItemType;

pub(super) fn export(source: &Source, name: &str) -> Export {
    let folder = format!("openspec/changes/{name}");
    let requirements = source.of(ItemType::Requirement);
    let not_exported = source.skipped(Tool::OpenSpec, &[ItemType::Term, ItemType::Critique]);

    // Success signals that verify a requirement become its scenarios. The
    // others go under "Why", next to the goals.
    let req_ids: Vec<&str> = requirements.iter().map(|r| r.id).collect();
    let scenario_signals = signals_for(source, &req_ids);
    let other_signals: Vec<Item> = source
        .of(ItemType::SuccessSignal)
        .into_iter()
        .filter(|s| !scenario_signals.iter().any(|x| x.id == s.id))
        .collect();

    let mut meta = format!(
        "schema: spec-driven\ncreated: \"{}\"\n",
        date(source.str("created_at"))
    );
    if requirements.is_empty() {
        meta.push_str("skip_specs: true\n");
    }

    let mut files = vec![File {
        path: format!("{folder}/.openspec.yaml"),
        text: meta,
    }];
    files.push(File {
        path: format!("{folder}/proposal.md"),
        text: proposal(source, name, &requirements, &other_signals, &not_exported),
    });
    if let Some(text) = design(source) {
        files.push(File {
            path: format!("{folder}/design.md"),
            text,
        });
    }
    if !requirements.is_empty() {
        files.push(File {
            path: format!("{folder}/specs/{name}/spec.md"),
            text: spec(source, &requirements),
        });
    }
    Export {
        tool: Tool::OpenSpec,
        folder,
        files,
        not_exported,
    }
}

/// The date part of an RFC 3339 date and time.
fn date(timestamp: &str) -> &str {
    timestamp.get(..10).unwrap_or(timestamp)
}

fn proposal(
    source: &Source,
    name: &str,
    requirements: &[Item],
    signals: &[Item],
    not_exported: &[super::NotExported],
) -> String {
    let mut out = format!("# Proposal: {}\n\n", source.title());
    out.push_str(&format!(
        "Exported from the OpenSkopos document `{}`. Each item ends with its reference string.\n\n",
        source.label()
    ));

    out.push_str("## Why\n\n");
    if let Some(summary) = source.summary() {
        out.push_str(summary);
        out.push_str("\n\n");
    }
    let goals = source.of(ItemType::Goal);
    if !goals.is_empty() {
        out.push_str("Goals:\n\n");
        for g in &goals {
            out.push_str(&goal(source, g));
        }
        out.push('\n');
    }
    if !signals.is_empty() {
        out.push_str("Success signals:\n\n");
        for s in signals {
            out.push_str(&signal(source, s));
        }
        out.push('\n');
    }

    out.push_str("## What Changes\n\n");
    if requirements.is_empty() {
        out.push_str("- No requirements. This change has no spec delta.\n");
    } else {
        let ids: Vec<&str> = requirements.iter().map(|r| r.id).collect();
        out.push_str(&format!(
            "- Adds {} to `specs/{name}/spec.md`: {}.\n",
            plural(requirements.len(), "requirement"),
            ids.join(", ")
        ));
    }
    let stories = source.of(ItemType::Story);
    for st in &stories {
        out.push_str(&story(source, st));
    }
    out.push('\n');

    out.push_str("## Capabilities\n\n### New Capabilities\n\n");
    out.push_str(&format!("- `{name}`: {}\n\n", source.title()));

    out.push_str("## Impact\n\n");
    out.push_str(&format!(
        "- Source: OpenSkopos `{}`. A reference string `<artifact>@<version>#<item>` names the item that a line comes from.\n",
        source.label()
    ));
    out.push_str(&not_exported_note(source, not_exported, "##"));
    out
}

fn story(source: &Source, st: &Item) -> String {
    let mut out = format!("- Story: {} {}\n", st.text(), source.tag(st.id));
    let refines = st.links("refines", source.artifact);
    if !refines.is_empty() {
        out.push_str(&format!("  - Refines: {}\n", refines.join(", ")));
    }
    for ac in st.list("acceptance_criteria") {
        out.push_str(&format!("  - Acceptance: {ac}\n"));
    }
    for (kind, text) in st.examples() {
        out.push_str(&format!("  - {}: {text}\n", example_label(kind)));
    }
    if let Some(d) = st.detail() {
        out.push_str(&format!("\n{}\n", indent(d, "  ")));
    }
    out
}

fn design(source: &Source) -> Option<String> {
    let constraints = source.of(ItemType::Constraint);
    let assumptions = source.of(ItemType::Assumption);
    let non_goals = source.of(ItemType::NonGoal);
    let decisions = source.of(ItemType::Decision);
    let risks = source.of(ItemType::Risk);
    let questions = source.of(ItemType::OpenQuestion);
    if constraints.is_empty()
        && assumptions.is_empty()
        && non_goals.is_empty()
        && decisions.is_empty()
        && risks.is_empty()
        && questions.is_empty()
    {
        return None;
    }

    let mut out = format!("# Design: {}\n\n", source.title());
    if !constraints.is_empty() || !assumptions.is_empty() {
        out.push_str("## Context\n\n");
        if !constraints.is_empty() {
            out.push_str("Constraints:\n\n");
            for c in &constraints {
                out.push_str(&constraint(source, c));
            }
            out.push('\n');
        }
        if !assumptions.is_empty() {
            out.push_str("Assumptions:\n\n");
            for a in &assumptions {
                out.push_str(&assumption(source, a));
            }
            out.push('\n');
        }
    }

    out.push_str("## Goals / Non-Goals\n\n**Goals:**\n\n- See \"Why\" in proposal.md.\n\n");
    if !non_goals.is_empty() {
        out.push_str("**Non-Goals:**\n\n");
        for n in &non_goals {
            out.push_str(&format!("- {} {}\n", n.text(), source.tag(n.id)));
            if let Some(r) = n.field("reason") {
                out.push_str(&format!("  - Reason: {r}\n"));
            }
            if n.bool("deferred") {
                out.push_str("  - Deferred: later, not never.\n");
            }
        }
        out.push('\n');
    }

    if !decisions.is_empty() {
        out.push_str("## Decisions\n\n");
        for d in &decisions {
            out.push_str(&decision(source, d, "###"));
            out.push('\n');
        }
    }

    if !risks.is_empty() {
        out.push_str("## Risks / Trade-offs\n\n");
        for r in &risks {
            let mitigation = r
                .field("mitigation")
                .unwrap_or_else(|| "no mitigation yet".to_string());
            out.push_str(&format!(
                "- [Risk] {} → {mitigation} {}\n",
                r.text(),
                source.tag(r.id)
            ));
            let mut rating = Vec::new();
            if let Some(l) = r.field("likelihood") {
                rating.push(format!("likelihood {l}"));
            }
            if let Some(i) = r.field("impact") {
                rating.push(format!("impact {i}"));
            }
            if !rating.is_empty() {
                out.push_str(&format!("  - {}\n", rating.join(", ")));
            }
        }
        out.push('\n');
    }

    if !questions.is_empty() {
        out.push_str("## Open Questions\n\n");
        for q in &questions {
            out.push_str(&question(source, q));
        }
        out.push('\n');
    }
    out.truncate(out.trim_end().len());
    out.push('\n');
    Some(out)
}

fn assumption(source: &Source, a: &Item) -> String {
    let confirmation = a
        .field("confirmation")
        .unwrap_or_else(|| "unconfirmed".to_string());
    let mut out = format!("- {} ({confirmation}) {}\n", a.text(), source.tag(a.id));
    if let Some(by) = a.field("confirm_by") {
        out.push_str(&format!("  - Confirm by: {by}\n"));
    }
    out
}

fn question(source: &Source, q: &Item) -> String {
    let mut out = format!("- {} {}\n", q.text(), source.tag(q.id));
    if q.bool("blocking") {
        out.push_str("  - Blocking: the work cannot continue without an answer.\n");
    }
    let options = q.list("options");
    if !options.is_empty() {
        out.push_str(&format!("  - Options: {}\n", options.join("; ")));
    }
    if let Some(answer) = q.field("answer") {
        out.push_str(&format!("  - Answer: {answer}\n"));
    }
    out
}

fn spec(source: &Source, requirements: &[Item]) -> String {
    let mut out = String::from("# Spec Delta\n\n## Purpose\n\n");
    let purpose = source
        .summary()
        .and_then(|s| s.split("\n\n").next())
        .map(super::one_line)
        .filter(|p| p.chars().count() >= 50)
        .unwrap_or_else(|| {
            format!(
                "The requirements of the OpenSkopos document \"{}\" ({}).",
                source.title(),
                source.label()
            )
        });
    out.push_str(&purpose);
    out.push_str("\n\n## ADDED Requirements\n");

    for r in requirements {
        out.push_str(&format!("\n### Requirement: {} - {}\n\n", r.id, r.short()));
        out.push_str(&format!("{}\n\n", r.text()));
        let mut line = format!("Reference: {}", source.tag(r.id));
        if let Some(p) = r.field("priority") {
            line.push_str(&format!(". Priority: {p}"));
        }
        out.push_str(&line);
        out.push('\n');
        if let Some(d) = r.detail() {
            out.push_str(&format!("\n{d}\n"));
        }

        let mut scenarios = Vec::new();
        for (n, (kind, text)) in r.examples().into_iter().enumerate() {
            let then = match kind {
                "forbidden" => "this MUST NOT happen",
                _ => "the system behaves as the example says",
            };
            scenarios.push(format!(
                "#### Scenario: {} {}\n\n- **WHEN** {text}\n- **THEN** {then}\n",
                example_label(kind),
                n + 1
            ));
        }
        for s in signals_for(source, &[r.id]) {
            let when = s
                .field("verification")
                .unwrap_or_else(|| "the team checks the change".to_string());
            scenarios.push(format!(
                "#### Scenario: Success signal {}\n\n- **WHEN** {when}\n- **THEN** {} {}\n",
                s.id,
                s.text(),
                source.tag(s.id)
            ));
        }
        if scenarios.is_empty() {
            scenarios.push(format!(
                "#### Scenario: {} holds\n\n- **WHEN** the change is done\n- **THEN** {}\n",
                r.id,
                r.text()
            ));
        }
        for s in scenarios {
            out.push('\n');
            out.push_str(&s);
        }
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
