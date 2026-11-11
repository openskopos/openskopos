<p>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="brand/mark-dark.svg">
    <img src="brand/mark-light.svg" width="72" alt="OpenSkopos">
  </picture>
</p>

# OpenSkopos

OpenSkopos is an open format for the decisions that a team makes before an AI
agent builds the work. A document holds typed items: goals, success signals,
requirements, stories, constraints, assumptions, non-goals, risks, open
questions, decisions, terms and critiques. Each item has a stable ID such as
`S1` or `CR3`. The ID stays the same in every version of the document, so
people, commits, tickets and agents can refer to it.

*Skopos* is Greek for aim or purpose.

## Status

Version 0.1 is in draft. The format can change between 0.x versions.

- [Specification 0.1](spec/0.1/README.md)
- [JSON Schema 0.1](schema/artifact/0.1.json)
- [Conformance cases](conformance/README.md)
- [Changelog](spec/CHANGELOG.md)
- [Documentation](docs/README.md), also on [openskopos.org](https://openskopos.org)

| Folder | Contents |
|---|---|
| `spec/` | The specification, by version |
| `schema/` | The JSON Schema files. A tagged version is published at `https://openskopos.org/schema/`. |
| `conformance/` | Valid and invalid documents that every implementation must handle |
| `docs/` | The documentation of the tools and the libraries |
| `crates/` | The reference implementation in Rust: the library [`skopos-core`](crates/skopos-core/README.md), the command line tool [`skopos`](crates/skopos/README.md) and the WebAssembly build [`skopos-wasm`](crates/skopos-wasm/README.md) |
| `npm/` | The npm packages [`@openskopos/core`](npm/core/README.md) and [`@openskopos/cli`](npm/cli/README.md), from the WebAssembly build |
| `site/` | The website at [openskopos.org](https://openskopos.org) |
| `brand/` | The logo and the brand rules ([BRAND.md](brand/BRAND.md)) |

## Files

A team keeps a readable Markdown file, such as `INTENT.md`, next to a data
file in JSON or YAML, such as `intent.skopos.json`. The Markdown file can be
edited by hand and read back.

## Tools

The `skopos` command line tool checks, formats and converts documents in
JSON, YAML and Markdown, and gives IDs to new items. It also reads an edited
Markdown file back into the next version of a document. It compares two
versions item by item, and exports a document to OpenSpec, Kiro or Spec Kit
(see [docs/exporters.md](docs/exporters.md)). To install it with Homebrew on
macOS or Linux, run this command:

```bash
brew install openskopos/tap/skopos
```

[docs/install.md](docs/install.md) has the other ways to install it: install
scripts for the shell and PowerShell, binaries for each platform, and
`cargo install --locked skopos`.

Then run `skopos check intent.skopos.json`. See
[crates/skopos/README.md](crates/skopos/README.md) for all commands.

The same tool is on npm, for Node on every platform. It needs no Rust:

```bash
npx @openskopos/cli check intent.skopos.json
```

For JavaScript and TypeScript code, the package
[`@openskopos/core`](npm/core/README.md) reads, checks and writes documents in
Node and in browsers.

## Reference user

[Clarvelo](https://clarvelo.com) facilitates team sessions that produce
OpenSkopos documents. Any tool can read and write the format without
Clarvelo.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). [GOVERNANCE.md](GOVERNANCE.md) says who
decides on changes during 0.x.

## License

[Apache License 2.0](LICENSE), for the specification and the code.
