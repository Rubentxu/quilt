//! Annotation-related HTTP handlers
//!
//! REST surface for the annotation API. All routes require
//! `Authorization: Bearer <key>` (enforced by the auth middleware in
//! `routes.rs`). The handlers are thin: they parse path/query/body
//! inputs, delegate to [`AnnotationUseCases`], and convert the
//! returned domain entities into [`AnnotationDto`].
//!
//! # Routes
//!
//! - `POST   /api/v1/annotations` — create
//! - `GET    /api/v1/annotations` — list (filters: `block_id`, `status`, `scope`)
//! - `GET    /api/v1/annotations/:id` — get one
//! - `PATCH  /api/v1/annotations/:id/status` — update status
//! - `DELETE /api/v1/annotations/:id` — delete
//! - `GET    /api/v1/blocks/:block_id/annotations` — list by block (convenience)

use axum::{
    Json, Router,
    extract::{Extension, Path, Query},
    http::StatusCode,
    routing::{get, patch, post},
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::instrument;

use crate::error::AppError;
use quilt_application::use_cases::annotation::AnnotationUseCases;
use quilt_application::{
    AnnotationDto, AnnotationFilters, AnnotationScope, AnnotationStatus, Uuid,
};
use quilt_domain::entities::AuthorType;
use quilt_domain::errors::DomainError;

// ── Request / Query types ─────────────────────────────────────────

/// Query params for the list endpoint.
#[derive(Debug, Default, Deserialize)]
pub struct ListAnnotationsQuery {
    pub block_id: Option<String>,
    pub status: Option<String>,
    pub scope: Option<String>,
    pub author_name: Option<String>,
}

/// Path param for the convenience "list by block" route.
///
/// The `Path<T>` extractor deserializes from path-segment names, NOT
/// from JSON — so we MUST keep the field name in snake_case to
/// match the path segment `:block_id`. Adding `#[serde(rename_all
/// = "camelCase")]` would make axum look for `:blockId`, which
/// doesn't exist in our route.
#[derive(Debug, Deserialize)]
pub struct BlockPath {
    pub block_id: String,
}

// ── Routing ────────────────────────────────────────────────────────

/// Build the annotation router. Mounted at `/api/v1/annotations`.
pub fn routes() -> Router {
    Router::new()
        .route("/", post(create_annotation).get(list_annotations))
        .route("/:id", get(get_annotation).delete(delete_annotation))
        .route("/:id/status", patch(update_annotation_status))
}

/// Convenience route mounted at `/api/v1/blocks/:block_id/annotations`
/// so the frontend can fetch annotations in the same request shape as
/// the existing `GET /api/v1/blocks/:id/backlinks`.
pub fn block_routes() -> Router {
    Router::new().route("/:block_id/annotations", get(list_annotations_for_block))
}

// ── Handlers ───────────────────────────────────────────────────────

/// POST /api/v1/annotations
///
/// Returns:
/// - `201 Created` with the new annotation JSON
/// - `400 Bad Request` for invalid UUIDs, unknown enum values, or
///   empty content
#[instrument(skip(use_cases, payload))]
pub async fn create_annotation(
    Extension(use_cases): Extension<Arc<dyn AnnotationUseCases>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<AnnotationDto>), AppError> {
    let block_id = payload
        .get("blockId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing 'blockId'".to_string()))?;
    let scope = payload
        .get("scope")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing 'scope'".to_string()))?;
    let author_type = payload
        .get("authorType")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing 'authorType'".to_string()))?;
    let author_name = payload
        .get("authorName")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing 'authorName'".to_string()))?;
    let content = payload
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing 'content'".to_string()))?;
    let parent = payload.get("parentAnnotationId").and_then(|v| v.as_str());
    let highlight_start = payload
        .get("highlightStart")
        .and_then(|v| v.as_u64())
        .map(|n| n as u32);
    let highlight_end = payload
        .get("highlightEnd")
        .and_then(|v| v.as_u64())
        .map(|n| n as u32);

    let annotation = use_cases
        .create_from_dto(
            block_id,
            scope,
            author_type,
            author_name,
            content,
            parent,
            highlight_start,
            highlight_end,
        )
        .await
        .map_err(map_app_error)?;

    Ok((StatusCode::CREATED, Json(AnnotationDto::from(annotation))))
}

/// GET /api/v1/annotations
///
/// Returns:
/// - `200 OK` with an array of annotation DTOs
#[instrument(skip(use_cases))]
pub async fn list_annotations(
    Extension(use_cases): Extension<Arc<dyn AnnotationUseCases>>,
    Query(params): Query<ListAnnotationsQuery>,
) -> Result<Json<Vec<AnnotationDto>>, AppError> {
    // Empty filters return every annotation (DESC by `created_at`).
    let filters = build_filters(&params)?;
    let annotations = use_cases
        .list_by_filters(&filters)
        .await
        .map_err(map_app_error)?;

    Ok(Json(
        annotations.into_iter().map(AnnotationDto::from).collect(),
    ))
}

/// GET /api/v1/annotations/:id
///
/// Returns:
/// - `200 OK` with the annotation JSON
/// - `404 Not Found` when the id does not exist
#[instrument(skip(use_cases))]
pub async fn get_annotation(
    Extension(use_cases): Extension<Arc<dyn AnnotationUseCases>>,
    Path(id): Path<String>,
) -> Result<Json<AnnotationDto>, AppError> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| AppError::BadRequest(format!("Invalid annotation UUID: {}", id)))?;

    let annotation = use_cases
        .get_by_id(uuid)
        .await
        .map_err(map_app_error)?
        .ok_or_else(|| AppError::NotFound(format!("Annotation not found: {}", id)))?;

    Ok(Json(AnnotationDto::from(annotation)))
}

/// GET /api/v1/blocks/:block_id/annotations
///
/// Convenience: same as `GET /api/v1/annotations?block_id=...` but
/// keeps the path symmetric with the existing block endpoints.
#[instrument(skip(use_cases))]
pub async fn list_annotations_for_block(
    Extension(use_cases): Extension<Arc<dyn AnnotationUseCases>>,
    Path(path): Path<BlockPath>,
) -> Result<Json<Vec<AnnotationDto>>, AppError> {
    let block_uuid = Uuid::parse_str(&path.block_id)
        .map_err(|_| AppError::BadRequest(format!("Invalid block UUID: {}", path.block_id)))?;

    let annotations = use_cases
        .list_by_block(block_uuid)
        .await
        .map_err(map_app_error)?;

    Ok(Json(
        annotations.into_iter().map(AnnotationDto::from).collect(),
    ))
}

/// PATCH /api/v1/annotations/:id/status
///
/// Body: `{ "status": "resolved", "resolvedBy": "claude" }`
///
/// Returns:
/// - `200 OK` with the updated annotation
/// - `400 Bad Request` when status is invalid or `resolvedBy` is
///   missing for `status: "resolved"`
/// - `404 Not Found` when the id does not exist
#[instrument(skip(use_cases, payload))]
pub async fn update_annotation_status(
    Extension(use_cases): Extension<Arc<dyn AnnotationUseCases>>,
    Path(id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<AnnotationDto>, AppError> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| AppError::BadRequest(format!("Invalid annotation UUID: {}", id)))?;

    let status_str = payload
        .get("status")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing 'status'".to_string()))?;
    let status = AnnotationStatus::try_from_str(status_str).ok_or_else(|| {
        AppError::BadRequest(format!(
            "Invalid status: '{}'. Expected pending, in_progress, resolved, dismissed",
            status_str
        ))
    })?;
    let resolved_by = payload
        .get("resolvedBy")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let annotation = use_cases
        .update_status(uuid, status, resolved_by)
        .await
        .map_err(map_app_error)?;

    Ok(Json(AnnotationDto::from(annotation)))
}

/// DELETE /api/v1/annotations/:id
///
/// Returns:
/// - `204 No Content` on success (idempotent — also returns 204 when
///   the id does not exist)
#[instrument(skip(use_cases))]
pub async fn delete_annotation(
    Extension(use_cases): Extension<Arc<dyn AnnotationUseCases>>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| AppError::BadRequest(format!("Invalid annotation UUID: {}", id)))?;

    use_cases.delete(uuid).await.map_err(map_app_error)?;

    Ok(StatusCode::NO_CONTENT)
}

// ── Internal helpers ──────────────────────────────────────────────

/// Convert a [`ListAnnotationsQuery`] into an [`AnnotationFilters`]
/// value. Each present field is validated: unknown enum strings
/// produce a `400 Bad Request` so the wire contract is enforced at
/// the boundary.
fn build_filters(params: &ListAnnotationsQuery) -> Result<AnnotationFilters, AppError> {
    let mut f = AnnotationFilters::default();
    if let Some(ref id) = params.block_id {
        let uuid = Uuid::parse_str(id)
            .map_err(|_| AppError::BadRequest(format!("Invalid block UUID: {}", id)))?;
        f = f.with_block_id(uuid);
    }
    if let Some(ref s) = params.status {
        // Validate against the enum — `with_status` would accept any
        // string and let the repo decide; we want to surface a clean
        // 400 here instead of an empty result.
        if AnnotationStatus::try_from_str(s).is_none() {
            return Err(AppError::BadRequest(format!(
                "Invalid status filter: '{}'. Expected pending, in_progress, resolved, dismissed",
                s
            )));
        }
        f = f.with_status(s);
    }
    if let Some(ref s) = params.scope {
        let scope = AnnotationScope::try_from_str(s).ok_or_else(|| {
            AppError::BadRequest(format!(
                "Invalid scope filter: '{}'. Expected block, inline",
                s
            ))
        })?;
        f = f.with_scope(scope);
    }
    if let Some(ref name) = params.author_name {
        f = f.with_author_name(name);
    }
    Ok(f)
}

/// Map an [`ApplicationError`] onto the HTTP layer's [`AppError`].
/// Validation errors are surfaced as `400`, not-found as `404`,
/// domain `InvalidData` / `Validation` errors as `400`, everything
/// else as `500`.
fn map_app_error(e: quilt_application::ApplicationError) -> AppError {
    use quilt_application::ApplicationError;
    match e {
        ApplicationError::Validation(msg) => AppError::BadRequest(msg),
        ApplicationError::NotFound(kind, id) => {
            AppError::NotFound(format!("{} not found: {}", kind, id))
        }
        ApplicationError::Domain(d) => match d {
            // Domain validation errors come from invariants checked
            // inside entity constructors (e.g. `Annotation::new`
            // rejecting empty content or invalid inline offsets).
            // Surface those as `400 Bad Request` so the wire
            // contract matches the use-case `Validation` variant.
            DomainError::InvalidData(msg) => AppError::BadRequest(msg),
            _ => AppError::Internal(d.to_string()),
        },
        ApplicationError::Infrastructure(msg) => AppError::Internal(msg),
    }
}

// ── Tests ──────────────────────────────────────────────────────────
//
// DTO/serde-level tests for the request and response wire shapes.
// Full-stack HTTP tests live in `crates/quilt-server/tests/annotation_api_tests.rs`.

#[cfg(test)]
mod tests {
    use super::*;

    /// `ListAnnotationsQuery` must accept the camelCase field names
    /// the frontend sends and tolerate every field being absent
    /// (empty filter → "give me everything").
    #[test]
    fn list_query_empty_is_default() {
        let q: ListAnnotationsQuery = serde_json::from_str("{}").expect("empty query must parse");
        assert!(q.block_id.is_none());
        assert!(q.status.is_none());
        assert!(q.scope.is_none());
        assert!(q.author_name.is_none());
    }

    /// `ListAnnotationsQuery` must accept the snake_case field names
    /// that show up in the URL (`?block_id=...&status=...&scope=...&author_name=...`).
    /// This matches the wire shape the frontend sends.
    #[test]
    fn list_query_full_snakecase() {
        let q: ListAnnotationsQuery = serde_json::from_str(
            r#"{"block_id":"550e8400-e29b-41d4-a716-446655440000","status":"pending","scope":"block","author_name":"alice"}"#,
        )
        .unwrap();
        assert_eq!(
            q.block_id.as_deref(),
            Some("550e8400-e29b-41d4-a716-446655440000")
        );
        assert_eq!(q.status.as_deref(), Some("pending"));
        assert_eq!(q.scope.as_deref(), Some("block"));
        assert_eq!(q.author_name.as_deref(), Some("alice"));
    }

    /// `build_filters` must reject unknown `status` strings at the
    /// boundary — the repository would otherwise return an empty
    /// result for `"pendng"` (typo) and the frontend would see zero
    /// annotations and assume the call worked.
    #[test]
    fn build_filters_rejects_unknown_status() {
        let q = ListAnnotationsQuery {
            block_id: None,
            status: Some("pendng".into()),
            scope: None,
            author_name: None,
        };
        let err = build_filters(&q).unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[test]
    fn build_filters_rejects_unknown_scope() {
        let q = ListAnnotationsQuery {
            block_id: None,
            status: None,
            scope: Some("sideways".into()),
            author_name: None,
        };
        let err = build_filters(&q).unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[test]
    fn build_filters_accepts_known_values() {
        let q = ListAnnotationsQuery {
            block_id: Some("550e8400-e29b-41d4-a716-446655440000".into()),
            status: Some("pending".into()),
            scope: Some("block".into()),
            author_name: Some("alice".into()),
        };
        let f = build_filters(&q).unwrap();
        assert_eq!(
            f.block_id.unwrap().to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
        assert_eq!(f.status.as_deref(), Some("pending"));
        assert_eq!(f.scope, Some(AnnotationScope::Block));
        assert_eq!(f.author_name.as_deref(), Some("alice"));
    }

    /// `AnnotationDto` MUST round-trip through serde with camelCase
    /// keys — the TypeScript client reads `blockId`, `parentAnnotationId`,
    /// etc., and any drift breaks the wire contract.
    #[test]
    fn annotation_dto_round_trips_camelcase() {
        let a = quilt_domain::entities::Annotation::new(quilt_domain::entities::AnnotationCreate {
            block_id: Uuid::new_v4(),
            scope: AnnotationScope::Block,
            author_type: AuthorType::Human,
            author_name: "alice".into(),
            content: "x".into(),
            parent_annotation_id: None,
            highlight_start: None,
            highlight_end: None,
        })
        .unwrap();
        let dto = AnnotationDto::from(a.clone());
        let json = serde_json::to_string(&dto).unwrap();
        // camelCase keys present
        assert!(json.contains("\"blockId\""), "json was: {json}");
        assert!(json.contains("\"authorType\""), "json was: {json}");
        assert!(json.contains("\"authorName\""), "json was: {json}");
        // snake_case keys absent
        assert!(!json.contains("\"block_id\""), "json was: {json}");
        // enums as lowercase strings
        assert!(json.contains("\"scope\":\"block\""), "json was: {json}");
        assert!(
            json.contains("\"authorType\":\"human\""),
            "json was: {json}"
        );
        assert!(json.contains("\"status\":\"pending\""), "json was: {json}");
    }
}
