// ─── resolveSelection — pure selection resolver ────────────────────────────────
//
// Resolves the current selection from route parameters. This is a pure
// function with no side effects — the reducer calls it on every action
// to compute the next state. The route-key guard (clear on route change)
// is implemented by the caller (SelectionContext) comparing prev/new routeKey.

import type { Selection, RouteKey } from './types'

/**
 * Derive a Selection from the current pathname + optional blockId.
 *
 * ## Route-key guard (clear on nav)
 * When `prevRouteKey !== nextRouteKey` the block-level selection is
 * cleared unconditionally. This makes "clear selection on navigation"
 * implicit in the resolver rather than wiring a nav effect to clear state.
 *
 * ## Resolution order
 * 1. blockId present → BlockSelection (highest priority)
 * 2. page/journal route → PageSelection
 * 3. everything else → GraphSelection (null)
 *
 * Exported for unit-testing without React machinery.
 */
export function resolveSelection(params: {
  pathname: string
  blockId?: string | null
  prevRouteKey?: RouteKey
  nextRouteKey: RouteKey
}): Selection {
  const { pathname, blockId: originalBlockId, prevRouteKey, nextRouteKey } = params

  // Route-key guard: clear block selection on navigation
  const effectiveBlockId =
    prevRouteKey !== undefined && prevRouteKey !== nextRouteKey
      ? undefined
      : originalBlockId

  // Block-level selection (takes priority over page)
  if (effectiveBlockId && typeof effectiveBlockId === 'string' && effectiveBlockId.length > 0) {
    const segments = pathname.split('/').filter(Boolean)
    // Extract pageName from /page/<name> or /journal/<YYYY-MM-DD>
    let pageName: string | null = null
    if (segments.length >= 2 && segments[0] === 'page') {
      pageName = decodeURIComponentSafe(segments[1])
    } else if (segments.length >= 2 && segments[0] === 'journal') {
      pageName = decodeURIComponentSafe(segments[1])
    }
    return {
      type: 'block',
      blockId: effectiveBlockId,
      pageName: pageName ?? '',
    }
  }

  // Page or journal selection
  const segments = pathname.split('/').filter(Boolean)
  if (segments.length >= 2) {
    if (segments[0] === 'page') {
      return {
        type: 'page',
        pageName: decodeURIComponentSafe(segments[1]) ?? '',
        isJournal: false,
      }
    }
    if (segments[0] === 'journal') {
      const raw = segments[1]
      const isJournal = /^\d{4}-\d{2}-\d{2}$/.test(raw)
      return {
        type: 'page',
        pageName: decodeURIComponentSafe(raw) ?? '',
        isJournal,
      }
    }
  }

  // Everything else: graph-wide context
  return null
}

/**
 * Safe decoder — returns null on malformed URIs rather than throwing.
 */
function decodeURIComponentSafe(str: string): string | null {
  try {
    return decodeURIComponent(str)
  } catch {
    return null
  }
}
