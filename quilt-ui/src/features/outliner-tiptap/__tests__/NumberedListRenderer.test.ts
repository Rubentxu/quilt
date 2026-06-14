import { describe, it, expect } from 'vitest'
import type { Block } from '@shared/types/api'
import type { BlockRendererContext } from '../rendering/types'
import { NumberedListRenderer } from '../rendering/NumberedListRenderer'

function makeBlock(overrides: Partial<Block> = {}): Block {
  return {
    id: 'b1',
    pageName: 'test',
    parentId: null,
    order: 0,
    level: 0,
    content: '',
    blockType: 'numbered',
    marker: null,
    priority: null,
    collapsed: false,
    properties: [],
    createdAt: '',
    updatedAt: '',
    ...overrides,
  } as Block
}

function makeCtx(overrides: Partial<BlockRendererContext> = {}): BlockRendererContext {
  return {
    block: makeBlock(),
    strategy: 'default',
    onUpdate: () => {},
    onCycleMarker: () => {},
    ...overrides,
  }
}

describe('NumberedListRenderer', () => {
  it('matches blocks with blockType numbered', () => {
    expect(NumberedListRenderer.match(makeBlock({ blockType: 'numbered' }), 'default')).toBe(true)
  })

  it('does NOT match paragraph blocks', () => {
    expect(NumberedListRenderer.match(makeBlock({ blockType: 'paragraph' }), 'default')).toBe(false)
  })

  it('renders "1." for a single numbered block', () => {
    const ctx = makeCtx({
      block: makeBlock({ id: 'b1', parentId: 'p1', blockType: 'numbered' }),
      allBlocks: [makeBlock({ id: 'b1', parentId: 'p1', blockType: 'numbered' })],
    })
    const result = NumberedListRenderer.renderBullet!(ctx, null)
    // The result is a React element — check its children (the text content)
    expect(result).not.toBeNull()
    // @ts-expect-error — accessing props of React element
    expect(result?.props?.children).toBe('1.')
  })

  it('renders sequential numbers for consecutive numbered siblings', () => {
    const blocks = [
      makeBlock({ id: 'b1', parentId: 'p1', blockType: 'numbered', order: 0 }),
      makeBlock({ id: 'b2', parentId: 'p1', blockType: 'numbered', order: 1 }),
      makeBlock({ id: 'b3', parentId: 'p1', blockType: 'numbered', order: 2 }),
    ]

    const ctx1 = makeCtx({ block: blocks[0], allBlocks: blocks })
    const ctx2 = makeCtx({ block: blocks[1], allBlocks: blocks })
    const ctx3 = makeCtx({ block: blocks[2], allBlocks: blocks })

    // @ts-expect-error
    expect(NumberedListRenderer.renderBullet!(ctx1, null)?.props?.children).toBe('1.')
    // @ts-expect-error
    expect(NumberedListRenderer.renderBullet!(ctx2, null)?.props?.children).toBe('2.')
    // @ts-expect-error
    expect(NumberedListRenderer.renderBullet!(ctx3, null)?.props?.children).toBe('3.')
  })

  it('resets numbering when a non-numbered block interrupts the sequence', () => {
    const blocks = [
      makeBlock({ id: 'b1', parentId: 'p1', blockType: 'numbered', order: 0 }),
      makeBlock({ id: 'b2', parentId: 'p1', blockType: 'paragraph', order: 1 }),
      makeBlock({ id: 'b3', parentId: 'p1', blockType: 'numbered', order: 2 }),
    ]

    const ctx3 = makeCtx({ block: blocks[2], allBlocks: blocks })
    // b3 should be "1." because the paragraph b2 breaks the sequence
    // @ts-expect-error
    expect(NumberedListRenderer.renderBullet!(ctx3, null)?.props?.children).toBe('1.')
  })

  it('numbers independently per parent (different parentId)', () => {
    const blocks = [
      makeBlock({ id: 'b1', parentId: 'p1', blockType: 'numbered', order: 0 }),
      makeBlock({ id: 'b2', parentId: 'p2', blockType: 'numbered', order: 0 }),
    ]

    const ctx2 = makeCtx({ block: blocks[1], allBlocks: blocks })
    // b2 has different parent, so it's "1." not "2."
    // @ts-expect-error
    expect(NumberedListRenderer.renderBullet!(ctx2, null)?.props?.children).toBe('1.')
  })

  it('falls back to 1 when allBlocks is undefined', () => {
    const ctx = makeCtx({ allBlocks: undefined })
    // @ts-expect-error
    expect(NumberedListRenderer.renderBullet!(ctx, null)?.props?.children).toBe('1.')
  })
})
