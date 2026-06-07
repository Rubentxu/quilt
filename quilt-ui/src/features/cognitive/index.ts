/**
 * Cognitive Feature — `cognitivo::` family namespace (ADR-0001, ADR-DRAFT).
 *
 * Exports components belonging to the cognitive feature family. The
 * `cognitivo::` namespace marks surfaces that read or surface what AI
 * agents have added to the graph — Quilt itself does not perform
 * semantic analysis (see ADR-0001).
 *
 * Current panels:
 * - `AgentActivityFeed` — passive view of recent agent-authored blocks.
 *
 * Note: The MirrorPanel, SerendipityFeed, and the analysis hooks
 * (`useAnalysisQuery`, `useRefreshInterval`) were removed as part of
 * the P0 fix because their backing endpoints (`/api/v1/analysis/*`)
 * are not mounted in `crates/quilt-server/src/routes.rs`. They will
 * return when the server route is registered.
 */

export { AgentActivityFeed } from './AgentActivityFeed'
