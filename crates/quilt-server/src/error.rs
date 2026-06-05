//! Application error types for the HTTP server
//!
//! Provides centralized error handling with proper HTTP status codes
//! and consistent error response format.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
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

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg.clone()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg.clone()),
            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                msg.clone(),
            ),
        };

        let body = Json(json!({
            "error": message,
            "code": code
        }));

        (status, body).into_response()
    }
}
