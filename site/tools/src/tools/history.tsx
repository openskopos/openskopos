// History: several versions of one or more artifacts, read as one story with
// core.history. Which decisions are in force now, which changed, and which
// were superseded, dropped or rejected.
import { useMemo, useState } from "preact/hooks";
import { next, type Document, type History as HistoryResult, type ItemHistory, type ItemRefTo, type ItemState } from "../core";
import { detectFormat, read } from "../doc";
import { historyChain } from "../examples";
import type { Go, Handoff } from "../nav";
import { schemaInfo } from "../schema";
import { copy, DropArea, mdSession, openFiles, Segmented, useStored, type TextFile } from "../ui";

type Filter = "now" | "all" | "changed" | "superseded" | "dropped" | "rejected" | "proposed";
const stateLabel: Record<ItemState, string> = { in_force: "in force", proposed: "proposed", rejected: "rejected", superseded: "superseded", dropped: "dropped" };

export function History(_: { go: Go; incoming?: Handoff }) {
  const [files, setFiles] = useStored<TextFile[]>("history:files", []);
  const [filter, setFilter] = useState<Filter>("now");
  const [type, setType] = useState("");
  const [q, setQ] = useState("");
  const [open, setOpen] = useState<string | null>(null);

  const reads = useMemo(() => files.map((f) => ({ f, r: read(f.text, detectFormat(f.text, f.name), { ...mdSession(), filename: f.name }) })), [files]);
  const docs = reads.filter((x) => x.r.doc).map((x) => x.r.doc!) as Document[];
  const result = useMemo((): { h?: HistoryResult; error?: string } => {
    if (!docs.length || !next.history) return {};
    try {
      return { h: next.history(docs) };
    } catch (e) {
      const err = e as Error & { problems?: { message: string }[] };
      return { error: err.problems?.map((p) => p.message).join("; ") || err.message };
    }
  }, [reads]);
  const add = (fs: TextFile[]) => setFiles([...files, ...fs]);

  if (!next.history)
    return (
      <div class="start">
        <h1>History</h1>
        <p class="muted">History needs a newer version of <code>@openskopos/core</code> than this page loads. It comes with the next release.</p>
      </div>
    );

  const h = result.h;
  const items = h?.items ?? [];
  const info = schemaInfo();
  const needle = q.trim().toLowerCase();
  const shown = items.filter(
    (i) =>
      (filter === "all" || (filter === "now" ? i.state === "in_force" : filter === "changed" ? i.changed : i.state === filter)) &&
      (!type || i.type === type) &&
      (!needle || `${i.id} ${i.text}`.toLowerCase().includes(needle)),
  );
  const count = (f: Filter) => items.filter((i) => (f === "all" ? true : f === "now" ? i.state === "in_force" : f === "changed" ? i.changed : i.state === f)).length;
  const versionsOf = (artifact: string) => h?.chains.find((c) => c.artifact === artifact)?.versions.map((v) => v.version) ?? [];

  return (
    <DropArea class="history" onFiles={add} hint="Drop to add these versions">
      <div class="compare-bar">
        <h1>History</h1>
        <span class="muted">{files.length ? `${files.length} ${files.length === 1 ? "file" : "files"}` : ""}</span>
        <span class="grow" />
        <button type="button" class="btn ghost" onClick={() => setFiles(historyChain)}>
          Try an example
        </button>
        <button type="button" class="btn ghost" onClick={async () => add(await openFiles(true))}>
          Add files
        </button>
        {files.length > 0 && (
          <button type="button" class="btn ghost" onClick={() => setFiles([])}>
            Clear
          </button>
        )}
      </div>

      {!files.length && (
        <div class="blank inline">
          <p class="blank-title">Drop several versions of a document here</p>
          <p class="muted">In any order, and from more than one document. They are sorted into versions, linked to the documents they took as input, and every item gets its story.</p>
          <div class="row center wrap">
            <button type="button" class="btn" onClick={async () => add(await openFiles(true))}>
              Open files
            </button>
            <button type="button" class="btn ghost" onClick={() => setFiles(historyChain)}>
              Try an example
            </button>
          </div>
        </div>
      )}

      {reads.some((x) => !x.r.doc) && (
        <p class="bad-text">
          Core cannot read {reads.filter((x) => !x.r.doc).map((x) => x.f.name || "a pasted file").join(", ")}. Open it in Validate to see why.
        </p>
      )}
      {result.error && <p class="bad-text">{result.error}</p>}

      {h && (
        <>
          <div class="chains">
            {h.chains.map((c) => (
              <div class="chain">
                <div class="chain-head">
                  <span class="chain-title" title={c.artifact}>{c.title}</span>
                  <span class="muted small-text">{c.kind.replace(/_/g, " ")}</span>
                </div>
                <ol class="versions">
                  {c.versions.map((v) => (
                    <li class={`version st-${v.status}`}>
                      <span class="vnum">v{v.version}</span>
                      <span class="muted">{v.status}</span>
                      <span class="vcounts">
                        {v.diff ? (
                          <>
                            {v.diff.added.length > 0 && <span class="c-added">+{v.diff.added.length}</span>}
                            {v.diff.changed.length > 0 && <span class="c-changed">~{v.diff.changed.length}</span>}
                            {v.diff.superseded.length > 0 && <span class="c-superseded">⇢{v.diff.superseded.length}</span>}
                            {v.diff.dropped.length > 0 && <span class="c-dropped">−{v.diff.dropped.length}</span>}
                            {!v.diff.added.length && !v.diff.changed.length && !v.diff.superseded.length && !v.diff.dropped.length && <span class="muted">no change</span>}
                          </>
                        ) : (
                          <span class="muted">{v.items} items</span>
                        )}
                      </span>
                    </li>
                  ))}
                </ol>
              </div>
            ))}
          </div>
          <p class="muted small-text legend">● final version · ○ draft · +added ~changed −dropped</p>
          {h.warnings.length > 0 && (
            <div class="warnings muted small-text">
              {h.warnings.map((w) => (
                <div>{w.message}</div>
              ))}
            </div>
          )}

          <div class="history-filters">
            <Segmented
              label="Items"
              value={filter}
              onChange={setFilter}
              options={(["now", "changed", "superseded", "dropped", "rejected", "proposed", "all"] as Filter[]).filter((f) => f === filter || f === "all" || f === "now" || count(f) > 0).map((f) => ({
                value: f,
                label: `${f === "now" ? "In force now" : f[0].toUpperCase() + f.slice(1)} ${count(f)}`,
              }))}
            />
            <select aria-label="Item type" value={type} onChange={(e) => setType(e.currentTarget.value)}>
              <option value="">All types</option>
              {info.types
                .filter((t) => items.some((i) => i.type === t.type))
                .map((t) => (
                  <option value={t.type}>{t.plural}</option>
                ))}
            </select>
            <input type="search" class="filter" placeholder="Filter by ID or text" value={q} onInput={(e) => setQ(e.currentTarget.value)} aria-label="Filter items" />
          </div>

          <ol class="item-rows">
            {shown.map((i, n) => (
              <>
                {h.chains.length > 1 && i.artifact !== shown[n - 1]?.artifact && <li class="row-group">{h.chains.find((c) => c.artifact === i.artifact)?.title ?? i.artifact}</li>}
                <ItemRow item={i} versions={versionsOf(i.artifact)} showState={filter !== "now"} open={open === i.key} toggle={() => setOpen(open === i.key ? null : i.key)} />
              </>
            ))}
            {!shown.length && <li class="muted pad">No items match.</li>}
          </ol>
        </>
      )}
    </DropArea>
  );
}

const refText = (r: ItemRefTo | string) => (typeof r === "string" ? r : `${r.artifact}${r.version ? `@${r.version}` : ""}#${r.item}`);

function ItemRow({ item, versions, showState, open, toggle }: { item: ItemHistory; versions: number[]; showState: boolean; open: boolean; toggle: () => void }) {
  const byVersion = new Map(item.steps.map((s) => [s.version, s]));
  return (
    <li class={`item-row state-${item.state} ${open ? "open" : ""}`}>
      <button type="button" class="item-line" onClick={toggle} aria-expanded={open}>
        <span class="id">{item.id}</span>
        <span class="txt">{item.text}</span>
        <span class="timeline" aria-label={`Life across versions ${versions.join(", ")}`}>
          {versions.map((v) => {
            const s = byVersion.get(v);
            const present = v >= item.first_version && (item.state !== "dropped" && item.state !== "superseded" ? true : v <= item.last_version);
            const kind = s ? s.change : present ? "seen" : "none";
            return <i class={`tick t-${kind}`} title={`v${v}: ${s ? s.change : present ? "carried over" : "not in this version"}`} />;
          })}
        </span>
        {showState && <span class={`pill state ${item.state}`}>{stateLabel[item.state]}</span>}
      </button>
      {open && (
        <div class="story">
          <div class="row wrap">
            <code>{item.reference}</code>
            <button type="button" class="btn ghost small" onClick={() => copy(item.reference, "Reference copied")}>
              Copy reference
            </button>
          </div>
          <ol class="steps">
            {item.steps.map((s) => (
              <li class={`step t-${s.change}`}>
                <span class="vnum">v{s.version}</span>
                <div>
                  <b>{s.change === "seen" ? "first seen" : s.change}</b>
                  {s.fields && s.fields.length > 0 && <span class="muted"> {s.fields.join(", ")}</span>}
                  {s.status_from && s.status_to && (
                    <span class="muted">
                      {" "}
                      · status {s.status_from} → {s.status_to}
                    </span>
                  )}
                  {s.by && s.by.length > 0 && <span class="muted"> by {s.by.join(", ")}</span>}
                  {s.item && typeof s.item.text === "string" && <div class="step-text">{s.item.text}</div>}
                </div>
              </li>
            ))}
          </ol>
          {[
            ["Supersedes", item.supersedes],
            ["Superseded by", item.superseded_by],
            ["Derived from", item.derived_from],
            ["Carried into", item.derived_into],
          ]
            .filter(([, l]) => (l as unknown[]).length)
            .map(([label, l]) => (
              <div class="muted">
                {label as string}: {(l as (ItemRefTo | string)[]).map((r) => <code>{refText(r)} </code>)}
              </div>
            ))}
        </div>
      )}
    </li>
  );
}
