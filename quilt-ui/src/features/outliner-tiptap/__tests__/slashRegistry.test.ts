// ─── SlashActionRegistry — quilt-architecture-review-c4-slash-registry ───
//
// Unit tests for the slash-command action registry. The registry
// replaces the legacy `SLASH_MENU_ITEMS` array + 100-line switch on
// `item.action.split(':')` with a self-describing map where each
// action carries its own metadata AND its own handler.
//
// Coverage map (10 tests):
//   T1  Registry basics — register + getItem + getHandler
//   T2  unknown id → undefined (item and handler)
//   T3  allItems() returns registered items in registration order
//   T4  re-registering same id replaces entry (no duplicate ids)
//   T5  default registry exposes 18+ items (one per legacy SLASH_MENU_ITEMS)
//   T6  default registry covers every action prefix (status, priority,
//        date, property, ref, template, comment) + blockType
//   T7  handler invocation — registered handler runs with the supplied ctx
//   T8  defaultBlockTypeHandler updates the block via api
//   T9  defaultCommentHandler delegates to ctx.onAddComment
//   T10 SLASH_MENU_ITEMS (legacy) is now derived from the registry
//        and stays in lock-step with registry.allItems()
//
// All tests are pure (no jsdom, no React) — the registry is a plain
// TypeScript module that the React layer adapts via a SlashContext.

import { describe, it, expect, vi } from 'vitest'
import {
  SlashActionRegistry,
  defaultRegistry,
  defaultBlockTypeHandler,
  defaultCommentHandler,
  type SlashContext,
  type SlashHandler,
} from '../slashRegistry'
import { SLASH_MENU_ITEMS } from '../SlashCommandMenu'
import type { Block } from '@shared/types/api'

// ─── Test fixtures ────────────────────────────────────────────────────

function makeBlock(overrides: Partial<Block> = {}): Block {
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
    createdAt: '2026-06-02T00:00:00Z',
    updatedAt: '2026-06-02T00:00:00Z',
    properties: [],
    ...overrides,
  } as Block
}

function makeCtx(overrides: Partial<SlashContext> = {}): SlashContext {
  return {
    block: makeBlock(),
    allBlocks: [],
    api: {} as any,
    toast: { error: vi.fn(), success: vi.fn() } as any,
    navigate: vi.fn() as any,
    setContent: vi.fn(),
    setContentAtEnd: vi.fn(),
    onUpdate: vi.fn(),
    originalContent: '',
    onAddComment: undefined,
    ...overrides,
  }
}

// ─── T1: Registry basics ──────────────────────────────────────────────

describe('SlashActionRegistry — basics (T1)', () => {
  it('register + getItem + getHandler return the stored values', () => {
    const reg = new SlashActionRegistry()
    const item = {
      id: 'demo-1',
      label: 'Demo',
      description: 'Demo action',
      icon: null,
      keywords: ['demo'],
      category: 'Test',
    }
    const handler: SlashHandler = vi.fn()
    reg.register(item, handler)

    expect(reg.getItem('demo-1')).toBe(item)
    expect(reg.getHandler('demo-1')).toBe(handler)
  })

  // T2
  it('unknown id → getItem returns undefined, getHandler returns undefined', () => {
    const reg = new SlashActionRegistry()
    expect(reg.getItem('does-not-exist')).toBeUndefined()
    expect(reg.getHandler('does-not-exist')).toBeUndefined()
  })

  // T3
  it('allItems() returns registered items in registration order', () => {
    const reg = new SlashActionRegistry()
    const a = { id: 'a', label: 'A', description: '', icon: null, keywords: [], category: 'X' }
    const b = { id: 'b', label: 'B', description: '', icon: null, keywords: [], category: 'X' }
    const c = { id: 'c', label: 'C', description: '', icon: null, keywords: [], category: 'X' }
    reg.register(a, vi.fn())
    reg.register(b, vi.fn())
    reg.register(c, vi.fn())

    expect(reg.allItems().map(i => i.id)).toEqual(['a', 'b', 'c'])
  })

  // T4
  it('re-registering the same id replaces the previous entry', () => {
    const reg = new SlashActionRegistry()
    const v1 = { id: 'x', label: 'v1', description: '', icon: null, keywords: [], category: 'X' }
    const v2 = { id: 'x', label: 'v2', description: '', icon: null, keywords: [], category: 'X' }
    const h1 = vi.fn()
    const h2 = vi.fn()
    reg.register(v1, h1)
    reg.register(v2, h2)

    expect(reg.allItems()).toHaveLength(1)
    expect(reg.getItem('x')?.label).toBe('v2')
    expect(reg.getHandler('x')).toBe(h2)
  })
})

// ─── T5 + T6: Default registry content ────────────────────────────────

describe('SlashActionRegistry — default registry coverage (T5, T6)', () => {
  // T5
  it('default registry has 18+ items (parity with legacy SLASH_MENU_ITEMS)', () => {
    expect(defaultRegistry.allItems().length).toBeGreaterThanOrEqual(18)
  })

  // T6 — every action prefix used by the legacy switch must be present
  it('default registry covers every legacy action prefix + blockType', () => {
    const ids = defaultRegistry.allItems().map(i => i.id)

    // Status (6)
    for (const id of ['status-todo', 'status-doing', 'status-done', 'status-now', 'status-later', 'status-cancelled']) {
      expect(ids, `missing status item: ${id}`).toContain(id)
    }
    // Priority (3)
    for (const id of ['priority-a', 'priority-b', 'priority-c']) {
      expect(ids, `missing priority item: ${id}`).toContain(id)
    }
    // Dates (2)
    for (const id of ['date-today', 'date-tomorrow']) {
      expect(ids, `missing date item: ${id}`).toContain(id)
    }
    // Properties (2)
    for (const id of ['prop-deadline', 'prop-scheduled']) {
      expect(ids, `missing property item: ${id}`).toContain(id)
    }
    // References (2)
    for (const id of ['ref-page', 'ref-block']) {
      expect(ids, `missing ref item: ${id}`).toContain(id)
    }
    // Template (1)
    expect(ids).toContain('insert-template')
    // Comment (1)
    expect(ids).toContain('add-comment')
    // Block types (11)
    for (const id of ['paragraph', 'heading1', 'heading2', 'heading3', 'bullet', 'numbered', 'todo', 'quote', 'code', 'divider', 'image']) {
      expect(ids, `missing block-type item: ${id}`).toContain(id)
    }
  })

  it('every default item has a non-undefined handler', () => {
    // The switch never had a no-op branch; if a default item is missing
    // a handler, the slash menu becomes decorative. Catch it here.
    for (const item of defaultRegistry.allItems()) {
      expect(defaultRegistry.getHandler(item.id), `missing handler for ${item.id}`).toBeDefined()
    }
  })
})

// ─── T7 + T8 + T9: Handler behaviour ──────────────────────────────────

describe('SlashActionRegistry — handler behaviour (T7, T8, T9)', () => {
  // T7
  it('registered handler runs with the supplied SlashContext', () => {
    const reg = new SlashActionRegistry()
    const handler = vi.fn()
    reg.register(
      { id: 'spy', label: 'Spy', description: '', icon: null, keywords: [], category: 'Test' },
      handler,
    )
    const ctx = makeCtx()
    reg.getHandler('spy')!(ctx)
    expect(handler).toHaveBeenCalledWith(ctx)
  })

  // T8 — the builtin block-type handler must call api.updateBlock with the
  // item's blockType and surface errors via toast.
  it('defaultBlockTypeHandler updates the block and surfaces errors via toast', async () => {
    const updateBlock = vi.fn().mockResolvedValue({ id: 'b1', blockType: 'heading1' })
    const onUpdate = vi.fn()
    const toast = { error: vi.fn(), success: vi.fn() }
    const ctx = makeCtx({
      block: makeBlock({ id: 'b7' }),
      api: { updateBlock } as any,
      toast: toast as any,
      onUpdate,
    })
    const item = {
      id: 'heading1',
      label: 'Heading 1',
      description: '',
      icon: null,
      keywords: [],
      category: 'Block Types',
      blockType: 'heading1',
    }
    await defaultBlockTypeHandler(ctx, item)
    expect(updateBlock).toHaveBeenCalledWith('b7', { blockType: 'heading1' })
    expect(onUpdate).toHaveBeenCalled()

    // Error path
    updateBlock.mockRejectedValueOnce(new Error('boom'))
    await defaultBlockTypeHandler(ctx, item)
    expect(toast.error).toHaveBeenCalledWith('Failed to change block type')
  })

  // T9 — the builtin comment handler delegates to ctx.onAddComment, or
  // shows a toast if no callback is wired.
  it('defaultCommentHandler delegates to ctx.onAddComment; falls back to toast', () => {
    const onAddComment = vi.fn()
    const ctx = makeCtx({ onAddComment })
    const item = {
      id: 'add-comment',
      label: 'Add Comment',
      description: '',
      icon: null,
      keywords: [],
      category: 'Actions',
    }
    defaultCommentHandler(ctx, item)
    expect(onAddComment).toHaveBeenCalledWith('b1')

    // Fallback when onAddComment is missing
    const toast = { error: vi.fn(), success: vi.fn() }
    const noCbCtx = makeCtx({ onAddComment: undefined, toast: toast as any })
    defaultCommentHandler(noCbCtx, item)
    expect(toast.error).toHaveBeenCalledWith('Comment callback not available')
  })
})

// ─── T10: SLASH_MENU_ITEMS is derived from the registry ───────────────

describe('SlashCommandMenu — SLASH_MENU_ITEMS derives from the registry (T10)', () => {
  it('legacy SLASH_MENU_ITEMS ids match defaultRegistry.allItems() ids in order', () => {
    // The legacy constant is kept for backwards compat (SlashTemplateFlow
    // and other tests import it). It must stay in lock-step with the
    // registry so neither can drift.
    const legacyIds = SLASH_MENU_ITEMS.map(i => i.id)
    const registryIds = defaultRegistry.allItems().map(i => i.id)
    expect(legacyIds).toEqual(registryIds)
  })
})
