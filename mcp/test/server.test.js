import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import * as sk from "@openskopos/core";
import { createHandler } from "../src/server.js";

const root = new URL("../../", import.meta.url);
const read = (p) => readFileSync(new URL(p, root), "utf8");
const specText = read("spec/0.2/README.md");
const valid = read("conformance/valid/spec-all-types-v3.json");
const badCounter = read("conformance/invalid/counter-below-highest-id.json");

const png = new Uint8Array(readFileSync(new URL("brand/avatar-512-light.png", root)));
const images = {
  "/icon-light.png": { type: "image/png", body: png },
  "/icon-dark.png": { type: "image/png", body: png },
  "/icon.svg": { type: "image/svg+xml", body: read("brand/favicon.svg") },
};

function server(env = {}) {
  const logs = [];
  const handle = createHandler({ sk, specText, images, env: () => env, log: (r) => logs.push(JSON.stringify(r)) });
  let n = 0;
  const post = async (method, params, { ip = "203.0.113.1", body } = {}) => {
    const res = await handle(new Request("https://mcp.example/mcp", {
      method: "POST",
      headers: { "content-type": "application/json", "x-real-ip": ip },
      body: body ?? JSON.stringify({ jsonrpc: "2.0", id: ++n, method, params }),
    }));
    return { res, json: res.status === 202 ? null : await res.json() };
  };
  const call = async (name, args, opts) => (await post("tools/call", { name, arguments: args }, opts)).json.result;
  return { handle, post, call, logs };
}

const textOf = (result) => result.content.map((c) => c.text).join("\n");

/** A valid document with n requirements. */
function docWithItems(n) {
  const doc = JSON.parse(valid);
  const base = doc.items.find((i) => i.type === "requirement");
  doc.items = Array.from({ length: n }, (_, i) => ({ ...base, id: `FR${i + 1}`, text: `Requirement ${i + 1}` }));
  doc.id_counters = { FR: n };
  return JSON.stringify(doc);
}

test("answers the legacy handshake and the stateless protocol", async () => {
  const { post } = server();
  const init = await post("initialize", { protocolVersion: "2025-06-18", capabilities: {}, clientInfo: { name: "t", version: "1" } });
  assert.equal(init.json.result.protocolVersion, "2025-06-18");
  assert.ok(init.json.result.capabilities.tools);

  const old = await post("initialize", { protocolVersion: "2099-01-01", capabilities: {} });
  assert.equal(old.json.result.protocolVersion, "2025-11-25");

  const modern = { _meta: { "io.modelcontextprotocol/protocolVersion": "2026-07-28" } };
  const discover = await post("server/discover", modern);
  assert.equal(discover.json.result.resultType, "complete");
  assert.ok(discover.json.result.supportedVersions.includes("2026-07-28"));
  assert.equal(discover.json.result._meta["io.modelcontextprotocol/serverInfo"].name, "openskopos");

  const unknown = await post("tools/list", { _meta: { "io.modelcontextprotocol/protocolVersion": "1999-01-01" } });
  assert.equal(unknown.res.status, 400);
  assert.equal(unknown.json.error.code, -32022);

  const note = await post(undefined, undefined, { body: JSON.stringify({ jsonrpc: "2.0", method: "notifications/initialized" }) });
  assert.equal(note.res.status, 202);
});

test("check finds a problem with a fix, and fix gives a valid document in the format asked for", async () => {
  const { call } = server();
  assert.match(textOf(await call("check", { document: valid })), /^No problems/);

  const checked = textOf(await call("check", { document: badCounter }));
  assert.match(checked, /counters-cover-items/);
  assert.match(checked, /fix: Set the counter for S to 3/);

  const fixed = await call("fix", { document: badCounter, output_format: "yaml" });
  assert.equal(fixed.isError, undefined);
  assert.match(fixed.content[0].text, /The document is valid/);
  const doc = sk.parse(fixed.content[1].text, "yaml");
  assert.equal(doc.id_counters.S, 3);
  assert.deepEqual(sk.check(doc), []);
});

test("refuses requests and documents over the limits", async () => {
  const { post, call } = server({ MCP_MAX_BODY_BYTES: "100000", MCP_MAX_ITEMS: "50", MCP_MAX_MERGE_ITEMS: "10" });

  const big = await post("tools/call", { name: "check", arguments: { document: "x".repeat(150000) } });
  assert.equal(big.res.status, 413);

  // 51 items are over the limit of 50; 50 items are not.
  assert.match(textOf(await call("check", { document: docWithItems(50) })), /^No problems/);
  const over = await call("render", { document: docWithItems(51) });
  assert.equal(over.isError, true);
  assert.match(textOf(over), /at most 50 items/);

  const merge = await call("merge", { main: docWithItems(11), other: docWithItems(5) });
  assert.equal(merge.isError, true);
  assert.match(textOf(merge), /at most 10 items/);
});

test("limits tool calls per IP, and a flood gets 429", async () => {
  const { post, call } = server({ MCP_CALLS_PER_MINUTE: "3", MCP_REQUESTS_PER_MINUTE: "8" });
  for (let i = 0; i < 3; i++) assert.equal((await call("spec", { what: "rules" })).isError, undefined);
  const limited = await post("tools/call", { name: "spec", arguments: {} });
  assert.equal(limited.json.result.isError, true);
  assert.match(textOf(limited.json.result), /3 tool calls a minute/);
  assert.ok(Number(limited.res.headers.get("retry-after")) > 0);

  // Another IP has its own count.
  assert.equal((await call("spec", {}, { ip: "203.0.113.2" })).isError, undefined);

  for (let i = 0; i < 4; i++) await post("tools/list");
  const flood = await post("tools/list");
  assert.equal(flood.res.status, 429);
});

test("the kill switch pauses the server or one tool", async () => {
  const paused = server({ MCP_DISABLED: "1", MCP_DISABLED_MESSAGE: "Paused for maintenance." });
  const r = await paused.post("tools/list");
  assert.equal(r.res.status, 503);
  assert.equal(r.json.error.message, "Paused for maintenance.");
  const health = await paused.handle(new Request("https://mcp.example/health"));
  assert.equal(health.status, 503);

  const { call } = server({ MCP_DISABLED_TOOLS: "merge, render" });
  const off = await call("render", { document: valid });
  assert.equal(off.isError, true);
  assert.match(textOf(off), /paused/);
  assert.equal((await call("check", { document: valid })).isError, undefined);
});

test("the log has no document content and no IP address", async () => {
  const { post, call, logs } = server();
  const secret = "Zebra-7f3c-secret-text";
  const doc = JSON.parse(valid);
  doc.title = secret;
  await call("check", { document: JSON.stringify(doc) });
  await call("render", { document: `title: ${secret}\nitems: [` });
  await call("check", { document: `{"${secret}"` });
  await call("trace", { documents: [JSON.stringify(doc)], reference: secret });
  await post(undefined, undefined, { body: `{"${secret}` });
  assert.ok(logs.length >= 5);
  for (const line of logs) {
    assert.ok(!line.includes(secret), line);
    assert.ok(!line.includes("203.0.113.1"), line);
  }
  assert.ok(logs.some((l) => JSON.parse(l).tool === "check"));
});

test("the server and its tools name the logo on the origin the client used", async () => {
  const { handle } = server();
  // Behind a CDN the script gets plain HTTP and the original scheme in a header.
  const rpc = (method, params) => handle(new Request("http://mcp.example/mcp", {
    method: "POST",
    headers: { "content-type": "application/json", "x-forwarded-proto": "https" },
    body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
  })).then((r) => r.json());

  const init = await rpc("initialize", { protocolVersion: "2025-11-25", capabilities: {} });
  const icons = init.result.serverInfo.icons;
  assert.equal(icons[0].src, "https://mcp.example/icon-light.png");
  assert.equal(icons[0].mimeType, "image/png");
  assert.deepEqual(icons.map((i) => i.theme), ["light", "dark", undefined]);

  const modern = await rpc("tools/list", { _meta: { "io.modelcontextprotocol/protocolVersion": "2026-07-28" } });
  assert.deepEqual(modern.result._meta["io.modelcontextprotocol/serverInfo"].icons, icons);
  assert.ok(modern.result.tools.every((t) => t.icons?.[0].src === icons[0].src));

  const image = await handle(new Request(icons[0].src));
  assert.equal(image.status, 200);
  assert.equal(image.headers.get("content-type"), "image/png");
  assert.deepEqual(new Uint8Array(await image.arrayBuffer()), png);
});
