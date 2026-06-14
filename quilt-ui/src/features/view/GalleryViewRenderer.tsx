import type { Block } from '@shared/types/api'

interface GalleryViewRendererProps {
  /** The source query block */
  source: Block
  /** All blocks on the current page (for finding children) */
  allBlocks: Block[]
}

export function GalleryViewRenderer({ source, allBlocks }: GalleryViewRendererProps) {
  const children = allBlocks
    .filter(b => b.parentId === source.id)
    .sort((a, b) => a.order - b.order)

  if (children.length === 0) {
    return (
      <div
        data-testid="saved-view-cards"
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
      data-testid="saved-view-cards"
      style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(auto-fill, minmax(240px, 1fr))',
        gap: 'var(--space-2)',
      }}
    >
      {children.map(child => (
        <div
          key={child.id}
          data-testid={`gallery-card-${child.id}`}
          style={{
            background: 'var(--color-surface, #fff)',
            border: '1px solid var(--color-border)',
            borderRadius: 'var(--radius-md)',
            padding: 'var(--space-3)',
            display: 'flex',
            flexDirection: 'column',
            gap: 'var(--space-1)',
          }}
        >
          <div
            style={{
              fontSize: '13px',
              color: 'var(--color-text-primary)',
              lineHeight: 1.5,
              overflow: 'hidden',
              display: '-webkit-box',
              WebkitLineClamp: 3,
              WebkitBoxOrient: 'vertical',
            }}
          >
            {child.content || '(empty)'}
          </div>

          <div
            style={{
              display: 'flex',
              gap: '6px',
              flexWrap: 'wrap',
              marginTop: 'auto',
              paddingTop: 'var(--space-1)',
            }}
          >
            {child.marker && (
              <span
                style={{
                  fontSize: '10px',
                  fontWeight: 600,
                  padding: '1px 6px',
                  borderRadius: 'var(--radius-pill)',
                  background: markerBg(child.marker),
                  color: markerColor(child.marker),
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
                  padding: '1px 6px',
                  borderRadius: 'var(--radius-pill)',
                  background: 'var(--color-text-muted)',
                  color: '#fff',
                }}
              >
                {child.priority}
              </span>
            )}
            {child.blockType && child.blockType !== 'paragraph' && (
              <span
                style={{
                  fontSize: '10px',
                  color: 'var(--color-text-muted)',
                }}
              >
                {child.blockType}
              </span>
            )}
          </div>
        </div>
      ))}
    </div>
  )
}

function markerBg(marker: string): string {
  switch (marker) {
    case 'Todo': return 'var(--color-info-subtle, rgba(59, 130, 246, 0.1))'
    case 'Doing': return 'var(--color-warning-subtle, rgba(245, 158, 11, 0.1))'
    case 'Done': return 'var(--color-success-subtle, rgba(34, 197, 94, 0.1))'
    case 'Cancelled': return 'var(--color-text-disabled, #9ca3af)'
    default: return 'var(--color-surface-subtle)'
  }
}

function markerColor(marker: string): string {
  switch (marker) {
    case 'Todo': return 'var(--color-info, #3b82f6)'
    case 'Doing': return 'var(--color-warning, #f59e0b)'
    case 'Done': return 'var(--color-success, #22c55e)'
    case 'Cancelled': return '#fff'
    default: return 'var(--color-text-primary)'
  }
}
