// The tools of the OpenSkopos MCP server. Each tool takes documents as text
// and gives text back. Nothing is stored between calls.

/** A problem with the input that the caller can fix. The tool result has isError. */
export class ToolError extends Error {}

const FORMATS = ["json", "yaml", "markdown"];

const documentInput = (what = "The whole document") => ({
  type: "string",
  description: `${what}: OpenSkopos JSON, YAML or Markdown text, as in the file.`,
});

const formatInput = {
  type: "string",
  enum: FORMATS,
  description:
    "The format of the text. Default: from `filename`, else detected (JSON starts with '{'; Markdown has a '# ' title line).",
};

const filenameInput = {
  type: "string",
  description:
    "The name of the file, for example INTENT.md or app.skopos.yaml. Sets the format, and the kind of a Markdown file without one.",
};

const outputFormatInput = {
  type: "string",
  enum: FORMATS,
  description: "The format of the document in the result. Default: the format of the input.",
};

function formatFromFilename(name) {
  if (!name) return undefined;
  const lower = name.toLowerCase();
  if (lower.endsWith(".json")) return "json";
  if (lower.endsWith(".yaml") || lower.endsWith(".yml")) return "yaml";
  if (lower.endsWith(".md") || lower.endsWith(".markdown")) return "markdown";
  return undefined;
}

function detectFormat(text) {
  const start = text.trimStart();
  if (start.startsWith("{")) return "json";
  // A Markdown document has a '# ' title line, after the front matter if it
  // has one. A YAML document has an 'items:' key; its comments also start
  // with '# ', so it is YAML when it has that key.
  if (/^# \S/m.test(text) && !/^items:/m.test(text)) return "markdown";
  return "yaml";
}

function describeError(e) {
  if (e?.name === "SkoposError") {
    if (e.problems) return formatProblems(e.problems);
    if (e.errors) {
      return e.errors
        .map((x) => (x.line == null ? "- " : `- line ${x.line}: `) + x.message)
        .join("\n");
    }
  }
  return null;
}

/** Runs a function of the core and turns a SkoposError into a ToolError. */
function core(label, f) {
  try {
    return f();
  } catch (e) {
    const text = describeError(e);
    if (text !== null) throw new ToolError(`${label}:\n${text}`);
    throw e;
  }
}

function formatProblems(problems) {
  return problems
    .map((p) => {
      const where = p.path === "" ? "(document)" : p.path;
      const rule = p.rule ? ` [${p.rule}]` : "";
      let line = `- ${where}${rule}: ${p.message}\n  hint: ${p.hint}`;
      if (p.fix) line += `\n  fix: ${p.fix.label} (the fix tool applies it)`;
      return line;
    })
    .join("\n");
}

export function createTools({ sk, limits, specText }) {
  function read(text, { format, filename } = {}, label = "document") {
    if (typeof text !== "string" || text.length === 0) {
      throw new ToolError(`${label}: give the document as text.`);
    }
    const fmt = format ?? formatFromFilename(filename) ?? detectFormat(text);
    if (!FORMATS.includes(fmt)) throw new ToolError(`${label}: unknown format '${fmt}'.`);
    let doc;
    let warnings = [];
    if (fmt === "markdown") {
      const opts = filename ? { filename } : {};
      ({ doc, warnings } = core(`The Markdown of ${label} cannot be read`, () => sk.markdownRead(text, opts)));
    } else {
      doc = core(`The ${fmt.toUpperCase()} of ${label} cannot be read`, () => sk.parse(text, fmt));
    }
    if (doc === null || typeof doc !== "object" || Array.isArray(doc)) {
      throw new ToolError(`${label}: the text is not an OpenSkopos document (an object).`);
    }
    const items = Array.isArray(doc.items) ? doc.items.length : 0;
    if (items > limits.maxItems) {
      throw new ToolError(
        `${label} has ${items} items. This public server takes at most ${limits.maxItems} items per document. Use the skopos CLI for bigger documents: https://openskopos.org/docs/install/`,
      );
    }
    return { doc, format: fmt, warnings, items };
  }

  function write(doc, format) {
    if (format === "markdown") return core("The document cannot be written as Markdown", () => sk.markdownWrite(doc, []));
    return core(`The document cannot be written as ${format.toUpperCase()}`, () => sk.write(doc, format));
  }

  function warningsText(warnings) {
    if (!warnings?.length) return "";
    return "\n\nMarkdown reader warnings:\n" + warnings.map((w) => `- line ${w.line}: ${w.message}`).join("\n");
  }

  function readMany(texts, label) {
    if (!Array.isArray(texts) || texts.length === 0) throw new ToolError(`${label}: give at least one document.`);
    if (texts.length > limits.maxDocuments) {
      throw new ToolError(`${label}: at most ${limits.maxDocuments} documents per call.`);
    }
    return texts.map((t, i) => read(t, {}, `document ${i + 1}`).doc);
  }

  const text = (t) => ({ type: "text", text: t });
  const ro = { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false };

  const tools = [
    {
      name: "check",
      title: "Check a document",
      description:
        "Check an OpenSkopos document against the JSON Schema and the rules of the specification. Gives each problem with its JSON Pointer path, a hint for people, and whether the fix tool can fix it.",
      inputSchema: {
        type: "object",
        properties: { document: documentInput(), format: formatInput, filename: filenameInput },
        required: ["document"],
      },
      run(args) {
        let input;
        try {
          input = read(args.document, args);
        } catch (e) {
          // A document that cannot be read is a check result, not a failure of the tool.
          if (e instanceof ToolError) return { content: [text(`The document is not valid.\n${e.message}`)] };
          throw e;
        }
        const problems = sk.check(input.doc);
        const head = problems.length === 0
          ? `No problems. The ${input.format} document is valid (${input.items} items).`
          : `${problems.length} problem${problems.length === 1 ? "" : "s"}:\n${formatProblems(problems)}`;
        return { content: [text(head + warningsText(input.warnings))] };
      },
    },
    {
      name: "fix",
      title: "Fix a document",
      description:
        "Apply every fix that needs no choice from a person (for example ID counters), then check again. Gives the fixed document, the fixes applied and the problems that are left.",
      inputSchema: {
        type: "object",
        properties: { document: documentInput(), format: formatInput, filename: filenameInput, output_format: outputFormatInput },
        required: ["document"],
      },
      run(args) {
        const input = read(args.document, args);
        const { doc, fixed } = core("The fixes cannot be applied", () => sk.fixAll(input.doc));
        const left = sk.check(doc);
        const summary = [
          fixed.length ? `Applied ${fixed.length} fix${fixed.length === 1 ? "" : "es"}:\n` + fixed.map((f) => `- ${f.label}`).join("\n") : "Nothing to fix.",
          left.length ? `${left.length} problem${left.length === 1 ? "" : "s"} left for a person:\n${formatProblems(left)}` : "The document is valid.",
        ].join("\n\n");
        return { content: [text(summary), text(write(doc, args.output_format ?? input.format))] };
      },
    },
    {
      name: "convert",
      title: "Convert or format a document",
      description:
        "Write a document as JSON, YAML or Markdown in the canonical key order and layout. Use the same format to format a file (as `skopos fmt` does). Markdown is the readable agent file (specification section 10).",
      inputSchema: {
        type: "object",
        properties: { document: documentInput(), format: formatInput, filename: filenameInput, to: { ...outputFormatInput, description: "The format to write." } },
        required: ["document", "to"],
      },
      run(args) {
        const input = read(args.document, args);
        return { content: [text(write(input.doc, args.to))] };
      },
    },
    {
      name: "assign_ids",
      title: "Assign item IDs",
      description: "Give an ID to each item that has none, and update id_counters. Gives the new IDs and the document.",
      inputSchema: {
        type: "object",
        properties: { document: documentInput(), format: formatInput, filename: filenameInput, output_format: outputFormatInput },
        required: ["document"],
      },
      run(args) {
        const input = read(args.document, args);
        const r = core("IDs cannot be assigned", () => sk.assignIds(input.doc));
        const items = input.doc.items ?? [];
        const list = r.assigned.length
          ? r.assigned.map((a) => `- ${a.id}: ${String(items[a.index]?.text ?? "").slice(0, 80)}`).join("\n")
          : "Every item already has an ID.";
        return { content: [text(list), text(write(r.doc, args.output_format ?? input.format))] };
      },
    },
    {
      name: "render",
      title: "Render a document",
      description:
        "Show a document as plain text (default) or as one self-contained HTML page. With `previous`, the view marks what was added, changed and removed since that version.",
      inputSchema: {
        type: "object",
        properties: {
          document: documentInput(),
          previous: documentInput("An earlier version of the same artifact (optional)"),
          as: { type: "string", enum: ["text", "html"], description: "Default: text." },
          width: { type: "integer", minimum: 40, maximum: 200, description: "Text only: the line width. Default: 100." },
        },
        required: ["document"],
      },
      run(args) {
        const input = read(args.document, {});
        const previous = args.previous ? read(args.previous, {}, "previous").doc : undefined;
        const out = args.as === "html"
          ? core("The document cannot be rendered", () => sk.renderHtml(input.doc, previous ? { previous } : {}))
          : core("The document cannot be rendered", () => sk.renderTerminal(input.doc, { previous, color: false, width: args.width ?? 100 }));
        return { content: [text(out)] };
      },
    },
    {
      name: "diff",
      title: "Compare two versions",
      description: "The items that were added, changed, dropped and superseded between two documents, as `skopos diff` gives them.",
      inputSchema: {
        type: "object",
        properties: { old: documentInput("The older document"), new: documentInput("The newer document") },
        required: ["old", "new"],
      },
      run(args) {
        const a = read(args.old, {}, "old").doc;
        const b = read(args.new, {}, "new").doc;
        return { content: [text(core("The documents cannot be compared", () => sk.diffText(a, b)))] };
      },
    },
    {
      name: "history",
      title: "History of documents",
      description:
        "The history of several documents from one or more artifacts, in any order: each version and what changed, and the links between artifacts.",
      inputSchema: {
        type: "object",
        properties: {
          documents: { type: "array", items: documentInput("One document"), minItems: 1, description: "The documents, in any order." },
          since: { type: "integer", minimum: 0, description: "Show only the versions after this version number." },
        },
        required: ["documents"],
      },
      run(args) {
        const docs = readMany(args.documents, "history");
        const options = args.since != null ? { since: args.since } : {};
        return { content: [text(core("The history cannot be made", () => sk.historyText(docs, options)))] };
      },
    },
    {
      name: "trace",
      title: "Trace one item",
      description:
        "The life of one item across versions and artifacts, from a reference string such as 'FR3' or 'pa_xxxxxxxxxxxxxxxx#FR3'.",
      inputSchema: {
        type: "object",
        properties: {
          documents: { type: "array", items: documentInput("One document"), minItems: 1 },
          reference: { type: "string", description: "The item, for example FR3 or <artifact id>#FR3." },
        },
        required: ["documents", "reference"],
      },
      run(args) {
        const docs = readMany(args.documents, "trace");
        return { content: [text(core("The item cannot be traced", () => sk.traceText(docs, String(args.reference))))] };
      },
    },
    {
      name: "merge",
      title: "Merge two documents",
      description:
        "Merge another document into main. Gives the next version of main as a draft, with each conflict as a pair of proposed items. Resolve conflicts with the resolve tool. Give `base`, the common earlier version, for a three-way merge.",
      inputSchema: {
        type: "object",
        properties: {
          main: documentInput("The main document"),
          other: documentInput("The document to merge into main"),
          base: documentInput("The common earlier version (optional)"),
          pairs: {
            type: "array",
            items: { type: "array", items: { type: "string" }, minItems: 2, maxItems: 2 },
            description: "Pairs that people confirmed, as [main ID, other ID]. No other items pair.",
          },
          output_format: outputFormatInput,
        },
        required: ["main", "other"],
      },
      run(args) {
        const main = read(args.main, {}, "main");
        const other = read(args.other, {}, "other");
        const base = args.base ? read(args.base, {}, "base") : undefined;
        for (const d of [main, other, base]) {
          if (d && d.items > limits.maxMergeItems) {
            throw new ToolError(
              `merge takes at most ${limits.maxMergeItems} items per document on this public server. Use \`skopos merge\` for bigger documents: https://openskopos.org/docs/install/`,
            );
          }
        }
        const options = {};
        if (base) options.base = base.doc;
        if (Array.isArray(args.pairs)) options.pairs = args.pairs;
        const r = core("The documents cannot be merged", () => sk.merge(main.doc, other.doc, options));
        const summary = core("The documents cannot be merged", () => sk.mergeText(main.doc, other.doc, options));
        return { content: [text(summary), text(write(r.doc, args.output_format ?? main.format))] };
      },
    },
    {
      name: "conflicts",
      title: "Open conflicts",
      description: "The open conflicts of a merged document: each pair of items where a person must choose one.",
      inputSchema: { type: "object", properties: { document: documentInput() }, required: ["document"] },
      run(args) {
        const input = read(args.document, {});
        const list = core("The conflicts cannot be read", () => sk.conflicts(input.doc));
        const out = list.length
          ? list.map((c) => `- ${c.main} or ${c.other}${c.fields.length ? ` (${c.fields.join(", ")})` : ""}`).join("\n")
          : "No open conflicts.";
        return { content: [text(out)] };
      },
    },
    {
      name: "resolve",
      title: "Resolve a conflict",
      description: "Accept the item with this ID and reject the other item of its open conflict. Gives the new document.",
      inputSchema: {
        type: "object",
        properties: { document: documentInput(), id: { type: "string", description: "The ID of the item to accept." }, output_format: outputFormatInput },
        required: ["document", "id"],
      },
      run(args) {
        const input = read(args.document, {});
        const r = core("The conflict cannot be resolved", () => sk.resolve(input.doc, String(args.id)));
        return { content: [text(`Accepted ${r.accepted}, rejected ${r.rejected}.`), text(write(r.doc, args.output_format ?? input.format))] };
      },
    },
    {
      name: "export",
      title: "Export to a spec tool",
      description:
        "Export a document to the files of OpenSpec, Kiro or GitHub Spec Kit. Gives each file with its path, and the items that the tool has no place for.",
      inputSchema: {
        type: "object",
        properties: {
          document: documentInput(),
          tool: { type: "string", enum: ["openspec", "kiro", "speckit"] },
          name: { type: "string", description: "The name of the change or feature. Default: from the title." },
          number: { type: "integer", minimum: 1, description: "Spec Kit only: the feature number. Default: 1." },
        },
        required: ["document", "tool"],
      },
      run(args) {
        const input = read(args.document, {});
        const options = {};
        if (args.name) options.name = String(args.name);
        if (args.number != null) options.number = args.number;
        const r = core("The document cannot be exported", () => sk.exportTo(input.doc, args.tool, options));
        const parts = [`Files for ${r.tool} in ${r.folder}:`];
        for (const f of r.files) parts.push(`\n=== ${f.path} ===\n${f.text}`);
        if (r.not_exported.length) {
          parts.push("\nNot exported:\n" + r.not_exported.map((n) => `- ${n.reference}: ${n.reason}`).join("\n"));
        }
        if (r.warnings.length) parts.push("\nWarnings:\n" + r.warnings.map((w) => `- ${w}`).join("\n"));
        return { content: [text(parts.join("\n"))] };
      },
    },
    {
      name: "spec",
      title: "Read the specification",
      description:
        "The OpenSkopos 0.2 specification (Markdown), one section of it, its JSON Schema, or the names of its rules. Read section 4 for the item types and section 10 for the Markdown format.",
      inputSchema: {
        type: "object",
        properties: {
          section: { type: "integer", minimum: 1, maximum: 13, description: "One numbered section. Default: the contents." },
          what: { type: "string", enum: ["spec", "schema", "rules"], description: "Default: spec." },
        },
      },
      run(args) {
        if (args.what === "schema") return { content: [text(sk.schema())] };
        if (args.what === "rules") return { content: [text(sk.rules().join("\n"))] };
        return { content: [text(specSection(specText, args.section))] };
      },
    },
  ];

  for (const t of tools) t.annotations = { title: t.title, ...ro };
  return tools;
}

/** One `## N.` section of the specification, or the text up to the first section. */
export function specSection(specText, n) {
  const parts = specText.split(/^(?=## \d+\. )/m);
  if (n == null) {
    return parts[0].trimEnd() + "\n\nCall the spec tool with `section` to read one section, or read the resource https://openskopos.org/spec/0.2/.";
  }
  const part = parts.find((p) => p.startsWith(`## ${n}. `));
  if (!part) throw new ToolError(`The specification has no section ${n}.`);
  return part.trimEnd();
}
