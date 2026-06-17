//! Migration module for importing external data into Quilt.
//!
//! Currently supports Markdown-flavored files (Quilt format).

pub mod md_import_parser;
pub mod migration_engine;

pub use md_import_parser::{Frontmatter, FrontmatterProperty, RawBlock, parse_md_import};
pub use migration_engine::{ImportResult, MigrationEngine, infer_property_value};

/// Re-export MigrationError for use in error handling.
pub use md_import_parser::MigrationError;

// ── Value Objects for GS-9 Manual Ingestion & Reindex ───────────────────────

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Status of an ingestion candidate relative to the current database state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CandidateStatus {
    /// File exists on disk but no ingested page with this source_path.
    New,
    /// File was previously ingested and its mtime is newer than stored.
    Modified,
    /// File was previously ingested and its mtime is unchanged.
    Skipped,
}

impl CandidateStatus {
    /// Returns the lowercase string representation for CLI output.
    pub fn as_str(&self) -> &'static str {
        match self {
            CandidateStatus::New => "new",
            CandidateStatus::Modified => "modified",
            CandidateStatus::Skipped => "skipped",
        }
    }
}

/// A single file candidate for ingestion or reindexing (GS-9).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestionCandidate {
    /// Relative POSIX path to the source file (never absolute, relative to graph root).
    pub source_path: String,
    /// Current status of this candidate.
    pub status: CandidateStatus,
    /// Current modification time of the file on disk (None if file is unreadable).
    pub source_mtime: Option<DateTime<Utc>>,
    /// Stored modification time from the last ingestion/reindex (None if never ingested).
    pub stored_mtime: Option<DateTime<Utc>>,
}

/// Summary counts for an ingestion plan.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSummary {
    pub new: usize,
    pub modified: usize,
    pub skipped: usize,
}

impl PlanSummary {
    /// Compute summary by folding over a slice of candidates (single pass, O(n)).
    pub fn from_candidates(candidates: &[IngestionCandidate]) -> Self {
        let mut summary = Self::default();
        for c in candidates {
            match c.status {
                CandidateStatus::New => summary.new += 1,
                CandidateStatus::Modified => summary.modified += 1,
                CandidateStatus::Skipped => summary.skipped += 1,
            }
        }
        summary
    }
}

/// An ingestion plan returned by `scan_directory` and consumed by `ingest` / `reindex`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestionPlan {
    /// Graph root used for this scan (for display/debugging).
    pub graph_root: PathBuf,
    /// Per-file candidates in scan order.
    pub candidates: Vec<IngestionCandidate>,
    /// Aggregated counts.
    pub summary: PlanSummary,
}

impl IngestionPlan {
    /// Build a plan from graph root and candidates.
    pub fn new(graph_root: PathBuf, candidates: Vec<IngestionCandidate>) -> Self {
        let summary = PlanSummary::from_candidates(&candidates);
        Self {
            graph_root,
            candidates,
            summary,
        }
    }

    /// Returns true if there are no candidates.
    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }
}

/// Result of a single reindex operation on one candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReindexResultEntry {
    pub source_path: String,
    /// "updated" | "skipped" | "error"
    pub status: String,
    pub blocks_updated: usize,
    pub warning: Option<String>,
}

/// Aggregate result of a full reindex operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReindexResult {
    pub results: Vec<ReindexResultEntry>,
    pub total_updated: usize,
    pub total_skipped: usize,
    pub warnings: Vec<String>,
}

impl ReindexResult {
    /// Build from a list of entries.
    pub fn from_entries(entries: Vec<ReindexResultEntry>) -> Self {
        let total_updated = entries.iter().filter(|e| e.status == "updated").count();
        let total_skipped = entries.iter().filter(|e| e.status == "skipped").count();
        let warnings = entries.iter().filter_map(|e| e.warning.clone()).collect();
        Self {
            results: entries,
            total_updated,
            total_skipped,
            warnings,
        }
    }
}

/// Result of a single ingest operation on one candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestResultEntry {
    pub source_path: String,
    /// "created" | "skipped" | "error"
    pub status: String,
    pub pages_created: usize,
    pub blocks_created: usize,
    pub warning: Option<String>,
}

/// Aggregate result of a full ingest operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestResult {
    pub results: Vec<IngestResultEntry>,
    pub total_pages_created: usize,
    pub total_blocks_created: usize,
    pub warnings: Vec<String>,
}

impl IngestResult {
    /// Build from a list of entries.
    pub fn from_entries(entries: Vec<IngestResultEntry>) -> Self {
        let total_pages_created: usize = entries
            .iter()
            .filter(|e| e.status == "created")
            .map(|e| e.pages_created)
            .sum();
        let total_blocks_created: usize = entries
            .iter()
            .filter(|e| e.status == "created")
            .map(|e| e.blocks_created)
            .sum();
        let warnings = entries.iter().filter_map(|e| e.warning.clone()).collect();
        Self {
            results: entries,
            total_pages_created,
            total_blocks_created,
            warnings,
        }
    }
}

#[cfg(test)]
mod vo_tests {
    use super::*;

    #[test]
    fn candidate_status_as_str() {
        assert_eq!(CandidateStatus::New.as_str(), "new");
        assert_eq!(CandidateStatus::Modified.as_str(), "modified");
        assert_eq!(CandidateStatus::Skipped.as_str(), "skipped");
    }

    #[test]
    fn plan_summary_from_candidates() {
        let candidates = vec![
            IngestionCandidate {
                source_path: "a.md".into(),
                status: CandidateStatus::New,
                source_mtime: None,
                stored_mtime: None,
            },
            IngestionCandidate {
                source_path: "b.md".into(),
                status: CandidateStatus::Modified,
                source_mtime: None,
                stored_mtime: None,
            },
            IngestionCandidate {
                source_path: "c.md".into(),
                status: CandidateStatus::Skipped,
                source_mtime: None,
                stored_mtime: None,
            },
            IngestionCandidate {
                source_path: "d.md".into(),
                status: CandidateStatus::New,
                source_mtime: None,
                stored_mtime: None,
            },
        ];
        let summary = PlanSummary::from_candidates(&candidates);
        assert_eq!(summary.new, 2);
        assert_eq!(summary.modified, 1);
        assert_eq!(summary.skipped, 1);
        assert_eq!(
            summary.new + summary.modified + summary.skipped,
            candidates.len()
        );
    }

    #[test]
    fn ingestion_plan_empty() {
        let plan = IngestionPlan::new(PathBuf::from("/tmp"), vec![]);
        assert!(plan.is_empty());
        assert_eq!(plan.summary.new, 0);
    }

    #[test]
    fn reindex_result_from_entries() {
        let entries = vec![
            ReindexResultEntry {
                source_path: "a.md".into(),
                status: "updated".into(),
                blocks_updated: 5,
                warning: None,
            },
            ReindexResultEntry {
                source_path: "b.md".into(),
                status: "skipped".into(),
                blocks_updated: 0,
                warning: Some("unchanged".into()),
            },
        ];
        let result = ReindexResult::from_entries(entries);
        assert_eq!(result.total_updated, 1);
        assert_eq!(result.total_skipped, 1);
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn ingest_result_from_entries() {
        let entries = vec![
            IngestResultEntry {
                source_path: "a.md".into(),
                status: "created".into(),
                pages_created: 1,
                blocks_created: 3,
                warning: None,
            },
            IngestResultEntry {
                source_path: "b.md".into(),
                status: "skipped".into(),
                pages_created: 0,
                blocks_created: 0,
                warning: Some("already exists".into()),
            },
        ];
        let result = IngestResult::from_entries(entries);
        assert_eq!(result.total_pages_created, 1);
        assert_eq!(result.total_blocks_created, 3);
    }

    #[test]
    fn plan_summary_invariant() {
        // new + modified + skipped == candidates.len() always holds
        for len in 0..20 {
            let candidates: Vec<_> = (0..len)
                .map(|i| IngestionCandidate {
                    source_path: format!("{}.md", i),
                    status: match i % 3 {
                        0 => CandidateStatus::New,
                        1 => CandidateStatus::Modified,
                        _ => CandidateStatus::Skipped,
                    },
                    source_mtime: None,
                    stored_mtime: None,
                })
                .collect();
            let summary = PlanSummary::from_candidates(&candidates);
            assert_eq!(
                summary.new + summary.modified + summary.skipped,
                len,
                "invariant violated for len={}",
                len
            );
        }
    }
}
