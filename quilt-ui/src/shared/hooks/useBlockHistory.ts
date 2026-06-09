import { useState, useEffect, useCallback, useRef } from 'react'
import {
  wasmHistoryNew,
  wasmHistoryFree,
  wasmHistoryApply,
  wasmHistoryUndo,
  wasmHistoryRedo,
  wasmHistoryCanUndo,
  wasmHistoryCanRedo,
} from '@core/wasm-bridge/wasm-loader'
import type { Block, OutlinerCommand } from '@shared/types/api'

interface UseBlockHistoryOptions {
  /** Page name (or id). Used as a re-init key. */
  pageName: string
  /** Current blocks for the page. */
  blocks: Block[]
  /** Called with the new block list after every apply/undo/redo. */
  onBlocksChanged: (newBlocks: Block[]) => void
  /**
   * Optional. Called after an undo/redo with the list of blocks whose
   * `content` changed between `blocks` and the post-undo result, so the
   * caller can persist the reverted state to the server. Without this
   * the undo is in-memory only — a subsequent poll/sync will overwrite
   * the reverted React state with the post-edit server content.
   *
   * The hook deliberately does not import the API client itself; the
   * caller decides what "persist" means (network call, optimistic
   * with retry, etc.) and can run the persist in parallel with the
   * sync-suppression window it needs.
   */
  onAfterHistoryChange?: (changed: Block[]) => void | Promise<void>
  /** Disable the hook (no stack, all calls no-op). */
  enabled?: boolean
  /** Maximum undo depth. Default: 100. */
  maxSize?: number
}

interface UseBlockHistoryResult {
  /** Record a command in the WASM history and apply it. */
  applyCommand: (command: OutlinerCommand) => boolean
  /** Undo the last command. Returns true on success. */
  undo: () => boolean
  /** Redo the next command. Returns true on success. */
  redo: () => boolean
  canUndo: boolean
  canRedo: boolean
}

/**
 * Bridge between the React block state and the Rust `HistoryStack`
 * (exposed via WASM). Each page gets its own stack; the stack id is
 * tracked in a ref so re-renders don't trigger re-init.
 *
 * The hook does NOT re-init on every `blocks` change — that would
 * erase history. It re-inits only when `pageName` (or `enabled`)
 * changes.
 *
 * **Sticky-undo contract.** By default, `undo()` / `redo()` only
 * change the in-memory `blocks` list. If the host app wants the
 * revert to survive an SSE/polling resync (the polling interval in
 * PageView is 15s — plenty of time to clobber an in-memory undo),
 * pass `onAfterHistoryChange`. The hook fires it with the list of
 * blocks whose `content` actually changed so the host can PATCH
 * the server.
 */
export function useBlockHistory({
  pageName,
  blocks,
  onBlocksChanged,
  onAfterHistoryChange,
  enabled = true,
  maxSize: _maxSize = 100,
}: UseBlockHistoryOptions): UseBlockHistoryResult {
  const stackIdRef = useRef<number | null>(null)
  const [canUndo, setCanUndo] = useState(false)
  const [canRedo, setCanRedo] = useState(false)
  const onBlocksChangedRef = useRef(onBlocksChanged)
  onBlocksChangedRef.current = onBlocksChanged
  const onAfterHistoryChangeRef = useRef(onAfterHistoryChange)
  onAfterHistoryChangeRef.current = onAfterHistoryChange
  const blocksRef = useRef<Block[]>(blocks)
  blocksRef.current = blocks

  /**
   * Diff `prev` against `next` by id and return the subset of `next`
   * whose `content` changed. Used to know which blocks need a server
   * re-save after an undo/redo. We diff by `content` only — the
   * server's PATCH /blocks/:id accepts a partial update and we only
   * care about the textual revert for the sticky-undo fix.
   */
  const diffChangedContent = useCallback(
    (prev: Block[], next: Block[]): Block[] => {
      const prevById = new Map(prev.map(b => [b.id, b]))
      const changed: Block[] = []
      for (const b of next) {
        const before = prevById.get(b.id)
        if (!before) continue
        if (before.content !== b.content) changed.push(b)
      }
      return changed
    },
    [],
  )

  // ── Re-init the stack when the page changes ──
  useEffect(() => {
    if (!enabled) {
      // Free any leftover stack.
      if (stackIdRef.current !== null) {
        try {
          wasmHistoryFree(stackIdRef.current)
        } catch {
          /* swallow — we are tearing down */
        }
        stackIdRef.current = null
        setCanUndo(false)
        setCanRedo(false)
      }
      return
    }

    // Free the previous stack before allocating a new one.
    if (stackIdRef.current !== null) {
      try {
        wasmHistoryFree(stackIdRef.current)
      } catch (err) {
        console.warn('history_free failed', err)
      }
      stackIdRef.current = null
    }

    try {
      const newId = wasmHistoryNew(blocks)
      stackIdRef.current = newId
      setCanUndo(wasmHistoryCanUndo(newId))
      setCanRedo(wasmHistoryCanRedo(newId))
    } catch (err) {
      // WASM not yet loaded (or pkg out of date) — degrade gracefully.
      console.warn('useBlockHistory: history stack init failed', err)
      stackIdRef.current = null
      setCanUndo(false)
      setCanRedo(false)
    }

    return () => {
      const id = stackIdRef.current
      if (id !== null) {
        try {
          wasmHistoryFree(id)
        } catch {
          /* swallow */
        }
        stackIdRef.current = null
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pageName, enabled])

  // ── Apply a command and record it ──
  const applyCommand = useCallback(
    (command: OutlinerCommand): boolean => {
      const id = stackIdRef.current
      if (!enabled || id === null) return false
      try {
        const newBlocks = wasmHistoryApply(id, command)
        onBlocksChangedRef.current(newBlocks)
        setCanUndo(wasmHistoryCanUndo(id))
        setCanRedo(false) // clear redo on a new command
        return true
      } catch (err) {
        console.error('useBlockHistory: applyCommand failed', err)
        return false
      }
    },
    [enabled],
  )

  // ── Undo ──
  const undo = useCallback((): boolean => {
    const id = stackIdRef.current
    if (!enabled || id === null) return false
    try {
      const newBlocks = wasmHistoryUndo(id)
      if (newBlocks == null) return false
      onBlocksChangedRef.current(newBlocks)
      // Persist the reverted content to the server so the next
      // SSE/poll tick doesn't re-apply the post-edit state. The
      // caller is responsible for any API call + sync suppression;
      // we just tell them which blocks changed.
      const changed = diffChangedContent(blocksRef.current, newBlocks)
      if (changed.length > 0) {
        onAfterHistoryChangeRef.current?.(changed)
      }
      setCanUndo(wasmHistoryCanUndo(id))
      setCanRedo(wasmHistoryCanRedo(id))
      return true
    } catch (err) {
      console.error('useBlockHistory: undo failed', err)
      return false
    }
  }, [enabled, diffChangedContent])

  // ── Redo ──
  const redo = useCallback((): boolean => {
    const id = stackIdRef.current
    if (!enabled || id === null) return false
    try {
      const newBlocks = wasmHistoryRedo(id)
      if (newBlocks == null) return false
      onBlocksChangedRef.current(newBlocks)
      // Same persistence story as `undo` — without this, redo would
      // also be clobbered by the next sync tick.
      const changed = diffChangedContent(blocksRef.current, newBlocks)
      if (changed.length > 0) {
        onAfterHistoryChangeRef.current?.(changed)
      }
      setCanUndo(wasmHistoryCanUndo(id))
      setCanRedo(wasmHistoryCanRedo(id))
      return true
    } catch (err) {
      console.error('useBlockHistory: redo failed', err)
      return false
    }
  }, [enabled, diffChangedContent])

  return { applyCommand, undo, redo, canUndo, canRedo }
}
