<p>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="brand/mark-dark.svg">
    <img src="brand/mark-light.svg" width="72" alt="OpenSkopos">
  </picture>
</p>

# OpenSkopos

OpenSkopos is an open format for what a team agrees before an AI agent builds
the work. A document holds typed items: goals, success signals, requirements,
stories, constraints, assumptions, non-goals, risks, open questions,
decisions, terms and critiques. Each item has a stable ID such as `S1` or
`CR3`. The ID stays the same in every version of the document, so people,
commits, tickets and agents can refer to it.

*Skopos* is Greek for aim or purpose.

## Status

Version 0.1 is in draft. The format can change between 0.x versions. This
repository will hold:

| Folder | Contents |
|---|---|
| `spec/` | The specification, by version |
| `schema/` | The JSON Schema files, published at `https://openskopos.org/schema/` |
| `conformance/` | Valid and invalid documents that every implementation must handle |
| `crates/` | The reference implementation in Rust (`skopos-core`) and the `skopos` CLI |
| `npm/` | The WebAssembly build for JavaScript |
| `site/` | The website at [openskopos.org](https://openskopos.org) |
| `brand/` | The logo and the brand rules ([BRAND.md](brand/BRAND.md)) |

## Files

A team keeps a readable Markdown file, such as `INTENT.md`, next to a data
file in JSON or YAML, such as `intent.skopos.json`. The Markdown file can be
edited by hand and read back.

## Reference user

[Clarvelo](https://clarvelo.com) facilitates the team sessions that produce
OpenSkopos documents. Any tool can read and write the format without
Clarvelo.

## License

[Apache License 2.0](LICENSE), for the specification and the code.
