//! FileRepository trait - abstraction for file data access

use crate::entities::File;
use crate::errors::DomainError;
use crate::value_objects::Uuid;
use async_trait::async_trait;

/// FileRepository is the abstraction for file data access.
#[async_trait]
pub trait FileRepository: Send + Sync {
    /// Get a file by its ID
    async fn get_by_id(&self, id: Uuid) -> Result<Option<File>, DomainError>;

    /// Get a file by its path
    async fn get_by_path(&self, path: &str) -> Result<Option<File>, DomainError>;

    /// Get all files in a directory
    async fn get_by_directory(&self, dir: &str) -> Result<Vec<File>, DomainError>;

    /// Insert a new file record
    async fn insert(&self, file: &File) -> Result<(), DomainError>;

    /// Update a file record
    async fn update(&self, file: &File) -> Result<(), DomainError>;

    /// Delete a file by ID
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;

    /// Delete a file by path
    async fn delete_by_path(&self, path: &str) -> Result<(), DomainError>;

    /// Get files by type (images, pdfs, etc.)
    async fn get_by_type(&self, mime_prefix: &str) -> Result<Vec<File>, DomainError>;

    /// Search files by path
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<File>, DomainError>;
}

/// FileRepositoryExt provides additional convenience methods
#[async_trait]
pub trait FileRepositoryExt: FileRepository {
    /// Check if a file exists by path
    async fn exists_by_path(&self, path: &str) -> Result<bool, DomainError> {
        Ok(self.get_by_path(path).await?.is_some())
    }

    /// Get a file or fail with an error
    async fn get_or_fail(&self, id: Uuid) -> Result<File, DomainError> {
        self.get_by_id(id)
            .await?
            .ok_or(DomainError::FileNotFound(id))
    }

    /// Get all image files
    async fn get_images(&self) -> Result<Vec<File>, DomainError> {
        self.get_by_type("image").await
    }

    /// Get all PDF files
    async fn get_pdfs(&self) -> Result<Vec<File>, DomainError> {
        self.get_by_type("application/pdf").await
    }
}

impl<T: FileRepository + ?Sized> FileRepositoryExt for T {}
