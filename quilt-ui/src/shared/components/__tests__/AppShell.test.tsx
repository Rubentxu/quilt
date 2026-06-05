import { describe, it, expect, vi } from 'vitest'
import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { deriveCurrentPageName, TopbarMenu } from '../AppShell'

describe('AppShell — deriveCurrentPageName (G6: backlinks on every page)', () => {
  it('returns the decoded page name for /page/<name> routes', () => {
    expect(deriveCurrentPageName('/page/Foo')).toBe('Foo')
    expect(deriveCurrentPageName('/page/Foo%20Bar')).toBe('Foo Bar')
  })

  it('returns the date for /journal/<YYYY-MM-DD> routes (G6 fix)', () => {
    // The pre-fix code only handled /page/ paths, so journal pages
    // were shown without any page name — the panel rendered but had
    // nothing to query. This test pins down the fix.
    expect(deriveCurrentPageName('/journal/2026-06-03')).toBe('2026-06-03')
  })

  it('returns null for the home route', () => {
    expect(deriveCurrentPageName('/')).toBeNull()
  })

  it('returns null for non-page routes', () => {
    expect(deriveCurrentPageName('/settings')).toBeNull()
    expect(deriveCurrentPageName('/pages')).toBeNull()
    expect(deriveCurrentPageName('/graph')).toBeNull()
  })

  it('returns null for empty /page/ segments', () => {
    // Defensive: trailing slash or accidental empty segment
    expect(deriveCurrentPageName('/page/')).toBeNull()
    expect(deriveCurrentPageName('/page')).toBeNull()
  })

  it('returns null for empty /journal/ segments', () => {
    expect(deriveCurrentPageName('/journal/')).toBeNull()
    expect(deriveCurrentPageName('/journal')).toBeNull()
  })

  it('handles nested page names (e.g. namespaced pages)', () => {
    expect(deriveCurrentPageName('/page/Projects%2F2026')).toBe('Projects/2026')
  })
})

// ─── TopbarMenu — kebab replaces 6 dead buttons (F1) ──────────────

describe('AppShell — TopbarMenu (F1 of quilt-fase2-ux-dead-buttons)', () => {
  it('opens a dropdown with the three real actions when the trigger is clicked', async () => {
    const user = userEvent.setup()
    render(
      <TopbarMenu
        onRefresh={vi.fn()}
        onToggleSidebar={vi.fn()}
        onOpenHelp={vi.fn()}
      />,
    )

    // Closed by default
    expect(screen.queryByTestId('topbar-menu')).not.toBeInTheDocument()

    await user.click(screen.getByTestId('topbar-kebab'))

    // The menu opens with three labelled actions — Refresh, Toggle
    // sidebar, Keyboard shortcuts — all visible to the user.
    const menu = screen.getByTestId('topbar-menu')
    expect(menu).toBeInTheDocument()
    expect(within(menu).getByRole('menuitem', { name: /refresh page/i })).toBeInTheDocument()
    expect(within(menu).getByRole('menuitem', { name: /toggle sidebar/i })).toBeInTheDocument()
    expect(within(menu).getByRole('menuitem', { name: /keyboard shortcuts/i })).toBeInTheDocument()
  })

  it('clicking the Refresh item fires the onRefresh callback and closes the menu', async () => {
    const user = userEvent.setup()
    const onRefresh = vi.fn()
    render(
      <TopbarMenu
        onRefresh={onRefresh}
        onToggleSidebar={vi.fn()}
        onOpenHelp={vi.fn()}
      />,
    )

    await user.click(screen.getByTestId('topbar-kebab'))
    await user.click(screen.getByTestId('topbar-menu-refresh'))

    expect(onRefresh).toHaveBeenCalledTimes(1)
    // Menu auto-closes after an action (standard dropdown pattern)
    expect(screen.queryByTestId('topbar-menu')).not.toBeInTheDocument()
  })

  it('clicking the Keyboard shortcuts item fires onOpenHelp', async () => {
    const user = userEvent.setup()
    const onOpenHelp = vi.fn()
    render(
      <TopbarMenu
        onRefresh={vi.fn()}
        onToggleSidebar={vi.fn()}
        onOpenHelp={onOpenHelp}
      />,
    )

    await user.click(screen.getByTestId('topbar-kebab'))
    await user.click(screen.getByTestId('topbar-menu-shortcuts'))

    expect(onOpenHelp).toHaveBeenCalledTimes(1)
  })

  it('clicking the Toggle sidebar item fires onToggleSidebar', async () => {
    const user = userEvent.setup()
    const onToggleSidebar = vi.fn()
    render(
      <TopbarMenu
        onRefresh={vi.fn()}
        onToggleSidebar={onToggleSidebar}
        onOpenHelp={vi.fn()}
      />,
    )

    await user.click(screen.getByTestId('topbar-kebab'))
    await user.click(screen.getByTestId('topbar-menu-toggle-sidebar'))

    expect(onToggleSidebar).toHaveBeenCalledTimes(1)
  })
})
