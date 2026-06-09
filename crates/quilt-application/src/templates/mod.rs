//! Template extension types.
//!
//! G6: SchemaPack — template metadata as JSON-in-string-property.
//! F15: ReapplyTemplate — re-apply template to existing blocks with conflict detection.
//! Q030: ApplyTemplateWithContract — apply template *with* its
//!      declared `TemplateContract` (required / locked / version
//!      checks) extracted from `reapply.rs`.

pub mod contract;
pub mod reapply;
pub mod schema_pack;
