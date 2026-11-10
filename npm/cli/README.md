# @openskopos/cli

`@openskopos/cli` is the `skopos` command line tool for
[OpenSkopos](https://openskopos.org) documents. It runs in Node on Linux,
macOS and Windows, from one package.

The commands are the commands of the native `skopos` tool, built to
WebAssembly in [`@openskopos/core`](https://www.npmjs.com/package/@openskopos/core).
They have the same flags, output and exit codes. This package only connects
them to the files and the terminal.

## Run

To run the tool with no installation, use `npx`:

```bash
npx @openskopos/cli check intent.skopos.json
```

To install the command `skopos`, run this command:

```bash
npm install --global @openskopos/cli
```

The package needs Node 20 or later.

## Commands

| Command | What it does |
|---|---|
| `skopos check FILE...` | Checks documents against the schema and the rules of the specification. |
| `skopos fmt FILE...` | Writes documents again in the canonical layout. |
| `skopos convert FILE --to json\|yaml\|md` | Converts a document between JSON, YAML and Markdown. |
| `skopos import FILE.md --stored FILE` | Reads an edited Markdown file and writes the version to store. |
| `skopos ids FILE...` | Gives an ID to each item that has none. |
| `skopos export FILE --to openspec\|kiro\|speckit` | Writes the files of a spec-driven development tool. |
| `skopos diff OLD NEW` | Shows the items that changed between two versions. |

Run `skopos --help` or `skopos <command> --help` for the flags. The
[README of the native tool](https://github.com/openskopos/openskopos/blob/main/crates/skopos/README.md)
explains each command.

The exit code is 0 if all went well, 1 if a command found a problem, and 2 if
the arguments are wrong.

On Windows, two things are different from the native tool. A path that the
tool makes, such as the path of an exported file, has `/` between folders. A
message about a file error, such as a missing file, comes from Node.

## License

[Apache License 2.0](https://github.com/openskopos/openskopos/blob/main/LICENSE)
