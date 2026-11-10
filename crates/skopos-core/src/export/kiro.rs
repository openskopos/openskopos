//! Kiro: one feature spec, `.kiro/specs/<name>/`.
//!
//! Checked against Kiro IDE 1.4.121 and Kiro CLI 3.1.0. Kiro does not publish
//! its templates, so the layout follows the files that Kiro generates.

use std::fmt::{self, Write as _};

use super::{
    constraint, decision, example_label, goal, not_exported_note, render, signal, term, Export,
    File, Item, NotExported, Source, Tool,
};
use crate::model::ItemType;

pub(super) fn export(source: &Source, name: &str) -> Export {
    let folder = format!(".kiro/specs/{name}");
    let not_exported = source.skipped(
        Tool::Kiro,
        &[
            ItemType::Assumption,
            ItemType::NonGoal,
            ItemType::Risk,
            ItemType::OpenQuestion,
        ],
    );
    let requirements_path = format!("{folder}/requirements.md");
    let design_path = format!("{folder}/design.md");
    let mut files = vec![File {
        path: requirements_path.clone(),
        text: render(|out| requirements(out, source, &not_exported)),
    }];
    if let Some(text) = design(source) {
        files.push(File {
            path: design_path.clone(),
            text,
        });
    }
    Export {
        tool: Tool::Kiro,
        folder,
        files,
        not_exported,
        owned: vec![requirements_path, design_path],
        warnings: Vec::new(),
    }
}

/// One Kiro requirement: a requirement item, or a story that refines no
/// exported requirement.
struct Requirement<'a> {
    main: Item<'a>,
    stories: Vec<Item<'a>>,
}

impl Requirement<'_> {
    fn is_requirement(&self) -> bool {
        self.main.item_type == Some(ItemType::Requirement)
    }

    /// The IDs of the main item and its stories.
    fn ids(&self) -> Vec<&str> {
        let mut ids = vec![self.main.id];
        ids.extend(self.stories.iter().map(|s| s.id));
        ids.dedup();
        ids
    }
}

fn requirement_list<'a>(source: &Source<'a>) -> Vec<Requirement<'a>> {
    let stories = source.of(ItemType::Story);
    let mut list: Vec<Requirement> = source
        .of(ItemType::Requirement)
        .into_iter()
        .map(|r| Requirement {
            main: r,
            stories: stories
                .iter()
                .filter(|s| s.links("refines", source.artifact).contains(&r.id))
                .copied()
                .collect(),
        })
        .collect();
    for s in &stories {
        if !list.iter().any(|r| r.stories.iter().any(|x| x.id == s.id)) {
            list.push(Requirement {
                main: *s,
                stories: vec![*s],
            });
        }
    }
    list
}

fn requirements(out: &mut String, source: &Source, not_exported: &[NotExported]) -> fmt::Result {
    writeln!(out, "# Requirements Document\n\n## Introduction\n")?;
    writeln!(
        out,
        "{}. Exported from the OpenSkopos document `{}`. Each line ends with the reference string of its item.\n",
        source.title().trim_end_matches('.'),
        source.label()
    )?;
    if let Some(summary) = source.summary() {
        writeln!(out, "{summary}\n")?;
    }

    let list = requirement_list(source);
    let goals = source.of(ItemType::Goal);
    if !goals.is_empty() {
        writeln!(out, "Goals:\n")?;
        for g in &goals {
            goal(out, source, g)?;
        }
        writeln!(out)?;
    }
    // Success signals that verify a requirement or a story are its
    // acceptance criteria. The others go into the introduction.
    let placed: Vec<&str> = list
        .iter()
        .flat_map(|r| source.signals_for(&r.ids()))
        .map(|s| s.id)
        .collect();
    let other_signals: Vec<Item> = source
        .of(ItemType::SuccessSignal)
        .into_iter()
        .filter(|s| !placed.contains(&s.id))
        .collect();
    if !other_signals.is_empty() {
        writeln!(out, "Success signals:\n")?;
        for s in &other_signals {
            signal(out, source, s)?;
        }
        writeln!(out)?;
    }

    let terms = source.of(ItemType::Term);
    if !terms.is_empty() {
        writeln!(out, "## Glossary\n")?;
        for t in &terms {
            term(out, source, t)?;
        }
        writeln!(out)?;
    }

    writeln!(out, "## Requirements")?;
    for (n, r) in list.iter().enumerate() {
        requirement(out, source, n + 1, r)?;
    }
    not_exported_note(out, source, not_exported, "##")
}

fn requirement(out: &mut String, source: &Source, number: usize, r: &Requirement) -> fmt::Result {
    writeln!(out, "\n### Requirement {number}: {}\n", r.main.short())?;
    for s in &r.stories {
        writeln!(out, "**User Story:** {} {}\n", s.text(), source.tag(s.id))?;
    }
    // The details come before the criteria, so that a list in a detail does
    // not continue the numbered list of criteria.
    let details = std::iter::once(&r.main)
        .filter(|_| r.is_requirement())
        .chain(&r.stories)
        .filter_map(Item::detail);
    for d in details {
        writeln!(out, "{d}\n")?;
    }

    let mut criteria = Vec::new();
    if r.is_requirement() {
        let mut line = format!("{} {}", r.main.text(), source.tag(r.main.id));
        if let Some(p) = r.main.field("priority") {
            write!(line, " (priority: {p})")?;
        }
        criteria.push(line);
        for (kind, text) in r.main.examples() {
            criteria.push(format!(
                "{}: {text} {}",
                example_label(kind),
                source.tag(r.main.id)
            ));
        }
    }
    for s in &r.stories {
        for ac in s.list("acceptance_criteria") {
            criteria.push(format!("{ac} {}", source.tag(s.id)));
        }
        for (kind, text) in s.examples() {
            criteria.push(format!(
                "{}: {text} {}",
                example_label(kind),
                source.tag(s.id)
            ));
        }
    }
    for s in source.signals_for(&r.ids()) {
        let mut line = format!("{} {}", s.text(), source.tag(s.id));
        if let Some(v) = s.field("verification") {
            write!(line, " Verification: {v}")?;
        }
        criteria.push(line);
    }
    if criteria.is_empty() {
        criteria.push(format!("{} {}", r.main.text(), source.tag(r.main.id)));
    }
    writeln!(out, "#### Acceptance Criteria\n")?;
    for (i, c) in criteria.iter().enumerate() {
        writeln!(out, "{}. {c}", i + 1)?;
    }
    Ok(())
}

fn design(source: &Source) -> Option<String> {
    let constraints = source.of(ItemType::Constraint);
    let decisions = source.of(ItemType::Decision);
    if constraints.is_empty() && decisions.is_empty() {
        return None;
    }
    Some(render(|out| {
        writeln!(out, "# Design Document\n\n## Overview\n")?;
        writeln!(
            out,
            "The constraints and decisions of the OpenSkopos document `{}`. The requirements are in requirements.md.",
            source.label()
        )?;
        if !constraints.is_empty() {
            writeln!(out, "\n## Constraints\n")?;
            for c in &constraints {
                constraint(out, source, c)?;
            }
        }
        if !decisions.is_empty() {
            writeln!(out, "\n## Decisions")?;
            for d in &decisions {
                writeln!(out)?;
                decision(out, source, d, "###")?;
            }
        }
        Ok(())
    }))
}
