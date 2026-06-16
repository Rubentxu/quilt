//! Infrastructure error types
//!
//! These errors represent failures in the infrastructure layer,
//! separate from domain errors.

use thiserror::Error;
use quilt_domain::errors::DomainError;

/// InfrastructureError represents errors that occur in the infrastructure layer.
///
/// These include database errors, connection failures, and serialization issues.
#[derive(Debug, Error)]
pub enum InfrastructureError {
    #[error("Database error in '{operation}': {message}")]
    Database {
        /// The operation that failed
        operation: &'static str,
        /// The error message
        message: String,
    },

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Connection error: {0}")]
    Connection(String),
}

/// Map a sqlx error to a DomainError
pub fn map_sqlx_error(operation: &'static str, e: sqlx::Error) -> DomainError {
    DomainError::Storage(format!("Database error in '{}': {}", operation, e))
}

/// Map a storage error to an InfrastructureError
pub fn map_storage_error(message: String) -> InfrastructureError {
    InfrastructureError::Database {
        operation: "storage",
        message,
    }
}
