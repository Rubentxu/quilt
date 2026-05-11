//! Application layer errors

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0} with id {1}")]
    NotFound(&'static str, quilt_domain::value_objects::Uuid),

    #[error("Domain error: {0}")]
    Domain(#[from] quilt_domain::DomainError),

    #[error("Infrastructure error: {0}")]
    Infrastructure(String),
}

pub type ApplicationResult<T> = Result<T, ApplicationError>;
