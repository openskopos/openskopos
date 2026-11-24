// Validate: check a document with core, show each problem at its place in the
// text, and convert a valid document to the other formats.
import { useEffect, useMemo, useRef, useState } from "preact/hooks";
import { Editor, type EditorHandle } from "../editor";
import { describePath, detectFormat, fileName, formatLabel, formatMime, formats, write, type Format, type Mark } from "../doc";
import { readShared, shareUrl } from "../share";
import { schemaInfo } from "../schema";
import type { Go, Handoff } from "../nav";
import { copy, download, DropArea, ExamplePicker, hasReadable, openFiles, Readable, ruleUrl, Segmented, Status, useRead, useStored, verdictOf, type TextFile } from "../ui";
import type { Document } from "../core";

type FormatChoice = "auto" | Format;

export function Validate({ go, incoming }: { go: Go; incoming?: Handoff }) {
  const [file, setFile] = useStored<TextFile>("validate:file", { name: "", text: "" });
  const [choice, setChoice] = useState<FormatChoice>("auto");
  const [side, setSide] = useStored<"report" | "readable">("validate:side", "report");
  const editor = useRef<EditorHandle | null>(null);

  const detected = useMemo(() => detectFormat(file.text, file.name), [file.text, file.name]);
  const format = choice === "auto" ? detected : choice;
  const result = useRead(file, format);
  const verdict = verdictOf(file.text, result);

  const open = (f: TextFile) => {
    setChoice("auto");
    setFile(f);
  };
  useEffect(() => void (incoming && open(incoming.file)), [incoming?.n]);
  useEffect(() => {
    readShared<TextFile>().then((f) => f && open(f));
  }, []);

  const marks = result?.marks ?? [];
  const errors = marks.filter((m) => m.severity === "error");
  const warnings = marks.filter((m) => m.severity === "warning");

  return (
    <div class="split-2 validate">
      <DropArea class="pane source" onFiles={(f) => f[0] && open(f[0])} hint="Drop to check this file">
        <div class="pane-head">
          <span class="file" title={file.name || undefined}>
            {file.name || (file.text ? `Pasted ${formatLabel[format]}` : "No file")}
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
          <button type="button" class="btn ghost" onClick={async () => (await openFiles()).slice(0, 1).forEach(open)}>
            Open
          </button>
          <ExamplePicker onPick={(e) => open({ name: e.name, text: e.text })} />
          {file.text && (
            <button type="button" class="btn ghost" onClick={() => open({ name: "", text: "" })}>
              Clear
            </button>
          )}
        </div>
        <div class="pane-body">
          <Editor
            label="Document source"
            value={file.text}
            format={format}
            marks={marks}
            onChange={(text) => setFile({ ...file, text })}
            handle={(h) => (editor.current = h)}
            complete
          />
          {!file.text && <Empty onOpen={open} />}
        </div>
      </DropArea>

      <aside class="pane report" aria-label="Report">
        <div class="pane-head">
          <Status verdict={verdict} count={result?.problems.length} />
          <span class="grow" />
          {hasReadable() && result?.doc && (
            <Segmented
              label="Show"
              value={side}
              onChange={setSide}
              options={[
                { value: "report", label: "Report" },
                { value: "readable", label: "Readable" },
              ]}
            />
          )}
        </div>
        {hasReadable() && result?.doc && side === "readable" ? (
          <div class="pane-body scroll">
            <Readable doc={result.doc} />
          </div>
        ) : (
        <div class="pane-body scroll">
          {verdict === "empty" && <p class="muted pad">Paste, drop or open a document. The check runs on every change.</p>}
          {result?.doc && <Summary doc={result.doc} />}
          {errors.length > 0 && (
            <Section title={verdict === "unreadable" ? "Why core cannot read it" : "Problems"}>
              <ProblemList marks={errors} doc={result?.doc} reveal={(m) => editor.current?.reveal(m.from, m.to)} />
            </Section>
          )}
          {warnings.length > 0 && (
            <Section title="Markdown reader notes">
              <ProblemList marks={warnings} doc={result?.doc} reveal={(m) => editor.current?.reveal(m.from, m.to)} />
            </Section>
          )}
          {result?.doc && <Actions doc={result.doc} valid={verdict === "valid"} format={format} file={file} go={go} replace={(f) => open(f)} />}
        </div>
        )}
      </aside>
    </div>
  );
}

function Empty({ onOpen }: { onOpen: (f: TextFile) => void }) {
  return (
    <div class="blank">
      <p class="blank-title">Drop an OpenSkopos document here</p>
      <p class="muted">
        <code>.skopos.json</code>, <code>.skopos.yaml</code> or a Markdown file such as <code>INTENT.md</code>. You can also paste into the editor.
      </p>
      <div class="row">
        <button type="button" class="btn" onClick={async () => (await openFiles()).slice(0, 1).forEach(onOpen)}>
          Open a file
        </button>
        <ExamplePicker label="Try an example" onPick={(e) => onOpen({ name: e.name, text: e.text })} />
      </div>
    </div>
  );
}

function Section({ title, children }: { title: string; children: preact.ComponentChildren }) {
  return (
    <section class="section">
      <h2>{title}</h2>
      {children}
    </section>
  );
}

function Summary({ doc }: { doc: Document }) {
  const info = schemaInfo();
  const items = Array.isArray(doc.items) ? doc.items : [];
  const counts = info.types.map((t) => [t, items.filter((i) => i?.type === t.type).length] as const).filter(([, n]) => n > 0);
  return (
    <div class="summary">
      <div class="summary-title">{typeof doc.title === "string" && doc.title ? doc.title : <span class="muted">No title</span>}</div>
      <div class="meta">
        {[doc.kind && `${doc.kind}`, doc.version !== undefined && `version ${doc.version}`, doc.status, `${items.length} ${items.length === 1 ? "item" : "items"}`].filter(Boolean).join(" · ")}
        {typeof doc.id === "string" && <code class="aid">{doc.id}</code>}
      </div>
      {counts.length > 0 && (
        <ul class="counts">
          {counts.map(([t, n]) => (
            <li title={t.plural}>
              <span class="prefix">{t.prefix}</span>
              {t.plural} <b>{n}</b>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function ProblemList({ marks, doc, reveal }: { marks: Mark[]; doc?: Document; reveal: (m: Mark) => void }) {
  return (
    <ol class="problems">
      {marks.map((m) => (
        <li class={m.severity}>
          <button type="button" class="problem" onClick={() => reveal(m)}>
            {m.path !== undefined && <span class="where">{describePath(doc, m.path)}</span>}
            <span class="msg">{m.message}</span>
          </button>
          {m.rule ? (
            <a class="rule" href={ruleUrl} target="_blank" rel="noopener" title="Section 9 of the specification">
              {m.rule}
            </a>
          ) : m.path !== undefined ? (
            <span class="rule schema" title="A rule of the JSON Schema">
              schema
            </span>
          ) : null}
        </li>
      ))}
    </ol>
  );
}

function Actions({ doc, valid, format, file, go, replace }: { doc: Document; valid: boolean; format: Format; file: TextFile; go: Go; replace: (f: TextFile) => void }) {
  const targets = formats.filter((f) => f !== format);
  const [target, setTarget] = useState<Format>(targets[0]);
  const to = targets.includes(target) ? target : targets[0];
  const output = useMemo(() => {
    if (!valid) return undefined;
    try {
      return { text: write(doc, to) };
    } catch (e) {
      return { error: (e as Error).message };
    }
  }, [doc, to, valid]);
  const name = fileName(doc, to);
  return (
    <>
      <Section title="Next">
        <div class="row wrap">
          <button type="button" class="btn" onClick={() => go("author", { name: file.name, text: file.text })}>
            Edit in Author
          </button>
          <button type="button" class="btn ghost" onClick={() => go("agent-file", { name: file.name, text: file.text })}>
            Make an agent file
          </button>
          <button type="button" class="btn ghost" onClick={async () => copy(await shareUrl(file), "Link copied. The document is in the link, not on a server.")}>
            Copy share link
          </button>
        </div>
      </Section>
      {valid && output && (
        <Section title="Convert">
          <div class="row">
            <Segmented label="Convert to" value={to} onChange={setTarget} options={targets.map((f) => ({ value: f, label: formatLabel[f] }))} />
            <span class="grow" />
            {output.text !== undefined && (
              <>
                <button type="button" class="btn ghost" onClick={() => copy(output.text!)}>
                  Copy
                </button>
                <button type="button" class="btn ghost" onClick={() => download(name, output.text!, formatMime[to])}>
                  Download {name}
                </button>
              </>
            )}
          </div>
          {output.error ? (
            <p class="bad-text">{output.error}</p>
          ) : (
            <div class="output">
              <Editor label={`${formatLabel[to]} output`} value={output.text!} format={to} readOnly />
            </div>
          )}
          {output.text !== undefined && (
            <button type="button" class="link" onClick={() => replace({ name: name, text: output.text! })}>
              Open the {formatLabel[to]} in the editor
            </button>
          )}
        </Section>
      )}
    </>
  );
}
