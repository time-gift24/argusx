use regex::Regex;
use std::sync::LazyLock;

static BEARER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"Bearer\s+\S+"#).unwrap()
});

static AUTH_KEY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)"authorization"\s*:\s*"[^"]*""#).unwrap()
});

/// Redacts sensitive information from a preview string.
/// Currently redacts authorization headers and Bearer tokens.
pub fn redact_preview(raw: &str, limit: usize) -> String {
    let truncated: String = raw.chars().take(limit).collect();

    // Redact Bearer tokens: "Bearer secret123" -> "[REDACTED]"
    let result = BEARER_REGEX.replace_all(&truncated, "[REDACTED]");

    // Redact authorization keys: "authorization":"..." -> "[REDACTED]"
    AUTH_KEY_REGEX.replace_all(&result, "\"[REDACTED]\"").into_owned()
}
