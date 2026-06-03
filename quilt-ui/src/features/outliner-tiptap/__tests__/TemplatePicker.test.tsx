import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { TemplatePicker } from '../TemplatePicker'

// Mock the api-client
const mockListTemplates = vi.fn()
vi.mock('@core/api-client', () => ({
  api: {
    listTemplates: (...args: unknown[]) => mockListTemplates(...args),
  },
}))

beforeEach(() => {
  mockListTemplates.mockReset()
})

describe('TemplatePicker (ADR-0007)', () => {
  it('renders the fallback "Add first block" button when no templates exist', async () => {
    mockListTemplates.mockResolvedValue([])
    const onCreateBlock = vi.fn()
    render(<TemplatePicker onCreateBlock={onCreateBlock} />)

    // Wait for the load to finish
    await waitFor(() => {
      expect(screen.getByTestId('template-picker-empty')).toBeInTheDocument()
    })
    expect(screen.getByText('Add first block')).toBeInTheDocument()
  })

  it('renders a card for each template returned by the API', async () => {
    mockListTemplates.mockResolvedValue([
      { name: 'reference', full_name: 'template/reference', block_count: 1, card_shape: 'reference', icon: '🔗', cssclass: null },
      { name: 'documentation', full_name: 'template/documentation', block_count: 3, card_shape: 'content', icon: '📄', cssclass: null },
    ])
    const onCreateBlock = vi.fn()
    render(<TemplatePicker onCreateBlock={onCreateBlock} />)

    await waitFor(() => {
      expect(screen.getByTestId('template-card-reference')).toBeInTheDocument()
      expect(screen.getByTestId('template-card-documentation')).toBeInTheDocument()
    })

    expect(screen.getByText('reference')).toBeInTheDocument()
    expect(screen.getByText('documentation')).toBeInTheDocument()
    expect(screen.getByText('Reference · 1 block')).toBeInTheDocument()
    expect(screen.getByText('Documentation · 3 blocks')).toBeInTheDocument()
  })

  it('filters templates by the search input', async () => {
    mockListTemplates.mockResolvedValue([
      { name: 'reference', full_name: 'template/reference', block_count: 1, card_shape: 'reference', icon: '🔗', cssclass: null },
      { name: 'documentation', full_name: 'template/documentation', block_count: 1, card_shape: 'content', icon: '📄', cssclass: null },
      { name: 'meeting-notes', full_name: 'template/meeting-notes', block_count: 5, card_shape: 'reference', icon: '📋', cssclass: null },
    ])
    render(<TemplatePicker onCreateBlock={vi.fn()} />)

    await waitFor(() => {
      expect(screen.getByTestId('template-card-reference')).toBeInTheDocument()
    })

    const search = screen.getByTestId('template-picker-search')
    fireEvent.change(search, { target: { value: 'meet' } })

    expect(screen.queryByTestId('template-card-reference')).not.toBeInTheDocument()
    expect(screen.queryByTestId('template-card-documentation')).not.toBeInTheDocument()
    expect(screen.getByTestId('template-card-meeting-notes')).toBeInTheDocument()
  })

  it('calls onCreateBlock with the template name and a derived title when confirmed', async () => {
    mockListTemplates.mockResolvedValue([
      { name: 'meeting-notes', full_name: 'template/meeting-notes', block_count: 2, card_shape: 'reference', icon: '📋', cssclass: null },
    ])
    const onCreateBlock = vi.fn()
    render(<TemplatePicker onCreateBlock={onCreateBlock} />)

    await waitFor(() => {
      expect(screen.getByTestId('template-card-meeting-notes')).toBeInTheDocument()
    })

    // Select the card
    fireEvent.click(screen.getByTestId('template-card-meeting-notes'))

    // Confirm
    fireEvent.click(screen.getByTestId('template-picker-confirm'))

    expect(onCreateBlock).toHaveBeenCalledWith('meeting-notes', 'Meeting Notes')
  })

  it('disables the confirm button when no template is selected', async () => {
    mockListTemplates.mockResolvedValue([
      { name: 'reference', full_name: 'template/reference', block_count: 1, card_shape: 'reference', icon: '🔗', cssclass: null },
    ])
    render(<TemplatePicker onCreateBlock={vi.fn()} />)

    await waitFor(() => {
      expect(screen.getByTestId('template-card-reference')).toBeInTheDocument()
    })

    const confirm = screen.getByTestId('template-picker-confirm')
    expect(confirm).toBeDisabled()
  })

  it('shows the search empty state when no templates match the filter', async () => {
    mockListTemplates.mockResolvedValue([
      { name: 'reference', full_name: 'template/reference', block_count: 1, card_shape: 'reference', icon: '🔗', cssclass: null },
    ])
    render(<TemplatePicker onCreateBlock={vi.fn()} />)

    await waitFor(() => {
      expect(screen.getByTestId('template-card-reference')).toBeInTheDocument()
    })

    fireEvent.change(screen.getByTestId('template-picker-search'), { target: { value: 'xyz-nope' } })

    expect(screen.queryByTestId('template-card-reference')).not.toBeInTheDocument()
    expect(screen.getByText('No templates match "xyz-nope"')).toBeInTheDocument()
  })

  it('falls back to a plain block when the API fails and there are no templates', async () => {
    // Suppress expected console.warn
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    mockListTemplates.mockRejectedValue(new Error('network down'))
    const onCreateBlock = vi.fn()
    render(<TemplatePicker onCreateBlock={onCreateBlock} />)

    await waitFor(() => {
      expect(screen.getByTestId('template-picker-empty')).toBeInTheDocument()
    })
    expect(screen.getByText('Add first block')).toBeInTheDocument()
    warn.mockRestore()
  })
})
