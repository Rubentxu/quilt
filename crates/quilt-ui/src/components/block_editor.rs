//! Shared editor types.
//!
//! NOTE: The legacy `BlockEditor` contenteditable component was removed.
//!       It has been replaced by `Cm6BlockEditor` (see `cm6_block_editor.rs`).
//!       This file now retains only the shared `TreeOps` struct used by both
//!       `block.rs` and `cm6_block_editor.rs`.
//!
//! Legacy files still on disk (NOT declared as modules, not compiled):
//! - `outliner_block.rs` — separate tree-view editing path (contenteditable)
//! - `outliner_tree.rs`   — tree-view renderer (uses outliner_block)
//!
//! These may be re-enabled in a future phase if the tree-view UI is revived.

use std::sync::Arc;

pub struct TreeOps {
    pub on_indent: Arc<dyn Fn() + Send + Sync>,
    pub on_outdent: Arc<dyn Fn() + Send + Sync>,
    pub on_split: Arc<dyn Fn(u32) + Send + Sync>,
    pub on_merge_next: Arc<dyn Fn() + Send + Sync>,
}

impl Default for TreeOps {
    fn default() -> Self {
        Self {
            on_indent: Arc::new(|| {}),
            on_outdent: Arc::new(|| {}),
            on_split: Arc::new(|_| {}),
            on_merge_next: Arc::new(|| {}),
        }
    }
}

impl Clone for TreeOps {
    fn clone(&self) -> Self {
        Self {
            on_indent: self.on_indent.clone(),
            on_outdent: self.on_outdent.clone(),
            on_split: self.on_split.clone(),
            on_merge_next: self.on_merge_next.clone(),
        }
    }
}
