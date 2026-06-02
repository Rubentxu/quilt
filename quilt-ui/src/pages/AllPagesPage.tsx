import { useState, useEffect } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { FileText, Calendar, Search, Bot } from 'lucide-react'
import { api } from '@core/api-client'
import { ErrorBoundary } from '@shared/components/ErrorBoundary'
import { EmptyState } from '@shared/components/EmptyState'
import { useTabs } from '@shared/contexts/TabsContext'
import type { Page } from '@shared/types/api'
import toast from 'react-hot-toast'

export function AllPagesPage() {
  const [pages, setPages] = useState<Page[]>([])
  const [loading, setLoading] = useState(true)
  const [search, setSearch] = useState('')
  const [sortBy, setSortBy] = useState<'name' | 'createdAt'>('name')
  const [showJournals, setShowJournals] = useState(false)
  const [authorFilter, setAuthorFilter] = useState('')
  const [matchingPages, setMatchingPages] = useState<Set<string> | null>(null)
  const [authorLoading, setAuthorLoading] = useState(false)
  const navigate = useNavigate()
  const { openTab } = useTabs()

  // Auto-open tab for all-pages view
  useEffect(() => {
    openTab({ name: 'all-pages', type: 'all-pages', title: 'All Pages', params: {} })
  }, [openTab])

  useEffect(() => {
    api.listPages()
      .then(setPages)
      .catch(() => toast.error('Failed to load pages'))
      .finally(() => setLoading(false))
  }, [])

  // ADR-0003 — when the user types in the "Created by" filter, fetch
  // matching blocks and compute the set of page names that contain
  // at least one block with the matching `created_by` property.
  useEffect(() => {
    const trimmed = authorFilter.trim()
    if (!trimmed) {
      setMatchingPages(null)
      return
    }
    let cancelled = false
    setAuthorLoading(true)
    api.listBlocksByAuthor(trimmed, 200)
      .then((blocks) => {
        if (cancelled) return
        // We can't resolve page names from this endpoint, so we just
        // record the page ids. The filter below matches by page id.
        setMatchingPages(new Set(blocks.map(b => b.pageId)))
      })
      .catch(() => {
        if (cancelled) return
        setMatchingPages(new Set())
        toast.error('Failed to filter by author')
      })
      .finally(() => {
        if (!cancelled) setAuthorLoading(false)
      })
    return () => { cancelled = true }
  }, [authorFilter])

  const journalCount = pages.filter(p => p.journal).length

  const filtered = pages
    .filter(p => {
      if (!showJournals && p.journal) return false
      if (matchingPages !== null && !matchingPages.has(p.id)) return false
      if (!search) return true
      const q = search.toLowerCase()
      return p.name.toLowerCase().includes(q) || (p.title && p.title.toLowerCase().includes(q))
    })
    .sort((a, b) => {
      if (sortBy === 'name') return a.name.localeCompare(b.name)
      return new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime()
    })

  const regularPages = filtered.filter(p => !p.journal)
  const journalPages = filtered.filter(p => p.journal)

  function renderRow(page: Page) {
    return (
      <div
        key={page.id}
        onClick={() => {
          if (page.journal && page.journalDay) {
            const d = page.journalDay.toString()
            navigate({
              to: '/journal/$date',
              params: {
                date: `${d.slice(0, 4)}-${d.slice(4, 6)}-${d.slice(6, 8)}`,
              },
            })
          } else {
            navigate({ to: '/page/$name', params: { name: page.name } })
          }
        }}
        style={{
          display: 'grid',
          gridTemplateColumns: '1fr 120px 120px',
          padding: 'var(--space-3) var(--space-4)',
          cursor: 'pointer',
          borderBottom: '1px solid var(--color-border)',
          fontSize: '14px',
          transition: 'background var(--motion-fast)',
        }}
        onMouseEnter={e => { e.currentTarget.style.background = 'var(--color-surface-subtle)' }}
        onMouseLeave={e => { e.currentTarget.style.background = 'transparent' }}
      >
        <span style={{
          display: 'flex',
          alignItems: 'center',
          gap: 'var(--space-2)',
          color: 'var(--color-text-primary)',
        }}>
          {page.journal
            ? <Calendar size={14} style={{ color: 'var(--color-accent)' }} />
            : <FileText size={14} style={{ color: 'var(--color-text-muted)' }} />
          }
          {page.title || page.name}
        </span>
        <span style={{ color: 'var(--color-text-muted)' }}>
          {page.journal ? 'Journal' : 'Page'}
        </span>
        <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
          {new Date(page.createdAt).toLocaleDateString()}
        </span>
      </div>
    )
  }

  return (
    <ErrorBoundary>
    <div>
      <h2 style={{
        fontSize: '28px',
        fontWeight: 700,
        color: 'var(--color-text-primary)',
        marginBottom: 'var(--space-4)',
      }}>
        All Pages
      </h2>

      {/* Search + Sort controls */}
      <div style={{ display: 'flex', gap: 'var(--space-3)', marginBottom: 'var(--space-4)', flexWrap: 'wrap' }}>
        <div style={{
          flex: '1 1 200px',
          display: 'flex',
          alignItems: 'center',
          gap: 'var(--space-2)',
          padding: 'var(--space-2) var(--space-3)',
          border: '1px solid var(--color-border)',
          borderRadius: 'var(--radius-md)',
          background: 'var(--color-surface)',
        }}>
          <Search size={16} style={{ color: 'var(--color-text-muted)' }} />
          <input
            value={search}
            onChange={e => setSearch(e.target.value)}
            placeholder="Filter pages..."
            style={{
              flex: 1,
              border: 'none',
              outline: 'none',
              background: 'transparent',
              color: 'var(--color-text-primary)',
              fontSize: '14px',
            }}
          />
        </div>

        {/* ADR-0003 — Created by filter (e.g. `agent::claude`, `me`) */}
        <div
          data-testid="author-filter"
          style={{
            flex: '0 1 260px',
            display: 'flex',
            alignItems: 'center',
            gap: 'var(--space-2)',
            padding: 'var(--space-2) var(--space-3)',
            border: '1px solid var(--color-border)',
            borderRadius: 'var(--radius-md)',
            background: 'var(--color-surface)',
          }}
        >
          <Bot size={14} style={{ color: 'var(--color-text-muted)' }} />
          <input
            value={authorFilter}
            onChange={e => setAuthorFilter(e.target.value)}
            placeholder="Filter by author (e.g. agent::claude, user::alice)"
            style={{
              flex: 1,
              border: 'none',
              outline: 'none',
              background: 'transparent',
              color: 'var(--color-text-primary)',
              fontSize: '14px',
            }}
          />
          {authorLoading && (
            <span style={{ fontSize: '11px', color: 'var(--color-text-muted)' }}>…</span>
          )}
        </div>
        <select
          value={sortBy}
          onChange={e => setSortBy(e.target.value as any)}
          style={{
            padding: 'var(--space-2) var(--space-3)',
            border: '1px solid var(--color-border)',
            borderRadius: 'var(--radius-md)',
            background: 'var(--color-surface)',
            color: 'var(--color-text-primary)',
            fontSize: '14px',
          }}
        >
          <option value="name">Sort by name</option>
          <option value="createdAt">Sort by recent</option>
        </select>
        <label style={{
          display: 'flex',
          alignItems: 'center',
          gap: 'var(--space-2)',
          fontSize: '13px',
          color: 'var(--color-text-secondary)',
          cursor: 'pointer',
          userSelect: 'none',
          whiteSpace: 'nowrap',
        }}>
          <input
            type="checkbox"
            checked={showJournals}
            onChange={e => setShowJournals(e.target.checked)}
            style={{ cursor: 'pointer', accentColor: 'var(--color-accent)' }}
          />
          Show journals
        </label>
      </div>

      {/* Pages table */}
      {loading ? (
        <div style={{ color: 'var(--color-text-muted)', textAlign: 'center', padding: 'var(--space-8)' }}>
          Loading...
        </div>
      ) : (
        <div style={{
          border: '1px solid var(--color-border)',
          borderRadius: 'var(--radius-lg)',
          overflow: 'hidden',
        }}>
          {/* Header */}
          <div style={{
            display: 'grid',
            gridTemplateColumns: '1fr 120px 120px',
            padding: 'var(--space-2) var(--space-4)',
            background: 'var(--color-surface-subtle)',
            fontSize: '12px',
            fontWeight: 600,
            color: 'var(--color-text-muted)',
            textTransform: 'uppercase',
            letterSpacing: '0.05em',
            borderBottom: '1px solid var(--color-border)',
          }}>
            <span>Name</span>
            <span>Type</span>
            <span>Created</span>
          </div>

          {/* Rows */}
          {filtered.length === 0 ? (
            <EmptyState
              icon={<FileText size={24} aria-hidden="true" />}
              title="No pages found"
              description={
                search.trim()
                  ? `No pages match "${search.trim()}". Try a different search term.`
                  : 'Pages you create will appear here. Start by creating one from the sidebar.'
              }
              action={
                <button
                  onClick={() => {
                    const name = window.prompt('Page name:')
                    if (name && name.trim()) {
                      navigate({ to: '/page/$name', params: { name: name.trim() } })
                    }
                  }}
                  style={{
                    display: 'inline-flex',
                    alignItems: 'center',
                    gap: 'var(--space-2)',
                    padding: '8px var(--space-4)',
                    background: 'var(--color-primary)',
                    color: 'var(--color-on-primary)',
                    border: 'none',
                    borderRadius: 'var(--radius-md)',
                    fontSize: '13px',
                    fontWeight: 600,
                    cursor: 'pointer',
                    fontFamily: 'inherit',
                  }}
                  className="btn-primary"
                >
                  <Search size={14} />
                  Create your first page
                </button>
              }
            />
          ) : (
            <>
              {regularPages.map(page => renderRow(page))}
              {showJournals && journalPages.length > 0 && (
                <>
                  <div style={{
                    padding: 'var(--space-3) var(--space-4)',
                    fontSize: '12px',
                    fontWeight: 600,
                    color: 'var(--color-text-muted)',
                    textTransform: 'uppercase',
                    letterSpacing: '0.05em',
                    borderTop: '1px solid var(--color-border)',
                    borderBottom: '1px solid var(--color-border)',
                    background: 'var(--color-surface-subtle)',
                  }}>
                    Journals ({journalPages.length})
                  </div>
                  {journalPages.map(page => renderRow(page))}
                </>
              )}
            </>
          )}
        </div>
      )}

      {/* Stats */}
      <div style={{ marginTop: 'var(--space-4)', fontSize: '13px', color: 'var(--color-text-muted)' }}>
        {regularPages.length} pages
        {!showJournals && journalCount > 0 && (
          <span> · {journalCount} journals hidden</span>
        )}
        {showJournals && journalPages.length > 0 && (
          <span> · {journalPages.length} journals</span>
        )}
        {authorFilter.trim() && (
          <span> · filtered by author &quot;{authorFilter.trim()}&quot;</span>
        )}
      </div>
    </div>
    </ErrorBoundary>
  )
}
