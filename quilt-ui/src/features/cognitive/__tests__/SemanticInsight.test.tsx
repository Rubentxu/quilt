// ─── SemanticInsight.test.tsx ──────────────────────────────────────
//
// Tests for the `SemanticInsight` cognitive panel.
//
// Contract (per `docs/adr/drafts/DRAFT-cognitive-panel-family-namespace.md`):
//   - PASSIVE view: it lists blocks tagged with `type:: insight` from
//     the current page. Quilt does NOT generate insights; the external
//     agent writes them and Quilt surfaces them.
//   - Returns `null` when the panel is closed or no page is selected.
//   - Skips the API call when closed.
//   - Empty state when the page has no insight blocks.

import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { SemanticInsight } from '../SemanticInsight'

const mockGetPageBlocks = vi.fn()
const mockNavigate = vi.fn()

vi.mock('@core/api-client', () => ({
  api: {
    getPageBlocks: (name: string) => mockGetPageBlocks(name),
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

describe('SemanticInsight — rendering', () => {
  beforeEach(() => {
    mockGetPageBlocks.mockReset()
    mockNavigate.mockReset()
  })

  it('returns null when isOpen is false', () => {
    const { container } = render(
      <SemanticInsight pageName="demo" isOpen={false} />,
    )
    expect(container.firstChild).toBeNull()
  })

  it('returns null when pageName is null', () => {
    const { container } = render(
      <SemanticInsight pageName={null} isOpen={true} />,
    )
    expect(container.firstChild).toBeNull()
  })

  it('does not fetch when isOpen is false', () => {
    render(<SemanticInsight pageName="demo" isOpen={false} />)
    expect(mockGetPageBlocks).not.toHaveBeenCalled()
  })

  it('renders a header with the panel title', async () => {
    mockGetPageBlocks.mockResolvedValue([])

    render(<SemanticInsight pageName="demo" isOpen={true} />)

    expect(screen.getByTestId('semantic-insight-header')).toBeInTheDocument()
    expect(screen.getByText(/Semantic Insight/i)).toBeInTheDocument()
  })
})

describe('SemanticInsight — filtering by type::insight', () => {
  beforeEach(() => {
    mockGetPageBlocks.mockReset()
  })

  it('lists only blocks with type::insight property', async () => {
    mockGetPageBlocks.mockResolvedValue([
      makeBlock({
        id: 'b1',
        content: 'Authorship matters more than perfection',
        properties: [{ key: 'type', value: 'insight', type: 'string' }],
      }),
      makeBlock({
        id: 'b2',
        content: 'A regular block — not an insight',
      }),
      makeBlock({
        id: 'b3',
        content: 'A second insight, written by an agent',
        properties: [{ key: 'type', value: 'insight', type: 'string' }],
      }),
    ])

    render(<SemanticInsight pageName="demo" isOpen={true} />)

    await waitFor(() =>
      expect(screen.getByTestId('semantic-insight-item-b1')).toBeInTheDocument(),
    )
    expect(screen.getByTestId('semantic-insight-item-b3')).toBeInTheDocument()
    // Regular block is NOT rendered.
    expect(screen.queryByTestId('semantic-insight-item-b2')).not.toBeInTheDocument()
  })

  it('shows an empty state when the page has no insight blocks', async () => {
    mockGetPageBlocks.mockResolvedValue([
      makeBlock({ id: 'b1', content: 'Just a regular block' }),
      makeBlock({ id: 'b2', content: 'Another regular block' }),
    ])

    render(<SemanticInsight pageName="demo" isOpen={true} />)

    await waitFor(() =>
      expect(screen.getByTestId('semantic-insight-empty')).toBeInTheDocument(),
    )
    expect(screen.getByText(/No insights on this page/i)).toBeInTheDocument()
  })

  it('shows the agent author when the block has a created_by property', async () => {
    mockGetPageBlocks.mockResolvedValue([
      makeBlock({
        id: 'b1',
        content: 'An insight with provenance',
        properties: [
          { key: 'type', value: 'insight', type: 'string' },
          { key: 'created_by', value: 'agent::claude', type: 'string' },
        ],
      }),
    ])

    render(<SemanticInsight pageName="demo" isOpen={true} />)

    await waitFor(() =>
      expect(screen.getByTestId('semantic-insight-item-b1')).toBeInTheDocument(),
    )
    expect(screen.getByText('agent::claude')).toBeInTheDocument()
  })
})

describe('SemanticInsight — refresh', () => {
  beforeEach(() => {
    mockGetPageBlocks.mockReset()
  })

  it('re-fetches when the refresh button is clicked', async () => {
    mockGetPageBlocks.mockResolvedValue([])

    render(<SemanticInsight pageName="demo" isOpen={true} />)

    await waitFor(() => expect(mockGetPageBlocks).toHaveBeenCalledTimes(1))

    const user = userEvent.setup()
    await user.click(screen.getByTestId('semantic-insight-refresh'))

    await waitFor(() => expect(mockGetPageBlocks).toHaveBeenCalledTimes(2))
  })
})
