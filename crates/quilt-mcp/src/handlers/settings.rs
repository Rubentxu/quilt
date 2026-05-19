//! Settings handler implementation for MCP tools.
//!
//! Implements [`SettingsHandler`](super::SettingsHandler) trait for settings
//! MCP tools like get_settings, update_settings.

use super::{HandlerResult, SettingsHandler as SettingsHandlerTrait};
use async_trait::async_trait;
use quilt_domain::repositories::SettingsRepository;
use std::sync::Arc;
use tracing::instrument;

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

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "timezone": settings.timezone,
            "journal_format": settings.journal_format,
            "start_of_week": settings.start_of_week,
            "preferred_format": format!("{:?}", settings.preferred_format),
        }))
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

        Ok(serde_json::json!({
            "status": "updated",
        })
        .to_string())
    }
}
