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
    /// User-edited context override (Q028: Editable Backlinks).
    ///
    /// `None` means "no override" — the Backlinks panel falls back to
    /// the source block's content snippet. `Some("")` and `Some("...")`
    /// are meaningful: the former clears any default-derived text, the
    /// latter is the user's custom snippet.
    pub custom_context: Option<String>,
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

    /// Insert a single reference.
    ///
    /// Unlike `sync_refs`, this does not replace existing refs — it adds
    /// one ref to the set. If the ref already exists (duplicate source_id,
    /// target_id, ref_type), it is silently ignored (idempotent).
    async fn insert_ref(
        &self,
        source_id: Uuid,
        target_id: Uuid,
        ref_type: RefType,
    ) -> Result<(), DomainError>;

    /// Get unlinked references for a page.
    ///
    /// Finds blocks whose raw content text mentions `page_name` (case-insensitive)
    /// but do NOT have an explicit `[[page_name]]` reference in the refs table.
    ///
    /// Returns a list of `(block_id, source_page_id, content_snippet)` tuples.
    /// Each snippet is up to ~100 characters of the block's content.
    async fn get_unlinked_references(
        &self,
        page_name: &str,
        page_id: Uuid,
    ) -> Result<Vec<(Uuid, Uuid, String)>, DomainError>;

    /// Set or clear the user-edited context override for a single reference
    /// (Q028: Editable Backlinks).
    ///
    /// - `Some("...")` stores the custom snippet (or an empty string to
    ///   override the default with an explicit blank).
    /// - `None` clears the override — the Backlinks panel falls back to
    ///   the source block's content snippet.
    ///
    /// Implementations should:
    /// 1. Verify the reference `(source_id, target_id, ref_type)` exists.
    /// 2. Return `Ok(false)` if the reference does not exist (caller
    ///    maps that to a 404).
    /// 3. Otherwise update the `custom_context` column and return `Ok(true)`.
    async fn set_custom_context(
        &self,
        source_id: Uuid,
        target_id: Uuid,
        ref_type: RefType,
        context: Option<&str>,
    ) -> Result<bool, DomainError>;

    /// Get the user-edited context override for a single reference.
    ///
    /// Returns `None` if the reference does not exist OR if no override
    /// has been set. Callers that need to distinguish "no reference" from
    /// "reference exists but no override" should combine this with
    /// `get_forward_refs` or `get_backlinks`.
    async fn get_custom_context(
        &self,
        source_id: Uuid,
        target_id: Uuid,
        ref_type: RefType,
    ) -> Result<Option<String>, DomainError>;

    /// Get the user-edited context overrides for every reference that
    /// points AT a given target page.
    ///
    /// Returns a list of `(source_id, ref_type, custom_context)` tuples
    /// for each ref that has a non-`None` `custom_context`. References
    /// without an override are omitted — the caller can detect "no
    /// override" by absence from the result.
    ///
    /// This is the bulk-read used by the page backlinks handler to
    /// avoid N+1 queries when enriching a list of backlinks with
    /// their overrides.
    async fn get_custom_contexts_for_target(
        &self,
        target_id: Uuid,
    ) -> Result<Vec<(Uuid, RefType, String)>, DomainError>;
}
