//! Default projection — universal fallback with no predicates.
//!
//! The [`DefaultProjection`] is a ZST (zero-sized type) that always matches
//! every block because it has no predicates. Its priority is `u32::MAX`,
//! which makes it the lowest-priority candidate in normal cases.
//!
//! When all other candidates tie in score, the resolver falls back to
//! `DefaultProjection` and materializes a conflict.

use crate::entities::Block;
use crate::projection::contract::ProjectionContractId;
use crate::projection::projection_trait::{Projection, ProjectionContext};
use crate::projection::view::ProjectionViewDelta;

/// The default / universal fallback projection.
///
/// Always applicable — no predicates, no guard, no score override.
/// Priority is `u32::MAX` so it loses to any specialized contract
/// with a non-default priority in normal single-winner resolution.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct DefaultProjection;

impl DefaultProjection {
    /// Construct a new `DefaultProjection`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Projection for DefaultProjection {
    fn contract_id(&self) -> ProjectionContractId {
        ProjectionContractId::new("default").expect("'default' is a valid contract ID")
    }

    fn apply(
        &self,
        _block: &Block,
        _ctx: &ProjectionContext,
    ) -> ProjectionViewDelta {
        // DefaultProjection produces no decorations — pure base surface only
        ProjectionViewDelta::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{Block, PropertyKey};
    use crate::projection::projection_trait::Projection;
    use crate::projection::view::ProjectionViewDelta;
    use crate::value_objects::{PropertyValue, Uuid};
    use chrono::Utc;
    use std::collections::HashMap;

    fn make_block() -> Block {
        Block {
            id: Uuid::new_v4(),
            page_id: Uuid::new_v4(),
            parent_id: None,
            order: 0.0,
            level: 1,
            format: crate::value_objects::BlockFormat::Markdown,
            block_type: crate::value_objects::BlockType::Paragraph,
            marker: None,
            priority: None,
            content: "Hello world".into(),
            properties: HashMap::new(),
            refs: vec![],
            tags: vec![],
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
            logbook: None,
            completed_at: None,
            cancelled_at: None,
            collapsed: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn default_projection_size_of_is_zero() {
        assert_eq!(std::mem::size_of::<DefaultProjection>(), 0);
    }

    #[test]
    fn default_projection_is_copy() {
        // ZSTs are always Copy
        fn _assert_copy<T: Copy>() {}
        _assert_copy::<DefaultProjection>();
    }

    #[test]
    fn default_projection_default_equals_new() {
        assert_eq!(DefaultProjection::default(), DefaultProjection::new());
    }

    #[test]
    fn default_contract_id_returns_default() {
        let p = DefaultProjection::new();
        assert_eq!(p.contract_id().as_str(), "default");
    }

    #[test]
    fn default_apply_returns_empty_delta_regardless_of_block() {
        let p = DefaultProjection::new();
        let block = make_block();
        let ctx = ProjectionContext::page(Utc::now());

        let delta = p.apply(&block, &ctx);

        assert!(delta.decorations.is_empty());
        assert!(delta.view_properties.is_empty());
        assert!(delta.conflicts.is_empty());
    }

    #[test]
    fn default_apply_does_not_depend_on_now() {
        let p = DefaultProjection::new();
        let block = make_block();

        let ctx1 = ProjectionContext::page(Utc::now());
        let ctx2 = ProjectionContext::page(Utc::now());

        let delta1 = p.apply(&block, &ctx1);
        let delta2 = p.apply(&block, &ctx2);

        assert_eq!(delta1, delta2);
    }
}
