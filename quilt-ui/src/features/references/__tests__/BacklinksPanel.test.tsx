import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { BacklinksPanel } from '../BacklinksPanel'

const mockGetPageBacklinks = vi.fn()
const mockListPages = vi.fn()
const mockSearchBlocks = vi.fn()
const mockUpdateBlock = vi.fn()
const mockNavigate = vi.fn()
const mockWriteText = vi.fn().mockResolvedValue(undefined)

vi.mock('@core/api-client', () => ({
  api: {
    getPageBacklinks: (name: string) => mockGetPageBacklinks(name),
    listPages: () => mockListPages(),
    searchBlocks: (q: string, limit?: number) => mockSearchBlocks(q, limit),
    updateBlock: (id: string, data: { content: string }) => mockUpdateBlock(id, data),
  },
}))

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}))

vi.mock('react-hot-toast', () => ({
  default: {
    success: vi.fn(),
    error: vi.fn(),
  },
}))

function makeBacklinks(count: number, sourcePrefix = 'source-page') {
  return Array.from({ length: count }, (_, i) => ({
    sourceBlockId: `block-${i}`,
    sourcePageName: `${sourcePrefix}-${i % 2}`,
    contentPreview: `Preview text for backlink ${i} — this is a longer preview to test clamping`,
  }))
}

describe('BacklinksPanel — G6: auto-shown on every page', () => {
  beforeEach(() => {
    mockGetPageBacklinks.mockReset()
    mockListPages.mockReset()
    mockSearchBlocks.mockReset()
    mockUpdateBlock.mockReset()
    mockNavigate.mockReset()
    mockWriteText.mockReset()
    // Default: empty graph so the unlinked-ref hook stays quiet
    // and resolves with no extra state updates that escape `act()`.
    mockListPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue([])
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText: mockWriteText },
    })
  })

  it('returns null when isOpen is false (sidebar hidden)', () => {
    const { container } = render(
      <BacklinksPanel pageName="demo" isOpen={false} />,
    )
    expect(container.firstChild).toBeNull()
  })

  it('renders the header with the count badge by default', async () => {
    mockGetPageBacklinks.mockResolvedValue(makeBacklinks(3))

    render(<BacklinksPanel pageName="demo" isOpen={true} />)

    const header = screen.getByTestId('backlinks-panel-header')
    expect(header).toBeInTheDocument()
    expect(screen.getByText('Linked References')).toBeInTheDocument()

    // Count starts at 0 and updates after fetch resolves
    expect(screen.getByTestId('backlinks-panel-count')).toHaveTextContent('0')

    await waitFor(() =>
      expect(screen.getByTestId('backlinks-panel-count')).toHaveTextContent('3'),
    )
  })

  it('is collapsed by default — content (filter, list) is not visible', async () => {
    mockGetPageBacklinks.mockResolvedValue(makeBacklinks(3))

    render(<BacklinksPanel pageName="demo" isOpen={true} />)

    // Wait for backlinks to load — the count badge would have updated
    await waitFor(() =>
      expect(screen.getByTestId('backlinks-panel-count')).toHaveTextContent('3'),
    )

    // But the content (filter input / list) must NOT be in the DOM yet
    expect(screen.queryByPlaceholderText('Filter references...')).not.toBeInTheDocument()
    expect(screen.queryByTestId('backlinks-panel-content')).not.toBeInTheDocument()
  })

  it('marks the header as aria-expanded="false" when collapsed', async () => {
    mockGetPageBacklinks.mockResolvedValue([])

    render(<BacklinksPanel pageName="demo" isOpen={true} />)

    const header = screen.getByTestId('backlinks-panel-header')
    expect(header).toHaveAttribute('aria-expanded', 'false')
    expect(header).toHaveAttribute('aria-controls', 'backlinks-panel-content')

    // Let the fetch complete so React commits the post-resolve state
    await waitFor(() =>
      expect(mockGetPageBacklinks).toHaveBeenCalledWith('demo'),
    )
  })

  it('expands the content when the header is clicked', async () => {
    const user = userEvent.setup()
    mockGetPageBacklinks.mockResolvedValue(makeBacklinks(2))

    render(<BacklinksPanel pageName="demo" isOpen={true} />)

    const header = screen.getByTestId('backlinks-panel-header')
    await user.click(header)

    // After click, the content is rendered
    expect(screen.getByTestId('backlinks-panel-content')).toBeInTheDocument()
    expect(screen.getByPlaceholderText('Filter references...')).toBeInTheDocument()
    expect(header).toHaveAttribute('aria-expanded', 'true')
  })

  it('collapses the content when the header is clicked a second time', async () => {
    const user = userEvent.setup()
    mockGetPageBacklinks.mockResolvedValue(makeBacklinks(2))

    render(<BacklinksPanel pageName="demo" isOpen={true} />)

    const header = screen.getByTestId('backlinks-panel-header')
    await user.click(header)
    expect(screen.getByTestId('backlinks-panel-content')).toBeInTheDocument()

    await user.click(header)
    expect(screen.queryByTestId('backlinks-panel-content')).not.toBeInTheDocument()
    expect(header).toHaveAttribute('aria-expanded', 'false')
  })

  it('does not fetch when pageName is null (no page to query)', () => {
    render(<BacklinksPanel pageName={null} isOpen={true} />)

    // Header still renders (so the user sees the panel exists)
    expect(screen.getByTestId('backlinks-panel-header')).toBeInTheDocument()
    // But no API call
    expect(mockGetPageBacklinks).not.toHaveBeenCalled()
  })

  it('respects defaultExpanded=true — content is visible on first render', async () => {
    mockGetPageBacklinks.mockResolvedValue(makeBacklinks(2))

    render(
      <BacklinksPanel pageName="demo" isOpen={true} defaultExpanded={true} />,
    )

    expect(screen.getByTestId('backlinks-panel-content')).toBeInTheDocument()
    expect(screen.getByTestId('backlinks-panel-header')).toHaveAttribute(
      'aria-expanded',
      'true',
    )

    // Wait for the load to finish so the filter input is in the DOM
    await waitFor(() =>
      expect(screen.getByPlaceholderText('Filter references...')).toBeInTheDocument(),
    )
  })

  it('resets expansion state when the page changes', async () => {
    const user = userEvent.setup()
    mockGetPageBacklinks.mockResolvedValue(makeBacklinks(2))

    const { rerender } = render(
      <BacklinksPanel pageName="page-a" isOpen={true} defaultExpanded={true} />,
    )
    expect(screen.getByTestId('backlinks-panel-content')).toBeInTheDocument()

    // Navigate to a different page
    rerender(<BacklinksPanel pageName="page-b" isOpen={true} />)

    // Expansion resets to defaultExpanded (false) on page change
    await waitFor(() =>
      expect(
        screen.queryByTestId('backlinks-panel-content'),
      ).not.toBeInTheDocument(),
    )
  })

  it('shows an empty state inside the content when there are no backlinks', async () => {
    const user = userEvent.setup()
    mockGetPageBacklinks.mockResolvedValue([])

    render(<BacklinksPanel pageName="demo" isOpen={true} />)

    await waitFor(() =>
      expect(screen.getByTestId('backlinks-panel-count')).toHaveTextContent('0'),
    )

    // Expand
    await user.click(screen.getByTestId('backlinks-panel-header'))

    expect(screen.getByText('No linked references')).toBeInTheDocument()
    expect(
      screen.getByText(/This page is not linked from other notes/i),
    ).toBeInTheDocument()
  })
})

describe('BacklinksPanel — Q029: unlinked ref queue integration', () => {
  beforeEach(() => {
    mockGetPageBacklinks.mockReset()
    mockListPages.mockReset()
    mockSearchBlocks.mockReset()
    mockUpdateBlock.mockReset()
    mockNavigate.mockReset()
    mockWriteText.mockReset()
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText: mockWriteText },
    })
  })

  it('does NOT render the queue section when pageName is null', () => {
    render(<BacklinksPanel pageName={null} isOpen={true} defaultExpanded={true} />)
    expect(screen.queryByTestId('unlinked-ref-queue')).not.toBeInTheDocument()
  })

  it('renders the queue section with count "0" when the workspace is empty', async () => {
    mockGetPageBacklinks.mockResolvedValue([])
    mockListPages.mockResolvedValue([])
    mockSearchBlocks.mockResolvedValue([])

    render(<BacklinksPanel pageName="demo" isOpen={true} defaultExpanded={true} />)

    await waitFor(() =>
      expect(screen.getByTestId('unlinked-ref-queue-count')).toHaveTextContent('0'),
    )
  })

  it('populates the queue with a candidate when the scan finds an unlinked mention', async () => {
    mockGetPageBacklinks.mockResolvedValue([])
    mockListPages.mockResolvedValue([
      { id: 'p-1', name: 'demo', title: null, journal: false, journalDay: null, createdAt: '' },
    ])
    mockSearchBlocks.mockResolvedValue([
      {
        blockId: 'b-99',
        pageId: 'p-other',
        pageName: 'Other',
        content: 'look at demo for the answer',
        snippet: '',
        score: 1,
      },
    ])

    render(<BacklinksPanel pageName="demo" isOpen={true} defaultExpanded={true} />)

    await waitFor(() =>
      expect(screen.getByTestId('unlinked-ref-queue-count')).toHaveTextContent('1'),
    )

    // Expand the queue section so we can find the row.
    await userEvent.setup().click(screen.getByTestId('unlinked-ref-queue-header'))

    expect(screen.getByTestId('unlinked-ref-queue-item')).toBeInTheDocument()
  })

  it('"Link" action PUTs the updated content to the block and removes the candidate', async () => {
    const user = userEvent.setup()
    mockGetPageBacklinks.mockResolvedValue([])
    mockListPages.mockResolvedValue([
      { id: 'p-1', name: 'demo', title: null, journal: false, journalDay: null, createdAt: '' },
    ])
    mockSearchBlocks.mockResolvedValue([
      {
        blockId: 'b-99',
        pageId: 'p-other',
        pageName: 'Other',
        content: 'look at demo for the answer',
        snippet: '',
        score: 1,
      },
    ])
    mockUpdateBlock.mockResolvedValue({} as any)

    render(<BacklinksPanel pageName="demo" isOpen={true} defaultExpanded={true} />)

    await waitFor(() =>
      expect(screen.getByTestId('unlinked-ref-queue-count')).toHaveTextContent('1'),
    )
    await user.click(screen.getByTestId('unlinked-ref-queue-header'))

    await user.click(screen.getByTestId('unlinked-ref-queue-link'))

    await waitFor(() => expect(mockUpdateBlock).toHaveBeenCalledWith('b-99', { content: 'look at [[demo]] for the answer' }))
    await waitFor(() =>
      expect(screen.getByTestId('unlinked-ref-queue-count')).toHaveTextContent('0'),
    )
  })

  it('"Dismiss" action removes the candidate without calling the API', async () => {
    const user = userEvent.setup()
    mockGetPageBacklinks.mockResolvedValue([])
    mockListPages.mockResolvedValue([
      { id: 'p-1', name: 'demo', title: null, journal: false, journalDay: null, createdAt: '' },
    ])
    mockSearchBlocks.mockResolvedValue([
      {
        blockId: 'b-99',
        pageId: 'p-other',
        pageName: 'Other',
        content: 'look at demo for the answer',
        snippet: '',
        score: 1,
      },
    ])

    render(<BacklinksPanel pageName="demo" isOpen={true} defaultExpanded={true} />)

    await waitFor(() =>
      expect(screen.getByTestId('unlinked-ref-queue-count')).toHaveTextContent('1'),
    )
    await user.click(screen.getByTestId('unlinked-ref-queue-header'))

    await user.click(screen.getByTestId('unlinked-ref-queue-dismiss'))

    await waitFor(() =>
      expect(screen.getByTestId('unlinked-ref-queue-count')).toHaveTextContent('0'),
    )
    expect(mockUpdateBlock).not.toHaveBeenCalled()
  })
})
