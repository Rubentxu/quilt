/**
 * Tests for `useBlockStrategy` — React hook that wraps the WASM
 * `StrategySelector` (or a JS fallback) and returns the strategy
 * name a block should be rendered/edited with.
 *
 * Strategy names (mirroring `crates/quilt-core/src/strategy.rs`):
 *   - "task"       — `type:: task`
 *   - "query"      — `type:: query`
 *   - "view"       — `type:: view`
 *   - "agent-run"  — `type:: agent-run`
 *   - "default"    — fallback for any other / missing `type`
 *
 * The hook must:
 *   1. Resolve a strategy for every kind of Block.
 *   2. Return "default" when the WASM is unavailable (the JS fallback).
 *   3. Re-resolve when the block reference changes.
 *   4. Coerce Block.properties (an array of {key,value}) into the
 *      `{"properties": {"type": "..."}}` shape the WASM bridge wants.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import type { Block, BlockProperty } from '@shared/types/api'

// Mock the wasm-loader. Each test overrides `wasmStrategySelect` and
// `wasmStrategyAll` to simulate the WASM being present (or missing).
const mockSelect = vi.fn()
const mockAll = vi.fn()

vi.mock('@core/wasm-bridge/wasm-loader', () => ({
  wasmStrategySelect: (...args: unknown[]) => mockSelect(...args),
  wasmStrategyAll: (...args: unknown[]) => mockAll(...args),
}))

// Mock the WasmProvider's `useWasm` so we can toggle `loaded`.
const mockUseWasm = vi.fn()

vi.mock('@core/wasm-bridge/WasmProvider', () => ({
  useWasm: () => mockUseWasm(),
}))

// Import after the mocks are set up. The hook reads `useWasm()` so the
// import path doesn't matter for ordering here.
import { useBlockStrategy } from '../useBlockStrategy'

/** Build a minimal Block fixture. */
function makeBlock(overrides: Partial<Block> = {}, typeProp?: string): Block {
  const props: BlockProperty[] = []
  if (typeProp) {
    props.push({ key: 'type', value: typeProp, type: 'string' })
  }
  return {
    id: 'b1',
    pageId: 'p1',
    pageName: 'demo',
    content: '',
    blockType: 'paragraph',
    marker: null,
    priority: null,
    parentId: null,
    order: 1,
    level: 0,
    collapsed: false,
    properties: props,
    createdAt: '2026-06-02T00:00:00Z',
    updatedAt: '2026-06-02T00:00:00Z',
    ...overrides,
  } as Block
}

describe('useBlockStrategy', () => {
  beforeEach(() => {
    mockSelect.mockReset()
    mockAll.mockReset()
    mockUseWasm.mockReset()
    // Default: WASM is loaded; the bridge returns a sensible answer.
    mockUseWasm.mockReturnValue({ loaded: true, error: null })
    mockSelect.mockReturnValue('default')
    mockAll.mockReturnValue(['task', 'query', 'view', 'agent-run', 'default'])
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  it('returns the strategy name for a task block (WASM path)', () => {
    mockSelect.mockReturnValue('task')
    const { result } = renderHook(() => useBlockStrategy(makeBlock({}, 'task')))
    expect(result.current).toBe('task')
  })

  it('returns "query" for type:: query', () => {
    mockSelect.mockReturnValue('query')
    const { result } = renderHook(() =>
      useBlockStrategy(makeBlock({}, 'query')),
    )
    expect(result.current).toBe('query')
  })

  it('returns "view" for type:: view', () => {
    mockSelect.mockReturnValue('view')
    const { result } = renderHook(() =>
      useBlockStrategy(makeBlock({}, 'view')),
    )
    expect(result.current).toBe('view')
  })

  it('returns "agent-run" for type:: agent-run', () => {
    mockSelect.mockReturnValue('agent-run')
    const { result } = renderHook(() =>
      useBlockStrategy(makeBlock({}, 'agent-run')),
    )
    expect(result.current).toBe('agent-run')
  })

  it('returns "default" for an unknown type', () => {
    mockSelect.mockReturnValue('default')
    const { result } = renderHook(() =>
      useBlockStrategy(makeBlock({}, 'something-else')),
    )
    expect(result.current).toBe('default')
  })

  it('returns "default" for a block with no properties', () => {
    mockSelect.mockReturnValue('default')
    const block = makeBlock()
    block.properties = []
    const { result } = renderHook(() => useBlockStrategy(block))
    expect(result.current).toBe('default')
  })

  it('returns "default" for a block with no type property (only unrelated ones)', () => {
    mockSelect.mockReturnValue('default')
    const block = makeBlock()
    block.properties = [{ key: 'priority', value: 'A', type: 'string' }]
    const { result } = renderHook(() => useBlockStrategy(block))
    expect(result.current).toBe('default')
  })

  it('passes the Block through to the bridge (conversion is the bridge\'s job)', () => {
    // The hook itself is a thin memoized wrapper: it forwards the
    // block to `wasmStrategySelect` and the bridge (tested in
    // `wasm-loader.test.ts`) does the array→object JSON conversion.
    // Pin that contract: the hook hands the block to the bridge
    // without touching its shape.
    mockSelect.mockReturnValue('task')
    const block = makeBlock()
    block.properties = [
      { key: 'type', value: 'task', type: 'string' },
      { key: 'priority', value: 'A', type: 'string' },
    ]
    renderHook(() => useBlockStrategy(block))
    expect(mockSelect).toHaveBeenCalledTimes(1)
    // The hook passes the block as-is; the bridge converts.
    expect(mockSelect.mock.calls[0]?.[0]).toBe(block)
  })

  it('falls back to JS-only selector when WASM is not loaded (returns the correct strategy)', () => {
    // Simulate WasmProvider reporting not-loaded. The hook should
    // NOT call the bridge; instead it runs the JS-only selector
    // (which mirrors the Rust one) so the UI behaves identically
    // with or without WASM.
    mockUseWasm.mockReturnValue({ loaded: false, error: null })
    mockSelect.mockClear()
    const { result: r1 } = renderHook(() =>
      useBlockStrategy(makeBlock({}, 'task')),
    )
    expect(r1.current).toBe('task')
    const { result: r2 } = renderHook(() =>
      useBlockStrategy(makeBlock({}, 'view')),
    )
    expect(r2.current).toBe('view')
    const { result: r3 } = renderHook(() =>
      useBlockStrategy(makeBlock({}, 'agent-run')),
    )
    expect(r3.current).toBe('agent-run')
    const { result: r4 } = renderHook(() =>
      useBlockStrategy(makeBlock({}, 'something-else')),
    )
    expect(r4.current).toBe('default')
    // The fallback path must not touch the WASM bridge at all.
    expect(mockSelect).not.toHaveBeenCalled()
  })

  it('falls back to JS-only selector when WASM bridge throws', () => {
    mockUseWasm.mockReturnValue({ loaded: true, error: null })
    mockSelect.mockImplementation(() => {
      throw new Error('WASM not ready')
    })
    const { result } = renderHook(() => useBlockStrategy(makeBlock({}, 'task')))
    expect(result.current).toBe('task')
  })

  it('falls back to JS-only selector when WASM bridge returns null', () => {
    mockSelect.mockReturnValue(null)
    const { result } = renderHook(() => useBlockStrategy(makeBlock({}, 'task')))
    expect(result.current).toBe('task')
  })

  it('falls back to JS-only selector when WASM bridge returns undefined', () => {
    mockSelect.mockReturnValue(undefined)
    const { result } = renderHook(() => useBlockStrategy(makeBlock({}, 'task')))
    expect(result.current).toBe('task')
  })

  it('falls back to JS-only selector when the WASM context reports an error', () => {
    mockUseWasm.mockReturnValue({ loaded: true, error: 'load failed' })
    mockSelect.mockClear()
    const { result } = renderHook(() => useBlockStrategy(makeBlock({}, 'task')))
    expect(result.current).toBe('task')
    expect(mockSelect).not.toHaveBeenCalled()
  })

  it('re-resolves when the block reference changes', () => {
    mockSelect.mockImplementation(() => 'task')
    const { result, rerender } = renderHook(
      ({ block }) => useBlockStrategy(block),
      { initialProps: { block: makeBlock({}, 'task') } },
    )
    expect(result.current).toBe('task')

    mockSelect.mockImplementation(() => 'query')
    rerender({ block: makeBlock({}, 'query') })
    expect(result.current).toBe('query')
  })

  it('is stable across re-renders for the same block reference', () => {
    mockSelect.mockReturnValue('task')
    const block = makeBlock({}, 'task')
    const { result, rerender } = renderHook(() => useBlockStrategy(block))
    const first = result.current
    rerender()
    expect(result.current).toBe(first)
  })
})
