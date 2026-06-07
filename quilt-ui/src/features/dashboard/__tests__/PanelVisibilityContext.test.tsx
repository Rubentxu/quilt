// ─── PanelVisibilityContext.test.tsx — dashboard panel state ────
//
// The context is the single source of truth for "which panels does
// the user want to see right now". Tests cover:
//   - initial state comes from the default preset
//   - localStorage round-trip (read on mount, write on change)
//   - togglePanel / applyPreset / setVisiblePanels transitions
//   - graceful degradation when localStorage is unavailable

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { act, render, screen, renderHook } from '@testing-library/react'
import { type ReactNode } from 'react'
import {
  PanelVisibilityProvider,
  usePanelVisibility,
  DEFAULT_PANELS,
  PANEL_LABELS,
  type PanelId,
  type PresetId,
} from '../PanelVisibilityContext'
import { getPreset, PRESETS } from '../presets'

const STORAGE_KEY = 'quilt-dashboard-layout'

/** A minimal wrapper that lets a hook test mount the provider. */
function wrapper({ children }: { children: ReactNode }) {
  return <PanelVisibilityProvider>{children}</PanelVisibilityProvider>
}

beforeEach(() => {
  // Reset localStorage between tests so the provider starts clean.
  localStorage.clear()
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('PanelVisibilityProvider — default state', () => {
  it('starts with the default preset when localStorage is empty', () => {
    const { result } = renderHook(() => usePanelVisibility(), { wrapper })
    const expected = getPreset('default')
    for (const panel of DEFAULT_PANELS) {
      expect(result.current.visiblePanels.has(panel)).toBe(expected.has(panel))
    }
  })

  it('exposes the static list of known panel ids', () => {
    expect(DEFAULT_PANELS).toContain('sidebar')
    expect(DEFAULT_PANELS).toContain('backlinks')
    expect(DEFAULT_PANELS).toContain('agent-activity')
    expect(DEFAULT_PANELS).toContain('outline')
  })

  it('exposes a human label for every panel id', () => {
    for (const id of DEFAULT_PANELS) {
      expect(PANEL_LABELS[id as PanelId]).toBeTruthy()
    }
  })
})

describe('PanelVisibilityProvider — mutations', () => {
  it('togglePanel flips a single panel id', () => {
    const { result } = renderHook(() => usePanelVisibility(), { wrapper })
    const startWith = result.current.visiblePanels.has('agent-activity')
    act(() => result.current.togglePanel('agent-activity'))
    expect(result.current.visiblePanels.has('agent-activity')).toBe(!startWith)
    // Other panels unchanged
    expect(result.current.visiblePanels.has('sidebar')).toBe(
      getPreset('default').has('sidebar'),
    )
  })

  it('setVisiblePanels replaces the entire set', () => {
    const { result } = renderHook(() => usePanelVisibility(), { wrapper })
    act(() =>
      result.current.setVisiblePanels(new Set<PanelId>(['backlinks'])),
    )
    expect(result.current.visiblePanels).toEqual(new Set<PanelId>(['backlinks']))
  })

  it('applyPreset(name) sets the visibility to the named preset', () => {
    const { result } = renderHook(() => usePanelVisibility(), { wrapper })
    act(() => result.current.applyPreset('focus'))
    expect(result.current.visiblePanels).toEqual(getPreset('focus'))
  })

  it('applyPreset(garbage) does not throw — falls back to default', () => {
    const { result } = renderHook(() => usePanelVisibility(), { wrapper })
    act(() => result.current.applyPreset('does-not-exist' as PresetId))
    expect(result.current.visiblePanels).toEqual(getPreset('default'))
  })
})

describe('PanelVisibilityProvider — localStorage persistence', () => {
  it('persists the current visibility set to localStorage on change', () => {
    const { result } = renderHook(() => usePanelVisibility(), { wrapper })
    act(() =>
      result.current.setVisiblePanels(new Set<PanelId>(['backlinks', 'outline'])),
    )
    const stored = localStorage.getItem(STORAGE_KEY)
    expect(stored).not.toBeNull()
    // Stored as a JSON array of panel ids
    const parsed = JSON.parse(stored!) as string[]
    expect(new Set(parsed)).toEqual(new Set<PanelId>(['backlinks', 'outline']))
  })

  it('persists the active preset name when applyPreset is called', () => {
    const { result } = renderHook(() => usePanelVisibility(), { wrapper })
    act(() => result.current.applyPreset('review'))
    expect(localStorage.getItem(STORAGE_KEY)).not.toBeNull()
    // The provider stores the panel set; the preset name is
    // derivable from the set. We assert the resulting panels
    // match the 'review' preset.
    const parsed = JSON.parse(localStorage.getItem(STORAGE_KEY)!) as string[]
    const restored = new Set<PanelId>(parsed)
    expect(restored).toEqual(getPreset('review'))
  })

  it('reads the saved panel set back on mount', () => {
    // Pre-seed localStorage BEFORE the provider mounts.
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify(['backlinks', 'outline']),
    )
    const { result } = renderHook(() => usePanelVisibility(), { wrapper })
    expect(result.current.visiblePanels).toEqual(
      new Set<PanelId>(['backlinks', 'outline']),
    )
  })

  it('survives a corrupt localStorage value (falls back to default)', () => {
    localStorage.setItem(STORAGE_KEY, 'this-is-not-json{')
    const { result } = renderHook(() => usePanelVisibility(), { wrapper })
    // Did not throw, and the visibility set is a valid set.
    expect(result.current.visiblePanels).toBeInstanceOf(Set)
    expect(result.current.visiblePanels.size).toBeGreaterThan(0)
  })

  it('survives localStorage being unavailable (private mode / quota)', () => {
    // Replace the localStorage shim with one that throws on write.
    const originalSetItem = localStorage.setItem.bind(localStorage)
    const setItemSpy = vi
      .spyOn(Storage.prototype, 'setItem')
      .mockImplementation(() => {
        throw new Error('QuotaExceededError')
      })
    const { result } = renderHook(() => usePanelVisibility(), { wrapper })
    // Should not throw on a mutation.
    expect(() =>
      act(() => result.current.togglePanel('agent-activity')),
    ).not.toThrow()
    setItemSpy.mockRestore()
    // And localStorage is back to the working shim.
    expect(typeof originalSetItem).toBe('function')
  })
})

describe('PanelVisibilityProvider — PRESETS export', () => {
  it('re-exports the preset table for callers that need it', () => {
    expect(PRESETS.default).toBeDefined()
    expect(PRESETS.focus).toBeDefined()
    expect(PRESETS.review).toBeDefined()
  })
})

describe('usePanelVisibility — context boundary', () => {
  it('throws (or returns a no-op shape) when used outside a provider', () => {
    // The default React behaviour for `useContext` on an
    // uninitialised context is to return the defaultValue. The
    // provider wires a defaultValue with no-op functions, so we
    // assert the contract is "does not throw" instead of asserting
    // a specific throw.
    const { result } = renderHook(() => usePanelVisibility())
    expect(typeof result.current.togglePanel).toBe('function')
    expect(typeof result.current.setVisiblePanels).toBe('function')
    expect(typeof result.current.applyPreset).toBe('function')
  })
})
