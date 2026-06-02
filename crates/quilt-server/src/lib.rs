//! Quilt Server Library
//!
//! Provides the HTTP server components for the Quilt knowledge graph.

pub mod error;
pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod state;

// Re-export commonly used types
pub use error::AppError;
pub use state::AppState;
