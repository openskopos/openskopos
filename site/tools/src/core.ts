// Every check, conversion and comparison on these pages comes from
// @openskopos/core, the WebAssembly build of the reference implementation.
// The pages never reimplement the format in JavaScript, so they give the same
// results as the skopos command line tool.
import init, * as all from "@openskopos/core";

export const core = all;
export type {
  Document,
  Item,
  Warning,
  MarkdownError,
  Diff,
  Export,
  ExportFile,
  Fix,
  Problem,
  TextSpan,
  History,
  ItemHistory,
  ItemState,
  MergeConflict,
  MergeResult,
} from "@openskopos/core";

export const ready: Promise<unknown> = init();
