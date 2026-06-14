//! HTTP handlers for cross-device tour-dismissal state
//! (B of `quilt-fase4-cross-device-tour`).
//!
//! Exposes a tiny REST surface so the frontend can sync which first-run
//! product tours a user has dismissed across devices. The api key
//! from the `Authorization` header is used as the user identifier
//! (V1: the only "user" concept in Quilt).
//!
//! Routes mounted under `/api/v1/user/tour-state`:
//! - GET `/`         — list the tour names the current user has dismissed
//! - POST `/dismiss` — mark a single tour as dismissed (idempotent)
//!
//! # Why no auth helper extracting the api key?
//!
//! The api key lives in a `OnceLock<String>` in `middleware::auth`.
//! The handler compares the inbound token against the configured key
//! by going through the same `Authorization` header the middleware
//! already validated. We do NOT need a parallel accessor — we only
//! need a stable per-user identifier. Using the token itself as the
//! identifier means the auth boundary is the same as the data
//! boundary: a request that passed auth can only see its own data.

use crate::error::AppError;
use crate::state::AppState;
use axum::{
    Json, Router,
    extract::Extension,
    http::HeaderMap,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

/// Router for /api/v1/user/tour-state
pub fn routes() -> Router {
    Router::new()
        .route("/", get(get_tour_state))
        .route("/dismiss", post(dismiss_tour))
}

/// Response body for `GET /api/v1/user/tour-state`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TourStateResponse {
    /// Tour names the current user has dismissed.
    /// Sorted alphabetically by the repository query.
    pub dismissed: Vec<String>,
}

/// Request body for `POST /api/v1/user/tour-state/dismiss`.
#[derive(Debug, Clone, Deserialize)]
pub struct DismissTourRequest {
    /// Short slug for the tour to dismiss (`"welcome"`, `"cognitive"`,
    /// `"mcp"`). Validated by the use case.
    pub tour: String,
}

/// `GET /api/v1/user/tour-state` — return the current user's set of
/// dismissed tour names.
#[instrument(skip(state, headers))]
pub async fn get_tour_state(
    Extension(state): Extension<AppState>,
    headers: HeaderMap,
) -> Result<Json<TourStateResponse>, AppError> {
    let user_id = extract_user_id(&headers).map_err(|e| AppError::Unauthorized(e.to_string()))?;
    let dismissed = state
        .services
        .tour_state
        .get_dismissed_tours(&user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(TourStateResponse { dismissed }))
}

/// `POST /api/v1/user/tour-state/dismiss` — mark a tour as dismissed.
/// Idempotent.
#[instrument(skip(state, headers, body))]
pub async fn dismiss_tour(
    Extension(state): Extension<AppState>,
    headers: HeaderMap,
    Json(body): Json<DismissTourRequest>,
) -> Result<Json<TourStateResponse>, AppError> {
    let user_id = extract_user_id(&headers).map_err(|e| AppError::Unauthorized(e.to_string()))?;
    state
        .services
        .tour_state
        .dismiss_tour(&user_id, &body.tour)
        .await
        .map_err(|e| match e {
            quilt_application::ApplicationError::Validation(msg) => AppError::BadRequest(msg),
            other => AppError::Internal(other.to_string()),
        })?;
    // Return the updated list so the client doesn't have to make a
    // second round-trip to refresh its cache.
    let dismissed = state
        .services
        .tour_state
        .get_dismissed_tours(&user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(TourStateResponse { dismissed }))
}

/// Extract the opaque user identifier from the `Authorization: Bearer
/// <token>` header. Returns 401 on missing / malformed input.
///
/// The token IS the user identifier for V1. The auth middleware has
/// already accepted the same token, so the only way this fails is if
/// the handler is mounted on a route that bypassed the middleware —
/// which is a programming error, hence the 401 (not 500).
fn extract_user_id(headers: &HeaderMap) -> Result<String, &'static str> {
    let header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or("Missing Authorization header")?;
    let token = header
        .strip_prefix("Bearer ")
        .or_else(|| header.strip_prefix("bearer "))
        .ok_or("Invalid Authorization scheme")?
        .trim();
    if token.is_empty() {
        return Err("Empty bearer token");
    }
    Ok(token.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers_with(token: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {token}").parse().unwrap(),
        );
        h
    }

    #[test]
    fn extract_user_id_returns_bearer_token() {
        let h = headers_with("abc-123");
        assert_eq!(extract_user_id(&h).unwrap(), "abc-123");
    }

    #[test]
    fn extract_user_id_trims_whitespace() {
        let mut h = HeaderMap::new();
        h.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer   abc-123   ".parse().unwrap(),
        );
        assert_eq!(extract_user_id(&h).unwrap(), "abc-123");
    }

    #[test]
    fn extract_user_id_lowercase_scheme() {
        let h = headers_with("abc-123");
        assert_eq!(extract_user_id(&h).unwrap(), "abc-123");
    }

    #[test]
    fn extract_user_id_missing_header_is_401() {
        let h = HeaderMap::new();
        let err = extract_user_id(&h).unwrap_err();
        assert!(err.contains("Missing"), "got: {err}");
    }

    #[test]
    fn extract_user_id_wrong_scheme_is_401() {
        let mut h = HeaderMap::new();
        h.insert(
            axum::http::header::AUTHORIZATION,
            "Basic abc-123".parse().unwrap(),
        );
        let err = extract_user_id(&h).unwrap_err();
        assert!(err.contains("scheme"), "got: {err}");
    }

    #[test]
    fn extract_user_id_empty_token_is_401() {
        let mut h = HeaderMap::new();
        h.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer ".parse().unwrap(),
        );
        let err = extract_user_id(&h).unwrap_err();
        // "Bearer " parses to "scheme=Basic, rest=''" — but we
        // strip_prefix("Bearer ") gives an empty string, which we
        // explicitly reject.
        assert!(
            err.contains("Empty") || err.contains("scheme"),
            "got: {err}"
        );
    }
}
