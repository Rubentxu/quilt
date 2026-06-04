//! Page-related HTTP handlers

use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    Json,
};
use axum::{routing::get, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::instrument;

use crate::error::AppError;
use crate::state::AppState;
use quilt_domain::entities::{Block, BlockCreate, Page, PageCreate};
use quilt_domain::repositories::{BlockRepository, PageRepository, SettingsRepository};
use quilt_domain::value_objects::{BlockFormat, JournalDay, Uuid};
use quilt_infrastructure::database::sqlite::repositories::{
    SqliteBlockRepository, SqlitePageRepository,
};

/// A page returned to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageDto {
    pub id: String,
    pub name: String,
    pub title: Option<String>,
    pub journal: bool,
    pub journal_day: Option<i64>,
    pub created_at: String,
}

impl From<Page> for PageDto {
    fn from(page: Page) -> Self {
        Self {
            id: page.id.to_string(),
            name: page.name,
            title: page.title,
            journal: page.journal,
            journal_day: page.journal_day.map(|d| d.as_i32() as i64),
            created_at: page.created_at.to_rfc3339(),
        }
    }
}

/// Create page request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePageRequest {
    pub name: String,
    pub title: Option<String>,
    /// Mark this page as a journal/daily-note page
    pub is_journal: Option<bool>,
    /// Journal day in YYYYMMDD format (derived from name if not provided)
    pub journal_day: Option<String>,
}

/// GET /api/v1/pages/:name/unlinked-references
///
/// Returns all blocks whose content text mentions the page name (case-insensitive)
/// but do NOT have an explicit `[[page]]` reference.
#[instrument(skip(state))]
pub async fn get_page_unlinked_references(
    Path(name): Path<String>,
    Extension(state): Extension<AppState>,
) -> Result<Json<Vec<BacklinkDto>>, AppError> {
    let page_repo = SqlitePageRepository::new(state.pool.clone());
    let block_repo =
        quilt_infrastructure::database::sqlite::repositories::SqliteBlockRepository::new(
            state.pool.clone(),
        );

    // Look up the page by name
    let page = page_repo
        .get_by_name(&name)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Page not found: {}", name)))?;

    // Query unlinked references via the ref service
    let unlinked = {
        let ref_service = state.ref_service.read().await;
        ref_service
            .get_page_unlinked_references(&name, page.id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
    };

    if unlinked.is_empty() {
        return Ok(Json(Vec::new()));
    }

    // Enrich with source page names and content previews
    let mut dtos = Vec::with_capacity(unlinked.len());
    for (source_block_id, source_page_id, content_snippet) in &unlinked {
        let source_page_name = page_repo
            .get_by_id(*source_page_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .map(|p| p.name)
            .unwrap_or_else(|| "unknown".to_string());

        let block = block_repo
            .get_by_id(*source_block_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let content_preview = if let Some(source_block) = block {
            let plain_text = source_block.content;
            if plain_text.len() > 100 {
                format!("{}...", &plain_text[..100])
            } else {
                plain_text
            }
        } else {
            content_snippet.clone()
        };

        dtos.push(BacklinkDto {
            source_block_id: source_block_id.to_string(),
            source_page_name,
            content_preview,
        });
    }

    Ok(Json(dtos))
}

/// Create router for /api/v1/pages
pub fn routes() -> Router {
    Router::new()
        .route("/", get(list_pages).post(create_page))
        // NOTE: `/from-template` is a literal segment and must be registered
        // BEFORE the `/:name` catch-all so that axum does not route the
        // literal `from-template` string to the page-by-name handler.
        .route("/from-template", axum::routing::post(create_page_from_template))
        .route("/journal/:date", get(get_journal))
        .route("/:name", get(get_page))
        .route("/:name/blocks", get(get_page_blocks))
        .route("/:name/backlinks", get(get_page_backlinks))
        .route(
            "/:name/unlinked-references",
            get(get_page_unlinked_references),
        )
}

/// GET /api/v1/pages
#[instrument(skip(state))]
pub async fn list_pages(
    Extension(state): Extension<AppState>,
) -> Result<Json<Vec<PageDto>>, AppError> {
    let page_repo = SqlitePageRepository::new(state.pool.clone());

    let pages = page_repo
        .get_all()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let dtos: Vec<PageDto> = pages.into_iter().map(PageDto::from).collect();

    Ok(Json(dtos))
}

/// GET /api/v1/pages/:name
#[instrument(skip(state))]
pub async fn get_page(
    Path(name): Path<String>,
    Extension(state): Extension<AppState>,
) -> Result<Json<PageDto>, AppError> {
    let page_repo = SqlitePageRepository::new(state.pool.clone());

    let page = page_repo
        .get_by_name(&name)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    match page {
        Some(p) => Ok(Json(PageDto::from(p))),
        None => Err(AppError::NotFound(format!("Page not found: {}", name))),
    }
}

/// POST /api/v1/pages
#[instrument(skip(state))]
pub async fn create_page(
    Extension(state): Extension<AppState>,
    Json(payload): Json<CreatePageRequest>,
) -> Result<(StatusCode, Json<PageDto>), AppError> {
    let page_repo = SqlitePageRepository::new(state.pool.clone());

    if payload.is_journal.unwrap_or(false) {
        // Create journal page
        let day = if let Some(ref jd) = payload.journal_day {
            JournalDay::from_str(jd)
                .map_err(|e| AppError::BadRequest(format!("Invalid journal day: {}", e)))?
        } else {
            JournalDay::from_str(&payload.name)
                .map_err(|e| AppError::BadRequest(format!("Invalid journal date: {}", e)))?
        };

        let settings = state
            .settings_repo
            .get_user_settings()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let page = Page::new_journal(day, BlockFormat::Markdown, &settings.journal_format)
            .map_err(|e| AppError::Internal(e.to_string()))?;

        page_repo
            .insert(&page)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok((StatusCode::CREATED, Json(PageDto::from(page))))
    } else {
        // Create regular page
        let page = Page::new(PageCreate {
            name: payload.name,
            title: payload.title,
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
            properties: std::collections::HashMap::new(),
        })
        .map_err(|e| AppError::Internal(e.to_string()))?;

        page_repo
            .insert(&page)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok((StatusCode::CREATED, Json(PageDto::from(page))))
    }
}

/// GET /api/v1/pages/journal/:date
#[instrument(skip(state))]
pub async fn get_journal(
    Path(date): Path<String>,
    Extension(state): Extension<AppState>,
) -> Result<Json<PageDto>, AppError> {
    let page_repo = SqlitePageRepository::new(state.pool.clone());
    let day = JournalDay::from_str(&date)
        .map_err(|e| AppError::BadRequest(format!("Invalid journal date: {}", e)))?;

    let page = match page_repo.get_journal(day).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            // Read user's journal format setting
            let settings = state
                .settings_repo
                .get_user_settings()
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            let p = Page::new_journal(day, BlockFormat::Markdown, &settings.journal_format)
                .map_err(|e| AppError::Internal(e.to_string()))?;
            page_repo
                .insert(&p)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            p
        }
        Err(e) => return Err(AppError::Internal(e.to_string())),
    };

    Ok(Json(PageDto::from(page)))
}

/// A backlink DTO returned by the page backlinks endpoint
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacklinkDto {
    /// UUID of the source block (the block that contains the reference)
    pub source_block_id: String,
    /// Name of the page that contains the source block
    pub source_page_name: String,
    /// Content preview of the source block (first ~100 chars)
    pub content_preview: String,
}

/// GET /api/v1/pages/:name/backlinks
///
/// Returns all blocks that reference the given page.
/// Uses the in-memory RefIndex for O(1) lookup.
#[instrument(skip(state))]
pub async fn get_page_backlinks(
    Path(name): Path<String>,
    Extension(state): Extension<AppState>,
) -> Result<Json<Vec<BacklinkDto>>, AppError> {
    let page_repo = SqlitePageRepository::new(state.pool.clone());
    let block_repo =
        quilt_infrastructure::database::sqlite::repositories::SqliteBlockRepository::new(
            state.pool.clone(),
        );

    // Look up the page by name
    let page = page_repo
        .get_by_name(&name)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Page not found: {}", name)))?;

    // Query the in-memory ref index for O(1) backlinks
    let backlinks = {
        let ref_service = state.ref_service.read().await;
        ref_service.get_backlinks(page.id)
    };

    if backlinks.is_empty() {
        return Ok(Json(Vec::new()));
    }

    // Enrich backlinks with source block content and page names
    let mut dtos = Vec::with_capacity(backlinks.len());
    for (source_id, _ref_type) in &backlinks {
        let block = block_repo
            .get_by_id(*source_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        if let Some(source_block) = block {
            let source_page_name = page_repo
                .get_by_id(source_block.page_id)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
                .map(|p| p.name)
                .unwrap_or_else(|| "unknown".to_string());

            let plain_text = source_block.content;
            let content_preview = if plain_text.len() > 100 {
                format!("{}...", &plain_text[..100])
            } else {
                plain_text
            };

            dtos.push(BacklinkDto {
                source_block_id: source_id.to_string(),
                source_page_name,
                content_preview,
            });
        }
    }

    Ok(Json(dtos))
}

/// GET /api/v1/pages/:name/blocks
/// Returns all blocks for a page
#[instrument(skip(state))]
pub async fn get_page_blocks(
    Path(name): Path<String>,
    Extension(state): Extension<AppState>,
) -> Result<Json<Vec<crate::handlers::blocks::BlockDto>>, AppError> {
    let page_repo = SqlitePageRepository::new(state.pool.clone());
    let block_repo =
        quilt_infrastructure::database::sqlite::repositories::SqliteBlockRepository::new(
            state.pool.clone(),
        );

    // Find the page by name
    let page = page_repo
        .get_by_name(&name)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Page not found: {}", name)))?;

    // Get all blocks for this page
    let blocks = block_repo
        .get_by_page(page.id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let block_dtos: Vec<crate::handlers::blocks::BlockDto> = blocks
        .into_iter()
        .map(|b| (b, Some(page.name.clone())).into())
        .collect();

    Ok(Json(block_dtos))
}

// ════════════════════════════════════════════════════════════════════════
//  Templates (ADR-0003)
// ════════════════════════════════════════════════════════════════════════
//
// A template is a regular page whose name starts with `template/`. The
// `create_page_from_template` endpoint reads a template's block tree,
// substitutes `{{placeholder}}` variables, and clones those blocks into a
// brand new page — preserving parent/child structure, ordering, level,
// marker, and other block metadata.

/// Request body for `POST /api/v1/pages/from-template`.
///
/// `template_name` must reference an existing page whose name starts with
/// `template/` (e.g. `template/daily-note`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FromTemplateRequest {
    /// The template page name (must be a template).
    pub template_name: String,
    /// The new page's name.
    pub page_name: String,
    /// Optional display title for the new page.
    pub title: Option<String>,
    /// Optional user-provided variable substitutions.
    ///
    /// Variables in template block content use the `{{key}}` or `${key}`
    /// syntax. Keys are case-sensitive. Unknown placeholders are left
    /// intact in the cloned content (caller can then edit them).
    pub variables: Option<HashMap<String, String>>,
}

/// Response body for `POST /api/v1/pages/from-template`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FromTemplateResponse {
    /// The newly created page.
    pub page: PageDto,
    /// Number of blocks that were cloned from the template into the new page.
    pub blocks_created: usize,
}

/// `POST /api/v1/pages/from-template`
///
/// Creates a new page by cloning the block tree of a template page.
///
/// Workflow:
/// 1. Look up the template page by name (404 if missing).
/// 2. Verify the page is actually a template (name starts with `template/`)
///    — reject regular pages with 400 BAD_REQUEST.
/// 3. Create the destination page (`page_name`).
/// 4. Load the template's blocks.
/// 5. Substitute `{{var}}` / `${var}` placeholders in every block's content
///    using built-in variables (`{{title}}`, `{{name}}`, `{{date}}`) and any
///    user-supplied `variables`.
/// 6. Clone the blocks with fresh UUIDs in two passes: first insert with
///    `parent_id = None` (root level), then update `parent_id` to point at
///    the corresponding new block. The two-pass design avoids needing the
///    parent UUID before it has been generated.
#[instrument(skip(state, req), fields(template_name = %req.template_name, page_name = %req.page_name))]
pub async fn create_page_from_template(
    Extension(state): Extension<AppState>,
    Json(req): Json<FromTemplateRequest>,
) -> Result<(StatusCode, Json<FromTemplateResponse>), AppError> {
    let page_repo = SqlitePageRepository::new(state.pool.clone());
    let block_repo = SqliteBlockRepository::new(state.pool.clone());

    // 1. Verify template exists
    let template = page_repo
        .get_by_name(&req.template_name)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| {
            AppError::NotFound(format!("Template not found: {}", req.template_name))
        })?;

    // 2. Verify it's a template (name starts with "template/" or is exactly "template")
    if !template.is_template() {
        return Err(AppError::BadRequest(format!(
            "Page is not a template: {} (names must start with 'template/')",
            req.template_name
        )));
    }

    // 3. Create the new page
    let title = req.title.clone().or_else(|| Some(req.page_name.clone()));
    let new_page = Page::new(PageCreate {
        name: req.page_name.clone(),
        title,
        namespace_id: None,
        journal_day: None,
        format: BlockFormat::Markdown,
        file_id: None,
        properties: std::collections::HashMap::new(),
    })
    .map_err(|e| AppError::Internal(e.to_string()))?;

    page_repo
        .insert(&new_page)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // 4. Load template blocks (ordered by `order` so we preserve the
    //    visual sequence of the source page).
    let mut template_blocks = block_repo
        .get_by_page(template.id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    // Stable order so the parent pass works deterministically.
    template_blocks.sort_by(|a, b| {
        a.order
            .partial_cmp(&b.order)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // 5 + 6. Clone blocks. Two-pass approach:
    //   pass 1: insert new blocks with parent_id = None (root) and a new UUID.
    //           Record the mapping template_id -> new_id.
    //   pass 2: walk template blocks again, look up the parent's new id, and
    //           update the child's parent_id and level.
    //
    // Use the *normalized* page name (the value stored on the new page) for
    // placeholder substitution, not the raw user input — names are
    // lowercased and trimmed by `Page::normalize_name`, so `{{title}}`
    // should match what shows up in the UI for the new page.
    let normalized_name = new_page.name.clone();
    let mut id_map: HashMap<Uuid, Uuid> = HashMap::new();
    let mut blocks_created: usize = 0;

    for template_block in &template_blocks {
        let substituted = substitute_placeholders(
            &template_block.content,
            req.variables.as_ref(),
            &normalized_name,
        );

        let new_block = Block::new(BlockCreate {
            page_id: new_page.id,
            content: substituted,
            parent_id: None, // fixed in pass 2
            order: template_block.order,
            marker: template_block.marker,
            format: template_block.format,
            properties: template_block.properties.clone(),
        })
        .map_err(|e| AppError::Internal(e.to_string()))?;

        block_repo
            .insert(&new_block)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        id_map.insert(template_block.id, new_block.id);
        blocks_created += 1;
    }

    // Second pass: rebuild the parent/child relationships.
    for template_block in &template_blocks {
        // Skip root-level blocks (parent_id = None) — they have nothing to fix.
        let Some(template_parent_id) = template_block.parent_id else {
            continue;
        };

        // Look up the new ids for the child and its parent.
        let Some(&new_child_id) = id_map.get(&template_block.id) else {
            // Should never happen — every block in pass 1 was added to the map.
            continue;
        };
        let Some(&new_parent_id) = id_map.get(&template_parent_id) else {
            // The parent is outside the template (shouldn't happen for a
            // well-formed template). Skip rather than orphaning the child.
            continue;
        };

        let mut child = block_repo
            .get_by_id(new_child_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| {
                AppError::Internal(format!("Cloned block disappeared: {new_child_id}"))
            })?;

        child.parent_id = Some(new_parent_id);
        // Preserve the source's indentation level. The default in
        // `Block::new` would otherwise be 1 (root) for newly inserted rows.
        child.level = template_block.level.max(2);
        child.updated_at = chrono::Utc::now();

        block_repo
            .update(&child)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }

    Ok((
        StatusCode::CREATED,
        Json(FromTemplateResponse {
            page: PageDto::from(new_page),
            blocks_created,
        }),
    ))
}

/// Substitute `{{key}}` and `${key}` placeholders in a content string.
///
/// Built-in variables:
/// - `{{title}}` / `${title}` → `page_name`
/// - `{{name}}`  / `${name}`  → `page_name`
/// - `{{date}}`  / `${date}`  → today's date in ISO format (`YYYY-MM-DD`)
///
/// User-supplied `variables` are applied after built-ins so users can
/// override them if needed. Unknown placeholders are left intact so the
/// caller can find and fix them after the page is created.
pub fn substitute_placeholders(
    content: &str,
    variables: Option<&HashMap<String, String>>,
    page_name: &str,
) -> String {
    let mut result = content.to_string();

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    // Built-in: title / name → page name
    for token in ["{{title}}", "${title}"] {
        result = result.replace(token, page_name);
    }
    for token in ["{{name}}", "${name}"] {
        result = result.replace(token, page_name);
    }
    // Built-in: date → ISO today
    for token in ["{{date}}", "${date}"] {
        result = result.replace(token, &today);
    }

    // User-provided variables
    if let Some(vars) = variables {
        for (key, value) in vars {
            // Skip empty keys to avoid touching unrelated braces.
            if key.is_empty() {
                continue;
            }
            for token in [
                format!("{{{{{key}}}}}"),
                format!("${{{key}}}"),
            ] {
                result = result.replace(&token, value);
            }
        }
    }

    result
}
