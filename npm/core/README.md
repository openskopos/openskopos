# @openskopos/core

`@openskopos/core` reads, checks and writes
[OpenSkopos](https://openskopos.org) documents in Node and in browsers. It is
the Rust reference implementation `skopos-core`, built to WebAssembly. It
gives the same results as the Rust library and the `skopos` command line tool.

## Install

```bash
npm install @openskopos/core
```

The package needs Node 20 or later. In a browser, it needs a bundler or
native ES modules.

## Use in Node

In Node, the package loads its WebAssembly module when you import it. You can
call the functions at once.

```js
import { readFileSync } from "node:fs";
import { check, parse } from "@openskopos/core";

const doc = parse(readFileSync("intent.skopos.json", "utf8"), "json");
for (const problem of check(doc)) {
  console.log(`${problem.path}: ${problem.message}`);
}
```

## Use in a browser

In a browser, call `init()` once before you call another function. `init()`
loads the WebAssembly file `skopos_bg.wasm` from the package. Vite, webpack
and other bundlers find the file through `import.meta.url`.

```js
import init, { check, parse } from "@openskopos/core";

await init();
const problems = check(parse(text, "yaml"));
```

If you serve the file from another place, give its URL:
`await init({ module_or_path: "/assets/skopos_bg.wasm" })`. In Node, `init()`
is not necessary, but it does no harm.

## Values

A document goes in and comes out as the plain object that `JSON.parse` gives.
Problems, warnings and the other results are plain objects too. Their keys
are in snake_case, as in the file format and in the JSON output of the
`skopos` tool.

A `Problem` has three fields:

| Field | Meaning |
|---|---|
| `path` | Where the problem is, as a JSON Pointer. An empty string is the whole document. `/items/3/id` is the `id` of the fourth item. |
| `rule` | The name of the rule in section 9 of the specification, for example `unique-item-ids`. `null` for a schema problem. |
| `message` | What is wrong, in words for people. |

If a function cannot give a result, it throws an `Error` with the name
`SkoposError`. The error has the details in `problems` (a list of `Problem`)
or in `errors` (a list of Markdown errors, each with a `line` and a
`message`). A wrong argument, such as an unknown format, gives a `TypeError`.

## Functions

| Function | What it does |
|---|---|
| `check(doc)` | Finds every problem in a document: schema problems first, then problems with the rules in section 9. An empty list means that the document is valid. |
| `checkSchema(doc)`, `checkRules(doc)` | The two parts of `check`. |
| `parse(text, format)` | Reads JSON or YAML text. `format` is `"json"` or `"yaml"`. It does not check the document. |
| `write(doc, format)` | Writes a document as JSON or YAML in the canonical layout. `toJson(doc)` and `toYaml(doc)` do the same for one format. |
| `assignIds(doc)` | Gives an ID to each item that has none, and updates `id_counters`. Returns `{ doc, assigned, counters_changed }`, and does not change the document that you give. |
| `parseItemId(id)` | Reads an item ID such as `CR12` into `{ type, number }`. Returns `undefined` for an ID that is not valid. |
| `markdownWrite(doc, inputs?)` | Writes a document as Markdown. `inputs` are other documents to copy material from, such as pinned glossaries. |
| `markdownRead(md, options?)` | Reads a Markdown file into `{ doc, warnings }`. The options are `filename`, `now` and `artifact_id`. If the file has no `created_at` or `id`, the reader uses `now` and `artifact_id`. Without them, it uses the current time and a new random ID. |
| `nextVersion(stored, readDoc, now?)` | The document to store after a person edits the Markdown of a stored version (section 10.8). |
| `sameContent(a, b)` | True if two documents have the same content in the sense of section 10.8. |
| `exportTo(doc, tool, options?)` | Makes the files of OpenSpec, Kiro or Spec Kit. `tool` is `"openspec"`, `"kiro"` or `"speckit"`. The options are `name`, `number` and `existing`. |
| `exportName(doc, options?)` | The name that `exportTo` uses for the folder. |
| `diff(oldDoc, newDoc)` | Compares two versions by item ID. `diffText(oldDoc, newDoc)` gives the same result as text. |
| `newArtifactId()` | A new random artifact ID, such as `pa_7Hk2mQ9xR4tV8wZ1`. |
| `schema()`, `schemaUrl()` | The JSON Schema of format 0.1 as text, and its URL. |
| `rules()` | The names of the four rules in section 9. |
| `version()` | The version of the package. |
| `runCli(args, host)` | Runs the `skopos` command line tool. The package `@openskopos/cli` uses it. |

The TypeScript types are in the package. See the
[specification](https://github.com/openskopos/openskopos/blob/main/spec/0.1/README.md)
for the format.

## License

[Apache License 2.0](https://github.com/openskopos/openskopos/blob/main/LICENSE)
