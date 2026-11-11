# Releases

This file lists the changes to the `skopos` tool, the Rust crates and the npm
packages, by release. [spec/CHANGELOG.md](spec/CHANGELOG.md) lists the changes
to the format. The first two numbers of a release are the format version that
it reads and writes (see "Versions" in [CONTRIBUTING.md](CONTRIBUTING.md)).

## 0.1.0

The first release. It reads and writes format 0.1.

- The `skopos` command line tool, with the commands `check`, `fmt`,
  `convert`, `import`, `ids`, `export` and `diff`. It reads and writes JSON,
  YAML and Markdown.
- Binaries for macOS, Linux and Windows, install scripts for the shell and
  PowerShell, and the Homebrew formula `openskopos/tap/skopos`.
- The Rust crates `skopos-core` and `skopos` on crates.io.
- The npm packages `@openskopos/core` and `@openskopos/cli`.
