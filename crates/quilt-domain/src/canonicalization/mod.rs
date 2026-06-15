//! Canonicalization module — pure-function pipeline for input transformation.
//!
//! This module implements the non-destructive canonicalization pipeline from ADR-0025.
//! It transforms raw user input (Markdown, paste, slash-command, etc.) into
//! [`BlockContent`] + typed [`PropertyPatch`] entries without destroying information.

pub mod value_objects;
pub mod canonicalizer;
pub mod apply;

pub use value_objects::{
    CanonicalInput, CanonicalizationResult, PropertyPatch, PropertyPatchProvenance,
    PatchOutcome, ProjectionConflict, SourceKind,
};
pub use canonicalizer::{Canonicalizer, PropertyDefinitionRegistry};
