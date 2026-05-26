//! References — value objects for the bidirectional reference model
//!
//! This module implements the reference model from ADR-0008:
//! - [`Ref`]: a value object pairing a target UUID with a [`RefType`]
//! - [`RefType`]: an enum distinguishing page refs, block refs, tags, and aliases
//! - [`RefIndex`]: an in-memory bidirectional index for O(1) backlinks
//!
//! The reference model is pure domain — no infrastructure dependencies.
//! Persistence is handled by the [`RefRepository`] trait in `repositories/`.

mod ref_;
mod ref_index;
mod ref_type;

pub use ref_::Ref;
pub use ref_index::RefIndex;
pub use ref_type::RefType;
