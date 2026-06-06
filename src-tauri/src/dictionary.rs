//! Post-transcription phrase replacements (BridgeVoice-style custom dictionary).

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

pub const MAX_FROM_LEN: usize = 120;
pub const MAX_TO_LEN: usize = 2000;
pub const MAX_ENTRIES: usize = 500;
/// Short `from` phrases only match at word boundaries to avoid corrupting Russian words.
const SHORT_RULE_BOUNDARY_MAX_LEN: usize = 4;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DictionaryEntry {
    /// Spoken phrase or common mis-transcription (matched case-insensitively).
    pub from: String,
    /// Text to insert instead.
    pub to: String,
}

const SEED_DICTIONARY_JSON: &str = include_str!("../resources/seed_dictionary.json");
const SEED_DICTIONARY_OVERFLOW_JSON: &str = include_str!("../resources/seed_dictionary_overflow.json");

static SEED_ENTRIES: Lazy<Vec<DictionaryEntry>> = Lazy::new(load_seed_entries);

fn load_seed_entries() -> Vec<DictionaryEntry> {
    let mut entries = parse_seed_json(SEED_DICTIONARY_JSON);
    entries.extend(parse_seed_json(SEED_DICTIONARY_OVERFLOW_JSON));
    dedupe_entries(entries)
}

fn parse_seed_json(json: &str) -> Vec<DictionaryEntry> {
    serde_json::from_str(json).unwrap_or_default()
}

fn dedupe_entries(entries: Vec<DictionaryEntry>) -> Vec<DictionaryEntry> {
    let mut out: Vec<DictionaryEntry> = Vec::with_capacity(entries.len());
    for entry in entries {
        if out
            .iter()
            .any(|e| e.from.eq_ignore_ascii_case(&entry.from))
        {
            continue;
        }
        out.push(entry);
    }
    out
}

/// Number of bundled IT/ASR correction rules shipped with the app.
pub fn seed_dictionary_count() -> usize {
    SEED_ENTRIES.len()
}

/// Merge user rules with optional bundled seed rules (user wins on duplicate `from`).
pub fn effective_dictionary_entries(
    user: &[DictionaryEntry],
    seed_enabled: bool,
) -> Vec<DictionaryEntry> {
    if !seed_enabled {
        return user.to_vec();
    }

    let mut out = user.to_vec();
    for entry in SEED_ENTRIES.iter() {
        if out
            .iter()
            .any(|e| e.from.eq_ignore_ascii_case(&entry.from))
        {
            continue;
        }
        out.push(entry.clone());
    }
    out
}

/// Dictionary rule with pre-lowercased `from` for matching (built at settings load).
#[derive(Debug, Clone)]
pub struct CompiledDictionaryRule {
    from_lower: Vec<char>,
    pub to: String,
}

impl CompiledDictionaryRule {
    pub fn compile_all(entries: &[DictionaryEntry]) -> Vec<Self> {
        entries
            .iter()
            .filter(|e| !e.from.is_empty())
            .map(|e| Self {
                from_lower: e
                    .from
                    .chars()
                    .flat_map(|c| c.to_lowercase())
                    .collect(),
                to: e.to.clone(),
            })
            .collect()
    }
}

/// Apply dictionary replacements with longest-phrase-first, non-overlapping matches.
pub fn apply_dictionary(text: &str, rules: &[CompiledDictionaryRule]) -> String {
    if rules.is_empty() {
        return text.to_string();
    }

    let hay_chars: Vec<char> = text.chars().collect();
    let mut result = String::with_capacity(text.len());
    let mut i = 0;

    while i < hay_chars.len() {
        if let Some((rule, match_len)) = longest_match_at(&hay_chars, i, rules) {
            result.push_str(&rule.to);
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
    rules: &'a [CompiledDictionaryRule],
) -> Option<(&'a CompiledDictionaryRule, usize)> {
    let mut best: Option<(&'a CompiledDictionaryRule, usize)> = None;
    for rule in rules {
        let n = rule.from_lower.len();
        if n == 0 || start + n > hay_chars.len() {
            continue;
        }
        let matches = hay_chars[start..start + n]
            .iter()
            .zip(rule.from_lower.iter())
            .all(|(h, n)| chars_eq_ignore_case(*h, *n));
        if !matches {
            continue;
        }
        if n <= SHORT_RULE_BOUNDARY_MAX_LEN && !match_has_word_boundaries(hay_chars, start, n) {
            continue;
        }
        if best.map(|(_, len)| n > len).unwrap_or(true) {
            best = Some((rule, n));
        }
    }
    best
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn match_has_word_boundaries(hay_chars: &[char], start: usize, len: usize) -> bool {
    let left_ok = start == 0 || !is_word_char(hay_chars[start - 1]);
    let end = start + len;
    let right_ok = end >= hay_chars.len() || !is_word_char(hay_chars[end]);
    left_ok && right_ok
}

fn chars_eq_ignore_case(a: char, b: char) -> bool {
    if a == b {
        return true;
    }
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();
    a_lower.eq(b_lower)
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

/// Export dictionary entries as CSV with `from,to` header.
pub fn export_csv(entries: &[DictionaryEntry]) -> String {
    let mut out = String::from("from,to\n");
    for entry in entries {
        out.push_str(&csv_escape(&entry.from));
        out.push(',');
        out.push_str(&csv_escape(&entry.to));
        out.push('\n');
    }
    out
}

fn csv_escape(value: &str) -> String {
    if value.contains(['"', ',', '\n', '\r']) {
        format!(
            "\"{}\"",
            value.replace('"', "\"\"")
        )
    } else {
        value.to_string()
    }
}

/// Parse CSV (header optional) and merge with existing entries when `merge` is true.
pub fn import_csv(csv: &str, existing: &[DictionaryEntry], merge: bool) -> Result<Vec<DictionaryEntry>, String> {
    let mut parsed = Vec::new();
    for (line_no, line) in csv.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if line_no == 0 && trimmed.eq_ignore_ascii_case("from,to") {
            continue;
        }
        let cols = parse_csv_line(trimmed)?;
        if cols.len() < 2 {
            return Err(format!("Line {}: expected from,to columns", line_no + 1));
        }
        parsed.push(DictionaryEntry {
            from: cols[0].clone(),
            to: cols[1].clone(),
        });
    }

    if parsed.is_empty() {
        return Err("No dictionary rows found in CSV".to_string());
    }

    let combined = if merge {
        let mut all = existing.to_vec();
        all.extend(parsed);
        all
    } else {
        parsed
    };

    Ok(sanitize_entries(combined))
}

fn parse_csv_line(line: &str) -> Result<Vec<String>, String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            }
            '"' => in_quotes = true,
            ',' if !in_quotes => {
                fields.push(current.trim().to_string());
                current.clear();
            }
            c => current.push(c),
        }
    }
    fields.push(current.trim().to_string());
    Ok(fields)
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

    fn compiled(entries: Vec<DictionaryEntry>) -> Vec<CompiledDictionaryRule> {
        CompiledDictionaryRule::compile_all(&entries)
    }

    #[test]
    fn replaces_case_insensitive() {
        let rules = compiled(vec![rule("bridge mind", "BridgeMind")]);
        assert_eq!(
            apply_dictionary("I use bridge mind daily", &rules),
            "I use BridgeMind daily"
        );
    }

    #[test]
    fn longest_match_first() {
        let rules = compiled(vec![
            rule("use", "USE"),
            rule("use effect", "useEffect"),
        ]);
        assert_eq!(
            apply_dictionary("call use effect hook", &rules),
            "call useEffect hook"
        );
    }

    #[test]
    fn multiple_rules() {
        let rules = compiled(vec![
            rule("next js", "Next.js"),
            rule("typescript", "TypeScript"),
        ]);
        assert_eq!(
            apply_dictionary("next js and typescript", &rules),
            "Next.js and TypeScript"
        );
    }

    #[test]
    fn csv_roundtrip() {
        let entries = vec![rule("foo", "bar"), rule("a,b", "c\"d")];
        let csv = export_csv(&entries);
        let imported = import_csv(&csv, &[], false).unwrap();
        assert_eq!(imported.len(), 2);
        assert_eq!(imported[1].from, "a,b");
        assert_eq!(imported[1].to, "c\"d");
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

    #[test]
    fn seed_dictionary_loads() {
        assert!(seed_dictionary_count() > 0);
    }

    #[test]
    fn user_overrides_seed_on_duplicate_from() {
        let user = vec![rule("гид сервер", "my git")];
        let merged = effective_dictionary_entries(&user, true);
        let git_server = merged
            .iter()
            .find(|e| e.from.eq_ignore_ascii_case("гид сервер"))
            .expect("entry");
        assert_eq!(git_server.to, "my git");
    }

    #[test]
    fn seed_disabled_returns_user_only() {
        let user = vec![rule("foo", "bar")];
        let merged = effective_dictionary_entries(&user, false);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].from, "foo");
    }

    #[test]
    fn seed_fixes_mixed_script_readme() {
        let merged = effective_dictionary_entries(&[], true);
        let rules = CompiledDictionaryRule::compile_all(&merged);
        assert_eq!(
            apply_dictionary("Изучi реadmi эmdi.", &rules),
            "Изучi README.md."
        );
    }

    #[test]
    fn seed_fixes_gitserver_hybrid() {
        let merged = effective_dictionary_entries(&[], true);
        let rules = CompiledDictionaryRule::compile_all(&merged);
        assert_eq!(
            apply_dictionary("Вправляй на gitсerвер.", &rules),
            "отправляй на git server."
        );
    }

    #[test]
    fn seed_fixes_doubled_backend() {
        let merged = effective_dictionary_entries(&[], true);
        let rules = CompiledDictionaryRule::compile_all(&merged);
        assert_eq!(
            apply_dictionary("Смотри, что там на backendэкэнд.", &rules),
            "Смотри, что там на backend."
        );
    }

    #[test]
    fn short_rules_require_word_boundaries() {
        let rules = compiled(vec![rule("стр", "str"), rule("гид сервер", "git server")]);
        assert_eq!(
            apply_dictionary("что есть две страницы", &rules),
            "что есть две страницы"
        );
        assert_eq!(apply_dictionary("тип стр в Rust", &rules), "тип str в Rust");
        assert_eq!(
            apply_dictionary("закоммить на гид сервер", &rules),
            "закоммить на git server"
        );
    }

    #[test]
    fn seed_fixes_cyrillic_backend() {
        let merged = effective_dictionary_entries(&[], true);
        let rules = CompiledDictionaryRule::compile_all(&merged);
        assert_eq!(
            apply_dictionary("Смотри, что на бэкенд.", &rules),
            "Смотри, что на backend."
        );
        assert_eq!(
            apply_dictionary("смотри на бек энд", &rules),
            "смотри на backend"
        );
    }

    #[test]
    fn seed_fixes_vrast_as_rust() {
        let merged = effective_dictionary_entries(&[], true);
        let rules = CompiledDictionaryRule::compile_all(&merged);
        assert_eq!(
            apply_dictionary("Тип  Стр  Враст .", &rules),
            "Тип  str  Rust ."
        );
        assert_eq!(
            apply_dictionary("что есть две страницы", &rules),
            "что есть две страницы"
        );
    }
}
