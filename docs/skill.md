# Agent skill

This skill teaches an AI coding agent to record decisions in the OpenSkopos
format. It shows the agent how to write a document, check and fix it with the
`skopos` tool, make new versions, and merge several documents into one. It is
one file in the [Agent Skills](https://agentskills.io) format, so it works in
Claude Code, Codex, Amp, Cursor, GitHub Copilot, Gemini CLI and other agents
that read skills.

[Download SKILL.md](../skills/openskopos/SKILL.md)

## Install

Put the file in a folder named `openskopos` in the skills folder of your
agent:

| Agent | In a project | For all your projects |
|---|---|---|
| Claude Code | `.claude/skills/openskopos/SKILL.md` | `~/.claude/skills/openskopos/SKILL.md` |
| Codex, Amp, Cursor, GitHub Copilot, Gemini CLI | `.agents/skills/openskopos/SKILL.md` | `~/.agents/skills/openskopos/SKILL.md` |

For example, to add it to a project for Codex, Amp and the other agents in
the second row:

```bash
mkdir -p .agents/skills/openskopos
curl -sSLf -o .agents/skills/openskopos/SKILL.md https://openskopos.org/skills/openskopos/SKILL.md
```

Commit the file, so everyone who works in the project gets the skill. The
agent uses the [skopos tool](install.md) if it is installed, and otherwise
`npx @openskopos/cli` or the [browser tools](https://openskopos.org/tools/).

## MCP server

If your agent cannot install the `skopos` tool or run a shell, connect it to
the OpenSkopos MCP server. The skill tells the agent when to use it. It needs
no account and stores no documents. Each IP can make 30 tool calls a minute
and 1,000 a day, and a document can have up to 500 items.

```bash
claude mcp add --transport http openskopos https://mcp.openskopos.org/mcp
```

[mcp.openskopos.org](https://mcp.openskopos.org/) has the commands for
Codex, Amp, Cursor, VS Code and Claude.

## The skill

The whole text of [SKILL.md](../skills/openskopos/SKILL.md):
