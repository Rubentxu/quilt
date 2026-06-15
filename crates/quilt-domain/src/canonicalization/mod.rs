//! Canonicalization module — pure-function pipeline for input transformation.
//!
//! This module implements the non-destructive canonicalization pipeline from ADR-0025.
//! It transforms raw user input (Markdown, paste, slash-command, etc.) into
//! [`BlockContent`] + typed [`PropertyPatch`] entries without destroying information.

pub mod apply;
pub mod canonicalizer;
pub mod presets;
pub mod registry;
pub mod value_objects;

pub use canonicalizer::{Canonicalizer, PropertyDefinitionRegistry};
pub use presets::{PresetArg, PresetArgKind, PresetArgs, PresetId, PropertyPreset};
pub use registry::PresetRegistry;
pub use value_objects::{
    CanonicalInput, CanonicalizationResult, PatchOutcome, ProjectionConflict, PropertyPatch,
    PropertyPatchProvenance, SourceKind,
};
