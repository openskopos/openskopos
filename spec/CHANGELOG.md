# Changelog

This file lists the changes to the OpenSkopos format, by format version. While
the format is at 0.x, each 0.y version can have breaking changes. Each entry
lists every breaking change, so that tools can migrate documents.

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
