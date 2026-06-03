import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import type { BlockCard } from '../CardRenderer'
import { getBlockCard, getCardMetas, buildTemplateIndex } from '../blockCard'
import type { Block, BlockProperty, Page } from '@shared/types/api'

// ──── Fixtures ──────────────────────────────────────────────────────

function block(props: BlockProperty[]): Block {
  return {
    id: 'b1',
    pageId: 'p1',
    pageName: 'somename',
    content: 'Hello',
    blockType: 'paragraph',
    marker: null,
    priority: null,
    parentId: null,
    order: 1,
    level: 0,
    collapsed: false,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    properties: props,
  }
}

function prop(key: string, value: string | number | boolean): BlockProperty {
  const type: 'string' | 'number' | 'boolean' =
    typeof value === 'string' ? 'string' : typeof value === 'number' ? 'number' : 'boolean'
  return { key, value: value as any, type }
}

const refTemplate: BlockCard = {
  shape: 'reference',
  icon: '🔗',
  templateName: 'reference',
}
const docTemplate: BlockCard = {
  shape: 'content',
  icon: '📄',
  templateName: 'documentation',
}
const customTemplate: BlockCard = {
  shape: 'inline',
  icon: '🎯',
  cssclass: 'my-class',
  templateName: 'meeting-notes',
}

const templates = new Map<string, BlockCard>([
  ['reference', refTemplate],
  ['documentation', docTemplate],
  ['meeting-notes', customTemplate],
])

// Suppress expected console.warn calls during the test run
beforeEach(() => {
  vi.spyOn(console, 'warn').mockImplementation(() => {})
})
afterEach(() => {
  vi.restoreAllMocks()
})

// ──── getBlockCard ──────────────────────────────────────────────────

describe('getBlockCard (ADR-0007)', () => {
  it('returns null when block has no properties', () => {
    expect(getBlockCard(block([]), templates)).toBeNull()
  })

  it('returns null when block has unrelated properties', () => {
    const b = block([prop('priority', 'A'), prop('deadline', '2026-01-15')])
    expect(getBlockCard(b, templates)).toBeNull()
  })

  // Primary path: `template::` activation
  it('returns the template card when `template::` matches an index entry', () => {
    const b = block([prop('template', 'reference')])
    expect(getBlockCard(b, templates)).toEqual(refTemplate)
  })

  it('returns the documentation template card for `template:: documentation`', () => {
    const b = block([prop('template', 'documentation')])
    expect(getBlockCard(b, templates)).toEqual(docTemplate)
  })

  it('returns the inline template card with cssclass preserved', () => {
    const b = block([prop('template', 'meeting-notes')])
    expect(getBlockCard(b, templates)).toEqual(customTemplate)
  })

  it('returns null and warns when `template::` references an unknown template', () => {
    const b = block([prop('template', 'no-such-template')])
    expect(getBlockCard(b, templates)).toBeNull()
    expect(console.warn).toHaveBeenCalledWith(
      expect.stringContaining('no-such-template'),
    )
  })

  it('returns null when the templates index is empty (still has `template::`)', () => {
    const b = block([prop('template', 'reference')])
    expect(getBlockCard(b, new Map())).toBeNull()
  })

  // Legacy fallback: `type::` activation (V1 transitional)
  it('falls back to reference shape for legacy `type:: reference`', () => {
    const b = block([prop('type', 'reference')])
    const card = getBlockCard(b, templates)
    expect(card?.shape).toBe('reference')
    expect(card?.templateName).toBe('reference')
    expect(console.warn).toHaveBeenCalledWith(
      expect.stringContaining('legacy "type:: reference"'),
    )
  })

  it('falls back to content shape for legacy `type:: documentacion`', () => {
    const b = block([prop('type', 'documentacion')])
    const card = getBlockCard(b, templates)
    expect(card?.shape).toBe('content')
    expect(card?.templateName).toBe('documentation')
  })

  it('falls back to content shape for legacy `type:: documentation` (English spelling)', () => {
    const b = block([prop('type', 'documentation')])
    const card = getBlockCard(b, templates)
    expect(card?.shape).toBe('content')
  })

  it('returns null for unknown `type::` values (no fallback, no warn)', () => {
    const b = block([prop('type', 'paragraph')])
    expect(getBlockCard(b, templates)).toBeNull()
  })

  // `template::` takes priority over `type::`
  it('prefers `template::` over `type::` when both are present', () => {
    const b = block([
      prop('template', 'reference'),
      prop('type', 'documentacion'),
    ])
    const card = getBlockCard(b, templates)
    expect(card?.shape).toBe('reference')
    // No warn because `template::` resolved successfully
    expect(console.warn).not.toHaveBeenCalled()
  })
})

// ──── getCardMetas ──────────────────────────────────────────────────

describe('getCardMetas', () => {
  it('returns empty array when block has no properties', () => {
    expect(getCardMetas(block([]))).toEqual([])
  })

  it('excludes reserved card keys (template, type, card-shape, icon, cssclass)', () => {
    const b = block([
      prop('template', 'reference'),
      prop('type', 'reference'), // legacy
      prop('card-shape', 'reference'),
      prop('icon', '🔗'),
      prop('cssclass', 'my-style'),
      prop('dda-relacionada', 'DDA v1'),
    ])
    expect(getCardMetas(b)).toEqual([{ key: 'dda-relacionada', value: 'DDA v1' }])
  })

  it('returns all non-reserved string properties as metas', () => {
    const b = block([
      prop('template', 'reference'),
      prop('dda-relacionada', 'DDA v1'),
      prop('fecha-creacion', '26-05-2026'),
      prop('author', 'claude'),
    ])
    expect(getCardMetas(b)).toEqual([
      { key: 'dda-relacionada', value: 'DDA v1' },
      { key: 'fecha-creacion', value: '26-05-2026' },
      { key: 'author', value: 'claude' },
    ])
  })

  it('skips non-string property values (numbers, booleans)', () => {
    const b = block([
      prop('priority', 1),
      prop('resolved', false),
      prop('note', 'kept'),
    ])
    expect(getCardMetas(b)).toEqual([{ key: 'note', value: 'kept' }])
  })

  it('skips the `collapsed` property (handled by the outliner, not the card)', () => {
    const b = block([
      prop('collapsed', 'true'),
      prop('note', 'visible'),
    ])
    expect(getCardMetas(b)).toEqual([{ key: 'note', value: 'visible' }])
  })
})

// ──── buildTemplateIndex ────────────────────────────────────────────

describe('buildTemplateIndex', () => {
  it('indexes only pages whose name starts with `template/`', () => {
    const pages: Page[] = [
      { id: '1', name: 'regular-page', title: null } as any,
      { id: '2', name: 'template/reference', title: null } as any,
      { id: '3', name: 'template/documentation', title: null } as any,
      { id: '4', name: 'templated', title: null } as any, // not a template
    ]
    const propsByPageId = new Map<string, BlockProperty[]>([
      ['2', [prop('card-shape', 'reference')]],
      ['3', [prop('card-shape', 'content')]],
    ])
    const index = buildTemplateIndex(pages, propsByPageId)
    expect(index.size).toBe(2)
    expect(index.has('reference')).toBe(true)
    expect(index.has('documentation')).toBe(true)
    expect(index.has('regular-page')).toBe(false)
    expect(index.has('templated')).toBe(false)
  })

  it('uses the page name without the `template/` prefix as the index key', () => {
    const pages: Page[] = [
      { id: '1', name: 'template/meeting-notes', title: null } as any,
    ]
    const index = buildTemplateIndex(pages, new Map())
    expect(index.has('meeting-notes')).toBe(true)
  })

  it('falls back to inline shape when card-shape is missing or invalid', () => {
    const pages: Page[] = [
      { id: '1', name: 'template/loose', title: null } as any,
    ]
    const propsByPageId = new Map<string, BlockProperty[]>([
      ['1', [prop('card-shape', 'invalid-value')]],
    ])
    const index = buildTemplateIndex(pages, propsByPageId)
    expect(index.get('loose')?.shape).toBe('inline')
  })

  it('returns empty index when no template pages exist', () => {
    const pages: Page[] = [
      { id: '1', name: 'regular-page', title: null } as any,
    ]
    const index = buildTemplateIndex(pages, new Map())
    expect(index.size).toBe(0)
  })
})
