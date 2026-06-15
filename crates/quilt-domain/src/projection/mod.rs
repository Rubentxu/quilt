//! Projection domain types — declarative contracts for block → view resolution.
//!
//! # Module map
//!
//! | File | Contents |
//! |------|----------|
//! | `predicate.rs` | `PropertyPredicate` enum — declarative match conditions |
//! | `contract.rs` | `ProjectionContract`, `ProjectionContractId` |
//! | `conflict.rs` | `ProjectionConflict` (projection-level, distinct from patch-level) |
//! | `view.rs` | `ProjectionView`, `Decoration`, `LinkView`, `ProjectionViewDelta` |
//! | `projection_trait.rs` | `Projection` trait, `ProjectionContext` |
//! | `default.rs` | `DefaultProjection` (ZST universal fallback) |
//! | `registry.rs` | `ProjectionRegistry` port trait, `RegisteredProjection` |

pub mod conflict;
pub mod contract;
pub mod default;
pub mod predicate;
pub mod projection_trait;
pub mod registry;
pub mod view;

// Re-exports for ergonomic public API
pub use conflict::ProjectionConflict;
pub use contract::{ContractIdError, ProjectionContract, ProjectionContractId};
pub use default::DefaultProjection;
pub use predicate::PropertyPredicate;
pub use projection_trait::{Projection, ProjectionContext};
pub use registry::{ProjectionRegistry, RegisteredProjection};
pub use view::{
    Decoration, DecorationKind, LinkKind, LinkView, ProjectionView, ProjectionViewBuilder,
    ProjectionViewDelta,
};
