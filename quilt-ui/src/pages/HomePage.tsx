import { useEffect } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { api } from '@core/api-client'

/**
 * HomePage — root route (`/`) redirects based on global state.
 *
 * Per ADR-0030 §8, the home always lands on today's journal when
 * a valid last_opened_graph exists. If there is no valid graph
 * (first run or invalid path), it redirects to the graph selector.
 *
 * We compute "today" in the local timezone (matches what the rest of
 * the app does — see `formatToday` in `features/sidebar/Sidebar.tsx`
 * and the `gj` / `gt` shortcuts in `AppShell.tsx`). Using `toISOString`
 * would shift by the UTC offset and could send the user to "yesterday"
 * or "tomorrow" depending on the wall clock.
 */
export function HomePage() {
  const navigate = useNavigate()

  useEffect(() => {
    let cancelled = false

    api
      .getGlobalState()
      .then((state) => {
        if (cancelled) return
        const today = (() => {
          const now = new Date()
          const y = now.getFullYear()
          const m = String(now.getMonth() + 1).padStart(2, '0')
          const d = String(now.getDate()).padStart(2, '0')
          return `${y}-${m}-${d}`
        })()

        if (state.lastOpenedGraph) {
          // Valid last graph → go to today's journal
          navigate({ to: '/journal/$date', params: { date: today } })
        } else {
          // No last graph → go to the selector
          navigate({ to: '/select-graph' })
        }
      })
      .catch(() => {
        // Network/server error → go to selector (safe fallback)
        if (!cancelled) navigate({ to: '/select-graph' })
      })

    return () => {
      cancelled = true
    }
  }, [navigate])

  return null
}
