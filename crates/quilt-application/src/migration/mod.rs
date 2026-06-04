//! Migration module for importing external data into Quilt.
//!
//! Currently supports Logseq-flavored markdown files.

pub mod logseq_parser;
pub mod migration_engine;

pub use logseq_parser::{parse_logseq, Frontmatter, FrontmatterProperty, RawBlock};
pub use migration_engine::{ImportResult, MigrationEngine, infer_property_value};

/// Re-export MigrationError for use in error handling.
pub use logseq_parser::MigrationError;
