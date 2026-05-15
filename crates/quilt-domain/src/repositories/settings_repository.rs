//! SettingsRepository trait - abstraction for user settings persistence

use crate::entities::UserSettings;
use crate::errors::DomainError;
use async_trait::async_trait;

/// Repository for user settings persistence.
///
/// Implementations must ensure thread-safe access to the singleton settings row.
#[async_trait]
pub trait SettingsRepository: Send + Sync {
    /// Get the current user settings.
    ///
    /// If no settings exist, returns default settings.
    async fn get_user_settings(&self) -> Result<UserSettings, DomainError>;

    /// Update user settings.
    ///
    /// This replaces ALL settings fields. Partial updates should
    /// be done by getting current settings, modifying, and saving.
    async fn update_user_settings(&self, settings: &UserSettings) -> Result<(), DomainError>;

    /// Reset settings to defaults.
    async fn reset_to_defaults(&self) -> Result<(), DomainError> {
        self.update_user_settings(&UserSettings::default()).await
    }
}
