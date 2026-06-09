import { useState, useEffect, useRef, useMemo } from 'react'
import { Search, FileText, Calendar, Hash, X, BookmarkPlus, History, Bookmark, Trash2 } from 'lucide-react'
import { useNavigate } from '@tanstack/react-router'
import { api } from '@core/api-client'
import type { Page, SearchResult } from '@shared/types/api'
import toast from 'react-hot-toast'
import { SaveAsViewModal, type SaveAsViewRequest } from './SaveAsViewModal'
import {
  loadRecentSearches,
  loadSavedSearches,
  recordRecentSearch,
  addSavedSearch,
  deleteSavedSearch,
  SAVED_SEARCH_VIEW_TYPES,
  type RecentSearch,
  type SavedSearch,
  type SavedSearchViewType,
} from './savedSearches'

interface SearchModalProps {
  isOpen: boolean
  onClose: () => void
}

const PAGE_LIMIT = 10
const BLOCK_LIMIT = 10

// ──── Property filter parsing ───────────────────────────────────────
//
// The FTS5 endpoint doesn't understand property syntax (`status:: todo`),
// so we let the user type short filters (`status:todo`) client-side and
// post-filter the FTS results by regex-matching the block content.
//
// Keys are case-insensitive on input and matched against the keys below.
// Anything else (e.g. `author:foo`) is treated as free text and passed
// to FTS5 verbatim.

const SUPPORTED_FILTER_KEYS = [
  'status',
  'priority',
  'created_by',
  'card-shape',
  'card_shape',
] as const

type FilterKey = (typeof SUPPORTED_FILTER_KEYS)[number]

/** A single property filter parsed from the user's query. */
export interface PropertyFilter {
  key: FilterKey
  value: string
}

/** Result of splitting a query into the FTS text part and any filters. */
export interface ParsedQuery {
  text: string
  filters: PropertyFilter[]
}

/**
 * Split a raw query into the FTS text part and any `key:value` property
 * filters. Whitespace-separated tokens matching `<key>:<value>` where
 * `<key>` is one of {@link SUPPORTED_FILTER_KEYS} become filters; the
 * rest stays as free text. The value keeps everything after the colon
 * (whitespace inside the token is not allowed — `status: todo` is two
 * tokens and does NOT match).
 */
export function parseQuery(query: string): ParsedQuery {
  const tokens = query.split(/\s+/).filter(Boolean)
  const filters: PropertyFilter[] = []
  const textTokens: string[] = []

  for (const token of tokens) {
    const match = token.match(/^([a-z][a-z0-9_-]*):(.+)$/i)
    if (match) {
      const key = match[1].toLowerCase() as FilterKey
      if ((SUPPORTED_FILTER_KEYS as readonly string[]).includes(key)) {
        filters.push({ key, value: match[2] })
        continue
      }
    }
    textTokens.push(token)
  }

  return { text: textTokens.join(' ').trim(), filters }
}

/**
 * Test whether a block's content includes the property `key:: value`
 * or `key: value` (case-insensitive). Used to post-filter FTS results
 * client-side. The match requires a word boundary before the key so
 * that `substatus:: todo` does NOT satisfy a filter on `status`.
 */
export function blockMatchesFilter(content: string, filter: PropertyFilter): boolean {
  const escapedKey = filter.key.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  const escapedValue = filter.value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  const pattern = new RegExp(
    `(?:^|\\s)${escapedKey}(?:::|:)\\s*${escapedValue}(?=\\s|$|[,;])`,
    'i',
  )
  return pattern.test(content)
}

// ──── Search-DSL reconstruction ────────────────────────────────────
//
// ROADMAP #25 ("Save as View" desde search) needs to materialise the
// current search into a reusable `type:: query` block. The FTS
// query we sent to the backend is the free-text part of the parsed
// query; the property filters were applied client-side. To make the
// saved view reproducible by anyone — including the QueryBlock
// renderer that will eventually execute the DSL — we round-trip the
// filters into the Logseq-style `key:: value` property syntax and
// concatenate everything with single spaces.
//
//   parseQuery("task status:todo priority:A")
//     → { text: "task", filters: [status:todo, priority:A] }
//   buildSearchDsl(...) → "task status:: todo priority:: A"
//
// The output is the value we store on the new query block's `dsl::`
// property.

/**
 * Build a search DSL string from a parsed query. Free text comes
 * first, then each filter as `key:: value` (Logseq property syntax),
 * joined by single spaces. The result is `''` for an empty query
 * and is always trimmed.
 */
export function buildSearchDsl(parsed: ParsedQuery): string {
  const parts: string[] = []
  const text = parsed.text.trim()
  if (text) parts.push(text)
  for (const f of parsed.filters) {
    parts.push(`${f.key}:: ${f.value}`)
  }
  return parts.join(' ').trim()
}

/**
 * One row in the unified result list. The modal flattens page and block
 * results into a single array so arrow-key navigation feels natural
 * (Quilt parity). Section headers are non-navigable dividers that the
 * keyboard skips.
 */
type ResultItem =
  | { kind: 'page'; page: Page; id: string }
  | { kind: 'block'; block: SearchResult; id: string }

/** Storage key used to hand off a "focus this block" request to PageView
 *  after navigation. Survives HMR and is cleared on read. */
export const FOCUS_BLOCK_STORAGE_KEY = 'quilt:focusBlock'

export function SearchModal({ isOpen, onClose }: SearchModalProps) {
  const [query, setQuery] = useState('')
  const [pageResults, setPageResults] = useState<Page[]>([])
  const [blockResults, setBlockResults] = useState<SearchResult[]>([])
  const [selectedIndex, setSelectedIndex] = useState(0)
  const [loading, setLoading] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)
  const navigate = useNavigate()

  // ── Saved/Recent searches (ROADMAP #22) ────────────────────────
  //
  // The user can save explicit named searches (persisted under the
  // `saved-searches` localStorage key) and the modal also auto-records
  // the last few queries they ran under `recent-searches` (FIFO-capped
  // at 10). Both lists render as panels in the modal when the input
  // is empty; otherwise the normal search results take over.
  const [recentSearches, setRecentSearches] = useState<RecentSearch[]>([])
  const [savedSearches, setSavedSearches] = useState<SavedSearch[]>([])

  // Save-search inline form state. When `saveFormOpen` is true, a
  // small row appears next to the save button with a name input and
  // a view-type selector.
  const [saveFormOpen, setSaveFormOpen] = useState(false)
  const [saveName, setSaveName] = useState('')
  const [saveViewType, setSaveViewType] = useState<SavedSearchViewType | ''>('')

  // Refs that survive across renders without triggering re-renders.
  // `lastExecutedQuery` captures the query string at the moment a
  // search actually fired (i.e. the debounce timer settled) so we
  // don't auto-record empty strings or duplicates during typing.
  const lastExecutedQueryRef = useRef('')
  const lastResultCountRef = useRef(0)

  // ── Save-as-View state (ROADMAP #25) ──────────────────────────
  //
  // The user clicks the "Save as View" button on a result row; we
  // remember which result they picked, then mount `SaveAsViewModal`
  // with the same page list we already loaded. We keep the entire
  // `ParsedQuery` in state so the modal's submit handler can rebuild
  // the DSL after the user picks a name/type/page.
  const [saveViewTarget, setSaveViewTarget] = useState<
    | {
        item: ResultItem
        parsed: ParsedQuery
        pages: Page[]
      }
    | null
  >(null)
  const [saveViewSubmitting, setSaveViewSubmitting] = useState(false)
  const [saveViewError, setSaveViewError] = useState<string | null>(null)

  // Parse the query into the FTS text part and any property filters.
  // Memoized on `query` so the array reference stays stable across
  // re-renders that don't change the query — that keeps the search
  // useEffect's debounce timer from being reset on every render.
  const parsed = useMemo(() => parseQuery(query), [query])
  const filters = parsed.filters
  const hasFilters = filters.length > 0

  // Flatten pages + blocks into a single navigable list. Sections are
  // rendered as headers but don't participate in the keyboard cursor.
  const items: ResultItem[] = [
    ...pageResults.map(p => ({ kind: 'page' as const, page: p, id: `page:${p.id}` })),
    ...blockResults.map(b => ({ kind: 'block' as const, block: b, id: `block:${b.blockId}` })),
  ]

  // Focus input when opening
  useEffect(() => {
    if (isOpen) {
      // Reset state on open
      setQuery('')
      setPageResults([])
      setBlockResults([])
      setSelectedIndex(0)
      // Reset the save-form state so a stale "Save search" row from
      // a previous open doesn't bleed into the new one.
      setSaveFormOpen(false)
      setSaveName('')
      setSaveViewType('')
      // Reload the persisted histories every time the modal opens
      // so external writes (e.g. the delete button re-renders) and
      // multi-tab edits are reflected.
      setRecentSearches(loadRecentSearches())
      setSavedSearches(loadSavedSearches())
      // Use RAF to ensure DOM is ready
      const raf = requestAnimationFrame(() => inputRef.current?.focus())
      return () => cancelAnimationFrame(raf)
    }
  }, [isOpen])

  // Record the just-executed query into the recent-searches history
  // whenever the modal transitions to `isOpen === false`. We do this
  // in an effect (not in the close handler) so it covers EVERY close
  // path uniformly:
  //   - ESC button / backdrop click → handleClose → onClose → isOpen false
  //   - Parent flips isOpen directly (e.g. AppShell keyboard shortcut)
  //   - Selecting a result navigates away and the parent unmounts us
  // If we did this in `handleClose` only, parent-driven closes would
  // never record to recents.
  useEffect(() => {
    if (isOpen) return
    const executed = lastExecutedQueryRef.current
    if (executed.trim()) {
      recordRecentSearch(
        { query: executed, resultCount: lastResultCountRef.current },
        Date.now(),
      )
    }
    // Reset for the next open. The in-memory copy is refreshed on
    // the next mount by the open-effect above.
    lastExecutedQueryRef.current = ''
    lastResultCountRef.current = 0
  }, [isOpen])

  // Search with debounce — shows recent pages when query is empty.
  //
  // When the query is non-empty we issue BOTH calls in parallel:
  //   - listPages() filtered client-side (instant, no FTS round-trip
  //     for what is usually a small page set)
  //   - searchBlocks() for FTS over block content (the new G3 wiring)
  // The two result sets render in distinct sections and the user can
  // arrow-key between them.
  useEffect(() => {
    const trimmed = query.trim()

    if (!trimmed) {
      setLoading(true)
      api.listPages()
        .then(pages => {
          setPageResults(pages.slice(0, PAGE_LIMIT))
          setBlockResults([])
        })
        .catch(() => {
          // silently fail for initial load
        })
        .finally(() => setLoading(false))
      return
    }

    setLoading(true)
    const timer = setTimeout(async () => {
      try {
        // FTS5 receives only the free-text part of the query. When the
        // user typed ONLY filters (e.g. `status:todo`), fall back to the
        // first filter's value so block search still produces a result
        // set to post-filter. If there are no filters and no text, the
        // FTS call is skipped entirely (the empty-query branch above
        // already covered the no-input case).
        const ftsQuery = parsed.text || (hasFilters ? filters[0].value : '')

        // Run both calls in parallel. We intentionally don't `await`
        // them sequentially because the page filter is fast and the
        // FTS call is the slow one — kicking them off together means
        // the UI updates as soon as either settles.
        const [pages, blocks] = await Promise.all([
          api.listPages().catch(() => [] as Page[]),
          ftsQuery
            ? api.searchBlocks(ftsQuery, BLOCK_LIMIT).catch(() => [] as SearchResult[])
            : Promise.resolve([] as SearchResult[]),
        ])

        // Post-filter blocks by the parsed property filters. FTS5
        // doesn't understand property syntax, so the filter is a
        // client-side regex on the block's raw content. ALL filters
        // must match (AND semantics).
        const filteredBlocks = filters.length > 0
          ? blocks.filter(b => filters.every(f => blockMatchesFilter(b.content, f)))
          : blocks

        const q = parsed.text.toLowerCase()
        const filteredPages = pages
          .filter(
            p => p.name.toLowerCase().includes(q) || (p.title && p.title.toLowerCase().includes(q)),
          )
          // Sort by most recently updated (proxy: createdAt desc, since
          // the Page DTO doesn't expose updatedAt yet). ISO-8601 date
          // strings sort correctly via localeCompare.
          .sort((a, b) => b.createdAt.localeCompare(a.createdAt))
          .slice(0, PAGE_LIMIT)

        setPageResults(filteredPages)
        setBlockResults(filteredBlocks)
        setSelectedIndex(0)

        // Remember what the user actually ran (and how many results
        // it produced) so the close-handler can write one entry to
        // the recent-searches history. We capture the RAW trimmed
        // query — not the parsed text — so the user sees the same
        // string they typed reappear in the recents list later.
        lastExecutedQueryRef.current = trimmed
        lastResultCountRef.current = filteredPages.length + filteredBlocks.length
      } catch (e) {
        toast.error('Search failed')
      } finally {
        setLoading(false)
      }
    }, 200)

    return () => clearTimeout(timer)
  }, [query, parsed, hasFilters, filters])

  // Keyboard navigation
  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setSelectedIndex(i => Math.min(i + 1, items.length - 1))
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setSelectedIndex(i => Math.max(i - 1, 0))
    } else if (e.key === 'Enter' && items[selectedIndex]) {
      e.preventDefault()
      selectItem(items[selectedIndex])
    } else if (e.key === 'Escape') {
      handleClose()
    }
  }

  /**
   * Remove one of the active property filters by index. Reconstructs
   * the query string from the free-text part + the surviving filters,
   * so the user sees the filter disappear from both the chip row and
   * the input. Text comes first to match the natural order the user
   * originally typed (e.g. `hello status:todo priority:A`).
   */
  function removeFilter(index: number) {
    const remaining = filters.filter((_, i) => i !== index)
    const parts = [
      parsed.text,
      ...remaining.map(f => `${f.key}:${f.value}`),
    ].filter(Boolean)
    setQuery(parts.join(' '))
  }

  /**
   * Close the modal. Recording into the recent-searches history is
   * handled by the `isOpen`-watching effect above, so all this helper
   * does is notify the parent (which will flip `isOpen` to `false`).
   * Centralising the close path here means the ESC button, the
   * backdrop click, and any future "close from inside" affordance
   * all share a single function — easier to test, easier to extend.
   */
  function handleClose() {
    onClose()
  }

  // ── Saved/Recent click handlers (ROADMAP #22) ───────────────────
  //
  // Clicking a recent or saved search puts its query back into the
  // input. The next render of the search effect will fire a fresh
  // debounced FTS call and the result area will switch from the
  // "Recent / Saved" panel back to the normal "Pages / Blocks" view.

  function executeRecentSearch(q: string) {
    setQuery(q)
    setSelectedIndex(0)
    // Focus the input so the user can keep typing to refine the
    // search without first clicking back into it.
    inputRef.current?.focus()
  }

  function executeSavedSearch(s: SavedSearch) {
    setQuery(s.query)
    setSelectedIndex(0)
    inputRef.current?.focus()
  }

  function removeSavedSearch(id: string) {
    deleteSavedSearch(id)
    setSavedSearches(loadSavedSearches())
  }

  /**
   * Confirm the save-search form. Validates the name and the (still-
   * typed) input, then writes a new saved-search entry. Resets the
   * inline form on success. Refuses to do anything when the name or
   * the current query is empty.
   */
  function confirmSaveSearch() {
    const name = saveName.trim()
    const q = trimmedQuery
    if (!name || !q) {
      // The test for "refuses to save when the name is empty" expects
      // the click to be a no-op. We could toast an error, but the
      // spec keeps the UX minimal — the disabled state on the confirm
      // button communicates the same thing.
      return
    }
    addSavedSearch(
      {
        name,
        query: q,
        viewType: saveViewType === '' ? undefined : saveViewType,
      },
      makeSavedSearchId,
      () => Date.now(),
    )
    setSavedSearches(loadSavedSearches())
    setSaveFormOpen(false)
    setSaveName('')
    setSaveViewType('')
  }

  /**
   * Stable, locally-generated id for a saved search. We prefer
   * `crypto.randomUUID()` (available in modern browsers and the
   * jsdom shim) and fall back to a Math.random-based shape so the
   * function never throws in older environments.
   */
  function makeSavedSearchId(): string {
    const g = globalThis as { crypto?: { randomUUID?: () => string } }
    if (g.crypto && typeof g.crypto.randomUUID === 'function') {
      return g.crypto.randomUUID()
    }
    return `s-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`
  }

  function selectItem(item: ResultItem) {
    // Record the executed query into recents BEFORE we tear down the
    // modal — otherwise the lastExecutedQueryRef would still hold the
    // right value, but routing through handleClose keeps the close
    // path single-sourced and easy to test.
    handleClose()
    if (item.kind === 'page') {
      const page = item.page
      if (page.journal && page.journalDay) {
        // Convert journalDay (YYYYMMDD integer) to YYYY-MM-DD string
        const day = page.journalDay.toString()
        const date = `${day.slice(0, 4)}-${day.slice(4, 6)}-${day.slice(6, 8)}`
        navigate({ to: '/journal/$date', params: { date } })
      } else {
        navigate({ to: '/page/$name', params: { name: page.name } })
      }
    } else {
      const block = item.block
      if (block.pageName) {
        // Hand off the focus request via sessionStorage. PageView reads
        // this on mount, focuses the block, then clears the key. The
        // sessionStorage channel is used in preference to URL search
        // params because the openTab/id dance in PageViewPage makes
        // param-based handoff timing-sensitive.
        sessionStorage.setItem(FOCUS_BLOCK_STORAGE_KEY, block.blockId)
        navigate({ to: '/page/$name', params: { name: block.pageName } })
      }
    }
  }

  // ── Save as View (ROADMAP #25) ────────────────────────────────
  //
  // The user clicked the per-row "Save as View" button. We remember
  // which row they picked (used to keep the modal scoped to the
  // right result) and capture the current parsed query so the
  // submit handler can rebuild the DSL.
  //
  // We also fetch the FULL page list (unfiltered) for the page
  // selector — the user should be able to save the view into ANY
  // page in the graph, not just the ones whose name happens to
  // match the search query. While the fetch is in flight we open
  // the modal with an empty list and let the user wait; the
  // selector renders nothing rather than confusing them with a
  // truncated list.

  function openSaveAsView(item: ResultItem) {
    setSaveViewError(null)
    setSaveViewTarget({ item, parsed, pages: [] })
    api.listPages()
      .then(pages => {
        setSaveViewTarget(prev => (prev && prev.item === item ? { ...prev, pages } : prev))
      })
      .catch(() => {
        // Fail silently — the user can cancel and try again, or
        // the saved-view modal will show 0 pages (which makes the
        // page selector useless). The submit button is disabled
        // when `pageName` is empty, so the user can't accidentally
        // create a view in a phantom page.
        setSaveViewError('Could not load the page list. Please try again.')
      })
  }

  function closeSaveAsView() {
    if (saveViewSubmitting) return
    setSaveViewTarget(null)
    setSaveViewError(null)
  }

  /**
   * Build a `type:: query` block carrying the current search as
   * a DSL, then a `type:: view` block referencing it. The two calls
   * are sequential because the view's `data-source::` needs the
   * query block's UUID. If the first call fails, the view is not
   * created and the modal stays open with the error visible.
   */
  async function handleSaveAsViewConfirm(req: SaveAsViewRequest) {
    if (!saveViewTarget) return
    setSaveViewSubmitting(true)
    setSaveViewError(null)
    try {
      const dsl = buildSearchDsl(saveViewTarget.parsed)
      // Use the original free-text as the block's content so the
      // query block reads as "the search the user just ran" in the
      // outliner. Empty when the query was filter-only.
      const content = saveViewTarget.parsed.text.trim()

      const queryBlock = await api.createBlock({
        pageName: req.pageName,
        content,
        properties: {
          type: 'query',
          dsl,
        },
      })

      await api.createBlock({
        pageName: req.pageName,
        content: '',
        properties: {
          type: 'view',
          'view-type': req.viewType,
          'view-name': req.name,
          'data-source': queryBlock.id,
        },
      })

      setSaveViewTarget(null)
      toast.success(`View "${req.name}" saved to ${req.pageName}`)
    } catch (e) {
      const message =
        e instanceof Error ? e.message : 'Failed to save view'
      setSaveViewError(message)
    } finally {
      setSaveViewSubmitting(false)
    }
  }

  if (!isOpen) return null

  // Compute the running index of the current item so we can highlight
  // it. Items in the same section share an offset that gets bumped as
  // we walk the list.
  const showPageSection = pageResults.length > 0
  const showBlockSection = blockResults.length > 0
  const noResults = !loading && !showPageSection && !showBlockSection && query.length > 0
  const isFirstLoad = loading && items.length === 0

  // ── Saved/Recent visibility (ROADMAP #22) ──────────────────────
  //
  // The "Recent / Saved" panel only takes over the result area when
  // the user has at least one entry to show. If both lists are empty
  // we fall back to the regular "Pages" listing — that preserves the
  // behaviour of the empty-input branch (which the existing tests
  // rely on: opening the modal with no history still shows the page
  // list).
  const trimmedQuery = query.trim()
  const showSavedRecentPanel =
    trimmedQuery.length === 0 &&
    (recentSearches.length > 0 || savedSearches.length > 0)

  // The "Save search" affordance is shown whenever the user has run
  // a non-empty search that produced at least one result. The form
  // it opens is inline (name input + viewType select) and persists
  // to the `saved-searches` localStorage key.
  const showSaveSearchAffordance =
    trimmedQuery.length > 0 && (showPageSection || showBlockSection)

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 100,
        display: 'flex',
        alignItems: 'flex-start',
        justifyContent: 'center',
        paddingTop: '15vh',
        background: 'rgba(0, 0, 0, 0.4)',
      }}
      onClick={onClose}
    >
      <div
        style={{
          width: '100%',
          maxWidth: '640px',
          background: 'var(--color-surface)',
          borderRadius: 'var(--radius-lg)',
          boxShadow: 'var(--shadow-lg)',
          overflow: 'hidden',
        }}
        onClick={e => e.stopPropagation()}
      >
        {/* ─── Search input ─── */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            padding: 'var(--space-3) var(--space-4)',
            borderBottom: '1px solid var(--color-border)',
            gap: 'var(--space-3)',
          }}
        >
          <Search size={18} style={{ color: 'var(--color-text-muted)', flexShrink: 0 }} />
          <input
            ref={inputRef}
            value={query}
            onChange={e => { setQuery(e.target.value); setSelectedIndex(0) }}
            onKeyDown={handleKeyDown}
            placeholder="Search pages and blocks…"
            style={{
              flex: 1,
              border: 'none',
              outline: 'none',
              fontSize: '16px',
              background: 'transparent',
              color: 'var(--color-text-primary)',
            }}
          />
          <button
            onClick={handleClose}
            style={{
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              color: 'var(--color-text-muted)',
              fontSize: '12px',
              fontFamily: 'var(--font-family)',
            }}
          >
            ESC
          </button>
        </div>

        {/* ─── Filter chips ───
         *
         * Rendered between the input and the result list. Each parsed
         * filter becomes a removable pill. The X button is exposed via
         * aria-label so screen readers and tests can target it.
         */}
        {hasFilters && (
          <div
            data-testid="filter-chips"
            style={{
              display: 'flex',
              flexWrap: 'wrap',
              gap: 'var(--space-2)',
              padding: 'var(--space-2) var(--space-4)',
              borderBottom: '1px solid var(--color-border)',
              background: 'var(--color-surface-subtle)',
            }}
          >
            {filters.map((f, i) => (
              <span
                key={`${f.key}:${f.value}:${i}`}
                data-testid={`filter-chip-${f.key}-${i}`}
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: 'var(--space-1)',
                  padding: '2px var(--space-2)',
                  background: 'var(--color-accent-bg, var(--color-surface))',
                  color: 'var(--color-accent, var(--color-text-primary))',
                  border: '1px solid var(--color-accent, var(--color-border))',
                  borderRadius: 'var(--radius-pill)',
                  fontSize: 'var(--font-size-micro)',
                  fontWeight: 600,
                }}
              >
                {f.key}:{f.value}
                <button
                  onClick={() => removeFilter(i)}
                  aria-label={`Remove filter ${f.key}:${f.value}`}
                  data-testid={`filter-chip-remove-${f.key}-${i}`}
                  style={{
                    background: 'none',
                    border: 'none',
                    cursor: 'pointer',
                    color: 'inherit',
                    display: 'inline-flex',
                    alignItems: 'center',
                    padding: 0,
                    marginLeft: '2px',
                    lineHeight: 1,
                  }}
                >
                  <X size={12} />
                </button>
              </span>
            ))}
          </div>
        )}

        {/* ─── Results ─── */}
        <div style={{ maxHeight: '400px', overflowY: 'auto' }}>
          {isFirstLoad && (
            <div
              style={{
                padding: 'var(--space-8)',
                textAlign: 'center',
                color: 'var(--color-text-muted)',
                fontSize: '14px',
              }}
            >
              Searching…
            </div>
          )}

          {noResults && (
            <div
              style={{
                padding: 'var(--space-8)',
                textAlign: 'center',
                color: 'var(--color-text-muted)',
                fontSize: '14px',
              }}
            >
              No results for &quot;{query}&quot;
            </div>
          )}

          {/* ─── Saved / Recent searches panel (ROADMAP #22) ───
           *
           * Rendered in place of the "Pages" / "Blocks" sections when
           * the input is empty AND we have something to show. The
           * saved-searches section always comes first (the user
           * deliberately curated those), then the recents.
           */}
          {showSavedRecentPanel && (
            <>
              {savedSearches.length > 0 && (
                <>
                  <SectionHeader label="Saved searches" />
                  {savedSearches.map(s => (
                    <SavedSearchRow
                      key={s.id}
                      saved={s}
                      onClick={() => executeSavedSearch(s)}
                      onDelete={() => removeSavedSearch(s.id)}
                    />
                  ))}
                </>
              )}

              {recentSearches.length > 0 && (
                <>
                  <SectionHeader label="Recent searches" />
                  {recentSearches.map((r, i) => (
                    <RecentSearchRow
                      key={`${r.query}-${i}`}
                      recent={r}
                      onClick={() => executeRecentSearch(r.query)}
                    />
                  ))}
                </>
              )}
            </>
          )}

          {/* ─── Pages section ─── */}
          {!showSavedRecentPanel && showPageSection && (
            <SectionHeader label="Pages" />
          )}
          {!showSavedRecentPanel && showPageSection && pageResults.map((page, idx) => {
            const itemIndex = idx
            return (
              <ResultButton
                key={`page:${page.id}`}
                testId={`result-row-page-${page.id}`}
                selected={itemIndex === selectedIndex}
                onClick={() => selectItem({ kind: 'page', page, id: `page:${page.id}` })}
                onMouseEnter={() => setSelectedIndex(itemIndex)}
                icon={
                  page.journal ? (
                    <Calendar size={16} style={{ flexShrink: 0, color: 'var(--color-accent)' }} />
                  ) : (
                    <FileText size={16} style={{ flexShrink: 0, color: 'var(--color-text-muted)' }} />
                  )
                }
                title={page.title || page.name}
                badge={page.journal ? 'Journal' : null}
                trailing={
                  <SaveAsViewButton
                    testId={`save-as-view-page-${page.id}`}
                    onClick={() => openSaveAsView({ kind: 'page', page, id: `page:${page.id}` })}
                  />
                }
              />
            )
          })}

          {/* ─── Blocks section ─── */}
          {!showSavedRecentPanel && showBlockSection && (
            <SectionHeader label="Blocks" />
          )}
          {!showSavedRecentPanel && showBlockSection && blockResults.map((block, idx) => {
            const itemIndex = pageResults.length + idx
            const preview = block.snippet || block.content
            return (
              <ResultButton
                key={`block:${block.blockId}`}
                testId={`result-row-block-${block.blockId}`}
                selected={itemIndex === selectedIndex}
                onClick={() => selectItem({ kind: 'block', block, id: `block:${block.blockId}` })}
                onMouseEnter={() => setSelectedIndex(itemIndex)}
                icon={<Hash size={16} style={{ flexShrink: 0, color: 'var(--color-text-muted)' }} />}
                title={preview.length > 80 ? `${preview.slice(0, 80)}…` : preview || '(empty block)'}
                subtitle={block.pageName || undefined}
                badge="Block"
                trailing={
                  <SaveAsViewButton
                    testId={`save-as-view-block-${block.blockId}`}
                    onClick={() => openSaveAsView({ kind: 'block', block, id: `block:${block.blockId}` })}
                  />
                }
              />
            )
          })}

          {/* ─── Save-search affordance (ROADMAP #22) ───
           *
           * Rendered below the result list when the user has just
           * executed a non-empty search that produced at least one
           * hit. Clicking the button toggles the inline save form.
           * The form stays open across re-renders until the user
           * confirms or cancels.
           */}
          {showSaveSearchAffordance && !saveFormOpen && (
            <div
              data-testid="save-search-bar"
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                gap: 'var(--space-3)',
                padding: 'var(--space-2) var(--space-4)',
                borderTop: '1px solid var(--color-border)',
                background: 'var(--color-surface-subtle)',
              }}
            >
              <span
                style={{
                  fontSize: 'var(--font-size-caption)',
                  color: 'var(--color-text-muted)',
                }}
              >
                Save this search
              </span>
              <button
                type="button"
                data-testid="save-search-button"
                onClick={() => setSaveFormOpen(true)}
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: '4px',
                  padding: '4px var(--space-3)',
                  background: 'var(--color-accent, var(--color-surface))',
                  color: 'var(--color-surface, var(--color-text-primary))',
                  border: 'none',
                  borderRadius: 'var(--radius-sm)',
                  fontSize: 'var(--font-size-caption)',
                  fontWeight: 600,
                  cursor: 'pointer',
                }}
              >
                <BookmarkPlus size={12} />
                Save
              </button>
            </div>
          )}

          {showSaveSearchAffordance && saveFormOpen && (
            <div
              data-testid="save-search-form"
              style={{
                display: 'flex',
                flexDirection: 'column',
                gap: 'var(--space-2)',
                padding: 'var(--space-3) var(--space-4)',
                borderTop: '1px solid var(--color-border)',
                background: 'var(--color-surface-subtle)',
              }}
            >
              <input
                type="text"
                data-testid="save-search-name-input"
                value={saveName}
                onChange={e => setSaveName(e.target.value)}
                placeholder="Name this search…"
                style={{
                  padding: '6px var(--space-2)',
                  border: '1px solid var(--color-border)',
                  borderRadius: 'var(--radius-sm)',
                  fontSize: '14px',
                  background: 'var(--color-surface)',
                  color: 'var(--color-text-primary)',
                }}
              />
              <select
                data-testid="save-search-viewtype-select"
                value={saveViewType}
                onChange={e =>
                  setSaveViewType(e.target.value as SavedSearchViewType | '')
                }
                style={{
                  padding: '6px var(--space-2)',
                  border: '1px solid var(--color-border)',
                  borderRadius: 'var(--radius-sm)',
                  fontSize: '14px',
                  background: 'var(--color-surface)',
                  color: 'var(--color-text-primary)',
                }}
              >
                <option value="">(no view type)</option>
                {SAVED_SEARCH_VIEW_TYPES.map(vt => (
                  <option key={vt} value={vt}>
                    {vt}
                  </option>
                ))}
              </select>
              <div
                style={{
                  display: 'flex',
                  gap: 'var(--space-2)',
                  justifyContent: 'flex-end',
                }}
              >
                <button
                  type="button"
                  data-testid="save-search-cancel"
                  onClick={() => {
                    setSaveFormOpen(false)
                    setSaveName('')
                    setSaveViewType('')
                  }}
                  style={{
                    padding: '4px var(--space-3)',
                    background: 'transparent',
                    border: '1px solid var(--color-border)',
                    borderRadius: 'var(--radius-sm)',
                    color: 'var(--color-text-muted)',
                    fontSize: 'var(--font-size-caption)',
                    cursor: 'pointer',
                  }}
                >
                  Cancel
                </button>
                <button
                  type="button"
                  data-testid="save-search-confirm"
                  disabled={saveName.trim().length === 0}
                  onClick={confirmSaveSearch}
                  style={{
                    padding: '4px var(--space-3)',
                    background: 'var(--color-accent, var(--color-surface))',
                    color: 'var(--color-surface, var(--color-text-primary))',
                    border: 'none',
                    borderRadius: 'var(--radius-sm)',
                    fontSize: 'var(--font-size-caption)',
                    fontWeight: 600,
                    cursor: 'pointer',
                    opacity: saveName.trim().length === 0 ? 0.5 : 1,
                  }}
                >
                  Save search
                </button>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* ─── Save as View modal (ROADMAP #25) ───
       *
       * Mounted at the same zIndex layer as the search modal but
       * with a higher z (110) so it appears on top. The modal is
       * self-contained: it stops click propagation, has its own
       * form, and only fires onConfirm/onCancel.
       */}
      {saveViewTarget && (
        <SaveAsViewModal
          pages={saveViewTarget.pages}
          isSubmitting={saveViewSubmitting}
          errorMessage={saveViewError}
          onConfirm={handleSaveAsViewConfirm}
          onCancel={closeSaveAsView}
        />
      )}
    </div>
  )
}

// ──── Small presentational helpers ─────────────────────────────────

function SectionHeader({ label }: { label: string }) {
  return (
    <div
      style={{
        padding: 'var(--space-2) var(--space-4)',
        fontSize: 'var(--font-size-micro)',
        fontWeight: 600,
        textTransform: 'uppercase',
        letterSpacing: 'var(--tracking-wider)',
        color: 'var(--color-text-muted)',
        background: 'var(--color-surface-subtle)',
        borderTop: '1px solid var(--color-border)',
      }}
    >
      {label}
    </div>
  )
}

// ──── Save as View action button ──────────────────────────────────
//
// Small icon button rendered at the end of each result row. Clicking
// it does NOT navigate (the row's onClick is stopped via
// `e.stopPropagation()`), so the parent row's Enter / click handler
// is bypassed. The button's `aria-label` describes the action in
// words so screen-reader and getByLabelText tests can target it.

function SaveAsViewButton({
  testId,
  onClick,
}: {
  testId: string
  onClick: () => void
}) {
  return (
    <button
      type="button"
      data-testid={testId}
      aria-label="Save as view"
      title="Save as view"
      onClick={e => {
        e.stopPropagation()
        onClick()
      }}
      onMouseDown={e => e.stopPropagation()}
      onKeyDown={e => e.stopPropagation()}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: '4px',
        padding: '4px var(--space-2)',
        background: 'transparent',
        border: '1px solid var(--color-border)',
        borderRadius: 'var(--radius-sm)',
        color: 'var(--color-text-muted)',
        fontSize: 'var(--font-size-micro)',
        fontWeight: 600,
        textTransform: 'uppercase',
        letterSpacing: 'var(--tracking-wider)',
        cursor: 'pointer',
        flexShrink: 0,
      }}
    >
      <BookmarkPlus size={12} />
      <span>Save as view</span>
    </button>
  )
}

interface ResultButtonProps {
  selected: boolean
  onClick: () => void
  onMouseEnter: () => void
  icon: React.ReactNode
  title: string
  subtitle?: string
  badge?: string | null
  /**
   * Optional action button shown at the end of the row. Used for
   * "Save as View" (ROADMAP #25) and any future per-result
   * affordances. The outer element is a `<div role="button">` (not
   * a `<button>`) so a real button can be nested inside without
   * producing invalid HTML.
   */
  trailing?: React.ReactNode
  /** Test ID for the outer element so individual rows can be targeted. */
  testId?: string
}

function ResultButton({
  selected,
  onClick,
  onMouseEnter,
  icon,
  title,
  subtitle,
  badge,
  trailing,
  testId,
}: ResultButtonProps) {
  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      onClick()
    }
  }
  return (
    <div
      role="button"
      tabIndex={0}
      data-testid={testId}
      onClick={onClick}
      onKeyDown={handleKeyDown}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 'var(--space-3)',
        width: '100%',
        padding: 'var(--space-3) var(--space-4)',
        border: 'none',
        cursor: 'pointer',
        textAlign: 'left',
        background: selected ? 'var(--color-surface-subtle)' : 'transparent',
        color: selected ? 'var(--color-text-primary)' : 'var(--color-text-secondary)',
        fontSize: '14px',
        transition: 'background var(--motion-fast) var(--ease-standard)',
      }}
      onMouseEnter={onMouseEnter}
    >
      {icon}
      <div
        style={{
          flex: 1,
          minWidth: 0,
          display: 'flex',
          flexDirection: 'column',
          gap: '2px',
        }}
      >
        <span
          style={{
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            color: 'var(--color-text-primary)',
          }}
        >
          {title}
        </span>
        {subtitle && (
          <span
            style={{
              fontSize: 'var(--font-size-caption)',
              color: 'var(--color-text-muted)',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {subtitle}
          </span>
        )}
      </div>
      {badge && (
        <span
          style={{
            fontSize: 'var(--font-size-micro)',
            fontWeight: 600,
            textTransform: 'uppercase',
            letterSpacing: 'var(--tracking-wider)',
            color: 'var(--color-text-muted)',
            background: 'var(--color-surface)',
            border: '1px solid var(--color-border)',
            borderRadius: 'var(--radius-pill)',
            padding: '2px var(--space-2)',
            flexShrink: 0,
          }}
        >
          {badge}
        </span>
      )}
      {trailing}
    </div>
  )
}

// ──── Saved / Recent search row helpers (ROADMAP #22) ─────────────
//
// Both rows share the same visual treatment as `ResultButton` (a
// full-width clickable area, accent-on-hover, muted secondary text)
// but with different leading icons and trailing affordances:
//   - Saved rows show a × delete button on the right.
//   - Recent rows show the result count as a small caption.
//
// The rows are NOT keyboard-navigable through the same `selectedIndex`
// the search-results use — they have their own implicit order (saved
// first by id, recents already ordered newest-first by the writer).

function SavedSearchRow({
  saved,
  onClick,
  onDelete,
}: {
  saved: SavedSearch
  onClick: () => void
  onDelete: () => void
}) {
  return (
    <div
      role="button"
      tabIndex={0}
      data-testid={`saved-search-row-${saved.id}`}
      onClick={onClick}
      onKeyDown={e => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault()
          onClick()
        }
      }}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 'var(--space-3)',
        width: '100%',
        padding: 'var(--space-2) var(--space-4)',
        border: 'none',
        cursor: 'pointer',
        textAlign: 'left',
        background: 'transparent',
        color: 'var(--color-text-secondary)',
        fontSize: '14px',
        transition: 'background var(--motion-fast) var(--ease-standard)',
      }}
    >
      <Bookmark
        size={16}
        style={{ flexShrink: 0, color: 'var(--color-accent)' }}
      />
      <div
        style={{
          flex: 1,
          minWidth: 0,
          display: 'flex',
          flexDirection: 'column',
          gap: '2px',
        }}
      >
        <span
          style={{
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            color: 'var(--color-text-primary)',
          }}
        >
          {saved.name}
        </span>
        <span
          style={{
            fontSize: 'var(--font-size-caption)',
            color: 'var(--color-text-muted)',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          {saved.query}
          {saved.viewType ? ` · ${saved.viewType}` : ''}
        </span>
      </div>
      <button
        type="button"
        data-testid={`delete-saved-search-${saved.id}`}
        aria-label={`Delete saved search ${saved.name}`}
        onClick={e => {
          e.stopPropagation()
          onDelete()
        }}
        onMouseDown={e => e.stopPropagation()}
        onKeyDown={e => e.stopPropagation()}
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          justifyContent: 'center',
          padding: '4px',
          background: 'transparent',
          border: 'none',
          borderRadius: 'var(--radius-sm)',
          color: 'var(--color-text-muted)',
          cursor: 'pointer',
          flexShrink: 0,
        }}
      >
        <Trash2 size={14} />
      </button>
    </div>
  )
}

function RecentSearchRow({
  recent,
  onClick,
}: {
  recent: RecentSearch
  onClick: () => void
}) {
  return (
    <div
      role="button"
      tabIndex={0}
      data-testid={`recent-search-row-${recent.query}`}
      onClick={onClick}
      onKeyDown={e => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault()
          onClick()
        }
      }}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 'var(--space-3)',
        width: '100%',
        padding: 'var(--space-2) var(--space-4)',
        border: 'none',
        cursor: 'pointer',
        textAlign: 'left',
        background: 'transparent',
        color: 'var(--color-text-secondary)',
        fontSize: '14px',
        transition: 'background var(--motion-fast) var(--ease-standard)',
      }}
    >
      <History
        size={16}
        style={{ flexShrink: 0, color: 'var(--color-text-muted)' }}
      />
      <div
        style={{
          flex: 1,
          minWidth: 0,
          display: 'flex',
          flexDirection: 'column',
          gap: '2px',
        }}
      >
        <span
          style={{
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            color: 'var(--color-text-primary)',
          }}
        >
          {recent.query}
        </span>
        <span
          style={{
            fontSize: 'var(--font-size-caption)',
            color: 'var(--color-text-muted)',
          }}
        >
          {recent.resultCount === 1 ? '1 result' : `${recent.resultCount} results`}
        </span>
      </div>
    </div>
  )
}
