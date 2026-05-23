//! Reusable UI components

pub mod agent_notification_feed;
pub mod agent_panel;
pub mod agent_status;
pub mod annotations_overlay;
pub mod backlinks_panel;
pub mod block;
pub mod block_item;
pub mod conflict_resolution;
pub mod empty_state;
pub mod error_boundary;
pub mod graph_switcher;
pub mod keyboard_shortcuts;
pub mod loading;
pub mod mini_backlinks_graph;
pub mod outliner_block;
pub mod outliner_tree;
pub mod properties_editor;
pub mod retry_button;
pub mod right_sidebar;
pub mod sidebar;
pub mod slash_command;
pub mod sync_status;
pub mod task_item;
pub mod theme;
pub mod toast;
pub mod toast_container;
pub mod virtual_list;
pub use agent_notification_feed::{
    ActivityItem, ActivityType, AgentActivityStream, AgentNotification, AgentNotificationFeed,
    NotificationType,
};
pub use agent_panel::AgentPanel;
pub use agent_status::{AgentActivity, AgentStatus, AgentStatusBar};
pub use annotations_overlay::{Annotation, AnnotationType, AnnotationsOverlay};
pub use backlinks_panel::{Backlink, BacklinksPanel, RelationshipType};
pub use block::{Block, EmptyState};
pub use block_item::BlockItem;
pub use conflict_resolution::{ConflictCard, ConflictDetector, ConflictDisplay, ConflictMarker};
pub use error_boundary::ErrorDisplay;
pub use graph_switcher::GraphSwitcher;
pub use keyboard_shortcuts::{KeyboardShortcuts, ShortcutAction};

pub use loading::Loading;
pub use outliner_block::{Marker, OnDedent, OnDeleteEmpty, OnIndent, OutlinerBlock, Priority};
pub use outliner_tree::{build_tree, OutlinerTree, TreeBlock};
pub use properties_editor::{BlockProperties, PropertiesEditor, Property};
pub use retry_button::RetryButton;
pub use right_sidebar::{RightSidebar, RightSidebarToggle, SidebarTab};
/// Re-export common components
pub use sidebar::Sidebar;
pub use slash_command::{SlashCommand, SlashCommandPalette};
pub use sync_status::{SyncStateDisplay, SyncStatusCompact};
pub use task_item::TaskItem;
pub use toast::{show_error, show_info, show_success, show_warning, Toast, ToastState, ToastType};
pub use toast_container::ToastContainer;
pub use virtual_list::VirtualListViewport;
