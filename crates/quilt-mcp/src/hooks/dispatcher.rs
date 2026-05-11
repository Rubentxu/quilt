//! Hook dispatcher for routing events to plugins
//!
//! The dispatcher is the central component that manages plugin subscriptions
//! and routes hook events to the appropriate plugin handlers.
//!
//! # Architecture
//!
//! - **`subscriptions`**: `HashMap<HookEventKind, Vec<(String, Priority)>>`
//!   Maps event kinds to a list of (plugin_name, priority) pairs.
//!   Priority determines dispatch order (higher = earlier).
//!
//! - **`plugin_hooks`**: `HashMap<String, Arc<dyn HookPlugin>>`
//!   Maps plugin names to their hook handler implementations.
//!
//! # Dispatch Algorithm
//!
//! When `dispatch` is called with an event:
//!
//! 1. Look up subscribers for the event's [`HookEventKind`]
//! 2. Sort subscribers by priority (descending)
//! 3. For each subscriber:
//!    a. Look up the plugin's hook handler
//!    b. Call `std::panic::catch_unwind` to isolate panics
//!    c. If panic: record error and continue to next plugin
//!    d. If success: record result and continue
//! 4. Return all results collected from all plugins
//!
//! # Thread Safety
//!
//! The dispatcher uses interior mutability for thread-safe concurrent access:
//! - `Arc` allows sharing across threads
//! - `Mutex` serializes write access to subscriptions

use crate::hooks::error::HookError;
use crate::hooks::event::{HookEvent, HookEventKind, HookResult, Priority};
use crate::plugin::Plugin;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{instrument, warn};

/// Trait for plugins that can receive hook events.
///
/// This is a separate trait from [`crate::plugin::Plugin`] to allow
/// hook functionality without requiring full plugin implementation.
/// Plugins implementing `Plugin` get automatic hook support via default methods.
pub trait HookPlugin: Send + Sync {
    /// Returns the plugin's unique name.
    fn name(&self) -> &str;

    /// Returns the list of hook events this plugin subscribes to.
    fn subscribed_hooks(&self) -> Vec<HookEventKind>;

    /// Called when a hook event is dispatched to this plugin.
    ///
    /// # Arguments
    ///
    /// * `event` - The hook event being dispatched
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error message on failure.
    /// The error message is recorded in the [`HookResult`].
    fn on_hook(&self, event: HookEvent) -> Result<(), String>;
}

/// Blanket implementation: any `dyn Plugin` also implements `HookPlugin`.
///
/// This allows plugins that implement the full `Plugin` trait to be used
/// with the hook dispatch system without additional boilerplate.
impl<T: Plugin + ?Sized> HookPlugin for T {
    fn name(&self) -> &str {
        Plugin::name(self)
    }

    fn subscribed_hooks(&self) -> Vec<HookEventKind> {
        Plugin::subscribed_hooks(self)
            .into_iter()
            .map(|sub| sub.event)
            .collect()
    }

    fn on_hook(&self, event: HookEvent) -> Result<(), String> {
        Plugin::on_hook(self, event).map_err(|e| e.to_string())
    }
}

/// Wrapper to adapt `Arc<dyn Plugin>` to `dyn HookPlugin`.
///
/// Since Rust doesn't allow direct coercion from `Arc<dyn Plugin>` to
/// `Arc<dyn HookPlugin>`, we use this wrapper to enable registration.
pub struct HookPluginAdapter {
    plugin: Arc<dyn Plugin>,
}

impl HookPluginAdapter {
    /// Creates a new adapter wrapping the given plugin.
    pub fn new(plugin: Arc<dyn Plugin>) -> Self {
        Self { plugin }
    }
}

impl HookPlugin for HookPluginAdapter {
    fn name(&self) -> &str {
        self.plugin.name()
    }

    fn subscribed_hooks(&self) -> Vec<HookEventKind> {
        self.plugin
            .subscribed_hooks()
            .into_iter()
            .map(|sub| sub.event)
            .collect()
    }

    fn on_hook(&self, event: HookEvent) -> Result<(), String> {
        self.plugin.on_hook(event).map_err(|e| e.to_string())
    }
}

/// Central dispatcher for hook events.
///
/// Manages plugin subscriptions and routes events to the appropriate
/// plugin handlers with panic isolation.
#[derive(Default)]
pub struct HookDispatcher {
    /// Maps event kinds to subscribed plugins (plugin_name, priority)
    subscriptions: HashMap<HookEventKind, Vec<(String, Priority)>>,
    /// Maps plugin names to their hook implementations
    plugin_hooks: HashMap<String, Arc<dyn HookPlugin>>,
    /// Protects concurrent access to subscriptions
    guard: Mutex<()>,
}

impl std::fmt::Debug for HookDispatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookDispatcher")
            .field("subscriptions", &self.subscriptions.len())
            .field("plugins", &self.plugin_hooks.len())
            .finish()
    }
}

impl HookDispatcher {
    /// Creates a new empty hook dispatcher.
    #[instrument(skip_all)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a plugin with the dispatcher.
    ///
    /// The plugin's subscribed hooks are recorded, and its hook handler
    /// is stored for later dispatch.
    ///
    /// # Arguments
    ///
    /// * `plugin` - The plugin to register, wrapped in `Arc`
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if registration fails.
    #[instrument(skip(self, plugin))]
    pub fn register(&mut self, plugin: Arc<dyn HookPlugin>) -> Result<(), HookError> {
        let name = plugin.name().to_string();
        let hooks = plugin.subscribed_hooks();

        if hooks.is_empty() {
            // Allow plugins with no hook subscriptions (no-op for hooks)
            return Ok(());
        }

        // Store plugin hook handler
        self.plugin_hooks.insert(name.clone(), Arc::clone(&plugin));

        // Register subscriptions for each hook type
        let _guard = self.guard.lock().unwrap();
        for hook_kind in hooks {
            let entry = self.subscriptions.entry(hook_kind).or_default();
            // Insert in priority order (will sort later)
            entry.push((name.clone(), Priority::DEFAULT));
        }

        Ok(())
    }

    /// Registers a plugin with explicit priorities for each hook type.
    ///
    /// This allows fine-grained control over dispatch ordering.
    ///
    /// # Arguments
    ///
    /// * `plugin` - The plugin to register
    /// * `subscriptions` - Iterator of (hook_kind, priority) pairs
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success.
    #[instrument(skip(self, plugin, subscriptions))]
    pub fn register_with_priority(
        &mut self,
        plugin: Arc<dyn HookPlugin>,
        subscriptions: impl IntoIterator<Item = (HookEventKind, Priority)>,
    ) -> Result<(), HookError> {
        let name = plugin.name().to_string();

        // Store plugin hook handler
        self.plugin_hooks.insert(name.clone(), Arc::clone(&plugin));

        // Register subscriptions with priorities
        let _guard = self.guard.lock().unwrap();
        for (hook_kind, priority) in subscriptions {
            let entry = self.subscriptions.entry(hook_kind).or_default();
            entry.push((name.clone(), priority));
        }

        Ok(())
    }

    /// Unregisters a plugin from all hook subscriptions.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the plugin to unregister
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the plugin was removed, or an error if not found.
    #[instrument(skip(self))]
    pub fn unregister(&mut self, name: &str) -> Result<(), HookError> {
        if !self.plugin_hooks.contains_key(name) {
            return Err(HookError::NotSubscribed(name.to_string()));
        }

        // Remove plugin from all subscription lists
        let _guard = self.guard.lock().unwrap();
        for subscribers in self.subscriptions.values_mut() {
            subscribers.retain(|(n, _)| n != name);
        }

        // Remove plugin hook handler
        self.plugin_hooks.remove(name);

        Ok(())
    }

    /// Dispatches a hook event to all subscribed plugins.
    ///
    /// Plugins are called in priority order (highest first). Each plugin's
    /// execution is isolated via `catch_unwind` — a panic in one plugin
    /// does not affect others.
    ///
    /// # Arguments
    ///
    /// * `event` - The hook event to dispatch
    ///
    /// # Returns
    ///
    /// Returns a vector of results, one per plugin that was called.
    /// The vector is ordered by priority (highest first).
    #[instrument(skip(self), fields(event_kind = event.kind().name()))]
    pub fn dispatch(&self, event: &HookEvent) -> Vec<HookResult> {
        let kind = event.kind();
        let mut results = Vec::new();

        // Get subscribers for this event kind
        let subscribers = {
            let _guard = self.guard.lock().unwrap();
            self.subscriptions.get(&kind).cloned().unwrap_or_default()
        };

        if subscribers.is_empty() {
            return results;
        }

        // Sort by priority descending
        let mut sorted_subscribers = subscribers;
        sorted_subscribers.sort_by(|a, b| b.1.cmp(&a.1));

        // Dispatch to each plugin
        for (plugin_name, _priority) in sorted_subscribers {
            let result = self.dispatch_to_plugin(plugin_name.clone(), event.clone());
            results.push(result);
        }

        results
    }

    /// Dispatches an event to a specific plugin with panic isolation.
    ///
    /// Uses `std::panic::catch_unwind` to isolate plugin execution.
    /// If the plugin panics, returns a [`HookError::PluginPanicked`] result.
    fn dispatch_to_plugin(&self, plugin_name: String, event: HookEvent) -> HookResult {
        // Look up plugin
        let plugin = match self.plugin_hooks.get(&plugin_name) {
            Some(p) => Arc::clone(p),
            None => {
                return HookResult::failure(plugin_name, "Plugin hook handler not found");
            }
        };

        // Execute with panic isolation
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| plugin.on_hook(event)));

        match result {
            Ok(Ok(())) => HookResult::success(&plugin_name),
            Ok(Err(e)) => HookResult::failure(&plugin_name, e),
            Err(panic_info) => {
                let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic".to_string()
                };

                warn!(
                    plugin = %plugin_name,
                    panic = %msg,
                    "Plugin hook panicked, isolating and continuing"
                );

                HookResult::failure(&plugin_name, format!("Plugin panicked: {}", msg))
            }
        }
    }

    /// Returns the list of subscribed plugins for a given event kind.
    ///
    /// Returns plugin names in priority order (highest first).
    #[instrument(skip(self))]
    pub fn subscribers_for(&self, kind: HookEventKind) -> Vec<String> {
        let _guard = self.guard.lock().unwrap();
        let mut subscribers = self.subscriptions.get(&kind).cloned().unwrap_or_default();

        // Sort by priority descending and extract names
        subscribers.sort_by(|a, b| b.1.cmp(&a.1));
        subscribers.into_iter().map(|(name, _)| name).collect()
    }

    /// Returns the number of registered plugins.
    pub fn plugin_count(&self) -> usize {
        self.plugin_hooks.len()
    }

    /// Returns the total number of subscriptions across all event kinds.
    pub fn subscription_count(&self) -> usize {
        let _guard = self.guard.lock().unwrap();
        self.subscriptions.values().map(|v| v.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::event::{BlockPayload, ChangeType, PagePayload};

    struct TestPlugin {
        name: String,
        hooks: Vec<HookEventKind>,
        should_panic: bool,
        call_count: std::sync::atomic::AtomicUsize,
    }

    impl TestPlugin {
        fn new(name: &str, hooks: Vec<HookEventKind>) -> Self {
            Self {
                name: name.to_string(),
                hooks,
                should_panic: false,
                call_count: std::sync::atomic::AtomicUsize::new(0),
            }
        }

        fn with_panic(name: &str, hooks: Vec<HookEventKind>) -> Self {
            Self {
                name: name.to_string(),
                hooks,
                should_panic: true,
                call_count: std::sync::atomic::AtomicUsize::new(0),
            }
        }
    }

    impl HookPlugin for TestPlugin {
        fn name(&self) -> &str {
            &self.name
        }

        fn subscribed_hooks(&self) -> Vec<HookEventKind> {
            self.hooks.clone()
        }

        fn on_hook(&self, _event: HookEvent) -> Result<(), String> {
            self.call_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if self.should_panic {
                panic!("Test panic from {}", self.name);
            }
            Ok(())
        }
    }

    #[test]
    fn test_register_and_dispatch() {
        let mut dispatcher = HookDispatcher::new();

        let plugin = Arc::new(TestPlugin::new("test", vec![HookEventKind::BlockChanged]));
        dispatcher.register(plugin).unwrap();

        let event = HookEvent::BlockChanged(BlockPayload {
            id: "block-1".to_string(),
            page_id: "page-1".to_string(),
            change_type: ChangeType::Created,
            content: Some("Hello".to_string()),
        });

        let results = dispatcher.dispatch(&event);
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert_eq!(results[0].plugin_name, "test");
    }

    #[test]
    fn test_priority_ordering() {
        let mut dispatcher = HookDispatcher::new();

        let low = Arc::new(TestPlugin::new("low", vec![HookEventKind::BlockChanged]));
        let high = Arc::new(TestPlugin::new("high", vec![HookEventKind::BlockChanged]));
        let normal = Arc::new(TestPlugin::new("normal", vec![HookEventKind::BlockChanged]));

        // Register in non-sorted order
        dispatcher
            .register_with_priority(low, [(HookEventKind::BlockChanged, Priority::LOW)])
            .unwrap();
        dispatcher
            .register_with_priority(high, [(HookEventKind::BlockChanged, Priority::HIGH)])
            .unwrap();
        dispatcher
            .register_with_priority(normal, [(HookEventKind::BlockChanged, Priority::NORMAL)])
            .unwrap();

        let event = HookEvent::BlockChanged(BlockPayload {
            id: "block-1".to_string(),
            page_id: "page-1".to_string(),
            change_type: ChangeType::Created,
            content: None,
        });

        let results = dispatcher.dispatch(&event);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].plugin_name, "high");
        assert_eq!(results[1].plugin_name, "normal");
        assert_eq!(results[2].plugin_name, "low");
    }

    #[test]
    fn test_panic_isolation() {
        let mut dispatcher = HookDispatcher::new();

        let panicking = Arc::new(TestPlugin::with_panic(
            "panicking",
            vec![HookEventKind::BlockChanged],
        ));
        let normal = Arc::new(TestPlugin::new("normal", vec![HookEventKind::BlockChanged]));

        dispatcher.register(panicking).unwrap();
        dispatcher.register(normal).unwrap();

        let event = HookEvent::BlockChanged(BlockPayload {
            id: "block-1".to_string(),
            page_id: "page-1".to_string(),
            change_type: ChangeType::Updated,
            content: None,
        });

        // Should not panic - panicking plugin is isolated
        let results = dispatcher.dispatch(&event);
        assert_eq!(results.len(), 2);

        // Find the results for each plugin
        let panicking_result = results
            .iter()
            .find(|r| r.plugin_name == "panicking")
            .unwrap();
        let normal_result = results.iter().find(|r| r.plugin_name == "normal").unwrap();

        assert!(!panicking_result.success);
        assert!(panicking_result
            .error
            .as_ref()
            .unwrap()
            .contains("Plugin panicked"));
        assert!(normal_result.success);
    }

    #[test]
    fn test_unregister() {
        let mut dispatcher = HookDispatcher::new();

        let plugin = Arc::new(TestPlugin::new("test", vec![HookEventKind::BlockChanged]));
        dispatcher.register(plugin).unwrap();

        assert_eq!(dispatcher.plugin_count(), 1);

        dispatcher.unregister("test").unwrap();

        assert_eq!(dispatcher.plugin_count(), 0);

        let event = HookEvent::BlockChanged(BlockPayload {
            id: "block-1".to_string(),
            page_id: "page-1".to_string(),
            change_type: ChangeType::Deleted,
            content: None,
        });
        let results = dispatcher.dispatch(&event);
        assert!(results.is_empty());
    }

    #[test]
    fn test_no_subscribers() {
        let dispatcher = HookDispatcher::new();

        let event = HookEvent::BlockChanged(BlockPayload {
            id: "block-1".to_string(),
            page_id: "page-1".to_string(),
            change_type: ChangeType::Created,
            content: Some("Hello".to_string()),
        });

        let results = dispatcher.dispatch(&event);
        assert!(results.is_empty());
    }

    #[test]
    fn test_multiple_event_types() {
        let mut dispatcher = HookDispatcher::new();

        let block_plugin = Arc::new(TestPlugin::new(
            "block_plugin",
            vec![HookEventKind::BlockChanged],
        ));
        let page_plugin = Arc::new(TestPlugin::new(
            "page_plugin",
            vec![HookEventKind::PageChanged],
        ));
        let both_plugin = Arc::new(TestPlugin::new(
            "both",
            vec![HookEventKind::BlockChanged, HookEventKind::PageChanged],
        ));

        dispatcher.register(block_plugin).unwrap();
        dispatcher.register(page_plugin).unwrap();
        dispatcher.register(both_plugin).unwrap();

        let block_event = HookEvent::BlockChanged(BlockPayload {
            id: "block-1".to_string(),
            page_id: "page-1".to_string(),
            change_type: ChangeType::Created,
            content: Some("Hello".to_string()),
        });

        let results = dispatcher.dispatch(&block_event);
        assert_eq!(results.len(), 2); // block_plugin and both_plugin

        let page_event = HookEvent::PageChanged(PagePayload {
            id: "page-1".to_string(),
            name: "Test Page".to_string(),
            change_type: ChangeType::Created,
        });

        let results = dispatcher.dispatch(&page_event);
        assert_eq!(results.len(), 2); // page_plugin and both_plugin
    }
}
