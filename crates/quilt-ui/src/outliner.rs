//! Outliner module — core outliner logic
//!
//! Handles block tree operations:
//! - Building tree from flat list
//! - Indent/outdent
//! - Split/merge blocks
//! - Ordering
//! - Undo/redo history

pub mod history;
pub mod page;
pub mod tree;
