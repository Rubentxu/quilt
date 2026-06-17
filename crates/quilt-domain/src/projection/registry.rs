//! Projection registry port — lookup and iteration over registered projections.
//!
//! The [`ProjectionRegistry`] is the **port** through which the resolver
//! obtains all registered contracts and their associated [`Projection`] adapters.
//! The application layer provides the implementation ([`StaticProjectionRegistry`]).

use crate::projection::contract::ProjectionContract;
use crate::projection::contract::ProjectionContractId;
use crate::projection::projection_trait::Projection;
use std::sync::Arc;

/// A projection registered alongside its contract.
#[derive(Clone)]
pub struct RegisteredProjection {
    /// The declarative contract.
    pub contract: ProjectionContract,
    /// The concrete projection adapter implementing the contract.
    pub projection: Arc<dyn Projection>,
}

impl std::fmt::Debug for RegisteredProjection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegisteredProjection")
            .field("contract", &self.contract)
            .field("projection", &"<dyn Projection>")
            .finish()
    }
}

/// Registry port — look up projections by ID and iterate over all registered ones.
///
/// This is the **port** interface. The application layer provides the
/// [`StaticProjectionRegistry::v1()`] implementation with the 6 V1 contracts.
pub trait ProjectionRegistry: Send + Sync + 'static {
    /// Look up a registered projection by its contract ID.
    fn get(&self, id: &ProjectionContractId) -> Option<RegisteredProjection>;

    /// Iterate over all registered projections.
    ///
    /// Returns a boxed iterator for object-safety.
    fn iter(&self) -> Box<dyn Iterator<Item = RegisteredProjection> + Send + Sync>;

    /// Return the list of V1 contract IDs in deterministic order.
    ///
    /// Used by clients that need to know the stable V1 surface
    /// without iterating the full registry.
    fn v1_contract_ids(&self) -> Vec<ProjectionContractId>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::default::DefaultProjection;
    use crate::projection::projection_trait::Projection;

    // Minimal mock registry for testing the trait bounds
    struct MockRegistry {
        items: Vec<RegisteredProjection>,
    }

    impl ProjectionRegistry for MockRegistry {
        fn get(&self, id: &ProjectionContractId) -> Option<RegisteredProjection> {
            self.items.iter().find(|rp| &rp.contract.id == id).cloned()
        }

        fn iter(&self) -> Box<dyn Iterator<Item = RegisteredProjection> + Send + Sync> {
            Box::new(self.items.clone().into_iter())
        }

        fn v1_contract_ids(&self) -> Vec<ProjectionContractId> {
            self.items.iter().map(|rp| rp.contract.id.clone()).collect()
        }
    }

    #[test]
    fn registered_projection_clone_is_cheap() {
        // RegisteredProjection wraps Arc<dyn Projection> so cloning is cheap
        let rp = RegisteredProjection {
            contract: crate::projection::contract::ProjectionContract::new(
                ProjectionContractId::new("test").unwrap(),
            ),
            projection: Arc::new(DefaultProjection::new()),
        };
        let _cloned = rp.clone();
        // Clone should not panic
    }

    #[test]
    fn mock_registry_get() {
        let contract = crate::projection::contract::ProjectionContract::new(
            ProjectionContractId::new("task").unwrap(),
        );
        let rp = RegisteredProjection {
            contract,
            projection: Arc::new(DefaultProjection::new()),
        };

        let registry = MockRegistry { items: vec![rp] };
        let found = registry.get(&ProjectionContractId::new("task").unwrap());
        assert!(found.is_some());

        let not_found = registry.get(&ProjectionContractId::new("nonexistent").unwrap());
        assert!(not_found.is_none());
    }

    #[test]
    fn mock_registry_iter() {
        let registry = MockRegistry {
            items: vec![
                RegisteredProjection {
                    contract: crate::projection::contract::ProjectionContract::new(
                        ProjectionContractId::new("a").unwrap(),
                    ),
                    projection: Arc::new(DefaultProjection::new()),
                },
                RegisteredProjection {
                    contract: crate::projection::contract::ProjectionContract::new(
                        ProjectionContractId::new("b").unwrap(),
                    ),
                    projection: Arc::new(DefaultProjection::new()),
                },
            ],
        };

        let ids: Vec<_> = registry
            .iter()
            .map(|rp| rp.contract.id.as_str().to_string())
            .collect();
        assert_eq!(ids, vec!["a", "b"]);
    }

    #[test]
    fn registry_trait_is_object_safe() {
        // Verify the trait can be used as dyn ProjectionRegistry
        fn _check<T: ProjectionRegistry>() {}
        _check::<MockRegistry>();
    }
}
