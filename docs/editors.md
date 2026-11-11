# Editor support

An editor can check an OpenSkopos document against the JSON Schema while you
type. It then shows errors and completes field names and values. The schema
for format 0.1 is at this URL:

```
https://openskopos.org/schema/artifact/0.1.json
```

## Name your files

Give a data file the suffix `.skopos.json` or `.skopos.yaml`, for example
`intent.skopos.yaml`. The [SchemaStore](https://www.schemastore.org) catalog
connects these suffixes to the OpenSkopos schema. Editors that read the
catalog then find the schema with no setup, for example VS Code and JetBrains
IDEs.

For YAML in VS Code, install the
[YAML extension](https://marketplace.visualstudio.com/items?itemName=redhat.vscode-yaml)
from Red Hat. VS Code reads JSON with no extension.

## Name the schema in the file

Every document has the field `$schema` with the URL of its format version.
Editors that read the `$schema` field of JSON files, such as VS Code, use it
to find the schema.

If your editor does not find the schema of a YAML file, add this comment as
the first line of the file:

```yaml
# yaml-language-server: $schema=https://openskopos.org/schema/artifact/0.1.json
```

## What the schema does not check

JSON Schema cannot express some rules of the format. For example, each item
ID must be unique in a version. To check all rules, run `skopos check`:

```bash
skopos check intent.skopos.yaml
```

Section 9 of the [specification](../spec/0.1/README.md) lists these rules.
