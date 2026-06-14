//! HTTP handlers for semantic property relations (PI-8).

use axum::{
    Extension, Json,
    extract::Query,
    routing::get,
    Router,
};
use std::sync::Arc;
use quilt_domain::properties::relation::{PropertyRelation, RelationType};
use quilt_domain::repositories::RelationRepository;
use quilt_domain::value_objects::Uuid;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

pub fn routes() -> Router {
    Router::new()
        .route("/", get(list_relations).post(create_relation))
        .route("/by-key/{key}", get(get_relations_by_key))
        .route("/from", get(get_relations_from))
        .route("/{id}", get(get_relation).delete(delete_relation))
}

#[derive(Debug, Deserialize)]
pub struct CreateRelationRequest {
    pub source_key: String,
    pub source_value: String,
    pub target_key: String,
    pub target_value: String,
    #[serde(default = "default_relation_type")]
    pub relation_type: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
}

fn default_relation_type() -> String { "precedes".to_string() }
fn default_confidence() -> f64 { 1.0 }

#[derive(Debug, Serialize)]
pub struct RelationListResponse {
    pub relations: Vec<PropertyRelation>,
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct FromQueryParams {
    pub key: String,
    pub value: String,
}

#[tracing::instrument(skip(relation_repo))]
pub async fn list_relations(
    Extension(relation_repo): Extension<Arc<dyn RelationRepository>>,
) -> Result<Json<RelationListResponse>, AppError> {
    let relations = relation_repo.list_all().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
    let count = relations.len();
    Ok(Json(RelationListResponse { relations, count }))
}

#[tracing::instrument(skip(relation_repo))]
pub async fn get_relation(
    axum::extract::Path(id_str): axum::extract::Path<String>,
    Extension(relation_repo): Extension<Arc<dyn RelationRepository>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let id = id_str.parse::<Uuid>().map_err(|_| AppError::BadRequest("Invalid UUID".to_string()))?;
    let rel = relation_repo.get_by_id(id).await.map_err(|e| AppError::BadRequest(e.to_string()))?;
    match rel {
        Some(r) => Ok(Json(serde_json::to_value(r).unwrap())),
        None => Err(AppError::BadRequest("Relation not found".to_string())),
    }
}

#[tracing::instrument(skip(relation_repo))]
pub async fn get_relations_by_key(
    axum::extract::Path(key): axum::extract::Path<String>,
    Extension(relation_repo): Extension<Arc<dyn RelationRepository>>,
) -> Result<Json<RelationListResponse>, AppError> {
    let relations = relation_repo.get_by_key(&key).await.map_err(|e| AppError::BadRequest(e.to_string()))?;
    let count = relations.len();
    Ok(Json(RelationListResponse { relations, count }))
}

#[tracing::instrument(skip(relation_repo))]
pub async fn get_relations_from(
    Query(params): Query<FromQueryParams>,
    Extension(relation_repo): Extension<Arc<dyn RelationRepository>>,
) -> Result<Json<RelationListResponse>, AppError> {
    let relations = relation_repo.get_from(&params.key, &params.value).await.map_err(|e| AppError::BadRequest(e.to_string()))?;
    let count = relations.len();
    Ok(Json(RelationListResponse { relations, count }))
}

#[tracing::instrument(skip(relation_repo))]
pub async fn create_relation(
    Extension(relation_repo): Extension<Arc<dyn RelationRepository>>,
    Json(body): Json<CreateRelationRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rt = match body.relation_type.as_str() {
        "precedes" => RelationType::Precedes,
        "broadens" => RelationType::Broadens,
        "implies" => RelationType::Implies,
        "requires" => RelationType::Requires,
        other => RelationType::Custom(other.to_string()),
    };

    let relation = PropertyRelation::new(
        Uuid::new_v4(),
        body.source_key,
        body.source_value,
        body.target_key,
        body.target_value,
        rt,
        body.description,
        body.confidence,
    );

    relation_repo.insert(&relation).await.map_err(|e| AppError::BadRequest(e.to_string()))?;
    Ok(Json(serde_json::to_value(&relation).unwrap()))
}

#[tracing::instrument(skip(relation_repo))]
pub async fn delete_relation(
    axum::extract::Path(id_str): axum::extract::Path<String>,
    Extension(relation_repo): Extension<Arc<dyn RelationRepository>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let id = id_str.parse::<Uuid>().map_err(|_| AppError::BadRequest("Invalid UUID".to_string()))?;
    relation_repo.delete(id).await.map_err(|e| AppError::BadRequest(e.to_string()))?;
    Ok(Json(serde_json::json!({"deleted": true})))
}
