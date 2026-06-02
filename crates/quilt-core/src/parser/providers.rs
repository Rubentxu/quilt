//! Concrete autocomplete providers for the Quilt outliner.
//!
//! Provides implementations of `AutocompleteProvider` for page refs,
//! tags, and property values. These are the default providers used
//! by the autocomplete dropdown UI.
//!
//! All providers are stateless, deterministic, and pure — they only
//! filter pre-loaded data against the trigger prefix. No I/O, no
//! side effects, no embedded agents.

use crate::parser::autocomplete::{
    AutocompleteCategory, AutocompleteItem, AutocompleteProvider, AutocompleteTrigger,
};

// ── PageRefProvider ──

/// Suggests page names for `[[prefix` triggers.
///
/// Data must be pre-loaded from the bridge before creating this provider.
/// The provider itself is synchronous — it only filters the list.
pub struct PageRefProvider {
    pages: Vec<String>,
}

impl PageRefProvider {
    pub fn new(pages: Vec<String>) -> Self {
        Self { pages }
    }
}

impl AutocompleteProvider for PageRefProvider {
    fn can_handle(&self, trigger: &AutocompleteTrigger) -> bool {
        matches!(trigger, AutocompleteTrigger::PageRef { .. })
    }

    fn suggest(&self, trigger: &AutocompleteTrigger) -> Vec<AutocompleteItem> {
        match trigger {
            AutocompleteTrigger::PageRef { prefix } => {
                let lower = prefix.to_lowercase();
                self.pages
                    .iter()
                    .filter(|name| name.to_lowercase().contains(&lower))
                    .map(|name| AutocompleteItem {
                        label: name.clone(),
                        insert_text: name.clone(),
                        description: None,
                        category: AutocompleteCategory::Page,
                    })
                    .collect()
            }
            _ => vec![],
        }
    }

    fn name(&self) -> &str {
        "PageRefProvider"
    }
}

// ── TagProvider ──

/// Suggests tag names for `#prefix` triggers.
///
/// Can be constructed with a custom tag list or with sensible defaults.
pub struct TagProvider {
    tags: Vec<String>,
}

impl TagProvider {
    pub fn new(tags: Vec<String>) -> Self {
        Self { tags }
    }

    /// Create with common default tags used across Quilt projects.
    pub fn with_defaults() -> Self {
        Self {
            tags: vec![
                "todo".into(),
                "doing".into(),
                "done".into(),
                "bug".into(),
                "feature".into(),
                "urgent".into(),
                "wip".into(),
                "blocked".into(),
                "review".into(),
                "meeting".into(),
                "idea".into(),
                "question".into(),
                "note".into(),
                "reference".into(),
            ],
        }
    }
}

impl AutocompleteProvider for TagProvider {
    fn can_handle(&self, trigger: &AutocompleteTrigger) -> bool {
        matches!(trigger, AutocompleteTrigger::Tag { .. })
    }

    fn suggest(&self, trigger: &AutocompleteTrigger) -> Vec<AutocompleteItem> {
        match trigger {
            AutocompleteTrigger::Tag { prefix } => {
                let lower = prefix.to_lowercase();
                self.tags
                    .iter()
                    .filter(|name| name.to_lowercase().contains(&lower))
                    .map(|name| AutocompleteItem {
                        label: format!("#{}", name),
                        insert_text: name.clone(),
                        description: None,
                        category: AutocompleteCategory::Tag,
                    })
                    .collect()
            }
            _ => vec![],
        }
    }

    fn name(&self) -> &str {
        "TagProvider"
    }
}

// ── PropertyValueProvider ──

/// Suggests typed values for known property keys (status, priority).
///
/// Currently supports:
/// - `status` → TODO, DOING, DONE
/// - `priority` → A, B, C
///
/// These are the v1 first-class properties from the outliner baseline.
pub struct PropertyValueProvider {
    /// Map from property key to its allowed values.
    values_by_key: Vec<(String, Vec<String>)>,
}

impl Default for PropertyValueProvider {
    fn default() -> Self {
        Self {
            values_by_key: vec![
                (
                    "status".into(),
                    vec!["TODO".into(), "DOING".into(), "DONE".into()],
                ),
                ("priority".into(), vec!["A".into(), "B".into(), "C".into()]),
            ],
        }
    }
}

impl PropertyValueProvider {
    pub fn new() -> Self {
        Self::default()
    }
}

impl AutocompleteProvider for PropertyValueProvider {
    fn can_handle(&self, trigger: &AutocompleteTrigger) -> bool {
        matches!(trigger, AutocompleteTrigger::PropertyValue { .. })
    }

    fn suggest(&self, trigger: &AutocompleteTrigger) -> Vec<AutocompleteItem> {
        match trigger {
            AutocompleteTrigger::PropertyValue { key, prefix } => {
                let lower_key = key.to_lowercase();
                let lower_prefix = prefix.trim().to_lowercase();

                for (known_key, values) in &self.values_by_key {
                    if known_key.as_str() == lower_key {
                        let matched: Vec<AutocompleteItem> = values
                            .iter()
                            .filter(|v| {
                                lower_prefix.is_empty() || v.to_lowercase().contains(&lower_prefix)
                            })
                            .map(|v| AutocompleteItem {
                                label: format!("{}:: {}", key, v),
                                insert_text: v.clone(),
                                description: None,
                                category: AutocompleteCategory::PropertyValue,
                            })
                            .collect();
                        return matched;
                    }
                }

                vec![]
            }
            _ => vec![],
        }
    }

    fn name(&self) -> &str {
        "PropertyValueProvider"
    }
}

/// Helper to build a pre-configured autocomplete service with all standard providers.
///
/// This is the recommended way to set up autocomplete for the editor.
/// Pass the page list from bridge::list_pages().
pub fn create_default_service(page_names: Vec<String>) -> super::autocomplete::AutocompleteService {
    let mut service = super::autocomplete::AutocompleteService::new();
    service.register(Box::new(PageRefProvider::new(page_names)));
    service.register(Box::new(TagProvider::with_defaults()));
    service.register(Box::new(PropertyValueProvider::new()));
    service
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── PageRefProvider Tests ──

    #[test]
    fn test_page_ref_provider_filters_by_prefix() {
        let provider = PageRefProvider::new(vec![
            "Project Alpha".into(),
            "Project Beta".into(),
            "Personal Notes".into(),
        ]);

        let trigger = AutocompleteTrigger::PageRef {
            prefix: "pro".into(),
        };
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
    fn test_page_ref_provider_no_match() {
        let provider = PageRefProvider::new(vec!["Project Alpha".into()]);

        let trigger = AutocompleteTrigger::PageRef {
            prefix: "zzzz".into(),
        };
        let items = provider.suggest(&trigger);

        assert!(items.is_empty(), "No pages should match 'zzzz'");
    }

    #[test]
    fn test_page_ref_provider_case_insensitive() {
        let provider = PageRefProvider::new(vec!["Project Alpha".into()]);

        let trigger = AutocompleteTrigger::PageRef {
            prefix: "ALPHA".into(),
        };
        let items = provider.suggest(&trigger);

        assert_eq!(items.len(), 1, "Should match case-insensitively");
    }

    #[test]
    fn test_page_ref_provider_wont_handle_non_page_trigger() {
        let provider = PageRefProvider::new(vec![]);

        assert!(!provider.can_handle(&AutocompleteTrigger::Tag {
            prefix: "test".into()
        }));
        assert!(provider.can_handle(&AutocompleteTrigger::PageRef { prefix: "".into() }));
    }

    // ── TagProvider Tests ──

    #[test]
    fn test_tag_provider_defaults() {
        let provider = TagProvider::with_defaults();

        let trigger = AutocompleteTrigger::Tag {
            prefix: "bug".into(),
        };
        let items = provider.suggest(&trigger);

        assert_eq!(items.len(), 1, "Should match 'bug'");
        assert_eq!(items[0].label, "#bug");
    }

    #[test]
    fn test_tag_provider_partial_match() {
        let provider = TagProvider::with_defaults();

        let trigger = AutocompleteTrigger::Tag {
            prefix: "do".into(),
        };
        let items = provider.suggest(&trigger);

        assert!(!items.is_empty(), "Should match tags containing 'do'");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"#todo"), "Should include #todo");
        assert!(labels.contains(&"#doing"), "Should include #doing");
        assert!(labels.contains(&"#done"), "Should include #done");
    }

    #[test]
    fn test_tag_provider_empty_prefix_shows_all() {
        let provider = TagProvider::with_defaults();

        let trigger = AutocompleteTrigger::Tag { prefix: "".into() };
        let items = provider.suggest(&trigger);

        assert!(!items.is_empty(), "Empty prefix should show all tags");
    }

    #[test]
    fn test_tag_provider_custom_list() {
        let provider = TagProvider::new(vec!["custom1".into(), "custom2".into()]);

        let trigger = AutocompleteTrigger::Tag {
            prefix: "custom".into(),
        };
        let items = provider.suggest(&trigger);

        assert_eq!(items.len(), 2);
    }

    // ── PropertyValueProvider Tests ──

    #[test]
    fn test_property_value_provider_status_values() {
        let provider = PropertyValueProvider::new();

        let trigger = AutocompleteTrigger::PropertyValue {
            key: "status".into(),
            prefix: "".into(),
        };
        let items = provider.suggest(&trigger);

        assert_eq!(items.len(), 3, "Status should have 3 values");
        assert!(items.iter().any(|i| i.insert_text == "TODO"));
        assert!(items.iter().any(|i| i.insert_text == "DOING"));
        assert!(items.iter().any(|i| i.insert_text == "DONE"));
    }

    #[test]
    fn test_property_value_provider_filters_by_prefix() {
        let provider = PropertyValueProvider::new();

        // "DO" matches TODO + DOING + DONE (all contain "do")
        let trigger = AutocompleteTrigger::PropertyValue {
            key: "status".into(),
            prefix: "DO".into(),
        };
        let items = provider.suggest(&trigger);

        assert_eq!(
            items.len(),
            3,
            "Should match TODO, DOING, and DONE (all contain 'DO')"
        );
        assert!(items.iter().any(|i| i.insert_text == "TODO"));
        assert!(items.iter().any(|i| i.insert_text == "DOING"));
        assert!(items.iter().any(|i| i.insert_text == "DONE"));
    }

    #[test]
    fn test_property_value_provider_filters_specific_prefix() {
        let provider = PropertyValueProvider::new();

        // "DOIN" only matches DOING
        let trigger = AutocompleteTrigger::PropertyValue {
            key: "status".into(),
            prefix: "DOIN".into(),
        };
        let items = provider.suggest(&trigger);

        assert_eq!(items.len(), 1, "Should match only DOING");
        assert_eq!(items[0].insert_text, "DOING");
    }

    #[test]
    fn test_property_value_provider_priority_values() {
        let provider = PropertyValueProvider::new();

        let trigger = AutocompleteTrigger::PropertyValue {
            key: "priority".into(),
            prefix: "A".into(),
        };
        let items = provider.suggest(&trigger);

        assert_eq!(items.len(), 1, "Priority A should match");
        assert_eq!(items[0].insert_text, "A");
    }

    #[test]
    fn test_property_value_provider_unknown_key_returns_empty() {
        let provider = PropertyValueProvider::new();

        let trigger = AutocompleteTrigger::PropertyValue {
            key: "unknown_prop".into(),
            prefix: "test".into(),
        };
        let items = provider.suggest(&trigger);

        assert!(items.is_empty(), "Unknown property key should return empty");
    }

    // ── create_default_service Integration ──

    #[test]
    fn test_create_default_service_page_ref() {
        let service = create_default_service(vec!["Home".into(), "Projects".into()]);

        let trigger = AutocompleteTrigger::PageRef {
            prefix: "hom".into(),
        };
        let result = service.suggest(&trigger);

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].label, "Home");
    }

    #[test]
    fn test_create_default_service_tag() {
        let service = create_default_service(vec![]);

        let trigger = AutocompleteTrigger::Tag {
            prefix: "urg".into(),
        };
        let result = service.suggest(&trigger);

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].label, "#urgent");
    }

    #[test]
    fn test_create_default_service_property() {
        let service = create_default_service(vec![]);

        let trigger = AutocompleteTrigger::PropertyValue {
            key: "status".into(),
            prefix: "done".into(),
        };
        let result = service.suggest(&trigger);

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].insert_text, "DONE");
    }

    #[test]
    fn test_create_default_service_unknown_trigger() {
        let service = create_default_service(vec![]);

        // BlockRef is not yet handled by any default provider
        let trigger = AutocompleteTrigger::BlockRef {
            prefix: "550e".into(),
        };
        let result = service.suggest(&trigger);

        assert!(
            result.is_empty(),
            "BlockRef should return empty (no provider registered)"
        );
    }
}
