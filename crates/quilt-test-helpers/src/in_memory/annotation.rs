//! In-memory implementation of [`AnnotationRepository`].
//!
//! Mirrors `SqliteAnnotationRepository` semantics for use in unit
//! and component tests that don't want to spin up a real SQLite.
//! Stored in a `HashMap<Uuid, Annotation>` protected by a `RwLock`.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;

use quilt_domain::entities::Annotation;
use quilt_domain::errors::DomainError;
use quilt_domain::repositories::{AnnotationFilters, AnnotationRepository};
use quilt_domain::value_objects::Uuid;

/// In-memory annotation repository, wrapped for test usability.
///
/// Provides a builder API that returns `Arc<Self>` for chaining.
#[derive(Debug)]
pub struct InMemoryAnnotationRepo {
    /// The inner store, keyed by `Annotation.id`.
    repo: RwLock<HashMap<Uuid, Annotation>>,
}

impl Default for InMemoryAnnotationRepo {
    fn default() -> Self {
        Self {
            repo: RwLock::new(HashMap::new()),
        }
    }
}

impl InMemoryAnnotationRepo {
    /// Create a new empty in-memory annotation repository.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            repo: RwLock::new(HashMap::new()),
        })
    }

    /// Add pre-existing annotations to the repository.
    ///
    /// Consumes `self` and returns an `Arc<Self>` for chaining.
    pub fn with_annotations(self: Arc<Self>, annotations: Vec<Annotation>) -> Arc<Self> {
        {
            let mut repo = self.repo.write();
            for a in annotations {
                repo.insert(a.id, a);
            }
        }
        self
    }

    /// Get a trait object reference for use in traits that require
    /// `dyn AnnotationRepository`.
    pub fn as_trait(self: Arc<Self>) -> Arc<dyn AnnotationRepository> {
        self
    }

    /// Snapshot of the number of annotations currently in the
    /// repository. Useful for assertion in tests.
    pub fn len(&self) -> usize {
        self.repo.read().len()
    }

    /// Whether the repository has zero annotations.
    pub fn is_empty(&self) -> bool {
        self.repo.read().is_empty()
    }
}

#[async_trait]
impl AnnotationRepository for InMemoryAnnotationRepo {
    async fn insert(&self, annotation: &Annotation) -> Result<(), DomainError> {
        let mut repo = self.repo.write();
        if repo.contains_key(&annotation.id) {
            return Err(DomainError::AlreadyExists(format!(
                "annotation id {}",
                annotation.id
            )));
        }
        repo.insert(annotation.id, annotation.clone());
        Ok(())
    }

    async fn update(&self, annotation: &Annotation) -> Result<(), DomainError> {
        let mut repo = self.repo.write();
        if !repo.contains_key(&annotation.id) {
            return Err(DomainError::NotFound(format!(
                "annotation id {}",
                annotation.id
            )));
        }
        repo.insert(annotation.id, annotation.clone());
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        self.repo.write().remove(&id);
        Ok(())
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Option<Annotation>, DomainError> {
        Ok(self.repo.read().get(&id).cloned())
    }

    async fn get_by_block(&self, block_id: Uuid) -> Result<Vec<Annotation>, DomainError> {
        let mut all: Vec<Annotation> = self
            .repo
            .read()
            .values()
            .filter(|a| a.block_id == block_id)
            .cloned()
            .collect();
        // Tie-break by string form of id (Uuid is not Ord). Order is
        // stable across calls but not meaningful — only `created_at`
        // order is part of the contract.
        all.sort_by(|a, b| {
            a.created_at
                .cmp(&b.created_at)
                .then_with(|| a.id.to_string().cmp(&b.id.to_string()))
        });
        Ok(all)
    }

    async fn get_by_author(&self, author_name: &str) -> Result<Vec<Annotation>, DomainError> {
        let mut all: Vec<Annotation> = self
            .repo
            .read()
            .values()
            .filter(|a| a.author_name == author_name)
            .cloned()
            .collect();
        all.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.id.to_string().cmp(&a.id.to_string()))
        });
        Ok(all)
    }

    async fn get_by_status(&self, status: &str) -> Result<Vec<Annotation>, DomainError> {
        let mut all: Vec<Annotation> = self
            .repo
            .read()
            .values()
            .filter(|a| a.status.as_str() == status)
            .cloned()
            .collect();
        all.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.id.to_string().cmp(&a.id.to_string()))
        });
        Ok(all)
    }

    async fn get_root_annotations(&self) -> Result<Vec<Annotation>, DomainError> {
        let mut all: Vec<Annotation> = self
            .repo
            .read()
            .values()
            .filter(|a| a.parent_annotation_id.is_none())
            .cloned()
            .collect();
        all.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.id.to_string().cmp(&a.id.to_string()))
        });
        Ok(all)
    }

    async fn get_thread_replies(&self, parent_id: Uuid) -> Result<Vec<Annotation>, DomainError> {
        let mut all: Vec<Annotation> = self
            .repo
            .read()
            .values()
            .filter(|a| a.parent_annotation_id == Some(parent_id))
            .cloned()
            .collect();
        all.sort_by(|a, b| {
            a.created_at
                .cmp(&b.created_at)
                .then_with(|| a.id.to_string().cmp(&b.id.to_string()))
        });
        Ok(all)
    }

    async fn get_by_filters(
        &self,
        filters: &AnnotationFilters,
    ) -> Result<Vec<Annotation>, DomainError> {
        let all: Vec<Annotation> = self.repo.read().values().cloned().collect();
        let mut filtered: Vec<Annotation> = all
            .into_iter()
            .filter(|a| {
                if let Some(ref bid) = filters.block_id {
                    if &a.block_id != bid {
                        return false;
                    }
                }
                if let Some(ref status) = filters.status {
                    if a.status.as_str() != status {
                        return false;
                    }
                }
                if let Some(ref scope) = filters.scope {
                    if &a.scope != scope {
                        return false;
                    }
                }
                if let Some(ref name) = filters.author_name {
                    if &a.author_name != name {
                        return false;
                    }
                }
                true
            })
            .collect();
        filtered.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.id.to_string().cmp(&a.id.to_string()))
        });
        Ok(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quilt_domain::entities::{AnnotationCreate, AnnotationScope, AuthorType};

    fn make_ann(block_id: Uuid, content: &str, author: &str, scope: AnnotationScope) -> Annotation {
        Annotation::new(AnnotationCreate {
            block_id,
            scope,
            author_type: AuthorType::Human,
            author_name: author.to_string(),
            content: content.to_string(),
            parent_annotation_id: None,
            highlight_start: None,
            highlight_end: None,
        })
        .unwrap()
    }

    #[tokio::test]
    async fn empty_repo_is_empty() {
        let repo = InMemoryAnnotationRepo::new();
        assert!(repo.is_empty());
        assert_eq!(repo.len(), 0);
    }

    #[tokio::test]
    async fn with_annotations_seeds() {
        let a = make_ann(Uuid::new_v4(), "x", "u", AnnotationScope::Block);
        let b = make_ann(Uuid::new_v4(), "y", "u", AnnotationScope::Block);
        let repo = InMemoryAnnotationRepo::new().with_annotations(vec![a, b]);
        assert_eq!(repo.len(), 2);
    }

    #[tokio::test]
    async fn insert_duplicate_id_fails() {
        let repo = InMemoryAnnotationRepo::new();
        let a = make_ann(Uuid::new_v4(), "x", "u", AnnotationScope::Block);
        repo.insert(&a).await.unwrap();
        let res = repo.insert(&a).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn update_missing_fails() {
        let repo = InMemoryAnnotationRepo::new();
        let a = make_ann(Uuid::new_v4(), "x", "u", AnnotationScope::Block);
        let res = repo.update(&a).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn get_by_filters_matches_all() {
        let repo = InMemoryAnnotationRepo::new();
        let block = Uuid::new_v4();
        let a = make_ann(block, "a", "alice", AnnotationScope::Block);
        // Inline requires offsets, build it directly with the create
        // shape that satisfies the invariants.
        let b = Annotation::new(AnnotationCreate {
            block_id: block,
            scope: AnnotationScope::Inline,
            author_type: AuthorType::Human,
            author_name: "bob".to_string(),
            content: "b".to_string(),
            parent_annotation_id: None,
            highlight_start: Some(0),
            highlight_end: Some(1),
        })
        .unwrap();
        repo.insert(&a).await.unwrap();
        repo.insert(&b).await.unwrap();

        let f = AnnotationFilters::default().with_block_id(block);
        let loaded = repo.get_by_filters(&f).await.unwrap();
        assert_eq!(loaded.len(), 2);
    }
}
