// The tools of the workbench.
import type { TextFile } from "./ui";

export type ToolId = "home" | "validate" | "author" | "compare" | "history" | "agent-file";

export const tools: { id: Exclude<ToolId, "home">; label: string; blurb: string }[] = [
  { id: "validate", label: "Validate", blurb: "Check a document in JSON, YAML or Markdown, see each problem at its line, and convert between formats." },
  { id: "author", label: "Author", blurb: "Write a document with item cards and the source side by side, checked as you type." },
  { id: "compare", label: "Compare", blurb: "See what was added, changed, superseded and dropped between two versions." },
  { id: "history", label: "History", blurb: "Read a chain of versions: which decisions are in force now, and which changed, were superseded, dropped or rejected." },
  { id: "agent-file", label: "Agent file", blurb: "Combine a document and its inputs into one Markdown file for an agent, or export to OpenSpec, Kiro and Spec Kit." },
];

/** A document that one tool hands to another, for example Validate to Author. */
export interface Handoff {
  tool: ToolId;
  file: TextFile;
  n: number;
}

export type Go = (tool: ToolId, file?: TextFile) => void;

