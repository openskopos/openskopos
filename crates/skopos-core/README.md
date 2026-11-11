# skopos-core

`skopos-core` is the reference implementation of the
[OpenSkopos](https://github.com/openskopos/openskopos) format 0.1 in Rust. It
reads, checks and writes documents.

All functions are pure. Their inputs are strings or `serde_json::Value`
values, and they return strings or values. The crate does not read files and
does not use the network. It builds for `wasm32-unknown-unknown`.

## Functions

| Function | What it does |
|---|---|
| `check(&Value) -> Vec<Problem>` | Finds all problems in a document. The schema problems are first, then the problems with the rules in section 9 of the specification. If the list is empty, the document is valid. |
| `check_schema`, `check_rules` | Run only the schema checks, or only the section 9 rules. |
| `parse(&str, Format) -> Result<Value, Problem>` | Reads JSON or YAML text. |
| `write(&Value, Format) -> String` | Writes a document as JSON or YAML with the canonical layout. If you parse the text again, you get the same value. |
| `assign_ids(&mut Value) -> Result<IdReport, Vec<Problem>>` | Adds an ID to each item that has no ID, and updates `id_counters` (specification section 7). |
| `parse_item_id(&str)` | Reads an item ID, for example `CR12`, and returns its type and number. |
| `diff::diff(&Value, &Value) -> Diff` | Compares two versions by item ID. The result has the items that were added, changed, dropped and superseded. `Diff::to_text` returns one line for each item. |
| `export::export(&Value, Tool, &Options) -> Export` | Makes the files of OpenSpec, Kiro or Spec Kit from a document. The result has the files, and the items that the function did not export. See [docs/exporters.md](../../docs/exporters.md). |

A `Problem` has three fields:

| Field | Meaning |
|---|---|
| `path` | The location of the problem, as a JSON Pointer. An empty string is the full document. `/items/3/id` is the `id` of the fourth item. |
| `rule` | The name of the rule in section 9, for example `unique-item-ids`. For a schema problem, `rule` is `None`. |
| `message` | What is wrong, in plain words. |

The module `model` has serde types for a valid document, for example
`Document`, `Item` and `ItemType`. Use `check` on the value before you
deserialize it. If a document is not valid, deserialization can fail. The
types keep every field, so a value that goes through `Document` and back comes
out unchanged.

The module `rules` has the names of the four rules in section 9.

## Markdown

The module `markdown` reads and writes Markdown, as section 10 of the
specification says. A Markdown file holds all fields of a document.

| Function | What it does |
|---|---|
| `markdown::write(&Value, &[Value]) -> String` | Writes a document as Markdown. The slice holds other documents. The function copies data from these documents (section 10.7). If the slice is empty, the function copies no data. |
| `markdown::read(&str, Option<&str>) -> Result<(Value, Vec<Warning>), Vec<Error>>` | Reads a Markdown file and returns a document. The second argument is the file name. The function uses it to find the kind (the kind of `INTENT.md` is `intent`). The reader does not need other files. |
| `markdown::next_version(&Value, &Value) -> Value` | Returns the document that you must store after a person edits the Markdown of a stored version (section 10.8). The arguments are the stored version and the document from `read`. |
| `markdown::same_content(&Value, &Value) -> bool` | Compares the content of two documents, as section 10.8 defines it. |

The reader accepts many different ways to write the same data. For example,
it accepts `## Out of scope` for the non-goals and `[FR2]` for an item ID. A
warning tells you that the reader ignored or changed a part of the file, and
continued. An error tells you that the reader cannot make a valid document.
Section 10.6 of the specification lists the two. The reader runs `check` on
the result, so a schema problem is also an error.

A `Warning` has the `line` (the first line is 1) and the `message`. An
`Error` has these fields:

| Field | Meaning |
|---|---|
| `line` | The line in the file. The first line is 1. If the error is about the full file, `line` is `None`. |
| `message` | A description of the problem, and of the correct text. |
| `problem` | For an error from `check`, the `Problem`. |

If the file has no `id` or `created_at`, `read` makes a new random artifact
ID and uses the current time. `next_version` uses the current time for a new
version. To set these values yourself, use `markdown::read_with` and
`markdown::next_version_at`. The WebAssembly build has no clock, so it has
only these two functions.

```rust
use skopos_core::markdown;

let text = std::fs::read_to_string("INTENT.md")?;
match markdown::read(&text, Some("INTENT.md")) {
    Ok((doc, warnings)) => {
        for w in warnings {
            eprintln!("warning: {w}");
        }
        print!("{}", markdown::write(&doc, &[]));
    }
    Err(errors) => {
        for e in errors {
            eprintln!("{e}");
        }
    }
}
```

```rust
use skopos_core::{check, parse, write, Format};

let text = std::fs::read_to_string("intent.skopos.json")?;
let doc = parse(&text, Format::Json)?;
for problem in check(&doc) {
    println!("{problem}");
}
let yaml = write(&doc, Format::Yaml);
```

`check` sees one version at a time, so it cannot enforce the rule that an ID
from an earlier version is never reused for a different item.

## WebAssembly

The schema validator uses `getrandom`. For `wasm32-unknown-unknown`, the crate
enables the `wasm_js` feature of `getrandom`. A build for this target must
also set a compiler flag. The file `.cargo/config.toml` of the repository sets
this flag:

```toml
[target.wasm32-unknown-unknown]
rustflags = ['--cfg', 'getrandom_backend="wasm_js"']
```

If you build the crate for this target in your project, add the same
configuration.

## The schema

The crate contains a copy of `schema/artifact/0.1.json` in
`schema/0.1.json`, so the published crate includes the schema. A test makes sure
that the two files are the same. If you change the schema, copy it again.

## License

[Apache License 2.0](../../LICENSE)
