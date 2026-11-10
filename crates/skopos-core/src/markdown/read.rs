//! The generous Markdown reader (specification 10.5 and 10.6).

use std::collections::{BTreeMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Map, Value};

use super::{
    enum_values, evidence_main, is_rel, kind, Error, Fill, Kind, Warning, ENVELOPE, OTHER,
    SCHEMA_KEY,
};
use crate::model::ItemType;
use crate::{whole_number, SCHEMA_URL};

/// Reads a Markdown file into a document.
///
/// `filename` is the name of the file, if there is one. If the front matter
/// has no `kind`, the reader takes it from a file name in capital letters:
/// `INTENT.md` gives `intent`.
///
/// The result is the document and the warnings, or the errors (10.6). Each
/// warning and error names its line where it can. The document is valid
/// against the schema and the rules of section 9: the reader reports their
/// problems as errors.
///
/// If the file has no `id` or `created_at`, the reader gives a new random
/// artifact ID and the current time. Use [`read_with`] to give them.
///
/// This function is not in the WebAssembly build. Use [`read_with`] there.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn read(md: &str, filename: Option<&str>) -> Result<(Value, Vec<Warning>), Vec<Error>> {
    read_with(md, filename, &Fill::new())
}

/// [`read`], with the artifact ID and the time to use if the file has none.
pub fn read_with(
    md: &str,
    filename: Option<&str>,
    fill: &Fill,
) -> Result<(Value, Vec<Warning>), Vec<Error>> {
    let mut r = Reader::default();
    let doc = r.read(md, filename, fill);
    if !r.errors.is_empty() {
        return Err(r.errors);
    }
    let problems = crate::check(&doc);
    if !problems.is_empty() {
        return Err(problems.into_iter().map(|p| r.problem_error(p)).collect());
    }
    Ok((doc, r.warnings))
}

// ------------------------------------------------------------------ text

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("a valid pattern")
}

static ITEM: LazyLock<Regex> = LazyLock::new(|| re(r"^ ?(?:[-*+]|\d+[.)])\s+(.*)$"));
static ID: LazyLock<Regex> = LazyLock::new(|| {
    re(r"^(?:\*\*|__)?([A-Za-z]{1,2})(\d+)(?:\*\*|__)?\s*(?::|\.|\)|\s[-–])\s*(.*)$")
});
static BRACKET_ID: LazyLock<Regex> =
    LazyLock::new(|| re(r"^\[([A-Za-z]{1,2})(\d+)\]\s*(?::|\.|\)|[-–])?\s*(.*)$"));
static NEW: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)^new\s*:\s*(.*)$"));
static FIELD_LINE: LazyLock<Regex> = LazyLock::new(|| re(r"^\s+[-*+]\s"));
static FIELD: LazyLock<Regex> = LazyLock::new(|| {
    re(r"^(\s+)[-*+]\s+(?:\*\*)?([A-Za-z][\w .\-]*?)(?:\*\*)?\s*:(?:\s+(.*)|\s*)$")
});
static VALUE: LazyLock<Regex> = LazyLock::new(|| re(r"^(\s+)[-*+]\s+(.*)$"));
/// An ATX heading. As in CommonMark, a closing run of `#` is taken off only
/// after a space, so `# Support for C#` keeps its `#`.
static HEADING: LazyLock<Regex> = LazyLock::new(|| re(r"^(#{1,6})\s+(.*?)(?:\s+#+)?\s*$"));
static REF: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"^(?:(pa_[0-9A-Za-z]{16,32})(?:@(\d+))?(?:#([A-Za-z]{1,2}[1-9]\d*))?|([A-Za-z]{1,2}[1-9]\d*))$",
    )
});
static CODE_LINES_IN_PATH: LazyLock<Regex> = LazyLock::new(|| re(r"^(.*?):(\d+)(?:-(\d+))?$"));
static LINES: LazyLock<Regex> = LazyLock::new(|| re(r"^(\d+)(?:\s*-\s*(\d+))?$"));
static VOTES: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)(\d+)\s*(yes|no|eligible)"));
static EMPHASIS: LazyLock<Regex> = LazyLock::new(|| re(r"[*_`]+"));
static PARENS: LazyLock<Regex> = LazyLock::new(|| re(r"\(.*?\)"));

/// A name with case, `-`, `_`, emphasis and words in parentheses taken out:
/// `**Success Signals** (S)` gives `success signals`.
fn norm(s: &str) -> String {
    let s = EMPHASIS.replace_all(s, " ");
    let s = PARENS.replace_all(&s, " ");
    let s = s.replace('-', " ").to_lowercase();
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A key as a field name: `Enforced by` gives `enforced_by`.
fn key_of(raw: &str) -> String {
    norm(raw).replace(' ', "_")
}

/// The other names of each section (10.5).
fn section_names(t: ItemType) -> &'static [&'static str] {
    match t {
        ItemType::SuccessSignal => &["success criteria"],
        ItemType::Requirement => &["functional requirements"],
        ItemType::Story => &["user stories", "story", "user story"],
        ItemType::Constraint => &["rules", "boundaries"],
        ItemType::NonGoal => &["out of scope"],
        ItemType::OpenQuestion => &["questions"],
        ItemType::Term => &["glossary", "glossaries"],
        ItemType::Critique => &["findings"],
        _ => &[],
    }
}

/// The item type of an H2, or `None` for an unknown section.
fn heading_type(text: &str) -> Option<ItemType> {
    let n = norm(text);
    ItemType::ALL.into_iter().find(|t| {
        let as_str = t.as_str().replace('_', " ");
        let mut names = vec![t.section(), as_str.as_str()];
        names.extend(section_names(*t));
        names.into_iter().any(|name| {
            let a = norm(name);
            // The singular or the plural.
            let other = match a.strip_suffix('s') {
                Some(s) => s.to_string(),
                None => format!("{a}s"),
            };
            n == a || n == other
        })
    })
}

/// The other keys that the reader accepts for a field (10.5).
const KEY_ALIASES: &[(&str, &[&str])] = &[
    (
        "acceptance_criteria",
        &[
            "ac",
            "acceptance_criterion",
            "acceptance_criteria",
            "criterion",
        ],
    ),
    ("options", &["option", "options"]),
    ("alternatives", &["alternative", "alternatives"]),
    ("consequences", &["consequence", "consequences"]),
    ("aliases", &["alias", "aliases", "also_called"]),
    ("code_names", &["code_name", "code_names", "in_code"]),
    ("confirm_by", &["confirm_by", "due"]),
    ("proposed_by", &["proposed_by", "by"]),
    ("ratification", &["ratification", "votes", "vote"]),
];

/// The field name for a key, if the item can have that field.
fn canon_key(k: &str, allowed: &[&str]) -> Option<String> {
    for (field, aliases) in KEY_ALIASES {
        if aliases.contains(&k) && allowed.contains(field) {
            return Some((*field).to_string());
        }
    }
    if allowed.contains(&k) {
        return Some(k.to_string());
    }
    if let Some(s) = k.strip_suffix('s') {
        if allowed.contains(&s) {
            return Some(s.to_string());
        }
    }
    let plural = format!("{k}s");
    allowed.contains(&plural.as_str()).then_some(plural)
}

/// A reference: an item ID such as `FR1`, or a reference string such as
/// `pa_7Hk2mQ9xR4tV8wZ1@2#FR1`. Item IDs become capital letters.
pub(super) fn parse_ref(raw: &str) -> Option<Map<String, Value>> {
    let c = REF.captures(raw.trim())?;
    let mut o = Map::new();
    if let Some(a) = c.get(1) {
        o.insert("artifact".into(), a.as_str().into());
        if let Some(v) = c.get(2) {
            o.insert("version".into(), number(v.as_str())?);
        }
    }
    if let Some(i) = c.get(3).or_else(|| c.get(4)) {
        o.insert("item".into(), i.as_str().to_uppercase().into());
    }
    Some(o)
}

fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn number(s: &str) -> Option<Value> {
    s.parse::<u64>().ok().map(Value::from)
}

// ------------------------------------------------------------------ fields

/// A nested list item under an item: `- key: value`, or `- value`.
struct Node {
    indent: usize,
    key: Option<String>,
    value: String,
    /// The text after the dash, also for a line with a key.
    raw: String,
    line: usize,
    children: Vec<Node>,
}

/// Turns the field lines of an item into a tree, by indent.
fn tree(lines: &[(usize, String)]) -> Vec<Node> {
    let flat: Vec<Node> = lines
        .iter()
        .map(|(line, text)| {
            let text = text.replace('\t', "    ");
            let v = VALUE.captures(&text).expect("a field line is a list item");
            let raw = v[2].trim().to_string();
            match FIELD.captures(&text) {
                Some(f) => Node {
                    indent: f[1].len(),
                    key: Some(f[2].to_string()),
                    value: f.get(3).map_or("", |m| m.as_str()).trim().to_string(),
                    raw,
                    line: *line,
                    children: vec![],
                },
                None => Node {
                    indent: v[1].len(),
                    key: None,
                    value: raw.clone(),
                    raw,
                    line: *line,
                    children: vec![],
                },
            }
        })
        .collect();
    let mut iter = flat.into_iter().peekable();
    build(&mut iter, None)
}

fn build(
    iter: &mut std::iter::Peekable<impl Iterator<Item = Node>>,
    parent: Option<usize>,
) -> Vec<Node> {
    let mut out = vec![];
    while let Some(next) = iter.peek() {
        if parent.is_some_and(|p| next.indent <= p) {
            break;
        }
        let mut node = iter.next().expect("peeked");
        node.children = build(iter, Some(node.indent));
        out.push(node);
    }
    out
}

impl Node {
    fn key(&self) -> String {
        key_of(self.key.as_deref().unwrap_or_default())
    }

    /// The values of a list: the value of this line, or the nested lines.
    fn values(&self) -> Vec<String> {
        if self.value.is_empty() {
            self.children.iter().map(|c| c.raw.clone()).collect()
        } else {
            vec![self.value.clone()]
        }
    }

    fn shown_key(&self) -> &str {
        self.key.as_deref().unwrap_or(&self.raw)
    }
}

// ------------------------------------------------------------------ reader

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    /// Before the first H2.
    Start,
    /// An unknown H2: its lines are ignored.
    Unknown,
    Type(ItemType),
    /// `## From other artifacts`: ignored (10.7).
    Other,
}

struct Pending {
    ty: ItemType,
    given: Option<(String, u64)>,
    item: Map<String, Value>,
    line: usize,
}

#[derive(Default)]
struct Reader {
    errors: Vec<Error>,
    warnings: Vec<Warning>,
    /// The line of each top-level key in the front matter.
    fm_lines: BTreeMap<String, usize>,
    title_line: Option<usize>,
    /// The line of each item, in the order of `items`.
    item_lines: Vec<usize>,
}

impl Reader {
    fn error(&mut self, line: usize, message: impl Into<String>) {
        self.errors.push(Error::at(line, message));
    }

    fn warn(&mut self, line: usize, message: impl Into<String>) {
        self.warnings.push(Warning {
            line,
            message: message.into(),
        });
    }

    fn read(&mut self, md: &str, filename: Option<&str>, fill: &Fill) -> Value {
        let md = md
            .strip_prefix('\u{feff}')
            .unwrap_or(md)
            .replace("\r\n", "\n");
        let lines: Vec<&str> = md.split('\n').collect();
        let (mut doc, body_start) = self.front_matter(&lines);

        let mut title: Option<String> = None;
        let mut summary: Vec<&str> = vec![];
        let mut items: Vec<Pending> = vec![];
        let mut section = Section::Start;
        // The marker of the open code fence, for example three backticks.
        let mut fence: Option<(char, usize)> = None;
        // The item that is open, its field lines and its detail lines.
        let mut cur: Option<Pending> = None;
        let mut fields: Vec<(usize, String)> = vec![];
        let mut detail: Option<Vec<String>> = None;

        for (i, &line) in lines.iter().enumerate().skip(body_start) {
            let no = i + 1;
            let start = line.trim_start();
            let marker = fence_marker(start);
            match fence {
                Some((ch, len)) => {
                    let closes = marker.is_some_and(|(c, n)| c == ch && n >= len)
                        && start.trim_start_matches(ch).trim().is_empty();
                    // A writer indents each line of a detail, so an H1, an H2
                    // or an item at column 0 is never in a fence. It ends a
                    // fence that has no closing line.
                    let top = !line.starts_with([' ', '\t'])
                        && (ITEM.is_match(line)
                            || HEADING.captures(line).is_some_and(|h| h[1].len() <= 2));
                    if closes || top {
                        fence = None;
                    }
                }
                None => fence = marker,
            }
            let in_fence = fence.is_some();
            let heading = if in_fence {
                None
            } else {
                HEADING.captures(line)
            };
            if let Some(h) = heading {
                let level = h[1].len();
                let text = h[2].to_string();
                if detail.is_none() || level <= 2 {
                    if level == 1 && title.is_none() && section == Section::Start {
                        title = Some(text);
                        self.title_line = Some(no);
                        continue;
                    }
                    if level == 2 {
                        self.close(&mut cur, &mut fields, &mut detail, &mut items);
                        section = if norm(&text) == norm(OTHER) {
                            Section::Other
                        } else if let Some(t) = heading_type(&text) {
                            Section::Type(t)
                        } else {
                            self.warn(
                                no,
                                format!("unknown section '{text}': its lines are ignored"),
                            );
                            Section::Unknown
                        };
                        continue;
                    }
                    if level >= 3 && matches!(section, Section::Type(_) | Section::Unknown) {
                        // H3 and smaller headings only group items.
                        self.close(&mut cur, &mut fields, &mut detail, &mut items);
                        continue;
                    }
                }
            }
            let ty = match section {
                Section::Other | Section::Unknown => continue,
                Section::Start => {
                    if title.is_some() {
                        summary.push(line);
                    }
                    continue;
                }
                Section::Type(t) => t,
            };
            if !line.starts_with("  ") && !in_fence {
                if let Some(m) = ITEM.captures(line) {
                    self.close(&mut cur, &mut fields, &mut detail, &mut items);
                    cur = Some(new_item(ty, m[1].trim(), no));
                    continue;
                }
            }
            let Some(item) = cur.as_mut() else {
                if !line.trim().is_empty() {
                    self.warn(no, "text that is not in an item is ignored");
                }
                continue;
            };
            if let Some(d) = detail.as_mut() {
                d.push(dedent(line));
                continue;
            }
            if line.trim().is_empty() {
                // A blank line starts the detail.
                detail = Some(vec![]);
            } else if FIELD_LINE.is_match(line) {
                fields.push((no, line.to_string()));
            } else if line.starts_with(' ') {
                match fields.last_mut() {
                    // A line that continues the value of the last field.
                    Some((_, last)) => {
                        last.push(' ');
                        last.push_str(line.trim());
                    }
                    // A line that continues the text of the item.
                    None => {
                        let text = item.item.get("text").and_then(Value::as_str).unwrap_or("");
                        let joined = format!("{text} {}", line.trim()).trim().to_string();
                        item.item.insert("text".into(), joined.into());
                    }
                }
            } else {
                detail = Some(vec![dedent(line)]);
            }
        }
        self.close(&mut cur, &mut fields, &mut detail, &mut items);

        // Title and summary.
        let fm_title = doc.remove("title");
        let title = match title {
            Some(t) => Some(Value::String(t)),
            None => fm_title,
        };
        match title {
            Some(t) => {
                doc.insert("title".into(), t);
            }
            None => self.errors.push(Error::file(
                "the file has no title: add a line '# <title>' after the front matter",
            )),
        }
        let summary = summary.join("\n");
        if !summary.trim().is_empty() {
            doc.insert("summary".into(), summary.trim().into());
        }

        // The kind from the file name.
        if !doc.contains_key("kind") {
            if let Some(kind) = filename.and_then(kind_from_filename) {
                doc.insert("kind".into(), kind.into());
            }
        }
        if !doc.contains_key("kind") {
            self.errors.push(Error::file(
                "the file has no kind: add 'kind: intent' (or another kind) to the front matter, \
                 or name the file after its kind, for example INTENT.md",
            ));
        }

        // Bookkeeping fields.
        doc.entry(SCHEMA_KEY).or_insert_with(|| SCHEMA_URL.into());
        doc.entry("id")
            .or_insert_with(|| fill.artifact_id.clone().into());
        doc.entry("version").or_insert_with(|| 1.into());
        doc.entry("status").or_insert_with(|| "draft".into());
        doc.entry("created_at")
            .or_insert_with(|| fill.now.clone().into());

        self.assign_ids(&mut doc, &mut items);
        self.item_lines = items.iter().map(|p| p.line).collect();
        doc.insert(
            "items".into(),
            Value::Array(items.into_iter().map(|p| Value::Object(p.item)).collect()),
        );
        Value::Object(doc)
    }

    /// Reads the front matter. Returns the envelope fields and the index of
    /// the first line of the body.
    fn front_matter(&mut self, lines: &[&str]) -> (Map<String, Value>, usize) {
        let mut doc = Map::new();
        if lines.first().map(|l| l.trim()) != Some("---") {
            return (doc, 0);
        }
        let Some(end) = (1..lines.len()).find(|&i| matches!(lines[i].trim(), "---" | "...")) else {
            self.error(
                1,
                "the front matter has no closing ---: add a line --- after it",
            );
            return (doc, lines.len());
        };
        let yaml = lines[1..end].join("\n");
        for (i, line) in lines[1..end].iter().enumerate() {
            if let Some((key, _)) = line.split_once(':') {
                if !key.is_empty() && !key.starts_with([' ', '-', '#']) {
                    let key = key.trim_matches(['"', '\'']);
                    let key = if key == "schema" { SCHEMA_KEY } else { key };
                    self.fm_lines.entry(key.to_string()).or_insert(i + 2);
                }
            }
        }
        let fm: Value = if yaml.trim().is_empty() {
            Value::Object(Map::new())
        } else {
            match serde_yaml_ng::from_str(&yaml) {
                Ok(v) => v,
                Err(e) => {
                    // The YAML starts on line 2. An error at the end of the
                    // YAML gets the line of the last YAML line.
                    let line = e.location().map_or(2, |l| (l.line() + 1).min(end.max(2)));
                    let what = e.to_string();
                    let what = what.split(" at line ").next().unwrap_or(&what);
                    self.error(line, format!("the front matter is not valid YAML: {what}"));
                    return (doc, end + 1);
                }
            }
        };
        let Value::Object(fm) = fm else {
            self.error(
                2,
                "the front matter is not a map of keys and values, such as 'kind: intent'",
            );
            return (doc, end + 1);
        };
        for (k, v) in fm {
            let key = if k == "schema" {
                SCHEMA_KEY.to_string()
            } else {
                k.clone()
            };
            if ENVELOPE.contains(&key.as_str()) || key.starts_with("x-") {
                doc.insert(key, v);
            } else {
                let line = self.fm_lines.get(&k).copied().unwrap_or(1);
                self.warn(
                    line,
                    format!("unknown key '{k}' in the front matter: kept as x-{k}"),
                );
                doc.insert(format!("x-{k}"), v);
            }
        }
        coerce(&mut doc);
        (doc, end + 1)
    }

    fn close(
        &mut self,
        cur: &mut Option<Pending>,
        fields: &mut Vec<(usize, String)>,
        detail: &mut Option<Vec<String>>,
        items: &mut Vec<Pending>,
    ) {
        let lines = std::mem::take(fields);
        let detail = detail.take();
        let Some(mut p) = cur.take() else { return };
        self.apply_fields(&mut p.item, p.ty, &tree(&lines));
        if let Some(d) = detail {
            let d = d.join("\n");
            let d = d.trim_matches('\n');
            if !d.trim().is_empty() {
                p.item.insert("detail".into(), d.into());
            }
        }
        items.push(p);
    }

    /// Item IDs (10.5): written IDs first, then new IDs from the counters.
    fn assign_ids(&mut self, doc: &mut Map<String, Value>, items: &mut [Pending]) {
        let mut counters = match doc.get("id_counters") {
            None => Map::new(),
            Some(Value::Object(m)) => m.clone(),
            // The schema reports a value that is not a map.
            Some(_) => return,
        };
        let mut used: HashSet<(String, u64)> = HashSet::new();
        for p in items.iter_mut() {
            let Some((prefix, n)) = p.given.clone() else {
                continue;
            };
            let label = format!("{prefix}{n}");
            if prefix != p.ty.prefix() {
                let what = p.ty.as_str().replace('_', " ");
                self.warn(
                    p.line,
                    format!("{label} is not a {what} ID, so the item gets a new ID"),
                );
                continue;
            }
            if !used.insert((prefix.clone(), n)) {
                self.warn(
                    p.line,
                    format!("{label} appears twice, so the second item gets a new ID"),
                );
                continue;
            }
            p.item.insert("id".into(), label.into());
            let c = counters.get(&prefix).and_then(whole_number).unwrap_or(0);
            counters.insert(prefix, c.max(n).into());
        }
        for p in items.iter_mut() {
            if p.item.contains_key("id") {
                continue;
            }
            let prefix = p.ty.prefix();
            let n = counters.get(prefix).and_then(whole_number).unwrap_or(0) + 1;
            counters.insert(prefix.into(), n.into());
            p.item.insert("id".into(), format!("{prefix}{n}").into());
        }
        if !items.is_empty() || doc.contains_key("id_counters") {
            doc.insert("id_counters".into(), Value::Object(counters));
        }
    }

    fn apply_fields(&mut self, it: &mut Map<String, Value>, ty: ItemType, nodes: &[Node]) {
        let mut allowed: Vec<&str> = ty.fields().to_vec();
        allowed.extend(["proposed_by", "ratification", "evidence", "detail"]);
        // The fields with one value that this item has.
        let mut seen = HashSet::new();
        for n in nodes {
            let Some(raw) = n.key.as_deref() else {
                self.warn(n.line, format!("'{}' has no key and is ignored", n.value));
                continue;
            };
            if raw.to_ascii_lowercase().starts_with("x-") {
                let v = serde_json::from_str(&n.value)
                    .unwrap_or_else(|_| Value::String(n.value.clone()));
                it.insert(raw.to_string(), v);
                continue;
            }
            let k = key_of(raw);
            if k == "status" {
                if let Some(v) = self.enum_value("status", &n.value, n.line) {
                    self.set_once(it, &mut seen, "status", v.into(), n.line);
                }
                continue;
            }
            if allowed.contains(&"examples")
                && matches!(
                    k.as_str(),
                    "example" | "normal" | "edge" | "forbidden" | "examples"
                )
            {
                self.examples(it, &k, n);
                continue;
            }
            let rel = if is_rel(&k) {
                Some(k.clone())
            } else if k == "related_to" {
                Some("relates_to".to_string())
            } else {
                None
            };
            if rel.is_some() || k == "ref" || k == "refs" {
                for v in n.values() {
                    match parse_ref(&v).filter(|r| r.contains_key("item")) {
                        Some(mut r) => {
                            if let Some(rel) = &rel {
                                r.insert("rel".into(), rel.clone().into());
                            }
                            push(it, "refs", Value::Object(r));
                        }
                        None => self.error(
                            n.line,
                            format!(
                                "'{v}' is not an item ID or a reference string: \
                                 write an ID such as FR1, or pa_7Hk2mQ9xR4tV8wZ1@2#FR1"
                            ),
                        ),
                    }
                }
                continue;
            }
            let Some(f) = canon_key(&k, &allowed) else {
                self.warn(n.line, format!("unknown field '{raw}': kept as x-{k}"));
                it.insert(format!("x-{k}"), n.value.clone().into());
                continue;
            };
            match f.as_str() {
                "detail" | "proposed_by" => {
                    self.set_once(it, &mut seen, &f, n.value.clone().into(), n.line)
                }
                "ratification" => {
                    if let Some(r) = self.votes(n) {
                        self.set_once(it, &mut seen, &f, r, n.line);
                    }
                }
                "evidence" => {
                    if !n.value.is_empty() {
                        self.error(
                            n.line,
                            "put each piece of evidence on a nested line under '- evidence:', \
                             for example '- quote: <text>'",
                        );
                    }
                    for c in &n.children {
                        if let Some(ev) = self.evidence(c) {
                            push(it, "evidence", ev);
                        }
                    }
                }
                _ => match kind(&f) {
                    Kind::Text => self.set_once(it, &mut seen, &f, n.value.clone().into(), n.line),
                    Kind::Enum => {
                        if let Some(v) = self.enum_value(&f, &n.value, n.line) {
                            self.set_once(it, &mut seen, &f, v.into(), n.line);
                        }
                    }
                    Kind::Bool => {
                        if let Some(v) = self.bool_value(&f, &n.value, n.line) {
                            self.set_once(it, &mut seen, &f, v.into(), n.line);
                        }
                    }
                    Kind::List => {
                        for v in n.values().into_iter().filter(|v| !v.is_empty()) {
                            push(it, &f, v.into());
                        }
                    }
                    Kind::Examples => self.examples(it, &k, n),
                    Kind::Target => {
                        let t = self.target(n);
                        self.set_once(it, &mut seen, &f, t, n.line);
                    }
                    Kind::Resolution => {
                        if let Some(r) = self.resolution(n) {
                            self.set_once(it, &mut seen, &f, r, n.line);
                        }
                    }
                },
            }
        }
    }

    /// Sets a field with one value, or reports that it appears twice.
    fn set_once(
        &mut self,
        it: &mut Map<String, Value>,
        seen: &mut HashSet<String>,
        f: &str,
        v: Value,
        line: usize,
    ) {
        if !seen.insert(f.to_string()) {
            self.error(
                line,
                format!("{f} appears twice in this item: keep one {f} line"),
            );
            return;
        }
        it.insert(f.to_string(), v);
    }

    fn enum_value(&mut self, field: &str, raw: &str, line: usize) -> Option<String> {
        let v = norm(raw).replace(' ', "_");
        let values = enum_values(field);
        if values.contains(&v) {
            return Some(v);
        }
        self.error(
            line,
            format!("{field} '{raw}' is not one of: {}", values.join(", ")),
        );
        None
    }

    fn bool_value(&mut self, field: &str, raw: &str, line: usize) -> Option<bool> {
        match raw.trim().to_lowercase().as_str() {
            "true" | "yes" | "y" => Some(true),
            "false" | "no" | "n" => Some(false),
            _ => {
                self.error(line, format!("{field} '{raw}' is not true or false"));
                None
            }
        }
    }

    fn examples(&mut self, it: &mut Map<String, Value>, k: &str, n: &Node) {
        let pairs: Vec<(String, String)> = if !n.value.is_empty() {
            vec![(k.to_string(), n.value.clone())]
        } else {
            n.children
                .iter()
                .map(|c| match c.key.as_ref().map(|_| c.key()) {
                    None => ("example".to_string(), c.raw.clone()),
                    Some(ck)
                        if matches!(ck.as_str(), "example" | "normal" | "edge" | "forbidden") =>
                    {
                        (ck, c.value.clone())
                    }
                    Some(_) => ("example".to_string(), c.raw.clone()),
                })
                .collect()
        };
        for (ek, text) in pairs {
            let mut ex = Map::new();
            ex.insert("text".into(), text.into());
            if matches!(ek.as_str(), "normal" | "edge" | "forbidden") {
                ex.insert("kind".into(), ek.into());
            }
            push(it, "examples", Value::Object(ex));
        }
    }

    /// A whole number from digits, or an error if it is too large.
    fn int(&mut self, digits: &str, line: usize) -> Option<Value> {
        let n = number(digits);
        if n.is_none() {
            self.error(line, format!("{digits} is too large for a number"));
        }
        n
    }

    fn votes(&mut self, n: &Node) -> Option<Value> {
        let errors = self.errors.len();
        let mut r = Map::new();
        for c in VOTES.captures_iter(&n.value) {
            if let Some(v) = self.int(&c[1], n.line) {
                r.insert(c[2].to_lowercase(), v);
            }
        }
        for c in &n.children {
            let k = c.key();
            if matches!(k.as_str(), "yes" | "no" | "eligible") && is_digits(c.value.trim()) {
                if let Some(v) = self.int(c.value.trim(), c.line) {
                    r.insert(k, v);
                }
            }
        }
        if ["yes", "no", "eligible"].iter().all(|k| r.contains_key(*k)) {
            return Some(Value::Object(r));
        }
        if self.errors.len() > errors {
            return None;
        }
        self.error(
            n.line,
            "votes need yes, no and eligible, for example '- votes: 2 yes, 0 no, 2 eligible'",
        );
        None
    }

    fn target(&mut self, n: &Node) -> Value {
        let mut t = Map::new();
        if !n.value.is_empty() {
            match parse_ref(&n.value) {
                Some(r) => t.extend(r),
                None => {
                    t.insert("section".into(), n.value.clone().into());
                }
            }
        }
        for c in &n.children {
            match c.key().as_str() {
                "section" => {
                    t.insert("section".into(), c.value.clone().into());
                }
                "item" => match parse_ref(&c.value) {
                    Some(r) => t.extend(r),
                    None => {
                        t.insert("item".into(), c.value.clone().into());
                    }
                },
                _ => self.error(
                    c.line,
                    format!(
                        "a target has no field '{}': use section or item",
                        c.shown_key()
                    ),
                ),
            }
        }
        Value::Object(t)
    }

    fn resolution(&mut self, n: &Node) -> Option<Value> {
        let mut r = Map::new();
        if !n.value.is_empty() {
            if let Some(v) = self.enum_value("outcome", &n.value, n.line) {
                r.insert("outcome".into(), v.into());
            }
        }
        for c in &n.children {
            match c.key().as_str() {
                "outcome" => {
                    if let Some(v) = self.enum_value("outcome", &c.value, c.line) {
                        r.insert("outcome".into(), v.into());
                    }
                }
                k @ ("note" | "decided_by") => {
                    r.insert(k.into(), c.value.clone().into());
                }
                _ => self.error(
                    c.line,
                    format!(
                        "a resolution has no field '{}': use note or decided_by",
                        c.shown_key()
                    ),
                ),
            }
        }
        if !r.contains_key("outcome") {
            self.error(
                n.line,
                "a resolution needs an outcome, for example '- resolution: fix_spec'",
            );
            return None;
        }
        Some(Value::Object(r))
    }

    fn evidence(&mut self, n: &Node) -> Option<Value> {
        let k = match n.key().as_str() {
            "said" => "discussion".to_string(),
            "url" => "link".to_string(),
            other => other.to_string(),
        };
        let Some(main) = evidence_main(&k) else {
            self.error(
                n.line,
                format!(
                    "evidence is quote, code, discussion or link, not '{}'",
                    n.shown_key()
                ),
            );
            return None;
        };
        let mut ev = Map::new();
        ev.insert("kind".into(), k.clone().into());
        // `path:41-48` gives the lines in the path, unless a nested line gives
        // them. A writer puts the lines on a nested line, so a path that ends
        // in `:<number>` stays as it is.
        let nested_lines = n.children.iter().any(|c| {
            matches!(
                c.key().as_str(),
                "lines" | "line" | "line_start" | "line_end"
            )
        });
        let in_path = CODE_LINES_IN_PATH
            .captures(&n.value)
            .filter(|_| k == "code" && !nested_lines);
        match in_path {
            Some(c) => {
                ev.insert("path".into(), c[1].into());
                if let Some(v) = self.int(&c[2], n.line) {
                    ev.insert("line_start".into(), v);
                }
                if let Some(e) = c.get(3) {
                    if let Some(v) = self.int(e.as_str(), n.line) {
                        ev.insert("line_end".into(), v);
                    }
                }
            }
            None => {
                ev.insert(main.into(), n.value.clone().into());
            }
        }
        for c in &n.children {
            let ck = c.key();
            match (k.as_str(), ck.as_str()) {
                ("code", "lines" | "line") => match LINES.captures(c.value.trim()) {
                    Some(m) => {
                        if let Some(v) = self.int(&m[1], c.line) {
                            ev.insert("line_start".into(), v);
                        }
                        if let Some(e) = m.get(2) {
                            if let Some(v) = self.int(e.as_str(), c.line) {
                                ev.insert("line_end".into(), v);
                            }
                        }
                    }
                    None => self.error(
                        c.line,
                        "lines are a number or a range, for example '- lines: 41-48'",
                    ),
                },
                ("code", "line_start" | "line_end") => {
                    if !is_digits(c.value.trim()) {
                        self.error(c.line, format!("{ck} is a number, for example 41"));
                    } else if let Some(v) = self.int(c.value.trim(), c.line) {
                        ev.insert(ck, v);
                    }
                }
                ("code", "repo" | "commit" | "snippet")
                | ("quote", "location")
                | ("discussion", "speaker")
                | ("link", "title") => {
                    ev.insert(ck, c.value.clone().into());
                }
                ("quote", "from") => {
                    let mut from = parse_ref(&c.value).unwrap_or_else(|| {
                        let mut m = Map::new();
                        m.insert("title".into(), c.value.clone().into());
                        m
                    });
                    for cc in &c.children {
                        if cc.key() == "title" {
                            from.insert("title".into(), cc.value.clone().into());
                        } else {
                            self.error(
                                cc.line,
                                format!(
                                    "the from of a quote has no field '{}': use title",
                                    cc.shown_key()
                                ),
                            );
                        }
                    }
                    ev.insert("from".into(), Value::Object(from));
                }
                _ => self.error(
                    c.line,
                    format!("{k} evidence has no field '{}'", c.shown_key()),
                ),
            }
        }
        Some(Value::Object(ev))
    }

    /// An error for a problem that [`crate::check`] found, at the line of its
    /// item or its front matter key.
    fn problem_error(&self, p: crate::Problem) -> Error {
        let mut parts = p.path.split('/').skip(1);
        let line = match parts.next().unwrap_or("") {
            "items" => parts
                .next()
                .and_then(|s| s.parse::<usize>().ok())
                .and_then(|i| self.item_lines.get(i).copied()),
            "title" => self.title_line,
            key => {
                let key = key.replace("~1", "/").replace("~0", "~");
                self.fm_lines.get(&key).copied()
            }
        };
        Error {
            line,
            message: p.message.clone(),
            problem: Some(p),
        }
    }
}

fn new_item(ty: ItemType, content: &str, line: usize) -> Pending {
    let mut given = None;
    let mut text = content.to_string();
    let id = ID
        .captures(content)
        .or_else(|| BRACKET_ID.captures(content));
    let known = id.as_ref().and_then(|c| {
        let prefix = c[1].to_uppercase();
        let n = c[2].parse::<u64>().ok()?;
        ItemType::from_prefix(&prefix).map(|_| (prefix, n, c[3].trim().to_string()))
    });
    if let Some((prefix, n, rest)) = known {
        given = Some((prefix, n));
        text = rest;
    } else if let Some(c) = NEW.captures(content) {
        text = c[1].trim().to_string();
    }
    let mut item = Map::new();
    item.insert("type".into(), ty.as_str().into());
    item.insert("text".into(), text.into());
    item.insert("status".into(), "accepted".into());
    Pending {
        ty,
        given,
        item,
        line,
    }
}

/// The marker of a code fence: three or more backticks or tildes.
fn fence_marker(start: &str) -> Option<(char, usize)> {
    let ch = start.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let len = start.chars().take_while(|c| *c == ch).count();
    (len >= 3).then_some((ch, len))
}

/// A detail line with the indent of two spaces taken off.
fn dedent(line: &str) -> String {
    match line.strip_prefix("  ") {
        Some(rest) => rest.to_string(),
        None => line.trim_start().to_string(),
    }
}

fn push(it: &mut Map<String, Value>, key: &str, v: Value) {
    match it.get_mut(key) {
        Some(Value::Array(list)) => list.push(v),
        _ => {
            it.insert(key.into(), Value::Array(vec![v]));
        }
    }
}

/// `INTENT.md` gives `intent`. A name that is not all capital letters gives
/// nothing.
fn kind_from_filename(name: &str) -> Option<String> {
    let base = name.rsplit(['/', '\\']).next()?;
    let base = base.split('.').next()?;
    let ok = !base.is_empty()
        && base.chars().any(|c| c.is_ascii_uppercase())
        && base
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
    ok.then(|| base.to_lowercase())
}

/// Changes a YAML value with the wrong type, where the change is exact
/// (10.5): a number where the schema wants text becomes text, and text with a
/// whole number where the schema wants a number becomes a number.
fn coerce(doc: &mut Map<String, Value>) {
    fn to_text(v: &mut Value) {
        if let Value::Number(n) = v {
            *v = Value::String(n.to_string());
        }
    }
    fn to_number(v: &mut Value) {
        if let Some(n) = v.as_str().and_then(number) {
            *v = n;
        }
    }
    for key in ["method", "session"] {
        if let Some(v) = doc.get_mut(key) {
            if v.is_string() || v.is_number() {
                to_text(v);
                let mut m = Map::new();
                m.insert("id".into(), v.take());
                *v = Value::Object(m);
            }
            if let Some(m) = v.as_object_mut() {
                for sub in ["id", "version", "title"] {
                    if let Some(x) = m.get_mut(sub) {
                        to_text(x);
                    }
                }
            }
        }
    }
    for key in ["title", "kind", "decider", "summary"] {
        if let Some(v) = doc.get_mut(key) {
            to_text(v);
        }
    }
    if let Some(v) = doc.get_mut("version") {
        to_number(v);
    }
    if let Some(Value::Object(m)) = doc.get_mut("id_counters") {
        m.values_mut().for_each(to_number);
    }
    if let Some(Value::Array(list)) = doc.get_mut("participants") {
        for p in list.iter_mut().filter_map(Value::as_object_mut) {
            for sub in ["id", "name"] {
                if let Some(x) = p.get_mut(sub) {
                    to_text(x);
                }
            }
        }
    }
    if let Some(Value::Array(list)) = doc.get_mut("parents") {
        for p in list.iter_mut().filter_map(Value::as_object_mut) {
            if let Some(x) = p.get_mut("version") {
                to_number(x);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_names() {
        assert_eq!(
            heading_type("Success Signals (S)"),
            Some(ItemType::SuccessSignal)
        );
        assert_eq!(
            heading_type("success signal"),
            Some(ItemType::SuccessSignal)
        );
        assert_eq!(heading_type("open_question"), Some(ItemType::OpenQuestion));
        assert_eq!(heading_type("Out of scope"), Some(ItemType::NonGoal));
        assert_eq!(heading_type("User stories"), Some(ItemType::Story));
        assert_eq!(heading_type("Story"), Some(ItemType::Story));
        assert_eq!(heading_type("Non-goals"), Some(ItemType::NonGoal));
        assert_eq!(heading_type("Glossary"), Some(ItemType::Term));
        assert_eq!(heading_type("Notes"), None);
    }

    #[test]
    fn keys() {
        assert_eq!(key_of("Enforced by"), "enforced_by");
        assert_eq!(key_of("**Code-name**"), "code_name");
        let allowed = ["options", "owner", "blocking", "answer"];
        assert_eq!(canon_key("option", &allowed).as_deref(), Some("options"));
        assert_eq!(canon_key("owners", &allowed).as_deref(), Some("owner"));
        assert_eq!(canon_key("colour", &allowed), None);
    }

    #[test]
    fn refs() {
        assert_eq!(parse_ref("fr1").unwrap()["item"], "FR1");
        let r = parse_ref("pa_7Hk2mQ9xR4tV8wZ1@2#FR1").unwrap();
        assert_eq!(r["artifact"], "pa_7Hk2mQ9xR4tV8wZ1");
        assert_eq!(r["version"], 2);
        assert!(parse_ref("the plan picker").is_none());
        assert!(parse_ref("FR0").is_none());
    }

    #[test]
    fn kinds_from_file_names() {
        assert_eq!(kind_from_filename("INTENT.md").as_deref(), Some("intent"));
        assert_eq!(
            kind_from_filename("docs/AGENT_RULES.md").as_deref(),
            Some("agent_rules")
        );
        assert_eq!(kind_from_filename("notes.md"), None);
    }
}
