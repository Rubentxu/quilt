/**
 * Tests for WasmProvider — context provider for WASM engine.
 * Mocks the wasm-loader to avoid loading real WASM binaries.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, renderHook } from '@testing-library/react'
import { WasmProvider, useWasm } from '@core/wasm-bridge/WasmProvider'

// ── Mock wasm-loader (hoisted) ─────────────────────────────

vi.mock('@core/wasm-bridge/wasm-loader', () => ({
  loadWasm: vi.fn(),
  ping: () => true,
  get_version: () => '0.1.0-test',
  wasmGetState: vi.fn(),
  wasmLoadPage: vi.fn(),
  wasmDispatch: vi.fn(),
  wasmUndo: vi.fn(),
  wasmRedo: vi.fn(),
  wasmParseInline: vi.fn(),
}))

describe('WasmProvider', () => {
  it('renders children', () => {
    render(
      <WasmProvider>
        <div data-testid="child">Hello</div>
      </WasmProvider>,
    )
    expect(screen.getByTestId('child')).toBeDefined()
  })

  it('provides WASM functions via context', () => {
    render(
      <WasmProvider>
        <ContextConsumer />
      </WasmProvider>,
    )
    expect(screen.getByTestId('version')).toHaveTextContent('0.1.0-test')
    expect(screen.getByTestId('ping')).toHaveTextContent('true')
    // WASM is lazy-loaded, not auto-loaded
    expect(screen.getByTestId('loaded')).toHaveTextContent('false')
  })

  it('context includes all expected functions', () => {
    render(
      <WasmProvider>
        <FullContextConsumer />
      </WasmProvider>,
    )
    expect(screen.getByTestId('has-retry')).toHaveTextContent('true')
    expect(screen.getByTestId('has-dispatch')).toHaveTextContent('true')
    expect(screen.getByTestId('has-parseInline')).toHaveTextContent('true')
  })
})

// ── Context consumers for testing ──────────────────────────

function ContextConsumer() {
  const ctx = useWasm()
  return (
    <div>
      <span data-testid="version">{ctx.wasmGetVersion()}</span>
      <span data-testid="ping">{String(ctx.wasmPing())}</span>
      <span data-testid="loaded">{String(ctx.loaded)}</span>
    </div>
  )
}

function FullContextConsumer() {
  const ctx = useWasm()
  return (
    <div>
      <span data-testid="has-retry">{String(typeof ctx.retry === 'function')}</span>
      <span data-testid="has-dispatch">{String(typeof ctx.wasmDispatch === 'function')}</span>
      <span data-testid="has-parseInline">{String(typeof ctx.wasmParseInline === 'function')}</span>
    </div>
  )
}

describe('useWasm', () => {
  it('throws when used outside WasmProvider', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})

    expect(() => {
      renderHook(() => useWasm())
    }).toThrow('useWasm must be used within WasmProvider')

    spy.mockRestore()
  })
})
