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

Do not edit a published schema file. A published file never changes. A change
to a published version goes into a new version, with a new schema file.

## Add conformance cases

[conformance/README.md](conformance/README.md) describes the layout and the
steps. Run the conformance check before you open a pull request:

```bash
cargo run --locked --manifest-path conformance/runner/Cargo.toml
```

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
