//! Tour state use cases (B of `quilt-fase4-cross-device-tour`).
//!
//! Tracks which first-run product tours (Welcome, Cognitive, MCP) the
//! current user has dismissed so the dismissal syncs across devices.
//! The api-key is the user identifier for V1 — there is no real
//! `User` entity in Quilt yet (F6 of the backlog).
//!
//! The use case is intentionally thin: presentation layers pass an
//! opaque `user_id` (typically the api key from the Authorization
//! header) and the use case delegates to the repository. Validation
//! of tour names lives here so the HTTP layer doesn't have to repeat
//! it on every request.
//!
//! # Why a use case and not a direct repo call from the handler?
//!
//! Consistency with the rest of the codebase (template, page, block
//! use cases all live in the application layer) and a place to hang
//! future logic (e.g. "auto-dismiss welcome after N days") without
//! touching the HTTP handler.

use crate::errors::ApplicationError;
use async_trait::async_trait;
use quilt_domain::repositories::TourStateRepository;
use std::sync::Arc;
use tracing::instrument;

/// Maximum tour-name length accepted by the use case. Tour names are
/// short slugs (`"welcome"`, `"cognitive"`, `"mcp"`) — anything longer
/// than 64 chars is almost certainly a bug or an attack.
const MAX_TOUR_NAME_LEN: usize = 64;

/// Use cases for cross-device tour-dismissal state.
///
/// Object-safe (`Send + Sync`) so the server can hold it as
/// `Arc<dyn TourStateUseCases>` next to the other use cases.
#[async_trait]
pub trait TourStateUseCases: Send + Sync {
    /// List the tour names the user has dismissed. Empty `Vec` when
    /// the user has dismissed nothing yet.
    async fn get_dismissed_tours(&self, user_id: &str) -> Result<Vec<String>, ApplicationError>;

    /// Mark a single tour as dismissed. Idempotent.
    ///
    /// Returns `Validation` when `tour_name` is empty, contains
    /// control characters, or is longer than [`MAX_TOUR_NAME_LEN`].
    async fn dismiss_tour(&self, user_id: &str, tour_name: &str) -> Result<(), ApplicationError>;
}

/// Generic implementation backed by any [`TourStateRepository`].
pub struct TourStateUseCasesImpl<R: TourStateRepository> {
    repo: Arc<R>,
}

impl<R: TourStateRepository> TourStateUseCasesImpl<R> {
    /// Create a new `TourStateUseCasesImpl` with the given repository.
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: TourStateRepository + 'static> TourStateUseCases for TourStateUseCasesImpl<R> {
    #[instrument(skip(self))]
    async fn get_dismissed_tours(&self, user_id: &str) -> Result<Vec<String>, ApplicationError> {
        if user_id.is_empty() {
            return Err(ApplicationError::Validation(
                "user_id must not be empty".to_string(),
            ));
        }
        let tours = self
            .repo
            .get_dismissed_tours(user_id)
            .await
            .map_err(ApplicationError::Domain)?;
        Ok(tours)
    }

    #[instrument(skip(self))]
    async fn dismiss_tour(&self, user_id: &str, tour_name: &str) -> Result<(), ApplicationError> {
        if user_id.is_empty() {
            return Err(ApplicationError::Validation(
                "user_id must not be empty".to_string(),
            ));
        }
        let trimmed = tour_name.trim();
        if trimmed.is_empty() {
            return Err(ApplicationError::Validation(
                "tour name must not be empty".to_string(),
            ));
        }
        if trimmed.len() > MAX_TOUR_NAME_LEN {
            return Err(ApplicationError::Validation(format!(
                "tour name too long (max {MAX_TOUR_NAME_LEN} chars)"
            )));
        }
        // Reject control characters and whitespace inside the name —
        // tour names are slugs, not free-form strings.
        if trimmed.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err(ApplicationError::Validation(
                "tour name must not contain control characters or whitespace".to_string(),
            ));
        }
        self.repo
            .dismiss_tour(user_id, trimmed)
            .await
            .map_err(ApplicationError::Domain)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use parking_lot::Mutex;
    use quilt_domain::errors::DomainError;
    use std::collections::HashMap;

    /// In-memory fake of the repository for unit tests.
    /// Records every (user, tour) pair it has ever seen, so we can
    /// assert on the persistence side without spinning up SQLite.
    #[derive(Default)]
    struct FakeRepo {
        /// `user -> set of dismissed tour names`
        store: Mutex<HashMap<String, Vec<String>>>,
        /// Force every call to fail — for the error path.
        fail: bool,
    }

    #[async_trait]
    impl TourStateRepository for FakeRepo {
        async fn get_dismissed_tours(&self, user_id: &str) -> Result<Vec<String>, DomainError> {
            if self.fail {
                return Err(DomainError::Storage("forced".to_string()));
            }
            Ok(self.store.lock().get(user_id).cloned().unwrap_or_default())
        }

        async fn dismiss_tour(&self, user_id: &str, tour_name: &str) -> Result<(), DomainError> {
            if self.fail {
                return Err(DomainError::Storage("forced".to_string()));
            }
            let mut store = self.store.lock();
            let entry = store.entry(user_id.to_string()).or_default();
            if !entry.iter().any(|t| t == tour_name) {
                entry.push(tour_name.to_string());
            }
            Ok(())
        }
    }

    fn make_uc() -> (TourStateUseCasesImpl<FakeRepo>, Arc<FakeRepo>) {
        let repo = Arc::new(FakeRepo::default());
        let uc = TourStateUseCasesImpl::new(repo.clone());
        (uc, repo)
    }

    #[tokio::test]
    async fn get_dismissed_returns_empty_for_new_user() {
        let (uc, _) = make_uc();
        let tours = uc.get_dismissed_tours("user-1").await.unwrap();
        assert!(tours.is_empty());
    }

    #[tokio::test]
    async fn dismiss_then_get_round_trips() {
        let (uc, _) = make_uc();
        uc.dismiss_tour("user-1", "welcome").await.unwrap();
        let tours = uc.get_dismissed_tours("user-1").await.unwrap();
        assert_eq!(tours, vec!["welcome".to_string()]);
    }

    #[tokio::test]
    async fn dismiss_is_idempotent() {
        let (uc, repo) = make_uc();
        uc.dismiss_tour("user-1", "welcome").await.unwrap();
        uc.dismiss_tour("user-1", "welcome").await.unwrap();
        uc.dismiss_tour("user-1", "welcome").await.unwrap();
        let tours = uc.get_dismissed_tours("user-1").await.unwrap();
        assert_eq!(tours, vec!["welcome".to_string()]);
        // The fake dedupes in the in-memory map, but the real repo
        // relies on the SQL `INSERT OR REPLACE`. The use case's
        // contract is "no duplicates in the response" — the fake
        // proves the use case doesn't accidentally introduce them.
        assert_eq!(repo.store.lock().get("user-1").unwrap().len(), 1);
    }

    #[tokio::test]
    async fn multiple_tours_per_user_are_preserved() {
        let (uc, _) = make_uc();
        uc.dismiss_tour("u", "welcome").await.unwrap();
        uc.dismiss_tour("u", "cognitive").await.unwrap();
        uc.dismiss_tour("u", "mcp").await.unwrap();
        let mut tours = uc.get_dismissed_tours("u").await.unwrap();
        tours.sort();
        assert_eq!(
            tours,
            vec![
                "cognitive".to_string(),
                "mcp".to_string(),
                "welcome".to_string()
            ]
        );
    }

    #[tokio::test]
    async fn users_are_isolated() {
        let (uc, _) = make_uc();
        uc.dismiss_tour("alice", "welcome").await.unwrap();
        let bob = uc.get_dismissed_tours("bob").await.unwrap();
        assert!(bob.is_empty());
    }

    #[tokio::test]
    async fn empty_user_id_rejected_on_get() {
        let (uc, _) = make_uc();
        let err = uc.get_dismissed_tours("").await.unwrap_err();
        assert!(matches!(err, ApplicationError::Validation(_)));
    }

    #[tokio::test]
    async fn empty_user_id_rejected_on_dismiss() {
        let (uc, _) = make_uc();
        let err = uc.dismiss_tour("", "welcome").await.unwrap_err();
        assert!(matches!(err, ApplicationError::Validation(_)));
    }

    #[tokio::test]
    async fn empty_tour_name_rejected() {
        let (uc, _) = make_uc();
        let err = uc.dismiss_tour("u", "").await.unwrap_err();
        assert!(matches!(err, ApplicationError::Validation(_)));
    }

    #[tokio::test]
    async fn whitespace_only_tour_name_rejected() {
        let (uc, _) = make_uc();
        let err = uc.dismiss_tour("u", "   ").await.unwrap_err();
        assert!(matches!(err, ApplicationError::Validation(_)));
    }

    #[tokio::test]
    async fn tour_name_with_internal_whitespace_rejected() {
        let (uc, _) = make_uc();
        let err = uc.dismiss_tour("u", "we lcome").await.unwrap_err();
        assert!(matches!(err, ApplicationError::Validation(_)));
    }

    #[tokio::test]
    async fn tour_name_with_newline_rejected() {
        let (uc, _) = make_uc();
        let err = uc.dismiss_tour("u", "we\nlcome").await.unwrap_err();
        assert!(matches!(err, ApplicationError::Validation(_)));
    }

    #[tokio::test]
    async fn tour_name_too_long_rejected() {
        let (uc, _) = make_uc();
        let too_long = "a".repeat(MAX_TOUR_NAME_LEN + 1);
        let err = uc.dismiss_tour("u", &too_long).await.unwrap_err();
        assert!(matches!(err, ApplicationError::Validation(_)));
    }

    #[tokio::test]
    async fn tour_name_at_max_length_accepted() {
        let (uc, _) = make_uc();
        let name = "a".repeat(MAX_TOUR_NAME_LEN);
        uc.dismiss_tour("u", &name).await.unwrap();
        let tours = uc.get_dismissed_tours("u").await.unwrap();
        assert_eq!(tours, vec![name]);
    }

    #[tokio::test]
    async fn leading_trailing_whitespace_trimmed() {
        let (uc, _) = make_uc();
        uc.dismiss_tour("u", "  welcome  ").await.unwrap();
        let tours = uc.get_dismissed_tours("u").await.unwrap();
        assert_eq!(tours, vec!["welcome".to_string()]);
    }

    #[tokio::test]
    async fn repo_error_is_propagated() {
        let repo = Arc::new(FakeRepo {
            fail: true,
            ..Default::default()
        });
        let uc = TourStateUseCasesImpl::new(repo.clone());
        let err = uc.get_dismissed_tours("u").await.unwrap_err();
        assert!(matches!(err, ApplicationError::Domain(_)));
        let err = uc.dismiss_tour("u", "welcome").await.unwrap_err();
        assert!(matches!(err, ApplicationError::Domain(_)));
    }
}
