// Every check, conversion and comparison on these pages comes from
// @openskopos/core, the WebAssembly build of the reference implementation.
// The pages never reimplement the format in JavaScript, so they give the same
// results as the skopos command line tool.
import init from "@openskopos/core";

export * as core from "@openskopos/core";
export type { Document, Item, Problem, Warning, MarkdownError, Diff, Export, ExportFile } from "@openskopos/core";

export const ready: Promise<unknown> = init();
