import { renderHook, act } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { useBlockHistory } from '../useBlockHistory'

// ──── WASM loader mock ─────────────────────────────────────────────
//
// useBlockHistory delegates to wasm-loader's history_* exports. We
// replace the whole module with deterministic stubs so the test
// can exercise the React wiring (effect init, apply, undo, redo,
// ref-based stack id) without standing up the WASM engine.
//
// `vi.mock` is hoisted to the top of the file, so the mock object
// itself has to live in a `vi.hoisted` block — otherwise the
// factory's reference to `mockHistory` fires before the `const`
// is initialised and vitest throws.

const { mockHistory, resetHistoryState } = vi.hoisted(() => {
  // Mutable per-test state lives on the object, not in outer scope.
  const state = { canUndo: false, canRedo: false }
  return {
    mockHistory: {
      wasmHistoryNew: vi.fn(() => 1),
      wasmHistoryFree: vi.fn(),
      wasmHistoryApply: vi.fn((_id: number, cmd: any) => {
        state.canUndo = true
        state.canRedo = false
        return [{ id: cmd.blockId ?? '1', content: cmd.after ?? 'updated' }]
      }),
      wasmHistoryUndo: vi.fn(() => {
        state.canUndo = false
        state.canRedo = true
        return [{ id: '1', content: 'original' }]
      }),
      wasmHistoryRedo: vi.fn(() => {
        state.canUndo = true
        state.canRedo = false
        return [{ id: '1', content: 'updated' }]
      }),
      wasmHistoryCanUndo: vi.fn(() => state.canUndo),
      wasmHistoryCanRedo: vi.fn(() => state.canRedo),
    },
    resetHistoryState: () => {
      state.canUndo = false
      state.canRedo = false
    },
  }
})

vi.mock('@core/wasm-bridge/wasm-loader', () => mockHistory)

beforeEach(() => {
  vi.clearAllMocks()
  resetHistoryState()
})

describe('useBlockHistory', () => {
  const baseBlocks = [
    {
      id: '1',
      pageId: 'p1',
      pageName: 'test',
      content: 'initial',
      blockType: 'paragraph',
      marker: null,
      priority: null,
      parentId: null,
      order: 0,
      level: 0,
      collapsed: false,
      createdAt: '',
      updatedAt: '',
    },
  ] as any

  it('allocates a stack on mount and reports canUndo/canRedo state', () => {
    const { result } = renderHook(() =>
      useBlockHistory({
        pageName: 'test',
        blocks: baseBlocks,
        onBlocksChanged: vi.fn(),
      }),
    )
    expect(mockHistory.wasmHistoryNew).toHaveBeenCalledWith(baseBlocks)
    // No commands yet → no undo/redo available.
    expect(result.current.canUndo).toBe(false)
    expect(result.current.canRedo).toBe(false)
  })

  it('applyCommand writes the new blocks through the callback', () => {
    const onBlocksChanged = vi.fn()
    const { result } = renderHook(() =>
      useBlockHistory({
        pageName: 'test',
        blocks: baseBlocks,
        onBlocksChanged,
      }),
    )

    act(() => {
      const ok = result.current.applyCommand({
        type: 'setContent',
        blockId: '1',
        before: 'initial',
        after: 'updated',
      } as any)
      expect(ok).toBe(true)
    })

    expect(mockHistory.wasmHistoryApply).toHaveBeenCalledTimes(1)
    expect(onBlocksChanged).toHaveBeenCalledWith([
      { id: '1', content: 'updated' },
    ])
    // canUndo flips to true after a successful apply.
    expect(result.current.canUndo).toBe(true)
  })

  it('undo restores the previous state and reports canRedo', () => {
    const onBlocksChanged = vi.fn()
    const { result } = renderHook(() =>
      useBlockHistory({
        pageName: 'test',
        blocks: baseBlocks,
        onBlocksChanged,
      }),
    )

    act(() => {
      result.current.applyCommand({
        type: 'setContent',
        blockId: '1',
        before: 'initial',
        after: 'updated',
      } as any)
    })
    act(() => {
      const ok = result.current.undo()
      expect(ok).toBe(true)
    })

    // Two callbacks: one from apply, one from undo.
    expect(onBlocksChanged).toHaveBeenCalledTimes(2)
    // Last call should be the undo result.
    expect(onBlocksChanged.mock.calls[1][0]).toEqual([
      { id: '1', content: 'original' },
    ])
    expect(result.current.canRedo).toBe(true)
  })

  it('frees the stack when pageName changes', () => {
    const { rerender } = renderHook(
      ({ pageName }: { pageName: string }) =>
        useBlockHistory({
          pageName,
          blocks: baseBlocks,
          onBlocksChanged: vi.fn(),
        }),
      { initialProps: { pageName: 'page-a' } },
    )

    rerender({ pageName: 'page-b' })
    expect(mockHistory.wasmHistoryFree).toHaveBeenCalled()
    // A new stack is allocated for the new page.
    expect(mockHistory.wasmHistoryNew).toHaveBeenCalledTimes(2)
  })

  it('returns false from applyCommand when disabled', () => {
    const { result } = renderHook(() =>
      useBlockHistory({
        pageName: 'test',
        blocks: baseBlocks,
        onBlocksChanged: vi.fn(),
        enabled: false,
      }),
    )
    act(() => {
      const ok = result.current.applyCommand({
        type: 'setContent',
        blockId: '1',
        before: 'initial',
        after: 'updated',
      } as any)
      expect(ok).toBe(false)
    })
    expect(mockHistory.wasmHistoryApply).not.toHaveBeenCalled()
  })
})
