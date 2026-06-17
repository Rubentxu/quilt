// ─── import/types — GS-9 manual resource ingestion types ────────────────────
//
// Mirrors the backend IngestionCandidate, PlanSummary, IngestionPlan,
// and MigrationResponse value objects (crates/quilt-application/src/migration/mod.rs).

/** Status of an ingestion candidate relative to the current database state. */
export type CandidateStatus = 'new' | 'modified' | 'skipped'

/** A single file candidate for ingestion or reindexing (GS-9). */
export interface IngestionCandidate {
  sourcePath: string
  status: CandidateStatus
  sourceMtime: string | null
  storedMtime: string | null
}

/** Summary counts for an ingestion plan. */
export interface PlanSummary {
  total: number
  new: number
  modified: number
  skipped: number
}

/** Full ingestion plan returned by GET /api/v1/migration/candidates. */
export interface IngestionPlan {
  candidates: IngestionCandidate[]
  summary: PlanSummary
}

/** Per-candidate result from ingestion or reindex operations. */
export interface CandidateResult {
  sourcePath: string
  status: 'created' | 'updated' | 'skipped' | 'error'
  pagesCreated?: number
  blocksCreated?: number
  warning?: string
  error?: string
}

/** Response from POST /api/v1/migration/md or POST /api/v1/migration/reindex. */
export interface MigrationResponse {
  results: CandidateResult[]
  totalPagesCreated: number
  totalBlocksCreated: number
  totalUpdated: number
  totalSkipped: number
  warnings: string[]
}
