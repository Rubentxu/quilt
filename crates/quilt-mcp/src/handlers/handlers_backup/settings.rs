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
            .get()
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "settings": settings,
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    #[instrument(skip(self))]
    async fn update_settings(&self, settings: serde_json::Value) -> HandlerResult {
        self.settings_repo
            .update(settings)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::json!({
            "status": "updated",
        })
        .to_string())
    }
}
