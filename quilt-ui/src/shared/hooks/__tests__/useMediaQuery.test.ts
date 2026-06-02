import { renderHook, act } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import { useMediaQuery } from '../useMediaQuery'

describe('useMediaQuery', () => {
  it('returns false when no match (default jsdom mock)', () => {
    // The setup file in src/test/setup.ts installs a matchMedia stub
    // that always returns `matches: false`. If that ever regresses,
    // this test starts failing first.
    const { result } = renderHook(() => useMediaQuery('(max-width: 767px)'))
    expect(result.current).toBe(false)
  })

  it('reacts to media query changes', () => {
    const listeners: Array<(e: { matches: boolean }) => void> = []
    const mockMql = {
      matches: false,
      addEventListener: vi.fn((event: string, cb: (e: any) => void) => {
        if (event === 'change') listeners.push(cb)
      }),
      removeEventListener: vi.fn((event: string, cb: (e: any) => void) => {
        if (event === 'change') {
          const i = listeners.indexOf(cb)
          if (i >= 0) listeners.splice(i, 1)
        }
      }),
    }
    window.matchMedia = vi.fn().mockReturnValue(mockMql) as any

    const { result } = renderHook(() => useMediaQuery('(max-width: 767px)'))
    expect(result.current).toBe(false)

    act(() => {
      listeners.forEach(l => l({ matches: true }))
    })
    expect(result.current).toBe(true)
  })

  it('removes its listener on unmount', () => {
    const removeEventListener = vi.fn()
    const mockMql = {
      matches: false,
      addEventListener: vi.fn(),
      removeEventListener,
    }
    window.matchMedia = vi.fn().mockReturnValue(mockMql) as any

    const { unmount } = renderHook(() => useMediaQuery('(min-width: 1024px)'))
    unmount()
    expect(removeEventListener).toHaveBeenCalledWith('change', expect.any(Function))
  })
})
