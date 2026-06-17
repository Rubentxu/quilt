// ─── selection/types — SelectionContext value objects ────────────────────────
//
// Immutable types that model "what is currently selected in the right sidebar".
// These are pure data — no React, no side effects.

/**
 * A page-level selection: the user has a specific page or journal in context.
 * This is the fallback when no block is selected.
 */
export interface PageSelection {
  type: 'page'
  pageName: string
  /**
   * True when the page is a daily journal (derived from pageName pattern
   * YYYY-MM-DD). Used by sections that need journal-specific behaviour.
   */
  isJournal: boolean
}

/**
 * A block-level selection: the user has focused into a specific block
 * within a page. This takes priority over page-level for section ranking.
 */
export interface BlockSelection {
  type: 'block'
  blockId: string
  pageName: string
}

/**
 * Graph-wide context — no specific page or block selected. Shown when the
 * user is on the home screen, graph view, or all-pages list.
 */
export interface GraphSelection {
  type: 'graph'
}

/**
 * The complete set of possible selection values the right sidebar can
 * receive. Wrapped in a discriminated union so reducers and rankers can
 * narrow by `selection.type` without ambiguity.
 */
export type Selection =
  | PageSelection
  | BlockSelection
  | GraphSelection
  | null

/**
 * Returns true when `selection` represents a block-level selection.
 */
export function isBlockSelection(selection: Selection): selection is BlockSelection {
  return selection !== null && selection.type === 'block'
}

/**
 * Returns true when `selection` represents a page-level selection
 * (including journals).
 */
export function isPageSelection(selection: Selection): selection is PageSelection {
  return selection !== null && selection.type === 'page'
}

/**
 * Returns true when `selection` represents a graph-level (null) selection.
 */
export function isGraphSelection(selection: Selection): selection is GraphSelection {
  return selection === null || selection.type === 'graph'
}

/**
 * The action types understood by the SelectionContext reducer.
 */
export type SelectionAction =
  | { type: 'BLOCK_FOCUSED'; blockId: string; pageName: string }
  | { type: 'PAGE_SELECTED'; pageName: string }
  | { type: 'CLEAR' }

/**
 * Route-key guard: a string that identifies the current navigation
 * context. When the route key changes, the reducer automatically
 * clears block-level selections (but NOT page-level) so that
 * navigating between pages doesn't leave stale block selection state.
 */
export type RouteKey = string

/**
 * SelectionContext value exposed to consumers via useSelection().
 */
export interface SelectionContextValue {
  selection: Selection
  routeKey: RouteKey
}
