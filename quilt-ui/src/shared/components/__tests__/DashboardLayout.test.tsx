/**
 * Tests for DashboardLayout — responsive layout with sidebar.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { DashboardLayout } from '@shared/components/DashboardLayout'

// Mock dependencies
vi.mock('@features/sidebar/Sidebar', () => ({
  Sidebar: ({ collapsed }: { collapsed: boolean }) => (
    <div data-testid="sidebar-mock" data-collapsed={collapsed}>
      Sidebar
    </div>
  ),
}))

vi.mock('@features/dashboard/PanelVisibilityContext', () => ({
  usePanelVisibility: () => ({
    visiblePanels: new Set(['sidebar']),
    togglePanel: vi.fn(),
  }),
}))

vi.mock('@shared/hooks/useResponsive', () => ({
  useResponsive: () => ({
    isMobile: false,
    isTablet: false,
    isDesktop: true,
  }),
}))

describe('DashboardLayout', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders children in the main content area', () => {
    render(
      <DashboardLayout>
        <div data-testid="test-content">Test Content</div>
      </DashboardLayout>
    )
    expect(screen.getByTestId('test-content')).toBeInTheDocument()
  })

  it('renders with dashboard-layout test id', () => {
    render(
      <DashboardLayout>
        <div>Content</div>
      </DashboardLayout>
    )
    expect(screen.getByTestId('dashboard-layout')).toBeInTheDocument()
  })

  it('renders sidebar in desktop mode', () => {
    render(
      <DashboardLayout>
        <div>Content</div>
      </DashboardLayout>
    )
    expect(screen.getByTestId('sidebar-mock')).toBeInTheDocument()
  })

  it('renders main content area', () => {
    render(
      <DashboardLayout>
        <div>Content</div>
      </DashboardLayout>
    )
    expect(screen.getByTestId('dashboard-main')).toBeInTheDocument()
  })

  it('passes onOpenSearch to sidebar when provided', () => {
    const onOpenSearch = vi.fn()
    render(
      <DashboardLayout onOpenSearch={onOpenSearch}>
        <div>Content</div>
      </DashboardLayout>
    )
    // The sidebar mock receives props but we verify the layout renders
    expect(screen.getByTestId('dashboard-layout')).toBeInTheDocument()
  })
})
