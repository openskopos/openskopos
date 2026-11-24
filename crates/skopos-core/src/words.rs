//! The difference between two texts, word by word.
//!
//! [`word_diff`] gives the pieces of text that are the same in both texts,
//! removed or new. [`crate::diff`] and [`crate::merge`] use it for the texts
//! of changed items, so every view of a change marks the same words.

use serde::{Deserialize, Serialize};

/// What happened to a piece of text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    /// In both texts.
    Same,
    /// Only in the old text.
    Del,
    /// Only in the new text.
    Ins,
}

/// A piece of text: one or more words with one space between them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub op: Op,
    pub text: String,
}

impl Span {
    fn new(op: Op, text: &str) -> Self {
        Span {
            op,
            text: text.to_string(),
        }
    }
}

/// The difference between two texts, word by word.
///
/// Words are separated by white space, and the white space is not compared.
/// Removed words come before new words. Next pieces with the same `op` are
/// joined. For very long texts, the result is the old text as removed and the
/// new text as new.
///
/// ```
/// use skopos_core::words::{word_diff, Op};
/// let spans = word_diff("Seats are counted every hour.", "Seats are counted at once.");
/// assert_eq!(spans[0].op, Op::Same);
/// assert_eq!(spans[1].text, "every hour.");
/// assert_eq!(spans[2].op, Op::Ins);
/// ```
pub fn word_diff(old: &str, new: &str) -> Vec<Span> {
    let a: Vec<&str> = old.split_whitespace().collect();
    let b: Vec<&str> = new.split_whitespace().collect();
    let mut out: Vec<Span> = Vec::new();
    let mut push = |op: Op, word: &str| match out.last_mut() {
        Some(last) if last.op == op => {
            last.text.push(' ');
            last.text.push_str(word);
        }
        _ => out.push(Span::new(op, word)),
    };
    if a.len().saturating_mul(b.len()) > 250_000 {
        if !a.is_empty() {
            push(Op::Del, &a.join(" "));
        }
        if !b.is_empty() {
            push(Op::Ins, &b.join(" "));
        }
        return out;
    }
    // Longest common subsequence, filled from the end.
    let (n, m) = (a.len(), b.len());
    let mut lcs = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if a[i] == b[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && a[i] == b[j] {
            push(Op::Same, a[i]);
            i += 1;
            j += 1;
        } else if i < n && (j == m || lcs[i + 1][j] >= lcs[i][j + 1]) {
            push(Op::Del, a[i]);
            i += 1;
        } else {
            push(Op::Ins, b[j]);
            j += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(list: &[(Op, &str)]) -> Vec<Span> {
        list.iter().map(|(op, t)| Span::new(*op, t)).collect()
    }

    #[test]
    fn marks_changed_words() {
        assert_eq!(
            word_diff(
                "Seats are counted every hour.",
                "Seats are  counted\nat once."
            ),
            spans(&[
                (Op::Same, "Seats are counted"),
                (Op::Del, "every hour."),
                (Op::Ins, "at once."),
            ])
        );
        assert_eq!(word_diff("a b", "a b"), spans(&[(Op::Same, "a b")]));
        assert_eq!(word_diff("", "new"), spans(&[(Op::Ins, "new")]));
        assert_eq!(word_diff("", ""), spans(&[]));
        assert_eq!(
            word_diff("keep this here", "keep here"),
            spans(&[(Op::Same, "keep"), (Op::Del, "this"), (Op::Same, "here")])
        );
    }

    #[test]
    fn very_long_texts_are_replaced_whole() {
        let a = "a ".repeat(600);
        let b = "b ".repeat(600);
        let d = word_diff(&a, &b);
        assert_eq!(d.len(), 2);
        assert_eq!((d[0].op, d[1].op), (Op::Del, Op::Ins));
    }
}
