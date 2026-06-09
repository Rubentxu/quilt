//! BlockRepository trait - abstraction for block data access

use crate::entities::Block;
use crate::errors::DomainError;
use crate::value_objects::Uuid;
use async_trait::async_trait;

/// BlockRepository is the abstraction for block data access.
///
/// Implementations (like SqliteBlockRepository) implement this trait,
/// allowing the domain to be independent of the storage mechanism.
#[async_trait]
pub trait BlockRepository: Send + Sync {
    /// Get a block by its ID
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Block>, DomainError>;

    /// Get all blocks belonging to a page
    async fn get_by_page(&self, page_id: Uuid) -> Result<Vec<Block>, DomainError>;

    /// Get direct children of a block
    async fn get_children(&self, parent_id: Uuid) -> Result<Vec<Block>, DomainError>;

    /// Get a block with its references
    async fn get_with_refs(&self, id: Uuid) -> Result<(Block, Vec<Uuid>), DomainError>;

    /// Insert a new block
    async fn insert(&self, block: &Block) -> Result<(), DomainError>;

    /// Update an existing block
    async fn update(&self, block: &Block) -> Result<(), DomainError>;

    /// Delete a block by ID
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;

    /// Move a block to a new parent with new order
    async fn move_block(
        &self,
        id: Uuid,
        new_parent: Option<Uuid>,
        new_order: f64,
    ) -> Result<(), DomainError>;

    /// Get all blocks that reference a given block (backlinks)
    async fn get_backlinks(&self, block_id: Uuid) -> Result<Vec<Block>, DomainError>;

    /// Search blocks by content (full-text or fuzzy)
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Block>, DomainError>;

    /// Get all blocks updated since a given timestamp
    async fn get_updated_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Block>, DomainError>;

    /// Get the count of blocks on a page
    async fn count_by_page(&self, page_id: Uuid) -> Result<usize, DomainError>;

    /// Get the total count of all blocks
    async fn count_all(&self) -> Result<usize, DomainError>;

    /// Execute a DSL query and return blocks.
    /// Used by SearchUseCases to execute parsed DSL queries.
    async fn query_dsl(&self, sql: &str, params: &[String]) -> Result<Vec<Block>, DomainError>;

    /// List blocks whose `properties` JSON map contains `key` mapped to
    /// the given string `value`.
    ///
    /// This is the primary lookup for the `created_by` convention
    /// (`user::name`, `agent::claude`, ...). Returns at most `limit`
    /// blocks, ordered by `created_at DESC` so the most recent
    /// creations show up first. `limit == 0` means "no limit".
    ///
    /// Match semantics: string equality on the JSON-encoded property
    /// value. Booleans/numbers will not match a string query and
    /// vice-versa — callers should pass the same shape they wrote.
    async fn list_by_property(
        &self,
        key: &str,
        value: &str,
        limit: usize,
    ) -> Result<Vec<Block>, DomainError>;

    /// List distinct top-level property **keys** that appear in any
    /// block's `properties` map, ordered lexicographically ascending.
    ///
    /// This is the backend for `GET /api/v1/properties/keys`. It is the
    /// first cursor-paginated read in Quilt, so the convention set
    /// here is project-wide:
    ///
    /// * `cursor == None` → return the smallest `limit` keys.
    /// * `cursor == Some(s)` → return only keys that are **strictly
    ///   greater** than `s` (lexicographic, byte-wise UTF-8).
    /// * `limit` is the upper bound on how many keys to return. The
    ///   handler is expected to validate `limit ∈ 1..=100` before
    ///   calling this method — implementations trust the input.
    /// * The result is `Vec<String>`, sorted ASC. Callers decide
    ///   whether the page was the last one by comparing
    ///   `keys.len() < limit`.
    ///
    /// Implementations must deduplicate — the same key can appear in
    /// many blocks.
    async fn list_distinct_keys(
        &self,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<Vec<String>, DomainError>;

    /// List blocks whose `properties` map contains the given `key`,
    /// regardless of the value mapped to that key.
    ///
    /// Powers the Graph Lens `property:<key>` focus. Returns at most
    /// `limit` blocks, ordered by `created_at DESC`; `limit == 0`
    /// means "no limit".
    ///
    /// This is distinct from [`list_by_property`](Self::list_by_property)
    /// which requires the value to match exactly. For the lens use
    /// case we only care that the key exists.
    async fn list_by_property_key(&self, key: &str, limit: u32) -> Result<Vec<Block>, DomainError>;

    /// List distinct string values of a property that satisfy an
    /// optional `LIKE` prefix.
    ///
    /// Used by `GET /api/v1/blocks/authors` to enumerate "which agent
    /// identifiers have ever authored a block" without hardcoding
    /// the set in the UI. The default empty prefix matches all
    /// non-NULL string values; pass `Some("agent::")` to scope the
    /// result to AI authors.
    ///
    /// Semantics:
    /// * Values are deduplicated and sorted ASC.
    /// * NULL and non-text values are excluded (defense in depth on
    ///   top of the SQL `typeof` filter).
    /// * Empty strings are also excluded.
    /// * No limit is applied — the set of distinct authors is
    ///   expected to be tiny (single digits). If a deployment grows
    ///   to thousands of distinct authors, this method should grow
    ///   a `cursor` + `limit` pair, mirroring `list_distinct_keys`.
    async fn list_distinct_authors(
        &self,
        prefix: Option<&str>,
    ) -> Result<Vec<String>, DomainError>;
}

/// BlockRepositoryExt provides additional convenience methods
#[async_trait]
pub trait BlockRepositoryExt: BlockRepository {
    /// Check if a block exists
    async fn exists(&self, id: Uuid) -> Result<bool, DomainError> {
        Ok(self.get_by_id(id).await?.is_some())
    }

    /// Get a block or fail with an error
    async fn get_or_fail(&self, id: Uuid) -> Result<Block, DomainError> {
        self.get_by_id(id)
            .await?
            .ok_or(DomainError::BlockNotFound(id))
    }
}

impl<T: BlockRepository + ?Sized> BlockRepositoryExt for T {}
