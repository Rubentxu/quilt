import { useCallback, useState, useRef, useEffect } from 'react'

/**
 * A single undoable action. The `restore` callback is what gets
 * invoked when the user triggers undo; it owns all the side effects
 * needed to reverse whatever produced this action in the first
 * place (e.g. re-creating a deleted block via the API and re-inserting
 * it into local state).
 *
 * `restore` MAY be async. The manager awaits it before resolving
 * `undo()` so callers can chain further work after the restore has
 * finished.
 */
export interface UndoAction {
  /** Discriminator — useful for debugging and for tests that
   *  assert *which* action ran. Not used by the manager itself. */
  type: string
  /** The reverse operation. May be sync or async. */
  restore: () => Promise<void> | void
}

/**
 * Session-scoped LIFO undo stack.
 *
 * Design notes:
 *
 *  - **No persistence.** Per requirements, the stack resets when the
 *    page unloads. We don't write to localStorage; the React hook
 *    re-instantiates the manager on remount, which is the natural
 *    session boundary.
 *
 *  - **Generics-free.** Actions are typed as `UndoAction` and the
 *    `restore` callback is responsible for closing over whatever
 *    context it needs (block snapshot, page name, state setter).
 *    This keeps the manager small and the call sites explicit.
 *
 *  - **Async restore handling.** A failing `restore` is logged but
 *    does not stop the pop — the alternative is a stuck stack that
 *    blocks the user from undoing anything else. The contract is
 *    "we tried; we move on".
 *
 *  - **Max depth is enforced on push**, not on undo. Eviction is
 *    FIFO (oldest entry is dropped). This matches every editor
 *    undo stack the author has ever used.
 */
export class UndoManager {
  private stack: UndoAction[] = []
  private readonly maxDepth: number

  constructor(maxDepth = 50) {
    if (maxDepth < 1) {
      // Defensive: a 0-depth stack would make push a no-op and break
      // the contract that `canUndo()` reflects the last push.
      throw new Error(`UndoManager: maxDepth must be >= 1, got ${maxDepth}`)
    }
    this.maxDepth = maxDepth
  }

  /** Number of actions currently on the stack. */
  get size(): number {
    return this.stack.length
  }

  /** True iff there is at least one action to undo. */
  canUndo(): boolean {
    return this.stack.length > 0
  }

  /**
   * Add an action. If the stack is at `maxDepth`, the oldest action
   * is dropped to make room (FIFO eviction).
   */
  push(action: UndoAction): void {
    this.stack.push(action)
    if (this.stack.length > this.maxDepth) {
      this.stack.shift()
    }
  }

  /**
   * Pop the most recent action and run its `restore` callback.
   * Returns `true` if an action was popped, `false` if the stack
   * was empty. The returned promise resolves once `restore` settles
   * (including its async work, if any).
   */
  async undo(): Promise<boolean> {
    const action = this.stack.pop()
    if (!action) return false
    try {
      await action.restore()
    } catch (err) {
      // Don't let a broken restore freeze the stack. The action is
      // already popped; we log so the developer can investigate but
      // we don't rethrow — the user expects "undo" to advance the
      // stack regardless of what the restore did.
      // eslint-disable-next-line no-console
      console.error(`UndoManager: restore() for action "${action.type}" threw:`, err)
    }
    return true
  }

  /** Empty the stack. Use when navigating between pages or when
   *  the user explicitly wants to start fresh. */
  clear(): void {
    this.stack = []
  }
}

// ─── React adapter ───────────────────────────────────────────────

export interface UseUndoManagerResult {
  /** Stable manager instance — the same reference across renders
   *  for a given hook lifetime. */
  manager: UndoManager
  /** Reactive boolean mirror of `manager.canUndo()`. Components
   *  can read this to disable the undo button, render a hint, etc. */
  canUndo: boolean
  /** Push an action and update the reactive `canUndo`. */
  push: (action: UndoAction) => void
  /** Pop + restore the top action. Returns the manager's result
   *  (`true` if something was undone). */
  undo: () => Promise<boolean>
  /** Empty the stack. */
  clear: () => void
}

/**
 * React hook that wraps an `UndoManager` instance and keeps a
 * `canUndo` boolean in state so the UI can re-render when the
 * stack changes.
 *
 * The manager itself is created once per mount and stored in a ref
 * to keep its identity stable across renders — that way callbacks
 * captured at construction time still resolve to the live manager.
 */
export function useUndoManager(maxDepth = 50): UseUndoManagerResult {
  const managerRef = useRef<UndoManager | null>(null)
  if (managerRef.current === null) {
    managerRef.current = new UndoManager(maxDepth)
  }
  const [canUndo, setCanUndo] = useState<boolean>(managerRef.current.canUndo())

  // Keep the depth ref fresh in case the caller ever changes it,
  // but don't recreate the manager — that would erase the stack.
  // (Recreating is a deliberate operation the caller can do by
  // unmounting/remounting the hook.)
  useEffect(() => {
    // No-op: the manager is created lazily above.
  }, [maxDepth])

  const push = useCallback((action: UndoAction) => {
    managerRef.current!.push(action)
    setCanUndo(managerRef.current!.canUndo())
  }, [])

  const undo = useCallback(async (): Promise<boolean> => {
    const ok = await managerRef.current!.undo()
    setCanUndo(managerRef.current!.canUndo())
    return ok
  }, [])

  const clear = useCallback(() => {
    managerRef.current!.clear()
    setCanUndo(managerRef.current!.canUndo())
  }, [])

  return {
    manager: managerRef.current,
    canUndo,
    push,
    undo,
    clear,
  }
}
