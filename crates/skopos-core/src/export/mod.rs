//! Export a document to the files of a spec-driven development tool.
//!
//! Teams can keep what they agreed in OpenSkopos and give it to the tool that
//! their agents already use. Each exporter writes the files of one tool:
//!
//! | [`Tool`] | Files |
//! |---|---|
//! | [`Tool::OpenSpec`] | `openspec/changes/<name>/`: `proposal.md`, `design.md` and `specs/<name>/spec.md` |
//! | [`Tool::Kiro`] | `.kiro/specs/<name>/`: `requirements.md` and `design.md` |
//! | [`Tool::SpecKit`] | `specs/<NNN>-<name>/`: `spec.md`, `plan.md` and `research.md` |
//!
//! `docs/exporters.md` in the repository explains where each item type goes.
//!
//! Three rules apply to every exporter:
//!
//! - Every exported item keeps its reference string, for example
//!   `pa_2Xc5Rv8Tn1Mb4Kz7@3#FR4` (specification section 6). A review can
//!   then trace a line of the tool's files back to the item.
//! - Only accepted items are exported. A proposed item is not agreed yet, and
//!   an agent must not act on a rejected item (specification 4.1).
//! - An item that is not exported is never dropped in silence. It is in
//!   [`Export::not_exported`], and the exporter writes a short "Not
//!   exported" note into the files.
//!
//! The output is deterministic: the same document always gives the same
//! bytes. No date or time of the export goes into the files.

use std::collections::HashMap;
use std::fmt::{self, Write as _};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::model::ItemType;

mod kiro;
mod openspec;
mod speckit;

/// A spec-driven development tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tool {
    OpenSpec,
    Kiro,
    SpecKit,
}

impl Tool {
    /// All tools.
    pub const ALL: [Tool; 3] = [Tool::OpenSpec, Tool::Kiro, Tool::SpecKit];

    /// The name on the command line: `openspec`, `kiro` or `speckit`.
    pub fn as_str(self) -> &'static str {
        match self {
            Tool::OpenSpec => "openspec",
            Tool::Kiro => "kiro",
            Tool::SpecKit => "speckit",
        }
    }

    /// The name of the tool for people, for example `Spec Kit`.
    pub fn display_name(self) -> &'static str {
        match self {
            Tool::OpenSpec => "OpenSpec",
            Tool::Kiro => "Kiro",
            Tool::SpecKit => "Spec Kit",
        }
    }
}

impl fmt::Display for Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The error of [`Tool::from_str`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownTool(pub String);

impl fmt::Display for UnknownTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown tool {:?}: use openspec, kiro or speckit",
            self.0
        )
    }
}

impl std::error::Error for UnknownTool {}

impl FromStr for Tool {
    type Err = UnknownTool;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Tool::ALL
            .into_iter()
            .find(|t| t.as_str() == s)
            .ok_or_else(|| UnknownTool(s.to_string()))
    }
}

/// Choices for an export.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// The name of the change, feature or capability, in kebab case. Without
    /// it, the exporter makes a name from the title of the document.
    pub name: Option<String>,
    /// Spec Kit only: the feature number, for example 3 for `003-<name>`.
    /// Without it, the number is 1. The `skopos` CLI uses the next free
    /// number in the `specs/` folder.
    pub number: Option<u32>,
}

/// One file that an exporter writes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct File {
    /// The path from the root of the project, with `/` between folders.
    pub path: String,
    pub text: String,
}

/// An item that the export leaves out, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotExported {
    /// The reference string of the item, for example
    /// `pa_2Xc5Rv8Tn1Mb4Kz7@3#CR1`.
    pub reference: String,
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub reason: String,
}

/// The result of an export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Export {
    pub tool: Tool,
    /// The folder that holds the files, from the root of the project.
    pub folder: String,
    /// The files, in a fixed order.
    pub files: Vec<File>,
    /// The items that are not in the files, in document order.
    pub not_exported: Vec<NotExported>,
    /// Every path that this exporter can write in `folder` for this name,
    /// also the paths that are not in `files` this time. A tool that writes
    /// the files again can remove the paths that are in `owned` but not in
    /// `files`: they are left over from an earlier export.
    pub owned: Vec<String>,
}

/// Exports a document to the files of a tool.
///
/// The function works on any JSON value and does not check it. Use
/// [`crate::check`] first: an invalid document can give incomplete files.
///
/// ```
/// use serde_json::json;
/// use skopos_core::export::{export, Options, Tool};
/// let doc = json!({
///     "id": "pa_7Hk2mQ9xR4tV8wZ1", "version": 2, "kind": "spec",
///     "title": "Seat limits", "status": "final",
///     "created_at": "2026-11-05T09:00:00Z",
///     "items": [
///         {"id": "FR1", "type": "requirement", "status": "accepted",
///          "text": "The system blocks a plan change above the seat limit."},
///         {"id": "CR1", "type": "critique", "status": "accepted",
///          "text": "The limit is not defined.", "severity": "major",
///          "category": "gap", "evidence": [{"kind": "discussion", "text": "?"}]}
///     ]
/// });
/// let out = export(&doc, Tool::Kiro, &Options::default());
/// assert_eq!(out.folder, ".kiro/specs/seat-limits");
/// assert!(out.files[0].text.contains("pa_7Hk2mQ9xR4tV8wZ1@2#FR1"));
/// assert_eq!(out.not_exported[0].id, "CR1");
/// ```
pub fn export(doc: &Value, tool: Tool, options: &Options) -> Export {
    let source = Source::new(doc);
    let name = name(doc, options);
    match tool {
        Tool::OpenSpec => openspec::export(&source, &name),
        Tool::Kiro => kiro::export(&source, &name),
        Tool::SpecKit => speckit::export(&source, &name, options.number.unwrap_or(1)),
    }
}

/// The name that [`export`] uses for the change, feature or capability:
/// [`Options::name`] in kebab case, or a name from the title of the document.
pub fn name(doc: &Value, options: &Options) -> String {
    options
        .name
        .as_deref()
        .map(slug)
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| {
            let name = slug(doc.get("title").and_then(Value::as_str).unwrap_or(""));
            if name.is_empty() {
                "change".to_string()
            } else {
                name
            }
        })
}

/// Runs a function that writes into a new string. Writing to a `String`
/// cannot fail.
pub(crate) fn render(f: impl FnOnce(&mut String) -> fmt::Result) -> String {
    let mut out = String::new();
    f(&mut out).expect("writing to a String cannot fail");
    out
}

/// One item of the document.
#[derive(Clone, Copy)]
pub(crate) struct Item<'a> {
    pub id: &'a str,
    pub item_type: Option<ItemType>,
    map: &'a Map<String, Value>,
}

impl<'a> Item<'a> {
    /// A string field as it is stored, or "".
    pub fn str(&self, key: &str) -> &'a str {
        self.map.get(key).and_then(Value::as_str).unwrap_or("")
    }

    /// The text, safe for one line of Markdown.
    pub fn text(&self) -> String {
        inline(self.str("text"))
    }

    /// A string field, safe for one line of Markdown, if it is present and
    /// not empty.
    pub fn field(&self, key: &str) -> Option<String> {
        Some(inline(self.map.get(key)?.as_str()?)).filter(|s| !s.is_empty())
    }

    /// A list of strings, each safe for one line of Markdown.
    pub fn list(&self, key: &str) -> Vec<String> {
        self.map
            .get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(inline)
            .collect()
    }

    /// The items of a list field joined with `sep`, if there are any.
    pub fn joined(&self, key: &str, sep: &str) -> Option<String> {
        Some(self.list(key).join(sep)).filter(|s| !s.is_empty())
    }

    pub fn bool(&self, key: &str) -> bool {
        self.map.get(key).and_then(Value::as_bool).unwrap_or(false)
    }

    /// `detail`, a Markdown text, as a block that cannot start a section of
    /// the tool's file (see [`block`]).
    pub fn detail(&self) -> Option<String> {
        Some(block(self.str("detail"))).filter(|d| !d.is_empty())
    }

    /// The examples: (kind, text). A missing kind is `normal`.
    pub fn examples(&self) -> Vec<(&'a str, String)> {
        self.map
            .get("examples")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|e| {
                let text = e.get("text")?.as_str()?;
                let kind = e.get("kind").and_then(Value::as_str).unwrap_or("normal");
                Some((kind, inline(text)))
            })
            .collect()
    }

    /// The IDs of the items in the same artifact that this item links to with
    /// `rel`.
    pub fn links(&self, rel: &str, artifact: &str) -> Vec<&'a str> {
        self.map
            .get("refs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|r| r.get("rel").and_then(Value::as_str) == Some(rel))
            .filter(|r| {
                r.get("artifact")
                    .and_then(Value::as_str)
                    .is_none_or(|a| a == artifact)
            })
            .filter_map(|r| r.get("item")?.as_str())
            .collect()
    }

    /// A short name for a heading: the first sentence of the text, at most
    /// about 60 characters, with no full stop at the end.
    pub fn short(&self) -> String {
        short_title(&self.text())
    }
}

/// One item and whether it is exported.
struct Entry<'a> {
    item: Item<'a>,
    /// Why the item is never exported, or `None` for an accepted item.
    held: Option<&'static str>,
}

/// The document, ready for the exporters.
pub(crate) struct Source<'a> {
    pub artifact: &'a str,
    pub version: Option<u64>,
    doc: &'a Value,
    /// Every item with an ID, in document order. If two items have the same
    /// ID, only the first one is here.
    entries: Vec<Entry<'a>>,
    /// Item ID to its position in `entries`.
    index: HashMap<&'a str, usize>,
    /// Item ID to the accepted success signals that verify it, in document
    /// order.
    verified_by: HashMap<&'a str, Vec<Item<'a>>>,
}

impl<'a> Source<'a> {
    fn new(doc: &'a Value) -> Self {
        let artifact = doc.get("id").and_then(Value::as_str).unwrap_or("");
        let mut source = Source {
            artifact,
            version: doc.get("version").and_then(crate::whole_number),
            doc,
            entries: Vec::new(),
            index: HashMap::new(),
            verified_by: HashMap::new(),
        };
        let items = doc.get("items").and_then(Value::as_array);
        for map in items.into_iter().flatten().filter_map(Value::as_object) {
            let Some(id) = map
                .get("id")
                .and_then(Value::as_str)
                .filter(|i| !i.is_empty())
            else {
                continue;
            };
            if source.index.contains_key(id) {
                continue;
            }
            let item_type = map
                .get("type")
                .and_then(Value::as_str)
                .and_then(|t| t.parse::<ItemType>().ok());
            let item = Item { id, item_type, map };
            let held = match map.get("status").and_then(Value::as_str) {
                Some("accepted") => None,
                Some("proposed") => Some("proposed: the team has not agreed to it yet"),
                Some("rejected") => Some("rejected: an agent must not act on it"),
                _ => Some("the item has no valid status"),
            };
            if held.is_none() && item_type == Some(ItemType::SuccessSignal) {
                for target in item.links("verifies", artifact) {
                    source.verified_by.entry(target).or_default().push(item);
                }
            }
            source.index.insert(id, source.entries.len());
            source.entries.push(Entry { item, held });
        }
        source
    }

    /// The reference string of an item ID, for example
    /// `pa_2Xc5Rv8Tn1Mb4Kz7@3#FR4`.
    pub fn reference(&self, id: &str) -> String {
        crate::reference_string(self.artifact, self.version, id)
    }

    /// The reference string in backticks, for the end of a line.
    pub fn tag(&self, id: &str) -> String {
        format!("`{}`", self.reference(id))
    }

    /// The artifact and version, for example `pa_2Xc5Rv8Tn1Mb4Kz7@3`.
    pub fn label(&self) -> String {
        crate::version_label(self.artifact, self.version)
    }

    pub fn str(&self, key: &str) -> &'a str {
        self.doc.get(key).and_then(Value::as_str).unwrap_or("")
    }

    pub fn title(&self) -> String {
        inline(self.str("title"))
    }

    /// The date of `created_at`, for example `2026-11-08`.
    pub fn date(&self) -> &'a str {
        crate::date_of(self.str("created_at"))
    }

    /// The summary, as a Markdown block (see [`block`]), if there is one.
    pub fn summary(&self) -> Option<String> {
        Some(block(self.str("summary"))).filter(|s| !s.is_empty())
    }

    /// The accepted items, in document order.
    fn accepted(&self) -> impl Iterator<Item = Item<'a>> + '_ {
        self.entries
            .iter()
            .filter(|e| e.held.is_none())
            .map(|e| e.item)
    }

    /// The accepted items of one type, in document order.
    pub fn of(&self, t: ItemType) -> Vec<Item<'a>> {
        self.accepted().filter(|i| i.item_type == Some(t)).collect()
    }

    /// The accepted item with this ID.
    pub fn get(&self, id: &str) -> Option<Item<'a>> {
        let entry = &self.entries[*self.index.get(id)?];
        entry.held.is_none().then_some(entry.item)
    }

    /// The accepted success signals that verify one of these item IDs, in
    /// document order, each one once.
    pub fn signals_for(&self, ids: &[&str]) -> Vec<Item<'a>> {
        let mut out: Vec<Item<'a>> = Vec::new();
        for id in ids {
            for s in self.verified_by.get(id).into_iter().flatten() {
                if !out.iter().any(|x| x.id == s.id) {
                    out.push(*s);
                }
            }
        }
        out.sort_by_key(|s| self.index[s.id]);
        out
    }

    fn not_exported(&self, item: &Item, reason: String) -> NotExported {
        NotExported {
            reference: self.reference(item.id),
            id: item.id.to_string(),
            item_type: item.str("type").to_string(),
            reason,
        }
    }

    /// The items that are not exported, in document order: items that are
    /// not accepted, critiques, items of an unknown type and accepted items
    /// of the given types, which the tool has no place for.
    pub fn skipped(&self, tool: Tool, types: &[ItemType]) -> Vec<NotExported> {
        self.entries
            .iter()
            .filter_map(|e| {
                let reason = match (e.held, e.item.item_type) {
                    (Some(held), _) => held.to_string(),
                    (None, None) => "the item has no known type".to_string(),
                    (None, Some(ItemType::Critique)) => {
                        "critiques are not exported: they are about an input, not about what to build"
                            .to_string()
                    }
                    (None, Some(t)) if types.contains(&t) => format!(
                        "{} has no place for {}",
                        tool.display_name(),
                        t.section().to_lowercase()
                    ),
                    (None, Some(_)) => return None,
                };
                Some(self.not_exported(&e.item, reason))
            })
            .collect()
    }
}

/// Writes the "Not exported" note: a Markdown section that lists each item
/// that is not in the files. Writes nothing if every item is exported.
pub(crate) fn not_exported_note(
    out: &mut String,
    source: &Source,
    items: &[NotExported],
    heading: &str,
) -> fmt::Result {
    if items.is_empty() {
        return Ok(());
    }
    writeln!(out, "\n{heading} Not exported\n")?;
    writeln!(
        out,
        "These items of {} are not in these files. Read them in the OpenSkopos document.\n",
        source.label()
    )?;
    for n in items {
        writeln!(out, "- `{}` ({}): {}", n.reference, n.item_type, n.reason)?;
    }
    Ok(())
}

/// The words for a constraint modality, for example `Ask first`.
pub(crate) fn modality(item: &Item) -> &'static str {
    match item.str("modality") {
        "always" => "Always",
        "never" => "Never",
        "ask_first" => "Ask first",
        "prefer" => "Prefer",
        "avoid" => "Avoid",
        _ => "Rule",
    }
}

/// Writes a constraint as a list item, with its other fields as nested
/// items.
pub(crate) fn constraint(out: &mut String, source: &Source, item: &Item) -> fmt::Result {
    writeln!(
        out,
        "- **{}**: {} {}",
        modality(item),
        item.text(),
        source.tag(item.id)
    )?;
    let fields = [
        ("Why", item.field("rationale")),
        ("Applies to", item.joined("scope", "; ")),
        ("Enforced by", item.field("enforced_by")),
        ("Ask", item.field("approver")),
        ("Bring", item.joined("bring", "; ")),
    ];
    for (label, value) in fields {
        if let Some(v) = value {
            writeln!(out, "  - {label}: {v}")?;
        }
    }
    Ok(())
}

/// Writes a decision as a section with a heading of the given level.
pub(crate) fn decision(out: &mut String, source: &Source, item: &Item, level: &str) -> fmt::Result {
    writeln!(out, "{level} {}: {}\n", item.id, item.short())?;
    writeln!(
        out,
        "- **Decision**: {} {}",
        item.text(),
        source.tag(item.id)
    )?;
    if let Some(r) = item.field("rationale") {
        writeln!(out, "- **Rationale**: {r}")?;
    }
    if let Some(a) = item.joined("alternatives", "; ") {
        writeln!(out, "- **Alternatives considered**: {a}")?;
    }
    if let Some(c) = item.joined("consequences", "; ") {
        writeln!(out, "- **Consequences**: {c}")?;
    }
    if let Some(d) = item.detail() {
        writeln!(out, "\n{d}")?;
    }
    Ok(())
}

/// Writes a goal as a list item.
pub(crate) fn goal(out: &mut String, source: &Source, item: &Item) -> fmt::Result {
    writeln!(out, "- {} {}", item.text(), source.tag(item.id))?;
    if let Some(f) = item.field("for") {
        writeln!(out, "  - For: {f}")?;
    }
    Ok(())
}

/// Writes a success signal as a list item.
pub(crate) fn signal(out: &mut String, source: &Source, item: &Item) -> fmt::Result {
    writeln!(out, "- {} {}", item.text(), source.tag(item.id))?;
    if let Some(v) = item.field("verification") {
        writeln!(out, "  - Verification: {v}")?;
    }
    Ok(())
}

/// Writes a term as a list item: `**word**: definition`.
pub(crate) fn term(out: &mut String, source: &Source, item: &Item) -> fmt::Result {
    writeln!(
        out,
        "- **{}**: {} {}",
        bold(&item.text()),
        item.field("definition").unwrap_or_default(),
        source.tag(item.id)
    )?;
    if let Some(a) = item.joined("aliases", ", ") {
        writeln!(out, "  - Also called: {a}")?;
    }
    let names: Vec<String> = item.list("code_names").iter().map(|n| code(n)).collect();
    if !names.is_empty() {
        writeln!(out, "  - In the code: {}", names.join(", "))?;
    }
    Ok(())
}

/// The label of an example kind.
pub(crate) fn example_label(kind: &str) -> &'static str {
    match kind {
        "edge" => "Edge case",
        "forbidden" => "Must not happen",
        _ => "Example",
    }
}

/// `1 word` or `n words`.
pub(crate) fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}

/// Indents every line of a Markdown text, for a block inside a list item.
pub(crate) fn indent(text: &str, by: &str) -> String {
    text.lines()
        .map(|l| {
            if l.trim().is_empty() {
                String::new()
            } else {
                format!("{by}{l}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Plain text for one line of Markdown: line breaks and runs of spaces become
/// one space. A `#` at the start gets a backslash, so that the line cannot be
/// read as a heading.
pub(crate) fn inline(text: &str) -> String {
    let line = crate::one_line(text);
    if line.starts_with('#') {
        format!("\\{line}")
    } else {
        line
    }
}

/// Text for inside `**...**`: each `*` gets a backslash.
pub(crate) fn bold(text: &str) -> String {
    text.replace('*', "\\*")
}

/// Text in backticks. A text with a backtick gets two backticks and spaces,
/// as CommonMark allows.
pub(crate) fn code(text: &str) -> String {
    if text.contains('`') {
        format!("`` {text} ``")
    } else {
        format!("`{text}`")
    }
}

/// A Markdown text (a `detail` or the `summary`) as a block that cannot start
/// a section of the tool's file. Each ATX heading outside a code fence, such
/// as `## Background`, becomes a bold line. Tools read the headings of their
/// files, for example OpenSpec ends a requirement at a `##` heading.
/// Surrounding blank lines are removed.
pub(crate) fn block(markdown: &str) -> String {
    // The open fence: its character and length.
    let mut fence: Option<(char, usize)> = None;
    let mut out = Vec::new();
    for line in markdown.trim().lines() {
        let trimmed = line.trim_start_matches(' ');
        let indent = line.len() - trimmed.len();
        let marker = trimmed.chars().next().filter(|c| *c == '`' || *c == '~');
        if indent <= 3 {
            if let Some(c) = marker {
                let run = trimmed.chars().take_while(|x| *x == c).count();
                if run >= 3 {
                    match fence {
                        None => fence = Some((c, run)),
                        Some((open, len))
                            if open == c && run >= len && trimmed[run..].trim().is_empty() =>
                        {
                            fence = None
                        }
                        Some(_) => {}
                    }
                    out.push(line.to_string());
                    continue;
                }
            }
        }
        match (fence, heading_text(line)) {
            (None, Some("")) => out.push(String::new()),
            (None, Some(text)) => out.push(format!("**{}**", bold(text))),
            _ => out.push(line.to_string()),
        }
    }
    out.join("\n").trim().to_string()
}

/// The text of an ATX heading line such as `## Background ##`, or `None` if
/// the line is not a heading.
fn heading_text(line: &str) -> Option<&str> {
    let trimmed = line.trim_start_matches(' ');
    if line.len() - trimmed.len() > 3 {
        return None;
    }
    let level = trimmed.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = &trimmed[level..];
    if !(rest.is_empty() || rest.starts_with([' ', '\t'])) {
        return None;
    }
    let rest = rest.trim();
    // A closing run of `#` after a space is not part of the text.
    let without_closing = rest.trim_end_matches('#');
    if without_closing.is_empty() || without_closing.ends_with([' ', '\t']) {
        return Some(without_closing.trim_end());
    }
    Some(rest)
}

/// A name in kebab case: lowercase ASCII letters and digits, with `-`
/// between words, at most 50 characters, cut at a word.
pub(crate) fn slug(text: &str) -> String {
    let mut out = String::new();
    let words = text
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty());
    for w in words {
        if !out.is_empty() && out.len() + 1 + w.len() > 50 {
            break;
        }
        if !out.is_empty() {
            out.push('-');
        }
        out.push_str(&w.to_ascii_lowercase());
    }
    out.truncate(50);
    out
}

/// The first sentence of a text, at most about 60 characters, cut at a word,
/// with no full stop at the end. A sentence ends at `.`, `?` or `!` before a
/// space or the end of the text, so `v2.5` and `e.g.` do not end it.
pub(crate) fn short_title(text: &str) -> String {
    let mut end = text.len();
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if matches!(c, '.' | '?' | '!') && chars.peek().is_none_or(|(_, n)| n.is_whitespace()) {
            end = i + c.len_utf8();
            if !is_abbreviation(&text[..end]) {
                break;
            }
            end = text.len();
        }
    }
    let first = text[..end].trim();
    let first = first.strip_suffix('.').unwrap_or(first);
    if first.chars().count() <= 60 {
        return first.to_string();
    }
    let mut out = String::new();
    for word in first.split(' ') {
        if out.chars().count() + word.chars().count() + 1 > 57 {
            break;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    if out.is_empty() {
        out = first.chars().take(57).collect();
    }
    out.push_str("...");
    out
}

/// True if a text ends with a common abbreviation, such as `e.g.`.
fn is_abbreviation(text: &str) -> bool {
    let last = text.rsplit(' ').next().unwrap_or(text).to_ascii_lowercase();
    matches!(last.as_str(), "e.g." | "i.e." | "etc." | "vs." | "approx.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_is_kebab_case() {
        assert_eq!(
            slug("Spec: seat limits on plan changes"),
            "spec-seat-limits-on-plan-changes"
        );
        assert_eq!(slug("  Ünïcode -- only!  "), "n-code-only");
        assert_eq!(slug("???"), "");
        assert!(slug(&"word ".repeat(30)).len() <= 50);
    }

    #[test]
    fn short_title_takes_the_first_sentence() {
        assert_eq!(
            short_title("The seat count never goes below zero."),
            "The seat count never goes below zero"
        );
        assert_eq!(short_title("Do guests count? Maybe."), "Do guests count?");
        let long = short_title(
            "A plan change that leaves more seats in use than the new limit is blocked.",
        );
        assert!(
            long.ends_with("...") && long.chars().count() <= 60,
            "{long}"
        );
    }

    #[test]
    fn short_title_does_not_end_inside_numbers_or_abbreviations() {
        assert_eq!(
            short_title("Support API v2.5 for all clients."),
            "Support API v2.5 for all clients"
        );
        assert_eq!(
            short_title("Use a cache, e.g. Redis. Then more."),
            "Use a cache, e.g. Redis"
        );
    }

    #[test]
    fn block_turns_headings_into_bold_lines_outside_fences() {
        let md = "\n## Background\nWhy.\n\n### Requirement: fake ###\n```sh\n# a comment\n## not a heading\n```\n#hashtag\n    # indented code\n";
        assert_eq!(
            block(md),
            "**Background**\nWhy.\n\n**Requirement: fake**\n```sh\n# a comment\n## not a heading\n```\n#hashtag\n    # indented code"
        );
        assert_eq!(
            block("~~~\n# x\n```\n# y\n~~~\n# z"),
            "~~~\n# x\n```\n# y\n~~~\n**z**"
        );
    }

    #[test]
    fn inline_text_cannot_start_a_heading() {
        assert_eq!(inline("## Not\na heading"), "\\## Not a heading");
        assert_eq!(bold("a*b"), "a\\*b");
        assert_eq!(code("a`b"), "`` a`b ``");
    }

    #[test]
    fn tool_names_round_trip() {
        for t in Tool::ALL {
            assert_eq!(t.as_str().parse::<Tool>().unwrap(), t);
        }
        assert!("jira".parse::<Tool>().is_err());
    }
}
