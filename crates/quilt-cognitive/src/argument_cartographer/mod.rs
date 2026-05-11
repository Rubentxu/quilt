//! Argument Cartographer
//!
//! Maps the argument structure of a page by analyzing block content and
//! reference relationships to build argument graphs with typed edges.

pub mod engine;
pub mod types;

pub use engine::ArgumentCartographer;
pub use types::*;
