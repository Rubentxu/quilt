import type { Block } from '@shared/types/api'

interface ListViewRendererProps {
  /** The source query block */
  source: Block
  /** All blocks on the current page (for finding children) */
  allBlocks: Block[]
}

export function ListViewRenderer({ source, allBlocks }: ListViewRendererProps) {
  const children = allBlocks
    .filter(b => b.parentId === source.id)
    .sort((a, b) => a.order - b.order)

  if (children.length === 0) {
    return (
      <div
        data-testid="saved-view-list"
        style={{
          padding: 'var(--space-3)',
          color: 'var(--color-text-muted)',
          fontSize: '13px',
          textAlign: 'center',
          border: '1px dashed var(--color-border)',
          borderRadius: 'var(--radius-sm)',
        }}
      >
        No items to display
      </div>
    )
  }

  return (
    <div
      data-testid="saved-view-list"
      style={{
        display: 'flex',
        flexDirection: 'column',
        border: '1px solid var(--color-border)',
        borderRadius: 'var(--radius-md)',
        overflow: 'hidden',
      }}
    >
      {children.map((child, idx) => (
        <div
          key={child.id}
          data-testid={`list-item-${child.id}`}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 'var(--space-2)',
            padding: 'var(--space-1) var(--space-2)',
            borderBottom: idx < children.length - 1
              ? '1px solid var(--color-border)'
              : 'none',
            fontSize: '13px',
            minHeight: '32px',
          }}
        >
          <span
            style={{
              width: '6px',
              height: '6px',
              borderRadius: '50%',
              background: 'var(--color-border-strong)',
              flexShrink: 0,
            }}
          />

          <span
            style={{
              flex: 1,
              color: 'var(--color-text-primary)',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {child.content || '(empty)'}
          </span>

          {child.marker && (
            <span
              style={{
                fontSize: '10px',
                fontWeight: 600,
                padding: '0 6px',
                borderRadius: 'var(--radius-pill)',
                background: 'var(--color-surface-subtle)',
                color: 'var(--color-text-muted)',
              }}
            >
              {child.marker}
            </span>
          )}
          {child.priority && (
            <span
              style={{
                fontSize: '10px',
                fontWeight: 600,
                padding: '0 6px',
                borderRadius: 'var(--radius-pill)',
                background: 'var(--color-text-muted)',
                color: '#fff',
              }}
            >
              {child.priority}
            </span>
          )}
        </div>
      ))}
    </div>
  )
}
