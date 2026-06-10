//! ClassValidator - validates blocks against class constraints

#[cfg(test)]
use crate::content::BlockContent;
use crate::entities::Block;
use crate::errors::DomainError;
use crate::repositories::ClassRepository;
use crate::repositories::PropertyRepository;
use crate::value_objects::Uuid;
use std::collections::HashSet;
use std::sync::Arc;

/// ClassValidator validates blocks against class constraints.
///
/// It checks:
/// - All required properties are present
/// - Property values pass PropertyValidator checks
/// - No circular inheritance exists
pub struct ClassValidator<C: ClassRepository, P: PropertyRepository> {
    class_repo: Arc<C>,
    property_repo: Arc<P>,
}

impl<C: ClassRepository, P: PropertyRepository> ClassValidator<C, P> {
    /// Create a new ClassValidator
    pub fn new(class_repo: Arc<C>, property_repo: Arc<P>) -> Self {
        Self {
            class_repo,
            property_repo,
        }
    }

    /// Validate a block against its class constraints.
    ///
    /// Takes the block and a list of class IDs (tags) that the block belongs to.
    /// Returns Ok(()) if all constraints are satisfied.
    pub async fn validate_block(
        &self,
        block: &Block,
        class_ids: &[Uuid],
    ) -> Result<(), DomainError> {
        if class_ids.is_empty() {
            return Ok(()); // No classes to validate against
        }

        // Get all required properties across all classes
        let all_required = self.get_all_required_properties(class_ids).await?;

        // Check each required property is present
        for prop_id in &all_required {
            if !block.properties.contains_key(&prop_id.to_string()) {
                // Try with common naming patterns
                let has_property = block.properties.keys().any(|k: &String| {
                    // Check if any property key could match this required property
                    // This is a simplified check - in real impl would look up property definition
                    k.to_lowercase()
                        .contains(&prop_id.to_string().to_lowercase())
                });

                if !has_property {
                    return Err(DomainError::MissingRequiredProperty {
                        property_id: *prop_id,
                    });
                }
            }
        }

        // Validate property values using PropertyValidator
        let prop_validator = crate::properties::PropertyValidator::new(self.property_repo.clone());
        prop_validator.validate(&block.properties).await?;

        Ok(())
    }

    /// Get all required properties for a set of classes (including inherited).
    async fn get_all_required_properties(
        &self,
        class_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, DomainError> {
        let mut seen = HashSet::new();
        let mut result = Vec::new();

        for class_id in class_ids {
            let ancestors = self.class_repo.get_ancestors(*class_id).await?;
            for ancestor_id in ancestors {
                if seen.insert(ancestor_id) {
                    let required = self.class_repo.get_required_properties(ancestor_id).await?;
                    for prop_id in required {
                        if seen.insert(prop_id) {
                            result.push(prop_id);
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// Check if adding an inheritance link would create a cycle.
    ///
    /// Returns Err(DomainError::CircularInheritance) if the link would create a cycle.
    pub async fn check_inheritance_cycle(
        &self,
        child_id: Uuid,
        parent_id: Uuid,
    ) -> Result<(), DomainError> {
        // Self-reference check
        if child_id == parent_id {
            return Err(DomainError::CircularInheritance {
                class_id: child_id,
                message: "A class cannot extend itself".to_string(),
            });
        }

        // Check if parent is already an ancestor of child
        let ancestors = self.class_repo.get_ancestors(child_id).await?;
        if ancestors.contains(&parent_id) {
            return Err(DomainError::CircularInheritance {
                class_id: child_id,
                message: format!(
                    "Adding {} as parent of {} would create a circular inheritance",
                    parent_id, child_id
                ),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classes::types::Class;
    use crate::services::TimezoneService;
    use crate::value_objects::Uuid;
    use std::collections::HashMap;

    // Mock implementations for testing
    struct MockClassRepository {
        classes: HashMap<Uuid, Class>,
        inheritance: HashMap<Uuid, Vec<Uuid>>,
        required_props: HashMap<Uuid, Vec<Uuid>>,
    }

    impl MockClassRepository {
        fn new() -> Self {
            Self {
                classes: HashMap::new(),
                inheritance: HashMap::new(),
                required_props: HashMap::new(),
            }
        }

        fn _with_class(mut self, class: Class) -> Self {
            let id = class.id;
            self.classes.insert(id, class);
            self
        }

        fn _with_inheritance(mut self, child_id: Uuid, parent_id: Uuid) -> Self {
            self.inheritance
                .entry(child_id)
                .or_default()
                .push(parent_id);
            self
        }

        fn _with_required_props(mut self, class_id: Uuid, props: Vec<Uuid>) -> Self {
            self.required_props.insert(class_id, props);
            self
        }
    }

    #[async_trait::async_trait]
    impl ClassRepository for MockClassRepository {
        async fn get_by_id(&self, id: Uuid) -> Result<Option<Class>, DomainError> {
            Ok(self.classes.get(&id).cloned())
        }

        async fn get_by_db_ident(&self, _ident: &str) -> Result<Option<Class>, DomainError> {
            Ok(None)
        }

        async fn get_ancestors(&self, class_id: Uuid) -> Result<Vec<Uuid>, DomainError> {
            let mut result = vec![class_id];
            let mut visited = HashSet::new();
            visited.insert(class_id);

            let mut queue = vec![class_id];
            while let Some(current) = queue.pop() {
                if let Some(parents) = self.inheritance.get(&current) {
                    for &parent in parents {
                        if visited.insert(parent) {
                            result.push(parent);
                            queue.push(parent);
                        }
                    }
                }
            }

            Ok(result)
        }

        async fn get_required_properties(&self, class_id: Uuid) -> Result<Vec<Uuid>, DomainError> {
            Ok(self
                .required_props
                .get(&class_id)
                .cloned()
                .unwrap_or_default())
        }

        async fn get_default_properties(
            &self,
            _class_id: Uuid,
        ) -> Result<Vec<(Uuid, String)>, DomainError> {
            Ok(Vec::new())
        }

        async fn insert(&self, _class: &Class) -> Result<(), DomainError> {
            Ok(())
        }

        async fn update(&self, _class: &Class) -> Result<(), DomainError> {
            Ok(())
        }

        async fn delete(&self, _id: Uuid) -> Result<(), DomainError> {
            Ok(())
        }
        async fn get_by_db_idents(&self, _idents: &[&str]) -> Result<Vec<PropertyDefinition>, DomainError> {
            Ok(Vec::new())
        }
        async fn search(&self, _query: &str, _limit: usize) -> Result<Vec<PropertyDefinition>, DomainError> {
            Ok(Vec::new())
        }
        async fn list_by_usage(&self, _limit: usize) -> Result<Vec<PropertyDefinition>, DomainError> {
            Ok(Vec::new())
        }

        async fn add_inheritance(
            &self,
            _child_id: Uuid,
            _parent_id: Uuid,
        ) -> Result<(), DomainError> {
            Ok(())
        }

        async fn remove_inheritance(
            &self,
            _child_id: Uuid,
            _parent_id: Uuid,
        ) -> Result<(), DomainError> {
            Ok(())
        }

        async fn add_required_property(
            &self,
            _class_id: Uuid,
            _property_id: Uuid,
        ) -> Result<(), DomainError> {
            Ok(())
        }

        async fn remove_required_property(
            &self,
            _class_id: Uuid,
            _property_id: Uuid,
        ) -> Result<(), DomainError> {
            Ok(())
        }

        async fn add_default_property(
            &self,
            _class_id: Uuid,
            _property_id: Uuid,
            _default_json: &str,
        ) -> Result<(), DomainError> {
            Ok(())
        }

        async fn remove_default_property(
            &self,
            _class_id: Uuid,
            _property_id: Uuid,
        ) -> Result<(), DomainError> {
            Ok(())
        }
    }

    struct MockPropertyRepository;

    #[async_trait::async_trait]
    impl PropertyRepository for MockPropertyRepository {
        async fn get_by_id(
            &self,
            _id: Uuid,
        ) -> Result<Option<crate::properties::PropertyDefinition>, DomainError> {
            Ok(None)
        }

        async fn get_by_db_ident(
            &self,
            _ident: &str,
        ) -> Result<Option<crate::properties::PropertyDefinition>, DomainError> {
            Ok(None)
        }

        async fn get_all(&self) -> Result<Vec<crate::properties::PropertyDefinition>, DomainError> {
            Ok(Vec::new())
        }

        async fn insert(
            &self,
            _def: &crate::properties::PropertyDefinition,
        ) -> Result<(), DomainError> {
            Ok(())
        }

        async fn update(
            &self,
            _def: &crate::properties::PropertyDefinition,
        ) -> Result<(), DomainError> {
            Ok(())
        }

        async fn get_closed_values(
            &self,
            _property_id: Uuid,
        ) -> Result<Vec<crate::properties::types::ClosedValue>, DomainError> {
            Ok(Vec::new())
        }

        async fn delete(&self, _id: Uuid) -> Result<(), DomainError> {
            Ok(())
        }
        async fn get_by_db_idents(&self, _idents: &[&str]) -> Result<Vec<PropertyDefinition>, DomainError> {
            Ok(Vec::new())
        }
        async fn search(&self, _query: &str, _limit: usize) -> Result<Vec<PropertyDefinition>, DomainError> {
            Ok(Vec::new())
        }
        async fn list_by_usage(&self, _limit: usize) -> Result<Vec<PropertyDefinition>, DomainError> {
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn test_validate_block_with_no_classes() {
        let class_repo = Arc::new(MockClassRepository::new());
        let prop_repo = Arc::new(MockPropertyRepository);
        let validator = ClassValidator::new(class_repo, prop_repo);

        let tz = TimezoneService::from_tz_string("UTC").unwrap();
        let block = Block::new(
            crate::entities::BlockCreate {
                page_id: Uuid::new_v4(),
                content: BlockContent::from_text("Test"),
                parent_id: None,
                order: 1.0,
                marker: None,
                format: crate::value_objects::BlockFormat::Markdown,
                properties: HashMap::new(),
            },
            &tz,
        )
        .unwrap();

        // No classes - should pass
        let result = validator.validate_block(&block, &[]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_inheritance_cycle_self_reference() {
        let class_repo = Arc::new(MockClassRepository::new());
        let prop_repo = Arc::new(MockPropertyRepository);
        let validator = ClassValidator::new(class_repo, prop_repo);

        let id = Uuid::new_v4();
        let result = validator.check_inheritance_cycle(id, id).await;
        assert!(result.is_err());
        if let Err(DomainError::CircularInheritance { class_id, message }) = result {
            assert_eq!(class_id, id);
            assert!(message.contains("cannot extend itself"));
        }
    }
}
