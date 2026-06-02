//! Inline semantic parser
//!
//! Parses [[Page]], ((Block)), #tag, property:: value, and markdown formatting
//! from block content. Pure algorithm — no framework coupling.

pub mod autocomplete;
pub mod autocomplete_pipeline;
pub mod inline;
pub mod providers;
pub mod semantic_adapter;
