//! Graph Space endpoints (ADR-0030, Slice B + Slice C).
//!
//! Exposes:
//! - `POST /api/v1/graphs/validate` — validate a graph layout, return 200 or 422
//! - `GET /api/v1/graphs/recent` — list recent graphs from global state
//! - `POST /api/v1/graphs/create` — create a graph or open an existing one
//!
//! Auth: required (Bearer token, enforced by the global middleware).

use axum::{
    Json, Router,
    extract::Extension,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::instrument;

use quilt_platform::graph_validation::validate_graph_layout;
use quilt_platform::init::{init_graph, init_graph_validated};

use crate::error::AppError;
use crate::state::AppState;

/// Body for `POST /api/v1/graphs/validate`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateGraphRequest {
    /// Absolute path to the graph root directory.
    pub graph_path: String,
}

/// Body for `POST /api/v1/graphs/create`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGraphRequest {
    /// Absolute path to the graph root directory.
    pub graph_path: String,
}

/// Response for `POST /api/v1/graphs/create` and `GET /api/v1/graphs/recent`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGraphResponse {
    pub graph_path: String,
    pub created: bool,
}

/// Response for `GET /api/v1/graphs/recent`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentGraphsResponse {
    pub recent_graphs: Vec<String>,
}

/// Create the router for `/api/v1/graphs`.
pub fn routes() -> Router {
    Router::new()
        .route("/validate", post(validate_graph))
        .route("/recent", get(list_recent))
        .route("/create", post(create_graph))
}

/// `POST /api/v1/graphs/validate`
///
/// Validate a graph layout and return 200 (valid) or 422
/// (structured `GRAPH_INVALID` body with the typed `validationError`).
pub async fn validate_graph(
    Json(payload): Json<ValidateGraphRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let graph_path = PathBuf::from(payload.graph_path);

    if graph_path.as_os_str().is_empty() {
        return Err(AppError::BadRequest(
            "graphPath is required and must be non-empty".to_string(),
        ));
    }
    if !graph_path.is_absolute() {
        return Err(AppError::BadRequest(
            "graphPath must be an absolute path".to_string(),
        ));
    }

    let layout = validate_graph_layout(&graph_path)?;
    Ok(Json(serde_json::json!({
        "graphPath": layout.graph_path.display().to_string(),
        "dbPath": layout.db_path.display().to_string(),
        "quiltDir": layout.quilt_dir.display().to_string(),
        "valid": true,
    })))
}

/// `GET /api/v1/graphs/recent`
///
/// Return the list of recently opened graphs (most-recent-first).
#[instrument(skip(state))]
pub async fn list_recent(
    Extension(state): Extension<AppState>,
) -> Result<Json<RecentGraphsResponse>, AppError> {
    let graphs = state.recent_graphs.read().await;
    let recent: Vec<String> = graphs.iter().map(|p| p.display().to_string()).collect();
    Ok(Json(RecentGraphsResponse {
        recent_graphs: recent,
    }))
}

/// `POST /api/v1/graphs/create`
///
/// Create a new graph or open an existing one at the given path.
/// Returns 201 if a new graph was created, 200 if it already existed.
/// Fails with 400 if the path is empty or not absolute.
/// Fails with 422 if validation fails for an existing graph.
#[instrument(skip(state))]
pub async fn create_graph(
    Extension(state): Extension<AppState>,
    Json(payload): Json<CreateGraphRequest>,
) -> Result<Json<CreateGraphResponse>, AppError> {
    let graph_path = PathBuf::from(payload.graph_path);

    if graph_path.as_os_str().is_empty() {
        return Err(AppError::BadRequest(
            "graphPath is required and must be non-empty".to_string(),
        ));
    }
    if !graph_path.is_absolute() {
        return Err(AppError::BadRequest(
            "graphPath must be an absolute path".to_string(),
        ));
    }

    // Check if the layout already exists (→ existing graph) or needs creation
    let layout_already_exists = graph_path.join(".quilt").join("quilt.db").exists();
    let created;

    if layout_already_exists {
        // Validate strictly; this returns a typed error on failure
        match init_graph_validated(graph_path.clone()) {
            Ok(_) => {
                created = false;
            }
            Err(quilt_platform::init::GraphError::Validation(gve)) => {
                return Err(AppError::GraphInvalid(gve));
            }
            Err(e) => {
                return Err(AppError::Internal(e.to_string()));
            }
        }
    } else {
        // Fresh create — init_graph handles idempotent creation
        init_graph(graph_path.clone())
            .map_err(|e| AppError::Internal(format!("failed to create graph layout: {}", e)))?;
        created = true;
    }

    // Write-through to global state: set last_opened + push to recents
    let path_for_state = graph_path.clone();
    if let Err(e) = state
        .global_state_repo
        .set_last_opened_graph(Some(&path_for_state))
        .await
    {
        tracing::warn!("failed to persist last_opened_graph after create: {}", e);
    }
    if let Err(e) = state.global_state_repo.push_recent(&path_for_state).await {
        tracing::warn!("failed to push recent after create: {}", e);
    }

    // Update in-memory caches
    {
        let mut last = state.last_opened_graph.write().await;
        *last = Some(path_for_state.clone());
    }
    {
        let mut recents = state.recent_graphs.write().await;
        recents.retain(|p| p != &path_for_state);
        recents.insert(0, path_for_state.clone());
        recents.truncate(10);
    }

    Ok(Json(CreateGraphResponse {
        graph_path: graph_path.display().to_string(),
        created,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use quilt_platform::graph_validation::GraphValidationError;
    use tempfile::tempdir;

    #[test]
    fn request_serde_uses_camel_case() {
        let req: ValidateGraphRequest =
            serde_json::from_str(r#"{"graphPath":"/var/data/g1"}"#).unwrap();
        assert_eq!(req.graph_path, "/var/data/g1");
    }

    #[test]
    fn validation_passes_for_valid_layout() {
        use rusqlite::Connection;
        let tmp = tempdir().unwrap();
        let quilt_dir = tmp.path().join(".quilt");
        std::fs::create_dir_all(&quilt_dir).unwrap();
        let db_path = quilt_dir.join("quilt.db");
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch("CREATE TABLE user_settings (id INTEGER PRIMARY KEY);")
            .unwrap();

        let layout = validate_graph_layout(tmp.path()).expect("valid");
        assert_eq!(layout.graph_path, tmp.path());
    }

    #[test]
    fn validation_fails_for_missing_directory() {
        let tmp = tempdir().unwrap();
        let p = tmp.path().join("nope");
        let err = validate_graph_layout(&p).expect_err("must fail");
        assert!(matches!(err, GraphValidationError::DirectoryMissing(_)));
        assert_eq!(err.code(), "DirectoryMissing");
    }
}
