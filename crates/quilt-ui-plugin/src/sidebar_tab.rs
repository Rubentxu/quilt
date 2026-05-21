//! Sidebar tabs and registry for right sidebar extension

/// A tab displayed in the right sidebar
///
/// v1 uses string identifiers — the UI maps IDs to actual components
#[derive(Clone)]
pub struct SidebarTab {
    /// Unique identifier
    pub id: String,
    /// Display label
    pub label: String,
    /// Emoji or short text icon
    pub icon: String,
    /// Panel identifier — UI maps this to actual component
    pub panel_id: String,
    /// Priority for sorting (higher = first)
    pub priority: u8,
}

/// Registry for sidebar tabs (currently global only, per-block in v2)
#[derive(Default)]
pub struct SidebarTabRegistry {
    /// Tabs registered for global scope
    global_tabs: Vec<SidebarTab>,
}

impl SidebarTabRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a sidebar tab for the global scope
    ///
    /// If a tab with the same `id` already exists, the new tab is skipped.
    pub fn register_sidebar_tab(&mut self, tab: SidebarTab) {
        // Skip duplicates
        if self.global_tabs.iter().any(|t| t.id == tab.id) {
            tracing::warn!(
                "Skipping duplicate SidebarTab with id: {}, label: {}",
                tab.id,
                tab.label
            );
            return;
        }

        self.global_tabs.push(tab);
    }

    /// Get all global tabs in registration order
    pub fn global_tabs(&self) -> &[SidebarTab] {
        &self.global_tabs
    }

    /// Get all global tabs sorted by priority (descending)
    pub fn sorted_global(&self) -> Vec<&SidebarTab> {
        let mut tabs: Vec<_> = self.global_tabs.iter().collect();
        tabs.sort_by(|a, b| b.priority.cmp(&a.priority));
        tabs
    }

    /// Get tabs for a specific scope (currently only "global" supported in v1)
    pub fn tabs_for(&self, scope: &str) -> &[SidebarTab] {
        if scope == "global" {
            self.global_tabs()
        } else {
            // v2 could look up scope-specific tabs from a BTreeMap
            &[]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_tab() {
        let mut registry = SidebarTabRegistry::new();
        registry.register_sidebar_tab(SidebarTab {
            id: "my-tab".into(),
            label: "My Tab".into(),
            icon: "📋".into(),
            panel_id: "my-panel".into(),
            priority: 10,
        });

        assert_eq!(registry.global_tabs().len(), 1);
        assert_eq!(registry.global_tabs()[0].id, "my-tab");
    }

    #[test]
    fn test_sorted_global() {
        let mut registry = SidebarTabRegistry::new();
        registry.register_sidebar_tab(SidebarTab {
            id: "low".into(),
            label: "Low".into(),
            icon: "1️⃣".into(),
            panel_id: "low-panel".into(),
            priority: 1,
        });
        registry.register_sidebar_tab(SidebarTab {
            id: "high".into(),
            label: "High".into(),
            icon: "2️⃣".into(),
            panel_id: "high-panel".into(),
            priority: 100,
        });

        let sorted = registry.sorted_global();
        assert_eq!(sorted[0].id, "high");
        assert_eq!(sorted[1].id, "low");
    }

    #[test]
    fn test_duplicate_skipped() {
        let mut registry = SidebarTabRegistry::new();
        registry.register_sidebar_tab(SidebarTab {
            id: "dup".into(),
            label: "First".into(),
            icon: "1️⃣".into(),
            panel_id: "first-panel".into(),
            priority: 10,
        });
        registry.register_sidebar_tab(SidebarTab {
            id: "dup".into(),
            label: "Second".into(),
            icon: "2️⃣".into(),
            panel_id: "second-panel".into(),
            priority: 20,
        });

        let tabs: Vec<_> = registry.global_tabs().iter().filter(|t| t.id == "dup").collect();
        assert_eq!(tabs.len(), 1);
        assert_eq!(tabs[0].label, "First");
    }
}
