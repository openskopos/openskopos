# Readable views of a document

JSON, YAML and Markdown are formats for tools and for editing. The renderers
make views for people to read: text for a terminal, and HTML for a browser or
for print. The CLI, the npm package and openskopos.org all use the same
renderers in `skopos-core`, so a view looks the same everywhere.

```bash
skopos render spec.skopos.yaml                      # text in the terminal
skopos render spec.skopos.yaml --html -o spec.html  # one self-contained page
skopos render v3.skopos.yaml --previous v2.skopos.yaml --html -o v3.html
```

## What a view shows

1. The kind, version and status, the title and the summary.
2. A count of the items by status.
3. One section for each item type that has items, in the order of the
   specification. Each item is one line: its ID, its text, and short values
   such as the priority or `proposed`. The other fields are in the details.
4. With `--previous`: a line that counts what changed, a mark on each item
   that was added or changed, the removed and new words of each changed text,
   and a list of the items that were superseded or dropped.
5. Facts about the version: the reference string, the date, the people, the
   session and the sources. `x-` keys are for tools, so the view leaves them
   out.

A view shows any file, also one that is not valid. Use `skopos check` to find
problems.

## Terminal

The terminal view uses colors when the output is a terminal. It is plain text
when the output goes to a file or a pipe, or when `NO_COLOR` is set. In plain
text, changed words are marked `[-old-]` and `{+new+}`. Long lines wrap at
80 characters. Use `--width` to change that.

## HTML

`skopos render --html` writes one page with no scripts and no files to load.
The default stylesheet is in the page. It has light and dark colors and rules
for print. Each item that changed since `--previous` has a small dot to the
left of its ID: green for added, orange for changed, and a red ring for
superseded or dropped. The stylesheet resets the elements inside `.sk-doc`,
so the styles of a host page, such as `text-align: center` or a rule for
`footer`, do not change the view. In print, the details of every item move to an appendix at the end.

| Option | What it does |
|---|---|
| `--no-style` | Leaves out the default stylesheet. |
| `--css FILE` | Adds your CSS after the default stylesheet. |
| `--fragment` | Writes only the `<article class="sk-doc">` element, to put in a page of your own. |
| `--previous FILE` | Compares with an earlier version. |

In JavaScript, `renderHtml(doc, options)` takes the same options as an object:
`previous`, `fragment`, `style` and `css`. `stylesheet()` returns the default
CSS.

### Colors and fonts

Every color and font in the stylesheet is a CSS custom property. The defaults
have no specificity, so set the properties on `:root` or on `.sk-doc` to
change them:

```css
:root {
  --sk-font: Inter, sans-serif;
  --sk-accent: #e8512f;
  --sk-bg: #fff7ea;
}
@media (prefers-color-scheme: dark) {
  :root { --sk-bg: #18181b; }
}
```

| Property | Use |
|---|---|
| `--sk-font`, `--sk-mono` | Text, and IDs and code |
| `--sk-ink`, `--sk-muted` | Text, and labels and counts |
| `--sk-bg`, `--sk-surface`, `--sk-line` | Page, the item that the URL points to, and the lines under the header and above the footer |
| `--sk-accent` | The dot of a changed item, and links on hover |
| `--sk-ins`, `--sk-ins-bg` | The dot of an added item, and the tint of new words |
| `--sk-del` | Removed words, and the ring of a removed item |
| `--sk-proposed` | The `proposed` badge |
| `--sk-del-bg`, `--sk-proposed-bg` | Not used by the default stylesheet. They stay defined for stylesheets of your own. |
| `--sk-width` | The width of the text column |
| `--sk-id-width` | The width of the ID column |

### Class names

The class names and attributes are stable. You can style them and select
them in scripts.

| Class | Element |
|---|---|
| `sk-doc` | The document. `data-artifact`, `data-version`, `data-kind`, `data-status`. |
| `sk-header`, `sk-eyebrow`, `sk-kind`, `sk-version`, `sk-doc-status` | The top of the document. |
| `sk-title`, `sk-summary`, `sk-overview` | The title, the summary, and the count of items. |
| `sk-since`, `sk-envelope`, `sk-note` | With a previous version: what changed. |
| `sk-key` | In `sk-since`, one count, such as "6 added", with `data-change`. Its dot is the key to the dots of the items. |
| `sk-section`, `sk-section-title`, `sk-count`, `sk-items` | One section for each item type. `data-type` on the section. |
| `sk-item` | One item. `id` and `data-id` are the item ID, so `#FR4` links to it. `data-type`, `data-status`, and `data-change` (`added`, `changed`, `superseded` or `dropped`). |
| `sk-item--accepted`, `sk-item--proposed`, `sk-item--rejected` | The status of the item. |
| `sk-item--superseded` | An item that another item of the document supersedes. |
| `sk-item--added`, `sk-item--changed`, `sk-item--dropped` | With a previous version: the change. |
| `sk-line`, `sk-id`, `sk-text` | The line of an item, its ID and its text. Removed and new words are `<del>` and `<ins>`. |
| `sk-badges`, `sk-badge` | Short values. `data-field` (for example `priority`) and `data-value` (for example `must`). |
| `sk-by`, `sk-changed`, `sk-was` | "superseded by", and what changed. |
| `sk-details`, `sk-fields`, `sk-detail` | The details of an item: a `<details>` element with a `<dl>` of fields and the `detail` text. |
| `sk-ref` | A reference to an item. A link when the item is in the document. |
| `sk-removed` | With a previous version: the section of removed items. |
| `sk-appendix`, `sk-appendix-item` | The details of every item, for print. Hidden on screen. |
| `sk-footer`, `sk-meta` | Facts about the version. |
| `sk-page` | The `<body>` of a full page. |
