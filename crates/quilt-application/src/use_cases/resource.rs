//! Resource use cases
//!
//! Implements [`ResourceUseCases`] trait for graph snapshots and summaries.
//!
//! These use cases provide structured data for presentation layers (MCP, REST).
//! Unlike the MCP server which returns JSON strings, these return typed structs.

use crate::errors::ApplicationError;
use async_trait::async_trait;
use quilt_domain::entities::Page;
use quilt_domain::repositories::{BlockRepository, PageRepository, TagRepository};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::instrument;

/// Resource use cases trait - graph snapshots and summaries.
///
/// This trait is object-safe (`Send + Sync`) and uses `#[async_trait]`
/// for async ergonomics.
#[async_trait]
pub trait ResourceUseCases: Send + Sync {
    /// Get a snapshot of the entire graph with statistics.
    async fn graph_snapshot(&self) -> Result<GraphSnapshot, ApplicationError>;

    /// List all pages with summaries.
    async fn list_pages(&self) -> Result<Vec<PageSummary>, ApplicationError>;

    /// List all journal pages with summaries.
    async fn list_journals(&self) -> Result<Vec<JournalSummary>, ApplicationError>;

    /// List all tags with usage counts.
    async fn list_tags(&self) -> Result<Vec<TagSummary>, ApplicationError>;
}

/// Graph snapshot containing aggregate statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSnapshot {
    /// Total number of pages
    pub pages_count: usize,
    /// Total number of journals
    pub journals_count: usize,
    /// Total number of blocks
    pub blocks_count: usize,
    /// Recent pages for display
    pub recent_pages: Vec<PageSummary>,
}

/// Summary of a page for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSummary {
    /// Page ID
    pub id: String,
    /// Page name
    pub name: String,
    /// Page title (may differ from name)
    pub title: Option<String>,
    /// Whether this is a journal page
    pub is_journal: bool,
}

impl From<Page> for PageSummary {
    fn from(page: Page) -> Self {
        Self {
            id: page.id.to_string(),
            name: page.name.clone(),
            title: page.title,
            is_journal: page.journal,
        }
    }
}

/// Summary of a journal page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalSummary {
    /// Page ID
    pub id: String,
    /// Journal name (date string)
    pub name: String,
    /// Journal day as YYYYMMDD integer
    pub journal_day: Option<i32>,
}

impl From<Page> for JournalSummary {
    fn from(page: Page) -> Self {
        Self {
            id: page.id.to_string(),
            name: page.name.clone(),
            journal_day: page.journal_day.map(|d| d.as_i32()),
        }
    }
}

/// Summary of a tag with usage count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagSummary {
    /// Tag name
    pub name: String,
    /// Number of times this tag is used
    pub usage_count: usize,
}

/// Implementation of [`ResourceUseCases`] for generic repository types.
///
/// Type parameters:
/// - `BR`: Block repository
/// - `PR`: Page repository
/// - `TR`: Tag repository
pub struct ResourceUseCasesImpl<BR: BlockRepository, PR: PageRepository, TR: TagRepository> {
    block_repo: Arc<BR>,
    page_repo: Arc<PR>,
    tag_repo: Arc<TR>,
}

impl<BR: BlockRepository, PR: PageRepository, TR: TagRepository> ResourceUseCasesImpl<BR, PR, TR> {
    /// Create a new ResourceUseCasesImpl instance.
    pub fn new(block_repo: Arc<BR>, page_repo: Arc<PR>, tag_repo: Arc<TR>) -> Self {
        Self {
            block_repo,
            page_repo,
            tag_repo,
        }
    }
}

#[async_trait]
impl<BR: BlockRepository + 'static, PR: PageRepository + 'static, TR: TagRepository + 'static>
    ResourceUseCases for ResourceUseCasesImpl<BR, PR, TR>
{
    #[instrument(skip(self))]
    async fn graph_snapshot(&self) -> Result<GraphSnapshot, ApplicationError> {
        // Get all pages
        let pages = self
            .page_repo
            .get_all()
            .await
            .map_err(ApplicationError::Domain)?;

        let pages_count = pages.len();
        let journals_count = pages.iter().filter(|p| p.journal).count();

        // Count all blocks in a single query
        let blocks_count = self
            .block_repo
            .count_all()
            .await
            .map_err(ApplicationError::Domain)?;

        // Get recent pages (last 10)
        let recent_pages = self
            .page_repo
            .get_recent(10)
            .await
            .map_err(ApplicationError::Domain)?
            .into_iter()
            .map(PageSummary::from)
            .collect();

        Ok(GraphSnapshot {
            pages_count,
            journals_count,
            blocks_count,
            recent_pages,
        })
    }

    #[instrument(skip(self))]
    async fn list_pages(&self) -> Result<Vec<PageSummary>, ApplicationError> {
        let pages = self
            .page_repo
            .get_all()
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(pages.into_iter().map(PageSummary::from).collect())
    }

    #[instrument(skip(self))]
    async fn list_journals(&self) -> Result<Vec<JournalSummary>, ApplicationError> {
        let pages = self
            .page_repo
            .get_all()
            .await
            .map_err(ApplicationError::Domain)?;

        let journals: Vec<JournalSummary> = pages
            .into_iter()
            .filter(|p| p.journal)
            .map(JournalSummary::from)
            .collect();

        Ok(journals)
    }

    #[instrument(skip(self))]
    async fn list_tags(&self) -> Result<Vec<TagSummary>, ApplicationError> {
        let tag_counts = self
            .tag_repo
            .get_tag_counts()
            .await
            .map_err(ApplicationError::Domain)?;

        let tags: Vec<TagSummary> = tag_counts
            .into_iter()
            .map(|(name, count)| TagSummary {
                name,
                usage_count: count,
            })
            .collect();

        Ok(tags)
    }
}
