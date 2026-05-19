//! Infrastructure error types
//!
//! These errors represent failures in the infrastructure layer,
//! separate from domain errors.

use thiserror::Error;

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
