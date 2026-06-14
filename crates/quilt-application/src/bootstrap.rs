//! Composition root — bundles all use cases for presentation layers.
//!
//! Presentation layers (MCP, CLI, REST) use `AppServices` to access
//! all use cases through a single struct.

use crate::use_cases::{
    BlockUseCases, PageUseCases, ResourceUseCases, SearchUseCases, TemplateUseCases, TourStateUseCases,
};
use std::sync::Arc;

/// Wired application services ready for use by presentation layers.
///
/// All use cases are stored as `Arc<dyn Trait>` for dynamic dispatch.
pub struct AppServices {
    /// Block use cases (create, delete, link, tree, backlinks)
    pub block: Arc<dyn BlockUseCases>,
    /// Page use cases (create, list, journal)
    pub page: Arc<dyn PageUseCases>,
    /// Search use cases (query, search)
    pub search: Arc<dyn SearchUseCases>,
    /// Resource use cases (graph snapshot, page/tag info)
    pub resource: Arc<dyn ResourceUseCases>,
    /// Template use cases (list templates, get schema)
    pub template: Arc<dyn TemplateUseCases>,
    /// Tour state use cases (get/dismiss tours)
    pub tour_state: Arc<dyn TourStateUseCases>,
}

impl AppServices {
    /// Create a new AppServices instance by bundling pre-built use cases.
    ///
    /// This is the single composition root for the application.
    /// Presentation layers construct use cases from infrastructure (pool + repos)
    /// and pass them here for standardized grouping.
    pub fn new(
        block: Arc<dyn BlockUseCases>,
        page: Arc<dyn PageUseCases>,
        search: Arc<dyn SearchUseCases>,
        resource: Arc<dyn ResourceUseCases>,
        template: Arc<dyn TemplateUseCases>,
        tour_state: Arc<dyn TourStateUseCases>,
    ) -> Self {
        Self {
            block,
            page,
            search,
            resource,
            template,
            tour_state,
        }
    }
}
