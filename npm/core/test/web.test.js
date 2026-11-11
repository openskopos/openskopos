// The entry point for browsers and bundlers (dist/skopos.js), which needs
// `await init()` before the first call. Node runs each test file in its own
// process, so this file loads the module on its own.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import init, { check, parse } from "../dist/skopos.js";

test("the browser entry point works after init()", async () => {
  const wasm = new URL("../dist/skopos_bg.wasm", import.meta.url);
  await init({ module_or_path: readFileSync(wasm) });
  const text = readFileSync(
    new URL("../../../conformance/valid/minimal-draft.json", import.meta.url),
    "utf8",
  );
  assert.deepEqual(check(parse(text, "json")), []);
});
