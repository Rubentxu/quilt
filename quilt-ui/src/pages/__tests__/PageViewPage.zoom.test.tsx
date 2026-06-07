/**
 * Integration tests for PageViewPage's Block Zoom wiring.
 *
 * The page wrapper does three things for zoom:
 *   1. Read `?zoom=$blockId` from the URL on mount.
 *   2. Pass it down to <PageView zoomBlockId={...} />.
 *   3. When PageView's onZoomOut fires, strip the `?zoom=` param
 *      from the URL.
 *   4. Respond to popstate (browser back/forward) by re-reading
 *      the param.
 */
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { PageViewPage } from '../PageViewPage'

// ── Mocks ───────────────────────────────────────────────────────

const mockOpenTab = vi.fn()
const mockUseParams = vi.fn(() => ({ name: 'demo' }))

vi.mock('@shared/contexts/TabsContext', () => ({
  useTabs: () => ({ openTab: mockOpenTab }),
}))

vi.mock('@tanstack/react-router', () => ({
  useParams: (...args: any[]) => mockUseParams(...args),
}))

// PageView is heavy. We replace it with a small probe that exposes
// the props the page passes, so the test can assert the URL ↔ state
// wiring without re-implementing PageView's rendering logic.
const mockPageViewProps: { current: any } = { current: null }

vi.mock('@features/outliner-tiptap/PageView', () => ({
  PageView: (props: any) => {
    mockPageViewProps.current = props
    return (
      <div data-testid="probe-pageview">
        <span data-testid="probe-page-name">{props.pageName}</span>
        <span data-testid="probe-zoom-block-id">
          {props.zoomBlockId ?? '__none__'}
        </span>
        <button
          data-testid="probe-zoom-out"
          onClick={() => props.onZoomOut?.()}
        >
          Zoom out
        </button>
      </div>
    )
  },
}))

// ── Lifecycle ──────────────────────────────────────────────────

beforeEach(() => {
  vi.clearAllMocks()
  mockPageViewProps.current = null
  // Each test starts with a clean URL.
  window.history.replaceState({}, '', '/page/demo')
})

afterEach(() => {
  // Clean up URL state.
  window.history.replaceState({}, '', '/')
})

// ── Tests ──────────────────────────────────────────────────────

describe('PageViewPage — Block Zoom wiring', () => {
  it('reads ?zoom= from the URL on mount and passes it to PageView', async () => {
    window.history.replaceState({}, '', '/page/demo?zoom=block-abc')

    render(<PageViewPage />)

    await waitFor(() => {
      expect(mockPageViewProps.current).not.toBeNull()
    })

    expect(mockPageViewProps.current.zoomBlockId).toBe('block-abc')
    expect(mockPageViewProps.current.pageName).toBe('demo')
  })

  it('passes null zoomBlockId when ?zoom= is not in the URL', async () => {
    window.history.replaceState({}, '', '/page/demo')

    render(<PageViewPage />)

    await waitFor(() => {
      expect(mockPageViewProps.current).not.toBeNull()
    })

    expect(mockPageViewProps.current.zoomBlockId).toBeNull()
  })

  it('strips ?zoom= from the URL when onZoomOut is called', async () => {
    window.history.replaceState({}, '', '/page/demo?zoom=block-abc')

    render(<PageViewPage />)

    await waitFor(() => {
      expect(mockPageViewProps.current).not.toBeNull()
    })

    // Verify the probe got the initial zoom.
    expect(mockPageViewProps.current.zoomBlockId).toBe('block-abc')

    // Trigger zoom-out via the probe button.
    fireEvent.click(screen.getByTestId('probe-zoom-out'))

    // URL should no longer contain zoom=.
    await waitFor(() => {
      expect(window.location.search).not.toContain('zoom=')
    })
    expect(mockPageViewProps.current.zoomBlockId).toBeNull()
  })

  it('preserves other query params when stripping ?zoom=', async () => {
    window.history.replaceState({}, '', '/page/demo?view=kanban&zoom=block-abc')

    render(<PageViewPage />)

    await waitFor(() => {
      expect(mockPageViewProps.current).not.toBeNull()
    })

    fireEvent.click(screen.getByTestId('probe-zoom-out'))

    await waitFor(() => {
      expect(window.location.search).not.toContain('zoom=')
    })
    // view=kanban is still there.
    expect(window.location.search).toContain('view=kanban')
  })

  it('updates zoomBlockId when the URL changes via popstate', async () => {
    window.history.replaceState({}, '', '/page/demo')

    render(<PageViewPage />)

    await waitFor(() => {
      expect(mockPageViewProps.current).not.toBeNull()
    })

    expect(mockPageViewProps.current.zoomBlockId).toBeNull()

    // Simulate user navigating to a URL with ?zoom= (e.g. clicking
    // a "zoom into block" link from elsewhere in the app).
    act_history('/page/demo?zoom=block-xyz')

    await waitFor(() => {
      expect(mockPageViewProps.current.zoomBlockId).toBe('block-xyz')
    })
  })

  it('updates zoomBlockId back to null when popstate removes the param', async () => {
    window.history.replaceState({}, '', '/page/demo?zoom=block-abc')

    render(<PageViewPage />)

    await waitFor(() => {
      expect(mockPageViewProps.current).not.toBeNull()
    })

    expect(mockPageViewProps.current.zoomBlockId).toBe('block-abc')

    // Simulate the user pressing back → URL drops the param.
    act_history('/page/demo')

    await waitFor(() => {
      expect(mockPageViewProps.current.zoomBlockId).toBeNull()
    })
  })

  it('returns URL-decoded zoom value (consistent with URLSearchParams.get)', async () => {
    // The useUrlParam hook follows URLSearchParams.get() semantics
    // — percent-encoded values are decoded. The page wrapper
    // forwards the decoded value as-is to PageView.
    window.history.replaceState({}, '', '/page/demo?zoom=hello%20world')

    render(<PageViewPage />)

    await waitFor(() => {
      expect(mockPageViewProps.current).not.toBeNull()
    })

    expect(mockPageViewProps.current.zoomBlockId).toBe('hello world')
  })
})

// ── Helpers ────────────────────────────────────────────────────

/**
 * Simulate a URL change + popstate event. jsdom does not
 * automatically dispatch popstate on `pushState` / `replaceState`,
 * so we do it manually.
 */
function act_history(url: string) {
  window.history.pushState({}, '', url)
  window.dispatchEvent(new PopStateEvent('popstate'))
}
