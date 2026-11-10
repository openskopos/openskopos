//! A YAML writer for JSON data.
//!
//! The writer uses block style, in the layout of the YAML example in the
//! conformance cases. It writes a string with no quotes only when every YAML
//! 1.1 and 1.2 reader reads it as the same string. So dates, times, numbers in
//! strings, and words such as `yes`, `no`, `on` and `null` get double quotes
//! (specification 2.2). A string with line breaks uses a literal block (`|`)
//! when that keeps every character, and double quotes when it does not.

use serde_json::{Map, Value};

use crate::order::{entries, Shape};

pub(crate) fn write(doc: &Value) -> String {
    let mut out = String::new();
    match doc {
        Value::Object(map) if !map.is_empty() => write_map(&mut out, map, Shape::Document, 0),
        Value::Array(list) if !list.is_empty() => write_seq(&mut out, list, Shape::Free, 0),
        Value::String(s) if literal_ok(s) => {
            out.push_str(&literal_header(s));
            write_literal_body(&mut out, s, 0);
        }
        other => {
            out.push_str(&scalar(other));
            out.push('\n');
        }
    }
    out
}

fn write_map(out: &mut String, map: &Map<String, Value>, shape: Shape, indent: usize) {
    for (key, value, child) in entries(map, shape) {
        push_indent(out, indent);
        out.push_str(&string(key));
        out.push(':');
        match value {
            Value::Object(m) if !m.is_empty() => {
                out.push('\n');
                write_map(out, m, child, indent + 2);
            }
            Value::Array(list) if !list.is_empty() => {
                out.push('\n');
                // A list under a key has the same indent as the key.
                write_seq(out, list, child, indent);
            }
            Value::String(s) if literal_ok(s) => {
                out.push(' ');
                out.push_str(&literal_header(s));
                write_literal_body(out, s, indent + 2);
            }
            other => {
                out.push(' ');
                out.push_str(&scalar(other));
                out.push('\n');
            }
        }
    }
}

fn write_seq(out: &mut String, list: &[Value], shape: Shape, indent: usize) {
    for value in list {
        push_indent(out, indent);
        out.push('-');
        match value {
            Value::Object(m) if !m.is_empty() => {
                // Write the object as if it were indented under the dash, and
                // then put its first line on the line of the dash.
                let mut nested = String::new();
                write_map(&mut nested, m, shape, indent + 2);
                out.push(' ');
                out.push_str(&nested[indent + 2..]);
            }
            Value::Array(l) if !l.is_empty() => {
                let mut nested = String::new();
                write_seq(&mut nested, l, shape, indent + 2);
                out.push(' ');
                out.push_str(&nested[indent + 2..]);
            }
            Value::String(s) if literal_ok(s) => {
                out.push(' ');
                out.push_str(&literal_header(s));
                write_literal_body(out, s, indent + 2);
            }
            other => {
                out.push(' ');
                out.push_str(&scalar(other));
                out.push('\n');
            }
        }
    }
}

fn push_indent(out: &mut String, indent: usize) {
    out.extend(std::iter::repeat_n(' ', indent));
}

/// A scalar, or an empty list or object, on one line.
fn scalar(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => string(s),
        Value::Array(_) => "[]".to_string(),
        Value::Object(_) => "{}".to_string(),
    }
}

/// A string on one line: plain if that is safe, else in double quotes.
fn string(s: &str) -> String {
    if plain_ok(s) {
        s.to_string()
    } else {
        double_quoted(s)
    }
}

/// True if every YAML 1.1 and 1.2 reader reads `s` with no quotes as the same
/// string. The test is strict: some safe strings get quotes too.
fn plain_ok(s: &str) -> bool {
    let Some(first) = s.chars().next() else {
        return false;
    };
    let last = s.chars().next_back().unwrap_or(first);
    if first.is_ascii_digit() || "-?:,[]{}#&*!|>'\"%@`+.<=~ ".contains(first) {
        return false;
    }
    if last == ' ' || last == ':' {
        return false;
    }
    if s.contains(": ") || s.contains(" #") {
        return false;
    }
    if !s
        .chars()
        .all(|c| c != '\t' && c != '\n' && c != '\u{85}' && printable(c) && !is_bom_or_separator(c))
    {
        return false;
    }
    const WORDS: [&str; 10] = [
        "null", "~", "true", "false", "yes", "no", "on", "off", "y", "n",
    ];
    !WORDS.iter().any(|w| s.eq_ignore_ascii_case(w))
}

/// The printable characters of YAML 1.1, as libyaml checks them.
fn printable(c: char) -> bool {
    matches!(c as u32, 0x0A | 0x20..=0x7E | 0x85 | 0xA0..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF)
        && c != '\u{FEFF}'
}

/// Characters that YAML 1.1 reads as a byte order mark or a line break.
fn is_bom_or_separator(c: char) -> bool {
    matches!(c, '\u{FEFF}' | '\u{2028}' | '\u{2029}')
}

fn double_quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c != '\n' && c != '\u{85}' && printable(c) && !is_bom_or_separator(c) => {
                out.push(c)
            }
            c if (c as u32) <= 0xFF => out.push_str(&format!("\\x{:02X}", c as u32)),
            c if (c as u32) <= 0xFFFF => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push_str(&format!("\\U{:08X}", c as u32)),
        }
    }
    out.push('"');
    out
}

/// True if a literal block keeps every character of `s`.
fn literal_ok(s: &str) -> bool {
    if !s.contains('\n') {
        return false;
    }
    let body = s.trim_end_matches('\n');
    // The reader finds the indent from the first line, so the first line must
    // have text that does not start with a space.
    let Some(first) = body.chars().next() else {
        return false;
    };
    if first == ' ' || first == '\t' || first == '\n' {
        return false;
    }
    if !body
        .chars()
        .all(|c| c == '\t' || (c != '\u{85}' && printable(c) && !is_bom_or_separator(c)))
    {
        return false;
    }
    // A line with only spaces or tabs can be read as an empty line.
    body.split('\n')
        .all(|line| line.is_empty() || !line.chars().all(|c| c == ' ' || c == '\t'))
}

/// `|-`, `|` or `|+`, for no, one or more line breaks at the end.
fn literal_header(s: &str) -> String {
    let breaks = s.len() - s.trim_end_matches('\n').len();
    match breaks {
        0 => "|-\n".to_string(),
        1 => "|\n".to_string(),
        _ => "|+\n".to_string(),
    }
}

fn write_literal_body(out: &mut String, s: &str, indent: usize) {
    let body = s.trim_end_matches('\n');
    let breaks = s.len() - body.len();
    for line in body.split('\n') {
        if !line.is_empty() {
            push_indent(out, indent);
            out.push_str(line);
        }
        out.push('\n');
    }
    for _ in 1..breaks {
        out.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::{parse, to_yaml, Format};

    fn round_trip(value: &Value) {
        let text = to_yaml(value);
        let back = parse(&text, Format::Yaml).unwrap_or_else(|e| panic!("{e}\n{text}"));
        assert_eq!(&back, value, "YAML:\n{text}");
    }

    const STRINGS: &[&str] = &[
        "",
        " ",
        "a",
        "plain text",
        "yes",
        "No",
        "ON",
        "off",
        "y",
        "N",
        "null",
        "Null",
        "~",
        "true",
        "False",
        "6001",
        "007",
        "1.5",
        "1e3",
        ".5",
        "-1",
        "+1",
        "0x1F",
        "0o17",
        "1_000",
        "1:20",
        ".inf",
        "-.inf",
        ".NaN",
        "2026-11-04",
        "2026-11-04T10:30:00Z",
        "2026-11-04 10:30:00",
        "- item",
        "-item",
        "? key",
        ": value",
        "key: value",
        "a #b",
        "a#b",
        "#comment",
        "&anchor",
        "*alias",
        "!tag",
        "%directive",
        "@at",
        "`tick`",
        "|",
        ">",
        "'single'",
        "\"double\"",
        "[list]",
        "{map}",
        "a, b",
        "ends with colon:",
        "trailing ",
        " leading",
        "tab\there",
        "back\\slash",
        "<<",
        "=",
        "x-key",
        "$schema",
        "pa_7Hk2mQ9xR4tV8wZ1@2#CR3",
        "https://example.com/a?b=c#d",
        "Zoë Ångström",
        "日本語",
        "emoji 🎉",
        "nbsp\u{a0}here",
        "nel\u{85}here",
        "ls\u{2028}here",
        "ps\u{2029}here",
        "bom\u{feff}here",
        "bell\u{7}",
        "del\u{7f}",
        "cr\rhere",
        "a\nb",
        "a\nb\n",
        "a\nb\n\n",
        "a\n\nb",
        "\nleading break",
        "  indented\nsecond",
        "first\n  indented",
        "first\n   \nlast",
        "first\n\t\nlast",
        "trailing spaces  \nnext",
        "last line spaces\n  ",
        "a\n---\nb",
        "a\n...\nb",
        "a\n# not a comment",
        "\n",
        "\n\n",
        "x\r\ny",
        "tab\n\tline",
        "- [x] done\n- [ ] todo\n",
        "Markdown:\n\n- one\n  - nested\n\n```\ncode\n```\n",
    ];

    #[test]
    fn strings_survive_a_round_trip() {
        for s in STRINGS {
            let keyed =
                |k: &str, v: Value| Value::Object([(k.to_string(), v)].into_iter().collect());
            round_trip(&json!({ "v": s }));
            round_trip(&json!({ "list": [s, [s], keyed(s, json!(s))] }));
            round_trip(&keyed(s, json!({ "nested": s })));
        }
    }

    #[test]
    fn other_values_survive_a_round_trip() {
        round_trip(&json!({
            "null": null, "t": true, "f": false, "int": 42, "neg": -7, "big": u64::MAX,
            "float": 1.5, "whole_float": 2.0, "small": 1e-7, "large": 1e300,
            "empty_list": [], "empty_map": {}, "list_of_lists": [[1, 2], [], [[3]]],
            "list_of_maps": [{"a": 1, "b": [{"c": {}}]}, {}],
        }));
        round_trip(&json!([1, "two", {"three": 3}]));
        round_trip(&json!("just a string"));
        round_trip(&json!(null));
    }

    #[test]
    fn quotes_dates_and_yes_and_no() {
        let text = to_yaml(&json!({
            "created_at": "2026-11-04T10:30:00Z",
            "session": {"id": "6001"},
            "items": [{"ratification": {"yes": 2, "no": 0, "eligible": 2}}],
        }));
        assert!(
            text.contains("created_at: \"2026-11-04T10:30:00Z\"\n"),
            "{text}"
        );
        assert!(text.contains("id: \"6001\"\n"), "{text}");
        assert!(text.contains("\"yes\": 2\n"), "{text}");
        assert!(text.contains("\"no\": 0\n"), "{text}");
    }

    #[test]
    fn writes_multi_line_text_as_a_literal_block() {
        let text = to_yaml(&json!({"summary": "One.\n\nTwo."}));
        assert_eq!(text, "summary: |-\n  One.\n\n  Two.\n");
    }
}
