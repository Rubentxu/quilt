//! Migration-related HTTP handlers (GS-9).
//!
//! Endpoints for importing Markdown files into Quilt, scanning for candidates,
//! and reindexing changed files.

use axum::{
    Json, Router,
    extract::{Extension, Query},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::instrument;

use crate::error::AppError;
use crate::state::AppState;
use quilt_application::use_cases::MigrationUseCases;
use quilt_platform::graph_validation::validate_graph_layout;

// ── Request/Response DTOs ────────────────────────────────────────────────────

/// Query params for GET /candidates
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidatesParams {
    /// Sub-path within the graph root to scope the scan (relative path).
    /// Defaults to the graph root itself (".").
    pub path: Option<String>,
    /// Maximum directory depth for the recursive scan.
    /// Defaults to 8.
    pub depth: Option<u32>,
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

/// Request body for POST /migration/reindex and refactored /migration/md
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationPlanRequest {
    /// The ingestion plan from a prior scan
    pub plan: quilt_application::migration::IngestionPlan,
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

// ── Graph Root Resolution ───────────────────────────────────────────────────

/// Resolve the active graph root from AppState, or return 503.
///
/// Reads `state.last_opened_graph`, returns 503 if None.
/// If a path exists, validates it via `validate_graph_layout` (ADR-0030 §6).
/// Returns 422 if the layout is invalid.
fn resolve_graph_root(state: &AppState) -> Result<PathBuf, AppError> {
    let graph_path = state.last_opened_graph.blocking_read();
    let path = graph_path
        .as_ref()
        .ok_or_else(|| AppError::ServiceUnavailable("No graph is currently open".into()))?;

    // Validate the graph layout (ADR-0030 §6)
    validate_graph_layout(path)?;

    Ok(path.clone())
}

// ── Path Validation ────────────────────────────────────────────────────────

/// Validate a user-provided path against the graph root.
///
/// Security hardening for path traversal attacks:
/// 1. Canonicalize the user path to resolve symlinks
/// 2. Verify the canonical path is within the graph root
/// 3. Reject symlinks (DOS prevention and security)
/// 4. Enforce a file count limit to prevent DOS
fn validate_path(graph_root: &Path, user_path: &str) -> Result<PathBuf, AppError> {
    // 1. Parse the user path
    let raw = PathBuf::from(user_path);

    // 2. Canonicalize to resolve symlinks and get absolute path
    let canonical = raw
        .canonicalize()
        .map_err(|_| AppError::BadRequest("Path does not exist or is inaccessible".into()))?;

    // 3. Verify the path is within the graph root
    let base = graph_root
        .canonicalize()
        .map_err(|_| AppError::Internal("Graph root path is invalid".into()))?;
    if !canonical.starts_with(&base) {
        return Err(AppError::BadRequest(
            "Path is outside the allowed graph directory".into(),
        ));
    }

    // 4. Reject symlinks (security: prevent traversing symlinks outside graph)
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

// ── Handlers ───────────────────────────────────────────────────────────────

/// `GET /api/v1/migration/candidates`
///
/// Perform a read-only scan of the graph directory for `.md` files
/// and return an ingestion plan with per-file status (new/modified/skipped).
///
/// Returns 503 if no graph is open.
/// Returns 422 if the graph layout is invalid.
/// Returns 400 if the path escapes the graph root.
#[instrument(skip(state))]
pub async fn candidates(
    Extension(state): Extension<Arc<AppState>>,
    Query(params): Query<CandidatesParams>,
) -> Result<Json<quilt_application::migration::IngestionPlan>, AppError> {
    let graph_root = resolve_graph_root(&state)?;

    // Resolve the sub-path (defaults to graph root itself)
    let scan_path = match &params.path {
        Some(p) if p != "." => {
            let resolved = validate_path(&graph_root, p)?;
            // Further scope to the requested subdirectory
            if !resolved.is_dir() {
                return Err(AppError::BadRequest(format!(
                    "Path is not a directory: {}",
                    p
                )));
            }
            resolved
        }
        _ => graph_root.clone(),
    };

    let depth = params.depth.unwrap_or(8);

    // Build MigrationUseCases from repos
    let use_cases = MigrationUseCases::new(
        state.repos.page.clone(),
        state.repos.block.clone(),
        state.repos.property.clone(),
    );

    let plan = use_cases
        .scan(&scan_path, depth)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(plan))
}

/// `POST /api/v1/migration/md`
///
/// Ingest NEW pages from an approved ingestion plan.
///
/// This endpoint only processes candidates with status "new".
/// Candidates with status "modified" or "skipped" are ignored.
///
/// Returns 503 if no graph is open.
/// Returns 422 if the graph layout is invalid.
#[instrument(skip(state))]
pub async fn migrate_md_import(
    Extension(state): Extension<Arc<AppState>>,
    Json(body): Json<MigrationPlanRequest>,
) -> Result<(StatusCode, Json<ImportMdResponse>), AppError> {
    let _graph_root = resolve_graph_root(&state)?;

    let use_cases = MigrationUseCases::new(
        state.repos.page.clone(),
        state.repos.block.clone(),
        state.repos.property.clone(),
    );

    let result = use_cases
        .ingest(&body.plan)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let results: Vec<ImportResultDto> = result
        .results
        .into_iter()
        .map(|e| ImportResultDto {
            pages_created: e.pages_created,
            blocks_created: e.blocks_created,
            warnings: e.warning.into_iter().collect(),
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(ImportMdResponse {
            total_pages_created: result.total_pages_created,
            total_blocks_created: result.total_blocks_created,
            warnings: result.warnings,
            results,
        }),
    ))
}

/// `POST /api/v1/migration/reindex`
///
/// Reindex MODIFIED pages from an approved ingestion plan.
///
/// This endpoint only processes candidates with status "modified".
/// Candidates with status "new" or "skipped" are ignored.
///
/// Returns 503 if no graph is open.
/// Returns 422 if the graph layout is invalid.
#[instrument(skip(state))]
pub async fn reindex(
    Extension(state): Extension<Arc<AppState>>,
    Json(body): Json<MigrationPlanRequest>,
) -> Result<Json<quilt_application::migration::ReindexResult>, AppError> {
    let _graph_root = resolve_graph_root(&state)?;

    let use_cases = MigrationUseCases::new(
        state.repos.page.clone(),
        state.repos.block.clone(),
        state.repos.property.clone(),
    );

    let result = use_cases
        .reindex(&body.plan)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(result))
}

/// Create router for /api/v1/migration
pub fn routes() -> Router {
    Router::new()
        .route("/candidates", get(candidates))
        .route("/md", post(migrate_md_import))
        .route("/reindex", post(reindex))
}
