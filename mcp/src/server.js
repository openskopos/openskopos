// A public, stateless MCP server for OpenSkopos over Streamable HTTP.
//
// It speaks both eras of the protocol: the `initialize` handshake of
// 2025-11-25 and earlier, and the per-request `_meta` and `server/discover`
// of 2026-07-28. Every request is complete in itself, so any instance can
// answer it. The server keeps no documents: they are in the request and the
// response only, and the log has sizes and timings, never content.
//
// The handler uses only web standard APIs (Request, Response, crypto.subtle),
// so the same code runs on Bunny Edge Scripting, Cloudflare Workers, Deno and
// Node.

import { createTools, ToolError } from "./tools.js";

const MODERN = ["2026-07-28"];
const LEGACY = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];
const SUPPORTED = [...MODERN, ...LEGACY];
const META = "io.modelcontextprotocol/";
const CACHE = { ttlMs: 3600000, cacheScope: "public" };

const INSTRUCTIONS = `Tools for OpenSkopos documents, the file format for the decisions a team makes before an AI agent builds the work. Pass the whole document text (JSON, YAML or Markdown) in each call; the server stores nothing. Start with check; fix applies the safe fixes; convert writes JSON, YAML or Markdown. Limits: LIMITS. For files on disk and bigger documents, use the skopos CLI: https://openskopos.org/docs/install/

Read the spec tool (section 4 for item types, section 10 for Markdown) before you write a new document. Record items as status: proposed unless the team decided them.`;

const num = (v, d) => (v === undefined || v === "" || Number.isNaN(Number(v)) ? d : Number(v));
const on = (v) => v !== undefined && /^(1|true|yes|on)$/i.test(String(v));

/** The settings, from environment variables. Read on every request, so a change applies at once where the platform allows it. */
export function settings(env = {}) {
  return {
    disabled: on(env.MCP_DISABLED),
    disabledMessage: env.MCP_DISABLED_MESSAGE ||
      "The OpenSkopos MCP server is paused. Use the skopos CLI instead: https://openskopos.org/docs/install/",
    disabledTools: new Set(String(env.MCP_DISABLED_TOOLS ?? "").split(",").map((s) => s.trim()).filter(Boolean)),
    maxBodyBytes: num(env.MCP_MAX_BODY_BYTES, 1_000_000),
    maxItems: num(env.MCP_MAX_ITEMS, 500),
    maxMergeItems: num(env.MCP_MAX_MERGE_ITEMS, 200),
    maxDocuments: num(env.MCP_MAX_DOCUMENTS, 20),
    callsPerMinute: num(env.MCP_CALLS_PER_MINUTE, 30),
    callsPerDay: num(env.MCP_CALLS_PER_DAY, 1000),
    requestsPerMinute: num(env.MCP_REQUESTS_PER_MINUTE, 120),
    cpuMsPerHour: num(env.MCP_CPU_MS_PER_HOUR, 60_000),
    globalCallsPerMinute: num(env.MCP_GLOBAL_CALLS_PER_MINUTE, 1200),
    ipHeader: (env.MCP_IP_HEADER || "x-real-ip").toLowerCase(),
    logSalt: env.MCP_LOG_SALT || "",
  };
}

/**
 * Counters per client and for the whole instance. They live in the memory of
 * one instance, so on an edge network they are per location and approximate:
 * they stop one client from using a whole instance. The hard limit per IP for
 * the whole network belongs in front of the script (Bunny Shield, or a
 * Cloudflare rate limiting rule).
 */
class Limiter {
  constructor() {
    this.clients = new Map();
    this.globalMinute = 0;
    this.globalCount = 0;
  }

  client(key, now) {
    let c = this.clients.get(key);
    if (!c) {
      if (this.clients.size > 50_000) this.clients.clear();
      c = { minute: 0, calls: 0, requests: 0, day: 0, dayCalls: 0, hour: 0, cpuMs: 0 };
      this.clients.set(key, c);
    }
    const minute = Math.floor(now / 60_000);
    if (c.minute !== minute) Object.assign(c, { minute, calls: 0, requests: 0 });
    const day = Math.floor(now / 86_400_000);
    if (c.day !== day) Object.assign(c, { day, dayCalls: 0 });
    const hour = Math.floor(now / 3_600_000);
    if (c.hour !== hour) Object.assign(c, { hour, cpuMs: 0 });
    return c;
  }

  /** Counts any request to /mcp. True if the client sends far too many. */
  flood(key, s, now) {
    const c = this.client(key, now);
    c.requests++;
    return c.requests > s.requestsPerMinute;
  }

  /** Counts a tool call. Gives the reason and the seconds to wait if it is over a limit. */
  call(key, s, now) {
    const c = this.client(key, now);
    const minute = Math.floor(now / 60_000);
    if (this.globalMinute !== minute) { this.globalMinute = minute; this.globalCount = 0; }
    const toNextMinute = Math.ceil((60_000 - (now % 60_000)) / 1000);
    if (this.globalCount >= s.globalCallsPerMinute) {
      return { reason: "the server is busy", retry: toNextMinute };
    }
    if (c.calls >= s.callsPerMinute) {
      return { reason: `the limit is ${s.callsPerMinute} tool calls a minute`, retry: toNextMinute };
    }
    if (c.dayCalls >= s.callsPerDay) {
      return { reason: `the limit is ${s.callsPerDay} tool calls a day`, retry: Math.ceil((86_400_000 - (now % 86_400_000)) / 1000) };
    }
    if (c.cpuMs >= s.cpuMsPerHour) {
      return { reason: "the compute limit for an hour is used up", retry: Math.ceil((3_600_000 - (now % 3_600_000)) / 1000) };
    }
    c.calls++;
    c.dayCalls++;
    this.globalCount++;
    return null;
  }

  charge(key, ms, now) {
    this.client(key, now).cpuMs += ms;
  }
}

// Spread into a new object for each response: some Response implementations
// (Hono's Node server) write content-length into the headers object they get.
const CORS = Object.freeze({
  "access-control-allow-origin": "*",
  "access-control-allow-methods": "POST, GET, OPTIONS",
  "access-control-allow-headers":
    "content-type, accept, authorization, mcp-protocol-version, mcp-session-id, mcp-method, mcp-name, last-event-id",
  "access-control-max-age": "86400",
});

function json(body, status = 200, headers = {}) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json", "cache-control": "no-store", ...CORS, ...headers },
  });
}

const rpcError = (id, code, message, data) => ({ jsonrpc: "2.0", id: id ?? null, error: { code, message, ...(data ? { data } : {}) } });

class BodyTooLarge extends Error {}

async function readBody(request, max) {
  const length = Number(request.headers.get("content-length") ?? "NaN");
  if (length > max) throw new BodyTooLarge();
  if (!request.body) return "";
  const reader = request.body.getReader();
  const chunks = [];
  let size = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    size += value.byteLength;
    if (size > max) {
      reader.cancel().catch(() => {});
      throw new BodyTooLarge();
    }
    chunks.push(value);
  }
  const all = new Uint8Array(size);
  let at = 0;
  for (const c of chunks) { all.set(c, at); at += c.byteLength; }
  return new TextDecoder().decode(all);
}

async function hashIp(ip, salt, now) {
  // The salt changes every day, so a hash cannot be followed across days or
  // turned back into the address with a table of all IPv4 addresses without the salt.
  const day = new Date(now).toISOString().slice(0, 10);
  const data = new TextEncoder().encode(`${salt}|${day}|${ip}`);
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", data));
  return Array.from(digest.slice(0, 8), (b) => b.toString(16).padStart(2, "0")).join("");
}

/**
 * Makes the request handler.
 *
 * - `sk`: the @openskopos/core module, initialized.
 * - `specText`: the Markdown of the specification.
 * - `env`: a function that gives the environment variables.
 * - `log`: a function that writes one log record (an object).
 * - `healthy`: optional; called after an unexpected error. Returns false if the core no longer works.
 * - `images`: optional; the logo files to serve, by path, such as
 *   `{ "/icon-light.png": { type: "image/png", body } }`.
 */
export function createHandler({ sk, specText, env = () => ({}), log = (r) => console.log(JSON.stringify(r)), healthy = () => true, images = {} }) {
  const limiter = new Limiter();
  const baseInfo = { name: "openskopos", title: "OpenSkopos", version: sk.version(), websiteUrl: "https://openskopos.org" };
  const randomSalt = crypto.randomUUID();
  let broken = false;

  const limitsLine = (s) =>
    `${s.maxBodyBytes >= 1e6 ? `${s.maxBodyBytes / 1e6} MB` : `${Math.round(s.maxBodyBytes / 1000)} kB`} per request, ${s.maxItems} items per document (${s.maxMergeItems} for merge), ${s.callsPerMinute} calls a minute and ${s.callsPerDay} a day per IP`;

  const resources = () => [
    {
      uri: "https://openskopos.org/spec/0.2/",
      name: "spec-0.2",
      title: "OpenSkopos specification 0.2",
      mimeType: "text/markdown",
      description: "The file format: documents, items, IDs, versions, rules and Markdown.",
    },
    {
      uri: sk.schemaUrl(),
      name: "schema-0.2",
      title: "OpenSkopos JSON Schema 0.2",
      mimeType: "application/schema+json",
      description: "The JSON Schema of a document of format 0.2.",
    },
  ];

  const resourceText = (uri) => {
    if (uri === "https://openskopos.org/spec/0.2/") return { mimeType: "text/markdown", text: specText };
    if (uri === sk.schemaUrl()) return { mimeType: "application/schema+json", text: sk.schema() };
    return null;
  };

  return async function handle(request) {
    const started = performance.now();
    const now = Date.now();
    const s = settings(env());
    const url = new URL(request.url);
    const path = url.pathname.replace(/\/+$/, "") || "/";
    // Behind a CDN or proxy the request reaches the script over HTTP.
    const proto = request.headers.get("x-forwarded-proto");
    if (proto === "https" || proto === "http") url.protocol = `${proto}:`;
    // Clients show the logo only from the same origin as the server (VS Code
    // checks this), so the icons point to this server. VS Code does not
    // accept SVG, so the PNGs come first.
    const icon = (file, extra) => ({ src: `${url.origin}${file}`, mimeType: images[file].type, ...extra });
    const icons = images["/icon-light.png"]
      ? [
          icon("/icon-light.png", { sizes: ["512x512"], theme: "light" }),
          icon("/icon-dark.png", { sizes: ["512x512"], theme: "dark" }),
          icon("/icon.svg", { sizes: ["any"] }),
        ]
      : undefined;
    const serverInfo = icons ? { ...baseInfo, icons } : baseInfo;

    if (request.method === "OPTIONS") return new Response(null, { status: 204, headers: { ...CORS } });

    if (path === "/") {
      if (request.method !== "GET" && request.method !== "HEAD") return new Response("Use POST /mcp", { status: 405, headers: { allow: "GET", ...CORS } });
      return new Response(landing(url, s, limitsLine(s)), { headers: { "content-type": "text/html; charset=utf-8", "cache-control": "public, max-age=300", ...CORS } });
    }
    if (path === "/health") {
      const ok = !s.disabled && !broken;
      return json({ status: ok ? "ok" : s.disabled ? "disabled" : "broken", version: serverInfo.version }, ok ? 200 : 503);
    }
    const image = images[url.pathname];
    if (image && (request.method === "GET" || request.method === "HEAD")) {
      return new Response(image.body, { headers: { "content-type": image.type, "cache-control": "public, max-age=86400", ...CORS } });
    }
    if (path !== "/mcp") return new Response("Not found", { status: 404, headers: { ...CORS } });

    // The GET stream for server messages (legacy) and DELETE of a session: this server has neither.
    if (request.method !== "POST") return new Response("Method not allowed", { status: 405, headers: { allow: "POST", ...CORS } });

    const ip = request.headers.get(s.ipHeader) || request.headers.get("x-forwarded-for")?.split(",")[0].trim() || "unknown";
    const record = { t: new Date(now).toISOString(), ip: await hashIp(ip, s.logSalt || randomSalt, now), ua: (request.headers.get("user-agent") || "").slice(0, 80) };
    const done = (response, extra = {}) => {
      log({ ...record, ...extra, status: response.status, ms: Math.round(performance.now() - started) });
      return response;
    };

    if (s.disabled || broken) {
      return done(json(rpcError(null, -32000, s.disabled ? s.disabledMessage : "The server has an internal problem. Use the skopos CLI: https://openskopos.org/docs/install/"), 503, { "retry-after": "3600" }), { outcome: "disabled" });
    }
    if (limiter.flood(ip, s, now)) {
      return done(json(rpcError(null, -32000, "Too many requests. Wait a minute."), 429, { "retry-after": "60" }), { outcome: "flood" });
    }

    let body;
    try {
      body = await readBody(request, s.maxBodyBytes);
    } catch (e) {
      if (e instanceof BodyTooLarge) {
        return done(json(rpcError(null, -32600, `The request is larger than ${s.maxBodyBytes} bytes. Use the skopos CLI for big documents: https://openskopos.org/docs/install/`), 413), { outcome: "too_large" });
      }
      throw e;
    }
    record.in = body.length;

    let msg;
    try {
      msg = JSON.parse(body);
    } catch {
      return done(json(rpcError(null, -32700, "Parse error"), 400), { outcome: "bad_json" });
    }
    if (Array.isArray(msg)) return done(json(rpcError(null, -32600, "Batches are not supported"), 400), { outcome: "batch" });
    if (msg === null || typeof msg !== "object" || msg.jsonrpc !== "2.0") return done(json(rpcError(null, -32600, "Invalid request"), 400), { outcome: "invalid" });

    // A notification or a response from the client: nothing to answer.
    if (msg.id === undefined || msg.id === null || typeof msg.method !== "string") {
      return done(new Response(null, { status: 202, headers: { ...CORS } }), { method: msg.method });
    }

    const { id, method } = msg;
    const params = msg.params && typeof msg.params === "object" ? msg.params : {};
    const meta = params._meta && typeof params._meta === "object" ? params._meta : {};
    const version = meta[`${META}protocolVersion`] ?? request.headers.get("mcp-protocol-version") ?? undefined;
    const modern = MODERN.includes(version);
    const client = meta[`${META}clientInfo`];
    record.method = method;
    if (version) record.pv = version;
    if (client?.name) record.client = `${String(client.name).slice(0, 40)}/${String(client.version ?? "").slice(0, 20)}`;

    if (version && !SUPPORTED.includes(version) && method !== "initialize") {
      return done(json(rpcError(id, -32022, "Unsupported protocol version", { supported: SUPPORTED, requested: version }), 400), { outcome: "version" });
    }

    const ok = (result, extra = {}, headers = {}) => {
      if (modern) {
        result = { resultType: "complete", ...result, _meta: { ...(result._meta ?? {}), [`${META}serverInfo`]: serverInfo } };
      }
      return done(json({ jsonrpc: "2.0", id, result }, 200, headers), { outcome: "ok", ...extra });
    };
    const fail = (code, message, extra = {}) => done(json(rpcError(id, code, message)), { outcome: "error", ...extra });

    const capabilities = { tools: { listChanged: false }, resources: { listChanged: false } };
    const instructions = INSTRUCTIONS.replace("LIMITS", limitsLine(s));

    switch (method) {
      case "initialize": {
        const requested = params.protocolVersion;
        if (params.clientInfo?.name) record.client = `${String(params.clientInfo.name).slice(0, 40)}/${String(params.clientInfo.version ?? "").slice(0, 20)}`;
        record.pv = LEGACY.includes(requested) ? requested : LEGACY[0];
        return ok({ protocolVersion: record.pv, capabilities, serverInfo, instructions });
      }
      case "server/discover":
        return ok({ supportedVersions: SUPPORTED, capabilities, instructions, ...CACHE });
      case "ping":
      case "logging/setLevel":
        return ok({});
      case "tools/list":
        return ok({ tools: tools(s).map(({ run, ...t }) => (icons ? { ...t, icons } : t)), ...CACHE });
      case "resources/list":
        return ok({ resources: resources(), ...CACHE });
      case "resources/templates/list":
        return ok({ resourceTemplates: [], ...CACHE });
      case "prompts/list":
        return ok({ prompts: [], ...CACHE });
      case "resources/read": {
        const r = resourceText(params.uri);
        if (!r) return fail(-32602, `Unknown resource: ${String(params.uri).slice(0, 200)}`);
        return ok({ contents: [{ uri: params.uri, ...r }], ...CACHE });
      }
      case "tools/call": {
        const name = params.name;
        const tool = tools(s).find((t) => t.name === name);
        if (!tool) return fail(-32602, `Unknown tool: ${String(name).slice(0, 100)}`);
        record.tool = name;
        if (s.disabledTools.has(name)) {
          return ok({ content: [{ type: "text", text: `The ${name} tool is paused on this server. The skopos CLI has the same command: https://openskopos.org/docs/install/` }], isError: true }, { outcome: "tool_disabled" });
        }
        const over = limiter.call(ip, s, now);
        if (over) {
          return ok(
            { content: [{ type: "text", text: `Rate limit: ${over.reason}. Try again in ${over.retry} seconds, or use the skopos CLI, which has no limits: https://openskopos.org/docs/install/` }], isError: true },
            { outcome: "rate_limited" },
            { "retry-after": String(over.retry) },
          );
        }
        const args = params.arguments && typeof params.arguments === "object" ? params.arguments : {};
        const t0 = performance.now();
        try {
          const result = tool.run(args);
          const ms = performance.now() - t0;
          limiter.charge(ip, ms, now);
          return ok(result, { cpu: Math.round(ms), out: result.content.reduce((n, c) => n + c.text.length, 0) });
        } catch (e) {
          const ms = performance.now() - t0;
          limiter.charge(ip, ms, now);
          if (e instanceof ToolError) {
            return ok({ content: [{ type: "text", text: e.message }], isError: true }, { outcome: "tool_error", cpu: Math.round(ms) });
          }
          // Not the caller's fault. Log the kind of error, never its message: it can quote the document.
          const crash = { outcome: "internal_error", err: e?.name ?? typeof e, cpu: Math.round(ms) };
          if (!healthy()) broken = true;
          return ok({ content: [{ type: "text", text: "Internal error in the OpenSkopos core. Try the skopos CLI: https://openskopos.org/docs/install/" }], isError: true }, crash);
        }
      }
      default:
        return fail(-32601, `Method not found: ${String(method).slice(0, 100)}`);
    }
  };

  function tools(s) {
    return createTools({ sk, specText, limits: s });
  }
}

function landing(url, s, limits) {
  const endpoint = `${url.origin}/mcp`;
  const status = s.disabled ? `<p><strong>Paused.</strong> ${escapeHtml(s.disabledMessage)}</p>` : "";
  return `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>OpenSkopos MCP server</title>
<link rel="icon" href="/favicon.ico" sizes="48x48"><link rel="icon" href="/icon.svg" type="image/svg+xml"><link rel="apple-touch-icon" href="/icon-light.png">
<style>body{font:16px/1.5 system-ui,sans-serif;max-width:44rem;margin:3rem auto;padding:0 1rem;color:#222}code,pre{font:14px ui-monospace,monospace;background:#f4f4f4;border-radius:4px}pre{padding:.75rem;white-space:pre-wrap;overflow-wrap:anywhere}code{padding:.1rem .3rem}</style>
</head><body>
<h1>OpenSkopos MCP server</h1>
${status}
<p>Check, fix, convert, render, compare and merge <a href="https://openskopos.org">OpenSkopos</a> documents from your agent. The tools run the reference implementation. The server stores no documents. It needs no account.</p>
<p>Endpoint (Streamable HTTP): <code>${escapeHtml(endpoint)}</code></p>
<pre>claude mcp add --transport http openskopos ${escapeHtml(endpoint)}
codex mcp add openskopos --url ${escapeHtml(endpoint)}
amp mcp add openskopos ${escapeHtml(endpoint)}</pre>
<p>Cursor and VS Code: add a server with <code>"url": "${escapeHtml(endpoint)}"</code>.</p>
<p>Limits: ${escapeHtml(limits)}. For files on disk and bigger documents, use the <a href="https://openskopos.org/docs/install/">skopos command line tool</a>.</p>
<p>Privacy: the server keeps no documents. Its log has the time, a hashed IP address that changes every day, the client name, the tool, sizes and timings.</p>
</body></html>`;
}

const escapeHtml = (t) => String(t).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
