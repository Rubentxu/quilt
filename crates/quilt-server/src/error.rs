//! Application error types for the HTTP server
//!
//! Provides centralized error handling with proper HTTP status codes
//! and consistent error response format.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use quilt_platform::graph_validation::GraphValidationError;
use serde_json::json;

/// Application error types
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    /// 409 Conflict — the request could not be completed because of a conflict
    /// with the current state of the target resource (e.g. trying to delete
    /// a block that still has children).
    #[error("Conflict: {0}")]
    Conflict(String),

    /// 401 Unauthorized — the request could not be authenticated or
    /// the auth context could not be derived (e.g. missing or
    /// malformed `Authorization` header on a handler that needs the
    /// user identity). The auth middleware normally catches this
    /// earlier, so reaching this variant from a handler is a
    /// programming error.
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// 503 Service Unavailable — no graph is currently open
    /// (GS-9: migration endpoints require an active graph).
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    /// 422 Unprocessable Entity — the request was well-formed but
    /// the Graph Space layout is invalid (ADR-0030 §6).
    /// Per the design this is the dedicated code path for
    /// `GraphValidationError`; the response body carries
    /// `{ code: "GRAPH_INVALID", validationError, path }` so the
    /// frontend can render a structured message keyed off the
    /// `validationError` field.
    #[error("Graph validation failed: {0}")]
    GraphInvalid(GraphValidationError),

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl From<GraphValidationError> for AppError {
    fn from(e: GraphValidationError) -> Self {
        AppError::GraphInvalid(e)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // 422 — typed validation error with structured body.
        if let AppError::GraphInvalid(err) = &self {
            let path_str = err
                .path()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            let body = Json(json!({
                "error": err.to_string(),
                "code": "GRAPH_INVALID",
                "validationError": err.code(),
                "path": path_str,
            }));
            return (StatusCode::UNPROCESSABLE_ENTITY, body).into_response();
        }

        let (status, code, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg.clone()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg.clone()),
            AppError::ServiceUnavailable(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "SERVICE_UNAVAILABLE",
                msg.clone(),
            ),
            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                msg.clone(),
            ),
            // Handled above via early return.
            AppError::GraphInvalid(_) => unreachable!(),
        };

        let body = Json(json!({
            "error": message,
            "code": code
        }));

        (status, body).into_response()
    }
}
