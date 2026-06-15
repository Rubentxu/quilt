//! Projection endpoint handler
//!
//! `GET /api/v1/blocks/:id/projection` — resolves the winning projection for a block.

use crate::error::AppError;
use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use quilt_domain::projection::projection_trait::ProjectionContext;
use quilt_domain::value_objects::Uuid;
use quilt_application::use_cases::projection_resolver::ProjectionResolver;
use std::sync::Arc;
use tracing::instrument;

/// GET /api/v1/blocks/:id/projection
///
/// Resolves the winning projection for the block and returns the `ProjectionView`.
#[instrument(skip_all, fields(block_id = %block_id))]
pub async fn get_projection(
    Path(block_id): Path<String>,
    Extension(resolver): Extension<Arc<ProjectionResolver>>,
    Extension(block_repo): Extension<Arc<dyn quilt_domain::repositories::BlockRepository>>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Parse and fetch the block
    let id = Uuid::parse_str(&block_id)
        .map_err(|_| AppError::BadRequest(format!("Invalid block UUID: {block_id}")))?;

    let block = block_repo
        .get_by_id(id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Block {block_id} not found")))?;

    // 2. Build resolution context
    let ctx = ProjectionContext::page(Utc::now());

    // 3. Resolve the projection
    let outcome = resolver.resolve(&block, &ctx)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // 4. Emit tracing event
    tracing::info!(
        block_id = %block_id,
        projection_id = %outcome.winner_id.as_ref().map(|id| id.as_str()).unwrap_or("default"),
        conflict_count = outcome.view.conflicts.len(),
        "Projection resolved"
    );

    // 5. Return response with cache headers
    let response = Json(outcome.view);
    Ok((
        StatusCode::OK,
        [("cache-control", "private, max-age=30")],
        response,
    ))
}
