//! HTTP Error Types
//!
//! Error types that map to HTTP responses.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

/// HTTP error types that map to specific HTTP status codes
#[derive(Debug, Error)]
pub enum HttpError {
    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Internal server error: {0}")]
    InternalError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),
}

/// Error response body format
#[derive(Serialize)]
pub struct HttpErrorBody {
    pub code: String,
    pub message: String,
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            HttpError::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            HttpError::ValidationError(_) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR"),
            HttpError::InternalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
            HttpError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR"),
            HttpError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
        };

        let body = HttpErrorBody {
            code: code.to_string(),
            message: self.to_string(),
        };

        (status, Json(body)).into_response()
    }
}

impl From<sqlx::Error> for HttpError {
    fn from(err: sqlx::Error) -> Self {
        HttpError::DatabaseError(err.to_string())
    }
}

impl From<quilt_domain::DomainError> for HttpError {
    fn from(err: quilt_domain::DomainError) -> Self {
        HttpError::InternalError(err.to_string())
    }
}
