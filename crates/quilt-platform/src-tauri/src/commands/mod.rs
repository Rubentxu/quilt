//! Tauri IPC commands
//!
//! These commands are the bridge between the frontend and backend.

pub mod agent;
pub mod ai_config;
pub mod blocks;
pub mod cognitive;
pub mod navigation;
pub mod pages;

pub use agent::query_agent;
pub use ai_config::{configure_ai_provider, get_ai_status};
pub use blocks::{
    create_block, create_task, delete_block, get_backlinks, get_block_tree, link_blocks,
    query_blocks, search_blocks,
};
pub use cognitive::{
    argument_map, cognitive_available, cognitive_mirror, get_availability, mental_model,
    morning_briefing, serendipity,
};
pub use navigation::{navigate_to_block, navigate_to_page};
pub use pages::{create_page, get_journal, get_page, list_pages};
