// ─── ensureTaskShape — unit tests ───────────────────────────────────────────
//
// Tests for the one-way blockType→todo conversion when a marker is set.
// Per ADR-0023 deviation (documented in ADR-0025): setting ANY marker on a
// paragraph/bullet/numbered/heading block converts blockType to 'todo' in
// a single update. This is intentionally NOT symmetrical — clearing the
// marker does NOT revert blockType back to the original type.

import { describe, it, expect } from 'vitest'
import { ensureTaskShape, NON_TASK_BLOCK_TYPES } from '../ensureTaskShape'
import type { Block, TaskMarker } from '@shared/types/api'

// ─── Fixtures ──────────────────────────────────────────────────────────────

function makeBlock(overrides: Partial<Block> = {}): Block {
  return {
    id: 'b1',
    pageId: 'p1',
    pageName: 'demo',
    content: 'test block',
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

const ALL_TASK_MARKERS: TaskMarker[] = ['Now', 'Later', 'Todo', 'Doing', 'Done', 'Cancelled', 'Waiting']

// ─── Tests ─────────────────────────────────────────────────────────────────

describe('ensureTaskShape — block-to-task one-way conversion', () => {

  describe('paragraph block with non-null marker → converts to todo', () => {
    for (const marker of ALL_TASK_MARKERS) {
      it(`paragraph + ${marker} → blockType:'todo', marker:'${marker}'`, () => {
        const block = makeBlock({ blockType: 'paragraph', marker: null })
        const result = ensureTaskShape(block, marker)
        expect(result).toEqual({ blockType: 'todo', marker })
      })
    }
  })

  describe('bullet block with non-null marker → converts to todo', () => {
    for (const marker of ALL_TASK_MARKERS) {
      it(`bullet + ${marker} → blockType:'todo', marker:'${marker}'`, () => {
        const block = makeBlock({ blockType: 'bullet', marker: null })
        const result = ensureTaskShape(block, marker)
        expect(result).toEqual({ blockType: 'todo', marker })
      })
    }
  })

  describe('numbered block with non-null marker → converts to todo', () => {
    for (const marker of ALL_TASK_MARKERS) {
      it(`numbered + ${marker} → blockType:'todo', marker:'${marker}'`, () => {
        const block = makeBlock({ blockType: 'numbered', marker: null })
        const result = ensureTaskShape(block, marker)
        expect(result).toEqual({ blockType: 'todo', marker })
      })
    }
  })

  describe('heading blocks with non-null marker → converts to todo (ADR-0025 deviation)', () => {
    for (const heading of ['heading1', 'heading2', 'heading3'] as const) {
      for (const marker of ALL_TASK_MARKERS) {
        it(`${heading} + ${marker} → blockType:'todo', marker:'${marker}'`, () => {
          const block = makeBlock({ blockType: heading, marker: null })
          const result = ensureTaskShape(block, marker)
          expect(result).toEqual({ blockType: 'todo', marker })
        })
      }
    }
  })

  describe('non-convertible block types — marker updates but blockType is unchanged', () => {
    for (const blockType of NON_TASK_BLOCK_TYPES) {
      for (const marker of ALL_TASK_MARKERS) {
        it(`${blockType} + ${marker} → only marker update`, () => {
          const block = makeBlock({ blockType, marker: null })
          const result = ensureTaskShape(block, marker)
          expect(result).toEqual({ marker })
          expect(result).not.toHaveProperty('blockType')
        })
      }
    }
  })

  describe('null marker — one-way: blockType stays as-is (ADR-0025 deviation)', () => {
    for (const blockType of ['paragraph', 'bullet', 'numbered', 'heading1', 'heading2', 'heading3'] as const) {
      it(`${blockType} + null → only marker:null, blockType unchanged`, () => {
        const block = makeBlock({ blockType, marker: 'Todo' })
        const result = ensureTaskShape(block, null)
        expect(result).toEqual({ marker: null })
        expect(result).not.toHaveProperty('blockType')
      })
    }
  })

  describe('todo block + non-null marker — just marker update, no blockType change', () => {
    for (const marker of ALL_TASK_MARKERS) {
      it(`todo + ${marker} → only marker update`, () => {
        const block = makeBlock({ blockType: 'todo', marker: null })
        const result = ensureTaskShape(block, marker)
        expect(result).toEqual({ marker })
        expect(result).not.toHaveProperty('blockType')
      })
    }
  })

  describe('todo block + null marker — stays todo (one-way, not reverted)', () => {
    it('todo + null → only marker:null', () => {
      const block = makeBlock({ blockType: 'todo', marker: 'Todo' })
      const result = ensureTaskShape(block, null)
      expect(result).toEqual({ marker: null })
      expect(result).not.toHaveProperty('blockType')
    })
  })

  describe('NON_TASK_BLOCK_TYPES constant includes all non-convertible types', () => {
    it('includes code, quote, divider, image, todo', () => {
      expect(NON_TASK_BLOCK_TYPES).toContain('code')
      expect(NON_TASK_BLOCK_TYPES).toContain('quote')
      expect(NON_TASK_BLOCK_TYPES).toContain('divider')
      expect(NON_TASK_BLOCK_TYPES).toContain('image')
      expect(NON_TASK_BLOCK_TYPES).toContain('todo')
    })
  })
})
