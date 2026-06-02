import { useState, useEffect, useMemo } from 'react'
import { Search, Copy, ChevronDown, ChevronRight, Link2 } from 'lucide-react'
import { useNavigate } from '@tanstack/react-router'
import { api } from '@core/api-client'
import type { Backlink } from '@shared/types/api'
import toast from 'react-hot-toast'

interface BacklinksPanelProps {
  pageName: string | null
  isOpen: boolean
}

export function BacklinksPanel({ pageName, isOpen }: BacklinksPanelProps) {
  const [backlinks, setBacklinks] = useState<Backlink[]>([])
  const [loading, setLoading] = useState(false)
  const [filter, setFilter] = useState('')
  const [sortBy, setSortBy] = useState<'recent' | 'page' | 'count'>('recent')
  const [collapsedPages, setCollapsedPages] = useState<Set<string>>(new Set())
  const navigate = useNavigate()

  useEffect(() => {
    if (!isOpen || !pageName) return

    setLoading(true)
    api
      .getPageBacklinks(pageName)
      .then(setBacklinks)
      .catch(() => toast.error('Failed to load backlinks'))
      .finally(() => setLoading(false))
  }, [pageName, isOpen])

  // Filter backlinks by source page name or content preview
  const filtered = useMemo(() => {
    if (!filter) return backlinks
    const q = filter.toLowerCase()
    return backlinks.filter(
      (b) =>
        b.sourcePageName.toLowerCase().includes(q) ||
        b.contentPreview.toLowerCase().includes(q),
    )
  }, [backlinks, filter])

  // Group by source page
  const grouped = useMemo(() => {
    const map = new Map<string, Backlink[]>()
    for (const b of filtered) {
      const group = map.get(b.sourcePageName)
      if (group) {
        group.push(b)
      } else {
        map.set(b.sourcePageName, [b])
      }
    }
    return map
  }, [filtered])

  // Sort groups
  const sortedGroups = useMemo(() => {
    return [...grouped.entries()].sort(([a, refsA], [b, refsB]) => {
      switch (sortBy) {
        case 'page':
          return a.localeCompare(b)
        case 'count':
          return refsB.length - refsA.length
        default:
          return 0
      }
    })
  }, [grouped, sortBy])

  function toggleCollapse(page: string) {
    setCollapsedPages((prev) => {
      const next = new Set(prev)
      if (next.has(page)) {
        next.delete(page)
      } else {
        next.add(page)
      }
      return next
    })
  }

  async function copyBacklink(sourcePageName: string) {
    const url = `${window.location.origin}/page/${encodeURIComponent(sourcePageName)}`
    try {
      await navigator.clipboard.writeText(url)
      toast.success('Link copied')
    } catch {
      toast.error('Failed to copy link')
    }
  }

  if (!isOpen) return null

  return (
    <aside
      style={{
        width: '320px',
        borderLeft: '1px solid var(--color-border)',
        background: 'var(--color-surface)',
        overflow: 'auto',
        flexShrink: 0,
        padding: 'var(--space-5)',
        boxShadow: 'var(--shadow-sm)',
      }}
    >
      {/* Header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 'var(--space-2)',
          marginBottom: 'var(--space-3)',
          fontSize: '13px',
          fontWeight: 600,
          color: 'var(--color-text-secondary)',
          textTransform: 'uppercase',
          letterSpacing: '0.05em',
        }}
      >
        <Link2 size={14} />
        Linked References
        <span
          style={{
            fontSize: '12px',
            fontWeight: 400,
            color: 'var(--color-text-muted)',
            marginLeft: 'auto',
          }}
        >
          {backlinks.length}
        </span>
      </div>

      {/* Controls */}
      {backlinks.length > 0 && (
        <div
          style={{
            display: 'flex',
            gap: 'var(--space-2)',
            marginBottom: 'var(--space-3)',
          }}
        >
          {/* Filter input */}
          <div style={{ flex: 1, position: 'relative' }}>
            <Search
              size={12}
              style={{
                position: 'absolute',
                left: '8px',
                top: '50%',
                transform: 'translateY(-50%)',
                color: 'var(--color-text-muted)',
                pointerEvents: 'none',
              }}
            />
            <input
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              placeholder="Filter references..."
              style={{
                width: '100%',
                padding: '5px 8px 5px 24px',
                border: '1px solid var(--color-border)',
                borderRadius: 'var(--radius-sm)',
                background: 'var(--color-surface)',
                color: 'var(--color-text-primary)',
                fontSize: '12px',
                outline: 'none',
              }}
            />
          </div>

          {/* Sort dropdown */}
          <select
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as 'recent' | 'page' | 'count')}
            style={{
              padding: '5px 8px',
              border: '1px solid var(--color-border)',
              borderRadius: 'var(--radius-sm)',
              background: 'var(--color-surface)',
              color: 'var(--color-text-primary)',
              fontSize: '12px',
              cursor: 'pointer',
              outline: 'none',
            }}
          >
            <option value="recent">Recent</option>
            <option value="page">By page</option>
            <option value="count">By count</option>
          </select>
        </div>
      )}

      {/* Loading */}
      {loading && (
        <div
          style={{
            color: 'var(--color-text-muted)',
            fontSize: '13px',
            textAlign: 'center',
            padding: 'var(--space-4)',
          }}
        >
          Loading...
        </div>
      )}

      {/* Empty state per DESIGN.md §15 */}
      {!loading && backlinks.length === 0 && (
        <div style={{ padding: 'var(--space-4)', textAlign: 'center' }}>
          <div
            style={{
              fontSize: '13px',
              color: 'var(--color-text-muted)',
              marginBottom: 'var(--space-2)',
            }}
          >
            No linked references
          </div>
          <div
            style={{
              fontSize: '12px',
              color: 'var(--color-text-disabled)',
            }}
          >
            This page is not linked from other notes.
            Create links using [[Page Name]].
          </div>
        </div>
      )}

      {/* Filtered empty state */}
      {!loading && backlinks.length > 0 && sortedGroups.length === 0 && (
        <div
          style={{
            padding: 'var(--space-4)',
            textAlign: 'center',
            fontSize: '12px',
            color: 'var(--color-text-muted)',
          }}
        >
          No matches
        </div>
      )}

      {/* Grouped backlink list */}
      {!loading && sortedGroups.length > 0 && (
        <div>
          {sortedGroups.map(([sourcePage, refs]) => {
            const isCollapsed = collapsedPages.has(sourcePage)

            return (
              <div
                key={sourcePage}
                style={{
                  marginBottom: 'var(--space-2)',
                  borderRadius: 'var(--radius-md)',
                  border: '1px solid var(--color-border)',
                  overflow: 'hidden',
                }}
              >
                {/* Group header */}
                <div
                  onClick={() => toggleCollapse(sourcePage)}
                  style={{
                     display: 'flex',
                     alignItems: 'center',
                     gap: 'var(--space-2)',
                     padding: 'var(--space-3) var(--space-4)',
                     cursor: 'pointer',
                     background: 'var(--color-surface-subtle)',
                     fontSize: '13px',
                     fontWeight: 600,
                     color: 'var(--color-text-primary)',
                     userSelect: 'none',
                  }}
                >
                  {isCollapsed ? (
                    <ChevronRight size={12} />
                  ) : (
                    <ChevronDown size={12} />
                  )}
                  <span style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                    {sourcePage}
                  </span>
                  <span
                    style={{
                      fontSize: '10px',
                      color: 'var(--color-text-muted)',
                      background: 'var(--color-surface)',
                      padding: '0 6px',
                      borderRadius: 'var(--radius-pill)',
                      lineHeight: '16px',
                    }}
                  >
                    {refs.length}
                  </span>
                  <button
                    onClick={(e) => {
                      e.stopPropagation()
                      copyBacklink(sourcePage)
                    }}
                    style={{
                      background: 'none',
                      border: 'none',
                      cursor: 'pointer',
                      color: 'var(--color-text-muted)',
                      padding: '2px',
                      display: 'flex',
                      alignItems: 'center',
                      borderRadius: 'var(--radius-sm)',
                    }}
                    aria-label="Copy link to page"
                    title="Copy link to page"
                  >
                    <Copy size={11} />
                  </button>
                </div>

                {/* Reference items */}
                {!isCollapsed &&
                  refs.map((ref, i) => (
                    <div
                      key={ref.sourceBlockId + i}
                      onClick={() =>
                        navigate({
                          to: '/page/$name',
                          params: { name: ref.sourcePageName },
                        })
                      }
                      style={{
                         padding: 'var(--space-3) var(--space-4)',
                         cursor: 'pointer',
                         fontSize: '13px',
                         color: 'var(--color-text-secondary)',
                         borderTop: '1px solid var(--color-border)',
                        transition:
                          'background var(--motion-fast) var(--ease-standard)',
                      }}
                      onMouseEnter={(e) =>
                        (e.currentTarget.style.background =
                          'var(--color-surface-subtle)')
                      }
                      onMouseLeave={(e) =>
                        (e.currentTarget.style.background = 'transparent')
                      }
                    >
                      <div
                        style={{
                          display: 'flex',
                          alignItems: 'center',
                          gap: 'var(--space-2)',
                        }}
                      >
                        <Link2
                          size={11}
                          style={{
                            color: 'var(--color-text-muted)',
                            flexShrink: 0,
                          }}
                        />
                        <span
                          style={{
                            overflow: 'hidden',
                            textOverflow: 'ellipsis',
                            display: '-webkit-box',
                            WebkitLineClamp: 2,
                            WebkitBoxOrient: 'vertical',
                            lineHeight: 1.4,
                          }}
                        >
                          {ref.contentPreview}
                        </span>
                      </div>
                    </div>
                  ))}
              </div>
            )
          })}
        </div>
      )}
    </aside>
  )
}
