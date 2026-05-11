//! Repository traits - abstractions for data access

mod block_repository;
mod block_summary_repository;
mod class_repository;
mod deep_link_repository;
mod file_repository;
mod page_repository;
mod property_repository;
mod scheduled_task_repository;
mod tag_repository;

pub use block_repository::{BlockRepository, BlockRepositoryExt};
pub use block_summary_repository::BlockSummaryRepository;
pub use class_repository::{ClassRepository, ClassRepositoryExt};
pub use deep_link_repository::{DeepLinkRepository, DeepLinkRepositoryExt};
pub use file_repository::{FileRepository, FileRepositoryExt};
pub use page_repository::{PageRepository, PageRepositoryExt};
pub use property_repository::{PropertyRepository, PropertyRepositoryExt};
pub use scheduled_task_repository::ScheduledTaskRepository;
pub use tag_repository::{TagRepository, TagRepositoryExt};
