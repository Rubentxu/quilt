//! Infrastructure error types

pub mod infrastructure_error;

pub use infrastructure_error::{InfrastructureError, map_sqlx_error, map_storage_error};
