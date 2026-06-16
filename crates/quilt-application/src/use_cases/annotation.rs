//! Annotation use cases
//!
//! Implements [`AnnotationUseCases`] for CRUD + status transitions on
//! annotation entities. This is the application-layer seam that REST
//! and MCP adapters depend on (instead of binding to a specific
//! repository).

use crate::errors::ApplicationError;
use async_trait::async_trait;
use quilt_domain::entities::{
    Annotation, AnnotationCreate, AnnotationScope, AnnotationStatus, AuthorType,
};
use quilt_domain::repositories::{AnnotationFilters, AnnotationRepository};
use quilt_domain::value_objects::Uuid;
use std::sync::Arc;
use tracing::instrument;

/// Use cases for annotation lifecycle and querying.
///
/// Adapters (REST handlers, MCP tool handlers) depend on this trait
/// via `Arc<dyn AnnotationUseCases>` — never on a concrete repository.
/// All methods are `async` and `Send + Sync` for use in Axum
/// handlers and Tokio tasks.
#[async_trait]
pub trait AnnotationUseCases: Send + Sync {
    /// Create a new annotation from the wire DTO. Validates the wire
    /// fields (`scope`, `author_type`, non-empty content, scope ↔
    /// offset invariants) and returns the persisted entity.
    async fn create_from_dto(
        &self,
        block_id_str: &str,
        scope_str: &str,
        author_type_str: &str,
        author_name: &str,
        content: &str,
        parent_annotation_id_str: Option<&str>,
        highlight_start: Option<u32>,
        highlight_end: Option<u32>,
    ) -> Result<Annotation, ApplicationError>;

    /// Get a single annotation by id. Returns `None` when not found.
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Annotation>, ApplicationError>;

    /// List every annotation targeting a given block (ordered by
    /// `created_at` ASC by the repository).
    async fn list_by_block(&self, block_id: Uuid) -> Result<Vec<Annotation>, ApplicationError>;

    /// Apply an [`AnnotationFilters`] and return the matching rows.
    /// An empty filter returns every annotation in the repository
    /// (DESC by `created_at`).
    async fn list_by_filters(
        &self,
        filters: &AnnotationFilters,
    ) -> Result<Vec<Annotation>, ApplicationError>;

    /// Move an annotation to a new lifecycle status. The domain entity
    /// itself owns the transition logic (see [`Annotation::resolve`],
    /// [`Annotation::set_in_progress`], [`Annotation::dismiss`]). On
    /// any non-resolved transition, `resolved_at` and `resolved_by`
    /// are cleared.
    ///
    /// `resolved_by` is required when transitioning to `Resolved` so
    /// the timestamp is paired with a name. For other statuses the
    /// field is ignored.
    async fn update_status(
        &self,
        id: Uuid,
        status: AnnotationStatus,
        resolved_by: Option<String>,
    ) -> Result<Annotation, ApplicationError>;

    /// Convenience: resolve an annotation by id and the resolver's
    /// name. Equivalent to `update_status(id, Resolved, Some(by))`.
    async fn resolve(
        &self,
        id: Uuid,
        resolved_by: String,
    ) -> Result<Annotation, ApplicationError>;

    /// Delete an annotation. Returns `Ok(())` whether or not the row
    /// existed — matches the underlying repository contract.
    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError>;
}

/// Generic implementation parameterized over any repository impl.
pub struct AnnotationUseCasesImpl<R: AnnotationRepository> {
    repo: Arc<R>,
}

impl<R: AnnotationRepository> AnnotationUseCasesImpl<R> {
    /// Create a new use-case instance backed by the given repository.
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: AnnotationRepository + 'static> AnnotationUseCases for AnnotationUseCasesImpl<R> {
    #[instrument(skip(self, author_name, content, parent_annotation_id_str))]
    async fn create_from_dto(
        &self,
        block_id_str: &str,
        scope_str: &str,
        author_type_str: &str,
        author_name: &str,
        content: &str,
        parent_annotation_id_str: Option<&str>,
        highlight_start: Option<u32>,
        highlight_end: Option<u32>,
    ) -> Result<Annotation, ApplicationError> {
        // 1. Parse UUIDs
        let block_id = Uuid::parse_str(block_id_str)
            .map_err(|e| ApplicationError::Validation(format!("Invalid block UUID: {} - {}", block_id_str, e)))?;
        let parent_annotation_id = match parent_annotation_id_str {
            Some(s) if !s.is_empty() => Some(
                Uuid::parse_str(s)
                    .map_err(|e| ApplicationError::Validation(format!("Invalid parent UUID: {} - {}", s, e)))?,
            ),
            _ => None,
        };

        // 2. Parse enum strings
        let scope = AnnotationScope::try_from_str(scope_str)
            .ok_or_else(|| ApplicationError::Validation(format!("Invalid scope: '{}'", scope_str)))?;
        let author_type = AuthorType::try_from_str(author_type_str)
            .ok_or_else(|| ApplicationError::Validation(format!("Invalid authorType: '{}'", author_type_str)))?;

        // 2b. Pre-validate content at the use-case layer so the API
        // gets a `Validation` error (→ 400) rather than a wrapped
        // `Domain(InvalidData)`. The domain also re-validates in
        // `Annotation::new` — defense in depth.
        if content.trim().is_empty() {
            return Err(ApplicationError::Validation(
                "Annotation content cannot be empty".to_string(),
            ));
        }

        // 3. Construct via the domain — this re-validates invariants
        // (non-empty content, scope ↔ offset bounds) and is the
        // single source of truth for those checks.
        let create = AnnotationCreate {
            block_id,
            scope,
            author_type,
            author_name: author_name.to_string(),
            content: content.to_string(),
            parent_annotation_id,
            highlight_start,
            highlight_end,
        };
        let annotation = Annotation::new(create).map_err(ApplicationError::Domain)?;

        // 4. Persist
        self.repo
            .insert(&annotation)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(annotation)
    }

    #[instrument(skip(self))]
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Annotation>, ApplicationError> {
        self.repo
            .get_by_id(id)
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn list_by_block(&self, block_id: Uuid) -> Result<Vec<Annotation>, ApplicationError> {
        self.repo
            .get_by_block(block_id)
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self, filters))]
    async fn list_by_filters(
        &self,
        filters: &AnnotationFilters,
    ) -> Result<Vec<Annotation>, ApplicationError> {
        self.repo
            .get_by_filters(filters)
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self, resolved_by))]
    async fn update_status(
        &self,
        id: Uuid,
        status: AnnotationStatus,
        resolved_by: Option<String>,
    ) -> Result<Annotation, ApplicationError> {
        let mut annotation = self
            .repo
            .get_by_id(id)
            .await
            .map_err(ApplicationError::Domain)?
            .ok_or_else(|| ApplicationError::NotFound("Annotation", id))?;

        // Drive the entity through its lifecycle method so all
        // status invariants stay inside the domain. For
        // status == Resolved we use the dedicated `resolve(by)` —
        // for everything else we drop the `resolved_by` arg and
        // clear the resolved_at stamp.
        match status {
            AnnotationStatus::Resolved => {
                let by = resolved_by.ok_or_else(|| {
                    ApplicationError::Validation(
                        "resolved_by is required when status is 'resolved'".to_string(),
                    )
                })?;
                annotation.resolve(by);
            }
            AnnotationStatus::InProgress => {
                annotation.set_in_progress();
                // `set_in_progress` only moves out of Pending; if we
                // were already past it (e.g. resolved) this is a
                // no-op. We still apply the requested status to keep
                // the contract simple.
                annotation.status = AnnotationStatus::InProgress;
                annotation.resolved_at = None;
                annotation.resolved_by = None;
            }
            AnnotationStatus::Pending => {
                annotation.status = AnnotationStatus::Pending;
                annotation.resolved_at = None;
                annotation.resolved_by = None;
            }
            AnnotationStatus::Dismissed => {
                annotation.dismiss();
                annotation.resolved_at = None;
                annotation.resolved_by = None;
            }
        }

        self.repo
            .update(&annotation)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(annotation)
    }

    #[instrument(skip(self, resolved_by))]
    async fn resolve(
        &self,
        id: Uuid,
        resolved_by: String,
    ) -> Result<Annotation, ApplicationError> {
        self.update_status(id, AnnotationStatus::Resolved, Some(resolved_by))
            .await
    }

    #[instrument(skip(self))]
    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError> {
        self.repo.delete(id).await.map_err(ApplicationError::Domain)
    }
}

// ── Tests ──────────────────────────────────────────────────────────
//
// Pure unit tests for the use-case layer. They use
// `quilt_test_helpers::InMemoryAnnotationRepo` so we don't need a
// real database. The use cases are generic over `R: AnnotationRepository`
// so the same tests cover the production SQLite-backed impl.

#[cfg(test)]
mod tests {
    use super::*;
    use quilt_domain::entities::{AnnotationScope, AnnotationStatus, AuthorType};
    use quilt_test_helpers::InMemoryAnnotationRepo;

    /// Build a use-case instance backed by an in-memory repo.
    fn service() -> AnnotationUseCasesImpl<InMemoryAnnotationRepo> {
        AnnotationUseCasesImpl::new(InMemoryAnnotationRepo::new())
    }

    /// Convenience: create a block-scope annotation via the use case.
    async fn create_block_annotation(
        svc: &AnnotationUseCasesImpl<InMemoryAnnotationRepo>,
        block: Uuid,
        content: &str,
    ) -> Annotation {
        svc.create_from_dto(
            &block.to_string(),
            "block",
            "human",
            "alice",
            content,
            None,
            None,
            None,
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn create_block_scope_annotation_persists() {
        let svc = service();
        let block = Uuid::new_v4();
        let a = create_block_annotation(&svc, block, "Please review this").await;

        assert_eq!(a.block_id, block);
        assert_eq!(a.scope, AnnotationScope::Block);
        assert_eq!(a.author_type, AuthorType::Human);
        assert_eq!(a.author_name, "alice");
        assert_eq!(a.content, "Please review this");
        assert_eq!(a.status, AnnotationStatus::Pending);
        assert_eq!(a.resolved_at, None);
        assert_eq!(a.resolved_by, None);
    }

    #[tokio::test]
    async fn create_inline_scope_requires_offsets() {
        let svc = service();
        let block = Uuid::new_v4();
        let res = svc
            .create_from_dto(
                &block.to_string(),
                "inline",
                "agent",
                "claude",
                "Typo",
                None,
                None,
                None,
            )
            .await;
        assert!(res.is_err(), "inline scope without offsets must be rejected");
    }

    #[tokio::test]
    async fn create_inline_scope_with_offsets_succeeds() {
        let svc = service();
        let block = Uuid::new_v4();
        let a = svc
            .create_from_dto(
                &block.to_string(),
                "inline",
                "agent",
                "claude",
                "Typo here",
                None,
                Some(0),
                Some(5),
            )
            .await
            .unwrap();
        assert_eq!(a.scope, AnnotationScope::Inline);
        assert_eq!(a.highlight_start, Some(0));
        assert_eq!(a.highlight_end, Some(5));
    }

    #[tokio::test]
    async fn create_rejects_invalid_block_uuid() {
        let svc = service();
        let res = svc
            .create_from_dto(
                "not-a-uuid",
                "block",
                "human",
                "alice",
                "x",
                None,
                None,
                None,
            )
            .await;
        assert!(matches!(res, Err(ApplicationError::Validation(_))));
    }

    #[tokio::test]
    async fn create_rejects_invalid_scope() {
        let svc = service();
        let res = svc
            .create_from_dto(
                &Uuid::new_v4().to_string(),
                "sideways",
                "human",
                "alice",
                "x",
                None,
                None,
                None,
            )
            .await;
        assert!(matches!(res, Err(ApplicationError::Validation(_))));
    }

    #[tokio::test]
    async fn create_rejects_empty_content() {
        let svc = service();
        let res = svc
            .create_from_dto(
                &Uuid::new_v4().to_string(),
                "block",
                "human",
                "alice",
                "   ",
                None,
                None,
                None,
            )
            .await;
        assert!(matches!(res, Err(ApplicationError::Validation(_))));
    }

    #[tokio::test]
    async fn get_by_id_returns_existing() {
        let svc = service();
        let block = Uuid::new_v4();
        let a = create_block_annotation(&svc, block, "x").await;
        let loaded = svc.get_by_id(a.id).await.unwrap().unwrap();
        assert_eq!(loaded.id, a.id);
        assert_eq!(loaded.content, a.content);
    }

    #[tokio::test]
    async fn get_by_id_returns_none_for_missing() {
        let svc = service();
        let loaded = svc.get_by_id(Uuid::new_v4()).await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn list_by_block_filters_to_target_block() {
        let svc = service();
        let b1 = Uuid::new_v4();
        let b2 = Uuid::new_v4();
        let a1 = create_block_annotation(&svc, b1, "a1").await;
        let _a2 = create_block_annotation(&svc, b2, "a2").await;
        let _a3 = create_block_annotation(&svc, b1, "a3").await;

        let list = svc.list_by_block(b1).await.unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|a| a.id == a1.id));
    }

    #[tokio::test]
    async fn list_by_filters_with_status_returns_matching() {
        let svc = service();
        let block = Uuid::new_v4();
        let a1 = create_block_annotation(&svc, block, "x").await;
        let _a2 = create_block_annotation(&svc, block, "y").await;
        // resolve a1
        svc.resolve(a1.id, "claude".into()).await.unwrap();

        let f = AnnotationFilters::default().with_status("resolved");
        let list = svc.list_by_filters(&f).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, a1.id);
    }

    #[tokio::test]
    async fn list_by_filters_empty_returns_all() {
        let svc = service();
        let _a1 = create_block_annotation(&svc, Uuid::new_v4(), "x").await;
        let _a2 = create_block_annotation(&svc, Uuid::new_v4(), "y").await;
        let list = svc.list_by_filters(&AnnotationFilters::default()).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn update_status_resolve_stamps_resolved_at_and_by() {
        let svc = service();
        let a = create_block_annotation(&svc, Uuid::new_v4(), "x").await;
        let updated = svc
            .update_status(a.id, AnnotationStatus::Resolved, Some("claude".into()))
            .await
            .unwrap();
        assert_eq!(updated.status, AnnotationStatus::Resolved);
        assert_eq!(updated.resolved_by.as_deref(), Some("claude"));
        assert!(updated.resolved_at.is_some());

        // The DB row should match
        let from_db = svc.get_by_id(a.id).await.unwrap().unwrap();
        assert_eq!(from_db.status, AnnotationStatus::Resolved);
        assert_eq!(from_db.resolved_by.as_deref(), Some("claude"));
    }

    #[tokio::test]
    async fn update_status_resolved_without_resolved_by_is_rejected() {
        let svc = service();
        let a = create_block_annotation(&svc, Uuid::new_v4(), "x").await;
        let res = svc
            .update_status(a.id, AnnotationStatus::Resolved, None)
            .await;
        assert!(matches!(res, Err(ApplicationError::Validation(_))));
    }

    #[tokio::test]
    async fn update_status_dismiss_clears_resolved_fields() {
        let svc = service();
        let a = create_block_annotation(&svc, Uuid::new_v4(), "x").await;
        // First resolve
        svc.resolve(a.id, "claude".into()).await.unwrap();
        // Then dismiss
        let updated = svc
            .update_status(a.id, AnnotationStatus::Dismissed, None)
            .await
            .unwrap();
        assert_eq!(updated.status, AnnotationStatus::Dismissed);
        assert_eq!(updated.resolved_at, None);
        assert_eq!(updated.resolved_by, None);
    }

    #[tokio::test]
    async fn update_status_in_progress_is_applied() {
        let svc = service();
        let a = create_block_annotation(&svc, Uuid::new_v4(), "x").await;
        let updated = svc
            .update_status(a.id, AnnotationStatus::InProgress, None)
            .await
            .unwrap();
        assert_eq!(updated.status, AnnotationStatus::InProgress);
    }

    #[tokio::test]
    async fn update_status_missing_returns_not_found() {
        let svc = service();
        let res = svc
            .update_status(Uuid::new_v4(), AnnotationStatus::Resolved, Some("x".into()))
            .await;
        assert!(matches!(res, Err(ApplicationError::NotFound("Annotation", _))));
    }

    #[tokio::test]
    async fn resolve_is_convenience_for_update_status() {
        let svc = service();
        let a = create_block_annotation(&svc, Uuid::new_v4(), "x").await;
        let resolved = svc.resolve(a.id, "claude-desktop".into()).await.unwrap();
        assert_eq!(resolved.status, AnnotationStatus::Resolved);
        assert_eq!(resolved.resolved_by.as_deref(), Some("claude-desktop"));
        assert!(resolved.resolved_at.is_some());
    }

    #[tokio::test]
    async fn delete_removes_annotation() {
        let svc = service();
        let a = create_block_annotation(&svc, Uuid::new_v4(), "x").await;
        svc.delete(a.id).await.unwrap();
        let loaded = svc.get_by_id(a.id).await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn delete_unknown_id_is_noop() {
        let svc = service();
        // Should not error
        svc.delete(Uuid::new_v4()).await.unwrap();
    }

    #[tokio::test]
    async fn create_with_parent_uuid_persists_thread() {
        let svc = service();
        let block = Uuid::new_v4();
        let parent = create_block_annotation(&svc, block, "parent").await;
        let reply = svc
            .create_from_dto(
                &block.to_string(),
                "block",
                "agent",
                "claude",
                "reply",
                Some(&parent.id.to_string()),
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(reply.parent_annotation_id, Some(parent.id));
    }
}
