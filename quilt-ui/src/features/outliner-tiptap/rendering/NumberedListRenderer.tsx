import type { BlockRenderer, BlockRendererContext } from './types'

/**
 * Count how many consecutive numbered-block siblings precede `block`
 * (including the block itself) to determine the list item number.
 */
function getNumberedIndex(ctx: BlockRendererContext): number {
  const { block, allBlocks } = ctx
  if (!allBlocks) return 1

  const siblings = allBlocks.filter(b => b.parentId === block.parentId)
  const blockIdx = siblings.findIndex(b => b.id === block.id)
  if (blockIdx < 0) return 1

  // Walk backwards counting consecutive numbered blocks
  let count = 1
  for (let i = blockIdx - 1; i >= 0; i--) {
    if (siblings[i].blockType === 'numbered') {
      count++
    } else {
      break
    }
  }
  return count
}

export const NumberedListRenderer: BlockRenderer = {
  id: 'numbered-list',
  priority: 3,

  match(block) {
    return block.blockType === 'numbered'
  },

  renderBullet(ctx, _defaultBullet) {
    const number = getNumberedIndex(ctx)
    return (
      <span
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          justifyContent: 'center',
          width: '18px',
          height: '18px',
          fontSize: '12px',
          fontWeight: 600,
          color: 'var(--color-text-muted)',
          fontVariantNumeric: 'tabular-nums',
          flexShrink: 0,
          userSelect: 'none',
        }}
      >
        {`${number}.`}
      </span>
    )
  },
}
