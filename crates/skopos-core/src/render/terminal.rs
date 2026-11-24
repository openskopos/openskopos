//! The terminal view of a document.
//!
//! With colors, the view uses ANSI escape codes: bold for titles and IDs,
//! dim for details, red and green for removed and new words. Without colors
//! it is plain text, and changed words are marked `[-old-]` and `{+new+}`.

use serde_json::Value;

use super::{capital, view, words, Change, Field, ItemView, Line, Part, Span, View};

/// Options for [`terminal`].
#[derive(Debug, Clone, Copy)]
pub struct TerminalOptions<'a> {
    /// An earlier version of the artifact, as for
    /// [`super::HtmlOptions::previous`].
    pub previous: Option<&'a Value>,
    /// Use ANSI colors. Give `false` when the output is not a terminal or
    /// `NO_COLOR` is set.
    pub color: bool,
    /// The width of a line, in characters. Long texts wrap at this width.
    /// The default is 80.
    pub width: usize,
}

impl Default for TerminalOptions<'_> {
    fn default() -> Self {
        TerminalOptions {
            previous: None,
            color: false,
            width: 80,
        }
    }
}

/// Renders a document as text for a terminal.
pub fn terminal(doc: &Value, options: &TerminalOptions) -> String {
    let v = view(doc, options.previous);
    let mut t = Term {
        out: String::new(),
        color: options.color,
        width: options.width.max(20),
    };
    t.document(&v);
    t.out
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Style {
    Plain,
    Bold,
    Dim,
    Accent,
    Del,
    Ins,
    Warn,
    Bad,
}

impl Style {
    fn code(self) -> &'static str {
        match self {
            Style::Plain => "",
            Style::Bold => "\x1b[1m",
            Style::Dim => "\x1b[2m",
            Style::Accent => "\x1b[38;5;203m",
            Style::Del => "\x1b[31;9m",
            Style::Ins => "\x1b[32m",
            Style::Warn => "\x1b[33m",
            Style::Bad => "\x1b[31m",
        }
    }
}

/// A word and its style. Words are joined with one space.
struct Word {
    text: String,
    style: Style,
}

fn w(text: impl Into<String>, style: Style) -> Word {
    Word {
        text: text.into(),
        style,
    }
}

fn len(s: &str) -> usize {
    s.chars().count()
}

struct Term {
    out: String,
    color: bool,
    width: usize,
}

impl Term {
    fn paint(&self, text: &str, style: Style) -> String {
        if !self.color || style == Style::Plain || text.is_empty() {
            text.to_string()
        } else {
            format!("{}{text}\x1b[0m", style.code())
        }
    }

    /// Words of a text in one style.
    fn split(text: &str, style: Style) -> Vec<Word> {
        text.split_whitespace().map(|t| w(t, style)).collect()
    }

    /// Words of a text with changes. Without colors, a removed run is
    /// `[-...-]` and a new run is `{+...+}`.
    fn spans(&self, spans: &[Span], base: Style) -> Vec<Word> {
        let mut out = Vec::new();
        for s in spans {
            let (text, style, open, close) = match s {
                Span::Same(t) => (t, base, "", ""),
                Span::Del(t) => (t, Style::Del, "[-", "-]"),
                Span::Ins(t) => (t, Style::Ins, "{+", "+}"),
            };
            let mut ws = Self::split(text, style);
            if !self.color && !ws.is_empty() {
                ws[0].text.insert_str(0, open);
                ws.last_mut().expect("not empty").text.push_str(close);
            }
            out.extend(ws);
        }
        out
    }

    /// Writes words, wrapped at the width. The first line starts with
    /// `first` (of visible width `first_len`), the next lines with `indent`
    /// spaces.
    fn wrap(&mut self, first: &str, first_len: usize, indent: usize, words: &[Word]) {
        let mut line = first.to_string();
        let mut col = first_len;
        let mut empty = true;
        for word in words {
            let n = len(&word.text);
            if !empty && col + 1 + n > self.width {
                self.out.push_str(line.trim_end());
                self.out.push('\n');
                line = " ".repeat(indent);
                col = indent;
                empty = true;
            }
            if !empty {
                line.push(' ');
                col += 1;
            }
            line.push_str(&self.paint(&word.text, word.style));
            col += n;
            empty = false;
        }
        self.out.push_str(line.trim_end());
        self.out.push('\n');
    }

    fn blank(&mut self) {
        if !self.out.is_empty() && !self.out.ends_with("\n\n") {
            self.out.push('\n');
        }
    }

    fn document(&mut self, v: &View) {
        let mut eyebrow = Vec::new();
        if !v.kind.is_empty() {
            eyebrow.push(capital(&words(&v.kind)));
        }
        if let Some(n) = v.version {
            eyebrow.push(format!("version {n}"));
        }
        if !v.status.is_empty() {
            eyebrow.push(v.status.clone());
        }
        let mark = format!(
            "{}{}",
            self.paint("\u{3a3}", Style::Bold),
            self.paint("\u{2022}", Style::Accent)
        );
        let line = format!(
            "{mark} {}",
            self.paint(&eyebrow.join(" \u{b7} "), Style::Dim)
        );
        self.out.push_str(line.trim_end());
        self.out.push('\n');
        let title = Self::split(&v.title, Style::Bold);
        self.wrap("", 0, 0, &title);
        for p in &v.summary {
            self.blank();
            self.wrap("", 0, 0, &Self::split(p, Style::Plain));
        }
        self.blank();
        self.wrap("", 0, 0, &Self::split(&v.overview, Style::Dim));
        if let Some(since) = &v.since {
            let mut ws = Self::split(&since.line, Style::Plain);
            if !since.envelope.is_empty() {
                ws.extend(Self::split(
                    &format!("Also changed: {}.", since.envelope.join(", ")),
                    Style::Dim,
                ));
            }
            self.wrap("", 0, 0, &ws);
            if let Some(note) = &since.note {
                self.wrap("", 0, 0, &Self::split(note, Style::Warn));
            }
        }

        let marks = v.since.is_some();
        let id_width = v
            .sections
            .iter()
            .flat_map(|s| s.items.iter().map(|i| len(&i.id)))
            .chain(
                v.since
                    .iter()
                    .flat_map(|s| s.removed.iter().map(|r| len(&r.id))),
            )
            .max()
            .unwrap_or(0);
        for s in &v.sections {
            self.blank();
            let heading = format!(
                "{} {}",
                self.paint(&s.name, Style::Bold),
                self.paint(&format!("({})", s.items.len()), Style::Dim)
            );
            self.out.push_str(&heading);
            self.out.push('\n');
            for item in &s.items {
                self.item(item, marks, id_width);
            }
        }
        if let Some(since) = &v.since {
            if !since.removed.is_empty() {
                self.blank();
                let what = since
                    .version
                    .map_or_else(|| "the previous version".into(), |n| format!("version {n}"));
                let heading = format!(
                    "{} {}",
                    self.paint(&format!("Removed since {what}"), Style::Bold),
                    self.paint(&format!("({})", since.removed.len()), Style::Dim)
                );
                self.out.push_str(&heading);
                self.out.push('\n');
                for r in &since.removed {
                    let first = format!(
                        "{} {}  ",
                        self.paint("-", Style::Bad),
                        self.paint(&format!("{:id_width$}", r.id), Style::Bold)
                    );
                    let mut ws = Self::split(&r.text, Style::Del);
                    if !r.by.is_empty() {
                        ws.extend(Self::split(
                            &format!("(superseded by {})", r.by.join(", ")),
                            Style::Dim,
                        ));
                    }
                    self.wrap(&first, 2 + id_width + 2, 2 + id_width + 2, &ws);
                }
            }
        }
        if !v.meta.is_empty() {
            self.blank();
            for f in &v.meta {
                self.field(f, 0);
            }
        }
    }

    fn item(&mut self, item: &ItemView, marks: bool, id_width: usize) {
        let (mark, style) = match &item.change {
            Some(Change::Added) => ("+", Style::Ins),
            Some(Change::Changed { .. }) => ("~", Style::Accent),
            None => (" ", Style::Plain),
        };
        let mark = if marks {
            format!("{} ", self.paint(mark, style))
        } else {
            "  ".to_string()
        };
        let first = format!(
            "{mark}{}  ",
            self.paint(&format!("{:id_width$}", item.id), Style::Bold)
        );
        let indent = 2 + id_width + 2;
        let faded = item.status == "rejected" || !item.superseded_by.is_empty();
        // A new item needs no `{+...+}`: the `+` before its ID says it.
        let mut ws = match &item.change {
            Some(Change::Added) => {
                let text: Vec<&str> = item.text.iter().map(Span::text).collect();
                Self::split(&text.join(" "), Style::Ins)
            }
            _ => self.spans(&item.text, if faded { Style::Dim } else { Style::Plain }),
        };
        for b in &item.badges {
            let style = match b.value.as_str() {
                "proposed" => Style::Warn,
                "rejected" => Style::Bad,
                _ => Style::Dim,
            };
            ws.push(w(format!("[{}]", b.label), style));
        }
        if !item.superseded_by.is_empty() {
            ws.extend(Self::split(
                &format!("(superseded by {})", item.superseded_by.join(", ")),
                Style::Dim,
            ));
        }
        self.wrap(&first, indent, indent, &ws);
        if let Some(Change::Changed {
            fields,
            status_from,
        }) = &item.change
        {
            let mut s = format!("changed: {}", fields.join(", "));
            if let Some(from) = status_from {
                s.push_str(&format!(" (was {})", words(from)));
            }
            let pad = " ".repeat(indent);
            self.wrap(&pad, indent, indent, &Self::split(&s, Style::Accent));
        }
        for f in &item.fields {
            self.field(f, indent);
        }
        for p in &item.detail {
            let pad = " ".repeat(indent);
            self.wrap(&pad, indent, indent, &Self::split(p, Style::Dim));
        }
    }

    fn line_words(line: &Line) -> Vec<Word> {
        let mut out = Vec::new();
        for p in line {
            match p {
                Part::Text(t) | Part::Code(t) | Part::Ref { text: t, .. } => {
                    out.extend(Self::split(t, Style::Plain));
                }
                Part::Link { text, href } => {
                    out.extend(Self::split(text, Style::Plain));
                    if text != href && !href.is_empty() {
                        out.push(w(format!("<{href}>"), Style::Dim));
                    }
                }
            }
        }
        // Punctuation that a part starts with belongs to the word before.
        let mut joined: Vec<Word> = Vec::new();
        for word in out {
            if matches!(word.text.chars().next(), Some(',' | ':' | ')')) {
                if let Some(prev) = joined.last_mut() {
                    prev.text.push_str(&word.text);
                    continue;
                }
            }
            joined.push(word);
        }
        joined
    }

    fn field(&mut self, f: &Field, indent: usize) {
        let pad = " ".repeat(indent);
        let label = format!("{}:", f.name);
        if f.list {
            let first = format!("{pad}{}", self.paint(&label, Style::Dim));
            self.out.push_str(&first);
            self.out.push('\n');
            for line in &f.values {
                let lead = format!("{pad}  - ");
                self.wrap(&lead, indent + 4, indent + 4, &Self::line_words(line));
            }
        } else {
            let mut ws = Vec::new();
            for (n, line) in f.values.iter().enumerate() {
                let mut lw = Self::line_words(line);
                if n + 1 < f.values.len() {
                    if let Some(last) = lw.last_mut() {
                        last.text.push(',');
                    }
                }
                ws.extend(lw);
            }
            let first = format!("{pad}{} ", self.paint(&label, Style::Dim));
            self.wrap(&first, indent + len(&label) + 1, indent + 2, &ws);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_with_a_hanging_indent() {
        let mut t = Term {
            out: String::new(),
            color: false,
            width: 20,
        };
        let ws = Term::split("one two three four five six", Style::Plain);
        t.wrap("  FR1  ", 7, 7, &ws);
        assert_eq!(t.out, "  FR1  one two three\n       four five six\n");
    }
}
