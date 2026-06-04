//! HTTP handler for query execution (F18).
//!
//! `POST /api/v1/query` — execute a QueryAst and return matching blocks.
//!
//! Body: `{ ast: QueryAst, limit?: number }`
//! Response: `{ results: Block[], total: number, elapsed_ms: number }`

use crate::error::AppError;
use crate::state::AppState;
use axum::{Json, Router, extract::Extension, routing::post};
use quilt_application::query::executor::QueryExecutorService;
use quilt_domain::entities::Block;
use quilt_infrastructure::database::sqlite::repositories::SqliteBlockRepository;
use quilt_query::ast::QueryAst;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tracing::instrument;

/// Router for /api/v1/query
pub fn routes() -> Router {
    Router::new().route("/", post(execute_query))
}

/// Request body for POST /api/v1/query.
#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub ast: QueryAst,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    100
}

/// Response body for POST /api/v1/query.
#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub results: Vec<Block>,
    pub total: usize,
    pub elapsed_ms: u64,
}

#[instrument(skip(state))]
pub async fn execute_query(
    Extension(state): Extension<AppState>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, AppError> {
    let start = Instant::now();

    // Cap limit at 1000 (server-side constraint)
    let effective_limit = req.limit.min(1000);

    // Build the query executor service
    let block_repo = Arc::new(SqliteBlockRepository::new(state.pool.clone()));
    let executor = QueryExecutorService::new(block_repo);

    // Execute the query
    let blocks = executor
        .execute(&req.ast, effective_limit)
        .await
        .map_err(|e| AppError::BadRequest(format!("Query error: {}", e)))?;

    let total = blocks.len();
    let elapsed_ms = start.elapsed().as_millis() as u64;

    Ok(Json(QueryResponse {
        results: blocks,
        total,
        elapsed_ms,
    }))
}
