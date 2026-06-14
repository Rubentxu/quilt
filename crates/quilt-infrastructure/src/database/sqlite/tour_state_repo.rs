//! SQLite implementation of the [`TourStateRepository`] trait.

use async_trait::async_trait;
use sqlx::Row;

use crate::database::sqlite::connection::DbPool;
use crate::errors::map_sqlx_error;
use quilt_domain::errors::DomainError;
use quilt_domain::repositories::TourStateRepository;

/// SQLite implementation of the [`TourStateRepository`] trait.
///
/// Persists tour-dismissal state in the `tour_dismissals` table added
/// by [`connection::run_migrations`]. The user identifier is an opaque
/// string (V1: the api key from the `Authorization` header) — we do
/// not validate it here because the auth middleware has already
/// accepted it by the time we get a request.
#[derive(Clone)]
pub struct SqliteTourStateRepository {
    pool: DbPool,
}

impl SqliteTourStateRepository {
    /// Creates a new `SqliteTourStateRepository` with the given connection pool.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TourStateRepository for SqliteTourStateRepository {
    async fn get_dismissed_tours(&self, user_id: &str) -> Result<Vec<String>, DomainError> {
        let rows = sqlx::query(
            "SELECT tour_name FROM tour_dismissals WHERE user_id = ? ORDER BY tour_name",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_sqlx_error("get_dismissed_tours", e))?;

        Ok(rows
            .iter()
            .map(|r| r.get::<String, _>("tour_name"))
            .collect())
    }

    async fn dismiss_tour(&self, user_id: &str, tour_name: &str) -> Result<(), DomainError> {
        // `INSERT OR REPLACE` makes the operation idempotent. The
        // composite primary key `(user_id, tour_name)` is what makes
        // the conflict detection work — a second dismissal of the
        // same pair updates the `dismissed_at` timestamp rather than
        // raising a constraint error.
        sqlx::query(
            "INSERT INTO tour_dismissals (user_id, tour_name, dismissed_at) \
             VALUES (?, ?, unixepoch('now')) \
             ON CONFLICT(user_id, tour_name) DO UPDATE SET \
             dismissed_at = excluded.dismissed_at",
        )
        .bind(user_id)
        .bind(tour_name)
        .execute(&self.pool)
        .await
        .map_err(|e| map_sqlx_error("dismiss_tour", e))?;

        Ok(())
    }
}
