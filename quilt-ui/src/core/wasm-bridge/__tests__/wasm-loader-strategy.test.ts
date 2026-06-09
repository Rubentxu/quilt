/**
 * Tests for the WASM bridge shim functions in `wasm-loader.ts`.
 *
 * The StrategySelector bridge (`wasmStrategySelect`, `wasmStrategyAll`)
 * has to do one thing that isn't obvious from the type:
 *
 *   1. Coerce the front-end `Block.properties: BlockProperty[]` shape
 *      into the `{"properties": {key: value}}` JSON the WASM function
 *      expects. This is the JSON-contract test.
 *
 * The `requireExport('WasmStrategySelector')` call inside the bridge
 * would throw "export not found" if the pkg mock doesn't provide it.
 * We pre-load the module with a default export that has the
 * constructor attached, mirroring how `wasm-pack` generates the
 * real pkg output.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

// `vi.mock` is hoisted to the top of the file — variables it closes
// over must be created with `vi.hoisted`, not declared at the
// top-level (TDZ otherwise).
const { mockInstance, MockWasmStrategySelector } = vi.hoisted(() => {
  const instance = {
    select: vi.fn(),
    all_strategies: vi.fn(),
  }
  const Ctor = vi.fn(function MockWasmStrategySelector() {
    return instance
  })
  return { mockInstance: instance, MockWasmStrategySelector: Ctor }
})

// Mock the pkg module BEFORE the loader resolves. The loader does
//   import initWasmRaw, { undo, redo, ... } from './pkg/quilt_core.js'
// so we need every named export the loader pulls in. We attach the
// mock class to the default export so `requireExport('WasmStrategySelector')`
// inside the loader finds it (it walks `(initWasmRaw as any)` first).
vi.mock('../pkg/quilt_core.js', () => {
  const noop = vi.fn()
  const ns = {
    init: vi.fn().mockResolvedValue(undefined),
    ping: () => true,
    get_version: () => 'test',
    get_state: noop,
    load_page: noop,
    dispatch: noop,
    undo: noop,
    redo: noop,
    parse_inline: noop,
    WasmStrategySelector: MockWasmStrategySelector,
  }
  // The default export is a function (the wasm-bindgen init shim).
  // Attach the namespace so `(initWasmRaw as any).WasmStrategySelector`
  // resolves. This mirrors how the real pkg module works.
  const defaultExport = Object.assign(vi.fn().mockResolvedValue(undefined), ns)
  return {
    default: defaultExport,
    ...ns,
  }
})

// Import after mocks. The loader will pick up our `WasmStrategySelector`
// from the mocked pkg.
import { wasmStrategySelect, wasmStrategyAll } from '../wasm-loader'

describe('wasm-loader StrategySelector bridge', () => {
  beforeEach(() => {
    MockWasmStrategySelector.mockClear()
    mockInstance.select.mockReset()
    mockInstance.all_strategies.mockReset()
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  describe('wasmStrategySelect', () => {
    it('builds the {"properties": {...}} JSON shape from a BlockProperty[]', () => {
      mockInstance.select.mockReturnValue('task')
      wasmStrategySelect({
        properties: [
          { key: 'type', value: 'task', type: 'string' },
          { key: 'priority', value: 'A', type: 'string' },
        ],
      })
      const lastCallArg = mockInstance.select.mock.calls[0]?.[0]
      expect(typeof lastCallArg).toBe('string')
      expect(JSON.parse(lastCallArg as string)).toEqual({
        properties: { type: 'task', priority: 'A' },
      })
    })

    it('coerces non-string property values to strings (e.g. booleans)', () => {
      mockInstance.select.mockReturnValue('default')
      wasmStrategySelect({
        properties: [
          { key: 'type', value: 'task', type: 'string' },
          { key: 'resolved', value: true, type: 'boolean' },
          { key: 'count', value: 42, type: 'number' },
        ],
      })
      const lastCallArg = mockInstance.select.mock.calls[0]?.[0] as string
      expect(JSON.parse(lastCallArg)).toEqual({
        properties: { type: 'task', resolved: 'true', count: '42' },
      })
    })

    it('omits null-valued properties from the JSON', () => {
      mockInstance.select.mockReturnValue('default')
      wasmStrategySelect({
        properties: [
          { key: 'type', value: 'task', type: 'string' },
          { key: 'cleared', value: null, type: 'string' },
        ],
      })
      const lastCallArg = mockInstance.select.mock.calls[0]?.[0] as string
      const parsed = JSON.parse(lastCallArg)
      expect(parsed.properties).toEqual({ type: 'task' })
    })

    it('returns the strategy name when WASM returns a string', () => {
      mockInstance.select.mockReturnValue('query')
      const out = wasmStrategySelect({
        properties: [{ key: 'type', value: 'query', type: 'string' }],
      })
      expect(out).toBe('query')
    })

    it('returns null when WASM returns null (no matching strategy)', () => {
      mockInstance.select.mockReturnValue(null)
      const out = wasmStrategySelect({
        properties: [{ key: 'type', value: 'unknown', type: 'string' }],
      })
      expect(out).toBeNull()
    })

    it('returns null when WASM returns undefined', () => {
      mockInstance.select.mockReturnValue(undefined)
      const out = wasmStrategySelect({
        properties: [{ key: 'type', value: 'task', type: 'string' }],
      })
      expect(out).toBeNull()
    })

    it('handles a block with undefined properties', () => {
      mockInstance.select.mockReturnValue('default')
      wasmStrategySelect({})
      const lastCallArg = mockInstance.select.mock.calls[0]?.[0] as string
      expect(JSON.parse(lastCallArg)).toEqual({ properties: {} })
    })
  })

  describe('wasmStrategyAll', () => {
    it('returns the array of strategy names from WASM', () => {
      mockInstance.all_strategies.mockReturnValue([
        'task',
        'query',
        'view',
        'agent-run',
        'default',
      ])
      const out = wasmStrategyAll()
      expect(out).toEqual(['task', 'query', 'view', 'agent-run', 'default'])
    })

    it('returns an empty array when WASM returns null', () => {
      mockInstance.all_strategies.mockReturnValue(null)
      const out = wasmStrategyAll()
      expect(out).toEqual([])
    })

    it('returns an empty array when WASM returns undefined', () => {
      mockInstance.all_strategies.mockReturnValue(undefined)
      const out = wasmStrategyAll()
      expect(out).toEqual([])
    })
  })
})
