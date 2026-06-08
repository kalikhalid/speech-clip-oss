//! Deterministic post-ASR normalization for common developer dictation.
//!
//! This layer handles high-confidence lexical and filename cases before the
//! small LLM sees the text. The LLM should not have to memorize every spelling
//! variant for terms like `Grafana` or extensions like `md`.

use once_cell::sync::Lazy;
use serde::Deserialize;

const RULES_JSON: &str = include_str!("../resources/spoken_normalization_rules.json");

static RULES: Lazy<Ruleset> = Lazy::new(load_rules);

#[derive(Debug, Clone)]
struct PhraseRule {
    from: Vec<String>,
    to: String,
}

#[derive(Debug)]
struct Ruleset {
    stems: Vec<PhraseRule>,
    extensions: Vec<PhraseRule>,
    terms: Vec<PhraseRule>,
    dot_words: Vec<String>,
    at_words: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RulesFile {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    stems: Vec<JsonRule>,
    #[serde(default)]
    extensions: Vec<JsonRule>,
    #[serde(default)]
    terms: Vec<JsonRule>,
    #[serde(default)]
    dot_words: Vec<String>,
    #[serde(default)]
    at_words: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct JsonRule {
    from: Vec<String>,
    to: String,
}

#[derive(Debug, Clone)]
struct Token {
    original: String,
    prefix: String,
    suffix: String,
    norm: String,
}

pub fn normalize_text(text: &str) -> String {
    let rules = rules();
    let tokens = tokenize(text);
    if tokens.is_empty() {
        return text.to_string();
    }

    let mut out = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if is_at_word(&tokens[i], rules) {
            if let Some((replacement, consumed)) = match_file_at(&tokens, i + 1, rules) {
                out.push(replacement);
                i += consumed + 1;
                continue;
            }
        }

        if let Some((replacement, consumed)) = match_file_at(&tokens, i, rules) {
            out.push(replacement);
            i += consumed;
            continue;
        }

        if let Some((rule, consumed)) = match_rule_at(&tokens, i, &rules.terms) {
            out.push(replace_phrase(
                &tokens[i],
                &tokens[i + consumed - 1],
                &rule.to,
            ));
            i += consumed;
            continue;
        }

        out.push(tokens[i].original.clone());
        i += 1;
    }

    out.join(" ")
}

/// Reject obvious full-sentence translations from the LLM.
pub fn guard_model_output(deterministic_input: &str, model_output: &str) -> String {
    let before = count_cyrillic_words(deterministic_input);
    let after = count_cyrillic_words(model_output);
    if before >= 3 && after * 2 < before {
        deterministic_input.to_string()
    } else {
        model_output.to_string()
    }
}

fn rules() -> &'static Ruleset {
    &RULES
}

fn load_rules() -> Ruleset {
    let file: RulesFile =
        serde_json::from_str(RULES_JSON).expect("spoken normalization rules JSON must parse");
    let _version = file.version;
    Ruleset {
        stems: normalize_rules(file.stems),
        extensions: normalize_rules(file.extensions),
        terms: normalize_rules(file.terms),
        dot_words: normalize_words(file.dot_words),
        at_words: normalize_words(file.at_words),
    }
}

fn normalize_rules(rules: Vec<JsonRule>) -> Vec<PhraseRule> {
    rules
        .into_iter()
        .filter_map(|rule| {
            let from = normalize_words(rule.from);
            if from.is_empty() || rule.to.trim().is_empty() {
                return None;
            }
            Some(PhraseRule {
                from,
                to: rule.to.trim().to_string(),
            })
        })
        .collect()
}

fn normalize_words(words: Vec<String>) -> Vec<String> {
    words
        .into_iter()
        .map(|word| normalize_core(word.trim()))
        .filter(|word| !word.is_empty())
        .collect()
}

fn tokenize(text: &str) -> Vec<Token> {
    text.split_whitespace()
        .map(|part| {
            let (prefix, core, suffix) = split_token(part);
            let norm = normalize_core(&core);
            Token {
                original: part.to_string(),
                prefix,
                suffix,
                norm,
            }
        })
        .collect()
}

fn split_token(token: &str) -> (String, String, String) {
    let chars: Vec<char> = token.chars().collect();
    let start = chars
        .iter()
        .position(|c| is_core_char(*c))
        .unwrap_or(chars.len());
    let end = chars
        .iter()
        .rposition(|c| is_core_char(*c))
        .map(|idx| idx + 1)
        .unwrap_or(start);

    (
        chars[..start].iter().collect(),
        chars[start..end].iter().collect(),
        chars[end..].iter().collect(),
    )
}

fn is_core_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

fn normalize_core(value: &str) -> String {
    value
        .chars()
        .flat_map(|c| c.to_lowercase())
        .map(|c| if c == 'ё' { 'е' } else { c })
        .collect()
}

fn is_at_word(token: &Token, rules: &Ruleset) -> bool {
    rules.at_words.iter().any(|word| token.norm == *word)
}

fn is_dot_word(token: &Token, rules: &Ruleset) -> bool {
    rules.dot_words.iter().any(|word| token.norm == *word)
}

fn match_file_at(tokens: &[Token], start: usize, rules: &Ruleset) -> Option<(String, usize)> {
    let (stem, stem_len) = match_rule_at(tokens, start, &rules.stems)?;
    let dot_idx = start + stem_len;
    if dot_idx >= tokens.len() || !is_dot_word(&tokens[dot_idx], rules) {
        return None;
    }

    let ext_start = dot_idx + 1;
    let (ext, ext_len) = match_rule_at(tokens, ext_start, &rules.extensions)?;
    let first = &tokens[start];
    let last = &tokens[ext_start + ext_len - 1];
    let replacement = format!("{}@{}.{}{}", first.prefix, stem.to, ext.to, last.suffix);
    Some((replacement, stem_len + 1 + ext_len))
}

fn match_rule_at<'a>(
    tokens: &[Token],
    start: usize,
    rules: &'a [PhraseRule],
) -> Option<(&'a PhraseRule, usize)> {
    let mut best: Option<(&PhraseRule, usize)> = None;
    for rule in rules {
        let len = rule.from.len();
        if len == 0 || start + len > tokens.len() {
            continue;
        }
        let matches = rule
            .from
            .iter()
            .enumerate()
            .all(|(offset, expected)| tokens[start + offset].norm == *expected);
        if matches && best.map(|(_, best_len)| len > best_len).unwrap_or(true) {
            best = Some((rule, len));
        }
    }
    best
}

fn replace_phrase(first: &Token, last: &Token, replacement: &str) -> String {
    format!("{}{}{}", first.prefix, replacement, last.suffix)
}

fn count_cyrillic_words(text: &str) -> usize {
    text.split_whitespace()
        .filter(|word| word.chars().any(is_cyrillic))
        .count()
}

fn is_cyrillic(c: char) -> bool {
    ('а'..='я').contains(&c) || ('А'..='Я').contains(&c) || c == 'ё' || c == 'Ё'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_rules_load() {
        let rules = rules();
        assert!(!rules.stems.is_empty());
        assert!(!rules.extensions.is_empty());
        assert!(!rules.terms.is_empty());
        assert!(rules.dot_words.iter().any(|word| word == "точка"));
    }

    #[test]
    fn assembles_history_md_from_spoken_extension() {
        assert_eq!(normalize_text("Хистори точка мди"), "@history.md");
        assert_eq!(
            normalize_text("Открой хистори, точка Эмди."),
            "Открой @history.md."
        );
        assert_eq!(normalize_text("собака Хистори точка эм дэ"), "@history.md");
    }

    #[test]
    fn assembles_known_source_files() {
        assert_eq!(normalize_text("открой Мэйн точка Раст"), "открой @main.rs");
        assert_eq!(
            normalize_text("запусти клинап точка пай"),
            "запусти @cleanup.py"
        );
        assert_eq!(
            normalize_text("проверь пакедж лок точка джейсон"),
            "проверь @package-lock.json"
        );
        assert_eq!(
            normalize_text("открой докер комполз точка игрек эм эл"),
            "открой @docker-compose.yml"
        );
        assert_eq!(
            normalize_text("посмотри апп точка ти эс икс"),
            "посмотри @App.tsx"
        );
        assert_eq!(
            normalize_text("исправь стайлс точка си эс эс"),
            "исправь @styles.css"
        );
    }

    #[test]
    fn keeps_config_as_term_without_extension() {
        assert_eq!(normalize_text("проверь конфиг"), "проверь config");
        assert_eq!(
            normalize_text("проверь конфиг точка ямл"),
            "проверь @config.yaml"
        );
    }

    #[test]
    fn normalizes_high_confidence_terms() {
        assert_eq!(
            normalize_text("посмотри графане и докер комполз"),
            "посмотри Grafana и docker compose"
        );
        assert_eq!(
            normalize_text("закоммить на гид сервер"),
            "закоммить на git server"
        );
        assert_eq!(
            normalize_text("проверь бекенд эндпоинт и редис"),
            "проверь backend endpoint и Redis"
        );
        assert_eq!(
            normalize_text("откати деплой на стейджинге через роллбэк"),
            "откати deploy на стейджинге через rollback"
        );
        assert_eq!(
            normalize_text("открой вс код и постман"),
            "открой VS Code и Postman"
        );
    }

    #[test]
    fn does_not_assemble_unknown_files() {
        assert_eq!(normalize_text("Вася точка мди"), "Вася точка мди");
        assert_eq!(
            normalize_text("Хистори точка кракозябра"),
            "Хистори точка кракозябра"
        );
    }

    #[test]
    fn preserves_clean_russian() {
        assert_eq!(
            normalize_text("позвони маме после обеда"),
            "позвони маме после обеда"
        );
    }

    #[test]
    fn guard_rejects_full_translation() {
        assert_eq!(
            guard_model_output("позвони маме после обеда", "call mom after lunch"),
            "позвони маме после обеда"
        );
        assert_eq!(
            guard_model_output(
                "сделай commit и запуш в master",
                "make commit and push to master"
            ),
            "сделай commit и запуш в master"
        );
    }
}
