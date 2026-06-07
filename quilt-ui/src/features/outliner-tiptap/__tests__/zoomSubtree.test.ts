/**
 * Tests for zoomSubtree — the pure filter that returns the set of
 * block IDs to display when zooming into a specific block.
 *
 * The zoom contract: when `zoomBlockId` is set, only the zoomed block
 * and its descendants (transitive children) are visible. When null,
 * all top-level blocks are visible (the regular flattenBlockTree
 * behaviour).
 */
import { describe, it, expect } from 'vitest'
import { collectZoomSubtree, type BlockIdSet } from '@features/outliner-tiptap/zoomSubtree'
import type { Block } from '@shared/types/api'

// ── Helpers ──────────────────────────────────────────────────

function b(
  id: string,
  parentId: string | null,
  extra: Partial<Block> = {},
): Block {
  return {
    id,
    pageId: 'p1',
    pageName: null,
    content: `block ${id}`,
    blockType: 'paragraph',
    marker: null,
    priority: null,
    parentId,
    order: 0,
    level: parentId ? 1 : 0,
    collapsed: false,
    createdAt: '2026-01-01',
    updatedAt: '2026-01-01',
    ...extra,
  }
}

// ── Tests ──────────────────────────────────────────────────────

describe('collectZoomSubtree', () => {
  it('returns null when zoomBlockId is null (no zoom active)', () => {
    const blocks = [b('a', null), b('b', null)]
    expect(collectZoomSubtree(blocks, null)).toBeNull()
  })

  it('returns null when zoomBlockId is empty string', () => {
    const blocks = [b('a', null)]
    expect(collectZoomSubtree(blocks, '')).toBeNull()
  })

  it('returns just the zoomed block when it has no children', () => {
    const blocks = [b('a', null), b('b', null)]
    const result = collectZoomSubtree(blocks, 'b')
    expect(result).toBeInstanceOf(Set)
    expect(result as BlockIdSet).toEqual(new Set(['b']))
  })

  it('returns the zoomed block + direct children', () => {
    const blocks = [
      b('a', null),
      b('b', null),
      b('b1', 'b'),
      b('b2', 'b'),
    ]
    const result = collectZoomSubtree(blocks, 'b')
    expect(result).toEqual(new Set(['b', 'b1', 'b2']))
  })

  it('returns the zoomed block + transitive descendants', () => {
    const blocks = [
      b('root', null),
      b('child', 'root'),
      b('grandchild', 'child'),
      b('great-grandchild', 'grandchild'),
    ]
    const result = collectZoomSubtree(blocks, 'root')
    expect(result).toEqual(new Set(['root', 'child', 'grandchild', 'great-grandchild']))
  })

  it('does NOT include non-descendants (sibling subtrees)', () => {
    const blocks = [
      b('a', null),
      b('a1', 'a'),
      b('a2', 'a'),
      b('b', null),                   // sibling of a
      b('b1', 'b'),                   // b's child
      b('c', null),                   // another sibling
    ]
    const result = collectZoomSubtree(blocks, 'a')
    // Only a, a1, a2 — b/b1 and c are siblings, not descendants
    expect(result).toEqual(new Set(['a', 'a1', 'a2']))
  })

  it('returns an empty Set when zoomBlockId does not exist in blocks', () => {
    const blocks = [b('a', null), b('b', null)]
    const result = collectZoomSubtree(blocks, 'nonexistent')
    expect(result).toBeInstanceOf(Set)
    expect(result as BlockIdSet).toEqual(new Set())
  })

  it('handles deep trees (10 levels) correctly', () => {
    let parent: string | null = null
    const blocks: Block[] = []
    for (let i = 0; i < 10; i++) {
      const id = `lvl${i}`
      blocks.push(b(id, parent))
      parent = id
    }
    const result = collectZoomSubtree(blocks, 'lvl0')
    expect(result?.size).toBe(10)
    for (let i = 0; i < 10; i++) {
      expect(result?.has(`lvl${i}`)).toBe(true)
    }
  })

  it('does not infinite-loop on cycles (defensive guard)', () => {
    // Manually craft a cycle: a -> b -> a. The function must terminate.
    const a: Block = { ...b('a', 'b'), parentId: 'b' }
    const bBlock: Block = { ...b('b', 'a'), parentId: 'a' }
    const result = collectZoomSubtree([a, bBlock], 'a')
    // Should at least contain 'a'. May or may not contain 'b' depending
    // on cycle handling, but MUST terminate.
    expect(result).toBeInstanceOf(Set)
    expect(result?.has('a')).toBe(true)
  })
})
