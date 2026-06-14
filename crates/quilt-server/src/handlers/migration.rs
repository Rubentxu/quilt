//! Migration-related HTTP handlers
//!
//! Endpoints for importing Markdown files into Quilt.

use axum::{Json, Router,extract::Extension, http::StatusCode, routing::post};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::instrument;

use crate::error::AppError;
use quilt_application::migration::MigrationEngine;
use quilt_domain::repositories::{BlockRepository, PageRepository, PropertyRepository};

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
    /// Warning messages (e.g. page collisions)
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

/// Validate a user-provided path against the vault base directory.
///
/// Security hardening for path traversal attacks:
/// 1. Canonicalize the user path to resolve symlinks
/// 2. Verify the canonical path is within the vault base
/// 3. Reject symlinks (DOS prevention and security)
/// 4. Enforce a file count limit to prevent DOS
fn validate_path(vault_base: &Path, user_path: &str) -> Result<PathBuf, AppError> {
    // 1. Parse the user path
    let raw = PathBuf::from(user_path);

    // 2. Canonicalize to resolve symlinks and get absolute path
    let canonical = raw
        .canonicalize()
        .map_err(|_| AppError::BadRequest("Path does not exist or is inaccessible".into()))?;

    // 3. Verify the path is within the vault base
    let base = vault_base
        .canonicalize()
        .map_err(|_| AppError::Internal("Vault base path is invalid".into()))?;
    if !canonical.starts_with(&base) {
        return Err(AppError::BadRequest(
            "Path is outside the allowed vault directory".into(),
        ));
    }

    // 4. Reject symlinks (security: prevent traversing symlinks outside vault)
    let metadata = fs::symlink_metadata(&canonical)
        .map_err(|_| AppError::Internal("Cannot read path metadata".into()))?;
    if metadata.file_type().is_symlink() {
        return Err(AppError::BadRequest("Symlinks are not allowed".into()));
    }

    // 5. File count limit (DOS prevention)
    if metadata.is_dir() {
        let count = fs::read_dir(&canonical)
            .map_err(|e| AppError::Internal(format!("Cannot read directory: {}", e)))?;
        const MAX_FILES: usize = 10_000;
        let file_count = count.count();
        if file_count > MAX_FILES {
            return Err(AppError::BadRequest(format!(
                "Directory contains too many files ({} > {})",
                file_count, MAX_FILES
            )));
        }
    }

    Ok(canonical)
}

/// Get the vault base directory.
///
/// Uses QUILT_VAULT_BASE environment variable, defaulting to the current directory.
fn get_vault_base() -> PathBuf {
    std::env::var("QUILT_VAULT_BASE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// POST /api/v1/migration/md
///
/// Import Markdown files from a directory into Quilt.
///
/// The directory should contain `.md` files in Quilt format.
/// Each file becomes a page, and nested blocks are preserved.
#[instrument(skip(page_repo, block_repo, property_repo))]
pub async fn migrate_md_import(
    Extension(page_repo): Extension<Arc<dyn PageRepository>>,
    Extension(block_repo): Extension<Arc<dyn BlockRepository>>,
    Extension(property_repo): Extension<Arc<dyn PropertyRepository>>,
    Json(body): Json<ImportMdRequest>,
) -> Result<(StatusCode, Json<ImportMdResponse>), AppError> {
    let vault_base = get_vault_base();
    let path = validate_path(&vault_base, &body.path)?;

    if !path.is_dir() {
        return Err(AppError::BadRequest(format!(
            "Path is not a directory: {}",
            body.path
        )));
    }

    // Create migration engine with injected repositories
    let engine = MigrationEngine::new(page_repo, block_repo, property_repo);

    // Import all files from directory
    let results = engine
        .import_directory(&path)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Convert results to DTOs
    let result_dtos: Vec<ImportResultDto> =
        results.into_iter().map(ImportResultDto::from).collect();

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
