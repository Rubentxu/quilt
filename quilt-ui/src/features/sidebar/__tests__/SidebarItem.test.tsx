import { render, screen } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import { FileText } from 'lucide-react'
import { SidebarItem } from '../sections/SidebarItem'

// `SidebarItem` is a presentational wrapper around TanStack's `Link`.
// We mock `Link` with a plain anchor so the test focuses on the
// SidebarItem contract (active/collapsed state, a11y, data attrs)
// and not the router internals.
vi.mock('@tanstack/react-router', () => ({
  Link: ({
    to,
    title,
    children,
    ...rest
  }: {
    to: string
    title?: string
    children: React.ReactNode
    [key: string]: unknown
  }) => (
    <a href={to} title={title} {...rest}>
      {children}
    </a>
  ),
}))

// Approval tests for SidebarItem (DESIGN.md §9.1).
// Behaviour captured from the inlined implementation in Sidebar.tsx:
//   - default: text-secondary, transparent bg, no left border
//   - active:  text-primary, primary-container bg, 3px left border,
//              data-active="true", aria-current="page", fontWeight 600
//   - collapsed: hides the label, sets `title` for a tooltip,
//                reduces the left padding
//   - data-testid propagates to the rendered <a>

describe('SidebarItem — DESIGN.md §9.1', () => {
  it('renders the label as visible text by default', () => {
    render(
      <SidebarItem icon={<FileText data-testid="icon" />} label="My Page" href="/page/foo" />,
    )
    expect(screen.getByText('My Page')).toBeInTheDocument()
  })

  it('renders an anchor pointing at the href', () => {
    render(
      <SidebarItem icon={<FileText data-testid="icon" />} label="My Page" href="/page/foo" />,
    )
    const link = screen.getByRole('link', { name: /My Page/ })
    expect(link).toHaveAttribute('href', '/page/foo')
  })

  it('renders the icon as content of the link', () => {
    render(
      <SidebarItem icon={<FileText data-testid="page-icon" />} label="My Page" href="/page/foo" />,
    )
    // The icon is rendered inside the link so screen readers can
    // announce the trailing text correctly.
    const link = screen.getByRole('link', { name: /My Page/ })
    expect(link).toContainElement(screen.getByTestId('page-icon'))
  })

  it('does NOT mark the link as current by default', () => {
    render(
      <SidebarItem icon={<FileText data-testid="icon" />} label="My Page" href="/page/foo" />,
    )
    const link = screen.getByRole('link', { name: /My Page/ })
    expect(link).not.toHaveAttribute('data-active')
    expect(link).not.toHaveAttribute('aria-current')
  })

  it('marks the link as current and active when active=true', () => {
    render(
      <SidebarItem
        icon={<FileText data-testid="icon" />}
        label="My Page"
        href="/page/foo"
        active
      />,
    )
    const link = screen.getByRole('link', { name: /My Page/ })
    expect(link).toHaveAttribute('data-active', 'true')
    expect(link).toHaveAttribute('aria-current', 'page')
  })

  it('renders the active-state left border indicator when active=true (DESIGN.md §9.1: must not depend on colour alone)', () => {
    const { container } = render(
      <SidebarItem
        icon={<FileText data-testid="icon" />}
        label="My Page"
        href="/page/foo"
        active
      />,
    )
    // The indicator is a span, hidden from assistive tech.
    const indicator = container.querySelector('span[aria-hidden="true"]')
    expect(indicator).toBeInTheDocument()
    expect(indicator?.tagName).toBe('SPAN')
  })

  it('does NOT render the left border indicator when active is omitted', () => {
    const { container } = render(
      <SidebarItem icon={<FileText data-testid="icon" />} label="My Page" href="/page/foo" />,
    )
    expect(container.querySelector('span[aria-hidden="true"]')).toBeNull()
  })

  it('hides the label when collapsed=true', () => {
    render(
      <SidebarItem
        icon={<FileText data-testid="icon" />}
        label="My Page"
        href="/page/foo"
        collapsed
      />,
    )
    // The label is no longer in the accessible name when collapsed.
    const link = screen.getByRole('link')
    expect(link).not.toHaveTextContent('My Page')
  })

  it('sets the title attribute (for a tooltip) when collapsed', () => {
    render(
      <SidebarItem
        icon={<FileText data-testid="icon" />}
        label="My Page"
        href="/page/foo"
        collapsed
      />,
    )
    const link = screen.getByRole('link')
    expect(link).toHaveAttribute('title', 'My Page')
  })

  it('does not set the title attribute when expanded (label is visible, no tooltip needed)', () => {
    render(
      <SidebarItem
        icon={<FileText data-testid="icon" />}
        label="My Page"
        href="/page/foo"
      />,
    )
    const link = screen.getByRole('link', { name: /My Page/ })
    expect(link).not.toHaveAttribute('title')
  })

  it('propagates dataTestId to the rendered link', () => {
    render(
      <SidebarItem
        icon={<FileText data-testid="icon" />}
        label="My Page"
        href="/page/foo"
        dataTestId="nav-foo"
      />,
    )
    expect(screen.getByTestId('nav-foo')).toBeInTheDocument()
  })
})
