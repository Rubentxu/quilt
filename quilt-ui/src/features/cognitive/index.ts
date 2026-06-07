/**
 * Cognitive Feature — G7 Dream Cycle Display
 *
 * Exports cognitive-related components and hooks.
 *
 * Note: The MirrorPanel, SerendipityFeed, and the analysis hooks
 * (`useAnalysisQuery`, `useRefreshInterval`) were removed as part of
 * the P0 fix because their backing endpoints (`/api/v1/analysis/*`)
 * are not mounted in `crates/quilt-server/src/routes.rs`. They will
 * return when the server route is registered.
 */

export { AgentActivityPanel } from './AgentActivityPanel'
