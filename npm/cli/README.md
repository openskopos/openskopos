# @openskopos/cli

`@openskopos/cli` is the `skopos` command line tool for
[OpenSkopos](https://openskopos.org) documents. One package operates in Node
on Linux, macOS and Windows.

The commands are the commands of the native `skopos` tool, built to
WebAssembly in [`@openskopos/core`](https://www.npmjs.com/package/@openskopos/core).
They have the same flags, output and exit codes. This package only connects
the commands to the files and the terminal.

## Use

You can use the tool without an installation. Use `npx`:

```bash
npx @openskopos/cli check intent.skopos.json
```

To install the command `skopos`, enter this command:

```bash
npm install --global @openskopos/cli
```

The package needs Node 20 or later.

## Commands

| Command | What it does |
|---|---|
| `skopos check FILE...` | Compares documents with the schema and the rules of the specification. |
| `skopos fmt FILE...` | Writes documents again with the canonical layout. |
| `skopos convert FILE --to json\|yaml\|md` | Changes a document from JSON, YAML or Markdown to one of the other formats. |
| `skopos import FILE.md --stored FILE` | Reads an edited Markdown file, and writes the version that you must store. |
| `skopos ids FILE...` | Adds an ID to each item that has no ID. |
| `skopos export FILE --to openspec\|kiro\|speckit` | Writes the files of a spec-driven development tool. |
| `skopos diff OLD NEW` | Shows the items that are different in two versions. |

For the flags, enter `skopos --help` or `skopos <command> --help`. The
[README of the native tool](https://github.com/openskopos/openskopos/blob/main/crates/skopos/README.md)
describes each command.

These are the exit codes:

- 0: the command found no problem.
- 1: the command found a problem.
- 2: the arguments are not correct.

Some details are different from the native tool:

- On Windows, the paths that the tool makes have `/` between folders, for
  example the path of an exported file.
- On Windows, Node writes the message about a file error, for example a
  missing file.
- On Linux and macOS, the message about a usual file error is the same as the
  message of the native tool. For an unusual error, for example a network
  drive that does not work, Node can write the message.

## License

[Apache License 2.0](https://github.com/openskopos/openskopos/blob/main/LICENSE)
