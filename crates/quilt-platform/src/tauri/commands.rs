//! Tauri IPC commands
//!
//! These commands are the bridge between the frontend (FASE 7) and backend.
//! They mirror the CLI commands but return JSON-serializable results.

// Future command signatures (when Tauri is enabled):
//
// #[tauri::command]
// async fn query_blocks(dsl: String, limit: usize) -> Result<Vec<BlockDto>, String> { ... }
//
// #[tauri::command]
// async fn create_block(page_name: String, content: String) -> Result<BlockDto, String> { ... }
//
// #[tauri::command]
// async fn search_blocks(query: String) -> Result<Vec<SearchResultDto>, String> { ... }
//
// #[tauri::command]
// async fn get_page(name: String) -> Result<PageDto, String> { ... }
//
// #[tauri::command]
// async fn list_pages() -> Result<Vec<PageDto>, String> { ... }
//
// #[tauri::command]
// async fn get_journal(date: String) -> Result<PageDto, String> { ... }
//
// #[tauri::command]
// async fn delete_block(block_id: String) -> Result<(), String> { ... }
//
// #[tauri::command]
// async fn link_blocks(source_id: String, target_id: String) -> Result<(), String> { ... }
//
// #[tauri::command]
// async fn get_backlinks(target_id: String) -> Result<Vec<BlockDto>, String> { ... }

// Placeholder — remove when Tauri is enabled
