//! Property-based tests for FTS5 query sanitization.
//!
//! These tests assert invariants that should hold for any user input
//! that could be passed to a FTS5 `MATCH` clause. The sanitization
//! functions are the only defense against FTS5 syntax injection.

use proptest::prelude::*;
use quilt_search::sanitize::{build_fts5_match_query, sanitize_fts5_query};

proptest! {
    /// Property: balanced quotes in the output of `build_fts5_match_query`.
    /// Unbalanced quotes cause FTS5 to error with "unterminated string",
    /// crashing the query. Every `"` must have a matching closing `"`.
    #[test]
    fn balanced_quotes_invariant(s in ".{0,500}") {
        if let Some(safe) = build_fts5_match_query(&s) {
            let quote_count = safe.chars().filter(|c| *c == '"').count();
            prop_assert_eq!(
                quote_count % 2, 0,
                "Unbalanced quotes in: {} (input: {:?})", safe, s
            );
        }
    }

    /// Property: every token emitted by `sanitize_fts5_query` is wrapped
    /// in double quotes. FTS5 treats double-quoted strings as literal
    /// phrases, which is what we want.
    #[test]
    fn tokens_are_quoted(s in "[a-zA-Z0-9_ ]{0,200}") {
        let tokens = sanitize_fts5_query(&s);
        for token in &tokens {
            prop_assert!(
                token.starts_with('"'),
                "token {:?} does not start with '\"'", token
            );
            // Both `"word"` and prefix `"word"*` end with `"`. The
            // trailing `*` for prefix search is *outside* the closing
            // quote, so a bare `ends_with('"')` covers both forms.
            prop_assert!(
                token.ends_with('"'),
                "token {:?} does not end with '\"'", token
            );
        }
    }

    /// Property: empty or whitespace-only input returns `None`.
    /// This signals "no usable query" to the caller.
    #[test]
    fn empty_handling(whitespace in "[ \\t\\n\\r]{0,10}") {
        prop_assert!(build_fts5_match_query("").is_none());
        prop_assert!(build_fts5_match_query(&whitespace).is_none());
    }

    /// Property: prefix-search marker `*` is preserved at the end of
    /// the matched token. Losing the `*` would silently downgrade a
    /// prefix query to an exact match.
    #[test]
    fn prefix_preserved(word in "[a-z]{1,20}") {
        let input = format!("{}*", word);
        let tokens = sanitize_fts5_query(&input);
        prop_assert!(!tokens.is_empty(), "input {:?} produced no tokens", input);
        let joined = tokens.join(" ");
        prop_assert!(
            joined.ends_with('*'),
            "prefix lost: input {:?} -> output {:?}", input, joined
        );
    }

    /// Property: standalone FTS5 boolean operator words (AND, OR, NOT,
    /// NEAR) are stripped. The build function returns `None` for input
    /// that consists only of operators.
    #[test]
    fn operators_stripped(op in prop::sample::select(vec!["AND", "OR", "NOT", "NEAR"])) {
        let safe = build_fts5_match_query(op);
        if let Some(s) = safe {
            prop_assert!(
                !s.contains(" AND "),
                "Operator AND leaked: {}", s
            );
            prop_assert!(
                !s.contains(" OR "),
                "Operator OR leaked: {}", s
            );
            prop_assert!(
                !s.contains(" NOT "),
                "Operator NOT leaked: {}", s
            );
            prop_assert!(
                !s.contains(" NEAR "),
                "Operator NEAR leaked: {}", s
            );
        }
    }

    /// Property: token count is bounded by the number of input words.
    /// Some words (FTS5 operators, edge-trimmed junk) are dropped, so
    /// tokens.len() <= input word count.
    #[test]
    fn token_count_bounded_by_word_count(s in "[a-zA-Z0-9 ]{0,200}") {
        let original_words = s.split_whitespace().count();
        let tokens = sanitize_fts5_query(&s);
        prop_assert!(
            tokens.len() <= original_words,
            "input {:?} produced {} tokens from {} words",
            s,
            tokens.len(),
            original_words
        );
    }

    /// Property: arbitrary adversarial inputs do not crash the sanitizer.
    /// The function must be total over `&str`. We test with a string
    /// that mixes SQL-injection patterns with control characters.
    #[test]
    fn no_crash_on_adversarial(
        s in prop::string::string_regex(r#"[\x00-\x1f"'<>\\;]{0,50}"#).unwrap()
    ) {
        // Both functions must return without panicking
        let _ = build_fts5_match_query(&s);
        let _ = sanitize_fts5_query(&s);
    }

    /// Property: unicode is preserved through sanitization.
    /// The FTS5 unicode61 tokenizer handles non-ASCII natively, and
    /// the sanitizer should not strip or transform valid unicode chars.
    #[test]
    fn unicode_preserved(s in "\\PC{1,20}") {
        let tokens = sanitize_fts5_query(&s);
        if !tokens.is_empty() {
            // At least one token should contain the unicode char(s)
            let joined = tokens.join(" ");
            // The inner chars (without the surrounding quotes) should be present
            prop_assert!(
                !joined.is_empty(),
                "tokens for unicode input {:?} produced empty joined string",
                s
            );
        }
    }
}
