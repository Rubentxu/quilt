/**
 * useNavigation — focused hook for navigation state and actions.
 *
 * Composable pattern: wraps @tanstack/react-router hooks to provide
 * a clean interface for page navigation. Composes with usePage
 * to navigate to fetched pages.
 */
import { useCallback } from 'react'
import { useNavigate, useLocation } from '@tanstack/react-router'

interface UseNavigationResult {
  /** Navigate to a page by name. */
  navigateToPage: (name: string) => void
  /** Navigate to today's journal. */
  navigateToJournal: () => void
  /** Navigate to a specific journal date (YYYY-MM-DD). */
  navigateToJournalDate: (date: string) => void
  /** Navigate to the pages list. */
  navigateToPages: () => void
  /** Navigate to the graph view. */
  navigateToGraph: () => void
  /** Navigate to settings. */
  navigateToSettings: () => void
  /** Navigate to the home/dashboard. */
  navigateToHome: () => void
  /** Current pathname (decoded). */
  pathname: string
  /** Current page name if on a page route, null otherwise. */
  currentPageName: string | null
  /** Current journal date if on a journal route, null otherwise. */
  currentJournalDate: string | null
  /** True if currently on a page route. */
  isOnPage: boolean
  /** True if currently on a journal route. */
  isOnJournal: boolean
}

/**
 * Derive the page name from the current location.
 * Returns decoded page name for `/page/<name>` routes.
 */
function derivePageName(pathname: string): string | null {
  const segments = pathname.split('/').filter(Boolean)
  if (segments.length >= 2 && segments[0] === 'page') {
    const raw = segments[1]
    if (!raw) return null
    try {
      return decodeURIComponent(raw)
    } catch {
      return raw
    }
  }
  return null
}

/**
 * Derive the journal date from the current location.
 * Returns the date string for `/journal/<YYYY-MM-DD>` routes.
 */
function deriveJournalDate(pathname: string): string | null {
  const segments = pathname.split('/').filter(Boolean)
  if (segments.length >= 2 && segments[0] === 'journal') {
    const raw = segments[1]
    if (!raw) return null
    try {
      return decodeURIComponent(raw)
    } catch {
      return raw
    }
  }
  return null
}

export function useNavigation(): UseNavigationResult {
  const navigate = useNavigate()
  const location = useLocation()

  const pathname = decodeURIComponent(location.pathname)
  const currentPageName = derivePageName(location.pathname)
  const currentJournalDate = deriveJournalDate(location.pathname)
  const isOnPage = location.pathname.startsWith('/page/')
  const isOnJournal = location.pathname.startsWith('/journal/')

  const navigateToPage = useCallback((name: string) => {
    navigate({ to: '/page/$name', params: { name } })
  }, [navigate])

  const navigateToJournal = useCallback(() => {
    const today = new Date().toISOString().split('T')[0]
    navigate({ to: '/journal/$date', params: { date: today } })
  }, [navigate])

  const navigateToJournalDate = useCallback((date: string) => {
    navigate({ to: '/journal/$date', params: { date } })
  }, [navigate])

  const navigateToPages = useCallback(() => {
    navigate({ to: '/pages' })
  }, [navigate])

  const navigateToGraph = useCallback(() => {
    navigate({ to: '/graph' })
  }, [navigate])

  const navigateToSettings = useCallback(() => {
    navigate({ to: '/settings' })
  }, [navigate])

  const navigateToHome = useCallback(() => {
    navigate({ to: '/' })
  }, [navigate])

  return {
    navigateToPage,
    navigateToJournal,
    navigateToJournalDate,
    navigateToPages,
    navigateToGraph,
    navigateToSettings,
    navigateToHome,
    pathname,
    currentPageName,
    currentJournalDate,
    isOnPage,
    isOnJournal,
  }
}
