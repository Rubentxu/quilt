/**
 * QueryResults — interactive result table with keyboard navigation.
 *
 * Features:
 * - Arrow key navigation through results
 * - Enter to navigate to selected result
 * - Expand row to show block context
 * - Visual selection highlight
 */

import { useCallback, useEffect, useRef, useState } from 'react'
import { ChevronRight, ChevronDown, ExternalLink, FileText } from 'lucide-react'
import type { QueryResult } from '@shared/types/queryAst'
import { useNavigate } from '@tanstack/react-router'

interface QueryResultsProps {
  /** Query execution result. */
  result: QueryResult | null
  /** Whether results are loading. */
  loading?: boolean
  /** Called when a result is selected for navigation. */
  onNavigate?: (blockId: string, pageName: string) => void
}

interface ExpandedRow {
  id: string
}

/**
 * QueryResults — virtualised result display with keyboard navigation.
 *
 * Keyboard:
 * - ArrowDown / ArrowUp: move selection
 * - Enter: navigate to selected row
 * - Escape: clear selection
 * - Space: expand/collapse selected row
 */
export function QueryResults({ result, loading, onNavigate }: QueryResultsProps) {
  const [selectedIndex, setSelectedIndex] = useState(0)
  const [expanded, setExpanded] = useState<ExpandedRow | null>(null)
  const navigate = useNavigate()
  const listRef = useRef<HTMLDivElement>(null)

  const rows = result?.results ?? []
  const total = result?.total ?? 0
  const elapsed = result?.elapsed_ms ?? 0

  // Reset selection when results change
  useEffect(() => {
    setSelectedIndex(0)
    setExpanded(null)
  }, [result])

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'ArrowDown') {
        e.preventDefault()
        setSelectedIndex(i => Math.min(i + 1, rows.length - 1))
      } else if (e.key === 'ArrowUp') {
        e.preventDefault()
        setSelectedIndex(i => Math.max(i - 1, 0))
      } else if (e.key === 'Enter') {
        e.preventDefault()
        const row = rows[selectedIndex]
        if (row) {
          const pageName = (row.pageName as string | undefined) ?? (row.name as string | undefined)
          if (pageName) {
            if (onNavigate) {
              onNavigate(row.id as string, pageName)
            } else {
              navigateToBlock(row, navigate)
            }
          }
        }
      } else if (e.key === ' ') {
        e.preventDefault()
        const row = rows[selectedIndex]
        if (row) {
          setExpanded(prev =>
            prev?.id === row.id ? null : { id: row.id as string },
          )
        }
      } else if (e.key === 'Escape') {
        setSelectedIndex(0)
        setExpanded(null)
      }
    },
    [rows, selectedIndex, navigate, onNavigate],
  )

  // Scroll selected row into view
  useEffect(() => {
    const el = listRef.current?.querySelector(
      `[data-row-index="${selectedIndex}"]`,
    ) as HTMLElement | null
    if (el && typeof el.scrollIntoView === 'function') {
      el.scrollIntoView({ block: 'nearest' })
    }
  }, [selectedIndex])

  if (loading) {
    return (
      <div data-testid="query-results-loading" style={{ padding: 'var(--space-4)', textAlign: 'center' }}>
        <span style={{ color: 'var(--color-text-muted)', fontSize: '14px' }}>Loading…</span>
      </div>
    )
  }

  if (!result || rows.length === 0) {
    return (
      <div
        data-testid="query-results-empty"
        style={{
          padding: 'var(--space-8)',
          textAlign: 'center',
          color: 'var(--color-text-muted)',
          fontSize: '14px',
        }}
      >
        No results
      </div>
    )
  }

  return (
    <div
      data-testid="query-results"
      tabIndex={0}
      onKeyDown={handleKeyDown}
      style={{ outline: 'none' }}
    >
      {/* ─── Result meta ─── */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '4px var(--space-3)',
          borderBottom: '1px solid var(--color-border)',
          fontSize: 'var(--font-size-micro)',
          color: 'var(--color-text-muted)',
        }}
      >
        <span data-testid="query-results-count">
          {total} result{total !== 1 ? 's' : ''}
        </span>
        <span data-testid="query-results-elapsed">{elapsed}ms</span>
      </div>

      {/* ─── Keyboard hint ─── */}
      <div
        style={{
          padding: '2px var(--space-3)',
          background: 'var(--color-surface-subtle)',
          borderBottom: '1px solid var(--color-border)',
          fontSize: 'var(--font-size-micro)',
          color: 'var(--color-text-muted)',
        }}
      >
        ↑↓ navigate · Enter navigate · Space expand · Esc clear
      </div>

      {/* ─── Rows ─── */}
      <div ref={listRef} style={{ maxHeight: '400px', overflowY: 'auto' }}>
        {rows.map((row, idx) => {
          const isSelected = idx === selectedIndex
          const isExpanded = expanded?.id === row.id
          const pageName = (row.pageName as string | undefined) ?? (row.name as string | undefined)
          const content = (row.content as string | undefined) ?? ''
          const snippet = content.length > 120 ? `${content.slice(0, 120)}…` : content

          return (
            <div
              key={row.id as string}
              data-row-index={idx}
              data-testid={`query-result-row-${idx}`}
              role="button"
              tabIndex={-1}
              onClick={() => {
                setSelectedIndex(idx)
                if (pageName) {
                  if (onNavigate) {
                    onNavigate(row.id as string, pageName)
                  } else {
                    navigateToBlock(row, navigate)
                  }
                }
              }}
              onDoubleClick={() => {
                if (pageName) {
                  if (onNavigate) {
                    onNavigate(row.id as string, pageName)
                  } else {
                    navigateToBlock(row, navigate)
                  }
                }
              }}
              onMouseEnter={() => setSelectedIndex(idx)}
              style={{
                display: 'flex',
                alignItems: 'flex-start',
                gap: 'var(--space-2)',
                padding: 'var(--space-2) var(--space-3)',
                cursor: pageName ? 'pointer' : 'default',
                background: isSelected
                  ? 'var(--color-surface-subtle)'
                  : 'transparent',
                borderLeft: isSelected
                  ? '2px solid var(--color-accent)'
                  : '2px solid transparent',
                transition: 'background var(--motion-fast) var(--ease-standard)',
              }}
            >
              {/* Expand toggle */}
              <button
                type="button"
                data-testid={`query-result-expand-${idx}`}
                onClick={e => {
                  e.stopPropagation()
                  setExpanded(prev =>
                    prev?.id === row.id ? null : { id: row.id as string },
                  )
                }}
                onMouseDown={e => e.stopPropagation()}
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  padding: '2px',
                  background: 'transparent',
                  border: 'none',
                  color: 'var(--color-text-muted)',
                  cursor: 'pointer',
                  flexShrink: 0,
                }}
              >
                {isExpanded ? (
                  <ChevronDown size={12} />
                ) : (
                  <ChevronRight size={12} />
                )}
              </button>

              {/* Content */}
              <div style={{ flex: 1, minWidth: 0 }}>
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 'var(--space-2)',
                    marginBottom: '2px',
                  }}
                >
                  <span
                    style={{
                      fontSize: 'var(--font-size-caption)',
                      fontWeight: 600,
                      color: 'var(--color-text-primary)',
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    {pageName ?? '(no page)'}
                  </span>
                  {pageName && (
                    <ExternalLink
                      size={10}
                      style={{ color: 'var(--color-text-muted)', flexShrink: 0 }}
                    />
                  )}
                </div>

                {/* Snippet preview */}
                <div
                  style={{
                    fontSize: 'var(--font-size-caption)',
                    color: 'var(--color-text-secondary)',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                  }}
                >
                  {snippet || (
                    <span style={{ color: 'var(--color-text-muted)', fontStyle: 'italic' }}>
                      (empty block)
                    </span>
                  )}
                </div>

                {/* Expanded context */}
                {isExpanded && (
                  <div
                    data-testid={`query-result-expanded-${idx}`}
                    style={{
                      marginTop: 'var(--space-2)',
                      padding: 'var(--space-2)',
                      background: 'var(--color-surface-subtle)',
                      borderRadius: 'var(--radius-sm)',
                      fontSize: 'var(--font-size-caption)',
                      color: 'var(--color-text-secondary)',
                      fontFamily: 'var(--font-family-mono)',
                      whiteSpace: 'pre-wrap',
                      wordBreak: 'break-word',
                    }}
                  >
                    {content || '(empty)'}
                  </div>
                )}
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}

/**
 * Navigate to a block result using the standard handoff mechanism.
 */
function navigateToBlock(
  row: Record<string, unknown>,
  navigate: ReturnType<typeof useNavigate>,
) {
  const pageName = (row.pageName as string | undefined) ?? (row.name as string | undefined)
  const blockId = row.id as string

  if (!pageName) return

  // Hand off focus via sessionStorage
  sessionStorage.setItem('quilt:focusBlock', blockId)
  navigate({ to: '/page/$name', params: { name: pageName } })
}
