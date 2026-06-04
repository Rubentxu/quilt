//! References — value objects for the bidirectional reference model
//!
//! This module implements the reference model from ADR-0008:
//! - [`Ref`]: a value object pairing a target UUID with a [`RefType`]
//! - [`RefType`]: an enum distinguishing page refs, block refs, tags, and aliases
//! - [`EdgeType`]: enum for typed edges in the Quilt graph (G4)
//! - [`TypedEdge`]: struct representing a typed edge with weight and timestamp
//! - [`RefIndex`]: an in-memory bidirectional index for O(1) backlinks
//!
//! The reference model is pure domain — no infrastructure dependencies.
//! Persistence is handled by the [`RefRepository`] trait in `repositories/`.

mod edge_type;
mod ref_;
mod ref_index;
mod ref_type;
#[cfg(test)]
mod edge_type_test;

pub use edge_type::{EdgeType, TypedEdge};
pub use ref_::Ref;
pub use ref_index::RefIndex;
pub use ref_type::RefType;
