# Changelog

This file lists the changes to the OpenSkopos format, by format version. While
the format is at 0.x, each 0.y version can have breaking changes. Each entry
lists every breaking change, so that tools can migrate documents.

## 0.3 (draft)

Breaking change: adds the `replaces` parent `rel` for compaction and merge.
A 0.2 validator rejects a document that uses `replaces`. A 0.2 document is a
valid 0.3 document when its `$schema` is the URL of 0.3.

- Section 8.2: adds the `replaces` artifact parent `rel`. A `replaces` parent
  to another artifact marks that artifact as retired by compaction or merge.
  A `replaces` parent to the document's own `previous_version` marks a
  compacted next version of a continuing artifact.
- Section 8.3 (informative): adds the `carried` item state, orders chains by
  `replaces` parents as well as `input` parents, keeps the prior state of
  rejected, settled and carried items removed by a compacted next version,
  and propagates supersession backward along `derived_from` chains across
  repeated compactions.

## 0.2 (2026-11-24)

No breaking change. A document of format 0.1 is a valid document of format
0.2 when its `$schema` is the URL of 0.2.

- Section 8.3 (informative): how a tool reads several documents as one
  history, and the state of each item: in force, proposed, rejected,
  superseded or dropped.
- Section 8.4 (informative): how a tool merges two documents of the same
  work into the next version of one of them. A conflict stays in the format,
  as two `proposed` items with a `contradicts` ref.
- Conformance cases for histories in `conformance/history/` and for merges in
  `conformance/merge/`.

## 0.1 (2026-11-11)

The first version of the format.

- The envelope: `$schema`, `id`, `version`, `kind`, `title`, `summary`,
  `status`, `method`, `session`, `participants`, `decider`, `parents`,
  `id_counters`, `items`, `created_at` and `finalized_at`.
- Twelve item types: goal (`G`), success signal (`S`), requirement (`FR`),
  story (`ST`), constraint (`C`), assumption (`A`), non-goal (`N`), risk (`R`),
  open question (`Q`), decision (`D`), term (`T`) and critique (`CR`).
- Stable item IDs, reference strings and ID counters.
- Versions with `previous_version` parents, and `input` parents for other
  artifacts and external material.
- A Markdown format that contains every field, and generous rules to read it.
- Glossaries as artifacts of kind `glossary`.
