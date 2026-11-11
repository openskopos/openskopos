# Editor support

An editor can compare an OpenSkopos document with the JSON Schema while you
type. The editor then shows errors, and it completes field names and values.
The schema for format 0.1 is at this URL:

```
https://openskopos.org/schema/artifact/0.1.json
```

## File names

Give a data file the suffix `.skopos.json` or `.skopos.yaml`, for example
`intent.skopos.yaml`. The [SchemaStore](https://www.schemastore.org) catalog
connects these suffixes to the OpenSkopos schema. Editors that read this
catalog find the schema automatically. Examples are VS Code and JetBrains
IDEs.

For YAML files in VS Code, install the
[YAML extension](https://marketplace.visualstudio.com/items?itemName=redhat.vscode-yaml)
from Red Hat. VS Code reads JSON files without an extension.

## The schema URL in the file

Each document has the field `$schema`, which contains the URL of its format
version. Some editors use the `$schema` field of a JSON file to find the
schema, for example VS Code.

If your editor does not find the schema of a YAML file, add this comment as
the first line of the file:

```yaml
# yaml-language-server: $schema=https://openskopos.org/schema/artifact/0.1.json
```

## Rules that the schema cannot check

JSON Schema cannot express some rules of the format. For example, each item
ID in a version must be unique. To find problems with all rules, use
`skopos check`:

```bash
skopos check intent.skopos.yaml
```

Section 9 of the [specification](../spec/0.1/README.md) gives these rules.
