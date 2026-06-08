//! Post-dictionary text cleanup (filler word removal).

use once_cell::sync::Lazy;
use regex::Regex;

static FILLER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(um+|uh+|er+|ah+|hmm+|hm+)\b|\b(you know|i mean)\b")
        .expect("filler regex must compile")
});

/// Remove common English filler words/phrases when enabled in settings.
pub fn strip_filler_words(text: &str) -> String {
    let stripped = FILLER_RE.replace_all(text, "");
    collapse_whitespace(&stripped)
}

fn collapse_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !prev_space && !out.is_empty() {
                out.push(' ');
                prev_space = true;
            }
        } else {
            prev_space = false;
            out.push(ch);
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_um_uh() {
        assert_eq!(strip_filler_words("um hello uh world"), "hello world");
    }

    #[test]
    fn removes_you_know() {
        assert_eq!(strip_filler_words("so you know it works"), "so it works");
    }

    #[test]
    fn preserves_real_words() {
        assert_eq!(strip_filler_words("I like this"), "I like this");
    }
}
