//! RefRepository — abstraction for reference persistence
//!
//! This trait defines the contract for persisting and querying references.
//! Implementations (like `SqliteRefRepository`) live in the infrastructure layer.
//!
//! The domain layer depends on this trait, not on any concrete implementation —
//! following the Dependency Inversion Principle.

use crate::errors::DomainError;
use crate::references::RefType;
use crate::value_objects::Uuid;
use async_trait::async_trait;

/// A single reference row as stored in and retrieved from the repository.
///
/// Unlike the domain `Ref` value object, this includes the source UUID
/// since it comes from a database row which has all three columns
/// (source_id, target_id, ref_type).
#[derive(Debug, Clone)]
pub struct RefRow {
    /// The UUID of the source entity (the entity containing the reference)
    pub source_id: Uuid,
    /// The UUID of the target entity (the entity being referenced)
    pub target_id: Uuid,
    /// The type of reference
    pub ref_type: RefType,
}

/// Repository trait for reference persistence.
///
/// All methods are async and return `DomainError` on failure.
/// Implementations handle the underlying storage (SQLite, in-memory, etc.).
#[async_trait]
pub trait RefRepository: Send + Sync {
    /// Get all references from a specific source entity.
    ///
    /// Returns a list of `(target_id, ref_type)` pairs.
    async fn get_forward_refs(&self, source_id: Uuid) -> Result<Vec<(Uuid, RefType)>, DomainError>;

    /// Get all references that point to a specific target entity (backlinks).
    ///
    /// Returns a list of `(source_id, ref_type)` pairs.
    async fn get_backlinks(&self, target_id: Uuid) -> Result<Vec<(Uuid, RefType)>, DomainError>;

    /// Synchronize references for a source entity.
    ///
    /// This replaces ALL existing references from `source_id` with the
    /// provided `refs`. The implementation should:
    /// 1. Delete existing refs for `source_id`
    /// 2. Insert the new set of refs
    /// 3. Do both in a single transaction
    async fn sync_refs(&self, source_id: Uuid, refs: &[(Uuid, RefType)])
        -> Result<(), DomainError>;

    /// Rebuild the entire reference index from persistent storage.
    ///
    /// Returns all reference rows. The caller (typically `RefService`)
    /// uses this to populate its in-memory `RefIndex`.
    async fn rebuild_index(&self) -> Result<Vec<RefRow>, DomainError>;
}
