//! Navigation items and registry for sidebar extension

/// A navigation item displayed in the sidebar
pub struct NavItem {
    /// Unique identifier (kebab-case recommended)
    pub id: String,
    /// Display label
    pub label: String,
    /// Emoji or short text icon
    pub icon: String,
    /// Target href (must start with `/`)
    pub href: String,
    /// Optional custom active state predicate
    pub active_if: Option<Box<dyn Fn(&str) -> bool + Send + Sync>>,
}

impl NavItem {
    /// Check if this nav item is active for a given path
    pub fn is_active(&self, path: &str) -> bool {
        if let Some(predicate) = &self.active_if {
            predicate(path)
        } else {
            path == self.href
        }
    }
}

/// Registry for navigation items
#[derive(Default)]
pub struct NavRegistry {
    items: Vec<NavItem>,
}

impl NavRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a navigation item
    ///
    /// If an item with the same `id` already exists, the new item is skipped.
    pub fn register_nav_item(&mut self, item: NavItem) {
        // Validate href starts with /
        if !item.href.starts_with('/') {
            panic!(
                "NavItem href must start with '/', got: {}",
                item.href
            );
        }

        // Skip duplicates
        if self.items.iter().any(|i| i.id == item.id) {
            tracing::warn!(
                "Skipping duplicate NavItem with id: {}, label: {}",
                item.id,
                item.label
            );
            return;
        }

        self.items.push(item);
    }

    /// Get all registered navigation items in registration order
    pub fn items(&self) -> &[NavItem] {
        &self.items
    }

    /// Find a navigation item by its href
    pub fn find_by_href(&self, href: &str) -> Option<&NavItem> {
        self.items.iter().find(|i| i.href == href)
    }

    /// Find a navigation item by its id
    pub fn find_by_id(&self, id: &str) -> Option<&NavItem> {
        self.items.iter().find(|i| i.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_nav(id: &str, href: &str) -> NavItem {
        NavItem {
            id: id.into(),
            label: id.into(),
            icon: "📄".into(),
            href: href.into(),
            active_if: None,
        }
    }

    #[test]
    fn test_register_nav_item() {
        let mut registry = NavRegistry::new();
        registry.register_nav_item(make_nav("test", "/test"));

        assert_eq!(registry.items().len(), 1);
        assert_eq!(registry.items()[0].id, "test");
    }

    #[test]
    fn test_duplicate_skipped() {
        let mut registry = NavRegistry::new();
        registry.register_nav_item(make_nav("test", "/test"));
        registry.register_nav_item(make_nav("test", "/test2"));

        assert_eq!(registry.items().len(), 1);
        assert_eq!(registry.items()[0].href, "/test");
    }

    #[test]
    fn test_find_by_href() {
        let mut registry = NavRegistry::new();
        registry.register_nav_item(make_nav("test", "/test"));

        let found = registry.find_by_href("/test");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "test");
    }

    #[test]
    fn test_active_if_none_exact_match() {
        let nav = make_nav("test", "/test");

        assert!(nav.is_active("/test"));
        assert!(!nav.is_active("/test-other"));
    }

    #[test]
    fn test_active_if_custom_predicate() {
        let nav = NavItem {
            id: "prefix".into(),
            label: "Prefix".into(),
            icon: "📂".into(),
            href: "/prefix".into(),
            active_if: Some(Box::new(|path| path.starts_with("/prefix"))),
        };

        assert!(nav.is_active("/prefix"));
        assert!(nav.is_active("/prefix/deep"));
        assert!(!nav.is_active("/other"));
    }
}
