import { useState, useEffect, useRef, useMemo } from 'react'
import { Search, FileText, Calendar, Hash, X } from 'lucide-react'
import { useNavigate } from '@tanstack/react-router'
import { api } from '@core/api-client'
import type { Page, SearchResult } from '@shared/types/api'
import toast from 'react-hot-toast'

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
      // Use RAF to ensure DOM is ready
      const raf = requestAnimationFrame(() => inputRef.current?.focus())
      return () => cancelAnimationFrame(raf)
    }
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
      onClose()
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

  function selectItem(item: ResultItem) {
    onClose()
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

  if (!isOpen) return null

  // Compute the running index of the current item so we can highlight
  // it. Items in the same section share an offset that gets bumped as
  // we walk the list.
  const showPageSection = pageResults.length > 0
  const showBlockSection = blockResults.length > 0
  const noResults = !loading && !showPageSection && !showBlockSection && query.length > 0
  const isFirstLoad = loading && items.length === 0

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
            onClick={onClose}
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

          {/* ─── Pages section ─── */}
          {showPageSection && (
            <SectionHeader label="Pages" />
          )}
          {showPageSection && pageResults.map((page, idx) => {
            const itemIndex = idx
            return (
              <ResultButton
                key={`page:${page.id}`}
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
              />
            )
          })}

          {/* ─── Blocks section ─── */}
          {showBlockSection && (
            <SectionHeader label="Blocks" />
          )}
          {showBlockSection && blockResults.map((block, idx) => {
            const itemIndex = pageResults.length + idx
            const preview = block.snippet || block.content
            return (
              <ResultButton
                key={`block:${block.blockId}`}
                selected={itemIndex === selectedIndex}
                onClick={() => selectItem({ kind: 'block', block, id: `block:${block.blockId}` })}
                onMouseEnter={() => setSelectedIndex(itemIndex)}
                icon={<Hash size={16} style={{ flexShrink: 0, color: 'var(--color-text-muted)' }} />}
                title={preview.length > 80 ? `${preview.slice(0, 80)}…` : preview || '(empty block)'}
                subtitle={block.pageName || undefined}
                badge="Block"
              />
            )
          })}
        </div>
      </div>
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

interface ResultButtonProps {
  selected: boolean
  onClick: () => void
  onMouseEnter: () => void
  icon: React.ReactNode
  title: string
  subtitle?: string
  badge?: string | null
}

function ResultButton({ selected, onClick, onMouseEnter, icon, title, subtitle, badge }: ResultButtonProps) {
  return (
    <button
      onClick={onClick}
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
            marginLeft: 'auto',
            flexShrink: 0,
          }}
        >
          {badge}
        </span>
      )}
    </button>
  )
}
