//! Projection-level conflict — detected when multiple contracts tie in score.
//!
//! This is architecturally distinct from the **patch-level** [`ProjectionConflict`]
//! in [`crate::canonicalization`], which tracks property-merge conflicts during
//! canonicalization. This type tracks **resolution conflicts**: when two or more
//! projection contracts tie in score and the resolver must fall back to
//! `DefaultProjection` while materializing the conflict as special properties.
//!
//! The two types live in different modules and are **not** interchangeable.

use crate::value_objects::Uuid;
use serde::{Deserialize, Serialize};

/// A conflict arising from the projection resolution algorithm.
///
/// When two or more [`super::contract::ProjectionContract`] candidates tie in score
/// (and in priority tie-break), the resolver falls back to `DefaultProjection`
/// and records the conflict here. The conflict includes the candidate IDs and
/// a human-readable reason string.
///
/// This is distinct from [`crate::canonicalization::ProjectionConflict`] (patch-level).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectionConflict {
    /// Human-readable reason for the conflict.
    pub reason: String,

    /// IDs of all contracts that tied in score/priority.
    pub candidates: Vec<super::contract::ProjectionContractId>,

    /// The winning contract ID, if one could be determined.
    /// In V1 this is always `None` — the algorithm falls back to Default
    /// rather than picking a winner.
    pub winner: Option<super::contract::ProjectionContractId>,

    /// The block ID this conflict pertains to.
    pub block_id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::contract::ProjectionContractId;

    #[test]
    fn projection_conflict_constructs() {
        let conflict = ProjectionConflict {
            reason: "tied score: 2 contracts".into(),
            candidates: vec![
                ProjectionContractId::new("task").unwrap(),
                ProjectionContractId::new("media").unwrap(),
            ],
            winner: None,
            block_id: Uuid::new_v4(),
        };
        assert_eq!(conflict.candidates.len(), 2);
        assert!(conflict.winner.is_none());
    }

    #[test]
    fn projection_conflict_serializes() {
        let conflict = ProjectionConflict {
            reason: "tied score: 2 contracts".into(),
            candidates: vec![
                ProjectionContractId::new("task").unwrap(),
                ProjectionContractId::new("media").unwrap(),
            ],
            winner: Some(ProjectionContractId::new("task").unwrap()),
            block_id: Uuid::new_v4(),
        };
        let json = serde_json::to_string(&conflict).expect("serialize");
        let parsed: ProjectionConflict = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(conflict, parsed);
    }

    #[test]
    fn distinct_from_patch_level_via_path() {
        // Both types are named ProjectionConflict but live in different modules.
        // Using explicit paths, they can coexist without collision.
        use crate::canonicalization::ProjectionConflict as PatchLevel;
        use crate::projection::ProjectionConflict as LayerLevel;

        // Patch-level has: property, existing_value, attempted_value, policy, reason
        let _patch = PatchLevel {
            property: crate::entities::PropertyKey::new("type").unwrap(),
            existing_value: crate::value_objects::PropertyValue::string("a"),
            attempted_value: crate::value_objects::PropertyValue::string("b"),
            policy: crate::properties::types::MergePolicy::Overwrite,
            reason: "patch conflict".into(),
        };

        // Layer-level has: reason, candidates, winner, block_id
        let _layer = LayerLevel {
            reason: "layer conflict".into(),
            candidates: vec![],
            winner: None,
            block_id: Uuid::new_v4(),
        };
    }
}
