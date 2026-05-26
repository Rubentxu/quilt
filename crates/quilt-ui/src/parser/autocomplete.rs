//! Autocomplete abstractions for inline content.
//!
//! Provides the base types and service abstractions for autocompleting
//! `[[Page]]`, `((Block))`, `#tag`, and `property::` triggers.
//!
//! This module defines the **contract** between the editor layer and
//! the data layer — providers are implemented separately for each data
//! source (pages, blocks, tags, properties).
//!
//! No embedded agents or LLMs. Data sources are domain repositories
//! accessed via the bridge.
//!
//! All methods are synchronous at this abstraction level. Async bridge
//! calls are dispatched by the caller (e.g., a Leptos component using
//! `spawn_local`).

/// What triggered the autocomplete and what prefix to search for.
#[derive(Debug, Clone, PartialEq)]
pub enum AutocompleteTrigger {
    /// User typed `[[partial`
    PageRef { prefix: String },
    /// User typed `((partial`
    BlockRef { prefix: String },
    /// User typed `#partial`
    Tag { prefix: String },
    /// User typed `key:: partial` (or `key::` with partial value)
    PropertyValue { key: String, prefix: String },
}

/// A single autocomplete suggestion item.
#[derive(Debug, Clone, PartialEq)]
pub struct AutocompleteItem {
    /// Display text shown in the dropdown
    pub label: String,
    /// Text to insert when selected
    pub insert_text: String,
    /// Optional description or preview
    pub description: Option<String>,
    /// Category for grouping in the UI
    pub category: AutocompleteCategory,
}

/// Category of autocomplete items for UI grouping.
#[derive(Debug, Clone, PartialEq)]
pub enum AutocompleteCategory {
    Page,
    Block,
    Tag,
    Property,
    PropertyValue,
    SlashCommand,
}

/// Result of an autocomplete request.
#[derive(Debug, Clone, Default)]
pub struct AutocompleteResult {
    /// The trigger that produced this result
    pub trigger: Option<AutocompleteTrigger>,
    /// Suggested items
    pub items: Vec<AutocompleteItem>,
}

impl AutocompleteResult {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

/// Provider of autocomplete suggestions for a specific trigger type.
///
/// Implementations are synchronous and stateless. For async bridge access,
/// cache or preload data before calling into the suggestion path.
pub trait AutocompleteProvider: Send + Sync {
    /// The kind of trigger this provider handles.
    fn can_handle(&self, trigger: &AutocompleteTrigger) -> bool;

    /// Return suggestions for the given trigger.
    ///
    /// Implementations should return results quickly — this is called
    /// on every keystroke while the user types. For expensive operations,
    /// preload/cache data and filter here.
    fn suggest(&self, trigger: &AutocompleteTrigger) -> Vec<AutocompleteItem>;

    /// Human-readable name for this provider (for debugging).
    fn name(&self) -> &str;
}

/// Detect an autocomplete trigger at a cursor position in content.
///
/// Scans backward from the cursor to find trigger patterns:
/// - `[[prefix` → PageRef trigger
/// - `((prefix` → BlockRef trigger
/// - `#prefix` → Tag trigger
/// - `key:: prefix` → PropertyValue trigger
///
/// Returns `None` if no trigger pattern is found.
pub fn detect_trigger(content: &str, cursor_pos: usize) -> Option<AutocompleteTrigger> {
    if content.is_empty() || cursor_pos == 0 {
        return None;
    }

    let cursor = cursor_pos.min(content.len());

    // Look backward from cursor for trigger patterns
    let before = &content[..cursor];

    // Check for [[ prefix
    if let Some(double_bracket) = before.rfind("[[") {
        // Ensure no closing ]] between [[ and cursor
        let candidate = &before[double_bracket + 2..];
        if !candidate.contains("]]") {
            let prefix = candidate.to_string();
            return Some(AutocompleteTrigger::PageRef { prefix });
        }
    }

    // Check for (( prefix (block refs)
    if let Some(double_paren) = before.rfind("((") {
        let candidate = &before[double_paren + 2..];
        if !candidate.contains("))") {
            let prefix = candidate.to_string();
            return Some(AutocompleteTrigger::BlockRef { prefix });
        }
    }

    // Check for # prefix (tags)
    if let Some(hash) = before.rfind('#') {
        // # should be at word start or preceded by whitespace
        if hash == 0 || before.as_bytes()[hash - 1] == b' ' {
            let candidate = &before[hash + 1..];
            // Only valid tag characters
            if !candidate.is_empty()
                && candidate
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            {
                let prefix = candidate.to_string();
                return Some(AutocompleteTrigger::Tag { prefix });
            }
        }
    }

    // Check for property:: prefix
    // Look for word:: at the end of before
    if let Some(colon_colon) = before.rfind("::") {
        let before_colons = &before[..colon_colon];
        // Find the key/word before ::
        let key_start = before_colons
            .rfind(|c: char| c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(0);
        let key = &before_colons[key_start..];
        if !key.is_empty()
            && key
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            let value_prefix = &before[colon_colon + 2..];
            return Some(AutocompleteTrigger::PropertyValue {
                key: key.to_string(),
                prefix: value_prefix.to_string(),
            });
        }
    }

    None
}

/// A compiled collection of providers that dispatches to the right one.
///
/// In the current phase, this is a foundation abstraction. Concrete
/// providers will be added as the autocomplete UI is built out.
#[derive(Default)]
pub struct AutocompleteService {
    providers: Vec<Box<dyn AutocompleteProvider>>,
}

impl AutocompleteService {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Register a provider.
    pub fn register(&mut self, provider: Box<dyn AutocompleteProvider>) {
        self.providers.push(provider);
    }

    /// Get suggestions for a trigger by dispatching to matching providers.
    pub fn suggest(&self, trigger: &AutocompleteTrigger) -> AutocompleteResult {
        let mut result = AutocompleteResult {
            trigger: Some(trigger.clone()),
            items: Vec::new(),
        };

        for provider in &self.providers {
            if provider.can_handle(trigger) {
                let mut items = provider.suggest(trigger);
                result.items.append(&mut items);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── detect_trigger tests ──

    #[test]
    fn test_detect_page_ref_trigger() {
        let trigger = detect_trigger("see [[Proj", 12);
        assert!(trigger.is_some(), "Should detect PageRef trigger after [[");
        if let Some(AutocompleteTrigger::PageRef { prefix }) = trigger {
            assert_eq!(prefix, "Proj");
        } else {
            panic!("Expected PageRef trigger, got {:?}", trigger);
        }
    }

    #[test]
    fn test_detect_page_ref_closed_no_trigger() {
        // If [[ is closed with ]], no autocomplete
        let trigger = detect_trigger("[[Project]]", 12);
        assert!(trigger.is_none(), "Closed [[ should not trigger");
    }

    #[test]
    fn test_detect_page_ref_at_start() {
        let trigger = detect_trigger("[[Proj", 6);
        assert!(trigger.is_some());
        if let Some(AutocompleteTrigger::PageRef { prefix }) = trigger {
            assert_eq!(prefix, "Proj");
        } else {
            panic!("Expected PageRef trigger");
        }
    }

    #[test]
    fn test_detect_block_ref_trigger() {
        let trigger = detect_trigger("see ((550e", 11);
        assert!(trigger.is_some(), "Should detect BlockRef trigger after ((");
        if let Some(AutocompleteTrigger::BlockRef { prefix }) = trigger {
            assert_eq!(prefix, "550e");
        } else {
            panic!("Expected BlockRef trigger, got {:?}", trigger);
        }
    }

    #[test]
    fn test_detect_tag_trigger() {
        let trigger = detect_trigger("meeting #ur", 12);
        assert!(trigger.is_some(), "Should detect Tag trigger after #");
        if let Some(AutocompleteTrigger::Tag { prefix }) = trigger {
            assert_eq!(prefix, "ur");
        } else {
            panic!("Expected Tag trigger, got {:?}", trigger);
        }
    }

    #[test]
    fn test_detect_tag_at_start() {
        let trigger = detect_trigger("#urg", 4);
        assert!(trigger.is_some());
        if let Some(AutocompleteTrigger::Tag { prefix }) = trigger {
            assert_eq!(prefix, "urg");
        }
    }

    #[test]
    fn test_detect_property_value_trigger() {
        let trigger = detect_trigger("status:: act", 13);
        assert!(
            trigger.is_some(),
            "Should detect PropertyValue trigger after ::"
        );
        if let Some(AutocompleteTrigger::PropertyValue { key, prefix }) = trigger {
            assert_eq!(key, "status");
            assert_eq!(prefix, " act");
        } else {
            panic!("Expected PropertyValue trigger, got {:?}", trigger);
        }
    }

    #[test]
    fn test_detect_no_trigger_empty() {
        assert!(detect_trigger("", 0).is_none());
    }

    #[test]
    fn test_detect_no_trigger_plain() {
        assert!(detect_trigger("hello world", 11).is_none());
    }

    #[test]
    fn test_detect_no_trigger_at_start_no_pattern() {
        assert!(detect_trigger("hello", 0).is_none());
    }

    // ── AutocompleteService + Provider tests ──

    struct TestPageProvider;

    impl AutocompleteProvider for TestPageProvider {
        fn can_handle(&self, trigger: &AutocompleteTrigger) -> bool {
            matches!(trigger, AutocompleteTrigger::PageRef { .. })
        }

        fn suggest(&self, trigger: &AutocompleteTrigger) -> Vec<AutocompleteItem> {
            match trigger {
                AutocompleteTrigger::PageRef { prefix } => {
                    ["Project Alpha", "Project Beta", "Notes"]
                        .iter()
                        .filter(|name| name.to_lowercase().contains(&prefix.to_lowercase()))
                        .map(|name| AutocompleteItem {
                            label: name.to_string(),
                            insert_text: name.to_string(),
                            description: None,
                            category: AutocompleteCategory::Page,
                        })
                        .collect()
                }
                _ => vec![],
            }
        }

        fn name(&self) -> &str {
            "TestPageProvider"
        }
    }

    #[test]
    fn test_provider_suggestions() {
        let provider = TestPageProvider;

        let trigger = AutocompleteTrigger::PageRef {
            prefix: "pro".to_string(),
        };
        assert!(provider.can_handle(&trigger));

        let items = provider.suggest(&trigger);
        assert_eq!(
            items.len(),
            2,
            "Should match 'Project Alpha' and 'Project Beta'"
        );
        assert!(items.iter().any(|i| i.label == "Project Alpha"));
        assert!(items.iter().any(|i| i.label == "Project Beta"));
    }

    #[test]
    fn test_provider_no_match() {
        let provider = TestPageProvider;

        let trigger = AutocompleteTrigger::Tag {
            prefix: "urgent".to_string(),
        };
        assert!(!provider.can_handle(&trigger));
    }

    #[test]
    fn test_autocomplete_service_dispatch() {
        let mut service = AutocompleteService::new();
        service.register(Box::new(TestPageProvider));

        let trigger = AutocompleteTrigger::PageRef {
            prefix: "pro".to_string(),
        };
        let result = service.suggest(&trigger);

        assert!(!result.is_empty());
        assert_eq!(result.trigger, Some(trigger));
        assert_eq!(result.items.len(), 2);
    }

    #[test]
    fn test_autocomplete_service_no_match() {
        let service = AutocompleteService::new();

        let trigger = AutocompleteTrigger::Tag {
            prefix: "urgent".to_string(),
        };
        let result = service.suggest(&trigger);

        assert!(result.is_empty());
        assert_eq!(result.trigger, Some(trigger));
    }

    #[test]
    fn test_detect_trigger_cursor_boundary() {
        // Cursor at end of content with no trigger
        assert!(detect_trigger("hello", 5).is_none());

        // Cursor beyond content length (clamped)
        assert!(detect_trigger("[[ab", 10).is_some());
    }

    #[test]
    fn test_detect_trigger_mixed_content() {
        // Content with multiple potential triggers — only the one nearest cursor
        let trigger = detect_trigger("[[Done]] and #ur", 17);
        assert!(trigger.is_some());
        if let Some(AutocompleteTrigger::Tag { prefix }) = trigger {
            assert_eq!(prefix, "ur");
        } else {
            panic!("Expected Tag trigger nearest cursor");
        }
    }

    #[test]
    fn test_autocomplete_result_helpers() {
        let empty = AutocompleteResult::default();
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);

        let result = AutocompleteResult {
            trigger: None,
            items: vec![AutocompleteItem {
                label: "test".to_string(),
                insert_text: "test".to_string(),
                description: None,
                category: AutocompleteCategory::Tag,
            }],
        };
        assert!(!result.is_empty());
        assert_eq!(result.len(), 1);
    }
}
