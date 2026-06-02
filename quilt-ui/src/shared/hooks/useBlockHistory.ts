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
 */
export function useBlockHistory({
  pageName,
  blocks,
  onBlocksChanged,
  enabled = true,
  maxSize: _maxSize = 100,
}: UseBlockHistoryOptions): UseBlockHistoryResult {
  const stackIdRef = useRef<number | null>(null)
  const [canUndo, setCanUndo] = useState(false)
  const [canRedo, setCanRedo] = useState(false)
  const onBlocksChangedRef = useRef(onBlocksChanged)
  onBlocksChangedRef.current = onBlocksChanged

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
      setCanUndo(wasmHistoryCanUndo(id))
      setCanRedo(wasmHistoryCanRedo(id))
      return true
    } catch (err) {
      console.error('useBlockHistory: undo failed', err)
      return false
    }
  }, [enabled])

  // ── Redo ──
  const redo = useCallback((): boolean => {
    const id = stackIdRef.current
    if (!enabled || id === null) return false
    try {
      const newBlocks = wasmHistoryRedo(id)
      if (newBlocks == null) return false
      onBlocksChangedRef.current(newBlocks)
      setCanUndo(wasmHistoryCanUndo(id))
      setCanRedo(wasmHistoryCanRedo(id))
      return true
    } catch (err) {
      console.error('useBlockHistory: redo failed', err)
      return false
    }
  }, [enabled])

  return { applyCommand, undo, redo, canUndo, canRedo }
}
