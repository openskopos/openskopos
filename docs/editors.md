# Editor support

Your editor can check an OpenSkopos document against the JSON Schema as you
type, show errors, and complete field names and values. The schema for format
0.1 is at this URL:

```
https://openskopos.org/schema/artifact/0.1.json
```

## VS Code

Every document has a `$schema` field with the URL of its format version.
VS Code uses this field to find the schema, so a document gets validation and
completion as soon as it has its `$schema` line.

JSON works with no extension. For YAML, install the
[YAML extension](https://marketplace.visualstudio.com/items?itemName=redhat.vscode-yaml)
from Red Hat.

## File names

Give a data file the suffix `.skopos.json` or `.skopos.yaml`, for example
`intent.skopos.yaml`. A
[pull request](https://github.com/SchemaStore/schemastore/pull/6625) adds
these suffixes to the [SchemaStore](https://www.schemastore.org) catalog. Once
it is merged, editors that use the catalog, such as VS Code and JetBrains
IDEs, will find the schema for a new file before it has a `$schema` line.

## Other editors

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
