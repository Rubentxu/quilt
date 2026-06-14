import type { BlockRenderer } from './types'

function readProperty(block: any, key: string): string | null {
  const prop = block.properties?.find((p: any) => p.key === key)
  if (!prop || prop.value == null) return null
  return String(prop.value)
}

export const CreatedByRenderer: BlockRenderer = {
  id: 'created-by',
  priority: 5,

  match(block) {
    const val = readProperty(block, 'created_by')
    return val != null && val.length > 0
  },

  renderBeforeContent(ctx) {
    const createdByStr = readProperty(ctx.block, 'created_by')!
    const isAgentAuthor = createdByStr.startsWith('agent::')

    return (
      <span
        data-testid="created-by-badge"
        title={`Created by ${createdByStr}`}
        style={{
          flexShrink: 0,
          alignSelf: 'center',
          fontSize: '10px',
          fontWeight: 500,
          padding: '1px 6px',
          borderRadius: 'var(--radius-pill)',
          background: isAgentAuthor
            ? 'var(--color-accent-subtle, rgba(99, 102, 241, 0.12))'
            : 'var(--color-surface-subtle)',
          color: isAgentAuthor ? 'var(--color-accent)' : 'var(--color-text-muted)',
          display: 'inline-flex',
          alignItems: 'center',
          gap: '3px',
          letterSpacing: '0.01em',
          whiteSpace: 'nowrap',
          maxWidth: '160px',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
        }}
      >
        <span aria-hidden="true">{isAgentAuthor ? '🤖' : '👤'}</span>
        <span style={{ overflow: 'hidden', textOverflow: 'ellipsis' }}>{createdByStr}</span>
      </span>
    )
  },
}
