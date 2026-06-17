// ─── selection/index — SelectionContext public API ─────────────────────────

export { SelectionProvider, useSelection, useSelectionRouteKey, useSelectionDispatch, useSelectionFromRoute } from './SelectionContext'
export type { Selection, BlockSelection, PageSelection, GraphSelection, SelectionAction, SelectionContextValue, RouteKey } from './types'
export { resolveSelection } from './resolveSelection'
