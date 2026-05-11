//! ClassRepository trait - abstraction for class persistence

use crate::classes::types::Class;
use crate::errors::DomainError;
use crate::value_objects::Uuid;
use async_trait::async_trait;

/// ClassRepository provides access to class definitions.
///
/// This trait defines the contract for persisting and retrieving
/// class definitions with their inheritance relationships.
#[async_trait]
pub trait ClassRepository: Send + Sync {
    /// Get a class by ID
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Class>, DomainError>;

    /// Get a class by database identifier
    async fn get_by_db_ident(&self, ident: &str) -> Result<Option<Class>, DomainError>;

    /// Get all ancestor class IDs for a given class (including the class itself)
    /// Uses recursive inheritance resolution.
    async fn get_ancestors(&self, class_id: Uuid) -> Result<Vec<Uuid>, DomainError>;

    /// Get the required property IDs for a class (includes inherited properties)
    async fn get_required_properties(&self, class_id: Uuid) -> Result<Vec<Uuid>, DomainError>;

    /// Get the default property values for a class (includes inherited defaults)
    async fn get_default_properties(
        &self,
        class_id: Uuid,
    ) -> Result<Vec<(Uuid, String)>, DomainError>;

    /// Insert a new class
    async fn insert(&self, class: &Class) -> Result<(), DomainError>;

    /// Update an existing class
    async fn update(&self, class: &Class) -> Result<(), DomainError>;

    /// Delete a class
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;

    /// Add an inheritance relationship (child extends parent)
    async fn add_inheritance(&self, child_id: Uuid, parent_id: Uuid) -> Result<(), DomainError>;

    /// Remove an inheritance relationship
    async fn remove_inheritance(&self, child_id: Uuid, parent_id: Uuid) -> Result<(), DomainError>;

    /// Add a required property to a class
    async fn add_required_property(
        &self,
        class_id: Uuid,
        property_id: Uuid,
    ) -> Result<(), DomainError>;

    /// Remove a required property from a class
    async fn remove_required_property(
        &self,
        class_id: Uuid,
        property_id: Uuid,
    ) -> Result<(), DomainError>;

    /// Add a default property to a class
    async fn add_default_property(
        &self,
        class_id: Uuid,
        property_id: Uuid,
        default_json: &str,
    ) -> Result<(), DomainError>;

    /// Remove a default property from a class
    async fn remove_default_property(
        &self,
        class_id: Uuid,
        property_id: Uuid,
    ) -> Result<(), DomainError>;
}

/// ClassRepositoryExt provides convenience methods built on ClassRepository.
#[async_trait]
pub trait ClassRepositoryExt: ClassRepository {
    /// Check if a class with the given database identifier exists
    async fn exists(&self, db_ident: &str) -> Result<bool, DomainError> {
        Ok(self.get_by_db_ident(db_ident).await?.is_some())
    }

    /// Find a class or return a builtin class if not found
    async fn find_or_builtin(&self, db_ident: &str) -> Result<Option<Class>, DomainError> {
        if let Some(class) = self.get_by_db_ident(db_ident).await? {
            return Ok(Some(class));
        }
        // Check builtin classes
        for class in crate::classes::types::builtin_classes::all() {
            if class.db_ident == db_ident {
                return Ok(Some(class));
            }
        }
        Ok(None)
    }
}

impl<T: ClassRepository + ?Sized> ClassRepositoryExt for T {}

#[cfg(test)]
mod tests {
    // Trait tests would require a mock implementation
    // Integration tests are in quilt-infrastructure
}
