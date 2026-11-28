# Compaction

Teams accumulate versions and parallel documents over months of work. Old
decisions get rejected or replaced, open questions get answered, and two teams
can end up with two specs for the same area. Compaction folds all of those
files back into one clean current draft per kind, plus one glossary draft for
the terms, while keeping every older decision traceable.

Compaction is a prototype for format 0.3. It writes drafts and leaves your
existing files untouched so your team can review the result before finalizing.

## How compaction works at a glance

```text
BEFORE (one area, many files and versions)
┌──────────────────────────────────────────────────────────┐
│ spec pa_4Hs8… v1..v3          spec pa_7Wn3… v1           │
│   D1 accepted (kept)            D1 accepted              │
│   D2 accepted ◄─── supersedes ──┘  (replaces pa_4Hs8#D2) │
│   D3 rejected (left out)        FR1 accepted             │
│   Q1 answered (left out)        T1 "Seat" (duplicate)    │
│   T1 "Seat"                     T2 "Plan" (new def)      │
│                                                          │
│ glossary pa_9Lp4… v2          intent pa_2Xc5… v4         │
│   T4 "Plan" (existing def)      G1 accepted (unchanged)  │
└──────────────────────────┬───────────────────────────────┘
                           │  skopos compact ... --out compacted
                           ▼
AFTER (one clean draft per changed kind, full traceability)
┌──────────────────────────────────────────────────────────┐
│ compacted/spec.skopos.json    new artifact pa_5Cb8… v1   │
│   parents: replaces pa_4Hs8…@3, replaces pa_7Wn3…@1      │
│   D1  (derived_from pa_4Hs8…@3#D1)                       │
│   D2  (derived_from pa_7Wn3…@1#D1, supersedes pa_4Hs8#D2)│
│   FR1 (derived_from pa_7Wn3…@1#FR1, refs rewritten)      │
│                                                          │
│ compacted/glossary.skopos.json   pa_9Lp4… v3 (continues) │
│   parents: previous_version + replaces pa_9Lp4…@2        │
│   T4 "Plan" (proposed) ◄── contradicts ── T6 "Plan"      │
│   T5 "Seat" (accepted, derived_from both T1 sources)     │
│                                                          │
│ unchanged: intent pa_2Xc5…@4  (no draft needed)          │
└──────────────────────────────────────────────────────────┘
```

Every carried item keeps a link back to where it came from, and every old
reference string (such as `pa_7Wn3Kd5Pz1Gy8Fb2#D1`) still resolves when you run
`skopos history` or `skopos trace` across the old and new files.

## What stays and what gets left out

Compaction looks at the latest final version of each artifact in the area (plus
any earlier versions you pass so history can verify the full chain). Each item
falls into one of two buckets:

| Item state or field | Kept in draft? | Reason in report |
|---|---|---|
| Accepted (`in_force`) or `proposed` | Yes | Carried into the draft |
| `rejected` | No | `rejected` |
| Superseded by an `accepted` item | No | `superseded` |
| Superseded only by a `proposed` item | Yes | Kept until the replacement is accepted |
| `open_question` with `status: accepted` and a non-empty `answer` | No | `answered` |
| `critique` with `resolution.outcome` other than `open` | No | `resolved` |
| `critique` with no `resolution` or `resolution.outcome: open` | Yes | Still open |
| `assumption` with `confirmation: refuted` | No | `refuted` |
| `term` with the same name and definition as another kept term | Merged into one | `duplicate` |

If any artifact's latest version is still a `draft`, compaction stops with an
error. Finalize or discard open drafts before compacting.

## Two ways a draft is written

Compaction chooses between continuing an existing artifact and starting a new
one based on how many artifacts of that kind exist in the area.

### One artifact of a kind: continue with the same IDs

When an area has only one `spec` (even across 40 versions), the compacted draft
is simply the next version of that same artifact:

```text
pa_4Hs8… v1 ──► v2 ──► v3 (has rejected D1, answered Q1, term T1)
                         │
                         ▼  compact
                       v4 draft (same ID pa_4Hs8…)
                         parents: previous_version pa_4Hs8…@3
                                  replaces         pa_4Hs8…@3
                         items:   FR2, FR3, Q2  (IDs stay the same)
                         id_counters: max across v1..v3 (no ID reused)
```

Because the artifact ID and item IDs do not change, every reference string in
commits, tickets, and code (`pa_4Hs8…#FR3`) keeps working as-is.

The self-`replaces` parent tells `skopos history` that version 4 is a
compaction. Rejected items stay `rejected` in history, settled items keep their
state, and terms moved into the glossary become `carried` instead of looking
like someone accidentally deleted them (`dropped`).

If a single artifact loses no items, gets no terms, and has no changes, no
draft is written and it appears under `unchanged`.

### Several artifacts of a kind: fold into a new artifact

When parallel work left two or more artifacts of the same kind in one area,
compaction folds their current items into a new version 1 draft:

```text
pa_a001… v1 (D1, D2)      pa_b001… v1 (D1 supersedes a#D2, FR1 refines D1)
        \                        /
         \ replaces    replaces /
          ▼                    ▼
        pa_new1… v1 draft (new artifact)
          D1  ◄── derived_from pa_a001…@1#D1
          D2  ◄── derived_from pa_b001…@1#D1, supersedes pa_a001…@1#D2
          FR1 ◄── derived_from pa_b001…@1#FR1, refines D2  (rewritten!)
```

What happens to links and metadata when items fold together:

- Parents: Each folded artifact becomes a `replaces` parent on the new
  draft. `skopos history` marks their carried items as `carried` and treats the
  old artifacts as retired.
- Rewritten refs: Refs between carried items point to the new artifact and
  new IDs (`FR1` now refines `D2`). Refs into unchanged artifacts or outside
  the area stay intact. `supersedes` refs keep pointing at the original
  superseded item. Other refs to left-out items are removed.
- Participants: Participant tables (`P1`, `P2`, ...) are merged by name and
  kind, and participant IDs in `proposed_by`, `decided_by`, `owner`, `approver`,
  `resolution.decided_by`, and `evidence[].speaker` are remapped so people are
  never mixed up across artifacts.
- Critique targets: Open critiques whose `target` names a carried item get
  their `target` rewritten to the new artifact and item ID.

## How terms and conflicts work

Compaction moves every current `term` item from every kind into the glossary
draft. Existing glossary items always come first so the glossary keeps its own
item IDs.

- Exact duplicates collapse cleanly. If two artifacts define `"Seat"` with
  the same definition (ignoring letter case and outer spaces), one term stays in
  the glossary. If one copy is `accepted` and the other is `proposed`, the
  merged term is `accepted`. Their `aliases`, `code_names`, and `evidence` lists
  are combined, and the kept term carries `derived_from` refs to both sources so
  both old references stay traceable.
- Conflicting definitions stay for review. If two artifacts define `"Plan"`
  differently (or give different `detail` text), both stay in the glossary draft
  as `proposed`, with a `contradicts` ref between them:

```text
glossary draft (pa_9Lp4… v3)
  T4  "Plan"  proposed   "A price list."
  T6  "Plan"  proposed   "What the customer pays for."
        └── derived_from pa_7Wn3…@1#T2
        └── contradicts  T4
```

You resolve term conflicts with the same commands used after `skopos merge`:

```bash
skopos conflicts compacted/glossary.skopos.json
skopos resolve compacted/glossary.skopos.json T4
```

## Repeated compaction and traceability

When an area is compacted more than once, history follows `replaces` parents
and `derived_from` chains across every generation:

```text
Compaction 1                  Later decision                 Compaction 2
────────────                  ──────────────                 ────────────
A#D1 ("Count nightly")                                       E#D3 ("Count at change")
  │ derived_from                                               ▲ derived_from
  ▼                                                            │
C#D1 ◄─────────────────────── supersedes ─────────────────── D#D1
```

When you run `skopos history` or `skopos trace A#D1` over all five documents:

- `A#D1` and `C#D1` both show `superseded` by `D@1#D1` and `E@1#D3`.
- Items that were never superseded (such as `A#D2` carried to `C#D2` and then
  `E#D1`) show `carried` on `A` and `C`, and `in force` on `E`.
- Each compacted item links only to its immediate predecessor (`E` links to `C`,
  `C` links to `A`), so `derived_from` lists stay short after many compactions.

## Cross-area supersession with `--context`

Sometimes a decision in one area supersedes a decision in another area. If you
compact only the first area's files, compaction cannot see the superseding item
in the second area.

Pass the other areas' latest files with `--context` so compaction reads their
`supersedes` links without compacting them:

```bash
skopos compact billing/*.skopos.json \
  --context onboarding/spec.skopos.json \
  --out compacted
```

## Searching current documents and archives

After old versions move into compressed `.skopos.jsonl.zst` archives (JSON Lines
compressed with Zstandard), search reads both current documents and archives
together as a stream.

- Each matching item appears once with its latest known version and status.
- Each hit is labeled with `source` (`current` or `archive`), with current hits
  sorted first.
- If a query only matched an older version before the item was rewritten,
  rejected, or removed, the hit reports `matched_version`, `matched_source`,
  `matched_in_latest: false`, and `removed_in_version` when removed.
- Archives support concatenated Zstandard frames and enforce an 8 MiB window
  limit so browser and WebAssembly readers stay fast and use little memory.

## Using compaction from CLI, Rust, and JavaScript

### CLI

```bash
# Preview the compaction report without writing files
skopos compact billing/*.skopos.json

# Write <kind>.skopos.json drafts into a folder
skopos compact billing/*.skopos.json --out compacted

# Include read-only context from other areas and emit JSON
skopos compact billing/*.skopos.json --context other/spec.skopos.json --json
```

### Rust (`skopos-core`)

```rust
use std::collections::BTreeMap;
use skopos_core::compact::{compact, Options};
use skopos_core::search::search_all;

let options = Options {
    now: Some("2026-11-28T10:00:00Z".into()),
    context: &other_area_docs,
    titles: BTreeMap::from([("spec".into(), "Billing specification".into())]),
};
let result = compact(&area_docs, &options, &mut || new_random_id())?;
let hits = search_all(&current_docs, [archive_bytes.as_slice()], "seat")?;
```

### JavaScript / WebAssembly (`@openskopos/core`)

```js
import { compact, compactText, search, searchArchive } from "@openskopos/core";

const result = compact(areaDocs, {
  context: otherAreaDocs,
  titles: { spec: "Billing specification" },
});
const found = search(currentDocs, [archiveBytes], "seat");
```

See the [CLI reference](../crates/skopos/README.md#compact) and the
[draft 0.3 specification](../spec/0.3/README.md) for full details.
