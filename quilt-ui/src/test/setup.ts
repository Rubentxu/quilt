// Vitest setup — runs once before every test file.
//
// We mock the browser APIs that the app uses but jsdom doesn't
// implement, so components don't crash on mount.

import '@testing-library/jest-dom'
import { vi } from 'vitest'

// ── matchMedia ─────────────────────────────────────────────────────

// ── localStorage ────────────────────────────────────────────────────
// vitest 3 + jsdom 25 occasionally hands tests a `localStorage`
// stub that is an empty object (no `setItem` / `getItem` / `clear`).
// Components that persist user data (favorites, recents, theme) and
// the tests that exercise them would crash. We replace the stub with
// an in-memory `Storage` shim before any user code runs.

if (typeof globalThis.localStorage === 'undefined' || typeof globalThis.localStorage.setItem !== 'function') {
  const store = new Map<string, string>()
  const shim: Storage = {
    get length() {
      return store.size
    },
    clear() {
      store.clear()
    },
    getItem(key) {
      return store.has(key) ? store.get(key)! : null
    },
    key(index) {
      return Array.from(store.keys())[index] ?? null
    },
    removeItem(key) {
      store.delete(key)
    },
    setItem(key, value) {
      store.set(key, String(value))
    },
  }
  // jsdom provides `window` but not always `globalThis.localStorage` —
  // assign both so the rest of the suite sees the shim.
  ;(globalThis as { localStorage?: Storage }).localStorage = shim
  if (typeof window !== 'undefined') {
    ;(window as { localStorage: Storage }).localStorage = shim
  }
}


// useMediaQuery, useResponsive, lucide-react, and a few radix-style
// primitives all call window.matchMedia. jsdom returns a stub that
// throws on `.addEventListener`, so we replace it with a no-op.

Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation(query => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
})

// ── IntersectionObserver ──────────────────────────────────────────
// jsdom doesn't implement it. Components like HoverPreview lazy-load
// and several feature folders use it for viewport-based effects.

class MockIntersectionObserver {
  observe = vi.fn()
  unobserve = vi.fn()
  disconnect = vi.fn()
  takeRecords = vi.fn()
  root = null
  rootMargin = ''
  thresholds = []
}
globalThis.IntersectionObserver = MockIntersectionObserver as any

// ── ResizeObserver (some libraries) ───────────────────────────────
class MockResizeObserver {
  observe = vi.fn()
  unobserve = vi.fn()
  disconnect = vi.fn()
}
globalThis.ResizeObserver = MockResizeObserver as any

// ── Range / DOMRect ──────────────────────────────────────────────
// jsdom does not implement `Range.prototype.getBoundingClientRect`.
// BlockRow (and other editors) call it on every keystroke to position
// autocomplete dropdowns. Returning a zero-rect at the origin keeps
// dropdowns visible in tests without measuring real layout.
if (typeof Range !== 'undefined' && !Range.prototype.getBoundingClientRect) {
  Range.prototype.getBoundingClientRect = function getBoundingClientRect() {
    return {
      top: 0,
      right: 0,
      bottom: 0,
      left: 0,
      width: 0,
      height: 0,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    } as DOMRect
  }
}
