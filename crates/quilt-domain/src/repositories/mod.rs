//! Repository traits — abstractions for data access

mod block_repository;
mod page_repository;
mod property_repository;
mod ref_repository;
mod schema_repository;
mod settings_repository;
mod tag_repository;
mod tour_state_repository;

pub use block_repository::{BlockRepository, BlockRepositoryExt};
pub use page_repository::{PageRepository, PageRepositoryExt};
pub use property_repository::{PropertyRepository, PropertyRepositoryExt};
pub use ref_repository::{RefRepository, RefRow};
pub use schema_repository::SchemaRepository;
pub use settings_repository::SettingsRepository;
pub use tag_repository::{TagRepository, TagRepositoryExt};
pub use tour_state_repository::TourStateRepository;
