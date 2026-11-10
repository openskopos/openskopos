# skopos-core

`skopos-core` is the reference implementation of the
[OpenSkopos](https://github.com/openskopos/openskopos) format 0.1 in Rust. It
reads, checks and writes documents.

All functions are pure. They take strings or `serde_json::Value` and give
strings or values. The crate reads no files and uses no network. It builds for
`wasm32-unknown-unknown`.

## Functions

| Function | What it does |
|---|---|
| `check(&Value) -> Vec<Problem>` | Finds every problem in a document: schema problems first, then problems with the rules in section 9 of the specification. An empty list means that the document is valid. |
| `check_schema`, `check_rules` | The two parts of `check`. |
| `parse(&str, Format) -> Result<Value, Problem>` | Reads JSON or YAML text. |
| `write(&Value, Format) -> String` | Writes a document as JSON or YAML in the canonical layout. Reading the text again gives the same value. |
| `assign_ids(&mut Value) -> Result<IdReport, Vec<Problem>>` | Gives an ID to each item that has none, and updates `id_counters` (specification section 7). |
| `parse_item_id(&str)` | Reads an item ID such as `CR12` into its type and number. |

A `Problem` has three fields:

| Field | Meaning |
|---|---|
| `path` | Where the problem is, as a JSON Pointer. An empty string is the whole document. `/items/3/id` is the `id` of the fourth item. |
| `rule` | The name of the rule in section 9, for example `unique-item-ids`. `None` for a schema problem. |
| `message` | What is wrong, in words for people. |

The module `model` has serde types for a valid document, such as `Document`,
`Item` and `ItemType`. Use `check` on the value before you deserialize it. A
document that is not valid can fail to deserialize. The types keep every
field, so a value gives the same value after a round trip through `Document`.

The module `rules` has the names of the four rules in section 9.

```rust
use skopos_core::{check, parse, write, Format};

let text = std::fs::read_to_string("intent.skopos.json")?;
let doc = parse(&text, Format::Json)?;
for problem in check(&doc) {
    println!("{problem}");
}
let yaml = write(&doc, Format::Yaml);
```

`check` sees one version only. So it cannot check the rule that an ID of an
earlier version is never used for a different item.

## WebAssembly

The schema validator uses `getrandom`. For `wasm32-unknown-unknown`, the crate
turns on the `wasm_js` feature of `getrandom`. A build for this target must
also set a flag for the compiler. The file `.cargo/config.toml` of the
repository sets it:

```toml
[target.wasm32-unknown-unknown]
rustflags = ['--cfg', 'getrandom_backend="wasm_js"']
```

If you build the crate for this target in your own project, add the same
configuration.

## The schema

The crate contains a copy of `schema/artifact/0.1.json` in
`schema/0.1.json`, so that the published crate has the schema. A test makes
sure that the two files are the same. If you change the schema, copy it again.

## License

[Apache License 2.0](../../LICENSE)
