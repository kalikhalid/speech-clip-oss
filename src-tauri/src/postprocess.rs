//! Post-dictionary text cleanup.

use once_cell::sync::Lazy;
use regex::Regex;

const MESSENGER_APP_NAMES: &[&str] = &[
    "discord",
    "mattermost",
    "messages",
    "messenger",
    "signal",
    "slack",
    "telegram",
    "viber",
    "whatsapp",
    "сообщения",
];

static FILLER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(um+|uh+|er+|ah+|hmm+|hm+)\b|\b(you know|i mean)\b")
        .expect("filler regex must compile")
});

/// Remove common English filler words/phrases when enabled in settings.
pub fn strip_filler_words(text: &str) -> String {
    let stripped = FILLER_RE.replace_all(text, "");
    collapse_whitespace(&stripped)
}

/// In messengers, CIS users often write short messages without a final period.
pub fn strip_messenger_terminal_period(text: &str, app_name: Option<&str>) -> String {
    if !is_messenger_app(app_name) {
        return text.to_string();
    }

    let trimmed = text.trim_end();
    let Some(without_period) = trimmed.strip_suffix('.') else {
        return text.trim().to_string();
    };
    if without_period.ends_with('.') {
        return text.trim().to_string();
    }
    without_period.trim_end().to_string()
}

fn is_messenger_app(app_name: Option<&str>) -> bool {
    let Some(app_name) = app_name else {
        return false;
    };
    let app_name = app_name.trim().to_lowercase();
    MESSENGER_APP_NAMES
        .iter()
        .any(|messenger| app_name.contains(messenger))
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

    #[test]
    fn removes_terminal_period_in_messengers() {
        assert_eq!(
            strip_messenger_terminal_period("Привет, я уже тут.", Some("Telegram")),
            "Привет, я уже тут"
        );
        assert_eq!(
            strip_messenger_terminal_period("Буду через 5 минут.   ", Some("WhatsApp")),
            "Буду через 5 минут"
        );
    }

    #[test]
    fn keeps_terminal_period_outside_messengers() {
        assert_eq!(
            strip_messenger_terminal_period("Готово.", Some("Cursor")),
            "Готово."
        );
        assert_eq!(strip_messenger_terminal_period("Готово.", None), "Готово.");
    }

    #[test]
    fn keeps_non_period_sentence_endings_in_messengers() {
        assert_eq!(
            strip_messenger_terminal_period("Ты где?", Some("Slack")),
            "Ты где?"
        );
        assert_eq!(
            strip_messenger_terminal_period("Отлично!", Some("Discord")),
            "Отлично!"
        );
        assert_eq!(
            strip_messenger_terminal_period("Подожди...", Some("Signal")),
            "Подожди..."
        );
    }
}
