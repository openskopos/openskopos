# Releases

This file lists the changes to the `skopos` tool, the Rust crates and the npm
packages, by release. [spec/CHANGELOG.md](spec/CHANGELOG.md) lists the changes
to the format. The first two numbers of a release are the format version that
it reads and writes (see "Versions" in [CONTRIBUTING.md](CONTRIBUTING.md)).

## 0.2.0

It reads and writes format 0.2, and it still reads documents of format 0.1.
Format 0.2 has no breaking change (see
[spec/CHANGELOG.md](spec/CHANGELOG.md)).

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
- `skopos-core` gets the modules `history`, `merge`, `render`, `fix` and
  `words`. `@openskopos/core` gets `history`, `trace`, `merge`, `conflicts`,
  `resolve`, `renderHtml`, `renderTerminal`, `stylesheet`, `applyFix`,
  `fixAll` and `wordDiff`, and the text versions of history, trace and
  merge.
- Breaking changes in `skopos-core`: `Problem` has the new fields `hint` and
  `fix`, and it no longer implements `Hash`. `diff::ChangedItem` has the new
  fields `text_spans` and `field_spans`. Code that builds these structs
  must set the new fields.
- The tools at [openskopos.org/tools](https://openskopos.org/tools/) check,
  write, compare and merge documents in the browser.

## 0.1.0

The first release. It reads and writes format 0.1.

- The `skopos` command line tool, with the commands `check`, `fmt`,
  `convert`, `import`, `ids`, `export` and `diff`. It reads and writes JSON,
  YAML and Markdown.
- Binaries for macOS, Linux and Windows, install scripts for the shell and
  PowerShell, and the Homebrew formula `openskopos/tap/skopos`.
- The Rust crates `skopos-core` and `skopos` on crates.io.
- The npm packages `@openskopos/core` and `@openskopos/cli`.
