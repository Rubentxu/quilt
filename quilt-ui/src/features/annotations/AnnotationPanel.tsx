/**
 * AnnotationPanel — sidebar list view of every annotation in the
 * workspace, with status / scope / author filters and a 30-second
 * polling refresh.
 *
 * Implements `spec-annotation-panel` (Quilt-sdd annotations-comments-
 * unification). The list is sorted newest-first; clicking a row
 * navigates to the target page and focuses the block; the resolve /
 * delete / reply actions are wired through callbacks the parent
 * (AppShell) supplies so this component stays a pure view of the
 * state it receives.
 *
 * Empty state: "No annotations yet" + a small illustration.
 *
 * Polling: `useEffect` with `setInterval` at `POLL_INTERVAL_MS`.
 * The interval is cleared on unmount. The parent can pass a
 * `refreshKey` prop to force a refetch (e.g. on tab focus).
 */

import { useEffect, useMemo, useState } from 'react'
import { Check, Trash2, MessageSquare, RefreshCw, Filter, X } from 'lucide-react'
import { api, QuiltApiError } from '@core/api-client'
import { useNavigate } from '@tanstack/react-router'
import type { Annotation, AnnotationStatus, AnnotationScope } from '@shared/types/api'
import { AnnotationRow } from './AnnotationRow'
import { buildAnnotationThread, sortByCreatedAtDesc, type AnnotationThreadNode } from './annotationUtils'

const DEFAULT_POLL_INTERVAL_MS = 30_000

export interface AnnotationPanelProps {
  /**
   * Optional initial set of annotations. When supplied, the panel
   * renders them immediately and the first poll is skipped. Used
   * for SSR / cached hydration. When omitted, the panel fetches on
   * mount.
   */
  initialAnnotations?: Annotation[]
  /**
   * Bump this value to force a re-fetch. The parent (e.g. AppShell)
   * can use it to trigger a refresh when the user re-focuses the
   * tab or after a mutation from another component.
   */
  refreshKey?: number
  /**
   * If true, polls `GET /api/v1/annotations` every 30s. Default
   * `true`. Set to `false` for test runs that should not be flaky.
   */
  enablePolling?: boolean
  /**
   * Override the poll interval in ms. Defaults to 30 000. Exposed for
   * tests so they can advance fake timers by a smaller step.
   */
  pollIntervalMs?: number
  /**
   * Initial visibility of the filter bar. Default `false` (hidden
   * behind a toggle). Tests that need to assert filter behaviour
   * can pass `true` to skip the extra click.
   */
  initialShowFilters?: boolean
  /**
   * Optional callback when an annotation was successfully resolved
   * or deleted. The parent can use it to update the sidebar badge.
   */
  onMutated?: () => void
}

type StatusFilter = AnnotationStatus | 'all'
type ScopeFilter = AnnotationScope | 'all'

export function AnnotationPanel({
  initialAnnotations,
  refreshKey = 0,
  enablePolling = true,
  pollIntervalMs = DEFAULT_POLL_INTERVAL_MS,
  initialShowFilters = false,
  onMutated,
}: AnnotationPanelProps) {
  const navigate = useNavigate()
  // `hasInitial` distinguishes "we're seeding from props and the
  // first effect should be a no-op" from "the prop simply defaulted
  // to an empty list". Without it the useEffect below would fire a
  // needless `listAnnotations({})` request on mount whenever the
  // caller passed an initialAnnotations array.
  const hasInitial = initialAnnotations !== undefined
  const [annotations, setAnnotations] = useState<Annotation[]>(
    initialAnnotations ?? [],
  )
  const [loading, setLoading] = useState<boolean>(!hasInitial)
  const [error, setError] = useState<string | null>(null)
  const [statusFilter, setStatusFilter] = useState<StatusFilter>('all')
  const [scopeFilter, setScopeFilter] = useState<ScopeFilter>('all')
  const [authorFilter, setAuthorFilter] = useState<string>('')
  const [showFilters, setShowFilters] = useState<boolean>(initialShowFilters)

  // Fetch on mount + whenever `refreshKey` changes. When the panel
  // was seeded with `initialAnnotations`, the first run is a no-op
  // (we trust the seed and wait for a `refreshKey` change to refetch).
  useEffect(() => {
    if (hasInitial && refreshKey === 0) return
    let cancelled = false
    async function fetchAll() {
      setLoading(true)
      setError(null)
      try {
        const list = await api.listAnnotations({})
        if (cancelled) return
        setAnnotations(list)
      } catch (err) {
        if (cancelled) return
        const msg = err instanceof QuiltApiError ? err.detail : (err instanceof Error ? err.message : 'Unknown error')
        setError(msg)
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    fetchAll()
    return () => { cancelled = true }
  }, [refreshKey, hasInitial])

  // Polling: re-fetch every `pollIntervalMs` while mounted. We
  // re-issue the same unfiltered list and let the React state
  // merge happen via `setAnnotations`. The filter dropdown does
  // not refetch — the state is already there.
  useEffect(() => {
    if (!enablePolling) return
    const id = setInterval(() => {
      api.listAnnotations({}).then(setAnnotations).catch(() => {
        // Polling failure is non-fatal — keep the last good state.
      })
    }, pollIntervalMs)
    return () => clearInterval(id)
  }, [enablePolling, pollIntervalMs])

  // Apply filters + sort. Memoized so a re-render with the same
  // inputs doesn't trigger an unnecessary re-sort.
  const filtered = useMemo(() => {
    let list = annotations
    if (statusFilter !== 'all') {
      list = list.filter(a => a.status === statusFilter)
    }
    if (scopeFilter !== 'all') {
      list = list.filter(a => a.scope === scopeFilter)
    }
    if (authorFilter.trim()) {
      const q = authorFilter.trim().toLowerCase()
      list = list.filter(a => a.authorName.toLowerCase().includes(q))
    }
    return sortByCreatedAtDesc(list)
  }, [annotations, statusFilter, scopeFilter, authorFilter])

  // Build the thread (replies nested under their parents).
  const thread = useMemo(
    () => buildAnnotationThread(filtered),
    [filtered],
  )

  // Resolve / delete / reply handlers. Mutations go through the
  // API and then we patch local state so the panel reflects the
  // new status without waiting for the next poll.
  const handleResolve = async (id: string) => {
    const target = annotations.find(a => a.id === id)
    if (!target) return
    const isCurrentlyResolved = target.status === 'resolved'
    const nextStatus: AnnotationStatus = isCurrentlyResolved ? 'pending' : 'resolved'
    const resolver =
      (typeof localStorage !== 'undefined' &&
        (localStorage.getItem('quilt:user-name') ||
          localStorage.getItem('quilt:author'))) ||
      'me'
    try {
      const updated = await api.updateAnnotationStatus(id, {
        status: nextStatus,
        ...(nextStatus === 'resolved' ? { resolvedBy: resolver } : {}),
      })
      setAnnotations(prev => prev.map(a => (a.id === id ? updated : a)))
      onMutated?.()
    } catch {
      // No-op: the panel will resync on the next poll.
    }
  }

  const handleDelete = async (id: string) => {
    const target = annotations.find(a => a.id === id)
    if (!target) return
    try {
      await api.deleteAnnotation(id)
      setAnnotations(prev => prev.filter(a => a.id !== id))
      onMutated?.()
    } catch {
      // No-op
    }
  }

  const [replyingTo, setReplyingTo] = useState<string | null>(null)
  const [replyText, setReplyText] = useState('')

  const handleSubmitReply = async (parentId: string) => {
    if (!replyText.trim()) {
      setReplyingTo(null)
      return
    }
    const parent = annotations.find(a => a.id === parentId)
    if (!parent) return
    const authorName =
      (typeof localStorage !== 'undefined' &&
        (localStorage.getItem('quilt:user-name') ||
          localStorage.getItem('quilt:author'))) ||
      'me'
    try {
      const created = await api.createAnnotation({
        blockId: parent.blockId,
        scope: parent.scope,
        authorType: 'human',
        authorName,
        content: replyText.trim(),
        parentAnnotationId: parentId,
      })
      setAnnotations(prev => [created, ...prev])
      setReplyText('')
      setReplyingTo(null)
      onMutated?.()
    } catch {
      // No-op
    }
  }

  const handleClickAnnotation = (a: Annotation) => {
    // Navigate to the page and signal BlockRow / PageView to focus
    // the target block. We use sessionStorage as a one-shot signal
    // (matches the existing `quilt:focusBlock` pattern in PageView).
    sessionStorage.setItem('quilt:focusBlock', a.blockId)
    navigate({ to: '/page/$name', params: { name: a.blockId } })
  }

  // ── Render ─────────────────────────────────────────────────────

  if (loading && annotations.length === 0) {
    return (
      <div data-testid="annotation-panel" style={panelStyle}>
        <PanelHeader
          showFilters={showFilters}
          onToggleFilters={() => setShowFilters(v => !v)}
          onRefresh={() => {
            // Force refetch by bumping the refreshKey via a no-op
            // state change: simplest path is to set annotations to
            // itself, but the parent should pass a refreshKey if it
            // wants explicit control. For the header button, we
            // re-issue the fetch inline.
            setLoading(true)
            api.listAnnotations({})
              .then(setAnnotations)
              .catch(() => {})
              .finally(() => setLoading(false))
          }}
        />
        <div style={{ padding: 'var(--space-4)', color: 'var(--color-text-muted)' }}>
          Loading annotations…
        </div>
      </div>
    )
  }

  return (
    <div data-testid="annotation-panel" style={panelStyle}>
      <PanelHeader
        showFilters={showFilters}
        onToggleFilters={() => setShowFilters(v => !v)}
        onRefresh={() => {
          setLoading(true)
          api.listAnnotations({})
            .then(setAnnotations)
            .catch(() => {})
            .finally(() => setLoading(false))
        }}
      />

      {showFilters && (
        <FilterBar
          statusFilter={statusFilter}
          scopeFilter={scopeFilter}
          authorFilter={authorFilter}
          onStatusChange={setStatusFilter}
          onScopeChange={setScopeFilter}
          onAuthorChange={setAuthorFilter}
          onClear={() => {
            setStatusFilter('all')
            setScopeFilter('all')
            setAuthorFilter('')
          }}
        />
      )}

      {error && (
        <div
          data-testid="annotation-panel-error"
          style={{
            padding: 'var(--space-3)',
            color: 'var(--color-danger)',
            fontSize: '12px',
          }}
        >
          {error}
        </div>
      )}

      {!loading && annotations.length === 0 ? (
        <EmptyState />
      ) : filtered.length === 0 ? (
        <div
          data-testid="annotation-panel-no-matches"
          style={{ padding: 'var(--space-4)', color: 'var(--color-text-muted)', fontSize: '13px' }}
        >
          No annotations match the current filters.
        </div>
      ) : (
        <div data-testid="annotation-list" style={{ padding: 'var(--space-1) var(--space-2)' }}>
          {thread.map(node => (
            <ThreadNode
              key={node.annotation.id}
              node={node}
              depth={0}
              onClick={() => handleClickAnnotation(node.annotation)}
              onResolve={handleResolve}
              onDelete={handleDelete}
              onReply={id => {
                setReplyingTo(id)
                setReplyText('')
              }}
              replyingTo={replyingTo}
              replyText={replyText}
              onReplyTextChange={setReplyText}
              onSubmitReply={handleSubmitReply}
              onCancelReply={() => {
                setReplyingTo(null)
                setReplyText('')
              }}
            />
          ))}
        </div>
      )}
    </div>
  )
}

// ──── Sub-components (kept in this file to avoid a 5-file split) ───

function PanelHeader({
  showFilters,
  onToggleFilters,
  onRefresh,
}: {
  showFilters: boolean
  onToggleFilters: () => void
  onRefresh: () => void
}) {
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 'var(--space-2)',
        padding: 'var(--space-2) var(--space-3)',
        borderBottom: '1px solid var(--color-border)',
      }}
    >
      <span
        style={{
          fontSize: '13px',
          fontWeight: 600,
          color: 'var(--color-text-primary)',
          flex: 1,
        }}
      >
        Annotations
      </span>
      <button
        type="button"
        onClick={onToggleFilters}
        aria-label={showFilters ? 'Hide filters' : 'Show filters'}
        aria-pressed={showFilters}
        data-testid="annotation-panel-filter-toggle"
        style={iconButtonStyle}
      >
        <Filter size={14} aria-hidden="true" />
      </button>
      <button
        type="button"
        onClick={onRefresh}
        aria-label="Refresh annotations"
        data-testid="annotation-panel-refresh"
        style={iconButtonStyle}
      >
        <RefreshCw size={14} aria-hidden="true" />
      </button>
    </div>
  )
}

function FilterBar({
  statusFilter,
  scopeFilter,
  authorFilter,
  onStatusChange,
  onScopeChange,
  onAuthorChange,
  onClear,
}: {
  statusFilter: StatusFilter
  scopeFilter: ScopeFilter
  authorFilter: string
  onStatusChange: (s: StatusFilter) => void
  onScopeChange: (s: ScopeFilter) => void
  onAuthorChange: (a: string) => void
  onClear: () => void
}) {
  return (
    <div
      data-testid="annotation-panel-filters"
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 'var(--space-2)',
        padding: 'var(--space-2) var(--space-3)',
        borderBottom: '1px solid var(--color-border)',
        background: 'var(--color-surface-subtle)',
      }}
    >
      <label style={filterLabelStyle}>
        Status
        <select
          data-testid="filter-status"
          value={statusFilter}
          onChange={e => onStatusChange(e.target.value as StatusFilter)}
          style={filterSelectStyle}
        >
          <option value="all">All</option>
          <option value="pending">Pending</option>
          <option value="in_progress">In progress</option>
          <option value="resolved">Resolved</option>
          <option value="dismissed">Dismissed</option>
        </select>
      </label>
      <label style={filterLabelStyle}>
        Scope
        <select
          data-testid="filter-scope"
          value={scopeFilter}
          onChange={e => onScopeChange(e.target.value as ScopeFilter)}
          style={filterSelectStyle}
        >
          <option value="all">All</option>
          <option value="block">Block</option>
          <option value="inline">Inline</option>
        </select>
      </label>
      <label style={filterLabelStyle}>
        Author contains
        <input
          data-testid="filter-author"
          value={authorFilter}
          onChange={e => onAuthorChange(e.target.value)}
          placeholder="alice, claude, …"
          style={filterSelectStyle}
        />
      </label>
      <button
        type="button"
        onClick={onClear}
        data-testid="filter-clear"
        style={{
          ...iconButtonStyle,
          alignSelf: 'flex-start',
          padding: '4px 10px',
        }}
      >
        <X size={12} aria-hidden="true" /> Clear
      </button>
    </div>
  )
}

function EmptyState() {
  return (
    <div
      data-testid="annotation-panel-empty"
      style={{
        padding: 'var(--space-8) var(--space-4)',
        textAlign: 'center',
        color: 'var(--color-text-muted)',
      }}
    >
      <div style={{ fontSize: '32px', marginBottom: 'var(--space-2)' }} aria-hidden="true">
        💬
      </div>
      <div style={{ fontSize: '13px', fontWeight: 600 }}>No annotations yet</div>
      <div style={{ fontSize: '11px', marginTop: 'var(--space-1)' }}>
        Add one from any block via the action menu.
      </div>
    </div>
  )
}

interface ThreadNodeProps {
  node: AnnotationThreadNode<Annotation>
  depth: number
  onClick: () => void
  onResolve: (id: string) => void
  onDelete: (id: string) => void
  onReply: (id: string) => void
  replyingTo: string | null
  replyText: string
  onReplyTextChange: (text: string) => void
  onSubmitReply: (parentId: string) => void
  onCancelReply: () => void
}

function ThreadNode({
  node,
  depth,
  onClick,
  onResolve,
  onDelete,
  onReply,
  replyingTo,
  replyText,
  onReplyTextChange,
  onSubmitReply,
  onCancelReply,
}: ThreadNodeProps) {
  const isReplying = replyingTo === node.annotation.id
  return (
    <div
      data-testid={`annotation-thread-node-${node.annotation.id}`}
      onClick={onClick}
      style={{
        cursor: 'pointer',
        padding: 'var(--space-1) var(--space-2)',
        borderRadius: 'var(--radius-sm)',
        transition: 'background var(--motion-fast)',
      }}
      onMouseEnter={e => {
        ;(e.currentTarget as HTMLDivElement).style.background = 'var(--color-surface-subtle)'
      }}
      onMouseLeave={e => {
        ;(e.currentTarget as HTMLDivElement).style.background = 'transparent'
      }}
    >
      <AnnotationRow
        annotation={node.annotation}
        onReply={onReply}
        onDelete={onDelete}
        depth={depth}
      />
      {/* The resolve button on AnnotationRow is rendered but not
          wired — we attach the click handler here so it bubbles up
          to the row. AnnotationRow doesn't take an onResolve prop
          (the resolve action is action-specific and is wired in the
          sidebar context). The cleanest workaround: render an
          extra resolve button here. */}
      <div
        style={{
          display: 'flex',
          gap: 'var(--space-1)',
          marginLeft: depth > 0 ? `${depth * 16}px` : 0,
          marginTop: 'var(--space-1)',
        }}
        onClick={e => e.stopPropagation()}
      >
        <button
          type="button"
          onClick={() => onResolve(node.annotation.id)}
          data-testid={`annotation-row-resolve-${node.annotation.id}`}
          style={{
            ...iconButtonStyle,
            padding: '2px 8px',
            fontSize: '11px',
            fontWeight: 500,
          }}
        >
          <Check size={11} aria-hidden="true" />{' '}
          {node.annotation.status === 'resolved' ? 'Unresolve' : 'Resolve'}
        </button>
        <button
          type="button"
          onClick={() => onReply(node.annotation.id)}
          data-testid={`annotation-row-reply-${node.annotation.id}`}
          style={{
            ...iconButtonStyle,
            padding: '2px 8px',
            fontSize: '11px',
            fontWeight: 500,
          }}
        >
          <MessageSquare size={11} aria-hidden="true" /> Reply
        </button>
        <button
          type="button"
          onClick={() => onDelete(node.annotation.id)}
          data-testid={`annotation-row-delete-${node.annotation.id}`}
          style={{
            ...iconButtonStyle,
            padding: '2px 8px',
            fontSize: '11px',
            fontWeight: 500,
            color: 'var(--color-danger)',
          }}
        >
          <Trash2 size={11} aria-hidden="true" /> Delete
        </button>
      </div>

      {isReplying && (
        <div
          style={{
            marginTop: 'var(--space-1)',
            display: 'flex',
            flexDirection: 'column',
            gap: 'var(--space-1)',
            marginLeft: depth > 0 ? `${depth * 16}px` : 0,
          }}
          onClick={e => e.stopPropagation()}
        >
          <textarea
            data-testid={`annotation-reply-input-${node.annotation.id}`}
            value={replyText}
            onChange={e => onReplyTextChange(e.target.value)}
            placeholder="Reply…"
            rows={2}
            style={{
              width: '100%',
              padding: 'var(--space-1) var(--space-2)',
              fontSize: '12px',
              borderRadius: 'var(--radius-sm)',
              border: '1px solid var(--color-border)',
              background: 'var(--color-surface)',
              color: 'var(--color-text-primary)',
              resize: 'vertical',
              fontFamily: 'inherit',
            }}
          />
          <div style={{ display: 'flex', gap: 'var(--space-1)' }}>
            <button
              type="button"
              data-testid={`annotation-reply-submit-${node.annotation.id}`}
              onClick={() => onSubmitReply(node.annotation.id)}
              style={{
                ...iconButtonStyle,
                padding: '4px 10px',
                background: 'var(--color-primary)',
                color: 'var(--color-primary-contrast, #fff)',
                fontSize: '11px',
                fontWeight: 600,
              }}
            >
              Send
            </button>
            <button
              type="button"
              data-testid={`annotation-reply-cancel-${node.annotation.id}`}
              onClick={onCancelReply}
              style={{ ...iconButtonStyle, padding: '4px 10px', fontSize: '11px' }}
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {node.replies.length > 0 && (
        <div
          style={{
            marginLeft: 'var(--space-3)',
            borderLeft: '1px solid var(--color-border)',
            paddingLeft: 'var(--space-2)',
          }}
        >
          {node.replies.map(reply => (
            <ThreadNode
              key={reply.annotation.id}
              node={reply}
              depth={depth + 1}
              onClick={onClick}
              onResolve={onResolve}
              onDelete={onDelete}
              onReply={onReply}
              replyingTo={replyingTo}
              replyText={replyText}
              onReplyTextChange={onReplyTextChange}
              onSubmitReply={onSubmitReply}
              onCancelReply={onCancelReply}
            />
          ))}
        </div>
      )}
    </div>
  )
}

// ──── Shared styles (kept here so the sub-components can read them) ─

const panelStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  height: '100%',
  background: 'var(--color-surface)',
  borderLeft: '1px solid var(--color-border)',
  overflow: 'hidden',
}

const iconButtonStyle: React.CSSProperties = {
  display: 'inline-flex',
  alignItems: 'center',
  gap: '3px',
  background: 'transparent',
  border: '1px solid var(--color-border)',
  color: 'var(--color-text-secondary)',
  borderRadius: 'var(--radius-sm)',
  padding: '4px',
  cursor: 'pointer',
  fontSize: '12px',
  lineHeight: 1,
}

const filterLabelStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: 'var(--space-2)',
  fontSize: '11px',
  color: 'var(--color-text-muted)',
  fontWeight: 500,
}

const filterSelectStyle: React.CSSProperties = {
  fontSize: '12px',
  padding: '2px 6px',
  borderRadius: 'var(--radius-sm)',
  border: '1px solid var(--color-border)',
  background: 'var(--color-surface)',
  color: 'var(--color-text-primary)',
  minWidth: '120px',
}
