---
name: openskopos
description: Record a team's decisions in the OpenSkopos format (.skopos.json, .skopos.yaml, INTENT.md, SPEC.md, GLOSSARY.md, CRITIQUE.md) and use the skopos command line tool to check, fix, version, compare, merge and export them. Use when the user asks to write down goals, requirements, constraints, assumptions, risks, open questions, decisions, terms or critiques, to update or merge OpenSkopos documents, or to hand them to OpenSpec, Kiro or Spec Kit.
license: Apache-2.0
---

# OpenSkopos

OpenSkopos is a file format for the decisions that a team makes before an AI
agent builds the work. A document has typed items. Each item has a stable ID,
such as `FR3`, that people, commits and agents cite. The full specification is
at https://openskopos.org/spec/0.2/index.md. This skill covers format 0.2.

## Rules

- Record what the user and the team decided. Do not add your own ideas as
  accepted items. Give an item you propose the status `proposed`, and let the
  user accept or reject it.
- Never choose an item ID or an artifact ID yourself. Leave IDs out and let
  `skopos` assign them (see "Write a new document").
- An ID belongs to one item forever and is never used again. If the meaning of
  an item changes, remove it and add a new item with a `supersedes` ref to it.
- Keep rejected items. They show what the team decided against. Never act on a
  rejected item.
- Do not write email addresses, account IDs or other personal identifiers.
  Participants have only `id` (`P1`), `name` and `kind` (`human` or `ai`).
- A `final` version never changes. To change it, make the next version.
- Run `skopos check` after every edit, and fix every problem before you stop.

## Get the tool

Run `skopos --version`. If it prints a version, use it. If not, pick the first
way that fits the machine (check with `uname -sm`, `brew --version`,
`cargo --version` or `node --version`), tell the user what you will install,
and ask before you install it. If you cannot install anything, see "Without
the command line tool".

1. macOS or Linux with Homebrew: `brew install openskopos/tap/skopos`
2. macOS or Linux: the install script. It puts `skopos` in `~/.cargo/bin`.
   If that folder is not on `PATH`, run `~/.cargo/bin/skopos`.

   ```bash
   curl --proto '=https' --tlsv1.2 -LsSf https://github.com/openskopos/openskopos/releases/latest/download/skopos-installer.sh | sh
   ```

3. Windows: the install script, in PowerShell.

   ```powershell
   powershell -ExecutionPolicy Bypass -c "irm https://github.com/openskopos/openskopos/releases/latest/download/skopos-installer.ps1 | iex"
   ```

4. A Rust toolchain: `cargo install --locked skopos`
5. Node 20 or later: `npm install --global @openskopos/cli`, or use
   `npx @openskopos/cli` in place of `skopos` with nothing installed, for
   example `npx @openskopos/cli check intent.skopos.yaml`. It has the same
   commands and output.

If scripts and package managers are not allowed, download the archive for
the platform from
`https://github.com/openskopos/openskopos/releases/latest/download/<archive>`,
check it against `<archive>.sha256`, and extract it. A `.tar.xz` has the
binary in a folder, such as `skopos-x86_64-unknown-linux-gnu/skopos`. The
`.zip` has `skopos.exe`.

| Platform | Archive |
|---|---|
| macOS, Apple silicon | `skopos-aarch64-apple-darwin.tar.xz` |
| macOS, Intel | `skopos-x86_64-apple-darwin.tar.xz` |
| Linux, x86-64 | `skopos-x86_64-unknown-linux-gnu.tar.xz` |
| Linux, x86-64, static (musl) | `skopos-x86_64-unknown-linux-musl.tar.xz` |
| Linux, ARM64 | `skopos-aarch64-unknown-linux-gnu.tar.xz` |
| Windows, x86-64 | `skopos-x86_64-pc-windows-msvc.zip` |

More: https://openskopos.org/docs/install/index.md

## Files

| Kind | Content | Data file | Markdown file |
|---|---|---|---|
| `intent` | Why the work exists and how the team knows it is done | `intent.skopos.json` or `.yaml` | `INTENT.md` |
| `spec` | The behavior that the work must have | `spec.skopos.json` or `.yaml` | `SPEC.md` |
| `glossary` | Terms with agreed meanings | `glossary.skopos.json` or `.yaml` | `GLOSSARY.md` |
| `critique` | Problems found in an input | `critique.skopos.json` or `.yaml` | `CRITIQUE.md` |
| `agent_rules` | Rules for agents in a project | `agent_rules.skopos.json` or `.yaml` | A section of `AGENTS.md` |

JSON is the canonical format. YAML has the same data. Markdown has every field
too, and is easiest for people to read and edit. With two artifacts of one kind
in a folder, add a name: `billing.spec.skopos.yaml`. In YAML, put dates and
times in quotes.

## Item types

Every item has `id`, `type`, `text` (one plain statement), `status`
(`proposed`, `accepted` or `rejected`), and can have `detail` (Markdown),
`refs`, `evidence`, `proposed_by` (a participant ID) and `x-` keys. A field of
one type is not allowed on another type.

| Prefix | `type` | Use it for | Its own fields |
|---|---|---|---|
| `G` | `goal` | Why the work exists | `for` |
| `S` | `success_signal` | How the team knows a goal is met | `verification` |
| `FR` | `requirement` | A behavior the system must have, or an invariant | `priority` (`must`, `should`, `could`), `examples` |
| `ST` | `story` | A requirement from the view of a user | `acceptance_criteria`, `examples` |
| `C` | `constraint` | A rule about how to do the work | `modality` (required), `rationale`, `scope`, `enforced_by`, `approver`, `bring` |
| `A` | `assumption` | Taken as true without a check | `confirmation` (`unconfirmed`, `confirmed`, `refuted`), `owner`, `confirm_by` (a date) |
| `N` | `non_goal` | What the work will not do | `reason`, `deferred` (true or false) |
| `R` | `risk` | A problem that can occur | `likelihood`, `impact` (`low`, `medium`, `high`), `mitigation`, `owner` |
| `Q` | `open_question` | A question without an answer yet | `owner`, `blocking`, `options`, `answer` |
| `D` | `decision` | A choice that the team made | `rationale`, `alternatives`, `consequences`, `decided_by` |
| `T` | `term` | A word with an agreed meaning (the word is the `text`) | `definition` (required), `aliases`, `code_names` |
| `CR` | `critique` | A problem found in an input or released work | `severity` and `category` and `evidence` (required), `suggestion`, `target`, `resolution` |

- `modality` is `always`, `never`, `prefer`, `avoid` or `ask_first`. Only an
  `ask_first` constraint has `approver` (required for it: a role or a
  participant ID) and `bring` (what to show the approver).
- `severity` is `blocker`, `major`, `minor` or `note`. `category` is
  `ambiguity`, `gap`, `conflict`, `untestable`, `incorrect`,
  `unstated_assumption`, `scope_creep`, `over_specified` or `other`.
- `owner`, `decided_by` and `proposed_by` are participant IDs, such as `P2`.
- An example has `text` and an optional `kind`: `normal`, `edge` or
  `forbidden`.

A ref links to another item: `item`, an optional `rel`, and `artifact` and
`version` for an item in another artifact. The values of `rel` are `refines`,
`derived_from`, `depends_on`, `threatens`, `mitigates`, `resolves`,
`verifies`, `supersedes`, `contradicts` and `relates_to`. For example, `S1`
`verifies` `G1`, `ST1` `refines` `FR1`, `R1` `threatens` `FR1`, and `D1`
`resolves` `Q1`.

Evidence entries have a `kind`: `quote` (`text`, optional `from`,
`location`), `code` (`path`, `line_start`, optional `repo`, `line_end`,
`commit`, `snippet`), `discussion` (`text`, optional `speaker`) or `link`
(`uri`, optional `title`). Remove secrets from a `snippet`.

Outside the document, cite an item with a reference string:
`<artifact id>@<version>#<item id>`, such as `pa_7Hk2mQ9xR4tV8wZ1@2#FR3`.

## Write a new document

Start in Markdown with no front matter and no IDs. The reader adds the
artifact ID, the version, the status, `created_at` and the item IDs.

```markdown
# Self-serve plan downgrades

Customers want to pay less without talking to support.

## Goals

- Customers can lower their bill without waiting for support.
  - for: Org admins on monthly plans

## Success signals

- Support gets fewer downgrade tickets.
  - verification: Count downgrade tickets per month in the help desk.
  - verifies: G1

## Constraints

- Ask before adding a database migration.
  - modality: ask_first
  - approver: Tech lead
  - bring: The migration

## Open questions

- What happens to seats above the new limit?
  - status: proposed
  - blocking: true
  - option: Block the change
  - option: Remove the newest seats
```

- The H2 sections, in this order: Goals, Success signals, Requirements,
  Stories, Constraints, Assumptions, Non-goals, Risks, Open questions,
  Decisions, Terms, Critiques.
- An item with no status line is accepted. Write `- status: proposed` on each
  item that the user has not agreed to.
- A list field has one line per value with a singular key: `ac`, `scope`,
  `bring`, `option`, `alternative`, `consequence`, `alias`, `code_name`.
  Examples use `example`, `normal`, `edge` or `forbidden` as the key.
- A ref is `- <rel>: <item>`, such as `- refines: FR1`. Refs to items in the
  same file use the IDs that the reader will assign: the first goal is `G1`,
  and so on in file order. Check them in the result.
- Evidence goes under `- evidence:`, one nested entry per piece, such as
  `- quote: ...`, `- code: lib/billing/retry.ex:41-48`, `- discussion: ...` or
  `- link: https://...`.
- Detail paragraphs go after a blank line, indented by two spaces.

The kind comes from the file name (`INTENT.md` is `intent`). Check the file,
then make the data file:

```bash
skopos check INTENT.md
skopos convert --to yaml INTENT.md -o intent.skopos.yaml
skopos check intent.skopos.yaml
```

A team can keep only the Markdown file. Keep the data file when other tools
read it, or when the work will have more versions.

## Add items to a data file

Add the item without `id`, then let the tool assign the ID and update
`id_counters`:

```yaml
- type: risk
  text: Admins downgrade by mistake.
  status: proposed
  likelihood: medium
  refs:
  - item: G1
    rel: threatens
```

```bash
skopos ids intent.skopos.yaml
```

## Check and fix

```bash
skopos check intent.skopos.yaml SPEC.md
```

Each problem is one line (file, JSON Pointer, message, and a rule name for
rules that the schema cannot check), and a hint line under it. The exit code
is 1 while a problem is left. `--json` writes the problems with hints and, for
some, a fix as a JSON Patch.

```bash
skopos check --fix intent.skopos.yaml
```

`--fix` applies the fixes that need no choice: a missing item ID, a counter
that is too low, a duplicate ID, a value such as `Accepted` or `ask first`,
and a missing `$schema` or status (an item gets `proposed`). It rewrites JSON
and YAML files in the canonical layout and names each fix. It does not change
Markdown files. Fix the problems that are left yourself, from the hints. If a
fix needs a decision, such as which modality a constraint has, ask the user.

`skopos fmt FILE` writes a JSON or YAML file in the canonical key order.
`skopos fmt --check FILE` only reports.

## Versions

A version is `draft` or `final`. To finalize a draft, set `status: final` and
add `finalized_at` (an RFC 3339 time, such as `"2026-11-25T16:00:00Z"`), then
run `skopos check`. Finalize only when the user says so, and not while a merge
conflict is open.

To change a final version, edit its Markdown and import it:

```bash
skopos convert --to md intent.skopos.yaml -o INTENT.md
# edit INTENT.md: new items without IDs, or "- new: <text>"
skopos import INTENT.md --stored intent.skopos.yaml -o intent.v2.skopos.yaml
```

If the content changed, the result is version n+1 as a draft, with a
`previous_version` parent. If the stored version is a draft, the file replaces
it and keeps its number. To make the next version by hand: copy the file, add
1 to `version`, set `status: draft`, remove `finalized_at`, `method` and
`session`, set a new `created_at`, and make the first parent
`{rel: previous_version, artifact: <same id>, version: <n>}`. Keep every ID and
`id_counters`.

To replace an item, add the new item with a ref `supersedes: <old id>`, and
remove the old item. To answer an open question, add a decision that
`resolves` it, and set the question's `answer`.

Read what changed and what is in force now:

```bash
skopos diff intent.skopos.yaml intent.v2.skopos.yaml
skopos history intent*.skopos.yaml
skopos trace Q1 intent*.skopos.yaml
```

## Merge several documents into one

Use merge for two drafts of the same work, such as two sessions or two agent
runs from the same input. Pick one document as main. The result is the next
version of main, as a draft, with the other document as an `input` parent.

```bash
skopos merge main.skopos.json other.skopos.json --base v1.skopos.json -o merged.skopos.json
```

- With `--base` (the common earlier version), items pair by ID, and a change
  that only one side made merges by itself.
- Without `--base`, items of the same type with similar text pair. Read each
  `paired` line of the summary. If a pair is wrong, merge again with
  `--pair MAIN_ID=OTHER_ID` for each right pair. Then no other items pair.
- A new item from the other document gets a new ID from main's counters and a
  `derived_from` ref to where it came from.
- For three or more documents, merge them one at a time into the result:

```bash
skopos merge a.skopos.json b.skopos.json -o merged.skopos.json
skopos merge merged.skopos.json c.skopos.json -o merged.skopos.json
```

A conflict keeps both items as `proposed`, with a `contradicts` ref from the
other item to main's item, so the file is always valid. List the open
conflicts (exit code 1 while one is open):

```bash
skopos conflicts merged.skopos.json
```

Show each conflict to the user, and let them choose. Then accept their choice;
the other item becomes `rejected` and stays in the record:

```bash
skopos resolve merged.skopos.json FR3
skopos check merged.skopos.json
```

## Give the decisions to an agent or a tool

```bash
skopos convert --to md spec.skopos.json --input glossary.skopos.json -o SPEC.md
skopos export spec.skopos.json --to openspec    # or kiro, speckit
skopos render spec.skopos.json                  # text; --html -o spec.html for a page
```

`--input` copies the accepted terms of pinned glossaries, the accepted rules of
pinned `agent_rules` artifacts and the items that refs point to into the
Markdown file. `export` writes only accepted items, and lists the others. Use
`--dry-run` to see the files first. It does not overwrite files without
`--force`.

## Without the command line tool

Use the CLI when you can: it works on files on disk and has no limits. If you
cannot install it or run a shell, use the OpenSkopos MCP server or the browser
tools.

### MCP server

`https://mcp.openskopos.org/mcp` is a remote MCP server (Streamable HTTP). It
needs no account and stores no documents. If its tools are not in your tool
list, give the user the command for their agent:

```bash
claude mcp add --transport http openskopos https://mcp.openskopos.org/mcp   # Claude Code
codex mcp add openskopos --url https://mcp.openskopos.org/mcp               # Codex
amp mcp add openskopos https://mcp.openskopos.org/mcp                       # Amp
```

Other agents: https://mcp.openskopos.org/

Its tools are `check`, `fix`, `convert`, `assign_ids`, `render`, `diff`,
`history`, `trace`, `merge`, `conflicts`, `resolve`, `export` and `spec`. They
do the work of the CLI commands with the same names (`assign_ids` is
`skopos ids`, `fix` is `skopos check --fix`). You pass each document as text
(JSON, YAML or Markdown), and the tool returns the result as text. Write the
result to the file yourself.

Limits: 1 MB per request, 500 items per document (200 for `merge`), 20
documents for `history` and `trace`, and per IP 30 tool calls a minute and
1,000 a day. When a call fails because of a limit, tell the user and use the
CLI.

### Browser tools

https://openskopos.org/tools/ runs the same checks in the browser:
Validate (check, see problems, convert), Author (write a document), Compare,
History, Merge (two drafts and their conflicts) and Agent file (one Markdown
file for an agent, or an export). The JSON Schema is at
https://openskopos.org/schema/artifact/0.2.json for editors.
