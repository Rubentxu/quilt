// ─── StructuralGraph.test.tsx ───────────────────────────────────────
//
// Tests for the `StructuralGraph` cognitive panel.
//
// Contract (per `docs/adr/drafts/DRAFT-cognitive-panel-family-namespace.md`):
//   - Renders block count, property count, reference count, and the
//     most-used property keys for the current page.
//   - Detects orphan blocks (blocks with no incoming refs and no
//     outgoing [[wikilinks]]) and surfaces them in an "Orphans" row.
//   - Returns `null` when no page is selected (no `pageName`).
//   - Skips the API call when the panel is closed (`isOpen=false`).
//
// All data is derived client-side from `api.getPageBlocks` and
// `api.getPageBacklinks` — no new backend route. The structural
// mirror in `quilt-analysis` is NOT yet mounted as an HTTP route
// (see the `quilt-cognitive` follow-up), so we mirror the trivial
// stats from the two endpoints that ARE available.

import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { StructuralGraph } from '../StructuralGraph'

const mockGetPageBlocks = vi.fn()
const mockGetPageBacklinks = vi.fn()
const mockNavigate = vi.fn()

vi.mock('@core/api-client', () => ({
  api: {
    getPageBlocks: (name: string) => mockGetPageBlocks(name),
    getPageBacklinks: (name: string) => mockGetPageBacklinks(name),
  },
}))

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}))

interface FakeBlockArgs {
  id: string
  pageName?: string | null
  content?: string
  properties?: Array<{ key: string; value: string | number | boolean | null; type: 'string' | 'number' | 'boolean' | 'date' | 'select' | 'page_ref' }>
}

function makeBlock({ id, pageName = 'demo', content = 'x', properties = [] }: FakeBlockArgs) {
  return {
    id,
    pageId: 'page-1',
    pageName,
    content,
    blockType: 'paragraph' as const,
    marker: null,
    priority: null,
    parentId: null,
    order: 0,
    level: 0,
    collapsed: false,
    properties,
    createdAt: '2026-06-09T10:00:00Z',
    updatedAt: '2026-06-09T10:00:00Z',
  }
}

describe('StructuralGraph — rendering', () => {
  beforeEach(() => {
    mockGetPageBlocks.mockReset()
    mockGetPageBacklinks.mockReset()
    mockNavigate.mockReset()
  })

  it('returns null when isOpen is false', () => {
    const { container } = render(
      <StructuralGraph pageName="demo" isOpen={false} />,
    )
    expect(container.firstChild).toBeNull()
  })

  it('returns null when pageName is null (no page to analyze)', () => {
    const { container } = render(
      <StructuralGraph pageName={null} isOpen={true} />,
    )
    expect(container.firstChild).toBeNull()
  })

  it('does not fetch when isOpen is false', () => {
    render(<StructuralGraph pageName="demo" isOpen={false} />)
    expect(mockGetPageBlocks).not.toHaveBeenCalled()
    expect(mockGetPageBacklinks).not.toHaveBeenCalled()
  })

  it('renders a header with the panel title', async () => {
    mockGetPageBlocks.mockResolvedValue([])
    mockGetPageBacklinks.mockResolvedValue([])

    render(<StructuralGraph pageName="demo" isOpen={true} />)

    expect(screen.getByTestId('structural-graph-header')).toBeInTheDocument()
    expect(
      screen.getByTestId('structural-graph-header').textContent,
    ).toMatch(/Structural Graph/i)
  })
})

describe('StructuralGraph — stats computation', () => {
  beforeEach(() => {
    mockGetPageBlocks.mockReset()
    mockGetPageBacklinks.mockReset()
  })

  it('shows block count, property count, and reference count for the current page', async () => {
    mockGetPageBlocks.mockResolvedValue([
      makeBlock({ id: 'b1', content: 'First', properties: [{ key: 'status', value: 'open', type: 'string' }] }),
      makeBlock({ id: 'b2', content: 'Second', properties: [{ key: 'status', value: 'closed', type: 'string' }, { key: 'priority', value: 'A', type: 'string' }] }),
      makeBlock({ id: 'b3', content: 'Third [[Other]]' }),
    ])
    mockGetPageBacklinks.mockResolvedValue([
      { sourceBlockId: 'src-1', sourcePageName: 'ref-a', contentPreview: 'A links here' },
      { sourceBlockId: 'src-2', sourcePageName: 'ref-b', contentPreview: 'B links here' },
      { sourceBlockId: 'src-3', sourcePageName: 'ref-c', contentPreview: 'C links here' },
    ])

    render(<StructuralGraph pageName="demo" isOpen={true} />)

    await waitFor(() =>
      expect(screen.getByTestId('structural-graph-block-count')).toHaveTextContent('3'),
    )
    expect(screen.getByTestId('structural-graph-property-count')).toHaveTextContent('3')
    expect(screen.getByTestId('structural-graph-reference-count')).toHaveTextContent('3')
  })

  it('lists the most-used property keys (descending by count)', async () => {
    mockGetPageBlocks.mockResolvedValue([
      makeBlock({ id: 'b1', properties: [{ key: 'status', value: 'a', type: 'string' }] }),
      makeBlock({ id: 'b2', properties: [{ key: 'status', value: 'b', type: 'string' }] }),
      makeBlock({ id: 'b3', properties: [{ key: 'status', value: 'c', type: 'string' }] }),
      makeBlock({ id: 'b4', properties: [{ key: 'priority', value: 'A', type: 'string' }] }),
      makeBlock({ id: 'b5', properties: [{ key: 'tag', value: 'idea', type: 'string' }] }),
    ])
    mockGetPageBacklinks.mockResolvedValue([])

    render(<StructuralGraph pageName="demo" isOpen={true} />)

    await waitFor(() =>
      expect(screen.getByTestId('structural-graph-property-list')).toBeInTheDocument(),
    )

    const list = screen.getByTestId('structural-graph-property-list')
    // `status` appears on 3 blocks; should be first.
    // `priority` and `tag` each appear once — order between them
    // is stable but not asserted.
    expect(list.textContent?.indexOf('status')).toBeLessThan(
      list.textContent?.indexOf('priority') ?? Number.POSITIVE_INFINITY,
    )
    expect(list.textContent?.indexOf('status')).toBeLessThan(
      list.textContent?.indexOf('tag') ?? Number.POSITIVE_INFINITY,
    )
  })
})

describe('StructuralGraph — orphan detection', () => {
  beforeEach(() => {
    mockGetPageBlocks.mockReset()
    mockGetPageBacklinks.mockReset()
  })

  it('flags blocks with no incoming backlinks AND no outgoing [[wikilinks]] as orphans', async () => {
    // Zero incoming backlinks at the page level — so a block with
    // no outgoing `[[wikilinks]]` is an orphan.
    mockGetPageBlocks.mockResolvedValue([
      makeBlock({ id: 'b1', content: 'Linked block [[Other Page]]' }),
      makeBlock({ id: 'b2', content: 'Orphan — no links, no refs' }),
      makeBlock({ id: 'b3', content: 'Another orphan' }),
    ])
    mockGetPageBacklinks.mockResolvedValue([])

    render(<StructuralGraph pageName="demo" isOpen={true} />)

    await waitFor(() =>
      expect(screen.getByTestId('structural-graph-orphans')).toBeInTheDocument(),
    )

    const orphans = screen.getByTestId('structural-graph-orphans')
    // The block with an outgoing [[wikilink]] is NOT an orphan.
    expect(orphans.textContent).not.toContain('Linked block')
    // The two "orphan" blocks ARE flagged.
    expect(orphans.textContent).toContain('Orphan — no links, no refs')
    expect(orphans.textContent).toContain('Another orphan')
  })

  it('shows an empty-orphans state when every block has at least one connection', async () => {
    // Page has incoming backlinks — so per-block incoming refs
    // exist at the page level. Combined with at least one outgoing
    // link on each block, the page has no orphans.
    mockGetPageBlocks.mockResolvedValue([
      makeBlock({ id: 'b1', content: 'A [[B]]' }),
      makeBlock({ id: 'b2', content: 'B [[A]]' }),
    ])
    mockGetPageBacklinks.mockResolvedValue([
      { sourceBlockId: 'src-1', sourcePageName: 'ref-a', contentPreview: 'links in' },
    ])

    render(<StructuralGraph pageName="demo" isOpen={true} />)

    await waitFor(() =>
      expect(screen.getByTestId('structural-graph-orphans')).toBeInTheDocument(),
    )

    expect(screen.getByText(/No orphan blocks/i)).toBeInTheDocument()
  })
})

describe('StructuralGraph — refresh', () => {
  beforeEach(() => {
    mockGetPageBlocks.mockReset()
    mockGetPageBacklinks.mockReset()
  })

  it('re-fetches when the refresh button is clicked', async () => {
    mockGetPageBlocks.mockResolvedValue([])
    mockGetPageBacklinks.mockResolvedValue([])

    render(<StructuralGraph pageName="demo" isOpen={true} />)

    await waitFor(() => expect(mockGetPageBlocks).toHaveBeenCalledTimes(1))

    const user = userEvent.setup()
    await user.click(screen.getByTestId('structural-graph-refresh'))

    await waitFor(() => expect(mockGetPageBlocks).toHaveBeenCalledTimes(2))
  })
})
