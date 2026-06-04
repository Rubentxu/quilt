/**
 * KanbanCard — F22 Kanban View
 *
 * A draggable card within a Kanban column. Shows the block's title,
 * two meta properties, and a footer with drag handle.
 */

import type { Block } from '@shared/types/api'

export interface KanbanCardProps {
  /** Block to display */
  block: Block
  /** Property key being used for grouping (to show in footer) */
  propertyKey: string
  /** Called when a block's property is changed */
  onPropertyChange: (blockId: string, key: string, value: string) => void
}

/**
 * Extracts a display-friendly title from a block.
 */
function getBlockTitle(block: Block): string {
  if (block.content && block.content.trim()) {
    return block.content.trim()
  }
  // Fallback: show first property value or a placeholder
  const firstProp = block.properties?.[0]
  if (firstProp) {
    return String(firstProp.value)
  }
  return '(empty)'
}

/**
 * Gets two meta properties to display (excluding the grouping property).
 */
function getCardMetas(block: Block, excludeKey: string): Array<{ key: string; value: string }> {
  if (!block.properties) return []

  return block.properties
    .filter(p => p.key !== excludeKey && p.key !== 'template')
    .slice(0, 2)
    .map(p => ({
      key: p.key,
      value: String(p.value),
    }))
}

export function KanbanCard({ block, propertyKey, onPropertyChange }: KanbanCardProps) {
  const title = getBlockTitle(block)
  const metas = getCardMetas(block, propertyKey)

  return (
    <div
      data-testid="kanban-card"
      data-block-id={block.id}
      style={{
        background: 'var(--color-surface)',
        borderRadius: 'var(--radius-md)',
        padding: 'var(--space-3)',
        boxShadow: 'var(--shadow-sm)',
        cursor: 'grab',
        userSelect: 'none',
      }}
    >
      {/* Title */}
      <div
        style={{
          fontSize: '14px',
          fontWeight: 500,
          color: 'var(--color-text-primary)',
          marginBottom: metas.length > 0 ? 'var(--space-2)' : 0,
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap',
        }}
      >
        {title}
      </div>

      {/* Meta properties */}
      {metas.length > 0 && (
        <div
          style={{
            display: 'flex',
            gap: 'var(--space-2)',
            marginBottom: 'var(--space-2)',
          }}
        >
          {metas.map(meta => (
            <span
              key={meta.key}
              style={{
                fontSize: '11px',
                color: 'var(--color-text-muted)',
                background: 'var(--color-surface-subtle)',
                padding: '2px 6px',
                borderRadius: 'var(--radius-sm)',
              }}
            >
              {meta.key}: {meta.value}
            </span>
          ))}
        </div>
      )}

      {/* Footer with grouping property */}
      <div
        style={{
          fontSize: '10px',
          color: 'var(--color-text-muted)',
          borderTop: '1px solid var(--color-border)',
          paddingTop: 'var(--space-2)',
          marginTop: 'var(--space-2)',
        }}
      >
        {propertyKey}
      </div>
    </div>
  )
}
