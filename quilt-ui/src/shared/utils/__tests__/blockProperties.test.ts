/**
 * Tests for blockProperties utilities — pure functions that transform
 * block property data between backend maps and frontend BlockProperty arrays.
 */
import { describe, it, expect } from 'vitest'
import {
  blockPropertiesFromMap,
  getBlockProperty,
  findCommentChildren,
  buildCommentTree,
  isCommentBlock,
} from '@shared/utils/blockProperties'
import type { Block, BlockProperty } from '@shared/types/api'

// ── blockPropertiesFromMap ───────────────────────────────────

describe('blockPropertiesFromMap', () => {
  it('returns empty array for null', () => {
    expect(blockPropertiesFromMap(null)).toEqual([])
  })

  it('returns empty array for undefined', () => {
    expect(blockPropertiesFromMap(undefined)).toEqual([])
  })

  it('returns empty array for empty object', () => {
    expect(blockPropertiesFromMap({})).toEqual([])
  })

  it('converts string values', () => {
    const result = blockPropertiesFromMap({ status: 'draft' })
    expect(result).toEqual([
      { key: 'status', value: 'draft', type: 'string' },
    ])
  })

  it('converts boolean values', () => {
    const result = blockPropertiesFromMap({ active: true })
    expect(result).toEqual([
      { key: 'active', value: true, type: 'boolean' },
    ])
  })

  it('converts number values', () => {
    const result = blockPropertiesFromMap({ count: 42 })
    expect(result).toEqual([
      { key: 'count', value: 42, type: 'number' },
    ])
  })

  it('converts array values to select type with JSON string', () => {
    const result = blockPropertiesFromMap({ tags: ['a', 'b'] })
    expect(result).toEqual([
      { key: 'tags', value: '["a","b"]', type: 'select' },
    ])
  })

  it('converts null values', () => {
    const result = blockPropertiesFromMap({ deleted: null })
    expect(result).toEqual([
      { key: 'deleted', value: null, type: 'string' },
    ])
  })

  it('converts multiple properties at once', () => {
    const result = blockPropertiesFromMap({
      title: 'Hello',
      priority: 1,
      done: false,
    })
    expect(result).toHaveLength(3)
    expect(result).toEqual(
      expect.arrayContaining([
        { key: 'title', value: 'Hello', type: 'string' },
        { key: 'priority', value: 1, type: 'number' },
        { key: 'done', value: false, type: 'boolean' },
      ]),
    )
  })

  it('falls back to string for object values (serialised as JSON)', () => {
    const result = blockPropertiesFromMap({ meta: { version: 2 } })
    expect(result).toEqual([
      { key: 'meta', value: '{"version":2}', type: 'string' },
    ])
  })
})

// ── getBlockProperty ─────────────────────────────────────────

describe('getBlockProperty', () => {
  const props: BlockProperty[] = [
    { key: 'status', value: 'draft', type: 'string' },
    { key: 'priority', value: 1, type: 'number' },
  ]

  it('returns the value for an existing key', () => {
    expect(getBlockProperty(props, 'status')).toBe('draft')
    expect(getBlockProperty(props, 'priority')).toBe(1)
  })

  it('returns undefined for a missing key', () => {
    expect(getBlockProperty(props, 'nonexistent')).toBeUndefined()
  })

  it('returns undefined when properties is undefined', () => {
    expect(getBlockProperty(undefined, 'status')).toBeUndefined()
  })

  it('returns undefined when properties is empty', () => {
    expect(getBlockProperty([], 'status')).toBeUndefined()
  })
})

// ── Helpers to build test blocks ─────────────────────────────

function makeBlock(overrides: Partial<Block> = {}): Block {
  return {
    id: 'b1',
    pageId: 'p1',
    pageName: null,
    content: '',
    blockType: 'paragraph',
    marker: null,
    priority: null,
    parentId: null,
    order: 0,
    level: 1,
    collapsed: false,
    properties: [],
    createdAt: '2026-01-01',
    updatedAt: '2026-01-01',
    ...overrides,
  }
}

// ── findCommentChildren ──────────────────────────────────────

describe('findCommentChildren', () => {
  it('returns empty array when no comments exist', () => {
    const blocks: Block[] = [makeBlock({ id: 'b1' }), makeBlock({ id: 'b2' })]
    expect(findCommentChildren(blocks, 'b1')).toEqual([])
  })

  it('returns blocks with type=comment property that are children of the given id', () => {
    const blocks: Block[] = [
      makeBlock({ id: 'parent' }),
      makeBlock({
        id: 'comment1',
        parentId: 'parent',
        properties: [{ key: 'type', value: 'comment', type: 'string' }],
      }),
      makeBlock({
        id: 'comment2',
        parentId: 'parent',
        properties: [{ key: 'type', value: 'comment', type: 'string' }],
      }),
      // Not a child of parent
      makeBlock({
        id: 'other',
        parentId: 'otherParent',
        properties: [{ key: 'type', value: 'comment', type: 'string' }],
      }),
    ]
    const result = findCommentChildren(blocks, 'parent')
    expect(result).toHaveLength(2)
    expect(result.map(b => b.id)).toEqual(['comment1', 'comment2'])
  })

  it('excludes children that are not comments', () => {
    const blocks: Block[] = [
      makeBlock({ id: 'parent' }),
      makeBlock({
        id: 'comment',
        parentId: 'parent',
        properties: [{ key: 'type', value: 'comment', type: 'string' }],
      }),
      makeBlock({ id: 'regular', parentId: 'parent' }),
    ]
    const result = findCommentChildren(blocks, 'parent')
    expect(result).toHaveLength(1)
    expect(result[0].id).toBe('comment')
  })
})

// ── buildCommentTree ─────────────────────────────────────────

describe('buildCommentTree', () => {
  it('returns empty array for block with no comments', () => {
    const blocks: Block[] = [makeBlock({ id: 'b1' })]
    expect(buildCommentTree(blocks, 'b1')).toEqual([])
  })

  it('builds a single-level comment tree', () => {
    const blocks: Block[] = [
      makeBlock({ id: 'parent' }),
      makeBlock({
        id: 'c1',
        parentId: 'parent',
        order: 0,
        properties: [{ key: 'type', value: 'comment', type: 'string' }],
      }),
    ]
    const tree = buildCommentTree(blocks, 'parent')
    expect(tree).toHaveLength(1)
    expect(tree[0].comment.id).toBe('c1')
    expect(tree[0].replies).toEqual([])
  })

  it('builds nested comment replies', () => {
    const blocks: Block[] = [
      makeBlock({ id: 'parent' }),
      makeBlock({
        id: 'c1',
        parentId: 'parent',
        order: 0,
        properties: [{ key: 'type', value: 'comment', type: 'string' }],
      }),
      makeBlock({
        id: 'c2',
        parentId: 'c1',
        order: 0,
        properties: [{ key: 'type', value: 'comment', type: 'string' }],
      }),
    ]
    const tree = buildCommentTree(blocks, 'parent')
    expect(tree).toHaveLength(1)
    expect(tree[0].comment.id).toBe('c1')
    expect(tree[0].replies).toHaveLength(1)
    expect(tree[0].replies[0].comment.id).toBe('c2')
  })

  it('sorts comments by order', () => {
    const blocks: Block[] = [
      makeBlock({ id: 'parent' }),
      makeBlock({
        id: 'c2',
        parentId: 'parent',
        order: 2,
        properties: [{ key: 'type', value: 'comment', type: 'string' }],
      }),
      makeBlock({
        id: 'c1',
        parentId: 'parent',
        order: 1,
        properties: [{ key: 'type', value: 'comment', type: 'string' }],
      }),
    ]
    const tree = buildCommentTree(blocks, 'parent')
    expect(tree.map(c => c.comment.id)).toEqual(['c1', 'c2'])
  })
})

// ── isCommentBlock ───────────────────────────────────────────

describe('isCommentBlock', () => {
  it('returns true for a block with type=comment', () => {
    const block = makeBlock({
      properties: [{ key: 'type', value: 'comment', type: 'string' }],
    })
    expect(isCommentBlock(block)).toBe(true)
  })

  it('returns false for a block without type property', () => {
    expect(isCommentBlock(makeBlock())).toBe(false)
  })

  it('returns false for a block with type != comment', () => {
    const block = makeBlock({
      properties: [{ key: 'type', value: 'note', type: 'string' }],
    })
    expect(isCommentBlock(block)).toBe(false)
  })

  it('returns false for a block with empty properties array', () => {
    expect(isCommentBlock(makeBlock({ properties: [] }))).toBe(false)
  })
})
