//! Quilt UI Plugin System
//!
//! Provides a plugin architecture for extending the Quilt UI with custom
//! navigation items, routes, sidebar tabs, and slash commands.
//!
//! # Example
//!
//! ```rust
//! use quilt_ui_plugin::{UiPlugin, PluginRegistry, NavItem, NavRegistry};
//!
//! struct MyPlugin;
//!
//! impl UiPlugin for MyPlugin {
//!     fn name(&self) -> &'static str { "my-plugin" }
//!     fn version(&self) -> &'static str { "0.1.0" }
//!
//!     fn register_nav_items(&self, nav: &mut NavRegistry) {
//!         nav.register_nav_item(NavItem {
//!             id: "my-page".into(),
//!             label: "My Page".into(),
//!             icon: "📦".into(),
//!             href: "/my-page".into(),
//!             active_if: None,
//!         });
//!     }
//! }
//!
//! let mut registry = PluginRegistry::new();
//! registry.register(MyPlugin);
//! ```

mod nav;
mod routes;
mod sidebar_tab;
mod slash_command;

pub use nav::{NavItem, NavRegistry};
pub use routes::{RouteRegistration, RouteRegistry};
pub use sidebar_tab::{SidebarTab, SidebarTabRegistry};
pub use slash_command::{SlashCommand, SlashCommandRegistry};

use std::sync::Arc;

/// Main plugin trait that UI plugins must implement.
///
/// Each method has a default no-op implementation, so plugins
/// only need to override the extension points they care about.
pub trait UiPlugin: Send + Sync {
    /// Unique identifier for this plugin (kebab-case recommended)
    fn name(&self) -> &'static str;

    /// Semantic version string
    fn version(&self) -> &'static str;

    /// Register navigation items for the sidebar
    fn register_nav_items(&self, _nav: &mut NavRegistry) {}

    /// Register additional routes
    fn register_routes(&self, _routes: &mut RouteRegistry) {}

    /// Register tabs in the right sidebar
    fn register_sidebar_tabs(&self, _tabs: &mut SidebarTabRegistry) {}

    /// Register slash commands
    fn register_slash_commands(&self, _commands: &mut SlashCommandRegistry) {}
}

/// Central registry holding all plugin contributions.
///
/// Created with default built-in nav items pre-registered.
#[derive(Default)]
pub struct PluginRegistry {
    plugins: Vec<Arc<dyn UiPlugin>>,
    nav: NavRegistry,
    routes: RouteRegistry,
    sidebar_tabs: SidebarTabRegistry,
    slash_commands: SlashCommandRegistry,
}

impl PluginRegistry {
    /// Create a new registry with default built-in nav items
    pub fn new() -> Self {
        let mut registry = Self::default();

        // Register default built-in navigation items
        let default_navs = vec![
            NavItem {
                id: "journal".into(),
                label: "Journal".into(),
                icon: "📅".into(),
                href: "/journal".into(),
                active_if: None,
            },
            NavItem {
                id: "pages".into(),
                label: "Pages".into(),
                icon: "📄".into(),
                href: "/pages".into(),
                active_if: None,
            },
            NavItem {
                id: "search".into(),
                label: "Search".into(),
                icon: "🔍".into(),
                href: "/search".into(),
                active_if: None,
            },
            NavItem {
                id: "query".into(),
                label: "Query".into(),
                icon: "💬".into(),
                href: "/query".into(),
                active_if: None,
            },
            NavItem {
                id: "graph".into(),
                label: "Graph".into(),
                icon: "🌐".into(),
                href: "/graph".into(),
                active_if: None,
            },
            NavItem {
                id: "cognitive".into(),
                label: "Cognitive".into(),
                icon: "🧠".into(),
                href: "/cognitive".into(),
                active_if: None,
            },
        ];

        for nav in default_navs {
            registry.nav.register_nav_item(nav);
        }

        registry
    }

    /// Register a plugin instance
    pub fn register<P: UiPlugin + 'static>(&mut self, plugin: P) {
        let arc = Arc::new(plugin);
        arc.register_nav_items(&mut self.nav);
        arc.register_routes(&mut self.routes);
        arc.register_sidebar_tabs(&mut self.sidebar_tabs);
        arc.register_slash_commands(&mut self.slash_commands);
        self.plugins.push(arc);
    }

    /// Get the navigation registry
    pub fn nav(&self) -> &NavRegistry {
        &self.nav
    }

    /// Get the route registry
    pub fn routes(&self) -> &RouteRegistry {
        &self.routes
    }

    /// Get the sidebar tab registry
    pub fn sidebar_tabs(&self) -> &SidebarTabRegistry {
        &self.sidebar_tabs
    }

    /// Get the slash command registry
    pub fn slash_commands(&self) -> &SlashCommandRegistry {
        &self.slash_commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin;

    impl UiPlugin for TestPlugin {
        fn name(&self) -> &'static str { "test-plugin" }
        fn version(&self) -> &'static str { "0.1.0" }

        fn register_nav_items(&self, nav: &mut NavRegistry) {
            nav.register_nav_item(NavItem {
                id: "test-page".into(),
                label: "Test Page".into(),
                icon: "🧪".into(),
                href: "/test".into(),
                active_if: None,
            });
        }

        fn register_slash_commands(&self, commands: &mut SlashCommandRegistry) {
            commands.register_slash_command(SlashCommand {
                id: "test-cmd".into(),
                label: "Test Command".into(),
                icon: "⚡".into(),
                description: "A test command".into(),
                action: Box::new(|_| true),
            });
        }
    }

    #[test]
    fn test_registry_starts_with_builtin_navs() {
        let registry = PluginRegistry::new();
        assert!(!registry.nav().items().is_empty());
    }

    #[test]
    fn test_register_plugin() {
        let mut registry = PluginRegistry::new();
        registry.register(TestPlugin);

        // Plugin nav item should be added
        let nav_items = registry.nav().items();
        assert!(nav_items.iter().any(|n| n.id == "test-page"));

        // Plugin slash command should be added
        let cmds = registry.slash_commands().commands();
        assert!(cmds.iter().any(|c| c.id == "test-cmd"));
    }

    #[test]
    fn test_nav_registry_no_duplicate_ids() {
        let mut registry = NavRegistry::new();
        registry.register_nav_item(NavItem {
            id: "dup".into(),
            label: "First".into(),
            icon: "1️⃣".into(),
            href: "/first".into(),
            active_if: None,
        });
        registry.register_nav_item(NavItem {
            id: "dup".into(),
            label: "Second".into(),
            icon: "2️⃣".into(),
            href: "/second".into(),
            active_if: None,
        });

        // Should only have one
        let items: Vec<_> = registry.items().iter().filter(|n| n.id == "dup").collect();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "First");
    }

    #[test]
    fn test_slash_command_filter() {
        let mut registry = SlashCommandRegistry::new();
        registry.register_slash_command(SlashCommand {
            id: "search".into(),
            label: "Search".into(),
            icon: "🔍".into(),
            description: "Search the graph".into(),
            action: Box::new(|_| true),
        });
        registry.register_slash_command(SlashCommand {
            id: "graph".into(),
            label: "Graph View".into(),
            icon: "🌐".into(),
            description: "Open graph".into(),
            action: Box::new(|_| true),
        });

        let results = registry.filter("sea");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "search");
    }
}
