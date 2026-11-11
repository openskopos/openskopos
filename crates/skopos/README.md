# skopos

`skopos` is the command line tool for [OpenSkopos](https://github.com/openskopos/openskopos)
documents. The tool reads and writes the files. The library
[`skopos-core`](../skopos-core/README.md) does all other work.

The commands are in the library of this crate (`src/lib.rs`). They read and
write the files and the terminal through the trait `Host`. The `skopos`
binary implements `Host` with the Rust standard library. The WebAssembly build
in [`skopos-wasm`](../skopos-wasm/README.md) implements `Host` with Node. Thus
the npm package `@openskopos/cli` has the same commands, output and exit
codes.

## Install

If you use Homebrew on macOS or Linux, enter this command:

```bash
brew install openskopos/tap/skopos
```

If you have a Rust toolchain, install the tool from crates.io:

```bash
cargo install --locked skopos
```

[docs/install.md](../../docs/install.md) has the install scripts for the
shell and PowerShell, and the binaries for each platform. To install the tool
from a clone of the repository, enter
`cargo install --locked --path crates/skopos` in the root folder of the
repository.

The npm package [`@openskopos/cli`](../../npm/cli/README.md) has the same
tool for Node. You can use it without an installation:
`npx @openskopos/cli check FILE`.

## Commands

A file name must end with `.json`, `.yaml`, `.yml` or `.md`. A `.md` file is
Markdown, with the layout of section 10 of the specification. The commands
`check`, `convert` and `import` read Markdown. The commands `fmt` and `ids`
read only JSON and YAML.

The Markdown reader accepts many different ways to write the same data. Thus
a file that a person writes by hand does not cause many errors. The reader
adds only bookkeeping fields, for example the artifact ID. If the front
matter has no `kind`, the reader gets the kind from the file name. The file
name must be in capital letters. For example, the kind of `INTENT.md` is
`intent`. For each change that the reader makes, and for each part of the
file that it ignores, it writes a warning to standard error:

```
SPEC.md: warning: line 16: FR5 appears twice, so the second item gets a new ID
```

### check

`skopos check` finds the problems in one or more documents. It compares each
document with the JSON Schema and with the rules in section 9 of the
specification.

```bash
skopos check intent.skopos.json spec.skopos.yaml
```

The tool writes one line for each problem:

```
spec.skopos.yaml: /items/2/id: the item ID S1 is also the ID of /items/1 [unique-item-ids]
```

The line has the file name, the location in the document as a JSON Pointer,
and the message. If the problem is with a rule of section 9, the line ends
with the name of the rule. If a document has a problem, the exit code is 1.

For a Markdown file, the message starts with the line number in the file:

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

For a schema problem, `rule` is `null`.

### fmt

`skopos fmt` writes each file again with the canonical layout. Each file
keeps its format. The keys of the document are in the order of section 3 of
the specification. The keys of each item are in this order:

1. `id`, `type`, `text` and `status`.
2. The fields of the item type, in the order of section 4.
3. `refs`, `evidence`, `proposed_by`, `ratification` and `detail`.
4. The `x-` keys.

In JSON, the indent is two spaces. In YAML, dates, times and the keys `"yes"`
and `"no"` are in quotes, as section 2.2 of the specification requires. The
tool removes comments from YAML files.

```bash
skopos fmt intent.skopos.json
```

With `--check`, the tool does not change files. It writes the name of each
file that does not have the canonical layout. If one or more files do not
have the canonical layout, the exit code is 1.

### convert

`skopos convert` writes a document in a different format: `json`, `yaml` or
`markdown` (or `md`). The data does not change.

```bash
skopos convert --to yaml intent.skopos.json -o intent.skopos.yaml
skopos convert --to md intent.skopos.json -o INTENT.md
skopos convert --to json INTENT.md -o intent.skopos.json
```

Without `-o`, the tool writes the document to standard output.

A Markdown file holds all fields of the document. Markdown does not keep the
order of items that have different types. In Markdown, the items are in the
order of the sections.

A Markdown file can also hold data from other artifacts. Thus an agent that
reads the file does not need the other files (section 10.7 of the
specification). To copy this data, add each of the other documents with
`--input`:

```bash
skopos convert --to md spec.skopos.json --input glossary.skopos.json -o SPEC.md
```

The tool copies these items:

- The accepted terms of each pinned glossary.
- The accepted constraints of each pinned `agent_rules` artifact.
- Each item that a ref of the document points to.

A reader ignores the copied data.

### import

`skopos import` reads a Markdown file that a person edited, and the stored
version that the file came from. Then it writes the version that you must
store, as section 10.8 of the specification says:

- If the stored version is final and the content did not change, the result
  is the stored version.
- If the stored version is final and the content changed, the result is the
  next version as a draft. Its first parent is a `previous_version` parent
  that points to the stored version.
- If the stored version is a draft, the file replaces it. The draft keeps its
  version number.

```bash
skopos import INTENT.md --stored intent.skopos.json -o intent.skopos.v3.json
```

The tool writes a summary to standard error, for example
`INTENT.md: new draft version 3`. Without `-o`, the tool writes the document
to standard output, in the format of the stored version. The result always
has the artifact ID of the stored version.

### ids

`skopos ids` adds an ID to each item that has no ID, and updates
`id_counters`. An item has no ID if its `id` is absent, `null` or empty. The
tool gets the prefix from the `type` of the item, and the number from the
counter, as section 7 of the specification says. If a counter is less than
the highest ID number of its prefix, the tool first increases the counter to
that number.

To add an item by hand, do not write its `id`. Then use the tool:

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
changed file with the canonical layout.

With `--check`, the tool does not change files. It writes the name of each
file that needs a change. If one or more files need a change, the exit code
is 1.

### export

`skopos export` changes a document into the files of a spec-driven
development tool: [OpenSpec](https://github.com/Fission-AI/OpenSpec),
[Kiro](https://kiro.dev) or [Spec Kit](https://github.com/github/spec-kit).
Thus a team can keep its agreements in OpenSkopos, and also give them to the
tool that its agents use.

```bash
skopos export spec.skopos.json --to openspec
skopos export spec.skopos.json --to kiro --name seat-limits
skopos export spec.skopos.json --to speckit -o ../billing
```

| Tool | Files |
|---|---|
| `openspec` | `openspec/changes/<name>-v<version>/`: `.openspec.yaml`, `proposal.md`, `design.md` and a spec delta in `specs/<capability>/spec.md` |
| `kiro` | `.kiro/specs/<name>/`: `requirements.md` and `design.md` |
| `speckit` | `specs/<NNN>-<name>/`: `spec.md`, `plan.md` and `research.md` |

- `-o` is the root folder of the project. The default is the current folder.
- `--name` is the name of the change or feature. Without `--name`, the tool
  makes the name from the title of the document.
- For Spec Kit, `--number` sets the feature number. Without `--number`, the
  tool uses the number of a folder in `specs/` that has the same name. If
  there is no such folder, it uses the next free number.
- The tool does not write over a file that exists. To write over it, use
  `--force`. With `--force`, the tool also removes the files of an earlier
  export that this export does not write. Thus no old items stay in the
  project.
- With `--dry-run`, the tool writes the names of the files, but it does not
  write the files.
- For OpenSpec, the tool reads `openspec/specs/` and the open changes. A
  later export writes MODIFIED and REMOVED requirements for the items that
  earlier exports put into the specs. If an earlier export is not archived,
  the tool stops with an error. See
  [docs/exporters.md](../../docs/exporters.md#openspec).
- For Spec Kit, the tool writes the value of `.specify/feature.json` that
  makes Spec Kit work on the exported feature. The tool does not change that
  file.

The tool exports only accepted items. Each exported line ends with the
reference string of its item, for example `pa_2Xc5Rv8Tn1Mb4Kz7@3#FR4`. The
tool puts these items in a "Not exported" note in the files:

- Items that have no equivalent in the other tool.
- Items that are proposed or rejected.

The tool also writes this list to standard error.
[docs/exporters.md](../../docs/exporters.md) shows where each item type goes.

The document can be JSON, YAML or Markdown. The document must be valid. If
`skopos check` finds a problem, the tool exports nothing.

### diff

`skopos diff` compares two versions of an artifact, item by item. An item ID
has the same meaning in all versions. Thus the tool compares the items that
have the same ID.

```bash
skopos diff spec-v2.skopos.json spec-v3.skopos.json
```

Each file can be JSON, YAML or Markdown. Thus
`skopos diff spec.skopos.json SPEC.md` shows the changes that a person made
by hand in the Markdown file.

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
| `added` | The ID is only in the new version. |
| `changed` | The ID is in the two versions, and the fields after the ID are different. If only the order of the items or of the `refs` is different, the item did not change. |
| `superseded` | The ID is only in the old version, and an item of the new version points to it with `supersedes`. A rejected item does not supersede an item. |
| `dropped` | The ID is only in the old version, and no item supersedes it. |
| `envelope` | The fields of the document that are different, for example `title` or `summary`. The tool does not compare `version`, `status`, `parents`, `created_at`, `finalized_at`, `method`, `session` and `id_counters`. |

With `--json`, the tool writes the differences as JSON. If the two documents
are valid, the exit code is 0, also when the documents are different.

## License

[Apache License 2.0](../../LICENSE)
