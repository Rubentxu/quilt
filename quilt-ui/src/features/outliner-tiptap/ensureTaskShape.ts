// ─── ensureTaskShape.ts ────────────────────────────────────────────────────
//
// Converts a paragraph/bullet/numbered/heading block to a task block when
// a marker is set. This is a ONE-WAY conversion (ADR-0023 deviation,
// documented in ADR-0025).
//
// WHY this coupling exists (ADR-0023 deviation):
//   ADR-0023 states that marker and blockType are orthogonal — a block
//   should be able to have any marker regardless of its blockType. The
//   UX design (ADR-0025) intentionally departs from this principle:
//   when a user sets a marker via slash command on a plain text block,
//   the block visually "becomes a task" (blockType→todo) so that:
//     (a) the checkbox renders unambiguously
//     (b) the block appears in task queries by default
//     (c) the conversion is permanent — clearing the marker does NOT
//         revert blockType, because once a user has made something a
//         task, they probably still want task semantics even if they
//         clear the marker.
//
//   This coupling is a deliberate UX trade-off: we gain clarity at the
//   cost of ADR-0023's pure orthogonality. The one-way nature means
//   users can't "undo" the conversion, but this matches user intent:
//   they marked it as a task, not "I want a todo item occasionally."

import type { Block, TaskMarker, BlockType } from '@shared/types/api'
import type { UpdateBlockRequest } from '@shared/types/api'

/**
 * Block types that do NOT convert to 'todo' when a marker is set.
 * These are inherently non-task types (code, quote, divider, image)
 * or already-tasks (todo).
 */
export const NON_TASK_BLOCK_TYPES: BlockType[] = ['code', 'quote', 'divider', 'image', 'todo']

/**
 * Converts a paragraph/bullet/numbered/heading block to a task block
 * when a marker is set. Returns the partial UpdateBlockRequest fields
 * to merge into the api.updateBlock call.
 *
 * ONE-WAY: clearing the marker does NOT revert blockType (ADR-0025 deviation).
 *
 * @param currentBlock - The block being updated
 * @param newMarker    - The marker being set (null = clearing marker)
 * @returns Partial<UpdateBlockRequest> with blockType and/or marker fields
 */
export function ensureTaskShape(
  currentBlock: Block,
  newMarker: TaskMarker | null,
): Partial<UpdateBlockRequest> {
  // Case 1: clearing marker — one-way, do NOT touch blockType
  if (newMarker === null) {
    return { marker: null }
  }

  // Case 2: block types that are already tasks or non-task types —
  // just update the marker, don't touch blockType
  if (NON_TASK_BLOCK_TYPES.includes(currentBlock.blockType)) {
    return { marker: newMarker }
  }

  // Case 3: paragraph/bullet/numbered/heading → convert to todo
  // (also covers heading1/heading2/heading3 per ADR-0025 deviation)
  return { blockType: 'todo', marker: newMarker }
}
