//! HTTP handlers for property aggregation endpoints.
//!
//! Currently exposes:
//!
//! - `GET /api/v1/properties/keys` — list distinct top-level property
//!   **keys** that appear in any block's `properties` JSON column,
//!   paginated by key (lexicographic ASC, forward-only, key-as-cursor).
//!
//! This is the first cursor-paginated read in Quilt, so the
//! convention set here is project-wide:
//!
//! * `cursor`: optional, non-empty if present. Pages forward by
//!   returning keys **strictly greater** than `cursor`.
//! * `limit`: optional, default 50, bounds 1..=100. Out of range
//!   yields 400.
//! * Response shape: `{ keys: string[], nextCursor: string | null }`.
//!   `nextCursor` is `null` when `keys.len() < limit` (definitive
//!   last page); otherwise it's the last key in `keys` (caller may
//!   have more pages).
//!
//! Auth is enforced by the global middleware on `/api/v1/*`.

use axum::{
    Json, Router,
    extract::{Extension, Query},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::instrument;

use crate::error::AppError;
use crate::state::AppState;
use quilt_application::property::{
    PropertyService, PropertyServiceTrait, PropertySuggestion,
};
use quilt_domain::properties::analytics::AnalyticsParams;
use quilt_domain::properties::definition::PropertyDefinition;
use quilt_domain::repositories::{BlockRepository, PropertyRepository};

/// Default page size when `?limit=` is absent.
fn default_limit() -> u32 {
    50
}

/// Lower bound (inclusive) for `?limit=`.
const MIN_LIMIT: u32 = 1;

/// Upper bound (inclusive) for `?limit=`.
const MAX_LIMIT: u32 = 100;

/// Query string for `GET /api/v1/properties/keys`.
///
/// `cursor` and `limit` are both optional. `limit` defaults to 50
/// when missing; axum uses `default_limit` for that. `cursor` of the
/// empty string is rejected at validation time (NOT at
/// deserialization time) so we can return a clean `400 BadRequest`
/// with a domain-meaningful error message.
#[derive(Debug, Clone, Deserialize)]
pub struct PropertyKeysParams {
    /// Optional cursor — keys must be strictly greater than this.
    /// Empty string is a client error (400).
    pub cursor: Option<String>,
    /// Max number of keys to return. Default: 50. Bounds: 1..=100.
    #[serde(default = "default_limit")]
    pub limit: u32,
}

/// Response body for `GET /api/v1/properties/keys`.
///
/// Field names are camelCase on the wire to match the rest of the
/// Quilt JSON API (see `BlockDto` for the convention).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyKeysResponse {
    /// Distinct top-level property keys, lexicographically ascending.
    pub keys: Vec<String>,
    /// Cursor for the next page, or `null` if this was the last page.
    pub next_cursor: Option<String>,
}

/// Router for `/api/v1/properties`.
///
/// Mounted at `/api/v1/properties` (see `routes.rs`). Only one route
/// is exposed today; additional aggregations (e.g. distinct values
/// per key) will hang off the same nest.
pub fn routes() -> Router {
    Router::new()
        .route("/keys", get(list_property_keys))
        .route("/batch", post(batch_properties))
        .route("/suggest", get(suggest_properties))
        .route("/analytics", get(analytics_properties))
        .route("/lifecycle/deprecate", post(deprecate_property))
        .route("/lifecycle/merge", post(merge_property))
        .route("/lifecycle/alias", post(alias_property))
        .route("/", get(list_properties))
}

/// `GET /api/v1/properties/keys?cursor=&limit=`
///
/// Returns a paginated list of distinct top-level property keys.
/// Validates inputs *before* calling the repository so the repo
/// sees only well-formed values.
#[instrument(skip(_state, block_repo))]
pub async fn list_property_keys(
    Query(params): Query<PropertyKeysParams>,
    Extension(_state): Extension<AppState>,
    Extension(block_repo): Extension<Arc<dyn BlockRepository>>,
) -> Result<Json<PropertyKeysResponse>, AppError> {
    // 1. Validate `limit`. Out-of-range → 400. The trait trusts the
    //    caller for `limit`, so this is the only place we enforce
    //    the bound.
    if params.limit < MIN_LIMIT || params.limit > MAX_LIMIT {
        return Err(AppError::BadRequest(format!(
            "limit must be between {MIN_LIMIT} and {MAX_LIMIT}"
        )));
    }

    // 2. Validate `cursor`: an empty string is a client error. The
    //    spec says "empty cursor string rejected" — distinct from
    //    "cursor past end" which is a valid forward-past-end
    //    request and returns an empty page.
    if let Some(ref c) = params.cursor
        && c.is_empty()
    {
        return Err(AppError::BadRequest("cursor must not be empty".to_string()));
    }

    // 3. Query the repository.
    let keys = block_repo
        .list_distinct_keys(params.cursor.as_deref(), params.limit)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // 4. Compute `nextCursor`: only set when the page might have more
    //    results. We use the `keys.len() == limit` heuristic — if we
    //    returned fewer, this is definitively the last page.
    let next_cursor = if keys.len() == params.limit as usize {
        keys.last().cloned()
    } else {
        None
    };

    Ok(Json(PropertyKeysResponse { keys, next_cursor }))
}

// ── PI-3: Batch & list property definitions ──

/// Request body for `POST /api/v1/properties/batch`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchPropertiesRequest {
    /// Specific keys to fetch.
    #[serde(default)]
    pub keys: Vec<String>,
    /// Substring search query (matches key or title).
    pub query: Option<String>,
    /// Max results. Default: 50.
    #[serde(default = "default_limit")]
    pub limit: u32,
}

/// Response body for batch and list endpoints.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertiesResponse {
    pub definitions: Vec<PropertyDefinition>,
    pub count: usize,
}

/// `POST /api/v1/properties/batch`
///
/// Accepts optional `keys` to fetch by name, and/or a `query` for
/// substring search. If both are present, results are merged.
#[instrument(skip(_state, prop_repo))]
pub async fn batch_properties(
    Extension(_state): Extension<AppState>,
    Extension(prop_repo): Extension<Arc<dyn PropertyRepository>>,
    Json(req): Json<BatchPropertiesRequest>,
) -> Result<Json<PropertiesResponse>, AppError> {
    let service = PropertyService::new(prop_repo);

    let limit = req.limit.min(100) as usize;
    let mut results = Vec::new();

    // Fetch by specific keys
    if !req.keys.is_empty() {
        let by_keys = service
            .batch_get(&req.keys)
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?;
        results.extend(by_keys);
    }

    // Search by query
    if let Some(ref query) = req.query {
        if !query.is_empty() {
            let searched = service
                .search(query, limit)
                .await
                .map_err(|e| AppError::BadRequest(e.to_string()))?;
            // Merge without duplicates (by db_ident)
            let existing_keys: std::collections::HashSet<_> =
                results.iter().map(|d| d.db_ident.clone()).collect();
            for def in searched {
                if !existing_keys.contains(&def.db_ident) {
                    results.push(def);
                }
            }
        }
    }

    // If no keys and no query, list by usage
    if req.keys.is_empty() && req.query.as_ref().map_or(true, |q| q.is_empty()) {
        results = service
            .list_by_usage(limit)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }

    let count = results.len();
    Ok(Json(PropertiesResponse {
        definitions: results,
        count,
    }))
}

/// `GET /api/v1/properties`
///
/// Returns all property definitions.
#[instrument(skip(_state, prop_repo))]
pub async fn list_properties(
    Extension(_state): Extension<AppState>,
    Extension(prop_repo): Extension<Arc<dyn PropertyRepository>>,
) -> Result<Json<PropertiesResponse>, AppError> {
    let service = PropertyService::new(prop_repo);

    let definitions = service
        .list_all()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let count = definitions.len();
    Ok(Json(PropertiesResponse {
        definitions,
        count,
    }))
}

// ── PI-4: Property suggestions (discovery UX) ──

/// Query params for `GET /api/v1/properties/suggest`.
#[derive(Debug, Deserialize)]
pub struct SuggestParams {
    /// Partial input to match against keys and titles.
    pub q: String,
    /// Max results. Default: 10.
    #[serde(default = "default_suggest_limit")]
    pub limit: u32,
}

fn default_suggest_limit() -> u32 {
    10
}

/// Response body for suggest endpoint.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestResponse {
    pub suggestions: Vec<PropertySuggestion>,
    pub count: usize,
}

/// `GET /api/v1/properties/suggest?q=sta&limit=10`
///
/// Returns property suggestions ranked by relevance (prefix match >
/// substring match > usage count). Only active properties are returned.
#[instrument(skip(_state, prop_repo))]
pub async fn suggest_properties(
    Query(params): Query<SuggestParams>,
    Extension(_state): Extension<AppState>,
    Extension(prop_repo): Extension<Arc<dyn PropertyRepository>>,
) -> Result<Json<SuggestResponse>, AppError> {
    if params.q.is_empty() {
        return Ok(Json(SuggestResponse {
            suggestions: vec![],
            count: 0,
        }));
    }

    let limit = params.limit.min(50) as usize;
    let service = PropertyService::new(prop_repo);

    let suggestions = service
        .suggest(&params.q, limit)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let count = suggestions.len();
    Ok(Json(SuggestResponse {
        suggestions,
        count,
    }))
}

// ── PI-5: Property Analytics ──

/// Query params for `GET /api/v1/properties/analytics`.
#[derive(Debug, Deserialize)]
pub struct AnalyticsQueryParams {
    /// Max co-occurrence pairs. Default: 20.
    #[serde(default = "default_co_occurrence_limit")]
    pub co_occurrence_limit: u32,
    /// Max trending properties. Default: 20.
    #[serde(default = "default_trend_limit")]
    pub trend_limit: u32,
    /// Period in days for trend comparison. Default: 30.
    #[serde(default = "default_trend_period")]
    pub trend_period_days: u32,
}

fn default_co_occurrence_limit() -> u32 {
    20
}
fn default_trend_limit() -> u32 {
    20
}
fn default_trend_period() -> u32 {
    30
}

/// `GET /api/v1/properties/analytics`
///
/// Returns co-occurrence pairs (with PMI scores), usage trends,
/// and aggregate statistics about property usage.
#[instrument(skip(_state, prop_repo))]
pub async fn analytics_properties(
    Query(params): Query<AnalyticsQueryParams>,
    Extension(_state): Extension<AppState>,
    Extension(prop_repo): Extension<Arc<dyn PropertyRepository>>,
) -> Result<Json<quilt_domain::properties::analytics::PropertyAnalytics>, AppError> {
    let analytics_params = AnalyticsParams {
        co_occurrence_limit: params.co_occurrence_limit.min(100) as usize,
        trend_limit: params.trend_limit.min(100) as usize,
        trend_period_days: params.trend_period_days.min(365),
    };

    let service = PropertyService::new(prop_repo);

    let result = service
        .analytics(&analytics_params)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    Ok(Json(result))
}

// ── PI-6: Lifecycle management ──

/// `POST /api/v1/properties/lifecycle/deprecate`
#[derive(Debug, Deserialize)]
pub struct DeprecateRequest {
    pub key: String,
}

#[instrument(skip(_state, prop_repo))]
pub async fn deprecate_property(
    Extension(_state): Extension<AppState>,
    Extension(prop_repo): Extension<Arc<dyn PropertyRepository>>,
    Json(body): Json<DeprecateRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let service = PropertyService::new(prop_repo);
    let def = service
        .deprecate(&body.key)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    Ok(Json(serde_json::to_value(def).unwrap()))
}

/// `POST /api/v1/properties/lifecycle/merge`
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeRequest {
    pub source_key: String,
    pub target_key: String,
}

#[instrument(skip(_state, prop_repo))]
pub async fn merge_property(
    Extension(_state): Extension<AppState>,
    Extension(prop_repo): Extension<Arc<dyn PropertyRepository>>,
    Json(body): Json<MergeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let service = PropertyService::new(prop_repo);
    let def = service
        .merge(&body.source_key, &body.target_key)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    Ok(Json(serde_json::to_value(def).unwrap()))
}

/// `POST /api/v1/properties/lifecycle/alias`
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AliasRequest {
    pub new_key: String,
    pub target_key: String,
}

#[instrument(skip(_state, prop_repo))]
pub async fn alias_property(
    Extension(_state): Extension<AppState>,
    Extension(prop_repo): Extension<Arc<dyn PropertyRepository>>,
    Json(body): Json<AliasRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let service = PropertyService::new(prop_repo);
    let def = service
        .alias(&body.new_key, &body.target_key)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    Ok(Json(serde_json::to_value(def).unwrap()))
}
