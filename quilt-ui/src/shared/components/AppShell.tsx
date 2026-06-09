import { lazy, Suspense, useState, useEffect, useRef } from 'react'
import { Outlet, useLocation, Link, useNavigate } from '@tanstack/react-router'
import { Toaster } from 'react-hot-toast'
import { Menu, Sun, Moon, Settings, Link2, X, PanelRight, MoreVertical, RefreshCw, Keyboard, Sidebar as SidebarIcon } from 'lucide-react'
import { Sidebar } from '@features/sidebar/Sidebar'
import { BacklinksPanel } from '@features/references/BacklinksPanel'
import { CognitivePanels } from '@features/cognitive/CognitivePanels'
import { TabsBar } from './TabsBar'
import { FloatingHelpButton } from './FloatingHelpButton'
import { WelcomeTour } from './WelcomeTour'
import { useTabs } from '@shared/contexts/TabsContext'
import { usePanelVisibility, LayoutMenu } from '@features/dashboard'
import { useResponsive } from '@shared/hooks/useResponsive'
import { useConnection } from '@shared/contexts/ConnectionContext'
import { usePerformance } from '@shared/hooks/usePerformance'
import { STORAGE_KEYS } from '@features/sidebar/storage-keys'
import { api } from '@core/api-client'

// SearchModal is only mounted when the user opens the command palette
// (Ctrl+K). Keeping it out of the initial bundle saves ~3 KB and the
// lucide icons it imports.
const SearchModal = lazy(() =>
  import('@features/search/SearchModal').then(m => ({ default: m.SearchModal })),
)

// CommandCenter is mounted only when the user opens the command
// palette (Cmd/Ctrl+Shift+K). Same lazy-load rationale as
// SearchModal: keeps the initial bundle lean.
const CommandCenter = lazy(() =>
  import('@features/command-center/CommandCenter').then(m => ({ default: m.CommandCenter })),
)

function formatPathTitle(pathname: string): string {
  if (pathname === '/') return 'Home'
  const segments = pathname.split('/').filter(Boolean)
  if (segments.length >= 2 && segments[0] === 'page') {
    return decodeURIComponent(segments[1])
  }
  if (segments.length >= 2 && segments[0] === 'journal') {
    const dateStr = segments[1]
    try {
      const d = new Date(dateStr + 'T00:00:00')
      return d.toLocaleDateString('en-US', {
        weekday: 'short',
        month: 'short',
        day: 'numeric',
        year: 'numeric',
      })
    } catch {
      return dateStr
    }
  }
  return pathname
}

/**
 * Derive the page name for the right-side BacklinksPanel from the
 * current location pathname. Returns the decoded page or journal
 * name when the user is on `/page/<name>` or `/journal/<YYYY-MM-DD>`,
 * and `null` for routes that don't have a backing page (home,
 * settings, all-pages, graph, etc.).
 *
 * Exported so the AppShell can be unit-tested for every route shape
 * without having to render the whole shell.
 */
export function deriveCurrentPageName(pathname: string): string | null {
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

function ConnectionStatus() {
  const { sseConnected } = useConnection()
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 'var(--space-1)',
      }}
      title={sseConnected ? 'Live updates (SSE)' : 'Polling mode'}
    >
      <div
        style={{
          width: '6px',
          height: '6px',
          borderRadius: '50%',
          background: sseConnected ? 'var(--color-success)' : 'var(--color-text-disabled)',
        }}
      />
      <span
        style={{
          fontSize: '11px',
          color: 'var(--color-text-muted)',
        }}
      >
        {sseConnected ? 'Live' : 'Polling'}
      </span>
    </div>
  )
}

function executeGoTo(combo: string, navigate: ReturnType<typeof useNavigate>) {
  switch (combo) {
    case 'gh':
      navigate({ to: '/' })
      break
    case 'gj': {
      // Journal — go to today's
      const today = new Date().toISOString().split('T')[0]
      navigate({ to: '/journal/$date', params: { date: today } })
      break
    }
    case 'gt': {
      // Today's journal (alias for gj)
      const t = new Date().toISOString().split('T')[0]
      navigate({ to: '/journal/$date', params: { date: t } })
      break
    }
    case 'gn': {
      // New page — open prompt or focus new page input
      const name = window.prompt('New page name:')
      if (name) navigate({ to: '/page/$name', params: { name } })
      break
    }
    case 'gp':
    case 'ga':
      // All pages
      navigate({ to: '/pages' })
      break
    case 'gg':
      // Graph
      navigate({ to: '/graph' })
      break
    case 'gs':
      // Settings
      navigate({ to: '/settings' })
      break
    default:
      // Unknown combo — no-op
      break
  }
}

function useGlobalShortcuts() {
  const navigate = useNavigate()
  const [leaderKey, setLeaderKey] = useState<string | null>(null)
  const leaderTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      // Don't intercept if typing
      const target = e.target as HTMLElement
      if (target.contentEditable === 'true' || target.tagName === 'INPUT' || target.tagName === 'TEXTAREA') return

      // Leader key mode: waiting for second key
      if (leaderKey) {
        e.preventDefault()
        const combo = leaderKey + e.key.toLowerCase()
        executeGoTo(combo, navigate)
        setLeaderKey(null)
        if (leaderTimeoutRef.current) clearTimeout(leaderTimeoutRef.current)
        return
      }

      // First key: 'g' activates leader mode
      if (e.key === 'g' && !e.ctrlKey && !e.metaKey && !e.altKey) {
        e.preventDefault()
        setLeaderKey('g')
        // Auto-cancel after 1.5s
        leaderTimeoutRef.current = setTimeout(() => setLeaderKey(null), 1500)
        return
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => {
      document.removeEventListener('keydown', handleKeyDown)
      if (leaderTimeoutRef.current) clearTimeout(leaderTimeoutRef.current)
    }
  }, [leaderKey, navigate])

  return { leaderKey }
}

/**
 * Kebab (three-dots) menu that lives in the top-bar. Replaces the
 * 6 dead buttons that previously sat in the quick-actions cluster
 * — every entry here is wired to a real action.
 *
 * Exported so it can be unit-tested in isolation (the cluster
 * styling and dropdown geometry are not interesting to tests).
 */
export interface TopbarMenuAction {
  key: string
  label: string
  icon: React.ReactNode
  onClick: () => void
}

interface TopbarMenuProps {
  onRefresh: () => void
  onToggleSidebar: () => void
  onOpenHelp: () => void
}

export function TopbarMenu({ onRefresh, onToggleSidebar, onOpenHelp }: TopbarMenuProps) {
  const [open, setOpen] = useState(false)
  const wrapperRef = useRef<HTMLDivElement>(null)

  // Close on outside click and Escape — matches the BlockContextMenu
  // pattern (DESIGN.md §11.3). Keeping it consistent means users
  // only have to learn the dismiss behaviour once.
  useEffect(() => {
    if (!open) return
    function handleClickOutside(e: MouseEvent) {
      const target = e.target as Node
      if (wrapperRef.current && !wrapperRef.current.contains(target)) {
        setOpen(false)
      }
    }
    function handleKey(e: KeyboardEvent) {
      if (e.key === 'Escape') setOpen(false)
    }
    document.addEventListener('mousedown', handleClickOutside)
    document.addEventListener('keydown', handleKey)
    return () => {
      document.removeEventListener('mousedown', handleClickOutside)
      document.removeEventListener('keydown', handleKey)
    }
  }, [open])

  const actions: TopbarMenuAction[] = [
    {
      key: 'refresh',
      label: 'Refresh page',
      icon: <RefreshCw size={14} aria-hidden="true" />,
      onClick: () => { onRefresh(); setOpen(false) },
    },
    {
      key: 'toggle-sidebar',
      label: 'Toggle sidebar',
      icon: <SidebarIcon size={14} aria-hidden="true" />,
      onClick: () => { onToggleSidebar(); setOpen(false) },
    },
    {
      key: 'shortcuts',
      label: 'Keyboard shortcuts',
      icon: <Keyboard size={14} aria-hidden="true" />,
      onClick: () => { onOpenHelp(); setOpen(false) },
    },
  ]

  return (
    <div ref={wrapperRef} style={{ position: 'relative' }}>
      <button
        type="button"
        onClick={() => setOpen(v => !v)}
        aria-label="More actions"
        aria-haspopup="menu"
        aria-expanded={open}
        data-testid="topbar-kebab"
        className="ghost-icon-button topbar-action"
        style={{
          width: '32px',
          height: '32px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          cursor: 'pointer',
          color: 'var(--color-text-secondary)',
        }}
      >
        <MoreVertical size={17} />
      </button>
      {open && (
        <div
          role="menu"
          aria-label="Top bar actions"
          data-testid="topbar-menu"
          style={{
            position: 'absolute',
            top: 'calc(100% + 4px)',
            right: 0,
            minWidth: '200px',
            background: 'var(--color-surface-elevated)',
            border: '1px solid var(--color-border)',
            borderRadius: 'var(--radius-md)',
            boxShadow: 'var(--shadow-md)',
            padding: 'var(--space-1)',
            zIndex: 100,
            display: 'flex',
            flexDirection: 'column',
            gap: '2px',
          }}
        >
          {actions.map(action => (
            <button
              key={action.key}
              role="menuitem"
              data-testid={`topbar-menu-${action.key}`}
              onClick={action.onClick}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 'var(--space-2)',
                width: '100%',
                padding: 'var(--space-2) var(--space-3)',
                border: 'none',
                background: 'transparent',
                color: 'var(--color-text-primary)',
                fontSize: '13px',
                fontWeight: 400,
                textAlign: 'left',
                cursor: 'pointer',
                borderRadius: 'var(--radius-sm)',
                fontFamily: 'inherit',
                lineHeight: 1.2,
                whiteSpace: 'nowrap',
              }}
              onMouseEnter={(e) => { e.currentTarget.style.background = 'var(--color-surface-subtle)' }}
              onMouseLeave={(e) => { e.currentTarget.style.background = 'transparent' }}
            >
              {action.icon}
              <span>{action.label}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  )
}

/**
 * Top-bar user avatar. F3 of quilt-fase3-backlog-small-fixes.
 *
 * Replaces the prior static `<div>A</div>` with a real `<button>`
 * that:
 *   - has an `aria-label` + `title` ("User menu") for a11y
 *   - shows the user's initial letter (from `quilt:user-name`
 *     or the legacy `quilt:author` localStorage key, with "U" as
 *     the fallback)
 *   - calls `onClick` when activated — the parent decides what
 *     that means (V1: navigate to /settings).
 *
 * Exported so the F3 tests can mount it without rendering the
 * full AppShell (the shell pulls in WASM, SSE, tabs, the whole
 * kitchen sink).
 */
export interface UserAvatarProps {
  onClick: () => void
}

export function UserAvatar({ onClick }: UserAvatarProps) {
  // "U" is the safe fallback — the first letter of "User". It
  // matches the existing design language (the sidebar's "Q" mark
  // for Quilt) and reads as a placeholder when the user has
  // not set a name.
  const [initial, setInitial] = useState('U')

  useEffect(() => {
    try {
      const name =
        localStorage.getItem('quilt:user-name') ||
        localStorage.getItem('quilt:author')
      if (name && name.trim()) {
        const trimmed = name.trim()
        // Take the first non-whitespace character. `charAt` is
        // safe for surrogate pairs; the trim above guarantees we
        // don't surface a leading space.
        const first = trimmed.charAt(0).toUpperCase()
        setInitial(first || 'U')
      }
    } catch {
      // localStorage unavailable (private mode / quota) — leave
      // the placeholder "U" in place. The user can still click
      // the avatar; only the letter is wrong.
    }
  }, [])

  return (
    <button
      type="button"
      onClick={onClick}
      aria-label="User menu"
      title="User menu"
      data-testid="user-avatar"
      className="topbar-action"
      style={{
        width: '32px',
        height: '32px',
        borderRadius: '999px',
        background: 'linear-gradient(180deg, #4F7BFF 0%, #355CFF 100%)',
        color: '#fff',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        fontSize: '14px',
        fontWeight: 700,
        boxShadow: '0 6px 18px rgba(79, 123, 255, 0.16)',
        border: 'none',
        cursor: 'pointer',
        padding: 0,
        fontFamily: 'inherit',
      }}
    >
      {initial}
    </button>
  )
}

export function AppShell() {
  // Surface any unexpectedly slow mount of the app shell itself.
  usePerformance('AppShell mount', 32)
  // DashboardLayout — sidebar / backlinks visibility is driven by
  // the PanelVisibilityContext so it can be persisted across
  // reloads and switched via presets. See
  // `features/dashboard/presets.ts` for the named presets.
  const { visiblePanels, togglePanel } = usePanelVisibility()
  const sidebarOpen = visiblePanels.has('sidebar')
  const backlinksOpen = visiblePanels.has('backlinks')
  const [mobileBacklinksOpen, setMobileBacklinksOpen] = useState(false)
  const [darkMode, setDarkMode] = useState(() => {
    return localStorage.getItem('quilt-theme') === 'dark'
  })
  const location = useLocation()
  const pageTitle = formatPathTitle(location.pathname)
  const currentPageName = deriveCurrentPageName(location.pathname)

  const [searchOpen, setSearchOpen] = useState(false)
  // Command palette state. Opened with Cmd/Ctrl+Shift+K (the
  // SearchModal takes Cmd/Ctrl+K). Kept separate from
  // `searchOpen` so the two palettes can never both be open at
  // the same time — opening one closes the other.
  const [commandCenterOpen, setCommandCenterOpen] = useState(false)
  const [helpExpanded, setHelpExpanded] = useState(false)
  // F3 of quilt-fase2-ux-empty-states + B of
  // quilt-fase4-cross-device-tour — first-run welcome tour.
  // `null` = "haven't checked yet" (avoids a flash of the dialog
  // during hydration). After the effect runs, the value is `true`
  // when the user has already dismissed the tour (on this device
  // OR on any other device, via the server) and `false`
  // otherwise. The localStorage flag is the fast cache for instant
  // render; the server is the source of truth so a dismissal on
  // desktop also hides the tour on mobile.
  const [tourDismissed, setTourDismissed] = useState<boolean | null>(null)
  const { isMobile, isTablet } = useResponsive()
  const { tabs, activeTabId, closeTab, openTab, switchTab } = useTabs()
  const navigate = useNavigate()
  const { leaderKey } = useGlobalShortcuts()

  useEffect(() => {
    // 1. Fast path: read the localStorage cache. This avoids a
    //    flash of the dialog on hard refreshes for users who have
    //    already seen it on this device.
    let fromCache = false
    try {
      fromCache = localStorage.getItem(STORAGE_KEYS.WELCOME_SEEN) === '1'
      if (fromCache) {
        setTourDismissed(true)
      }
    } catch {
      // localStorage unavailable (private mode / quota) — fall
      // through to the server check, which will be the only
      // signal we have.
    }

    // 2. Authoritative path: ask the server. The api key in the
    //    Authorization header is the user identifier, so a
    //    dismissal on any other device of the same user is
    //    visible here. We always issue the request — even when
    //    the cache said "dismissed" — so cross-device sync works
    //    in both directions.
    let cancelled = false
    api
      .getTourState()
      .then((state) => {
        if (cancelled) return
        const dismissed = state.dismissed.includes('welcome')
        // Reconcile the cache with the server. If the server
        // says "dismissed" and the cache didn't, write through so
        // the next mount takes the fast path.
        if (dismissed && !fromCache) {
          try {
            localStorage.setItem(STORAGE_KEYS.WELCOME_SEEN, '1')
          } catch {
            // localStorage may still be unavailable; the
            // in-memory state is correct for this session.
          }
        }
        setTourDismissed(dismissed)
      })
      .catch((err) => {
        if (cancelled) return
        // Server unreachable — fall back to the cache value. If
        // the cache was empty (false), the user will see the
        // tour, which is the right behavior for a first visit.
        // If the cache was true, stay dismissed.
        // eslint-disable-next-line no-console
        console.warn('Failed to fetch tour state, using local cache:', err)
        setTourDismissed(fromCache)
      })

    return () => {
      cancelled = true
    }
  }, [])

  // Global keyboard shortcuts
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      // Cmd/Ctrl+Shift+K — Command palette (the one driven by
      // CommandRegistry). Opens the new modal; closes the legacy
      // search palette so they don't fight for the same screen
      // real estate. Cmd/Ctrl+K without the Shift is the
      // SearchModal.
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === 'K' || e.key === 'k')) {
        e.preventDefault()
        setCommandCenterOpen(prev => !prev)
        setSearchOpen(false)
        return
      }

      // Ctrl+K — Search/Command palette
      if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
        e.preventDefault()
        setSearchOpen(prev => !prev)
        return
      }

      // Ctrl+T — New tab
      if ((e.ctrlKey || e.metaKey) && e.key === 't') {
        e.preventDefault()
        const name = window.prompt('Page name:')
        if (name && name.trim()) {
          openTab({ name: name.trim(), type: 'page', title: name.trim(), params: {} })
          navigate({ to: '/page/$name', params: { name: name.trim() } })
        }
        return
      }

      // Ctrl+W — Close active tab
      if ((e.ctrlKey || e.metaKey) && e.key === 'w') {
        e.preventDefault()
        if (activeTabId) closeTab(activeTabId)
        return
      }

      // Ctrl+Tab — Next tab
      if ((e.ctrlKey || e.metaKey) && e.key === 'Tab') {
        e.preventDefault()
        if (tabs.length > 0 && activeTabId) {
          const currentIdx = tabs.findIndex((t) => t.id === activeTabId)
          const nextIdx = (currentIdx + 1) % tabs.length
          const next = tabs[nextIdx]
          switchTab(next.id)
          if (next.type === 'page')
            navigate({ to: '/page/$name', params: { name: next.name } })
          else if (next.type === 'graph')
            navigate({ to: '/graph' })
          else if (next.type === 'all-pages')
            navigate({ to: '/pages' })
          else if (next.type === 'settings')
            navigate({ to: '/settings' })
          else if (next.type === 'journal' && next.params?.date)
            navigate({ to: '/journal/$date', params: { date: next.params.date } })
        }
        return
      }
    }
    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [tabs, activeTabId, closeTab, openTab, switchTab, navigate])

  function toggleTheme() {
    const next = !darkMode
    setDarkMode(next)
    document.documentElement.setAttribute('data-theme', next ? 'dark' : 'light')
    localStorage.setItem('quilt-theme', next ? 'dark' : 'light')
  }

  return (
    <div
      data-testid="app-shell"
      style={{
        display: 'flex',
        height: '100%',
        background: 'var(--color-background)',
      }}
    >
      {/* ─── Sidebar ─── */}
      {isMobile ? (
        /* Mobile: overlay drawer */
        sidebarOpen && (
          <>
            {/* Drawer backdrop */}
            <div
              onClick={() => togglePanel('sidebar')}
              style={{
                position: 'fixed',
                inset: 0,
                background: 'rgba(0,0,0,0.5)',
                zIndex: 40,
              }}
            />
            {/* Drawer */}
            <div
              style={{
                position: 'fixed',
                top: 0,
                left: 0,
                bottom: 0,
                width: '280px',
                zIndex: 50,
                background: 'var(--color-surface)',
                boxShadow: 'var(--shadow-lg)',
                overflow: 'hidden',
              }}
            >
              <div
                style={{ width: '280px', height: '100%', overflowY: 'auto' }}
                className="sidebar-scroll"
              >
                <Sidebar
                  collapsed={false}
                  onOpenSearch={() => setSearchOpen(true)}
                  onClose={() => togglePanel('sidebar')}
                />
              </div>
            </div>
          </>
        )
      ) : (
        /* Desktop/Tablet: inline sidebar */
        <aside
          style={{
            width: isTablet ? '60px' : (sidebarOpen ? 'var(--sidebar-width)' : 'var(--sidebar-collapsed-width)'),
            transition: 'width var(--motion-slow) var(--ease-standard)',
            background: 'var(--color-surface)',
            borderRight: '1px solid var(--color-border)',
            boxShadow: 'var(--shadow-sm)',
            overflow: 'hidden',
            flexShrink: 0,
          }}
        >
          <div
            style={{
              width: 'var(--sidebar-width)',
              height: '100%',
              overflowY: 'auto',
            }}
            className="sidebar-scroll"
          >
            <Sidebar
              collapsed={isTablet || !sidebarOpen}
              onOpenSearch={() => setSearchOpen(true)}
            />
          </div>
        </aside>
      )}

      {/* ─── Main area ─── */}
      <div
        style={{
          flex: 1,
          display: 'flex',
          flexDirection: 'column',
          minWidth: 0,
        }}
      >
        {/* ─── TopBar ─── */}
        <header
          style={{
            height: 'var(--topbar-height)',
            background: 'var(--color-surface)',
            borderBottom: '1px solid var(--color-border)',
            display: 'flex',
            alignItems: 'center',
            padding: '0 var(--space-4)',
            gap: 'var(--space-3)',
            position: 'sticky',
            top: 0,
            zIndex: 10,
            boxShadow: '0 1px 0 rgba(15, 23, 42, 0.02)',
          }}
        >
          {/* Hamburger menu */}
          <button
            onClick={() => togglePanel('sidebar')}
            aria-label={sidebarOpen ? 'Close sidebar' : 'Open sidebar'}
            title={sidebarOpen ? 'Close sidebar' : 'Open sidebar'}
            data-testid="mobile-menu-button"
            style={{
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              color: 'var(--color-text-secondary)',
              padding: 'var(--space-2)',
              borderRadius: 'var(--radius-md)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
            className="topbar-action"
          >
            {isMobile && sidebarOpen ? <X size={18} /> : <Menu size={18} />}
          </button>

          {/* Breadcrumb */}
          <span
            data-testid="breadcrumb"
            className="type-title-md"
            style={{
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {pageTitle}
          </span>

          {/* Spacer */}
          <div style={{ flex: 1 }} />

          {/* F1 of quilt-fase2-ux-dead-buttons: the previous
              "quick actions cluster" rendered 6 buttons (Search,
              Refresh, Hash, EyeOff, Bell, Help) with no onClick
              handlers — a 6-mystery-button anti-pattern. Replaced
              with a single kebab menu whose entries are all real
              actions. */}
          {!isMobile && (
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                padding: '4px',
                marginLeft: 'var(--space-2)',
                background: 'var(--color-surface-subtle)',
                borderRadius: '12px',
              }}
            >
              {/* LayoutMenu — DashboardLayout presets + per-panel
                  toggles. Lives next to the kebab in the same
                  surface-subtle pill. */}
              <LayoutMenu />
              <TopbarMenu
                onRefresh={() => window.location.reload()}
                onToggleSidebar={() => togglePanel('sidebar')}
                onOpenHelp={() => setHelpExpanded(true)}
              />
            </div>
          )}

          {/* Backlinks toggle */}
          {isMobile ? (
            <button
              onClick={() => setMobileBacklinksOpen(!mobileBacklinksOpen)}
              aria-label={mobileBacklinksOpen ? 'Close backlinks panel' : 'Open backlinks panel'}
              title={mobileBacklinksOpen ? 'Close backlinks panel' : 'Open backlinks panel'}
              style={{
                background: 'none',
                border: 'none',
                cursor: 'pointer',
                color: mobileBacklinksOpen ? 'var(--color-link)' : 'var(--color-text-muted)',
                padding: 'var(--space-1)',
                borderRadius: 'var(--radius-md)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
              }}
              className="topbar-action"
            >
              <PanelRight size={18} />
            </button>
          ) : (
            <button
              onClick={() => togglePanel('backlinks')}
              aria-label={backlinksOpen ? 'Close backlinks panel' : 'Open backlinks panel'}
              title={backlinksOpen ? 'Close backlinks panel' : 'Open backlinks panel'}
              style={{
                background: 'none',
                border: 'none',
                cursor: 'pointer',
                color: backlinksOpen ? 'var(--color-link)' : 'var(--color-text-muted)',
                padding: 'var(--space-1)',
                borderRadius: 'var(--radius-md)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
              }}
              className="topbar-action"
            >
              <Link2 size={18} />
            </button>
          )}

          {/* Theme toggle */}
          <button
            onClick={toggleTheme}
            aria-label={darkMode ? 'Switch to light theme' : 'Switch to dark theme'}
            title={darkMode ? 'Switch to light theme' : 'Switch to dark theme'}
            data-testid="theme-toggle"
            style={{
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              color: 'var(--color-text-muted)',
              padding: 'var(--space-1)',
              borderRadius: 'var(--radius-md)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
            className="topbar-action"
          >
            {darkMode ? <Sun size={18} /> : <Moon size={18} />}
          </button>

          {/* Settings */}
          <Link
            to="/settings"
            aria-label="Settings"
            title="Settings"
            style={{
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              color: 'var(--color-text-muted)',
              padding: 'var(--space-1)',
              borderRadius: 'var(--radius-md)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              textDecoration: 'none',
            }}
            className="topbar-action"
          >
            <Settings size={18} />
          </Link>

          {/* Connection status indicator */}
          <ConnectionStatus />

          {/* User avatar — F3 of quilt-fase3-backlog-small-fixes.
              Previously a non-interactive <div>A</div>; now a
              real <button> with a11y labels and an onClick that
              navigates to /settings. The full user menu
              (dropdown with profile / logout / theme) is a
              follow-up. */}
          <UserAvatar onClick={() => navigate({ to: '/settings' })} />
        </header>

        {/* ─── Tabs ─── */}
        <TabsBar />

        {/* ─── Content ─── */}
        <main
          style={{
            flex: 1,
            overflow: 'auto',
            padding: 'var(--space-6) var(--space-5) var(--space-10)',
          }}
        >
          <div
            style={{
              maxWidth: '1120px',
              margin: '0 auto',
            }}
          >
            <Outlet />
          </div>
        </main>
      </div>

      {/* ─── Backlinks Panel ─── */}
      {isMobile ? (
        /* Mobile: bottom sheet */
        mobileBacklinksOpen && (
          <>
            {/* Backdrop */}
            <div
              onClick={() => setMobileBacklinksOpen(false)}
              style={{
                position: 'fixed',
                inset: 0,
                background: 'rgba(0,0,0,0.5)',
                zIndex: 40,
              }}
            />
            {/* Bottom sheet */}
            <div
              style={{
                position: 'fixed',
                bottom: 0,
                left: 0,
                right: 0,
                maxHeight: '60vh',
                zIndex: 50,
                background: 'var(--color-surface)',
                borderTopLeftRadius: 'var(--radius-lg)',
                borderTopRightRadius: 'var(--radius-lg)',
                boxShadow: 'var(--shadow-lg)',
                overflow: 'auto',
              }}
            >
              <BacklinksPanel pageName={currentPageName} isOpen={true} />
            </div>
          </>
        )
      ) : (
        <BacklinksPanel pageName={currentPageName} isOpen={backlinksOpen} />
      )}

      {/* ─── Cognitive Panels (cognitivo:: family) ───
       *
       * Right-side column for the three cognitive panels
       * (`docs/adr/drafts/DRAFT-cognitive-panel-family-namespace.md`).
       * Each panel is gated by its own `PanelVisibilityContext`
       * flag; toggling via CommandRegistry (`cog/toggle-*`) shows
       * or hides individual sections without affecting siblings.
       * The column itself renders only when at least one panel
       * is visible, so it doesn't take up dead space.
       *
       * Hidden on mobile — the bottom-sheet pattern for the
       * cognitive family is a follow-up; for now, mobile users
       * toggle via the command palette or the layout menu. */}
      {!isMobile && <CognitivePanels pageName={currentPageName} />}

      {/* ─── Toast notifications ─── */}
      <Toaster
        position="bottom-right"
        toastOptions={{
          duration: 3000,
          style: {
            background: 'var(--color-surface-elevated)',
            color: 'var(--color-text-primary)',
            fontSize: '14px',
            borderRadius: 'var(--radius-md)',
            border: '1px solid var(--color-border)',
          },
        }}
      />

      {/* ─── Search / Command Palette ─── */}
      {searchOpen && (
        <Suspense fallback={null}>
          <SearchModal isOpen={searchOpen} onClose={() => setSearchOpen(false)} />
        </Suspense>
      )}

      {/* ─── Command Center (Cmd/Ctrl+Shift+K) ───
       *
       * The Registry-driven command palette. Lazy-loaded for the
       * same reason as SearchModal — keeps the initial bundle
       * lean and pulls the new feature in on first invocation.
       * The `CommandRegistryProvider` is mounted in `main.tsx`,
       * so the modal just consumes `useCommandRegistry()`.
       */}
      {commandCenterOpen && (
        <Suspense fallback={null}>
          <CommandCenter isOpen={commandCenterOpen} onClose={() => setCommandCenterOpen(false)} />
        </Suspense>
      )}

      {/* ─── Leader key indicator ─── */}
      {leaderKey && (
        <div
          style={{
            position: 'fixed',
            bottom: 'var(--space-4)',
            left: '50%',
            transform: 'translateX(-50%)',
            background: 'var(--color-surface)',
            border: '1px solid var(--color-border)',
            borderRadius: 'var(--radius-pill)',
            padding: 'var(--space-2) var(--space-4)',
            boxShadow: 'var(--shadow-md)',
            zIndex: 1000,
            fontSize: '13px',
            color: 'var(--color-text-secondary)',
          }}
        >
          <kbd
            style={{
              padding: '2px 6px',
              background: 'var(--color-surface-subtle)',
              borderRadius: '4px',
            }}
          >
            g
          </kbd>
          <span style={{ margin: '0 8px' }}>then...</span>
        </div>
      )}

      {/* ─── Floating help button — DESIGN.md §9.10 ─── */}
      <FloatingHelpButton
        label="Help & keyboard shortcuts"
        expanded={helpExpanded}
        onExpandedChange={setHelpExpanded}
        panel={
          <div>
            <h3
              className="type-title-md"
              style={{
                margin: '0 0 var(--space-3)',
              }}
            >
              Keyboard shortcuts
            </h3>
            <ul
              className="type-caption"
              style={{
                listStyle: 'none',
                padding: 0,
                margin: 0,
                display: 'flex',
                flexDirection: 'column',
                gap: 'var(--space-2)',
                color: 'var(--color-text-secondary)',
              }}
            >
              <li style={{ display: 'flex', justifyContent: 'space-between', gap: 'var(--space-3)' }}>
                <span>Search / command palette</span>
                <kbd style={kbdStyle}>Ctrl K</kbd>
              </li>
              <li style={{ display: 'flex', justifyContent: 'space-between', gap: 'var(--space-3)' }}>
                <span>New tab</span>
                <kbd style={kbdStyle}>Ctrl T</kbd>
              </li>
              <li style={{ display: 'flex', justifyContent: 'space-between', gap: 'var(--space-3)' }}>
                <span>Close tab</span>
                <kbd style={kbdStyle}>Ctrl W</kbd>
              </li>
              <li style={{ display: 'flex', justifyContent: 'space-between', gap: 'var(--space-3)' }}>
                <span>Today&apos;s journal</span>
                <kbd style={kbdStyle}>g d</kbd>
              </li>
              <li style={{ display: 'flex', justifyContent: 'space-between', gap: 'var(--space-3)' }}>
                <span>Graph</span>
                <kbd style={kbdStyle}>g g</kbd>
              </li>
              <li style={{ display: 'flex', justifyContent: 'space-between', gap: 'var(--space-3)' }}>
                <span>All pages</span>
                <kbd style={kbdStyle}>g p</kbd>
              </li>
            </ul>
          </div>
        }
      />

      {/* ─── Welcome tour (F3 of quilt-fase2-ux-empty-states + ──
          ─── B of quilt-fase4-cross-device-tour) ────────────
          Mounts a portal-rendered modal the FIRST time a user
          opens Quilt. The dialog persists `quilt-welcome-seen`
          to localStorage on close (fast cache) AND calls
          `api.dismissTour('welcome')` (server source of truth),
          so re-mounts after the initial dismissal are a no-op
          on every device the user owns. The
          `tourDismissed === null` branch (pre-effect) does NOT
          render the dialog, which avoids a flash of the tour on
          hard refreshes where the user has already dismissed it. */}
      {tourDismissed === false && (
        <WelcomeTour onClose={() => setTourDismissed(true)} />
      )}
    </div>
  )
}

const kbdStyle: React.CSSProperties = {
  fontSize: '10px',
  fontFamily: 'inherit',
  padding: '2px 6px',
  background: 'var(--color-surface-subtle)',
  border: '1px solid var(--color-border)',
  borderRadius: 'var(--radius-sm)',
  color: 'var(--color-text-secondary)',
  whiteSpace: 'nowrap',
}
