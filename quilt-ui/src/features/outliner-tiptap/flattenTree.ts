import type { Block } from '@shared/types/api'
import { isCommentBlock } from '@shared/utils/blockProperties'

export interface FlatBlock {
  block: Block
  depth: number
  hasChildren: boolean
}

/**
 * Filter that excludes blocks that should not appear in the regular
 * outline tree. By default, comment blocks (those with a
 * `type: "comment"` property) are filtered out — they are rendered
 * inline below their parent block instead.
 */
function isRegularOutlineBlock(block: Block): boolean {
  return !isCommentBlock(block)
}

/**
 * Flatten a tree of blocks into a flat list with depth info.
 * Respects collapsed state — collapsed children are excluded.
 *
 * Comment blocks are filtered out of the regular tree and rendered
 * inline below their parent by the BlockRow component.
 */
export function flattenBlockTree(
  blocks: Block[],
  parentId: string | null,
  collapsedIds: Set<string>,
  depth: number = 0,
): FlatBlock[] {
  const result: FlatBlock[] = []

  const children = blocks
    .filter(b => b.parentId === parentId && isRegularOutlineBlock(b))
    .sort((a, b) => a.order - b.order)

  for (const block of children) {
    // Count only non-comment children for the expand/collapse bullet
    const childBlocks = blocks.filter(
      b => b.parentId === block.id && isRegularOutlineBlock(b),
    )
    const hasChildren = childBlocks.length > 0

    result.push({ block, depth, hasChildren })

    // Only recurse if not collapsed
    if (!collapsedIds.has(block.id) && hasChildren) {
      result.push(...flattenBlockTree(blocks, block.id, collapsedIds, depth + 1))
    }
  }

  return result
}
