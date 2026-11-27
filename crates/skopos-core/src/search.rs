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

/// The largest Zstandard window an archive may use, so that any reader, a
/// browser included, decodes it with little memory.
pub const MAX_WINDOW: u64 = 8 << 20;

/// Decodes concatenated Zstandard frames (RFC 8878 allows several in a file),
/// and refuses a frame whose window is larger than [`MAX_WINDOW`].
struct Frames<R: BufRead> {
    decoder: Option<ruzstd::decoding::StreamingDecoder<R, ruzstd::decoding::FrameDecoder>>,
}

impl<R: BufRead> Frames<R> {
    fn start(
        mut source: R,
    ) -> std::io::Result<
        Option<ruzstd::decoding::StreamingDecoder<R, ruzstd::decoding::FrameDecoder>>,
    > {
        let head = source.fill_buf()?;
        if head.is_empty() {
            return Ok(None);
        }
        let window = window_size(head)
            .ok_or_else(|| std::io::Error::other("the archive is not a Zstandard frame"))?;
        if window > MAX_WINDOW {
            return Err(std::io::Error::other(format!(
                "the archive uses a Zstandard window of {window} bytes; the limit is {MAX_WINDOW}"
            )));
        }
        let d = ruzstd::decoding::StreamingDecoder::new(source).map_err(std::io::Error::other)?;
        Ok(Some(d))
    }
}

/// The window size of a Zstandard frame, from its header (RFC 8878, 3.1.1.1).
/// A single-segment frame needs its whole content as the window.
fn window_size(h: &[u8]) -> Option<u64> {
    if h.len() < 6 || h[..4] != [0x28, 0xB5, 0x2F, 0xFD] {
        return None;
    }
    let fhd = h[4];
    if fhd & 0x20 == 0 {
        let wd = h[5];
        let log = 10 + u32::from(wd >> 3);
        let base = 1u64 << log;
        return Some(base + (base / 8) * u64::from(wd & 7));
    }
    let dict = [0, 1, 2, 4][usize::from(fhd & 3)];
    let at = 5 + dict;
    let fcs = match fhd >> 6 {
        0 => u64::from(*h.get(at)?),
        1 => u64::from(u16::from_le_bytes(h.get(at..at + 2)?.try_into().ok()?)) + 256,
        2 => u64::from(u32::from_le_bytes(h.get(at..at + 4)?.try_into().ok()?)),
        _ => u64::from_le_bytes(h.get(at..at + 8)?.try_into().ok()?),
    };
    Some(fcs)
}

impl<R: BufRead> Read for Frames<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        while let Some(d) = self.decoder.as_mut() {
            let n = d.read(buf)?;
            if n > 0 || buf.is_empty() {
                return Ok(n);
            }
            let source = self.decoder.take().expect("a decoder").into_inner();
            self.decoder = Self::start(source)?;
        }
        Ok(0)
    }
}

/// Search the items of every document in a `.skopos.jsonl.zst` archive for a
/// word or phrase, ignoring case. Each item appears once, from the newest
/// version that matches. The search reads the text of each archived version;
/// it does not say whether a later version changed the item.
pub fn search_archive<R: Read>(archive: R, query: &str) -> Result<SearchResult, String> {
    let frames = Frames {
        decoder: Frames::start(BufReader::new(archive)).map_err(|e| e.to_string())?,
    };
    search_lines(BufReader::with_capacity(1 << 20, frames), query)
}

/// The same search on plain JSON Lines.
pub fn search_lines<R: BufRead>(lines: R, query: &str) -> Result<SearchResult, String> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Err("the query is empty".into());
    }
    // A line that does not contain the query cannot have a matching item, unless
    // JSON escaping changed the query's characters, as it can for quotes,
    // backslashes, control characters and any non-ASCII character (\u00e9).
    // Skip parsing those lines only when no escaping is possible.
    let prefilter = q
        .chars()
        .all(|c| c.is_ascii() && c != '"' && c != '\\' && !c.is_control());
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
