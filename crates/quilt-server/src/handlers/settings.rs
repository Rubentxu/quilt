//! Settings HTTP handlers

use axum::extract::Extension;
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::error::AppError;
use quilt_domain::entities::UserSettings;
use quilt_domain::repositories::SettingsRepository;
use std::sync::Arc;

use quilt_domain::value_objects::BlockFormat;

/// A date format option returned by the `/formats` endpoint
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DateFormatOption {
    /// The strftime format pattern (e.g., "%Y-%m-%d")
    pub pattern: &'static str,
    /// An example output for May 14, 2026
    pub example: &'static str,
}

/// Create router for /api/v1/settings
pub fn routes() -> Router {
    Router::new()
        .route("/", get(get_settings).put(update_settings))
        .route("/formats", get(get_date_formats))
}

/// GET /api/v1/settings/formats
///
/// Returns available date format options for journal pages.
#[instrument]
pub async fn get_date_formats() -> Json<Vec<DateFormatOption>> {
    Json(
        UserSettings::common_date_formats()
            .into_iter()
            .map(|(pattern, example)| DateFormatOption { pattern, example })
            .collect(),
    )
}

/// GET /api/v1/settings
#[instrument(skip(settings_repo))]
pub async fn get_settings(
    Extension(settings_repo): Extension<Arc<dyn SettingsRepository>>,
) -> Result<Json<UserSettings>, AppError> {
    let settings = settings_repo
        .get_user_settings()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(settings))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsRequest {
    pub timezone: Option<String>,
    pub journal_format: Option<String>,
    pub start_of_week: Option<u8>,
    pub preferred_format: Option<String>,
    pub journal_aggregate: Option<bool>,
}

/// PUT /api/v1/settings
///
/// Partial update: only provided fields are updated. Missing fields keep their current value.
#[instrument(skip(settings_repo))]
pub async fn update_settings(
    Extension(settings_repo): Extension<Arc<dyn SettingsRepository>>,
    Json(req): Json<UpdateSettingsRequest>,
) -> Result<Json<UserSettings>, AppError> {
    // Fetch current settings
    let current = settings_repo
        .get_user_settings()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Merge with updates
    let updated = UserSettings {
        timezone: req.timezone.unwrap_or(current.timezone),
        journal_format: req.journal_format.unwrap_or(current.journal_format),
        start_of_week: req.start_of_week.unwrap_or(current.start_of_week),
        preferred_format: req
            .preferred_format
            .and_then(|f| BlockFormat::parse_str(&f).ok())
            .unwrap_or(current.preferred_format),
        journal_aggregate: req.journal_aggregate.unwrap_or(current.journal_aggregate),
    };

    updated
        .validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    settings_repo
        .update_user_settings(&updated)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(updated))
}
