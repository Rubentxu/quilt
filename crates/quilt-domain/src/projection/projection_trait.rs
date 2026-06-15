//! Projection trait and context — port interface for projection adapters.
//!
//! The [`Projection`] trait is the **port** (in Hexagonal Architecture terms) that
//! the application layer implements for each V1 contract (Task, Media, Heading, etc.).
//! The domain defines the trait; the application layer provides the implementations.
//!
//! [`ProjectionContext`] carries runtime information available during resolution.

use crate::entities::Block;
use crate::projection::contract::ProjectionContractId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Context passed to [`Projection::apply`].
///
/// Carries runtime information that projections may need to compute
/// their deltas (e.g. the current time for date-based decorations).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectionContext {
    /// Which view is currently active (e.g. `"page"`, `"slide"`).
    pub active_view: String,
    /// Current timestamp — injected by the resolver so tests can be deterministic.
    pub now: DateTime<Utc>,
}

impl ProjectionContext {
    /// Build a context with the page view and current time.
    #[must_use]
    pub fn new(active_view: impl Into<String>, now: DateTime<Utc>) -> Self {
        Self {
            active_view: active_view.into(),
            now,
        }
    }

    /// Build a context for the page view with the current wall-clock time.
    #[must_use]
    pub fn page(now: DateTime<Utc>) -> Self {
        Self::new("page", now)
    }
}

/// The projection port — implemented by each V1 contract's adapter.
///
/// Each implementation represents one visual projection of a block
/// (task checkbox, media preview, heading anchor, etc.).
///
/// # Object safety
///
/// This trait is **object-safe** (`Send + Sync + 'static` bounds)
/// and can be stored in a `Box<dyn Projection>`.
pub trait Projection: Send + Sync + 'static {
    /// The contract ID this projection implements.
    fn contract_id(&self) -> ProjectionContractId;

    /// Apply this projection to a block, returning an additive delta.
    ///
    /// The delta carries ONLY decorations, derived properties, and conflicts.
    /// The base surface (text, links, children) comes from the block itself.
    fn apply(
        &self,
        block: &Block,
        ctx: &ProjectionContext,
    ) -> super::view::ProjectionViewDelta;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{Block, PropertyKey};
    use crate::projection::view::{Decoration, DecorationKind, ProjectionViewDelta};
    use crate::value_objects::{PropertyValue, Uuid};
    use chrono::Utc;
    use std::collections::HashMap;

    // A minimal concrete Projection for testing object safety.
    struct DummyProjection {
        id: ProjectionContractId,
    }

    impl Projection for DummyProjection {
        fn contract_id(&self) -> ProjectionContractId {
            self.id.clone()
        }

        fn apply(
            &self,
            _block: &Block,
            _ctx: &ProjectionContext,
        ) -> ProjectionViewDelta {
            ProjectionViewDelta::default()
        }
    }

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
            content: "test".into(),
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
    fn projection_trait_has_exactly_two_methods() {
        // Document that Projection has exactly 2 required methods
        fn _check_trait<T: Projection>() {}

        struct TwoMethodProjection {
            id: ProjectionContractId,
        }

        impl Projection for TwoMethodProjection {
            fn contract_id(&self) -> ProjectionContractId {
                self.id.clone()
            }

            fn apply(
                &self,
                _block: &Block,
                _ctx: &ProjectionContext,
            ) -> ProjectionViewDelta {
                ProjectionViewDelta::default()
            }
        }

        // This compiles: TwoMethodProjection implements Projection with exactly 2 methods
        _check_trait::<TwoMethodProjection>();
    }

    #[test]
    fn projection_is_object_safe() {
        // Verify that Box<dyn Projection> compiles — the trait is object-safe
        let _boxed: Box<dyn Projection> = Box::new(DummyProjection {
            id: ProjectionContractId::new("test").unwrap(),
        });

        // Can call both methods through the trait object
        let block = make_block();
        let ctx = ProjectionContext::page(Utc::now());
        let delta = _boxed.apply(&block, &ctx);
        assert_eq!(delta.decorations.len(), 0);
    }

    #[test]
    fn projection_context_new() {
        let now = Utc::now();
        let ctx = ProjectionContext::new("slide", now);
        assert_eq!(ctx.active_view, "slide");
        assert_eq!(ctx.now, now);
    }

    #[test]
    fn projection_context_page() {
        let now = Utc::now();
        let ctx = ProjectionContext::page(now);
        assert_eq!(ctx.active_view, "page");
        assert_eq!(ctx.now, now);
    }
}
