import type { BlockRenderer } from './types'

export const BulletListRenderer: BlockRenderer = {
  id: 'bullet-list',
  priority: 5,

  match(block) {
    return block.blockType === 'bullet'
  },

  renderBullet(_ctx, _defaultBullet) {
    return (
      <span
        style={{
          fontSize: '1.2em',
          lineHeight: 1,
          color: 'var(--color-text-muted)',
          userSelect: 'none',
        }}
        aria-hidden="true"
      >
        •
      </span>
    )
  },
}
