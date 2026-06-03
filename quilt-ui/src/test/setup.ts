// Vitest setup — runs once before every test file.
//
// We mock the browser APIs that the app uses but jsdom doesn't
// implement, so components don't crash on mount.

import '@testing-library/jest-dom'
import { vi } from 'vitest'

// ── matchMedia ─────────────────────────────────────────────────────
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
