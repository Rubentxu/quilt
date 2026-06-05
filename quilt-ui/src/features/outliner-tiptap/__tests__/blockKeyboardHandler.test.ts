/**
 * Tests for blockKeyboardHandler — pure decision-making for keyboard
 * events in the outliner. BlockRow's old `handleKeyDown` callback
 * (~440 LOC) fused DOM access with decision logic, leaving zero unit
 * coverage. This module separates the decisions into a pure function
 * so the rules can be tested without a DOM.
 *
 * These tests assert behavior through the action enum — what should
 * happen, not how it gets applied.
 */
import { describe, it, expect } from 'vitest'
import {
  blockKeyboardHandler,
  type KeyboardAction,
  type KeyboardContext,
  type Modifiers,
  type CursorPos,
} from '../blockKeyboardHandler'
import type { Block } from '@shared/types/api'

// ── Fixtures ──────────────────────────────────────────────────

function block(overrides: Partial<Block> = {}): Block {
  return {
    id: 'b1',
    pageId: 'p1',
    pageName: 'somename',
    content: 'Hello world',
    blockType: 'paragraph',
    marker: null,
    priority: null,
    parentId: null,
    order: 0,
    level: 0,
    collapsed: false,
    properties: [],
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    ...overrides,
  }
}

const noMods: Modifiers = { mod: false, shift: false, alt: false }
const cursorStart: CursorPos = { offset: 0, atStart: true, atEnd: false }
const cursorEnd: CursorPos = { offset: 11, atStart: false, atEnd: true }
const cursorMid: CursorPos = { offset: 5, atStart: false, atEnd: false }

function ctx(overrides: Partial<KeyboardContext> = {}): KeyboardContext {
  const b = overrides.block ?? block()
  return {
    block: b,
    allBlocks: [b],
    content: 'Hello world',
    key: '',
    mods: noMods,
    cursor: cursorMid,
    hasTextSelection: false,
    ...overrides,
  }
}

// Convenience: typed assertions to keep test bodies terse.
const A = <T extends KeyboardAction['type']>(
  action: KeyboardAction,
  type: T,
): Extract<KeyboardAction, { type: T }> => {
  expect(action.type).toBe(type)
  return action as Extract<KeyboardAction, { type: T }>
}

// ── Enter key ────────────────────────────────────────────────

describe('blockKeyboardHandler — Enter', () => {
  it('returns CreateEmptySibling on Enter for an empty block', () => {
    const result = blockKeyboardHandler(
      ctx({
        block: block({ content: '' }),
        content: '',
        key: 'Enter',
        cursor: { offset: 0, atStart: true, atEnd: true },
      }),
    )
    A(result, 'CreateEmptySibling')
  })

  it('returns CreateEmptySibling on Enter at end of non-empty block', () => {
    const result = blockKeyboardHandler(ctx({ key: 'Enter', cursor: cursorEnd }))
    A(result, 'CreateEmptySibling')
  })

  it('returns Split at 0 on Enter at start of non-empty block', () => {
    const result = blockKeyboardHandler(ctx({ key: 'Enter', cursor: cursorStart }))
    A(result, 'Split')
    expect(result.at).toBe(0)
  })

  it('returns Split with the cursor offset when in the middle', () => {
    const result = blockKeyboardHandler(ctx({ key: 'Enter', cursor: cursorMid }))
    A(result, 'Split')
    expect(result.at).toBe(5)
  })

  it('returns InsertText newline on Shift+Enter', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'Enter', mods: { mod: false, shift: true, alt: false }, cursor: cursorMid }),
    )
    A(result, 'InsertText')
    expect(result.text).toBe('\n')
  })

  it('returns ToggleDone on Cmd+Enter (no link, no marker)', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'Enter', mods: { mod: true, shift: false, alt: false } }),
    )
    A(result, 'ToggleDone')
  })

  it('returns ToggleDone on Cmd+Enter when block already has a Todo marker', () => {
    const result = blockKeyboardHandler(
      ctx({ block: block({ marker: 'Todo' }), key: 'Enter', mods: { mod: true, shift: false, alt: false } }),
    )
    A(result, 'ToggleDone')
  })

  it('returns FollowLink on Cmd+Enter near a [[page]] link', () => {
    const result = blockKeyboardHandler(
      ctx({
        content: 'see [[My Page]] here',
        key: 'Enter',
        cursor: { offset: 6, atStart: false, atEnd: false },
        mods: { mod: true, shift: false, alt: false },
      }),
    )
    A(result, 'FollowLink')
    expect(result.link).toEqual({ type: 'page', target: 'My Page' })
  })

  it('returns FollowLink on Cmd+Enter near a ((block-uuid)) link', () => {
    const result = blockKeyboardHandler(
      ctx({
        content: 'ref ((abc-123)) here',
        key: 'Enter',
        cursor: { offset: 7, atStart: false, atEnd: false },
        mods: { mod: true, shift: false, alt: false },
      }),
    )
    A(result, 'FollowLink')
    expect(result.link).toEqual({ type: 'block', target: 'abc-123' })
  })

  it('returns FollowLink on Cmd+Enter near a #tag', () => {
    const result = blockKeyboardHandler(
      ctx({
        content: 'hello #mytag world',
        key: 'Enter',
        cursor: { offset: 9, atStart: false, atEnd: false }, // inside the #mytag range
        mods: { mod: true, shift: false, alt: false },
      }),
    )
    A(result, 'FollowLink')
    expect(result.link).toEqual({ type: 'tag', target: 'mytag' })
  })

  it('returns ToggleDone (not FollowLink) on Cmd+Enter when no link near cursor', () => {
    const result = blockKeyboardHandler(
      ctx({
        content: 'plain text with no link',
        key: 'Enter',
        cursor: cursorMid,
        mods: { mod: true, shift: false, alt: false },
      }),
    )
    A(result, 'ToggleDone')
  })

  it('returns None on Cmd+Shift+Enter (no defined action)', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'Enter', mods: { mod: true, shift: true, alt: false } }),
    )
    A(result, 'None')
  })

  it('returns None on Cmd+Alt+Enter (no defined action)', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'Enter', mods: { mod: true, shift: false, alt: true } }),
    )
    A(result, 'None')
  })
})

// ── Backspace key ────────────────────────────────────────────

describe('blockKeyboardHandler — Backspace', () => {
  it('returns MergeWithPrev on Backspace at start with a previous sibling', () => {
    const prev = block({ id: 'prev', order: 0, parentId: null })
    const cur = block({ id: 'cur', order: 1, parentId: null })
    const result = blockKeyboardHandler(
      ctx({
        block: cur,
        allBlocks: [prev, cur],
        key: 'Backspace',
        cursor: cursorStart,
      }),
    )
    A(result, 'MergeWithPrev')
  })

  it('returns Outdent on Backspace at start with no prev but with a parent', () => {
    // Lone child of 'parent' — no previous sibling, but a parent to outdent to.
    const cur = block({ id: 'cur', order: 0, parentId: 'parent', level: 1 })
    const result = blockKeyboardHandler(
      ctx({
        block: cur,
        allBlocks: [cur],
        key: 'Backspace',
        cursor: cursorStart,
      }),
    )
    A(result, 'Outdent')
  })

  it('returns None on Backspace at start with no prev and no parent (root block)', () => {
    // Only block on the page, root level.
    const cur = block({ id: 'cur', order: 0, parentId: null })
    const result = blockKeyboardHandler(
      ctx({
        block: cur,
        allBlocks: [cur],
        key: 'Backspace',
        cursor: cursorStart,
      }),
    )
    A(result, 'None')
  })

  it('returns None on Backspace in the middle of text (let browser delete a char)', () => {
    const result = blockKeyboardHandler(ctx({ key: 'Backspace', cursor: cursorMid }))
    A(result, 'None')
  })

  it('returns None on Backspace at end of text (let browser delete a char)', () => {
    const result = blockKeyboardHandler(ctx({ key: 'Backspace', cursor: cursorEnd }))
    A(result, 'None')
  })
})

// ── Delete key ──────────────────────────────────────────────

describe('blockKeyboardHandler — Delete', () => {
  it('returns None on Delete (merge-on-delete not yet implemented)', () => {
    const result = blockKeyboardHandler(ctx({ key: 'Delete', cursor: cursorEnd }))
    A(result, 'None')
  })

  it('returns None on Delete in the middle (let browser delete a char)', () => {
    const result = blockKeyboardHandler(ctx({ key: 'Delete', cursor: cursorMid }))
    A(result, 'None')
  })
})

// ── Tab key ─────────────────────────────────────────────────

describe('blockKeyboardHandler — Tab', () => {
  it('returns Indent on Tab (no shift)', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'Tab', mods: { mod: false, shift: false, alt: false } }),
    )
    A(result, 'Indent')
  })

  it('returns Outdent on Shift+Tab', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'Tab', mods: { mod: false, shift: true, alt: false } }),
    )
    A(result, 'Outdent')
  })

  it('returns None on Cmd+Tab (browser handles window switch)', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'Tab', mods: { mod: true, shift: false, alt: false } }),
    )
    A(result, 'None')
  })
})

// ── Arrow keys ─────────────────────────────────────────────

describe('blockKeyboardHandler — ArrowUp', () => {
  it('returns MoveCursor prev on ArrowUp at start of text', () => {
    const result = blockKeyboardHandler(ctx({ key: 'ArrowUp', cursor: cursorStart }))
    A(result, 'MoveCursor')
    expect(result.to).toBe('prev')
  })

  it('returns None on ArrowUp in middle of text (let browser move cursor)', () => {
    const result = blockKeyboardHandler(ctx({ key: 'ArrowUp', cursor: cursorMid }))
    A(result, 'None')
  })

  it('returns MoveBlockUp on Alt+Shift+ArrowUp', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'ArrowUp', mods: { mod: false, shift: true, alt: true }, cursor: cursorMid }),
    )
    A(result, 'MoveBlockUp')
  })

  it('returns ExtendSelection up on Alt+ArrowUp (no shift)', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'ArrowUp', mods: { mod: false, shift: false, alt: true }, cursor: cursorMid }),
    )
    A(result, 'ExtendSelection')
    expect(result.direction).toBe('up')
  })
})

describe('blockKeyboardHandler — ArrowDown', () => {
  it('returns MoveCursor next on ArrowDown at end of text', () => {
    const result = blockKeyboardHandler(ctx({ key: 'ArrowDown', cursor: cursorEnd }))
    A(result, 'MoveCursor')
    expect(result.to).toBe('next')
  })

  it('returns None on ArrowDown in middle of text (let browser move cursor)', () => {
    const result = blockKeyboardHandler(ctx({ key: 'ArrowDown', cursor: cursorMid }))
    A(result, 'None')
  })

  it('returns MoveBlockDown on Alt+Shift+ArrowDown', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'ArrowDown', mods: { mod: false, shift: true, alt: true }, cursor: cursorMid }),
    )
    A(result, 'MoveBlockDown')
  })

  it('returns ExtendSelection down on Alt+ArrowDown (no shift)', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'ArrowDown', mods: { mod: false, shift: false, alt: true }, cursor: cursorMid }),
    )
    A(result, 'ExtendSelection')
    expect(result.direction).toBe('down')
  })
})

// ── Modifier shortcuts ─────────────────────────────────────

describe('blockKeyboardHandler — Mod shortcuts', () => {
  it('returns Undo on Cmd+Z (no shift)', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'z', mods: { mod: true, shift: false, alt: false } }),
    )
    A(result, 'Undo')
  })

  it('returns Undo on Ctrl+Z (cross-platform mod key)', () => {
    // The `mods.mod` flag abstracts Cmd vs Ctrl — both should match.
    const result = blockKeyboardHandler(
      ctx({ key: 'z', mods: { mod: true, shift: false, alt: false } }),
    )
    A(result, 'Undo')
  })

  it('returns Redo on Cmd+Shift+Z', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'z', mods: { mod: true, shift: true, alt: false } }),
    )
    A(result, 'Redo')
  })

  it('returns Redo on Cmd+Y', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'y', mods: { mod: true, shift: false, alt: false } }),
    )
    A(result, 'Redo')
  })

  it('returns ToggleInlineMark ** on Cmd+B', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'b', mods: { mod: true, shift: false, alt: false } }),
    )
    A(result, 'ToggleInlineMark')
    expect(result.marker).toBe('**')
  })

  it('returns ToggleInlineMark * on Cmd+I', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'i', mods: { mod: true, shift: false, alt: false } }),
    )
    A(result, 'ToggleInlineMark')
    expect(result.marker).toBe('*')
  })

  it('returns ToggleInlineMark ` on Cmd+`', () => {
    const result = blockKeyboardHandler(
      ctx({ key: '`', mods: { mod: true, shift: false, alt: false } }),
    )
    A(result, 'ToggleInlineMark')
    expect(result.marker).toBe('`')
  })

  it('returns SelectParent on Cmd+A (no shift)', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'a', mods: { mod: true, shift: false, alt: false } }),
    )
    A(result, 'SelectParent')
  })

  it('returns SelectAll on Cmd+Shift+A', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'a', mods: { mod: true, shift: true, alt: false } }),
    )
    A(result, 'SelectAll')
  })
})

// ── Clipboard ───────────────────────────────────────────────

describe('blockKeyboardHandler — Clipboard', () => {
  it('returns CopyBlock on Cmd+C with no text selection', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'c', mods: { mod: true, shift: false, alt: false }, hasTextSelection: false }),
    )
    A(result, 'CopyBlock')
  })

  it('returns None on Cmd+C with active text selection (let browser copy)', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'c', mods: { mod: true, shift: false, alt: false }, hasTextSelection: true }),
    )
    A(result, 'None')
  })

  it('returns CutBlock on Cmd+X with no text selection', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'x', mods: { mod: true, shift: false, alt: false }, hasTextSelection: false }),
    )
    A(result, 'CutBlock')
  })

  it('returns None on Cmd+X with active text selection (let browser cut)', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'x', mods: { mod: true, shift: false, alt: false }, hasTextSelection: true }),
    )
    A(result, 'None')
  })

  it('returns PasteAsNewBlock on Cmd+V', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'v', mods: { mod: true, shift: false, alt: false } }),
    )
    A(result, 'PasteAsNewBlock')
  })
})

// ── Escape and pass-through ───────────────────────────────

describe('blockKeyboardHandler — Escape and pass-through', () => {
  it('returns Blur on Escape', () => {
    const result = blockKeyboardHandler(ctx({ key: 'Escape' }))
    A(result, 'Blur')
  })

  it('returns None on plain letter "a" (let browser insert text)', () => {
    const result = blockKeyboardHandler(ctx({ key: 'a' }))
    A(result, 'None')
  })

  it('returns None on digit "1"', () => {
    const result = blockKeyboardHandler(ctx({ key: '1' }))
    A(result, 'None')
  })

  it('returns None on space key (let browser insert space)', () => {
    const result = blockKeyboardHandler(ctx({ key: ' ' }))
    A(result, 'None')
  })

  it('returns None on plain uppercase "A" (Shift modifies → still None)', () => {
    const result = blockKeyboardHandler(
      ctx({ key: 'A', mods: { mod: false, shift: true, alt: false } }),
    )
    A(result, 'None')
  })

  it('returns None on ArrowLeft (horizontal cursor movement is browser territory)', () => {
    const result = blockKeyboardHandler(ctx({ key: 'ArrowLeft', cursor: cursorMid }))
    A(result, 'None')
  })

  it('returns None on ArrowRight (horizontal cursor movement is browser territory)', () => {
    const result = blockKeyboardHandler(ctx({ key: 'ArrowRight', cursor: cursorMid }))
    A(result, 'None')
  })

  it('returns None on empty key string', () => {
    const result = blockKeyboardHandler(ctx({ key: '' }))
    A(result, 'None')
  })
})

// ── Determinism: same input → same output ─────────────────

describe('blockKeyboardHandler — Determinism', () => {
  it('is a pure function: identical inputs return identical outputs', () => {
    const input = ctx({ key: 'Enter', cursor: cursorMid })
    const a1 = blockKeyboardHandler(input)
    const a2 = blockKeyboardHandler(input)
    expect(a1).toEqual(a2)
  })

  it('does not mutate the context object', () => {
    const input: KeyboardContext = ctx({ key: 'Enter', cursor: cursorMid })
    const snapshot = JSON.stringify(input)
    blockKeyboardHandler(input)
    expect(JSON.stringify(input)).toBe(snapshot)
  })

  it('does not mutate the block', () => {
    const b = block()
    const snapshot = JSON.stringify(b)
    blockKeyboardHandler(ctx({ block: b, key: 'Enter', cursor: cursorMid }))
    expect(JSON.stringify(b)).toBe(snapshot)
  })
})

// ── Property-style: every key produces one of the valid action types ─

describe('blockKeyboardHandler — Action set closure', () => {
  const validActionTypes = new Set<KeyboardAction['type']>([
    'None',
    'Split',
    'CreateEmptySibling',
    'Indent',
    'Outdent',
    'MergeWithPrev',
    'MergeWithNext',
    'InsertText',
    'ToggleDone',
    'SetPriority',
    'SetMarker',
    'MoveCursor',
    'SelectParent',
    'SelectAll',
    'ExtendSelection',
    'MoveBlockUp',
    'MoveBlockDown',
    'CopyBlock',
    'CutBlock',
    'PasteAsNewBlock',
    'ToggleInlineMark',
    'Undo',
    'Redo',
    'Blur',
    'FollowLink',
  ])

  const SAMPLE_KEYS = [
    '',
    'Enter',
    'Escape',
    'Backspace',
    'Delete',
    'Tab',
    'ArrowUp',
    'ArrowDown',
    'ArrowLeft',
    'ArrowRight',
    'a',
    'z',
    'b',
    'i',
    'A',
    'Z',
    '`',
    'y',
    '1',
    ' ',
    '?',
  ]
  const SAMPLE_MODS: Modifiers[] = [
    { mod: false, shift: false, alt: false },
    { mod: true, shift: false, alt: false },
    { mod: false, shift: true, alt: false },
    { mod: false, shift: false, alt: true },
    { mod: true, shift: true, alt: false },
    { mod: true, shift: false, alt: true },
    { mod: false, shift: true, alt: true },
    { mod: true, shift: true, alt: true },
  ]
  const SAMPLE_CURSORS: CursorPos[] = [
    { offset: 0, atStart: true, atEnd: false },
    { offset: 0, atStart: true, atEnd: true },
    { offset: 5, atStart: false, atEnd: false },
    { offset: 11, atStart: false, atEnd: true },
  ]
  const SAMPLE_TEXTS = ['', 'x', 'Hello world', 'see [[Page]] here', '#tag']

  it('always returns an action whose type is in the KeyboardAction union', () => {
    let count = 0
    for (const key of SAMPLE_KEYS) {
      for (const mods of SAMPLE_MODS) {
        for (const cursor of SAMPLE_CURSORS) {
          for (const content of SAMPLE_TEXTS) {
            const result = blockKeyboardHandler(
              ctx({ key, mods, cursor, content, hasTextSelection: false }),
            )
            expect(validActionTypes.has(result.type)).toBe(true)
            count++
          }
        }
      }
    }
    // Sanity check that we actually exercised the cartesian product.
    expect(count).toBe(
      SAMPLE_KEYS.length *
        SAMPLE_MODS.length *
        SAMPLE_CURSORS.length *
        SAMPLE_TEXTS.length,
    )
  })
})
