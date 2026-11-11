# Exporters

A spec-driven development tool gives a written spec to an AI agent before the
agent writes code. OpenSpec, Kiro and Spec Kit are three of these tools. Each
tool keeps its spec in Markdown files with its own layout.

OpenSkopos does not replace these tools. A team can keep its agreements in an
OpenSkopos document, and export the document to the tool that its agents use:

```bash
skopos export spec.skopos.json --to openspec
skopos export spec.skopos.json --to kiro
skopos export spec.skopos.json --to speckit
```

This page shows where each item type goes in each tool. The code is in
`crates/skopos-core/src/export/`. The golden tests in
`crates/skopos-core/tests/golden/` show the full output for three documents.

## Rules for every tool

1. The exporter exports only accepted items. The team did not agree to a
   proposed item. An agent must not do work for a rejected item
   (specification section 4.1).
2. Each exported item keeps its reference string. Each line or section that
   comes from an item ends with the reference string of the item, in
   backticks, for example `` `pa_2Xc5Rv8Tn1Mb4Kz7@3#FR4` ``. Thus a person who
   reviews the files later can find the source item of each line. The
   reference string contains the version. Thus it points to the version that
   the team exported.
3. The exporter tells you about each item that it does not export. It puts
   each of these items, with the reason, in a "Not exported" section at the
   end of the main file. `skopos export` also writes the list to standard
   error. The section contains proposed items, rejected items, critiques, and
   the item types that the tool cannot hold.
4. The output does not change between exports. The same document always makes
   the same bytes. The files do not contain the date of the export. If a tool
   needs a date, the exporter uses the date of `created_at`.
5. Free text cannot change the layout of a file. `detail` and `summary` are
   Markdown. The exporter changes each heading in them, for example
   `## Background`, into a bold line. The tools read the headings of their
   files: for example, OpenSpec ends a requirement at a `##` heading. Headings
   in code fences do not change. If a text starts with `#`, the exporter puts
   a backslash before it.
6. The exporter does not export all fields. Evidence, votes, `proposed_by`,
   refs and `x-` keys stay in the OpenSkopos document. To read them, use the
   reference string to find the item. The exporter uses refs to find the
   location of items: for example, a story goes with the requirement that it
   `refines`.

When `skopos export --force` writes the files again, it also removes the
files of an earlier export that this export does not write. An example is a
`design.md` for a version that does not have decisions now. Without
`--force`, the tool writes the names of those files and does not write files.

The tool makes the name of the change or feature from the title of the
document, in kebab case, for example `spec-seat-limits-on-plan-changes`. To
use a different name, use `--name`.

## Where each item type goes

| Item | OpenSpec | Kiro | Spec Kit |
|---|---|---|---|
| Goal (`G`) | `proposal.md`, "Why" | `requirements.md`, "Introduction" | `plan.md`, "Summary" |
| Success signal (`S`) | A scenario of the requirement that it verifies. If there is no such requirement, `proposal.md`, "Why" | An acceptance criterion of the requirement or story that it verifies. If there is no such requirement or story, "Introduction" | `spec.md`, "Success Criteria": `SC-00n` |
| Requirement (`FR`) | `specs/<name>/spec.md`: `### Requirement:` | `requirements.md`: a numbered requirement. Its text is the first acceptance criterion | `spec.md`, "Functional Requirements": `FR-00n` |
| Example of a requirement or story | `#### Scenario:` with WHEN and THEN | An acceptance criterion | Under its requirement. Edge cases go to "Edge Cases" |
| Story (`ST`) | `proposal.md`, "What Changes" | The user story of the requirement that it refines. If there is no such requirement, a new requirement | "User Scenarios & Testing": a user story with acceptance scenarios |
| Constraint (`C`) | `design.md`, "Context" | `design.md`, "Constraints" | `plan.md`, "Technical Context", "Constraints" |
| Assumption (`A`) | `design.md`, "Context" | Not exported | `spec.md`, "Assumptions" |
| Non-goal (`N`) | `design.md`, "Goals / Non-Goals" | Not exported | `spec.md`, "Assumptions", as "Out of scope" |
| Risk (`R`) | `design.md`, "Risks / Trade-offs" | Not exported | Not exported |
| Open question (`Q`) | `design.md`, "Open Questions" | Not exported | With an answer: "Clarifications". Without an answer: `[NEEDS CLARIFICATION: …]` in "Functional Requirements" |
| Decision (`D`) | `design.md`, "Decisions" | `design.md`, "Decisions" | `research.md`: Decision, Rationale, Alternatives considered |
| Term (`T`) | Not exported | `requirements.md`, "Glossary" | `spec.md`, "Key Entities" |
| Critique (`CR`) | Not exported | Not exported | Not exported |

The exporter never exports critiques. A critique is about an input or about
work that a team released. It is not a part of the work that the agent must
do. The exporter exports the decision that resolves a critique.

The exporter writes a file only if the file has content. For example, if a
document has no constraints and no decisions, the Kiro export has no
`design.md`.

## OpenSpec

The exporter was tested with OpenSpec 1.17.0 (released 2026-11-09): the
templates of its `spec-driven` schema, its parser and validator, and
`openspec archive`. A CI job uses the real OpenSpec CLI on four exports of one
artifact (see "Tests" on this page).

OpenSpec keeps the agreed behavior of a project in main specs,
`openspec/specs/<capability>/spec.md`. A change folder holds deltas to these
specs. `openspec archive` applies the deltas to the main specs. Thus the
exporter writes one change folder, `openspec/changes/<name>-v<version>/`:

| File | Content |
|---|---|
| `.openspec.yaml` | `schema: spec-driven` and `created`. If the change has no spec delta, also `skip_specs: true`. Without it, `openspec validate` does not accept the change. |
| `proposal.md` | Why (summary, goals, other success signals), What Changes (the deltas and the stories), Capabilities, Impact, and the "Not exported" note. |
| `design.md` | Context (constraints and assumptions), Goals / Non-Goals, Decisions, Risks / Trade-offs, Open Questions. |
| `specs/<capability>/spec.md` | One spec delta for each capability that changes. |

### The first export

The first export of an artifact ADDs all requirements to a new capability.
The exporter gets the name of the capability from `--name`, or from the title
of the document. The delta starts with a Purpose, because OpenSpec needs a
Purpose for a new capability.

### Later exports

`skopos export` reads the main specs and the open changes of the project.
Each exported requirement block contains the reference string of its item.
Thus the exporter finds the blocks that earlier exports of the same artifact
wrote. The team can change the name or the location of a capability, for
example to `billing/seat-limits`. The exporter finds the blocks at their new
location.

Then the exporter compares each accepted requirement with its block:

| The requirement | The delta |
|---|---|
| Has no block | ADDED, to the capability that has the most requirements of the artifact. To select a different capability, use `--name`. |
| Has the same content as its block | No delta. The block keeps the reference string of the export that wrote it. |
| Changed | MODIFIED, with the name of the block. A team can change the name of a requirement in OpenSpec, and the export keeps the new name. |
| Changed, and has one scenario less | REMOVED, and ADDED again with a new name, for example `FR4 - ... (v4)`. `openspec archive` does not accept a MODIFIED requirement that has one scenario less. |
| Is dropped, rejected or only proposed | REMOVED, with the reason. If an accepted item supersedes it, the Migration gives the ID of that item. |

In two cases, the exporter stops with an error and writes nothing:

- An earlier export of the artifact is a change that is not archived. Its
  deltas are not in the main specs. Thus a new delta would be wrong. Archive
  the change with `openspec archive <change>`, or delete the change. Then
  export again. An export of the same version writes over its own open change.
- The requirements of the artifact are in two or more capabilities, with the
  same number of requirements in each. To select the capability for new
  requirements, use `--name`.

Each exported requirement block ends with a comment, for example
`<!-- skopos-export: 9f3a1c2b7d4e6f80 -->`. The comment contains a hash of the
block that the exporter wrote. If a person changes the block in OpenSpec after
the export, the next export finds the change. The export replaces the block
and writes a warning with the name of the requirement. If a block has no
comment, the export replaces it without a warning.

### Notes

- The name of a new requirement is its ID and the start of its text, for
  example `FR4 - A plan change that leaves more seats in use than the new...`.
  The ID makes sure that each name is unique.
- `openspec validate` gives an error for a requirement that has no scenario.
  Thus the exporter makes a scenario for each requirement from each example
  and from each success signal that verifies the requirement. If a
  requirement has no examples and no success signals, the exporter makes one
  scenario that repeats the text of the requirement.
- If the text of a requirement does not contain SHALL or MUST,
  `openspec validate` gives a warning (or an error with `--strict`). The
  exporter does not change the words of the team. If you use `--strict`,
  write the requirement with SHALL or MUST.
- The exporter does not write `tasks.md`. OpenSpec makes the tasks from the
  proposal, the design and the specs, and its apply step needs these tasks.
- OpenSpec cannot hold terms. If the agent needs the terms, add them to
  `openspec/config.yaml` under `context`.

## Kiro

The exporter was tested with Kiro IDE 1.4.121 and Kiro CLI 3.1.0 (both
released 2026-11-05). Kiro does not publish its spec templates. The layout
comes from the Kiro documentation and from the files that Kiro makes.

The exporter writes one feature spec, `.kiro/specs/<name>/`:

| File | Content |
|---|---|
| `requirements.md` | Introduction (title, summary, goals, other success signals), Glossary (terms), Requirements, and the "Not exported" note. |
| `design.md` | Overview, Constraints and Decisions. |

Notes:

- Each requirement has the user stories that refine it, then its acceptance
  criteria. The acceptance criteria are the text of the requirement, its
  examples, the acceptance criteria and examples of its stories, and the
  success signals that verify it.
- Kiro writes acceptance criteria in EARS form ("WHEN …, THE system SHALL
  …"). The exporter keeps the words of the team and does not change them.
- The exporter does not write `tasks.md` or `.config.kiro`. Kiro makes the
  tasks from the requirements and the design.
- A Kiro spec cannot hold assumptions, non-goals, risks and open questions. A
  constraint for the full project can also go into a steering file in
  `.kiro/steering/`. The exporter keeps the constraint in `design.md`, because
  a steering file applies to all work, and a spec is for one feature.

## Spec Kit

The exporter was tested with Spec Kit 1.3.3 (released 2026-11-09): its
`spec-template.md`, `plan-template.md` and scripts.

The exporter writes one feature folder, `specs/<NNN>-<name>/`:

| File | Content |
|---|---|
| `spec.md` | The header, Clarifications (answered questions), User Scenarios & Testing (stories and edge cases), Requirements (requirements, questions without an answer, and terms), Success Criteria, Assumptions (assumptions and non-goals), and the "Not exported" note. |
| `plan.md` | Summary (goals) and Technical Context (constraints). |
| `research.md` | One section for each decision. |

Notes:

- A requirement or a success signal keeps its OpenSkopos number: `FR4`
  becomes `FR-004` and `S2` becomes `SC-002`.
- The exporter gets the priority of a user story from the requirement that it
  refines: `must` is P1, `should` is P2 and `could` is P3. If a story does not
  refine a requirement, its priority is P2.
- `skopos export` uses the feature number of a folder in `specs/` that has the
  same name. If there is no such folder, it uses the next free number. The
  same as Spec Kit, it counts only prefixes of three or more digits. It
  ignores folders with a timestamp prefix, for example
  `20261110-093000-login`. To select the number, use `--number`.
- Spec Kit gets the current feature from `.specify/feature.json` (the key
  `feature_directory`) or from the variable `SPECIFY_FEATURE_DIRECTORY`. It
  does not use the folder name. `skopos export` does not change the state of
  Spec Kit. It writes the necessary value to standard error. Set this value
  before you use `/speckit.plan`. If you do not set it, Spec Kit works on a
  different feature.
- If `plan.md` exists, `/speckit.plan` keeps it and does not copy its template
  over it. The plan step then adds its own sections after the exported
  Summary and Technical Context.
- Spec Kit has no "Out of scope" section. Its template puts the limits of the
  scope in "Assumptions". Thus the non-goals go there.
- The exporter does not change `.specify/memory/constitution.md`. The
  constitution applies to the full project, and an export is for one feature.
- Spec Kit cannot hold risks.

## Tests

The golden tests in `crates/skopos-core/tests/golden.rs` cover each tool.
For OpenSpec, `crates/skopos/tests/openspec_roundtrip.sh` uses the real
OpenSpec CLI. In a new project with one capability that a team wrote by hand,
the script exports versions 2 to 5 of one artifact. After each export, it
uses `openspec validate` and `openspec archive`. The versions add, change,
replace, supersede, reject and drop requirements. The script compares each
change with the golden files. It compares each archived state with the
fixtures in `crates/skopos-core/tests/fixtures/openspec/`. The golden tests
read the same fixtures. Thus they test the exporter with main specs that
OpenSpec wrote. Other tests change those fixtures: a hand edit, a capability
with a new name in a subfolder, requirements in two capabilities, and an open
change.

## Import is planned

A later release will import from these tools. It will read OpenSpec or Kiro
files into a new OpenSkopos artifact. Thus a team that has specs can start
from them. An imported item gets a new ID (specification section 6). The
importer will use the reference strings in exported files to find the items
that came from OpenSkopos.

## Comparing versions

`skopos diff old.skopos.json new.skopos.json` shows the items that were
added, changed, dropped and superseded between two versions. A team can use
it to find the items to export again. See
[crates/skopos/README.md](../crates/skopos/README.md#diff).
