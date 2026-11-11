# @openskopos/core

`@openskopos/core` reads, checks and writes
[OpenSkopos](https://openskopos.org) documents in Node and in browsers. It is
the Rust reference implementation `skopos-core`, built to WebAssembly. Its
results are the same as the results of the Rust library and of the `skopos`
command line tool.

## Install

```bash
npm install @openskopos/core
```

The package needs Node 20 or later. In a browser, it needs a bundler or
native ES modules.

## Use in Node

In Node, the package loads its WebAssembly module when you import it. You can
call the functions immediately.

```js
import { readFileSync } from "node:fs";
import { check, parse } from "@openskopos/core";

const doc = parse(readFileSync("intent.skopos.json", "utf8"), "json");
for (const problem of check(doc)) {
  console.log(`${problem.path}: ${problem.message}`);
}
```

## Use in a browser

In a browser, call `init()` one time before you call a different function.
`init()` loads the WebAssembly file `skopos_bg.wasm` from the package. Vite,
webpack and other bundlers find the file through `import.meta.url`.

```js
import init, { check, parse } from "@openskopos/core";

await init();
const problems = check(parse(text, "yaml"));
```

If the file is at a different location, give its URL:
`await init({ module_or_path: "/assets/skopos_bg.wasm" })`. In Node, `init()`
is not necessary, but you can call it.

## Values

A document goes in and comes out as a plain object, the same as the result of
`JSON.parse`. Problems, warnings and the other results are also plain
objects. Their keys are in snake_case, the same as in the file format and in
the JSON output of the `skopos` tool.

A `Problem` has three fields:

| Field | Meaning |
|---|---|
| `path` | The location of the problem, as a JSON Pointer. An empty string is the full document. `/items/3/id` is the `id` of the fourth item. |
| `rule` | The name of the rule in section 9 of the specification, for example `unique-item-ids`. For a schema problem, `rule` is `null`. |
| `message` | A description of the problem, for people to read. |

A value goes into the WebAssembly module as JSON text. Thus the rules of
`JSON.stringify` apply:

- If the value of a key is `undefined`, the key is not in the text.
- A JavaScript number does not keep a decimal point in a whole number. Thus
  `version: 3.0` in a file becomes `3` in the object, and `write` writes `3`.
  The `skopos fmt` command reads the text directly and keeps `3.0`.
- An integer that is more than 2^53 is not exact, the same as in all
  JavaScript numbers.

If a function cannot return a result, it throws an `Error` with the name
`SkoposError`. The error has the details in `problems` (a list of `Problem`)
or in `errors` (a list of Markdown errors, each with a `line` and a
`message`). If an argument is not correct, for example an unknown format, the
function throws a `TypeError`.

If the Rust code panics, the call throws an `Error` with the panic message.
A panic is a bug. Please report it. After a panic, the state of the module is
unknown. Load the module again before the next call, for example in a new
worker.

## Functions

| Function | What it does |
|---|---|
| `check(doc)` | Finds all problems in a document. The schema problems are first, then the problems with the rules in section 9. If the list is empty, the document is valid. |
| `checkSchema(doc)`, `checkRules(doc)` | The two parts of `check`. |
| `parse(text, format)` | Reads JSON or YAML text. `format` is `"json"` or `"yaml"`. The function does not check the document. |
| `write(doc, format)` | Writes a document as JSON or YAML with the canonical layout. `toJson(doc)` and `toYaml(doc)` do the same for one format. |
| `assignIds(doc)` | Adds an ID to each item that has no ID, and updates `id_counters`. Returns `{ doc, assigned, counters_changed }`. It does not change the document that you give to it. |
| `parseItemId(id)` | Reads an item ID, for example `CR12`, and returns `{ type, number }`. If the ID is not valid, it returns `undefined`. |
| `markdownWrite(doc, inputs?)` | Writes a document as Markdown. `inputs` are other documents. The function copies data from them, for example from pinned glossaries. |
| `markdownRead(md, options?)` | Reads a Markdown file and returns `{ doc, warnings }`. The options are `filename`, `now` and `artifact_id`. If the file has no `created_at` or `id`, the reader uses `now` and `artifact_id`. Without these options, it uses the current time and a new random ID. |
| `nextVersion(stored, readDoc, now?)` | Returns the document that you must store after a person edits the Markdown of a stored version (section 10.8). |
| `sameContent(a, b)` | Returns true if two documents have the same content, as section 10.8 defines it. |
| `exportTo(doc, tool, options?)` | Makes the files of OpenSpec, Kiro or Spec Kit. `tool` is `"openspec"`, `"kiro"` or `"speckit"`. The options are `name`, `number` and `existing`. |
| `exportName(doc, options?)` | Returns the name that `exportTo` uses for the folder. |
| `diff(oldDoc, newDoc)` | Compares two versions by item ID. `diffText(oldDoc, newDoc)` returns the same result as text. |
| `newArtifactId()` | Returns a new random artifact ID, for example `pa_7Hk2mQ9xR4tV8wZ1`. |
| `schema()`, `schemaUrl()` | Return the JSON Schema of format 0.1 as text, and its URL. |
| `rules()` | Returns the names of the four rules in section 9. |
| `version()` | Returns the version of the package. |
| `runCli(args, host)` | Operates the `skopos` command line tool. The package `@openskopos/cli` uses this function. |

The TypeScript types are in the package. For the format, see the
[specification](https://github.com/openskopos/openskopos/blob/main/spec/0.1/README.md).

## License

[Apache License 2.0](https://github.com/openskopos/openskopos/blob/main/LICENSE)
