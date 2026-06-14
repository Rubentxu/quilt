/**
 * DashboardLayout — layout wrapper for the dashboard view.
 *
 * Provides a consistent shell: sidebar + main content area.
 * Uses the existing Sidebar feature and integrates with the
 * PanelVisibilityContext for panel persistence.
 *
 * Responsive: sidebar becomes an overlay drawer on mobile.
 * Accessibility: supports keyboard navigation, focus management,
 * and semantic HTML.
 */

import { useState, useEffect, type ReactNode } from 'react'
import { Sidebar } from '@features/sidebar/Sidebar'
import { usePanelVisibility } from '@features/dashboard/PanelVisibilityContext'
import { useResponsive } from '@shared/hooks/useResponsive'
import { Menu, X } from 'lucide-react'

interface DashboardLayoutProps {
  /** Content to render in the main area. */
  children: ReactNode
  /**
   * Optional callback when the user requests search.
   * Opens the search modal/drawer.
   */
  onOpenSearch?: () => void
}

/**
 * DashboardLayout — composable layout component.
 *
 * Usage:
 * ```tsx
 * function MyDashboard() {
 *   return (
 *     <DashboardLayout>
 *       <MyContent />
 *     </DashboardLayout>
 *   )
 * }
 * ```
 */
export function DashboardLayout({ children, onOpenSearch }: DashboardLayoutProps) {
  const { isMobile, isTablet } = useResponsive()
  const { visiblePanels, togglePanel } = usePanelVisibility()
  const sidebarOpen = visiblePanels.has('sidebar')
  const [mobileSidebarOpen, setMobileSidebarOpen] = useState(false)

  // Close mobile sidebar on route change
  useEffect(() => {
    if (mobileSidebarOpen) {
      setMobileSidebarOpen(false)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [location.pathname])

  // Handle escape key to close mobile sidebar
  useEffect(() => {
    if (!mobileSidebarOpen) return
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        setMobileSidebarOpen(false)
      }
    }
    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [mobileSidebarOpen])

  const handleToggleSidebar = () => {
    if (isMobile) {
      setMobileSidebarOpen(prev => !prev)
    } else {
      togglePanel('sidebar')
    }
  }

  const handleCloseMobileSidebar = () => {
    setMobileSidebarOpen(false)
  }

  return (
    <div
      className="flex h-screen"
      data-testid="dashboard-layout"
    >
      {/* ─── Sidebar ─── */}
      {isMobile ? (
        /* Mobile: overlay drawer */
        mobileSidebarOpen && (
          <>
            {/* Backdrop */}
            <div
              onClick={handleCloseMobileSidebar}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  handleCloseMobileSidebar()
                }
              }}
              role="button"
              tabIndex={0}
              aria-label="Close sidebar"
              data-testid="sidebar-backdrop"
              className="fixed inset-0 bg-black/50 z-40 transition-opacity"
            />
            {/* Drawer */}
            <div
              role="dialog"
              aria-modal="true"
              aria-label="Sidebar navigation"
              data-testid="sidebar-drawer"
              className="fixed top-0 left-0 bottom-0 w-[280px] z-50 bg-[var(--color-surface)] shadow-lg overflow-hidden flex flex-col"
            >
              <div className="flex flex-col h-full overflow-y-auto">
                <Sidebar
                  collapsed={false}
                  onOpenSearch={onOpenSearch}
                  onClose={handleCloseMobileSidebar}
                />
              </div>
            </div>
          </>
        )
      ) : (
        /* Desktop/Tablet: inline sidebar */
        <aside
          data-testid="sidebar"
          aria-label="Main navigation"
          className={`
            transition-all duration-300 ease-in-out
            bg-[var(--color-surface)]
            border-r border-[var(--color-border)]
            shadow-sm
            flex-shrink-0
            overflow-hidden
            ${isTablet
              ? 'w-[60px]'
              : sidebarOpen
                ? 'w-[var(--sidebar-width,260px)]'
                : 'w-[var(--sidebar-collapsed-width,60px)]'
            }
          `}
        >
          <div
            className="h-full overflow-y-auto sidebar-scroll"
            style={{ width: 'var(--sidebar-width, 260px)' }}
          >
            <Sidebar
              collapsed={isTablet || !sidebarOpen}
              onOpenSearch={onOpenSearch}
            />
          </div>
        </aside>
      )}

      {/* ─── Main content area ─── */}
      <div
        data-testid="dashboard-main"
        className="flex-1 flex flex-col min-w-0 overflow-hidden"
      >
        {/* Mobile header with hamburger */}
        {isMobile && (
          <header
            className="flex items-center gap-3 px-4 h-14 bg-[var(--color-surface)] border-b border-[var(--color-border)]"
          >
            <button
              onClick={handleToggleSidebar}
              aria-label={mobileSidebarOpen ? 'Close sidebar' : 'Open sidebar'}
              aria-expanded={mobileSidebarOpen}
              data-testid="mobile-menu-button"
              className="p-2 rounded-md hover:bg-[var(--color-surface-subtle)] transition-colors"
            >
              {mobileSidebarOpen ? <X size={18} /> : <Menu size={18} />}
            </button>
            <span className="font-semibold text-sm">Quilt</span>
          </header>
        )}

        {/* Page content */}
        <main
          data-testid="dashboard-content"
          className="flex-1 overflow-auto"
          tabIndex={-1}
        >
          {children}
        </main>
      </div>
    </div>
  )
}
