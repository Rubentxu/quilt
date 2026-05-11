//! Plugin system for quilt-mcp
//!
//! This module provides the Plugin trait and PluginRegistry for extending
//! the MCP server with custom tools, resources, and notifications.
//!
//! # Plugin Architecture
//!
//! The plugin system follows a **provider pattern** where plugins self-describe
//! their capabilities (tools, resources, notifications) and handle execution
//! via a unified interface. This design allows the MCP server to:
//!
//! - **Discover** plugins at runtime without hardcoding dependencies
//! - **Index** tools for fast lookup without coupling to specific plugins
//! - **Dispatch** tool execution to the correct plugin transparently
//!
//! # Lifecycle
//!
//! 1. **Registration**: Plugin is added to the [`PluginRegistry`] via [`register`](PluginRegistry::register)
//! 2. **Initialization**: [`on_register`](Plugin::on_register) is called with a [`PluginContext`]
//! 3. **Usage**: Tools are exposed via MCP `tools/list` and executed via `tools/call`
//! 4. **Cleanup**: [`on_unregister`](Plugin::on_unregister) is called before removal
//!
//! # Thread Safety
//!
//! Plugins must be [`Send`] and [`Sync`] because the registry may invoke them
//! concurrently from multiple async tasks. This allows a single plugin instance
//! to serve multiple tool execution requests simultaneously without duplication.

pub mod error;
pub mod registry;

pub use error::PluginError;
pub use registry::PluginRegistry;

use crate::hooks::{HookError, HookEvent, HookSubscription};
use crate::notifications::Notification;
use crate::resources::Resource;
use crate::tools::Tool;
use serde::{Deserialize, Serialize};

/// Context passed to a plugin when it is registered.
///
/// Provides access to server-side dependencies that plugins may need,
/// such as database connections, configuration, or other services.
/// Currently reserved for future extension — plugins should be self-contained
/// and only depend on their own configuration.
#[derive(Debug, Clone)]
pub struct PluginContext {
    // Reserved for future extension, e.g., access to repositories or services
}

/// Manifest describing a plugin's identity and capabilities.
///
/// Created by plugins to self-describe during registration. The manifest
/// provides the basic metadata that tools/list returns to MCP clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique name identifying the plugin. This name is used as a namespace
    /// for the plugin's tools (e.g., "git" produces "git::status").
    pub name: String,
    /// Semantic version string following semver conventions.
    pub version: String,
    /// Human-readable description of what the plugin does.
    pub description: Option<String>,
}

/// Plugin trait for extending the MCP server with domain-specific functionality.
///
/// Implement this trait to add custom tools, resources, and notifications
/// to the quilt-mcp server. Plugins are the primary extension point for
/// adding new AI-usable capabilities without modifying core server code.
///
/// # Why Plugins?
///
/// Rather than hardcoding every tool in the MCP server, plugins allow:
/// - **Domain experts** to add tools for specific workflows (git, PDF, LaTeX)
/// - **Hot loading** of new capabilities without restarts
/// - **Isolation** — a plugin crash doesn't crash the server
/// - **Testing** — plugins can be tested in isolation with mock dependencies
///
/// # Tool Naming Convention
///
/// Plugins use `::` as a namespace separator in tool names. The plugin's `name()`
/// forms the namespace prefix. This prevents collisions between plugins:
/// - `git::status` — git plugin's status tool
/// - `git::log` — git plugin's log tool
/// - `zotero::search` — hypothetical zotero plugin
///
/// # Implementing a Plugin
///
/// A minimal plugin needs only `name()`, `version()`, and `execute_tool()`.
/// The other methods have sensible defaults for plugins that don't need them:
///
/// ```
/// use quilt_mcp::plugin::{Plugin, PluginContext, PluginError};
/// use quilt_mcp::tools::Tool;
///
/// struct MyPlugin;
///
/// impl Plugin for MyPlugin {
///     fn name(&self) -> &str { "my_plugin" }
///     fn version(&self) -> &str { "0.1.0" }
///     fn tools(&self) -> Vec<Tool> { vec![] }
///     fn execute_tool(&self, name: &str, _args: serde_json::Value) -> Result<serde_json::Value, PluginError> {
///         Err(PluginError::NotFound(name.to_string()))
///     }
/// }
/// ```
///
/// # Lifecycle Hooks
///
/// Use [`on_register`](Plugin::on_register) to initialize resources like database
/// connections or external service clients. Use [`on_unregister`](Plugin::on_unregister)
/// for graceful shutdown. Note: due to interior mutability in the registry,
/// `on_unregister` is not automatically called — prefer `Drop` or explicit
/// lifecycle management for cleanup.
pub trait Plugin: Send + Sync {
    /// Returns the plugin's unique name.
    ///
    /// This name is used as the namespace prefix for all tools provided
    /// by this plugin. By convention, use lowercase with underscores.
    fn name(&self) -> &str;

    /// Returns the plugin's semantic version string.
    ///
    /// Should follow semver conventions (e.g., "1.0.0", "0.2.1-beta").
    fn version(&self) -> &str;

    /// Returns the list of tools this plugin provides.
    ///
    /// Each tool has a unique name within the plugin's namespace.
    /// The registry indexes these for fast lookup during dispatch.
    fn tools(&self) -> Vec<Tool> {
        vec![]
    }

    /// Returns the list of resources this plugin provides.
    ///
    /// Resources are data sources (like files or database views) that
    /// AI agents can read. Default returns empty list.
    fn resources(&self) -> Vec<Resource> {
        vec![]
    }

    /// Returns the list of notifications this plugin sends.
    ///
    /// Notifications are async events pushed to clients (e.g., "file changed").
    /// Default returns empty list.
    fn notifications(&self) -> Vec<Notification> {
        vec![]
    }

    /// Called when the plugin is registered with the server.
    ///
    /// Use this to initialize resources, connect to external services,
    /// or validate configuration. If this returns an error, the
    /// plugin registration fails and the plugin is not added to the registry.
    ///
    /// # Example
    ///
    /// ```text
    /// // In your plugin implementation:
    /// fn on_register(&mut self, ctx: &PluginContext) -> Result<(), PluginError> {
    ///     self.db = Some(connect_to_database()?);
    ///     self.validate_config()?;
    ///     Ok(())
    /// }
    /// ```
    fn on_register(&mut self, _ctx: &PluginContext) -> Result<(), error::PluginError> {
        Ok(())
    }

    /// Called when the plugin is unregistered from the server.
    ///
    /// Use this to clean up resources, close connections, or save state.
    /// Note: due to interior mutability constraints in the registry,
    /// this is not automatically invoked on [`PluginRegistry::unregister`].
    /// For guaranteed cleanup, implement `Drop` or use explicit lifecycle management.
    fn on_unregister(&mut self) {}

    /// Returns the hook subscriptions for this plugin.
    ///
    /// This declares which hook events the plugin wants to receive.
    /// Default returns empty list (plugin doesn't subscribe to any hooks).
    ///
    /// # Example
    ///
    /// A plugin that subscribes to block changed events:
    ///
    /// ```ignore
    /// // In your plugin implementation:
    /// fn subscribed_hooks(&self) -> Vec<HookSubscription> {
    ///     vec![
    ///         HookSubscription {
    ///             event: HookEventKind::BlockChanged,
    ///             priority: Priority::NORMAL,
    ///             filter: None,
    ///         },
    ///     ]
    /// }
    /// ```
    fn subscribed_hooks(&self) -> Vec<HookSubscription> {
        vec![]
    }

    /// Handle a hook event dispatched to this plugin.
    ///
    /// This is called by the hook dispatcher when an event matches
    /// one of this plugin's subscriptions.
    ///
    /// # Synchronous Implementation
    ///
    /// This method is synchronous. If a plugin needs to perform async work
    /// in response to a hook event, it should spawn its own async task.
    /// This avoids blocking the critical mutation/dispatch path.
    ///
    /// # Arguments
    ///
    /// * `event` - The hook event being dispatched
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error on failure.
    /// Errors are recorded in the [`HookResult`](crate::hooks::HookResult) but
    /// do not stop dispatch to other plugins.
    fn on_hook(&self, _event: HookEvent) -> Result<(), HookError> {
        Ok(())
    }

    /// Execute a tool by name with the given arguments.
    ///
    /// This is the core dispatch point. The registry looks up the tool
    /// in its index and calls this method with the tool's full name
    /// (including namespace) and the JSON arguments from the MCP request.
    ///
    /// # Dispatch Flow
    ///
    /// 1. MCP client calls `tools/call` with tool name and arguments
    /// 2. Server checks built-in tools first
    /// 3. If not found, server delegates to [`PluginRegistry::execute_tool`]
    /// 4. Registry looks up tool in index, finds plugin
    /// 5. Registry calls this method with tool name and arguments
    ///
    /// # Arguments
    ///
    /// * `name` - The full tool name including namespace (e.g., "git::status")
    /// * `args` - JSON object with tool-specific arguments
    ///
    /// # Returns
    ///
    /// Returns the tool's result as JSON, or a [`PluginError`] if the tool
    /// was not found or execution failed. The JSON structure should match
    /// what the tool's `input_schema` declares.
    fn execute_tool(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, error::PluginError>;
}
