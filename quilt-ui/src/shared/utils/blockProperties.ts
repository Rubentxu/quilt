// ──── Property helpers ────────────────────────────────────────────

import type { BlockProperty } from '@shared/types/api'

/**
 * Convert the backend's `Record<string, unknown>` properties map into the
 * `BlockProperty[]` shape used by the rest of the frontend (and by the
 * `BlockPropertiesPanel`).
 *
 * Each entry's `type` is inferred from the JSON value:
 * - string → "string"
 * - boolean → "boolean"
 * - number → "number"
 * - array → "select"
 * - null / undefined → "string" (fallback)
 */
export function blockPropertiesFromMap(
  map: Record<string, unknown> | null | undefined,
): BlockProperty[] {
  if (!map) return []
  return Object.entries(map).map(([key, value]) => ({
    key,
    value: normalizePropertyValue(value),
    type: inferPropertyType(value),
  }))
}

/**
 * Coerce an unknown JSON value into a BlockProperty-compatible
 * `string | number | boolean | null`.
 */
function normalizePropertyValue(
  value: unknown,
): string | number | boolean | null {
  if (value === null || value === undefined) return null
  if (typeof value === 'string') return value
  if (typeof value === 'number') return value
  if (typeof value === 'boolean') return value
  // Arrays / objects: serialize as JSON for display
  try {
    return JSON.stringify(value)
  } catch {
    return String(value)
  }
}

/**
 * Best-effort type inference for a JSON property value.
 */
function inferPropertyType(value: unknown): BlockProperty['type'] {
  if (typeof value === 'boolean') return 'boolean'
  if (typeof value === 'number') return 'number'
  if (Array.isArray(value)) return 'select'
  return 'string'
}

/**
 * Find a single property value on a block by key.
 * Returns `undefined` if the property isn't set.
 */
export function getBlockProperty(
  properties: BlockProperty[] | undefined,
  key: string,
): string | number | boolean | null | undefined {
  return properties?.find(p => p.key === key)?.value
}

/**
 * Find all comment children of a given block.
 *
 * A comment is any block with a `type` property equal to "comment".
 */
export function findCommentChildren(blocks: import('@shared/types/api').Block[], blockId: string) {
  return blocks.filter(
    b =>
      b.parentId === blockId &&
      getBlockProperty(b.properties, 'type') === 'comment',
  )
}

/**
 * Recursively collect all comment descendants of a block.
 *
 * Returns a tree of comment blocks: each entry has a `comment` and a
 * `replies` array containing nested comments.
 */
export interface CommentTree {
  comment: import('@shared/types/api').Block
  replies: CommentTree[]
}

export function buildCommentTree(
  blocks: import('@shared/types/api').Block[],
  blockId: string,
): CommentTree[] {
  const direct = findCommentChildren(blocks, blockId).sort(
    (a, b) => a.order - b.order,
  )
  return direct.map(comment => ({
    comment,
    replies: buildCommentTree(blocks, comment.id),
  }))
}

/**
 * Check if a block looks like a comment.
 */
export function isCommentBlock(block: import('@shared/types/api').Block): boolean {
  return getBlockProperty(block.properties, 'type') === 'comment'
}
