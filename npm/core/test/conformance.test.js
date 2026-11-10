// Runs the conformance suite (conformance/ in the repository) through the
// npm package: the valid and invalid documents, and the Markdown cases.
import assert from "node:assert/strict";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { test } from "node:test";

import {
  check,
  checkRules,
  checkSchema,
  markdownRead,
  markdownWrite,
  nextVersion,
  parse,
  rules,
  write,
} from "@openskopos/core";

const root = new URL("../../../conformance/", import.meta.url);
const NOW = "2026-11-10T12:00:00Z";
const FILL = { now: NOW, artifact_id: "pa_0000000000000000" };

const text = (path) => readFileSync(new URL(path, root), "utf8");
const formatOf = (path) => (/\.ya?ml$/.test(path) ? "yaml" : "json");
const doc = (path) => parse(text(path), formatOf(path));
const index = (path) => JSON.parse(text(path));

const ITEM_TYPES = [
  "goal",
  "success_signal",
  "requirement",
  "story",
  "constraint",
  "assumption",
  "non_goal",
  "risk",
  "open_question",
  "decision",
  "term",
  "critique",
];

// The document with its items in the order of the sections (section 13).
function normal(d) {
  const rank = (item) => {
    const n = ITEM_TYPES.indexOf(item.type);
    return n < 0 ? ITEM_TYPES.length : n;
  };
  const items = (d.items ?? []).slice().sort((a, b) => rank(a) - rank(b));
  return { ...d, items };
}

function readOk(md, filename, what) {
  try {
    return markdownRead(md, { ...FILL, filename });
  } catch (e) {
    assert.fail(`${what}: ${e.message}`);
  }
}

test("every valid document has no problems", () => {
  for (const { file } of index("valid/index.json").cases) {
    assert.deepEqual(check(doc(`valid/${file}`)), [], `valid/${file}`);
  }
});

test("every invalid document fails for the named rule", () => {
  const semantic = new Set();
  for (const c of index("invalid/index.json").cases) {
    const d = doc(`invalid/${c.file}`);
    const problems = check(d);
    if (c.check === "schema") {
      assert.ok(
        problems.some((p) => p.rule === null),
        `invalid/${c.file} must fail the schema`,
      );
      assert.ok(checkSchema(d).length > 0, `invalid/${c.file}`);
    } else {
      assert.equal(c.check, "semantic", `invalid/${c.file}`);
      semantic.add(c.rule);
      assert.deepEqual(checkSchema(d), [], `invalid/${c.file}`);
      assert.deepEqual(
        [...new Set(checkRules(d).map((p) => p.rule))],
        [c.rule],
        `invalid/${c.file} must break only ${c.rule}`,
      );
    }
  }
  assert.deepEqual([...semantic].sort(), rules().slice().sort());
});

test("JSON and YAML conversion keeps every value", () => {
  for (const { file } of index("valid/index.json").cases) {
    const d = doc(`valid/${file}`);
    const fromYaml = parse(write(d, "yaml"), "yaml");
    assert.deepEqual(fromYaml, d, `valid/${file} through YAML`);
    assert.deepEqual(parse(write(fromYaml, "json"), "json"), d);
  }
});

test("Markdown round trip", () => {
  for (const { case: name } of index("markdown/index.json").round_trip) {
    const dir = `markdown/round-trip/${name}/`;
    const want = doc(`${dir}document.json`);

    const got = readOk(text(`${dir}document.md`), "document.md", name);
    assert.deepEqual(normal(got.doc), normal(want), `${name}: reading document.md`);
    assert.deepEqual(got.warnings, [], `${name}: warnings`);

    const inputsDir = new URL(`${dir}inputs/`, root);
    const inputs = existsSync(inputsDir)
      ? readdirSync(inputsDir).map((f) => doc(`${dir}inputs/${f}`))
      : [];
    const md = markdownWrite(want, inputs);
    const back = readOk(md, "document.md", name);
    assert.deepEqual(normal(back.doc), normal(want), `${name}: written and read again`);
    assert.deepEqual(back.warnings, [], `${name}: warnings`);

    const body = (s) => s.split("\n---\n").slice(1).join("\n---\n");
    assert.equal(body(md), body(text(`${dir}document.md`)), `${name}: body`);
  }
});

test("Markdown generous reading", () => {
  for (const c of index("markdown/index.json").generous) {
    const dir = `markdown/generous/${c.case}/`;
    const md = text(`${dir}${c.file}`);
    const want = doc(`${dir}expected.json`);
    const got = readOk(md, c.file, c.case);
    // If the file has no id or created_at, the comparison ignores it.
    for (const key of ["id", "created_at"]) {
      if (!md.split("\n").some((l) => l.startsWith(`${key}:`))) {
        want[key] = got.doc[key];
      }
    }
    assert.deepEqual(normal(got.doc), normal(want), c.case);
    assert.equal(got.warnings.length, c.warnings, `${c.case}: warnings`);
  }
});

test("Markdown next version", () => {
  for (const { case: name } of index("markdown/index.json").next_version) {
    const dir = `markdown/next-version/${name}/`;
    const stored = doc(`${dir}stored.json`);
    const want = doc(`${dir}expected.json`);
    const read = readOk(text(`${dir}edited.md`), "edited.md", name);
    const got = nextVersion(stored, read.doc, NOW);
    if (want.version !== stored.version) want.created_at = got.created_at;
    assert.deepEqual(normal(got), normal(want), name);
    assert.deepEqual(check(got), [], `${name}: the result is valid`);
  }
});

test("Markdown invalid files are rejected", () => {
  for (const c of index("markdown/index.json").invalid) {
    const md = text(`markdown/invalid/${c.case}/${c.file}`);
    assert.throws(
      () => markdownRead(md, { ...FILL, filename: c.file }),
      (e) => {
        assert.equal(e.name, "SkoposError");
        assert.ok(e.errors.length > 0, c.case);
        for (const err of e.errors) assert.ok(err.message, c.case);
        return true;
      },
      c.case,
    );
  }
});

test("every valid document goes to Markdown and back", () => {
  for (const { file } of index("valid/index.json").cases) {
    const want = doc(`valid/${file}`);
    const md = markdownWrite(want);
    const got = readOk(md, file, file);
    assert.deepEqual(normal(got.doc), normal(want), `valid/${file}`);
    assert.deepEqual(got.warnings, [], `valid/${file}: warnings`);
    assert.equal(markdownWrite(got.doc), md, `valid/${file}: written again`);
  }
});
