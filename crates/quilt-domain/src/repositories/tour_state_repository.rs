//! TourStateRepository trait - abstraction for tour dismissal persistence
//!
//! Tracks which first-run product tours (Welcome, Cognitive, MCP) a
//! user has dismissed. The repository is keyed by an opaque user
//! identifier — for V1 this is the API key (the only "user" concept
//! in Quilt today); once a real user table exists, this trait stays
//! the same and only the implementation changes.
//!
//! # Design note
//!
//! This is intentionally a tiny, append-friendly interface. Each tour
//! is identified by a short, well-known string (`"welcome"`,
//! `"cognitive"`, `"mcp"`). The repository does NOT know what a tour
//! is, when to show one, or what happens after dismissal — it just
//! records the fact.

use crate::errors::DomainError;
use async_trait::async_trait;

/// Repository for tour-dismissal state.
///
/// Implementations must:
/// - Be safe to call from multiple concurrent requests.
/// - Treat `user_id` as an opaque string (typically the API key).
/// - Be idempotent: dismissing the same tour twice MUST NOT error
///   and MUST NOT create duplicate rows.
#[async_trait]
pub trait TourStateRepository: Send + Sync {
    /// Return the set of tour names dismissed by the given user.
    ///
    /// Returns an empty `Vec` when the user has dismissed nothing yet
    /// or when the user has never been seen. Never errors on a missing
    /// user — that is a normal first-run state.
    async fn get_dismissed_tours(&self, user_id: &str) -> Result<Vec<String>, DomainError>;

    /// Mark `tour_name` as dismissed by `user_id`.
    ///
    /// Idempotent: a second call with the same `(user_id, tour_name)`
    /// pair is a no-op. The current implementation overwrites the
    /// `dismissed_at` timestamp so re-dismissing refreshes the
    /// "last seen" marker.
    async fn dismiss_tour(&self, user_id: &str, tour_name: &str) -> Result<(), DomainError>;
}
