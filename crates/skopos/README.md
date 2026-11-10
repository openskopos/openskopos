# skopos

`skopos` is the command line tool for [OpenSkopos](https://github.com/openskopos/openskopos)
documents. It reads and writes files. The library
[`skopos-core`](../skopos-core/README.md) does the other work.

## Install

To install the tool from a clone of the repository, run this command from the
root of the repository:

```bash
cargo install --locked --path crates/skopos
```

## Commands

A file name must end with `.json`, `.yaml`, `.yml` or `.md`. A `.md` file is
Markdown, in the layout of section 10 of the specification. `check`, `convert`
and `import` read Markdown. `fmt` and `ids` work on JSON and YAML only.

The reader of Markdown accepts many ways to write the same thing, so that a
file written by hand does not give a list of errors. It fills only bookkeeping
fields, such as the artifact ID. If the front matter has no `kind`, the reader
takes the kind from a file name in capital letters: `INTENT.md` gives
`intent`. The reader writes a warning to standard error for each thing that
it skips or changes, for example:

```
SPEC.md: warning: line 16: FR5 appears twice, so the second item gets a new ID
```

### check

`skopos check` finds the problems in one or more documents. It checks each
document against the JSON Schema and against the rules in section 9 of the
specification.

```bash
skopos check intent.skopos.json spec.skopos.yaml
```

The tool writes one line for each problem:

```
spec.skopos.yaml: /items/2/id: the item ID S1 is also the ID of /items/1 [unique-item-ids]
```

The line has the file, the place in the document as a JSON Pointer, and the
message. A problem with a rule of section 9 ends with the name of the rule.
If a document has a problem, the exit code is 1.

For a Markdown file, the message starts with the line in the file:

```
INTENT.md: line 6: priority 'urgent' is not one of: must, should, could
INTENT.md: /items/0: line 5: "modality" is a required property
```

With `--json`, the tool writes a JSON list with one entry for each file:

```json
[
  {
    "file": "spec.skopos.yaml",
    "problems": [
      {
        "path": "/items/2/id",
        "rule": "unique-item-ids",
        "message": "the item ID S1 is also the ID of /items/1"
      }
    ]
  }
]
```

`rule` is `null` for a schema problem.

### fmt

`skopos fmt` writes each file again in the canonical layout. Each file keeps
its format. The keys of the document are in the order of section 3 of the
specification. The keys of each item are in this order:

1. `id`, `type`, `text` and `status`.
2. The fields of the item type, in the order of section 4.
3. `refs`, `evidence`, `proposed_by`, `ratification` and `detail`.
4. The `x-` keys.

JSON has an indent of two spaces. YAML puts dates, times and the keys `"yes"`
and `"no"` in quotes, as section 2.2 of the specification requires. The tool
does not keep comments in YAML files.

```bash
skopos fmt intent.skopos.json
```

With `--check`, the tool changes no files. It writes the name of each file
that is not in the canonical layout. If there is one, the exit code is 1.

### convert

`skopos convert` writes a document in another format: `json`, `yaml` or
`markdown` (or `md`). The data does not change.

```bash
skopos convert --to yaml intent.skopos.json -o intent.skopos.yaml
skopos convert --to md intent.skopos.json -o INTENT.md
skopos convert --to json INTENT.md -o intent.skopos.json
```

Without `-o`, the document goes to standard output.

A Markdown file holds every field of the document. Markdown does not keep the
order of items between types: the items are in the order of the sections.

A Markdown file can also hold material from other artifacts, so that an agent
that reads the file does not need other files (section 10.7 of the
specification). To copy this material, give each other document with
`--input`:

```bash
skopos convert --to md spec.skopos.json --input glossary.skopos.json -o SPEC.md
```

The tool copies the accepted terms of each pinned glossary, the accepted
constraints of each pinned `agent_rules` artifact, and each item that a ref of
the document points to. A reader ignores this material.

### import

`skopos import` reads an edited Markdown file of a stored version. It writes
the document to store, as section 10.8 of the specification says:

- If the stored version is final and the content did not change, the result
  is the stored version.
- If the stored version is final and the content changed, the result is the
  next version as a draft. Its first parent is a `previous_version` parent for
  the stored version.
- If the stored version is a draft, the file replaces it. The draft keeps its
  version number.

```bash
skopos import INTENT.md --stored intent.skopos.json -o intent.skopos.v3.json
```

The tool writes what it did to standard error, for example
`INTENT.md: new draft version 3`. Without `-o`, the document goes to standard
output, in the format of the stored version. The result always has the
artifact ID of the stored version.

### ids

`skopos ids` gives an ID to each item that has none, and updates
`id_counters`. An item has no ID if its `id` is absent, `null` or empty. The
tool takes the prefix from the `type` of the item and the number from the
counter, as section 7 of the specification says. A counter that is less than
the highest ID of its prefix goes up to that ID first.

To add an item by hand, leave out its `id` and run the tool:

```yaml
items:
- type: constraint
  text: Ask before adding a database migration.
  status: proposed
  modality: ask_first
  approver: Tech lead
```

```bash
skopos ids intent.skopos.yaml
```

The tool writes each change, for example
`intent.skopos.yaml: /items/0 gets C4, id_counters changes`. It writes a
changed file in the canonical layout.

With `--check`, the tool changes no files. It names each file that needs a
change. If there is one, the exit code is 1.

### export

`skopos export` writes the files of a spec-driven development tool from a
document: [OpenSpec](https://github.com/Fission-AI/OpenSpec),
[Kiro](https://kiro.dev) or [Spec Kit](https://github.com/github/spec-kit).
Teams can keep what they agreed in OpenSkopos and give it to the tool that
their agents already use.

```bash
skopos export spec.skopos.json --to openspec
skopos export spec.skopos.json --to kiro --name seat-limits
skopos export spec.skopos.json --to speckit -o ../billing
```

| Tool | Files |
|---|---|
| `openspec` | `openspec/changes/<name>/`: `.openspec.yaml`, `proposal.md`, `design.md` and `specs/<name>/spec.md` |
| `kiro` | `.kiro/specs/<name>/`: `requirements.md` and `design.md` |
| `speckit` | `specs/<NNN>-<name>/`: `spec.md`, `plan.md` and `research.md` |

- `-o` is the root folder of the project. The default is the current folder.
- `--name` is the name of the change or feature. Without it, the name comes
  from the title of the document.
- For Spec Kit, `--number` sets the feature number. Without it, the tool uses
  the number of a folder in `specs/` with the same name, or the next free
  number.
- The tool does not write over a file that exists. Use `--force` to write
  over it. With `--force`, the tool also removes the files of an earlier
  export that this export does not write, so that no old items stay behind.
- `--dry-run` names the files and writes nothing.
- For Spec Kit, the tool names the `.specify/feature.json` setting that makes
  Spec Kit work on the exported feature. It does not change that file.

The tool exports only accepted items. Each exported line ends with the
reference string of its item, for example `pa_2Xc5Rv8Tn1Mb4Kz7@3#FR4`. Items
that have no place in the tool, and items that are proposed or rejected, are
in a "Not exported" note in the files. The tool also writes the list to
standard error. [docs/exporters.md](../../docs/exporters.md) explains where
each item type goes.

The document must be valid. If `skopos check` finds a problem, the tool
exports nothing.

### diff

`skopos diff` compares two versions of an artifact, item by item. The item ID
keeps its meaning in every version, so the tool compares the items with the
same ID.

```bash
skopos diff spec-v2.skopos.json spec-v3.skopos.json
```

```
pa_2Xc5Rv8Tn1Mb4Kz7@2 -> pa_2Xc5Rv8Tn1Mb4Kz7@3
envelope   summary
added      FR4  A plan change that leaves more seats in use than the new limit is blocked.
changed    A1   confirmation, evidence, text
changed    Q1   status (status proposed -> rejected)
superseded FR2  by FR4
dropped    N2   Emails to members above the limit.
```

| Line | Meaning |
|---|---|
| `added` | The ID is in the new version only. |
| `changed` | The ID is in both versions, and the fields after it are different. A change of display order, of items or of `refs`, is not a change. |
| `superseded` | The ID is in the old version only, and an item of the new version links to it with `supersedes`. A rejected item supersedes nothing. |
| `dropped` | The ID is in the old version only, and no item supersedes it. |
| `envelope` | Fields of the document that are different, such as `title` or `summary`. The tool does not compare `version`, `status`, `parents`, `created_at`, `finalized_at`, `method`, `session` and `id_counters`. |

With `--json`, the tool writes the difference as JSON. The exit code is 0 if
both documents are valid, also if they are different.

## License

[Apache License 2.0](../../LICENSE)
