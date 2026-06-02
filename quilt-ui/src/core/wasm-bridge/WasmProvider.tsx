import {
  createContext,
  useContext,
  useState,
  useCallback,
  type ReactNode,
} from 'react'
import {
  loadWasm,
  ping,
  get_version,
  wasmGetState,
  wasmLoadPage,
  wasmDispatch,
  wasmParseInline,
  wasmUndo,
  wasmRedo,
} from './wasm-loader'

// ─── Lazy WASM loading ─────────────────────────────────────────
// The WASM blob is ~1.7 MB raw (~462 KB gzipped). Loading it on mount
// blocks the entire UI until the engine is ready. Instead we expose
// `ensureWasmLoaded()` and let consumers (or the provider) trigger
// it on first use. The promise is cached so concurrent callers share
// one fetch and one instantiation.

let wasmLoadPromise: Promise<void> | null = null
let wasmLoadResult: 'idle' | 'loading' | 'ready' | 'error' = 'idle'

/** Returns the current load state of the WASM engine. */
export function getWasmLoadState(): 'idle' | 'loading' | 'ready' | 'error' {
  return wasmLoadResult
}

/**
 * Start loading the WASM engine if it isn't already. Idempotent —
 * concurrent calls share a single in-flight promise.
 */
export function ensureWasmLoaded(): Promise<void> {
  if (wasmLoadPromise) return wasmLoadPromise

  wasmLoadResult = 'loading'
  wasmLoadPromise = (async () => {
    try {
      await loadWasm()
      wasmLoadResult = 'ready'
    } catch (e) {
      wasmLoadResult = 'error'
      // Reset so a retry can kick a fresh load attempt
      wasmLoadPromise = null
      throw e
    }
  })()

  return wasmLoadPromise
}

interface WasmState {
  loaded: boolean
  error: string | null
  /** Returns the crate version string */
  wasmGetVersion: () => string
  /** Returns true if WASM is alive */
  wasmPing: () => boolean
  /** Returns parsed state for a page (JSON.parse of the return value) */
  wasmGetState: (pageId: string) => any
  /** Load blocks into WASM state for a page */
  wasmLoadPage: (pageId: string, blocks: any) => any
  /** Dispatch an OutlinerCommand, returns { accepted, stateHash } */
  wasmDispatch: (pageId: string, command: any) => any
  /** Undo for a page, returns { ok: bool } (legacy no-op stub) */
  wasmUndo: (pageId: string) => any
  /** Redo for a page, returns { ok: bool } (legacy no-op stub) */
  wasmRedo: (pageId: string) => any
  /** Parse inline content, returns { rawText, segments } */
  wasmParseInline: (content: string) => any
  retry: () => void
}

const WasmContext = createContext<WasmState | null>(null)

export function useWasm() {
  const ctx = useContext(WasmContext)
  if (!ctx) throw new Error('useWasm must be used within WasmProvider')
  return ctx
}

export function WasmProvider({ children }: { children: ReactNode }) {
  // Track load state locally so we can re-render on transition. We
  // start from the cached global state in case ensureWasmLoaded() has
  // already been called by another component.
  const [loaded, setLoaded] = useState(() => wasmLoadResult === 'ready')
  const [error, setError] = useState<string | null>(null)

  const loadWasmOnce = useCallback(async () => {
    setError(null)
    try {
      await ensureWasmLoaded()
      setLoaded(true)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load Quilt engine')
    }
  }, [])

  // Intentionally NO auto-load effect — WASM is loaded lazily on
  // first use by components that need it (PageView, InlineContent
  // call ensureWasmLoaded() before invoking wasm functions). This
  // keeps the ~1.7 MB WASM binary off the critical path of routes
  // that don't touch the engine (Settings, AllPages, Graph).

  if (error) {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="text-center">
          <h2 className="text-xl font-bold mb-2">Unable to load Quilt engine</h2>
          <p className="text-gray-500 mb-4">{error}</p>
          <button
            onClick={loadWasmOnce}
            className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
          >
            Retry
          </button>
        </div>
      </div>
    )
  }

  return (
    <WasmContext.Provider
      value={{
        loaded,
        error,
        wasmGetVersion: get_version,
        wasmPing: ping,
        wasmGetState,
        wasmLoadPage,
        wasmDispatch,
        wasmUndo,
        wasmRedo,
        wasmParseInline,
        retry: loadWasmOnce,
      }}
    >
      {children}
    </WasmContext.Provider>
  )
}
