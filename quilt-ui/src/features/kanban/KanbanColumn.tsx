/**
 * KanbanColumn — F22 Kanban View
 *
 * A single column in the Kanban board that displays blocks grouped by
 * a property value. Uses react-virtuoso for virtualization.
 */

import { useMemo } from 'react'
import { TableVirtuoso } from 'react-virtuoso'
import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import type { Block } from '@shared/types/api'
import { KanbanCard } from './KanbanCard'

export interface KanbanColumnProps {
  /** Unique identifier for this column */
  columnId: string
  /** Display title for the column header */
  title: string
  /** Blocks in this column */
  blocks: Block[]
  /** Property key being used for grouping */
  propertyKey: string
  /** Called when a block's property is changed */
  onPropertyChange: (blockId: string, key: string, value: string) => void
}

export function KanbanColumn({
  columnId,
  title,
  blocks,
  propertyKey,
  onPropertyChange,
}: KanbanColumnProps) {
  if (blocks.length === 0) {
    return (
      <div
        data-testid={`kanban-column-${columnId}`}
        style={{
          minWidth: '280px',
          maxWidth: '320px',
          background: 'var(--color-surface-subtle)',
          borderRadius: 'var(--radius-lg)',
          padding: 'var(--space-3)',
        }}
      >
        <div
          style={{
            fontSize: '13px',
            fontWeight: 600,
            color: 'var(--color-text-secondary)',
            marginBottom: 'var(--space-3)',
            padding: '0 var(--space-2)',
          }}
        >
          {title}
          <span
            style={{
              marginLeft: 'var(--space-2)',
              fontSize: '11px',
              color: 'var(--color-text-muted)',
            }}
          >
            (0)
          </span>
        </div>
        <div
          data-testid={`kanban-column-${columnId}-empty`}
          style={{
            padding: 'var(--space-4)',
            textAlign: 'center',
            color: 'var(--color-text-muted)',
            fontSize: '12px',
          }}
        >
          No cards
        </div>
      </div>
    )
  }

  return (
    <div
      data-testid={`kanban-column-${columnId}`}
      style={{
        minWidth: '280px',
        maxWidth: '320px',
        background: 'var(--color-surface-subtle)',
        borderRadius: 'var(--radius-lg)',
        padding: 'var(--space-3)',
        display: 'flex',
        flexDirection: 'column',
      }}
    >
      <div
        style={{
          fontSize: '13px',
          fontWeight: 600,
          color: 'var(--color-text-secondary)',
          marginBottom: 'var(--space-3)',
          padding: '0 var(--space-2)',
        }}
      >
        {title}
        <span
          style={{
            marginLeft: 'var(--space-2)',
            fontSize: '11px',
            color: 'var(--color-text-muted)',
          }}
        >
          ({blocks.length})
        </span>
      </div>

      <div style={{ flex: 1, overflow: 'auto' }}>
        <TableVirtuoso
          data={blocks}
          height={400}
          fixedItemHeight={80}
          initialItemCount={Math.min(blocks.length, 10)}
          components={{
            Table: (props) => (
              <table
                data-testid={`kanban-column-${columnId}-table`}
                role="table"
                aria-label={`${title} cards`}
                style={{ width: '100%', borderCollapse: 'collapse' }}
                {...props}
              />
            ),
            TableBody: (props) => (
              <tbody {...props} />
            ),
          }}
          itemContent={(index, block) => (
            <td
              key={block.id}
              style={{
                padding: 'var(--space-2) 0',
              }}
            >
              <SortableKanbanCard
                block={block}
                propertyKey={propertyKey}
                onPropertyChange={onPropertyChange}
              />
            </td>
          )}
        />
      </div>
    </div>
  )
}

interface SortableKanbanCardProps {
  block: Block
  propertyKey: string
  onPropertyChange: (blockId: string, key: string, value: string) => void
}

function SortableKanbanCard({ block, propertyKey, onPropertyChange }: SortableKanbanCardProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: block.id })

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  }

  return (
    <div ref={setNodeRef} style={style} {...attributes} {...listeners}>
      <KanbanCard
        block={block}
        propertyKey={propertyKey}
        onPropertyChange={onPropertyChange}
      />
    </div>
  )
}
