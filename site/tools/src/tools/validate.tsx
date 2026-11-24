// Validate: check a document with core, show each problem at its place in the
// text, and convert a valid document to the other formats.
import { useEffect, useMemo, useRef, useState } from "preact/hooks";
import { Editor, type EditorHandle } from "../editor";
import { describePath, detectFormat, fileName, formatLabel, formatMime, formats, write, type Format, type Mark } from "../doc";
import { readShared, shareUrl } from "../share";
import { schemaInfo } from "../schema";
import type { Go, Handoff } from "../nav";
import { copy, download, DropArea, EmptyState, ExamplePicker, hasReadable, lineOf, FixButton, Menu, openFiles, Readable, Segmented, Status, useRead, useStored, verdictOf, type TextFile } from "../ui";
import { next, type Document } from "../core";

type FormatChoice = "auto" | Format;
type Side = "readable" | "report";

export function Validate({ go, incoming }: { go: Go; incoming?: Handoff }) {
  const [file, setFile] = useStored<TextFile>("validate:file", { name: "", text: "" });
  const [choice, setChoice] = useState<FormatChoice>("auto");
  // The person's choice for this document only. A new document starts with
  // the readable view when it is valid, and with the problems when it is not.
  const [picked, setPicked] = useState<Side | null>(null);
  const editor = useRef<EditorHandle | null>(null);

  const detected = useMemo(() => detectFormat(file.text, file.name), [file.text, file.name]);
  const format = choice === "auto" ? detected : choice;
  const result = useRead(file, format);
  const verdict = verdictOf(file.text, result);
  const readable = hasReadable() && verdict === "valid";
  const side: Side = readable ? (picked ?? "readable") : "report";

  const open = (f: TextFile) => {
    setChoice("auto");
    setPicked(null);
    setFile(f);
  };
  useEffect(() => void (incoming && open(incoming.file)), [incoming?.n]);
  useEffect(() => {
    readShared<TextFile>().then((f) => f && open(f));
  }, []);

  const marks = result?.marks ?? [];
  const errors = marks.filter((m) => m.severity === "error");
  const warnings = marks.filter((m) => m.severity === "warning");
  const reveal = (m: Mark) => editor.current?.reveal(m.from, m.to);
  const empty = !file.text;
  /** A fixed document goes back into the editor, in the same format. */
  const apply = (d: Document) => setFile({ ...file, text: write(d, format) });
  const fixable = errors.filter((m) => m.fix).length;

  return (
    <div class="split-2 validate">
      <DropArea class="pane source" onFiles={(f) => f[0] && open(f[0])} hint="Drop to check this file">
        {!empty && (
          <div class="pane-head">
            <span class="file" title={file.name || undefined}>
              {file.name || `Pasted ${formatLabel[format]}`}
            </span>
            <label class="format-pick">
              <span class="sr">Format</span>
              <select value={choice} onChange={(e) => setChoice(e.currentTarget.value as FormatChoice)}>
                <option value="auto">{formatLabel[detected]} (detected)</option>
                {formats.map((f) => (
                  <option value={f}>{formatLabel[f]}</option>
                ))}
              </select>
            </label>
            <span class="grow" />
            <ExamplePicker onPick={(e) => open({ name: e.name, text: e.text })} />
            <Menu
              items={[
                { label: "Open a file", run: async () => (await openFiles()).slice(0, 1).forEach(open) },
                { label: "Clear", run: () => open({ name: "", text: "" }) },
              ]}
            />
          </div>
        )}
        <div class="pane-body">
          <Editor label="Document source" value={file.text} format={format} marks={marks} onChange={(text) => setFile({ ...file, text })} handle={(h) => (editor.current = h)} complete />
          {empty && (
            <EmptyState
              title="Check an OpenSkopos document"
              actions={
                <>
                  <button type="button" class="btn" onClick={async () => (await openFiles()).slice(0, 1).forEach(open)}>
                    Open a file
                  </button>
                  <ExamplePicker onPick={(e) => open({ name: e.name, text: e.text })} />
                </>
              }
            >
              Drop a <code>.skopos.json</code>, <code>.skopos.yaml</code> or Markdown file such as <code>INTENT.md</code> here, or paste it into the editor.
            </EmptyState>
          )}
        </div>
      </DropArea>

      <aside class="pane report" aria-label="Result" hidden={empty}>
        <div class="pane-head">
          <Status verdict={verdict} count={result?.problems.length} />
          <span class="grow" />
          {readable && (
            <Segmented
              label="Show"
              value={side}
              onChange={setPicked}
              options={[
                { value: "readable", label: "Readable" },
                { value: "report", label: "Details" },
              ]}
            />
          )}
        </div>
        {side === "readable" && result?.doc ? (
          <div class="pane-body scroll">
            <Readable doc={result.doc} />
            <Actions doc={result.doc} format={format} file={file} go={go} />
          </div>
        ) : (
          <div class="pane-body scroll">
            {errors.length > 0 && (
              <Section
                title={verdict === "unreadable" ? "Why the text cannot be read" : "Problems"}
                action={
                  fixable > 1 && next.fixAll && result?.doc ? (
                    <button type="button" class="btn ghost small" onClick={() => apply(next.fixAll!(result.doc!).doc)}>
                      Fix {fixable}
                    </button>
                  ) : null
                }
              >
                <ProblemList marks={errors} doc={result?.doc} text={file.text} reveal={reveal} apply={apply} />
              </Section>
            )}
            {warnings.length > 0 && (
              <Section title="Notes from the Markdown reader">
                <ProblemList marks={warnings} doc={result?.doc} text={file.text} reveal={reveal} apply={apply} />
              </Section>
            )}
            {result?.doc && <Summary doc={result.doc} />}
            {result?.doc && verdict === "valid" && <Actions doc={result.doc} format={format} file={file} go={go} />}
            {result?.doc && verdict === "invalid" && (
              <div class="row wrap section">
                <button type="button" class="btn ghost" onClick={() => go("author", { name: file.name, text: file.text })}>
                  Fix in Author
                </button>
              </div>
            )}
          </div>
        )}
      </aside>
    </div>
  );
}

function Section({ title, action, children }: { title: string; action?: preact.ComponentChildren; children: preact.ComponentChildren }) {
  return (
    <section class="section">
      <h2 class="row">
        <span class="grow">{title}</span>
        {action}
      </h2>
      {children}
    </section>
  );
}

function Summary({ doc }: { doc: Document }) {
  const info = schemaInfo();
  const items = Array.isArray(doc.items) ? doc.items : [];
  const counts = info.types.map((t) => [t, items.filter((i) => i?.type === t.type).length] as const).filter(([, n]) => n > 0);
  return (
    <div class="summary section">
      <div class="summary-title">{typeof doc.title === "string" && doc.title ? doc.title : <span class="muted">No title</span>}</div>
      <div class="meta" title={typeof doc.id === "string" ? doc.id : undefined}>
        {[doc.kind && `${doc.kind}`, doc.version !== undefined && `version ${doc.version}`, doc.status, `${items.length} ${items.length === 1 ? "item" : "items"}`].filter(Boolean).join(" · ")}
      </div>
      {counts.length > 0 && <p class="muted small-text">{counts.map(([t, n]) => `${n} ${n === 1 ? t.label.toLowerCase() : t.plural.toLowerCase()}`).join(", ")}</p>}
    </div>
  );
}

const sentence = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

function ProblemList({ marks, doc, text, reveal, apply }: { marks: Mark[]; doc?: Document; text: string; reveal: (m: Mark) => void; apply: (d: Document) => void }) {
  return (
    <ol class="problems">
      {marks.map((m) => (
        <li class={m.severity}>
          <button type="button" class="problem" onClick={() => reveal(m)} title={m.rule ? `Rule ${m.rule}` : undefined}>
            <span class="msg">{m.hint ?? sentence(m.message)}</span>
            <span class="where">
              Line {lineOf(text, m.from)}
              {m.path ? ` · ${describePath(doc, m.path)}` : ""}
            </span>
          </button>
          <FixButton doc={doc} fix={m.fix} apply={apply} />
        </li>
      ))}
    </ol>
  );
}

function Actions({ doc, format, file, go }: { doc: Document; format: Format; file: TextFile; go: Go }) {
  const targets = formats.filter((f) => f !== format);
  const [target, setTarget] = useState<Format>(targets[0]);
  const to = targets.includes(target) ? target : targets[0];
  const output = useMemo(() => {
    try {
      return { text: write(doc, to) };
    } catch (e) {
      return { error: (e as Error).message };
    }
  }, [doc, to]);
  const name = fileName(doc, to);
  return (
    <div class="actions section">
      <div class="row wrap">
        <button type="button" class="btn" onClick={() => go("author", { name: file.name, text: file.text })}>
          Edit in Author
        </button>
        <button type="button" class="btn ghost" onClick={async () => copy(await shareUrl(file), "Link copied. The document is in the link, not on a server.")}>
          Share
        </button>
        <button type="button" class="btn ghost" onClick={() => go("agent-file", { name: file.name, text: file.text })}>
          Make an agent file
        </button>
      </div>
      <div class="row wrap convert">
        <span class="muted small-text">Convert to</span>
        <Segmented label="Convert to" value={to} onChange={setTarget} options={targets.map((f) => ({ value: f, label: formatLabel[f] }))} />
        {output.text !== undefined ? (
          <>
            <button type="button" class="btn ghost small" onClick={() => copy(output.text!)}>
              Copy
            </button>
            <button type="button" class="btn ghost small" onClick={() => download(name, output.text!, formatMime[to])}>
              Download {name}
            </button>
          </>
        ) : (
          <span class="bad-text small-text">{output.error}</span>
        )}
      </div>
    </div>
  );
}
