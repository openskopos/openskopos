//! Search an archive (prototype for the 0.3 compaction plan, not released).
//!
//! An archive is JSON Lines compressed with Zstandard: one document per line.
//! [`search_archive`] reads it as a stream, so memory stays at about one
//! document plus the hits, whatever the size of the archive.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};

/// One item that matches, once per item, from the newest version that matches.
#[derive(Debug, Serialize)]
pub struct Hit {
    pub reference: String,
    pub artifact: String,
    pub item: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub status: String,
    pub version: u64,
    pub versions_matched: u64,
    pub text: String,
}

#[derive(Debug, Default, Serialize)]
pub struct SearchResult {
    pub documents: u64,
    pub documents_parsed: u64,
    pub items_scanned: u64,
    pub hits: Vec<Hit>,
}

#[derive(Deserialize)]
struct Doc {
    id: String,
    version: u64,
    #[serde(default)]
    items: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    id: String,
    #[serde(rename = "type")]
    item_type: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    text: String,
    #[serde(flatten)]
    rest: HashMap<String, serde_json::Value>,
}

const FIELDS: [&str; 6] = [
    "rationale",
    "detail",
    "definition",
    "mitigation",
    "reason",
    "answer",
];

fn contains(hay: &str, needle_lower: &str) -> bool {
    hay.to_lowercase().contains(needle_lower)
}

/// Search the items of every document in a `.skopos.jsonl.zst` archive for a
/// word or phrase, ignoring case. Each item appears once, from its newest version.
pub fn search_archive<R: Read>(archive: R, query: &str) -> Result<SearchResult, String> {
    let decoder = ruzstd::decoding::StreamingDecoder::new(archive).map_err(|e| e.to_string())?;
    search_lines(BufReader::with_capacity(1 << 20, decoder), query)
}

/// The same search on plain JSON Lines.
pub fn search_lines<R: BufRead>(lines: R, query: &str) -> Result<SearchResult, String> {
    let q = query.to_lowercase();
    // A line that does not contain the query cannot have a matching item, unless
    // JSON escaping changed the query's characters. Skip parsing those lines.
    let prefilter = !q.chars().any(|c| c == '"' || c == '\\' || c.is_control());
    let mut result = SearchResult::default();
    let mut found: HashMap<(String, String), Hit> = HashMap::new();
    for line in lines.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.is_empty() {
            continue;
        }
        result.documents += 1;
        if prefilter && !contains(&line, &q) {
            continue;
        }
        result.documents_parsed += 1;
        let doc: Doc = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        for it in doc.items {
            result.items_scanned += 1;
            let hit = contains(&it.text, &q)
                || FIELDS.iter().any(|f| matches!(it.rest.get(*f), Some(serde_json::Value::String(s)) if contains(s, &q)));
            if !hit {
                continue;
            }
            let key = (doc.id.clone(), it.id.clone());
            let e = found.entry(key).or_insert_with(|| Hit {
                reference: String::new(),
                artifact: doc.id.clone(),
                item: it.id.clone(),
                item_type: String::new(),
                status: String::new(),
                version: 0,
                versions_matched: 0,
                text: String::new(),
            });
            e.versions_matched += 1;
            if doc.version >= e.version {
                e.version = doc.version;
                e.reference = format!("{}@{}#{}", doc.id, doc.version, it.id);
                e.item_type = it.item_type;
                e.status = it.status;
                e.text = it.text;
            }
        }
    }
    let mut hits: Vec<Hit> = found.into_values().collect();
    hits.sort_by(|a, b| (&a.artifact, &a.item).cmp(&(&b.artifact, &b.item)));
    result.hits = hits;
    Ok(result)
}
