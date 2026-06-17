//! Page-related HTTP handlers

use axum::{
    Json,
    extract::{Extension, Path, Query},
    http::StatusCode,
};
use axum::{Router, routing::get};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tracing::instrument;

use crate::error::AppError;
use crate::handlers::blocks::map_app_error;
use crate::state::AppState;
use quilt_application::services::ref_service::RefServiceTrait;
use quilt_domain::entities::{Block, BlockCreate, Page, PageCreate};
use quilt_domain::repositories::{
    BlockRepository, PageRepository, RefRepository, SettingsRepository,
};
use quilt_domain::value_objects::{BlockFormat, JournalDay, Uuid};

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
#[instrument(skip(ref_service, page_repo, block_repo))]
pub async fn get_page_unlinked_references(
    Path(name): Path<String>,
    Extension(ref_service): Extension<Arc<dyn RefServiceTrait>>,
    Extension(page_repo): Extension<Arc<dyn PageRepository>>,
    Extension(block_repo): Extension<Arc<dyn BlockRepository>>,
) -> Result<Json<Vec<BacklinkDto>>, AppError> {
    // Look up the page by name
    let page = page_repo
        .get_by_name(&name)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Page not found: {}", name)))?;

    // Query unlinked references via the ref service
    let unlinked = ref_service
        .get_page_unlinked_references(&name, page.id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

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
            content_preview: content_preview.clone(),
            // Unlinked references have no `refs` row, so there is no
            // override to read — `context` falls back to the
            // content snippet, matching the GET-backlinks behavior.
            context: content_preview,
        });
    }

    Ok(Json(dtos))
}

/// Create router for /api/v1/pages
pub fn routes() -> Router {
    Router::new()
        .route("/", get(list_pages).post(create_page))
        // NOTE: literal segments (`/from-template`, `/search`) must be
        // registered BEFORE the `/:name` catch-all so that axum does not
        // route the literal strings `from-template` / `search` to the
        // page-by-name handler.
        .route(
            "/from-template",
            axum::routing::post(create_page_from_template),
        )
        .route("/search", get(search_pages))
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
    let pages = state.services.page.list().await.map_err(map_app_error)?;

    let dtos: Vec<PageDto> = pages.into_iter().map(PageDto::from).collect();

    Ok(Json(dtos))
}

/// Query parameters for `GET /api/v1/pages/search`.
///
/// `q` is the user-supplied search string. When absent or empty the
/// handler treats it as "match everything" and delegates to
/// `PageRepository::get_all()` so the frontend can call the same
/// endpoint for both the empty-query and the typed-query case (the
/// previous design forced two separate calls).
///
/// `limit` is the maximum number of pages to return. The repository
/// implementation already does an `ORDER BY name LIMIT ?`, so the
/// `limit` is enforced server-side. We clamp it to a sane upper bound
/// (200) to keep response payloads predictable; the frontend's
/// `PAGE_LIMIT` of 10 means the typical response is far smaller.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPagesQuery {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

/// GET /api/v1/pages/search?q=...&limit=...
///
/// Server-side page-name search — S2-03. The previous frontend
/// implementation called `GET /api/v1/pages` (which returns the entire
/// page list) and then `Array.prototype.includes` filtered it on every
/// keystroke. That is O(n) work on the client AND a multi-MB JSON
/// payload for graphs with thousands of pages. This endpoint delegates
/// the `LIKE` filter to SQLite (single round-trip, O(log n) with the
/// existing `pages_name` index when present) and returns only the
/// matches.
///
/// Behavior:
/// - `q` empty / missing → returns up to `limit` pages ordered by name.
///   This lets the frontend call a single endpoint whether the input
///   is empty or non-empty.
/// - `q` non-empty → `WHERE name LIKE '%q%'`, also matching `title`
///   so pages with a custom title still surface in the results.
/// - `limit` is clamped to `[1, 200]` with a default of 50 — well above
///   the frontend's `PAGE_LIMIT` of 10 but small enough to keep the
///   payload bounded.
///
/// Note: `LIKE` is case-insensitive only for ASCII by default in
/// SQLite; the `Page` entity normalises names to lowercase at write
/// time, so the caller's lowercase-typed query is what gets stored.
/// This is intentional — page names are case-insensitive throughout
/// the rest of Quilt.
#[instrument(skip(_state, page_repo, query), fields(q = %query.q.as_deref().unwrap_or(""), limit = query.limit.unwrap_or(0)))]
pub async fn search_pages(
    Extension(_state): Extension<AppState>,
    Extension(page_repo): Extension<Arc<dyn PageRepository>>,
    Query(query): Query<SearchPagesQuery>,
) -> Result<Json<Vec<PageDto>>, AppError> {
    // Clamp limit to a sane range. `0` means "use the repo default"
    // so a caller that omits the param gets the same behaviour as a
    // caller that passes `limit=50`.
    let limit = match query.limit {
        Some(0) | None => 50usize,
        Some(n) => n.clamp(1, 200),
    };

    let pages = match query.q.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(q) => {
            // Search pages by name or title (S2-03: matches both columns)
            page_repo
                .search_by_name_or_title(q, limit)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
        }
        None => {
            // Empty / missing query — return a page-name-ordered slice
            // of the full set. Same shape as `list_pages` but with a
            // server-side limit so the response stays bounded even for
            // very large graphs.
            let mut all = page_repo
                .get_all()
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            all.sort_by(|a, b| a.name.cmp(&b.name));
            all.truncate(limit);
            all
        }
    };

    let dtos: Vec<PageDto> = pages.into_iter().map(PageDto::from).collect();
    Ok(Json(dtos))
}

/// GET /api/v1/pages/:name
#[instrument(skip(state))]
pub async fn get_page(
    Path(name): Path<String>,
    Extension(state): Extension<AppState>,
) -> Result<Json<PageDto>, AppError> {
    let page = state
        .services
        .page
        .get_by_name(&name)
        .await
        .map_err(map_app_error)?;

    match page {
        Some(p) => Ok(Json(PageDto::from(p))),
        None => Err(AppError::NotFound(format!("Page not found: {}", name))),
    }
}

/// POST /api/v1/pages
#[instrument(skip(page_repo, settings_repo))]
pub async fn create_page(
    Extension(page_repo): Extension<Arc<dyn PageRepository>>,
    Extension(settings_repo): Extension<Arc<dyn SettingsRepository>>,
    Json(payload): Json<CreatePageRequest>,
) -> Result<(StatusCode, Json<PageDto>), AppError> {
    if payload.is_journal.unwrap_or(false) {
        // Create journal page
        let day = if let Some(ref jd) = payload.journal_day {
            JournalDay::from_str(jd)
                .map_err(|e| AppError::BadRequest(format!("Invalid journal day: {}", e)))?
        } else {
            JournalDay::from_str(&payload.name)
                .map_err(|e| AppError::BadRequest(format!("Invalid journal date: {}", e)))?
        };

        let settings = settings_repo
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
            // Manually-created pages don't have a source file
            source_path: None,
            source_mtime: None,
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
#[instrument(skip(page_repo, settings_repo))]
pub async fn get_journal(
    Path(date): Path<String>,
    Extension(page_repo): Extension<Arc<dyn PageRepository>>,
    Extension(settings_repo): Extension<Arc<dyn SettingsRepository>>,
) -> Result<Json<PageDto>, AppError> {
    let day = JournalDay::from_str(&date)
        .map_err(|e| AppError::BadRequest(format!("Invalid journal date: {}", e)))?;

    let page = match page_repo.get_journal(day).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            // Read user's journal format setting
            let settings = settings_repo
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
    /// The snippet shown in the Backlinks panel for this reference.
    ///
    /// Q028 (Editable Backlinks): when the user has set a custom
    /// context override via `PUT /api/v1/references/:blockId`, this
    /// field reflects that override. Otherwise, it falls back to the
    /// source block's `contentPreview` so the panel always has
    /// something to render.
    pub context: String,
}

/// GET /api/v1/pages/:name/backlinks
///
/// Returns all blocks that reference the given page.
/// Uses the in-memory RefIndex for O(1) lookup.
#[instrument(skip(ref_service, page_repo, block_repo, ref_repo))]
pub async fn get_page_backlinks(
    Path(name): Path<String>,
    Extension(ref_service): Extension<Arc<dyn RefServiceTrait>>,
    Extension(page_repo): Extension<Arc<dyn PageRepository>>,
    Extension(block_repo): Extension<Arc<dyn BlockRepository>>,
    Extension(ref_repo): Extension<Arc<dyn RefRepository>>,
) -> Result<Json<Vec<BacklinkDto>>, AppError> {
    // Look up the page by name
    let page = page_repo
        .get_by_name(&name)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Page not found: {}", name)))?;

    // Query the in-memory ref index for O(1) backlinks
    let backlinks = ref_service.get_backlinks(page.id);

    if backlinks.is_empty() {
        return Ok(Json(Vec::new()));
    }

    // Q028 (Editable Backlinks): bulk-fetch every custom-context
    // override for the target page in a single SQL query so the
    // per-source enrichment below is an in-memory map lookup, not
    // an N+1 round-trip. References without an override (no row
    // in the result) fall back to the source block's content
    // snippet.
    let overrides = ref_repo
        .get_custom_contexts_for_target(page.id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let overrides_by_source: HashMap<Uuid, String> = overrides
        .into_iter()
        .map(|(source, _ref_type, ctx)| (source, ctx))
        .collect();

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
                plain_text.clone()
            };

            // `context` falls back to the source block's content snippet
            // when no override is set. An empty-string override stays
            // empty (the user explicitly cleared the text).
            let context = overrides_by_source
                .get(source_id)
                .cloned()
                .unwrap_or(content_preview.clone());

            dtos.push(BacklinkDto {
                source_block_id: source_id.to_string(),
                source_page_name,
                content_preview,
                context,
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
    let page_with_blocks = state
        .services
        .page
        .get_blocks(&name)
        .await
        .map_err(map_app_error)?;

    let block_dtos: Vec<crate::handlers::blocks::BlockDto> = page_with_blocks
        .blocks
        .into_iter()
        .map(|b| (b, Some(page_with_blocks.page.name.clone())).into())
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
#[instrument(skip(_state, page_repo, block_repo, req), fields(template_name = %req.template_name, page_name = %req.page_name))]
pub async fn create_page_from_template(
    Extension(_state): Extension<AppState>,
    Extension(page_repo): Extension<Arc<dyn PageRepository>>,
    Extension(block_repo): Extension<Arc<dyn BlockRepository>>,
    Json(req): Json<FromTemplateRequest>,
) -> Result<(StatusCode, Json<FromTemplateResponse>), AppError> {
    // 1. Verify template exists
    let template = page_repo
        .get_by_name(&req.template_name)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Template not found: {}", req.template_name)))?;

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
        // Pages created from templates are not ingested from files
        source_path: None,
        source_mtime: None,
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
            // Preserve the template's block type. Default `Paragraph` only
            // applies if the source block somehow has an unknown value,
            // which the parse_block_type helper in the SQLite repo handles
            // safely on read.
            block_type: template_block.block_type,
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
            for token in [format!("{{{{{key}}}}}"), format!("${{{key}}}")] {
                result = result.replace(&token, value);
            }
        }
    }

    result
}
