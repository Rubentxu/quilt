//! Use cases module
//!
//! Provides use case traits that define the contract between presentation layers
//! (MCP, REST, etc.) and the application layer.
//!
//! Each trait is object-safe (`Send + Sync`) and uses `#[async_trait]` for
//! async ergonomics. Implementations use generics internally for zero-cost
//! abstraction through monomorphization.
//!
//! # Design Decisions
//!
//! - **Name→id resolution** stays in the application layer — presentation
//!   layers pass `page_name: &str`, and use cases resolve to `page_id` internally.
//! - **ResourceUseCases** exists even though REST isn't built yet — common by design.
//! - **Dead cognitive code** — `quilt-analysis` is NOT referenced in any use case code.

pub mod annotation;
pub mod apply_preset;
pub mod block;
pub mod dtos;
pub mod migration;
pub mod page;
pub mod projection_resolver;
pub mod resource;
pub mod search;
pub mod template;
pub mod tour_state;

// Re-exports for convenience
pub use annotation::{AnnotationUseCases, AnnotationUseCasesImpl};
pub use apply_preset::ApplyPreset;
pub use block::{BlockTree, BlockUseCases, BlockUseCasesImpl};
pub use dtos::{AnnotationDto, BlockDto};
pub use migration::MigrationUseCases;
pub use page::{PageUseCases, PageUseCasesImpl, PageWithBlocks};
pub use resource::{
    GraphSnapshot, JournalSummary, PageSummary, ResourceUseCases, ResourceUseCasesImpl, TagSummary,
};
pub use search::{QueryPlan, SearchResult, SearchUseCases, SearchUseCasesImpl};
pub use template::{TemplateSchema, TemplateSummary, TemplateUseCases, TemplateUseCasesImpl};
pub use tour_state::{TourStateUseCases, TourStateUseCasesImpl};
