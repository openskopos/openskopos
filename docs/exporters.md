# Exporters

Spec-driven development tools give an AI agent a written spec before it
writes code. OpenSpec, Kiro and Spec Kit are three of these tools. Each one
keeps its spec in Markdown files with its own layout.

OpenSkopos does not compete with these tools. A team can keep what it agreed
in an OpenSkopos document, and export the document to the tool that its
agents already use:

```bash
skopos export spec.skopos.json --to openspec
skopos export spec.skopos.json --to kiro
skopos export spec.skopos.json --to speckit
```

This page tells where each item type goes in each tool. The code is in
`crates/skopos-core/src/export/`. The golden tests in
`crates/skopos-core/tests/golden/` show the full output for three documents.

## Rules for every tool

1. **Only accepted items are exported.** A proposed item is not agreed yet.
   An agent must not act on a rejected item (specification section 4.1).
2. **Every exported item keeps its reference string.** Each line or section
   from an item ends with the reference string of the item, in backticks, for
   example `` `pa_2Xc5Rv8Tn1Mb4Kz7@3#FR4` ``. A later review can find the item
   that a line of the tool's files came from. The reference string has the
   version, so it points to the exact version that the team exported.
3. **No item is dropped in silence.** Each item that is not in the files is
   in a "Not exported" section at the end of the main file, with the reason.
   `skopos export` also writes the list to standard error. The section names
   proposed and rejected items, critiques, and the item types that the tool
   has no place for.
4. **The output is the same each time.** The same document gives the same
   bytes. The files hold no date of the export. Where a tool wants a date, the
   exporter uses the date of `created_at`.
5. **Free text cannot change the layout of a file.** `detail` and `summary`
   are Markdown. Each heading in them, such as `## Background`, becomes a bold
   line, because tools read the headings of their files: for example, OpenSpec
   ends a requirement at a `##` heading. Headings in code fences stay as they
   are. A text that starts with `#` gets a backslash.
6. **Not every field is exported.** Evidence, votes, `proposed_by`, refs and
   `x-` keys stay in the OpenSkopos document. Follow the reference string to
   read them. Refs are used to place items: for example, a story goes with the
   requirement that it `refines`.

When `skopos export --force` writes the files again, it also removes the
files of an earlier export that this export does not write, for example a
`design.md` for a version that has no decisions any more. Without `--force`,
it names those files and writes nothing.

The name of the change or feature comes from the title of the document, in
kebab case, for example `spec-seat-limits-on-plan-changes`. Use `--name` to
choose another name.

## Where each item type goes

| Item | OpenSpec | Kiro | Spec Kit |
|---|---|---|---|
| Goal (`G`) | `proposal.md`, "Why" | `requirements.md`, "Introduction" | `plan.md`, "Summary" |
| Success signal (`S`) | A scenario of the requirement that it verifies. Otherwise `proposal.md`, "Why" | An acceptance criterion of the requirement or story that it verifies. Otherwise "Introduction" | `spec.md`, "Success Criteria": `SC-00n` |
| Requirement (`FR`) | `specs/<name>/spec.md`: `### Requirement:` | `requirements.md`: a numbered requirement. Its text is the first acceptance criterion | `spec.md`, "Functional Requirements": `FR-00n` |
| Example of a requirement or story | `#### Scenario:` with WHEN and THEN | An acceptance criterion | Under its requirement. Edge cases go to "Edge Cases" |
| Story (`ST`) | `proposal.md`, "What Changes" | The user story of the requirement that it refines. Otherwise its own requirement | "User Scenarios & Testing": a user story with acceptance scenarios |
| Constraint (`C`) | `design.md`, "Context" | `design.md`, "Constraints" | `plan.md`, "Technical Context", **Constraints** |
| Assumption (`A`) | `design.md`, "Context" | Not exported | `spec.md`, "Assumptions" |
| Non-goal (`N`) | `design.md`, "Goals / Non-Goals" | Not exported | `spec.md`, "Assumptions", as "Out of scope" |
| Risk (`R`) | `design.md`, "Risks / Trade-offs" | Not exported | Not exported |
| Open question (`Q`) | `design.md`, "Open Questions" | Not exported | With an answer: "Clarifications". Without an answer: `[NEEDS CLARIFICATION: …]` in "Functional Requirements" |
| Decision (`D`) | `design.md`, "Decisions" | `design.md`, "Decisions" | `research.md`: Decision, Rationale, Alternatives considered |
| Term (`T`) | Not exported | `requirements.md`, "Glossary" | `spec.md`, "Key Entities" |
| Critique (`CR`) | Not exported | Not exported | Not exported |

Critiques are never exported. A critique is about an input or about shipped
work. It is not part of what the agent must build. The decision that resolves
a critique is exported.

A file is written only if it has content. For example, a document with no
constraints and no decisions gives no Kiro `design.md`.

## OpenSpec

Checked against OpenSpec 1.17.0 (released 2026-11-09) and the templates of
its `spec-driven` schema.

The exporter writes one change folder, `openspec/changes/<name>/`:

| File | Content |
|---|---|
| `.openspec.yaml` | `schema: spec-driven` and `created`. If the document has no requirements, also `skip_specs: true`, so that `openspec validate` accepts a change with no spec delta. |
| `proposal.md` | Why (summary, goals, other success signals), What Changes (stories), Capabilities, Impact, and the "Not exported" note. |
| `design.md` | Context (constraints and assumptions), Goals / Non-Goals, Decisions, Risks / Trade-offs, Open Questions. |
| `specs/<name>/spec.md` | The spec delta: Purpose, then each requirement under "ADDED Requirements". |

Notes:

- The name of a requirement is its ID and the start of its text, for example
  `FR4 - A plan change that leaves more seats in use than the new...`. The ID
  keeps the names unique.
- `openspec validate` gives an error for a requirement with no scenario. So
  each requirement gets a scenario from each example and each success signal
  that verifies it. A requirement with none of these gets one scenario that
  restates its text.
- `openspec validate` gives a warning (an error with `--strict`) if the text of
  a requirement has no SHALL or MUST. The exporter does not change the words
  of the team. Write the requirement with SHALL or MUST if you use
  `--strict`.
- Every requirement is under "ADDED Requirements". The exporter does not yet
  make MODIFIED or REMOVED deltas from `skopos diff`.
- OpenSpec has no place for terms. Add them to `openspec/config.yaml` under
  `context` if the agent needs them.

## Kiro

Checked against Kiro IDE 1.4.121 and Kiro CLI 3.1.0 (both released
2026-11-05). Kiro does not publish its spec templates. The layout follows the
Kiro documentation and the files that Kiro generates.

The exporter writes one feature spec, `.kiro/specs/<name>/`:

| File | Content |
|---|---|
| `requirements.md` | Introduction (title, summary, goals, other success signals), Glossary (terms), Requirements, and the "Not exported" note. |
| `design.md` | Overview, Constraints and Decisions. |

Notes:

- Each requirement has the user stories that refine it, then its acceptance
  criteria: the text of the requirement, its examples, the acceptance
  criteria and examples of its stories, and the success signals that verify
  it.
- Kiro writes acceptance criteria in EARS form ("WHEN …, THE system SHALL
  …"). The exporter keeps the words of the team and does not rewrite them.
- The exporter does not write `tasks.md` or `.config.kiro`. Kiro makes the
  tasks from the requirements and the design.
- Kiro has no place in a spec for assumptions, non-goals, risks and open
  questions. A constraint for the whole project can also go into a steering
  file in `.kiro/steering/`. The exporter keeps it in `design.md`, because a
  steering file applies to all work, not to one feature.

## Spec Kit

Checked against Spec Kit 1.3.3 (released 2026-11-09): its
`spec-template.md`, `plan-template.md` and scripts.

The exporter writes one feature folder, `specs/<NNN>-<name>/`:

| File | Content |
|---|---|
| `spec.md` | The header, Clarifications (answered questions), User Scenarios & Testing (stories and edge cases), Requirements (requirements, unanswered questions and terms), Success Criteria, Assumptions (assumptions and non-goals), and the "Not exported" note. |
| `plan.md` | Summary (goals) and Technical Context (constraints). |
| `research.md` | One section for each decision. |

Notes:

- The number of a requirement or a success signal keeps its OpenSkopos number:
  `FR4` becomes `FR-004` and `S2` becomes `SC-002`.
- The priority of a user story comes from the requirement that it refines:
  `must` is P1, `should` is P2 and `could` is P3. A story with no such
  requirement is P2.
- `skopos export` takes the feature number of a folder in `specs/` with the
  same name, or the next free number. Like Spec Kit, it counts only prefixes
  of three or more digits, and it ignores folders with a timestamp prefix
  such as `20261110-093000-login`. Use `--number` to choose the number.
- Spec Kit finds the feature to work on in `.specify/feature.json` (the key
  `feature_directory`) or in the variable `SPECIFY_FEATURE_DIRECTORY`, not in
  the folder name. `skopos export` does not change Spec Kit's state. It
  writes the setting to use on standard error. Set it before you run
  `/speckit.plan`, or Spec Kit works on another feature.
- `/speckit.plan` keeps a `plan.md` that exists and does not copy its
  template over it. The plan step then adds its own sections to the exported
  Summary and Technical Context.
- Spec Kit has no "Out of scope" section. Its template puts the limits of
  scope in "Assumptions", so non-goals go there.
- The exporter does not change `.specify/memory/constitution.md`. The
  constitution applies to the whole project, and an export is for one
  feature.
- Spec Kit has no place for risks.

## Import is planned

Import from these tools is planned. It will read OpenSpec or Kiro files into
a new OpenSkopos artifact, so that a team that already has specs can start
from them. An imported item gets a new ID (specification section 6), and the
reference strings in exported files will let the importer find items that
came from OpenSkopos.

## Comparing versions

`skopos diff old.skopos.json new.skopos.json` names the items that were
added, changed, dropped and superseded between two versions. A team can use
it to see what to export again. See
[crates/skopos/README.md](../crates/skopos/README.md#diff).
