// ─── right-sidebar/index — Public API ───────────────────────────────────────
//
// Feature module entry point. Consumers import everything from here.

export { SelectionProvider, useSelection, useSelectionRouteKey, useSelectionDispatch, useSelectionFromRoute } from './selection'
export type { Selection, BlockSelection, PageSelection, GraphSelection, SelectionAction, SelectionContextValue, RouteKey } from './selection/types'
export { resolveSelection } from './selection/resolveSelection'

export { RightSidebarShell, RIGHT_SIDEBAR_PANEL_ID } from './RightSidebarShell'
export { RightSidebarEmptyState } from './RightSidebarEmptyState'

export {
  getSections,
  getVisibleSections,
  registerSection,
  isRegistered,
} from './sections/registry'
export type { RightSidebarSection, SectionPriority, SectionPredicate, RankedAction } from './sections/types'
export { rankMainAction, MAIN_ACTION_THRESHOLD, buildMainActionMap } from './sections/rankMainAction'
export type { SectionMainAction, MainActionTargetType } from './sections/rankMainAction'

// ─── Section IDs (for testing and PanelVisibilityContext mapping) ────────────

export const SIDEBAR_SECTION_IDS = {
  BACKLINKS: 'backlinks',
  BLOCK_PROPERTIES: 'block-properties',
  AGENT_ACTIVITY: 'agent-activity',
  AGENT_ROOM: 'agent-room',
  STRUCTURAL_GRAPH: 'structural-graph',
  SEMANTIC_INSIGHT: 'semantic-insight',
  COGNITIVE_GRAPH: 'cognitive-graph',
  DECAY_MONITOR: 'decay-monitor',
  WEEKLY_REVIEW: 'weekly-review',
  SERENDIPITY: 'serendipity',
} as const
