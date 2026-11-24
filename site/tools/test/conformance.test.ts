// Runs the conformance suite through the page logic: every valid case reads
// as valid, every invalid case shows its problem, and each problem is placed
// on the key that its JSON Pointer names.
import { readdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, test } from "vitest";
import { detectFormat, locate, read } from "../src/doc";

const root = resolve(import.meta.dirname, "../../../conformance");
const index = (dir: string) => JSON.parse(readFileSync(resolve(root, dir, "index.json"), "utf8")).cases as { file: string; check?: string; rule?: string }[];
const md = { now: "2026-11-24T00:00:00Z", artifact_id: "pa_0000000000000000" };

describe("valid cases", () => {
  for (const c of index("valid")) {
    test(c.file, () => {
      const text = readFileSync(resolve(root, "valid", c.file), "utf8");
      const r = read(text, detectFormat(text, c.file));
      expect(r.readErrors).toEqual([]);
      expect(r.problems).toEqual([]);
    });
  }
});

describe("invalid cases", () => {
  for (const c of index("invalid")) {
    test(c.file, () => {
      const text = readFileSync(resolve(root, "invalid", c.file), "utf8");
      const r = read(text, detectFormat(text, c.file));
      expect(r.doc).toBeDefined();
      expect(r.problems.length).toBeGreaterThan(0);
      if (c.check === "rules") expect(r.problems.map((p) => p.rule)).toContain(c.rule);
      // Each problem is marked in the text, on the key that its pointer ends with.
      expect(r.marks).toHaveLength(r.problems.length);
      r.problems.forEach((p, i) => {
        const last = p.path.split("/").pop()!;
        if (p.path && !/^\d+$/.test(last)) expect(text.slice(r.marks[i].from, r.marks[i].to)).toContain(`"${last}"`);
      });
    });
  }
});

describe("Markdown cases", () => {
  const dir = resolve(root, "markdown/generous");
  for (const name of readdirSync(dir)) {
    const file = readdirSync(resolve(dir, name)).find((f) => f.endsWith(".md"))!;
    test(`${name}/${file}`, () => {
      const text = readFileSync(resolve(dir, name, file), "utf8");
      expect(detectFormat(text)).toBe("markdown");
      const r = read(text, "markdown", { ...md, filename: file });
      expect(r.readErrors).toEqual([]);
      expect(r.problems).toEqual([]);
    });
  }
});

test("locate finds a nested value in YAML", () => {
  const text = "kind: intent\nitems:\n  - id: G1\n    text: ''\n  - id: FR1\n    text: Pick a plan\n";
  const [at] = locate(text, ["/items/1/text"]);
  expect(text.slice(at.from, at.to)).toBe("text: Pick a plan");
});

test("a JSON syntax error is marked at its line", () => {
  const text = '{\n  "a": 1,,\n}';
  const r = read(text, "json");
  expect(r.doc).toBeUndefined();
  expect(text.slice(0, r.marks[0].from).split("\n").length).toBe(2);
});
