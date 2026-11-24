// Reading and writing documents in the three file formats, and finding where
// a problem is in the text. The reading, writing and checking come from core;
// this file only chooses the core function and maps a JSON Pointer to a place
// in the text for the editor.
import { isMap, isSeq, isScalar, parseDocument, type Node } from "yaml";
import { core, type Document, type Fix, type Problem, type Warning } from "./core";

export type Format = "json" | "yaml" | "markdown";

export const formats: Format[] = ["yaml", "json", "markdown"];
export const formatLabel: Record<Format, string> = { json: "JSON", yaml: "YAML", markdown: "Markdown" };
export const formatMime: Record<Format, string> = {
  json: "application/json",
  yaml: "application/yaml",
  markdown: "text/markdown",
};

export function formatFromName(name: string): Format | null {
  if (/\.json$/i.test(name)) return "json";
  if (/\.ya?ml$/i.test(name)) return "yaml";
  if (/\.(md|markdown)$/i.test(name)) return "markdown";
  return null;
}

/** Guesses the format of a file from its name, or else from its first lines. */
export function detectFormat(text: string, name = ""): Format {
  const byName = formatFromName(name);
  if (byName) return byName;
  const t = text.trimStart();
  if (t.startsWith("{") || t.startsWith("[")) return "json";
  // Markdown has front matter, then a body. A YAML document has no body after
  // a second "---" line.
  const front = /^---\r?\n[\s\S]*?\r?\n---[ \t]*(\r?\n|$)/.exec(t);
  if (front) return t.slice(front[0].length).trim() ? "markdown" : "yaml";
  if (/^(\$schema|id|kind|title|version|status|items):/m.test(t)) return "yaml";
  if (/^#{1,6} \S/m.test(t)) return "markdown";
  return "yaml";
}

/** The file name for a document, following section 2.1 of the specification. */
export function fileName(doc: Document | undefined, format: Format): string {
  const kind = typeof doc?.kind === "string" && /^[a-z][a-z0-9_]*$/.test(doc.kind) ? doc.kind : "document";
  if (format === "markdown") return `${kind.toUpperCase()}.md`;
  return `${kind}.skopos.${format === "json" ? "json" : "yaml"}`;
}

/** A place in the text for the editor. */
export interface Mark {
  from: number;
  to: number;
  message: string;
  severity: "error" | "warning";
  rule?: string | null;
  path?: string;
  /** Core's plain sentence that says what to do. */
  hint?: string;
  fix?: Fix;
}

export interface ReadResult {
  doc?: Document;
  /** Problems from `check`, for a document that core could read. */
  problems: Problem[];
  /** Why core could not read the text, if it could not. */
  readErrors: string[];
  /** Markdown reader warnings. */
  warnings: Warning[];
  /** All of the above, placed in the text. */
  marks: Mark[];
}

export interface MarkdownOptions {
  filename?: string;
  now?: string;
  artifact_id?: string;
}

/** Reads the text with core and checks the document. Never throws. */
export function read(text: string, format: Format, md: MarkdownOptions = {}): ReadResult {
  const result: ReadResult = { problems: [], readErrors: [], warnings: [], marks: [] };
  try {
    if (format === "markdown") {
      const r = core.markdownRead(text, md);
      result.doc = r.doc;
      result.warnings = r.warnings;
      for (const w of r.warnings) {
        const at = lineRange(text, w.line);
        result.marks.push({ ...at, message: w.message, severity: "warning" });
      }
    } else {
      result.doc = core.parse(text, format);
    }
  } catch (e) {
    const err = e as { name?: string; message: string; problems?: Problem[]; errors?: { line: number | null; message: string; problem?: Problem }[] };
    if (err.name !== "SkoposError") throw e;
    if (err.errors) {
      for (const x of err.errors) {
        result.readErrors.push(x.line ? `Line ${x.line}: ${x.message}` : x.message);
        const at = x.line ? lineRange(text, x.line) : { from: 0, to: firstLineEnd(text, 0) };
        result.marks.push({ ...at, message: x.message, severity: "error", rule: x.problem?.rule, path: x.problem?.path });
      }
    } else {
      for (const p of err.problems ?? [{ message: err.message, path: "", rule: null }]) {
        result.readErrors.push(p.message);
        const m = /line (\d+) column (\d+)/.exec(p.message);
        const at = m ? pointRange(text, +m[1], +m[2]) : { from: 0, to: firstLineEnd(text, 0) };
        result.marks.push({ ...at, message: p.message, severity: "error" });
      }
    }
    return result;
  }
  result.problems = core.check(result.doc) as Problem[];
  if (format !== "markdown") {
    const places = locate(text, result.problems.map((p) => p.path));
    result.problems.forEach((p, i) => {
      result.marks.push({ ...places[i], message: p.message, severity: "error", rule: p.rule, path: p.path, hint: p.hint, fix: p.fix });
    });
  }
  return result;
}

/** Writes a document with core in the given format. */
export function write(doc: Document, format: Format, inputs: Document[] = []): string {
  if (format === "markdown") return core.markdownWrite(doc, inputs);
  return core.write(doc, format);
}

/**
 * Finds the text of each JSON Pointer in a JSON or YAML file. A pointer to a
 * value gives the key and the value; a pointer to an object or a list gives
 * only its first line, so the editor does not underline a whole item.
 */
export function locate(text: string, pointers: string[]): { from: number; to: number }[] {
  if (!pointers.length) return [];
  const ydoc = parseDocument(text, { uniqueKeys: false, strict: false, prettyErrors: false });
  return pointers.map((pointer) => {
    let node = ydoc.contents as Node | null;
    let from = node?.range?.[0] ?? 0;
    let to = node?.range?.[1] ?? 0;
    const segments = pointer === "" ? [] : pointer.slice(1).split("/").map((s) => s.replace(/~1/g, "/").replace(/~0/g, "~"));
    for (const seg of segments) {
      if (isMap(node)) {
        const pair = node.items.find((p) => isScalar(p.key) && String(p.key.value) === seg);
        if (!pair) break;
        const key = pair.key as Node;
        const value = pair.value as Node | null;
        from = key.range?.[0] ?? from;
        to = value?.range?.[1] ?? key.range?.[1] ?? to;
        node = value;
      } else if (isSeq(node)) {
        const item = node.items[Number(seg)] as Node | undefined;
        if (!item?.range) break;
        from = item.range[0];
        to = item.range[1];
        node = item;
      } else break;
    }
    if (!isScalar(node) || to - from > 400) to = Math.max(from + 1, firstLineEnd(text, from));
    return { from: Math.min(from, text.length), to: Math.min(Math.max(to, from), text.length) };
  });
}

/** Says where a JSON Pointer is, with the item ID when there is one: "FR2 › text". */
export function describePath(doc: Document | undefined, path: string): string {
  if (path === "") return "Document";
  const parts = path.slice(1).split("/");
  if (parts[0] === "items" && parts.length > 1) {
    const item = doc?.items?.[Number(parts[1])];
    const label = typeof item?.id === "string" && item.id ? item.id : `Item ${Number(parts[1]) + 1}`;
    return [label, ...parts.slice(2)].join(" › ");
  }
  return parts.join(" › ");
}

/** The index of the item that a JSON Pointer is inside, or null. */
export function itemIndex(path: string): number | null {
  const m = /^\/items\/(\d+)(\/|$)/.exec(path);
  return m ? Number(m[1]) : null;
}

function lineStart(text: string, line: number): number {
  let at = 0;
  for (let n = 1; n < line; n++) {
    const next = text.indexOf("\n", at);
    if (next < 0) return text.length;
    at = next + 1;
  }
  return at;
}

function firstLineEnd(text: string, from: number): number {
  const end = text.indexOf("\n", from);
  return end < 0 ? text.length : end;
}

function lineRange(text: string, line: number) {
  const from = lineStart(text, line);
  return { from, to: Math.max(from, firstLineEnd(text, from)) };
}

function pointRange(text: string, line: number, column: number) {
  const start = lineStart(text, line);
  const end = firstLineEnd(text, start);
  const from = Math.min(start + Math.max(0, column - 1), end);
  return { from: from === end && from > start ? from - 1 : from, to: Math.max(from + 1, end) };
}

/** The start of each item in a JSON or YAML file, in the order of `items`. */
export function itemRanges(text: string): { from: number; to: number }[] {
  const ydoc = parseDocument(text, { uniqueKeys: false, strict: false, prettyErrors: false });
  const items = isMap(ydoc.contents) ? ydoc.contents.get("items", true) : undefined;
  if (!isSeq(items)) return [];
  return items.items.map((n) => {
    const r = (n as Node).range;
    return { from: r?.[0] ?? 0, to: r?.[1] ?? 0 };
  });
}
