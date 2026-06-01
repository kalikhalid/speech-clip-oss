//! Post-transcription phrase replacements (BridgeVoice-style custom dictionary).

use serde::{Deserialize, Serialize};

pub const MAX_FROM_LEN: usize = 120;
pub const MAX_TO_LEN: usize = 2000;
pub const MAX_ENTRIES: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DictionaryEntry {
    /// Spoken phrase or common mis-transcription (matched case-insensitively).
    pub from: String,
    /// Text to insert instead.
    pub to: String,
}

/// Apply dictionary replacements with longest-phrase-first, non-overlapping matches.
pub fn apply_dictionary(text: &str, entries: &[DictionaryEntry]) -> String {
    let rules: Vec<&DictionaryEntry> = entries
        .iter()
        .filter(|e| !e.from.trim().is_empty())
        .collect();
    if rules.is_empty() {
        return text.to_string();
    }

    let hay_chars: Vec<char> = text.chars().collect();
    let mut result = String::with_capacity(text.len());
    let mut i = 0;

    while i < hay_chars.len() {
        if let Some((entry, match_len)) = longest_match_at(&hay_chars, i, &rules) {
            result.push_str(&entry.to);
            i += match_len;
        } else {
            result.push(hay_chars[i]);
            i += 1;
        }
    }
    result
}

fn longest_match_at<'a>(
    hay_chars: &[char],
    start: usize,
    rules: &[&'a DictionaryEntry],
) -> Option<(&'a DictionaryEntry, usize)> {
    let mut best: Option<(&DictionaryEntry, usize)> = None;
    for entry in rules {
        let needle: Vec<char> = entry.from.chars().collect();
        let n = needle.len();
        if n == 0 || start + n > hay_chars.len() {
            continue;
        }
        let matches = hay_chars[start..start + n]
            .iter()
            .zip(needle.iter())
            .all(|(h, n)| h.eq_ignore_ascii_case(n));
        if matches {
            if best.map(|(_, len)| n > len).unwrap_or(true) {
                best = Some((entry, n));
            }
        }
    }
    best
}

/// Normalize and validate entries for persistence.
pub fn sanitize_entries(entries: Vec<DictionaryEntry>) -> Vec<DictionaryEntry> {
    let mut out = Vec::new();
    for mut entry in entries {
        entry.from = entry.from.trim().to_string();
        entry.to = entry.to.trim().to_string();
        if entry.from.is_empty() {
            continue;
        }
        if entry.from.len() > MAX_FROM_LEN {
            entry.from.truncate(MAX_FROM_LEN);
        }
        if entry.to.len() > MAX_TO_LEN {
            entry.to.truncate(MAX_TO_LEN);
        }
        if out.iter().any(|e: &DictionaryEntry| e.from.eq_ignore_ascii_case(&entry.from)) {
            continue;
        }
        out.push(entry);
        if out.len() >= MAX_ENTRIES {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(from: &str, to: &str) -> DictionaryEntry {
        DictionaryEntry {
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    #[test]
    fn replaces_case_insensitive() {
        let entries = vec![rule("bridge mind", "BridgeMind")];
        assert_eq!(
            apply_dictionary("I use bridge mind daily", &entries),
            "I use BridgeMind daily"
        );
    }

    #[test]
    fn longest_match_first() {
        let entries = vec![
            rule("use", "USE"),
            rule("use effect", "useEffect"),
        ];
        assert_eq!(
            apply_dictionary("call use effect hook", &entries),
            "call useEffect hook"
        );
    }

    #[test]
    fn multiple_rules() {
        let entries = vec![
            rule("next js", "Next.js"),
            rule("typescript", "TypeScript"),
        ];
        assert_eq!(
            apply_dictionary("next js and typescript", &entries),
            "Next.js and TypeScript"
        );
    }

    #[test]
    fn sanitize_dedupes_and_trims() {
        let raw = vec![
            rule("  foo ", "bar "),
            rule("FOO", "baz"),
            rule("", "x"),
        ];
        let out = sanitize_entries(raw);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].from, "foo");
        assert_eq!(out[0].to, "bar");
    }
}
