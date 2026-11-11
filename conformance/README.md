# Conformance cases

These files, together with the JSON Schema in
[`schema/artifact/0.1.json`](../schema/artifact/0.1.json), define the correct
behavior for format 0.1. If an implementation in any language passes all
cases, it is correct. Section 13 of the
[specification](../spec/0.1/README.md#13-conformance) gives the cases that
each type of tool must pass.

## Layout

```
conformance/
  valid/                 Documents that every checker must accept
    index.json
  invalid/               Documents that every checker must reject
    index.json
  markdown/
    index.json
    round-trip/<case>/   A document and the same document in Markdown
    generous/<case>/     A hand-written Markdown file and the document that a reader makes from it
    next-version/<case>/ A stored version, an edited Markdown file and what the tool stores
    invalid/<case>/      Markdown files that a reader must reject
  runner/                A check of the cases themselves (Rust)
```

### valid/

Each file is a document in JSON or YAML. `index.json` lists each file with a
short description. The files use all twelve item types, all evidence kinds,
all constraint modalities, the two parent forms and `x-` keys.

### invalid/

Each file is a document with one fault. `index.json` lists each file with
these keys:

| Key | Meaning |
|---|---|
| `check` | `schema` if the JSON Schema rejects the document. `semantic` if the document passes the schema, but does not obey a rule that the schema cannot express (specification section 9). |
| `rule` | A short name for the rule. For a `semantic` case, it is one of the names in section 9. |
| `reason` | The reason why the document must fail. |

A checker must reject all cases. Its messages can be different from the
messages of other checkers.

### markdown/

`index.json` lists the cases of each folder with a description. Section 13 of
the specification tells how to compare documents.

| Folder | Files | Test |
|---|---|---|
| `round-trip/<case>/` | `document.json`, `document.md`, and `inputs/` if the file copies data from other artifacts | If you read `document.md`, the result is `document.json`. If you write `document.json` and read it again, the result is `document.json`. |
| `generous/<case>/` | A hand-written file, for example `INTENT.md`, and `expected.json` | If you read the file, the result is `expected.json`. The index gives the file name and the number of warnings. |
| `next-version/<case>/` | `stored.json`, `edited.md`, `expected.json` | If you read `edited.md` with the stored version, the result is `expected.json` (specification 10.8). |
| `invalid/<case>/` | One Markdown file | The reader reports an error. The index gives the file name and the reason. |

## The runner

The runner in `runner/` is a small Rust program. CI uses it on each pull
request. It does these checks:

- Each document in `valid/` passes the schema and the rules of section 9.
- Each `schema` case in `invalid/` fails the schema.
- Each `semantic` case in `invalid/` passes the schema, and does not obey
  only the rule that its entry gives. If the schema also rejects a semantic
  case, the case tests nothing. Thus the runner reports it.
- Each JSON file of the Markdown cases is valid. The front matter of each
  Markdown file of a stored document identifies that document.
- Each `next-version` case keeps the version number, or makes draft version
  n+1 after a final version n.
- Each file and folder is in its `index.json`.

The runner does not read or write Markdown. The reference implementation in
[`crates/`](../crates/) does that work.

The tests of the reference implementation also use the cases in `valid/`,
`invalid/` and `markdown/`. They use them through the library `skopos-core`
and through the `skopos` command line tool. They also write each document in
`valid/` as Markdown and read it again. To do these tests, enter `cargo test`
in the root folder of the repository.

If Rust is installed, enter this command in the root folder of the repository
to start the runner:

```bash
cargo run --locked --manifest-path conformance/runner/Cargo.toml
```

If Rust is not installed, use Docker:

```bash
docker run --rm -v "$PWD":/repo -w /repo -e CARGO_TARGET_DIR=/tmp/target rust:1 cargo run --locked --manifest-path conformance/runner/Cargo.toml
```

## Adding a case

1. Add the file or the folder.
2. Add an entry to the `index.json` of its folder.
3. Start the runner.

Make each invalid case break only one rule. Thus a failure has only one
cause. Start with a valid document and change one thing.
