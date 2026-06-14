/**
 * Helpers for working with `Annotation` lists.
 */

/** Count open annotations (pending + in_progress). */
export function countOpenAnnotations(
  annotations: readonly { status: string }[],
): number {
  let n = 0
  for (const a of annotations) {
    if (a.status === 'pending' || a.status === 'in_progress') n += 1
  }
  return n
}

/**
 * Sort annotations: `createdAt` DESC for the sidebar list.
 * Tie-break by `id` to keep the order deterministic when two
 * annotations share a timestamp (UUIDs sort lexicographically).
 */
export function sortByCreatedAtDesc<T extends { createdAt: string; id: string }>(
  annotations: readonly T[],
): T[] {
  return [...annotations].sort((a, b) => {
    if (a.createdAt !== b.createdAt) {
      return a.createdAt < b.createdAt ? 1 : -1
    }
    return a.id < b.id ? 1 : -1
  })
}

/**
 * Build a tree of root annotations + their replies.
 * Top-level annotations are those without a `parentAnnotationId`;
 * replies are nested under their parent. Annotations whose parent
 * is missing are treated as top-level (defensive — the server
 * should never return an orphan reply, but if it did we don't
 * want to lose the row from the panel).
 */
export interface AnnotationThreadNode<T> {
  annotation: T
  replies: AnnotationThreadNode<T>[]
}

export function buildAnnotationThread<T extends { id: string; parentAnnotationId?: string }>(
  annotations: readonly T[],
): AnnotationThreadNode<T>[] {
  // First pass: index every annotation by id so we can resolve the
  // parent reference for each child. Orphans (parent id not in the
  // set) fall back to being roots — the server should never return
  // an orphan reply, but if it does we don't want to lose the row
  // from the panel.
  const byId = new Map<string, T>()
  for (const a of annotations) byId.set(a.id, a)

  const byParent = new Map<string, T[]>()
  const roots: T[] = []

  for (const a of annotations) {
    const parent = a.parentAnnotationId
    if (parent && byId.has(parent)) {
      let bucket = byParent.get(parent)
      if (!bucket) {
        bucket = []
        byParent.set(parent, bucket)
      }
      bucket.push(a)
    } else {
      roots.push(a)
    }
  }

  const buildChildren = (parent: T): AnnotationThreadNode<T> => ({
    annotation: parent,
    replies: (byParent.get(parent.id) ?? []).map(buildChildren),
  })

  return roots.map(buildChildren)
}
