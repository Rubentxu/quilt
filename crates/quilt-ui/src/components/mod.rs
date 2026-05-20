//! Reusable UI components

pub mod agent_panel;
pub mod block;
pub mod block_item;
pub mod conflict_resolution;
pub mod empty_state;
pub mod loading;
pub mod outliner_block;
pub mod outliner_tree;
pub mod sidebar;
pub mod sync_status;
pub mod task_item;
pub use agent_panel::AgentPanel;
pub use block::{Block, EmptyState};
pub use block_item::BlockItem;
pub use conflict_resolution::{ConflictCard, ConflictDetector, ConflictDisplay, ConflictMarker};

pub use loading::Loading;
pub use outliner_block::{Marker, OutlinerBlock, Priority};
pub use outliner_tree::{build_tree, OutlinerTree, TreeBlock};
/// Re-export common components
pub use sidebar::Sidebar;
pub use sync_status::{SyncStateDisplay, SyncStatusCompact};
pub use task_item::TaskItem;
