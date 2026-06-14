//! Natural-language date resolution (V1).
//!
//! Mirrors the frontend `naturalDate.ts` module. Converts tokens like
//! "today", "tomorrow", "yesterday" to `YYYY-MM-DD` strings using the
//! *local* timezone of the server process.
//!
//! Scope: only rewrites exact single-token matches inside known date
//! property inline syntax (`deadline::`, `scheduled::`, `date::`).

use chrono::Local;

/// Resolve a single natural-language date token to `YYYY-MM-DD`.
///
/// Returns `None` for anything that isn't an exact match of
/// "today", "tomorrow", or "yesterday" (case-insensitive, trimmed).
pub fn resolve_natural_date(input: &str) -> Option<String> {
    let token = input.trim().to_lowercase();
    if token.is_empty() {
        return None;
    }

    let offset_days = match token.as_str() {
        "today" => 0,
        "tomorrow" => 1,
        "yesterday" => -1,
        _ => return None,
    };

    let base = Local::now().date_naive();
    let target = base + chrono::Duration::try_days(offset_days).unwrap();
    Some(target.format("%Y-%m-%d").to_string())
}

/// Canonical property keys whose values are dates.
const DATE_PROPERTY_KEYS: &[&str] = &["deadline", "scheduled", "date"];

/// Scan block content and resolve natural-date tokens in date-property
/// inline syntax (`deadline:: today` → `deadline:: 2026-06-12`).
///
/// Only rewrites values that are exactly a natural-date token.
/// Free-text mentions are left untouched.
pub fn resolve_natural_dates_in_content(content: &str) -> String {
    content
        .lines()
        .map(|line| resolve_natural_date_in_line(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn resolve_natural_date_in_line(line: &str) -> String {
    // Find the "::" separator (property inline syntax)
    let sep_pos = match line.find("::") {
        Some(p) => p,
        None => return line.to_string(),
    };

    // Extract the key (text before "::")
    let key_part = line[..sep_pos].trim_end();
    // Check if key is a known date property (case-insensitive)
    let key_lower = key_part.to_lowercase();
    if !DATE_PROPERTY_KEYS.contains(&key_lower.as_str()) {
        return line.to_string();
    }

    // Extract leading whitespace before the key
    let leading_ws_len = key_part.len() - key_part.trim_start().len();
    let leading_ws = &line[..leading_ws_len];

    // Extract the value after "::"
    let after_sep = &line[sep_pos + 2..];
    // Separate separator whitespace, value, and trailing whitespace
    let value_start = after_sep.find(|c: char| !c.is_whitespace());
    let (sep_ws, rest) = match value_start {
        Some(i) => (&after_sep[..i], &after_sep[i..]),
        None => return line.to_string(), // "::" with nothing after — no value to resolve
    };

    // Trim trailing whitespace from the value
    let trailing_ws_len = rest.len() - rest.trim_end().len();
    let value = &rest[..rest.len() - trailing_ws_len];
    let trailing_ws = &rest[rest.len() - trailing_ws_len..];

    match resolve_natural_date(value) {
        Some(resolved) => {
            format!("{}{}::{}{}{}", leading_ws, key_part, sep_ws, resolved, trailing_ws)
        }
        None => line.to_string(),
    }
}

/// Resolve a natural date value in a property write context.
/// If the key is a date property and the value is a natural token,
/// return the resolved ISO date. Otherwise return the value as-is.
pub fn maybe_resolve_date_property(key: &str, value: &str) -> String {
    let key_lower = key.trim().to_lowercase();
    if DATE_PROPERTY_KEYS.contains(&key_lower.as_str()) {
        match resolve_natural_date(value) {
            Some(resolved) => resolved,
            None => value.to_string(),
        }
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_today() {
        let result = resolve_natural_date("today").unwrap();
        let expected = Local::now().date_naive().format("%Y-%m-%d").to_string();
        assert_eq!(result, expected);
    }

    #[test]
    fn resolve_tomorrow() {
        let result = resolve_natural_date("tomorrow").unwrap();
        let expected =
            (Local::now().date_naive() + chrono::Duration::try_days(1).unwrap())
                .format("%Y-%m-%d")
                .to_string();
        assert_eq!(result, expected);
    }

    #[test]
    fn resolve_yesterday() {
        let result = resolve_natural_date("yesterday").unwrap();
        let expected =
            (Local::now().date_naive() + chrono::Duration::try_days(-1).unwrap())
                .format("%Y-%m-%d")
                .to_string();
        assert_eq!(result, expected);
    }

    #[test]
    fn case_insensitive() {
        assert!(resolve_natural_date("TODAY").is_some());
        assert!(resolve_natural_date("Tomorrow").is_some());
        assert!(resolve_natural_date("YESTERDAY").is_some());
    }

    #[test]
    fn whitespace_trimmed() {
        assert!(resolve_natural_date("  today  ").is_some());
    }

    #[test]
    fn rejects_non_tokens() {
        assert!(resolve_natural_date("foo").is_none());
        assert!(resolve_natural_date("next monday").is_none());
        assert!(resolve_natural_date("in 3 days").is_none());
        assert!(resolve_natural_date("today!").is_none());
        assert!(resolve_natural_date("not today").is_none());
    }

    #[test]
    fn rejects_empty() {
        assert!(resolve_natural_date("").is_none());
        assert!(resolve_natural_date("   ").is_none());
    }

    #[test]
    fn rejects_iso_dates() {
        assert!(resolve_natural_date("2026-01-15").is_none());
    }

    #[test]
    fn content_resolves_inline_deadline() {
        let result = resolve_natural_dates_in_content("deadline:: today");
        let expected = Local::now().date_naive().format("%Y-%m-%d").to_string();
        assert!(result.contains(&expected));
        assert!(result.starts_with("deadline::"));
    }

    #[test]
    fn content_preserves_non_date_properties() {
        let result = resolve_natural_dates_in_content("status:: todo priority:: A");
        assert_eq!(result, "status:: todo priority:: A");
    }

    #[test]
    fn content_preserves_real_dates() {
        let result = resolve_natural_dates_in_content("deadline:: 2026-01-15");
        assert_eq!(result, "deadline:: 2026-01-15");
    }

    #[test]
    fn content_preserves_free_text() {
        let result = resolve_natural_dates_in_content("I will finish this today");
        assert_eq!(result, "I will finish this today");
    }

    #[test]
    fn content_multiline_mixed() {
        let input = "some text\ndeadline:: today\nmore text";
        let result = resolve_natural_dates_in_content(input);
        assert!(result.starts_with("some text\n"));
        assert!(result.contains("deadline::"));
        assert!(result.ends_with("\nmore text"));
    }

    #[test]
    fn maybe_resolve_date_property_deadline_today() {
        let result = maybe_resolve_date_property("deadline", "today");
        let expected = Local::now().date_naive().format("%Y-%m-%d").to_string();
        assert_eq!(result, expected);
    }

    #[test]
    fn maybe_resolve_non_date_property_unchanged() {
        let result = maybe_resolve_date_property("status", "today");
        assert_eq!(result, "today");
    }

    #[test]
    fn maybe_resolve_real_date_unchanged() {
        let result = maybe_resolve_date_property("deadline", "2026-01-15");
        assert_eq!(result, "2026-01-15");
    }
}
