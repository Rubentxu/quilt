//! Cognitive Engine — Common Trait for All Cognitive Engines
//!
//! This module defines the `CognitiveEngine` trait that all cognitive engines
//! must implement, providing a common interface for the registry and reducing
//! boilerplate in the services layer.

use async_trait::async_trait;
use std::fmt::Debug;

/// Common trait implemented by all cognitive engines.
///
/// This trait provides a minimal interface that all engines share,
/// enabling generic handling in the registry and services layer.
///
/// # Implementors
///
/// All cognitive engines should implement this trait in addition to their
/// engine-specific trait (e.g., `CognitiveMirrorEngine`, `SerendipityEngineTrait`).
///
/// # Example
///
/// ```ignore
/// use crate::engine::CognitiveEngine;
///
/// struct MyEngine { ... }
///
/// impl CognitiveEngine for MyEngine {
///     fn engine_type(&self) -> &str { "my_engine" }
///     fn engine_name(&self) -> &str { "My Engine" }
/// }
/// ```
#[async_trait]
pub trait CognitiveEngine: Debug + Send + Sync {
    /// Returns the type identifier for this engine.
    ///
    /// This should be a lowercase, hyphenated identifier (e.g., "cognitive-mirror", "serendipity").
    fn engine_type(&self) -> &str;

    /// Returns the human-readable name for this engine.
    ///
    /// This is used for logging, debugging, and user-facing displays.
    fn engine_name(&self) -> &str;
}
