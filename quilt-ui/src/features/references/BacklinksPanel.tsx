import { useState, useEffect, useMemo } from 'react'
import { Search, Copy, ChevronDown, ChevronRight, Link2, Pencil, Check, X } from 'lucide-react'
import { useNavigate } from '@tanstack/react-router'
import { api } from '@core/api-client'
import type { Backlink } from '@shared/types/api'
import toast from 'react-hot-toast'
import { useUnlinkedRefQueue } from './useUnlinkedRefQueue'
import { UnlinkedRefQueue } from './UnlinkedRefQueue'

interface BacklinksPanelProps {
  pageName: string | null
  isOpen: boolean
  /**
   * Whether the inner content (filter, sort, list) is expanded.
   * Defaults to false — the panel shows the header with count and
   * keeps the content collapsed until the user clicks it open.
   * This matches the Quilt behaviour of "backlinks appear at the
   * bottom of every page automatically", where the reference list
   * is collapsed by default and only the count is visible.
   */
  defaultExpanded?: boolean
}

export function BacklinksPanel({ pageName, isOpen, defaultExpanded = false }: BacklinksPanelProps) {
  const [backlinks, setBacklinks] = useState<Backlink[]>([])
  const [loading, setLoading] = useState(false)
  const [filter, setFilter] = useState('')
  const [sortBy, setSortBy] = useState<'recent' | 'page' | 'count'>('recent')
  const [collapsedPages, setCollapsedPages] = useState<Set<string>>(new Set())
  const [expanded, setExpanded] = useState(defaultExpanded)
  const navigate = useNavigate()

  // Unlinked reference queue (Q029). The hook is enabled only when
  // the panel is actually visible — there's no point scanning for
  // mentions on a hidden sidebar. We still call the hook
  // unconditionally (rules-of-hooks) and pass `null` when disabled
  // so the hook short-circuits internally.
  const unlinked = useUnlinkedRefQueue(isOpen ? pageName : null)

  useEffect(() => {
    if (!isOpen || !pageName) return

    setLoading(true)
    api
      .getPageBacklinks(pageName)
      .then(setBacklinks)
      .catch(() => toast.error('Failed to load backlinks'))
      .finally(() => setLoading(false))
  }, [pageName, isOpen])

  // Reset the inner expanded state when the panel is reopened on a
  // different page (so the count badge reflects the new page and the
  // user re-decides whether to expand it).
  useEffect(() => {
    setExpanded(defaultExpanded)
  }, [pageName, defaultExpanded])

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

  // ─── Q028: Editable Backlinks state ─────────────────────────────
  //
  // The panel tracks which single row is currently being edited
  // (by sourceBlockId) and the in-progress text in that row's
  // textarea. We intentionally keep the editing state at the
  // panel level rather than in each row so a single shared
  // `BacklinkItem` sub-component is the only place that renders
  // edit affordances, but the click and save logic stays close
  // to the data.
  const [editingBlockId, setEditingBlockId] = useState<string | null>(null)
  const [editingDraft, setEditingDraft] = useState<string>('')
  const [savingBlockId, setSavingBlockId] = useState<string | null>(null)

  function startEdit(ref: Backlink) {
    setEditingBlockId(ref.sourceBlockId)
    // Pre-fill with the current `context` (either the user's override
    // or the default snippet). The textarea is bound to `editingDraft`
    // so the user can freely edit before saving.
    setEditingDraft(ref.context)
  }

  function cancelEdit() {
    setEditingBlockId(null)
    setEditingDraft('')
  }

  async function saveEdit(ref: Backlink) {
    if (savingBlockId) return // ignore double-clicks
    setSavingBlockId(ref.sourceBlockId)
    // Empty string and whitespace-only are both treated as "clear":
    // the server stores NULL and the panel falls back to the default
    // snippet. This matches the server's behavior of treating an
    // empty string as a clear (see editable_backlinks_api.rs).
    const trimmed = editingDraft.trim()
    const payload = trimmed.length === 0 ? null : trimmed

    try {
      const updated = await api.updateReferenceContext({
        sourceBlockId: ref.sourceBlockId,
        targetPageName: pageName ?? '',
        context: payload,
      })
      // Optimistic-ish local update: replace the row's DTO so the
      // panel reflects the new value without waiting for the next
      // refetch. The api-client already invalidated the page's
      // backlinks cache, so any future re-fetch will return the
      // server-authoritative version.
      setBacklinks(prev =>
        prev.map(b =>
          b.sourceBlockId === ref.sourceBlockId ? updated : b,
        ),
      )
      setEditingBlockId(null)
      setEditingDraft('')
      toast.success(payload === null ? 'Context cleared' : 'Context saved')
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to save context'
      toast.error(message)
    } finally {
      setSavingBlockId(null)
    }
  }

  if (!isOpen) return null

  return (
    <aside
      data-testid="backlinks-panel"
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
      {/* Header — always visible, click to expand/collapse the content */}
      <button
        type="button"
        onClick={() => setExpanded(v => !v)}
        aria-expanded={expanded}
        aria-controls="backlinks-panel-content"
        data-testid="backlinks-panel-header"
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 'var(--space-2)',
          marginBottom: expanded ? 'var(--space-3)' : 0,
          fontSize: '13px',
          fontWeight: 600,
          color: 'var(--color-text-secondary)',
          textTransform: 'uppercase',
          letterSpacing: '0.05em',
          background: 'transparent',
          border: 'none',
          padding: 0,
          cursor: 'pointer',
          width: '100%',
          textAlign: 'left',
        }}
      >
        {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        <Link2 size={14} />
        <span>Linked References</span>
        <span
          data-testid="backlinks-panel-count"
          style={{
            fontSize: '12px',
            fontWeight: 400,
            color: 'var(--color-text-muted)',
            marginLeft: 'auto',
            background: 'var(--color-surface-subtle)',
            borderRadius: 'var(--radius-pill)',
            padding: '0 8px',
            lineHeight: '18px',
            minWidth: '20px',
            textAlign: 'center',
          }}
        >
          {backlinks.length}
        </span>
      </button>

      {/* Content — collapsed by default, expanded on header click */}
      {expanded && (
        <div id="backlinks-panel-content" data-testid="backlinks-panel-content">
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
                      refs.map((ref, i) => {
                        const isEditing = editingBlockId === ref.sourceBlockId
                        const isSaving = savingBlockId === ref.sourceBlockId
                        return (
                          <div
                            key={ref.sourceBlockId + i}
                            data-testid={`backlinks-item-${ref.sourceBlockId}`}
                            onClick={() => {
                              // Q028: while editing, the row click
                              // is a no-op so the user can interact
                              // with the textarea / buttons without
                              // accidentally navigating away.
                              if (isEditing) return
                              navigate({
                                to: '/page/$name',
                                params: { name: ref.sourcePageName },
                              })
                            }}
                            style={{
                              padding: 'var(--space-3) var(--space-4)',
                              cursor: isEditing ? 'default' : 'pointer',
                              fontSize: '13px',
                              color: 'var(--color-text-secondary)',
                              borderTop: '1px solid var(--color-border)',
                              transition:
                                'background var(--motion-fast) var(--ease-standard)',
                            }}
                            onMouseEnter={(e) => {
                              if (!isEditing)
                                e.currentTarget.style.background =
                                  'var(--color-surface-subtle)'
                            }}
                            onMouseLeave={(e) => {
                              if (!isEditing)
                                e.currentTarget.style.background = 'transparent'
                            }}
                          >
                            {isEditing ? (
                              // ── Inline editor ──────────────────
                              <div
                                onClick={(e) => e.stopPropagation()}
                                style={{
                                  display: 'flex',
                                  flexDirection: 'column',
                                  gap: 'var(--space-2)',
                                }}
                              >
                                <textarea
                                  aria-label="Edit context"
                                  value={editingDraft}
                                  onChange={(e) => setEditingDraft(e.target.value)}
                                  onKeyDown={(e) => {
                                    // Cmd/Ctrl+Enter saves — matches
                                    // the convention in the block
                                    // editor. Escape cancels.
                                    if (
                                      (e.metaKey || e.ctrlKey) &&
                                      e.key === 'Enter'
                                    ) {
                                      e.preventDefault()
                                      void saveEdit(ref)
                                    } else if (e.key === 'Escape') {
                                      e.preventDefault()
                                      cancelEdit()
                                    }
                                  }}
                                  autoFocus
                                  rows={3}
                                  style={{
                                    width: '100%',
                                    resize: 'vertical',
                                    minHeight: '60px',
                                    padding: '6px 8px',
                                    border: '1px solid var(--color-border)',
                                    borderRadius: 'var(--radius-sm)',
                                    background: 'var(--color-surface)',
                                    color: 'var(--color-text-primary)',
                                    fontSize: '13px',
                                    lineHeight: 1.4,
                                    fontFamily: 'inherit',
                                    outline: 'none',
                                    boxSizing: 'border-box',
                                  }}
                                />
                                <div
                                  style={{
                                    display: 'flex',
                                    gap: 'var(--space-1)',
                                    justifyContent: 'flex-end',
                                  }}
                                >
                                  <button
                                    type="button"
                                    onClick={cancelEdit}
                                    disabled={isSaving}
                                    aria-label="Cancel"
                                    title="Cancel (Esc)"
                                    style={{
                                      display: 'inline-flex',
                                      alignItems: 'center',
                                      gap: '4px',
                                      padding: '4px 8px',
                                      border: '1px solid var(--color-border)',
                                      borderRadius: 'var(--radius-sm)',
                                      background: 'var(--color-surface)',
                                      color: 'var(--color-text-secondary)',
                                      fontSize: '12px',
                                      cursor: isSaving ? 'not-allowed' : 'pointer',
                                      opacity: isSaving ? 0.5 : 1,
                                    }}
                                  >
                                    <X size={11} />
                                    Cancel
                                  </button>
                                  <button
                                    type="button"
                                    onClick={() => void saveEdit(ref)}
                                    disabled={isSaving}
                                    aria-label="Save"
                                    title="Save (⌘+Enter)"
                                    style={{
                                      display: 'inline-flex',
                                      alignItems: 'center',
                                      gap: '4px',
                                      padding: '4px 8px',
                                      border: '1px solid var(--color-accent, var(--color-text-primary))',
                                      borderRadius: 'var(--radius-sm)',
                                      background: 'var(--color-accent, var(--color-text-primary))',
                                      color: 'var(--color-surface)',
                                      fontSize: '12px',
                                      cursor: isSaving ? 'not-allowed' : 'pointer',
                                      opacity: isSaving ? 0.5 : 1,
                                    }}
                                  >
                                    <Check size={11} />
                                    {isSaving ? 'Saving…' : 'Save'}
                                  </button>
                                </div>
                              </div>
                            ) : (
                              // ── Read-only row ──────────────────
                              <div
                                style={{
                                  display: 'flex',
                                  alignItems: 'flex-start',
                                  gap: 'var(--space-2)',
                                }}
                              >
                                <Link2
                                  size={11}
                                  style={{
                                    color: 'var(--color-text-muted)',
                                    flexShrink: 0,
                                    marginTop: '2px',
                                  }}
                                />
                                <span
                                  data-testid={`backlinks-item-context-${ref.sourceBlockId}`}
                                  style={{
                                    flex: 1,
                                    overflow: 'hidden',
                                    textOverflow: 'ellipsis',
                                    display: '-webkit-box',
                                    WebkitLineClamp: 3,
                                    WebkitBoxOrient: 'vertical',
                                    lineHeight: 1.4,
                                  }}
                                >
                                  {ref.context}
                                </span>
                                <button
                                  type="button"
                                  onClick={(e) => {
                                    e.stopPropagation()
                                    startEdit(ref)
                                  }}
                                  aria-label="Edit context"
                                  title="Edit context"
                                  data-testid={`backlinks-item-edit-${ref.sourceBlockId}`}
                                  style={{
                                    background: 'none',
                                    border: 'none',
                                    cursor: 'pointer',
                                    color: 'var(--color-text-muted)',
                                    padding: '2px',
                                    display: 'flex',
                                    alignItems: 'center',
                                    borderRadius: 'var(--radius-sm)',
                                    flexShrink: 0,
                                    opacity: 0.6,
                                    transition: 'opacity var(--motion-fast) var(--ease-standard)',
                                  }}
                                  onMouseEnter={(e) =>
                                    (e.currentTarget.style.opacity = '1')
                                  }
                                  onMouseLeave={(e) =>
                                    (e.currentTarget.style.opacity = '0.6')
                                  }
                                >
                                  <Pencil size={11} />
                                </button>
                              </div>
                            )}
                          </div>
                        )
                      })}
                  </div>
                )
              })}
            </div>
          )}

          {/* Unlinked reference queue (Q029) — appears below the
              linked-references list, hidden when the queue is empty
              and not loading. */}
          {pageName && (
            <UnlinkedRefQueue
              pageName={pageName}
              queue={unlinked.queue}
              loading={unlinked.loading}
              onLink={unlinked.link}
              onDismiss={unlinked.dismiss}
            />
          )}
        </div>
      )}
    </aside>
  )
}
