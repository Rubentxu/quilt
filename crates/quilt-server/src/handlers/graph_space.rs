//! GraphSpace HTTP handlers

use axum::extract::Extension;
use axum::{Json, Router, routing::get};
use serde::Deserialize;
use tracing::instrument;

use crate::error::AppError;
use quilt_domain::entities::GraphSpace;
use quilt_domain::repositories::GraphSpaceRepository;
use std::sync::Arc;

/// Create router for /api/v1/graph-space
pub fn routes() -> Router {
    Router::new().route("/", get(get_graph_space).put(update_graph_space))
}

/// GET /api/v1/graph-space
#[instrument(skip(graph_space_repo))]
pub async fn get_graph_space(
    Extension(graph_space_repo): Extension<Arc<dyn GraphSpaceRepository>>,
) -> Result<Json<GraphSpace>, AppError> {
    let graph_space = graph_space_repo
        .get_graph_space()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(graph_space))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGraphSpaceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// PUT /api/v1/graph-space
///
/// Partial update: only provided fields are updated. Missing fields keep their current value.
#[instrument(skip(graph_space_repo))]
pub async fn update_graph_space(
    Extension(graph_space_repo): Extension<Arc<dyn GraphSpaceRepository>>,
    Json(req): Json<UpdateGraphSpaceRequest>,
) -> Result<Json<GraphSpace>, AppError> {
    // Fetch current settings
    let current = graph_space_repo
        .get_graph_space()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Merge with updates
    let updated = GraphSpace {
        name: req.name.unwrap_or(current.name),
        description: req.description.unwrap_or(current.description),
        version: current.version,
    };

    updated
        .validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    graph_space_repo
        .update_graph_space(&updated)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(updated))
}
