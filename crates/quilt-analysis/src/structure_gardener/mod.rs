//! Structure Gardener
//!
//! Tracks belief evolution in journal pages over time, detects contradictions,
//! and suggests areas for deeper exploration.

pub mod engine;
pub mod types;

pub use engine::{StructureGardener, StructureGardenerError};
pub use types::*;
