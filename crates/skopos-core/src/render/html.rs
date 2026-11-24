//! The HTML view of a document.
//!
//! The markup is semantic HTML with stable class names that start with
//! `sk-`, and `data-` attributes for the type, the status and the change of
//! each item. It has no scripts and no utility classes. The class names are
//! listed in `docs/render.md` of the repository.

use std::fmt::Write as _;

use serde_json::Value;

use super::{capital, view, Change, Field, ItemView, Line, Part, View};
use crate::words::{Op, Span};

/// The default stylesheet. Every color and font is a CSS custom property
/// (`--sk-ink`, `--sk-accent` and so on) with a light and a dark value, and
/// the sheet has rules for print. Set the properties on `:root` to change
/// them: the defaults have no specificity, so any rule wins.
pub const STYLESHEET: &str = include_str!("style.css");

/// Options for [`html`].
#[derive(Debug, Clone, Copy)]
pub struct HtmlOptions<'a> {
    /// An earlier version of the artifact. The view then marks the items
    /// that were added and changed, shows the changed words, and lists the
    /// items that were removed.
    pub previous: Option<&'a Value>,
    /// Give only the `<article class="sk-doc">` element, to put in a page of
    /// your own. The page, `style` and `css` are then left out.
    pub fragment: bool,
    /// Put [`STYLESHEET`] in the page. True by default.
    pub style: bool,
    /// More CSS to put in the page, after the default stylesheet.
    pub css: Option<&'a str>,
}

impl Default for HtmlOptions<'_> {
    fn default() -> Self {
        HtmlOptions {
            previous: None,
            fragment: false,
            style: true,
            css: None,
        }
    }
}

/// Renders a document as HTML.
///
/// The result is one self-contained page, or with
/// [`HtmlOptions::fragment`] only the document element. Each item is an
/// element with the item ID as its `id`, so `#FR4` links to it.
pub fn html(doc: &Value, options: &HtmlOptions) -> String {
    let v = view(doc, options.previous);
    let mut out = String::new();
    if options.fragment {
        article(&mut out, &v);
        return out;
    }
    out.push_str("<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    let _ = writeln!(
        out,
        "<meta name=\"generator\" content=\"skopos {}\">",
        env!("CARGO_PKG_VERSION")
    );
    let _ = writeln!(out, "<title>{}</title>", esc(&v.title));
    if options.style {
        let _ = write!(out, "<style>\n{STYLESHEET}</style>\n");
    }
    if let Some(css) = options.css {
        // `</style` in the CSS would end the element early.
        let css = css.replace("</style", "<\\/style");
        let _ = write!(out, "<style>\n{css}\n</style>\n");
    }
    out.push_str("</head>\n<body class=\"sk-page\">\n");
    article(&mut out, &v);
    out.push_str("</body>\n</html>\n");
    out
}

/// Text with `&`, `<`, `>` and quotes escaped, for text and attributes.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// A link target that is safe to put in `href`: only web and mail links.
fn safe_href(href: &str) -> Option<&str> {
    let lower = href.trim_start().to_ascii_lowercase();
    ["http://", "https://", "mailto:"]
        .iter()
        .any(|p| lower.starts_with(p))
        .then_some(href)
}

fn article(out: &mut String, v: &View) {
    let _ = write!(out, "<article class=\"sk-doc\"");
    for (name, value) in [
        ("artifact", v.artifact.as_str()),
        ("kind", &v.kind),
        ("status", &v.status),
    ] {
        if !value.is_empty() {
            let _ = write!(out, " data-{name}=\"{}\"", esc(value));
        }
    }
    if let Some(n) = v.version {
        let _ = write!(out, " data-version=\"{n}\"");
    }
    out.push_str(">\n");
    header(out, v);
    for s in &v.sections {
        let _ = write!(out, "<section class=\"sk-section\"");
        if !s.item_type.is_empty() {
            let _ = write!(out, " data-type=\"{}\"", esc(&s.item_type));
        }
        let _ = writeln!(
            out,
            ">\n<h2 class=\"sk-section-title\">{} <span class=\"sk-count\">{}</span></h2>\n<ol class=\"sk-items\">",
            esc(&s.name),
            s.items.len()
        );
        for item in &s.items {
            item_html(out, item);
        }
        out.push_str("</ol>\n</section>\n");
    }
    if let Some(since) = &v.since {
        if !since.removed.is_empty() {
            let what = since
                .version
                .map_or_else(|| "the previous version".into(), |n| format!("version {n}"));
            let _ = writeln!(
                out,
                "<section class=\"sk-section sk-removed\">\n<h2 class=\"sk-section-title\">Removed since {what} <span class=\"sk-count\">{}</span></h2>\n<ol class=\"sk-items\">",
                since.removed.len()
            );
            for r in &since.removed {
                let change = if r.by.is_empty() {
                    "dropped"
                } else {
                    "superseded"
                };
                let _ = write!(
                    out,
                    "<li class=\"sk-item sk-item--{change}\" id=\"{id}\" data-id=\"{id}\" data-type=\"{}\" data-change=\"{change}\">\n<p class=\"sk-line\"><span class=\"sk-id\">{id}</span> <del class=\"sk-text\">{}</del>",
                    esc(&r.item_type),
                    esc(&r.text),
                    id = esc(&r.id),
                );
                if !r.by.is_empty() {
                    out.push_str(" <span class=\"sk-by\">superseded by ");
                    id_links(out, &r.by);
                    out.push_str("</span>");
                }
                out.push_str("</p>\n</li>\n");
            }
            out.push_str("</ol>\n</section>\n");
        }
    }
    appendix(out, v);
    footer(out, v);
    out.push_str("</article>\n");
}

fn header(out: &mut String, v: &View) {
    out.push_str("<header class=\"sk-header\">\n<p class=\"sk-eyebrow\">");
    let mut parts = Vec::new();
    if !v.kind.is_empty() {
        parts.push(format!(
            "<span class=\"sk-kind\">{}</span>",
            esc(&capital(&super::words(&v.kind)))
        ));
    }
    if let Some(n) = v.version {
        parts.push(format!("<span class=\"sk-version\">version {n}</span>"));
    }
    if !v.status.is_empty() {
        parts.push(format!(
            "<span class=\"sk-doc-status\" data-status=\"{0}\">{0}</span>",
            esc(&v.status)
        ));
    }
    out.push_str(&parts.join(" "));
    out.push_str("</p>\n");
    let _ = writeln!(out, "<h1 class=\"sk-title\">{}</h1>", esc(&v.title));
    if !v.summary.is_empty() {
        out.push_str("<div class=\"sk-summary\">\n");
        for p in &v.summary {
            let _ = writeln!(out, "<p>{}</p>", esc(p));
        }
        out.push_str("</div>\n");
    }
    let _ = writeln!(out, "<p class=\"sk-overview\">{}</p>", esc(&v.overview));
    if let Some(since) = &v.since {
        // The counts name the dots of the items, so they are the key.
        if since.counts.is_empty() {
            let _ = write!(out, "<p class=\"sk-since\">{}", esc(&since.line));
        } else {
            let _ = write!(out, "<p class=\"sk-since\">Since {}: ", esc(&since.what));
            for (n, (count, change)) in since.counts.iter().enumerate() {
                if n > 0 {
                    out.push_str(", ");
                }
                let _ = write!(
                    out,
                    "<span class=\"sk-key\" data-change=\"{change}\">{count} {change}</span>"
                );
            }
            out.push('.');
        }
        if !since.envelope.is_empty() {
            let _ = write!(
                out,
                " <span class=\"sk-envelope\">Also changed: {}.</span>",
                esc(&since.envelope.join(", "))
            );
        }
        out.push_str("</p>\n");
        if let Some(note) = &since.note {
            let _ = writeln!(out, "<p class=\"sk-note\">{}</p>", esc(note));
        }
    }
    out.push_str("</header>\n");
}

fn item_html(out: &mut String, item: &ItemView) {
    let mut class = String::from("sk-item");
    if !item.status.is_empty() {
        let _ = write!(class, " sk-item--{}", esc(&item.status));
    }
    if !item.superseded_by.is_empty() {
        class.push_str(" sk-item--superseded");
    }
    if let Some(c) = &item.change {
        let _ = write!(class, " sk-item--{}", c.name());
    }
    let id = esc(&item.id);
    let _ = write!(out, "<li class=\"{class}\"");
    if !id.is_empty() {
        let _ = write!(out, " id=\"{id}\" data-id=\"{id}\"");
    }
    for (name, value) in [("type", &item.item_type), ("status", &item.status)] {
        if !value.is_empty() {
            let _ = write!(out, " data-{name}=\"{}\"", esc(value));
        }
    }
    if let Some(c) = &item.change {
        let _ = write!(out, " data-change=\"{}\"", c.name());
    }
    out.push_str(">\n<p class=\"sk-line\">");
    if !id.is_empty() {
        let _ = write!(out, "<a class=\"sk-id\" href=\"#{id}\">{id}</a> ");
    }
    out.push_str("<span class=\"sk-text\">");
    spans(out, &item.text);
    out.push_str("</span>");
    if !item.badges.is_empty() {
        out.push_str(" <span class=\"sk-badges\">");
        for (n, b) in item.badges.iter().enumerate() {
            if n > 0 {
                out.push(' ');
            }
            let _ = write!(
                out,
                "<span class=\"sk-badge\" data-field=\"{}\" data-value=\"{}\">{}</span>",
                b.field,
                esc(&b.value),
                esc(&b.label)
            );
        }
        out.push_str("</span>");
    }
    if !item.superseded_by.is_empty() {
        out.push_str(" <span class=\"sk-by\">superseded by ");
        id_links(out, &item.superseded_by);
        out.push_str("</span>");
    }
    out.push_str("</p>\n");
    if let Some(Change::Changed {
        fields,
        status_from,
        ..
    }) = &item.change
    {
        if !fields.is_empty() {
            let _ = write!(
                out,
                "<p class=\"sk-changed\">Changed: {}",
                esc(&fields.join(", "))
            );
            if let Some(from) = status_from {
                let _ = write!(
                    out,
                    " (was <span class=\"sk-was\">{}</span>)",
                    esc(&super::words(from))
                );
            }
            out.push_str(".</p>\n");
        }
    }
    if item.has_details() {
        out.push_str("<details class=\"sk-details\">\n<summary>Details</summary>\n");
        details_body(out, item);
        out.push_str("</details>\n");
    }
    out.push_str("</li>\n");
}

fn details_body(out: &mut String, item: &ItemView) {
    if !item.fields.is_empty() {
        fields(out, "sk-fields", &item.fields);
    }
    if !item.detail.is_empty() {
        out.push_str("<div class=\"sk-detail\">\n");
        for p in &item.detail {
            let _ = writeln!(out, "<p>{}</p>", esc(p));
        }
        out.push_str("</div>\n");
    }
}

fn fields(out: &mut String, class: &str, fields: &[Field]) {
    let _ = writeln!(out, "<dl class=\"{class}\">");
    for f in fields {
        let _ = write!(out, "<dt>{}</dt><dd>", esc(&f.name));
        if f.list {
            out.push_str("<ul>");
            for line in &f.values {
                out.push_str("<li>");
                parts(out, line);
                out.push_str("</li>");
            }
            out.push_str("</ul>");
        } else {
            for (n, line) in f.values.iter().enumerate() {
                if n > 0 {
                    out.push_str(", ");
                }
                parts(out, line);
            }
        }
        out.push_str("</dd>\n");
    }
    out.push_str("</dl>\n");
}

fn parts(out: &mut String, line: &Line) {
    for p in line {
        match p {
            Part::Text(t) => out.push_str(&esc(t)),
            Part::Code(c) => {
                let _ = write!(out, "<code>{}</code>", esc(c));
            }
            Part::Link { text, href } => match safe_href(href) {
                Some(h) => {
                    let _ = write!(out, "<a href=\"{}\">{}</a>", esc(h), esc(text));
                }
                None => out.push_str(&esc(text)),
            },
            Part::Ref { text, anchor } => match anchor {
                Some(a) => {
                    let _ = write!(
                        out,
                        "<a class=\"sk-ref\" href=\"#{}\">{}</a>",
                        esc(a),
                        esc(text)
                    );
                }
                None => {
                    let _ = write!(out, "<span class=\"sk-ref\">{}</span>", esc(text));
                }
            },
        }
    }
}

fn spans(out: &mut String, spans: &[Span]) {
    for (n, s) in spans.iter().enumerate() {
        if n > 0 {
            out.push(' ');
        }
        let t = &s.text;
        match s.op {
            Op::Same => out.push_str(&esc(t)),
            Op::Del => {
                let _ = write!(out, "<del>{}</del>", esc(t));
            }
            Op::Ins => {
                let _ = write!(out, "<ins>{}</ins>", esc(t));
            }
        }
    }
}

fn id_links(out: &mut String, ids: &[String]) {
    for (n, id) in ids.iter().enumerate() {
        if n > 0 {
            out.push_str(", ");
        }
        let _ = write!(out, "<a class=\"sk-ref\" href=\"#{0}\">{0}</a>", esc(id));
    }
}

/// The details of every item, for print. A browser does not print the
/// content of a closed `<details>`, so the stylesheet hides the item details
/// and shows this section in print. It is `hidden` on screen.
fn appendix(out: &mut String, v: &View) {
    let items: Vec<&ItemView> = v
        .sections
        .iter()
        .flat_map(|s| &s.items)
        .filter(|i| i.has_details())
        .collect();
    if items.is_empty() {
        return;
    }
    out.push_str(
        "<section class=\"sk-appendix\" hidden>\n<h2 class=\"sk-section-title\">Details</h2>\n",
    );
    for item in items {
        let _ = writeln!(
            out,
            "<section class=\"sk-appendix-item\" data-id=\"{0}\">\n<h3><span class=\"sk-id\">{0}</span></h3>",
            esc(&item.id)
        );
        details_body(out, item);
        out.push_str("</section>\n");
    }
    out.push_str("</section>\n");
}

fn footer(out: &mut String, v: &View) {
    if v.meta.is_empty() {
        return;
    }
    out.push_str("<footer class=\"sk-footer\">\n");
    fields(out, "sk-meta", &v.meta);
    out.push_str("</footer>\n");
}
