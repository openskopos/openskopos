//! The Markdown writer (specification 10.1 to 10.4 and 10.7).

use serde_json::{Map, Value};

use super::{evidence_main, item_type, kind, list_key, Kind, ENVELOPE, OTHER, SCHEMA_KEY};
use crate::model::ItemType;
use crate::whole_number;

/// Writes a document as Markdown.
///
/// The file has the front matter, the title, the summary and one section for
/// each item type that has items (10.1 to 10.4). Reading the file with
/// [`super::read`] gives the same document again, except for the order of
/// items between types.
///
/// `inputs` are other documents that the writer can copy material from
/// (10.7): pinned glossaries and agent rules, and the artifacts that refs
/// point to. Give an empty slice to copy nothing. A reader ignores this
/// material.
///
/// The function writes any value, also a document that is not valid. It
/// leaves out what it cannot place.
pub fn write(doc: &Value, inputs: &[Value]) -> String {
    let empty = Map::new();
    let map = doc.as_object().unwrap_or(&empty);
    let mut out: Vec<String> = vec!["---".into()];
    let fm = front_matter(map);
    if !fm.is_empty() {
        out.push(fm.trim_end_matches('\n').to_string());
    }
    out.push("---".into());
    out.push(String::new());
    out.push(format!("# {}", one_line_value(map.get("title"))));
    if let Some(summary) = map.get("summary").and_then(Value::as_str) {
        if !summary.trim().is_empty() {
            out.push(String::new());
            out.push(summary.trim().to_string());
        }
    }
    let items: &[Value] = map
        .get("items")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice);
    for t in ItemType::ALL {
        let of_type: Vec<&Value> = items.iter().filter(|i| item_type(i) == Some(t)).collect();
        write_section(&mut out, t.section(), &of_type, None);
    }
    let copied = copied_material(map, items, inputs);
    if !copied.is_empty() {
        out.push(String::new());
        out.push(format!("## {OTHER}"));
        for group in copied {
            let src = group.source;
            let (id, version) = (text(src.get("id")), text(src.get("version")));
            let heading = format!(
                "### {} from {} ({id}@{version})",
                group.section,
                one_line_value(src.get("title"))
            );
            let prefix = format!("{id}@{version}#");
            write_section_body(&mut out, heading, &group.items, Some(&prefix));
        }
    }
    let mut text = out.join("\n");
    text.push('\n');
    text
}

fn write_section(out: &mut Vec<String>, name: &str, items: &[&Value], prefix: Option<&str>) {
    if items.is_empty() {
        return;
    }
    write_section_body(out, format!("## {name}"), items, prefix);
}

fn write_section_body(
    out: &mut Vec<String>,
    heading: String,
    items: &[&Value],
    prefix: Option<&str>,
) {
    out.push(String::new());
    out.push(heading);
    out.push(String::new());
    for (n, item) in items.iter().enumerate() {
        // A blank line between two items only when one of them has a detail.
        if n > 0 && (has_detail(items[n - 1]) || has_detail(item)) {
            out.push(String::new());
        }
        write_item(out, item, prefix);
    }
}

fn has_detail(item: &Value) -> bool {
    item.get("detail")
        .and_then(Value::as_str)
        .is_some_and(|d| !d.trim().is_empty())
}

/// The front matter: the envelope in the order of section 3, then the `x-`
/// keys. Not the title, the summary or the items.
fn front_matter(map: &Map<String, Value>) -> String {
    let mut fm = Map::new();
    for key in ENVELOPE {
        if matches!(*key, "title" | "summary" | "items") {
            continue;
        }
        if let Some(v) = map.get(*key) {
            fm.insert((*key).to_string(), v.clone());
        }
    }
    for (k, v) in map {
        if k.starts_with("x-") {
            fm.insert(k.clone(), v.clone());
        }
    }
    if fm.is_empty() {
        return String::new();
    }
    let yaml = crate::yaml::write(&Value::Object(fm));
    // The YAML writer puts `$schema` first. Markdown writes it `schema`.
    match yaml.strip_prefix(&format!("{SCHEMA_KEY}:")) {
        Some(rest) => format!("schema:{rest}"),
        None => yaml,
    }
}

/// A text with each line break, and the spaces around it, made one space.
pub(crate) fn one_line(s: &str) -> String {
    s.split('\n')
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// A value as text on one line. A value that is not a string is written as
/// JSON.
fn one_line_value(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => one_line(s),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

/// A value as text, with no change.
fn text(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => {
            whole_number(&Value::Number(n.clone())).map_or_else(|| n.to_string(), |n| n.to_string())
        }
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

/// `<artifact>@<version>#<item>`, with each part only if it is there.
pub(crate) fn ref_string(o: &Value) -> String {
    let mut s = String::new();
    if let Some(a) = o.get("artifact") {
        s.push_str(&text(Some(a)));
        if let Some(v) = o.get("version") {
            s.push('@');
            s.push_str(&text(Some(v)));
        }
        if o.get("item").is_some() {
            s.push('#');
        }
    }
    if let Some(i) = o.get("item") {
        s.push_str(&text(Some(i)));
    }
    s
}

/// The value of an `x-` key. A string that would read as JSON keeps its
/// quotes, so that it reads back as a string.
fn x_value(v: &Value) -> String {
    match v {
        Value::String(s) if serde_json::from_str::<Value>(s).is_err() && !s.contains('\n') => {
            s.trim().to_string()
        }
        other => other.to_string(),
    }
}

struct Lines<'a> {
    out: &'a mut Vec<String>,
    indent: &'a str,
}

impl Lines<'_> {
    /// `<indent><extra>- <key>: <value>`. An empty value gives `- <key>:`.
    fn field(&mut self, extra: usize, key: &str, value: &str) {
        let pad = " ".repeat(extra);
        if value.is_empty() {
            self.out.push(format!("{}{pad}- {key}:", self.indent));
        } else {
            self.out
                .push(format!("{}{pad}- {key}: {value}", self.indent));
        }
    }
}

fn write_item(out: &mut Vec<String>, item: &Value, prefix: Option<&str>) {
    let id = text(item.get("id"));
    let id = match prefix {
        Some(p) => format!("{p}{id}"),
        None => id,
    };
    let line = one_line_value(item.get("text"));
    out.push(if line.is_empty() {
        format!("- {id}:")
    } else {
        format!("- {id}: {line}")
    });
    let mut l = Lines { out, indent: "  " };
    let get = |k: &str| item.get(k);

    if let Some(status) = get("status") {
        if status != "accepted" {
            l.field(0, "status", &text(Some(status)));
        }
    }
    let fields = item_type(item).map_or(&[][..], ItemType::fields);
    for field in fields {
        let Some(v) = get(field) else { continue };
        match kind(field) {
            Kind::Text | Kind::Enum => l.field(0, field, &one_line_value(Some(v))),
            Kind::Bool => l.field(0, field, &v.to_string()),
            Kind::List => {
                for x in v.as_array().into_iter().flatten() {
                    l.field(0, list_key(field), &one_line_value(Some(x)));
                }
            }
            Kind::Examples => {
                for ex in v.as_array().into_iter().flatten() {
                    let key = ex.get("kind").and_then(Value::as_str).unwrap_or("example");
                    l.field(0, key, &one_line_value(ex.get("text")));
                }
            }
            Kind::Target => {
                if v.get("item").is_some() || v.get("artifact").is_some() {
                    l.field(0, "target", &ref_string(v));
                    if let Some(s) = v.get("section") {
                        l.field(2, "section", &one_line_value(Some(s)));
                    }
                } else {
                    l.field(0, "target", &one_line_value(v.get("section")));
                }
            }
            Kind::Resolution => {
                l.field(0, "resolution", &text(v.get("outcome")));
                for sub in ["note", "decided_by"] {
                    if let Some(s) = v.get(sub) {
                        l.field(2, sub, &one_line_value(Some(s)));
                    }
                }
            }
        }
    }
    for r in get("refs").and_then(Value::as_array).into_iter().flatten() {
        let rel = r.get("rel").and_then(Value::as_str).unwrap_or("ref");
        l.field(0, rel, &ref_string(r));
    }
    if let Some(list) = get("evidence").and_then(Value::as_array) {
        if !list.is_empty() {
            l.field(0, "evidence", "");
            for ev in list {
                write_evidence(&mut l, ev);
            }
        }
    }
    if let Some(p) = get("proposed_by") {
        l.field(0, "proposed_by", &one_line_value(Some(p)));
    }
    if let Some(r) = get("ratification") {
        let n = |k: &str| text(r.get(k));
        l.field(
            0,
            "votes",
            &format!(
                "{} yes, {} no, {} eligible",
                n("yes"),
                n("no"),
                n("eligible")
            ),
        );
    }
    if let Some(map) = item.as_object() {
        for (k, v) in map {
            if k.starts_with("x-") {
                l.field(0, k, &x_value(v));
            }
        }
    }
    if has_detail(item) {
        let detail = get("detail").and_then(Value::as_str).unwrap_or_default();
        out.push(String::new());
        for dl in detail.trim_matches('\n').split('\n') {
            out.push(if dl.trim().is_empty() {
                String::new()
            } else {
                format!("  {dl}")
            });
        }
    }
}

fn write_evidence(l: &mut Lines<'_>, ev: &Value) {
    let k = ev.get("kind").and_then(Value::as_str).unwrap_or_default();
    let get = |key: &str| ev.get(key);
    let main = evidence_main(k).unwrap_or("text");
    l.field(2, k, &one_line_value(get(main)));
    match k {
        "code" => {
            if let Some(repo) = get("repo") {
                l.field(4, "repo", &one_line_value(Some(repo)));
            }
            match (get("line_start"), get("line_end")) {
                (Some(s), Some(e)) => {
                    l.field(4, "lines", &format!("{}-{}", text(Some(s)), text(Some(e))))
                }
                (Some(s), None) => l.field(4, "lines", &text(Some(s))),
                (None, Some(e)) => l.field(4, "line_end", &text(Some(e))),
                (None, None) => {}
            }
            for key in ["commit", "snippet"] {
                if let Some(v) = get(key) {
                    l.field(4, key, &one_line_value(Some(v)));
                }
            }
        }
        "quote" => {
            if let Some(from) = get("from") {
                if from.get("artifact").is_some() || from.get("item").is_some() {
                    l.field(4, "from", &ref_string(from));
                    if let Some(t) = from.get("title") {
                        l.field(6, "title", &one_line_value(Some(t)));
                    }
                } else {
                    l.field(4, "from", &one_line_value(from.get("title")));
                }
            }
            if let Some(loc) = get("location") {
                l.field(4, "location", &one_line_value(Some(loc)));
            }
        }
        "discussion" => {
            if let Some(s) = get("speaker") {
                l.field(4, "speaker", &one_line_value(Some(s)));
            }
        }
        "link" => {
            if let Some(t) = get("title") {
                l.field(4, "title", &one_line_value(Some(t)));
            }
        }
        _ => {}
    }
}

// ------------------------------------------------------------------ 10.7

struct Group<'a> {
    section: &'static str,
    source: &'a Value,
    items: Vec<&'a Value>,
}

fn version_of(v: Option<&Value>) -> Option<u64> {
    v.and_then(whole_number)
}

/// The material to copy from other artifacts, in the order of 10.7.
fn copied_material<'a>(
    doc: &Map<String, Value>,
    items: &[Value],
    inputs: &'a [Value],
) -> Vec<Group<'a>> {
    let find = |artifact: &Value, version: Option<u64>| {
        inputs.iter().find(|o| {
            o.get("id") == Some(artifact)
                && version.is_some()
                && version_of(o.get("version")) == version
        })
    };
    let accepted = |i: &&Value| i.get("status").and_then(Value::as_str) == Some("accepted");
    let mut groups = Vec::new();
    let mut seen: Vec<(String, u64, String)> = Vec::new();

    // 1 and 2: pinned glossaries and agent rules.
    for p in doc
        .get("parents")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if p.get("rel").and_then(Value::as_str) != Some("input") {
            continue;
        }
        let Some(artifact) = p.get("artifact") else {
            continue;
        };
        let version = version_of(p.get("version"));
        let Some(src) = find(artifact, version) else {
            continue;
        };
        let want = match src.get("kind").and_then(Value::as_str) {
            Some("glossary") => ItemType::Term,
            Some("agent_rules") => ItemType::Constraint,
            _ => continue,
        };
        let its: Vec<&Value> = src
            .get("items")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|i| item_type(i) == Some(want))
            .filter(accepted)
            .collect();
        if its.is_empty() {
            continue;
        }
        for i in &its {
            seen.push((
                text(Some(artifact)),
                version.unwrap_or(0),
                text(i.get("id")),
            ));
        }
        groups.push(Group {
            section: want.section(),
            source: src,
            items: its,
        });
    }

    // 3: items that refs point to, each one time only.
    let mut referred: Vec<(&Value, &Value)> = Vec::new();
    for item in items {
        if item.get("status").and_then(Value::as_str) == Some("rejected") {
            continue;
        }
        for r in item
            .get("refs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let (Some(artifact), Some(version), Some(id)) = (
                r.get("artifact"),
                version_of(r.get("version")),
                r.get("item"),
            ) else {
                continue;
            };
            let key = (text(Some(artifact)), version, text(Some(id)));
            if seen.contains(&key) {
                continue;
            }
            let Some(src) = find(artifact, Some(version)) else {
                continue;
            };
            let found = src
                .get("items")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .find(|i| i.get("id") == Some(id));
            if let Some(it) = found {
                seen.push(key);
                referred.push((src, it));
            }
        }
    }
    for t in ItemType::ALL {
        let mut per: Vec<Group<'a>> = Vec::new();
        for (src, it) in &referred {
            if item_type(it) != Some(t) {
                continue;
            }
            match per.iter_mut().find(|g| std::ptr::eq(g.source, *src)) {
                Some(g) => g.items.push(it),
                None => per.push(Group {
                    section: t.section(),
                    source: src,
                    items: vec![it],
                }),
            }
        }
        groups.extend(per);
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_line_joins_lines() {
        assert_eq!(one_line("a\n  b \n\nc"), "a b c");
        assert_eq!(one_line("  a  "), "a");
    }

    #[test]
    fn ref_strings() {
        let r = serde_json::json!({"artifact": "pa_7Hk2mQ9xR4tV8wZ1", "version": 2, "item": "FR1"});
        assert_eq!(ref_string(&r), "pa_7Hk2mQ9xR4tV8wZ1@2#FR1");
        assert_eq!(ref_string(&serde_json::json!({"item": "G1"})), "G1");
        let a = serde_json::json!({"artifact": "pa_7Hk2mQ9xR4tV8wZ1", "version": 2});
        assert_eq!(ref_string(&a), "pa_7Hk2mQ9xR4tV8wZ1@2");
    }

    #[test]
    fn x_values_keep_their_type() {
        assert_eq!(x_value(&serde_json::json!("BILL-77")), "BILL-77");
        assert_eq!(x_value(&serde_json::json!("12")), "\"12\"");
        assert_eq!(x_value(&serde_json::json!(12)), "12");
        assert_eq!(x_value(&serde_json::json!({"a": [1]})), "{\"a\":[1]}");
    }
}
