//! The HTML and terminal views.
//!
//! The golden files are in `tests/golden/render/`. To write them again after
//! a deliberate change, run the tests with `UPDATE_GOLDEN=1` and review the
//! change in git.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use skopos_core::render::{html, terminal, HtmlOptions, TerminalOptions, STYLESHEET};
use skopos_core::{parse, Format};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn read(path: &str) -> Value {
    let text = fs::read_to_string(root().join(path)).unwrap();
    parse(&text, Format::Json).unwrap()
}

fn golden(name: &str, actual: &str) {
    let path = root().join("tests/golden/render").join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        fs::write(&path, actual).unwrap();
        return;
    }
    let expected = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}. Run with UPDATE_GOLDEN=1.", path.display()));
    assert_eq!(actual, expected, "{} is different", path.display());
}

fn v2() -> Value {
    read("tests/fixtures/spec-v2.json")
}

fn v3() -> Value {
    read("../../conformance/valid/spec-all-types-v3.json")
}

#[test]
fn golden_views() {
    let (old, new) = (v2(), v3());
    let fragment = |previous| HtmlOptions {
        previous,
        fragment: true,
        ..Default::default()
    };
    golden("spec-v3.html", &html(&new, &fragment(None)));
    golden("spec-v3-since-v2.html", &html(&new, &fragment(Some(&old))));
    golden("spec-v3.txt", &terminal(&new, &TerminalOptions::default()));
    let since = TerminalOptions {
        previous: Some(&old),
        ..Default::default()
    };
    golden("spec-v3-since-v2.txt", &terminal(&new, &since));
}

fn doc(items: Value) -> Value {
    json!({
        "id": "pa_7Hk2mQ9xR4tV8wZ1", "version": 2, "kind": "intent",
        "title": "T <b>", "status": "draft", "items": items,
    })
}

#[test]
fn html_escapes_text_and_links_only_web_urls() {
    let d = doc(json!([{
        "id": "G1", "type": "goal", "status": "accepted",
        "text": "<script>alert(1)</script> & \"q\"",
        "evidence": [
            {"kind": "link", "uri": "javascript:alert(1)", "title": "bad"},
            {"kind": "link", "uri": "https://example.com/?a=1&b=2", "title": "good"},
        ],
    }]));
    let page = html(&d, &HtmlOptions::default());
    assert!(!page.contains("<script>"));
    assert!(page.contains("&lt;script&gt;alert(1)&lt;/script&gt; &amp; &quot;q&quot;"));
    assert!(page.contains("<title>T &lt;b&gt;</title>"));
    assert!(!page.contains("javascript:"));
    assert!(page.contains("<a href=\"https://example.com/?a=1&amp;b=2\">good</a>"));
    // CSS cannot end the style element early.
    let css = HtmlOptions {
        css: Some("a{}</style><script>x</script>"),
        ..Default::default()
    };
    assert!(!html(&d, &css).contains("</style><script>"));
}

#[test]
fn html_page_fragment_and_style_options() {
    let d = doc(json!([{"id": "FR1", "type": "requirement", "text": "x", "status": "proposed"}]));
    let page = html(&d, &HtmlOptions::default());
    assert!(page.starts_with("<!doctype html>"));
    assert!(page.contains(STYLESHEET));
    let bare = html(
        &d,
        &HtmlOptions {
            style: false,
            css: Some(".sk-doc{--sk-accent:red}"),
            ..Default::default()
        },
    );
    assert!(!bare.contains("--sk-ink"));
    assert!(bare.contains(".sk-doc{--sk-accent:red}"));
    let frag = html(
        &d,
        &HtmlOptions {
            fragment: true,
            ..Default::default()
        },
    );
    assert!(frag.starts_with("<article class=\"sk-doc\""));
    assert!(!frag.contains("<style") && !frag.contains("<html"));
    assert!(frag.contains(
        "<li class=\"sk-item sk-item--proposed\" id=\"FR1\" data-id=\"FR1\" data-type=\"requirement\" data-status=\"proposed\">"
    ));
}

#[test]
fn every_custom_property_has_a_default() {
    let used: std::collections::BTreeSet<&str> = STYLESHEET
        .match_indices("var(--sk-")
        .map(|(i, _)| {
            let rest = &STYLESHEET[i + 4..];
            &rest[..rest.find(')').unwrap()]
        })
        .collect();
    for name in used {
        assert!(
            STYLESHEET.contains(&format!("{name}:")),
            "{name} is used but has no default"
        );
    }
}

#[test]
fn superseded_items_link_to_the_new_item() {
    let d = doc(json!([
        {"id": "FR1", "type": "requirement", "text": "old", "status": "accepted"},
        {"id": "FR2", "type": "requirement", "text": "new", "status": "accepted",
         "refs": [{"item": "FR1", "rel": "supersedes"}]},
        {"id": "FR3", "type": "requirement", "text": "no", "status": "rejected",
         "refs": [{"item": "FR2", "rel": "supersedes"}]},
    ]));
    let page = html(
        &d,
        &HtmlOptions {
            fragment: true,
            ..Default::default()
        },
    );
    assert!(page.contains("sk-item sk-item--accepted sk-item--superseded\" id=\"FR1\""));
    // A rejected item supersedes nothing.
    assert!(page.contains("<li class=\"sk-item sk-item--accepted\" id=\"FR2\""));
    assert!(!page.contains("superseded by <a class=\"sk-ref\" href=\"#FR1\">"));
    assert!(page.contains("superseded by <a class=\"sk-ref\" href=\"#FR2\">FR2</a>"));
}

#[test]
fn terminal_plain_text_has_no_escape_codes_and_color_has() {
    let (old, new) = (v2(), v3());
    let plain = terminal(
        &new,
        &TerminalOptions {
            previous: Some(&old),
            ..Default::default()
        },
    );
    assert!(!plain.contains('\x1b'));
    assert!(plain.contains("Seat counts are [-exact.-] {+exact at the time of the plan change.+}"));
    let color = terminal(
        &new,
        &TerminalOptions {
            previous: Some(&old),
            color: true,
            width: 100,
        },
    );
    assert!(color.contains("\x1b[31;9mexact.\x1b[0m"));
    assert!(!color.contains("[-exact.-]"));
}

#[test]
fn terminal_wraps_at_the_width() {
    let new = v3();
    for width in [40, 60, 100] {
        let out = terminal(
            &new,
            &TerminalOptions {
                width,
                ..Default::default()
            },
        );
        for line in out.lines() {
            // A single word longer than the width can not wrap.
            if line
                .split_whitespace()
                .all(|w| w.chars().count() < width / 2)
            {
                assert!(line.chars().count() <= width, "{width}: {line:?}");
            }
        }
    }
}

#[test]
fn renders_values_that_are_not_documents() {
    for v in [json!(null), json!([]), json!({"items": [1, {"text": 2}]})] {
        assert!(html(&v, &HtmlOptions::default()).contains("<article"));
        assert!(!terminal(&v, &TerminalOptions::default()).is_empty());
    }
}
