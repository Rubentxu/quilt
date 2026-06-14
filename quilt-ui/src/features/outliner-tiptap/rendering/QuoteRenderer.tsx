import type { BlockRenderer } from './types'

export const QuoteRenderer: BlockRenderer = {
  id: 'quote',
  priority: 15,

  match(block) {
    return block.blockType === 'quote'
  },

  wrapContent(_ctx, content) {
    return (
      <blockquote
        style={{
          borderLeft: '3px solid var(--color-border-strong)',
          paddingLeft: '12px',
          margin: '4px 0',
          fontStyle: 'italic',
          color: 'var(--color-text-secondary)',
          width: '100%',
        }}
      >
        {content}
      </blockquote>
    )
  },
}
