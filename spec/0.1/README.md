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
  served at `https://openskopos.org/schema/artifact/0.1.json` once it is
  published
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
10. [Markdown](#10-markdown)
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

The format version (0.1) and the version number of an artifact (1, 2, 3) are
different numbers. Version 3 of an artifact can use format 0.1.

## 2. Files and formats

A document has three formats:

| Format | Role | Holds every field |
|---|---|---|
| JSON | The canonical format. Tools store documents and make sure that they are valid in this format. | Yes |
| YAML | The same data as JSON. The same JSON Schema applies. | Yes |
| Markdown | The format for people who read and write documents by hand. See [section 10](#10-markdown). | Yes |

A tool, for example an AI agent that helps a team write a document, writes
JSON or YAML directly and makes sure that the document is valid against the
schema. Markdown is for people. Its reader is generous (10.5), because people
write by hand. A tool does not need it.

Files use UTF-8.

### 2.1 File names

A data file has the kind of the document and the suffix `.skopos.json` or
`.skopos.yaml`, for example `intent.skopos.json`. Editors can match the schema
to a file by this suffix.

A Markdown file has the kind in capital letters, for example `INTENT.md`,
`SPEC.md`, `GLOSSARY.md` or `CRITIQUE.md`.

The team chooses the folder. Because the Markdown file holds every field, a
team can keep only the Markdown file, and let tools make the JSON when they
need it. If one folder holds two artifacts of the same kind, the team adds a
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
| `participants` | No | The people and AI agents in the session. See 3.1. |
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
decided against. An agent must not act on a rejected item.

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
| `resolution` | No | What the team decided to do. A critique with no resolution is open. |

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

## 10. Markdown

Markdown is the format for people who write documents by hand. A Markdown file
holds every field of the document, so a tool can turn a document into Markdown
and back with no loss. A person can also start a document in Markdown, with no
other file.

A reader of Markdown is generous. It accepts many ways to write the same thing
(10.5). It fills only bookkeeping fields, such as the artifact ID, and never
content. It stops with an error only when it cannot make a valid document
(10.6).

### 10.1 Layout

A Markdown file has these parts, in this order:

1. Front matter (optional): YAML between two `---` lines. It holds the fields
   of the document in [section 3](#3-the-document), with the same names and
   shapes as in JSON. `$schema` is written `schema`. The title, the summary
   and the items are not in the front matter.
2. The title, as an H1: `# <title>`.
3. The summary (optional): the text between the H1 and the first H2.
4. One H2 section for each item type that has items, with these names in this
   order: Goals, Success signals, Requirements, Stories, Constraints,
   Assumptions, Non-goals, Risks, Open questions, Decisions, Terms, Critiques.
5. The section `## From other artifacts` (optional, see 10.7).

A short example:

```markdown
---
schema: https://openskopos.org/schema/artifact/0.1.json
id: pa_7Hk2mQ9xR4tV8wZ1
version: 1
kind: intent
status: draft
id_counters:
  G: 1
  C: 1
created_at: '2026-11-05T09:00:00Z'
---

# Self-serve plan downgrades

Customers want to pay less without talking to support.

## Goals

- G1: Customers can lower their bill without waiting for support.
  - for: Org admins on monthly plans

## Constraints

- C1: Ask before adding a database migration.
  - modality: ask_first
  - approver: Tech lead
  - bring: The migration
  - bring: A rollback plan
```

### 10.2 Items

Each item is a list item:

```
- <id>: <text>
```

The fields of the item follow as nested list items, indented by two spaces:

```
  - <key>: <value>
```

A field that has fields of its own, such as an evidence entry, has them as
list items indented by two more spaces.

The `detail` of an item comes after a blank line, indented by two spaces. It
continues until the next item. It can hold any Markdown, also lists.

```markdown
- FR3: The seat count never goes below zero.
  - priority: should

  An invariant. A property-based test fits it.
```

### 10.3 Where each field goes

| Field | In Markdown |
|---|---|
| `id`, `text` | The item line: `- FR1: The billing page lets an admin pick a plan.` |
| `type` | The H2 section of the item. |
| `status` | `- status: proposed` or `- status: rejected`. An item with no status line is accepted. |
| A field with one value | `- <field>: <value>`, for example `- priority: must` or `- deferred: true`. |
| A field with a list of strings | One line for each value, with these keys: `ac` (`acceptance_criteria`), `scope`, `bring`, `option`, `alternative`, `consequence`, `alias`, `code_name`. |
| `examples` | One line for each example. The key is the kind: `example` (no kind), `normal`, `edge` or `forbidden`. |
| `target` | `- target: <item>` for an item, or `- target: <section name>` for a section. For both, the section goes on a nested `- section:` line. |
| `resolution` | `- resolution: <outcome>`, with nested `- note:` and `- decided_by:` lines. |
| `refs` | One line for each ref: `- <rel>: <item>`, for example `- refines: FR1`. A ref with no `rel` uses `- ref: <item>`. |
| `evidence` | `- evidence:`, with one nested entry for each piece of evidence (see below). |
| `proposed_by` | `- proposed_by: P3` |
| `ratification` | `- votes: 2 yes, 0 no, 2 eligible` |
| `x-` keys | `- x-<name>: <value>`. A value that is not a string is written as JSON. |
| `detail` | Paragraphs after a blank line (10.2). |

In this table, `<item>` is an item ID, such as `FR1`, or a reference string to
an item in another artifact, such as `pa_7Hk2mQ9xR4tV8wZ1@2#FR1`.

Evidence entries:

```markdown
  - evidence:
    - quote: Retry each failed charge up to three times.
      - from: Payment retry spec
      - location: Retry policy
    - code: lib/billing/retry.ex
      - repo: acme/billing
      - lines: 41-48
      - commit: a1b2c3d
    - discussion: Support finds out from the customer today.
      - speaker: P2
    - link: https://example.com/incidents/42
      - title: Incident 42
```

The `from` of a quote is a title, or a reference string to an item. If it has
both, the title goes on a nested `- title:` line.

### 10.4 Writing Markdown

A writer uses the layout in 10.1 to 10.3:

- It writes the front matter fields in the order of the table in
  [section 3](#3-the-document), then the `x-` keys.
- It writes the status line of an item first. Then it writes the fields of the
  type in the order of the tables in [section 4](#4-items), then refs,
  evidence, `proposed_by`, votes and `x-` keys, and then the detail.
- In each section, the items are in the order of `items` in the document.
- It replaces a line break in a text or a value with one space.
- It leaves out a section with no items.
- It puts a blank line between two items only when one of them has a detail.

Markdown does not keep the order of items between types. A reader puts the
items in the order of the sections. Two documents that differ only in this
order are the same.

### 10.5 Reading Markdown

A reader accepts all of the following. These rules exist so that writing by
hand does not turn into a list of errors.

Front matter and the file:

- The front matter is optional. If a bookkeeping field is not there, the
  reader fills it: `schema` with the URL of format 0.1, `id` with a new
  artifact ID, `version` with 1, `status` with `draft` and `created_at` with
  the current time.
- If there is no `kind`, the reader takes it from a file name in capital
  letters: `INTENT.md` gives `intent`.
- The reader accepts `schema` and `$schema`, and a `title` in the front matter
  if the file has no H1.
- If a YAML value has the wrong type for its field, and the change is exact,
  the reader changes it. For example, a session ID of `4907` becomes `"4907"`,
  and a YAML date becomes its text.

Sections:

- The reader ignores case, `-` and `_`, singular and plural, and words in
  parentheses. `## Success Signals (S)` and `## success signal` are the same
  section. The type name, such as `open_question`, also works.
- The reader also accepts these names: `Out of scope` (Non-goals), `User
  stories` (Stories), `Functional requirements` (Requirements), `Questions`
  (Open questions), `Glossary` (Terms), `Findings` (Critiques), `Rules` and
  `Boundaries` (Constraints), `Success criteria` (Success signals).
- H3 and smaller headings inside a section only group items. The reader
  ignores them.

Items:

- An item can start with `-`, `*`, `+`, `1.` or `1)`.
- An item ID can be written `G1:`, `G1.`, `G1 -`, `**G1**:` or `[G1]`, in
  capital or small letters. An item can also have no ID, or start with
  `new:`.
- An indented line straight after the item line, which is not a list item,
  continues the text of the item.

Fields:

- The reader ignores case in keys, and treats spaces, `-` and `_` as the same.
  It accepts the singular and the plural of a key, for example `option` and
  `options`.
- It also accepts these keys: `acceptance criteria` and `criterion` for `ac`,
  `also called` for `alias`, `in code` for `code_name`, `by` for
  `proposed_by`, `due` for `confirm_by`, `vote` for `votes`, `related to` for
  `relates_to`, `said` for `discussion` and `url` for `link`.
- A list field can repeat its key, or have the values as nested list items
  under one key line.
- An example can be nested under an `examples` line. A nested example can
  start with its kind, for example `- edge: 12 seats in use.`
- The reader ignores case, spaces and `-` in a value from a fixed list:
  `Ask first` gives `ask_first`.
- A boolean can be `true`, `false`, `yes` or `no`.
- Code evidence can give its lines in the path, for example
  `- code: lib/billing/retry.ex:41-48`.
- Votes can also be nested lines: `- yes: 2`, `- no: 0`, `- eligible: 2`.

Item IDs:

1. The reader keeps each written ID that has the prefix of its section, if it
   is the first item with that ID.
2. Each counter starts at the higher of its value in `id_counters` and the
   highest written ID of its prefix.
3. Each other item gets the next ID from the counter of its prefix, in the
   order of the file (see [section 7](#7-id-counters)).
4. The reader writes `id_counters` if the document has items.

### 10.6 Warnings and errors

The reader reports a warning and continues in these cases:

| Case | What the reader does |
|---|---|
| A written ID has the prefix of another type. | The item gets a new ID. |
| A written ID appears a second time. | The second item gets a new ID. |
| An unknown key in the front matter or in an item. | It keeps the value as an `x-` key, for example `x-colour`. |
| An unknown H2 section. | It ignores the section. |
| Text that is not in an item. | It ignores the text. |
| A nested list item with no key. | It ignores the line. |

The reader reports an error and makes no document in these cases. Each error
names the line and says what to write:

- The front matter is not valid YAML, or it has no closing `---`.
- The file has no title.
- The reader cannot find the kind.
- A value is not in the fixed list of its field, or a boolean is not `true`
  or `false`.
- A ref is not an item ID or a reference string.
- A field with one value appears twice in an item.
- An evidence entry has an unknown kind, or an evidence entry, a target or a
  resolution has an unknown key.
- Code lines are not a number or a range such as `41-48`.
- A resolution has no outcome, or the votes do not give yes, no and eligible.
- The document is not valid against the schema or the rules in
  [section 9](#9-rules-that-the-schema-cannot-check). For example, a
  constraint has no modality, or a critique has no evidence.

### 10.7 Material from other artifacts

A writer can copy material from other artifacts into a Markdown file, so that
an agent that reads the file does not need other files. A reader ignores this
material: it reads nothing from `## From other artifacts` to the next H2 or
the end of the file.

If the writer has the documents, it copies:

1. The accepted terms of each `glossary` artifact that the document has as an
   `input` parent.
2. The accepted constraints of each `agent_rules` artifact that the document
   has as an `input` parent.
3. Each item in another artifact that an item of the document refers to with a
   ref that has `artifact` and `version`. It copies each item one time only.

The material has one H3 for each source artifact and item type:

```
### <section name> from <title> (<artifact id>@<version>)
```

The H3 headings for items 1 and 2 come first, in the order of the parents.
The H3 headings for item 3 follow, in the order of the item types. Each copied
item has the layout of 10.2, with the reference string in place of the ID:

```markdown
### Terms from Billing glossary (pa_9Lp4Wd7Ks2Hq6Nb3@1)

- pa_9Lp4Wd7Ks2Hq6Nb3@1#T1: Seat
  - definition: One user who can sign in to an org.
  - code_name: Billing.Seat
```

A copy is a snapshot. If the glossary changes, the file shows the old terms
until a tool writes it again. To change a copied item, change it in its own
artifact.

### 10.8 Editing a stored version

A tool that stores documents can read an edited Markdown file of a stored
version. The content of a document is all of it except `version`, `status`,
`parents`, `created_at`, `finalized_at`, `method` and `session`.

- If the stored version is final and the content did not change, nothing
  changes.
- If the stored version is final and the content changed, the tool stores
  version n+1 as a draft (see [section 8](#8-versions-and-parents)). Its first
  parent is a `previous_version` parent for version n. The other parents come
  from the file. It has no `finalized_at`, `method` or `session`, because
  these describe how version n was made. `created_at` is the current time.
- If the stored version is a draft, the document from the file replaces the
  draft. It keeps the version number.

## 11. Glossaries

A glossary is an artifact of kind `glossary`. Its items are terms (`T`).

- The team sets the scope of a glossary in its title and summary: a company, a
  project, or one area of a project. The format does not model a hierarchy of
  glossaries.
- An artifact can pin several glossaries as `input` parents. Each parent names
  the version of the glossary that the artifact used.
- If two pinned glossaries define the same term in different ways, tools show
  the conflict. They do not choose a winner.
- A Markdown file can copy the accepted terms of the pinned glossaries (see
  10.7), so that an agent knows what each word means.

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
- The maintainers publish a schema version with the git tag
  `schema-<version>`, for example `schema-0.1`. The site then serves the file
  as it was at the tag. Before the tag exists, the version is a draft and can
  change.
- A published schema file never changes. CI fails if it changes. A change
  publishes a new file with a new version.

The schema for 0.1 accepts any 0.y URL in `$schema`, so that a 0.1 tool can
read a document of a later 0.y version and report what it does not know.

## 13. Conformance

The JSON Schema and the conformance cases in [`conformance/`](../../conformance/)
define correct behavior. An implementation in any language that passes all
cases is correct. A tool that checks documents must:

- Accept every document in `conformance/valid/`.
- Reject every document in `conformance/invalid/`, for the reason in its index
  entry.

A tool that reads or writes Markdown must also pass the cases in
`conformance/markdown/`. Two documents are equal if they are equal after the
order of keys and the order of items between types are ignored.

- `round-trip/`: reading `document.md` gives `document.json`. Writing
  `document.json` and reading the result gives `document.json` again. A writer
  does not need to make the same bytes as `document.md`.
- `generous/`: reading the hand-written file gives `expected.json`. If the file
  has no `id` or no `created_at`, the comparison ignores that field.
- `next-version/`: reading `edited.md` against the stored version
  `stored.json` (10.8) gives `expected.json`. If the version number changes,
  the comparison ignores `created_at`.
- `invalid/`: the reader reports an error.

The index of the Markdown cases gives the number of warnings for each generous
case. A reader can word its warnings in its own way.
