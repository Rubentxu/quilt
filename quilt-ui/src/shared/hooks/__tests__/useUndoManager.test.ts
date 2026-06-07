/**
 * Tests for the session-scoped UndoManager.
 *
 * The UndoManager is a generic LIFO stack of `{ type, restore }` actions.
 * Tests assert behavior through the public API — they don't peek at
 * internal state. The `restore` callback is the contract: when we
 * push an action and then call `undo()`, the action's `restore` runs
 * (and we observe its side effect, not the implementation).
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { UndoManager, useUndoManager } from '../useUndoManager'
import { renderHook, act } from '@testing-library/react'

describe('UndoManager (class)', () => {
  let m: UndoManager

  beforeEach(() => {
    m = new UndoManager(3)
  })

  it('starts empty: canUndo() is false', () => {
    expect(m.canUndo()).toBe(false)
    expect(m.size).toBe(0)
  })

  it('canUndo() becomes true after push', () => {
    m.push({ type: 'noop', restore: () => {} })
    expect(m.canUndo()).toBe(true)
    expect(m.size).toBe(1)
  })

  it('undo() runs the most recently pushed restore callback', async () => {
    const a = vi.fn()
    const b = vi.fn()
    m.push({ type: 'a', restore: a })
    m.push({ type: 'b', restore: b })

    await m.undo()
    expect(b).toHaveBeenCalledTimes(1)
    expect(a).not.toHaveBeenCalled()
  })

  it('undo() pops the action so canUndo() reflects the new top', async () => {
    m.push({ type: 'a', restore: vi.fn() })
    m.push({ type: 'b', restore: vi.fn() })

    await m.undo()
    expect(m.canUndo()).toBe(true)
    expect(m.size).toBe(1)

    await m.undo()
    expect(m.canUndo()).toBe(false)
    expect(m.size).toBe(0)
  })

  it('undo() on an empty stack returns false and is a no-op', async () => {
    const result = await m.undo()
    expect(result).toBe(false)
    expect(m.canUndo()).toBe(false)
  })

  it('honors maxDepth by dropping the oldest action', async () => {
    m.push({ type: '1', restore: vi.fn() })
    m.push({ type: '2', restore: vi.fn() })
    m.push({ type: '3', restore: vi.fn() })
    m.push({ type: '4', restore: vi.fn() }) // exceeds depth=3, "1" should be evicted

    expect(m.size).toBe(3)

    // Undo three times — actions 4, 3, 2 should fire in LIFO order.
    // (1 was evicted, so it must not run.)
    const firedOrder: string[] = []
    m.push({ type: 'override', restore: () => firedOrder.push('2-or-3-or-4') })
    // Replace all actions with a tracked set so we can verify the order.
    m.clear()
    for (const t of ['1', '2', '3', '4']) {
      m.push({ type: t, restore: () => firedOrder.push(t) })
    }
    // Now size should be 3 (1 was evicted)
    expect(m.size).toBe(3)

    await m.undo()
    await m.undo()
    await m.undo()
    expect(firedOrder).toEqual(['4', '3', '2'])
  })

  it('default maxDepth is 50 when not specified', () => {
    const m2 = new UndoManager()
    for (let i = 0; i < 60; i++) {
      m2.push({ type: `a${i}`, restore: () => {} })
    }
    expect(m2.size).toBe(50)
  })

  it('clear() empties the stack', async () => {
    m.push({ type: 'a', restore: vi.fn() })
    m.push({ type: 'b', restore: vi.fn() })
    m.clear()
    expect(m.canUndo()).toBe(false)
    expect(m.size).toBe(0)
  })

  it('awaits async restore functions before resolving undo()', async () => {
    let completed = false
    m.push({
      type: 'async-restore',
      restore: () =>
        new Promise<void>(resolve => {
          setTimeout(() => {
            completed = true
            resolve()
          }, 10)
        }),
    })

    await m.undo()
    expect(completed).toBe(true)
  })

  it('a failing async restore still pops the action (we already moved on)', async () => {
    // If restore throws, we still want to drop the action — the
    // alternative is a stuck stack that blocks further undos. The
    // test asserts the action is gone so the user can keep going.
    m.push({
      type: 'broken',
      restore: () => Promise.reject(new Error('boom')),
    })

    // Swallow the rejection — the test asserts behavior, not whether
    // the manager itself throws.
    await m.undo().catch(() => {})

    expect(m.canUndo()).toBe(false)
  })

  it('pushes are independent of the order of the canUndo() observer', () => {
    // Sanity: pushing then reading canUndo() works without a React re-render.
    m.push({ type: 'x', restore: () => {} })
    expect(m.canUndo()).toBe(true)
  })
})

// ─── React hook adapter ──────────────────────────────────────────────

describe('useUndoManager (hook)', () => {
  it('returns a stable manager across renders and a reactive canUndo', () => {
    const { result, rerender } = renderHook(() => useUndoManager(5))
    const m1 = result.current.manager
    expect(result.current.canUndo).toBe(false)

    act(() => {
      result.current.push({ type: 'x', restore: () => {} })
    })
    expect(result.current.canUndo).toBe(true)

    rerender()
    // Same instance — no re-creation on rerender.
    expect(result.current.manager).toBe(m1)
    expect(result.current.canUndo).toBe(true)
  })

  it('undo() returns the same boolean the manager returns and flips canUndo', async () => {
    const restore = vi.fn()
    const { result } = renderHook(() => useUndoManager(5))

    act(() => {
      result.current.push({ type: 'x', restore })
    })

    let undoResult: boolean | undefined
    await act(async () => {
      undoResult = await result.current.undo()
    })
    expect(undoResult).toBe(true)
    expect(restore).toHaveBeenCalledTimes(1)
    expect(result.current.canUndo).toBe(false)
  })

  it('honors maxDepth through the hook', () => {
    const { result } = renderHook(() => useUndoManager(2))
    act(() => {
      result.current.push({ type: '1', restore: () => {} })
      result.current.push({ type: '2', restore: () => {} })
      result.current.push({ type: '3', restore: () => {} })
    })
    expect(result.current.manager.size).toBe(2)
  })

  it('clear() empties the stack and updates canUndo', () => {
    const { result } = renderHook(() => useUndoManager(5))
    act(() => {
      result.current.push({ type: 'x', restore: () => {} })
    })
    expect(result.current.canUndo).toBe(true)

    act(() => {
      result.current.clear()
    })
    expect(result.current.canUndo).toBe(false)
  })
})
