//! Semantic parser module
//!
//! Provides incremental parsing of inline semantic syntax in block content:
//! - `[[Page]]` page references
//! - `((Block))` block references
//! - `#tag` tags
//! - `property:: value` inline properties
//!
//! Also provides autocomplete abstractions (types, triggers, providers)
//! for building autocomplete UI on top of parsed content.

pub mod autocomplete;
pub mod autocomplete_pipeline;
pub mod inline;
pub mod providers;
pub mod semantic_adapter;

pub use autocomplete::{
    detect_trigger, AutocompleteCategory, AutocompleteItem, AutocompleteProvider,
    AutocompleteResult, AutocompleteService, AutocompleteTrigger,
};
pub use inline::{InlineParser, NormalizedContent, ParsedContent, Range, Segment, SemanticData};
pub use semantic_adapter::compute_semantic_data;
