import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { UnlinkedRefQueue } from '../UnlinkedRefQueue'
import type { UnlinkedCandidate } from '../unlinkedRefQueue'

const mockLink = vi.fn()
const mockDismiss = vi.fn()

function makeCandidate(overrides: Partial<UnlinkedCandidate> = {}): UnlinkedCandidate {
  return {
    blockId: 'b-1',
    pageName: 'Demo Page',
    mentionText: 'Demo Page',
    position: 4,
    createdAt: 1700000000000,
    ...overrides,
  }
}

beforeEach(() => {
  mockLink.mockReset()
  mockDismiss.mockReset()
  mockLink.mockResolvedValue(undefined)
})

describe('UnlinkedRefQueue — UI', () => {
  it('renders nothing when pageName is null', () => {
    const { container } = render(
      <UnlinkedRefQueue
        pageName={null}
        queue={[]}
        onLink={mockLink}
        onDismiss={mockDismiss}
      />,
    )
    expect(container.firstChild).toBeNull()
  })

  it('shows the count badge with N candidates when collapsed', () => {
    render(
      <UnlinkedRefQueue
        pageName="Demo Page"
        queue={[makeCandidate(), makeCandidate({ blockId: 'b-2' })]}
        onLink={mockLink}
        onDismiss={mockDismiss}
      />,
    )
    expect(screen.getByTestId('unlinked-ref-queue-header')).toBeInTheDocument()
    expect(screen.getByTestId('unlinked-ref-queue-count')).toHaveTextContent('2')
  })

  it('is collapsed by default — list not visible', () => {
    render(
      <UnlinkedRefQueue
        pageName="Demo Page"
        queue={[makeCandidate()]}
        onLink={mockLink}
        onDismiss={mockDismiss}
      />,
    )
    expect(screen.queryByTestId('unlinked-ref-queue-content')).not.toBeInTheDocument()
    expect(screen.getByTestId('unlinked-ref-queue-header')).toHaveAttribute('aria-expanded', 'false')
  })

  it('expands to show candidates when the header is clicked', async () => {
    const user = userEvent.setup()
    render(
      <UnlinkedRefQueue
        pageName="Demo Page"
        queue={[makeCandidate()]}
        onLink={mockLink}
        onDismiss={mockDismiss}
      />,
    )
    await user.click(screen.getByTestId('unlinked-ref-queue-header'))
    expect(screen.getByTestId('unlinked-ref-queue-content')).toBeInTheDocument()
    expect(screen.getByTestId('unlinked-ref-queue-item')).toBeInTheDocument()
    expect(screen.getByTestId('unlinked-ref-queue-link')).toBeInTheDocument()
    expect(screen.getByTestId('unlinked-ref-queue-dismiss')).toBeInTheDocument()
  })

  it('shows the empty state when the queue is empty and expanded', async () => {
    const user = userEvent.setup()
    render(
      <UnlinkedRefQueue
        pageName="Demo Page"
        queue={[]}
        onLink={mockLink}
        onDismiss={mockDismiss}
      />,
    )
    await user.click(screen.getByTestId('unlinked-ref-queue-header'))
    expect(screen.getByTestId('unlinked-ref-queue-empty')).toBeInTheDocument()
  })

  it('shows a "scanning" message while loading and no candidates are known yet', async () => {
    const user = userEvent.setup()
    render(
      <UnlinkedRefQueue
        pageName="Demo Page"
        queue={[]}
        loading={true}
        onLink={mockLink}
        onDismiss={mockDismiss}
      />,
    )
    await user.click(screen.getByTestId('unlinked-ref-queue-header'))
    expect(screen.getByText(/Scanning for unlinked mentions/i)).toBeInTheDocument()
  })

  it('"Link" calls onLink with the right candidate', async () => {
    const user = userEvent.setup()
    const candidate = makeCandidate({ blockId: 'b-7', position: 42 })
    render(
      <UnlinkedRefQueue
        pageName="Demo Page"
        queue={[candidate]}
        onLink={mockLink}
        onDismiss={mockDismiss}
        defaultExpanded={true}
      />,
    )

    await user.click(screen.getByTestId('unlinked-ref-queue-link'))

    await waitFor(() => expect(mockLink).toHaveBeenCalledWith(candidate))
  })

  it('"Dismiss" calls onDismiss with the right candidate', async () => {
    const user = userEvent.setup()
    const candidate = makeCandidate({ blockId: 'b-9' })
    render(
      <UnlinkedRefQueue
        pageName="Demo Page"
        queue={[candidate]}
        onLink={mockLink}
        onDismiss={mockDismiss}
        defaultExpanded={true}
      />,
    )

    await user.click(screen.getByTestId('unlinked-ref-queue-dismiss'))

    expect(mockDismiss).toHaveBeenCalledWith(candidate)
  })

  it('highlights the mention in the preview when block content is provided', () => {
    // "Demo Page" starts at index 13 of "read more on Demo Page today"
    //   index: 0123456789012345678901234567
    //          read more on Demo Page today
    //                   ^^^^^^^^^^^^^^^^
    //                          13
    const candidate = makeCandidate({ position: 13 })
    const content = 'read more on Demo Page today'

    render(
      <UnlinkedRefQueue
        pageName="Demo Page"
        queue={[candidate]}
        onLink={mockLink}
        onDismiss={mockDismiss}
        defaultExpanded={true}
        blockContentResolver={() => content}
      />,
    )

    const item = screen.getByTestId('unlinked-ref-queue-item')
    expect(item).toBeInTheDocument()
    const mark = item.querySelector('mark')
    expect(mark).toBeInTheDocument()
    expect(mark?.textContent).toBe('Demo Page')
  })

  it('shows a fallback preview when no block content is available', () => {
    render(
      <UnlinkedRefQueue
        pageName="Demo Page"
        queue={[makeCandidate()]}
        onLink={mockLink}
        onDismiss={mockDismiss}
        defaultExpanded={true}
      />,
    )
    const item = screen.getByTestId('unlinked-ref-queue-item')
    expect(item.querySelector('mark')).toBeNull()
    expect(item.textContent).toContain('Demo Page')
  })
})
