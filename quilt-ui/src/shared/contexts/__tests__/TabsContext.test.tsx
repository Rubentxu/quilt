import { renderHook, act } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import { TabsProvider, useTabs } from '../TabsContext'
import type { ReactNode } from 'react'

function wrapper({ children }: { children: ReactNode }) {
  return <TabsProvider>{children}</TabsProvider>
}

describe('TabsContext', () => {
  it('opens a new tab the first time', () => {
    const { result } = renderHook(() => useTabs(), { wrapper })
    let id!: string
    act(() => {
      id = result.current.openTab({ type: 'page', name: 'foo', title: 'Foo', params: {} })
    })
    expect(result.current.tabs).toHaveLength(1)
    expect(result.current.tabs[0].id).toBe(id)
    expect(result.current.activeTabId).toBe(id)
  })

  // ── The reported bug: clicking the same page multiple times should
  //    produce a single tab, not duplicates. ──
  it('does not create a duplicate tab when the same page is opened twice', () => {
    const { result } = renderHook(() => useTabs(), { wrapper })
    act(() => {
      result.current.openTab({ type: 'page', name: 'foo', title: 'Foo', params: {} })
    })
    act(() => {
      result.current.openTab({ type: 'page', name: 'foo', title: 'Foo', params: {} })
    })
    act(() => {
      result.current.openTab({ type: 'page', name: 'foo', title: 'Foo', params: {} })
    })
    expect(result.current.tabs).toHaveLength(1)
  })

  // Same scenario but with the PageViewPage-style params (the real call site).
  it('does not create a duplicate when params include redundant `name` (PageViewPage style)', () => {
    const { result } = renderHook(() => useTabs(), { wrapper })
    act(() => {
      result.current.openTab({
        type: 'page',
        name: 'foo',
        title: 'Foo',
        params: { name: 'foo' }, // PageViewPage.tsx line 19 passes this
      })
    })
    act(() => {
      result.current.openTab({
        type: 'page',
        name: 'foo',
        title: 'Foo',
        params: { name: 'foo' },
      })
    })
    expect(result.current.tabs).toHaveLength(1)
  })

  // Cross-page: open A, then B, then A again — should still have just 2 tabs.
  it('does not create duplicates when navigating A → B → A', () => {
    const { result } = renderHook(() => useTabs(), { wrapper })
    act(() => {
      result.current.openTab({ type: 'page', name: 'A', title: 'A', params: {} })
    })
    act(() => {
      result.current.openTab({ type: 'page', name: 'B', title: 'B', params: {} })
    })
    act(() => {
      result.current.openTab({ type: 'page', name: 'A', title: 'A', params: {} })
    })
    expect(result.current.tabs).toHaveLength(2)
    // The active tab should be the existing A
    const tabA = result.current.tabs.find((t) => t.name === 'A')!
    expect(result.current.activeTabId).toBe(tabA.id)
  })

  it('creates separate tabs for different journal dates', () => {
    const { result } = renderHook(() => useTabs(), { wrapper })
    act(() => {
      result.current.openTab({ type: 'journal', name: 'journal', title: '2026-06-02', params: { date: '2026-06-02' } })
    })
    act(() => {
      result.current.openTab({ type: 'journal', name: 'journal', title: '2026-06-01', params: { date: '2026-06-01' } })
    })
    expect(result.current.tabs).toHaveLength(2)
  })

  it('does not create a duplicate journal for the same date', () => {
    const { result } = renderHook(() => useTabs(), { wrapper })
    act(() => {
      result.current.openTab({ type: 'journal', name: 'journal', title: '2026-06-02', params: { date: '2026-06-02' } })
    })
    act(() => {
      result.current.openTab({ type: 'journal', name: 'journal', title: '2026-06-02', params: { date: '2026-06-02' } })
    })
    expect(result.current.tabs).toHaveLength(1)
  })

  it('reuses the existing tab and switches to it when opening a duplicate', () => {
    const { result } = renderHook(() => useTabs(), { wrapper })
    let firstId!: string
    act(() => {
      firstId = result.current.openTab({ type: 'page', name: 'foo', title: 'Foo', params: {} })
    })
    // Switch to something else
    act(() => {
      result.current.openTab({ type: 'page', name: 'bar', title: 'Bar', params: {} })
    })
    expect(result.current.activeTabId).not.toBe(firstId)
    // Re-open foo — should activate the existing foo tab, not create a new one
    let secondId!: string
    act(() => {
      secondId = result.current.openTab({ type: 'page', name: 'foo', title: 'Foo', params: {} })
    })
    expect(secondId).toBe(firstId)
    expect(result.current.activeTabId).toBe(firstId)
    expect(result.current.tabs).toHaveLength(2)
  })

  // StrictMode-style race: two openTab calls in the same render cycle
  // both see the same tabsRef and could both create a tab. This test
  // simulates the race by calling openTab twice synchronously.
  it('does not create duplicates when openTab is called twice synchronously', () => {
    const { result } = renderHook(() => useTabs(), { wrapper })
    act(() => {
      result.current.openTab({ type: 'page', name: 'foo', title: 'Foo', params: {} })
      result.current.openTab({ type: 'page', name: 'foo', title: 'Foo', params: {} })
    })
    expect(result.current.tabs).toHaveLength(1)
  })

  it('closes the active tab and activates an adjacent one', () => {
    const { result } = renderHook(() => useTabs(), { wrapper })
    let a!: string
    let b!: string
    act(() => {
      a = result.current.openTab({ type: 'page', name: 'A', title: 'A', params: {} })
    })
    act(() => {
      b = result.current.openTab({ type: 'page', name: 'B', title: 'B', params: {} })
    })
    expect(result.current.activeTabId).toBe(b)
    act(() => {
      result.current.closeTab(b)
    })
    expect(result.current.activeTabId).toBe(a)
    expect(result.current.tabs).toHaveLength(1)
  })
})
