import type { BlockRenderer } from './types'
import type { Priority } from '@shared/types/api'

const PRIORITY_STYLES: Record<Priority, { bg: string; text: string }> = {
  A: { bg: 'var(--color-danger)', text: '#fff' },
  B: { bg: 'var(--color-warning)', text: '#fff' },
  C: { bg: 'var(--color-text-muted)', text: '#fff' },
}

export const PriorityRenderer: BlockRenderer = {
  id: 'priority',
  priority: 8,

  match(block) {
    return block.priority != null
  },

  renderBeforeContent(ctx) {
    const p = ctx.block.priority!
    const style = PRIORITY_STYLES[p]
    return (
      <span
        data-testid="priority-badge"
        style={{
          flexShrink: 0,
          alignSelf: 'center',
          fontSize: '11px',
          fontWeight: 600,
          padding: '2px 8px',
          borderRadius: 'var(--radius-pill)',
          background: style.bg,
          color: style.text,
          lineHeight: 1.4,
        }}
      >
        {p}
      </span>
    )
  },
}
