# Editor support

Your editor can check an OpenSkopos document against the JSON Schema as you
type, show errors, and complete field names and values. The schema for format
0.1 is at this URL:

```
https://openskopos.org/schema/artifact/0.1.json
```

## The `$schema` field

Every document has a `$schema` field with the URL of its format version.
Editors that check YAML with
[yaml-language-server](https://github.com/redhat-developer/yaml-language-server),
or JSON with the JSON language server from VS Code, use this field to find
the schema. So a document gets validation and completion as soon as it has
its `$schema` line.

VS Code and the editors based on it, such as Cursor and Windsurf, have the
JSON language server built in, and get yaml-language-server from the
[YAML extension](https://open-vsx.org/extension/redhat/vscode-yaml) by Red
Hat. Zed and Helix use both servers by default. Neovim and Emacs can use them
through their language server support.

## File names

Give a data file the suffix `.skopos.json` or `.skopos.yaml`, for example
`intent.skopos.yaml`. The [SchemaStore](https://www.schemastore.org) catalog
connects these suffixes to the OpenSkopos schema, so editors that use the
catalog find the schema for a new file before it has a `$schema` line. These
include JetBrains IDEs and any editor that runs yaml-language-server with its
default settings.

## If your editor does not find the schema

If your editor does not find the schema of a YAML file, add this comment as
the first line of the file:

```yaml
# yaml-language-server: $schema=https://openskopos.org/schema/artifact/0.1.json
```

## Rules that the schema cannot check

JSON Schema cannot express some rules of the format. For example, each item
ID in a version must be unique. To check every rule, run `skopos check`:

```bash
skopos check intent.skopos.yaml
```

Section 9 of the [specification](../spec/0.1/README.md) lists these rules.
