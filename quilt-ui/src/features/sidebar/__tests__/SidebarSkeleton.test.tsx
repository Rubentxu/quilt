import { render } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import { SidebarSkeleton } from '../sections/SidebarSkeleton'

// Approval tests for SidebarSkeleton.
// The original implementation lived inside Sidebar.tsx as a 6-row
// loading placeholder. We pin the visible contract here so the
// extraction can't accidentally drop rows or change layout.

describe('SidebarSkeleton', () => {
  it('renders exactly 6 placeholder rows (matches original count)', () => {
    const { container } = render(<SidebarSkeleton />)
    // 6 child divs live inside the root container.
    const root = container.firstChild as HTMLElement
    expect(root.children).toHaveLength(6)
  })

  it('uses a column flex layout so the rows stack vertically', () => {
    const { container } = render(<SidebarSkeleton />)
    const root = container.firstChild as HTMLElement
    expect(root.style.display).toBe('flex')
    expect(root.style.flexDirection).toBe('column')
  })

  it('each placeholder has a fixed height of 16px and the subtle surface background', () => {
    const { container } = render(<SidebarSkeleton />)
    const root = container.firstChild as HTMLElement
    const rows = Array.from(root.children) as HTMLElement[]
    expect(rows.length).toBe(6)
    for (const row of rows) {
      expect(row.style.height).toBe('16px')
      expect(row.style.background).toBe('var(--color-surface-subtle)')
      expect(row.style.borderRadius).toBe('var(--radius-sm)')
    }
  })

  it('each placeholder has a percentage width between 60% and 90%', () => {
    const { container } = render(<SidebarSkeleton />)
    const root = container.firstChild as HTMLElement
    const rows = Array.from(root.children) as HTMLElement[]
    for (const row of rows) {
      const widthStr = row.style.width
      // The style is set as a percentage string e.g. "73.42%".
      expect(widthStr.endsWith('%')).toBe(true)
      const value = parseFloat(widthStr)
      expect(value).toBeGreaterThanOrEqual(60)
      expect(value).toBeLessThan(90)
    }
  })

  it('animates with the `pulse` keyframes for an in-progress feel', () => {
    const { container } = render(<SidebarSkeleton />)
    const root = container.firstChild as HTMLElement
    const rows = Array.from(root.children) as HTMLElement[]
    for (const row of rows) {
      expect(row.style.animation).toContain('pulse')
    }
  })

  it('is intentionally decorative — no role or aria-label exposed', () => {
    const { container } = render(<SidebarSkeleton />)
    const root = container.firstChild as HTMLElement
    // The skeleton is a loading affordance that should be hidden
    // from assistive technology (mirrors the original behaviour
    // — Sidebar.tsx shipped no role or aria-label on the rows).
    expect(root.hasAttribute('role')).toBe(false)
    expect(root.hasAttribute('aria-label')).toBe(false)
    const rows = Array.from(root.children) as HTMLElement[]
    for (const row of rows) {
      expect(row.hasAttribute('role')).toBe(false)
      expect(row.hasAttribute('aria-label')).toBe(false)
    }
  })
})
