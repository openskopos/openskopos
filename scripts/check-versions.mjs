// Makes sure that everything has the one version that "Versions" in
// CONTRIBUTING.md asks for:
//
// - The crates skopos-core, skopos and skopos-wasm and the npm packages
//   @openskopos/core and @openskopos/cli all have the same version.
// - @openskopos/cli depends on exactly that version of @openskopos/core.
// - The major and minor numbers of that version are the format version that
//   skopos-core reads and writes (the version in its SCHEMA_URL). For
//   example, the packages for format 0.2 are 0.2.0, 0.2.1 and so on.
// - The format version has its specification in spec/x.y/, its schema in
//   schema/artifact/x.y.json with that URL as $id, the same schema in
//   skopos-core, and an entry in spec/CHANGELOG.md. The version of the
//   packages has an entry in RELEASES.md.
//
// Run it from anywhere: node scripts/check-versions.mjs
import { existsSync, readFileSync } from "node:fs";

const file = (path) => new URL(`../${path}`, import.meta.url);
const read = (path) => readFileSync(file(path), "utf8");
const crateVersion = (name) => read(`crates/${name}/Cargo.toml`).match(/^version = "(.*)"$/m)[1];

const schemaUrl = read("crates/skopos-core/src/schema.rs").match(
  /SCHEMA_URL: &str = "https:\/\/openskopos\.org\/schema\/artifact\/(\d+\.\d+)\.json"/,
);
if (!schemaUrl) {
  console.error("Cannot find SCHEMA_URL in crates/skopos-core/src/schema.rs");
  process.exit(1);
}
const format = schemaUrl[1];

const core = JSON.parse(read("npm/core/package.json"));
const cli = JSON.parse(read("npm/cli/package.json"));
const found = {
  "crates/skopos-core/Cargo.toml: version": crateVersion("skopos-core"),
  "crates/skopos/Cargo.toml: version": crateVersion("skopos"),
  "crates/skopos-wasm/Cargo.toml: version": crateVersion("skopos-wasm"),
  "npm/core/package.json: version": core.version,
  "npm/cli/package.json: version": cli.version,
  'npm/cli/package.json: dependencies["@openskopos/core"]': cli.dependencies["@openskopos/core"],
};

const version = found["crates/skopos-core/Cargo.toml: version"];
let ok = true;
for (const [where, value] of Object.entries(found)) {
  if (value !== version) {
    console.error(`${where} is ${value}, but skopos-core is ${version}. All packages must have the same version.`);
    ok = false;
  }
}
if (!version.startsWith(`${format}.`) || !/^\d+\.\d+\.\d+$/.test(version)) {
  console.error(
    `The packages have version ${version}, but skopos-core reads and writes format ${format}. ` +
      `The version must be ${format}.<patch>, for example ${format}.0.`,
  );
  ok = false;
}

const fail = (message) => {
  console.error(message);
  ok = false;
};
const schemaFile = `schema/artifact/${format}.json`;
if (!existsSync(file(schemaFile))) {
  fail(`${schemaFile} is missing. Format ${format} needs its schema.`);
} else {
  if (JSON.parse(read(schemaFile)).$id !== `https://openskopos.org/schema/artifact/${format}.json`) {
    fail(`The $id of ${schemaFile} is not the URL of format ${format}.`);
  }
  const embedded = `crates/skopos-core/schema/${format}.json`;
  if (!existsSync(file(embedded)) || read(embedded) !== read(schemaFile)) {
    fail(`${embedded} must be a copy of ${schemaFile}.`);
  }
}
if (!existsSync(file(`spec/${format}/README.md`)) || !read(`spec/${format}/README.md`).startsWith(`# OpenSkopos ${format}\n`)) {
  fail(`spec/${format}/README.md is missing or does not start with "# OpenSkopos ${format}".`);
}
if (!new RegExp(`^## ${format.replace(".", "\\.")} `, "m").test(read("spec/CHANGELOG.md"))) {
  fail(`spec/CHANGELOG.md has no section for format ${format}.`);
}
if (!read("RELEASES.md").split("\n").includes(`## ${version}`)) {
  fail(`RELEASES.md has no section "## ${version}".`);
}
if (ok) {
  console.log(`All packages have version ${version}, for format ${format}.`);
}
process.exit(ok ? 0 : 1);
