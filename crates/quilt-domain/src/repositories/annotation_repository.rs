//! AnnotationRepository trait - abstraction for annotation data access

use crate::entities::Annotation;
use crate::errors::DomainError;
use crate::value_objects::Uuid;
use async_trait::async_trait;

#[async_trait]
pub trait AnnotationRepository: Send + Sync {
    async fn insert(&self, annotation: &Annotation) -> Result<(), DomainError>;
    async fn update(&self, annotation: &Annotation) -> Result<(), DomainError>;
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Annotation>, DomainError>;
    async fn get_by_block(&self, block_id: Uuid) -> Result<Vec<Annotation>, DomainError>;
    async fn get_by_author(&self, author_name: &str) -> Result<Vec<Annotation>, DomainError>;
    async fn get_by_status(&self, status: &str) -> Result<Vec<Annotation>, DomainError>;
    async fn get_root_annotations(&self) -> Result<Vec<Annotation>, DomainError>;
    async fn get_thread_replies(&self, parent_id: Uuid) -> Result<Vec<Annotation>, DomainError>;
}

#[async_trait]
pub trait AnnotationRepositoryExt: AnnotationRepository {
    async fn get_open_annotations(&self) -> Result<Vec<Annotation>, DomainError> {
        let pending = self.get_by_status("pending").await?;
        let in_progress = self.get_by_status("in_progress").await?;
        Ok(pending.into_iter().chain(in_progress).collect())
    }

    async fn get_terminal_annotations(&self) -> Result<Vec<Annotation>, DomainError> {
        let resolved = self.get_by_status("resolved").await?;
        let dismissed = self.get_by_status("dismissed").await?;
        Ok(resolved.into_iter().chain(dismissed).collect())
    }
}

impl<T: AnnotationRepository + ?Sized> AnnotationRepositoryExt for T {}
