/**
 * Tests for flattenBlockTree — flattens a hierarchical block list
 * into a depth-annotated flat list, respecting collapsed state.
 */
import { describe, it, expect } from 'vitest'
import { flattenBlockTree } from '@features/outliner-tiptap/flattenTree'
import type { Block } from '@shared/types/api'

// ── Helpers ──────────────────────────────────────────────────

function b(
  id: string,
  parentId: string | null,
  order: number,
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
    order,
    level: parentId ? 2 : 1,
    collapsed: false,
    properties: [],
    createdAt: '2026-01-01',
    updatedAt: '2026-01-01',
    ...extra,
  }
}

// ── Basic flattening ─────────────────────────────────────────

describe('flattenBlockTree', () => {
  it('returns empty array for empty block list', () => {
    expect(flattenBlockTree([], null, new Set())).toEqual([])
  })

  it('flattens a single root block', () => {
    const blocks = [b('1', null, 0)]
    const result = flattenBlockTree(blocks, null, new Set())
    expect(result).toHaveLength(1)
    expect(result[0]).toMatchObject({ block: blocks[0], depth: 0, hasChildren: false })
  })

  it('flattens root blocks only (depth 0)', () => {
    const blocks = [b('1', null, 0), b('2', null, 1)]
    const result = flattenBlockTree(blocks, null, new Set())
    expect(result).toHaveLength(2)
    expect(result[0].depth).toBe(0)
    expect(result[1].depth).toBe(0)
  })

  it('flattens nested children with increasing depth', () => {
    const blocks = [
      b('root', null, 0),
      b('child1', 'root', 0),
      b('grandchild', 'child1', 0),
    ]
    const result = flattenBlockTree(blocks, null, new Set())
    expect(result).toHaveLength(3)
    expect(result[0]).toMatchObject({ depth: 0, hasChildren: true })
    expect(result[1]).toMatchObject({ depth: 1, hasChildren: true })
    expect(result[2]).toMatchObject({ depth: 2, hasChildren: false })
  })

  it('sorts siblings by order', () => {
    const blocks = [
      b('b', null, 1),
      b('a', null, 0),
      b('c', null, 2),
    ]
    const result = flattenBlockTree(blocks, null, new Set())
    expect(result.map(r => r.block.id)).toEqual(['a', 'b', 'c'])
  })

  // ── Collapsed state ─────────────────────────────────────

  it('excludes children of collapsed blocks', () => {
    const blocks = [
      b('root', null, 0, { collapsed: true }),
      b('hidden', 'root', 0),
    ]
    const collapsed = new Set(['root'])
    const result = flattenBlockTree(blocks, null, collapsed)
    expect(result).toHaveLength(1)
    expect(result[0].block.id).toBe('root')
  })

  it('shows children when parent is NOT collapsed', () => {
    const blocks = [
      b('root', null, 0, { collapsed: false }),
      b('child', 'root', 0),
    ]
    const result = flattenBlockTree(blocks, null, new Set())
    expect(result).toHaveLength(2)
  })

  it('respects multiple collapsed blocks independently', () => {
    const blocks = [
      b('a', null, 0, { collapsed: true }),
      b('a_child', 'a', 0),
      b('b', null, 1, { collapsed: false }),
      b('b_child', 'b', 0),
    ]
    const collapsed = new Set(['a'])
    const result = flattenBlockTree(blocks, null, collapsed)
    const ids = result.map(r => r.block.id)
    expect(ids).toContain('a')
    expect(ids).not.toContain('a_child')
    expect(ids).toContain('b')
    expect(ids).toContain('b_child')
  })

  // ── Comment filtering ────────────────────────────────────

  it('filters out comment blocks from the regular tree', () => {
    const blocks = [
      b('root', null, 0),
      b('comment1', 'root', 0, {
        properties: [{ key: 'type', value: 'comment', type: 'string' }],
      }),
      b('normal', 'root', 1),
    ]
    const result = flattenBlockTree(blocks, null, new Set())
    const ids = result.map(r => r.block.id)
    expect(ids).toEqual(['root', 'normal'])
  })

  it('does not count comments as children for hasChildren flag', () => {
    const blocks = [
      b('root', null, 0),
      b('comment1', 'root', 0, {
        properties: [{ key: 'type', value: 'comment', type: 'string' }],
      }),
    ]
    const result = flattenBlockTree(blocks, null, new Set())
    expect(result[0].hasChildren).toBe(false)
  })

  // ── Deep nesting ────────────────────────────────────────

  it('handles deeply nested structure', () => {
    const blocks = [
      b('1', null, 0),
      b('2', '1', 0),
      b('3', '2', 0),
      b('4', '3', 0),
    ]
    const result = flattenBlockTree(blocks, null, new Set())
    expect(result).toHaveLength(4)
    expect(result.map(r => r.depth)).toEqual([0, 1, 2, 3])
  })

  it('handles multiple root blocks with their own subtrees', () => {
    const blocks = [
      b('r1', null, 0),
      b('r1_c1', 'r1', 0),
      b('r1_c2', 'r1', 1),
      b('r2', null, 1),
      b('r2_c1', 'r2', 0),
    ]
    const result = flattenBlockTree(blocks, null, new Set())
    expect(result).toHaveLength(5)
    expect(result[0].depth).toBe(0) // r1
    expect(result[1].depth).toBe(1) // r1_c1
    expect(result[2].depth).toBe(1) // r1_c2
    expect(result[3].depth).toBe(0) // r2
    expect(result[4].depth).toBe(1) // r2_c1
  })

  // ── Additional edge cases ─────────────────────────────

  it('returns empty when all blocks are comments', () => {
    const blocks = [
      b('c1', null, 0, {
        properties: [{ key: 'type', value: 'comment', type: 'string' }],
      }),
    ]
    const result = flattenBlockTree(blocks, null, new Set())
    expect(result).toHaveLength(0)
  })

  it('does not recurse into children of collapsed parent that itself has no children', () => {
    const blocks = [b('leaf', null, 0)]
    const collapsed = new Set(['leaf'])
    const result = flattenBlockTree(blocks, null, collapsed)
    expect(result).toHaveLength(1) // leaf shown, no children to hide
  })

  it('collapsed root hides its subtree but sibling subtrees are shown', () => {
    const blocks = [
      b('a', null, 0, { collapsed: true }),
      b('a1', 'a', 0),
      b('b', null, 1),
      b('b1', 'b', 0),
    ]
    const collapsed = new Set(['a'])
    const result = flattenBlockTree(blocks, null, collapsed)
    const ids = result.map(r => r.block.id)
    expect(ids).toEqual(['a', 'b', 'b1'])
  })

  it('mixed regular and comment children', () => {
    const blocks = [
      b('root', null, 0),
      b('regular', 'root', 0),
      b('comment', 'root', 1, {
        properties: [{ key: 'type', value: 'comment', type: 'string' }],
      }),
    ]
    const result = flattenBlockTree(blocks, null, new Set())
    // root + regular, comment filtered out
    expect(result).toHaveLength(2)
    expect(result[1].block.id).toBe('regular')
  })

  it('collapsed parent with only comment children shows hasChildren=false', () => {
    const blocks = [
      b('root', null, 0),
      b('comment1', 'root', 0, {
        properties: [{ key: 'type', value: 'comment', type: 'string' }],
      }),
    ]
    const result = flattenBlockTree(blocks, null, new Set())
    expect(result[0].hasChildren).toBe(false)
  })
})
