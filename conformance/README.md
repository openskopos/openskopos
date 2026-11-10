# Conformance cases

These files define correct behavior for format 0.1, together with the JSON
Schema in [`schema/artifact/0.1.json`](../schema/artifact/0.1.json). An
implementation in any language that passes all cases is correct. Section 13 of
the [specification](../spec/0.1/README.md#13-conformance) states what each
kind of tool must pass.

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
short description. Together, the files use all twelve item types, every
evidence kind, every constraint modality, both parent forms and `x-` keys.

### invalid/

Each file is a document with one fault. `index.json` lists each file with:

| Key | Meaning |
|---|---|
| `check` | `schema` if the JSON Schema rejects the document. `semantic` if the document passes the schema but breaks a rule that the schema cannot express (specification section 9). |
| `rule` | A short name for the rule. For a `semantic` case, it is one of the names in section 9. |
| `reason` | Why the document must fail. |

A checker must reject every case. It does not need to give the same message.

### markdown/

`index.json` lists the cases of each folder with a description. Section 13 of
the specification says how to compare documents.

| Folder | Files | Test |
|---|---|---|
| `round-trip/<case>/` | `document.json`, `document.md`, and `inputs/` if the file copies material from other artifacts | Reading `document.md` gives `document.json`. Writing `document.json` and reading it back gives `document.json`. |
| `generous/<case>/` | A hand-written file such as `INTENT.md`, and `expected.json` | Reading the file gives `expected.json`. The index gives the file name and the number of warnings. |
| `next-version/<case>/` | `stored.json`, `edited.md`, `expected.json` | Reading `edited.md` against the stored version gives `expected.json` (specification 10.8). |
| `invalid/<case>/` | One Markdown file | The reader reports an error. The index gives the file name and the reason. |

## The runner

The runner in `runner/` is a small Rust program. CI runs it on each pull
request. It does these checks:

- Each document in `valid/` passes the schema and the rules of section 9.
- Each `schema` case in `invalid/` fails the schema.
- Each `semantic` case in `invalid/` passes the schema and breaks only the rule
  that its entry names. A semantic case that the schema also rejects tests
  nothing, so the runner reports it.
- Each JSON file of the Markdown cases is valid, and the front matter of each
  Markdown file of a stored document names that document.
- Each `next-version` case keeps the version number, or makes draft version
  n+1 after a final version n.
- Each file and folder is in its `index.json`.

The runner does not read or write Markdown. That is the job of the reference
implementation in [`crates/`](../crates/).

The tests of the reference implementation also run the cases in `valid/`,
`invalid/` and `markdown/`, through the library `skopos-core` and through the
`skopos` command line tool. They also write each document in `valid/` as
Markdown and read it back. To run them, run `cargo test` from the root of the
repository.

To run the runner with Rust installed, run this command from the root of the
repository:

```bash
cargo run --locked --manifest-path conformance/runner/Cargo.toml
```

To run it without Rust installed, use Docker:

```bash
docker run --rm -v "$PWD":/repo -w /repo -e CARGO_TARGET_DIR=/tmp/target rust:1 cargo run --locked --manifest-path conformance/runner/Cargo.toml
```

## Adding a case

1. Add the file or the folder.
2. Add an entry to the `index.json` of its folder.
3. Run the runner.

Make each invalid case break one rule only, so that a failure points to one
cause. Start from a valid document and change one thing.
