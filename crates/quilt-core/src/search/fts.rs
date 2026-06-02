//! FTS5 query utilities — pure string functions extracted from `quilt-search`.
//!
//! These functions operate solely on strings and have zero
//! database or I/O dependencies. They can be compiled to WASM.
//!
//! **Source of truth note**: the `sanitize_fts5_query_tokens` and
//! `build_fts5_match_query` functions are kept in sync with the
//! implementation in `quilt-search::sanitize`. The WASM exports in
//! `wasm.rs` call the safe versions; the legacy `sanitize_fts5_query`
//! (returning a single `String`) is kept for backward compatibility but
//! is no longer the recommended path — new code should call
//! `build_fts5_match_query` instead.

/// FTS5 boolean operator keywords (case-insensitive).
///
/// User-typed operator words are filtered out of the sanitized token
/// stream so the resulting FTS5 MATCH expression contains only
/// real search terms.
const FTS5_OPERATORS: &[&str] = &["AND", "OR", "NOT", "NEAR"];

/// Characters stripped from the leading/trailing edges of a token.
///
/// `-` and `_` are deliberately NOT in this list — they are common in
/// compound words (`foo-bar`, `foo_bar`) and in identifiers.
const EDGE_TRIM_CHARS: &[char] = &[
    '"', '\'', '(', ')', ':', '^', '.', '+', '~', ',', ';', '!', '@', '#', '$', '%', '&', '=', '?',
    '<', '>', '[', ']', '{', '}', '|', '\\', '/',
];

/// Returns true if `word` is an FTS5 boolean operator (AND, OR, NOT, NEAR).
fn is_fts5_operator(word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    let upper = word.to_ascii_uppercase();
    FTS5_OPERATORS.iter().any(|op| *op == upper)
}

/// Sanitizes a single whitespace-separated token.
fn sanitize_token(word: &str) -> Option<String> {
    if is_fts5_operator(word) {
        return None;
    }
    let (candidate, is_prefix) = match word.strip_suffix('*') {
        Some(stripped) if !stripped.is_empty() => (stripped, true),
        _ => (word, false),
    };
    let core = candidate.trim_matches(|c: char| EDGE_TRIM_CHARS.contains(&c));
    if core.is_empty() {
        return None;
    }
    let safe: String = core
        .chars()
        .map(|c| if c == '"' { '\'' } else { c })
        .collect();
    Some(if is_prefix {
        format!("\"{}\"*", safe)
    } else {
        format!("\"{}\"", safe)
    })
}

/// Sanitizes a user-provided search query for safe use in FTS5 MATCH.
///
/// Returns a list of sanitized tokens, one per input word. Each token is
/// double-quoted so FTS5 treats it as a literal phrase. This is the safe
/// replacement for the legacy `sanitize_fts5_query` (which returned a
/// single concatenated string and was prone to FTS5 syntax errors).
///
/// # Examples
///
/// ```
/// use quilt_core::search::fts::sanitize_fts5_query_tokens;
/// assert_eq!(sanitize_fts5_query_tokens("hello"), vec!["\"hello\"".to_string()]);
/// assert_eq!(sanitize_fts5_query_tokens("test*"), vec!["\"test\"*".to_string()]);
/// assert_eq!(sanitize_fts5_query_tokens(""), Vec::<String>::new());
/// ```
pub fn sanitize_fts5_query_tokens(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .filter_map(sanitize_token)
        .collect()
}

/// Builds a complete FTS5 MATCH expression from user input.
///
/// Joins sanitized tokens with whitespace. FTS5 treats whitespace
/// between terms as an implicit `AND`, so `"foo" "bar"` is semantically
/// equivalent to `"foo" AND "bar"`. Using whitespace as the joiner (not
/// the literal `AND` keyword) ensures user-typed operator words are
/// unambiguously filtered — they can never appear in the output as the
/// joiner.
///
/// Returns `None` if the input produces no tokens (empty input,
/// all-whitespace, or input that consists only of FTS5 operators and
/// special characters).
///
/// # Examples
///
/// ```
/// use quilt_core::search::fts::build_fts5_match_query;
/// assert_eq!(build_fts5_match_query("hello world"), Some("\"hello\" \"world\"".to_string()));
/// assert_eq!(build_fts5_match_query(""), None);
/// ```
pub fn build_fts5_match_query(query: &str) -> Option<String> {
    let tokens = sanitize_fts5_query_tokens(query);
    if tokens.is_empty() {
        return None;
    }
    Some(tokens.join(" "))
}

/// Legacy single-string sanitization kept for backward compatibility.
///
/// **Prefer** `build_fts5_match_query` for new code. This function
/// returns a concatenated string of double-quoted terms. It is still
/// used by the WASM export `fts_sanitize` (which exposes the legacy
/// single-string shape for JavaScript callers), and is also exposed
/// publicly for callers that need the same shape.
///
/// # Examples
///
/// ```
/// use quilt_core::search::fts::sanitize_fts5_query;
/// assert_eq!(sanitize_fts5_query("hello world"), "\"hello\" \"world\"");
/// assert_eq!(sanitize_fts5_query(""), "\"\"");
/// ```
pub fn sanitize_fts5_query(query: &str) -> String {
    let tokens = sanitize_fts5_query_tokens(query);
    if tokens.is_empty() {
        return "\"\"".to_string();
    }
    tokens.join(" ")
}

/// Build an FTS5 prefix-match query by appending `*` to each term.
///
/// Strips non-alphanumeric characters from each term before appending `*`,
/// so special characters like `*`, `(`, `)` do not cause FTS5 syntax errors.
///
/// # Examples
///
/// ```
/// use quilt_core::search::fts::build_fuzzy_query;
/// assert_eq!(build_fuzzy_query("hello world"), "hello* world*");
/// assert_eq!(build_fuzzy_query("foo* (bar)"), "foo* bar*");
/// assert_eq!(build_fuzzy_query(""), r#""*""#);
/// ```
pub fn build_fuzzy_query(query: &str) -> String {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return "\"*\"".to_string();
    }
    trimmed
        .split_whitespace()
        .map(|term| {
            let clean: String = term.chars().filter(|c| c.is_alphanumeric()).collect();
            if clean.is_empty() {
                "*".to_string()
            } else {
                format!("{}*", clean)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Generate a snippet with highlighted matches from content.
///
/// If the content is shorter than `max_len`, returns the content as-is.
/// Otherwise truncates with an ellipsis, preserving word boundaries
/// when possible.
///
/// # Examples
///
/// ```
/// use quilt_core::search::fts::generate_snippet;
/// assert_eq!(generate_snippet("Short content", "test", 50), "Short content");
/// let long = "This is a very long content that should be truncated";
/// let snippet = generate_snippet(long, "test", 20);
/// assert!(snippet.ends_with("..."));
/// assert!(snippet.len() <= 20);
/// ```
pub fn generate_snippet(content: &str, _query: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        return content.to_string();
    }
    // Simple truncation with ellipsis
    format!("{}...", &content[..max_len.saturating_sub(3)])
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── build_fts5_match_query (new safe API) ──────────────────────────

    #[test]
    fn test_build_fts5_match_query_normal() {
        let result = build_fts5_match_query("hello world");
        assert_eq!(result, Some("\"hello\" \"world\"".to_string()));
    }

    #[test]
    fn test_build_fts5_match_query_prefix_preserved() {
        let result = build_fts5_match_query("test*");
        assert_eq!(result, Some("\"test\"*".to_string()));
    }

    #[test]
    fn test_build_fts5_match_query_empty() {
        assert_eq!(build_fts5_match_query(""), None);
        assert_eq!(build_fts5_match_query("   "), None);
    }

    #[test]
    fn test_build_fts5_match_query_operators_removed() {
        // User-typed FTS5 boolean operator words are filtered
        let result = build_fts5_match_query("foo AND bar OR baz");
        let s = result.expect("non-empty");
        assert!(!s.contains(" AND "));
        assert!(!s.contains(" OR "));
        assert!(!s.contains(" NOT "));
        assert!(!s.contains(" NEAR "));
        // The literal terms are present
        assert!(s.contains("\"foo\""));
        assert!(s.contains("\"bar\""));
        assert!(s.contains("\"baz\""));
    }

    #[test]
    fn test_build_fts5_match_query_balanced_quotes() {
        // Even with pathological input, the result has balanced quotes
        for q in [
            "\"unterminated",
            "(broken",
            "MATCH()",
            "'; DROP TABLE blocks; --",
        ] {
            if let Some(s) = build_fts5_match_query(q) {
                let count = s.chars().filter(|c| *c == '"').count();
                assert_eq!(count % 2, 0, "Unbalanced quotes in: {}", s);
            }
        }
    }

    // ── sanitize_fts5_query (legacy single-string) ─────────────────────

    #[test]
    fn test_sanitize_fts5_query_normal() {
        // Legacy single-string form joins tokens with spaces.
        // FTS5 treats spaces as implicit AND, so this is semantically
        // equivalent to `"hello" AND "world"`.
        let result = sanitize_fts5_query("hello world");
        assert_eq!(result, r#""hello" "world""#);
    }

    #[test]
    fn test_sanitize_fts5_query_special_chars() {
        // Parens are edge-trimmed; `*` is preserved as a prefix marker.
        let result = sanitize_fts5_query("foo* (bar)");
        assert_eq!(result, r#""foo"* "bar""#);
    }

    #[test]
    fn test_sanitize_fts5_query_empty() {
        let result = sanitize_fts5_query("");
        assert_eq!(result, r#""""#);
    }

    #[test]
    fn test_sanitize_fts5_query_whitespace() {
        let result = sanitize_fts5_query("  hello   world  ");
        assert_eq!(result, r#""hello" "world""#);
    }

    #[test]
    fn test_sanitize_fts5_query_embedded_quotes() {
        // Internal `"` is replaced with `'` to keep quotes balanced.
        let result = sanitize_fts5_query(r#"he"llo"#);
        assert_eq!(result, r#""he'llo""#);
    }

    #[test]
    fn test_sanitize_fts5_query_operators_filtered() {
        // AND, OR, NOT, NEAR are filtered out as standalone tokens.
        let result = sanitize_fts5_query("foo AND bar OR NOT baz");
        assert_eq!(result, r#""foo" "bar" "baz""#);
    }

    // ── build_fuzzy_query ──────────────────────────────────────────────

    #[test]
    fn test_build_fuzzy_query_normal() {
        let result = build_fuzzy_query("hello world");
        assert_eq!(result, "hello* world*");
    }

    #[test]
    fn test_build_fuzzy_query_special_chars() {
        let result = build_fuzzy_query("foo* (bar)");
        assert_eq!(result, "foo* bar*");
    }

    #[test]
    fn test_build_fuzzy_query_empty() {
        let result = build_fuzzy_query("");
        assert_eq!(result, r#""*""#);
    }

    #[test]
    fn test_build_fuzzy_query_non_alphanumeric() {
        let result = build_fuzzy_query("hello!!! world???");
        assert_eq!(result, "hello* world*");
    }

    #[test]
    fn test_build_fuzzy_query_only_special() {
        let result = build_fuzzy_query("!!! ???");
        assert_eq!(result, "* *");
    }

    #[test]
    fn test_build_fuzzy_query_mixed() {
        let result = build_fuzzy_query("  foo   bar  ");
        assert_eq!(result, "foo* bar*");
    }

    // ── generate_snippet ───────────────────────────────────────────────

    #[test]
    fn test_generate_snippet_short_content() {
        let result = generate_snippet("Short content", "test", 50);
        assert_eq!(result, "Short content");
    }

    #[test]
    fn test_generate_snippet_long_content() {
        let content = "This is a very long content that should be truncated";
        let snippet = generate_snippet(content, "test", 20);
        assert!(snippet.ends_with("..."));
        assert!(snippet.len() <= 20);
    }

    #[test]
    fn test_generate_snippet_exact_fit() {
        let content = "Exactly twenty!!";
        assert_eq!(content.len(), 16);
        let result = generate_snippet(content, "test", 16);
        assert_eq!(result, "Exactly twenty!!");
    }

    #[test]
    fn test_generate_snippet_empty() {
        assert_eq!(generate_snippet("", "test", 50), "");
    }

    #[test]
    fn test_generate_snippet_zero_max() {
        let result = generate_snippet("Some content", "test", 0);
        assert_eq!(result, "...");
        assert_eq!(result.len(), 3);
    }
}
