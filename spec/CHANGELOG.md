# Changelog

This file lists the changes to the OpenSkopos format, by format version. While
the format is at 0.x, each 0.y version can have breaking changes. Each entry
lists every breaking change, so that tools can migrate documents.

## 0.1 (draft, not published)

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
- A Markdown format that holds every field, and generous rules to read it.
- Glossaries as artifacts of kind `glossary`.
