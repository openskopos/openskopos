//! Search current documents and compressed archives (prototype for the 0.3
//! compaction plan, not released).
//!
//! An archive is JSON Lines compressed with Zstandard (`.skopos.jsonl.zst`):
//! one document per line, across one or more Zstandard frames.
//! [`Search`] accumulates both current documents ([`Search::add_doc`] /
//! [`Search::add_docs`]) and archived documents ([`Search::add_archive`] /
//! [`Search::add_lines`]) as a stream, keeping memory proportional to the
//! distinct items seen rather than the full documents.
//!
//! Each matching item appears once in [`SearchResult::hits`], tagged with its
//! [`Source`] (`current` or `archive`), its latest known version and status,
//! and whether the query matched only an earlier version (`matched_version`).

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Maximum Zstandard window size accepted when streaming an archive (8 MiB,
/// matching the 0.3 compaction plan). Larger windows (such as `--long=27`)
/// can exhaust WASM memory.
pub const MAX_WINDOW_SIZE: u64 = 8 * 1024 * 1024;

/// Where the latest version of a matching item lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// At least one current document holds the item's newest known version.
    Current,
    /// The item's newest known version was seen only in an archive.
    Archive,
}

/// One item that matched the query, returned once per `(artifact, item)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hit {
    /// Reference string of the item's newest known version (`<artifact>@<version>#<item>`).
    pub reference: String,
    pub artifact: String,
    pub item: String,
    #[serde(rename = "type")]
    pub item_type: String,
    /// Status of the item in its newest known version.
    pub status: String,
    /// Newest known version of the item across current docs and archives.
    pub version: u64,
    /// Where the newest known version lives (`current` or `archive`).
    pub source: Source,
    /// Newest version in which the query actually matched. When this is less
    /// than `version` (or `matched_in_latest` is false), the query matched an
    /// older version of the item before it was rewritten or removed.
    pub matched_version: u64,
    /// Where `matched_version` lives (`current` or `archive`).
    pub matched_source: Source,
    /// True if the query matches the item's newest known version (and the
    /// item was not removed in a newer version of its artifact).
    pub matched_in_latest: bool,
    /// Number of distinct versions in which the query matched.
    pub versions_matched: u64,
    /// Text of the item in its newest known version (or in `matched_version`
    /// if the item was removed in a newer version of its artifact).
    pub text: String,
    /// When the item was removed in a newer version of its artifact, that
    /// version number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub removed_in_version: Option<u64>,
}

/// The result of searching current documents and/or archives.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResult {
    pub documents: u64,
    pub documents_parsed: u64,
    pub items_scanned: u64,
    pub hits: Vec<Hit>,
}

#[derive(Deserialize)]
struct Doc {
    #[serde(default)]
    id: String,
    #[serde(default)]
    version: u64,
    #[serde(default)]
    items: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    #[serde(default)]
    id: String,
    #[serde(default, rename = "type")]
    item_type: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    text: String,
    #[serde(flatten)]
    rest: HashMap<String, Value>,
}

/// String fields on an item searched in addition to `text`.
const STRING_FIELDS: [&str; 8] = [
    "rationale",
    "detail",
    "definition",
    "mitigation",
    "reason",
    "answer",
    "suggestion",
    "verification",
];

/// String-array fields on an item searched in addition to `text`.
const LIST_FIELDS: [&str; 5] = [
    "aliases",
    "code_names",
    "alternatives",
    "consequences",
    "acceptance_criteria",
];

fn contains(hay: &str, needle_lower: &str) -> bool {
    hay.to_lowercase().contains(needle_lower)
}

fn item_matches(it: &Item, q: &str) -> bool {
    if contains(&it.text, q) {
        return true;
    }
    for f in STRING_FIELDS {
        if matches!(it.rest.get(f), Some(Value::String(s)) if contains(s, q)) {
            return true;
        }
    }
    for f in LIST_FIELDS {
        if let Some(Value::Array(list)) = it.rest.get(f) {
            if list
                .iter()
                .any(|v| matches!(v, Value::String(s) if contains(s, q)))
            {
                return true;
            }
        }
    }
    if let Some(Value::Object(res)) = it.rest.get("resolution") {
        if matches!(res.get("note"), Some(Value::String(s)) if contains(s, q)) {
            return true;
        }
    }
    if let Some(Value::Array(ev)) = it.rest.get("evidence") {
        for e in ev {
            if let Some(obj) = e.as_object() {
                if matches!(obj.get("text"), Some(Value::String(s)) if contains(s, q)) {
                    return true;
                }
                if matches!(obj.get("snippet"), Some(Value::String(s)) if contains(s, q)) {
                    return true;
                }
            }
        }
    }
    false
}

/// Newest metadata seen for a single `(artifact, item)` across all versions.
struct ItemState {
    latest_version: u64,
    latest_source: Source,
    item_type: String,
    status: String,
    text: String,
    /// Newest version where the query matched, if any.
    matched_version: Option<u64>,
    matched_source: Option<Source>,
    /// Distinct versions where the query matched.
    matched_versions: Vec<u64>,
}

/// Streaming search over current documents and `.skopos.jsonl.zst` archives.
///
/// Feed current documents with [`Self::add_doc`] or [`Self::add_docs`], and
/// archives with [`Self::add_archive`] or [`Self::add_lines`], in any order.
/// Call [`Self::finish`] to obtain deduplicated [`SearchResult`] hits that
/// distinguish `current` from `archive` and reflect each item's latest version.
pub struct Search {
    query: String,
    prefilter: bool,
    documents: u64,
    documents_parsed: u64,
    items_scanned: u64,
    /// Highest version seen for each artifact (and where it lives), used to
    /// detect items removed in a newer version of their artifact.
    artifact_latest: HashMap<String, (u64, Source)>,
    /// Artifacts that currently have at least one matched item, for O(1)
    /// lookup in the streaming prefilter path.
    matched_artifacts: std::collections::HashSet<String>,
    items: HashMap<(String, String), ItemState>,
}

impl Search {
    /// Starts a new search for `query` (case-insensitive substring match).
    pub fn new(query: &str) -> Self {
        let q = query.trim().to_lowercase();
        // Raw JSON-line prefilter is safe only when the query is ASCII and
        // contains no characters that JSON escapes (`"`, `\`, control chars).
        // Non-ASCII queries disable the prefilter so `\uXXXX` escapes match.
        let prefilter = !q.is_empty()
            && q.is_ascii()
            && !q.chars().any(|c| c == '"' || c == '\\' || c.is_control());
        Self {
            query: q,
            prefilter,
            documents: 0,
            documents_parsed: 0,
            items_scanned: 0,
            artifact_latest: HashMap::new(),
            matched_artifacts: std::collections::HashSet::new(),
            items: HashMap::new(),
        }
    }

    /// Adds a single current (non-archived) document.
    pub fn add_doc(&mut self, doc: &Value) -> Result<(), String> {
        if self.query.is_empty() {
            return Ok(());
        }
        let parsed: Doc = serde_json::from_value(doc.clone()).map_err(|e| e.to_string())?;
        self.documents += 1;
        self.documents_parsed += 1;
        self.ingest_doc(parsed, Source::Current);
        Ok(())
    }

    /// Adds a slice of current (non-archived) documents.
    pub fn add_docs(&mut self, docs: &[Value]) -> Result<(), String> {
        for doc in docs {
            self.add_doc(doc)?;
        }
        Ok(())
    }

    /// Streams a `.skopos.jsonl.zst` archive (one or more Zstandard frames).
    /// Rejects any frame whose window size exceeds [`MAX_WINDOW_SIZE`].
    pub fn add_archive<R: Read>(&mut self, archive: R) -> Result<(), String> {
        if self.query.is_empty() {
            return Ok(());
        }
        let reader = MultiFrameZstd::new(archive, MAX_WINDOW_SIZE);
        self.add_lines(BufReader::with_capacity(1 << 16, reader))
    }

    /// Streams plain JSON Lines as archived documents.
    pub fn add_lines<R: BufRead>(&mut self, lines: R) -> Result<(), String> {
        if self.query.is_empty() {
            return Ok(());
        }
        for line in lines.lines() {
            let line = line.map_err(|e| e.to_string())?;
            if line.is_empty() {
                continue;
            }
            self.documents += 1;
            // Track artifact version and item updates even when a newer
            // version no longer contains the query word, if we already have a
            // matching item in this artifact.
            let line_has_query = !self.prefilter || contains(&line, &self.query);
            if !line_has_query {
                // Fast skip when this artifact has no matched items; still
                // update `artifact_latest` so removal detection stays accurate
                // regardless of line order.
                if let Some((art, ver)) = peek_id_and_version(&line) {
                    self.record_artifact_version(&art, ver, Source::Archive);
                    if !self.matched_artifacts.contains(&art) {
                        continue;
                    }
                } else {
                    continue;
                }
            }
            self.documents_parsed += 1;
            let doc: Doc = serde_json::from_str(&line).map_err(|e| e.to_string())?;
            self.ingest_doc(doc, Source::Archive);
        }
        Ok(())
    }

    fn record_artifact_version(&mut self, artifact: &str, version: u64, source: Source) {
        if artifact.is_empty() || version == 0 {
            return;
        }
        let entry = self
            .artifact_latest
            .entry(artifact.to_string())
            .or_insert((version, source));
        if version > entry.0 || (version == entry.0 && source == Source::Current) {
            *entry = (version, source);
        }
    }

    fn ingest_doc(&mut self, doc: Doc, source: Source) {
        if doc.id.is_empty() || doc.version == 0 {
            return;
        }
        self.record_artifact_version(&doc.id, doc.version, source);
        for it in doc.items {
            if it.id.is_empty() {
                continue;
            }
            self.items_scanned += 1;
            let matched = item_matches(&it, &self.query);
            let key = (doc.id.clone(), it.id.clone());
            // Track every item of current documents (small set) so later
            // archived versions that match an old word still know the item's
            // current text and status; for archives, only allocate on a match
            // or an already-tracked item.
            if source == Source::Archive && !matched && !self.items.contains_key(&key) {
                continue;
            }
            let entry = self.items.entry(key).or_insert_with(|| ItemState {
                latest_version: 0,
                latest_source: source,
                item_type: String::new(),
                status: String::new(),
                text: String::new(),
                matched_version: None,
                matched_source: None,
                matched_versions: Vec::new(),
            });
            // Update latest known version/state (preferring Current on ties).
            if doc.version > entry.latest_version
                || (doc.version == entry.latest_version && source == Source::Current)
            {
                entry.latest_version = doc.version;
                entry.latest_source = source;
                entry.item_type = it.item_type;
                entry.status = it.status;
                entry.text = it.text;
            }
            if matched {
                self.matched_artifacts.insert(doc.id.clone());
                if !entry.matched_versions.contains(&doc.version) {
                    entry.matched_versions.push(doc.version);
                }
                let newer_match = entry.matched_version.is_none_or(|mv| {
                    doc.version > mv || (doc.version == mv && source == Source::Current)
                });
                if newer_match {
                    entry.matched_version = Some(doc.version);
                    entry.matched_source = Some(source);
                }
            }
        }
    }

    /// Consumes the search accumulator and returns sorted hits.
    pub fn finish(self) -> SearchResult {
        let mut hits: Vec<Hit> = self
            .items
            .into_iter()
            .filter_map(|((artifact, item), st)| {
                let matched_version = st.matched_version?;
                let matched_source = st.matched_source?;
                let (art_latest_v, art_latest_src) = self
                    .artifact_latest
                    .get(&artifact)
                    .copied()
                    .unwrap_or((st.latest_version, st.latest_source));
                let removed_in_version = (art_latest_v > st.latest_version).then_some(art_latest_v);
                let matched_in_latest =
                    removed_in_version.is_none() && matched_version == st.latest_version;
                let source = if removed_in_version.is_some() {
                    art_latest_src
                } else {
                    st.latest_source
                };
                Some(Hit {
                    reference: format!("{artifact}@{}#{item}", st.latest_version),
                    artifact,
                    item,
                    item_type: st.item_type,
                    status: st.status,
                    version: st.latest_version,
                    source,
                    matched_version,
                    matched_source,
                    matched_in_latest,
                    versions_matched: st.matched_versions.len() as u64,
                    text: st.text,
                    removed_in_version,
                })
            })
            .collect();
        // Current hits first, then archive hits; within each group sort by
        // artifact and item ID.
        hits.sort_by(|a, b| {
            let src_rank = |s: Source| match s {
                Source::Current => 0u8,
                Source::Archive => 1u8,
            };
            (src_rank(a.source), &a.artifact, &a.item).cmp(&(
                src_rank(b.source),
                &b.artifact,
                &b.item,
            ))
        });
        SearchResult {
            documents: self.documents,
            documents_parsed: self.documents_parsed,
            items_scanned: self.items_scanned,
            hits,
        }
    }
}

/// Quick scan of a JSON line for top-level `"id":"..."` and `"version":N`
/// without deserializing the entire `items` array. Used only when the
/// prefilter did not match the query so we can still keep `artifact_latest`
/// accurate.
fn peek_id_and_version(line: &str) -> Option<(String, u64)> {
    // Only scan before `"items"` when `"items"` appears after the header, so
    // nested item `"id"` or parent `"version"` fields cannot shadow the
    // top-level document fields.
    let head = line.split("\"items\"").next().unwrap_or(line);
    // Avoid matching nested `"parents"` / `"method"` / `"session"` objects by
    // tracking brace depth.
    let mut id: Option<String> = None;
    let mut version: Option<u64> = None;
    let bytes = head.as_bytes();
    let mut i = 0usize;
    let mut depth = 0i32;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                depth += 1;
                i += 1;
            }
            b'}' => {
                depth -= 1;
                i += 1;
            }
            b'"' => {
                let start = i + 1;
                i = skip_json_string(bytes, i)?;
                let key = &head[start..i - 1];
                if depth == 1 && (key == "id" || key == "version") {
                    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                        i += 1;
                    }
                    if i < bytes.len() && bytes[i] == b':' {
                        i += 1;
                        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                            i += 1;
                        }
                        if key == "id" && i < bytes.len() && bytes[i] == b'"' {
                            let vs = i + 1;
                            i = skip_json_string(bytes, i)?;
                            let val = &head[vs..i - 1];
                            if !val.contains('\\') && !val.is_empty() {
                                id = Some(val.to_string());
                            }
                        } else if key == "version" {
                            let vs = i;
                            while i < bytes.len() && bytes[i].is_ascii_digit() {
                                i += 1;
                            }
                            if let Ok(n) = head[vs..i].parse::<u64>() {
                                if n > 0 {
                                    version = Some(n);
                                }
                            }
                        }
                        if id.is_some() && version.is_some() {
                            return Some((id?, version?));
                        }
                    }
                }
            }
            _ => i += 1,
        }
    }
    Some((id?, version?))
}

/// Given `bytes[quote_pos] == b'"'`, returns the index just past the closing
/// quote, respecting `\` escapes.
fn skip_json_string(bytes: &[u8], quote_pos: usize) -> Option<usize> {
    let mut i = quote_pos + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return Some(i + 1),
            _ => i += 1,
        }
    }
    None
}

/// Search the items of every document in a `.skopos.jsonl.zst` archive for a
/// word or phrase, ignoring case. Each item appears once, with its newest
/// version and whether the query matched the latest version or only an older
/// one.
pub fn search_archive<R: Read>(archive: R, query: &str) -> Result<SearchResult, String> {
    let mut s = Search::new(query);
    s.add_archive(archive)?;
    Ok(s.finish())
}

/// The same search on plain JSON Lines (archived).
pub fn search_lines<R: BufRead>(lines: R, query: &str) -> Result<SearchResult, String> {
    let mut s = Search::new(query);
    s.add_lines(lines)?;
    Ok(s.finish())
}

/// Search current documents and zero or more `.skopos.jsonl.zst` archives
/// together, highlighting which hits are `current` and which live only in
/// `archive`.
pub fn search_all<R: Read>(
    current_docs: &[Value],
    archives: impl IntoIterator<Item = R>,
    query: &str,
) -> Result<SearchResult, String> {
    let mut s = Search::new(query);
    // Feed archives first, then current docs so even if a query only matched
    // an older archived version, current docs update its latest version/status
    // and removal state; feeding in reverse order also works, because
    // `add_lines` parses lines for artifacts that already have hits.
    for archive in archives {
        s.add_archive(archive)?;
    }
    s.add_docs(current_docs)?;
    Ok(s.finish())
}

/// Streaming reader over one or more concatenated Zstandard frames (RFC 8878),
/// enforcing a maximum window size before allocating each frame's decode
/// buffer.
struct MultiFrameZstd<R: Read> {
    inner: PeekReader<R>,
    max_window: u64,
    decoder: Option<ruzstd::decoding::FrameDecoder>,
    done: bool,
}

/// Tiny buffered reader that lets us peek the Zstandard frame header (at most
/// 14 bytes) and check EOF between concatenated frames without losing bytes.
struct PeekReader<R: Read> {
    inner: R,
    buf: Vec<u8>,
    pos: usize,
}

impl<R: Read> PeekReader<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            buf: Vec::new(),
            pos: 0,
        }
    }

    fn available(&self) -> &[u8] {
        &self.buf[self.pos..]
    }

    /// Ensures at least `n` bytes are buffered unless EOF is reached first.
    fn fill_to(&mut self, n: usize) -> io::Result<usize> {
        if self.pos > 0 {
            self.buf.drain(..self.pos);
            self.pos = 0;
        }
        while self.buf.len() < n {
            let mut tmp = [0u8; 64];
            let want = (n - self.buf.len()).min(tmp.len());
            let read = self.inner.read(&mut tmp[..want])?;
            if read == 0 {
                break;
            }
            self.buf.extend_from_slice(&tmp[..read]);
        }
        Ok(self.buf.len())
    }

    fn consume(&mut self, n: usize) {
        self.pos = (self.pos + n).min(self.buf.len());
        if self.pos == self.buf.len() {
            self.buf.clear();
            self.pos = 0;
        }
    }
}

impl<R: Read> Read for PeekReader<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.pos < self.buf.len() {
            let avail = &self.buf[self.pos..];
            let n = avail.len().min(out.len());
            out[..n].copy_from_slice(&avail[..n]);
            self.consume(n);
            return Ok(n);
        }
        self.inner.read(out)
    }
}

const ZSTD_MAGIC: u32 = 0xFD2F_B528;
const SKIPPABLE_MAGIC_MIN: u32 = 0x184D_2A50;
const SKIPPABLE_MAGIC_MAX: u32 = 0x184D_2A5F;

/// Inspects a Zstandard frame header from `bytes` without consuming them.
/// Returns `Ok(Some(skip_len))` for a skippable frame, `Ok(None)` for a valid
/// data frame whose window fits in `max_window`, or an `io::Error` if invalid
/// or over the window cap.
fn inspect_zstd_header(bytes: &[u8], max_window: u64) -> io::Result<Option<u64>> {
    if bytes.len() < 4 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "truncated Zstandard frame header",
        ));
    }
    let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    if (SKIPPABLE_MAGIC_MIN..=SKIPPABLE_MAGIC_MAX).contains(&magic) {
        if bytes.len() < 8 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "truncated Zstandard skippable frame",
            ));
        }
        let len = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as u64;
        return Ok(Some(8 + len));
    }
    if magic != ZSTD_MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("not a Zstandard frame (magic 0x{magic:08x})"),
        ));
    }
    if bytes.len() < 5 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "truncated Zstandard frame descriptor",
        ));
    }
    let desc = bytes[4];
    let single_segment = (desc & 0x20) != 0;
    let dict_id_flag = desc & 0x03;
    let fcs_flag = desc >> 6;
    let mut offset = 5usize;

    let window_size = if !single_segment {
        if bytes.len() <= offset {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "truncated Zstandard window descriptor",
            ));
        }
        let wd = bytes[offset];
        let exp = (wd >> 3) as u64;
        let mantissa = (wd & 0x07) as u64;
        let base = 1u64.checked_shl((10 + exp) as u32).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "Zstandard window too large")
        })?;
        base + (base / 8) * mantissa
    } else {
        let dict_bytes = match dict_id_flag {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 4,
            _ => unreachable!(),
        };
        offset += dict_bytes;
        let fcs_bytes = match fcs_flag {
            0 => 1,
            1 => 2,
            2 => 4,
            3 => 8,
            _ => unreachable!(),
        };
        if bytes.len() < offset + fcs_bytes {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "truncated Zstandard frame content size",
            ));
        }
        let mut fcs = 0u64;
        for i in 0..fcs_bytes {
            fcs |= (bytes[offset + i] as u64) << (8 * i);
        }
        if fcs_bytes == 2 {
            fcs += 256;
        }
        fcs
    };

    if window_size > max_window {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "archive Zstandard window size {window_size} bytes exceeds the {max_window}-byte limit"
            ),
        ));
    }
    Ok(None)
}

impl<R: Read> MultiFrameZstd<R> {
    fn new(inner: R, max_window: u64) -> Self {
        Self {
            inner: PeekReader::new(inner),
            max_window,
            decoder: None,
            done: false,
        }
    }

    fn start_next_frame(&mut self) -> io::Result<bool> {
        loop {
            let have = self.inner.fill_to(18)?;
            if have == 0 {
                self.done = true;
                return Ok(false);
            }
            match inspect_zstd_header(self.inner.available(), self.max_window)? {
                Some(skip_total) => {
                    // Discard skippable frame bytes.
                    let mut left = skip_total;
                    let mut sink = [0u8; 1024];
                    while left > 0 {
                        let chunk = (left as usize).min(sink.len());
                        let n = self.inner.read(&mut sink[..chunk])?;
                        if n == 0 {
                            return Err(io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "truncated Zstandard skippable frame payload",
                            ));
                        }
                        left -= n as u64;
                    }
                    continue;
                }
                None => {
                    let mut dec = self.decoder.take().unwrap_or_default();
                    dec.reset(&mut self.inner)
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
                    self.decoder = Some(dec);
                    return Ok(true);
                }
            }
        }
    }
}

impl<R: Read> Read for MultiFrameZstd<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() || self.done {
            return Ok(0);
        }
        loop {
            if self.decoder.is_none() && !self.start_next_frame()? {
                return Ok(0);
            }
            let dec = self.decoder.as_mut().expect("frame started");
            while dec.can_collect() < buf.len() && !dec.is_finished() {
                let need = buf.len() - dec.can_collect();
                dec.decode_blocks(
                    &mut self.inner,
                    ruzstd::decoding::BlockDecodingStrategy::UptoBytes(need),
                )
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
            }
            if dec.can_collect() > 0 {
                return dec.read(buf);
            }
            // Frame finished and drained; advance to the next concatenated frame.
            if !self.start_next_frame()? {
                return Ok(0);
            }
        }
    }
}
