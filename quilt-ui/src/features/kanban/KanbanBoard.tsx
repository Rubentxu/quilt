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
  /** Optional closed values for the grouping property (defines column order) */
  closedValues?: string[]
}

/**
 * Groups blocks by a property value, returning a map of column name -> blocks.
 * Uses closed_values if provided for column order, otherwise falls back to distinct values.
 */
function groupBlocksByProperty(
  blocks: Block[],
  propertyKey: string,
  closedValues?: string[],
): Map<string, Block[]> {
  const groups = new Map<string, Block[]>()

  // Use closed_values for column order if provided, otherwise use distinct values
  const columnOrder = closedValues
    ? new Set(closedValues)
    : new Set<string>()

  for (const block of blocks) {
    const prop = block.properties?.find(p => p.key === propertyKey)
    const columnKey = prop ? String(prop.value) : 'Unset'

    if (!groups.has(columnKey)) {
      groups.set(columnKey, [])
    }
    groups.get(columnKey)!.push(block)

    // If no closed_values provided, collect distinct values
    if (!closedValues) {
      columnOrder.add(columnKey)
    }
  }

  return groups
}

export function KanbanBoard({ propertyKey, blocks, onPropertyChange, closedValues }: KanbanBoardProps) {
  const columns = useMemo(() => {
    return groupBlocksByProperty(blocks, propertyKey, closedValues)
  }, [blocks, propertyKey, closedValues])

  // Sort columns: Unset last, then by closed_values order if provided, otherwise alphabetical
  const sortedColumnKeys = useMemo(() => {
    const keys = Array.from(columns.keys())

    // If closedValues provided, use that order (placing Unset at the end)
    if (closedValues && closedValues.length > 0) {
      const orderedKeys = closedValues.filter(v => columns.has(v))
      if (columns.has('Unset')) {
        orderedKeys.push('Unset')
      }
      return orderedKeys
    }

    // Otherwise sort: Unset last, then alphabetical
    return keys.sort((a, b) => {
      if (a === 'Unset') return 1
      if (b === 'Unset') return -1
      return a.localeCompare(b)
    })
  }, [columns, closedValues])

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
          columnId={columnKey === 'Unset' ? 'unset' : columnKey}
          title={columnKey === 'Unset' ? 'Unset' : columnKey}
          blocks={columns.get(columnKey) ?? []}
          propertyKey={propertyKey}
          onPropertyChange={onPropertyChange}
        />
      ))}
    </div>
  )
}
