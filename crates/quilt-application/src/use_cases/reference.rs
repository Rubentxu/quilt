//! Reference use cases (Q028 — Editable Backlinks)
//!
//! Wraps the Q028 custom-context override endpoint and the read-side
//! operations the Backlinks panel needs. The use case owns:
//!
//! - Validating inputs (UUID parse, target-page existence, source-block
//!   existence, max context length).
//! - The "no such reference" 404 distinction (the underlying repo
//!   returns `Ok(false)` when the row doesn't match).
//! - Building the response DTO the handler returns (source page name,
//!   content preview, context).
//!
//! Page-level backlinks (the `BlockUseCases::get_backlinks` style bulk
//! read) and unlinked-references live on `PageUseCases` because they
//! join across the `Page` table. This use case is the *edit* seam.

use crate::errors::ApplicationError;
use async_trait::async_trait;
use quilt_domain::references::RefType;
use quilt_domain::repositories::{BlockRepository, PageRepository, RefRepository};
use quilt_domain::value_objects::Uuid;
use std::sync::Arc;
use tracing::instrument;

/// Max length of a custom context string. 8 KiB is generous for a
/// hand-written snippet but rejects megabyte payloads that would
/// DoS the DB and the panel.
pub const MAX_CONTEXT_LEN: usize = 8 * 1024;

/// Returned by [`ReferenceUseCases::set_custom_context`] so the
/// HTTP handler can drop the value straight into the Backlinks
/// panel's list state.
#[derive(Debug, Clone)]
pub struct ReferenceContextUpdate {
    /// UUID of the source block (the block whose content contains the reference).
    pub source_block_id: Uuid,
    /// Name of the page that contains the source block.
    pub source_page_name: String,
    /// First 100 chars of the source block's content.
    pub content_preview: String,
    /// Context to render in the panel — falls back to `content_preview`
    /// when the override is empty/missing.
    pub context: String,
}

/// Reference use cases trait - Q028 editable backlinks and ref
/// operations that don't fit cleanly into `PageUseCases` (Q028 PUT
/// lives here because it operates on a single `(source, target)` row).
#[async_trait]
pub trait ReferenceUseCases: Send + Sync {
    /// Set or clear the user-edited context override for a single
    /// reference (Q028).
    ///
    /// # Validation
    /// - `block_id` must parse as a UUID.
    /// - `target_page_name` must be non-empty (after trim).
    /// - `context`, when provided, must be `MAX_CONTEXT_LEN` bytes or fewer.
    ///
    /// # Errors
    /// - `Validation` — invalid UUID, missing target page, oversized context.
    /// - `NotFound` — source block, target page, or the reference
    ///   row itself does not exist.
    async fn set_custom_context(
        &self,
        block_id: Uuid,
        target_page_name: &str,
        ref_type: RefType,
        context: Option<&str>,
    ) -> Result<ReferenceContextUpdate, ApplicationError>;

    /// Get the custom-context override for a single reference, if any.
    /// Returns `None` when the reference has no override.
    async fn get_custom_context(
        &self,
        block_id: Uuid,
        target_page_id: Uuid,
        ref_type: RefType,
    ) -> Result<Option<String>, ApplicationError>;
}

/// Implementation of [`ReferenceUseCases`] for any combination of
/// repositories.
pub struct ReferenceUseCasesImpl<BR: BlockRepository, PR: PageRepository, RR: RefRepository> {
    block_repo: Arc<BR>,
    page_repo: Arc<PR>,
    ref_repo: Arc<RR>,
}

impl<BR: BlockRepository, PR: PageRepository, RR: RefRepository>
    ReferenceUseCasesImpl<BR, PR, RR>
{
    /// Create a new use-case instance.
    pub fn new(block_repo: Arc<BR>, page_repo: Arc<PR>, ref_repo: Arc<RR>) -> Self {
        Self {
            block_repo,
            page_repo,
            ref_repo,
        }
    }
}

#[async_trait]
impl<BR: BlockRepository + 'static, PR: PageRepository + 'static, RR: RefRepository + 'static>
    ReferenceUseCases for ReferenceUseCasesImpl<BR, PR, RR>
{
    #[instrument(skip(self, context))]
    async fn set_custom_context(
        &self,
        block_id: Uuid,
        target_page_name: &str,
        ref_type: RefType,
        context: Option<&str>,
    ) -> Result<ReferenceContextUpdate, ApplicationError> {
        // 1. Validate context length (if provided)
        if let Some(ctx) = context {
            if ctx.len() > MAX_CONTEXT_LEN {
                return Err(ApplicationError::Validation(format!(
                    "Context too long: {} bytes (max {})",
                    ctx.len(),
                    MAX_CONTEXT_LEN
                )));
            }
        }

        // 2. Resolve target page (404 if missing)
        let target_page = self
            .page_repo
            .get_by_name(target_page_name)
            .await
            .map_err(ApplicationError::Domain)?
            .ok_or_else(|| {
                ApplicationError::Validation(format!("Target page not found: {}", target_page_name))
            })?;

        // 3. Resolve source block (404 if missing)
        let source_block = self
            .block_repo
            .get_by_id(block_id)
            .await
            .map_err(ApplicationError::Domain)?
            .ok_or_else(|| ApplicationError::NotFound("Block", block_id))?;

        // 4. Write the override. The repo returns `Ok(false)` when
        //    there is no `(source, target, ref_type)` row to update.
        let updated = self
            .ref_repo
            .set_custom_context(block_id, target_page.id, ref_type, context)
            .await
            .map_err(ApplicationError::Domain)?;

        if !updated {
            return Err(ApplicationError::Validation(format!(
                "No reference from block {} to page '{}'",
                block_id, target_page_name
            )));
        }

        // 5. Build the response DTO
        let source_page_name = self
            .page_repo
            .get_by_id(source_block.page_id)
            .await
            .map_err(ApplicationError::Domain)?
            .map(|p| p.name)
            .unwrap_or_else(|| "unknown".to_string());

        let plain_text = source_block.content;
        let content_preview = if plain_text.len() > 100 {
            format!("{}...", &plain_text[..100])
        } else {
            plain_text
        };

        // The override that was just written. When the client sent
        // `context: null` or an empty string we treat that as "clear"
        // — the response DTO falls back to the source content snippet.
        let context_value = context
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| content_preview.clone());

        Ok(ReferenceContextUpdate {
            source_block_id: block_id,
            source_page_name,
            content_preview,
            context: context_value,
        })
    }

    #[instrument(skip(self))]
    async fn get_custom_context(
        &self,
        block_id: Uuid,
        target_page_id: Uuid,
        ref_type: RefType,
    ) -> Result<Option<String>, ApplicationError> {
        self.ref_repo
            .get_custom_context(block_id, target_page_id, ref_type)
            .await
            .map_err(ApplicationError::Domain)
    }
}
