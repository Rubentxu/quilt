//! SQLite-backed settings repository.

use async_trait::async_trait;

use crate::database::sqlite::connection::DbPool;
use crate::errors::map_sqlx_error;
use quilt_domain::entities::UserSettings;
use quilt_domain::errors::DomainError;
use quilt_domain::repositories::SettingsRepository;
use quilt_domain::value_objects::BlockFormat;

/// SQLite-backed settings repository.
///
/// Uses the singleton `user_settings` table (single row with id=1).
/// If the table doesn't exist or no row is found, returns [`UserSettings::default`].
#[derive(Clone)]
pub struct SqliteSettingsRepository {
    pool: DbPool,
}

impl SqliteSettingsRepository {
    /// Creates a new `SqliteSettingsRepository` with the given connection pool.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SettingsRepository for SqliteSettingsRepository {
    async fn get_user_settings(&self) -> Result<UserSettings, DomainError> {
        let row = sqlx::query_as::<_, (String, String, u8, String)>(
            "SELECT timezone, journal_format, start_of_week, preferred_format \
             FROM user_settings WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| map_sqlx_error("get_user_settings", e))?;

        match row {
            Some((timezone, journal_format, start_of_week, preferred_format)) => Ok(UserSettings {
                timezone,
                journal_format,
                start_of_week,
                preferred_format: BlockFormat::parse_str(&preferred_format)
                    .unwrap_or(BlockFormat::Markdown),
            }),
            None => Ok(UserSettings::default()),
        }
    }

    async fn update_user_settings(&self, settings: &UserSettings) -> Result<(), DomainError> {
        let preferred_format = match settings.preferred_format {
            BlockFormat::Markdown => "markdown",
            BlockFormat::Org => "org",
        };

        sqlx::query(
            "INSERT INTO user_settings (id, timezone, journal_format, start_of_week, preferred_format, updated_at) \
             VALUES (1, ?, ?, ?, ?, unixepoch('now')) \
             ON CONFLICT(id) DO UPDATE SET \
             timezone = excluded.timezone, \
             journal_format = excluded.journal_format, \
             start_of_week = excluded.start_of_week, \
             preferred_format = excluded.preferred_format, \
             updated_at = excluded.updated_at",
        )
        .bind(&settings.timezone)
        .bind(&settings.journal_format)
        .bind(settings.start_of_week)
        .bind(preferred_format)
        .execute(&self.pool)
        .await
        .map_err(|e| map_sqlx_error("update_user_settings", e))?;

        Ok(())
    }
}
