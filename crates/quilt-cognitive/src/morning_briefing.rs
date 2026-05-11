//! Morning Briefing — Daily Cognitive Summary
//!
//! Provides a daily briefing that aggregates insights from all cognitive engines,
//! giving users a "pulse" of their knowledge graph and serendipitous discoveries.
//!
//! # Overview
//!
//! The MorningBriefing aggregates:
//! - **Cognitive Pulse**: Stats from CognitiveMirror (pages, blocks, clusters, frontiers, gaps)
//! - **Serendipity Highlights**: Top unexpected connections from SerendipityEngine
//! - **Decay Alerts**: Pages that haven't been updated recently (stale knowledge)
//! - **Activity Stats**: Pages, blocks, and queries created/run today
//!
//! # Example
//!
//! ```
//! use quilt_cognitive::MorningBriefing;
//! use std::sync::Arc;
//!
//! async {
//!     // let briefing = MorningBriefing::new(Some(cognitive_mirror), Some(serendipity), None);
//!     // let dto = briefing.generate().await;
//! };
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::instrument;

use crate::cognitive_mirror::CognitiveMirror;
use crate::knowledge_evolution::KnowledgeEvolutionTracker;
use crate::serendipity::SerendipityEngine;
use quilt_domain::repositories::{BlockRepository, PageRepository};
use quilt_domain::value_objects::Uuid;

/// Aggregated cognitive pulse metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitivePulseDto {
    /// Total number of pages in the graph
    pub total_pages: usize,
    /// Total number of blocks in the graph
    pub total_blocks: usize,
    /// Number of detected knowledge clusters
    pub clusters: usize,
    /// Number of knowledge frontiers
    pub frontiers: usize,
    /// Number of detected knowledge gaps
    pub gaps: usize,
}

/// A serendipitous connection highlight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerendipityHighlightDto {
    /// Source page name
    pub from_page: String,
    /// Target page name
    pub to_page: String,
    /// Type of connection (structural, temporal, semantic)
    pub connection_type: String,
    /// Confidence score [0.0, 1.0]
    pub confidence: f32,
}

/// An alert about a stale (decaying) page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayAlertDto {
    /// Name of the stale page
    pub page_name: String,
    /// Last modification timestamp
    pub last_modified: DateTime<Utc>,
    /// Number of days since last modification
    pub days_stale: i64,
}

/// Activity statistics for today
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefingStatsDto {
    /// Pages created today
    pub pages_created_today: usize,
    /// Blocks created today
    pub blocks_created_today: usize,
    /// Queries run today
    pub queries_run_today: usize,
}

/// Knowledge evolution insight from tracked topics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEvolutionDto {
    /// Topic being tracked
    pub topic: String,
    /// Belief changes detected
    pub belief_changes: usize,
    /// Ideas that were reinforced
    pub reinforced_count: usize,
    /// Ideas that were abandoned
    pub abandoned_count: usize,
}

/// The complete morning briefing DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MorningBriefingDto {
    /// Cognitive pulse metrics
    pub cognitive_pulse: CognitivePulseDto,
    /// Top serendipitous connections
    pub serendipity_highlights: Vec<SerendipityHighlightDto>,
    /// Stale page alerts
    pub decay_alerts: Vec<DecayAlertDto>,
    /// Today's activity statistics
    pub stats: BriefingStatsDto,
    /// Knowledge evolution insights (top topics)
    pub knowledge_evolution: Vec<KnowledgeEvolutionDto>,
    /// When this briefing was generated
    pub generated_at: DateTime<Utc>,
    /// Whether any engine was unavailable (degraded mode)
    pub degraded: bool,
}

/// Morning Briefing service that aggregates cognitive engine outputs
///
/// This service is optional — it works with partial data if some engines
/// are not available, setting `degraded: true` in the response.
#[derive(Clone)]
#[allow(dead_code)]
pub struct MorningBriefing {
    cognitive_mirror: Option<Arc<CognitiveMirror>>,
    serendipity_engine: Option<Arc<SerendipityEngine>>,
    knowledge_evolution: Option<Arc<KnowledgeEvolutionTracker>>,
    // Repository access for stats
    page_repo: Option<Arc<dyn PageRepository>>,
    block_repo: Option<Arc<dyn BlockRepository>>,
}

impl MorningBriefing {
    /// Create a new MorningBriefing with the given cognitive engines
    pub fn new(
        cognitive_mirror: Option<Arc<CognitiveMirror>>,
        serendipity_engine: Option<Arc<SerendipityEngine>>,
        knowledge_evolution: Option<Arc<KnowledgeEvolutionTracker>>,
        page_repo: Option<Arc<dyn PageRepository>>,
        block_repo: Option<Arc<dyn BlockRepository>>,
    ) -> Self {
        Self {
            cognitive_mirror,
            serendipity_engine,
            knowledge_evolution,
            page_repo,
            block_repo,
        }
    }

    /// Generate the morning briefing by aggregating all available cognitive engines
    ///
    /// Uses parallel execution with timeouts. If any engine is unavailable,
    /// the briefing is returned with `degraded: true` and partial data.
    #[instrument(skip(self))]
    pub async fn generate(&self) -> MorningBriefingDto {
        // Run all data collection in parallel using tokio::join!
        let (cognitive_result, serendipity_result, stats_result, decay_result, knowledge_result) = tokio::join!(
            self.collect_cognitive_pulse(),
            self.collect_serendipity_highlights(),
            self.collect_stats(),
            self.collect_decay_alerts(),
            self.collect_knowledge_evolution(),
        );

        let degraded = cognitive_result.is_err()
            || serendipity_result.is_err()
            || stats_result.is_err()
            || decay_result.is_err()
            || knowledge_result.is_err();

        MorningBriefingDto {
            cognitive_pulse: cognitive_result.unwrap_or(CognitivePulseDto {
                total_pages: 0,
                total_blocks: 0,
                clusters: 0,
                frontiers: 0,
                gaps: 0,
            }),
            serendipity_highlights: serendipity_result.unwrap_or_default(),
            decay_alerts: decay_result.unwrap_or_default(),
            stats: stats_result.unwrap_or(BriefingStatsDto {
                pages_created_today: 0,
                blocks_created_today: 0,
                queries_run_today: 0,
            }),
            knowledge_evolution: knowledge_result.unwrap_or_default(),
            generated_at: Utc::now(),
            degraded,
        }
    }

    /// Collect cognitive pulse metrics from CognitiveMirror
    async fn collect_cognitive_pulse(&self) -> Result<CognitivePulseDto, String> {
        let Some(mirror) = &self.cognitive_mirror else {
            return Err("CognitiveMirror not available".to_string());
        };

        let Some(page_repo) = &self.page_repo else {
            return Err("PageRepository not available".to_string());
        };

        let Some(block_repo) = &self.block_repo else {
            return Err("BlockRepository not available".to_string());
        };

        // Get all pages and blocks for overall stats
        let pages = page_repo.get_all().await.map_err(|e| e.to_string())?;
        let total_pages = pages.len();

        let mut total_blocks = 0;
        let mut total_clusters = 0;
        let mut total_frontiers = 0;
        let mut total_gaps = 0;

        // Analyze a sample of pages for cognitive metrics
        // (Full analysis of all pages could be expensive)
        let sample_size = pages.len().min(10);
        for page in pages.iter().take(sample_size) {
            let blocks = block_repo
                .get_by_page(page.id)
                .await
                .map_err(|e| e.to_string())?;
            total_blocks += blocks.len();

            if let Ok(map) = mirror.analyze(page.id).await {
                total_clusters += map.clusters.len();
                total_frontiers += map.frontiers.len();
                total_gaps += map.gaps.len();
            }
        }

        // Extrapolate for full graph (approximate)
        let scale_factor = if sample_size > 0 {
            total_pages as f32 / sample_size as f32
        } else {
            1.0
        };

        Ok(CognitivePulseDto {
            total_pages,
            total_blocks,
            clusters: (total_clusters as f32 * scale_factor).ceil() as usize,
            frontiers: (total_frontiers as f32 * scale_factor).ceil() as usize,
            gaps: (total_gaps as f32 * scale_factor).ceil() as usize,
        })
    }

    /// Collect top serendipity highlights from SerendipityEngine
    async fn collect_serendipity_highlights(&self) -> Result<Vec<SerendipityHighlightDto>, String> {
        let Some(engine) = &self.serendipity_engine else {
            return Err("SerendipityEngine not available".to_string());
        };

        let Some(page_repo) = &self.page_repo else {
            return Err("PageRepository not available".to_string());
        };

        let query = crate::serendipity::SerendipityQuery {
            topic: None,
            limit: 5,
            offset: 0,
            min_confidence: 0.3,
            temporal_window_days: Some(7),
            page_id: None,
        };

        let connections = engine
            .find_connections(query)
            .await
            .map_err(|e| e.to_string())?;

        // Build page_id -> name lookup
        let pages = page_repo.get_all().await.map_err(|e| e.to_string())?;
        let page_map: std::collections::HashMap<Uuid, String> =
            pages.iter().map(|p| (p.id, p.name.clone())).collect();

        let highlights: Vec<SerendipityHighlightDto> = connections
            .iter()
            .map(|conn| {
                let from_page = page_map
                    .get(&conn.idea_a)
                    .cloned()
                    .unwrap_or_else(|| "Unknown".to_string());
                let to_page = page_map
                    .get(&conn.idea_b)
                    .cloned()
                    .unwrap_or_else(|| "Unknown".to_string());
                let connection_type = match conn.connection_type {
                    crate::serendipity::ConnectionType::Structural => "structural",
                    crate::serendipity::ConnectionType::Content => "content",
                    crate::serendipity::ConnectionType::Temporal => "temporal",
                    crate::serendipity::ConnectionType::Semantic => "semantic",
                };

                SerendipityHighlightDto {
                    from_page,
                    to_page,
                    connection_type: connection_type.to_string(),
                    confidence: conn.confidence,
                }
            })
            .collect();

        Ok(highlights)
    }

    /// Collect today's activity statistics
    async fn collect_stats(&self) -> Result<BriefingStatsDto, String> {
        let Some(page_repo) = &self.page_repo else {
            return Err("PageRepository not available".to_string());
        };

        let Some(block_repo) = &self.block_repo else {
            return Err("BlockRepository not available".to_string());
        };

        // Get today's start (midnight UTC)
        let now = Utc::now();
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();

        // Count pages created today
        let all_pages = page_repo.get_all().await.map_err(|e| e.to_string())?;
        let pages_created_today = all_pages
            .iter()
            .filter(|p| p.created_at >= today_start)
            .count();

        // Count blocks created today (requires iterating pages)
        let mut blocks_created_today = 0;
        for page in &all_pages {
            let blocks = block_repo
                .get_by_page(page.id)
                .await
                .map_err(|e| e.to_string())?;
            blocks_created_today += blocks
                .iter()
                .filter(|b| b.created_at >= today_start)
                .count();
        }

        // Queries run today - we don't have this metric directly
        // Return 0 as placeholder (could be tracked via query_service)
        let queries_run_today = 0;

        Ok(BriefingStatsDto {
            pages_created_today,
            blocks_created_today,
            queries_run_today,
        })
    }

    /// Collect decay alerts for stale pages
    async fn collect_decay_alerts(&self) -> Result<Vec<DecayAlertDto>, String> {
        let Some(page_repo) = &self.page_repo else {
            return Err("PageRepository not available".to_string());
        };

        let stale_threshold_days = 14i64;
        let cutoff = Utc::now() - chrono::Duration::days(stale_threshold_days);

        let pages = page_repo.get_all().await.map_err(|e| e.to_string())?;

        let mut alerts: Vec<DecayAlertDto> = Vec::new();

        for page in pages {
            if page.updated_at < cutoff {
                let days_stale = (Utc::now() - page.updated_at).num_days();
                alerts.push(DecayAlertDto {
                    page_name: page.name,
                    last_modified: page.updated_at,
                    days_stale,
                });
            }
        }

        // Sort by staleness (most stale first)
        alerts.sort_by(|a, b| b.days_stale.cmp(&a.days_stale));

        // Limit to top 10 most stale
        alerts.truncate(10);

        Ok(alerts)
    }

    /// Collect knowledge evolution insights from tracked topics
    async fn collect_knowledge_evolution(&self) -> Result<Vec<KnowledgeEvolutionDto>, String> {
        let Some(tracker) = &self.knowledge_evolution else {
            return Err("KnowledgeEvolutionTracker not available".to_string());
        };

        // Get recent pages to identify topics to track
        let Some(page_repo) = &self.page_repo else {
            return Err("PageRepository not available".to_string());
        };

        let pages = page_repo.get_all().await.map_err(|e| e.to_string())?;

        // Extract topics from recent page names (skip journals, focus on regular pages)
        let topics: Vec<String> = pages
            .iter()
            .filter(|p| !p.journal)
            .filter(|p| {
                // Only recent pages (updated in last 30 days)
                let thirty_days_ago = Utc::now() - chrono::Duration::days(30);
                p.updated_at > thirty_days_ago
            })
            .take(5) // Track up to 5 topics
            .map(|p| p.name.clone())
            .collect();

        let mut insights = Vec::new();

        for topic in topics {
            // Track evolution for each topic over the past 30 days
            match tracker.track(&topic, 30).await {
                Ok(timeline) => {
                    insights.push(KnowledgeEvolutionDto {
                        topic: timeline.topic,
                        belief_changes: timeline.belief_changes.len(),
                        reinforced_count: timeline.reinforced_ideas.len(),
                        abandoned_count: timeline.abandoned_ideas.len(),
                    });
                }
                Err(_) => {
                    // Skip topics that fail to track
                }
            }
        }

        // Sort by belief changes (most active first)
        insights.sort_by(|a, b| b.belief_changes.cmp(&a.belief_changes));
        insights.truncate(5); // Return top 5

        Ok(insights)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cognitive_pulse_dto_serialization() {
        let pulse = CognitivePulseDto {
            total_pages: 100,
            total_blocks: 500,
            clusters: 25,
            frontiers: 10,
            gaps: 5,
        };
        let json = serde_json::to_string(&pulse).unwrap();
        assert!(json.contains("\"total_pages\":100"));
    }

    #[test]
    fn test_serendipity_highlight_dto() {
        let highlight = SerendipityHighlightDto {
            from_page: "Rust Async".to_string(),
            to_page: " Tokio".to_string(),
            connection_type: "temporal".to_string(),
            confidence: 0.75,
        };
        let json = serde_json::to_string(&highlight).unwrap();
        assert!(json.contains("\"confidence\":0.75"));
    }

    #[test]
    fn test_decay_alert_dto() {
        let alert = DecayAlertDto {
            page_name: "Old Notes".to_string(),
            last_modified: Utc::now(),
            days_stale: 30,
        };
        assert_eq!(alert.days_stale, 30);
    }

    #[test]
    fn test_briefing_stats_dto() {
        let stats = BriefingStatsDto {
            pages_created_today: 5,
            blocks_created_today: 23,
            queries_run_today: 12,
        };
        assert_eq!(stats.pages_created_today, 5);
    }

    #[test]
    fn test_morning_briefing_dto_complete() {
        let dto = MorningBriefingDto {
            cognitive_pulse: CognitivePulseDto {
                total_pages: 100,
                total_blocks: 500,
                clusters: 25,
                frontiers: 10,
                gaps: 5,
            },
            serendipity_highlights: vec![SerendipityHighlightDto {
                from_page: "A".to_string(),
                to_page: "B".to_string(),
                connection_type: "structural".to_string(),
                confidence: 0.5,
            }],
            decay_alerts: vec![],
            stats: BriefingStatsDto {
                pages_created_today: 0,
                blocks_created_today: 0,
                queries_run_today: 0,
            },
            knowledge_evolution: vec![KnowledgeEvolutionDto {
                topic: "Rust async".to_string(),
                belief_changes: 2,
                reinforced_count: 1,
                abandoned_count: 0,
            }],
            generated_at: Utc::now(),
            degraded: false,
        };
        assert!(!dto.degraded);
        assert_eq!(dto.cognitive_pulse.total_pages, 100);
    }
}
