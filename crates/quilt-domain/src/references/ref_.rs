//! Ref — a value object representing a single reference between entities
//!
//! A `Ref` pairs a target UUID with a `RefType`, forming the atomic unit
//! of the reference model. References are stored in `RefIndex` for O(1)
//! forward and reverse lookups, and persisted to the `refs` table via
//! the `RefRepository` trait.
//!
//! # Examples
//!
//! ```ignore
//! use quilt_domain::references::{Ref, RefType};
//! use quilt_domain::value_objects::Uuid;
//!
//! let target = Uuid::new_v4();
//! let reference = Ref::new(target, RefType::BlockRef);
//! assert_eq!(reference.target, target);
//! assert_eq!(reference.ref_type, RefType::BlockRef);
//! ```

use crate::value_objects::Uuid;
use serde::{Deserialize, Serialize};

/// A reference from one entity to another.
///
/// `Ref` is an immutable value object. Two refs are equal if they have
/// the same target and ref_type — the source is tracked separately by
/// the `RefIndex`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ref {
    /// The UUID of the target entity (page or block)
    pub target: Uuid,
    /// The type of this reference
    pub ref_type: super::RefType,
}

impl Ref {
    /// Creates a new `Ref` with the given target and type.
    pub fn new(target: Uuid, ref_type: super::RefType) -> Self {
        Self { target, ref_type }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::references::RefType;

    #[test]
    fn test_ref_creation() {
        let target = Uuid::new_v4();
        let r = Ref::new(target, RefType::BlockRef);
        assert_eq!(r.target, target);
        assert_eq!(r.ref_type, RefType::BlockRef);
    }

    #[test]
    fn test_ref_equality() {
        let target = Uuid::new_v4();
        let a = Ref::new(target, RefType::PageRef);
        let b = Ref::new(target, RefType::PageRef);
        assert_eq!(a, b);
    }

    #[test]
    fn test_ref_inequality() {
        let target_a = Uuid::new_v4();
        let target_b = Uuid::new_v4();
        let a = Ref::new(target_a, RefType::PageRef);
        let b = Ref::new(target_b, RefType::PageRef);
        assert_ne!(a, b);
    }

    #[test]
    fn test_ref_type_inequality() {
        let target = Uuid::new_v4();
        let a = Ref::new(target, RefType::PageRef);
        let b = Ref::new(target, RefType::BlockRef);
        assert_ne!(a, b);
    }

    #[test]
    fn test_ref_hash() {
        use std::collections::HashSet;
        let target = Uuid::new_v4();
        let r = Ref::new(target, RefType::Tag);
        let mut set = HashSet::new();
        set.insert(r);
        assert!(set.contains(&r));
    }
}
