import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { RecentsSection } from '../sections/RecentsSection'
import { STORAGE_KEYS } from '../storage-keys'

// ─── Mutable mock state ───────────────────────────────────────────
//
// `useLocation` returns an object whose `pathname` is read on every
// render. We expose a mutable `mockPathname` so tests can simulate
// route changes by reassigning it, then re-render. Tests do NOT use
// the real router — the whole point is to verify the recents
// tracking in isolation, not the router itself.

let mockPathname = '/'
const mockLocation = {
  get pathname() {
    return mockPathname
  },
}
const mockNavigate = vi.fn()
const mockGetPage = vi.fn()
const mockToastError = vi.fn()
const mockToastSuccess = vi.fn()

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
  useLocation: () => mockLocation,
}))

vi.mock('@core/api-client', () => ({
  // Export both `api` and `QuiltApiError` — the component does an
  // `instanceof QuiltApiError` check on the rejection. The shim class
  // accepts `{ status }` and is enough to make the check work; the
  // tests below construct their 404 error as a real `QuiltApiError`
  // to exercise the real code path.
  QuiltApiError: class FakeQuiltApiError extends Error {
    status: number
    code: string
    constructor(status: number, code: string, detail: string) {
      super(detail)
      this.name = 'QuiltApiError'
      this.status = status
      this.code = code
    }
  },
  api: {
    getPage: (...args: unknown[]) => mockGetPage(...args),
  },
}))

vi.mock('react-hot-toast', () => ({
  default: {
    error: (...args: unknown[]) => mockToastError(...args),
    success: (...args: unknown[]) => mockToastSuccess(...args),
  },
}))

function setMockPath(path: string) {
  mockPathname = path
}

function getRecentsStorage(): unknown[] {
  const raw = localStorage.getItem(STORAGE_KEYS.RECENTS)
  if (!raw) return []
  try {
    return JSON.parse(raw)
  } catch {
    return []
  }
}

function setRecentsStorage(entries: unknown) {
  const value = typeof entries === 'string' ? entries : JSON.stringify(entries)
  localStorage.setItem(STORAGE_KEYS.RECENTS, value)
}

beforeEach(() => {
  mockPathname = '/'
  mockNavigate.mockReset()
  mockGetPage.mockReset()
  mockToastError.mockReset()
  mockToastSuccess.mockReset()
  localStorage.clear()
  // Deterministic timestamps — we only fake `Date` so `Date.now()`
  // and `new Date()` return the same fixed value, but we leave the
  // real `setTimeout`/`setInterval` in place. That way `waitFor`
  // (which polls via `setTimeout`) still resolves normally.
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date('2026-06-05T12:00:00Z'))
})

afterEach(() => {
  vi.useRealTimers()
})

// ─── Tests ────────────────────────────────────────────────────────
//
// These cover the scenarios from `sidebar-recents.spec.md` and the
// "Recents" section of the orchestrator's PR 2 brief. Each test is
// named after the spec scenario it pins down.
//
// User event setup: `advanceTimers` is required so the click handler
// can `await` async `getPage` calls against the fake timer queue.

const setupUser = () => userEvent.setup({ advanceTimers: vi.advanceTimersByTime })

describe('RecentsSection — sidebar-recents capability', () => {
  describe('initial load from storage', () => {
    it('renders entries persisted in localStorage on mount (Scenario: Initial load)', async () => {
      setRecentsStorage([
        { name: 'foo', url: '/page/foo', visitedAt: 1_700_000_000_000 },
        { name: 'bar', url: '/page/bar', visitedAt: 1_699_000_000_000 },
      ])
      // Pick a path that is NOT a stored entry so we can isolate the
      // "render from storage" behaviour from the "record this route"
      // behaviour. The current route WILL be prepended, but `foo` and
      // `bar` must still be visible.
      setMockPath('/page/elsewhere')

      render(<RecentsSection collapsed={false} />)

      await waitFor(() => {
        expect(screen.getByText('foo')).toBeInTheDocument()
        expect(screen.getByText('bar')).toBeInTheDocument()
      })
    })

    it('tolerates malformed localStorage and still renders the empty/header state (Scenario: Malformed storage)', async () => {
      setRecentsStorage('not valid json')
      setMockPath('/page/elsewhere')

      // Spy on console.error to ensure no crash is logged.
      const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

      render(<RecentsSection collapsed={false} />)

      // The header is rendered (collapsed=false, the section is shown).
      expect(
        screen.getByRole('heading', { name: 'Recientes' }),
      ).toBeInTheDocument()

      // No error escaped to the console from the component itself.
      expect(errorSpy).not.toHaveBeenCalled()
      errorSpy.mockRestore()
    })

    it('drops schema-invalid entries on read — only valid {name,url,visitedAt} survive', async () => {
      setRecentsStorage([
        { name: 'foo', url: '/page/foo', visitedAt: 1 }, // valid
        { name: 'bar' }, // missing url, visitedAt
        { notanentry: true },
        null,
        'string',
        42,
      ])
      setMockPath('/page/elsewhere')

      render(<RecentsSection collapsed={false} />)

      await waitFor(() => {
        expect(screen.getByText('foo')).toBeInTheDocument()
      })
      // 'bar' (malformed) was dropped, so it must not be in the DOM.
      expect(screen.queryByText('bar')).not.toBeInTheDocument()
    })
  })

  describe('route tracking', () => {
    it('records the current /page/:name route on mount (Scenario: Visit records entry)', async () => {
      setMockPath('/page/foo')
      setRecentsStorage([])

      render(<RecentsSection collapsed={false} />)

      await waitFor(() => {
        expect(screen.getByText('foo')).toBeInTheDocument()
      })
      const stored = getRecentsStorage() as Array<{ name: string; url: string }>
      expect(stored[0]).toMatchObject({ name: 'foo', url: '/page/foo' })
    })

    it('ignores non-page routes on mount — storage stays empty (Scenario: Non-page routes ignored)', async () => {
      setMockPath('/settings')
      setRecentsStorage([])

      render(<RecentsSection collapsed={false} />)

      // Let a couple of ticks pass to make sure no recording happened.
      await new Promise((r) => setTimeout(r, 0))
      expect(getRecentsStorage()).toEqual([])
    })

    it('prepends a new entry when the route changes between /page/:name (Scenario: Visit records entry)', async () => {
      setMockPath('/page/foo')
      setRecentsStorage([])

      const { rerender } = render(<RecentsSection collapsed={false} />)

      await waitFor(() => {
        const stored = getRecentsStorage() as Array<{ name: string }>
        expect(stored).toHaveLength(1)
        expect(stored[0].name).toBe('foo')
      })

      setMockPath('/page/bar')
      rerender(<RecentsSection collapsed={false} />)

      await waitFor(() => {
        const stored = getRecentsStorage() as Array<{ name: string; visitedAt: number }>
        expect(stored[0].name).toBe('bar')
        expect(stored[1].name).toBe('foo')
        // 'bar' has a fresh visitedAt (the system time we set in beforeEach)
        expect(stored[0].visitedAt).toBe(new Date('2026-06-05T12:00:00Z').getTime())
      })
    })

    it('does NOT record when the route changes to a non-page path (Scenario: Non-page routes ignored)', async () => {
      setMockPath('/page/foo')
      setRecentsStorage([])

      const { rerender } = render(<RecentsSection collapsed={false} />)

      await waitFor(() => {
        expect(getRecentsStorage()).toHaveLength(1)
      })

      setMockPath('/settings')
      rerender(<RecentsSection collapsed={false} />)

      // Let a tick pass and re-check.
      await new Promise((r) => setTimeout(r, 0))
      const stored = getRecentsStorage() as Array<{ name: string }>
      expect(stored).toHaveLength(1)
      expect(stored[0].name).toBe('foo')
    })
  })

  describe('deduplication and cap', () => {
    it('moves an existing entry to the top on re-visit (Scenario: Re-visit moves to top)', async () => {
      setMockPath('/page/a')
      setRecentsStorage([
        { name: 'a', url: '/page/a', visitedAt: 100 },
        { name: 'b', url: '/page/b', visitedAt: 200 },
        { name: 'c', url: '/page/c', visitedAt: 300 },
      ])

      const { rerender } = render(<RecentsSection collapsed={false} />)

      // After mount: `a` is re-recorded at the top with fresh timestamp.
      await waitFor(() => {
        const stored = getRecentsStorage() as Array<{ name: string }>
        expect(stored[0].name).toBe('a')
      })

      setMockPath('/page/b')
      rerender(<RecentsSection collapsed={false} />)

      await waitFor(() => {
        const stored = getRecentsStorage() as Array<{ name: string }>
        // No duplicate — `b` is at the top, then `a`, then `c`.
        expect(stored.map((s) => s.name)).toEqual(['b', 'a', 'c'])
      })
    })

    it('caps the list at 5 — evicts oldest on overflow (Scenario: Cap enforced at 5)', async () => {
      // The component sorts by `visitedAt` desc on read and on write.
      // To set up a "list at the cap" without time-based flakiness, we
      // pre-seed with visitedAt values that are all in the PAST relative
      // to the fake clock's "now" (the system time set in beforeEach).
      setMockPath('/page/a')
      setRecentsStorage([
        { name: 'a', url: '/page/a', visitedAt: 100 },
        { name: 'b', url: '/page/b', visitedAt: 200 },
        { name: 'c', url: '/page/c', visitedAt: 300 },
        { name: 'd', url: '/page/d', visitedAt: 400 },
        { name: 'e', url: '/page/e', visitedAt: 500 },
      ])

      const { rerender } = render(<RecentsSection collapsed={false} />)

      // On mount: `a` is re-recorded with the fake clock's timestamp
      // (~1.748e12 — way above the stored values), so it lands at the
      // top. The other 4 are then sorted by visitedAt desc, giving
      // `a, e, d, c, b`. The cap still holds at 5.
      await waitFor(() => {
        const stored = getRecentsStorage() as Array<{ name: string }>
        expect(stored.map((s) => s.name)).toEqual(['a', 'e', 'd', 'c', 'b'])
        expect(stored).toHaveLength(5)
      })

      // Visit a new page. `b` (oldest visitedAt in the original 5) is
      // now at the tail of the cap — visiting a brand-new page bumps
      // it off.
      setMockPath('/page/f')
      rerender(<RecentsSection collapsed={false} />)

      await waitFor(() => {
        const stored = getRecentsStorage() as Array<{ name: string }>
        // f is new, a is preserved with its fresh timestamp; the rest
        // are sorted by visitedAt desc. b was evicted.
        expect(stored.map((s) => s.name)).toEqual(['f', 'a', 'e', 'd', 'c'])
        // Cap holds
        expect(stored).toHaveLength(5)
      })
    })

    it('deduplicates case-insensitively, preserving the original casing (Scenario: Case-insensitive dedup)', async () => {
      setMockPath('/page/Foo')
      setRecentsStorage([
        { name: 'Foo', url: '/page/Foo', visitedAt: 100 },
        { name: 'bar', url: '/page/bar', visitedAt: 200 },
      ])

      const { rerender } = render(<RecentsSection collapsed={false} />)

      await waitFor(() => {
        const stored = getRecentsStorage() as Array<{ name: string }>
        const fooEntries = stored.filter((s) => s.name.toLowerCase() === 'foo')
        expect(fooEntries).toHaveLength(1)
        // Original casing preserved
        expect(fooEntries[0].name).toBe('Foo')
      })

      // Re-visit with a different case; should still produce ONE entry.
      setMockPath('/page/foo')
      rerender(<RecentsSection collapsed={false} />)

      await waitFor(() => {
        const stored = getRecentsStorage() as Array<{ name: string }>
        const fooEntries = stored.filter((s) => s.name.toLowerCase() === 'foo')
        expect(fooEntries).toHaveLength(1)
        // Casing preserved on the original 'Foo' — we did NOT overwrite.
        expect(fooEntries[0].name).toBe('Foo')
      })
    })

    it('renders newest entry first (Scenario: Newest entry first)', async () => {
      setMockPath('/page/elsewhere')
      setRecentsStorage([
        { name: 'older', url: '/page/older', visitedAt: 100 },
        { name: 'newer', url: '/page/newer', visitedAt: 200 },
      ])

      render(<RecentsSection collapsed={false} />)

      await waitFor(() => {
        expect(screen.getByText('newer')).toBeInTheDocument()
        expect(screen.getByText('older')).toBeInTheDocument()
      })
      // Recents render as <button>s (the click handler does an async
      // self-heal check before navigating). Get them in DOM order.
      const buttons = screen.getAllByTestId(/^recent-/)
      const newerIdx = buttons.findIndex((b) => b.textContent?.includes('newer'))
      const olderIdx = buttons.findIndex((b) => b.textContent?.includes('older'))
      expect(newerIdx).toBeGreaterThanOrEqual(0)
      expect(olderIdx).toBeGreaterThan(newerIdx)
    })
  })

  describe('click navigation and self-heal', () => {
    it('navigates to the recorded url when an item is clicked (Scenario: Click navigates)', async () => {
      const user = setupUser()
      setMockPath('/page/elsewhere')
      setRecentsStorage([
        { name: 'foo', url: '/page/foo', visitedAt: 1 },
      ])
      mockGetPage.mockResolvedValue({ name: 'foo' })

      render(<RecentsSection collapsed={false} />)

      await waitFor(() => {
        expect(screen.getByText('foo')).toBeInTheDocument()
      })

      await user.click(screen.getByText('foo'))

      await waitFor(() => {
        expect(mockGetPage).toHaveBeenCalledWith('foo')
      })
      expect(mockNavigate).toHaveBeenCalledWith({ to: '/page/foo' })
    })

    it('self-heals on 404 — removes entry, toasts, does NOT navigate (Scenario: Dead entry self-heals)', async () => {
      const user = setupUser()
      // Use a non-page path so the component does NOT prepend a current-
      // page entry on mount — we want to isolate the `ghost` self-heal.
      setMockPath('/settings')
      setRecentsStorage([
        { name: 'ghost', url: '/page/ghost', visitedAt: 1 },
      ])
      // Use a real QuiltApiError (status: 404) so the component's
      // `instanceof` check exercises the production code path.
      const { QuiltApiError } = await import('@core/api-client')
      mockGetPage.mockRejectedValue(
        new QuiltApiError(404, 'NOT_FOUND', 'Page not found'),
      )

      render(<RecentsSection collapsed={false} />)

      await waitFor(() => {
        expect(screen.getByText('ghost')).toBeInTheDocument()
      })

      await user.click(screen.getByText('ghost'))

      await waitFor(() => {
        expect(mockToastError).toHaveBeenCalledWith('Page not found')
      })
      expect(mockNavigate).not.toHaveBeenCalled()

      // Entry removed from storage — only `ghost` was there.
      const stored = getRecentsStorage() as unknown[]
      expect(stored).toHaveLength(0)
      // Entry removed from DOM
      expect(screen.queryByText('ghost')).not.toBeInTheDocument()
    })
  })
})
