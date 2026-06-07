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
    routing::get,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::error::AppError;
use crate::state::AppState;
use quilt_domain::repositories::BlockRepository;
use quilt_infrastructure::database::sqlite::repositories::SqliteBlockRepository;

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
    Router::new().route("/keys", get(list_property_keys))
}

/// `GET /api/v1/properties/keys?cursor=&limit=`
///
/// Returns a paginated list of distinct top-level property keys.
/// Validates inputs *before* calling the repository so the repo
/// sees only well-formed values.
#[instrument(skip(state))]
pub async fn list_property_keys(
    Query(params): Query<PropertyKeysParams>,
    Extension(state): Extension<AppState>,
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
    let block_repo = SqliteBlockRepository::new(state.pool.clone());
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
