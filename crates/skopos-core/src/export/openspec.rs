//! OpenSpec: one change folder, `openspec/changes/<name>-v<version>/`.
//!
//! Checked against OpenSpec 1.17.0: the templates of its `spec-driven`
//! schema, its parser and validator, and `openspec archive`.
//!
//! The first export of an artifact ADDs every requirement to a new
//! capability. A later export reads the main specs in `openspec/specs/`
//! (from [`Options::existing`]) and finds the blocks of earlier exports by
//! their reference strings. Then it writes a delta for each capability that
//! holds them: ADDED for new requirements, MODIFIED for changed ones and
//! REMOVED for requirements that the artifact dropped.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as _};

use super::openspec_specs::{
    blocks_of, capability_of, hash, mentions, normalize, open_change_of, same_name,
    without_versions, Block, HASH_PREFIX,
};
use super::{
    capability_path, constraint, decision, example_label, goal, indent, not_exported_note, plural,
    render, signal, Export, ExportError, File, Item, NotExported, Options, Source, Tool,
};
use crate::model::ItemType;

pub(super) fn export(
    source: &Source,
    name: &str,
    options: &Options,
) -> Result<Export, ExportError> {
    let change = match source.version {
        Some(v) => format!("{name}-v{v}"),
        None => name.to_string(),
    };
    let folder = format!("openspec/changes/{change}");
    refuse_open_changes(source, &change, &options.existing)?;

    let requirements = source.of(ItemType::Requirement);
    let not_exported = source.skipped(Tool::OpenSpec, &[ItemType::Term]);
    let plan = Plan::new(source, options, &requirements)?;

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

    let mut files = vec![
        File {
            path: meta.clone(),
            text: render(|out| metadata(out, source, plan.deltas.is_empty())),
        },
        File {
            path: proposal_path.clone(),
            text: render(|out| proposal(out, source, &plan, &other_signals, &not_exported)),
        },
    ];
    if let Some(text) = design(source) {
        files.push(File {
            path: design_path.clone(),
            text,
        });
    }
    for (capability, delta) in &plan.deltas {
        files.push(File {
            path: format!("{folder}/specs/{capability}/spec.md"),
            text: render(|out| delta.write(out, source, plan.is_new(capability))),
        });
    }
    Ok(Export {
        tool: Tool::OpenSpec,
        folder: folder.clone(),
        files,
        not_exported,
        owned: vec![meta, proposal_path, design_path, format!("{folder}/specs/")],
        warnings: plan.warnings,
    })
}

/// Refuses when an earlier export of the artifact is an open change: its
/// deltas are not in the main specs yet, so a new delta would be wrong.
fn refuse_open_changes(
    source: &Source,
    change: &str,
    existing: &[File],
) -> Result<(), ExportError> {
    let open: BTreeSet<&str> = existing
        .iter()
        .filter(|f| mentions(&f.text, source.artifact))
        .filter_map(|f| open_change_of(&f.path))
        .filter(|c| *c != change)
        .collect();
    match open.first() {
        None => Ok(()),
        Some(other) => Err(ExportError::new(format!(
            "openspec/changes/{other} holds an earlier export of {} that is not archived yet. \
             Archive it with `openspec archive {other}`, or delete it, and export again.",
            source.artifact
        ))),
    }
}

/// The spec deltas of one export, by capability.
struct Plan {
    /// The capability for new requirements.
    main: String,
    /// The capabilities that exist in `openspec/specs/`.
    existing: BTreeSet<String>,
    deltas: BTreeMap<String, Delta>,
    warnings: Vec<String>,
}

#[derive(Default)]
struct Delta {
    /// (name, item ID).
    added: Vec<(String, String)>,
    /// (name, item ID).
    modified: Vec<(String, String)>,
    removed: Vec<Removed>,
}

struct Removed {
    name: String,
    item: String,
    reason: String,
    migration: String,
}

impl Plan {
    fn new(source: &Source, options: &Options, requirements: &[Item]) -> Result<Plan, ExportError> {
        let blocks = blocks_of(&options.existing, source.artifact);
        let existing: BTreeSet<String> = options
            .existing
            .iter()
            .filter_map(|f| capability_of(&f.path))
            .map(str::to_string)
            .collect();
        let main = main_capability(source, options, &blocks)?;
        let mut plan = Plan {
            main,
            existing,
            deltas: BTreeMap::new(),
            warnings: Vec::new(),
        };
        let label = source.label();

        for r in requirements {
            let matches: Vec<&Block> = blocks.iter().filter(|b| b.item == r.id).collect();
            if matches.is_empty() {
                let name = requirement_name(r);
                let main = plan.main.clone();
                plan.delta(&main).added.push((name, r.id.to_string()));
                continue;
            }
            let body = requirement_body(source, r);
            for block in matches {
                // A requirement that did not change keeps its block, and the
                // reference string of the export that wrote it.
                if normalize(&without_versions(&body)) == normalize(&without_versions(&block.body))
                {
                    continue;
                }
                if block.hand_edited() {
                    plan.warnings.push(edited_warning(block));
                }
                let new_scenarios = scenario_names_of(source, r);
                if contains_all(&new_scenarios, &block.scenarios) {
                    plan.delta(&block.capability)
                        .modified
                        .push((block.name.clone(), r.id.to_string()));
                } else {
                    // OpenSpec does not let a MODIFIED requirement drop a
                    // scenario. The requirement is replaced under a new name.
                    let mut name = requirement_name(r);
                    if same_name(&name, &block.name) {
                        if let Some(v) = source.version {
                            name = format!("{name} (v{v})");
                        }
                    }
                    let delta = plan.delta(&block.capability);
                    delta.removed.push(Removed {
                        name: block.name.clone(),
                        item: r.id.to_string(),
                        reason: format!(
                            "{} changed in {label} and lost a scenario. OpenSpec does not let a modified requirement drop a scenario.",
                            r.id
                        ),
                        migration: format!("Use the requirement \"{name}\"."),
                    });
                    delta.added.push((name, r.id.to_string()));
                }
            }
        }

        for block in &blocks {
            if requirements.iter().any(|r| r.id == block.item) {
                continue;
            }
            if block.hand_edited() {
                plan.warnings.push(edited_warning(block));
            }
            let by = source.superseded_by(&block.item);
            let (reason, migration) = if !by.is_empty() {
                let ids: Vec<&str> = by.iter().map(|i| i.id).collect();
                (
                    format!("{label} replaces {} with {}.", block.item, ids.join(", ")),
                    format!("Follow {}.", ids.join(", ")),
                )
            } else {
                let why = match source.status_of(&block.item) {
                    Some("rejected") => "is rejected",
                    Some("proposed") => "is only proposed",
                    Some(_) => "is not a requirement",
                    None => "was dropped",
                };
                (
                    format!("{} {why} in {label}.", block.item),
                    "None.".to_string(),
                )
            };
            let capability = block.capability.clone();
            plan.delta(&capability).removed.push(Removed {
                name: block.name.clone(),
                item: block.item.clone(),
                reason,
                migration,
            });
        }
        Ok(plan)
    }

    fn delta(&mut self, capability: &str) -> &mut Delta {
        self.deltas.entry(capability.to_string()).or_default()
    }

    fn is_new(&self, capability: &str) -> bool {
        !self.existing.contains(capability)
    }
}

/// The capability for new requirements: the one that `--name` names, or the
/// one that holds the most requirements of this artifact. With no earlier
/// export, a new capability with the name of the export.
fn main_capability(
    source: &Source,
    options: &Options,
    blocks: &[Block],
) -> Result<String, ExportError> {
    let named = capability_path(source.doc, options);
    if options.name.is_some() || blocks.is_empty() {
        return Ok(named);
    }
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for b in blocks {
        *counts.entry(b.capability.as_str()).or_default() += 1;
    }
    let most = counts.values().copied().max().unwrap_or(0);
    let top: Vec<&str> = counts
        .iter()
        .filter(|(_, n)| **n == most)
        .map(|(c, _)| *c)
        .collect();
    match top.as_slice() {
        [one] => Ok(one.to_string()),
        _ => Err(ExportError::new(format!(
            "the requirements of {} are in the capabilities {}, with {} in each. \
             Choose the capability for new requirements with --name.",
            source.artifact,
            top.join(" and "),
            plural(most, "requirement")
        ))),
    }
}

fn edited_warning(block: &Block) -> String {
    format!(
        "openspec/specs/{}/spec.md: \"{}\" ({}) changed after it was exported. This export replaces the change.",
        block.capability, block.name, block.item
    )
}

/// True if `have` holds every name of `need`, as many times as `need` does.
fn contains_all(have: &[String], need: &[String]) -> bool {
    let mut left: Vec<&String> = have.iter().collect();
    need.iter()
        .all(|n| match left.iter().position(|h| *h == n) {
            Some(i) => {
                left.swap_remove(i);
                true
            }
            None => false,
        })
}

fn metadata(out: &mut String, source: &Source, no_deltas: bool) -> fmt::Result {
    writeln!(out, "schema: spec-driven")?;
    writeln!(out, "created: \"{}\"", source.date())?;
    if no_deltas {
        // `openspec validate` accepts a change with no spec delta only with
        // this marker.
        writeln!(out, "skip_specs: true")?;
    }
    Ok(())
}

fn proposal(
    out: &mut String,
    source: &Source,
    plan: &Plan,
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
    if plan.deltas.is_empty() {
        writeln!(
            out,
            "- No requirement changes. This change has no spec delta."
        )?;
    }
    for (capability, delta) in &plan.deltas {
        let ids = |list: &[(String, String)]| {
            list.iter()
                .map(|(_, id)| id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };
        let mut parts = Vec::new();
        if !delta.added.is_empty() {
            parts.push(format!("adds {}", ids(&delta.added)));
        }
        if !delta.modified.is_empty() {
            parts.push(format!("changes {}", ids(&delta.modified)));
        }
        if !delta.removed.is_empty() {
            let removed: Vec<&str> = delta.removed.iter().map(|r| r.item.as_str()).collect();
            parts.push(format!("removes {}", removed.join(", ")));
        }
        writeln!(out, "- `{capability}`: {}.", parts.join("; "))?;
    }
    for st in source.of(ItemType::Story) {
        story(out, source, &st)?;
    }
    writeln!(out)?;

    writeln!(out, "## Capabilities\n")?;
    let (new, modified): (Vec<&String>, Vec<&String>) =
        plan.deltas.keys().partition(|c| plan.is_new(c));
    if new.is_empty() && modified.is_empty() {
        writeln!(out, "This change changes no capability.\n")?;
    }
    if !new.is_empty() {
        writeln!(out, "### New Capabilities\n")?;
        for c in new {
            writeln!(out, "- `{c}`: {}", source.title())?;
        }
        writeln!(out)?;
    }
    if !modified.is_empty() {
        writeln!(out, "### Modified Capabilities\n")?;
        for c in modified {
            writeln!(out, "- `{c}`: requirements from {}", source.label())?;
        }
        writeln!(out)?;
    }

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

/// The name of a new requirement. The ID keeps the names unique: OpenSpec
/// rejects two requirements with the same name.
fn requirement_name(r: &Item) -> String {
    format!("{} - {}", r.id, r.short())
}

/// The scenarios of a requirement: (name, WHEN, THEN).
fn scenarios(source: &Source, r: &Item) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for (n, (kind, text)) in r.examples().into_iter().enumerate() {
        let then = match kind {
            "forbidden" => "this MUST NOT happen",
            _ => "the system behaves as the example says",
        };
        out.push((
            format!("{} {}", example_label(kind), n + 1),
            text,
            then.to_string(),
        ));
    }
    for s in source.signals_for(&[r.id]) {
        let when = s
            .field("verification")
            .unwrap_or_else(|| "the team checks the change".to_string());
        out.push((
            format!("Success signal {}", s.id),
            when,
            format!("{} {}", s.text(), source.tag(s.id)),
        ));
    }
    if out.is_empty() {
        // OpenSpec rejects a requirement with no scenario.
        out.push((
            format!("{} holds", r.id),
            "the change is done".to_string(),
            r.text(),
        ));
    }
    out
}

fn scenario_names_of(source: &Source, r: &Item) -> Vec<String> {
    scenarios(source, r)
        .into_iter()
        .map(|(n, _, _)| n)
        .collect()
}

/// The lines of a requirement block after its header, without the hash
/// comment.
fn requirement_body(source: &Source, r: &Item) -> String {
    render(|out| {
        writeln!(out, "{}\n", r.text())?;
        write!(out, "Reference: {}", source.tag(r.id))?;
        if let Some(p) = r.field("priority") {
            write!(out, ". Priority: {p}")?;
        }
        writeln!(out)?;
        if let Some(d) = r.detail() {
            writeln!(out, "\n{d}")?;
        }
        for (name, when, then) in scenarios(source, r) {
            writeln!(
                out,
                "\n#### Scenario: {name}\n\n- **WHEN** {when}\n- **THEN** {then}"
            )?;
        }
        Ok(())
    })
}

/// A whole requirement block: the header, the body and the hash comment.
fn requirement_block(out: &mut String, source: &Source, name: &str, item: &str) -> fmt::Result {
    let r = source.get(item).expect("a planned requirement is accepted");
    let body = requirement_body(source, &r);
    writeln!(out, "\n### Requirement: {name}\n")?;
    write!(out, "{body}")?;
    writeln!(out, "\n{HASH_PREFIX}{} -->", hash(&body))
}

impl Delta {
    /// Writes the delta spec. Only a new capability gets a purpose.
    fn write(&self, out: &mut String, source: &Source, new_capability: bool) -> fmt::Result {
        writeln!(out, "# Spec Delta")?;
        if new_capability {
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
            writeln!(out, "\n## Purpose\n\n{purpose}")?;
        }
        if !self.added.is_empty() {
            writeln!(out, "\n## ADDED Requirements")?;
            for (name, item) in &self.added {
                requirement_block(out, source, name, item)?;
            }
        }
        if !self.modified.is_empty() {
            writeln!(out, "\n## MODIFIED Requirements")?;
            for (name, item) in &self.modified {
                requirement_block(out, source, name, item)?;
            }
        }
        if !self.removed.is_empty() {
            writeln!(out, "\n## REMOVED Requirements")?;
            for r in &self.removed {
                writeln!(out, "\n### Requirement: {}\n", r.name)?;
                writeln!(out, "**Reason**: {}", r.reason)?;
                writeln!(out, "**Migration**: {}", r.migration)?;
            }
        }
        Ok(())
    }
}
