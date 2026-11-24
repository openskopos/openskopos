// Every check, conversion and comparison on these pages comes from
// @openskopos/core, the WebAssembly build of the reference implementation.
// The pages never reimplement the format in JavaScript, so they give the same
// results as the skopos command line tool.
import init, * as all from "@openskopos/core";
import type { Diff, Document } from "@openskopos/core";

export const core = all;
export type { Document, Item, Problem, Warning, MarkdownError, Diff, Export, ExportFile } from "@openskopos/core";

export const ready: Promise<unknown> = init();

// Functions of the next release of @openskopos/core: the readable view and
// the history of several versions. The pages show those tools only when the
// loaded package has the functions. Remove these types when the pinned
// version has them.
export type ChangeKind = "seen" | "added" | "changed" | "superseded" | "dropped";
export type ItemState = "in_force" | "proposed" | "rejected" | "superseded" | "dropped";
export interface HistoryStep {
  version: number;
  change: ChangeKind;
  status?: string;
  fields?: string[];
  status_from?: string;
  status_to?: string;
  by?: string[];
  item?: Record<string, unknown>;
}
export interface ItemRefTo {
  artifact: string;
  version?: number | null;
  item: string;
}
export interface ItemHistory {
  key: string;
  artifact: string;
  id: string;
  type: string;
  text: string;
  status: string;
  reference: string;
  first_version: number;
  last_version: number;
  state: ItemState;
  changed: boolean;
  steps: HistoryStep[];
  supersedes: (ItemRefTo | string)[];
  superseded_by: (ItemRefTo | string)[];
  derived_from: (ItemRefTo | string)[];
  derived_into: (ItemRefTo | string)[];
}
export interface History {
  chains: {
    artifact: string;
    kind: string;
    title: string;
    versions: { version: number; status: string; title: string; created_at: string | null; finalized_at: string | null; items: number; diff: Diff | null }[];
  }[];
  links: { artifact: string; version: number; input_artifact: string; input_version: number; present: boolean }[];
  warnings: { artifact: string | null; message: string }[];
  items: ItemHistory[];
}
export interface MergePair { main: string; other: string; by: "id" | "text" | "given"; similarity?: number }
export interface MergeConflict { kind: "changed" | "removed_in_main" | "removed_in_other"; main: string | null; other: string | null; fields: string[] }
export interface MergeResult { doc: Document; pairs: MergePair[]; conflicts: MergeConflict[]; renamed: { from: string; to: string }[]; envelope: string[] }
export interface MergeOptions { base?: Document; pairs?: [string, string][]; now?: string }
interface Next {
  merge(main: Document, other: Document, options?: MergeOptions): MergeResult;
  mergeText(main: Document, other: Document, options?: MergeOptions): string;
  conflicts(doc: Document): { main: string; other: string; fields: string[] }[];
  resolve(doc: Document, id: string): { doc: Document; accepted: string; rejected: string };
  renderHtml(doc: Document, options?: { previous?: Document; fragment?: boolean; style?: boolean; css?: string }): string;
  stylesheet(): string;
  history(docs: Document[]): History;
  historyText(docs: Document[], options?: { since?: number }): string;
}
export const next = all as unknown as Partial<Next>;
