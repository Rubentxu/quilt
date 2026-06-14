import type { BlockRenderer } from './types'

export const DividerRenderer: BlockRenderer = {
  id: 'divider',
  priority: 20,

  match(block) {
    return block.blockType === 'divider'
  },

  wrapContent(_ctx, _content) {
    return (
      <hr
        style={{
          width: '100%',
          border: 'none',
          borderTop: '1px solid var(--color-border)',
          margin: 'var(--space-2) 0',
        }}
      />
    )
  },
}
