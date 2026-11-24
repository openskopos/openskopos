// Example documents from the conformance suite of this repository, so the
// examples on the pages are always the cases that the tests check.
const valid = import.meta.glob("../../../conformance/valid/*.{json,yaml}", { query: "?raw", import: "default", eager: true }) as Record<string, string>;
const invalid = import.meta.glob("../../../conformance/invalid/{duplicate-item-id,constraint-without-modality,counter-below-highest-id}.json", { query: "?raw", import: "default", eager: true }) as Record<string, string>;
const markdown = import.meta.glob("../../../conformance/markdown/generous/01-start-from-scratch/INTENT.md", { query: "?raw", import: "default", eager: true }) as Record<string, string>;
const versions = import.meta.glob("../../../conformance/markdown/next-version/01-edit-final-version/{stored,expected}.json", { query: "?raw", import: "default", eager: true }) as Record<string, string>;

export interface Example {
  name: string;
  label: string;
  text: string;
}

const base = (path: string) => path.slice(path.lastIndexOf("/") + 1);
const pick = (files: Record<string, string>, name: string) => Object.entries(files).find(([p]) => p.endsWith("/" + name))?.[1] ?? "";

const labels: Record<string, string> = {
  "intent-lock-intent-v2.json": "Intent, version 2 (11 item types)",
  "spec-all-types-v3.json": "Spec with all 12 item types",
  "spec-red-team-critique.json": "Critique of a spec",
  "glossary-v1.json": "Glossary",
  "agent-rules.skopos.yaml": "Agent rules (YAML)",
  "minimal-draft.json": "Smallest valid document",
  "duplicate-item-id.json": "Invalid: two items with one ID",
  "constraint-without-modality.json": "Invalid: constraint without modality",
  "counter-below-highest-id.json": "Invalid: ID counter too low",
};

/** Valid examples first, then a Markdown file, then invalid examples. */
export const examples: Example[] = [
  ...Object.entries(valid).map(([p, text]) => ({ name: base(p), text })),
  { name: "INTENT.md", text: pick(markdown, "INTENT.md") },
  ...Object.entries(invalid).map(([p, text]) => ({ name: base(p), text })),
]
  .map((e) => ({ ...e, label: labels[e.name] ?? (e.name === "INTENT.md" ? "Intent in Markdown" : e.name) }))
  .sort((a, b) => order(a.name) - order(b.name));

function order(name: string) {
  const keys = Object.keys(labels);
  const i = keys.indexOf(name);
  return i < 0 ? (name.endsWith(".md") ? 6.5 : 99) : i;
}

/** Two versions of one artifact: a final version 2 and the draft version 3 after it. */
export const versionPair = {
  old: { name: "intent-v2.skopos.json", text: pick(versions, "stored.json") },
  new: { name: "intent-v3.skopos.json", text: pick(versions, "expected.json") },
};

/** A chain for History: the critique that version 2 takes as input, then versions 2 and 3. */
export const historyChain = [
  { name: "critique.skopos.json", text: pick(valid, "spec-red-team-critique.json") },
  versionPair.old,
  versionPair.new,
];

const merges = import.meta.glob("../../../conformance/merge/*/{main,other,base}.json", { query: "?raw", import: "default", eager: true }) as Record<string, string>;
/** Merge cases from conformance/merge, when this checkout has them. */
export const mergeCases = [...new Set(Object.keys(merges).map((p) => p.split("/").at(-2)!))].sort().map((name) => ({
  name,
  main: { name: "main.json", text: pick(merges, `${name}/main.json`) },
  other: { name: "other.json", text: pick(merges, `${name}/other.json`) },
  base: { name: "base.json", text: pick(merges, `${name}/base.json`) },
}));

export const exampleByName = (name: string) => examples.find((e) => e.name === name);
