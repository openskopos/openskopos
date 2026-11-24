// Author: item cards and the source text side by side, kept in sync. Core
// reads and writes the text, gives new items their IDs, and checks the
// document on every change.
import { useEffect, useMemo, useRef, useState } from "preact/hooks";
import { core, type Document, type Item, type Problem } from "../core";
import { Editor, type EditorHandle } from "../editor";
import { detectFormat, fileName, formatLabel, formatMime, formats, itemIndex, itemRanges, locate, read, write, type Format, type Mark } from "../doc";
import { schemaInfo, type Field, type ItemTypeInfo } from "../schema";
import { readShared, shareUrl } from "../share";
import type { Go, Handoff } from "../nav";
import { copy, download, DropArea, ExamplePicker, FixButton, Menu, openFiles, Readable, Segmented, Status, mdSession, toast, useMedia, useStored, type TextFile } from "../ui";

type View = "both" | "cards" | "text";

interface Source {
  name: string;
  format: Format;
  text: string;
}

const kinds = ["intent", "spec", "agent_rules", "critique", "glossary"];

/** The item types that the add bar offers first, by the kind of document. */
const suggested: Record<string, string[]> = {
  intent: ["goal", "success_signal", "requirement", "constraint", "decision"],
  spec: ["requirement", "story", "constraint", "decision", "open_question"],
  agent_rules: ["constraint", "term", "decision"],
  critique: ["critique"],
  glossary: ["term"],
};

/** A problem message in plain words where the page knows a better one. */
function friendly(message: string, path?: string) {
  const field = path?.split("/").pop();
  if (/is shorter than 1 character/.test(message) && field) return field === "text" ? "Write this item in one sentence." : `Fill in ${field.replace(/_/g, " ")}.`;
  const req = /^"(\w+)" is a required property$/.exec(message);
  if (req) return `Add ${req[1].replace(/_/g, " ")}.`;
  return message.charAt(0).toUpperCase() + message.slice(1);
}

function blank(kind = "intent"): Document {
  return {
    $schema: core.schemaUrl(),
    id: core.newArtifactId(),
    version: 1,
    kind,
    title: kind === "agent_rules" ? "Agent rules" : `New ${kind}`,
    status: "draft",
    items: [],
    created_at: new Date().toISOString().replace(/\.\d+Z$/, "Z"),
  };
}

export function Author({ go, incoming }: { go: Go; incoming?: Handoff }) {
  const narrow = useMedia("(max-width: 900px)");
  const [storedView, setView] = useStored<View>("author:view", "both");
  const view: View = narrow && storedView === "both" ? "cards" : storedView;
  const [src, setSrc] = useStored<Source>("author:source", { name: "", format: "yaml", text: "" });
  const [doc, setDoc] = useState<Document | undefined>(() => (src.text ? read(src.text, src.format, mdSession()).doc : undefined));
  const [readErrors, setReadErrors] = useState<Mark[]>([]);
  const [selected, setSelectedRaw] = useState<number | null>(null);
  /** The item that was just added and not touched yet: no error on it until its card closes. */
  const [pristine, setPristine] = useState<number | null>(null);
  const setSelected = (i: number | null) => {
    if (i !== pristine) setPristine(null);
    setSelectedRaw(i);
  };
  const [filter, setFilter] = useState("");
  const [textSide, setTextSide] = useStored<"source" | "readable">("author:text", "source");
  const readable = textSide === "readable" && !!doc;
  const editor = useRef<EditorHandle | null>(null);
  const written = useRef(src.text);

  // Text typed in the editor: read it after a short pause, and at least
  // every 600 ms while the person keeps typing.
  const pendingSince = useRef<number | null>(null);
  useEffect(() => {
    if (src.text === written.current) return;
    pendingSince.current ??= Date.now();
    const wait = Math.max(0, Math.min(150, pendingSince.current + 600 - Date.now()));
    const t = setTimeout(() => {
      pendingSince.current = null;
      if (!src.text.trim()) {
        setDoc(undefined);
        setReadErrors([]);
        return;
      }
      const r = read(src.text, src.format, { ...mdSession(), filename: src.name });
      if (r.doc) {
        setDoc(r.doc);
        setReadErrors([]);
      } else setReadErrors(r.marks);
    }, wait);
    return () => clearTimeout(t);
  }, [src.text, src.format]);

  /** Loads text from a file, an example, another tool or a link. */
  function load(f: TextFile, undoable = false) {
    const before = src;
    if (undoable && before.text.trim() && before.text !== f.text)
      toast(`Opened ${f.name || "the file"}`, { label: "Undo", run: () => load({ name: before.name, text: before.text }) });
    const format = detectFormat(f.text, f.name);
    const r = read(f.text, format, { ...mdSession(), filename: f.name });
    written.current = f.text;
    setSrc({ name: f.name, format, text: f.text });
    setDoc(r.doc);
    setReadErrors(r.doc ? [] : r.marks);
    setSelected(null);
  }
  useEffect(() => void (incoming && load(incoming.file)), [incoming?.n]);
  useEffect(() => {
    readShared<TextFile>().then((f) => f && load(f));
  }, []);

  /** Sets the document from the cards, and writes the text again. */
  function commit(next: Document, format = src.format) {
    let text: string;
    try {
      text = write(next, format);
    } catch {
      text = write(next, "yaml");
      format = "yaml";
    }
    written.current = text;
    setDoc(next);
    setReadErrors([]);
    setSrc({ ...src, format, text });
  }

  const allProblems = useMemo(() => (doc ? (core.check(doc) as Problem[]) : []), [doc]);
  const problems = pristine === null ? allProblems : allProblems.filter((p) => itemIndex(p.path) !== pristine);
  const marks: Mark[] = useMemo(() => {
    if (readErrors.length) return readErrors;
    if (src.text !== written.current && src.format === "markdown") return [];
    if (src.format === "markdown") return [];
    const at = locate(src.text, problems.map((p) => p.path));
    return problems.map((p, i) => ({ ...at[i], message: p.message, severity: "error" as const, rule: p.rule, path: p.path }));
  }, [problems, readErrors, src.text]);

  // Scroll the text to the card that the person opened.
  useEffect(() => {
    if (selected === null || view !== "both" || src.format === "markdown") return;
    const r = itemRanges(src.text)[selected];
    if (r) editor.current?.show(r.from);
  }, [selected]);

  const verdict = !doc ? (readErrors.length ? "unreadable" : "empty") : readErrors.length ? "unreadable" : problems.length ? "invalid" : "valid";
  const stale = readErrors.length > 0;

  if (!doc && !src.text.trim()) return <Start onLoad={load} onNew={(kind) => commit(blank(kind), src.format)} />;

  const name = fileName(doc, src.format);
  function showFirstProblem() {
    const p = problems[0];
    if (!p) return;
    const i = itemIndex(p.path);
    if (view !== "text" && i !== null) {
      setFilter("");
      setSelected(i);
    }
    const m = marks.find((x) => x.path === p.path);
    if (view !== "cards" && m) editor.current?.reveal(m.from, m.to);
  }
  const viewOptions = narrow
    ? [
        { value: "cards" as View, label: "Cards" },
        { value: "text" as View, label: "Source" },
      ]
    : [
        { value: "both" as View, label: "Both" },
        { value: "cards" as View, label: "Cards" },
        { value: "text" as View, label: "Source" },
      ];

  return (
    <DropArea class="author" onFiles={(f) => f[0] && load(f[0])} hint="Drop to open this file">
      <div class="author-bar">
        {view === "text" && <span class="author-name">{typeof doc?.title === "string" && doc.title ? doc.title : "Untitled"}</span>}
        {problems.length > 0 && !stale ? (
          <button type="button" class="status-button" onClick={showFirstProblem} title="Show the first problem">
            <Status verdict={verdict} count={problems.length} />
          </button>
        ) : (
          <Status verdict={verdict} count={problems.length} />
        )}
        <span class="grow" />
        <Segmented label="Show" value={view} onChange={setView} options={viewOptions} />
        <ExamplePicker onPick={(e) => load({ name: e.name, text: e.text }, true)} filter={(e) => !e.label.startsWith("Invalid")} />
        <Menu
          items={[
            { label: "Open a file", run: async () => (await openFiles()).slice(0, 1).forEach((f) => load(f, true)) },
            {
              label: "New document",
              run: () => {
                const before = src;
                commit(blank(), src.format);
                toast("Started a new document", { label: "Undo", run: () => load({ name: before.name, text: before.text }) });
              },
            },
            { label: "Share a link", run: async () => copy(await shareUrl({ name, text: src.text }), "Link copied. The document is in the link, not on a server.") },
            { label: "Copy the text", run: () => copy(src.text) },
            { label: "Open in Validate", run: () => go("validate", { name, text: src.text }) },
          ]}
        />
        <button type="button" class="btn" onClick={() => download(name, src.text, formatMime[src.format])}>
          Download {name.slice(name.indexOf("."))}
        </button>
      </div>

      <div class={`author-body view-${view}`}>
        {view !== "text" && (
          <div class="pane cards-pane">
            {stale && (
              <div class="stale">
                The text cannot be read right now, so the cards show the last version that could. Fix the text, or{" "}
                <button type="button" class="link" onClick={() => doc && commit(doc)}>
                  go back to this version
                </button>
                .
              </div>
            )}
            {doc && (
              <Cards
                doc={doc}
                problems={problems}
                pristine={pristine}
                setPristine={setPristine}
                readOnly={stale}
                selected={selected}
                select={setSelected}
                filter={filter}
                setFilter={setFilter}
                commit={commit}
              />
            )}
          </div>
        )}
        {view !== "cards" && (
          <div class="pane text-pane">
            <div class="pane-head">
              <Segmented
                label="Text or readable view"
                value={textSide}
                onChange={setTextSide}
                options={[
                  { value: "source", label: "Source" },
                  { value: "readable", label: "Readable" },
                ]}
              />
              {!readable && <Segmented
                label="Text format"
                value={src.format}
                onChange={(f) => (doc && !stale ? commit(doc, f) : setSrc({ ...src, format: f }))}
                options={formats.map((f) => ({ value: f, label: formatLabel[f] }))}
              />}
              <span class="grow" />
            </div>
            {readable && (
              <div class="pane-body scroll">
                <Readable doc={doc!} />
              </div>
            )}
            <div class="pane-body" hidden={readable}>
              <Editor
                label="Document source"
                value={src.text}
                format={src.format}
                marks={marks}
                onChange={(text) => setSrc({ ...src, text })}
                handle={(h) => (editor.current = h)}
                onCursor={(pos) => {
                  if (src.format === "markdown" || view !== "both") return;
                  const i = itemRanges(src.text).findIndex((r) => pos >= r.from && pos <= r.to);
                  if (i >= 0 && i !== selected) setSelected(i);
                }}
                complete
              />
            </div>
            {!readable && <ProblemStrip marks={marks} doc={doc} reveal={(m) => editor.current?.reveal(m.from, m.to)} />}
          </div>
        )}
      </div>
    </DropArea>
  );
}

function Start({ onLoad, onNew }: { onLoad: (f: TextFile) => void; onNew: (kind: string) => void }) {
  return (
    <DropArea class="start" onFiles={(f) => f[0] && onLoad(f[0])} hint="Drop to open this file">
      <h1>Write an OpenSkopos document</h1>
      <p class="muted">Add goals, requirements, rules and decisions as cards. The text next to them is the file, in YAML, JSON or Markdown. The document is checked on every change, and each item gets its ID.</p>
      <div class="start-kinds">
        {kinds.map((k) => (
          <button type="button" class="kind" onClick={() => onNew(k)}>
            <span class="name">{k.replace("_", " ")}</span>
            <span class="muted">
              {
                {
                  intent: "Why the work exists and how the team knows it is done",
                  spec: "The behavior that the work must have",
                  agent_rules: "Rules for agents that work in a project",
                  critique: "Problems that a team found in an input",
                  glossary: "Terms with agreed meanings",
                }[k]
              }
            </span>
          </button>
        ))}
      </div>
      <div class="row center">
        <span class="muted">or</span>
        <button type="button" class="btn ghost" onClick={async () => (await openFiles()).slice(0, 1).forEach(onLoad)}>
          Open a file
        </button>
        <ExamplePicker label="Start from an example" onPick={(e) => onLoad({ name: e.name, text: e.text })} filter={(e) => !e.label.startsWith("Invalid")} />
      </div>
    </DropArea>
  );
}

function ProblemStrip({ marks, doc, reveal }: { marks: Mark[]; doc?: Document; reveal: (m: Mark) => void }) {
  if (!marks.length) return null;
  return (
    <div class="strip">
      {marks.slice(0, 6).map((m) => {
        const i = m.path ? itemIndex(m.path) : null;
        const id = i !== null ? doc?.items?.[i]?.id : undefined;
        return (
          <button type="button" class="strip-row" onClick={() => reveal(m)}>
            {id && <b>{String(id)}</b>} {m.hint ?? friendly(m.message, m.path)}
          </button>
        );
      })}
      {marks.length > 6 && <div class="muted">and {marks.length - 6} more</div>}
    </div>
  );
}

// ---------------------------------------------------------------- Cards

interface CardsProps {
  doc: Document;
  problems: Problem[];
  pristine: number | null;
  setPristine: (i: number | null) => void;
  readOnly: boolean;
  selected: number | null;
  select: (i: number | null) => void;
  filter: string;
  setFilter: (s: string) => void;
  commit: (d: Document) => void;
}

function Cards({ doc, problems, pristine, setPristine, readOnly, selected, select, filter, setFilter, commit }: CardsProps) {
  const [moreTypes, setMoreTypes] = useState(false);
  const info = schemaInfo();
  const items = (Array.isArray(doc.items) ? doc.items : []) as Item[];
  const list = useRef<HTMLDivElement>(null);
  const q = filter.trim().toLowerCase();
  const matches = (it: Item) => !q || `${it.id ?? ""} ${it.text ?? ""}`.toLowerCase().includes(q);

  const byItem = new Map<number, Problem[]>();
  const envelope: Problem[] = [];
  for (const p of problems) {
    const i = itemIndex(p.path);
    if (i === null) envelope.push(p);
    else byItem.set(i, [...(byItem.get(i) ?? []), p]);
  }

  const change = (fn: (d: Document) => void) => {
    const next = structuredClone(doc);
    fn(next);
    commit(next);
  };

  /** Adds an item of a type after the last item of that type, with an ID from core. */
  function add(t: ItemTypeInfo) {
    const next = structuredClone(doc);
    next.items = Array.isArray(next.items) ? next.items : [];
    const fresh: Item = { type: t.type, text: "", status: "proposed" };
    for (const f of t.fields) if (f.required && f.kind === "enum" && f.options) fresh[f.name] = f.options[0];
    if (t.type === "term") fresh.definition = "";
    let at = next.items.length;
    for (let i = next.items.length - 1; i >= 0; i--)
      if (next.items[i].type === t.type) {
        at = i + 1;
        break;
      }
    next.items.splice(at, 0, fresh);
    try {
      const r = core.assignIds(next);
      commit(r.doc);
    } catch {
      commit(next);
    }
    select(at);
    setPristine(at);
    setFilter("");
    requestAnimationFrame(() => list.current?.querySelector<HTMLTextAreaElement>(`[data-index="${at}"] textarea`)?.focus());
  }

  const groups = info.types.map((t) => ({ t, entries: items.map((it, i) => [it, i] as const).filter(([it]) => it.type === t.type && matches(it)) }));
  const unknown = items.map((it, i) => [it, i] as const).filter(([it]) => !info.byType[String(it.type)] && matches(it));

  return (
    <div class="cards" ref={list}>
      <Envelope doc={doc} problems={envelope} readOnly={readOnly} change={change} commit={commit} />
      {items.length > 6 && (
        <input class="filter" type="search" placeholder={`Filter ${items.length} items by ID or text`} value={filter} onInput={(e) => setFilter(e.currentTarget.value)} aria-label="Filter items" />
      )}
      {groups
        .filter((g) => g.entries.length)
        .map(({ t, entries }) => (
          <section class="group" aria-label={t.plural}>
            <div class="group-head">
              <span>{t.plural}</span>
              <span class="n">{entries.length}</span>
              {!readOnly && (
                <button type="button" class="add" onClick={() => add(t)}>
                  Add {t.label.toLowerCase()}
                </button>
              )}
            </div>
            {entries.map(([it, i]) => (
              <Card
                key={i}
                item={it}
                index={i}
                info={t}
                doc={doc}
                problems={byItem.get(i) ?? []}
                commit={commit}
                open={selected === i}
                readOnly={readOnly}
                toggle={() => select(selected === i ? null : i)}
                change={(fn) => change((d) => fn((d.items as Item[])[i]))}
                remove={() => {
                  change((d) => void (d.items as Item[]).splice(i, 1));
                  select(null);
                }}
              />
            ))}
          </section>
        ))}
      {unknown.length > 0 && (
        <section class="group">
          <div class="group-head">
            <span>Items of an unknown type</span>
          </div>
          {unknown.map(([it, i]) => (
            <div class="card err" data-index={i}>
              <div class="card-row">
                <span class="id">{String(it.id ?? "?")}</span>
                <span class="txt">{String(it.text ?? "")}</span>
              </div>
              <div class="card-problems">{(byItem.get(i) ?? []).map((p) => <div>{p.hint ?? p.message}</div>)}</div>
            </div>
          ))}
        </section>
      )}
      {!readOnly && (
        <div class="add-any">
          <span class="muted">Add</span>
          {info.types
            .filter((t) => moreTypes || (suggested[String(doc.kind)] ?? suggested.intent).includes(t.type))
            .map((t) => (
              <button type="button" class="chip" onClick={() => add(t)} title={`Add ${t.label.toLowerCase()} (${t.prefix})`}>
                {t.label}
              </button>
            ))}
          <button type="button" class="link" onClick={() => setMoreTypes(!moreTypes)}>
            {moreTypes ? "Fewer" : "More types"}
          </button>
        </div>
      )}
    </div>
  );
}

function Envelope({ doc, problems, readOnly, change, commit }: { doc: Document; problems: Problem[]; readOnly: boolean; change: (fn: (d: Document) => void) => void; commit: (d: Document) => void }) {
  const set = (key: string, value: string) =>
    change((d) => {
      if (value === "") delete d[key];
      else d[key] = value;
    });
  return (
    <div class={`envelope ${problems.length ? "err" : ""}`}>
      <input class="doc-title" value={String(doc.title ?? "")} placeholder="Title of the document" aria-label="Title" disabled={readOnly} onInput={(e) => set("title", e.currentTarget.value)} />
      <div class="env-row">
        <label>
          <span>Kind</span>
          <input list="kinds" value={String(doc.kind ?? "")} disabled={readOnly} onInput={(e) => set("kind", e.currentTarget.value)} />
          <datalist id="kinds">
            {kinds.map((k) => (
              <option value={k} />
            ))}
          </datalist>
        </label>
        <label>
          <span>Status</span>
          <select value={String(doc.status ?? "")} disabled={readOnly} onChange={(e) => set("status", e.currentTarget.value)}>
            <option value="draft">draft</option>
            <option value="final">final</option>
          </select>
        </label>
        <span class="muted env-meta">
          version {String(doc.version ?? "?")} · <code>{String(doc.id ?? "no ID")}</code>
        </span>
      </div>
      <AutoText class="summary-text" value={String(doc.summary ?? "")} placeholder="Summary (optional, Markdown)" disabled={readOnly} onInput={(v) => set("summary", v)} label="Summary" />
      {problems.length > 0 && (
        <div class="card-problems">
          {problems.map((p) => (
            <div class="row">
              <span class="grow">{p.hint ?? p.message}</span>
              <FixButton doc={doc} fix={p.fix} apply={commit} />
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

interface CardProps {
  item: Item;
  index: number;
  info: ItemTypeInfo;
  doc: Document;
  problems: Problem[];
  open: boolean;
  readOnly: boolean;
  toggle: () => void;
  change: (fn: (it: Item) => void) => void;
  remove: () => void;
  commit: (d: Document) => void;
}

function Card({ item, index, info, doc, problems, open, readOnly, toggle, change, remove, commit }: CardProps) {
  const [confirmDelete, setConfirmDelete] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (open) ref.current?.scrollIntoView({ block: "nearest", behavior: "smooth" });
    else setConfirmDelete(false);
  }, [open]);
  const set = (key: string, value: unknown) =>
    change((it) => {
      if (value === undefined || value === "" || (Array.isArray(value) && !value.length)) delete it[key];
      else it[key] = value;
    });
  const refs = Array.isArray(item.refs) ? (item.refs as { item: string; rel?: string; artifact?: string; version?: number }[]) : [];
  const shown = info.fields.filter((f) => item[f.name] !== undefined && ["enum", "bool", "date", "line"].includes(f.kind) && f.name !== "proposed_by").slice(0, 3);
  const status = String(item.status ?? "");
  const [extra, setExtra] = useState<string[]>([]);
  // Fields the editor can show; "other" fields without a value stay in the text.
  const fields = info.fields.filter((f) => f.name !== "refs" && (f.kind !== "other" || item[f.name] !== undefined));
  const hidden = [
    ...fields.filter((f) => !f.required && item[f.name] === undefined && !extra.includes(f.name)),
    ...(refs.length || extra.includes("refs") ? [] : [{ name: "refs" } as Field]),
  ];

  return (
    <div ref={ref} data-index={index} class={`card ${open ? "open" : ""} ${problems.length ? "err" : ""} st-${status}`}>
      <button type="button" class="card-row" onClick={toggle} aria-expanded={open}>
        <span class="id">{String(item.id ?? "new")}</span>
        <span class={`txt ${item.text && !open ? "" : "muted"}`}>{open ? info.label : item.text ? String(item.text) : `Write the ${info.label.toLowerCase()}`}</span>
        {!open && status !== "accepted" && <span class={`pill ${status}`}>{status || "no status"}</span>}
      </button>
      {!open && (shown.length > 0 || refs.length > 0) && (
        <div class="card-meta">
          {shown.map((f) => (
            <span>
              {f.name.replace(/_/g, " ")} <b>{fmt(item[f.name])}</b>
            </span>
          ))}
          {refs.map((r) => (
            <span>
              {(r.rel ?? "link").replace(/_/g, " ")} <b class="mono">{r.artifact ? `${r.artifact}@${r.version ?? "?"}#${r.item}` : r.item}</b>
            </span>
          ))}
        </div>
      )}
      {problems.length > 0 && (
        <div class="card-problems">
          {problems.map((p) => (
            <div class="row">
              <span class="grow">{p.hint ?? friendly(p.message, p.path)}</span>
              <FixButton doc={doc} fix={p.fix} apply={commit} />
            </div>
          ))}
        </div>
      )}
      {open && (
        <div
          class="card-edit"
          onKeyDown={(e) => {
            if (e.key === "Escape") toggle();
            else if (e.key === "Enter" && !e.shiftKey && (e.target as HTMLElement).classList.contains("big")) {
              e.preventDefault();
              toggle();
            }
          }}
        >
          <AutoText value={String(item.text ?? "")} label={`Text of ${item.id ?? "the item"}`} placeholder="One statement in plain text" disabled={readOnly} onInput={(v) => set("text", v)} big />
          <div class="field-row">
            <Segmented
              label="Status"
              value={status as "proposed"}
              onChange={(v) => !readOnly && set("status", v)}
              options={schemaInfo().statuses.map((s) => ({ value: s as "proposed", label: s }))}
            />
          </div>
          <div class="fields">
            {fields
              .filter((f) => f.required || item[f.name] !== undefined || extra.includes(f.name))
              .map((f) => (
                <FieldInput field={f} value={item[f.name]} disabled={readOnly} set={(v) => set(f.name, v)} />
              ))}
          </div>
          {(refs.length > 0 || extra.includes("refs")) && <Refs refs={refs} doc={doc} self={String(item.id ?? "")} disabled={readOnly} set={(v) => set("refs", v)} />}
          {!readOnly && hidden.length > 0 && (
            <div class="more-fields">
              {hidden.map((f) => (
                <button type="button" class="chip" onClick={() => setExtra([...extra, f.name])}>
                  + {f.name === "refs" ? "link" : f.name.replace(/_/g, " ")}
                </button>
              ))}
            </div>
          )}
          <div class="card-foot">
            <span class="grow" />
            {!readOnly && (
              <button type="button" class={`btn small ${confirmDelete ? "danger" : "ghost"}`} onClick={() => (confirmDelete ? remove() : setConfirmDelete(true))}>
                {confirmDelete ? `Delete ${item.id ?? "item"}` : "Delete"}
              </button>
            )}
            <button type="button" class="btn small ghost" onClick={toggle}>
              Done
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

const fmt = (v: unknown) => (typeof v === "boolean" ? (v ? "yes" : "no") : String(v).replace(/_/g, " "));

function FieldInput({ field, value, disabled, set }: { field: Field; value: unknown; disabled: boolean; set: (v: unknown) => void }) {
  const label = field.name.replace(/_/g, " ");
  const help = field.description;
  if (field.name === "detail")
    return (
      <label class="field wide">
        <span>detail</span>
        <AutoText value={String(value ?? "")} label="Detail" placeholder="Optional Markdown" disabled={disabled} onInput={(v) => set(v)} />
      </label>
    );
  switch (field.kind) {
    case "enum":
      return (
        <label class="field" title={help}>
          <span>
            {label}
            {field.required && " *"}
          </span>
          <select value={value === undefined ? "" : String(value)} disabled={disabled} onChange={(e) => set(e.currentTarget.value || undefined)}>
            {!field.required && <option value="">–</option>}
            {field.options!.map((o) => (
              <option value={o}>{o.replace(/_/g, " ")}</option>
            ))}
          </select>
        </label>
      );
    case "bool":
      return (
        <label class="field" title={help}>
          <span>{label}</span>
          <select value={value === undefined ? "" : String(value)} disabled={disabled} onChange={(e) => set(e.currentTarget.value === "" ? undefined : e.currentTarget.value === "true")}>
            <option value="">–</option>
            <option value="true">yes</option>
            <option value="false">no</option>
          </select>
        </label>
      );
    case "date":
      return (
        <label class="field" title={help}>
          <span>{label}</span>
          <input type="date" value={String(value ?? "")} disabled={disabled} onInput={(e) => set(e.currentTarget.value || undefined)} />
        </label>
      );
    case "line":
      return (
        <label class="field" title={help}>
          <span>
            {label}
            {field.required && " *"}
          </span>
          <input value={String(value ?? "")} disabled={disabled} placeholder={help} onInput={(e) => set(e.currentTarget.value)} />
        </label>
      );
    case "text":
      return (
        <label class="field wide" title={help}>
          <span>
            {label}
            {field.required && " *"}
          </span>
          <AutoText value={String(value ?? "")} label={label} placeholder={help} disabled={disabled} onInput={(v) => set(v)} />
        </label>
      );
    case "list":
      return (
        <label class="field wide" title={help}>
          <span>{label} (one per line)</span>
          <AutoText
            value={Array.isArray(value) ? value.join("\n") : ""}
            label={label}
            placeholder={help}
            disabled={disabled}
            onInput={(v) => set(v.split("\n").filter((x, i, all) => x.trim() || i === all.length - 1))}
            onBlurValue={(v) => set(v.split("\n").map((x) => x.trim()).filter(Boolean))}
          />
        </label>
      );
    default:
      if (value === undefined) return null;
      return (
        <div class="field wide">
          <span>{label}</span>
          <span class="muted">{Array.isArray(value) ? `${value.length} ${value.length === 1 ? "entry" : "entries"}` : "set"}. Edit in the text.</span>
        </div>
      );
  }
}

function Refs({ refs, doc, self, disabled, set }: { refs: { item: string; rel?: string; artifact?: string; version?: number }[]; doc: Document; self: string; disabled: boolean; set: (v: unknown) => void }) {
  const others = ((doc.items ?? []) as Item[]).filter((i) => i.id && i.id !== self);
  const rels = schemaInfo().relations;
  const update = (i: number, patch: Partial<{ item: string; rel: string }>) => set(refs.map((r, k) => (k === i ? { ...r, ...patch } : r)));
  return (
    <div class="refs">
      <span class="refs-label">Links</span>
      {refs.map((r, i) =>
        r.artifact ? (
          <div class="ref muted">
            {(r.rel ?? "link").replace(/_/g, " ")} <code>{`${r.artifact}@${r.version ?? "?"}#${r.item}`}</code>
          </div>
        ) : (
          <div class="ref">
            <select aria-label="Relation" value={r.rel ?? ""} disabled={disabled} onChange={(e) => update(i, { rel: e.currentTarget.value })}>
              <option value="">–</option>
              {rels.map((x) => (
                <option value={x}>{x.replace(/_/g, " ")}</option>
              ))}
            </select>
            <select aria-label="Linked item" value={r.item} disabled={disabled} onChange={(e) => update(i, { item: e.currentTarget.value })}>
              {!others.some((o) => o.id === r.item) && <option value={r.item}>{r.item}</option>}
              {others.map((o) => (
                <option value={String(o.id)}>
                  {String(o.id)} {String(o.text ?? "").slice(0, 60)}
                </option>
              ))}
            </select>
            {!disabled && (
              <button type="button" class="btn ghost icon small" aria-label="Remove link" onClick={() => set(refs.filter((_, k) => k !== i))}>
                ×
              </button>
            )}
          </div>
        ),
      )}
      {!disabled && others.length > 0 && (
        <button type="button" class="link" onClick={() => set([...refs, { item: String(others[0].id), rel: "relates_to" }])}>
          Add a link
        </button>
      )}
    </div>
  );
}

/** A textarea that grows with its text. */
function AutoText({ value, onInput, onBlurValue, label, placeholder, disabled, big, class: cls = "" }: { value: string; onInput: (v: string) => void; onBlurValue?: (v: string) => void; label: string; placeholder?: string; disabled?: boolean; big?: boolean; class?: string }) {
  const ref = useRef<HTMLTextAreaElement>(null);
  useEffect(() => {
    const el = ref.current!;
    el.style.height = "auto";
    el.style.height = el.scrollHeight + 2 + "px";
  }, [value]);
  return (
    <textarea
      ref={ref}
      class={`auto ${big ? "big" : ""} ${cls}`}
      rows={1}
      value={value}
      aria-label={label}
      placeholder={placeholder}
      disabled={disabled}
      onInput={(e) => onInput(e.currentTarget.value)}
      onBlur={(e) => onBlurValue?.(e.currentTarget.value)}
      onKeyDown={(e) => {
        if (e.key === "Escape") e.currentTarget.blur();
      }}
    />
  );
}
