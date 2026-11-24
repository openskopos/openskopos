// Agent file: one document and its inputs as one Markdown file for an agent
// (core.markdownWrite with inputs, specification 10.7), and the exports to
// OpenSpec, Kiro and Spec Kit (core.exportTo).
import { useEffect, useMemo, useState } from "preact/hooks";
import { zipSync, strToU8 } from "fflate";
import { core, type Document } from "../core";
import { Editor } from "../editor";
import { fileName } from "../doc";
import { exampleByName } from "../examples";
import type { Go, Handoff } from "../nav";
import { copy, DocInput, download, openFiles, Segmented, useRead, useStored, type TextFile } from "../ui";

type Output = "agent" | "openspec" | "kiro" | "speckit";
const empty: TextFile = { name: "", text: "" };

export function AgentFile({ incoming }: { go: Go; incoming?: Handoff }) {
  const [main, setMain] = useStored<TextFile>("agent:main", empty);
  const [inputs, setInputs] = useStored<TextFile[]>("agent:inputs", []);
  const [output, setOutput] = useState<Output>("agent");
  const mainRead = useRead(main);
  useEffect(() => void (incoming && setMain(incoming.file)), [incoming?.n]);

  return (
    <div class="agent">
      <div class="compare-bar">
        <h1>Make an agent file</h1>
        <span class="grow" />
        <button
          type="button"
          class="btn ghost"
          onClick={() => {
            const spec = exampleByName("spec-all-types-v3.json");
            const glossary = exampleByName("glossary-v1.json");
            if (spec) setMain({ name: spec.name, text: spec.text });
            if (glossary) setInputs([{ name: glossary.name, text: glossary.text }]);
          }}
        >
          Try an example
        </button>
      </div>
      <div class="agent-grid">
        <div class="agent-inputs">
          <DocInput title="Main document" value={main} onChange={setMain} result={mainRead} height="200px" />
          {inputs.map((f, i) => (
            <InputSlot key={i} file={f} n={i + 1} set={(nf) => setInputs(inputs.map((x, k) => (k === i ? nf : x)))} remove={() => setInputs(inputs.filter((_, k) => k !== i))} />
          ))}
          <div class="row wrap">
            <button type="button" class="btn ghost" onClick={() => setInputs([...inputs, empty])}>
              Add an input
            </button>
            <button type="button" class="btn ghost" onClick={async () => setInputs([...inputs, ...(await openFiles(true))])}>
              Open inputs
            </button>
            <span class="muted small-text">Inputs are glossaries, agent rules and other artifacts that the main document pins in its parents.</span>
          </div>
        </div>
        <ResultPane main={mainRead?.doc} valid={!!mainRead?.doc && !mainRead.problems.length} inputs={inputs} output={output} setOutput={setOutput} />
      </div>
    </div>
  );
}

function InputSlot({ file, n, set, remove }: { file: TextFile; n: number; set: (f: TextFile) => void; remove: () => void }) {
  const r = useRead(file);
  return <DocInput title={`Input ${n}`} value={file} onChange={set} onRemove={remove} result={r} height="140px" />;
}

function ResultPane({ main, valid, inputs, output, setOutput }: { main?: Document; valid: boolean; inputs: TextFile[]; output: Output; setOutput: (o: Output) => void }) {
  const inputDocs = inputs.map((f) => f).filter((f) => f.text.trim());
  const result = useMemo(() => {
    if (!main || !valid) return undefined;
    try {
      const docs = inputDocs.map((f) => core.parse(f.text, /^\s*[{[]/.test(f.text) ? "json" : "yaml"));
      if (output === "agent") return { files: [{ path: fileName(main, "markdown"), text: core.markdownWrite(main, docs) }], notes: [] as string[] };
      const ex = core.exportTo(main, output, {});
      return { files: ex.files, notes: [...ex.warnings, ...ex.not_exported.map((x) => `${x.id} is not exported: ${x.reason}`)] };
    } catch (e) {
      const err = e as Error & { problems?: { message: string }[] };
      return { error: err.problems?.map((p) => p.message).join("; ") || err.message };
    }
  }, [main, valid, output, JSON.stringify(inputDocs)]);
  const [pick, setPick] = useState(0);
  const files = (result && "files" in result && result.files) || [];
  const file = files[Math.min(pick, files.length - 1)];

  return (
    <div class="pane agent-out">
      <div class="pane-head">
        <Segmented
          label="Output"
          value={output}
          onChange={(o) => {
            setPick(0);
            setOutput(o);
          }}
          options={[
            { value: "agent", label: "Agent file" },
            { value: "openspec", label: "OpenSpec" },
            { value: "kiro", label: "Kiro" },
            { value: "speckit", label: "Spec Kit" },
          ]}
        />
        <span class="grow" />
        {file && (
          <>
            <button type="button" class="btn ghost small" onClick={() => copy(file.text)}>
              Copy
            </button>
            {files.length > 1 ? (
              <button type="button" class="btn small" onClick={() => download(`${output}.zip`, zipSync(Object.fromEntries(files.map((f) => [f.path, strToU8(f.text)]))), "application/zip")}>
                Download {files.length} files (.zip)
              </button>
            ) : (
              <button type="button" class="btn small" onClick={() => download(file.path.split("/").pop()!, file.text, "text/markdown")}>
                Download
              </button>
            )}
          </>
        )}
      </div>
      {!main && <p class="muted pad-x">Add a main document to see the output.</p>}
      {main && !valid && <p class="bad-text pad-x">The main document has problems. Fix them in Validate first.</p>}
      {result && "error" in result && <p class="bad-text pad-x">{result.error}</p>}
      {files.length > 1 && (
        <div class="file-tabs">
          {files.map((f, i) => (
            <button type="button" aria-pressed={f === file} onClick={() => setPick(i)}>
              {f.path}
            </button>
          ))}
        </div>
      )}
      {file && (
        <div class="pane-body">
          <Editor label={file.path} value={file.text} format="markdown" readOnly />
        </div>
      )}
      {result && "notes" in result && result.notes && result.notes.length > 0 && (
        <div class="strip notes">
          {result.notes!.map((n) => (
            <div>{n}</div>
          ))}
        </div>
      )}
    </div>
  );
}
