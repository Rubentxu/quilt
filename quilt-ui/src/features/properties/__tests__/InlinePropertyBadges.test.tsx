import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { InlinePropertyBadges } from '../InlinePropertyBadges'
import type { Block } from '@shared/types/api'

// Mock the api-client so we can observe the save call.
const mockSetBlockProperty = vi.fn()

vi.mock('@core/api-client', () => ({
  api: {
    setBlockProperty: (...args: any[]) => mockSetBlockProperty(...args),
  },
}))

function makeBlock(
  properties: Array<{ key: string; value: string | number | boolean; type?: 'string' | 'select' | 'date' | 'boolean' }>,
  blockType: Block['blockType'] = 'paragraph',
): Block {
  return {
    id: 'b1',
    pageId: 'p1',
    pageName: 'demo',
    content: 'hello world',
    blockType,
    marker: null,
    priority: null,
    parentId: null,
    order: 1,
    level: 0,
    collapsed: false,
    createdAt: '2026-06-01T00:00:00Z',
    updatedAt: '2026-06-01T00:00:00Z',
    properties: properties.map(p => ({
      key: p.key,
      value: p.value,
      type: p.type ?? 'string',
    })),
  } as Block
}

describe('InlinePropertyBadges', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockSetBlockProperty.mockResolvedValue(undefined)
  })

  afterEach(() => {
    cleanup()
  })

  it('renders nothing when the block has no inline properties (default template)', () => {
    const { container } = render(<InlinePropertyBadges block={makeBlock([{ key: 'foo', value: 'bar' }])} onUpdate={vi.fn()} />)
    expect(container.firstChild).toBeNull()
  })

  it('renders badges for the inline keys defined by the template (todo block)', () => {
    const block = makeBlock(
      [
        { key: 'status', value: 'open', type: 'select' },
        { key: 'priority', value: 'A', type: 'select' },
        { key: 'notes', value: 'free-form' },
      ],
      'todo',
    )
    render(<InlinePropertyBadges block={block} onUpdate={vi.fn()} />)
    // status and priority should be visible as badges
    expect(screen.getByTestId('inline-badge-status')).toBeInTheDocument()
    expect(screen.getByTestId('inline-badge-priority')).toBeInTheDocument()
    // `notes` is not in the inline template, so it should NOT appear here
    expect(screen.queryByTestId('inline-badge-notes')).not.toBeInTheDocument()
  })

  it('opens an inline editor when a badge is clicked', async () => {
    const block = makeBlock(
      [{ key: 'status', value: 'open', type: 'select' }],
      'todo',
    )
    render(<InlinePropertyBadges block={block} onUpdate={vi.fn()} />)
    const badge = screen.getByTestId('inline-badge-status')
    fireEvent.click(badge)
    // An input/editor appears
    const editor = await screen.findByTestId('inline-editor-status')
    expect(editor).toBeInTheDocument()
  })

  it('calls api.setBlockProperty on Enter with the new value', async () => {
    const onUpdate = vi.fn()
    const block = makeBlock(
      [{ key: 'status', value: 'open', type: 'select' }],
      'todo',
    )
    render(<InlinePropertyBadges block={block} onUpdate={onUpdate} />)

    fireEvent.click(screen.getByTestId('inline-badge-status'))
    const editor = await screen.findByTestId('inline-editor-status')
    fireEvent.change(editor, { target: { value: 'closed' } })
    fireEvent.keyDown(editor, { key: 'Enter' })

    await waitFor(() => {
      expect(mockSetBlockProperty).toHaveBeenCalledWith('b1', 'status', 'closed')
    })
  })

  it('calls api.setBlockProperty on blur with the new value', async () => {
    const onUpdate = vi.fn()
    const block = makeBlock(
      [{ key: 'status', value: 'open', type: 'select' }],
      'todo',
    )
    render(<InlinePropertyBadges block={block} onUpdate={onUpdate} />)

    fireEvent.click(screen.getByTestId('inline-badge-status'))
    const editor = await screen.findByTestId('inline-editor-status')
    fireEvent.change(editor, { target: { value: 'in-progress' } })
    fireEvent.blur(editor)

    await waitFor(() => {
      expect(mockSetBlockProperty).toHaveBeenCalledWith('b1', 'status', 'in-progress')
    })
  })

  it('cancels the edit on Escape and does NOT call setBlockProperty', async () => {
    const block = makeBlock(
      [{ key: 'status', value: 'open', type: 'select' }],
      'todo',
    )
    render(<InlinePropertyBadges block={block} onUpdate={vi.fn()} />)

    fireEvent.click(screen.getByTestId('inline-badge-status'))
    const editor = await screen.findByTestId('inline-editor-status')
    fireEvent.change(editor, { target: { value: 'should-not-save' } })
    fireEvent.keyDown(editor, { key: 'Escape' })

    // Wait a tick so any pending save would fire
    await new Promise(r => setTimeout(r, 50))
    expect(mockSetBlockProperty).not.toHaveBeenCalled()
  })
})
