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

use std::fmt;
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

/// One item of the document.
#[derive(Clone, Copy)]
pub(crate) struct Item<'a> {
    pub id: &'a str,
    pub item_type: Option<ItemType>,
    pub map: &'a Map<String, Value>,
}

impl<'a> Item<'a> {
    /// A string field, or "".
    pub fn str(&self, key: &str) -> &'a str {
        self.map.get(key).and_then(Value::as_str).unwrap_or("")
    }

    /// The text on one line.
    pub fn text(&self) -> String {
        one_line(self.str("text"))
    }

    /// A string field on one line, if it is present and not empty.
    pub fn field(&self, key: &str) -> Option<String> {
        Some(one_line(self.map.get(key)?.as_str()?)).filter(|s| !s.is_empty())
    }

    /// A list of strings, each on one line.
    pub fn list(&self, key: &str) -> Vec<String> {
        self.map
            .get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(one_line)
            .collect()
    }

    pub fn bool(&self, key: &str) -> bool {
        self.map.get(key).and_then(Value::as_bool).unwrap_or(false)
    }

    /// `detail`, a Markdown text, with surrounding blank lines removed.
    pub fn detail(&self) -> Option<&'a str> {
        Some(self.str("detail").trim()).filter(|d| !d.is_empty())
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
                Some((kind, one_line(text)))
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

/// The document, ready for the exporters.
pub(crate) struct Source<'a> {
    pub doc: &'a Value,
    pub artifact: &'a str,
    pub version: Option<u64>,
    /// Accepted items, in document order.
    pub items: Vec<Item<'a>>,
    /// Proposed, rejected and other items that are never exported.
    pub held_back: Vec<NotExported>,
}

impl<'a> Source<'a> {
    fn new(doc: &'a Value) -> Self {
        let artifact = doc.get("id").and_then(Value::as_str).unwrap_or("");
        let version = doc.get("version").and_then(crate::whole_number);
        let mut source = Source {
            doc,
            artifact,
            version,
            items: Vec::new(),
            held_back: Vec::new(),
        };
        for value in doc
            .get("items")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(map) = value.as_object() else {
                continue;
            };
            let Some(id) = map
                .get("id")
                .and_then(Value::as_str)
                .filter(|i| !i.is_empty())
            else {
                continue;
            };
            let item_type = map
                .get("type")
                .and_then(Value::as_str)
                .and_then(|t| ItemType::ALL.into_iter().find(|x| x.as_str() == t));
            let item = Item { id, item_type, map };
            match map.get("status").and_then(Value::as_str) {
                Some("accepted") => source.items.push(item),
                Some("proposed") => source.held_back.push(
                    source.not_exported(&item, "proposed: the team has not agreed to it yet"),
                ),
                Some("rejected") => source
                    .held_back
                    .push(source.not_exported(&item, "rejected: an agent must not act on it")),
                _ => source
                    .held_back
                    .push(source.not_exported(&item, "the item has no valid status")),
            }
        }
        source
    }

    /// The reference string of an item ID, for example
    /// `pa_2Xc5Rv8Tn1Mb4Kz7@3#FR4`.
    pub fn reference(&self, id: &str) -> String {
        match self.version {
            Some(v) => format!("{}@{v}#{id}", self.artifact),
            None => format!("{}#{id}", self.artifact),
        }
    }

    /// The reference string in backticks, for the end of a line.
    pub fn tag(&self, id: &str) -> String {
        format!("`{}`", self.reference(id))
    }

    /// The artifact and version, for example `pa_2Xc5Rv8Tn1Mb4Kz7@3`.
    pub fn label(&self) -> String {
        match self.version {
            Some(v) => format!("{}@{v}", self.artifact),
            None => self.artifact.to_string(),
        }
    }

    pub fn str(&self, key: &str) -> &'a str {
        self.doc.get(key).and_then(Value::as_str).unwrap_or("")
    }

    pub fn title(&self) -> String {
        one_line(self.str("title"))
    }

    /// The summary, a Markdown text, if there is one.
    pub fn summary(&self) -> Option<&'a str> {
        Some(self.str("summary").trim()).filter(|s| !s.is_empty())
    }

    /// The accepted items of one type, in document order.
    pub fn of(&self, t: ItemType) -> Vec<Item<'a>> {
        self.items
            .iter()
            .filter(|i| i.item_type == Some(t))
            .copied()
            .collect()
    }

    /// The accepted item with this ID.
    pub fn get(&self, id: &str) -> Option<Item<'a>> {
        self.items.iter().find(|i| i.id == id).copied()
    }

    pub fn not_exported(&self, item: &Item, reason: &str) -> NotExported {
        NotExported {
            reference: self.reference(item.id),
            id: item.id.to_string(),
            item_type: item.str("type").to_string(),
            reason: reason.to_string(),
        }
    }

    /// The items that are never exported, and the accepted items of the
    /// given types, which the tool has no place for. In document order.
    pub fn skipped(&self, tool: Tool, types: &[ItemType]) -> Vec<NotExported> {
        let mut out = Vec::new();
        for value in self
            .doc
            .get("items")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(id) = value.get("id").and_then(Value::as_str) else {
                continue;
            };
            if let Some(held) = self.held_back.iter().find(|n| n.id == id) {
                if !out.iter().any(|n: &NotExported| n.id == id) {
                    out.push(held.clone());
                }
                continue;
            }
            let Some(item) = self.get(id) else { continue };
            let skip = match item.item_type {
                Some(t) => types.contains(&t),
                None => true,
            };
            if skip && !out.iter().any(|n: &NotExported| n.id == id) {
                let reason = match item.item_type {
                    Some(ItemType::Critique) => {
                        "critiques are not exported: they are about an input, not about what to build"
                            .to_string()
                    }
                    Some(t) => format!(
                        "{} has no place for {}",
                        tool.display_name(),
                        t.section().to_lowercase()
                    ),
                    None => "the item has no known type".to_string(),
                };
                out.push(self.not_exported(&item, &reason));
            }
        }
        out
    }
}

/// The "Not exported" note: a Markdown section that lists each item that is
/// not in the files. Empty if every item is exported.
pub(crate) fn not_exported_note(source: &Source, items: &[NotExported], heading: &str) -> String {
    if items.is_empty() {
        return String::new();
    }
    let mut out = format!(
        "\n{heading} Not exported\n\n\
         These items of {} are not in these files. Read them in the OpenSkopos document.\n\n",
        source.label()
    );
    for n in items {
        out.push_str(&format!(
            "- `{}` ({}): {}\n",
            n.reference, n.item_type, n.reason
        ));
    }
    out
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

/// A constraint as a list item, with its other fields as nested items.
pub(crate) fn constraint(source: &Source, item: &Item) -> String {
    let mut out = format!(
        "- **{}**: {} {}\n",
        modality(item),
        item.text(),
        source.tag(item.id)
    );
    let mut sub = |label: &str, value: Option<String>| {
        if let Some(v) = value {
            out.push_str(&format!("  - {label}: {v}\n"));
        }
    };
    sub("Why", item.field("rationale"));
    sub(
        "Applies to",
        Some(item.list("scope").join("; ")).filter(|s| !s.is_empty()),
    );
    sub("Enforced by", item.field("enforced_by"));
    sub("Ask", item.field("approver"));
    sub(
        "Bring",
        Some(item.list("bring").join("; ")).filter(|s| !s.is_empty()),
    );
    out
}

/// A decision as a section with a heading of the given level.
pub(crate) fn decision(source: &Source, item: &Item, level: &str) -> String {
    let mut out = format!("{level} {}: {}\n\n", item.id, item.short());
    out.push_str(&format!(
        "- **Decision**: {} {}\n",
        item.text(),
        source.tag(item.id)
    ));
    if let Some(r) = item.field("rationale") {
        out.push_str(&format!("- **Rationale**: {r}\n"));
    }
    let alternatives = item.list("alternatives");
    if !alternatives.is_empty() {
        out.push_str(&format!(
            "- **Alternatives considered**: {}\n",
            alternatives.join("; ")
        ));
    }
    let consequences = item.list("consequences");
    if !consequences.is_empty() {
        out.push_str(&format!(
            "- **Consequences**: {}\n",
            consequences.join("; ")
        ));
    }
    if let Some(d) = item.detail() {
        out.push_str(&format!("\n{d}\n"));
    }
    out
}

/// A goal as a list item.
pub(crate) fn goal(source: &Source, item: &Item) -> String {
    let mut out = format!("- {} {}\n", item.text(), source.tag(item.id));
    if let Some(f) = item.field("for") {
        out.push_str(&format!("  - For: {f}\n"));
    }
    out
}

/// A success signal as a list item.
pub(crate) fn signal(source: &Source, item: &Item) -> String {
    let mut out = format!("- {} {}\n", item.text(), source.tag(item.id));
    if let Some(v) = item.field("verification") {
        out.push_str(&format!("  - Verification: {v}\n"));
    }
    out
}

/// A term as a list item: `**word**: definition`.
pub(crate) fn term(source: &Source, item: &Item) -> String {
    let mut out = format!(
        "- **{}**: {} {}\n",
        item.text(),
        item.field("definition").unwrap_or_default(),
        source.tag(item.id)
    );
    let aliases = item.list("aliases");
    if !aliases.is_empty() {
        out.push_str(&format!("  - Also called: {}\n", aliases.join(", ")));
    }
    let names = item.list("code_names");
    if !names.is_empty() {
        let names: Vec<String> = names.iter().map(|n| format!("`{n}`")).collect();
        out.push_str(&format!("  - In the code: {}\n", names.join(", ")));
    }
    out
}

/// The label of an example kind.
pub(crate) fn example_label(kind: &str) -> &'static str {
    match kind {
        "edge" => "Edge case",
        "forbidden" => "Must not happen",
        _ => "Example",
    }
}

/// The success signals that verify one of these item IDs, in document order.
pub(crate) fn signals_for<'a>(source: &Source<'a>, ids: &[&str]) -> Vec<Item<'a>> {
    source
        .of(ItemType::SuccessSignal)
        .into_iter()
        .filter(|s| {
            s.links("verifies", source.artifact)
                .iter()
                .any(|v| ids.contains(v))
        })
        .collect()
}

/// Text on one line: line breaks and runs of spaces become one space.
pub(crate) fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
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

/// A name in kebab case: lowercase ASCII letters and digits, with `-`
/// between words, at most 50 characters, cut at a word.
pub(crate) fn slug(text: &str) -> String {
    let mut words: Vec<String> = Vec::new();
    let mut word = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            word.push(c.to_ascii_lowercase());
        } else if !word.is_empty() {
            words.push(std::mem::take(&mut word));
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    let mut out = String::new();
    for w in words {
        if !out.is_empty() && out.len() + 1 + w.len() > 50 {
            break;
        }
        if !out.is_empty() {
            out.push('-');
        }
        out.push_str(&w);
    }
    out.truncate(50);
    out
}

/// The first sentence of a text, at most about 60 characters, cut at a word,
/// with no full stop at the end.
pub(crate) fn short_title(text: &str) -> String {
    let first = text
        .split_inclusive(['.', '?', '!'])
        .next()
        .unwrap_or(text)
        .trim();
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
    fn tool_names_round_trip() {
        for t in Tool::ALL {
            assert_eq!(t.as_str().parse::<Tool>().unwrap(), t);
        }
        assert!("jira".parse::<Tool>().is_err());
    }
}
