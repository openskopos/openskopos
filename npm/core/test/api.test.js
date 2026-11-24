// The other functions of the npm package, and its errors.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import init, {
  runCli,
  assignIds,
  check,
  diff,
  diffText,
  exportName,
  exportTo,
  history,
  historyText,
  markdownRead,
  merge,
  mergeText,
  conflicts,
  resolve,
  newArtifactId,
  nextVersion,
  parse,
  parseItemId,
  renderHtml,
  renderTerminal,
  sameContent,
  schema,
  schemaUrl,
  stylesheet,
  toJson,
  toYaml,
  wordDiff,
  trace,
  traceText,
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

test("wordDiff, and word changes in diff and merge", () => {
  assert.deepEqual(wordDiff("Count every hour.", "Count at once."), [
    { op: "same", text: "Count" },
    { op: "del", text: "every hour." },
    { op: "ins", text: "at once." },
  ]);
  const a = json("valid/spec-all-types-v3.json");
  const b = structuredClone(a);
  b.items[0].text = `${a.items[0].text} Now.`;
  const changed = diff(a, b).changed[0];
  assert.deepEqual(changed.text_spans.at(-1), { op: "ins", text: "Now." });

  const dir = "merge/02-edit-edit/";
  const [main, other, base] = ["main", "other", "base"].map((f) => json(`${dir}${f}.json`));
  const c = merge(main, other, { base }).conflicts[0];
  assert.equal(c.base_text, base.items.find((i) => i.id === "FR1").text);
  assert.ok(c.text_spans.some((s) => s.op === "del"));
});

test("history, historyText, trace and traceText", () => {
  const dir = "history/02-hand-over/";
  const docs = [json(`${dir}second-team.json`), json(`${dir}first-team.json`)];
  const h = history(docs);
  assert.deepEqual(
    h.chains.map((c) => c.artifact),
    ["pa_4Hs8Lm2Qx6Vr9Tc3", "pa_7Wn3Kd5Pz1Gy8Fb2"],
  );
  const d2 = h.items.find((i) => i.key === "pa_4Hs8Lm2Qx6Vr9Tc3#D2");
  assert.equal(d2.state, "superseded");
  assert.deepEqual(d2.superseded_by, ["pa_7Wn3Kd5Pz1Gy8Fb2@1#D1"]);
  assert.equal(d2.steps[0].change, "added");
  assert.match(historyText(docs, { since: 0 }), /superseded pa_4Hs8Lm2Qx6Vr9Tc3@1#D2  by D1/);
  assert.equal(trace(docs, "pa_4Hs8Lm2Qx6Vr9Tc3#D2").key, d2.key);
  assert.match(traceText(docs, "pa_4Hs8Lm2Qx6Vr9Tc3#D2"), /rationale: The nightly count/);
  assert.throws(() => trace(docs, "D1"), { name: "SkoposError", message: /more than one artifact/ });
  assert.throws(() => history({}), TypeError);
});

test("merge, mergeText, conflicts and resolve", () => {
  const dir = "merge/02-edit-edit/";
  const [main, other, base] = ["main", "other", "base"].map((f) => json(`${dir}${f}.json`));
  const m = merge(main, other, { base, now: "2026-11-24T12:00:00Z" });
  assert.equal(m.doc.version, main.version + 1);
  assert.equal(m.doc.created_at, "2026-11-24T12:00:00Z");
  const [c] = m.conflicts;
  assert.equal(m.conflicts.length, 1);
  assert.deepEqual([c.kind, c.main, c.other, c.fields], ["changed", "FR1", "FR3", ["text"]]);
  assert.deepEqual(check(m.doc), []);
  assert.match(mergeText(main, other, { base }), /FR1 or FR3: text/);
  assert.deepEqual(conflicts(m.doc), [{ main: "FR1", other: "FR3", fields: ["text"] }]);
  const r = resolve(m.doc, "FR3");
  assert.equal(r.rejected, "FR1");
  assert.deepEqual(conflicts(r.doc), []);
  assert.equal(conflicts(m.doc).length, 1, "resolve does not change its input");
  assert.throws(() => resolve(m.doc, "G1"), { name: "SkoposError" });

  const drafts = "merge/05-two-drafts/";
  const a = json(`${drafts}main.json`);
  const b = json(`${drafts}other.json`);
  const proposed = merge(a, b);
  assert.deepEqual(
    proposed.pairs.map((p) => [p.main, p.other, p.by]),
    [["G1", "G1", "text"], ["FR1", "FR2", "text"]],
  );
  const split = merge(a, b, { pairs: [["G1", "G1"]] });
  assert.deepEqual(split.conflicts, []);
  assert.throws(() => merge(a, b, { pairs: [["G1", "G9"]] }), { name: "SkoposError" });
});

test("renderHtml, renderTerminal and stylesheet", () => {
  const dir = "markdown/next-version/01-edit-final-version/";
  const a = json(`${dir}stored.json`);
  const b = json(`${dir}expected.json`);
  const page = renderHtml(b);
  assert.ok(page.startsWith("<!doctype html>"));
  assert.ok(page.includes(stylesheet()));
  const frag = renderHtml(b, { fragment: true, previous: a });
  assert.ok(frag.startsWith('<article class="sk-doc"'));
  assert.ok(!frag.includes("<style"));
  assert.match(frag, /class="sk-since"/);
  for (const item of b.items) assert.ok(frag.includes(` id="${item.id}" data-id="${item.id}"`));
  assert.ok(!renderHtml(b, { style: false }).includes("--sk-ink"));
  assert.ok(renderHtml(b, { style: false, css: ".x{}" }).includes(".x{}"));
  assert.throws(() => renderHtml(b, { fragment: "yes" }), TypeError);
  assert.match(stylesheet(), /--sk-accent:/);
  const text = renderTerminal(b, { previous: a, width: 50 });
  assert.ok(!text.includes("\x1b"));
  assert.ok(text.split("\n").every((l) => [...l].length <= 50));
  assert.ok(renderTerminal(b, { color: true }).includes("\x1b["));
});

// A host with files in memory, for runCli.
function memoryHost(files, platform) {
  const out = { stdout: "", stderr: "" };
  const host = {
    readFile: (path) => {
      if (!(path in files)) {
        throw Object.assign(new Error(`ENOENT: no such file or directory, open '${path}'`), {
          code: "ENOENT",
          errno: -2,
        });
      }
      return new TextEncoder().encode(files[path]);
    },
    writeFile: (path, text) => {
      files[path] = text;
    },
    exists: (path) => path in files,
    isDir: () => false,
    readDir: () => [],
    createDirAll: () => {},
    removeFile: (path) => {
      delete files[path];
    },
    removeDir: () => {},
    stdout: (text) => {
      out.stdout += text;
    },
    stderr: (text) => {
      out.stderr += text;
    },
    color: () => false,
  };
  if (platform) host.platform = () => platform;
  return { host, out };
}

test("runCli takes the kind from a file name with \\ or /", () => {
  // skopos-core splits file names at both separators, on every platform.
  for (const name of ["docs\\INTENT.md", "docs/INTENT.md"]) {
    const { host, out } = memoryHost({ [name]: "# Faster invoices\n\n## Goals\n\n- Ship it.\n" });
    assert.equal(runCli(["check", name], host), 0, out.stdout);
    assert.equal(out.stderr, "1 file: no problems.\n");
  }
});

test("runCli gives Rust messages for Node errors on Linux and macOS only", () => {
  const linux = memoryHost({}, "linux");
  assert.equal(runCli(["check", "a.json"], linux.host), 1);
  assert.match(linux.out.stdout, /cannot read the file: No such file or directory \(os error 2\)/);
  const windows = memoryHost({}, "win32");
  runCli(["check", "a.json"], windows.host);
  assert.match(windows.out.stdout, /cannot read the file: ENOENT: no such file/);
});
