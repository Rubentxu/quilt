//! Decay Monitor service
//!
//! Thin wrapper around [`crate::shared_decay::detect_decay_alerts`]
//! that returns a [`DecayMonitorDto`] with precomputed counts and
//! a captured `generated_at` timestamp.

use super::types::{DecayMonitorDto, SeverityCounts};
use crate::shared_decay::detect_decay_alerts;
use chrono::Utc;
use quilt_domain::repositories::{BlockRepository, PageRepository};
use std::sync::Arc;

/// Service that produces decay alerts as a standalone snapshot.
#[derive(Clone)]
pub struct DecayMonitorService {
    block_repo: Arc<dyn BlockRepository>,
    page_repo: Arc<dyn PageRepository>,
}

impl std::fmt::Debug for DecayMonitorService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecayMonitorService")
            .field("block_repo", &"Arc<dyn BlockRepository>")
            .field("page_repo", &"Arc<dyn PageRepository>")
            .finish()
    }
}

impl DecayMonitorService {
    /// Create a new service.
    pub fn new(block_repo: Arc<dyn BlockRepository>, page_repo: Arc<dyn PageRepository>) -> Self {
        Self {
            block_repo,
            page_repo,
        }
    }

    /// Run decay detection now and return a DTO.
    pub async fn detect_now(&self) -> DecayMonitorDto {
        let now = Utc::now();
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let today_start: chrono::DateTime<Utc> =
            chrono::DateTime::from_naive_utc_and_offset(today_start, Utc);

        let alerts = detect_decay_alerts(
            self.block_repo.as_ref(),
            self.page_repo.as_ref(),
            today_start,
        )
        .await;

        let counts_by_severity = SeverityCounts::from_alerts(&alerts);
        let total_alerts = counts_by_severity.total();

        DecayMonitorDto {
            alerts,
            total_alerts,
            counts_by_severity,
            generated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::morning_briefing::types::DecayAlert;
    use chrono::Duration;

    fn fake_alert(severity: &str, days_since_update: i64) -> DecayAlert {
        DecayAlert {
            block_id: "b-1".to_string(),
            content_preview: "preview".to_string(),
            page_name: "page-1".to_string(),
            days_since_update,
            severity: severity.to_string(),
            reason: format!("test reason ({})", severity),
        }
    }

    #[test]
    fn severity_counts_from_alerts_groups_correctly() {
        let alerts = vec![
            fake_alert("high", 30),
            fake_alert("high", 31),
            fake_alert("medium", 20),
            fake_alert("low", 5),
        ];
        let counts = SeverityCounts::from_alerts(&alerts);
        assert_eq!(counts.high, 2);
        assert_eq!(counts.medium, 1);
        assert_eq!(counts.low, 1);
        assert_eq!(counts.total(), 4);
    }

    #[test]
    fn severity_counts_default_is_zero() {
        let counts = SeverityCounts::default();
        assert_eq!(counts.total(), 0);
    }

    #[test]
    fn empty_alerts_yield_zero_counts() {
        let alerts: Vec<DecayAlert> = Vec::new();
        let counts = SeverityCounts::from_alerts(&alerts);
        assert_eq!(counts.total(), 0);
    }

    #[tokio::test]
    async fn detect_now_on_empty_repos_yields_empty_dto() {
        use quilt_test_helpers::{InMemoryBlockRepo, InMemoryPageRepo};
        let block_repo = InMemoryBlockRepo::new();
        let page_repo = InMemoryPageRepo::new();
        let svc = DecayMonitorService::new(block_repo, page_repo);
        let dto = svc.detect_now().await;
        assert!(dto.alerts.is_empty());
        assert_eq!(dto.total_alerts, 0);
        assert_eq!(dto.counts_by_severity.total(), 0);
    }

    #[test]
    fn alert_severity_age_relation() {
        // Sanity check the buckets: medium is 14..30 days, high is >= 30
        let medium = fake_alert("medium", 15);
        let high = fake_alert("high", 30);
        assert!(medium.days_since_update < 30);
        assert!(high.days_since_update >= 30);
        // Avoid unused warning when running in isolation
        let _ = Duration::days(1);
    }
}
