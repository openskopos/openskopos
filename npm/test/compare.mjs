// Runs the native skopos tool and the skopos tool of @openskopos/cli with the
// same arguments on the same files, and compares the results: standard
// output, standard error, the exit code and every file in the folder after
// the command.
//
//   node npm/test/compare.mjs <native skopos> <npm skopos>
//
// Each command runs in a new folder with a copy of conformance/ as `c/`.
// Both tools put the current time into a Markdown document that has no
// `created_at`, so the comparison replaces times from the run with `<now>`.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

// The commands run in other folders, so the paths of the tools must be absolute.
const [native, npm] = process.argv.slice(2).map((p) => path.resolve(p));
if (!native || !npm) {
  console.error("usage: node npm/test/compare.mjs <native skopos> <npm skopos>");
  process.exit(2);
}
const conformance = new URL("../../conformance/", import.meta.url);
const conformanceDir = fileURLToPath(conformance);
const work = fs.mkdtempSync(path.join(os.tmpdir(), "skopos-compare-"));
const started = Date.now();

const list = (dir) =>
  JSON.parse(fs.readFileSync(new URL(`${dir}/index.json`, conformance), "utf8"));
const valid = list("valid").cases.map((c) => `c/valid/${c.file}`);
const invalid = list("invalid").cases.map((c) => `c/invalid/${c.file}`);
const md = list("markdown");
const generous = md.generous.map((c) => `c/markdown/generous/${c.case}/${c.file}`);
const badMarkdown = md.invalid.map((c) => `c/markdown/invalid/${c.case}/${c.file}`);
const roundTrip = md.round_trip.map((c) => `c/markdown/round-trip/${c.case}`);
const nextVersion = md.next_version.map((c) => `c/markdown/next-version/${c.case}`);
// A Markdown file with no `id` gets a random artifact ID, so convert only
// the files that have one.
const generousWithId = generous.filter((f) =>
  /^id:/m.test(fs.readFileSync(new URL(f.slice(2), conformance), "utf8")),
);
const validJson = valid.filter((f) => f.endsWith(".json"));
const validYaml = valid.filter((f) => !f.endsWith(".json"));

// Extra files for the cases of errors and of `ids`.
function setup(dir) {
  fs.cpSync(conformanceDir, path.join(dir, "c"), {
    recursive: true,
    // Leave out conformance/runner/, the Rust program that checks the cases.
    filter: (src) => path.relative(conformanceDir, src).split(path.sep)[0] !== "runner",
  });
  fs.mkdirSync(path.join(dir, "folder.json"));
  fs.writeFileSync(path.join(dir, "latin1.json"), Buffer.from([0x7b, 0xe9, 0x7d]));
  fs.writeFileSync(path.join(dir, "notes.txt"), "text\n");
  fs.writeFileSync(path.join(dir, "broken.yaml"), "a: [1\n");
  const spec = JSON.parse(fs.readFileSync(path.join(dir, "c/valid/spec-all-types-v3.json")));
  delete spec.items[0].id;
  spec.items[2].id = "";
  fs.writeFileSync(path.join(dir, "no-ids.json"), JSON.stringify(spec, null, 2));
  spec.items[1].type = "wish";
  delete spec.items[1].id;
  fs.writeFileSync(path.join(dir, "unknown-type.json"), JSON.stringify(spec));
  fs.mkdirSync(path.join(dir, "specs/004-other"), { recursive: true });
  fs.mkdirSync(path.join(dir, "specs/20261110-093000-timed"), { recursive: true });
}

const cases = [
  // Arguments and help.
  [],
  ["--help"],
  ["--version"],
  ["help", "check"],
  ["check", "--help"],
  ["export", "--help"],
  ["lint", "x.json"],
  ["check"],
  ["convert", "--to", "xml", valid[0]],
  ["export", valid[0], "--to", "jira"],
  ["diff", valid[0]],
  ["check", "--jsn", valid[0]],

  // check
  ["check", ...valid],
  ["check", "--json", ...valid],
  ["check", ...invalid],
  ["check", "--json", ...invalid],
  ["check", ...generous],
  ["check", "--json", ...generous],
  ["check", ...badMarkdown],
  ["check", "--json", ...badMarkdown],
  ["check", ...roundTrip.map((d) => `${d}/document.md`)],
  ["check", "missing.json", "folder.json", "latin1.json", "notes.txt", "broken.yaml"],
  ["check", "--json", "missing.json", "folder.json", "latin1.json", "notes.txt"],
  ["check", valid[0], invalid[0]],

  // fmt
  ["fmt", "--check", ...valid],
  ["fmt", ...valid],
  ["fmt", "--check", "no-ids.json", "unknown-type.json"],
  ["fmt", "no-ids.json", "missing.json", "broken.yaml", generous[0]],

  // convert
  ...valid.flatMap((f) => [
    ["convert", "--to", "yaml", f],
    ["convert", "--to", "json", f],
    ["convert", "--to", "md", f],
  ]),
  ...roundTrip.flatMap((d) => [
    ["convert", "--to", "json", `${d}/document.md`],
    ["convert", "--to", "markdown", `${d}/document.json`],
  ]),
  ["convert", "--to", "md", `${roundTrip[2]}/document.json`, ...inputs(roundTrip[2])],
  ...generousWithId.map((f) => ["convert", "--to", "yaml", f]),
  ["convert", "--to", "yaml", validJson[0], "-o", "out.yaml"],
  ["convert", "--to", "md", validYaml[0], "--output", "OUT.md"],
  ["convert", "--to", "json", badMarkdown[0]],
  ["convert", "--to", "md", valid[0], "--input", "missing.json"],
  ["convert", "--to", "json", valid[0], "-o", "no-folder/out.json"],

  // import
  ...nextVersion.flatMap((d) => [
    ["import", `${d}/edited.md`, "--stored", `${d}/stored.json`],
    ["import", `${d}/edited.md`, "--stored", `${d}/stored.json`, "-o", "stored.yaml"],
  ]),
  ["import", valid[0], "--stored", valid[0]],
  ["import", generous[0], "--stored", "missing.json"],
  ["import", generous[0], "--stored", validJson[0], "-o", "out.md"],
  ["import", badMarkdown[0], "--stored", validJson[0]],

  // ids
  ["ids", "--check", ...valid],
  ["ids", "--check", "no-ids.json"],
  ["ids", "no-ids.json"],
  ["ids", "unknown-type.json", "missing.json", generous[0]],

  // export
  ...["openspec", "kiro", "speckit"].flatMap((tool) =>
    valid.flatMap((f) => [
      ["export", f, "--to", tool, "--dry-run"],
      ["export", f, "--to", tool, "-o", "project"],
    ]),
  ),
  ["export", validJson[2], "--to", "speckit"],
  ["export", validJson[2], "--to", "speckit", "--name", "seat limits", "--number", "12"],
  ["export", validJson[2], "--to", "openspec", "--name", "billing/seats", "-o", "project"],
  ["export", invalid[0], "--to", "kiro"],
  ["export", generousWithId[0], "--to", "kiro", "--output", "project"],

  // diff
  ...nextVersion.flatMap((d) => [
    ["diff", `${d}/stored.json`, `${d}/expected.json`],
    ["diff", "--json", `${d}/stored.json`, `${d}/expected.json`],
  ]),
  ["diff", valid[0], valid[1]],
  ["diff", valid[0], invalid[0]],
  ["diff", "missing.json", valid[0]],
];

// Commands that run twice in the same folder: the second export finds the
// files of the first.
const twice = [
  ["export", validJson[2], "--to", "kiro"],
  ["export", validJson[2], "--to", "kiro", "--force"],
  ["export", validJson[2], "--to", "openspec", "--name", "seats"],
  ["export", validJson[2], "--to", "speckit", "--force"],
];

function inputs(dir) {
  const files = fs.readdirSync(new URL(`${dir.slice(2)}/inputs/`, conformance));
  return files.flatMap((f) => ["--input", `${dir}/inputs/${f}`]);
}

// Times of this run, in the forms that the tools write.
function maskNow(text) {
  return text.replace(/\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ/g, (t) => {
    const ms = Date.parse(t);
    return ms >= started - 2000 && ms <= Date.now() + 2000 ? "<now>" : t;
  });
}

function tree(dir) {
  const out = {};
  const walk = (d) => {
    for (const name of fs.readdirSync(d).sort()) {
      const p = path.join(d, name);
      const rel = path.relative(dir, p);
      if (fs.statSync(p).isDirectory()) {
        out[`${rel}/`] = "";
        walk(p);
      } else {
        out[rel] = maskNow(fs.readFileSync(p, "latin1"));
      }
    }
  };
  walk(dir);
  return out;
}

function run(bin, dir, args) {
  const r = spawnSync(bin, args, { cwd: dir, encoding: "utf8", env: { ...process.env, NO_COLOR: "1" } });
  if (r.error) throw r.error;
  return { status: r.status, stdout: maskNow(r.stdout), stderr: maskNow(r.stderr) };
}

let n = 0;
let failed = 0;
function compare(sequence) {
  n += 1;
  const dirs = ["native", "npm"].map((side) => {
    const dir = path.join(work, `${n}-${side}`);
    fs.mkdirSync(dir);
    setup(dir);
    return dir;
  });
  const label = sequence.map((a) => `skopos ${a.join(" ")}`).join(" && ");
  try {
    for (const args of sequence) {
      const a = run(native, dirs[0], args);
      const b = run(npm, dirs[1], args);
      assert.deepEqual(b, a, label);
    }
    assert.deepEqual(tree(dirs[1]), tree(dirs[0]), `${label}: files`);
  } catch (e) {
    failed += 1;
    console.error(`DIFFERENT: ${label}\n${e.message}\n`);
  }
  for (const dir of dirs) fs.rmSync(dir, { recursive: true, force: true });
}

for (const args of cases) compare([args]);
compare(twice);
fs.rmSync(work, { recursive: true, force: true });

console.log(`${n - failed} of ${n} comparisons give the same result.`);
process.exit(failed === 0 ? 0 : 1);
