// Small parts that the tools share.
import type { ComponentChildren } from "preact";
import { useEffect, useMemo, useRef, useState } from "preact/hooks";
import { Editor, type EditorHandle } from "./editor";
import { detectFormat, formatLabel, read, type Format, type ReadResult } from "./doc";
import { examples, type Example } from "./examples";
import { core, next, type Document } from "./core";

/** State that the browser keeps between visits, in localStorage. */
export function useStored<T>(key: string, initial: T | (() => T)): [T, (v: T) => void] {
  const [value, setValue] = useState<T>(() => {
    try {
      const raw = localStorage.getItem("openskopos-tools:" + key);
      if (raw !== null) return JSON.parse(raw) as T;
    } catch {}
    return typeof initial === "function" ? (initial as () => T)() : initial;
  });
  const set = (v: T) => {
    setValue(v);
    try {
      localStorage.setItem("openskopos-tools:" + key, JSON.stringify(v));
    } catch {}
  };
  return [value, set];
}

/**
 * A value that follows another value after a short pause, and at least every
 * maxWait milliseconds while it keeps changing, so a check also runs during
 * long typing.
 */
export function useDebounced<T>(value: T, ms = 150, maxWait = 600): T {
  const [v, setV] = useState(value);
  const since = useRef<number | null>(null);
  useEffect(() => {
    since.current ??= Date.now();
    const wait = Math.max(0, Math.min(ms, since.current + maxWait - Date.now()));
    const t = setTimeout(() => {
      since.current = null;
      setV(value);
    }, wait);
    return () => clearTimeout(t);
  }, [value, ms, maxWait]);
  return v;
}

export function useMedia(query: string): boolean {
  const [match, setMatch] = useState(() => matchMedia(query).matches);
  useEffect(() => {
    const m = matchMedia(query);
    const on = () => setMatch(m.matches);
    m.addEventListener("change", on);
    return () => m.removeEventListener("change", on);
  }, [query]);
  return match;
}

let toastTimer: ReturnType<typeof setTimeout> | undefined;
export function toast(message: string) {
  let el = document.getElementById("toast");
  if (!el) {
    el = document.createElement("div");
    el.id = "toast";
    el.setAttribute("role", "status");
    document.body.append(el);
  }
  el.textContent = message;
  el.classList.add("on");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el!.classList.remove("on"), 1800);
}

export async function copy(text: string, what = "Copied") {
  await navigator.clipboard.writeText(text);
  toast(what);
}

export function download(name: string, data: string | Uint8Array, mime = "text/plain") {
  const blob = new Blob([data as BlobPart], { type: mime });
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
}

export interface TextFile {
  name: string;
  text: string;
}

export const accept = ".json,.yaml,.yml,.md,.markdown,application/json,text/yaml,text/markdown";

export function openFiles(multiple = false): Promise<TextFile[]> {
  return new Promise((done) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = accept;
    input.multiple = multiple;
    input.onchange = async () => done(await readFiles(input.files));
    input.click();
  });
}

export async function readFiles(list: FileList | null | undefined): Promise<TextFile[]> {
  return Promise.all([...(list ?? [])].map(async (f) => ({ name: f.name, text: await f.text() })));
}

/** Calls onFiles when files are dropped anywhere on the element. */
export function DropArea({ onFiles, children, class: cls = "", hint }: { onFiles: (f: TextFile[]) => void; children: ComponentChildren; class?: string; hint: string }) {
  const [over, setOver] = useState(false);
  const depth = useRef(0);
  return (
    <div
      class={`drop ${cls} ${over ? "over" : ""}`}
      onDragEnter={(e) => {
        if (!e.dataTransfer?.types.includes("Files")) return;
        depth.current++;
        setOver(true);
      }}
      onDragOver={(e) => e.dataTransfer?.types.includes("Files") && e.preventDefault()}
      onDragLeave={() => {
        depth.current = Math.max(0, depth.current - 1);
        if (!depth.current) setOver(false);
      }}
      onDrop={async (e) => {
        if (!e.dataTransfer?.files.length) return;
        e.preventDefault();
        depth.current = 0;
        setOver(false);
        onFiles(await readFiles(e.dataTransfer.files));
      }}
    >
      {children}
      {over && <div class="drop-hint">{hint}</div>}
    </div>
  );
}

export function Segmented<T extends string>({ value, options, onChange, label }: { value: T; options: { value: T; label: string }[]; onChange: (v: T) => void; label: string }) {
  return (
    <div class="seg" role="radiogroup" aria-label={label}>
      {options.map((o) => (
        <button type="button" role="radio" aria-checked={o.value === value} onClick={() => onChange(o.value)}>
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function ExamplePicker({ onPick, label = "Examples", filter }: { onPick: (e: Example) => void; label?: string; filter?: (e: Example) => boolean }) {
  const list = filter ? examples.filter(filter) : examples;
  return (
    <label class="btn ghost select">
      <span>{label}</span>
      <select
        aria-label={label}
        value=""
        onChange={(e) => {
          const ex = list.find((x) => x.name === e.currentTarget.value);
          e.currentTarget.value = "";
          if (ex) onPick(ex);
        }}
      >
        <option value="">{label}</option>
        {list.map((e) => (
          <option value={e.name}>{e.label}</option>
        ))}
      </select>
    </label>
  );
}

export type Verdict = "empty" | "valid" | "invalid" | "unreadable";

export function verdictOf(text: string, r: ReadResult | undefined): Verdict {
  if (!text.trim() || !r) return "empty";
  if (!r.doc) return "unreadable";
  return r.problems.length ? "invalid" : "valid";
}

export function Status({ verdict, count, version = "0.1" }: { verdict: Verdict; count?: number; version?: string }) {
  const text =
    verdict === "valid" ? `Valid OpenSkopos ${version}` : verdict === "invalid" ? `${count} ${count === 1 ? "problem" : "problems"}` : verdict === "unreadable" ? "Cannot read the text" : "No document";
  return (
    <span class={`status ${verdict}`} aria-live="polite">
      <i aria-hidden="true" />
      {text}
    </span>
  );
}

/** The anchor of a rule in section 9 of the specification. */
export const ruleUrl = "/spec/0.1/#9-rules-that-the-schema-cannot-check";

/** A document slot with a name, a small editor, and its status. For Compare and Agent file. */
export function DocInput({
  title,
  value,
  onChange,
  onRemove,
  result,
  height = "260px",
}: {
  title: string;
  value: TextFile;
  onChange: (f: TextFile) => void;
  onRemove?: () => void;
  result?: ReadResult;
  height?: string;
}) {
  const format = useMemo(() => detectFormat(value.text, value.name), [value.text, value.name]);
  const verdict = verdictOf(value.text, result);
  const doc = result?.doc;
  const editor = useRef<EditorHandle | null>(null);
  return (
    <DropArea class="slot" onFiles={(f) => f[0] && onChange(f[0])} hint="Drop to open">
      <div class="slot-head">
        <div class="slot-title">
          <span class="eyebrow">{title}</span>
          <span class="slot-name" title={value.name}>
            {doc?.title ? String(doc.title) : value.name || "No document yet"}
            {doc?.version !== undefined && <span class="muted"> · version {String(doc.version)}</span>}
          </span>
        </div>
        <div class="slot-actions">
          <Status verdict={verdict} count={result?.problems.length} />
          <button type="button" class="btn ghost" onClick={async () => {
              const [f] = await openFiles();
              if (f) onChange(f);
            }}>
            Open
          </button>
          {onRemove && (
            <button type="button" class="btn ghost icon" aria-label={`Remove ${title}`} onClick={onRemove}>
              ×
            </button>
          )}
        </div>
      </div>
      <div class="slot-editor" style={{ height }}>
        <Editor
          label={title}
          value={value.text}
          format={format}
          marks={result?.marks}
          onChange={(text) => onChange({ ...value, text })}
          placeholder={`Paste, drop or open a document (${formatLabel.json}, ${formatLabel.yaml} or ${formatLabel.markdown})`}
          handle={(h) => (editor.current = h)}
        />
      </div>
      {result && result.problems.length + result.readErrors.length > 0 && (
        <div class="slot-problems">
          {[...result.readErrors, ...result.problems.map((p) => p.message)].slice(0, 3).map((m) => (
            <div>{m}</div>
          ))}
          {result.problems.length + result.readErrors.length > 3 && <div class="muted">and {result.problems.length + result.readErrors.length - 3} more. Open the file in Validate to see all.</div>}
        </div>
      )}
    </DropArea>
  );
}

/** Reads a slot with core, with stable Markdown options for one page visit. */
export function useRead(file: TextFile, format?: Format): ReadResult | undefined {
  const text = useDebounced(file.text);
  const f = format ?? detectFormat(text, file.name);
  return useMemo(() => (text.trim() ? read(text, f, { ...mdSession(), filename: file.name }) : undefined), [text, file.name, f]);
}

export type { Format };

let session: { now: string; artifact_id: string } | undefined;
/**
 * Markdown options for one page visit. A Markdown file without a date or an
 * ID gets the same ones on every keystroke, so the output does not jump.
 */
export function mdSession() {
  session ??= { now: new Date().toISOString().replace(/\.\d+Z$/, "Z"), artifact_id: core.newArtifactId() };
  return session;
}

let sheetAdded = false;
/**
 * The readable view of a document: the HTML of core's renderer, styled by
 * core's stylesheet with the brand values from tools.css. Shows nothing if
 * the loaded core has no renderer.
 */
export function Readable({ doc, previous }: { doc: Document; previous?: Document }) {
  const html = useMemo(() => {
    if (!next.renderHtml) return "";
    try {
      return next.renderHtml(doc, { fragment: true, previous });
    } catch (e) {
      return `<p class="bad-text">${String((e as Error).message).replace(/[&<>]/g, "")}</p>`;
    }
  }, [doc, previous]);
  if (!sheetAdded && next.stylesheet) {
    const style = document.createElement("style");
    style.textContent = next.stylesheet();
    document.head.prepend(style);
    sheetAdded = true;
  }
  return (
    <div
      class="readable"
      dangerouslySetInnerHTML={{ __html: html }}
      onClick={(e) => {
        // The IDs link to their item. Scroll inside the pane, and keep the URL.
        const a = (e.target as HTMLElement).closest<HTMLAnchorElement>("a.sk-id, a[href^='#']");
        if (!a) return;
        const id = a.getAttribute("href")?.slice(1);
        const el = id && (e.currentTarget as HTMLElement).querySelector<HTMLElement>(`[data-id="${CSS.escape(id)}"]`);
        if (!el) return;
        e.preventDefault();
        el.scrollIntoView({ block: "center", behavior: "smooth" });
        el.classList.add("sk-flash");
        setTimeout(() => el.classList.remove("sk-flash"), 1200);
      }}
    />
  );
}

export const hasReadable = () => !!next.renderHtml;
