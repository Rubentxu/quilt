//! Settings handler implementation for MCP tools.
//!
//! Implements [`SettingsHandler`](super::SettingsHandler) trait for settings
//! MCP tools like get_settings, update_settings.

use super::{HandlerResult, SettingsHandler as SettingsHandlerTrait};
use async_trait::async_trait;
use quilt_domain::repositories::SettingsRepository;
use serde::Serialize;
use std::sync::Arc;
use tracing::instrument;

// ── Local wire-format DTOs ────────────────────────────────────────
//
// These mirror the existing `serde_json::json!({ ... })` shapes used
// by the handler responses. The `preferred_format` field is rendered
// via `format!("{:?}", ...)` (lowercase Debug) to preserve the
// previous wire-format string (e.g. `markdown`, `org_mode`).

/// Wire shape for the `get_settings` response. `UserSettings` itself
/// has `#[derive(Serialize)]`, but the previous handler also stringified
/// `preferred_format` via `format!("{:?}", ...).to_lowercase()` — this
/// DTO preserves that exact rendering.
#[derive(Serialize)]
struct GetSettingsResponse {
    timezone: String,
    journal_format: String,
    start_of_week: u8,
    preferred_format: String,
}

/// Wire shape for the `update_settings` response.
#[derive(Serialize)]
struct UpdateSettingsResponse {
    status: &'static str,
}

/// Default implementation of [`SettingsHandler`].
pub struct DefaultSettingsHandler {
    settings_repo: Arc<dyn SettingsRepository>,
}

impl DefaultSettingsHandler {
    /// Create a new settings handler.
    pub fn new(settings_repo: Arc<dyn SettingsRepository>) -> Self {
        Self { settings_repo }
    }
}

#[async_trait]
impl SettingsHandlerTrait for DefaultSettingsHandler {
    #[instrument(skip(self))]
    async fn get_settings(&self) -> HandlerResult {
        let settings = self
            .settings_repo
            .get_user_settings()
            .await
            .map_err(|e| e.to_string())?;

        let response = GetSettingsResponse {
            timezone: settings.timezone,
            journal_format: settings.journal_format,
            start_of_week: settings.start_of_week,
            preferred_format: format!("{:?}", settings.preferred_format),
        };
        Ok(serde_json::to_string_pretty(&response)
            .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    #[instrument(skip(self))]
    async fn update_settings(&self, settings_json: serde_json::Value) -> HandlerResult {
        // Parse settings from JSON manually since UserSettings doesn't implement Deserialize
        let timezone = settings_json
            .get("timezone")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing or invalid timezone".to_string())?;
        let journal_format = settings_json
            .get("journal_format")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing or invalid journal_format".to_string())?;
        let start_of_week = settings_json
            .get("start_of_week")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| "Missing or invalid start_of_week".to_string())?
            as u8;
        let preferred_format_str = settings_json
            .get("preferred_format")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing or invalid preferred_format".to_string())?;

        let settings = quilt_domain::entities::UserSettings {
            timezone: timezone.to_string(),
            journal_format: journal_format.to_string(),
            start_of_week,
            preferred_format: quilt_domain::value_objects::BlockFormat::from_str(
                preferred_format_str,
            )
            .ok_or_else(|| format!("Invalid preferred_format: {}", preferred_format_str))?,
        };

        self.settings_repo
            .update_user_settings(&settings)
            .await
            .map_err(|e| e.to_string())?;

        let response = UpdateSettingsResponse { status: "updated" };
        serde_json::to_string(&response).map_err(|e| e.to_string())
    }
}
