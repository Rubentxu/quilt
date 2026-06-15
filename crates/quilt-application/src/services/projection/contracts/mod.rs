//! V1 projection contracts — declarative block-to-view matching contracts.
//!
//! Each contract pairs a [`ProjectionContract`] (declarative predicates)
//! with a corresponding [`Projection`] adapter (logic that produces the delta).
//!
//! | Contract | Block signature | Decoration |
//! |----------|----------------|------------|
//! | `default` | (always matches) | — |
//! | `task` | `type:: task` + `status::` known | TaskCheckbox |
//! | `media` | `type:: media` + `media-type:: video|image` | MediaPreview |
//! | `heading` | `block-role:: heading` + `heading-level:: 1|2|3` | HeadingAnchor |
//! | `link` | `link::` set | LinkAffordance |
//! | `date` | `scheduled::` OR `deadline::` set | DateIndicator |

pub mod date;
pub mod default;
pub mod heading;
pub mod link;
pub mod media;
pub mod task;

// Re-export projection adapters for use in StaticProjectionRegistry
pub use date::DateProjection;
pub use default::DefaultProjection;
pub use heading::HeadingProjection;
pub use link::LinkProjection;
pub use media::MediaProjection;
pub use task::TaskProjection;
