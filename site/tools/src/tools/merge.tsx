// Merge: two drafts of a document into the next version of one of them, with
// core.merge. Conflicts stay in the document as pairs of proposed items; the
// page settles each pair with core.resolve, or by setting both to accepted.
import { useEffect, useMemo, useState } from "preact/hooks";
import { core, next, type Document, type Item, type MergeConflict, type MergeResult, type TextSpan } from "../core";
import { fileName, formatMime, write } from "../doc";
import { mergeCases } from "../examples";
import type { Go, Handoff } from "../nav";
import { copy, DocInput, download, Menu, Readable, hasReadable, Spans, useRead, useStored, type TextFile } from "../ui";

const empty: TextFile = { name: "", text: "" };
const exampleNames: Record<string, string> = {
  "clean-three-way": "Two edits that merge cleanly",
  "edit-edit": "Both edited the same item",
  "edit-supersede": "One edited, one replaced the item",
  "id-collision": "Both added an item with the same ID",
  "two-drafts": "Two separate drafts, no common base",
};
type Choice = "main" | "other" | "both";

export function Merge({ go }: { go: Go; incoming?: Handoff }) {
  const [main, setMain] = useStored<TextFile>("merge:main", empty);
  const [other, setOther] = useStored<TextFile>("merge:other", empty);
  const [base, setBase] = useStored<TextFile>("merge:base", empty);
  const [useBase, setUseBase] = useStored<boolean>("merge:useBase", false);
  const a = useRead(main);
  const b = useRead(other);
  const c = useRead(base);
  /** Pairs the person turned off, when core paired items by text. */
  const [unpaired, setUnpaired] = useState<string[]>([]);
  const [choices, setChoices] = useState<Record<number, Choice>>({});
  const [at, setAt] = useState(0);

  const ready = a?.doc && b?.doc && !a.problems.length && !b.problems.length;
  const baseDoc = useBase && c?.doc && !c.problems.length ? c.doc : undefined;

  // First run: core proposes the pairs. Later runs pass the pairs that stay on.
  const first = useMemo((): { r?: MergeResult; error?: string } => {
    if (!ready || !next.merge) return {};
    try {
      return { r: next.merge(a!.doc!, b!.doc!, { base: baseDoc }) };
    } catch (e) {
      return { error: errorText(e) };
    }
  }, [a?.doc, b?.doc, baseDoc]);
  const byText = !baseDoc && (first.r?.pairs.some((p) => p.by === "text") ?? false);
  const result = useMemo(() => {
    if (!first.r || !unpaired.length || !byText) return first;
    try {
      const pairs = first.r.pairs.filter((p) => !unpaired.includes(`${p.main}=${p.other}`)).map((p) => [p.main, p.other] as [string, string]);
      return { r: next.merge!(a!.doc!, b!.doc!, { pairs }) };
    } catch (e) {
      return { error: errorText(e) };
    }
  }, [first, unpaired]);
  useEffect(() => {
    setChoices({});
    setAt(0);
  }, [result]);
  useEffect(() => setUnpaired([]), [first]);

  // The document with every choice applied, in order.
  const settled = useMemo(() => {
    const r = result.r;
    if (!r) return undefined;
    let doc = r.doc;
    for (const [i, choice] of Object.entries(choices)) {
      const k = r.conflicts[Number(i)];
      if (!k) continue;
      try {
        if (choice === "both") doc = acceptBoth(doc, k);
        else {
          const id = choice === "main" ? k.main : k.other;
          if (!id) continue;
          if (k.main && k.other) doc = next.resolve!(doc, id).doc;
          else doc = acceptBoth(doc, k);
        }
      } catch {}
    }
    return doc;
  }, [result, choices]);

  const conflicts = result.r?.conflicts ?? [];
  const left = conflicts.filter((_, i) => !choices[i]).length;
  const problems = useMemo(() => (settled ? core.check(settled) : []), [settled]);
  const done = !!settled && !left && !problems.length;

  // Keyboard: j and k move between conflicts, 1, 2 and 3 choose.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!conflicts.length || document.getElementById("tool-merge")?.hidden) return;
      const t = e.target as HTMLElement;
      if (t.closest("input, textarea, select, .cm-editor")) return;
      if (e.key === "j") setAt((x) => Math.min(conflicts.length - 1, x + 1));
      else if (e.key === "k") setAt((x) => Math.max(0, x - 1));
      else if (["1", "2", "3"].includes(e.key)) {
        const choice = (["main", "other", "both"] as Choice[])[Number(e.key) - 1];
        setChoices((cs) => ({ ...cs, [at]: choice }));
        setAt((x) => Math.min(conflicts.length - 1, x + 1));
      } else return;
      e.preventDefault();
    };
    addEventListener("keydown", onKey);
    return () => removeEventListener("keydown", onKey);
  }, [conflicts, at]);
  useEffect(() => document.querySelector(`[data-conflict="${at}"]`)?.scrollIntoView({ block: "nearest", behavior: "smooth" }), [at]);

  if (!next.merge)
    return (
      <div class="start">
        <h1>Merge</h1>
        <p class="muted">Merge needs a newer version of <code>@openskopos/core</code> than this page loads. It comes with the next release.</p>
      </div>
    );

  const choose = (i: number, c: Choice) => {
    setChoices({ ...choices, [i]: c });
    const nextOpen = conflicts.findIndex((_, k) => k > i && !choices[k]);
    if (nextOpen >= 0) setAt(nextOpen);
  };
  const items = (d: Document | undefined) => ((d?.items ?? []) as Item[]);
  const find = (d: Document | undefined, id: string | null) => (id ? items(d).find((i) => i.id === id) : undefined);
  const name = fileName(settled, "json");

  return (
    <div class="merge">
      <div class="compare-bar">
        <h1>Merge</h1>
        <span class="grow" />
        {mergeCases.length > 0 && (
          <label class="btn ghost select">
            <span>Try an example</span>
            <select
              aria-label="Try an example"
              value=""
              onChange={(e) => {
                const m = mergeCases.find((x) => x.name === e.currentTarget.value);
                e.currentTarget.value = "";
                if (!m) return;
                setMain(m.main);
                setOther(m.other);
                setBase(m.base.text ? m.base : empty);
                setUseBase(!!m.base.text);
              }}
            >
              <option value="">Try an example</option>
              {mergeCases.map((m) => (
                <option value={m.name}>{exampleNames[m.name.replace(/^\d+-/, "")] ?? m.name.replace(/^\d+-/, "").replace(/-/g, " ")}</option>
              ))}
            </select>
          </label>
        )}
        <button
          type="button"
          class="btn ghost"
          disabled={!main.text.trim() || !other.text.trim()}
          onClick={() => {
            setMain(other);
            setOther(main);
          }}
        >
          Swap
        </button>
      </div>
      <div class="compare-inputs">
        <DocInput title="Main" hint="The result becomes its next version" value={main} onChange={setMain} result={a} height="180px" />
        <DocInput title="Other" hint="The draft to bring in" value={other} onChange={setOther} result={b} height="180px" />
      </div>
      <label class="row base-toggle">
        <input type="checkbox" checked={useBase} onChange={(e) => setUseBase(e.currentTarget.checked)} />
        <span title="Both drafts started from one earlier version. Items then pair by ID.">Use a common base</span>
      </label>
      {useBase && <DocInput title="Base" hint="The earlier version both drafts started from" value={base} onChange={setBase} result={c} height="140px" />}

      {!ready && <p class="muted center-text">Add two drafts to merge.</p>}
      {result.error && <p class="bad-text">{result.error}</p>}

      {result.r && settled && (
        <>
          {byText && (
            <details class="diff pairs-box">
              <summary>
                {first.r!.pairs.length} {first.r!.pairs.length === 1 ? "item" : "items"} matched by similar text
              </summary>
              <ul class="pairs">
                {first.r!.pairs.map((p) => {
                  const key = `${p.main}=${p.other}`;
                  const on = !unpaired.includes(key);
                  return (
                    <li>
                      <label class="row">
                        <input type="checkbox" checked={on} onChange={() => setUnpaired(on ? [...unpaired, key] : unpaired.filter((x) => x !== key))} />
                        <b class="mono">{p.main}</b> <span class="muted">and other's</span> <b class="mono">{p.other}</b>
                        <span class="pair-text">{String(find(a!.doc, p.main)?.text ?? "")}</span>
                      </label>
                    </li>
                  );
                })}
              </ul>
            </details>
          )}

          <div class="merge-progress">
            <div class="bar" role="progressbar" aria-valuemin={0} aria-valuemax={conflicts.length} aria-valuenow={conflicts.length - left}>
              <i style={{ width: conflicts.length ? `${(100 * (conflicts.length - left)) / conflicts.length}%` : "100%" }} />
            </div>
            <span>{conflicts.length ? (left ? `${left} ${left === 1 ? "conflict" : "conflicts"} left` : "All settled") : "No conflicts"}</span>
          </div>

          <ol class="conflicts">
            {conflicts.map((k, i) => {
              const m = find(result.r!.doc, k.main);
              const o = find(result.r!.doc, k.other);
              const choice = choices[i];
              return (
                <li class={`conflict ${i === at ? "current" : ""} ${choice ? "settled" : ""}`} data-conflict={i} onClick={() => setAt(i)}>
                  <div class="conflict-head">
                    <span class="muted">{kindText(k)}</span>
                    <span class="grow" />
                    {choice ? (
                      <button type="button" class="link" onClick={() => setChoices(omit(choices, i))}>
                        Undo
                      </button>
                    ) : (
                      m &&
                      o && (
                        <button type="button" class="link" title="Keep both (3)" onClick={() => choose(i, "both")}>
                          Keep both
                        </button>
                      )
                    )}
                  </div>
                  {k.base_text !== undefined && <div class="was"><span class="eyebrow">Base</span> {k.base_text}</div>}
                  <div class="sides">
                    <Side label="Main" item={m} spans={k.text_spans} side="old" picked={choice === "main" || choice === "both"} dropped={choice === "other"} keep={() => choose(i, "main")} keyHint="1" />
                    <Side label="Other" item={o} spans={k.text_spans} side="new" picked={choice === "other" || choice === "both"} dropped={choice === "main"} keep={() => choose(i, "other")} keyHint="2" />
                  </div>
                </li>
              );
            })}
          </ol>

          <section class="diff merge-result">
            <div class="diff-head">
              <span class={`status ${done ? "valid" : problems.length ? "invalid" : "empty"}`}>
                <i />
                {done ? "Ready" : problems.length ? `${problems.length} problems` : "Pick a side for each conflict"}
              </span>
              <span class="grow" />
              <Menu
                items={[
                  { label: "Copy summary", run: () => copy(next.mergeText!(a!.doc!, b!.doc!, { base: baseDoc })) },
                  { label: "Edit in Author", run: () => go("author", { name, text: write(settled, "yaml") }) },
                ]}
              />
              <button type="button" class="btn small" disabled={!done} onClick={() => download(name, write(settled, "json"), formatMime.json)}>
                Download
              </button>
            </div>
            {hasReadable() && <Readable doc={settled} previous={a!.doc!} />}
          </section>
        </>
      )}
    </div>
  );
}

function Side({ label, item, spans, side, picked, dropped, keep, keyHint }: { label: string; item?: Item; spans?: TextSpan[]; side: "old" | "new"; picked: boolean; dropped: boolean; keep: () => void; keyHint: string }) {
  return (
    <button type="button" class={`side ${picked ? "picked" : ""} ${dropped ? "dropped" : ""}`} disabled={!item} onClick={keep} title={item ? `Keep ${label.toLowerCase()} (${keyHint})` : undefined}>
      <span class="eyebrow">
        {label} {item?.id && <span class="mono">{String(item.id)}</span>}
      </span>
      {item ? <span>{spans ? <Spans spans={spans} side={side} /> : String(item.text ?? "")}</span> : <span class="muted">Removed</span>}
    </button>
  );
}

const kindText = (k: MergeConflict) => {
  if (k.kind === "removed_in_other") return "Other removed it, main changed it";
  if (k.kind === "removed_in_main") return "Main removed it, other changed it";
  const fields = k.fields.filter((f) => f !== "text");
  return fields.length ? `Both changed ${[...(k.fields.includes("text") ? ["text"] : []), ...fields].join(", ").replace(/_/g, " ")}` : "Both changed the text";
};

/** Settles a pair by keeping both items: sets their status to accepted. */
function acceptBoth(doc: Document, k: MergeConflict): Document {
  const d = structuredClone(doc);
  for (const it of (d.items ?? []) as Item[]) if (it.id === k.main || it.id === k.other) it.status = "accepted";
  return d;
}

const omit = <T,>(o: Record<number, T>, k: number) => Object.fromEntries(Object.entries(o).filter(([x]) => Number(x) !== k)) as Record<number, T>;
const errorText = (e: unknown) => {
  const err = e as Error & { problems?: { message: string }[] };
  return err.problems?.map((p) => p.message).join("; ") || err.message;
};
