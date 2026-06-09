import { describe, it, expect } from 'vitest'
import type { Block } from '@shared/types/api'
import {
  getInlinePropertyKeys,
  getPanelOnlyPropertyKeys,
  getPropertyTemplate,
  isPropertyKeyInTemplate,
  DEFAULT_PROPERTY_TEMPLATE,
} from '../propertyTemplate'

/**
 * Build a Block with the given properties array. Convenience helper
 * for the template tests so each `it` reads as a single intent.
 */
function makeBlock(properties: Array<{ key: string; value: string; type?: 'string' | 'select' | 'date' | 'boolean' }> = []): Block {
  return {
    id: 'b1',
    pageId: 'p1',
    pageName: 'demo',
    content: '',
    blockType: 'paragraph',
    marker: null,
    priority: null,
    parentId: null,
    order: 1,
    level: 0,
    collapsed: false,
    createdAt: '2026-06-01T00:00:00Z',
    updatedAt: '2026-06-01T00:00:00Z',
    properties: properties.map(p => ({
      key: p.key,
      value: p.value,
      type: p.type ?? 'string',
    })),
  } as Block
}

describe('propertyTemplate — default template', () => {
  it('returns a default template when no block type matches', () => {
    const tpl = getPropertyTemplate(makeBlock())
    expect(tpl.inline).toEqual([])
    // Default puts ALL keys in the panel (no panel-only restriction)
    expect(tpl.panelOnly).toEqual([])
  })

  it('exposes the DEFAULT_PROPERTY_TEMPLATE export for callers that need it', () => {
    expect(DEFAULT_PROPERTY_TEMPLATE).toBeDefined()
    expect(DEFAULT_PROPERTY_TEMPLATE.inline).toEqual([])
    expect(DEFAULT_PROPERTY_TEMPLATE.panelOnly).toEqual([])
  })
})

describe('propertyTemplate — task block type', () => {
  it('task blocks have status and priority inline', () => {
    const tpl = getPropertyTemplate(makeBlock([{ key: 'status', value: 'open' }]))
    // Default behaviour: every block type except "todo" should fall back
    // to the default. The task template kicks in only when the block
    // has the todo marker OR its `type:: todo` property — we encode
    // that via a `blockType` field on the Block object (or, if absent,
    // a `type::` property).
    const todoBlock: Block = { ...makeBlock(), blockType: 'todo' as Block['blockType'] }
    const todoTpl = getPropertyTemplate(todoBlock)
    expect(todoTpl.inline).toContain('status')
    expect(todoTpl.inline).toContain('priority')
    expect(todoTpl.inline).toContain('due')
  })

  it('task blocks keep dsl out of the inline area (panel-only)', () => {
    const todoBlock: Block = { ...makeBlock(), blockType: 'todo' as Block['blockType'] }
    const tpl = getPropertyTemplate(todoBlock)
    expect(tpl.panelOnly).toContain('dsl')
  })
})

describe('propertyTemplate — getInlinePropertyKeys', () => {
  it('returns the keys of the block\'s properties that match the inline template', () => {
    const todoBlock: Block = {
      ...makeBlock([
        { key: 'status', value: 'open' },
        { key: 'priority', value: 'A' },
        { key: 'notes', value: 'free-form' },
      ]),
      blockType: 'todo' as Block['blockType'],
    }
    expect(getInlinePropertyKeys(todoBlock).sort()).toEqual(['priority', 'status'])
  })

  it('returns an empty array for a block with no inline properties', () => {
    const block = makeBlock([{ key: 'foo', value: 'bar' }])
    expect(getInlinePropertyKeys(block)).toEqual([])
  })
})

describe('propertyTemplate — getPanelOnlyPropertyKeys', () => {
  it('returns the keys that are panel-only AND present on the block', () => {
    // dsl must be in the block's properties for the helper to surface
    // it — the helper is "what should I hide behind the panel?" not
    // "what does the template forbid inline?".
    const todoBlock: Block = {
      ...makeBlock([{ key: 'dsl', value: 'and [[x]]', type: 'string' }]),
      blockType: 'todo' as Block['blockType'],
    }
    const keys = getPanelOnlyPropertyKeys(todoBlock)
    expect(keys).toContain('dsl')
  })

  it('returns an empty array when the block has no panel-only keys', () => {
    const todoBlock: Block = {
      ...makeBlock([{ key: 'notes', value: 'free-form' }]),
      blockType: 'todo' as Block['blockType'],
    }
    expect(getPanelOnlyPropertyKeys(todoBlock)).toEqual([])
  })
})

describe('propertyTemplate — isPropertyKeyInTemplate', () => {
  it('returns true when the key is in the inline list', () => {
    const todoBlock: Block = { ...makeBlock(), blockType: 'todo' as Block['blockType'] }
    expect(isPropertyKeyInTemplate(todoBlock, 'status', 'inline')).toBe(true)
    expect(isPropertyKeyInTemplate(todoBlock, 'priority', 'inline')).toBe(true)
  })

  it('returns false when the key is not in the inline list', () => {
    const todoBlock: Block = { ...makeBlock(), blockType: 'todo' as Block['blockType'] }
    expect(isPropertyKeyInTemplate(todoBlock, 'notes', 'inline')).toBe(false)
  })
})
