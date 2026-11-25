# OpenSkopos MCP server

A public, remote [MCP](https://modelcontextprotocol.io) server for OpenSkopos
documents. Agents use it to check, fix, convert, render, compare and merge
documents with the reference implementation, the WebAssembly build of
`skopos-core` from `@openskopos/core`.

The server is stateless. A document is in the request and in the response
only. The server keeps no documents, needs no account, and has tight limits.

## Tools

| Tool | What it does |
| --- | --- |
| `check` | Finds the problems of a document, with a hint and a fix for each |
| `fix` | Applies the fixes that need no choice from a person |
| `convert` | Writes a document as JSON, YAML or Markdown, in the canonical layout |
| `assign_ids` | Gives an ID to each item without one |
| `render` | Shows a document as text or as an HTML page |
| `diff` | Compares two versions |
| `history`, `trace` | The history of documents, and the life of one item |
| `merge`, `conflicts`, `resolve` | Merges two documents and resolves the conflicts |
| `export` | Writes the files of OpenSpec, Kiro or Spec Kit |
| `spec` | The specification, one section of it, the JSON Schema or the rules |

The server also has two resources: the specification and the JSON Schema.
The `spec` tool gives the same text to clients without resources.

The tools take documents as text: JSON, YAML or Markdown. A tool that gives a
document back gives it in the format of the input, or in `output_format`.

## Protocol

The server uses Streamable HTTP at `/mcp`, with JSON responses only. It
answers the `initialize` handshake of protocol versions 2024-11-05 to
2025-11-25, and the stateless requests of 2026-07-28 (`server/discover` and
`_meta` on each request). It has no sessions and no `GET` stream, so any
instance can answer any request.

`serverInfo` and each tool have `icons`: the OpenSkopos logo as light and dark
PNGs and as SVG. They are served by the server itself (`/icon-light.png`,
`/icon-dark.png`, `/icon.svg`, `/favicon.ico`), because clients show icons
only from the origin of the server. Each client decides whether to show them.

## Run it

```sh
npm ci
npm run build   # dist/bunny.js for Bunny Edge Scripting, dist/node.js for Node
npm start       # listens on $PORT, default 8080
npm test
```

`dist/bunny.js` is one file with the WebAssembly module and the specification
in it. Upload it as a standalone Bunny Edge Script.

## Deploy to Bunny

1. In the Bunny dashboard, add a standalone Edge Script. Bunny makes a pull
   zone for it.
2. In the script's environment, add the secret `MCP_LOG_SALT` (any long random
   string). The other settings below have defaults.
3. Add the repository secrets `BUNNY_MCP_SCRIPT_ID` and `BUNNY_MCP_DEPLOY_KEY`
   from the script's Deployments > Settings page. Then run the
   "Deploy MCP server" workflow from the Actions tab. It builds, tests,
   uploads and publishes `dist/bunny.js`.
4. On the pull zone, turn on Bunny Shield and add a rate limit rule for
   `/mcp`, for example 120 requests a minute for one IP address.
5. Add the hostname `mcp.openskopos.org` to the pull zone, and a CNAME record
   for `mcp` to the pull zone's hostname.

To go back to an earlier version, publish an earlier release from the
script's Deployments page.

## Settings

Environment variables. The defaults are for the public server.

| Variable | Default | Meaning |
| --- | --- | --- |
| `MCP_DISABLED` | off | Kill switch: `1` answers every request to `/mcp` with 503 |
| `MCP_DISABLED_MESSAGE` | | The message for clients while the server is off |
| `MCP_DISABLED_TOOLS` | | Tools to turn off, for example `merge,render` |
| `MCP_MAX_BODY_BYTES` | 1000000 | The largest request |
| `MCP_MAX_ITEMS` | 500 | The most items in one document |
| `MCP_MAX_MERGE_ITEMS` | 200 | The most items in one document for `merge`. Merge time grows with the square of the items |
| `MCP_MAX_DOCUMENTS` | 20 | The most documents for `history` and `trace` |
| `MCP_CALLS_PER_MINUTE` | 30 | Tool calls a minute for one IP address |
| `MCP_CALLS_PER_DAY` | 1000 | Tool calls a day for one IP address |
| `MCP_CPU_MS_PER_HOUR` | 60000 | Milliseconds of tool time an hour for one IP address |
| `MCP_REQUESTS_PER_MINUTE` | 120 | All requests a minute for one IP address. More get 429 |
| `MCP_GLOBAL_CALLS_PER_MINUTE` | 1200 | Tool calls a minute for one instance |
| `MCP_IP_HEADER` | `x-real-ip` | The header with the client IP address |
| `MCP_LOG_SALT` | random | A secret for the hash of IP addresses in the log |

The counters are in the memory of one instance. On an edge network they are
per location. The limit for one IP address across the whole network belongs in
front of the script, for example a Bunny Shield rate limit rule.

## Log

The server writes one JSON line for each request to `/mcp`: the time, a hash
of the IP address with a salt and the date, the user agent, the client name,
the protocol version, the method and tool, the sizes in and out, the time, and
the outcome. The log never has document text or error messages, because an
error message can quote the document.
