// Completion in the source editors: field names and allowed values from the
// JSON Schema of core.
import type { Completion, CompletionContext, CompletionResult } from "@codemirror/autocomplete";
import { schemaInfo } from "./schema";

export function completions(ctx: CompletionContext): CompletionResult | null {
  const info = schemaInfo();
  const line = ctx.state.doc.lineAt(ctx.pos);
  const before = line.text.slice(0, ctx.pos - line.from);

  // A value after a key: `type: req` in YAML or `"type": "req` in JSON.
  const value = /"?([\w$]+)"?\s*:\s*"?([\w-]*)$/.exec(before);
  if (value && info.enums[value[1]]) {
    const { values, description } = info.enums[value[1]];
    const options: Completion[] = values.map((v) => ({ label: v, type: "enum", info: description }));
    if (value[1] === "type") options.forEach((o) => (o.detail = info.byType[o.label]?.prefix ? `${info.byType[o.label].prefix}…` : undefined));
    return { from: ctx.pos - value[2].length, options, validFor: /^[\w-]*$/ };
  }

  // A key at the start of a YAML line, or after a quote in JSON.
  const key = /^\s*(?:-\s+)?"?([\w$]*)$/.exec(before);
  if (key && (ctx.explicit || key[1].length > 0)) {
    const options: Completion[] = Object.entries(info.keys).map(([k, d]) => ({ label: k, type: "property", info: d }));
    return { from: ctx.pos - key[1].length, options, validFor: /^[\w$]*$/ };
  }
  return null;
}
