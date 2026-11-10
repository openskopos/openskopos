//! Reads the OpenSpec files of a project: the requirement blocks of the main
//! specs in `openspec/specs/`, and the open changes in `openspec/changes/`.
//!
//! The rules follow the parser of OpenSpec 1.17.0 (`requirement-blocks.ts`):
//! a block starts at a `### Requirement:` header and ends at the next one or
//! at a `##` header, outside code fences. Scenarios are `####` headers, with
//! an optional `Scenario:` before the name.

use super::File;

/// The comment at the end of each exported requirement block. It holds a
/// hash of the block as the exporter wrote it, so that a later export can
/// see a hand edit.
pub(super) const HASH_PREFIX: &str = "<!-- skopos-export: ";

/// One requirement block of a main spec.
#[derive(Debug, Clone)]
pub(super) struct Block {
    /// The capability path, for example `billing/seat-limits`.
    pub capability: String,
    /// The requirement name, as OpenSpec reads it.
    pub name: String,
    /// The item ID of the OpenSkopos item that the block came from.
    pub item: String,
    /// The scenario names.
    pub scenarios: Vec<String>,
    /// The body: the lines after the header, without the hash comment.
    pub body: String,
    /// The hash in the comment, if there is one.
    pub recorded: Option<String>,
}

impl Block {
    /// True if the block has a hash comment that does not match its body:
    /// someone changed the block after the export.
    pub fn hand_edited(&self) -> bool {
        self.recorded
            .as_deref()
            .is_some_and(|h| h != hash(&self.body))
    }
}

/// The capability of a main spec path such as
/// `openspec/specs/billing/seat-limits/spec.md`.
pub(super) fn capability_of(path: &str) -> Option<&str> {
    path.strip_prefix("openspec/specs/")?
        .strip_suffix("/spec.md")
        .filter(|c| !c.is_empty())
}

/// The change name of a file in an open change, such as
/// `openspec/changes/add-auth/proposal.md`. Archived changes are not open.
pub(super) fn open_change_of(path: &str) -> Option<&str> {
    let rest = path.strip_prefix("openspec/changes/")?;
    let (change, _) = rest.split_once('/')?;
    (change != "archive").then_some(change)
}

/// The requirement blocks of every main spec that came from this artifact,
/// in the order of the files and of the blocks.
pub(super) fn blocks_of(files: &[File], artifact: &str) -> Vec<Block> {
    let mut out = Vec::new();
    for f in files {
        let Some(capability) = capability_of(&f.path) else {
            continue;
        };
        for (name, lines) in requirement_blocks(&f.text) {
            let Some(item) = own_item(&lines, artifact) else {
                continue;
            };
            let recorded = lines.iter().find_map(|l| {
                l.trim()
                    .strip_prefix(HASH_PREFIX)?
                    .strip_suffix("-->")
                    .map(|h| h.trim().to_string())
            });
            let body_lines: Vec<&str> = lines
                .iter()
                .copied()
                .filter(|l| !l.trim().starts_with(HASH_PREFIX))
                .collect();
            out.push(Block {
                capability: capability.to_string(),
                name,
                item,
                scenarios: scenario_names(&body_lines),
                body: normalize(&body_lines.join("\n")),
                recorded,
            });
        }
    }
    out
}

/// True if a text holds a reference string of this artifact.
pub(super) fn mentions(text: &str, artifact: &str) -> bool {
    references(text).any(|(a, _)| a == artifact)
}

/// The reference strings in backticks in a text: (artifact, item).
fn references(text: &str) -> impl Iterator<Item = (&str, &str)> {
    text.split('`').skip(1).step_by(2).filter_map(|code| {
        let code = code.trim();
        if !code.starts_with("pa_") {
            return None;
        }
        let (head, item) = code.split_once('#')?;
        let artifact = head.split('@').next()?;
        let ok = !item.is_empty() && item.bytes().all(|b| b.is_ascii_alphanumeric());
        ok.then_some((artifact, item))
    })
}

/// The item that a block came from: the item of its `Reference:` line, or
/// the first requirement (`FR`) reference of this artifact.
fn own_item(lines: &[&str], artifact: &str) -> Option<String> {
    let from_reference_line = lines.iter().find_map(|l| {
        let rest = l.trim().strip_prefix("Reference:")?;
        references(rest)
            .find(|(a, _)| *a == artifact)
            .map(|(_, i)| i.to_string())
    });
    from_reference_line.or_else(|| {
        lines.iter().find_map(|l| {
            references(l)
                .find(|(a, i)| *a == artifact && i.starts_with("FR"))
                .map(|(_, i)| i.to_string())
        })
    })
}

/// The requirement blocks of a spec: (name, lines after the header).
fn requirement_blocks(text: &str) -> Vec<(String, Vec<&str>)> {
    let mut blocks: Vec<(String, Vec<&str>)> = Vec::new();
    let mut current: Option<(String, Vec<&str>)> = None;
    let mut fence: Option<(char, usize)> = None;
    for line in text.lines() {
        if let Some(f) = fence_marker(line) {
            fence = match fence {
                None => Some(f),
                Some((c, n)) if f.0 == c && f.1 >= n => None,
                open => open,
            };
            if let Some((_, lines)) = current.as_mut() {
                lines.push(line);
            }
            continue;
        }
        if fence.is_none() {
            if let Some(name) = requirement_name(line) {
                blocks.extend(current.take());
                current = Some((name, Vec::new()));
                continue;
            }
            if line.starts_with("## ") || line.trim_end() == "##" {
                blocks.extend(current.take());
                continue;
            }
        }
        if let Some((_, lines)) = current.as_mut() {
            lines.push(line);
        }
    }
    blocks.extend(current);
    blocks
}

/// The fence character and length of a fence line.
fn fence_marker(line: &str) -> Option<(char, usize)> {
    let trimmed = line.trim_start_matches(' ');
    if line.len() - trimmed.len() > 3 {
        return None;
    }
    let c = trimmed.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let run = trimmed.chars().take_while(|x| *x == c).count();
    (run >= 3).then_some((c, run))
}

/// The name of a `### Requirement: <name>` header.
fn requirement_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix("###")?;
    if rest.starts_with('#') {
        return None;
    }
    let rest = rest.trim_start();
    let (label, name) = rest.split_once(':')?;
    if !label.eq_ignore_ascii_case("requirement") {
        return None;
    }
    let name = strip_closing_hashes(name.trim());
    (!name.is_empty()).then(|| name.to_string())
}

/// The scenario names of a block: each `####` header outside a fence, with
/// an optional `Scenario:` before the name.
fn scenario_names(lines: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    let mut fenced = false;
    for line in lines {
        if fence_marker(line).is_some() {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let Some(rest) = line.strip_prefix("####") else {
            continue;
        };
        if rest.starts_with('#') {
            continue;
        }
        let rest = rest.trim();
        let name = match rest.split_once(':') {
            Some((label, name)) if label.trim().eq_ignore_ascii_case("scenario") => name.trim(),
            _ => rest,
        };
        out.push(strip_closing_hashes(name).to_string());
    }
    out
}

/// A name without a closing run of `#`: `Foo ###` is `Foo`.
fn strip_closing_hashes(name: &str) -> &str {
    let without = name.trim_end_matches('#');
    if without.len() < name.len() && (without.is_empty() || without.ends_with(' ')) {
        without.trim_end()
    } else {
        name
    }
}

/// Requirement names are the same if they are the same after case and runs
/// of white space are ignored. OpenSpec treats a name that differs only in
/// case or spacing as the same requirement.
pub(super) fn same_name(a: &str, b: &str) -> bool {
    crate::one_line(a).to_lowercase() == crate::one_line(b).to_lowercase()
}

/// A block body for comparison: lines without trailing white space, with no
/// blank lines at the start or the end.
pub(super) fn normalize(body: &str) -> String {
    body.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim_matches('\n')
        .to_string()
}

/// A text with the version taken out of each reference string:
/// `pa_A@3#FR4` becomes `pa_A#FR4`. Each export writes the reference
/// strings with its own version, so a requirement that did not change has a
/// different version but the same content.
pub(super) fn without_versions(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find("pa_") {
        let (before, from) = rest.split_at(i);
        out.push_str(before);
        let id_len = 3 + from[3..]
            .bytes()
            .take_while(u8::is_ascii_alphanumeric)
            .count();
        out.push_str(&from[..id_len]);
        let after = &from[id_len..];
        let digits = after
            .strip_prefix('@')
            .map(|a| a.bytes().take_while(u8::is_ascii_digit).count())
            .unwrap_or(0);
        rest = if digits > 0 && after[1 + digits..].starts_with('#') {
            &after[1 + digits..]
        } else {
            after
        };
    }
    out.push_str(rest);
    out
}

/// The hash of a block body: 64-bit FNV-1a of the normalized body, as 16
/// hex digits. It finds edits, it is not for security.
pub(super) fn hash(body: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in normalize(body).bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, text: &str) -> File {
        File {
            path: path.to_string(),
            text: text.to_string(),
        }
    }

    const SPEC: &str = "# seats Specification

## Purpose
Seat limits.

## Requirements
### Requirement: FR4 - Block the change
A plan change above the limit is blocked.

Reference: `pa_A@3#FR4`. Priority: must

#### Scenario: Example 1

- **WHEN** 12 seats
- **THEN** blocked

#### Scenario: Success signal S2 ##
- **THEN** `pa_A@3#S2`

```md
### Requirement: not a header
#### not a scenario
```
<!-- skopos-export: 0000000000000000 -->

### requirement:   Hand written   ###
No reference here.

#### Scenario: x
- **THEN** y

### Requirement: Older export
Mentions `pa_B@1#FR1` and `pa_A@2#FR9`.

#### Scenario: z
- **THEN** w

## Other section
### Requirement: after a section
";

    #[test]
    fn reads_the_blocks_of_one_artifact() {
        let files = [
            file("openspec/specs/billing/seats/spec.md", SPEC),
            file("openspec/changes/x/specs/seats/spec.md", SPEC),
        ];
        let blocks = blocks_of(&files, "pa_A");
        let summary: Vec<(&str, &str, &str)> = blocks
            .iter()
            .map(|b| (b.capability.as_str(), b.name.as_str(), b.item.as_str()))
            .collect();
        assert_eq!(
            summary,
            vec![
                ("billing/seats", "FR4 - Block the change", "FR4"),
                ("billing/seats", "Older export", "FR9"),
            ]
        );
        assert_eq!(blocks[0].scenarios, vec!["Example 1", "Success signal S2"]);
        assert!(!blocks[0].body.contains("skopos-export"));
        assert_eq!(blocks[0].recorded.as_deref(), Some("0000000000000000"));
        assert!(blocks[0].hand_edited());
        assert!(!blocks[1].hand_edited(), "no hash, no claim");
        assert!(blocks_of(&files, "pa_C").is_empty());
    }

    #[test]
    fn paths_name_capabilities_and_open_changes() {
        assert_eq!(capability_of("openspec/specs/a/b/spec.md"), Some("a/b"));
        assert_eq!(capability_of("openspec/specs/spec.md"), None);
        assert_eq!(
            open_change_of("openspec/changes/add-x/proposal.md"),
            Some("add-x")
        );
        assert_eq!(
            open_change_of("openspec/changes/archive/2026-01-01-x/proposal.md"),
            None
        );
        assert!(mentions("see `pa_A@2#FR1`", "pa_A"));
        assert!(!mentions("see pa_A@2#FR1", "pa_A"));
    }

    #[test]
    fn names_compare_like_openspec() {
        assert!(same_name("FR4 -  Block", "fr4 - block"));
        assert!(!same_name("FR4 - Block", "FR4 - Block (v4)"));
        assert_eq!(hash("a  \n\nb\n\n"), hash("a\n\nb"));
        assert_ne!(hash("a"), hash("b"));
    }

    #[test]
    fn versions_are_taken_out_of_reference_strings() {
        assert_eq!(
            without_versions("Reference: `pa_A1@3#FR4`, `pa_B@12#S2` and pa_C@x#Q1 at pa_"),
            "Reference: `pa_A1#FR4`, `pa_B#S2` and pa_C@x#Q1 at pa_"
        );
    }
}
