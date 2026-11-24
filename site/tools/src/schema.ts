// What the pages know about the format comes from the JSON Schema that core
// ships (core.schema()): the item types, their fields, and the allowed values.
import { core } from "./core";

type JsonSchema = {
  type?: string;
  enum?: string[];
  const?: string;
  description?: string;
  format?: string;
  pattern?: string;
  items?: JsonSchema;
  $ref?: string;
  required?: string[];
  properties?: Record<string, JsonSchema>;
  allOf?: { if?: JsonSchema; then?: JsonSchema }[];
  $defs?: Record<string, JsonSchema>;
};

export type FieldKind = "text" | "line" | "enum" | "bool" | "list" | "date" | "ref" | "other";

export interface Field {
  name: string;
  kind: FieldKind;
  options?: string[];
  description?: string;
  required: boolean;
}

export interface ItemTypeInfo {
  type: string;
  prefix: string;
  label: string;
  plural: string;
  fields: Field[];
  required: string[];
}

export interface SchemaInfo {
  types: ItemTypeInfo[];
  byType: Record<string, ItemTypeInfo>;
  statuses: string[];
  relations: string[];
  /** Allowed values of each field name that has an enum, for completion. */
  enums: Record<string, { values: string[]; description?: string }>;
  /** Field names with their description, for completion. */
  keys: Record<string, string | undefined>;
}

let cached: SchemaInfo | undefined;

const plurals: Record<string, string> = {
  goal: "Goals",
  success_signal: "Success signals",
  requirement: "Requirements",
  story: "Stories",
  constraint: "Constraints",
  assumption: "Assumptions",
  non_goal: "Non-goals",
  risk: "Risks",
  open_question: "Open questions",
  decision: "Decisions",
  term: "Terms",
  critique: "Critiques",
};

const human = (s: string) => (s.charAt(0).toUpperCase() + s.slice(1)).replace(/_/g, " ").replace("Non goal", "Non-goal");

export function schemaInfo(): SchemaInfo {
  if (cached) return cached;
  const root = JSON.parse(core.schema()) as JsonSchema;
  const defs = root.$defs ?? {};
  const deref = (s: JsonSchema | undefined): JsonSchema => (s?.$ref ? { ...defs[s.$ref.split("/").pop()!], description: s.description ?? defs[s.$ref.split("/").pop()!]?.description } : (s ?? {}));
  const item = defs.item;
  const base = item.properties ?? {};
  const enums: SchemaInfo["enums"] = {};
  const keys: SchemaInfo["keys"] = {};

  const fieldOf = (name: string, raw: JsonSchema, required: string[]): Field => {
    const s = deref(raw);
    let kind: FieldKind = "other";
    if (s.enum) kind = "enum";
    else if (s.type === "boolean") kind = "bool";
    else if (s.type === "string" && s.format === "date") kind = "date";
    else if (s.type === "string") kind = name === "detail" || name === "rationale" || name === "definition" || name === "mitigation" || name === "answer" || name === "suggestion" ? "text" : "line";
    else if (s.type === "array" && deref(s.items).type === "string") kind = "list";
    if (s.enum) enums[name] = { values: s.enum, description: s.description };
    keys[name] = s.description;
    return { name, kind, options: s.enum, description: s.description, required: required.includes(name) };
  };

  const types: ItemTypeInfo[] = (base.type.enum ?? []).map((type) => {
    const branch = item.allOf?.find((b) => b.if?.properties?.type?.const === type)?.then ?? {};
    const required = [...(item.required ?? []), ...(branch.required ?? [])];
    const prefix = /^\^([A-Z]+)/.exec(branch.properties?.id?.pattern ?? "")?.[1] ?? "";
    const own = Object.entries(branch.properties ?? {}).filter(([k]) => k !== "id");
    const shared = Object.entries(base).filter(([k]) => !["id", "type", "text", "status"].includes(k));
    const fields = [...own, ...shared].map(([k, v]) => fieldOf(k, own.some(([o]) => o === k) ? { ...deref(base[k]), ...v } : v, required));
    return { type, prefix, label: human(type), plural: plurals[type] ?? human(type), fields, required };
  });

  for (const [k, v] of Object.entries(root.properties ?? {})) fieldOf(k, v, []);
  fieldOf("type", base.type, []);
  fieldOf("status", base.status, []);
  const rel = defs.itemRef?.properties?.rel;
  if (rel) fieldOf("rel", rel, []);
  for (const [k, v] of Object.entries(defs.parent?.properties ?? {})) fieldOf(k, v, []);
  for (const [k, v] of Object.entries(defs.evidence?.properties ?? {})) fieldOf(k, v, []);
  for (const [k, v] of Object.entries(defs.example?.properties ?? {})) fieldOf(k, v, []);
  for (const [k, v] of Object.entries(defs.participant?.properties ?? {})) fieldOf(k, v, []);
  // The item status and the document status share the key "status".
  enums.status = { values: [...new Set([...(base.status.enum ?? []), ...(root.properties?.status?.enum ?? [])])], description: base.status.description };

  cached = {
    types,
    byType: Object.fromEntries(types.map((t) => [t.type, t])),
    statuses: base.status.enum ?? [],
    relations: rel?.enum ?? [],
    enums,
    keys,
  };
  return cached;
}
