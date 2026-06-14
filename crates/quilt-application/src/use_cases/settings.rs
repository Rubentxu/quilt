//! Settings use cases
//!
//! Implements [`SettingsUseCases`] for reading and updating user preferences.
//!
//! The `journal_format` is the bug fix: prior to this refactor
//! `PageUseCases::get_or_create_journal` hardcoded `"%Y-%m-%d"` instead of
//! reading the user's configured `journal_format`. Use cases that need to
//! respect the user's setting now inject a [`SettingsRepository`] (or
//! accept a `journal_format: &str` argument sourced from this use case)
//! so the live value always wins.

use crate::errors::ApplicationError;
use async_trait::async_trait;
use quilt_domain::entities::UserSettings;
use quilt_domain::repositories::SettingsRepository;
use quilt_domain::value_objects::BlockFormat;
use std::sync::Arc;
use tracing::instrument;

/// Settings use cases trait - read and update user preferences.
///
/// This trait is object-safe (`Send + Sync`) and uses `#[async_trait]`
/// for async ergonomics.
#[async_trait]
pub trait SettingsUseCases: Send + Sync {
    /// Fetch the current persisted settings.
    async fn get(&self) -> Result<UserSettings, ApplicationError>;

    /// Apply a partial update to the user's settings. Fields set to
    /// `None` keep their current value.
    async fn update(
        &self,
        timezone: Option<String>,
        journal_format: Option<String>,
        start_of_week: Option<u8>,
        preferred_format: Option<BlockFormat>,
    ) -> Result<UserSettings, ApplicationError>;

    /// List the common date format options exposed in the UI.
    ///
    /// Static — does not touch the repository. Mirrors
    /// [`UserSettings::common_date_formats`].
    fn list_date_formats(&self) -> Vec<(String, String)>;
}

/// Implementation of [`SettingsUseCases`] for any [`SettingsRepository`].
pub struct SettingsUseCasesImpl<R: SettingsRepository> {
    repo: Arc<R>,
}

impl<R: SettingsRepository> SettingsUseCasesImpl<R> {
    /// Create a new use-case instance backed by the given repository.
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: SettingsRepository + 'static> SettingsUseCases for SettingsUseCasesImpl<R> {
    #[instrument(skip(self))]
    async fn get(&self) -> Result<UserSettings, ApplicationError> {
        self.repo.get_user_settings().await.map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn update(
        &self,
        timezone: Option<String>,
        journal_format: Option<String>,
        start_of_week: Option<u8>,
        preferred_format: Option<BlockFormat>,
    ) -> Result<UserSettings, ApplicationError> {
        let current = self
            .repo
            .get_user_settings()
            .await
            .map_err(ApplicationError::Domain)?;

        let updated = UserSettings {
            timezone: timezone.unwrap_or(current.timezone),
            journal_format: journal_format.unwrap_or(current.journal_format),
            start_of_week: start_of_week.unwrap_or(current.start_of_week),
            preferred_format: preferred_format.unwrap_or(current.preferred_format),
        };

        updated
            .validate()
            .map_err(|e| ApplicationError::Validation(e.to_string()))?;

        self.repo
            .update_user_settings(&updated)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(updated)
    }

    fn list_date_formats(&self) -> Vec<(String, String)> {
        UserSettings::common_date_formats()
            .into_iter()
            .map(|(p, e)| (p.to_string(), e.to_string()))
            .collect()
    }
}
