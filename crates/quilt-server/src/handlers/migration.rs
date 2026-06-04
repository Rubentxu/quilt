//! Migration-related HTTP handlers
//!
//! Endpoints for importing Markdown files into Quilt.

use axum::{
    Json,
    extract::Extension,
    http::StatusCode,
    Router,
    routing::post,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::instrument;

use crate::error::AppError;
use crate::state::AppState;
use quilt_application::migration::MigrationEngine;
use quilt_infrastructure::database::sqlite::repositories::{
    SqliteBlockRepository, SqlitePageRepository,
};

/// Request body for POST /api/v1/migration/md
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportMdRequest {
    /// Path to the directory containing Markdown files
    pub path: String,
}

/// Response for a single file import result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResultDto {
    /// Number of pages created
    pub pages_created: usize,
    /// Number of blocks created
    pub blocks_created: usize,
    /// Warning messages (e.g., page collisions)
    pub warnings: Vec<String>,
}

/// Response for the migration endpoint
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportMdResponse {
    /// Results for each file that was imported
    pub results: Vec<ImportResultDto>,
    /// Total pages created across all files
    pub total_pages_created: usize,
    /// Total blocks created across all files
    pub total_blocks_created: usize,
    /// All warnings from all imports
    pub warnings: Vec<String>,
}

impl From<quilt_application::migration::ImportResult> for ImportResultDto {
    fn from(result: quilt_application::migration::ImportResult) -> Self {
        Self {
            pages_created: result.pages_created,
            blocks_created: result.blocks_created,
            warnings: result.warnings,
        }
    }
}

/// POST /api/v1/migration/md
///
/// Import Markdown files from a directory into Quilt.
///
/// The directory should contain `.md` files in Logseq/Quilt format.
/// Each file becomes a page, and nested blocks are preserved.
#[instrument(skip(state))]
pub async fn migrate_md_import(
    Extension(state): Extension<AppState>,
    Json(body): Json<ImportMdRequest>,
) -> Result<(StatusCode, Json<ImportMdResponse>), AppError> {
    // Validate path
    let path = PathBuf::from(&body.path);
    
    if !path.exists() {
        return Err(AppError::BadRequest(format!(
            "Path does not exist: {}",
            body.path
        )));
    }
    
    if !path.is_dir() {
        return Err(AppError::BadRequest(format!(
            "Path is not a directory: {}",
            body.path
        )));
    }
    
    // Create repositories and migration engine
    let page_repo = SqlitePageRepository::new(state.pool.clone());
    let block_repo = SqliteBlockRepository::new(state.pool.clone());
    let engine = MigrationEngine::new(Arc::new(page_repo), Arc::new(block_repo));
    
    // Import all files from directory
    let results = engine
        .import_directory(&path)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    
    // Convert results to DTOs
    let result_dtos: Vec<ImportResultDto> = results
        .into_iter()
        .map(ImportResultDto::from)
        .collect();
    
    // Calculate totals
    let total_pages_created: usize = result_dtos.iter().map(|r| r.pages_created).sum();
    let total_blocks_created: usize = result_dtos.iter().map(|r| r.blocks_created).sum();
    let warnings: Vec<String> = result_dtos
        .iter()
        .flat_map(|r| r.warnings.clone())
        .collect();
    
    Ok((
        StatusCode::OK,
        Json(ImportMdResponse {
            results: result_dtos,
            total_pages_created,
            total_blocks_created,
            warnings,
        }),
    ))
}

/// Create router for /api/v1/migration
pub fn routes() -> Router {
    Router::new().route("/md", post(migrate_md_import))
}
