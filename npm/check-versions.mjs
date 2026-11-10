// Makes sure that the npm packages have the version of the Rust crates, and
// that @openskopos/cli depends on the same version of @openskopos/core.
import { readFileSync } from "node:fs";

const read = (path) => readFileSync(new URL(path, import.meta.url), "utf8");
const crate = read("../crates/skopos-wasm/Cargo.toml").match(/^version = "(.*)"$/m)[1];
const core = JSON.parse(read("core/package.json"));
const cli = JSON.parse(read("cli/package.json"));

const found = {
  "npm/core/package.json: version": core.version,
  "npm/cli/package.json: version": cli.version,
  'npm/cli/package.json: dependencies["@openskopos/core"]': cli.dependencies["@openskopos/core"],
};
let ok = true;
for (const [where, value] of Object.entries(found)) {
  if (value !== crate) {
    console.error(`${where} is ${value}, but the crates have version ${crate}`);
    ok = false;
  }
}
process.exit(ok ? 0 : 1);
