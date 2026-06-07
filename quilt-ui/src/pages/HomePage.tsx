import { useEffect } from 'react'
import { useNavigate } from '@tanstack/react-router'

/**
 * HomePage — root route (`/`) is a redirect-only component.
 *
 * The / route used to render `null`, leaving the user on a blank shell.
 * Visiting `/` should land them on today's journal — the same date
 * format the `/journal/$date` route accepts (YYYY-MM-DD).
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
    const now = new Date()
    const y = now.getFullYear()
    const m = String(now.getMonth() + 1).padStart(2, '0')
    const d = String(now.getDate()).padStart(2, '0')
    const today = `${y}-${m}-${d}`
    navigate({ to: '/journal/$date', params: { date: today } })
  }, [navigate])

  return null
}
