import type { Block } from '@shared/types/api'

/**
 * A `BlockIdSet` is the result of collecting every block id that
 * falls inside a zoomed subtree (the zoomed block + all its
 * descendants). The set is used by the caller to filter the
 * block list before flattening.
 *
 * `null` is returned when no zoom is active — this is a deliberate
 * sentinel so the caller can cheaply skip the filter without
 * having to check `zoomBlockId != null` again.
 */
export type BlockIdSet = Set<string>

/**
 * Collect the set of block ids that should be visible when
 * zooming into `zoomBlockId`. Returns:
 *
 *   - `null`  → no zoom active (caller should render everything)
 *   - `Set()` → zoom is active but the block is missing (caller
 *               should treat this as "zoom out" or render an
 *               "empty" zoom viewport)
 *   - `Set(...)` → the zoomed block id + every transitive
 *                  descendant id
 *
 * The function is intentionally defensive against cycles (a
 * pathological block graph where A.parentId = B and B.parentId = A)
 * — a `visited` set ensures termination even with malformed data.
 *
 * Complexity: O(N) where N is the number of blocks. The recursion
 * walks each child once, and the visited set deduplicates.
 */
export function collectZoomSubtree(
  blocks: Block[],
  zoomBlockId: string | null | undefined,
): BlockIdSet | null {
  if (zoomBlockId == null || zoomBlockId === '') return null

  // Build a parent → children index in one pass. Faster than
  // repeated `blocks.filter(b => b.parentId === id)` lookups for
  // large trees. We also collect the set of all known block ids
  // so the caller can detect "zoom target missing" cheaply.
  const childrenByParent = new Map<string | null, Block[]>()
  const knownIds = new Set<string>()
  for (const block of blocks) {
    const key = block.parentId ?? null
    const bucket = childrenByParent.get(key)
    if (bucket) {
      bucket.push(block)
    } else {
      childrenByParent.set(key, [block])
    }
    knownIds.add(block.id)
  }

  // If the zoom target itself is not in the block list, return an
  // empty Set — the caller can distinguish "zoom inactive" (null)
  // from "zoom active but block missing" (empty Set) and react
  // appropriately (e.g. fire the auto-zoom-out callback).
  if (!knownIds.has(zoomBlockId)) {
    return new Set()
  }

  const result: BlockIdSet = new Set()
  const visited = new Set<string>()

  function collect(blockId: string): void {
    if (visited.has(blockId)) return
    visited.add(blockId)
    result.add(blockId)
    const children = childrenByParent.get(blockId) ?? []
    for (const child of children) {
      collect(child.id)
    }
  }

  collect(zoomBlockId)
  return result
}
