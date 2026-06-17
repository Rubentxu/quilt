//! Migration use cases - orchestrates scan/ingest/reindex (GS-9).
//!
//! Wraps [`MigrationEngine`] with repository access. This is a concrete struct
//! (not a trait) per design decision OQ7.

use crate::migration::migration_engine::MigrationEngine;
use crate::migration::{CandidateStatus, IngestResult, IngestionPlan, ReindexResult};
use quilt_domain::repositories::{BlockRepository, PageRepository, PropertyRepository};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tracing::instrument;

/// Errors specific to migration use cases.
#[derive(Debug, Error)]
pub enum MigrationUseCasesError {
    #[error("Migration error: {0}")]
    Migration(#[from] crate::migration::MigrationError),

    #[error("Migration engine error: {0}")]
    MigrationEngine(#[from] crate::migration::migration_engine::MigrationError),

    #[error("Domain error: {0}")]
    Domain(#[from] quilt_domain::errors::DomainError),
}

/// Orchestrates scan/ingest/reindex operations (GS-9).
///
/// Concrete struct (not a trait) — single implementation, matches the
/// [`MigrationEngine`] pattern. Composes [`MigrationEngine`] with
/// repositories for the full scan→plan→confirm flow.
pub struct MigrationUseCases {
    engine: Arc<MigrationEngine>,
    page_repo: Arc<dyn PageRepository>,
    block_repo: Arc<dyn BlockRepository>,
    property_repo: Arc<dyn PropertyRepository>,
}

impl MigrationUseCases {
    /// Create a new MigrationUseCases instance.
    pub fn new(
        page_repo: Arc<dyn PageRepository>,
        block_repo: Arc<dyn BlockRepository>,
        property_repo: Arc<dyn PropertyRepository>,
    ) -> Self {
        Self {
            engine: Arc::new(MigrationEngine::new(
                page_repo.clone(),
                block_repo.clone(),
                property_repo.clone(),
            )),
            page_repo,
            block_repo,
            property_repo,
        }
    }

    /// Scan a directory for ingestion candidates (read-only).
    ///
    /// Returns an [`IngestionPlan`] with per-file status (new/modified/skipped).
    /// No pages or blocks are created.
    #[instrument(skip(self))]
    pub async fn scan(
        &self,
        graph_root: &Path,
        depth: u32,
    ) -> Result<IngestionPlan, MigrationUseCasesError> {
        let candidates = self.engine.scan_directory(graph_root, depth).await?;

        Ok(IngestionPlan::new(graph_root.to_path_buf(), candidates))
    }

    /// Ingest new pages from an approved plan.
    ///
    /// Only processes candidates with `status == CandidateStatus::New`.
    /// Candidates with `Modified` or `Skipped` are ignored.
    #[instrument(skip(self))]
    pub async fn ingest(
        &self,
        plan: &IngestionPlan,
    ) -> Result<IngestResult, MigrationUseCasesError> {
        let mut entries = Vec::new();

        for candidate in &plan.candidates {
            if candidate.status != CandidateStatus::New {
                continue;
            }

            let entry = self
                .engine
                .ingest_candidate(candidate, &plan.graph_root)
                .await?;
            entries.push(entry);
        }

        Ok(IngestResult::from_entries(entries))
    }

    /// Reindex modified pages from an approved plan.
    ///
    /// Only processes candidates with `status == CandidateStatus::Modified`.
    /// Candidates with `New` or `Skipped` are ignored.
    #[instrument(skip(self))]
    pub async fn reindex(
        &self,
        plan: &IngestionPlan,
    ) -> Result<ReindexResult, MigrationUseCasesError> {
        use crate::migration::ReindexResultEntry;

        let mut entries = Vec::new();

        for candidate in &plan.candidates {
            if candidate.status != CandidateStatus::Modified {
                continue;
            }

            let expected_mtime = candidate.stored_mtime.ok_or_else(|| {
                MigrationUseCasesError::MigrationEngine(
                    crate::migration::migration_engine::MigrationError::InvalidPath(
                        "Modified candidate has no stored mtime".to_string(),
                    ),
                )
            })?;

            let entry = match self
                .engine
                .reindex_file(&candidate.source_path, expected_mtime)
                .await
            {
                Ok(e) => e,
                Err(
                    crate::migration::migration_engine::MigrationError::ConcurrentModification(_),
                ) => ReindexResultEntry {
                    source_path: candidate.source_path.clone(),
                    status: "skipped".to_string(),
                    blocks_updated: 0,
                    warning: Some("concurrent modification detected".to_string()),
                },
                Err(e) => ReindexResultEntry {
                    source_path: candidate.source_path.clone(),
                    status: "error".to_string(),
                    blocks_updated: 0,
                    warning: Some(e.to_string()),
                },
            };
            entries.push(entry);
        }

        Ok(ReindexResult::from_entries(entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn use_cases_compile_concrete_not_trait() {
        // MigrationUseCases is a concrete struct, not a trait object.
        // This test verifies the type is concrete and can be instantiated
        // (the actual behavior is tested via integration tests).
        fn _assert_concrete<T: std::marker::Sized>() {}
        _assert_concrete::<MigrationUseCases>();
    }
}
