//! Quilt Test Helpers
//!
//! In-memory repository wrappers and fixtures for testing.
//!
//! # Crate Layout
//!
//! - `in_memory/` — Arc-wrapped repository builders
//!   - [`InMemoryBlockRepo`] — block repo with `with_blocks()`, `with_page()`, `as_trait()`
//!   - [`InMemoryPageRepo`] — page repo with `with_pages()`, `as_trait()`
//!   - [`InMemoryTagRepo`] — tag repo with `with_tags()`, `as_trait()`
//! - `fixtures/` — test data factories
//!   - [`page_with_blocks`] — creates a `(Page, Vec<Block>)` pair with aligned IDs
//!
//! # Example
//!
//! ```ignore
//! use std::sync::Arc;
//! use quilt_domain::repositories::BlockRepository;
//! use quilt_test_helpers::{InMemoryBlockRepo, InMemoryPageRepo, page_with_blocks};
//!
//! // Create a page with some blocks
//! let (page, blocks) = page_with_blocks("Test Page", vec!["Block 1", "Block 2"]);
//!
//! // Build repos with data
//! let page_repo = InMemoryPageRepo::new()
//!     .with_pages(vec![page.clone()]);
//! let block_repo = InMemoryBlockRepo::new()
//!     .with_page(page.clone(), blocks);
//!
//! // Get trait objects for use in tests
//! let block_trait: Arc<dyn BlockRepository> = block_repo.as_trait();
//! ```

pub use fixtures::page_with_blocks;
pub use in_memory::{InMemoryBlockRepo, InMemoryPageRepo, InMemoryTagRepo};

mod fixtures;
mod in_memory;
