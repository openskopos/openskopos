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

A file name must end with `.json`, `.yaml` or `.yml`. The tool cannot read
Markdown yet.

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

`skopos convert` writes a document in the other format. The data does not
change.

```bash
skopos convert --to yaml intent.skopos.json -o intent.skopos.yaml
```

Without `-o`, the document goes to standard output.

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

## License

[Apache License 2.0](../../LICENSE)
