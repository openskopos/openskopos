# OpenSkopos 0.1

Status: draft. Format 0.1 can change until it is published. After that, each
0.y version can break the one before (see [Format versions](#12-format-versions)).

OpenSkopos is a file format for what a team agrees before an AI agent builds
the work. One document holds typed items, such as goals, requirements,
constraints, risks and critiques. Each item has a stable ID, for example `S1`
or `CR3`. People, commits, tickets and agents use the ID to refer to the item.
The ID keeps its meaning in every version of the document.

This specification is for people who write tools that read or write OpenSkopos
documents, and for people who want to know exactly what a file means.

- JSON Schema: [`schema/artifact/0.1.json`](../../schema/artifact/0.1.json),
  published at `https://openskopos.org/schema/artifact/0.1.json`
- Conformance cases: [`conformance/`](../../conformance/)
- Changes between versions: [CHANGELOG.md](../CHANGELOG.md)

## Contents

1. [Words in this specification](#1-words-in-this-specification)
2. [Files and formats](#2-files-and-formats)
3. [The document](#3-the-document)
4. [Items](#4-items)
5. [Links between items](#5-links-between-items)
6. [Item IDs and reference strings](#6-item-ids-and-reference-strings)
7. [ID counters](#7-id-counters)
8. [Versions and parents](#8-versions-and-parents)
9. [Rules that the schema cannot check](#9-rules-that-the-schema-cannot-check)
10. [The Markdown layout](#10-the-markdown-layout)
11. [Glossaries](#11-glossaries)
12. [Format versions](#12-format-versions)
13. [Conformance](#13-conformance)

## 1. Words in this specification

"Must" and "must not" state a requirement. "Can" states something that is
allowed but not required.

| Word | Meaning |
|---|---|
| Artifact | One thing that a team works on over time, for example the intent for a feature. It has an artifact ID and one or more versions. |
| Version | One state of an artifact. Each version is a full document, not a list of changes. |
| Document | The data of one version, in JSON, YAML or Markdown. |
| Item | One typed statement in a document, for example a goal or a risk. |
| Kind | The type of a document, for example `intent` or `glossary`. |
| Method | The process that made a version, for example a facilitated workshop. |
| Tool | Any software that reads or writes documents. |
| Rendering | A Markdown file made from a document. |

The format version (0.1) and the version number of an artifact (1, 2, 3) are
different numbers. Version 3 of an artifact can use format 0.1.

## 2. Files and formats

A document has three formats:

| Format | Role | Holds every field |
|---|---|---|
| JSON | The canonical format. Tools store and make sure that documents are valid in this format. | Yes |
| YAML | The same data as JSON, for people who edit every field by hand. The same JSON Schema applies. | Yes |
| Markdown | The format that people read, commit and edit most. Its layout is fixed, so that a tool can read it back. | No (see [section 10](#10-the-markdown-layout)) |

Files use UTF-8.

### 2.1 File names

A data file has the kind of the document and the suffix `.skopos.json` or
`.skopos.yaml`, for example `intent.skopos.json`. Editors can match the schema
to a file by this suffix.

A Markdown file has the kind in capital letters, for example `INTENT.md`,
`SPEC.md`, `GLOSSARY.md` or `CRITIQUE.md`.

The team chooses the folder. A team usually keeps the Markdown file next to its
data file. If one folder holds two artifacts of the same kind, the team adds a
name before the kind, for example `billing.intent.skopos.json`.

### 2.2 YAML

A YAML document must use only the data types of JSON: maps, lists, strings,
numbers, booleans and null. Some YAML parsers read unquoted dates as dates and
`yes` or `no` as booleans. To prevent this, a writer must put these in quotes:

- Dates and times, for example `"2026-11-04T10:30:00Z"`.
- The keys `"yes"` and `"no"` of `ratification`.

## 3. The document

A document is a JSON object. These are its fields:

| Field | Required | Meaning |
|---|---|---|
| `$schema` | Yes | The URL of the format version, for example `https://openskopos.org/schema/artifact/0.1.json`. |
| `id` | Yes | The artifact ID: `pa_` and 16 to 32 letters and digits, for example `pa_7Hk2mQ9xR4tV8wZ1`. All versions of an artifact have the same ID. The ID has no meaning of its own. |
| `version` | Yes | The version of the artifact. An integer that starts at 1. |
| `kind` | Yes | The type of document. Lowercase letters, digits and `_`, starting with a letter. The kind does not change between versions. |
| `title` | Yes | A short title, 1 to 200 characters. |
| `summary` | No | A short introduction in Markdown. |
| `status` | Yes | `draft` or `final`. A draft can change. A final version never changes. |
| `method` | No | The method that made this version, if a method made it: `id` (lowercase letters, digits and `-`) and an optional `version`. |
| `session` | No | The working session that made this version, if there was one: `id`, and optional `title`, `url` and `ended_at`. |
| `participants` | Yes | The people and AI agents in the session. The list can be empty. See 3.1. |
| `decider` | No | The participant ID of the person who makes the final call when the group does not agree. |
| `parents` | No | The material and the earlier version that this version came from. See [section 8](#8-versions-and-parents). |
| `id_counters` | No | The highest number used for each item prefix. See [section 7](#7-id-counters). |
| `items` | Yes | The items, in display order. The list can be empty. |
| `created_at` | Yes | When this version was created. An RFC 3339 date and time. |
| `finalized_at` | If `final` | When this version became final. |

Known kinds:

| Kind | Content | Markdown file |
|---|---|---|
| `intent` | Why the work exists and how the team knows it is done | `INTENT.md` |
| `spec` | The behavior that the work must have | `SPEC.md` |
| `agent_rules` | Rules for agents that work in a project | A section of `AGENTS.md` |
| `critique` | Problems that a team found in an input | `CRITIQUE.md` |
| `glossary` | Terms with agreed meanings | `GLOSSARY.md` |

A new kind does not need a new format version. The schema accepts any kind that
matches the pattern. The kinds `ritual_plan`, `handoff_brief` and
`drift_report` are planned but not defined yet.

`kind` and `method` are separate fields because a different method can make the
next version of an artifact.

`method` and `session` are for tools that run structured sessions, for example
a facilitated workshop. A person who writes or edits a document by hand leaves
them out.

### 3.1 Participants

A participant has three fields, and no others:

| Field | Meaning |
|---|---|
| `id` | A local ID: `P` and a positive integer, for example `P1`. It exists only for references inside the document. |
| `name` | A display name, 1 to 100 characters. |
| `kind` | `human` or `ai`. |

A document must not hold an email address, an account ID or any other
personal identifier. Teams commit these files to repositories, and some
repositories are public.

### 3.2 Extension keys

The document and each item accept keys that start with `x-`, for example
`x-tracker`. A tool can store its own data in these keys. Other tools keep them
when they copy an item, and can ignore them.

Every other unknown key makes the document invalid. This finds typing errors,
for example in a document that a language model helped to write.

## 4. Items

### 4.1 Fields of every item

| Field | Required | Meaning |
|---|---|---|
| `id` | Yes | The item ID. The prefix must match the type. See [section 6](#6-item-ids-and-reference-strings). |
| `type` | Yes | One of the twelve item types. |
| `text` | Yes | One statement in plain text, 1 to 2000 characters. |
| `status` | Yes | `proposed`, `accepted` or `rejected`. |
| `detail` | No | More information, in Markdown. |
| `evidence` | No | A list of evidence entries. See 4.4. |
| `refs` | No | A list of links to other items. See [section 5](#5-links-between-items). |
| `proposed_by` | No | The participant ID of the person or agent that proposed the item. |
| `ratification` | No | Vote counts: `yes`, `no` and `eligible`, each an integer of 0 or more. |

A rejected item stays in the version, so that the record shows what the team
decided against. Renderings for agents leave out rejected items.

### 4.2 The twelve item types

| Prefix | `type` | Meaning |
|---|---|---|
| `G` | `goal` | Why the work exists: the problem or the outcome that it serves. |
| `S` | `success_signal` | How the team knows that a goal is met. |
| `FR` | `requirement` | A behavior that the system must have. |
| `ST` | `story` | A requirement told from the view of a user. |
| `C` | `constraint` | A rule on how the work is done. |
| `A` | `assumption` | Something that the team takes as true but did not check, or a fact about the domain. |
| `N` | `non_goal` | Something that the work will not do. |
| `R` | `risk` | Something that can go wrong. |
| `Q` | `open_question` | A question that the team did not answer. |
| `D` | `decision` | A choice that the team made. |
| `T` | `term` | A word with an agreed meaning, and its names in the code. |
| `CR` | `critique` | A problem that the team found in an input or in shipped work. |

Each type has its own fields, in addition to the fields of every item. A field
of one type is not allowed on an item of another type.

#### Goal (`G`)

A goal states the purpose of this change in terms that a person can act on,
for example "exists to recover a compromised account". It is not a broad
objective such as "improve security".

| Field | Meaning |
|---|---|
| `for` | Who benefits. |

#### Success signal (`S`)

A goal says why. A success signal says how the team knows that the goal is
met. A success signal usually links to its goal with `verifies`.

| Field | Meaning |
|---|---|
| `verification` | How a person or an agent can test the signal. |

#### Requirement (`FR`)

A requirement can also be an invariant: a property that must always hold, for
example "the output never contains unbalanced tags".

| Field | Meaning |
|---|---|
| `priority` | `must`, `should` or `could`. |
| `examples` | A list of examples. See 4.3. |

#### Story (`ST`)

A story links to its requirement with `refines`.

| Field | Meaning |
|---|---|
| `acceptance_criteria` | A list of strings. |
| `examples` | A list of examples. See 4.3. |

#### Constraint (`C`)

A constraint applies to this work or to the whole project.

| Field | Required | Meaning |
|---|---|---|
| `modality` | Yes | `always`, `never`, `ask_first`, `prefer` or `avoid`. |
| `rationale` | No | Why the rule exists. |
| `scope` | No | A list of places where the rule applies, for example paths, areas or "this work only". |
| `enforced_by` | No | The check or hook that enforces the rule. If this field is absent, only the agent enforces the rule. |
| `approver` | For `ask_first` | Who the agent asks: a role or a participant ID. |
| `bring` | No | A list of things that the agent shows the approver, for example a plan. |

The modalities:

- `always` and `never` are hard rules.
- `prefer` and `avoid` are defaults. An agent can break them if it gives a
  reason.
- `ask_first` means that the agent stops and asks before it acts. The
  constraint must have an `approver`. It can have `bring`.

Only `ask_first` constraints can have `approver` or `bring`.

#### Assumption (`A`)

An open question has no answer yet. An assumption has a provisional answer that
someone must confirm.

| Field | Meaning |
|---|---|
| `confirmation` | `unconfirmed`, `confirmed` or `refuted`. If the field is absent, the value is `unconfirmed`. |
| `owner` | The participant ID of the person who confirms or refutes the assumption. |
| `confirm_by` | The date by which the owner does this. An RFC 3339 full date, for example `2026-11-20`. |

#### Non-goal (`N`)

| Field | Meaning |
|---|---|
| `reason` | Why the work leaves this out. |
| `deferred` | `true` means later, not never. |

#### Risk (`R`)

A risk links to what it threatens with `threatens`.

| Field | Meaning |
|---|---|
| `likelihood` | `low`, `medium` or `high`. |
| `impact` | `low`, `medium` or `high`. |
| `mitigation` | What makes the risk smaller. |
| `owner` | A participant ID. |

#### Open question (`Q`)

| Field | Meaning |
|---|---|
| `owner` | A participant ID. |
| `blocking` | `true` if the work cannot continue without an answer. |
| `options` | A list of possible answers. |
| `answer` | The answer, when the team has one. |

#### Decision (`D`)

A decision links to the question or critique that it settles with `resolves`.

| Field | Meaning |
|---|---|
| `rationale` | Why the team chose this. |
| `alternatives` | A list of the options that the team did not choose. |
| `consequences` | A list of results of the decision. |
| `decided_by` | A participant ID. |

#### Term (`T`)

The `text` of a term is the word itself.

| Field | Required | Meaning |
|---|---|---|
| `definition` | Yes | What the word means. |
| `aliases` | No | A list of other words for the same thing. |
| `code_names` | No | A list of identifiers in the code for this term. |

#### Critique (`CR`)

| Field | Required | Meaning |
|---|---|---|
| `severity` | Yes | `blocker`, `major`, `minor` or `note`. |
| `category` | Yes | See the list below. |
| `evidence` | Yes | At least one evidence entry. |
| `suggestion` | No | What the critic proposes. |
| `target` | No | What the critique is about. |
| `resolution` | If `accepted` | What the team decided to do. |

The categories are `ambiguity`, `gap`, `conflict`, `untestable`, `incorrect`,
`unstated_assumption`, `scope_creep`, `over_specified` and `other`. `incorrect`
means that the input contradicts the code or a known fact.

The `target` has an `item` or a `section`, or both:

- `item`: an item ID in an input artifact. Add `artifact` and `version` to name
  the artifact.
- `section`: the name of a section of an external input.

The `resolution` has these fields:

| Field | Required | Meaning |
|---|---|---|
| `outcome` | Yes | `open`, `fix_spec`, `fix_code`, `accept_risk` or `defer`. |
| `note` | No | More about the outcome. |
| `decided_by` | No | A participant ID. |

`fix_code` means that the code is wrong and the spec is right. If the team
thinks that a critique is wrong, the critique gets the status `rejected`.

### 4.3 Examples

An example belongs to a requirement or a story. It has two fields:

| Field | Required | Meaning |
|---|---|---|
| `text` | Yes | The example. |
| `kind` | No | `normal` (the usual case), `edge` (a boundary case) or `forbidden` (what must not happen). If the field is absent, the value is `normal`. |

### 4.4 Evidence

Each evidence entry has a `kind`. The other fields depend on the kind.

| `kind` | Required fields | Optional fields | Meaning |
|---|---|---|---|
| `quote` | `text` | `from`, `location` | Text from an input. `from` can have `artifact`, `version`, `item` and `title`. |
| `code` | `path`, `line_start` | `repo`, `line_end`, `commit`, `snippet` | Lines in a repository. `commit` is a hex hash of 7 to 64 characters. |
| `discussion` | `text` | `speaker` | What someone said in the session. `speaker` is a participant ID. |
| `link` | `uri` | `title` | A web page or another resource. |

A tool that writes a `snippet` must remove secrets from it first.

## 5. Links between items

A ref goes from the item that holds it to the linked item. It has these fields:

| Field | Required | Meaning |
|---|---|---|
| `item` | Yes | The ID of the linked item. |
| `artifact` | No | The artifact ID, if the linked item is in another artifact. |
| `version` | No | The version of that artifact. A ref with `version` must have `artifact`. |
| `rel` | No | How the two items relate. |

| `rel` | The item that holds the link… | Example |
|---|---|---|
| `refines` | gives more detail about the linked item. | `ST1` refines `FR1` |
| `derived_from` | comes from the linked item. | `FR2` is derived from `G1` |
| `depends_on` | needs the linked item to be true or done. | `FR3` depends on `A1` |
| `threatens` | can stop the linked item. | `R1` threatens `FR1` |
| `mitigates` | makes the linked risk smaller. | `C2` mitigates `R1` |
| `resolves` | settles the linked question or critique. | `D1` resolves `Q1` |
| `verifies` | tests the linked item. | `S1` verifies `G1` |
| `supersedes` | replaces the linked item, which the team dropped. | `FR4` supersedes `FR2` |
| `contradicts` | cannot be true together with the linked item. | `A2` contradicts `A1` |
| `relates_to` | has another relation to the linked item. | |

A ref can point to an item that an earlier version dropped. This is normal for
`supersedes`.

## 6. Item IDs and reference strings

1. An item ID is a type prefix and a positive integer with no leading zero,
   for example `S1` or `CR12`. Each prefix has its own numbers.
2. Software assigns item IDs. A language model or a person never chooses an
   ID. A tool assigns IDs with the counters in [section 7](#7-id-counters).
3. An ID belongs to one item for the life of the artifact, in all versions. If
   the text of the item changes, the ID stays the same.
4. An ID is never used again. If version 3 drops `S2` and the highest success
   signal ever used is `S4`, the next success signal is `S5`. Gaps are normal.
5. If the meaning of an item changes, the team drops the old item and adds a
   new item with a new ID. The new item links to the old one with
   `supersedes`.
6. The order of `items` is the display order. A change of order does not change
   the IDs.
7. Item IDs are unique in a version.

Inside one document, the item ID is enough, for example `CR3`. Outside the
document, for example in Markdown, commit messages and tickets, use a reference
string:

```
<artifact id>[@<version>]#<item id>
```

For example `pa_7Hk2mQ9xR4tV8wZ1@2#CR3`. A reference string without
`@<version>` points to the item in the latest final version of the artifact.

## 7. ID counters

`id_counters` holds the highest number ever used for each item prefix in the
artifact, in all versions, for example `{"S": 4, "CR": 7}`. A missing prefix
means 0. With the counters, any tool can assign IDs without a server.

To add an item, a tool does these steps:

1. Read the counter for the prefix of the item type.
2. Give the item the prefix and the counter plus 1.
3. Write the new counter back into the document.

If a tool does not write the counter back, the next tool can give the same ID
to a different item.

Each counter must be equal to or more than the highest number of its prefix in
`items`. A counter can be higher, because items that earlier versions dropped
used numbers too.

## 8. Versions and parents

### 8.1 Versions

1. An artifact has one or more versions. Each version is a full document.
2. A version starts as `draft`. The team can change a draft. When the method
   ends, the version becomes `final` and gets `finalized_at`. A final version
   never changes.
3. To change a final artifact, a tool makes version n+1. The new draft starts
   with the items of version n and their IDs. It has the parent
   `{"rel": "previous_version", "artifact": "<same id>", "version": n}`.
4. Version 1 has no `previous_version` parent. Every later version has exactly
   one.
5. Version numbers increase by one. An artifact has no branches. An artifact
   has at most one draft at a time.
6. If a second team wants to change an artifact while a draft exists, it starts
   a new artifact. The new artifact has the latest final version as an `input`
   parent.

### 8.2 Parents

A parent has one of two forms.

An artifact parent is a version of an OpenSkopos artifact:

| Field | Required | Meaning |
|---|---|---|
| `rel` | Yes | `previous_version` or `input`. |
| `artifact` | Yes | The artifact ID. |
| `version` | Yes | The version. |
| `kind` | No | The kind of that artifact. |
| `title` | No | The title of that artifact. |

An external parent is material that is not an OpenSkopos document:

| Field | Required | Meaning |
|---|---|---|
| `rel` | Yes | Always `input`. |
| `source` | Yes | The material. See the next table. |

| `source` field | Required | Meaning |
|---|---|---|
| `kind` | Yes | `pasted`, `url`, `repo_file`, `ticket` or `document`. |
| `title` | Yes | A title for the material. |
| `uri` | No | Where the material is. |
| `repo`, `path`, `commit` | No | Where the file is in a repository. |
| `digest` | No | `sha256:` and the hex SHA-256 hash of the text that the team used. |

`previous_version` names the version that this version changes. It has the
same artifact ID. `input` names material that the method worked from. With the
`digest`, a tool can find out if an input changed after the session.

## 9. Rules that the schema cannot check

JSON Schema cannot express these rules. A tool that writes a document must
make sure that they are true. A tool that checks a document must report each
one that is false. The conformance cases use the names in the first column.

| Name | Rule |
|---|---|
| `unique-item-ids` | No two items in a version have the same ID. |
| `counters-cover-items` | For each prefix in `items`, the counter in `id_counters` is equal to or more than the highest number of that prefix. A missing counter is 0. |
| `previous-version-same-artifact` | A `previous_version` parent has the same artifact ID as the document. |
| `previous-version-one-less` | A `previous_version` parent has the version of the document minus 1. |

One more rule needs the earlier versions of the artifact: an ID that an
earlier version used for one item is never used for a different item. A tool
that has the earlier versions must make sure that this is true.

## 10. The Markdown layout

A Markdown rendering is for agents and for people who edit by hand. The layout
is fixed, so that a tool can read an edited file back into a document.

A rendering does not hold every field. These fields stay in the JSON only:
`detail`, `evidence`, `refs`, `proposed_by`, `ratification`, `x-` keys,
`participants`, `parents`, `method`, `session`, `id_counters`, `created_at`
and `finalized_at`.

### 10.1 Layout

A writer makes the rendering in this order:

1. Front matter in YAML between two `---` lines, with these keys in this
   order: `schema` (the `$schema` URL), `id`, `version`, `kind`, `status`.
2. One blank line, then the title as an H1: `# <title>`.
3. If the document has a summary: one blank line, then the summary.
4. One H2 section for each item type that has items to show, in this order:
   Goals, Success signals, Requirements, Stories, Constraints, Assumptions,
   Non-goals, Risks, Open questions, Decisions, Terms, Critiques. The writer
   leaves out a section with no items.
5. If the rendering copies material from other artifacts: the section
   `## From other artifacts` (see 10.4).

Each H2 and H3 heading has one blank line before it and one blank line after
it. The
file ends with one line break.

The writer leaves out rejected items. In each section, the items are in the
order of `items` in the document.

### 10.2 Item lines

Each item is one list item:

```
- <id>: <text>
```

For example `- S1: A customer can downgrade without contacting support.`

The fields of the item follow as nested list items, indented by two spaces:

```
  - <key>: <value>
```

A text or a value must be on one line. If it holds a line break, the writer
replaces the line break with one space.

### 10.3 Field lines

If the status of the item is not `accepted`, the first field line is
`status: <status>`. An item with no status line is accepted. In the sections
of the document, this line is always `status: proposed`, because the writer
leaves out rejected items.

Then come the fields of the type, in this order:

| Type | Field lines |
|---|---|
| Goal | `for` |
| Success signal | `verification` |
| Requirement | `priority`, then one line for each example |
| Story | one `ac` line for each acceptance criterion, then one line for each example |
| Constraint | `modality`, `rationale`, one `scope` line for each value, `enforced_by`, `approver`, one `bring` line for each value |
| Assumption | `confirmation`, `owner`, `confirm_by` |
| Non-goal | `reason`, `deferred` |
| Risk | `likelihood`, `impact`, `mitigation`, `owner` |
| Open question | `owner`, `blocking`, one `options` line for each value, `answer` |
| Decision | `rationale`, one `alternatives` line for each value, one `consequences` line for each value, `decided_by` |
| Term | `definition`, one `aliases` line for each value, one `code_names` line for each value |
| Critique | `severity`, `category`, `suggestion`, `target.artifact`, `target.version`, `target.item`, `target.section`, `resolution.outcome`, `resolution.note`, `resolution.decided_by` |

The writer leaves out a field that the item does not have.

- An example uses its kind as the key: `example` for `normal`, `edge` for
  `edge`, and `forbidden` for `forbidden`.
- A list field repeats its key once for each value.
- A field inside `target` or `resolution` uses the name of the object, a dot
  and the name of the field.
- A boolean is `true` or `false`.

For example:

```markdown
- FR1: The billing page lets an org admin pick a smaller plan.
  - priority: must
  - example: An admin on Team picks Starter. The page shows Starter from 1 December.
  - edge: An admin with 12 seats picks a plan with 10 seats. The page blocks the change.
  - forbidden: A member who is not an admin sees the plan picker.
- C2: Ask before adding a database migration.
  - modality: ask_first
  - approver: Tech lead
  - bring: The migration
  - bring: A rollback plan
```

### 10.4 Material from other artifacts

A rendering is complete on its own, so that an agent does not need other files.
Agents do not follow links. The writer copies this material from other
artifacts, if it has their documents:

1. The accepted terms of each `glossary` artifact that the document has as an
   `input` parent.
2. The accepted constraints of each `agent_rules` artifact that the document
   has as an `input` parent.
3. Each item in another artifact that a shown item refers to with a ref that
   has `artifact` and `version`. The writer copies each item one time only.

The material goes under `## From other artifacts`, at the end of the file. It
has one H3 for each source artifact and item type:

```
### <section name> from <title> (<artifact id>@<version>)
```

The H3 headings for items 1 and 2 come first, in the order of the parents.
The H3 headings for item 3 follow, in the order of the item types, and then
in the order in which the shown items refer to them.

Each copied item has the same layout as an item line, with the reference string
in place of the ID:

```markdown
### Terms from Billing glossary (pa_9Lp4Wd7Ks2Hq6Nb3@1)

- pa_9Lp4Wd7Ks2Hq6Nb3@1#T1: Seat
  - definition: One user who can sign in to an org.
  - code_names: Billing.Seat
```

The JSON does not copy this material. It keeps the parents and the refs. A
copy is a snapshot: if the glossary changes, the rendering shows the old terms
until a tool renders it again. To change a copied item, change it in its own
artifact.

### 10.5 Reading a file back

A reader takes an edited rendering and the document that the rendering came
from. This document is the base version. The reader makes the next document
with these steps:

1. Read the front matter. `schema`, `id`, `version` and `kind` must be equal to
   the base version. The reader ignores `status`.
2. The H1 is the new title. The text between the H1 and the first H2 is the new
   summary, without blank lines at the start and the end. If there is no text,
   the new document has no summary.
3. Ignore all lines from `## From other artifacts` to the next H2 or the end of
   the file.
4. Each other H2 must be one of the section names in 10.1.
5. In a section, read each line:
   - A blank line: ignore it.
   - `- <id>: <text>`: an existing item. See step 6.
   - `- new: <text>`: a new item of the type of the section. See step 7.
   - A line indented by two or more spaces, in the form `- <key>: <value>`: a
     field of the item above it. The key must be a field line of that type
     (10.3), or `status`. A field that is not a list must appear only once.
   - Any other line is an error.
6. For an existing item:
   - The ID must be an item of the rendering: an item of the base version that
     is not rejected.
   - The ID must be under the section of its type.
   - The ID must appear only once.
   - The item gets its text and its field lines from the file. A field line that
     is not in the file removes that field.
   - The fields that stay in the JSON only keep their values from the base
     version.
7. For a new item, the reader assigns the next ID of the prefix (section 7). The
   item gets its text and its field lines from the file.
8. The status of an item comes from its `status` line. With no status line, the
   item is accepted. A person can reject an item with `- status: rejected`.
9. An item of the rendering that is not in the file is dropped from the new
   version. Its ID is not used again.
10. Rejected items of the base version do not appear in the rendering. They
    stay in the new version without change, at the end of `items`, in their
    base order.
11. The order of the other items is their order in the file.
12. Each counter in `id_counters` becomes the highest of: the base counter, the
    highest number of the prefix in the base version, and the IDs that step 7
    assigned. The reader writes each counter that is more than 0.
13. If the base version is final, the new document is version n+1, with the
    status `draft`. Its parents are a `previous_version` parent for version n,
    then the other parents of the base version. It has no `finalized_at`,
    `method` or `session`, because these describe how the base version was
    made.
14. If the base version is a draft, the new document replaces that draft. It
    keeps the version number, the parents, `created_at`, `method` and
    `session`.
15. For a new version, the tool sets `created_at`. If the tool runs a method or a
    session, it can also set `method` and `session`.
16. The new document must be valid against the schema and the rules in
    [section 9](#9-rules-that-the-schema-cannot-check). If it is not, the
    reader reports an error and makes no document.

Values in field lines:

- `deferred` and `blocking` must be `true` or `false`.
- `target.version` is an integer.
- Every other value is a string.
- The reader removes spaces at the start and the end of each text and value.
- Each example gets a `kind`, also for `normal`.

A rendering does not hold evidence. Thus a new critique, which needs evidence,
cannot come from Markdown alone. Add critiques in JSON or YAML.

## 11. Glossaries

A glossary is an artifact of kind `glossary`. Its items are terms (`T`).

- The team sets the scope of a glossary in its title and summary: a company, a
  project, or one area of a project. The format does not model a hierarchy of
  glossaries.
- An artifact can pin several glossaries as `input` parents. Each parent names
  the version of the glossary that the artifact used.
- If two pinned glossaries define the same term in different ways, tools show
  the conflict. They do not choose a winner.
- A rendering copies the accepted terms of the pinned glossaries (see 10.4), so
  that an agent knows what each word means.

A later format version can add an optional `scope` field to the envelope, if
teams need the scope as data.

## 12. Format versions

The `$schema` URL of a document names its format version:

```
https://openskopos.org/schema/artifact/<format version>.json
```

- The format stays at 0.x while early users try it. The first version is 0.1.
- Each 0.y version can break the one before. [CHANGELOG.md](../CHANGELOG.md)
  lists each breaking change. The reference implementation will migrate
  documents to the new version.
- From 1.0, a minor version (1.0 to 1.1) only adds things: optional fields,
  item types or enum values. A major version (2.0) is a breaking change.
- A published schema file never changes. A change publishes a new file with a
  new version.

The schema for 0.1 accepts any 0.y URL in `$schema`, so that a 0.1 tool can
read a document of a later 0.y version and report what it does not know.

## 13. Conformance

The JSON Schema and the conformance cases in [`conformance/`](../../conformance/)
define correct behavior. An implementation in any language that passes all
cases is correct. A tool that checks documents must:

- Accept every document in `conformance/valid/`.
- Reject every document in `conformance/invalid/`, for the reason in its index
  entry.

A tool that writes and reads Markdown must also pass the cases in
`conformance/markdown/`:

- From `document.json` and the documents in `inputs/`, it writes
  `rendering.md`, byte for byte.
- From `document.json` and `edited.md`, it makes a document equal to
  `expected.json`. The comparison ignores the order of keys and the field
  `created_at`.
- For each case in `markdown/invalid/`, it reports an error.
