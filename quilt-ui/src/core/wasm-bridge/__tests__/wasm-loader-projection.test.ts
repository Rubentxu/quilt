/**
 * Tests for the WASM bridge shim functions in `wasm-loader.ts`.
 *
 * The Projection bridge (`wasmProjectionResolve`) is the new ADR-0028
 * addition. It has to do five things that aren't obvious from the type:
 *
 *   1. Convert the front-end `Block.properties: BlockProperty[]` shape
 *      into the `BlockDto` JSON the WASM function expects. This is
 *      the JSON-contract test.
 *
 *   2. Return `null` if the WASM pkg doesn't have the
 *      `projection_resolve` export (the WASM build was not run yet,
 *      or the user's browser doesn't support WASM).
 *
 *   3. Coerce the WASM return value (a `WasmProjectionView` JSON) into
 *      the UI's `ProjectionView` shape — Date strings, array join,
 *      null preservation.
 *
 *   4. Lift the WASM-specific metadata (`wasm_contract_id`,
 *      `wasm_had_conflict`) to the top-level result so the hook can
 *      record them in metrics.
 *
 *   5. Swallow all exceptions and return `null` (the hook falls back
 *      to HTTP).
 *
 * We pre-load the module with a default export that has the
 * `projection_resolve` function attached, mirroring how `wasm-pack`
 * generates the real pkg output.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

// `vi.mock` is hoisted to the top of the file — variables it closes
// over must be created with `vi.hoisted`, not declared at the
// top-level (TDZ otherwise).
const { mockProjectionResolve } = vi.hoisted(() => {
  return { mockProjectionResolve: vi.fn() }
})

// Mock the pkg module BEFORE the loader resolves. The loader does
//   import initWasmRaw, { undo, redo, ... } from './pkg/quilt_core.js'
// so we need every named export the loader pulls in. We attach the
// mock function as a named export so the loader's `requireExport`
// / `getExport` shims find it.
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
    projection_resolve: mockProjectionResolve,
    WasmStrategySelector: vi.fn(),
  }
  // The default export is a function (the wasm-bindgen init shim).
  // Attach the namespace so `(initWasmRaw as any).projection_resolve`
  // resolves.
  const defaultExport = Object.assign(vi.fn().mockResolvedValue(undefined), ns)
  return {
    default: defaultExport,
    ...ns,
  }
})

// Import after mocks.
import { wasmProjectionResolve } from '../wasm-loader'

describe('wasm-loader Projection bridge', () => {
  beforeEach(() => {
    mockProjectionResolve.mockReset()
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  describe('blockToWasmJson (via call args)', () => {
    it('converts BlockProperty[] to a JSON object map', () => {
      mockProjectionResolve.mockReturnValue(
        JSON.stringify({
          text: 'Hello',
          links: [],
          children: [],
          decorations: [],
          conflicts: [],
          properties: { type: 'task' },
          wasm_source: true,
          wasm_contract_id: 'task',
          wasm_had_conflict: false,
        }),
      )
      wasmProjectionResolve({
        id: 'b1',
        pageId: 'p1',
        content: 'Hello',
        properties: [
          { key: 'type', value: 'task', type: 'string' },
          { key: 'status', value: 'done', type: 'string' },
        ],
      })
      const lastCallArg = mockProjectionResolve.mock.calls[0]?.[0] as string
      const parsed = JSON.parse(lastCallArg)
      expect(parsed.properties).toEqual({ type: 'task', status: 'done' })
      expect(parsed.id).toBe('b1')
      expect(parsed.pageId).toBe('p1')
      expect(parsed.content).toBe('Hello')
    })

    it('preserves boolean and number values as JSON natives', () => {
      mockProjectionResolve.mockReturnValue(
        JSON.stringify({ text: '', links: [], children: [], decorations: [], conflicts: [], properties: {}, wasm_source: true, wasm_contract_id: 'default', wasm_had_conflict: false }),
      )
      wasmProjectionResolve({
        id: 'b1',
        pageId: 'p1',
        content: '',
        properties: [
          { key: 'resolved', value: true, type: 'boolean' },
          { key: 'count', value: 42, type: 'number' },
        ],
      })
      const lastCallArg = mockProjectionResolve.mock.calls[0]?.[0] as string
      const parsed = JSON.parse(lastCallArg)
      expect(parsed.properties).toEqual({ resolved: true, count: 42 })
    })

    it('omits null-valued properties', () => {
      mockProjectionResolve.mockReturnValue(
        JSON.stringify({ text: '', links: [], children: [], decorations: [], conflicts: [], properties: {}, wasm_source: true, wasm_contract_id: 'default', wasm_had_conflict: false }),
      )
      wasmProjectionResolve({
        id: 'b1',
        pageId: 'p1',
        content: '',
        properties: [
          { key: 'type', value: 'task', type: 'string' },
          { key: 'cleared', value: null, type: 'string' },
        ],
      })
      const lastCallArg = mockProjectionResolve.mock.calls[0]?.[0] as string
      const parsed = JSON.parse(lastCallArg)
      expect(parsed.properties).toEqual({ type: 'task' })
    })

    it('handles a block with no properties', () => {
      mockProjectionResolve.mockReturnValue(
        JSON.stringify({ text: '', links: [], children: [], decorations: [], conflicts: [], properties: {}, wasm_source: true, wasm_contract_id: 'default', wasm_had_conflict: false }),
      )
      wasmProjectionResolve({ id: 'b1', pageId: 'p1', content: '' })
      const lastCallArg = mockProjectionResolve.mock.calls[0]?.[0] as string
      const parsed = JSON.parse(lastCallArg)
      expect(parsed.properties).toEqual({})
    })
  })

  describe('wasmProjectionResolve', () => {
    it('returns null when WASM returns null (no export)', () => {
      // Simulate the export throwing — the wrapper should return null.
      mockProjectionResolve.mockImplementation(() => {
        throw new Error('export not found')
      })
      const result = wasmProjectionResolve({
        id: 'b1',
        pageId: 'p1',
        content: 'Hello',
      })
      expect(result).toBeNull()
    })

    it('returns a WasmProjectionResult on success', () => {
      mockProjectionResolve.mockReturnValue(
        JSON.stringify({
          text: 'Buy milk',
          links: [],
          children: [],
          decorations: [
            {
              kind: 'task-checkbox',
              target: 'status',
              value: 'done',
              weight: 100,
            },
          ],
          conflicts: [],
          properties: { type: 'task', status: 'done', projection: 'task' },
          wasm_source: true,
          wasm_contract_id: 'task',
          wasm_had_conflict: false,
        }),
      )
      const result = wasmProjectionResolve({
        id: 'b1',
        pageId: 'p1',
        content: 'Buy milk',
        properties: [
          { key: 'type', value: 'task', type: 'string' },
          { key: 'status', value: 'done', type: 'string' },
        ],
      })
      expect(result).not.toBeNull()
      expect(result!.contractId).toBe('task')
      expect(result!.hadConflict).toBe(false)
      expect(result!.view.text).toBe('Buy milk')
      expect(result!.view.decorations).toEqual([
        { kind: 'task-checkbox', target: 'status', value: 'done', weight: 100 },
      ])
      expect(result!.view.properties).toEqual({
        type: 'task',
        status: 'done',
        projection: 'task',
      })
    })

    it('lifts contractId and hadConflict to the top level', () => {
      mockProjectionResolve.mockReturnValue(
        JSON.stringify({
          text: 'X',
          links: [],
          children: [],
          decorations: [],
          conflicts: [
            {
              reason: 'tied',
              candidates: ['task', 'media'],
              winner: null,
              blockId: 'b1',
            },
          ],
          properties: {},
          wasm_source: true,
          wasm_contract_id: 'default',
          wasm_had_conflict: true,
        }),
      )
      const result = wasmProjectionResolve({ id: 'b1', pageId: 'p1', content: 'X' })
      expect(result!.contractId).toBe('default')
      expect(result!.hadConflict).toBe(true)
      expect(result!.view.conflicts).toHaveLength(1)
    })

    it('preserves date strings as-is', () => {
      mockProjectionResolve.mockReturnValue(
        JSON.stringify({
          text: 'X',
          links: [],
          children: [],
          decorations: [
            {
              kind: 'date-indicator',
              target: 'deadline',
              value: '2026-12-31T00:00:00Z',
              weight: 95,
            },
          ],
          conflicts: [],
          properties: { deadline: '2026-12-31T00:00:00Z' },
          wasm_source: true,
          wasm_contract_id: 'date',
          wasm_had_conflict: false,
        }),
      )
      const result = wasmProjectionResolve({ id: 'b1', pageId: 'p1', content: 'X' })
      expect(result!.view.decorations[0]!.value).toBe('2026-12-31T00:00:00Z')
      expect(result!.view.properties.deadline).toBe('2026-12-31T00:00:00Z')
    })

    it('joins array values with comma + space', () => {
      mockProjectionResolve.mockReturnValue(
        JSON.stringify({
          text: 'X',
          links: [],
          children: [],
          decorations: [],
          conflicts: [],
          properties: { tags: ['rust', 'wasm', 'projection'] },
          wasm_source: true,
          wasm_contract_id: 'default',
          wasm_had_conflict: false,
        }),
      )
      const result = wasmProjectionResolve({ id: 'b1', pageId: 'p1', content: 'X' })
      expect(result!.view.properties.tags).toBe('rust, wasm, projection')
    })

    it('preserves null values', () => {
      mockProjectionResolve.mockReturnValue(
        JSON.stringify({
          text: 'X',
          links: [],
          children: [],
          decorations: [],
          conflicts: [],
          properties: { status: null },
          wasm_source: true,
          wasm_contract_id: 'default',
          wasm_had_conflict: false,
        }),
      )
      const result = wasmProjectionResolve({ id: 'b1', pageId: 'p1', content: 'X' })
      expect(result!.view.properties.status).toBeNull()
    })

    it('returns null when JSON.parse throws', () => {
      mockProjectionResolve.mockReturnValue('{not valid json')
      const result = wasmProjectionResolve({ id: 'b1', pageId: 'p1', content: 'X' })
      expect(result).toBeNull()
    })

    it('returns null when WASM returns a non-object', () => {
      mockProjectionResolve.mockReturnValue(JSON.stringify('just a string'))
      const result = wasmProjectionResolve({ id: 'b1', pageId: 'p1', content: 'X' })
      expect(result).toBeNull()
    })

    it('handles a missing wasm_contract_id gracefully (defaults to "default")', () => {
      mockProjectionResolve.mockReturnValue(
        JSON.stringify({
          text: 'X',
          links: [],
          children: [],
          decorations: [],
          conflicts: [],
          properties: {},
          wasm_source: true,
        }),
      )
      const result = wasmProjectionResolve({ id: 'b1', pageId: 'p1', content: 'X' })
      expect(result!.contractId).toBe('default')
    })
  })
})
