//! Plugin registry for managing plugins
//!
//! The registry is the central component for plugin lifecycle management.
//! It maintains indexes of plugins and their tools, enabling fast dispatch
//! without coupling the MCP server to specific plugin implementations.
//!
//! # Architecture
//!
//! The registry maintains three data structures:
//!
//! - **`plugins`**: `HashMap<String, Arc<Mutex<Arc<dyn Plugin>>>>`
//!   Maps plugin names to their instances. Uses `Arc<Mutex<Arc<_>>>` to allow
//!   interior mutability — the registry can update plugin state even though
//!   it holds `Arc<dyn Plugin>` (which implies shared ownership).
//!
//! - **`tool_index`**: `HashMap<String, (String, Tool)>`
//!   Maps tool names to (plugin_name, tool). This enables O(1) tool lookup
//!   without iterating through all plugins.
//!
//! - **`hook_index`**: `HashMap<HookEventKind, Vec<(String, Priority)>>`
//!   Maps hook event kinds to (plugin_name, priority). Used for fast
//!   hook subscription lookups.
//!
//! # Dispatch Algorithm
//!
//! When `execute_tool` is called:
//!
//! 1. Look up tool name in `tool_index` → gets (plugin_name, tool)
//! 2. Look up plugin in `plugins` → gets `Arc<Mutex<Arc<dyn Plugin>>>`
//! 3. Lock the mutex to get `Arc<dyn Plugin>`
//! 4. Call `plugin.execute_tool(name, args)`
//!
//! This two-level lookup separates concerns: the tool_index handles
//! routing, while the plugin map handles actual execution.
//!
//! # Thread Safety
//!
//! The registry is designed for concurrent access from multiple async tasks:
//! - `Arc` allows sharing across threads
//! - `Mutex` serializes write access to plugin state
//! - `dyn Plugin: Send + Sync` ensures plugins themselves are thread-safe

use crate::hooks::{HookDispatcher, HookEventKind, Priority};
use crate::plugin::error::PluginError;
use crate::plugin::Plugin;
use crate::tools::Tool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::instrument;

/// Registry for managing plugins and their exposed tools.
///
/// The registry maintains:
/// - A map of plugin name to plugin instance
/// - A map of tool name to (plugin, tool) for fast lookup
/// - A map of hook event kind to subscribed plugins with priorities
///
/// # Example
///
/// ```
/// use quilt_mcp::plugin::{Plugin, PluginRegistry, PluginError};
/// use std::sync::Arc;
///
/// struct MyPlugin;
///
/// impl Plugin for MyPlugin {
///     fn name(&self) -> &str { "my_plugin" }
///     fn version(&self) -> &str { "0.1.0" }
///     fn tools(&self) -> Vec<quilt_mcp::tools::Tool> { vec![] }
///     fn execute_tool(&self, name: &str, args: serde_json::Value) -> Result<serde_json::Value, PluginError> {
///         Err(PluginError::NotFound(name.to_string()))
///     }
/// }
///
/// let mut registry = PluginRegistry::new();
/// registry.register(Arc::new(MyPlugin)).unwrap();
/// ```
///
/// # Error Handling
///
/// All errors include the problematic name for debugging:
/// - `AlreadyRegistered("git")` — plugin "git" already exists
/// - `NotFound("Tool not found: git::status")` — tool not in index
#[derive(Default)]
pub struct PluginRegistry {
    // Arc<dyn Plugin> wrapped in Mutex wrapped in Arc for interior mutability
    plugins: HashMap<String, Arc<Mutex<Arc<dyn Plugin>>>>,
    tool_index: HashMap<String, (String, Tool)>, // tool_name -> (plugin_name, tool)
    // Hook subscriptions indexed by event kind: HookEventKind -> Vec<(plugin_name, priority)>
    hook_index: HashMap<HookEventKind, Vec<(String, Priority)>>,
    // Hook dispatcher for routing events to plugins
    hook_dispatcher: HookDispatcher,
}

impl std::fmt::Debug for PluginRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginRegistry")
            .field("plugins", &self.plugins.len())
            .field("tool_index", &self.tool_index.len())
            .field("hook_index", &self.hook_index.len())
            .field("hook_dispatcher", &self.hook_dispatcher)
            .finish()
    }
}

impl PluginRegistry {
    /// Creates a new empty plugin registry.
    ///
    /// The registry starts empty. Use [`register`](PluginRegistry::register)
    /// to add plugins.
    #[instrument(skip_all)]
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            tool_index: HashMap::new(),
            hook_index: HashMap::new(),
            hook_dispatcher: HookDispatcher::new(),
        }
    }

    /// Registers a plugin with the registry.
    ///
    /// The plugin's tools are indexed for fast lookup during dispatch.
    /// If a plugin with the same name is already registered, returns
    /// [`PluginError::AlreadyRegistered`].
    ///
    /// # Indexing Behavior
    ///
    /// All tools from the plugin are added to `tool_index` with keys
    /// equal to `tool.name`. If a tool name already exists (from another
    /// plugin), it will be overwritten by the new plugin. Prefer
    /// unique tool names with namespace prefixes.
    ///
    /// Hook subscriptions are indexed in `hook_index` and the plugin is
    /// registered with the hook dispatcher.
    ///
    /// # Arguments
    ///
    /// * `plugin` - The plugin instance to register, wrapped in `Arc`
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if a plugin with the
    /// same name is already registered.
    #[instrument(skip(self, plugin))]
    pub fn register(&mut self, plugin: Arc<dyn Plugin>) -> Result<(), PluginError> {
        let name = plugin.name().to_string();

        if self.plugins.contains_key(&name) {
            return Err(PluginError::AlreadyRegistered(name));
        }

        // Index all tools provided by this plugin
        for tool in plugin.tools() {
            let tool_name = tool.name.clone();
            self.tool_index.insert(tool_name, (name.clone(), tool));
        }

        // Index all hook subscriptions provided by this plugin
        for sub in plugin.subscribed_hooks() {
            let entry = self.hook_index.entry(sub.event).or_default();
            entry.push((name.clone(), sub.priority));
        }

        // Register with hook dispatcher for actual hook delivery
        use crate::hooks::dispatcher::HookPluginAdapter;
        self.hook_dispatcher
            .register(Arc::new(HookPluginAdapter::new(plugin.clone())))
            .map_err(PluginError::HookFailed)?;

        // Wrap in Mutex then Arc for interior mutability
        self.plugins.insert(name, Arc::new(Mutex::new(plugin)));
        Ok(())
    }

    /// Unregisters a plugin by name.
    ///
    /// Removes the plugin, its tools, and its hook subscriptions from the registry.
    ///
    /// # Caveat: on_unregister Not Called
    ///
    /// Due to the `Arc<Mutex<Arc<dyn Plugin>>` pattern, we cannot
    /// obtain a mutable reference to call `on_unregister`. The plugin
    /// will simply be dropped when its `Arc` refcount reaches zero.
    /// For plugins requiring explicit cleanup, use `Drop` or manage
    /// lifecycle separately.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the plugin to unregister
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the plugin was removed, or
    /// [`PluginError::NotFound`] if no plugin with that name exists.
    #[instrument(skip(self))]
    pub fn unregister(&mut self, name: &str) -> Result<(), PluginError> {
        let _plugin_arc_mutex = self
            .plugins
            .remove(name)
            .ok_or_else(|| PluginError::NotFound(format!("Plugin not found: {}", name)))?;

        // Remove all tools for this plugin from the index
        self.tool_index
            .retain(|_, (plugin_name, _)| plugin_name != name);

        // Remove all hook subscriptions for this plugin from the index
        for subscribers in self.hook_index.values_mut() {
            subscribers.retain(|(n, _)| n != name);
        }

        // Remove from hook dispatcher
        let _ = self.hook_dispatcher.unregister(name);

        // Note: on_unregister is not called here because Arc doesn't provide
        // mutable access. The plugin will be dropped when the Arc is dropped.
        // Plugins that need cleanup should use Drop or explicit lifecycle management.

        Ok(())
    }

    /// Gets a tool by name, returning the plugin and tool metadata.
    ///
    /// This is useful for introspection (listing tools with their providers)
    /// without executing them. For execution, use [`execute_tool`](Self::execute_tool).
    ///
    /// # Arguments
    ///
    /// * `name` - The tool name to look up (e.g., "git::status")
    ///
    /// # Returns
    ///
    /// Returns `Some((Arc<Mutex<Arc<dyn Plugin>>>, Tool))` if the tool exists,
    /// or `None` if no plugin provides this tool.
    #[instrument(skip(self))]
    #[allow(clippy::type_complexity)]
    pub fn get_tool(&self, name: &str) -> Option<(Arc<Mutex<Arc<dyn Plugin>>>, Tool)> {
        let (plugin_name, tool) = self.tool_index.get(name)?;
        let plugin_arc = self.plugins.get(plugin_name)?;
        Some((Arc::clone(plugin_arc), tool.clone()))
    }

    /// Lists all tools from all registered plugins.
    ///
    /// This returns only the tool metadata, not the plugins themselves.
    /// Use this for `tools/list` responses.
    ///
    /// # Returns
    ///
    /// A vector of all tools available through the registry.
    #[instrument(skip(self))]
    pub fn list_tools(&self) -> Vec<Tool> {
        self.tool_index
            .values()
            .map(|(_, tool)| tool.clone())
            .collect()
    }

    /// Lists all registered plugin names.
    ///
    /// # Returns
    ///
    /// A vector of plugin names.
    #[instrument(skip(self))]
    pub fn plugin_names(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    /// Executes a tool by name with the given arguments.
    ///
    /// Looks up the tool in the index, dispatches to the corresponding
    /// plugin's `execute_tool` method.
    ///
    /// # Dispatch Flow
    ///
    /// 1. Look up `name` in `tool_index` → gets (plugin_name, tool_def)
    /// 2. Look up `plugin_name` in `plugins` → gets `Arc<Mutex<Arc<dyn Plugin>>>`
    /// 3. Lock the mutex → gets `Arc<dyn Plugin>`
    /// 4. Call `plugin.execute_tool(name, args)`
    ///
    /// # Arguments
    ///
    /// * `name` - The tool name to execute (e.g., "git::status")
    /// * `args` - JSON arguments to pass to the tool
    ///
    /// # Returns
    ///
    /// Returns the tool's JSON result on success, or a [`PluginError`] if
    /// the tool was not found or execution failed.
    #[instrument(skip(self))]
    pub fn execute_tool(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, PluginError> {
        let (plugin_arc, _tool) = self
            .get_tool(name)
            .ok_or_else(|| PluginError::NotFound(format!("Tool not found: {}", name)))?;

        // Lock the Arc to get mutable access to the plugin
        let plugin = plugin_arc.lock().unwrap();
        // execute_tool takes &self, so we deref the Arc
        (*plugin).execute_tool(name, args)
    }

    /// Returns a reference to the hook dispatcher.
    ///
    /// This allows the MCP server to dispatch hook events to plugins.
    pub fn hook_dispatcher(&self) -> &HookDispatcher {
        &self.hook_dispatcher
    }

    /// Returns the list of subscribed plugins for a given hook event kind.
    ///
    /// Returns plugin names in priority order (highest first).
    #[instrument(skip(self))]
    pub fn hook_subscribers(&self, kind: HookEventKind) -> Vec<String> {
        let mut subscribers = self.hook_index.get(&kind).cloned().unwrap_or_default();

        // Sort by priority descending and extract names
        subscribers.sort_by(|a, b| b.1.cmp(&a.1));
        subscribers.into_iter().map(|(name, _)| name).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::event::{BlockPayload, ChangeType, PagePayload};
    use crate::hooks::{HookEvent, HookSubscription, Priority};
    use crate::tools::Tool;

    struct TestPlugin {
        name: String,
        version: String,
        tools: Vec<Tool>,
        hook_subscriptions: Vec<HookSubscription>,
    }

    impl TestPlugin {
        fn new(name: &str, tools: Vec<Tool>) -> Self {
            Self {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                tools,
                hook_subscriptions: vec![],
            }
        }

        fn with_hooks(name: &str, hooks: Vec<HookSubscription>) -> Self {
            Self {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                tools: vec![],
                hook_subscriptions: hooks,
            }
        }
    }

    impl Plugin for TestPlugin {
        fn name(&self) -> &str {
            &self.name
        }
        fn version(&self) -> &str {
            &self.version
        }
        fn tools(&self) -> Vec<Tool> {
            self.tools.clone()
        }
        fn subscribed_hooks(&self) -> Vec<HookSubscription> {
            self.hook_subscriptions.clone()
        }
        fn on_hook(&self, _event: HookEvent) -> Result<(), crate::hooks::HookError> {
            Ok(())
        }
        fn execute_tool(
            &self,
            name: &str,
            args: serde_json::Value,
        ) -> Result<serde_json::Value, PluginError> {
            if name == "test_tool" {
                Ok(args)
            } else {
                Err(PluginError::NotFound(name.to_string()))
            }
        }
    }

    #[test]
    fn test_register_and_list_tools() {
        let mut registry = PluginRegistry::new();
        let tool = Tool {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: serde_json::json!({}),
        };
        registry
            .register(Arc::new(TestPlugin::new("test", vec![tool])))
            .unwrap();

        let tools = registry.list_tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test_tool");
    }

    #[test]
    fn test_duplicate_registration() {
        let mut registry = PluginRegistry::new();
        registry
            .register(Arc::new(TestPlugin::new("test", vec![])))
            .unwrap();

        let result = registry.register(Arc::new(TestPlugin::new("test", vec![])));
        assert!(matches!(result, Err(PluginError::AlreadyRegistered(_))));
    }

    #[test]
    fn test_get_tool() {
        let mut registry = PluginRegistry::new();
        let tool = Tool {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: serde_json::json!({}),
        };
        registry
            .register(Arc::new(TestPlugin::new("test", vec![tool])))
            .unwrap();

        let result = registry.get_tool("test_tool");
        assert!(result.is_some());
        assert_eq!(result.unwrap().1.name, "test_tool");
    }

    #[test]
    fn test_execute_tool() {
        let mut registry = PluginRegistry::new();
        let tool = Tool {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: serde_json::json!({}),
        };
        registry
            .register(Arc::new(TestPlugin::new("test", vec![tool])))
            .unwrap();

        let args = serde_json::json!({"key": "value"});
        let result = registry.execute_tool("test_tool", args.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), args);
    }

    #[test]
    fn test_unknown_tool_execution() {
        let registry = PluginRegistry::new();
        let result = registry.execute_tool("unknown", serde_json::json!({}));
        assert!(matches!(result, Err(PluginError::NotFound(_))));
    }

    #[test]
    fn test_unregister() {
        let mut registry = PluginRegistry::new();
        registry
            .register(Arc::new(TestPlugin::new("test", vec![])))
            .unwrap();

        registry.unregister("test").unwrap();
        assert!(registry.get_tool("test_tool").is_none());
        assert!(registry.plugin_names().is_empty());
    }

    #[test]
    fn test_unregister_not_found() {
        let mut registry = PluginRegistry::new();
        let result = registry.unregister("nonexistent");
        assert!(matches!(result, Err(PluginError::NotFound(_))));
    }

    #[test]
    fn test_hook_registration() {
        use crate::hooks::HookEventKind;

        let mut registry = PluginRegistry::new();

        // Create a plugin with hook subscriptions
        let plugin = Arc::new(TestPlugin::with_hooks(
            "hook_plugin",
            vec![
                HookSubscription {
                    event: HookEventKind::BlockChanged,
                    priority: Priority::NORMAL,
                    filter: None,
                },
                HookSubscription {
                    event: HookEventKind::PageChanged,
                    priority: Priority::HIGH,
                    filter: None,
                },
            ],
        ));

        registry.register(plugin).unwrap();

        // Check hook subscribers are indexed correctly
        let block_subscribers = registry.hook_subscribers(HookEventKind::BlockChanged);
        assert_eq!(block_subscribers.len(), 1);
        assert_eq!(block_subscribers[0], "hook_plugin");

        let page_subscribers = registry.hook_subscribers(HookEventKind::PageChanged);
        assert_eq!(page_subscribers.len(), 1);
        assert_eq!(page_subscribers[0], "hook_plugin");

        // Check dispatcher received the plugin
        assert_eq!(registry.hook_dispatcher().plugin_count(), 1);
    }

    #[test]
    fn test_hook_dispatch() {
        use crate::hooks::HookEventKind;

        let mut registry = PluginRegistry::new();

        // Create and register a plugin
        let plugin = Arc::new(TestPlugin::with_hooks(
            "dispatch_test",
            vec![HookSubscription {
                event: HookEventKind::BlockChanged,
                priority: Priority::NORMAL,
                filter: None,
            }],
        ));

        registry.register(plugin).unwrap();

        // Emit a hook event
        let event = HookEvent::BlockChanged(BlockPayload {
            id: "block-1".to_string(),
            page_id: "page-1".to_string(),
            change_type: ChangeType::Created,
            content: Some("Test content".to_string()),
        });

        let results = registry.hook_dispatcher().dispatch(&event);
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert_eq!(results[0].plugin_name, "dispatch_test");
    }

    #[test]
    fn test_unknown_hook_ignored() {
        use crate::hooks::HookEventKind;

        let mut registry = PluginRegistry::new();

        // Create a plugin that subscribes only to BlockChanged
        let plugin = Arc::new(TestPlugin::with_hooks(
            "block_only_plugin",
            vec![HookSubscription {
                event: HookEventKind::BlockChanged,
                priority: Priority::NORMAL,
                filter: None,
            }],
        ));

        registry.register(plugin).unwrap();

        // Emit a PageChanged event (which the plugin did NOT subscribe to)
        let page_event = HookEvent::PageChanged(PagePayload {
            id: "page-1".to_string(),
            name: "Test Page".to_string(),
            change_type: ChangeType::Created,
        });

        let results = registry.hook_dispatcher().dispatch(&page_event);

        // Plugin should NOT have received the event since it only subscribes to BlockChanged
        assert!(
            results.is_empty(),
            "Plugin should not receive events it did not subscribe to"
        );
    }

    #[test]
    fn test_plugin_receives_only_subscribed_hook() {
        use crate::hooks::HookEventKind;

        let mut registry = PluginRegistry::new();

        // Create a plugin that subscribes to both BlockChanged and PageChanged
        let plugin = Arc::new(TestPlugin::with_hooks(
            "multi_hook_plugin",
            vec![
                HookSubscription {
                    event: HookEventKind::BlockChanged,
                    priority: Priority::NORMAL,
                    filter: None,
                },
                HookSubscription {
                    event: HookEventKind::PageChanged,
                    priority: Priority::NORMAL,
                    filter: None,
                },
            ],
        ));

        registry.register(plugin).unwrap();

        // Emit a BlockChanged event
        let block_event = HookEvent::BlockChanged(BlockPayload {
            id: "block-1".to_string(),
            page_id: "page-1".to_string(),
            change_type: ChangeType::Created,
            content: Some("Test content".to_string()),
        });

        let results = registry.hook_dispatcher().dispatch(&block_event);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].plugin_name, "multi_hook_plugin");

        // Emit a PageChanged event
        let page_event = HookEvent::PageChanged(PagePayload {
            id: "page-1".to_string(),
            name: "Test Page".to_string(),
            change_type: ChangeType::Updated,
        });

        let results = registry.hook_dispatcher().dispatch(&page_event);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].plugin_name, "multi_hook_plugin");
    }
}
