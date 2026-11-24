// Compare: two versions of a document, compared by item ID with core.diff.
import { useEffect, useMemo } from "preact/hooks";
import { core, type Diff, type Document, type Item } from "../core";
import { versionPair } from "../examples";
import type { Go, Handoff } from "../nav";
import { readShared, shareUrl } from "../share";
import { copy, DocInput, Readable, Segmented, Spans, useRead, useStored, type TextFile } from "../ui";
import type { TextSpan } from "../core";

const empty: TextFile = { name: "", text: "" };

export function Compare({ incoming }: { go: Go; incoming?: Handoff }) {
  const [older, setOlder] = useStored<TextFile>("compare:old", empty);
  const [newer, setNewer] = useStored<TextFile>("compare:new", empty);
  const [view, setView] = useStored<"changes" | "readable">("compare:view", "readable");
  const a = useRead(older);
  const b = useRead(newer);

  useEffect(() => void (incoming && setOlder(incoming.file)), [incoming?.n]);
  useEffect(() => {
    readShared<{ old: TextFile; new: TextFile }>().then((s) => {
      if (s?.old && s?.new) {
        setOlder(s.old);
        setNewer(s.new);
      }
    });
  }, []);

  const result = useMemo(() => {
    if (!a?.doc || !b?.doc) return undefined;
    try {
      return { diff: core.diff(a.doc, b.doc), text: core.diffText(a.doc, b.doc) };
    } catch (e) {
      return { error: (e as Error).message };
    }
  }, [a?.doc, b?.doc]);

  return (
    <div class="compare">
      <div class="compare-bar">
        <h1>Compare two versions</h1>
        <span class="grow" />
        <button
          type="button"
          class="btn ghost"
          onClick={() => {
            setOlder(versionPair.old);
            setNewer(versionPair.new);
          }}
        >
          Try an example
        </button>
        <button type="button" class="btn ghost" disabled={!older.text.trim() || !newer.text.trim()} onClick={() => {
            setOlder(newer);
            setNewer(older);
          }}>
          Swap
        </button>
        {result && "diff" in result && (
          <button type="button" class="btn ghost" onClick={async () => copy(await shareUrl({ old: older, new: newer }), "Link copied. The documents are in the link, not on a server.")}>
            Share
          </button>
        )}
      </div>
      <div class="compare-inputs">
        <DocInput title="Older version" hint="Paste, drop or open the earlier version" value={older} onChange={setOlder} result={a} height="220px" />
        <DocInput title="Newer version" hint="Paste, drop or open the later version" value={newer} onChange={setNewer} result={b} height="220px" />
      </div>
      {!result && <p class="muted center-text">Add two versions of one document. Their items are compared by ID, the same as <code>skopos diff</code>.</p>}
      {result && "error" in result && <p class="bad-text">{result.error}</p>}
      {result && "diff" in result && (
        <Segmented
          label="Show"
          value={view}
          onChange={setView}
          options={[
            { value: "readable", label: "Changes marked" },
            { value: "changes", label: "List" },
          ]}
        />
      )}
      {result && "diff" in result && view === "readable" && (
        <div class="diff">
          <Readable doc={b!.doc!} previous={a!.doc!} />
        </div>
      )}
      {result && "diff" in result && result.diff && view !== "readable" && <DiffView diff={result.diff} text={result.text!} oldDoc={a!.doc!} newDoc={b!.doc!} />}
    </div>
  );
}

function DiffView({ diff, text, oldDoc, newDoc }: { diff: Diff; text: string; oldDoc: Document; newDoc: Document }) {
  const byId = (d: Document, id: string) => ((d.items ?? []) as Item[]).find((i) => i.id === id);
  const counts = [
    ["added", diff.added.length],
    ["changed", diff.changed.length],
    ["superseded", diff.superseded.length],
    ["dropped", diff.dropped.length],
  ] as const;
  const nothing = counts.every(([, n]) => n === 0) && !diff.envelope.length;
  return (
    <div class="diff">
      <div class="diff-head">
        <b title={`${diff.from.artifact} → ${diff.to.artifact}`}>
          Version {diff.from.version ?? "?"} → version {diff.to.version ?? "?"}
        </b>
        {!diff.same_artifact && <span class="pill proposed">different documents</span>}
        <span class="grow" />
        {counts.filter(([, n]) => n > 0).map(([k, n]) => (
          <span class={`count ${k} ${n ? "" : "zero"}`}>
            <b>{n}</b> {k}
          </span>
        ))}
        <button type="button" class="btn ghost small" onClick={() => copy(text)}>
          Copy as text
        </button>
      </div>
      {nothing && <p class="muted">The two versions have the same items.</p>}
      {diff.envelope.length > 0 && (
        <p class="muted">
          Document fields that changed: {diff.envelope.map((f) => <code>{f}</code>).reduce((acc: preact.ComponentChildren[], el, i) => (i ? [...acc, ", ", el] : [el]), [])}
        </p>
      )}
      <ul class="diff-list">
        {diff.added.map((i) => (
          <li class="added">
            <span class="diff-kind">added</span>
            <span class="id">{i.id}</span>
            <span class="txt">{i.text}</span>
          </li>
        ))}
        {diff.changed.map((i) => {
          const before = byId(oldDoc, i.id);
          const after = byId(newDoc, i.id);
          return (
            <li class="changed">
              <span class="diff-kind">changed</span>
              <span class="id">{i.id}</span>
              <div class="txt">
                {i.status_from && i.status_to && (
                  <div class="muted small-text">
                    status {i.status_from} → {i.status_to}
                  </div>
                )}
                {i.fields.filter((f) => f !== "status").map((f) => {
                  const spans = (f === "text" ? (i as { text_spans?: TextSpan[] }).text_spans : (i as { field_spans?: Record<string, TextSpan[]> }).field_spans?.[f]);
                  return (
                    <div class="change-one">
                      {f !== "text" && <span class="muted small-text">{f.replace(/_/g, " ")}</span>}
                      <div>{spans ? <Spans spans={spans} /> : <>{show(before?.[f])} <span class="muted">→</span> {show(after?.[f])}</>}</div>
                    </div>
                  );
                })}
              </div>
            </li>
          );
        })}
        {diff.superseded.map((i) => (
          <li class="superseded">
            <span class="diff-kind">superseded</span>
            <span class="id">{i.id}</span>
            <span class="txt">
              <s>{i.text}</s> <span class="muted">by</span> {i.by.map((x) => <b class="mono">{x} </b>)}
            </span>
          </li>
        ))}
        {diff.dropped.map((i) => (
          <li class="dropped">
            <span class="diff-kind">dropped</span>
            <span class="id">{i.id}</span>
            <span class="txt">
              <s>{i.text}</s>
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}

const show = (v: unknown) => (v === undefined ? <span class="muted">(none)</span> : typeof v === "string" ? v : <code>{JSON.stringify(v)}</code>);
