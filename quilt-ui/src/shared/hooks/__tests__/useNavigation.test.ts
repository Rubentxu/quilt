/**
 * Tests for useNavigation — navigation state and action helpers.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import * as router from '@tanstack/react-router'

// Mock @tanstack/react-router
const mockNavigate = vi.fn()

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
  useLocation: vi.fn(),
}))

import { useNavigation } from '@shared/hooks/useNavigation'

describe('useNavigation', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    // Default location
    ;(router.useLocation as ReturnType<typeof vi.fn>).mockReturnValue({
      pathname: '/page/test',
      search: '',
      hash: '',
      state: null,
      key: 'default',
    })
  })

  describe('navigation actions', () => {
    it('navigates to a page by name', () => {
      const { result } = renderHook(() => useNavigation())
      result.current.navigateToPage('test-page')
      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/page/$name',
        params: { name: 'test-page' },
      })
    })

    it('navigates to today journal', () => {
      const { result } = renderHook(() => useNavigation())
      result.current.navigateToJournal()
      const today = new Date().toISOString().split('T')[0]
      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/journal/$date',
        params: { date: today },
      })
    })

    it('navigates to a specific journal date', () => {
      const { result } = renderHook(() => useNavigation())
      result.current.navigateToJournalDate('2026-01-15')
      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/journal/$date',
        params: { date: '2026-01-15' },
      })
    })

    it('navigates to pages list', () => {
      const { result } = renderHook(() => useNavigation())
      result.current.navigateToPages()
      expect(mockNavigate).toHaveBeenCalledWith({ to: '/pages' })
    })

    it('navigates to graph', () => {
      const { result } = renderHook(() => useNavigation())
      result.current.navigateToGraph()
      expect(mockNavigate).toHaveBeenCalledWith({ to: '/graph' })
    })

    it('navigates to settings', () => {
      const { result } = renderHook(() => useNavigation())
      result.current.navigateToSettings()
      expect(mockNavigate).toHaveBeenCalledWith({ to: '/settings' })
    })

    it('navigates to home', () => {
      const { result } = renderHook(() => useNavigation())
      result.current.navigateToHome()
      expect(mockNavigate).toHaveBeenCalledWith({ to: '/' })
    })
  })

  describe('route state', () => {
    it('derives page name from /page/ routes', () => {
      ;(router.useLocation as ReturnType<typeof vi.fn>).mockReturnValue({
        pathname: '/page/Test%20Page',
        search: '',
        hash: '',
        state: null,
        key: 'default',
      })
      const { result } = renderHook(() => useNavigation())
      expect(result.current.currentPageName).toBe('Test Page')
      expect(result.current.isOnPage).toBe(true)
      expect(result.current.isOnJournal).toBe(false)
    })

    it('derives journal date from /journal/ routes', () => {
      ;(router.useLocation as ReturnType<typeof vi.fn>).mockReturnValue({
        pathname: '/journal/2026-06-13',
        search: '',
        hash: '',
        state: null,
        key: 'default',
      })
      const { result } = renderHook(() => useNavigation())
      expect(result.current.currentJournalDate).toBe('2026-06-13')
      expect(result.current.isOnJournal).toBe(true)
      expect(result.current.isOnPage).toBe(false)
    })

    it('returns null page name for non-page routes', () => {
      ;(router.useLocation as ReturnType<typeof vi.fn>).mockReturnValue({
        pathname: '/pages',
        search: '',
        hash: '',
        state: null,
        key: 'default',
      })
      const { result } = renderHook(() => useNavigation())
      expect(result.current.currentPageName).toBeNull()
      expect(result.current.isOnPage).toBe(false)
    })

    it('returns null journal date for non-journal routes', () => {
      ;(router.useLocation as ReturnType<typeof vi.fn>).mockReturnValue({
        pathname: '/page/test',
        search: '',
        hash: '',
        state: null,
        key: 'default',
      })
      const { result } = renderHook(() => useNavigation())
      expect(result.current.currentJournalDate).toBeNull()
    })

    it('returns pathname as decoded', () => {
      ;(router.useLocation as ReturnType<typeof vi.fn>).mockReturnValue({
        pathname: '/page/Hello%20World',
        search: '',
        hash: '',
        state: null,
        key: 'default',
      })
      const { result } = renderHook(() => useNavigation())
      expect(result.current.pathname).toBe('/page/Hello World')
    })
  })
})
