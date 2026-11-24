# Changelog

This changelog lists what each release of OpenSkopos added, changed and removed:
the format, the `skopos` tool, the Rust crates and the npm packages. They all
have one version number (see "Versions" in [CONTRIBUTING.md](CONTRIBUTING.md)).
Release x.y.z reads and writes format x.y. [spec/CHANGELOG.md](spec/CHANGELOG.md)
gives more detail about each format version.

Each version has the sections Added, Changed and Removed. A section with
nothing in it is left out. The GitHub release of a version uses its section
as its notes.

## 0.2.0

The tools read and write format 0.2. They still read documents of format 0.1.

### Added

- Format 0.2. It has no breaking change from 0.1. It adds the informative
  sections 8.3 (reading a history) and 8.4 (merging two documents) to the
  [specification](spec/0.2/README.md), and conformance cases for both.
- `skopos history` reads versions of one or more artifacts, in any order,
  and shows what each version decided and the items in force now.
  `skopos trace` shows the life of one item across versions and artifacts.
- `skopos merge` merges two documents into the next version of one of them.
  A conflict stays a valid document: both items are `proposed`, with a
  `contradicts` ref. `skopos conflicts` lists the open conflicts, and
  `skopos resolve` accepts one item of a pair and rejects the other.
- `skopos render` shows a document for people, as text in the terminal or
  as an HTML page, and with `--previous` marks what changed. The HTML is
  styled with CSS custom properties (see [docs/render.md](docs/render.md)).
- Each problem from `skopos check` has a hint that says what to do. Problems
  with a certain fix, such as a low ID counter or a duplicate item ID, also
  have a fix, and `skopos check --fix` applies them.
- `skopos diff` gives the changed words of each changed item.
- `skopos-core` has the new modules `history`, `merge`, `render`, `fix` and
  `words`. `@openskopos/core` has the new functions `history`, `trace`,
  `merge`, `conflicts`, `resolve`, `renderHtml`, `renderTerminal`,
  `stylesheet`, `applyFix`, `fixAll` and `wordDiff`, and the text versions
  of history, trace and merge.
- The [tools](https://openskopos.org/tools/) on the site check, write,
  compare and merge documents in the browser.

### Changed

- The tools write the `$schema` URL of format 0.2 in new documents.
- `skopos-core`: `Problem` has the new fields `hint` and `fix`, and
  `diff::ChangedItem` has the new fields `text_spans` and `field_spans`.
  Code that builds these structs must set the new fields.

### Removed

- `skopos-core`: `Problem` no longer implements `Hash`.

## 0.1.0

The first release. The tools read and write format 0.1.

### Added

- Format 0.1: the envelope, twelve item types, stable item IDs, versions,
  a Markdown format and glossaries.
- The `skopos` command line tool, with the commands `check`, `fmt`,
  `convert`, `import`, `ids`, `export` and `diff`. It reads and writes JSON,
  YAML and Markdown.
- Binaries for macOS, Linux and Windows, install scripts for the shell and
  PowerShell, and the Homebrew formula `openskopos/tap/skopos`.
- The Rust crates `skopos-core` and `skopos` on crates.io.
- The npm packages `@openskopos/core` and `@openskopos/cli`.
