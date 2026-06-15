//! Presets endpoint handler
//!
//! `GET /api/v1/presets` — lists all V1 property presets.

use crate::error::AppError;
use axum::{
    extract::Extension,
    http::StatusCode,
    response::IntoResponse,
    Json,
    Router,
    routing::get,
};
use quilt_domain::canonicalization::{PresetId, PresetRegistry, PropertyPreset};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::instrument;

/// Response DTO for a single preset.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetResponse {
    /// Preset identifier (e.g., "/TODO").
    pub id: String,
    /// Human-readable label derived from the preset id.
    pub label: String,
    /// Human-readable description.
    pub description: String,
    /// Required argument kinds for this preset.
    pub required_args: Vec<String>,
    /// Keywords for search (empty for V1 presets).
    pub keywords: Vec<String>,
}

impl From<&PropertyPreset> for PresetResponse {
    fn from(preset: &PropertyPreset) -> Self {
        let label = preset.id.to_string(); // includes the leading /
        let required_args: Vec<String> = preset
            .required_args
            .iter()
            .map(|arg| match arg {
                quilt_domain::canonicalization::PresetArg::Date(_) => "date".to_string(),
                quilt_domain::canonicalization::PresetArg::Url(_) => "url".to_string(),
                quilt_domain::canonicalization::PresetArg::Text(_) => "text".to_string(),
            })
            .collect();

        Self {
            id: preset.id.to_string(),
            label,
            description: preset.description.clone(),
            required_args,
            keywords: vec![],
        }
    }
}

/// Response envelope for the presets list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetListResponse {
    pub presets: Vec<PresetResponse>,
    pub count: usize,
}

/// GET /api/v1/presets
///
/// Lists all registered V1 presets.
#[instrument(skip_all, fields(count))]
pub async fn list_presets(
    Extension(reg): Extension<Arc<dyn PresetRegistry>>,
) -> Result<impl IntoResponse, AppError> {
    // 1. List all preset IDs
    let ids: Vec<PresetId> = reg.list();
    let count = ids.len();

    // 2. Build response DTOs
    let presets: Vec<PresetResponse> = ids
        .iter()
        .filter_map(|id| reg.get(id))
        .map(|p| PresetResponse::from(&p))
        .collect();

    tracing::info!(count = count, "Presets listed");

    // 3. Return response with cache headers
    let response = PresetListResponse { presets, count };
    Ok((
        StatusCode::OK,
        [("cache-control", "private, max-age=300")],
        Json(response),
    ))
}

/// Create router for /api/v1/presets
pub fn routes() -> Router {
    Router::new().route("/", get(list_presets))
}
