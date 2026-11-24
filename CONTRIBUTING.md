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

OpenSkopos has one version number for everything, so you never need a table
to know which parts go together.

- One x.y version names the format, the specification in `spec/x.y/`, the
  schema at `https://openskopos.org/schema/artifact/x.y.json`, and the
  release of the tools: the crates `skopos-core`, `skopos` and `skopos-wasm`,
  the npm packages `@openskopos/core` and `@openskopos/cli`, and the `skopos`
  binaries. For example, release 0.2.0 has specification 0.2, schema 0.2,
  `skopos` 0.2.0 and `@openskopos/core` 0.2.0.
- Every x.y version is a tagged release. A new x.y version ships the
  specification, the schema and all packages together, with the tags
  `vx.y.0` and `schema-x.y` on the same commit.
- A bug fix or an addition that breaks nothing, such as a new command, a new
  exporter or a new library function, can ship as a fix release: 0.2.0, then
  0.2.1. A fix release changes all packages together and does not change the
  format. The changelog, the site and the documentation name the x.y version
  only; the full version is only for tags, package files and downloads.
- A change to the format, or a change that breaks code that uses
  `skopos-core` or `@openskopos/core`, needs the next x.y version.
- After 1.0, the same rules apply: 1.1.0 has format 1.1, and 2.0.0 has
  format 2.0.

CI runs `node scripts/check-versions.mjs`. It fails if the packages have
different versions, if their x.y is not the format that `skopos-core` reads
and writes, or if the specification, the schema, the changelog of the format
or [RELEASES.md](RELEASES.md) has no section for that x.y version.

## Make a release

1. Change the version of all crates and both `package.json` files, also the
   dependency of `@openskopos/cli` on `@openskopos/core`. Run `cargo update
   --workspace` and `node scripts/check-versions.mjs`.
2. Add the changes to the section `## x.y` of [RELEASES.md](RELEASES.md),
   under `### Added`, `### Changed` or `### Removed`. A new x.y version gets a
   new section. The Release notes workflow puts the section in the GitHub
   release, and the site shows it at /changelog/.
3. Merge the change, then push the tag, for example `v0.2.1`. The Release
   workflow builds the binaries, makes the GitHub release, updates the
   Homebrew formula, publishes the crates and stages the npm packages. For a
   new x.y version, also push the tag `schema-x.y`, for example `schema-0.2`,
   on the same commit. It publishes the schema.
4. Approve the staged npm packages on npmjs.com, `@openskopos/core` first and
   then `@openskopos/cli`.
5. Change the version of `@openskopos/core` in `site/tools/package.json` to
   the new version, and run `npm install` in `site/tools`. The tools on the
   site then use the release.

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
