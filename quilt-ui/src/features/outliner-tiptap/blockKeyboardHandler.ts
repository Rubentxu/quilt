/**
 * Pure keyboard decision-making for the outliner's block editor.
 *
 * BlockRow's old `handleKeyDown` callback (~440 LOC) fused DOM access
 * with decision logic, leaving zero unit coverage. This module extracts
 * the decisions into a pure function so the rules can be tested without
 * a DOM, and BlockRow becomes a thin adapter that applies the action.
 *
 * Contract:
 *   • Input:  the current block state + the keyboard event facts
 *   • Output: a `KeyboardAction` describing what SHOULD happen
 *   • No DOM access, no React state, no side effects
 *
 * The adapter in BlockRow is responsible for:
 *   1. Gathering the context (block, allBlocks, cursor position, etc.)
 *   2. Calling this function
 *   3. Switching on the returned action to apply DOM / state changes
 */
import type { Block, TaskMarker } from '@shared/types/api'

// ── Input types ───────────────────────────────────────────────

/** Keyboard modifier flags. `mod` abstracts Cmd (Mac) vs Ctrl (Win/Linux). */
export interface Modifiers {
  mod: boolean
  shift: boolean
  alt: boolean
}

/** Cursor position facts (already-computed; the function does no DOM access). */
export interface CursorPos {
  /** Character offset within the block content. */
  offset: number
  atStart: boolean
  atEnd: boolean
}

/** A link detected under or near the cursor. */
export type NearestLink =
  | { type: 'page'; target: string }
  | { type: 'block'; target: string }
  | { type: 'tag'; target: string }
  | null

/** Everything the function needs to decide. */
export interface KeyboardContext {
  block: Block
  /** All blocks on the same page (for sibling lookups). */
  allBlocks: Block[]
  /** Current text content (may include unsaved local edits). */
  content: string
  key: string
  mods: Modifiers
  cursor: CursorPos
  /** Whether there is an active (non-collapsed) text selection. */
  hasTextSelection: boolean
}

// ── Output: action enum ───────────────────────────────────────

/**
 * What should happen next. The adapter in BlockRow switches on this
 * and applies DOM / state changes.
 */
export type KeyboardAction =
  // ── Block structure changes
  | { type: 'Split'; at: number }
  | { type: 'CreateEmptySibling' }
  | { type: 'Indent' }
  | { type: 'Outdent' }
  | { type: 'MergeWithPrev' }
  | { type: 'MergeWithNext' }
  | { type: 'InsertText'; text: string }
  // ── Marker / priority
  | { type: 'ToggleDone' }
  | { type: 'SetPriority'; level: 'A' | 'B' | 'C' }
  | { type: 'SetMarker'; kind: TaskMarker }
  // ── Cursor movement
  | { type: 'MoveCursor'; to: 'prev' | 'next' | 'up' | 'down' }
  // ── Selection operations
  | { type: 'SelectParent' }
  | { type: 'SelectAll' }
  | { type: 'ExtendSelection'; direction: 'up' | 'down' }
  // ── Block move
  | { type: 'MoveBlockUp' }
  | { type: 'MoveBlockDown' }
  // ── Clipboard
  | { type: 'CopyBlock' }
  | { type: 'CutBlock' }
  | { type: 'PasteAsNewBlock' }
  // ── Inline formatting
  | { type: 'ToggleInlineMark'; marker: '**' | '*' | '`' }
  // ── History
  | { type: 'Undo' }
  | { type: 'Redo' }
  // ── Focus
  | { type: 'Blur' }
  // ── Link following
  | { type: 'FollowLink'; link: NearestLink }
  // ── Pass-through
  | { type: 'None' }

// ── Pure function ────────────────────────────────────────────

/**
 * Decide what keyboard action to take given the current state.
 * Pure: no DOM, no React, no side effects.
 */
export function blockKeyboardHandler(ctx: KeyboardContext): KeyboardAction {
  const { block, allBlocks, content, key, mods, cursor, hasTextSelection } = ctx

  // ── Escape: exit edit mode (autocomplete menus are handled by the
  //    adapter BEFORE calling this function) ────────────────────
  if (key === 'Escape') return { type: 'Blur' }

  // ── Cmd+Enter (no shift, no alt): follow link or cycle marker ──
  // Mirrors Quilt's `follow-link-under-cursor!` behavior: if the
  // cursor sits in a [[page]], ((block)), or #tag, follow it.
  // Otherwise, cycle the task marker (None → Todo → Done → None).
  if (key === 'Enter' && mods.mod && !mods.shift && !mods.alt) {
    const link = findNearestLink(content, cursor.offset)
    if (link) return { type: 'FollowLink', link }
    return { type: 'ToggleDone' }
  }

  // ── Undo / Redo ──────────────────────────────────────────────
  if (key === 'z' && mods.mod && !mods.shift && !mods.alt) {
    return { type: 'Undo' }
  }
  if ((key === 'z' && mods.mod && mods.shift) || (key === 'y' && mods.mod)) {
    return { type: 'Redo' }
  }

  // ── Inline formatting ────────────────────────────────────────
  if (key === 'b' && mods.mod && !mods.shift) return { type: 'ToggleInlineMark', marker: '**' }
  if (key === 'i' && mods.mod && !mods.shift) return { type: 'ToggleInlineMark', marker: '*' }
  if (key === '`' && mods.mod) return { type: 'ToggleInlineMark', marker: '`' }

  // ── Select parent / select all ───────────────────────────────
  if (key === 'a' && mods.mod) {
    return mods.shift ? { type: 'SelectAll' } : { type: 'SelectParent' }
  }

  // ── Clipboard (block-level when no text selection) ───────────
  if (key === 'c' && mods.mod && !mods.shift) {
    return hasTextSelection ? { type: 'None' } : { type: 'CopyBlock' }
  }
  if (key === 'x' && mods.mod) {
    return hasTextSelection ? { type: 'None' } : { type: 'CutBlock' }
  }
  if (key === 'v' && mods.mod) return { type: 'PasteAsNewBlock' }

  // ── Shift+Enter: soft newline (same block, no split) ─────────
  if (key === 'Enter' && mods.shift && !mods.mod) {
    return { type: 'InsertText', text: '\n' }
  }

  // ── Enter (plain): split or create empty sibling ─────────────
  if (key === 'Enter' && !mods.shift && !mods.mod) {
    if (!content.trim()) return { type: 'CreateEmptySibling' }
    if (cursor.atEnd) return { type: 'CreateEmptySibling' }
    return { type: 'Split', at: cursor.offset }
  }

  // ── Backspace at start: merge with prev, outdent, or nothing ─
  if (key === 'Backspace' && cursor.atStart) {
    const prev = findPrevSibling(block, allBlocks)
    if (prev) return { type: 'MergeWithPrev' }
    if (block.parentId) return { type: 'Outdent' }
    return { type: 'None' }
  }

  // ── Tab / Shift+Tab (no Cmd) ─────────────────────────────────
  if (key === 'Tab' && !mods.mod) {
    return mods.shift ? { type: 'Outdent' } : { type: 'Indent' }
  }

  // ── ArrowUp ──────────────────────────────────────────────────
  if (key === 'ArrowUp') {
    if (mods.alt && mods.shift) return { type: 'MoveBlockUp' }
    if (mods.alt) return { type: 'ExtendSelection', direction: 'up' }
    if (cursor.atStart) return { type: 'MoveCursor', to: 'prev' }
    return { type: 'None' }
  }

  // ── ArrowDown ────────────────────────────────────────────────
  if (key === 'ArrowDown') {
    if (mods.alt && mods.shift) return { type: 'MoveBlockDown' }
    if (mods.alt) return { type: 'ExtendSelection', direction: 'down' }
    if (cursor.atEnd) return { type: 'MoveCursor', to: 'next' }
    return { type: 'None' }
  }

  // Default: let the browser handle it (text input, cursor keys, etc.)
  return { type: 'None' }
}

// ── Helpers (pure) ───────────────────────────────────────────

/** Find the sibling immediately before this block (by order). */
function findPrevSibling(block: Block, allBlocks: Block[]): Block | null {
  const siblings = allBlocks
    .filter(b => b.id !== block.id && b.parentId === block.parentId)
    .sort((a, b) => a.order - b.order)

  const idx = siblings.findIndex(b => b.order >= block.order)
  if (idx > 0) return siblings[idx - 1]
  if (idx === -1 && siblings.length > 0) return siblings[siblings.length - 1]
  return null
}

/**
 * Find the link nearest to `cursorPos` in `text`.
 * Mirrors Quilt's `extract-nearest-link-from-text`: page-refs and
 * block-refs first, then tags. When the cursor sits inside a link
 * that link wins; otherwise we pick the closest one by absolute
 * distance to the cursor.
 *
 * Exported so the BlockRow adapter (and any other consumer) can use
 * the same implementation.
 */
export function findNearestLink(text: string, cursorPos: number): NearestLink {
  const candidates: Array<{ start: number; end: number; link: NearestLink }> = []

  // [[Page]] or [[Page|alias]]  — strip optional |alias
  const pageRe = /\[\[([^\]\|]+)(?:\|[^\]]*)?\]\]/g
  let m: RegExpExecArray | null
  while ((m = pageRe.exec(text)) !== null) {
    candidates.push({
      start: m.index,
      end: m.index + m[0].length,
      link: { type: 'page', target: m[1].trim() },
    })
  }

  // ((block-uuid))  — block reference
  const blockRe = /\(\(([^\)]+)\)\)/g
  while ((m = blockRe.exec(text)) !== null) {
    candidates.push({
      start: m.index,
      end: m.index + m[0].length,
      link: { type: 'block', target: m[1].trim() },
    })
  }

  // #tag  — preceded by start, whitespace, or punctuation so #1.5 isn't a tag
  const tagRe = /(?:^|[\s\(\[])#([A-Za-z0-9_\-]+)/g
  while ((m = tagRe.exec(text)) !== null) {
    // Adjust start: the match includes the leading boundary char.
    const matchStart = m.index + m[0].indexOf('#')
    candidates.push({
      start: matchStart,
      end: matchStart + (m[1].length + 1),
      link: { type: 'tag', target: m[1] },
    })
  }

  if (candidates.length === 0) return null

  // Prefer links that contain the cursor; otherwise pick the one with
  // the smallest distance to the cursor.
  const containing = candidates.find(c => cursorPos >= c.start && cursorPos <= c.end)
  if (containing) return containing.link

  const closest = candidates
    .map(c => ({ c, dist: Math.min(Math.abs(cursorPos - c.start), Math.abs(cursorPos - c.end)) }))
    .sort((a, b) => a.dist - b.dist)[0]
  return closest.c.link
}
