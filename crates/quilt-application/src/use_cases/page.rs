//! Page use cases
//!
//! Implements [`PageUseCases`] trait for page listing and retrieval.

use crate::errors::ApplicationError;
use async_trait::async_trait;
use quilt_domain::entities::{Page, PageCreate};
use quilt_domain::repositories::{BlockRepository, PageRepository};
use quilt_domain::value_objects::{BlockFormat, JournalDay, Uuid};
use std::str::FromStr;
use std::sync::Arc;
use tracing::instrument;

/// Page use cases trait - page listing and retrieval operations.
///
/// This trait is object-safe (`Send + Sync`) and uses `#[async_trait]`
/// for async ergonomics.
#[async_trait]
pub trait PageUseCases: Send + Sync {
    /// Create a new page with the given name.
    async fn create(&self, name: &str, title: Option<&str>) -> Result<Page, ApplicationError>;

    /// List all pages.
    async fn list(&self) -> Result<Vec<Page>, ApplicationError>;

    /// Get a page with its blocks by page name.
    async fn get_blocks(&self, page_name: &str) -> Result<PageWithBlocks, ApplicationError>;

    /// Get or create a journal page for a given date.
    async fn get_or_create_journal(&self, date: &str) -> Result<Page, ApplicationError>;
}

/// Page with its blocks returned by [`PageUseCases::get_blocks`].
///
/// Note: Page and Block don't implement Serialize/Deserialize, so this is
/// primarily for internal use. Use the individual fields as needed.
#[derive(Debug, Clone)]
pub struct PageWithBlocks {
    /// The page
    pub page: Page,
    /// Blocks belonging to this page
    pub blocks: Vec<quilt_domain::entities::Block>,
}

/// Implementation of [`PageUseCases`] for generic repository types.
///
/// Type parameters:
/// - `PR`: Page repository
/// - `BR`: Block repository
pub struct PageUseCasesImpl<PR: PageRepository, BR: BlockRepository> {
    page_repo: Arc<PR>,
    block_repo: Arc<BR>,
}

impl<PR: PageRepository, BR: BlockRepository> PageUseCasesImpl<PR, BR> {
    /// Create a new PageUseCasesImpl instance.
    pub fn new(page_repo: Arc<PR>, block_repo: Arc<BR>) -> Self {
        Self {
            page_repo,
            block_repo,
        }
    }
}

#[async_trait]
impl<PR: PageRepository + 'static, BR: BlockRepository + 'static> PageUseCases
    for PageUseCasesImpl<PR, BR>
{
    #[instrument(skip(self))]
    async fn create(&self, name: &str, title: Option<&str>) -> Result<Page, ApplicationError> {
        let page_create = PageCreate {
            name: name.to_string(),
            title: title.map(String::from),
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
            properties: std::collections::HashMap::new(),
        };

        let page = Page::new(page_create).map_err(ApplicationError::Domain)?;

        self.page_repo
            .insert(&page)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(page)
    }

    #[instrument(skip(self))]
    async fn list(&self) -> Result<Vec<Page>, ApplicationError> {
        self.page_repo
            .get_all()
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn get_blocks(&self, page_name: &str) -> Result<PageWithBlocks, ApplicationError> {
        // Find page by name
        let page = self
            .page_repo
            .get_by_name(page_name)
            .await
            .map_err(ApplicationError::Domain)?
            .ok_or_else(|| ApplicationError::NotFound("Page", Uuid::nil()))?;

        // Get blocks for this page
        let blocks = self
            .block_repo
            .get_by_page(page.id)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(PageWithBlocks { page, blocks })
    }

    #[instrument(skip(self))]
    async fn get_or_create_journal(&self, date: &str) -> Result<Page, ApplicationError> {
        // Parse the date string
        let day = JournalDay::from_str(date).map_err(ApplicationError::Domain)?;

        // Try to get existing journal
        if let Some(page) = self
            .page_repo
            .get_journal(day)
            .await
            .map_err(ApplicationError::Domain)?
        {
            return Ok(page);
        }

        // Create new journal page
        let page =
            Page::new_journal(day, BlockFormat::Markdown, "%Y-%m-%d").map_err(ApplicationError::Domain)?;

        self.page_repo
            .insert(&page)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(page)
    }
}
