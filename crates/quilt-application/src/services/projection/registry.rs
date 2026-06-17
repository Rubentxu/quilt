//! Static projection registry — V1 contract registration with build-time validation.
//!
//! `StaticProjectionRegistry` is the **application-layer implementation**
//! of the [`quilt_domain::projection::registry::ProjectionRegistry`] port.
//!
//! ## V1 contracts
//!
//! | ID | Priority | Predicate summary |
//! |----|----------|-------------------|
//! | `task` | 100 | `type:: task` + `status::` in known values |
//! | `heading` | 150 | `block-role:: heading` + `heading-level:: 1|2|3` |
//! | `media` | 200 | `type:: media` + `media-type:: video|image` |
//! | `date` | 250 | `scheduled::` OR `deadline::` set |
//! | `link` | 300 | `link::` set |
//! | `default` | u32::MAX | (wildcard — always matches) |

use crate::services::projection::contracts;
use quilt_domain::projection::contract::{ProjectionContract, ProjectionContractId};
use quilt_domain::projection::default::DefaultProjection as DomainDefaultProjection;
use quilt_domain::projection::registry::{ProjectionRegistry, RegisteredProjection};
use std::sync::Arc;

/// Build-time validated V1 projection registry.
///
/// Constructed via [`StaticProjectionRegistry::v1()`] which asserts
/// all registered IDs are unique and the default contract is present.
#[derive(Debug, Clone)]
pub struct StaticProjectionRegistry {
    /// Ordered by descending priority (most-specific first).
    items: Vec<RegisteredProjection>,
}

impl StaticProjectionRegistry {
    /// Build the V1 registry.
    ///
    /// # Build-time validation
    ///
    /// - All contract IDs are unique
    /// - The `default` contract is always present with priority `u32::MAX`
    /// - No two contracts share the same priority value
    #[must_use]
    pub fn v1() -> Self {
        let items = vec![
            // task — most specific: type:: task + status:: in known values (priority 100)
            RegisteredProjection {
                contract: contracts::task::task_contract(),
                projection: Arc::new(contracts::TaskProjection::new()),
            },
            // heading — block-role:: heading + heading-level:: 1|2|3 (priority 150)
            RegisteredProjection {
                contract: contracts::heading::heading_contract(),
                projection: Arc::new(contracts::HeadingProjection::new()),
            },
            // media — type:: media + media-type:: video|image (priority 200)
            RegisteredProjection {
                contract: contracts::media::media_contract(),
                projection: Arc::new(contracts::MediaProjection::new()),
            },
            // date — scheduled:: OR deadline:: set (priority 250)
            RegisteredProjection {
                contract: contracts::date::date_contract(),
                projection: Arc::new(contracts::DateProjection::new()),
            },
            // link — link:: set (priority 300)
            RegisteredProjection {
                contract: contracts::link::link_contract(),
                projection: Arc::new(contracts::LinkProjection::new()),
            },
            // default — wildcard, always matches (priority u32::MAX)
            RegisteredProjection {
                contract: ProjectionContract::new(ProjectionContractId::new("default").unwrap())
                    .with_priority(u32::MAX),
                projection: Arc::new(contracts::DefaultProjection::new()),
            },
        ];

        // ── Build-time validation ──────────────────────────────────

        // Ensure all IDs are unique
        let ids: Vec<_> = items.iter().map(|rp| rp.contract.id.as_str()).collect();
        let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(
            ids.len(),
            unique_ids.len(),
            "Duplicate projection contract IDs in registry: {ids:?}"
        );

        // Ensure default is present
        assert!(
            ids.contains(&"default"),
            "Default projection contract must be present in V1 registry"
        );

        // Ensure no two non-default contracts share the same priority
        let priorities: Vec<_> = items
            .iter()
            .filter(|rp| rp.contract.id.as_str() != "default")
            .map(|rp| rp.contract.priority)
            .collect();
        let unique_priorities: std::collections::HashSet<_> = priorities.iter().collect();
        assert_eq!(
            priorities.len(),
            unique_priorities.len(),
            "Duplicate priorities in V1 registry: {priorities:?}"
        );

        Self { items }
    }

    /// Returns a reference to the default projection (ZST).
    #[must_use]
    pub fn default_projection() -> DomainDefaultProjection {
        DomainDefaultProjection::new()
    }
}

impl ProjectionRegistry for StaticProjectionRegistry {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_registry_contains_six_contracts() {
        let registry = StaticProjectionRegistry::v1();
        let ids: Vec<_> = registry
            .v1_contract_ids()
            .into_iter()
            .map(|id| id.as_str().to_string())
            .collect();
        assert_eq!(ids.len(), 6);
    }

    #[test]
    fn v1_registry_has_all_expected_ids() {
        let registry = StaticProjectionRegistry::v1();
        let ids: std::collections::HashSet<_> = registry
            .v1_contract_ids()
            .into_iter()
            .map(|id| id.as_str().to_string())
            .collect();
        assert!(ids.contains("task"));
        assert!(ids.contains("heading"));
        assert!(ids.contains("media"));
        assert!(ids.contains("date"));
        assert!(ids.contains("link"));
        assert!(ids.contains("default"));
    }

    #[test]
    fn v1_registry_get_returns_correct_projection() {
        let registry = StaticProjectionRegistry::v1();
        let rp = registry
            .get(&ProjectionContractId::new("task").unwrap())
            .unwrap();
        assert_eq!(rp.contract.id.as_str(), "task");
        assert_eq!(rp.projection.contract_id().as_str(), "task");
    }

    #[test]
    fn v1_registry_get_returns_none_for_unknown() {
        let registry = StaticProjectionRegistry::v1();
        assert!(
            registry
                .get(&ProjectionContractId::new("nonexistent").unwrap())
                .is_none()
        );
    }

    #[test]
    fn v1_registry_iter_has_six_items() {
        let registry = StaticProjectionRegistry::v1();
        let count = registry.iter().count();
        assert_eq!(count, 6);
    }

    #[test]
    fn v1_registry_priorities_are_unique() {
        let registry = StaticProjectionRegistry::v1();
        let priorities: std::collections::HashSet<_> =
            registry.iter().map(|rp| rp.contract.priority).collect();
        assert_eq!(priorities.len(), registry.iter().count());
    }

    #[test]
    fn default_contract_has_max_priority() {
        let registry = StaticProjectionRegistry::v1();
        let default_rp = registry
            .iter()
            .find(|rp| rp.contract.id.as_str() == "default")
            .unwrap();
        assert_eq!(default_rp.contract.priority, u32::MAX);
    }

    #[test]
    fn task_contract_is_highest_priority() {
        let registry = StaticProjectionRegistry::v1();
        // task has priority 100 — lowest number = highest priority
        let task_rp = registry
            .iter()
            .find(|rp| rp.contract.id.as_str() == "task")
            .unwrap();
        assert_eq!(task_rp.contract.priority, 100);
    }
}
