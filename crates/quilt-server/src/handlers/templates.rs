//! HTTP handlers for template discovery (ADR-0007).
//!
//! Exposes the `TemplateUseCases` over REST so the frontend's
//! template picker can list available templates without going
//! through the MCP server. The MCP server also has the same
//! capabilities — both surfaces call the same application use case.
//!
//! Routes mounted under `/api/v1/templates`:
//! - GET `/`            — list all template pages
//! - GET `/:name/schema` — get the full schema of one template

use crate::error::AppError;
use crate::state::AppState;
use axum::{
    Json, Router,
    extract::{Extension, Path},
    routing::get,
};
use serde::Serialize;
use tracing::instrument;

/// Router for /api/v1/templates/*
pub fn routes() -> Router {
    Router::new()
        .route("/", get(list_templates))
        .route("/:name/schema", get(get_template_schema))
}

/// GET /api/v1/templates — list all template pages with their card
/// metadata. Used by the frontend's template picker.
#[instrument(skip(state))]
pub async fn list_templates(
    Extension(state): Extension<AppState>,
) -> Result<Json<Vec<TemplateSummaryResponse>>, AppError> {
    let templates = state
        .services
        .template
        .list_templates()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(templates.into_iter().map(Into::into).collect()))
}

/// GET /api/v1/templates/:name/schema — get the full schema of one
/// template by its short name. Returns 404 if the template page
/// does not exist.
#[instrument(skip(state))]
pub async fn get_template_schema(
    Extension(state): Extension<AppState>,
    Path(name): Path<String>,
) -> Result<Json<TemplateSchemaResponse>, AppError> {
    let schema = state
        .services
        .template
        .get_template_schema(&name)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Template not found: {}", name)))?;
    Ok(Json(schema.into()))
}

// ── Response DTOs (REST layer) ─────────────────────────────────────

/// Same fields as `quilt_application::TemplateSummary` but flattened
/// for the REST API. Avoids leaking the domain struct directly.
#[derive(Debug, Clone, Serialize)]
pub struct TemplateSummaryResponse {
    pub name: String,
    pub full_name: String,
    pub block_count: usize,
    pub card_shape: String,
    pub icon: Option<String>,
    pub cssclass: Option<String>,
}

impl From<quilt_application::use_cases::TemplateSummary> for TemplateSummaryResponse {
    fn from(t: quilt_application::use_cases::TemplateSummary) -> Self {
        Self {
            name: t.name,
            full_name: t.full_name,
            block_count: t.block_count,
            card_shape: t.card_shape,
            icon: t.icon,
            cssclass: t.cssclass,
        }
    }
}

/// Schema response. `properties` is the union of all block
/// properties declared on the template page, with `key` and `type`
/// (the JSON-ish type) and a stringified `value` example.
#[derive(Debug, Clone, Serialize)]
pub struct TemplateSchemaResponse {
    pub name: String,
    pub full_name: String,
    pub card_shape: String,
    pub icon: Option<String>,
    pub cssclass: Option<String>,
    pub block_count: usize,
    pub properties: Vec<TemplatePropertyResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplatePropertyResponse {
    pub key: String,
    pub value: String,
    #[serde(rename = "type")]
    pub type_: String,
    /// Canonical property type from quilt-domain. None when type_hint
    /// doesn't map to a known PropertyType.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property_type: Option<String>,
}

impl From<quilt_application::use_cases::TemplateSchema> for TemplateSchemaResponse {
    fn from(s: quilt_application::use_cases::TemplateSchema) -> Self {
        Self {
            name: s.name,
            full_name: s.full_name,
            card_shape: s.card_shape,
            icon: s.icon,
            cssclass: s.cssclass,
            block_count: s.blocks.len(),
            properties: s
                .properties
                .into_iter()
                .map(|p| TemplatePropertyResponse {
                    key: p.key,
                    value: p.value,
                    type_: p.r#type,
                    property_type: p.property_type.map(|pt| pt.as_str().to_string()),
                })
                .collect(),
        }
    }
}
