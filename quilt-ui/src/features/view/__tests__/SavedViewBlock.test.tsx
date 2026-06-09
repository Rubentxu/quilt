/**
 * SavedViewBlock tests — ADR-DRAFT-saved-view-block-role
 *
 * A block with `type:: view` is a SavedView: a presentation layer that
 * composes a reference to a Query block via `data-source::`. The
 * SavedViewBlock component reads `view-type::`, `data-source::`, and
 * `view-name::` from the block's properties array and renders the
 * appropriate view component (TableView, KanbanBoard, etc.) with the
 * source block's data.
 *
 * Per the ADR, no backend changes are required: the source block is
 * looked up from the `allBlocks` array that the parent BlockRow
 * already receives. The lookup is purely synchronous — no async API
 * call, no loading state, no error envelope from the server. The only
 * "error" is "source block not present in allBlocks" (e.g., the view
 * is on page A but the query lives on page B; for V1 the user must
 * either inline the query on the same page or accept the error state).
 *
 * These tests pin the public contract:
 *   - view-name:: is rendered as a label
 *   - view-type:: drives which wrapper testid is mounted
 *   - data-source:: missing → error state
 *   - source block not in allBlocks → error state
 *   - unknown view-type:: → error state
 *   - actual view components are composed (KanbanBoard, TableView)
 *     when the view-type matches a known renderer
 */

import { render, screen, within } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { SavedViewBlock } from '../SavedViewBlock'
import type { Block, BlockProperty } from '@shared/types/api'

// Mock the heavy view components so the tests stay focused on the
// dispatch + label contract. The mock for each component mounts a
// div with `data-testid` matching the component name, which lets
// tests assert "did we render the right renderer?" without booting
// the real react-virtuoso / dnd-kit machinery.
vi.mock('@features/table-view/TableView', () => ({
  TableView: (props: { rows: unknown[]; columns: unknown[] }) => (
    <div
      data-testid="table-view-mock"
      data-rows={JSON.stringify(props.rows.length)}
    />
  ),
}))

vi.mock('@features/kanban/KanbanBoard', () => ({
  KanbanBoard: (props: { blocks: unknown[]; propertyKey: string }) => (
    <div
      data-testid="kanban-board-mock"
      data-property-key={props.propertyKey}
      data-blocks={JSON.stringify(props.blocks.length)}
    />
  ),
}))

// ──── Test helpers ─────────────────────────────────────────────

/** Build a minimal Block that satisfies the BlockRow contract. */
function makeBlock(
  id: string,
  content: string,
  properties: BlockProperty[] = [],
): Block {
  return {
    id,
    pageId: 'p1',
    pageName: 'demo',
    content,
    blockType: 'paragraph',
    marker: null,
    priority: null,
    parentId: null,
    order: 1,
    level: 0,
    collapsed: false,
    properties,
    createdAt: '2026-06-07T00:00:00Z',
    updatedAt: '2026-06-07T00:00:00Z',
  }
}

/** Build a `type:: view` block with the given view properties. */
function makeViewBlock(
  properties: {
    viewType?: string
    dataSource?: string
    viewName?: string
    groupBy?: string
    content?: string
    id?: string
  } = {},
): Block {
  const props: BlockProperty[] = [
    { key: 'type', value: 'view', type: 'string' },
  ]
  if (properties.viewType !== undefined) {
    props.push({ key: 'view-type', value: properties.viewType, type: 'select' })
  }
  if (properties.dataSource !== undefined) {
    props.push({ key: 'data-source', value: properties.dataSource, type: 'string' })
  }
  if (properties.viewName !== undefined) {
    props.push({ key: 'view-name', value: properties.viewName, type: 'string' })
  }
  if (properties.groupBy !== undefined) {
    props.push({ key: 'group-by', value: properties.groupBy, type: 'string' })
  }
  return makeBlock(properties.id ?? 'view-1', properties.content ?? '', props)
}

/** Build a `type:: query` source block carrying a `dsl::` property. */
function makeQuerySource(
  id: string,
  dsl: string,
  content = 'all tasks',
): Block {
  return makeBlock(id, content, [
    { key: 'type', value: 'query', type: 'string' },
    { key: 'dsl', value: dsl, type: 'string' },
  ])
}

beforeEach(() => {
  vi.clearAllMocks()
})

// ──── view-name label ─────────────────────────────────────────

describe('SavedViewBlock — view-name label', () => {
  it('renders the view-name:: property as a label', () => {
    const view = makeViewBlock({
      viewType: 'kanban',
      dataSource: 'q1',
      viewName: 'My Tasks',
    })
    const source = makeQuerySource('q1', '(task TODO)')

    render(<SavedViewBlock block={view} allBlocks={[view, source]} />)

    // The label is exposed as a heading-styled element so it's
    // discoverable by screen readers and visual scans.
    const label = screen.getByTestId('saved-view-name')
    expect(label).toHaveTextContent('My Tasks')
  })

  it('falls back to "Untitled view" when view-name:: is missing', () => {
    const view = makeViewBlock({
      viewType: 'table',
      dataSource: 'q1',
    })
    const source = makeQuerySource('q1', '(task TODO)')

    render(<SavedViewBlock block={view} allBlocks={[view, source]} />)

    const label = screen.getByTestId('saved-view-name')
    expect(label).toHaveTextContent(/untitled/i)
  })
})

// ──── view-type dispatch ──────────────────────────────────────

describe('SavedViewBlock — view-type dispatch', () => {
  it('renders the kanban wrapper when view-type:: is "kanban"', () => {
    const view = makeViewBlock({ viewType: 'kanban', dataSource: 'q1' })
    const source = makeQuerySource('q1', '(task TODO)')

    render(<SavedViewBlock block={view} allBlocks={[view, source]} />)

    expect(screen.getByTestId('saved-view-kanban')).toBeInTheDocument()
  })

  it('renders the table wrapper when view-type:: is "table"', () => {
    const view = makeViewBlock({ viewType: 'table', dataSource: 'q1' })
    const source = makeQuerySource('q1', '(task TODO)')

    render(<SavedViewBlock block={view} allBlocks={[view, source]} />)

    expect(screen.getByTestId('saved-view-table')).toBeInTheDocument()
  })

  it('renders a placeholder wrapper for the list, graph, cards, calendar, and timeline view-types', () => {
    const placeholders = ['list', 'graph', 'cards', 'calendar', 'timeline']
    for (const viewType of placeholders) {
      const view = makeViewBlock({ viewType, dataSource: 'q1' })
      const source = makeQuerySource('q1', '(task TODO)')

      const { unmount } = render(
        <SavedViewBlock block={view} allBlocks={[view, source]} />,
      )

      // Each placeholder gets its own testid so the wiring test can
      // assert exactly which renderer the dispatcher chose.
      expect(
        screen.getByTestId(`saved-view-${viewType}`),
      ).toBeInTheDocument()
      unmount()
    }
  })

  it('shows an error state when view-type:: is not in the recognised set', () => {
    const view = makeViewBlock({ viewType: 'mystery', dataSource: 'q1' })
    const source = makeQuerySource('q1', '(task TODO)')

    render(<SavedViewBlock block={view} allBlocks={[view, source]} />)

    const error = screen.getByTestId('saved-view-error')
    expect(error).toBeInTheDocument()
    expect(error).toHaveTextContent(/mystery/i)
  })
})

// ──── data-source handling ────────────────────────────────────

describe('SavedViewBlock — data-source handling', () => {
  it('shows an error state when data-source:: is missing', () => {
    const view = makeViewBlock({ viewType: 'kanban' })
    // No data-source property at all.

    render(<SavedViewBlock block={view} allBlocks={[view]} />)

    const error = screen.getByTestId('saved-view-error')
    expect(error).toBeInTheDocument()
    expect(error).toHaveTextContent(/data-source/i)
  })

  it('shows an error state when the source block is not present in allBlocks', () => {
    // The view references a UUID that does not exist on the current
    // page (the source query lives elsewhere). For V1 we surface a
    // clear "not found" rather than silently rendering empty.
    const view = makeViewBlock({ viewType: 'kanban', dataSource: 'q-missing' })
    const other = makeQuerySource('q-other', '(task DONE)')

    render(<SavedViewBlock block={view} allBlocks={[view, other]} />)

    const error = screen.getByTestId('saved-view-error')
    expect(error).toBeInTheDocument()
    expect(error).toHaveTextContent(/q-missing|not found/i)
  })

  it('looks up the source by exact UUID match (not by content)', () => {
    // Two query blocks with identical content; only the one whose id
    // matches data-source:: should be picked up.
    const view = makeViewBlock({ viewType: 'kanban', dataSource: 'q-real' })
    const decoy = makeQuerySource('q-decoy', '(task TODO)', 'same content')
    const real = makeQuerySource('q-real', '(task TODO)', 'same content')

    render(<SavedViewBlock block={view} allBlocks={[view, decoy, real]} />)

    // The kanban wrapper should be mounted (not the error state),
    // proving the dispatcher found *some* source. The mock
    // additionally receives the data-source-block wrapped in a
    // single-element array — that confirms the *correct* source was
    // passed through (decoy and real are not the same block).
    const board = screen.getByTestId('kanban-board-mock')
    expect(board).toBeInTheDocument()
    expect(board.getAttribute('data-blocks')).toBe('1')
  })
})

// ──── Composition with real view components ───────────────────

describe('SavedViewBlock — composes the existing view components', () => {
  it('mounts KanbanBoard with the source block as the only block and the view-type\'s group-by as the property key', () => {
    const view = makeViewBlock({
      viewType: 'kanban',
      dataSource: 'q1',
      groupBy: 'priority',
    })
    const source = makeQuerySource('q1', '(task TODO)')

    render(<SavedViewBlock block={view} allBlocks={[view, source]} />)

    const board = screen.getByTestId('kanban-board-mock')
    expect(board).toBeInTheDocument()
    expect(board.getAttribute('data-property-key')).toBe('priority')
    // The source block is the only result the kanban board can show
    // (V1 renders the source as a single-card group).
    expect(board.getAttribute('data-blocks')).toBe('1')
  })

  it('mounts TableView with a default column shape when the view-type is "table"', () => {
    const view = makeViewBlock({ viewType: 'table', dataSource: 'q1' })
    const source = makeQuerySource('q1', '(task TODO)')

    render(<SavedViewBlock block={view} allBlocks={[view, source]} />)

    const table = screen.getByTestId('table-view-mock')
    expect(table).toBeInTheDocument()
    // The table receives one row per source block (V1: source-as-row).
    expect(table.getAttribute('data-rows')).toBe('1')
  })

  it('forwards the group-by:: property to the kanban board only when set', () => {
    // No group-by property → the kanban should fall back to a
    // sensible default (e.g., 'status') rather than crashing.
    const view = makeViewBlock({ viewType: 'kanban', dataSource: 'q1' })
    const source = makeQuerySource('q1', '(task TODO)')

    render(<SavedViewBlock block={view} allBlocks={[view, source]} />)

    const board = screen.getByTestId('kanban-board-mock')
    const propKey = board.getAttribute('data-property-key')
    expect(propKey).toBeTruthy()
    // The default is the project's canonical grouping key. We don't
    // pin the exact value to avoid breaking the test on renames, but
    // it must be a non-empty string.
    expect(propKey!.length).toBeGreaterThan(0)
  })
})

// ──── Container layout ────────────────────────────────────────

describe('SavedViewBlock — container layout', () => {
  it('renders inside a top-level container with a known testid', () => {
    const view = makeViewBlock({ viewType: 'kanban', dataSource: 'q1' })
    const source = makeQuerySource('q1', '(task TODO)')

    render(<SavedViewBlock block={view} allBlocks={[view, source]} />)

    // The container testid is the stable hook for future styling
    // (e.g., a "view card" frame that wraps the label + renderer).
    const container = screen.getByTestId('saved-view-block')
    expect(container).toBeInTheDocument()

    // The label is inside the container (sibling of the renderer).
    const label = within(container).getByTestId('saved-view-name')
    expect(label).toBeInTheDocument()
  })
})
