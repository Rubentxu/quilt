// ─── import/index — feature barrel ──────────────────────────────────────────
//
// Re-exports the MigrationPanel component and its section descriptor
// for registration in the right sidebar.

export { MigrationPanel, MIGRATION_SECTION_ID } from './MigrationPanel'
export type { IngestionCandidate, IngestionPlan, MigrationResponse } from './types'
