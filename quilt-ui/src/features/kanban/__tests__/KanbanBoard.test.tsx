import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { KanbanBoard } from '../KanbanBoard'
import type { Block } from '@shared/types/api'

// Mock react-virtuoso to avoid DOM issues in tests
vi.mock('react-virtuoso', () => ({
  TableVirtuoso: vi.fn(({ data, itemContent }) => (
    <div data-testid="virtuoso-mock">
      {data.map((item: unknown, index: number) =>
        itemContent(index, item)
      )}
    </div>
  )),
}))

const mockBlocks: Block[] = [
  {
    id: 'block-1',
    pageId: 'page-1',
    pageName: 'Test Page',
    content: 'Card 1',
    blockType: 'paragraph',
    marker: null,
    priority: null,
    parentId: null,
    order: 1,
    level: 0,
    collapsed: false,
    properties: [
      { key: 'status', value: 'todo', type: 'string' },
    ],
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-01T00:00:00Z',
  },
  {
    id: 'block-2',
    pageId: 'page-1',
    pageName: 'Test Page',
    content: 'Card 2',
    blockType: 'paragraph',
    marker: null,
    priority: null,
    parentId: null,
    order: 2,
    level: 0,
    collapsed: false,
    properties: [
      { key: 'status', value: 'done', type: 'string' },
    ],
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-01T00:00:00Z',
  },
  {
    id: 'block-3',
    pageId: 'page-1',
    pageName: 'Test Page',
    content: 'Card 3',
    blockType: 'paragraph',
    marker: null,
    priority: null,
    parentId: null,
    order: 3,
    level: 0,
    collapsed: false,
    properties: [
      { key: 'status', value: 'todo', type: 'string' },
    ],
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-01T00:00:00Z',
  },
]

describe('KanbanBoard', () => {
  const mockOnPropertyChange = vi.fn()

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('4.1 [TEST] groups blocks by property value', () => {
    render(
      <KanbanBoard
        propertyKey="status"
        blocks={mockBlocks}
        onPropertyChange={mockOnPropertyChange}
      />
    )

    // Should have 2 columns: "done" and "todo"
    const columns = screen.getAllByTestId(/kanban-column/)
    expect(columns).toHaveLength(2)
  })

  it('4.1 [TEST] maps correct blocks to each column', () => {
    render(
      <KanbanBoard
        propertyKey="status"
        blocks={mockBlocks}
        onPropertyChange={mockOnPropertyChange}
      />
    )

    const columns = screen.getAllByTestId(/kanban-column/)
    const columnContents = columns.map(col => col.textContent)

    // "done" column should have Card 2
    const doneColumn = columnContents.find(c => c?.includes('Card 2'))
    expect(doneColumn).toBeDefined()

    // "todo" column should have Card 1 and Card 3
    const todoColumn = columnContents.find(c => c?.includes('Card 1') && c?.includes('Card 3'))
    expect(todoColumn).toBeDefined()
  })

  it('4.1 [TEST] shows blocks without the property in an "uncategorized" column', () => {
    const blocksWithoutProperty: Block[] = [
      {
        ...mockBlocks[0],
        id: 'block-no-prop',
        properties: [],
      },
    ]

    render(
      <KanbanBoard
        propertyKey="status"
        blocks={blocksWithoutProperty}
        onPropertyChange={mockOnPropertyChange}
      />
    )

    // Should show an uncategorized column
    const uncategorized = screen.getByTestId('kanban-column-uncategorized')
    expect(uncategorized).toBeInTheDocument()
  })
})
