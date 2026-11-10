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
    round-trip/<case>/   A document, its rendering, an edited rendering and the next version
    invalid/<case>/      A document and an edited rendering that a reader must reject
  runner/                The check that CI runs (Rust)
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

### markdown/round-trip/

Each case folder holds:

| File | Meaning |
|---|---|
| `document.json` | The base version. |
| `inputs/` | Other documents that the rendering copies material from, if any. Each file is named `<artifact id>@<version>.json`. |
| `rendering.md` | The rendering of `document.json`. A writer must make this file byte for byte. |
| `edited.md` | `rendering.md` after a person edited it. |
| `expected.json` | The document that a reader makes from `document.json` and `edited.md`. A comparison ignores the order of keys and the fields `created_at`, `method` and `session`. |

`index.json` describes what each case tests.

### markdown/invalid/

Each case folder holds a `document.json` and an `edited.md`. A reader must
report an error for each case. `index.json` gives the reason.

## The runner

The runner in `runner/` is a small Rust program. CI runs it on each pull
request. It does these checks:

- Each document in `valid/` passes the schema and the rules of section 9.
- Each `schema` case in `invalid/` fails the schema.
- Each `semantic` case in `invalid/` passes the schema and breaks only the rule
  that its entry names. A semantic case that the schema also rejects tests
  nothing, so the runner reports it.
- Each JSON file of the Markdown cases is valid, and the front matter of each
  rendering names its `document.json`.
- Each file and folder is in its `index.json`.

The runner does not render or read Markdown. That is the job of the reference
implementation, which will run the Markdown cases when it exists.

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
