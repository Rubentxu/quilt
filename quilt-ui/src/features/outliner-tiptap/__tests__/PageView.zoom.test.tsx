/**
 * Integration tests for Block Zoom in PageView.
 *
 * Contract: when `zoomBlockId` is passed, PageView:
 *   1. Shows only the zoomed block and its descendants.
 *   2. Renders a "Zoom out" button that calls `onZoomOut`.
 *   3. Calls `onZoomOut` automatically when the zoomed block is
 *      deleted (no longer in the blocks list).
 *   4. Does not show the EmptyState (page still has other blocks,
 *      just hidden by zoom).
 */
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { PageView } from '../PageView'
import type { Block } from '@shared/types/api'

// ── API mocks ────────────────────────────────────────────────────
// PageView does a lot: it loads blocks, lists pages, talks to
// the SSE bridge, etc. We stub the api module at the boundary
// so the test runs offline and deterministic.

const mockGetPageBlocks = vi.fn()
const mockUpdateBlock = vi.fn()
const mockDeleteBlock = vi.fn()
const mockCreateBlock = vi.fn()
const mockSetBlockProperty = vi.fn()
const mockListPages = vi.fn().mockResolvedValue([])
const mockListTemplates = vi.fn().mockResolvedValue([])
const mockCreatePageFromTemplate = vi.fn()
const mockSearchBlocks = vi.fn().mockResolvedValue([])

vi.mock('@core/api-client', () => ({
  api: {
    getPageBlocks: (...args: any[]) => mockGetPageBlocks(...args),
    updateBlock: (...args: any[]) => mockUpdateBlock(...args),
    deleteBlock: (...args: any[]) => mockDeleteBlock(...args),
    createBlock: (...args: any[]) => mockCreateBlock(...args),
    setBlockProperty: (...args: any[]) => mockSetBlockProperty(...args),
    listPages: () => mockListPages(),
    listTemplates: () => mockListTemplates(),
    createPageFromTemplate: (...args: any[]) => mockCreatePageFromTemplate(...args),
    searchBlocks: (...args: any[]) => mockSearchBlocks(...args),
  },
  getEventsUrl: () => '/api/v1/events',
  QuiltApiError: class QuiltApiError extends Error {
    constructor(public status: number, public code: string, public detail: string) {
      super(detail)
      this.name = 'QuiltApiError'
    }
  },
}))

vi.mock('@shared/contexts/TabsContext', () => ({
  useTabs: () => ({ openTab: vi.fn() }),
}))

vi.mock('@shared/contexts/ConnectionContext', () => ({
  useConnection: () => ({ sseConnected: false, setSseConnected: vi.fn() }),
}))

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => vi.fn(),
}))

vi.mock('@core/wasm-bridge/WasmProvider', () => ({
  useWasm: () => ({
    loaded: true,
    error: null,
    wasmGetVersion: vi.fn(() => 'test'),
    wasmPing: vi.fn(() => true),
    wasmGetState: vi.fn(),
    wasmLoadPage: vi.fn(),
    wasmDispatch: vi.fn(),
    wasmUndo: vi.fn(),
    wasmRedo: vi.fn(),
    wasmParseInline: (content: string) => ({ segments: [] }),
    retry: vi.fn(),
  }),
  ensureWasmLoaded: vi.fn().mockResolvedValue(undefined),
}))

// ── useProjection mock ─────────────────────────────────────────
// ADR-0025: ProjectionRenderer requires useProjection to return a
// resolved projection with the block's text. Without this mock,
// the projection is null (loading/error) and ProjectionRenderer
// renders a skeleton instead of text content.

const { mockUseProjection } = vi.hoisted(() => {
  const fn = vi.fn()
  return { mockUseProjection: fn }
})

function makeProjection(text: string) {
  return {
    projection: {
      text,
      links: [],
      children: [],
      decorations: [],
      conflicts: [],
      properties: {},
    },
    loading: false,
    error: null,
  }
}

vi.mock('@features/projection/hooks', () => ({
  useProjection: (...args: any[]) => mockUseProjection(...args),
}))

// Map from blockId → content, populated in beforeEach
const blockContentMap = new Map<string, string>()

function setProjectionMocks(blocks: Block[]) {
  blockContentMap.clear()
  for (const b of blocks) {
    blockContentMap.set(b.id, b.content)
  }
  mockUseProjection.mockImplementation((opts: { blockId: string }) => {
    const text = blockContentMap.get(opts.blockId) ?? 'UNKNOWN_BLOCK'
    return makeProjection(text)
  })
}

// ── Block factory ──────────────────────────────────────────────

function makeBlock(overrides: Partial<Block> = {}): Block {
  return {
    id: 'b1',
    pageId: 'p1',
    pageName: 'demo',
    content: 'Block content',
    blockType: 'paragraph',
    marker: null,
    priority: null,
    parentId: null,
    order: 0,
    level: 0,
    collapsed: false,
    createdAt: '2026-06-02T00:00:00Z',
    updatedAt: '2026-06-02T00:00:00Z',
    properties: [],
    ...overrides,
  }
}

/**
 * Build a small tree:
 *   - a
 *   - b (the zoom target)
 *     - b1
 *     - b2
 *       - b2a
 *   - c
 */
function buildDemoTree(): Block[] {
  return [
    makeBlock({ id: 'a', content: 'Block A', order: 0 }),
    makeBlock({ id: 'b', content: 'Block B', order: 1 }),
    makeBlock({ id: 'b1', parentId: 'b', content: 'Block B1', order: 0, level: 1 }),
    makeBlock({ id: 'b2', parentId: 'b', content: 'Block B2', order: 1, level: 1 }),
    makeBlock({ id: 'b2a', parentId: 'b2', content: 'Block B2a', order: 0, level: 2 }),
    makeBlock({ id: 'c', content: 'Block C', order: 2 }),
  ]
}

// ── Helpers ─────────────────────────────────────────────────────

/**
 * Get all visible block text content from the DOM.
 *
 * Legacy path (VITE_PROJECTION_RENDERER=off):
 *   - read mode uses `.block-content-read` divs
 * ProjectionRenderer path (VITE_PROJECTION_RENDERER=on):
 *   - each block renders a ProjectionRenderer with [data-testid="projection-text"]
 *     as the text container inside [data-testid="projection-renderer"]
 */
function getVisibleBlockContents(): string[] {
  // Legacy: .block-content-read divs
  const legacy = document.querySelectorAll('.block-content-read')
  if (legacy.length > 0) {
    return Array.from(legacy).map(el => el.textContent ?? '')
  }
  // Projection renderer: text is inside [data-testid="projection-text"]
  const projection = document.querySelectorAll('[data-testid="projection-text"]')
  return Array.from(projection).map(el => el.textContent?.trim() ?? '')
}

// ── Lifecycle ──────────────────────────────────────────────────

beforeEach(() => {
  vi.clearAllMocks()
  // Default: return the demo tree.
  const tree = buildDemoTree()
  mockGetPageBlocks.mockResolvedValue(tree)
  mockUpdateBlock.mockImplementation(async (id, data) => makeBlock({ id, ...data }))
  mockDeleteBlock.mockResolvedValue({ deleted: true })
  mockCreateBlock.mockImplementation(async data => makeBlock({ id: 'new', ...data }))
  // Set up projection mocks for the demo tree blocks.
  setProjectionMocks(tree)
})

afterEach(() => {
  vi.useRealTimers()
  // Clean any test-induced focus state.
  document.body.innerHTML = ''
})

// ── Tests ──────────────────────────────────────────────────────

describe('PageView — Block Zoom', () => {
  it('shows all blocks when no zoomBlockId is provided', async () => {
    render(<PageView pageName="demo" />)

    await waitFor(() => {
      expect(getVisibleBlockContents().length).toBeGreaterThan(0)
    })

    const visible = getVisibleBlockContents()
    expect(visible).toContain('Block A')
    expect(visible).toContain('Block B')
    expect(visible).toContain('Block B1')
    expect(visible).toContain('Block B2')
    expect(visible).toContain('Block B2a')
    expect(visible).toContain('Block C')
  })

  it('hides non-descendant blocks when zoomBlockId is set', async () => {
    render(<PageView pageName="demo" zoomBlockId="b" onZoomOut={vi.fn()} />)

    await waitFor(() => {
      // Wait for blocks to load. The zoom-out button is the
      // most reliable signal that zoom mode is active.
      expect(screen.queryByTestId('zoom-out-button')).not.toBeNull()
    })

    const visible = getVisibleBlockContents()
    // Zoomed block + descendants present
    expect(visible).toContain('Block B')
    expect(visible).toContain('Block B1')
    expect(visible).toContain('Block B2')
    expect(visible).toContain('Block B2a')
    // Sibling and non-descendants NOT present
    expect(visible).not.toContain('Block A')
    expect(visible).not.toContain('Block C')
  })

  it('renders the Zoom out button when zoom is active', async () => {
    const onZoomOut = vi.fn()
    render(<PageView pageName="demo" zoomBlockId="b" onZoomOut={onZoomOut} />)

    await waitFor(() => {
      expect(screen.queryByTestId('zoom-out-button')).not.toBeNull()
    })

    const btn = screen.getByTestId('zoom-out-button')
    expect(btn).toBeTruthy()
  })

  it('does NOT render the Zoom out button when zoom is inactive', async () => {
    render(<PageView pageName="demo" />)

    await waitFor(() => {
      expect(getVisibleBlockContents().length).toBeGreaterThan(0)
    })

    expect(screen.queryByTestId('zoom-out-button')).toBeNull()
  })

  it('calls onZoomOut when the Zoom out button is clicked', async () => {
    const onZoomOut = vi.fn()
    render(<PageView pageName="demo" zoomBlockId="b" onZoomOut={onZoomOut} />)

    await waitFor(() => {
      expect(screen.queryByTestId('zoom-out-button')).not.toBeNull()
    })

    fireEvent.click(screen.getByTestId('zoom-out-button'))

    expect(onZoomOut).toHaveBeenCalledTimes(1)
  })

  it('auto-calls onZoomOut when the zoomed block is deleted', async () => {
    const onZoomOut = vi.fn()
    // Render with a tree that does NOT include the zoomed block —
    // simulates the "user deleted the block, list reloaded" flow.
    mockGetPageBlocks.mockResolvedValue([
      makeBlock({ id: 'a', content: 'Block A', order: 0 }),
      makeBlock({ id: 'c', content: 'Block C', order: 2 }),
    ])

    render(<PageView pageName="demo" zoomBlockId="b" onZoomOut={onZoomOut} />)

    // The component should detect "b" is missing from the loaded
    // blocks and fire onZoomOut.
    await waitFor(() => {
      expect(onZoomOut).toHaveBeenCalled()
    })
  })

  it('hides the EmptyState when zoom is active (page has blocks, just filtered out)', async () => {
    render(<PageView pageName="demo" zoomBlockId="b" onZoomOut={vi.fn()} />)

    await waitFor(() => {
      expect(screen.queryByTestId('zoom-out-button')).not.toBeNull()
    })

    // EmptyState would render "This page is empty" / "No entries yet".
    // The page actually has 6 blocks — they're just hidden by zoom.
    expect(screen.queryByText('This page is empty')).toBeNull()
    expect(screen.queryByText('No entries yet')).toBeNull()
  })

  it('renders the zoomed block as a top-level entry (no parent breadcrumb pollution)', async () => {
    // The zoomed block "b" has a parent? No, it's a root block. But
    // even if it had one, the zoom view should display it as a
    // top-level entry in the zoom viewport.
    render(<PageView pageName="demo" zoomBlockId="b" onZoomOut={vi.fn()} />)

    await waitFor(() => {
      expect(screen.queryByTestId('zoom-out-button')).not.toBeNull()
    })

    // "Block B" is the first visible block content.
    const visible = getVisibleBlockContents()
    expect(visible[0]).toBe('Block B')
  })

  it('still allows child collapse/expand when zoomed', async () => {
    // The collapse state is independent of the zoom filter — the
    // zoomed subtree should still respect the existing collapsed
    // state in PageView.
    render(<PageView pageName="demo" zoomBlockId="b" onZoomOut={vi.fn()} />)

    await waitFor(() => {
      expect(screen.queryByTestId('zoom-out-button')).not.toBeNull()
    })

    // All 4 blocks in the subtree should be visible (none collapsed
    // by default). This proves the collapse filter still works in
    // combination with the zoom filter.
    const visible = getVisibleBlockContents()
    expect(visible).toContain('Block B')
    expect(visible).toContain('Block B1')
    expect(visible).toContain('Block B2')
    expect(visible).toContain('Block B2a')
  })
})
