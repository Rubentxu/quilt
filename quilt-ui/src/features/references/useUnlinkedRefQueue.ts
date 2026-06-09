// ──── useUnlinkedRefQueue ────────────────────────────────────────
//
// React glue around the pure helpers in `./unlinkedRefQueue`. The
// hook owns three things:
//
//   1. Lifecycle — when `pageName` changes, kick off a re-scan.
//   2. Reconciliation — merge the persisted queue (localStorage)
//      with a fresh backend search so candidates that have already
//      been linked or that the user deleted no longer show up.
//   3. Actions — `link` (PUT to the block) and `dismiss` (drop
//      from the persisted queue). Both are exposed to UI code so
//      BacklinksPanel and UnlinkedRefQueue can call them.
//
// We talk to the backend through the shared `api` client. The only
// call we need is `api.searchBlocks`, which already returns blocks
// whose `content` includes the query — we then run `detectMentions`
// locally so the queue only contains plain-prose hits (not ones
// already wrapped in `[[ ... ]]`).

import { useCallback, useEffect, useRef, useState } from 'react'
import toast from 'react-hot-toast'
import { api } from '@core/api-client'
import {
  detectMentions,
  linkifyMention,
  loadQueue,
  saveQueue,
  removeCandidate,
  type UnlinkedCandidate,
} from './unlinkedRefQueue'

/**
 * Max blocks we ask the backend to scan for mentions. The search
 * endpoint returns relevance-scored hits, so 200 is plenty for a
 * typical workspace while still bounded.
 */
const SCAN_LIMIT = 200

export interface UseUnlinkedRefQueue {
  /** Persisted candidates for the current page. */
  queue: UnlinkedCandidate[]
  /** True while a backend scan is in flight. */
  loading: boolean
  /** Error from the last scan, if any (cleared on next success). */
  error: string | null
  /**
   * Promote a candidate to a real link. Wraps the mention in
   * `[[ ... ]]` and PUTs the new content via `api.updateBlock`.
   * On success the candidate is dropped from the queue.
   */
  link: (candidate: UnlinkedCandidate) => Promise<void>
  /**
   * Remove a candidate from the queue. Stays dismissed across
   * re-scans for the same block+position.
   */
  dismiss: (candidate: UnlinkedCandidate) => void
  /**
   * Force a fresh backend scan. Useful when the user edits a block
   * and we want to refresh the queue without waiting for the next
   * page navigation.
   */
  refresh: () => Promise<void>
}

/**
 * Hook entry point. Pass `null` to disable the queue (no page
 * loaded yet) — the hook stays inert and returns an empty queue.
 */
export function useUnlinkedRefQueue(pageName: string | null): UseUnlinkedRefQueue {
  const [queue, setQueue] = useState<UnlinkedCandidate[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Refs that mirror state so the async rescan effect can read the
  // latest values without re-creating the effect on every render.
  const pageNameRef = useRef(pageName)
  pageNameRef.current = pageName
  const mountedRef = useRef(true)

  const scan = useCallback(async (target: string) => {
    setLoading(true)
    setError(null)
    try {
      // 1. List every page so we can also pull aliases / canonical
      //    names from the same source. The page we're scanning FOR
      //    is `target`; everything else is just a hint in case we
      //    want to extend the heuristic later.
      //    (For V1 the page name is the only mention trigger.)
      const pages = await api.listPages().catch(() => [])
      const targetPage = pages.find((p) => p.name === target)
      if (!targetPage) {
        // Page doesn't exist on the backend (e.g. it was deleted).
        // The persisted queue is now stale — clear it.
        if (mountedRef.current) {
          setQueue([])
          saveQueue([])
        }
        return
      }

      // 2. Backend search returns blocks whose `content` includes
      //    the page name. We use a quoted query so the search
      //    engine treats it as a literal substring.
      const results = await api.searchBlocks(`"${target}"`, SCAN_LIMIT).catch(() => [])
      // 3. Run the local detector on every result so the queue
      //    contains ONLY plain-prose mentions (anything already
      //    inside [[...]] is filtered out).
      const candidates: UnlinkedCandidate[] = []
      for (const r of results) {
        const hits = detectMentions(r.content, target)
        for (const h of hits) {
          candidates.push({ ...h, blockId: r.blockId })
        }
      }
      // 4. Merge strategy: the persisted queue is the user's
      //    current "todo list" of unlinked refs (a dismissed
      //    candidate has already been removed from it). The new
      //    detection is what the backend *currently* thinks is
      //    unlinked. The persisted queue wins for entries that
      //    match; new detections are appended.
      const persisted = loadQueue()
      const persistedKeys = new Set(persisted.map((c) => `${c.blockId}:${c.position}`))
      const newKeys = new Set(candidates.map((c) => `${c.blockId}:${c.position}`))
      const next: UnlinkedCandidate[] = [
        // Keep all persisted entries that the backend still sees
        // as unlinked (the user hasn't actioned them yet).
        ...persisted.filter((c) => newKeys.has(`${c.blockId}:${c.position}`)),
        // Add any fresh detections the user hasn't seen yet.
        ...candidates.filter((c) => !persistedKeys.has(`${c.blockId}:${c.position}`)),
      ]
      if (mountedRef.current && pageNameRef.current === target) {
        setQueue(next)
        saveQueue(next)
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to scan for unlinked references'
      if (mountedRef.current) setError(msg)
    } finally {
      if (mountedRef.current) setLoading(false)
    }
  }, [])

  // Kick off a scan whenever the target page changes. We also seed
  // the queue with whatever is already in localStorage so the UI
  // shows something on the first paint instead of an empty state
  // for the brief moment the scan takes.
  useEffect(() => {
    mountedRef.current = true
    if (!pageName) {
      setQueue([])
      setLoading(false)
      return
    }
    // Seed from localStorage immediately
    setQueue(loadQueue())
    void scan(pageName)
    return () => {
      mountedRef.current = false
    }
  }, [pageName, scan])

  const dismiss = useCallback((candidate: UnlinkedCandidate) => {
    setQueue((prev) => prev.filter((c) => !(c.blockId === candidate.blockId && c.position === candidate.position)))
    removeCandidate(candidate.blockId, candidate.position)
  }, [])

  const link = useCallback(
    async (candidate: UnlinkedCandidate) => {
      // We need the current block content to splice `[[...]]` in.
      // The search endpoint already returned a snippet, but the
      // full content is required for an accurate splice. We
      // re-search the block to get it.
      try {
        const results = await api.searchBlocks(`"${candidate.pageName}"`, SCAN_LIMIT)
        const match = results.find((r) => r.blockId === candidate.blockId)
        if (!match) {
          toast.error('Block no longer exists — refreshing queue')
          dismiss(candidate)
          return
        }
        const next = linkifyMention(match.content, candidate)
        if (next === match.content) {
          toast.error('Block changed — refreshing queue')
          dismiss(candidate)
          return
        }
        await api.updateBlock(candidate.blockId, { content: next })
        // Promote succeeded — drop the candidate.
        setQueue((prev) =>
          prev.filter((c) => !(c.blockId === candidate.blockId && c.position === candidate.position)),
        )
        removeCandidate(candidate.blockId, candidate.position)
        toast.success('Linked')
      } catch (e) {
        const msg = e instanceof Error ? e.message : 'Failed to link'
        toast.error(msg)
        // Keep the candidate so the user can retry
      }
    },
    [dismiss],
  )

  const refresh = useCallback(async () => {
    if (pageNameRef.current) await scan(pageNameRef.current)
  }, [scan])

  return { queue, loading, error, link, dismiss, refresh }
}
