# Use OpenSkopos from an agent

The OpenSkopos MCP server lets an AI agent check, fix, convert, render,
compare and merge OpenSkopos documents. It runs the reference implementation,
so the results are the same as from the [skopos tool](install.md). It is free
and needs no account.

The server is at:

```text
https://mcp.openskopos.org/mcp
```

It uses the Streamable HTTP transport of the
[Model Context Protocol](https://modelcontextprotocol.io).

For files on disk, for big documents and for scripts and CI, use the
[skopos command line tool](install.md). The MCP server is for agents that
cannot run commands, and for a quick start.

## Connect your agent

### Claude Code

```bash
claude mcp add --transport http openskopos https://mcp.openskopos.org/mcp
```

### Codex

```bash
codex mcp add openskopos --url https://mcp.openskopos.org/mcp
```

### Amp

```bash
amp mcp add openskopos https://mcp.openskopos.org/mcp
```

### Cursor

Add the server to `~/.cursor/mcp.json`, or to `.cursor/mcp.json` in your
project:

```json
{
  "mcpServers": {
    "openskopos": {
      "url": "https://mcp.openskopos.org/mcp"
    }
  }
}
```

### VS Code

Add the server to `.vscode/mcp.json` in your project:

```json
{
  "servers": {
    "openskopos": {
      "type": "http",
      "url": "https://mcp.openskopos.org/mcp"
    }
  }
}
```

### Claude on the web and in the desktop app

In Settings > Connectors, add a custom connector with the URL
`https://mcp.openskopos.org/mcp`.

### Other clients

Add a remote server with the URL `https://mcp.openskopos.org/mcp`. The server
needs no sign-in and no API key.

## Tools

| Tool | What it does |
| --- | --- |
| `check` | Finds the problems of a document, with a hint and a fix for each |
| `fix` | Applies the fixes that need no choice from a person |
| `convert` | Writes a document as JSON, YAML or Markdown, in the canonical layout |
| `assign_ids` | Gives an ID to each item without one |
| `render` | Shows a document as text or as an HTML page |
| `diff` | Compares two versions of a document |
| `history` | The history of a list of versions |
| `trace` | The life of one item across a list of versions |
| `merge` | Merges two documents that changed from the same base |
| `conflicts` | Lists the conflicts of a merge |
| `resolve` | Resolves the conflicts with your choices |
| `export` | Writes the files of OpenSpec, Kiro or Spec Kit |
| `spec` | The specification, one section of it, the JSON Schema or the rules |

The tools take documents as text: JSON, YAML or Markdown. A tool that gives a
document back gives it in the format of the input, or in the format that you
ask for. The tools do not change files. Your agent writes the result to disk.

The server also has two resources: the
[specification 0.2](../spec/0.2/README.md) and the
[JSON Schema 0.2](../schema/artifact/0.2.json).

## Limits

The server is free for everyone, so it has limits:

- 1 MB for each request.
- 500 items in one document, and 200 items for `merge`.
- 20 documents for `history` and `trace`.
- For one IP address: 30 tool calls a minute, 1,000 tool calls a day, and
  one minute of tool time an hour.

When your agent goes over a limit, the tool tells it how long to wait. If you
need more, use the [skopos command line tool](install.md). It has no limits.

## Privacy

The server keeps no documents. A document is in the request and in the
response only.

The server writes a log for each request: the time, the client name, the
tool, the sizes, the time it took and the outcome. The log has a hash of
your IP address that changes each day. It never has the text of a document or
of an error message.

## Source

The server is in the
[mcp folder](https://github.com/openskopos/openskopos/tree/main/mcp) of the
repository, under the Apache License 2.0. You can run your own copy with
Node.
