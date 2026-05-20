//! Health check endpoint

use axum::Json;

/// Health check response
#[derive(serde::Serialize)]
pub struct HealthResponse {
    pub status: String,
}

/// GET /health
///
/// Returns the health status of the server.
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}
