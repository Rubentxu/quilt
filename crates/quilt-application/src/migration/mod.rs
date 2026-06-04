//! Migration module for importing external data into Quilt.
//!
//! Currently supports Markdown-flavored files (Logseq/Quilt format).

pub mod md_import_parser;
pub mod migration_engine;

pub use md_import_parser::{parse_md_import, Frontmatter, FrontmatterProperty, RawBlock};
pub use migration_engine::{ImportResult, MigrationEngine, infer_property_value};

/// Re-export MigrationError for use in error handling.
pub use md_import_parser::MigrationError;
