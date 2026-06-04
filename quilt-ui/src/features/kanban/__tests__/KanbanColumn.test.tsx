import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { KanbanColumn } from '../KanbanColumn'
import type { Block } from '@shared/types/api'

// Mock react-virtuoso
vi.mock('react-virtuoso', () => ({
  TableVirtuoso: vi.fn(({ data, itemContent }) => (
    <div data-testid="virtuoso-mock">
      {data.map((item: unknown, index: number) =>
        itemContent(index, item)
      )}
    </div>
  )),
}))

// Mock @dnd-kit
vi.mock('@dnd-kit/core', () => ({
  DndContext: vi.fn(({ children }) => <div>{children}</div>),
  useDndContext: () => ({}),
  closestCenter: vi.fn(),
  PointerSensor: vi.fn(),
  useSensor: vi.fn(),
}))

vi.mock('@dnd-kit/sortable', () => ({
  useSortable: vi.fn(() => ({
    attributes: {},
    listeners: {},
    setNodeRef: vi.fn(),
    setActivatorNodeRef: vi.fn(),
    transform: null,
    transition: null,
    isDragging: false,
    isOver: false,
    over: null,
  })),
  SortableContext: vi.fn(({ children }) => <div>{children}</div>),
  verticalListSortingStrategy: vi.fn(),
}))

vi.mock('@dnd-kit/utilities', () => ({
  CSS: {
    Transform: {
      toString: vi.fn(() => ''),
    },
  },
}))

const mockBlock: Block = {
  id: 'block-1',
  pageId: 'page-1',
  pageName: 'Test Page',
  content: 'Test Card',
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
}

describe('KanbanColumn', () => {
  const mockOnPropertyChange = vi.fn()

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('4.3 [TEST] shows "No cards" when column is empty', () => {
    render(
      <KanbanColumn
        columnId="todo"
        title="To Do"
        blocks={[]}
        propertyKey="status"
        onPropertyChange={mockOnPropertyChange}
      />
    )

    expect(screen.getByTestId('kanban-column-todo')).toBeInTheDocument()
    expect(screen.getByTestId('kanban-column-todo-empty')).toBeInTheDocument()
    expect(screen.getByTestId('kanban-column-todo-empty')).toHaveTextContent('No cards')
  })

  it('4.3 [TEST] shows count in header', () => {
    render(
      <KanbanColumn
        columnId="todo"
        title="To Do"
        blocks={[mockBlock]}
        propertyKey="status"
        onPropertyChange={mockOnPropertyChange}
      />
    )

    expect(screen.getByTestId('kanban-column-todo')).toHaveTextContent('To Do')
    expect(screen.getByTestId('kanban-column-todo')).toHaveTextContent('(1)')
  })

  it('4.3 [TEST] renders blocks with virtuoso', () => {
    render(
      <KanbanColumn
        columnId="todo"
        title="To Do"
        blocks={[mockBlock]}
        propertyKey="status"
        onPropertyChange={mockOnPropertyChange}
      />
    )

    expect(screen.getByTestId('virtuoso-mock')).toBeInTheDocument()
    expect(screen.getByTestId('kanban-card')).toBeInTheDocument()
  })
})
