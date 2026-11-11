# Contributing

Thank you for your interest in OpenSkopos. You can contribute in these ways:

- Report a problem or ask a question in an issue.
- Propose a change to the format.
- Add conformance cases.
- Fix the text of the specification.

[GOVERNANCE.md](GOVERNANCE.md) says who decides on changes.

## Propose a change to the format

1. Open an issue first. Describe the problem that the change solves, with a
   real example document if you have one.
2. When the maintainers agree on a direction, open a pull request. A change to
   the format includes all of these:
   - The text in `spec/<version>/`.
   - The schema in `schema/artifact/`.
   - Conformance cases in `conformance/` that show the change.
   - An entry in [spec/CHANGELOG.md](spec/CHANGELOG.md). If the change breaks
     existing documents, say so and say how to migrate them.

Do not edit a published schema file. A schema file is published when its tag
`schema-<version>` exists, for example `schema-0.1`. A published file never
changes, and CI fails if it changes. A change to a published version goes into
a new version, with a new schema file. Until its tag exists, a schema file is a
draft and can change.

## Add conformance cases

[conformance/README.md](conformance/README.md) describes the layout and the
steps. Run the conformance check before you open a pull request:

```bash
cargo run --locked --manifest-path conformance/runner/Cargo.toml
```

## Change the Rust code

The reference implementation is in `crates/`. Before you open a pull request,
run these commands from the root of the repository:

```bash
cargo fmt -p skopos-core -p skopos
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

`skopos-core` must build for WebAssembly. Do not add file access, network
access or a dependency that cannot build for `wasm32-unknown-unknown`. CI
builds the crate for this target on each pull request.

## Versions

The format and the tools share one version number, so you never need a table
to know which tools go with which format.

- The format has a two-part version, such as `0.1`. The specification is in
  `spec/0.1/`, and the schema is at `https://openskopos.org/schema/artifact/0.1.json`.
- Every package has the same three-part version: the crates `skopos-core`,
  `skopos` and `skopos-wasm`, and the npm packages `@openskopos/core` and
  `@openskopos/cli`. Its first two numbers are the format version that the
  tools read and write. `skopos` 0.1.3 and `@openskopos/core` 0.1.3 both work
  with format 0.1.
- A bug fix or an addition that breaks nothing, such as a new command, a new
  exporter or a new library function, increases the last number: 0.1.0, then
  0.1.1.
- A new format version moves every package to it. Format 0.2 ships as
  packages 0.2.0.
- Within one format version, the library APIs can only grow. A change that
  breaks code that uses `skopos-core` or `@openskopos/core` waits for the
  next format version.
- After 1.0, the same rules apply: format 1.1 ships as packages 1.1.0, and
  format 2.0 as packages 2.0.0.

CI runs `node scripts/check-versions.mjs`, which fails if the packages have
different versions or if their version does not match the format version.

## Make a release

1. Change the version of all crates and both `package.json` files, also the
   dependency of `@openskopos/cli` on `@openskopos/core`. Run
   `node scripts/check-versions.mjs`.
2. Add a section for the version to [RELEASES.md](RELEASES.md). The GitHub
   release uses it as its notes.
3. Merge the change, then push the tag, for example `v0.1.1`. The Release
   workflow builds the binaries, makes the GitHub release, updates the
   Homebrew formula, publishes the crates and stages the npm packages.
4. Approve the staged npm packages on npmjs.com, `@openskopos/core` first and
   then `@openskopos/cli`.

For a new format version, also push the tag `schema-<version>`, for example
`schema-0.2`, to publish the schema.

## Writing style

The specification is for readers in many countries, and many of them do not
speak English as a first language. Write plain English:

- Use short sentences, active voice and simple tenses.
- Use "must", "must not" and "can". Do not use "should" or "may".
- Use one word for one meaning in the whole text.
- Define a term at its first use.

## Licence

OpenSkopos is under the [Apache License 2.0](LICENSE). When you contribute, you
agree that your contribution is under the same licence (section 5 of the
licence).
