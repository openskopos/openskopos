// The other functions of the npm package, and its errors.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import init, {
  assignIds,
  check,
  diff,
  diffText,
  exportName,
  exportTo,
  markdownRead,
  newArtifactId,
  nextVersion,
  parse,
  parseItemId,
  sameContent,
  schema,
  schemaUrl,
  toJson,
  toYaml,
  version,
  write,
} from "@openskopos/core";

const root = new URL("../../../conformance/", import.meta.url);
const json = (path) => parse(readFileSync(new URL(path, root), "utf8"), "json");
const pkg = JSON.parse(readFileSync(new URL("../package.json", import.meta.url)));

test("init() is not needed in Node, and does no harm", async () => {
  await init();
  assert.deepEqual(check(json("valid/minimal-draft.json")), []);
});

test("the version is the version of the package", () => {
  assert.equal(version(), pkg.version);
});

test("the schema is the published schema", () => {
  const s = JSON.parse(schema());
  assert.equal(s.$id, schemaUrl());
  assert.equal(schemaUrl(), "https://openskopos.org/schema/artifact/0.1.json");
});

test("parse throws a SkoposError with one problem", () => {
  assert.throws(
    () => parse("{", "json"),
    (e) => e instanceof Error && e.name === "SkoposError" && e.problems.length === 1,
  );
  assert.throws(() => parse("{}", "toml"), TypeError);
});

test("problems are plain objects", () => {
  const d = json("invalid/duplicate-item-id.json");
  const [p] = check(d);
  assert.deepEqual(Object.keys(p).sort(), ["message", "path", "rule"]);
  assert.equal(p.rule, "unique-item-ids");
  assert.equal(Object.getPrototypeOf(p), Object.prototype);
});

test("write, toJson and toYaml give the canonical layout", () => {
  const d = json("valid/minimal-draft.json");
  assert.equal(toJson(d), write(d, "json"));
  assert.equal(toYaml(d), write(d, "yaml"));
  assert.ok(toYaml(d).startsWith(`$schema: ${schemaUrl()}\n`));
});

test("assignIds gives IDs and does not change its input", () => {
  const d = json("valid/spec-all-types-v3.json");
  const id = d.items[0].id;
  delete d.items[0].id;
  const out = assignIds(d);
  assert.equal(d.items[0].id, undefined);
  assert.equal(out.assigned.length, 1);
  assert.equal(out.assigned[0].index, 0);
  assert.notEqual(out.assigned[0].id, id);
  assert.equal(out.doc.items[0].id, out.assigned[0].id);
  assert.equal(typeof out.counters_changed, "boolean");
});

test("parseItemId", () => {
  assert.deepEqual(parseItemId("FR4"), { type: "requirement", number: 4 });
  assert.equal(parseItemId("FR04"), undefined);
});

test("newArtifactId matches the schema", () => {
  const id = newArtifactId();
  assert.match(id, /^pa_[0-9A-Za-z]{16}$/);
  assert.notEqual(id, newArtifactId());
});

test("markdownRead fills a new ID and the current time", () => {
  const { doc, warnings } = markdownRead("# Faster invoices\n\n## Goals\n\n- Ship it.\n", {
    filename: "INTENT.md",
  });
  assert.deepEqual(warnings, []);
  assert.equal(doc.kind, "intent");
  assert.match(doc.id, /^pa_[0-9A-Za-z]{16}$/);
  assert.match(doc.created_at, /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$/);
  assert.deepEqual(check(doc), []);
});

test("nextVersion and sameContent", () => {
  const dir = "markdown/next-version/02-unchanged-final-version/";
  const stored = json(`${dir}stored.json`);
  const md = readFileSync(new URL(`${dir}edited.md`, root), "utf8");
  const read = markdownRead(md, { filename: "edited.md" }).doc;
  assert.ok(sameContent(stored, read));
  assert.deepEqual(nextVersion(stored, read), stored);
});

test("exportTo and exportName", () => {
  const d = json("valid/spec-all-types-v3.json");
  const out = exportTo(d, "kiro");
  assert.equal(out.tool, "kiro");
  assert.equal(out.folder, `.kiro/specs/${exportName(d)}`);
  assert.ok(out.files.length > 0);
  assert.ok(Array.isArray(out.not_exported));
  const named = exportTo(d, "speckit", { name: "seats", number: 7 });
  assert.equal(named.folder, "specs/007-seats");
  assert.throws(() => exportTo(d, "jira"), TypeError);
});

test("diff and diffText", () => {
  const dir = "markdown/next-version/01-edit-final-version/";
  const a = json(`${dir}stored.json`);
  const b = json(`${dir}expected.json`);
  const d = diff(a, b);
  assert.equal(d.same_artifact, true);
  assert.equal(d.from.version + 1, d.to.version);
  assert.equal(typeof diffText(a, b), "string");
});
