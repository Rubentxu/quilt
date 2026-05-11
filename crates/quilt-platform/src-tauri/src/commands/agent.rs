//! Agent-related Tauri commands

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Agent query response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentQueryDto {
    pub available: bool,
    pub message: Option<String>,
    pub page_name: Option<String>,
}

/// Query the agent for a specific page
///
/// This command allows the frontend to query the MCP server for agent-related
/// information about a page.
#[tauri::command]
pub async fn query_agent(
    page_name: String,
    state: State<'_, AppState>,
) -> Result<AgentQueryDto, String> {
    // The MCP server is available but agent queries require the MCP client
    // which is not yet wired in the Tauri context
    let _ = state.mcp_server;
    let _ = page_name;

    Ok(AgentQueryDto {
        available: false,
        message: Some("Agent queries require MCP client wiring".to_string()),
        page_name: None,
    })
}
