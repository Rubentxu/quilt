//! RefService — application service for managing bidirectional references
//!
//! This service orchestrates the reference lifecycle:
//! 1. When a block is saved, parse its content for `((uuid))` and `[[name]]` refs
//! 2. Diff against existing refs and sync via `RefRepository`
//! 3. Update the in-memory `RefIndex` for O(1) backlink queries
//!
//! The service holds both an in-memory index and a repository reference,
//! keeping them in sync as changes occur.

use std::sync::Arc;

use quilt_domain::errors::DomainError;
use quilt_domain::references::{RefIndex, RefType};
use quilt_domain::repositories::RefRepository;
use quilt_domain::value_objects::Uuid;

use tracing::instrument;

/// Content reference parser result.
///
/// Separates references into UUID-resolved and name-based categories.
/// Block refs `((uuid))` are resolved directly. Page refs `[[name]]`
/// require name-to-UUID resolution (handled by the caller via resolver).
#[derive(Debug, Clone, Default)]
pub struct ParsedContentRefs {
    /// References that have been resolved to UUIDs
    pub resolved: Vec<(Uuid, RefType)>,
    /// Page names extracted from `[[page name]]` that need resolution
    pub page_names: Vec<String>,
}

/// Parse block content for references.
///
/// Extracts:
/// - `((block_uuid))` → `BlockRef` (resolved directly)
/// - `[[page name]]` → page name string (requires resolution)
///
/// Returns both resolved and unresolved references.
///
/// # Examples
///
/// ```
/// use quilt_application::services::ref_service::parse_refs_from_content;
/// use quilt_domain::references::RefType;
///
/// let result = parse_refs_from_content("Hello ((550e8400-e29b-41d4-a716-446655440000))");
/// assert_eq!(result.resolved.len(), 1);
/// assert_eq!(result.resolved[0].1, RefType::BlockRef);
/// ```
pub fn parse_refs_from_content(content: &str) -> ParsedContentRefs {
    let mut resolved = Vec::new();
    let mut page_names = Vec::new();

    let chars: Vec<char> = content.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Check for block reference opening ((
        if i + 1 < len && chars[i] == '(' && chars[i + 1] == '(' {
            i += 2;
            let start = i;

            // Find closing ))
            while i < len {
                if i + 1 < len && chars[i] == ')' && chars[i + 1] == ')' {
                    let inner: String = chars[start..i].iter().collect();
                    let trimmed = inner.trim();

                    // Try to parse as UUID
                    if let Ok(uuid) = uuid::Uuid::parse_str(trimmed) {
                        resolved.push((Uuid::from(uuid), RefType::BlockRef));
                    }
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Check for page reference opening [[
        if i + 1 < len && chars[i] == '[' && chars[i + 1] == '[' {
            i += 2;
            let start = i;

            // Find closing ]]
            while i < len {
                if i + 1 < len && chars[i] == ']' && chars[i + 1] == ']' {
                    let inner: String = chars[start..i].iter().collect();
                    let trimmed = inner.trim();
                    if !trimmed.is_empty() {
                        // G1: split on the FIRST `|` to drop the alias
                        // portion. The page lookup uses the part before
                        // `|`; the alias is display-only.
                        let name_part = trimmed
                            .find('|')
                            .map(|idx| trimmed[..idx].trim())
                            .unwrap_or(trimmed);
                        if !name_part.is_empty() {
                            // Try UUID first (possible [[uuid]] style), then page name
                            if let Ok(uuid) = uuid::Uuid::parse_str(name_part) {
                                resolved.push((Uuid::from(uuid), RefType::PageRef));
                            } else {
                                page_names.push(name_part.to_string());
                            }
                        }
                    }
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }

        i += 1;
    }

    ParsedContentRefs {
        resolved,
        page_names,
    }
}

/// Application service for managing references.
///
/// Holds an in-memory `RefIndex` for O(1) queries and delegates persistence
/// to a `RefRepository` implementation.
///
/// # Examples
///
/// ```ignore
/// use quilt_application::services::ref_service::RefService;
/// use quilt_domain::repositories::RefRepository;
/// use std::sync::Arc;
///
/// # async fn example(repo: Arc<dyn RefRepository>) {
/// let service = RefService::new(repo);
/// let backlinks = service.get_backlinks(target_id).await.unwrap();
/// # }
/// ```
pub struct RefService {
    /// In-memory bidirectional reference index
    index: RefIndex,
    /// Persistent storage for references
    repo: Arc<dyn RefRepository>,
}

impl RefService {
    /// Creates a new `RefService` with the given repository.
    ///
    /// The index starts empty. Call `rebuild_from_repo()` to populate it
    /// from persistent storage, or lazy-build it as blocks are saved.
    pub fn new(repo: Arc<dyn RefRepository>) -> Self {
        Self {
            index: RefIndex::new(),
            repo,
        }
    }

    /// Called when a block is saved (created or updated).
    ///
    /// This method:
    /// 1. Parses the content for `((uuid))` and `[[name]]` refs
    /// 2. Resolves page names to UUIDs via the optional resolver
    /// 3. Syncs the resolved refs to the repository
    /// 4. Updates the in-memory index
    ///
    /// # Arguments
    ///
    /// * `block_id` — UUID of the block being saved
    /// * `content` — raw content text to parse for references
    /// * `page_resolver` — optional callback to resolve page names to UUIDs;
    ///   returns `None` if the page name cannot be resolved
    #[instrument(skip(self, content, page_resolver), fields(block_id = %block_id))]
    pub async fn on_block_saved(
        &mut self,
        block_id: Uuid,
        content: &str,
        page_resolver: Option<&(dyn Fn(&str) -> Option<Uuid> + Sync)>,
    ) -> Result<(), DomainError> {
        // Parse content for references
        let parsed = parse_refs_from_content(content);

        // Collect all resolved refs: direct UUID refs + page name resolutions
        let mut all_refs: Vec<(Uuid, RefType)> = parsed.resolved;

        // Resolve page names if a resolver is provided
        if let Some(resolver) = page_resolver {
            for name in &parsed.page_names {
                if let Some(uuid) = resolver(name) {
                    all_refs.push((uuid, RefType::PageRef));
                }
            }
        }

        // Sync to repository (replaces all refs for this source)
        self.repo.sync_refs(block_id, &all_refs).await?;

        // Update in-memory index: remove old, add new
        self.index.remove_all_from_source(block_id);
        for (target, ref_type) in &all_refs {
            self.index.add_ref(block_id, *target, *ref_type);
        }

        Ok(())
    }

    /// Get all backlinks for a given target entity.
    ///
    /// Returns a list of `(source_id, ref_type)` pairs representing
    /// all entities that reference the given target.
    ///
    /// This is an O(1) index lookup — no database query.
    pub fn get_backlinks(&self, target_id: Uuid) -> Vec<(Uuid, RefType)> {
        self.index.get_backlinks(target_id)
    }

    /// Get all forward references from a given source entity.
    ///
    /// Returns a list of `(target_id, ref_type)` pairs representing
    /// all entities that the given source references.
    pub fn get_forward_refs(&self, source_id: Uuid) -> Vec<(Uuid, RefType)> {
        self.index.get_forward_refs(source_id)
    }

    /// Rebuild the in-memory index from the repository.
    ///
    /// This loads all reference rows from persistent storage and
    /// rebuilds the bidirectional index. Call this at startup.
    #[instrument(skip(self))]
    pub async fn rebuild_from_repo(&mut self) -> Result<(), DomainError> {
        let rows = self.repo.rebuild_index().await?;

        let mut new_index = RefIndex::new();
        for row in &rows {
            new_index.add_ref(row.source_id, row.target_id, row.ref_type);
        }

        self.index = new_index;
        Ok(())
    }

    /// Get unlinked references for a page.
    ///
    /// Finds blocks whose content text mentions `page_name` but do not have
    /// an explicit `[[page_name]]` reference. Delegates to the repository
    /// which uses FTS5 or LIKE for text search.
    pub async fn get_page_unlinked_references(
        &self,
        page_name: &str,
        page_id: Uuid,
    ) -> Result<Vec<(Uuid, Uuid, String)>, DomainError> {
        self.repo
            .get_unlinked_references(page_name, page_id)
            .await
    }

    /// Create an explicit block-to-block link.
    ///
    /// This inserts a `block_ref` into the `refs` table and updates the
    /// in-memory index. Unlike `on_block_saved`, this does not parse content
    /// or replace existing refs — it adds one link atomically.
    ///
    /// If the link already exists, this is a no-op (idempotent).
    #[instrument(skip(self))]
    pub async fn create_link(&mut self, source_id: Uuid, target_id: Uuid) -> Result<(), DomainError> {
        self.repo
            .insert_ref(source_id, target_id, RefType::BlockRef)
            .await?;
        self.index
            .add_ref(source_id, target_id, RefType::BlockRef);
        Ok(())
    }

    /// Get a reference to the in-memory index for inspection.
    pub fn index(&self) -> &RefIndex {
        &self.index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quilt_domain::repositories::RefRow;
    use std::sync::Mutex;

    /// A mock RefRepository for testing.
    struct MockRefRepository {
        refs: Mutex<Vec<RefRow>>,
    }

    impl MockRefRepository {
        fn new() -> Self {
            Self {
                refs: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl RefRepository for MockRefRepository {
        async fn get_forward_refs(
            &self,
            source_id: Uuid,
        ) -> Result<Vec<(Uuid, RefType)>, DomainError> {
            let refs = self.refs.lock().unwrap();
            Ok(refs
                .iter()
                .filter(|r| r.source_id == source_id)
                .map(|r| (r.target_id, r.ref_type))
                .collect())
        }

        async fn get_backlinks(
            &self,
            target_id: Uuid,
        ) -> Result<Vec<(Uuid, RefType)>, DomainError> {
            let refs = self.refs.lock().unwrap();
            Ok(refs
                .iter()
                .filter(|r| r.target_id == target_id)
                .map(|r| (r.source_id, r.ref_type))
                .collect())
        }

        async fn sync_refs(
            &self,
            source_id: Uuid,
            refs: &[(Uuid, RefType)],
        ) -> Result<(), DomainError> {
            let mut stored = self.refs.lock().unwrap();
            stored.retain(|r| r.source_id != source_id);
            for (target, ref_type) in refs {
                stored.push(RefRow {
                    source_id,
                    target_id: *target,
                    ref_type: *ref_type,
                });
            }
            Ok(())
        }

        async fn rebuild_index(&self) -> Result<Vec<RefRow>, DomainError> {
            let refs = self.refs.lock().unwrap();
            Ok(refs.clone())
        }

        async fn insert_ref(
            &self,
            source_id: Uuid,
            target_id: Uuid,
            ref_type: RefType,
        ) -> Result<(), DomainError> {
            let mut stored = self.refs.lock().unwrap();
            // Avoid duplicates (same behavior as INSERT OR IGNORE)
            if !stored.iter().any(|r| r.source_id == source_id && r.target_id == target_id && r.ref_type == ref_type) {
                stored.push(RefRow {
                    source_id,
                    target_id,
                    ref_type,
                });
            }
            Ok(())
        }

        async fn get_unlinked_references(
            &self,
            _page_name: &str,
            _page_id: Uuid,
        ) -> Result<Vec<(Uuid, Uuid, String)>, DomainError> {
            // Mock returns empty — unlinked refs require full-text search
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn test_add_ref_and_get_backlinks() {
        let repo = Arc::new(MockRefRepository::new());
        let mut service = RefService::new(repo);
        let block_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();

        let content = format!("Check this out: (({}))", target_id);
        service
            .on_block_saved(block_id, &content, None)
            .await
            .unwrap();

        let backlinks = service.get_backlinks(target_id);
        assert_eq!(backlinks.len(), 1);
        assert_eq!(backlinks[0].0, block_id);
        assert_eq!(backlinks[0].1, RefType::BlockRef);
    }

    #[tokio::test]
    async fn test_get_forward_refs() {
        let repo = Arc::new(MockRefRepository::new());
        let mut service = RefService::new(repo);
        let block_id = Uuid::new_v4();
        let target1 = Uuid::new_v4();
        let target2 = Uuid::new_v4();

        let content = format!("Refs: (({})) and (({}))", target1, target2);
        service
            .on_block_saved(block_id, &content, None)
            .await
            .unwrap();

        let forward = service.get_forward_refs(block_id);
        assert_eq!(forward.len(), 2);

        let targets: Vec<Uuid> = forward.iter().map(|(t, _)| *t).collect();
        assert!(targets.contains(&target1));
        assert!(targets.contains(&target2));
    }

    #[tokio::test]
    async fn test_rebuild_from_repo() {
        let repo = Arc::new(MockRefRepository::new());
        let block_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();

        // Directly insert into repo
        repo.sync_refs(block_id, &[(target_id, RefType::PageRef)])
            .await
            .unwrap();

        // Create a fresh service and rebuild from repo
        let mut service = RefService::new(repo);
        service.rebuild_from_repo().await.unwrap();

        let backlinks = service.get_backlinks(target_id);
        assert_eq!(backlinks.len(), 1);
        assert_eq!(backlinks[0].0, block_id);
    }

    #[tokio::test]
    async fn test_update_existing_block_refs() {
        let repo = Arc::new(MockRefRepository::new());
        let mut service = RefService::new(repo);
        let block_id = Uuid::new_v4();
        let target1 = Uuid::new_v4();
        let target2 = Uuid::new_v4();

        // First save: reference target1
        let content = format!("Ref: (({}))", target1);
        service
            .on_block_saved(block_id, &content, None)
            .await
            .unwrap();
        assert_eq!(service.get_forward_refs(block_id).len(), 1);
        assert_eq!(service.get_backlinks(target1).len(), 1);

        // Second save: reference target2 instead (no longer ref target1)
        let content = format!("Ref: (({}))", target2);
        service
            .on_block_saved(block_id, &content, None)
            .await
            .unwrap();

        let forward = service.get_forward_refs(block_id);
        assert_eq!(forward.len(), 1);
        assert_eq!(forward[0].0, target2);

        // target1 should have no backlinks now
        assert_eq!(service.get_backlinks(target1).len(), 0);
        assert_eq!(service.get_backlinks(target2).len(), 1);
    }

    #[tokio::test]
    async fn test_empty_content_clears_refs() {
        let repo = Arc::new(MockRefRepository::new());
        let mut service = RefService::new(repo);
        let block_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();

        // First save with ref
        let content = format!("Ref: (({}))", target_id);
        service
            .on_block_saved(block_id, &content, None)
            .await
            .unwrap();
        assert_eq!(service.get_forward_refs(block_id).len(), 1);

        // Save with empty content — should clear refs
        service.on_block_saved(block_id, "", None).await.unwrap();

        assert!(service.get_forward_refs(block_id).is_empty());
        assert_eq!(service.get_backlinks(target_id).len(), 0);
    }

    #[tokio::test]
    async fn test_page_ref_resolution() {
        let repo = Arc::new(MockRefRepository::new());
        let mut service = RefService::new(repo);
        let block_id = Uuid::new_v4();
        let page_id = Uuid::new_v4();

        let resolver = |name: &str| -> Option<Uuid> {
            match name {
                "My Page" => Some(page_id),
                _ => None,
            }
        };

        let content = "Check [[My Page]]".to_string();
        service
            .on_block_saved(block_id, &content, Some(&resolver))
            .await
            .unwrap();

        let backlinks = service.get_backlinks(page_id);
        assert_eq!(backlinks.len(), 1);
        assert_eq!(backlinks[0].0, block_id);
        assert_eq!(backlinks[0].1, RefType::PageRef);
    }

    // ── G1: alias split in `[[Page|alias]]` ───────────────────────────
    //
    // The ref service must use the part BEFORE `|` for page resolution.
    // The alias is display-only and must not influence the lookup.

    #[tokio::test]
    async fn test_page_ref_with_alias_resolves_to_page_name() {
        let repo = Arc::new(MockRefRepository::new());
        let mut service = RefService::new(repo);
        let block_id = Uuid::new_v4();
        let page_id = Uuid::new_v4();

        // The alias text must NOT match the resolver; only the page name
        // part is passed to it. If the alias leaked into the lookup, the
        // resolver would return None and the backlink would not be created.
        let resolver = |name: &str| -> Option<Uuid> {
            match name {
                "My Page" => Some(page_id),
                _ => None,
            }
        };

        let content = "Check [[My Page|display alias]]".to_string();
        service
            .on_block_saved(block_id, &content, Some(&resolver))
            .await
            .unwrap();

        let backlinks = service.get_backlinks(page_id);
        assert_eq!(
            backlinks.len(),
            1,
            "[[Page|alias]] must resolve the page name and create a backlink"
        );
        assert_eq!(backlinks[0].0, block_id);
        assert_eq!(backlinks[0].1, RefType::PageRef);
    }

    #[tokio::test]
    async fn test_page_ref_with_empty_alias_resolves() {
        // `[[Page|]]` is treated as no alias; resolution still works.
        let repo = Arc::new(MockRefRepository::new());
        let mut service = RefService::new(repo);
        let block_id = Uuid::new_v4();
        let page_id = Uuid::new_v4();

        let resolver = |name: &str| -> Option<Uuid> {
            (name == "Page").then_some(page_id)
        };

        let content = "Check [[Page|]]".to_string();
        service
            .on_block_saved(block_id, &content, Some(&resolver))
            .await
            .unwrap();

        assert_eq!(service.get_backlinks(page_id).len(), 1);
    }

    #[tokio::test]
    async fn test_e2e_write_then_read_backlinks() {
        // Full end-to-end test proving the write→read flow without SQLite.
        // 1. Create RefService with in-memory mock repo
        // 2. Call on_block_saved with [[TestPage]]
        // 3. Call get_backlinks(test_page_id)
        // 4. Verify backlink exists with correct source and type
        let repo = Arc::new(MockRefRepository::new());
        let mut service = RefService::new(repo);

        let block_id = Uuid::new_v4();
        let test_page_id = Uuid::new_v4();
        let other_page_id = Uuid::new_v4();

        // Resolver that knows about our pages
        let resolver = |name: &str| -> Option<Uuid> {
            match name {
                "TestPage" => Some(test_page_id),
                "OtherPage" => Some(other_page_id),
                _ => None,
            }
        };

        // Write: save a block that references "TestPage"
        let content = "This block links to [[TestPage]] and also ((some-other-uuid))".to_string();
        service
            .on_block_saved(block_id, &content, Some(&resolver))
            .await
            .unwrap();

        // Verify forward refs from source
        let forward = service.get_forward_refs(block_id);
        assert_eq!(
            forward.len(),
            1,
            "Only the resolved page ref should be in forward refs"
        );
        assert_eq!(forward[0], (test_page_id, RefType::PageRef));

        // Read: get backlinks to TestPage
        let backlinks = service.get_backlinks(test_page_id);
        assert_eq!(
            backlinks.len(),
            1,
            "TestPage should have exactly one backlink"
        );
        assert_eq!(
            backlinks[0].0, block_id,
            "Backlink source should be our block"
        );
        assert_eq!(
            backlinks[0].1,
            RefType::PageRef,
            "Backlink type should be PageRef"
        );

        // Verify OtherPage has no backlinks
        assert!(
            service.get_backlinks(other_page_id).is_empty(),
            "OtherPage should have no backlinks"
        );
        assert!(
            service.get_backlinks(Uuid::new_v4()).is_empty(),
            "Unknown page should have no backlinks"
        );
    }
}
