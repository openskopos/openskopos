//! Kiro: one feature spec, `.kiro/specs/<name>/`.
//!
//! Checked against Kiro IDE 1.4.121 and Kiro CLI 3.1.0. Kiro does not publish
//! its templates, so the layout follows the files that Kiro generates.

use super::{
    constraint, decision, example_label, goal, not_exported_note, signal, signals_for, term,
    Export, File, Item, Source, Tool,
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
            ItemType::Critique,
        ],
    );
    let mut files = vec![File {
        path: format!("{folder}/requirements.md"),
        text: requirements(source, &not_exported),
    }];
    if let Some(text) = design(source) {
        files.push(File {
            path: format!("{folder}/design.md"),
            text,
        });
    }
    Export {
        tool: Tool::Kiro,
        folder,
        files,
        not_exported,
    }
}

/// One Kiro requirement: a requirement item, or a story that refines no
/// exported requirement.
struct Requirement<'a> {
    main: Item<'a>,
    stories: Vec<Item<'a>>,
}

fn requirements(source: &Source, not_exported: &[super::NotExported]) -> String {
    let mut out = String::from("# Requirements Document\n\n## Introduction\n\n");
    out.push_str(&format!(
        "{}. Exported from the OpenSkopos document `{}`. Each line ends with the reference string of its item.\n\n",
        source.title().trim_end_matches('.'),
        source.label()
    ));
    if let Some(summary) = source.summary() {
        out.push_str(summary);
        out.push_str("\n\n");
    }

    let reqs = source.of(ItemType::Requirement);
    let stories = source.of(ItemType::Story);
    let mut list: Vec<Requirement> = reqs
        .iter()
        .map(|r| Requirement {
            main: *r,
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

    // Success signals that verify a requirement or a story are its
    // acceptance criteria. The others go into the introduction.
    let mut placed: Vec<&str> = Vec::new();
    for r in &list {
        let mut ids = vec![r.main.id];
        ids.extend(r.stories.iter().map(|s| s.id));
        placed.extend(signals_for(source, &ids).iter().map(|s| s.id));
    }
    let goals = source.of(ItemType::Goal);
    if !goals.is_empty() {
        out.push_str("Goals:\n\n");
        for g in &goals {
            out.push_str(&goal(source, g));
        }
        out.push('\n');
    }
    let other_signals: Vec<Item> = source
        .of(ItemType::SuccessSignal)
        .into_iter()
        .filter(|s| !placed.contains(&s.id))
        .collect();
    if !other_signals.is_empty() {
        out.push_str("Success signals:\n\n");
        for s in &other_signals {
            out.push_str(&signal(source, s));
        }
        out.push('\n');
    }

    let terms = source.of(ItemType::Term);
    if !terms.is_empty() {
        out.push_str("## Glossary\n\n");
        for t in &terms {
            out.push_str(&term(source, t));
        }
        out.push('\n');
    }

    out.push_str("## Requirements\n");
    for (n, r) in list.iter().enumerate() {
        out.push_str(&format!(
            "\n### Requirement {}: {}\n\n",
            n + 1,
            r.main.short()
        ));
        for s in &r.stories {
            out.push_str(&format!(
                "**User Story:** {} {}\n\n",
                s.text(),
                source.tag(s.id)
            ));
        }
        out.push_str("#### Acceptance Criteria\n\n");
        let mut criteria = Vec::new();
        let is_requirement = r.main.item_type == Some(ItemType::Requirement);
        if is_requirement {
            let mut line = format!("{} {}", r.main.text(), source.tag(r.main.id));
            if let Some(p) = r.main.field("priority") {
                line.push_str(&format!(" (priority: {p})"));
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
        let mut ids = vec![r.main.id];
        ids.extend(r.stories.iter().map(|s| s.id));
        for s in signals_for(source, &ids) {
            let mut line = format!("{} {}", s.text(), source.tag(s.id));
            if let Some(v) = s.field("verification") {
                line.push_str(&format!(" Verification: {v}"));
            }
            criteria.push(line);
        }
        if criteria.is_empty() {
            criteria.push(format!("{} {}", r.main.text(), source.tag(r.main.id)));
        }
        for (i, c) in criteria.iter().enumerate() {
            out.push_str(&format!("{}. {c}\n", i + 1));
        }
        let details: Vec<&str> = std::iter::once(&r.main)
            .filter(|_| is_requirement)
            .chain(r.stories.iter())
            .filter_map(|i| i.detail())
            .collect();
        for d in details {
            out.push_str(&format!("\n{d}\n"));
        }
    }
    out.push_str(&not_exported_note(source, not_exported, "##"));
    out
}

fn design(source: &Source) -> Option<String> {
    let constraints = source.of(ItemType::Constraint);
    let decisions = source.of(ItemType::Decision);
    if constraints.is_empty() && decisions.is_empty() {
        return None;
    }
    let mut out = String::from("# Design Document\n\n## Overview\n\n");
    out.push_str(&format!(
        "The constraints and decisions of the OpenSkopos document `{}`. The requirements are in requirements.md.\n",
        source.label()
    ));
    if !constraints.is_empty() {
        out.push_str("\n## Constraints\n\n");
        for c in &constraints {
            out.push_str(&constraint(source, c));
        }
    }
    if !decisions.is_empty() {
        out.push_str("\n## Decisions\n");
        for d in &decisions {
            out.push('\n');
            out.push_str(&decision(source, d, "###"));
        }
    }
    Some(out)
}
