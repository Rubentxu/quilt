//! FTS5 query sanitization — converts user input into safe FTS5 MATCH syntax.
//!
//! ## Why this module exists
//!
//! FTS5 has special characters that crash the parser if not properly escaped.
//! Examples of failures with raw user input:
//!
//! - `'"bar'`  → SQLite error: "unterminated string"
//! - `'(foo'`  → SQLite error: "fts5: syntax error"
//! - `'foo"'`  → SQLite error: "unterminated string"
//!
//! Passing user input directly to `MATCH ?` is an injection vector. This
//! module is the single point of defense.
//!
//! ## SOLID design
//!
//! - **SRP** (Single Responsibility): this module does ONE thing — produce
//!   a safe list of FTS5 tokens. The DB layer never sees raw user input.
//! - **OCP** (Open/Closed): the [`SanitizationStrategy`] trait is the
//!   extension point. A new strategy (stemming, fuzzy tokenization, etc.)
//!   plugs in without changing the call site.
//! - **DIP** (Dependency Inversion): all functions are pure. They take
//!   `&str` and return owned values. No I/O, no state, no hidden
//!   dependencies — trivially testable, trivially mockable.

/// FTS5 reserved characters that act as operators or delimiters.
///
/// This constant documents the set of characters that, if present in raw
/// user input, can confuse the FTS5 parser. The actual sanitization is
/// performed by [`sanitize_fts5_query`], which is the single source of
/// truth. Keep this list in sync with the trimming logic.
pub const FTS5_SPECIAL_CHARS: &[char] = &[
    '"', '(', ')', '*', ':', '^', '.', '+', '~', ',', ';', '!', '@', '#', '$', '%', '&', '=', '?',
    '<', '>', '[', ']', '{', '}', '|', '\\', '/',
];

/// FTS5 boolean operator keywords (case-insensitive).
///
/// When a user types `foo AND bar`, they almost certainly mean "both foo
/// and bar", not "the literal string AND between foo and bar". We strip
/// these operator words so the sanitized MATCH expression only contains
/// actual search terms.
const FTS5_OPERATORS: &[&str] = &["AND", "OR", "NOT", "NEAR"];

/// Characters stripped from the leading/trailing edges of a token.
///
/// `-` and `_` are deliberately NOT in this list — they are common in
/// compound words (`foo-bar`, `foo_bar`) and in identifiers
/// (`class_name`). Emoji and Unicode letters are also preserved; the FTS5
/// unicode61 tokenizer handles them natively.
const EDGE_TRIM_CHARS: &[char] = &[
    '"', '\'', '(', ')', ':', '^', '.', '+', '~', ',', ';', '!', '@', '#', '$', '%', '&', '=', '?',
    '<', '>', '[', ']', '{', '}', '|', '\\', '/',
];

/// Returns true if `word` is an FTS5 boolean operator (AND, OR, NOT, NEAR).
///
/// Comparison is case-insensitive — FTS5 itself treats these operators
/// case-insensitively.
fn is_fts5_operator(word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    let upper = word.to_ascii_uppercase();
    FTS5_OPERATORS.iter().any(|op| *op == upper)
}

/// Sanitizes a single whitespace-separated token.
///
/// Returns `None` for empty tokens, FTS5 boolean operator words, and
/// tokens that become empty after edge trimming (e.g. `(((` alone).
fn sanitize_token(word: &str) -> Option<String> {
    // 1. Reject standalone FTS5 boolean operator words.
    if is_fts5_operator(word) {
        return None;
    }

    // 2. Detect a trailing `*` (prefix-search marker) BEFORE edge-trimming.
    //    If the word is just `*`, `stripped` is empty and we return None.
    let (candidate, is_prefix) = match word.strip_suffix('*') {
        Some(stripped) if !stripped.is_empty() => (stripped, true),
        _ => (word, false),
    };

    // 3. Trim FTS5-reserved edge punctuation. trim_matches only affects
    //    leading and trailing characters; internal punctuation is kept.
    let core = candidate.trim_matches(|c: char| EDGE_TRIM_CHARS.contains(&c));

    if core.is_empty() {
        return None;
    }

    // 4. Defensive: replace any internal `"` with `'`. The edge trim
    //    already strips leading/trailing `"`, so this branch is hit only
    //    for the (theoretical) case where `"` is adjacent to other
    //    keep-chars. Cheap insurance.
    let safe: String = core
        .chars()
        .map(|c| if c == '"' { '\'' } else { c })
        .collect();

    // 5. Wrap in double-quotes; re-append the `*` suffix if this was a
    //    prefix-search request. FTS5 string literals are always
    //    double-quoted.
    Some(if is_prefix {
        format!("\"{}\"*", safe)
    } else {
        format!("\"{}\"", safe)
    })
}

/// Sanitizes a user-provided search query for safe use in FTS5 MATCH.
///
/// Returns a list of sanitized tokens, one per input word. Each token is:
/// - Filtered of FTS5 boolean operator words (AND, OR, NOT, NEAR)
/// - Trimmed of FTS5-reserved edge punctuation
/// - Wrapped in double-quotes so FTS5 treats it as a literal phrase
/// - Suffixed with `*` if the original word ended in `*` (prefix search)
///
/// The output is suitable for use in `MATCH ?` placeholders. Returns an
/// empty `Vec` for empty or all-whitespace input.
///
/// # Examples
///
/// ```
/// use quilt_search::sanitize::sanitize_fts5_query;
///
/// assert_eq!(sanitize_fts5_query("hello"), vec!["\"hello\"".to_string()]);
/// assert_eq!(sanitize_fts5_query("test*"), vec!["\"test\"*".to_string()]);
/// assert_eq!(sanitize_fts5_query(""), Vec::<String>::new());
/// ```
pub fn sanitize_fts5_query(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .filter_map(sanitize_token)
        .collect()
}

/// Builds a complete FTS5 MATCH expression from user input.
///
/// Joins sanitized tokens with whitespace. FTS5 treats whitespace between
/// terms as an implicit `AND`, so `"foo" "bar"` is semantically equivalent
/// to `"foo" AND "bar"`. Using whitespace (instead of the literal `AND`
/// keyword) as the joiner means user-typed operator words (`AND`, `OR`,
/// `NEAR`) are unambiguously filtered — they can never appear in the
/// output as the joiner.
///
/// Returns `None` if the input produces no tokens (empty input,
/// all-whitespace, or input that consists only of FTS5 operators and
/// special characters).
///
/// This is the function `SearchService` calls before binding the query to
/// the `MATCH ?` placeholder. The `None` case signals "no usable query"
/// to the caller, which converts it into an [`EmptyQuery`](crate::search::SearchError::EmptyQuery) error.
///
/// # Examples
///
/// ```
/// use quilt_search::sanitize::build_fts5_match_query;
///
/// assert_eq!(
///     build_fts5_match_query("hello world"),
///     Some("\"hello\" \"world\"".to_string()),
/// );
/// assert_eq!(build_fts5_match_query(""), None);
/// ```
pub fn build_fts5_match_query(query: &str) -> Option<String> {
    let tokens = sanitize_fts5_query(query);
    if tokens.is_empty() {
        return None;
    }
    Some(tokens.join(" "))
}

// ═══════════════════════════════════════════════════════════════════════
// OCP: Strategy trait — extension point for future sanitization variants
// ═══════════════════════════════════════════════════════════════════════

/// Strategy for converting a user query into a list of FTS5-safe tokens.
///
/// This trait is the Open/Closed extension point. The default
/// [`QuoteStrategy`] produces double-quoted tokens. A future strategy
/// could implement stemming, fuzzy tokenization, or any other
/// transformation while keeping the call site (`SearchService::search`)
/// unchanged.
pub trait SanitizationStrategy: Send + Sync {
    /// Returns the list of sanitized tokens for the given query.
    fn sanitize(&self, query: &str) -> Vec<String>;

    /// Returns a single FTS5 MATCH expression built from the sanitized
    /// tokens, or `None` if the query produces no tokens.
    fn build_match(&self, query: &str) -> Option<String> {
        let tokens = self.sanitize(query);
        if tokens.is_empty() {
            None
        } else {
            // Whitespace = implicit AND in FTS5. Using whitespace as the
            // joiner (instead of the literal "AND" keyword) keeps
            // user-typed operator words out of the output.
            Some(tokens.join(" "))
        }
    }
}

/// Default sanitization strategy: double-quote each token, filter FTS5
/// operators, allow trailing `*` for prefix search.
pub struct QuoteStrategy;

impl SanitizationStrategy for QuoteStrategy {
    fn sanitize(&self, query: &str) -> Vec<String> {
        sanitize_fts5_query(query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── sanitize_fts5_query — empty / whitespace ───────────────────────

    #[test]
    fn empty_query_returns_empty() {
        assert_eq!(sanitize_fts5_query(""), Vec::<String>::new());
    }

    #[test]
    fn whitespace_only_returns_empty() {
        assert_eq!(sanitize_fts5_query("   "), Vec::<String>::new());
        assert_eq!(sanitize_fts5_query("\t\n  "), Vec::<String>::new());
    }

    // ── sanitize_fts5_query — simple words ─────────────────────────────

    #[test]
    fn simple_word_quoted() {
        assert_eq!(sanitize_fts5_query("hello"), vec!["\"hello\"".to_string()]);
    }

    #[test]
    fn multiple_words_individually_quoted() {
        assert_eq!(
            sanitize_fts5_query("hello world"),
            vec!["\"hello\"".to_string(), "\"world\"".to_string()]
        );
    }

    // ── build_fts5_match_query — AND-joined ────────────────────────────

    #[test]
    fn multiple_words_joined_with_and() {
        // FTS5 treats whitespace between terms as implicit AND, so
        // space-join is semantically equivalent to literal `AND`.
        assert_eq!(
            build_fts5_match_query("hello world"),
            Some("\"hello\" \"world\"".to_string())
        );
    }

    #[test]
    fn build_query_empty_returns_none() {
        assert_eq!(build_fts5_match_query(""), None);
        assert_eq!(build_fts5_match_query("   "), None);
        assert_eq!(build_fts5_match_query("()"), None);
        assert_eq!(build_fts5_match_query("AND OR"), None);
    }

    #[test]
    fn build_query_single_token() {
        assert_eq!(build_fts5_match_query("rust"), Some("\"rust\"".to_string()));
    }

    // ── prefix matching ────────────────────────────────────────────────

    #[test]
    fn prefix_match_preserved() {
        assert_eq!(sanitize_fts5_query("test*"), vec!["\"test\"*".to_string()]);
        assert_eq!(
            build_fts5_match_query("test*"),
            Some("\"test\"*".to_string())
        );
    }

    #[test]
    fn bare_asterisk_produces_literal_wildcard_token() {
        // A bare `*` becomes a literal `"*"` token. FTS5 won't crash;
        // it just matches no rows. We don't try to be clever.
        assert_eq!(build_fts5_match_query("*"), Some("\"*\"".to_string()));
    }

    // ── special character escaping ─────────────────────────────────────

    #[test]
    fn special_chars_escaped() {
        // Internal `"` becomes `'` so the outer quote is balanced.
        // (Leading/trailing `"` are edge-trimmed, so `r#""quoted""#` would
        // lose its surrounding quotes entirely.)
        assert_eq!(sanitize_fts5_query(r#"a"b"#), vec!["\"a'b\"".to_string()]);
        // An embedded `"` in the middle of a longer word is also handled.
        assert_eq!(
            sanitize_fts5_query(r#"foo"bar"#),
            vec!["\"foo'bar\"".to_string()]
        );
    }

    #[test]
    fn edge_punctuation_trimmed() {
        // Leading/trailing FTS5-reserved chars are stripped.
        // Note: `-` and `_` are intentionally NOT trimmed — they are
        // common in compound words (`foo-bar`, `foo_bar`).
        assert_eq!(sanitize_fts5_query("\"foo\""), vec!["\"foo\"".to_string()]);
        assert_eq!(sanitize_fts5_query("(foo)"), vec!["\"foo\"".to_string()]);
        assert_eq!(sanitize_fts5_query("+foo"), vec!["\"foo\"".to_string()]);
        assert_eq!(sanitize_fts5_query("::foo::"), vec!["\"foo\"".to_string()]);
        // Internal punctuation is preserved (the trim only affects edges).
        assert_eq!(
            sanitize_fts5_query("...foo..."),
            vec!["\"foo\"".to_string()]
        );
    }

    // ── FTS5 boolean operators ─────────────────────────────────────────

    #[test]
    fn fts5_operators_removed() {
        let result = build_fts5_match_query("foo AND bar OR baz");
        assert!(result.is_some());
        let s = result.unwrap();
        // No FTS5 operator words survive
        assert!(!s.contains(" AND "), "Should not contain operator: {}", s);
        assert!(!s.contains(" OR "), "Should not contain operator: {}", s);
        // The literal terms ARE present
        assert!(s.contains("\"foo\""));
        assert!(s.contains("\"bar\""));
        assert!(s.contains("\"baz\""));
    }

    #[test]
    fn fts5_operators_case_insensitive() {
        let result = build_fts5_match_query("foo and bar Or baz");
        assert!(result.is_some());
        let s = result.unwrap();
        let lower = s.to_ascii_lowercase();
        assert!(
            !lower.contains(" and "),
            "Should not contain operator: {}",
            s
        );
        assert!(
            !lower.contains(" or "),
            "Should not contain operator: {}",
            s
        );
    }

    #[test]
    fn fts5_operator_words_alone_yield_none() {
        assert_eq!(build_fts5_match_query("AND"), None);
        assert_eq!(build_fts5_match_query("OR"), None);
        assert_eq!(build_fts5_match_query("NOT"), None);
        assert_eq!(build_fts5_match_query("NEAR"), None);
    }

    #[test]
    fn parens_removed() {
        let result = build_fts5_match_query("(foo bar)");
        assert!(result.is_some());
        let s = result.unwrap();
        assert!(!s.contains('('), "Should not contain paren: {}", s);
        assert!(!s.contains(')'), "Should not contain paren: {}", s);
    }

    // ── Injection safety ───────────────────────────────────────────────

    #[test]
    fn fts5_injection_attempts_neutralized() {
        let dangerous = vec![
            "'; DROP TABLE blocks; --",
            "foo\" OR 1=1 --",
            "MATCH()",
            "*\"'",
            "\"unterminated",
            "(broken",
            "foo AND",
            "\"",
            "*",
            "1; SELECT * FROM blocks;",
            "' OR '1'='1",
            "\"; DROP TABLE pages; --",
            "../etc/passwd",
            "<script>alert('xss')</script>",
        ];
        for q in dangerous {
            let result = build_fts5_match_query(q);
            // Should never crash, should always return Some or None
            if let Some(s) = result {
                // All double-quotes must be balanced (the FTS5 parser
                // rejects unbalanced strings with "unterminated string").
                let quote_count = s.chars().filter(|c| *c == '"').count();
                assert_eq!(quote_count % 2, 0, "Unbalanced quotes in: {}", s);
                // No standalone FTS5 boolean operator words may survive.
                // (Parens / brackets that are INSIDE quoted string
                // literals are fine — FTS5 won't interpret them as
                // operators — so we don't check for them here.)
                for op in [" AND ", " OR ", " NOT ", " NEAR "] {
                    assert!(!s.contains(op), "Operator word {} leaked: {}", op, s);
                }
            }
        }
    }

    // ── Unicode preservation ──────────────────────────────────────────

    #[test]
    fn unicode_preserved() {
        assert_eq!(sanitize_fts5_query("café"), vec!["\"café\"".to_string()]);
        assert_eq!(
            sanitize_fts5_query("日本語"),
            vec!["\"日本語\"".to_string()]
        );
        assert_eq!(sanitize_fts5_query("🎉"), vec!["\"🎉\"".to_string()]);
        assert_eq!(sanitize_fts5_query("über"), vec!["\"über\"".to_string()]);
    }

    // ── Compound words ─────────────────────────────────────────────────

    #[test]
    fn hyphens_underscores_preserved() {
        assert_eq!(
            sanitize_fts5_query("foo-bar"),
            vec!["\"foo-bar\"".to_string()]
        );
        assert_eq!(
            sanitize_fts5_query("foo_bar"),
            vec!["\"foo_bar\"".to_string()]
        );
        assert_eq!(
            sanitize_fts5_query("foo-bar_baz"),
            vec!["\"foo-bar_baz\"".to_string()]
        );
    }

    // ── OCP: strategy trait ────────────────────────────────────────────

    #[test]
    fn quote_strategy_produces_quoted_tokens() {
        let strategy = QuoteStrategy;
        assert_eq!(strategy.sanitize("foo"), vec!["\"foo\"".to_string()]);
        assert_eq!(
            strategy.build_match("foo bar"),
            Some("\"foo\" \"bar\"".to_string())
        );
    }

    #[test]
    fn strategy_default_build_match_matches_free_function() {
        let strategy = QuoteStrategy;
        assert_eq!(
            strategy.build_match("hello world"),
            build_fts5_match_query("hello world"),
        );
    }

    #[test]
    fn strategy_is_object_safe() {
        // Sanity check: trait is dyn-compatible (no `Self` in return types)
        let _: Box<dyn SanitizationStrategy> = Box::new(QuoteStrategy);
    }
}
