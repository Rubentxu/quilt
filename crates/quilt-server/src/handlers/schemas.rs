//! HTTP handlers for property schema endpoints (PI-7).

use axum::{
    Extension, Json,
    extract::Query,
    routing::{get, post, delete},
    Router,
};
use quilt_application::schema::{SchemaService, SchemaServiceTrait};
use quilt_domain::properties::schema::{AutoDetectParams, PropertySchema};
use quilt_domain::value_objects::Uuid;
use quilt_infrastructure::database::sqlite::repositories::SqliteSchemaRepository;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::state::AppState;

pub fn routes() -> Router {
    Router::new()
        .route("/", get(list_schemas).post(create_schema))
        .route("/auto-detect", post(auto_detect_schemas))
        .route("/{id}", get(get_schema).delete(delete_schema))
        .route("/by-name/{name}", get(get_schema_by_name))
}

// ── DTOs ──

#[derive(Debug, Deserialize)]
pub struct CreateSchemaRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub property_keys: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct AutoDetectQueryParams {
    #[serde(default = "default_min_co")]
    pub min_co_occurrence: u64,
    #[serde(default = "default_min_pmi")]
    pub min_pmi: f64,
    #[serde(default = "default_max")]
    pub max_schemas: usize,
    #[serde(default = "default_min_props")]
    pub min_properties: usize,
}

fn default_min_co() -> u64 { 3 }
fn default_min_pmi() -> f64 { 0.5 }
fn default_max() -> usize { 10 }
fn default_min_props() -> usize { 2 }

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaListResponse {
    pub schemas: Vec<PropertySchema>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoDetectResponse {
    pub detected: Vec<PropertySchema>,
    pub count: usize,
}

// ── Handlers ──

#[tracing::instrument(skip(state))]
pub async fn list_schemas(
    Extension(state): Extension<AppState>,
) -> Result<Json<SchemaListResponse>, AppError> {
    let schema_repo = std::sync::Arc::new(SqliteSchemaRepository::new(state.pool.clone()));
    let prop_repo = std::sync::Arc::new(quilt_infrastructure::database::sqlite::repositories::SqlitePropertyRepository::new(state.pool.clone()));
    let service = SchemaService::new(schema_repo, prop_repo);

    let schemas = service.list_all().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
    let count = schemas.len();
    Ok(Json(SchemaListResponse { schemas, count }))
}

#[tracing::instrument(skip(state))]
pub async fn get_schema(
    axum::extract::Path(id_str): axum::extract::Path<String>,
    Extension(state): Extension<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let id = id_str.parse::<Uuid>().map_err(|_| AppError::BadRequest("Invalid UUID".to_string()))?;
    let schema_repo = std::sync::Arc::new(SqliteSchemaRepository::new(state.pool.clone()));
    let prop_repo = std::sync::Arc::new(quilt_infrastructure::database::sqlite::repositories::SqlitePropertyRepository::new(state.pool.clone()));
    let service = SchemaService::new(schema_repo, prop_repo);

    let schema = service.get_by_id(id).await.map_err(|e| AppError::BadRequest(e.to_string()))?;
    match schema {
        Some(s) => Ok(Json(serde_json::to_value(s).unwrap())),
        None => Err(AppError::BadRequest("Schema not found".to_string())),
    }
}

#[tracing::instrument(skip(state))]
pub async fn get_schema_by_name(
    axum::extract::Path(name): axum::extract::Path<String>,
    Extension(state): Extension<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let schema_repo = std::sync::Arc::new(SqliteSchemaRepository::new(state.pool.clone()));
    let prop_repo = std::sync::Arc::new(quilt_infrastructure::database::sqlite::repositories::SqlitePropertyRepository::new(state.pool.clone()));
    let service = SchemaService::new(schema_repo, prop_repo);

    let schema = service.get_by_name(&name).await.map_err(|e| AppError::BadRequest(e.to_string()))?;
    match schema {
        Some(s) => Ok(Json(serde_json::to_value(s).unwrap())),
        None => Err(AppError::BadRequest("Schema not found".to_string())),
    }
}

#[tracing::instrument(skip(state))]
pub async fn create_schema(
    Extension(state): Extension<AppState>,
    Json(body): Json<CreateSchemaRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let schema_repo = std::sync::Arc::new(SqliteSchemaRepository::new(state.pool.clone()));
    let prop_repo = std::sync::Arc::new(quilt_infrastructure::database::sqlite::repositories::SqlitePropertyRepository::new(state.pool.clone()));
    let service = SchemaService::new(schema_repo, prop_repo);

    let schema = PropertySchema::new(
        Uuid::new_v4(),
        body.name,
        body.description,
        body.property_keys,
        false,
    );

    service.create(&schema).await.map_err(|e| AppError::BadRequest(e.to_string()))?;
    Ok(Json(serde_json::to_value(&schema).unwrap()))
}

#[tracing::instrument(skip(state))]
pub async fn delete_schema(
    axum::extract::Path(id_str): axum::extract::Path<String>,
    Extension(state): Extension<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let id = id_str.parse::<Uuid>().map_err(|_| AppError::BadRequest("Invalid UUID".to_string()))?;
    let schema_repo = std::sync::Arc::new(SqliteSchemaRepository::new(state.pool.clone()));
    let prop_repo = std::sync::Arc::new(quilt_infrastructure::database::sqlite::repositories::SqlitePropertyRepository::new(state.pool.clone()));
    let service = SchemaService::new(schema_repo, prop_repo);

    service.delete(id).await.map_err(|e| AppError::BadRequest(e.to_string()))?;
    Ok(Json(serde_json::json!({"deleted": true})))
}

#[tracing::instrument(skip(state))]
pub async fn auto_detect_schemas(
    Extension(state): Extension<AppState>,
    Query(params): Query<AutoDetectQueryParams>,
) -> Result<Json<AutoDetectResponse>, AppError> {
    let schema_repo = std::sync::Arc::new(SqliteSchemaRepository::new(state.pool.clone()));
    let prop_repo = std::sync::Arc::new(quilt_infrastructure::database::sqlite::repositories::SqlitePropertyRepository::new(state.pool.clone()));
    let service = SchemaService::new(schema_repo, prop_repo);

    let detect_params = AutoDetectParams {
        min_co_occurrence: params.min_co_occurrence,
        min_pmi: params.min_pmi,
        max_schemas: params.max_schemas,
        min_properties: params.min_properties,
    };

    let detected = service.auto_detect(&detect_params).await.map_err(|e| AppError::BadRequest(e.to_string()))?;
    let count = detected.len();
    Ok(Json(AutoDetectResponse { detected, count }))
}
