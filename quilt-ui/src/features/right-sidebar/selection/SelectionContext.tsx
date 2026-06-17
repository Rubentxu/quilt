// ─── SelectionContext — ephemeral right-sidebar selection state ───────────────
//
// Single source of truth for "what is selected in the right sidebar".
//
// Unlike PanelVisibilityContext, this is EPHEMERAL — no localStorage.
// Visibility is persisted, selection is not (ADR-0031).
//
// SelectionContext is a LEAF module — it does NOT import any feature
// panels. Only the barrel (index.ts) imports features to register them.

import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useReducer,
  type ReactNode,
} from 'react'
import type { Selection, SelectionAction, RouteKey, SelectionContextValue } from './types'
import { resolveSelection } from './resolveSelection'

// ─── Reducer ───────────────────────────────────────────────────────────────

function selectionReducer(
  state: { selection: Selection; routeKey: RouteKey },
  action: SelectionAction & { _routeKey?: RouteKey },
): { selection: Selection; routeKey: RouteKey } {
  switch (action.type) {
    case 'BLOCK_FOCUSED': {
      const next: Selection = {
        type: 'block',
        blockId: action.blockId,
        pageName: action.pageName,
      }
      return { selection: next, routeKey: action._routeKey ?? state.routeKey }
    }
    case 'PAGE_SELECTED': {
      const next: Selection = {
        type: 'page',
        pageName: action.pageName,
        isJournal: /^\d{4}-\d{2}-\d{2}$/.test(action.pageName),
      }
      return { selection: next, routeKey: action._routeKey ?? state.routeKey }
    }
    case 'CLEAR': {
      return { selection: null, routeKey: action._routeKey ?? state.routeKey }
    }
    default:
      return state
  }
}

// ─── Context ───────────────────────────────────────────────────────────────

const noop = () => {}
const defaultValue: SelectionContextValue = {
  selection: null,
  routeKey: '',
}

const SelectionContext = createContext<SelectionContextValue>(defaultValue)

// ─── Provider ──────────────────────────────────────────────────────────────

interface SelectionProviderProps {
  children: ReactNode
  /**
   * Stable route key derived from the current router location.
   * When this changes the reducer automatically clears block-level
   * selections (see resolveSelection route-key guard).
   */
  routeKey: RouteKey
}

export function SelectionProvider({ children, routeKey }: SelectionProviderProps) {
  const [state, dispatch] = useReducer(selectionReducer, {
    selection: null,
    routeKey,
  })

  // Sync routeKey into every dispatched action so the reducer can
  // apply the route-key guard (clear block selection on navigation).
  function dispatchWithRouteKey(action: SelectionAction): void {
    dispatch({ ...action, _routeKey: routeKey })
  }

  const value = useMemo<SelectionContextValue>(
    () => ({
      selection: state.selection,
      routeKey: state.routeKey,
    }),
    [state.selection, state.routeKey],
  )

  return (
    <SelectionContext.Provider value={value}>
      <SelectionContextDispatches.Provider value={dispatchWithRouteKey}>
        {children}
      </SelectionContextDispatches.Provider>
    </SelectionContext.Provider>
  )
}

// Separate context just for dispatches — keeps the public API surface minimal.
const SelectionContextDispatches = createContext<
  (action: SelectionAction) => void
>(noop)

// ─── Public API ─────────────────────────────────────────────────────────────

/** Consume the current selection (read-only). */
export function useSelection(): Selection {
  return useContext(SelectionContext).selection
}

/** Consume the current route key. */
export function useSelectionRouteKey(): RouteKey {
  return useContext(SelectionContext).routeKey
}

/**
 * Dispatch a selection action.
 *
 *   blockFocused(blockId, pageName)
 *   pageSelected(pageName)
 *   clearSelection()
 */
export function useSelectionDispatch(): {
  blockFocused: (blockId: string, pageName: string) => void
  pageSelected: (pageName: string) => void
  clearSelection: () => void
} {
  const dispatch = useContext(SelectionContextDispatches)

  return useMemo(
    () => ({
      blockFocused: (blockId: string, pageName: string) =>
        dispatch({ type: 'BLOCK_FOCUSED', blockId, pageName }),
      pageSelected: (pageName: string) =>
        dispatch({ type: 'PAGE_SELECTED', pageName }),
      clearSelection: () => dispatch({ type: 'CLEAR' }),
    }),
    [dispatch],
  )
}

/**
 * Resolve a new selection from route params and dispatch it.
 * Convenience for components that receive route params directly.
 */
export function useSelectionFromRoute(): {
  resolveAndSelect: (pathname: string, blockId?: string | null) => void
} {
  const { blockFocused, pageSelected } = useSelectionDispatch()
  const routeKey = useSelectionRouteKey()

  return useMemo(
    () => ({
      resolveAndSelect: (pathname: string, blockId?: string | null) => {
        const selection = resolveSelection({
          pathname,
          blockId,
          nextRouteKey: routeKey,
        })
        if (selection === null) {
          // graph context
        } else if (selection.type === 'block') {
          blockFocused(selection.blockId, selection.pageName)
        } else if (selection.type === 'page') {
          pageSelected(selection.pageName)
        }
      },
    }),
    [blockFocused, pageSelected, routeKey],
  )
}
