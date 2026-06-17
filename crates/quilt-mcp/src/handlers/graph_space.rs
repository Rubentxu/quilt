//! GraphSpace handler implementation for MCP tools.
//!
//! Implements [`GraphSpaceHandler`](super::GraphSpaceHandler) trait for graph_space
//! MCP tools like get_graph_space, update_graph_space.

use super::{GraphSpaceHandler, HandlerResult};
use async_trait::async_trait;
use quilt_domain::repositories::GraphSpaceRepository;
use serde::Serialize;
use std::sync::Arc;
use tracing::instrument;

/// Wire shape for the `get_graph_space` response.
#[derive(Serialize)]
struct GetGraphSpaceResponse {
    name: String,
    description: String,
    version: String,
}

/// Wire shape for the `update_graph_space` response.
#[derive(Serialize)]
struct UpdateGraphSpaceResponse {
    status: &'static str,
}

/// Default implementation of [`GraphSpaceHandler`].
pub struct DefaultGraphSpaceHandler {
    graph_space_repo: Arc<dyn GraphSpaceRepository>,
}

impl DefaultGraphSpaceHandler {
    /// Create a new graph_space handler.
    pub fn new(graph_space_repo: Arc<dyn GraphSpaceRepository>) -> Self {
        Self { graph_space_repo }
    }
}

#[async_trait]
impl GraphSpaceHandler for DefaultGraphSpaceHandler {
    #[instrument(skip(self))]
    async fn get_graph_space(&self) -> HandlerResult {
        let graph_space = self
            .graph_space_repo
            .get_graph_space()
            .await
            .map_err(|e| e.to_string())?;

        let response = GetGraphSpaceResponse {
            name: graph_space.name,
            description: graph_space.description,
            version: graph_space.version,
        };
        Ok(serde_json::to_string_pretty(&response)
            .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    #[instrument(skip(self))]
    async fn update_graph_space(&self, graph_space_json: serde_json::Value) -> HandlerResult {
        let name = graph_space_json
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing or invalid name".to_string())?;
        let description = graph_space_json
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let graph_space = quilt_domain::entities::GraphSpace {
            name: name.to_string(),
            description: description.to_string(),
            version: "1.0".to_string(),
        };

        graph_space
            .validate()
            .map_err(|e| e.to_string())?;

        self.graph_space_repo
            .update_graph_space(&graph_space)
            .await
            .map_err(|e| e.to_string())?;

        let response = UpdateGraphSpaceResponse { status: "updated" };
        serde_json::to_string(&response).map_err(|e| e.to_string())
    }
}
