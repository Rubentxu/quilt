//! Repository traits — abstractions for data access

mod block_repository;
mod page_repository;
mod ref_repository;
mod tag_repository;

pub use block_repository::{BlockRepository, BlockRepositoryExt};
pub use page_repository::{PageRepository, PageRepositoryExt};
pub use ref_repository::{RefRepository, RefRow};
pub use tag_repository::{TagRepository, TagRepositoryExt};
