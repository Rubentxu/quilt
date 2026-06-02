import { useState, useEffect, useCallback, useRef, useMemo } from 'react'
import { Calendar, Plus, ChevronLeft, ChevronRight, MoreHorizontal } from 'lucide-react'
import { useNavigate } from '@tanstack/react-router'
import toast from 'react-hot-toast'
import { DndContext, closestCenter, useDndContext, type DragEndEvent, PointerSensor, useSensor, useSensors } from '@dnd-kit/core'
import { SortableContext, verticalListSortingStrategy, useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { useWasm, ensureWasmLoaded } from '@core/wasm-bridge/WasmProvider'
import { api, QuiltApiError } from '@core/api-client'
import { PageSkeleton } from '@shared/components/Skeleton'
import { BlockRow } from './BlockRow'
import { setCursorAt } from '@shared/hooks/useCursor'
import { useBlockHistory } from '@shared/hooks/useBlockHistory'
import { useSSE } from '@shared/hooks/useSSE'
import { usePollingSync } from '@shared/hooks/usePollingSync'
import { useConnection } from '@shared/contexts/ConnectionContext'
import type { Block } from '@shared/types/api'
import { flattenBlockTree, type FlatBlock } from './flattenTree'
import { formatJournalDate } from '@shared/utils/formatJournalDate'

interface PageViewProps {
  pageName: string
  isJournal?: boolean
  journalFormat?: string
}

// ──── JournalDateHeader (DESIGN.md §9.4) ────────────────────────────
// Big date (display-sm: 36px bold), calendar icon, horizontal line

function JournalDateHeader({ pageName, format }: { pageName: string; format?: string }) {
  const navigate = useNavigate()
  const dateSegments = pageName.split('/').pop()
  const dateStr = dateSegments ?? pageName

  let displayDate = dateStr
  if (/^\d{4}-\d{2}-\d{2}$/.test(dateStr)) {
    try {
      const d = new Date(dateStr + 'T00:00:00')

      // Use the user-configured format if provided
      if (format) {
        // The format from settings is a strftime-like pattern
        // Convert common patterns to a formatted string
        displayDate = formatJournalDate(d, format)
      } else {
        // Match the visual reference and DESIGN.md examples: dd-mm-yyyy
        const dd = String(d.getDate()).padStart(2, '0')
        const mm = String(d.getMonth() + 1).padStart(2, '0')
        const yyyy = d.getFullYear()
        displayDate = `${dd}-${mm}-${yyyy}`
      }
    } catch {
      // fallback to raw string
    }
  }

  function navigateToPrevDay() {
    const d = new Date(dateStr + 'T00:00:00')
    d.setDate(d.getDate() - 1)
    const prev = d.toISOString().split('T')[0]
    navigate({ to: '/journal/$date', params: { date: prev } })
  }

  function navigateToNextDay() {
    const d = new Date(dateStr + 'T00:00:00')
    d.setDate(d.getDate() + 1)
    const next = d.toISOString().split('T')[0]
    navigate({ to: '/journal/$date', params: { date: next } })
  }

  return (
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 'var(--space-4)',
          marginBottom: 'var(--space-6)',
        }}
      >
        <Calendar
          size={20}
          style={{
            color: 'var(--color-text-muted)',
            flexShrink: 0,
          }}
        />
        <h1
          style={{
            fontSize: '36px',
            fontWeight: 700,
            color: 'var(--color-text-primary)',
            lineHeight: 1.2,
            letterSpacing: '-0.02em',
            margin: 0,
          }}
        >
          {displayDate}
        </h1>

        <div style={{ flex: 1, height: '1px', background: 'var(--color-border)' }} />

        <button
          type="button"
          style={{
            height: '36px',
            padding: '0 14px',
            borderRadius: 'var(--radius-md)',
            border: '1px solid var(--color-border)',
            background: 'var(--color-surface)',
            color: 'var(--color-text-secondary)',
            fontSize: '14px',
            fontWeight: 500,
            cursor: 'pointer',
            boxShadow: 'var(--shadow-sm)',
          }}
        >
          Hoy
        </button>

        {/* Prev / Next day navigation */}
        <div style={{ display: 'flex', gap: 'var(--space-1)' }}>
          <button
            onClick={navigateToPrevDay}
            data-testid="nav-prev-day"
            className="ghost-icon-button"
            style={{
              background: 'transparent',
              border: '1px solid var(--color-border)',
              borderRadius: 'var(--radius-sm)',
              padding: '8px',
              cursor: 'pointer',
              color: 'var(--color-text-muted)',
              display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
          aria-label="Previous day"
          title="Previous day"
        >
          <ChevronLeft size={16} />
        </button>
          <button
            onClick={navigateToNextDay}
            data-testid="nav-next-day"
            className="ghost-icon-button"
            style={{
              background: 'transparent',
              border: '1px solid var(--color-border)',
              borderRadius: 'var(--radius-sm)',
              padding: '8px',
              cursor: 'pointer',
              color: 'var(--color-text-muted)',
              display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
          aria-label="Next day"
          title="Next day"
          >
            <ChevronRight size={16} />
          </button>
          <button
            type="button"
            className="ghost-icon-button"
            aria-label="More day actions"
            style={{
              width: '34px',
              height: '34px',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              cursor: 'pointer',
            }}
          >
            <MoreHorizontal size={16} />
          </button>
        </div>
      </div>
  )
}

// ──── Empty state (DESIGN.md §15) ───────────────────────────────────

function EmptyState({ isJournal, onNewBlock }: { isJournal?: boolean; onNewBlock: () => void }) {
  return (
    <div
      style={{
        textAlign: 'center',
        padding: 'var(--space-12) var(--space-4)',
        color: 'var(--color-text-muted)',
      }}
    >
      <FileTextIcon />
      <h3
        style={{
          fontSize: '16px',
          fontWeight: 600,
          color: 'var(--color-text-secondary)',
          margin: 'var(--space-4) 0 var(--space-2)',
        }}
      >
        {isJournal ? 'No entries yet' : 'This page is empty'}
      </h3>
      <p
        style={{
          fontSize: '13px',
          color: 'var(--color-text-muted)',
          maxWidth: '320px',
          margin: '0 auto',
          lineHeight: 1.5,
        }}
      >
        {isJournal
          ? 'Start writing your daily notes below.'
          : 'No blocks yet. Start typing...'}
      </p>
      <button
        onClick={onNewBlock}
        style={{
          marginTop: 'var(--space-4)',
          padding: '8px 20px',
          fontSize: '14px',
          fontWeight: 500,
          background: 'var(--color-primary)',
          color: 'var(--color-on-primary, #fff)',
          border: 'none',
          borderRadius: 'var(--radius-md)',
          cursor: 'pointer',
        }}
      >
        Add first block
      </button>
    </div>
  )
}

function FileTextIcon() {
  return (
    <svg
      width="48"
      height="48"
      viewBox="0 0 48 48"
      fill="none"
      stroke="currentColor"
      strokeWidth="1"
      style={{ color: 'var(--color-text-disabled)', opacity: 0.5 }}
    >
      <path d="M16 6h12l8 8v26a4 4 0 01-4 4H16a4 4 0 01-4-4V10a4 4 0 014-4z" strokeLinecap="round" strokeLinejoin="round" />
      <path d="M28 6v8h8" strokeLinecap="round" strokeLinejoin="round" />
      <path d="M18 28h12M18 34h8" strokeLinecap="round" />
    </svg>
  )
}

// ──── Error state ───────────────────────────────────────────────────

function PageViewError({ message }: { message: string }) {
  return (
    <div style={{ textAlign: 'center', padding: 'var(--space-12) var(--space-4)' }}>
      <p
        style={{
          fontSize: '14px',
          fontWeight: 600,
          color: 'var(--color-danger)',
          marginBottom: 'var(--space-2)',
        }}
      >
        Failed to load page
      </p>
      <p style={{ fontSize: '12px', color: 'var(--color-text-muted)' }}>{message}</p>
    </div>
  )
}

// ──── Sortable block wrapper for flat virtual list ─────────────────

interface SortableBlockRowFlatProps {
  flatBlock: FlatBlock
  allBlocks: Block[]
  pageName: string
  collapsedIds: Set<string>
  blockRefs: Map<string, HTMLDivElement>
  onToggleCollapse: (blockId: string) => void
  onUpdate: (block: Block) => void
  onCreateBlock: (afterBlockId: string, content: string, parentId: string | null) => void
  onDeleteBlock: (blockId: string) => void
  onFocusBlock: (blockId: string, cursorPos: 'start' | 'end') => void
  onMoveBlockUp: (blockId: string) => void
  onMoveBlockDown: (blockId: string) => void
  onUndo: () => void
  onRedo: () => void
  selected: boolean
  onMultiSelect: (blockId: string, direction: 'up' | 'down') => void
  onSelectAll?: () => void
  onSelectParent?: (blockId: string, parentId: string | null) => void
  onAddComment?: (blockId: string) => void
  onResolveComment?: (commentId: string) => void
  onReplyComment?: (commentId: string) => void
  onDeleteComment?: (commentId: string) => void
}

function SortableBlockRowFlat({
  flatBlock,
  allBlocks,
  pageName,
  collapsedIds,
  blockRefs,
  onToggleCollapse,
  onUpdate,
  onCreateBlock,
  onDeleteBlock,
  onFocusBlock,
  onMoveBlockUp,
  onMoveBlockDown,
  onUndo,
  onRedo,
  selected,
  onMultiSelect,
  onSelectAll,
  onSelectParent,
  onAddComment,
  onResolveComment,
  onReplyComment,
  onDeleteComment,
}: SortableBlockRowFlatProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    setActivatorNodeRef,
    transform,
    transition,
    isDragging,
    isOver,
    over,
  } = useSortable({ id: flatBlock.block.id })

  // ── Drop indicator ────────────────────────────────────────────────
  const dndContext = useDndContext()
  const activeId = dndContext.active?.id ? String(dndContext.active.id) : null

  const dropIndicator = useMemo<'top' | 'child' | null>(() => {
    if (!isOver || !over || !activeId) return null
    const overId = String(over.id)
    if (overId === activeId) return null // Can't drop on self

    const activeBlock = allBlocks.find(b => b.id === activeId)
    const overBlock = allBlocks.find(b => b.id === overId)
    if (!activeBlock || !overBlock) return null

    // Same parent → reorder: show line above target
    if (activeBlock.parentId === overBlock.parentId) return 'top'
    // Cross-parent → drop as child of overBlock
    return 'child'
  }, [isOver, over, activeId, allBlocks])

  const style: React.CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
    position: 'relative',
  }

  return (
    <div ref={setNodeRef} style={style}>
      {/* Drop indicator — line at top (same-parent reorder) */}
      {dropIndicator === 'top' && (
        <div
          style={{
            position: 'absolute',
            top: -2,
            left: 0,
            right: 0,
            height: 3,
            background: 'var(--color-accent)',
            borderRadius: '2px',
            pointerEvents: 'none',
            zIndex: 10,
          }}
        />
      )}

      {/* Drop indicator — child drop (cross-parent) */}
      {dropIndicator === 'child' && (
        <div
          style={{
            position: 'absolute',
            top: -2,
            left: `${flatBlock.depth * 24 + 12}px`,
            right: 0,
            height: 3,
            background: 'var(--color-primary)',
            borderRadius: '2px',
            pointerEvents: 'none',
            zIndex: 10,
          }}
        />
      )}

      <div
        ref={el => {
          if (el) blockRefs.set(flatBlock.block.id, el)
          else blockRefs.delete(flatBlock.block.id)
        }}
      >
        <BlockRow
          block={flatBlock.block}
          allBlocks={allBlocks}
          pageName={pageName}
          hasChildren={flatBlock.hasChildren}
          isCollapsed={collapsedIds.has(flatBlock.block.id)}
          onToggleCollapse={onToggleCollapse}
          onUpdate={onUpdate}
          onCreateBlock={onCreateBlock}
          onDeleteBlock={onDeleteBlock}
           onFocusBlock={onFocusBlock}
           onMoveBlockUp={onMoveBlockUp}
           onMoveBlockDown={onMoveBlockDown}
           onUndo={onUndo}
           onRedo={onRedo}
           selected={selected}
           onMultiSelect={onMultiSelect}
           onSelectAll={onSelectAll}
           onSelectParent={onSelectParent}
           indent={flatBlock.depth}
          onAddComment={onAddComment}
          onResolveComment={onResolveComment}
          onReplyComment={onReplyComment}
          onDeleteComment={onDeleteComment}
          dragHandleProps={{ ref: setActivatorNodeRef, ...attributes, ...listeners }}
        />
      </div>
    </div>
  )
}

// ──── PageView ──────────────────────────────────────────────────────

export function PageView({ pageName, isJournal, journalFormat }: PageViewProps) {
  const { loaded: wasmLoaded, wasmLoadPage } = useWasm()
  const [blocks, setBlocks] = useState<Block[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [collapsedIds, setCollapsedIds] = useState<Set<string>>(new Set())
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set())
  const [selectionAnchor, setSelectionAnchor] = useState<string | null>(null)

  // Undo/redo history — backed by the Rust `HistoryStack` via WASM.
  // The hook re-initialises the WASM stack whenever `pageName` changes
  // and exposes `applyCommand` for any operation that maps to an
  // `OutlinerCommand` (SetContent, SplitBlock, etc.). Operations that
  // do not have a command variant (Create, Delete) still update local
  // state directly and are not tracked in undo history.
  const {
    applyCommand: wasmApplyCommand,
    undo: wasmUndo,
    redo: wasmRedo,
    canUndo,
    canRedo,
  } = useBlockHistory({
    pageName,
    blocks,
    onBlocksChanged: setBlocks,
    enabled: wasmLoaded && !!pageName,
  })

  // ── SSE + Polling sync ────────────────────────────────────────────

  // SSE connection — tries to connect, falls back to polling
  const { connected: sseConnected } = useSSE({
    url: `${api.baseUrl}/api/v1/events`,
    onEvent: (event) => {
      switch (event.type) {
        case 'block_updated':
          setBlocks(prev => prev.map(b =>
            b.id === event.data.id ? { ...b, ...event.data } : b
          ))
          break
        case 'block_created':
          setBlocks(prev => {
            if (prev.some(b => b.id === event.data.id)) return prev
            return [...prev, event.data]
          })
          break
        case 'block_deleted':
          setBlocks(prev => prev.filter(b => b.id !== event.data.id))
          break
        case 'page_updated':
          // Could update page metadata
          break
      }
    },
    enabled: !!pageName,
  })

  // Sync connection status to global context (for AppShell indicator)
  const { setSseConnected } = useConnection()

  useEffect(() => {
    setSseConnected(sseConnected)
  }, [sseConnected, setSseConnected])

  // Polling fallback — only active when SSE is NOT connected
  usePollingSync({
    pageName,
    interval: 15000,
    onBlocksChanged: setBlocks,
    enabled: !!pageName && !sseConnected,
  })

  // Ref to current blocks for history snapshot captures inside callbacks
  const blocksRef = useRef<Block[]>(blocks)
  blocksRef.current = blocks

  // Refs for focusing blocks
  const blockRefs = useRef<Map<string, HTMLDivElement>>(new Map()).current
  // Container ref for detecting clicks on empty space
  const containerRef = useRef<HTMLDivElement>(null)

  // Set of page names we've already auto-created the first block on. This
  // prevents a reload from creating duplicate empty blocks and survives
  // StrictMode's double-invocation of effects.
  const autoCreatedRef = useRef<Set<string>>(new Set())

  // A ref-mirrored copy of `handleNewBlockAtEnd` so the load effect can
  // call it without re-running every time the underlying callback changes
  // (we don't want the load effect to retrigger on every keystroke).
  const handleNewBlockAtEndRef = useRef<(() => void) | null>(null)

  // ── Load blocks ───────────────────────────────────────────────
  useEffect(() => {
    let cancelled = false

    async function load() {
      setLoading(true)
      setError(null)

      try {
        const fetchedBlocks = await api.getPageBlocks(pageName)
        if (cancelled) return

        // Load into WASM (non-blocking for rendering). We trigger the
        // lazy load here on first-use; the promise is cached so
        // subsequent calls share the in-flight fetch.
        if (!wasmLoaded) {
          try {
            await ensureWasmLoaded()
          } catch {
            // Engine failed to initialise — fall back to API-only mode
            if (cancelled) return
            setBlocks(fetchedBlocks)
            setLoading(false)
            return
          }
        }
        if (cancelled) return

        try {
          wasmLoadPage(pageName, fetchedBlocks)
        } catch (e) {
          console.warn('WASM load failed, rendering from API data:', e)
        }

        setBlocks(fetchedBlocks)
        setLoading(false)

        // Auto-create first block on a new journal page. A daily journal
        // is the user's primary "scratch pad" — dropping them into an
        // empty state on first visit is friction. We only fire this once
        // per page (guarded by autoCreatedRef) and only for journals, so
        // regular pages still show the EmptyState until the user is ready
        // to write.
        if (isJournal && fetchedBlocks.length === 0 && !autoCreatedRef.current.has(pageName)) {
          autoCreatedRef.current.add(pageName)
          // Defer the creation so it doesn't race with the initial render
          // and so the block row exists in the DOM when we attempt to
          // focus it.
          requestAnimationFrame(() => {
            if (cancelled) return
            handleNewBlockAtEndRef.current?.()
          })
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : 'Unknown error')
          setLoading(false)
        }
      }
    }

    load()
    return () => { cancelled = true }
  }, [pageName, wasmLoaded, wasmLoadPage])

  // ── Virtual-scroll flat blocks ────────────────────────────────
  const flatBlocks = useMemo(
    () => flattenBlockTree(blocks, null, collapsedIds),
    [blocks, collapsedIds],
  )
  const flatBlocksRef = useRef<FlatBlock[]>(flatBlocks)
  flatBlocksRef.current = flatBlocks

  const sortableIds = useMemo(
    () => flatBlocks.map(fb => fb.block.id),
    [flatBlocks],
  )

  // ── Focus management ──────────────────────────────────────────
  const onFocusBlock = useCallback(
    (blockId: string, cursorPos: 'start' | 'end') => {
      // Focus the contentEditable inside the block row. If the block is
      // still in read mode (e.g. just-created block, where isEditing is
      // false), the contentEditable doesn't exist yet. In that case we
      // click the read-mode div to enter edit mode and then focus the
      // contentEditable on the next frame.
      const wrapper = blockRefs.get(blockId)
      if (!wrapper) return

      // Native scrollIntoView is enough here and avoids the layout issues
      // we were seeing with Virtuoso in auto-height containers.
      wrapper.scrollIntoView({ behavior: 'smooth', block: 'center' })

      const tryFocus = () => {
        const ce = wrapper.querySelector<HTMLDivElement>('[contenteditable="true"]')
        if (ce) {
          ce.focus()
          setCursorAt(ce, cursorPos)
          return true
        }
        return false
      }

      if (tryFocus()) return

      // Block is in read mode — click it to enter edit mode, then focus
      // the contentEditable once React has rendered it.
      const readMode = wrapper.querySelector<HTMLDivElement>('.block-content-read')
      if (readMode) {
        readMode.click()
        requestAnimationFrame(() => {
          // Fall back to one more frame if React hasn't yet rendered the
          // contentEditable (very fast click→focus race).
          if (!tryFocus()) {
            requestAnimationFrame(tryFocus)
          }
        })
      }
    },
    [blockRefs],
  )

  // ── Block updates ─────────────────────────────────────────────
  const handleBlockUpdate = useCallback((updatedBlock: Block) => {
    const before = blocksRef.current.find(b => b.id === updatedBlock.id)
    if (before && before.content !== updatedBlock.content) {
      // Record the change in the Rust HistoryStack via WASM. The hook
      // will also call `setBlocks` with the new state — so we skip the
      // local `setBlocks` call here to avoid a double update.
      const recorded = wasmApplyCommand({
        type: 'setContent',
        blockId: updatedBlock.id,
        before: before.content,
        after: updatedBlock.content,
      })
      if (!recorded) {
        // WASM not ready or stack missing — fall back to local update.
        setBlocks(prev =>
          prev.map(b => (b.id === updatedBlock.id ? updatedBlock : b)),
        )
      }
    } else {
      // Metadata-only change (marker, priority, etc.) — no history,
      // but still apply the new block.
      setBlocks(prev =>
        prev.map(b => (b.id === updatedBlock.id ? updatedBlock : b)),
      )
    }
  }, [wasmApplyCommand])

  const handleDeleteBlock = useCallback((blockId: string) => {
    // No `Delete` command in the Rust `OutlinerCommand` enum yet —
    // we still need to update local state. The block can be recreated
    // by re-issuing the create-block call, but the delete is not
    // undoable through the WASM history. (Adding `Delete` is a
    // separate task; tracked as a follow-up.)
    setBlocks(prev => prev.filter(b => b.id !== blockId))
  }, [])

  const handleCreateBlock = useCallback(
    async (afterBlockId: string, content: string, parentId: string | null) => {
      // Create is not in the WASM history yet (no `Create` variant in
      // `OutlinerCommand`). The new block is appended to local state
      // optimistically and replaced with the server's response below.
      // The dependency list now needs `[wasmApplyCommand]` removed
      // because we no longer call the hook from here.

      // Find the block to create after to compute order
      const afterBlock = blocks.find(b => b.id === afterBlockId)
      const newOrder = afterBlock ? afterBlock.order + 0.5 : 1
      const level = afterBlock ? afterBlock.level : 0

      // Optimistic local creation
      const tempId = `temp-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`
      const optimisticBlock: Block = {
        id: tempId,
        pageId: afterBlock?.pageId ?? '',
        pageName,
        content,
        blockType: 'paragraph',
        marker: null,
        priority: null,
        parentId,
        order: newOrder,
        level,
        collapsed: false,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      }

      setBlocks(prev => {
        const next = [...prev, optimisticBlock]
        return next
      })

      // API call
      let realCreatedId = tempId
      try {
        const created = await api.createBlock({
          pageName,
          content,
          parentId,
          // Pass precedingBlockId when inserting after a specific block
          ...(afterBlockId ? { precedingBlockId: afterBlockId } : {}),
        })
        realCreatedId = created.id
        // Replace temp ID with real block
        setBlocks(prev =>
          prev.map(b => (b.id === tempId ? created : b)),
        )
      } catch {
        toast.error('Failed to create block')
        // Revert: remove the optimistic block
        setBlocks(prev => prev.filter(b => b.id !== tempId))
        return
      }

      // Focus the new block after React commits the render
      requestAnimationFrame(() => {
        onFocusBlock(realCreatedId, 'end')
      })
    },
    [blocks, pageName, onFocusBlock],
  )

  // ── DnD sensors ──────────────────────────────────────────────
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } })
  )

  // ── DnD drag end handler ─────────────────────────────────────
  const handleDragEnd = useCallback((event: DragEndEvent) => {
    const { active, over } = event
    if (!over || active.id === over.id) return

    const activeId = String(active.id)
    const overId = String(over.id)

    const currentBlocks = blocksRef.current
    const activeBlock = currentBlocks.find(b => b.id === activeId)
    const overBlock = currentBlocks.find(b => b.id === overId)
    if (!activeBlock || !overBlock) return

    // ── Same-parent: reorder (existing logic) ──
    if (activeBlock.parentId === overBlock.parentId) {
      const siblings = currentBlocks
        .filter(b => b.parentId === overBlock.parentId)
        .sort((a, b) => a.order - b.order)

      const overIdx = siblings.findIndex(b => b.id === overId)
      let newOrder: number

      if (overIdx === 0) {
        newOrder = siblings[0].order - 1
      } else if (overIdx === siblings.length - 1) {
        newOrder = siblings[siblings.length - 1].order + 1
      } else {
        newOrder = (siblings[overIdx - 1].order + siblings[overIdx + 1].order) / 2
      }

      // Optimistic update
      setBlocks(prev => prev.map(b =>
        b.id === activeId ? { ...b, order: newOrder } : b
      ))

      // Persist to API
      api.updateBlock(activeId, { order: newOrder }).catch(() => {
        toast.error('Failed to reorder')
        api.getPageBlocks(pageName).then(fetched => setBlocks(fetched)).catch(console.error)
      })
      return
    }

    // ── Cross-parent: drop activeBlock as child of overBlock ──
    const newParentId = overBlock.id
    const newLevel = (overBlock.level ?? 0) + 1

    // Compute order: place as last child of overBlock
    const childrenOfOver = currentBlocks.filter(b => b.parentId === overBlock.id)
    const newOrder = childrenOfOver.length > 0
      ? Math.max(...childrenOfOver.map(b => b.order)) + 1
      : 1.0

    // Optimistic update
    setBlocks(prev => prev.map(b =>
      b.id === activeId
        ? { ...b, parentId: newParentId, order: newOrder, level: newLevel }
        : b
    ))

    // Persist to API
    api.updateBlock(activeId, { parentId: newParentId, order: newOrder, level: newLevel })
      .catch(() => {
        toast.error('Failed to move block')
        api.getPageBlocks(pageName).then(fetched => setBlocks(fetched)).catch(console.error)
      })
  }, [pageName])

  // ── Move block up handler (KBD-014: Alt+Shift+Up) ──────────────────
  const handleMoveBlockUp = useCallback((blockId: string) => {
    const currentBlocks = blocksRef.current
    const current = currentBlocks.find(b => b.id === blockId)
    if (!current) return

    const siblings = currentBlocks
      .filter(b => b.parentId === current.parentId)
      .sort((a, b) => a.order - b.order)

    const currentIdx = siblings.findIndex(b => b.id === blockId)
    if (currentIdx <= 0) return // Already at top

    const prev = siblings[currentIdx - 1]
    const newOrder = prev.order - 1 // Place before prev

    // Optimistic update
    setBlocks(prevBlocks =>
      prevBlocks.map(b => (b.id === blockId ? { ...b, order: newOrder } : b)),
    )

    // Persist
    api.updateBlock(blockId, { order: newOrder }).catch(() => {
      toast.error('Failed to move block up')
      api.getPageBlocks(pageName).then(fetched => setBlocks(fetched)).catch(console.error)
    })
  }, [pageName])

  // ── Move block down handler (KBD-015: Alt+Shift+Down) ──────────────
  const handleMoveBlockDown = useCallback((blockId: string) => {
    const currentBlocks = blocksRef.current
    const current = currentBlocks.find(b => b.id === blockId)
    if (!current) return

    const siblings = currentBlocks
      .filter(b => b.parentId === current.parentId)
      .sort((a, b) => a.order - b.order)

    const currentIdx = siblings.findIndex(b => b.id === blockId)
    if (currentIdx >= siblings.length - 1) return // Already at bottom

    const next = siblings[currentIdx + 1]
    const newOrder = next.order + 1 // Place after next

    // Optimistic update
    setBlocks(prevBlocks =>
      prevBlocks.map(b => (b.id === blockId ? { ...b, order: newOrder } : b)),
    )

    // Persist
    api.updateBlock(blockId, { order: newOrder }).catch(() => {
      toast.error('Failed to move block down')
      api.getPageBlocks(pageName).then(fetched => setBlocks(fetched)).catch(console.error)
    })
  }, [pageName])

  // ── Collapse toggle ───────────────────────────────────────────
  const handleToggleCollapse = useCallback((blockId: string) => {
    setCollapsedIds(prev => {
      const next = new Set(prev)
      if (next.has(blockId)) next.delete(blockId)
      else next.add(blockId)
      return next
    })
  }, [])

  // ── Create block at end (new block button) ────────────────────
  const handleNewBlockAtEnd = useCallback(() => {
    const topLevelBlocks = blocks.filter(b => !b.parentId)
    const lastBlock = topLevelBlocks.length > 0
      ? topLevelBlocks.reduce((max, b) => (b.order > max.order ? b : max))
      : null

    if (lastBlock) {
      handleCreateBlock(lastBlock.id, '', null)
    } else {
      handleCreateBlock('', '', null)
    }
  }, [blocks, handleCreateBlock])

  // Mirror the latest callback into a ref so the auto-create effect can
  // call it without re-running on every blocks change.
  useEffect(() => {
    handleNewBlockAtEndRef.current = handleNewBlockAtEnd
  }, [handleNewBlockAtEnd])

  // ── Comment handlers ─────────────────────────────────────────────
  // Comments are regular blocks with `type: "comment"` and
  // `resolved: "false"|"true"` properties. They are stored as
  // children of the block they comment on.

  /**
   * Add a new comment as a child of the given block.
   * Prompts the user for comment text, then creates a child block
   * with the comment metadata properties.
   */
  const handleAddComment = useCallback(
    async (blockId: string) => {
      const commentText = window.prompt('Add comment:')
      if (!commentText?.trim()) return

      const targetBlock = blocks.find(b => b.id === blockId)
      if (!targetBlock) return

      // New comments go after the last child of the target block
      const childrenOfTarget = blocks.filter(b => b.parentId === blockId)
      const precedingBlockId =
        childrenOfTarget.length > 0
          ? childrenOfTarget.reduce((max, b) => (b.order > max.order ? b : max)).id
          : blockId

      // Best-effort: pick an author identifier from localStorage /
      // env. Falls back to "anonymous" if not set, so comments still
      // work for unauthenticated users.
      const createdBy =
        (typeof localStorage !== 'undefined' &&
          (localStorage.getItem('quilt:user-name') ||
            localStorage.getItem('quilt:author'))) ||
        'anonymous'

      try {
        const created = await api.createBlock({
          pageName,
          content: commentText.trim(),
          parentId: blockId,
          precedingBlockId,
          properties: {
            type: 'comment',
            resolved: 'false',
            created_at: new Date().toISOString(),
            created_by: createdBy,
          },
        })
        setBlocks(prev => [...prev, created])
        toast.success('Comment added')
      } catch (err) {
        const message = err instanceof Error ? err.message : 'Unknown error'
        toast.error('Failed to add comment: ' + message)
      }
    },
    [blocks, pageName],
  )

  /**
   * Toggle a comment's `resolved` property. Uses the existing
   * `setBlockProperty` API endpoint.
   */
  const handleResolveComment = useCallback(
    async (commentId: string) => {
      const block = blocksRef.current.find(b => b.id === commentId)
      if (!block) return

      const current = String(
        block.properties?.find(p => p.key === 'resolved')?.value ?? 'false',
      )
      const next = current === 'true' ? 'false' : 'true'

      // Optimistic update
      setBlocks(prev =>
        prev.map(b => {
          if (b.id !== commentId) return b
          const props = [...(b.properties ?? [])]
          const idx = props.findIndex(p => p.key === 'resolved')
          if (idx >= 0) {
            props[idx] = { ...props[idx], value: next }
          } else {
            props.push({ key: 'resolved', value: next, type: 'boolean' })
          }
          return { ...b, properties: props }
        }),
      )

      try {
        await api.setBlockProperty(commentId, 'resolved', next)
      } catch {
        toast.error('Failed to update comment')
        // Revert by re-fetching the page
        api.getPageBlocks(pageName).then(setBlocks).catch(console.error)
      }
    },
    [pageName],
  )

  /**
   * Add a reply to a comment. Reply is a child comment of the given
   * comment block. Reuses the same prompt UX as the main add handler.
   */
  const handleReplyComment = useCallback(
    async (commentId: string) => {
      const replyText = window.prompt('Reply:')
      if (!replyText?.trim()) return

      const parent = blocksRef.current.find(b => b.id === commentId)
      if (!parent) return

      const siblings = blocks.filter(b => b.parentId === commentId)
      const precedingBlockId =
        siblings.length > 0
          ? siblings.reduce((max, b) => (b.order > max.order ? b : max)).id
          : commentId

      const createdBy =
        (typeof localStorage !== 'undefined' &&
          (localStorage.getItem('quilt:user-name') ||
            localStorage.getItem('quilt:author'))) ||
        'anonymous'

      try {
        const created = await api.createBlock({
          pageName,
          content: replyText.trim(),
          parentId: commentId,
          precedingBlockId,
          properties: {
            type: 'comment',
            resolved: 'false',
            created_at: new Date().toISOString(),
            created_by: createdBy,
          },
        })
        setBlocks(prev => [...prev, created])
        toast.success('Reply added')
      } catch (err) {
        const message = err instanceof Error ? err.message : 'Unknown error'
        toast.error('Failed to add reply: ' + message)
      }
    },
    [blocks, pageName],
  )

  /**
   * Delete a comment (and its replies, since the database cascade
   * behaviour is not enforced for comment hierarchies).
   */
  const handleDeleteComment = useCallback(
    async (commentId: string) => {
      // No `Delete` command in the Rust `OutlinerCommand` enum yet —
      // we still need to update local state. Not undoable for now.
      // Optimistic remove — also drop any descendant comments
      const toDelete = new Set<string>([commentId])
      let changed = true
      while (changed) {
        changed = false
        for (const b of blocksRef.current) {
          if (b.parentId && toDelete.has(b.parentId) && !toDelete.has(b.id)) {
            toDelete.add(b.id)
            changed = true
          }
        }
      }
      setBlocks(prev => prev.filter(b => !toDelete.has(b.id)))

      try {
        await Promise.all(
          Array.from(toDelete).map(id => api.deleteBlock(id)),
        )
      } catch (err) {
        if (err instanceof QuiltApiError && err.status === 409) {
          // The server refused to delete a block that still has children.
          // Since we already cascade-collected descendants on the client,
          // this is a surprising state — fall back to a generic actionable
          // message and resync from the server.
          toast.error('Cannot delete: block has children. Delete them first.')
        } else {
          toast.error('Failed to delete comment')
        }
        api.getPageBlocks(pageName).then(setBlocks).catch(console.error)
      }
    },
    [pageName],
  )

  // ── Click empty area below last block ─────────────────────────
  const handleContainerClick = useCallback(
    (e: React.MouseEvent) => {
      // Only trigger if clicking directly on the container (not on a block)
      if (e.target === containerRef.current) {
        handleNewBlockAtEnd()
      }
    },
    [handleNewBlockAtEnd],
  )

  // ── Undo / Redo ─────────────────────────────────────────────
  // Delegates to the Rust `HistoryStack` through the WASM bridge.
  // The hook's `onBlocksChanged` callback updates `setBlocks` for us,
  // so we just call undo/redo here.
  const handleUndo = useCallback(() => {
    wasmUndo()
  }, [wasmUndo])

  const handleRedo = useCallback(() => {
    wasmRedo()
  }, [wasmRedo])

  // ── Global keyboard shortcuts (Ctrl+Z / Ctrl+Shift+Z) ──────
  useEffect(() => {
    function handleGlobalKeyDown(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key === 'z' && !e.shiftKey) {
        e.preventDefault()
        handleUndo()
      }
      if ((e.ctrlKey || e.metaKey) && (e.key === 'Z' || (e.key === 'z' && e.shiftKey))) {
        e.preventDefault()
        handleRedo()
      }
    }
    document.addEventListener('keydown', handleGlobalKeyDown)
    return () => document.removeEventListener('keydown', handleGlobalKeyDown)
  }, [handleUndo, handleRedo])

  // ── Selection (multi-block) ───────────────────────────────────────
  // KBD-020: Alt+Up/Down — multi-select blocks (Logseq-style)
  const handleSelectBlock = useCallback(
    (blockId: string, direction: 'up' | 'down') => {
      if (!selectionAnchor) {
        setSelectionAnchor(blockId)
        setSelectedIds(new Set([blockId]))

        // Focus the adjacent block in the direction pressed
        const flat = flatBlocksRef.current
        const idx = flat.findIndex(fb => fb.block.id === blockId)
        if (direction === 'up' && idx > 0) {
          onFocusBlock(flat[idx - 1].block.id, 'end')
        } else if (direction === 'down' && idx < flat.length - 1) {
          onFocusBlock(flat[idx + 1].block.id, 'start')
        }
        return
      }

      const flat = flatBlocksRef.current
      const anchorIdx = flat.findIndex(fb => fb.block.id === selectionAnchor)
      const targetIdx = flat.findIndex(fb => fb.block.id === blockId)

      if (anchorIdx === -1 || targetIdx === -1) return

      const start = Math.min(anchorIdx, targetIdx)
      const end = Math.max(anchorIdx, targetIdx)
      const newSelection = new Set(flat.slice(start, end + 1).map(fb => fb.block.id))
      setSelectedIds(newSelection)

      // Focus adjacent block in direction of extension
      if (direction === 'up' && targetIdx > 0) {
        onFocusBlock(flat[targetIdx - 1].block.id, 'end')
      } else if (direction === 'down' && targetIdx < flat.length - 1) {
        onFocusBlock(flat[targetIdx + 1].block.id, 'start')
      }
    },
    [selectionAnchor, onFocusBlock],
  )

  const clearSelection = useCallback(() => {
    setSelectedIds(new Set())
    setSelectionAnchor(null)
  }, [])

  // ── Mod+A: Select parent (all siblings of current block) ──────────
  const handleSelectParent = useCallback(
    (blockId: string, parentId: string | null) => {
      const siblings = blocks.filter(b => b.parentId === parentId)
      setSelectedIds(new Set(siblings.map(b => b.id)))
      setSelectionAnchor(blockId)
    },
    [blocks],
  )

  // ── Mod+Shift+A: Select all blocks ────────────────────────────────
  const handleSelectAll = useCallback(() => {
    if (blocks.length === 0) return
    setSelectedIds(new Set(blocks.map(b => b.id)))
    setSelectionAnchor(blocks[0]?.id ?? null)
  }, [blocks])

  // ── Global keyboard: Backspace/Delete/Escape on selection ─────────
  useEffect(() => {
    function handleSelectionKeyDown(e: KeyboardEvent) {
      if (selectedIds.size === 0) return

      const target = e.target as HTMLElement
      // Never interfere with text editing
      if (target.contentEditable === 'true' || target.tagName === 'INPUT' || target.tagName === 'TEXTAREA') return

      // KBD-021: Backspace/Delete on multi-selection — delete all selected blocks
      if (e.key === 'Backspace' || e.key === 'Delete') {
        e.preventDefault()
        // Only delete blocks that still exist
        const ids = Array.from(selectedIds).filter(id =>
          blocksRef.current.some(b => b.id === id),
        )
        if (ids.length === 0) {
          clearSelection()
          return
        }
        // No `Delete` command in the Rust `OutlinerCommand` enum yet
        // — bulk delete is not undoable through the WASM history.
        Promise.all(ids.map(id => api.deleteBlock(id)))
          .then(() => {
            setBlocks(prev => prev.filter(b => !selectedIds.has(b.id)))
            clearSelection()
            toast.success(`${ids.length} block${ids.length > 1 ? 's' : ''} deleted`)
          })
          .catch(err => {
            if (err instanceof QuiltApiError && err.status === 409) {
              // The server refuses to orphan children. The user must
              // delete or re-parent the children of any selected block
              // before retrying. The error detail (from the server) is
              // already actionable, so surface it as-is.
              toast.error(
                err.detail ||
                  'Cannot delete: one or more selected blocks have children. Delete or re-parent them first.',
              )
            } else {
              toast.error('Failed to delete blocks')
            }
          })
        return
      }

      // Escape clears selection (KBD-022)
      if (e.key === 'Escape') {
        e.preventDefault()
        clearSelection()
        return
      }

      // Enter with selection focuses the first selected block for editing
      if (e.key === 'Enter') {
        e.preventDefault()
        const first = Array.from(selectedIds)[0]
        if (first) {
          onFocusBlock(first, 'end')
        }
        return
      }
    }

    document.addEventListener('keydown', handleSelectionKeyDown)
    return () => document.removeEventListener('keydown', handleSelectionKeyDown)
  }, [selectedIds, clearSelection, onFocusBlock])

  // ── Render ────────────────────────────────────────────────────
  if (loading) return <PageSkeleton />
  if (error) return <PageViewError message={error} />

  return (
    <div
      ref={containerRef}
      onClick={handleContainerClick}
      style={{ height: '100%', display: 'flex', flexDirection: 'column', minHeight: '200px' }}
    >
      {/* Journal date header */}
      {isJournal && <JournalDateHeader pageName={pageName} format={journalFormat} />}

      {/* Non-journal page title */}
      {!isJournal && (
        <h1
          style={{
            fontSize: '28px',
            fontWeight: 700,
            color: 'var(--color-text-primary)',
            marginBottom: 'var(--space-6)',
            lineHeight: 1.2,
          }}
        >
          {pageName}
        </h1>
      )}

      {/* Block list (virtualized) or empty state */}
      {blocks.length === 0 ? (
        <EmptyState isJournal={isJournal} onNewBlock={handleNewBlockAtEnd} />
      ) : (
        <div style={{ flex: 1, minHeight: 0 }}>
          <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
            <SortableContext items={sortableIds} strategy={verticalListSortingStrategy}>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 0 }}>
                {flatBlocks.map((flatBlock) => (
                  <SortableBlockRowFlat
                    key={flatBlock.block.id}
                    flatBlock={flatBlock}
                    allBlocks={blocks}
                    pageName={pageName}
                    collapsedIds={collapsedIds}
                    blockRefs={blockRefs}
                    onToggleCollapse={handleToggleCollapse}
                    onUpdate={handleBlockUpdate}
                    onCreateBlock={handleCreateBlock}
                    onDeleteBlock={handleDeleteBlock}
                    onFocusBlock={onFocusBlock}
                    onMoveBlockUp={handleMoveBlockUp}
                    onMoveBlockDown={handleMoveBlockDown}
                    onUndo={handleUndo}
                    onRedo={handleRedo}
                    selected={selectedIds.has(flatBlock.block.id)}
                    onMultiSelect={handleSelectBlock}
                    onSelectAll={handleSelectAll}
                    onSelectParent={handleSelectParent}
                    onAddComment={handleAddComment}
                    onResolveComment={handleResolveComment}
                    onReplyComment={handleReplyComment}
                    onDeleteComment={handleDeleteComment}
                  />
                ))}
              </div>
            </SortableContext>
          </DndContext>
        </div>
      )}

      {/* New block button at the bottom */}
      {blocks.length > 0 && (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 'var(--space-2)',
            padding: 'var(--space-2) var(--space-2)',
            paddingLeft: 'var(--space-2)',
            color: 'var(--color-text-muted)',
            cursor: 'pointer',
            borderRadius: 'var(--radius-sm)',
            transition: 'color var(--motion-fast) var(--ease-standard)',
          }}
          className="block-row"
          onClick={handleNewBlockAtEnd}
          role="button"
          tabIndex={0}
          aria-label="Add new block"
          onKeyDown={e => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault()
              handleNewBlockAtEnd()
            }
          }}
        >
          <Plus size={16} />
          <span style={{ fontSize: '13px' }}>Add a block</span>
        </div>
      )}
    </div>
  )
}

// ──── Retained exports for backward compatibility ─────────────────

/** @deprecated Use `flattenBlockTree` from `./flattenTree` instead */
export interface BlockTreeNode {
  block: Block
  children: BlockTreeNode[]
}

/** @deprecated Use `flattenBlockTree` from `./flattenTree` instead */
export function buildBlockTree(blocks: Block[]): BlockTreeNode[] {
  const childrenMap = new Map<string | null, Block[]>()

  for (const block of blocks) {
    const pid = block.parentId ?? null
    if (!childrenMap.has(pid)) childrenMap.set(pid, [])
    childrenMap.get(pid)!.push(block)
  }

  for (const [, group] of childrenMap) {
    group.sort((a, b) => a.order - b.order)
  }

  function buildChildren(parentId: string | null): BlockTreeNode[] {
    return (childrenMap.get(parentId) ?? []).map(block => ({
      block,
      children: buildChildren(block.id),
    }))
  }

  return buildChildren(null)
}
