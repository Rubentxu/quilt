//! Reference-related HTTP handlers (Q028: Editable Backlinks).
//!
//! Backlinks are derived (read-only) — but per the ROADMAP Q028 remedy,
//! each individual reference carries a user-editable `context` override
//! that the Backlinks panel surfaces. This module owns the
//! `PUT /api/v1/references/:blockId` endpoint that writes that override.
//!
//! # Why a separate top-level resource
//!
//! References are addressed by `(source_block_id, target_page_name)`,
//! not by `page_name` like most other resources. Mounting under
//! `/api/v1/references/...` keeps the URL shape aligned with the
//! mental model the Backlinks panel uses: "edit the snippet for this
//! specific block" rather than "edit a page's backlinks list".

use axum::{
    Json, Router,
    extract::{Extension, Path, Query},
    routing::put,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::instrument;

use crate::error::AppError;
use crate::handlers::pages::BacklinkDto;
use quilt_domain::references::RefType;
use quilt_domain::repositories::{BlockRepository, PageRepository, RefRepository};
use quilt_domain::value_objects::Uuid;

/// Query string for the editable-context endpoint.
///
/// `targetPage` is required because a single source block can carry
/// multiple outgoing references (`[[Page1]]`, `[[Page2]]`, `((uuid))`),
/// and we need to know which one is being edited. We use the page NAME
/// (not UUID) because the Backlinks panel already knows it: the
/// current page being viewed.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PutReferenceContextQuery {
    pub target_page: Option<String>,
}

/// Body of the PUT endpoint.
///
/// `context` semantics:
/// - `Some("...")` — set the custom snippet (empty string is valid:
///   it explicitly blanks out the default).
/// - `None` — clear the override; the panel falls back to the source
///   block's content snippet.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PutReferenceContextRequest {
    pub context: Option<String>,
}

/// Create router for `/api/v1/references`.
pub fn routes() -> Router {
    Router::new().route("/:blockId", put(put_reference_context))
}

/// PUT /api/v1/references/:blockId?targetPage=<name>
///
/// Set or clear the user-edited context override for a single
/// reference. Q028 (Editable Backlinks).
///
/// # URL shape
///
/// `:blockId` is the UUID of the **source** block (the block whose
/// content contains the `[[target]]` link). The `?targetPage=`
/// query parameter names the target page (the page the user is
/// currently viewing in the Backlinks panel).
///
/// # Status codes
///
/// - `200 OK` — the override was set or cleared. The response is
///   the updated `BacklinkDto` (the Backlinks panel can drop the
///   result straight into its list state).
/// - `400 Bad Request` — `:blockId` is not a valid UUID, or the
///   `targetPage` query parameter is missing/empty.
/// - `401 Unauthorized` — no Bearer token (handled by the auth
///   middleware before this handler runs).
/// - `404 Not Found` — the source block does not exist, the target
///   page does not exist, or there is no reference from the source
///   block to the target page.
///
/// # Examples
///
/// ```text
/// PUT /api/v1/references/abc-...?targetPage=My%20Page
/// { "context": "A meaningful snippet" }
/// ```
#[instrument(skip(payload, page_repo, block_repo, ref_repo))]
pub async fn put_reference_context(
    Path(block_id): Path<String>,
    Query(query): Query<PutReferenceContextQuery>,
    Extension(page_repo): Extension<Arc<dyn PageRepository>>,
    Extension(block_repo): Extension<Arc<dyn BlockRepository>>,
    Extension(ref_repo): Extension<Arc<dyn RefRepository>>,
    Json(payload): Json<PutReferenceContextRequest>,
) -> Result<Json<BacklinkDto>, AppError> {
    // ---- Validate inputs (400) ------------------------------------
    let source_uuid = Uuid::parse_str(&block_id)
        .map_err(|_| AppError::BadRequest(format!("Invalid source block UUID: {}", block_id)))?;

    let target_page_name = query
        .target_page
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            AppError::BadRequest("Missing or empty 'targetPage' query parameter".to_string())
        })?
        .to_string();

    // Optional sanity cap: a context of several MB would DoS the DB
    // and the panel. The cap is generous enough for the longest
    // reasonable hand-written snippet (a few thousand characters).
    if let Some(ctx) = payload.context.as_deref()
        && ctx.len() > 8 * 1024
    {
        return Err(AppError::BadRequest(format!(
            "Context too long: {} bytes (max 8192)",
            ctx.len()
        )));
    }

    // ---- Verify target + source exist -----------------------------
    let target_page = page_repo
        .get_by_name(&target_page_name)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| {
            AppError::NotFound(format!("Target page not found: {}", target_page_name))
        })?;

    let source_block = block_repo
        .get_by_id(source_uuid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Source block not found: {}", block_id)))?;

    // ---- Write the override ---------------------------------------
    // Q028: page refs are the only kind that appear in the Backlinks
    // panel — the panel is rendered from the page-level
    // `get_page_backlinks` endpoint which only emits page_refs. We
    // still validate against `RefType::PageRef` so a malformed
    // client can't set a context on, say, a tag ref that the panel
    // would never see.
    let updated = ref_repo
        .set_custom_context(
            source_uuid,
            target_page.id,
            RefType::PageRef,
            payload.context.as_deref(),
        )
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if !updated {
        // The repo returns false when no row matches the key. This
        // happens when the source block has no ref to this target —
        // the user is trying to edit a link that doesn't exist.
        return Err(AppError::NotFound(format!(
            "No reference from block {} to page '{}'",
            block_id, target_page_name
        )));
    }

    // ---- Build the response DTO -----------------------------------
    // Reuse the same shape the GET endpoint returns so the panel
    // can drop it straight into its list state.
    let source_page_name = page_repo
        .get_by_id(source_block.page_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .map(|p| p.name)
        .unwrap_or_else(|| "unknown".to_string());

    let plain_text = source_block.content.clone();
    let content_preview = if plain_text.len() > 100 {
        format!("{}...", &plain_text[..100])
    } else {
        plain_text
    };

    // The override that was just written. When the client sent
    // `context: null` we treat that as "clear" — the response DTO
    // falls back to the source content snippet, matching what the
    // GET endpoint will return next time.
    let context = payload
        .context
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| content_preview.clone());

    Ok(Json(BacklinkDto {
        source_block_id: block_id,
        source_page_name,
        content_preview,
        context,
    }))
}
