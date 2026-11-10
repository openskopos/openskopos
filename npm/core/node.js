// The entry point for Node. It loads the WebAssembly module from the package
// folder at once, so that you can call the functions with no `await init()`.
// A call of `init()` does no harm: it returns at once.
import { readFileSync } from "node:fs";
import { initSync } from "./dist/skopos.js";

initSync({
  module: readFileSync(new URL("./dist/skopos_bg.wasm", import.meta.url)),
});

export * from "./dist/skopos.js";
export { default } from "./dist/skopos.js";
