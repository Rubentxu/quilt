import { lazy, Suspense, useState, useEffect, useRef } from 'react'
import { Outlet, useLocation, Link, useNavigate } from '@tanstack/react-router'
import { Toaster } from 'react-hot-toast'
import { Menu, Sun, Moon, Settings, Link2, X, PanelRight, Search, RefreshCw, Hash, EyeOff, Bell, HelpCircle } from 'lucide-react'
import { Sidebar } from '@features/sidebar/Sidebar'
import { BacklinksPanel } from '@features/references/BacklinksPanel'
import { TabsBar } from './TabsBar'
import { FloatingHelpButton } from './FloatingHelpButton'
import { useTabs } from '@shared/contexts/TabsContext'
import { useResponsive } from '@shared/hooks/useResponsive'
import { useConnection } from '@shared/contexts/ConnectionContext'
import { usePerformance } from '@shared/hooks/usePerformance'

// SearchModal is only mounted when the user opens the command palette
// (Ctrl+K). Keeping it out of the initial bundle saves ~3 KB and the
// lucide icons it imports.
const SearchModal = lazy(() =>
  import('@features/search/SearchModal').then(m => ({ default: m.SearchModal })),
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

export function AppShell() {
  // Surface any unexpectedly slow mount of the app shell itself.
  usePerformance('AppShell mount', 32)
  const [sidebarOpen, setSidebarOpen] = useState(true)
  const [backlinksOpen, setBacklinksOpen] = useState(true)
  const [mobileBacklinksOpen, setMobileBacklinksOpen] = useState(false)
  const [darkMode, setDarkMode] = useState(() => {
    return localStorage.getItem('quilt-theme') === 'dark'
  })
  const location = useLocation()
  const pageTitle = formatPathTitle(location.pathname)
  const currentPageName = location.pathname.startsWith('/page/')
    ? decodeURIComponent(location.pathname.split('/page/')[1])
    : null

  const [searchOpen, setSearchOpen] = useState(false)
  const { isMobile, isTablet } = useResponsive()
  const { tabs, activeTabId, closeTab, openTab, switchTab } = useTabs()
  const navigate = useNavigate()
  const { leaderKey } = useGlobalShortcuts()

  // Global keyboard shortcuts
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
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
              onClick={() => setSidebarOpen(false)}
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
                  onClose={() => setSidebarOpen(false)}
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
            onClick={() => setSidebarOpen(!sidebarOpen)}
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
            style={{
              fontSize: '16px',
              fontWeight: 600,
              color: 'var(--color-text-primary)',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              letterSpacing: '-0.01em',
            }}
          >
            {pageTitle}
          </span>

          {/* Spacer */}
          <div style={{ flex: 1 }} />

          {/* Quick actions cluster */}
          {!isMobile && (
            <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-1)' }}>
              {[Search, RefreshCw, Hash, EyeOff, Bell, HelpCircle].map((Icon, idx) => (
                <button
                  key={idx}
                  type="button"
                  className="ghost-icon-button topbar-action"
                  style={{
                    width: '34px',
                    height: '34px',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    cursor: 'pointer',
                  }}
                  aria-label="Toolbar action"
                >
                  <Icon size={17} />
                </button>
              ))}
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
              onClick={() => setBacklinksOpen(!backlinksOpen)}
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

          {/* User avatar */}
          <div
            aria-label="User avatar"
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
            }}
          >
            A
          </div>
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
        panel={
          <div>
            <h3 style={{
              fontSize: '14px',
              fontWeight: 600,
              color: 'var(--color-text-primary)',
              margin: '0 0 var(--space-3)',
            }}>
              Keyboard shortcuts
            </h3>
            <ul style={{
              listStyle: 'none',
              padding: 0,
              margin: 0,
              display: 'flex',
              flexDirection: 'column',
              gap: 'var(--space-2)',
              fontSize: '12px',
              color: 'var(--color-text-secondary)',
            }}>
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
