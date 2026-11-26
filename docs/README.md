# Documentation

These pages tell you how to use OpenSkopos documents and the tools of the
reference implementation. The [specification](../spec/0.2/README.md) defines
the format.

## Start here

- [Install the skopos tool](install.md) on macOS, Linux or Windows.
- [Use OpenSkopos from an agent](mcp.md): connect Claude Code, Codex,
  Cursor or another agent to the OpenSkopos MCP server.
- [Changelog](../RELEASES.md): what each release added, changed and removed.
- [Editor support](editors.md): get validation and completion for your
  documents in your editor.

## Tools

- [The skopos command line tool](../crates/skopos/README.md): all commands
  and their options.
- [Readable views](render.md): show a document as text in the terminal or
  as an HTML page.
- [Exporters](exporters.md): write a document as an OpenSpec, Kiro or Spec Kit
  spec.
- [The @openskopos/cli npm package](../npm/cli/README.md): the same tool for
  Node.

## Libraries

- [skopos-core](../crates/skopos-core/README.md), for Rust.
- [@openskopos/core](../npm/core/README.md), for JavaScript and TypeScript in
  Node and in browsers.

## The format

- [Specification 0.2](../spec/0.2/README.md)
- [Changelog of the format](../spec/CHANGELOG.md)
- [JSON Schema 0.2](../schema/artifact/0.2.json)
- [Conformance cases](../conformance/README.md): the documents that all
  implementations must accept or reject.
