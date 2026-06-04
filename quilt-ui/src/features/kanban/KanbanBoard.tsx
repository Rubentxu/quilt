/**
 * KanbanBoard — F22 Kanban View
 *
 * Displays blocks grouped by a property value (e.g., status) as a Kanban board
 * with drag-and-drop support for moving cards between columns.
 */

import { useMemo } from 'react'
import { KanbanColumn } from './KanbanColumn'
import type { Block } from '@shared/types/api'

export interface KanbanBoardProps {
  /** Property key to group by (e.g., "status", "priority") */
  propertyKey: string
  /** All blocks to display */
  blocks: Block[]
  /** Called when a block's property is changed (e.g., moving to a different column) */
  onPropertyChange: (blockId: string, key: string, value: string) => void
}

/**
 * Groups blocks by a property value, returning a map of column name -> blocks.
 */
function groupBlocksByProperty(
  blocks: Block[],
  propertyKey: string,
): Map<string, Block[]> {
  const groups = new Map<string, Block[]>()

  for (const block of blocks) {
    const prop = block.properties?.find(p => p.key === propertyKey)
    const columnKey = prop ? String(prop.value) : 'uncategorized'

    if (!groups.has(columnKey)) {
      groups.set(columnKey, [])
    }
    groups.get(columnKey)!.push(block)
  }

  return groups
}

export function KanbanBoard({ propertyKey, blocks, onPropertyChange }: KanbanBoardProps) {
  const columns = useMemo(() => {
    return groupBlocksByProperty(blocks, propertyKey)
  }, [blocks, propertyKey])

  // Sort columns: uncategorized last, then alphabetical
  const sortedColumnKeys = useMemo(() => {
    const keys = Array.from(columns.keys())
    return keys.sort((a, b) => {
      if (a === 'uncategorized') return 1
      if (b === 'uncategorized') return -1
      return a.localeCompare(b)
    })
  }, [columns])

  if (blocks.length === 0) {
    return (
      <div
        data-testid="kanban-board-empty"
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          height: '200px',
          color: 'var(--color-text-muted)',
        }}
      >
        No cards to display
      </div>
    )
  }

  return (
    <div
      data-testid="kanban-board"
      style={{
        display: 'flex',
        gap: 'var(--space-4)',
        overflowX: 'auto',
        padding: 'var(--space-4)',
      }}
    >
      {sortedColumnKeys.map(columnKey => (
        <KanbanColumn
          key={columnKey}
          columnId={columnKey === 'uncategorized' ? 'uncategorized' : columnKey}
          title={columnKey === 'uncategorized' ? 'Uncategorized' : columnKey}
          blocks={columns.get(columnKey) ?? []}
          propertyKey={propertyKey}
          onPropertyChange={onPropertyChange}
        />
      ))}
    </div>
  )
}
