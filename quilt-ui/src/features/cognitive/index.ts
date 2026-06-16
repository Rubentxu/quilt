/**
 * Cognitive Feature — `cognitivo::` family namespace (ADR-0001, ADR-DRAFT).
 *
 * Exports components belonging to the cognitive feature family. The
 * `cognitivo::` namespace marks surfaces that read or surface what AI
 * agents have added to the graph — Quilt itself does not perform
 * semantic analysis (see ADR-0001).
 *
 * Current panels (per
 * `docs/adr/drafts/DRAFT-cognitive-panel-family-namespace.md`,
 * Q013-P2):
 *
 * - `AgentActivityFeed` — passive view of recent agent-authored blocks.
 * - `StructuralGraph`   — page-level structural stats (block count,
 *                         property distribution, reference count,
 *                         orphan detection). Computed from
 *                         `getPageBlocks` + `getPageBacklinks`; will
 *                         be backed by the structural-mirror endpoint
 *                         when that route is mounted.
 * - `SemanticInsight`   — read-only list of `type:: insight` blocks
 *                         on the current page. Quilt never WRITES
 *                         insight blocks; agents do.
 *
 * The MirrorPanel, SerendipityFeed, and the analysis hooks
 * (`useAnalysisQuery`, `useRefreshInterval`) were removed in the P0
 * fix because their backing endpoints (`/api/v1/analysis/*`) are
 * not mounted in `crates/quilt-server/src/routes.rs`. They will
 * return when the server route is registered.
 */

export { AgentActivityFeed } from './AgentActivityFeed'
export { StructuralGraph } from './StructuralGraph'
export { SemanticInsight } from './SemanticInsight'
export { CognitivePanels } from './CognitivePanels'
export { MorningBriefing } from './MorningBriefing'
