/**
 * Tests for useResponsive — composes useMediaQuery for
 * mobile, tablet, and desktop breakpoints.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useResponsive } from '@shared/hooks/useResponsive'

// Mock useMediaQuery to return controlled values
const mockMatches = { mobile: false, tablet: false, desktop: false }

vi.mock('@shared/hooks/useMediaQuery', () => ({
  useMediaQuery: vi.fn((query: string) => {
    if (query === '(max-width: 767px)') return mockMatches.mobile
    if (query === '(min-width: 768px) and (max-width: 1023px)') return mockMatches.tablet
    if (query === '(min-width: 1024px)') return mockMatches.desktop
    return false
  }),
}))

import { useMediaQuery } from '@shared/hooks/useMediaQuery'

describe('useResponsive', () => {
  beforeEach(() => {
    mockMatches.mobile = false
    mockMatches.tablet = false
    mockMatches.desktop = false
  })

  it('reports mobile when viewport <= 767px', () => {
    mockMatches.mobile = true
    const { result } = renderHook(() => useResponsive())
    expect(result.current.isMobile).toBe(true)
    expect(result.current.isTablet).toBe(false)
    expect(result.current.isDesktop).toBe(false)
  })

  it('reports tablet when viewport 768-1023px', () => {
    mockMatches.tablet = true
    const { result } = renderHook(() => useResponsive())
    expect(result.current.isMobile).toBe(false)
    expect(result.current.isTablet).toBe(true)
    expect(result.current.isDesktop).toBe(false)
  })

  it('reports desktop when viewport >= 1024px', () => {
    mockMatches.desktop = true
    const { result } = renderHook(() => useResponsive())
    expect(result.current.isMobile).toBe(false)
    expect(result.current.isTablet).toBe(false)
    expect(result.current.isDesktop).toBe(true)
  })

  it('returns all false when no breakpoint matches', () => {
    const { result } = renderHook(() => useResponsive())
    expect(result.current).toEqual({
      isMobile: false,
      isTablet: false,
      isDesktop: false,
    })
  })
})
