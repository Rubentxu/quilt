//! Structure Mapper
//!
//! Maps the argument structure of a page by analyzing block content and
//! reference relationships to build structure graphs with typed edges.

pub mod engine;
pub mod types;

pub use engine::{StructureMapper, StructureMapperError};
pub use types::*;
