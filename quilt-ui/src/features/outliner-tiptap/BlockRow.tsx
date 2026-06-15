import { lazy, Suspense, useState, useRef, useCallback, useEffect, useMemo, type KeyboardEvent } from 'react'
import { DatePicker } from '@shared/components/DatePicker'
import { GripVertical, Plus, MoreHorizontal, ChevronDown, ChevronRight, Settings2, MessageCircle } from 'lucide-react'
import { useNavigate } from '@tanstack/react-router'
import toast from 'react-hot-toast'
import { api } from '@core/api-client'
import { setCursorAt, getCursorPosition, isCursorAtStart, isCursorAtEnd } from '@shared/hooks/useCursor'
import { useTemplateCreation } from '@shared/hooks/useTemplateCreation'
import type { Block, BlockType, TaskMarker, Priority, Page } from '@shared/types/api'
import { useTabs } from '@shared/contexts/TabsContext'
import { InlineContent } from './InlineContent'
import { BlockAutocomplete } from '@features/search/BlockAutocomplete'
import { TagAutocomplete } from '@features/search/TagAutocomplete'
import { CommentRow } from '@features/comments/CommentRow'
import { BlockContextMenu } from './BlockContextMenu'
import { buildCommentTree } from '@shared/utils/blockProperties'
import { normalizePageName } from '@shared/utils/pageName'
import { resolveNaturalDatesInContent } from '@shared/utils/naturalDate'
import { blockKeyboardHandler } from './blockKeyboardHandler'
// Type-only import — keeps the type for handleSlashSelect without
// pulling the (large) SlashCommandMenu module into the eager bundle.
import type { SlashMenuItem } from './SlashCommandMenu'
// Registry + SlashContext. The registry is the single source of
// truth for slash actions — adding a new one is one `register()`
// call, not three file edits.
import { defaultRegistry, type SlashContext } from './slashRegistry'
// Inline property badges (roadmap #13) — small, render-time-only
// component, eagerly imported because it's on the hot path of every
// block row.
import { InlinePropertyBadges } from '@features/properties/InlinePropertyBadges'
// PropertyStrip — renders multiple in-block properties as a compact card
import { PropertyStrip, type PropertyRow } from '@features/properties/PropertyStrip'
import { TASK_MARKER_CYCLE } from './rendering/TaskRenderer'
import { ensureTaskShape } from './ensureTaskShape'

// Re-export findNearestLink for backward compatibility — BlockRow.test.tsx
// (and any external consumer) imports it from this module. The actual
// implementation lives next to the pure handler so the regex set stays
// in one place.
export { findNearestLink } from './blockKeyboardHandler'

// Road-map #26: use the WASM `StrategySelector` (with a JS-only
// fallback) to decide what kind of block we're rendering. The hook
// returns one of `"task" | "query" | "view" | "agent-run" | "default"`
// and is the single source of truth for role detection in BlockRow.
import { useBlockStrategy, type BlockStrategyName } from './useBlockStrategy'

// Heavy overlays that only render on user action. Pulling them in via
// React.lazy means the page bundle stays small even though every
// block can theoretically show them.
const PageAutocomplete = lazy(() =>
  import('@features/search/PageAutocomplete').then(m => ({ default: m.PageAutocomplete })),
)
const SlashCommandMenu = lazy(() =>
  import('./SlashCommandMenu').then(m => ({ default: m.SlashCommandMenu })),
)
const BlockPropertiesPanel = lazy(() =>
  import('@features/properties/BlockPropertiesPanel').then(m => ({ default: m.BlockPropertiesPanel })),
)
const SavedViewBlock = lazy(() =>
  import('@features/view/SavedViewBlock').then(m => ({ default: m.SavedViewBlock })),
)

interface BlockRowProps {
  block: Block
  allBlocks: Block[]
  pageName: string
  hasChildren: boolean
  isCollapsed: boolean
  onToggleCollapse: (blockId: string) => void
  onUpdate: (block: Block) => void
  onCreateBlock: (afterBlockId: string, content: string, parentId: string | null) => void
  onDeleteBlock: (blockId: string) => void
  onFocusBlock: (blockId: string, cursorPos: 'start' | 'end') => void
  onMoveBlockUp: (blockId: string) => void
  onMoveBlockDown: (blockId: string) => void
  onUndo: () => void
  onRedo: () => void
  indent: number
  dragHandleProps?: React.HTMLAttributes<HTMLDivElement> & { ref?: React.Ref<HTMLDivElement> }
  /**
   * Optional richer cut handler. When provided, `Cmd+X` on a block
   * calls this with a snapshot of the block (id, content, properties,
   * marker, priority, etc.) AND skips the default `onDeleteBlock` call.
   * The parent is then responsible for: (1) pushing an undoable
   * restore action, (2) deleting the block, and (3) updating local
   * state. When NOT provided we fall back to the legacy flow
   * (clipboard + `onDeleteBlock`) for backward compatibility with
   * existing tests.
   */
  onCutBlock?: (snapshot: Block) => void
  selected?: boolean
  onMultiSelect?: (blockId: string, direction: 'up' | 'down') => void
  onSelectAll?: () => void
  onSelectParent?: (blockId: string, parentId: string | null) => void
  /** Add a comment to this block. */
  onAddComment?: (blockId: string) => void
  /** Toggle a comment's `resolved` property. */
  onResolveComment?: (commentId: string) => void
  /** Reply to a comment (add a child comment). */
  onReplyComment?: (commentId: string) => void
  /** Delete a comment. */
  onDeleteComment?: (commentId: string) => void
}

// ──── Badge styles per DESIGN.md §9.6 ────────────────────────────────
// Pill radius, compact, label-md (12px, 600 weight)

const MARKER_STYLES: Record<TaskMarker, { bg: string; text: string }> = {
  Todo: { bg: 'var(--color-info)', text: '#fff' },
  Doing: { bg: 'var(--color-accent)', text: '#fff' },
  Done: { bg: 'var(--color-success)', text: '#fff' },
  Now: { bg: 'var(--color-danger)', text: '#fff' },
  Later: { bg: 'var(--color-warning)', text: '#fff' },
  Cancelled: { bg: 'var(--color-text-disabled)', text: '#fff' },
  // Waiting: purple - var(--color-warning-soft) is purple-ish if defined, fallback #9333ea
  Waiting: { bg: 'var(--color-warning-soft, #9333ea)', text: '#fff' },
}

const PRIORITY_STYLES: Record<Priority, { bg: string; text: string }> = {
  A: { bg: 'var(--color-danger)', text: '#fff' },
  B: { bg: 'var(--color-warning)', text: '#fff' },
  C: { bg: 'var(--color-text-muted)', text: '#fff' },
}

// ──── AgentRun block role (ADR-DRAFT-agent-run-block-role) ───────────
//
// A block with `type:: agent-run` is a regular Block (no schema change)
// whose `properties` array carries run metadata. The header strip
// surfaces agent name, run-status, and started-at so users can scan
// runs at a glance. The block content remains editable.

/** All run-status values per ADR lifecycle. */
export const AGENT_RUN_STATUSES = [
  'Queued',
  'Running',
  'Completed',
  'Failed',
  'Cancelled',
] as const
export type AgentRunStatus = (typeof AGENT_RUN_STATUSES)[number]

/** Per-status badge colours. Failed/Completed/Running are the three
 *  terminal/in-flight states and get the highest-contrast colours
 *  (danger / success / info). Queued and Cancelled stay muted. */
const AGENT_RUN_STATUS_STYLES: Record<AgentRunStatus, { bg: string; text: string }> = {
  Queued: { bg: 'var(--color-text-muted)', text: '#fff' },
  Running: { bg: 'var(--color-info)', text: '#fff' },
  Completed: { bg: 'var(--color-success)', text: '#fff' },
  Failed: { bg: 'var(--color-danger)', text: '#fff' },
  Cancelled: { bg: 'var(--color-text-disabled)', text: '#fff' },
}

/** Read a string property from a block, or null if absent. */
function readProperty(block: Block, key: string): string | null {
  const prop = block.properties?.find(p => p.key === key)
  if (!prop || prop.value == null) return null
  return String(prop.value)
}

// ──── Helpers ────────────────────────────────────────────────────────

/** Find the sibling immediately before this block (by order) */
function findPrevSibling(block: Block, allBlocks: Block[]): Block | null {
  const siblings = allBlocks
    .filter(b => b.id !== block.id && b.parentId === block.parentId)
    .sort((a, b) => a.order - b.order)

  const idx = siblings.findIndex(b => b.order >= block.order)
  if (idx > 0) return siblings[idx - 1]
  if (idx === -1 && siblings.length > 0) return siblings[siblings.length - 1]
  return null
}

/** Find the sibling immediately after this block (by order) */
function findNextSibling(block: Block, allBlocks: Block[]): Block | null {
  const siblings = allBlocks
    .filter(b => b.id !== block.id && b.parentId === block.parentId)
    .sort((a, b) => a.order - b.order)

  const idx = siblings.findIndex(b => b.order > block.order)
  return idx >= 0 ? siblings[idx] : null
}

/** Find the parent block */
function findParentBlock(block: Block, allBlocks: Block[]): Block | null {
  if (!block.parentId) return null
  return allBlocks.find(b => b.id === block.parentId) ?? null
}

// ──── Inline link search (Quilt parity) ──────────────────────────────
//
// `findNearestLink` lives in `./blockKeyboardHandler` next to the pure
// keyboard handler so the regex set stays in one place. We re-export
// it from the top of this file for backward compatibility with
// existing tests and any external consumer.

// ──── Component ──────────────────────────────────────────────────────

export function BlockRow({
  block,
  allBlocks,
  pageName,
  hasChildren,
  isCollapsed,
  onToggleCollapse,
  onUpdate,
  onCreateBlock,
  onDeleteBlock,
  onFocusBlock,
  onMoveBlockUp,
  onMoveBlockDown,
  onUndo,
  onRedo,
  indent,
  dragHandleProps,
  onCutBlock,
  selected,
  onMultiSelect,
  onSelectAll,
  onSelectParent,
  onAddComment,
  onResolveComment,
  onReplyComment,
  onDeleteComment,
}: BlockRowProps) {
  // ── State & Refs ──────────────────────────────────────────────
  const [isEditing, setIsEditing] = useState(false)
  const [localContent, setLocalContent] = useState(block.content)
  const [showProperties, setShowProperties] = useState(false)
  const [showContextMenu, setShowContextMenu] = useState(false)
  const [extractedProperties, setExtractedProperties] = useState<PropertyRow[]>([])
  const handlePropsExtracted = useCallback((props: PropertyRow[]) => {
    setExtractedProperties(props)
  }, [])
  const contextMenuAnchorRef = useRef<HTMLButtonElement>(null)
  const [pageMap, setPageMap] = useState<Map<string, Page>>(new Map())
  const [autocomplete, setAutocomplete] = useState<{
    query: string
    position: { top: number; left: number }
  } | null>(null)
  const [blockAutocomplete, setBlockAutocomplete] = useState<{
    query: string
    position: { top: number; left: number }
  } | null>(null)
  const [tagAutocomplete, setTagAutocomplete] = useState<{
    query: string
    position: { top: number; left: number }
  } | null>(null)
  const [slashCommand, setSlashCommand] = useState<{
    query: string
    position: { top: number; left: number }
  } | null>(null)
  /** The date picker field ('deadline' | 'scheduled') when open, null when closed. */
  const [datePickerField, setDatePickerField] = useState<'deadline' | 'scheduled' | null>(null)
  /** Ref to the trigger element for the date picker popover positioning. */
  const datePickerTriggerRef = useRef<HTMLDivElement>(null)
  const contentRef = useRef<HTMLDivElement>(null)
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const rowRef = useRef<HTMLDivElement>(null)
  // Holds the latest `handleInsertTemplate` so `handleSlashSelect` can
  // dispatch into it without a declaration-order coupling (the
  // useCallback lives further down the file).
  const templateInsertRef = useRef<(originalContent: string) => Promise<void> | void>(() => {})

  const { openTab } = useTabs()
  const navigate = useNavigate()

  // Sync content back to raw content when exiting edit mode
  useEffect(() => {
    if (!isEditing) {
      setLocalContent(block.content)
      if (contentRef.current) {
        contentRef.current.textContent = block.content
      }
    }
  }, [block.content, isEditing])

  // Cleanup timer on unmount
  useEffect(() => {
    return () => {
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current)
    }
  }, [])

  // Fetch pages for page-ref existence check and navigation lookup
  useEffect(() => {
    api.listPages().then(pages => {
      const map = new Map<string, Page>()
      pages.forEach(p => {
        // The server already returns canonical (lowercase) names, but
        // we normalise again so any drift (e.g. a test fixture, a
        // pre-existing DB row, or a future server change) doesn't
        // silently break the wikilink existence check.
        map.set(normalizePageName(p.name), p)
      })
      setPageMap(map)
    }).catch(() => {
      // Non-critical; refs gracefully fall back
    })
  }, [])

  // ── Content save ──────────────────────────────────────────────
  const saveToApi = useCallback(
    async (text: string) => {
      // Preserve the exact text the user typed. Quilt-style editors do not
      // silently trim leading/trailing whitespace on blur.
      if (text === block.content) return
      // NL Dates V1: rewrite natural-date tokens in date-property
      // values (`deadline:: today`, `scheduled:: tomorrow`, …) to real
      // ISO YYYY-MM-DD strings before persisting. The backend stays
      // date-agnostic; the UI owns the "friendly" → "real" transform.
      const resolved = resolveNaturalDatesInContent(text)
      if (resolved === block.content) return
      try {
        const updated = await api.updateBlock(block.id, { content: resolved })
        onUpdate(updated)
      } catch (err) {
        toast.error('Failed to save block')
        // Revert in case the contentEditable is still mounted
        if (contentRef.current) {
          contentRef.current.textContent = block.content
        }
      }
    },
    [block.id, block.content, onUpdate],
  )

  const debouncedSave = useCallback(
    (text: string) => {
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current)
      saveTimerRef.current = setTimeout(() => saveToApi(text), 1000)
    },
    [saveToApi],
  )

  // ── Input handling ────────────────────────────────────────────
  const handleInput = useCallback((e: React.FormEvent<HTMLDivElement>) => {
    const text = (e.target as HTMLDivElement).textContent ?? ''
    setLocalContent(text)
    debouncedSave(text)

    const sel = window.getSelection()

    // ── [[ autocomplete ─────────────────────────────────────────
    if (sel && sel.rangeCount > 0 && contentRef.current) {
      const range = sel.getRangeAt(0)
      const textBefore = text.substring(0, getCursorPosition(contentRef.current))
      const match = textBefore.match(/\[\[([^\]]*?)$/)
      if (match) {
        const rect = range.getBoundingClientRect()
        setAutocomplete({
          query: match[1],
          position: { top: rect.bottom + 4, left: rect.left },
        })
      } else {
        setAutocomplete(null)
      }
    }

    // ── (( block autocomplete ─────────────────────────────────
    if (sel && sel.rangeCount > 0 && contentRef.current) {
      const range = sel.getRangeAt(0)
      const textBefore = text.substring(0, getCursorPosition(contentRef.current))
      const blockMatch = textBefore.match(/\(\(([^\)]*?)$/)
      if (blockMatch) {
        const rect = range.getBoundingClientRect()
        setBlockAutocomplete({
          query: blockMatch[1],
          position: { top: rect.bottom + 4, left: rect.left },
        })
      } else {
        setBlockAutocomplete(null)
      }
    }

    // ── # tag autocomplete ──────────────────────────────────────
    // Only fire when `#` is at the start of a word (start of line, or
    // preceded by whitespace). The negative lookbehind for `\S` keeps
    // markdown like `## Heading` and code with `foo#bar` from opening
    // the dropdown.
    if (sel && sel.rangeCount > 0 && contentRef.current) {
      const range = sel.getRangeAt(0)
      const textBefore = text.substring(0, getCursorPosition(contentRef.current))
      const tagMatch = textBefore.match(/(?<!\S)#(\w*)$/)
      if (tagMatch) {
        const rect = range.getBoundingClientRect()
        setTagAutocomplete({
          query: tagMatch[1],
          position: { top: rect.bottom + 4, left: rect.left },
        })
      } else {
        setTagAutocomplete(null)
      }
    }

    // ── Slash command: detect "/" at start of block ─────────────
    if (text.startsWith('/')) {
      if (sel && sel.rangeCount > 0) {
        const range = sel.getRangeAt(0)
        const rect = range.getBoundingClientRect()
        setSlashCommand({
          query: text.slice(1),
          position: { top: rect.bottom + 4, left: rect.left },
        })
      }
    } else {
      setSlashCommand(null)
    }
  }, [debouncedSave])

  const handleStartEdit = useCallback(() => {
    setIsEditing(true)
    // Populate and focus the contentEditable after React renders it
    requestAnimationFrame(() => {
      if (contentRef.current) {
        contentRef.current.textContent = block.content
        contentRef.current.focus()
      }
    })
  }, [block.content])

  const handleBlur = useCallback(() => {
    const text = contentRef.current?.textContent ?? ''
    setIsEditing(false)
    setAutocomplete(null)
    setBlockAutocomplete(null)
    setTagAutocomplete(null)
    setSlashCommand(null)
    // Flush save immediately on blur
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current)
    saveToApi(text)
  }, [saveToApi])

  // ── Autocomplete select ───────────────────────────────────────
  const handleAutocompleteSelect = useCallback(
    (pageName: string) => {
      const newContent = localContent.replace(/\[\[[^\]]*$/, `[[${pageName}]]`)
      setLocalContent(newContent)
      if (contentRef.current) contentRef.current.textContent = newContent
      setAutocomplete(null)
      debouncedSave(newContent)
    },
    [localContent, debouncedSave],
  )

  // ── Block autocomplete select ────────────────────────────────
  const handleBlockAutocompleteSelect = useCallback(
    (blockId: string) => {
      const newContent = localContent.replace(/\(\([^\)]*$/, `((${blockId}))`)
      setLocalContent(newContent)
      if (contentRef.current) contentRef.current.textContent = newContent
      setBlockAutocomplete(null)
      debouncedSave(newContent)
    },
    [localContent, debouncedSave],
  )

  // ── Tag autocomplete select ──────────────────────────────────
  // Replaces the partial `#partial` with `#tagname` (no closing
  // delimiter — Quilt tags are atomic and do not need a closing char).
  const handleTagAutocompleteSelect = useCallback(
    (tagName: string) => {
      const newContent = localContent.replace(/(?<!\S)#\w*$/, `#${tagName}`)
      setLocalContent(newContent)
      if (contentRef.current) contentRef.current.textContent = newContent
      setTagAutocomplete(null)
      debouncedSave(newContent)
    },
    [localContent, debouncedSave],
  )

  // ── Slash command select ──────────────────────────────────────
  // Replaces the legacy 100-line `switch (prefix)` on `item.action.split(':')`
  // with a single dispatch through the default registry. The action
  // handlers are pure SlashContext consumers; the React layer adapts
  // local state + DOM refs + debounced-save into a plain object.
  //
  // The `template:insert` flow needs to keep the original block
  // content (so cancel can restore it). We snapshot here, clear
  // upfront for every other action, and forward the snapshot to
  // the handler via `ctx.originalContent`.
  const handleSlashSelect = useCallback(
    (item: SlashMenuItem) => {
      setSlashCommand(null)

      const originalContent = localContent
      // `id === 'insert-template'` is the only action that preserves
      // the leading "/" so the user can keep editing. Everything
      // else replaces the content — but we defer the clear until
      // we know the handler succeeded (restore on fail).
      const preserveContent = item.id === 'insert-template'

      if (!preserveContent) {
        // Clear the "/" text from the block (only for mutating actions)
        const newContent = ''
        setLocalContent(newContent)
        if (contentRef.current) contentRef.current.textContent = newContent
      }

      // Adapters that lift the slash-handler surface into the
      // React component's local state + DOM ref + debounced save.
      // Handlers stay React-free; the component owns the side effects.
      const setContent = (text: string) => {
        setLocalContent(text)
        if (contentRef.current) contentRef.current.textContent = text
        debouncedSave(text)
      }
      const setContentAtEnd = (text: string) => {
        setLocalContent(text)
        if (contentRef.current) contentRef.current.textContent = text
        // Place cursor after the inserted text on the next frame so
        // the DOM has time to settle.
        setTimeout(() => {
          if (contentRef.current) setCursorAt(contentRef.current, 'end')
        }, 0)
        debouncedSave(text)
      }

      const ctx: SlashContext = {
        block,
        allBlocks,
        api,
        toast,
        navigate,
        setContent,
        setContentAtEnd,
        onUpdate,
        originalContent,
        onAddComment,
        // Read from the ref so the slash dispatcher doesn't need to
        // know where `handleInsertTemplate` is declared.
        templateInsert: templateInsertRef.current,
        openDatePicker: (field) => {
          setDatePickerField(field)
        },
      }

      const handler = defaultRegistry.getHandler(item.id)
      if (handler) {
        // For status/priority handlers that call api.updateBlock:
        // if the API call fails, the handler catches and shows toast.
        // But the "/" is already cleared and the content is "".
        // We need to restore the content on failure.
        //
        // For date/property/ref handlers: they call setContent/setContentAtEnd
        // which replaces the content. No restore needed.
        //
        // For template handler: preserveContent is true, so no clear happened.
        //
        // The safe pattern: try the handler, restore on failure.
        Promise.resolve(handler(ctx, item)).catch(() => {
          // Handler failed — restore the original content
          if (!preserveContent) {
            setLocalContent(originalContent)
            if (contentRef.current) contentRef.current.textContent = originalContent
          }
        })
      }
    },
    [
      block,
      allBlocks,
      onUpdate,
      debouncedSave,
      navigate,
      localContent,
      onAddComment,
    ],
  )

  /** Handles date selection from the DatePicker popover.
   *  Writes both the inline property text and the structured API field.
   *  Empty string isoDate means the user clicked "Clear". */
  const handleDatePickerSelect = useCallback(
    (isoDate: string) => {
      const field = datePickerField
      if (!field) return
      // Clear: remove the inline property and null the API field
      if (!isoDate) {
        setLocalContent('')
        if (contentRef.current) contentRef.current.textContent = ''
        debouncedSave('')
        api
          .updateBlock(block.id, { [field]: null })
          .then(updated => {
            onUpdate(updated)
          })
          .catch(() => {
            toast.error(`Failed to clear ${field}`)
          })
        setDatePickerField(null)
        return
      }
      // Build the ISO-8601 full timestamp (start of day in UTC) for the API
      const isoTimestamp = `${isoDate}T00:00:00Z`
      // Write inline property: "deadline:: YYYY-MM-DD"
      const propStr = `${field}:: ${isoDate}`
      setLocalContent(propStr)
      if (contentRef.current) contentRef.current.textContent = propStr
      debouncedSave(propStr)
      // Write structured API field
      api
        .updateBlock(block.id, { [field]: isoTimestamp })
        .then(updated => {
          onUpdate(updated)
        })
        .catch(() => {
          toast.error(`Failed to set ${field}`)
        })
      setDatePickerField(null)
    },
    [datePickerField, block.id, debouncedSave, onUpdate, api, toast],
  )

  // ── Keyboard handling ─────────────────────────────────────────
  //
  // This is the ADAPTER layer. The decision logic — "what should
  // happen when the user presses key X with mods Y at cursor Z" —
  // lives in `blockKeyboardHandler.ts` as a pure function. Here we:
  //   1. Intercept menu-driven keys (slash / autocomplete menus own
  //      their own Escape / Enter / Arrow keys)
  //   2. Gather DOM facts (cursor position, text selection)
  //   3. Delegate to the pure handler
  //   4. Switch on the returned action to apply DOM / state changes
  //
  // The split keeps React-state and DOM manipulation in this file
  // while moving the testable rules to a pure module.
  const handleKeyDown = useCallback(
    (e: KeyboardEvent<HTMLDivElement>) => {
      const el = contentRef.current
      if (!el) return

      // ── Inline formatting helper (DOM-coupled — stays local) ──
      const toggleInlineMark = (marker: string) => {
        const sel = window.getSelection()
        if (!sel || sel.rangeCount === 0) return

        const range = sel.getRangeAt(0)
        const text = el.textContent || ''

        // Selection exists: wrap or unwrap
        if (!range.collapsed) {
          const selectedText = range.toString()
          const start = getCursorPosition(el, 'start')
          const end = getCursorPosition(el, 'end')

          // Check if already wrapped with the same marker
          if (
            text.substring(start, start + marker.length) === marker &&
            text.substring(end - marker.length, end) === marker
          ) {
            // Remove markers (un-toggle)
            const newText = text.substring(0, start) + selectedText + text.substring(end)
            setLocalContent(newText)
            el.textContent = newText
            saveToApi(newText)
          } else {
            // Add markers around selection
            const newText = text.substring(0, start) + marker + selectedText + marker + text.substring(end)
            setLocalContent(newText)
            el.textContent = newText
            saveToApi(newText)
          }
          return
        }

        // No selection: insert markers at cursor, user types between them
        const pos = getCursorPosition(el)
        const newText = text.substring(0, pos) + marker + marker + text.substring(pos)
        setLocalContent(newText)
        el.textContent = newText
        // Place cursor between markers (after textContent reset, el has one text node)
        const textNode = el.firstChild
        if (textNode) {
          const range = document.createRange()
          range.setStart(textNode, pos + marker.length)
          range.collapse(true)
          sel.removeAllRanges()
          sel.addRange(range)
        }
        saveToApi(newText)
      }

      // ── Menu interception: when any autocomplete / slash menu is
      //    open, those menus own Escape / Enter / ArrowUp / ArrowDown.
      //    We just consume the events so the editor doesn't double-handle.
      const menuOpen = slashCommand ?? blockAutocomplete ?? tagAutocomplete ?? autocomplete
      if (menuOpen) {
        if (e.key === 'Escape') {
          e.preventDefault()
          if (slashCommand) setSlashCommand(null)
          if (blockAutocomplete) setBlockAutocomplete(null)
          if (tagAutocomplete) setTagAutocomplete(null)
          if (autocomplete) setAutocomplete(null)
          return
        }
        if (['Enter', 'ArrowUp', 'ArrowDown'].includes(e.key)) {
          e.preventDefault()
          return
        }
        return // Let other keys through (typing continues to filter)
      }

      // ── Gather DOM facts (the only DOM access in this branch) ──
      const sel = window.getSelection()
      const hasTextSelection = !!(sel && sel.rangeCount > 0 && !sel.getRangeAt(0).collapsed)
      const currentText = localContent || el.textContent || ''

      // ── Pure decision ──
      const action = blockKeyboardHandler({
        block,
        allBlocks,
        content: currentText,
        key: e.key,
        mods: {
          mod: e.ctrlKey || e.metaKey,
          shift: e.shiftKey,
          alt: e.altKey,
        },
        cursor: {
          offset: getCursorPosition(el),
          atStart: isCursorAtStart(el),
          atEnd: isCursorAtEnd(el),
        },
        hasTextSelection,
      })

      // ── Apply action ──
      switch (action.type) {
        case 'None':
          return

        case 'Blur':
          e.preventDefault()
          el.blur()
          return

        case 'Undo':
          e.preventDefault()
          onUndo()
          return

        case 'Redo':
          e.preventDefault()
          onRedo()
          return

        case 'ToggleInlineMark':
          e.preventDefault()
          toggleInlineMark(action.marker)
          return

        case 'SelectParent':
          e.preventDefault()
          onSelectParent?.(block.id, block.parentId)
          return

        case 'SelectAll':
          e.preventDefault()
          onSelectAll?.()
          return

        case 'CopyBlock':
          e.preventDefault()
          navigator.clipboard.writeText(currentText).catch(() => {})
          toast.success('Block copied')
          return

        case 'CutBlock':
          e.preventDefault()
          navigator.clipboard.writeText(currentText).catch(() => {})
          if (onCutBlock) {
            // Snapshot includes the latest *un-saved* text (e.g. user
            // typed but hasn't blurred yet). The parent owns both the
            // undo push and the delete — keeping the side effects
            // atomic at the call site prevents a half-cut state
            // (block gone from state but no undo entry) if the
            // delete API call fails.
            onCutBlock({ ...block, content: currentText })
          } else {
            // Legacy path: clipboard + delete, no undo support.
            onDeleteBlock(block.id)
          }
          toast.success('Block cut')
          return

        case 'PasteAsNewBlock':
          e.preventDefault()
          navigator.clipboard.readText().then(clipText => {
            if (clipText) onCreateBlock(block.id, clipText, block.parentId)
          }).catch(() => {})
          return

        case 'InsertText': {
          e.preventDefault()
          if (!sel || sel.rangeCount === 0) return
          const range = sel.getRangeAt(0)
          const newNode = document.createTextNode(action.text)
          range.deleteContents()
          range.insertNode(newNode)
          range.setStartAfter(newNode)
          range.setEndAfter(newNode)
          sel.removeAllRanges()
          sel.addRange(range)
          setTimeout(() => {
            const text = el.textContent || ''
            setLocalContent(text)
            debouncedSave(text)
          }, 0)
          return
        }

        case 'ToggleDone': {
          e.preventDefault()
          // Use the 7-step cycle from TaskRenderer: null → todo → waiting → doing → done → later → cancelled → null
          const currentIdx = TASK_MARKER_CYCLE.indexOf(block.marker)
          const nextIdx = currentIdx >= 0 ? (currentIdx + 1) % TASK_MARKER_CYCLE.length : 0
          const nextMarker = TASK_MARKER_CYCLE[nextIdx]
          // ADR-0023 deviation (ADR-0025): one-way paragraph/bullet/numbered/heading → todo
          // conversion when cycling to a non-null marker. Clearing marker does NOT revert blockType.
          const shape = ensureTaskShape(block, nextMarker)
          onUpdate({ ...block, marker: nextMarker })
          api.updateBlock(block.id, shape).catch(() => {
            toast.error('Failed to cycle marker')
          })
          return
        }

        case 'FollowLink': {
          e.preventDefault()
          // Save current edits first so navigation doesn't lose them
          saveToApi(currentText)
          // Narrow out `null` explicitly — the discriminated union
          // includes null, but in this branch the pure handler has
          // already verified the link is non-null. Re-bind to a
          // non-nullable variable so TypeScript narrows correctly.
          const link = action.link!
          if (link.type === 'page' || link.type === 'tag') {
            // Tags are pages too (Quilt parity). The server stores
            // page names in canonical form (lowercase + trimmed), so
            // we normalise before the existence check, the createPage
            // call, and the navigate URL — otherwise a user-typed
            // `[[My Notes]]` creates `mynotes` on the server but
            // navigates to `/page/My Notes` which 404s.
            const canonical = normalizePageName(link.target)
            if (!canonical) return
            const exists = pageMap.has(canonical)
            if (!exists) {
              api.createPage({ name: canonical }).catch(() => {
                // Race / network blip — navigate anyway, the page exists
                // server-side.
              })
            }
            navigate({ to: '/page/$name', params: { name: canonical } })
          } else {
            // block-ref — resolve UUID to its page and navigate. The
            // page name returned by the server is already canonical, so
            // we just normalise defensively in case a stale cache
            // surfaces an old mixed-case name.
            const refBlock = allBlocks.find(b => b.id === link.target)
            const targetPage = refBlock?.pageName || (refBlock?.pageId ?? null)
            if (targetPage) {
              navigate({ to: '/page/$name', params: { name: normalizePageName(targetPage) } })
            }
          }
          return
        }

        case 'CreateEmptySibling':
          e.preventDefault()
          onCreateBlock(block.id, '', block.parentId)
          return

        case 'Split': {
          e.preventDefault()
          const fullText = el.textContent ?? ''
          const beforeCursor = fullText.slice(0, action.at)
          const afterCursor = fullText.slice(action.at)
          el.textContent = beforeCursor
          setLocalContent(beforeCursor)
          if (saveTimerRef.current) clearTimeout(saveTimerRef.current)
          saveToApi(beforeCursor)
          setCursorAt(el, 'end')
          onCreateBlock(block.id, afterCursor, block.parentId)
          return
        }

        case 'MergeWithPrev': {
          e.preventDefault()
          const prev = findPrevSibling(block, allBlocks)
          if (!prev) {
            if (block.parentId) handleOutdent()
            return
          }
          const mergedContent = prev.content + currentText
          api.updateBlock(prev.id, { content: mergedContent }).then(updated => onUpdate(updated))
          api.deleteBlock(block.id).then(() => onDeleteBlock(block.id))
          onFocusBlock(prev.id, 'end')
          onUpdate({ ...prev, content: mergedContent })
          return
        }

        case 'Outdent':
          e.preventDefault()
          handleOutdent()
          return

        case 'Indent':
          e.preventDefault()
          handleIndent()
          return

        case 'MoveCursor': {
          e.preventDefault()
          if (action.to === 'prev') {
            const prev = findPrevSibling(block, allBlocks)
            if (prev) onFocusBlock(prev.id, 'end')
          } else if (action.to === 'next') {
            const next = findNextSibling(block, allBlocks)
            if (next) onFocusBlock(next.id, 'start')
          }
          // 'up' / 'down' reserved for future use
          return
        }

        case 'ExtendSelection':
          e.preventDefault()
          onMultiSelect?.(block.id, action.direction)
          return

        case 'MoveBlockUp':
          e.preventDefault()
          onMoveBlockUp(block.id)
          return

        case 'MoveBlockDown':
          e.preventDefault()
          onMoveBlockDown(block.id)
          return

        // Reserved actions — no keyboard binding in the editor yet.
        case 'SetPriority':
        case 'SetMarker':
        case 'MergeWithNext':
          return
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [block, allBlocks, autocomplete, blockAutocomplete, tagAutocomplete, slashCommand, saveToApi, debouncedSave, onUpdate, onCreateBlock, onDeleteBlock, onFocusBlock, onMoveBlockUp, onMoveBlockDown, onMultiSelect, onSelectAll, onSelectParent, onUndo, onRedo, navigate, pageMap, localContent],
  )

  // ── Indent / Outdent ──────────────────────────────────────────
  const handleIndent = useCallback(() => {
    const prev = findPrevSibling(block, allBlocks)
    if (!prev) return // nothing to indent under

    const newParentId = prev.id
    const newLevel = prev.level + 1
    // Place at the end of the new parent's children
    const childrenOfPrev = allBlocks.filter(b => b.parentId === prev.id)
    const newOrder = childrenOfPrev.length > 0
      ? Math.max(...childrenOfPrev.map(b => b.order)) + 1
      : prev.order + 0.5

    const updated: Block = { ...block, parentId: newParentId, level: newLevel, order: newOrder }
    onUpdate(updated)

    api.updateBlock(block.id, { parentId: newParentId, level: newLevel, order: newOrder })
      .catch(() => {
        toast.error('Failed to indent block')
        // Revert
        onUpdate(block)
      })
  }, [block, allBlocks, onUpdate])

  const handleOutdent = useCallback(() => {
    if (!block.parentId) return // already top-level

    const parent = findParentBlock(block, allBlocks)
    if (!parent) return

    const newParentId = parent.parentId // may be null (becomes top-level)
    const newLevel = Math.max(0, (parent.level ?? 0))

    // Calculate new order: right after parent
    // (sibling order of parent if exists, or parent.order + 1)
    const newOrder = parent.order + 1

    const updated: Block = { ...block, parentId: newParentId, level: newLevel, order: newOrder }
    onUpdate(updated)

    // Server PATCH treats JSON `null` for parentId as a no-op (PATCH semantics:
    // absent/null = don't modify). Empty string `""` is the wire signal to
    // clear the parent. See `crates/quilt-server/src/handlers/blocks.rs:539`.
    api.updateBlock(block.id, {
      parentId: newParentId ?? '',
      level: newLevel,
      order: newOrder,
    })
      .catch(() => {
        toast.error('Failed to outdent block')
        onUpdate(block)
      })
  }, [block, allBlocks, onUpdate])

  // ── Click on bullet: collapse toggle ──
  const handleBulletClick = useCallback(() => {
    if (hasChildren) {
      onToggleCollapse(block.id)
    }
  }, [hasChildren, block.id, onToggleCollapse])

  // ── Add child block ──
  const handleAddChild = useCallback(() => {
    onCreateBlock(block.id, '', block.id)
  }, [block.id, onCreateBlock])

  // ── Block context menu actions (DESIGN.md §11.3) ──────────────
  // Convert this block to a TODO task: sets blockType to 'todo' and
  // marker to 'Todo', then persists. No-op if it's already a todo.
  const handleConvertToTask = useCallback(() => {
    if (block.blockType === 'todo') return
    const next: Block = { ...block, blockType: 'todo' as BlockType, marker: 'Todo' as TaskMarker }
    onUpdate(next)
    api.updateBlock(block.id, { blockType: 'todo', marker: 'Todo' })
      .catch(() => toast.error('Failed to convert to task'))
  }, [block, onUpdate])

  // Copy a deep link to this block to the clipboard.
  // Format: <origin>/page/<page-name>?block=<id>
  // When hash routing is added, the id can move to a #fragment.
  const handleCopyBlockLink = useCallback(() => {
    const url = `${window.location.origin}/page/${encodeURIComponent(pageName)}?block=${encodeURIComponent(block.id)}`
    navigator.clipboard
      .writeText(url)
      .then(() => toast.success('Block link copied'))
      .catch(() => toast.error('Failed to copy link'))
  }, [block.id, pageName])

  // ── Insert Template (ADR-0003) ────────────────────────────────────
  //
  // The slash command's "Insert Template" action invokes this. The
  // multi-step wizard (list templates → prompt for choice → call API →
  // navigate) is owned by `useTemplateCreation` (architecture review
  // candidate #5). This wrapper just (a) asks the user for the new
  // page's name and (b) hands the hook a restore callback so it can
  // put back the original block content (including the leading "/")
  // on every cancel / error path.
  const { createFromTemplate } = useTemplateCreation({
    onRestore: (originalContent: string) => {
      setLocalContent(originalContent)
      if (contentRef.current) contentRef.current.textContent = originalContent
    },
  })

  const handleInsertTemplate = useCallback(
    async (originalContent: string) => {
      const newPageName = window.prompt('New page name:')
      if (!newPageName || !newPageName.trim()) {
        setLocalContent(originalContent)
        if (contentRef.current) contentRef.current.textContent = originalContent
        return
      }
      await createFromTemplate(newPageName, originalContent)
    },
    [createFromTemplate],
  )

  // Keep `templateInsertRef` pointed at the latest `handleInsertTemplate`
  // closure so the slash dispatcher (which can't reference this
  // `useCallback` due to declaration order) can call it.
  useEffect(() => {
    templateInsertRef.current = handleInsertTemplate
  }, [handleInsertTemplate])

  // ── Render ────────────────────────────────────────────────────
  const stripeLines = indent > 0 && (
    <div
      style={{
        position: 'absolute',
        left: 0,
        top: 0,
        bottom: 0,
        pointerEvents: 'none',
      }}
    >
      {Array.from({ length: indent }).map((_, i) => (
        <div
          key={i}
          style={{
            position: 'absolute',
            left: `${i * 24 + 11}px`, // center of bullet at each level
            top: 0,
            bottom: 0,
            width: '1px',
            background: 'var(--color-border)',
          }}
        />
      ))}
    </div>
  )

  // Comments live as child blocks of this block. Build a tree so we can
  // render replies nested under their parent comment.
  const commentTree = onResolveComment
    ? buildCommentTree(allBlocks, block.id)
    : []

  // ADR-0003: surface the `created_by` convention as a small badge so
  // users can tell human vs agent authorship at a glance. Convention:
  //   user::<name>  → human author (👤)
  //   agent::<name> → AI author (🤖)
  // Any other value is shown as-is so we don't hide unknown authors.
  const createdBy = block.properties?.find(p => p.key === 'created_by')?.value
  const createdByStr = createdBy == null ? '' : String(createdBy)
  const isAgentAuthor = createdByStr.startsWith('agent::')

  // Road-map #26: the WASM `StrategySelector` (with a JS-only
  // fallback when the engine is not loaded) is the single source of
  // truth for "what kind of block is this?". We then derive the
  // localised flags (`isView`, `isAgentRun`, ...) from the strategy
  // name — keeping the rest of the render path identical to the
  // pre-hook behaviour. The strategy is also surfaced via
  // `data-strategy` on the row for tests / debugging.
  const strategy: BlockStrategyName = useBlockStrategy(block)
  const isAgentRun = strategy === 'agent-run'
  const isView = strategy === 'view'
  const agentName = isAgentRun ? readProperty(block, 'agent') : null
  const agentModel = isAgentRun ? readProperty(block, 'model') : null
  const runStatusRaw = isAgentRun ? readProperty(block, 'run-status') : null
  const runStatus: AgentRunStatus | null =
    runStatusRaw && (AGENT_RUN_STATUSES as readonly string[]).includes(runStatusRaw)
      ? (runStatusRaw as AgentRunStatus)
      : null
  const startedAt = isAgentRun ? readProperty(block, 'started-at') : null
  const runError = isAgentRun ? readProperty(block, 'error') : null

  const isDimmed = block.marker === 'Done' || block.marker === 'Cancelled'

  return (
    <div
      ref={rowRef}
      className="block-row"
      data-testid={`block-row-${block.id}`}
      // Strategy surface for tests / debugging — the WASM
      // `StrategySelector` (or its JS fallback) decides what kind of
      // block this is, and the row reflects the verdict on the DOM.
      data-strategy={strategy}
      style={{
        display: 'flex',
        alignItems: 'flex-start',
        gap: '10px',
        padding: '6px 10px',
        paddingLeft: `calc(10px + ${indent * 24}px)`,
        borderRadius: '10px',
        transition: 'background var(--motion-fast) var(--ease-standard), border-left var(--motion-fast) var(--ease-standard)',
        position: 'relative',
        background: selected
          ? 'var(--color-accent-subtle)'
          : isEditing
            ? 'var(--color-surface-subtle)'
            : undefined,
        borderLeft: selected
          ? '2px solid var(--color-accent)'
          : isEditing
            ? '2px solid var(--color-primary)'
            : '2px solid transparent',
      }}
    >
      {/* Indent stripe lines */}
      {stripeLines}

      {/* Drag handle — visible on row hover.
          Remove hardcoded marginTop — use the container's row-height to
          vertically centre so it stays aligned with any blockType
          (paragraph, heading, code, quote). */}
      <div
        {...dragHandleProps}
        className="drag-handle"
        style={{
          cursor: 'grab',
          color: 'var(--color-text-disabled)',
          opacity: 0,
          transition: 'opacity var(--motion-fast)',
          padding: '0 2px',
          display: 'flex',
          alignItems: 'center',
          touchAction: 'none',
          alignSelf: 'stretch',
          justifyContent: 'center',
        }}
      >
        <GripVertical size={14} />
      </div>

      {/* Bullet / collapse toggle — chevron if has children, dot otherwise.
          Replace hardcoded marginTop with alignSelf + minHeight so the
          icon stays vertically centred regardless of heading size. */}
      <button
        onClick={handleBulletClick}
        aria-label={hasChildren ? (isCollapsed ? 'Expand block' : 'Collapse block') : 'Bullet'}
        title={hasChildren ? (isCollapsed ? 'Expand block' : 'Collapse block') : undefined}
        style={{
          alignSelf: 'center',
          flexShrink: 0,
          background: 'none',
          border: 'none',
          padding: 0,
          cursor: hasChildren ? 'pointer' : 'default',
          color: 'var(--color-border-strong)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          transition: 'color var(--motion-fast) var(--ease-standard)',
        }}
        className="block-bullet"
        onMouseEnter={e => {
          (e.currentTarget as HTMLButtonElement).style.color = 'var(--color-primary)'
        }}
        onMouseLeave={e => {
          (e.currentTarget as HTMLButtonElement).style.color = 'var(--color-border-strong)'
        }}
      >
        {hasChildren ? (
          isCollapsed ? <ChevronRight size={14} /> : <ChevronDown size={14} />
        ) : (
          <div
            style={{
              width: '8px',
              height: '8px',
              borderRadius: 'var(--radius-pill)',
              background: 'currentColor',
              opacity: 0.65,
            }}
          />
        )}
      </button>

      {/* Marker badge — pill shape per DESIGN.md §9.6.
          Use alignSelf:center so the badge stays vertically aligned
          when the row contains a heading with larger font-size. */}
      {block.marker && (
        <span
          style={{
            flexShrink: 0,
            alignSelf: 'center',
            fontSize: '11px',
            fontWeight: 600,
            padding: '2px 8px',
            borderRadius: 'var(--radius-pill)',
            background: MARKER_STYLES[block.marker].bg,
            color: MARKER_STYLES[block.marker].text,
            lineHeight: 1.4,
            letterSpacing: '0.01em',
          }}
        >
          {block.marker.toUpperCase()}
        </span>
      )}

      {/* Priority badge */}
      {block.priority && (
        <span
          style={{
            flexShrink: 0,
            alignSelf: 'center',
            fontSize: '11px',
            fontWeight: 600,
            padding: '2px 8px',
            borderRadius: 'var(--radius-pill)',
            background: PRIORITY_STYLES[block.priority].bg,
            color: PRIORITY_STYLES[block.priority].text,
            lineHeight: 1.4,
          }}
        >
          {block.priority}
        </span>
      )}

      {/* AgentRun header (ADR-DRAFT-agent-run-block-role) — only when
          the block carries `type:: agent-run`. The header is a small
          inline strip showing agent name, run-status badge, and the
          started-at timestamp. The block content below remains a
          normal editable block. */}
      {isAgentRun && (
        <div
          data-testid="agent-run-header"
          aria-label="Agent run"
          title={
            runError
              ? `Agent run (${runStatus ?? 'unknown'}): ${runError}`
              : runStatus
                ? `Agent run (${runStatus})`
                : 'Agent run'
          }
          style={{
            flexShrink: 0,
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
            alignSelf: 'center',
            flexWrap: 'wrap',
            maxWidth: '100%',
          }}
        >
          {agentName && (
            <span
              data-testid="agent-run-agent"
              style={{
                fontSize: '11px',
                fontWeight: 600,
                padding: '2px 8px',
                borderRadius: 'var(--radius-pill)',
                background: 'var(--color-accent-subtle, rgba(99, 102, 241, 0.12))',
                color: 'var(--color-accent)',
                lineHeight: 1.4,
                display: 'inline-flex',
                alignItems: 'center',
                gap: '4px',
                whiteSpace: 'nowrap',
              }}
            >
              <span aria-hidden="true">🤖</span>
              {agentName}
            </span>
          )}
          {agentModel && (
            <span
              data-testid="agent-run-model"
              style={{
                fontSize: '11px',
                fontWeight: 500,
                color: 'var(--color-text-muted)',
                whiteSpace: 'nowrap',
              }}
            >
              {agentModel}
            </span>
          )}
          {runStatus && (
            <span
              data-testid="agent-run-status"
              style={{
                fontSize: '11px',
                fontWeight: 600,
                padding: '2px 8px',
                borderRadius: 'var(--radius-pill)',
                background: AGENT_RUN_STATUS_STYLES[runStatus].bg,
                color: AGENT_RUN_STATUS_STYLES[runStatus].text,
                lineHeight: 1.4,
                letterSpacing: '0.01em',
                whiteSpace: 'nowrap',
              }}
            >
              {runStatus.toUpperCase()}
            </span>
          )}
          {startedAt && (
            <span
              data-testid="agent-run-started-at"
              title={`Started at ${startedAt}`}
              style={{
                fontSize: '11px',
                fontWeight: 400,
                color: 'var(--color-text-muted)',
                whiteSpace: 'nowrap',
              }}
            >
              {startedAt}
            </span>
          )}
        </div>
      )}

      {/* Inline property badges (roadmap #13). Rendered as part of the
          "chrome" around the block content — they sit between the
          priority/marker badges and the editable text. Lazy-loaded
          because the `InlinePropertyBadges` component (and its template
          helpers) is not on the critical path of every block. */}
      <Suspense fallback={null}>
        <InlinePropertyBadges block={block} onUpdate={onUpdate} />
      </Suspense>

      {/* Content: edit mode shows raw contentEditable, read mode shows rendered inline.
          For `type:: view` blocks the read-mode path delegates to SavedViewBlock instead
          of InlineContent — the view IS the content (table/kanban/etc.) and there is no
          literal text to edit. The user edits the view via the properties panel
          (view-type::, view-name::, data-source::). */}
      {isEditing && !isView ? (
        <div
          key="edit"
          ref={contentRef}
          className="block-content type-body-lg"
          contentEditable
          suppressContentEditableWarning
          role="textbox"
          aria-multiline="false"
          aria-label="Block content"
          tabIndex={0}
          style={{
            flex: 1,
            minWidth: 0,
            color: isDimmed ? 'var(--color-text-disabled)' : 'var(--color-text-primary)',
            outline: 'none',
            minHeight: '1.5em',
            wordBreak: 'break-word',
            whiteSpace: 'pre-wrap',
          }}
          onInput={handleInput}
          onBlur={handleBlur}
          onKeyDown={handleKeyDown}
        />
      ) : isView ? (
        // Lazy-loaded SavedViewBlock — the heavy view components
        // (TableView, KanbanBoard) only enter the bundle when the
        // user actually has a view block on screen.
        <div
          key="view"
          data-testid="block-view-content"
          style={{
            flex: 1,
            minWidth: 0,
          }}
        >
          <Suspense fallback={<SavedViewFallback />}>
            <SavedViewBlock block={block} allBlocks={allBlocks} />
          </Suspense>
        </div>
      ) : (
        <div
          key="read"
          onClick={handleStartEdit}
          className="block-content-read type-body-lg"
          style={{
            flex: 1,
            minWidth: 0,
            color: isDimmed ? 'var(--color-text-disabled)' : 'var(--color-text-primary)',
            cursor: 'text',
            textDecoration: block.marker === 'Cancelled' ? 'line-through' : 'none',
            minHeight: '1.5em',
            wordBreak: 'break-word',
          }}
        >
          <InlineContent
            content={block.content}
            blocks={allBlocks}
            pageMap={pageMap}
            openTab={openTab}
            suppressInlineProperties
            onPropertiesExtracted={handlePropsExtracted}
          />
        </div>
      )}

      {/* Property strip — renders multiple in-block properties (key:: value)
          as a compact, structured card below the content. Property text is
          stripped from the inline content to avoid double-rendering. */}
      {extractedProperties.length > 0 && (
        <PropertyStrip block={block} properties={extractedProperties} onUpdate={onUpdate} />
      )}

      {/* ADR-0003: `created_by` badge — small pill that distinguishes
          human vs agent authorship. Sits inline with the block content. */}
      {createdByStr && (
        <span
          data-testid="created-by-badge"
          title={`Created by ${createdByStr}`}
          style={{
            flexShrink: 0,
            alignSelf: 'center',
            fontSize: '10px',
            fontWeight: 500,
            padding: '1px 6px',
            borderRadius: 'var(--radius-pill)',
            background: isAgentAuthor
              ? 'var(--color-accent-subtle, rgba(99, 102, 241, 0.12))'
              : 'var(--color-surface-subtle)',
            color: isAgentAuthor
              ? 'var(--color-accent)'
              : 'var(--color-text-muted)',
            display: 'inline-flex',
            alignItems: 'center',
            gap: '3px',
            letterSpacing: '0.01em',
            whiteSpace: 'nowrap',
            maxWidth: '160px',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
          }}
        >
          <span aria-hidden="true">{isAgentAuthor ? '🤖' : '👤'}</span>
          <span style={{ overflow: 'hidden', textOverflow: 'ellipsis' }}>{createdByStr}</span>
        </span>
      )}

      {/* Block actions — appears on hover */}
      <div
        className="block-actions"
        style={{
          display: 'flex',
          gap: 'var(--space-1)',
          opacity: 0,
          transition: 'opacity var(--motion-fast) var(--ease-standard)',
          flexShrink: 0,
          alignItems: 'center',
          position: 'absolute',
          right: 'var(--space-2)',
          top: 'var(--space-1)',
        }}
        onMouseEnter={e => {
          (e.currentTarget as HTMLDivElement).style.opacity = '1'
        }}
        onMouseLeave={e => {
          (e.currentTarget as HTMLDivElement).style.opacity = '0'
        }}
      >
        {/* The parent .block-row hover makes these visible via CSS */}
        {onAddComment && (
          <button
            onClick={() => onAddComment(block.id)}
            aria-label="Add comment"
            title="Add comment"
            data-testid={`add-comment-${block.id}`}
            style={{
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              color: 'var(--color-text-muted)',
              padding: '2px',
              borderRadius: 'var(--radius-sm)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              lineHeight: 1,
            }}
          >
            <MessageCircle size={14} />
          </button>
        )}
        <button
          onClick={handleAddChild}
          aria-label="Add child block"
          title="Add child block"
          style={{
            background: 'none',
            border: 'none',
            cursor: 'pointer',
            color: 'var(--color-text-muted)',
            padding: '2px',
            borderRadius: 'var(--radius-sm)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            lineHeight: 1,
          }}
        >
          <Plus size={14} />
        </button>
        <button
          ref={contextMenuAnchorRef}
          onClick={() => setShowContextMenu((v) => !v)}
          aria-label="More actions"
          aria-haspopup="menu"
          aria-expanded={showContextMenu}
          title="More actions"
          data-testid="block-context-menu-trigger"
          style={{
            background: showContextMenu ? 'var(--color-surface-subtle)' : 'none',
            border: 'none',
            cursor: 'pointer',
            color: 'var(--color-text-muted)',
            padding: '2px',
            borderRadius: 'var(--radius-sm)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            lineHeight: 1,
          }}
        >
          <MoreHorizontal size={14} />
        </button>
        <button
          onClick={() => setShowProperties(!showProperties)}
          aria-label={showProperties ? 'Hide properties' : 'Show properties'}
          title={showProperties ? 'Hide properties' : 'Properties'}
          className="drag-handle"
          style={{
            background: 'none',
            border: 'none',
            cursor: 'pointer',
            color: showProperties
              ? 'var(--color-accent)'
              : 'var(--color-text-muted)',
            padding: '2px',
            borderRadius: 'var(--radius-sm)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            lineHeight: 1,
          }}
        >
          <Settings2 size={14} />
        </button>
      </div>

      {/* Page autocomplete dropdown */}
      {autocomplete && (
        <Suspense fallback={null}>
          <PageAutocomplete
            position={autocomplete.position}
            query={autocomplete.query}
            onSelect={handleAutocompleteSelect}
            onClose={() => setAutocomplete(null)}
          />
        </Suspense>
      )}

      {/* Block autocomplete dropdown */}
      {blockAutocomplete && (
        <BlockAutocomplete
          position={blockAutocomplete.position}
          query={blockAutocomplete.query}
          onSelect={handleBlockAutocompleteSelect}
          onClose={() => setBlockAutocomplete(null)}
        />
      )}

      {/* Tag autocomplete dropdown */}
      {tagAutocomplete && (
        <TagAutocomplete
          position={tagAutocomplete.position}
          query={tagAutocomplete.query}
          onSelect={handleTagAutocompleteSelect}
          onClose={() => setTagAutocomplete(null)}
        />
      )}

      {/* Slash command menu */}
      {slashCommand && (
        <Suspense fallback={null}>
          <SlashCommandMenu
            position={slashCommand.position}
            query={slashCommand.query}
            onSelect={handleSlashSelect}
            onClose={() => setSlashCommand(null)}
          />
        </Suspense>
      )}

      {/* Block properties panel */}
      {showProperties && (
        <div
          style={{
            marginLeft: `${indent * 24 + 32}px`,
            marginTop: 'var(--space-1)',
            width: 'calc(100% - 32px)',
          }}
        >
          <Suspense fallback={null}>
            <BlockPropertiesPanel
              blockId={block.id}
              onClose={() => setShowProperties(false)}
            />
          </Suspense>
        </div>
      )}

      {/* Comments thread — inline below the block */}
      {onResolveComment && commentTree.length > 0 && (
        <CommentsThread
          tree={commentTree}
          onResolve={onResolveComment}
          onReply={onReplyComment ?? (() => {})}
          onDelete={onDeleteComment}
          indent={indent}
        />
      )}

      {/* Block context menu — DESIGN.md §11.3 */}
      <BlockContextMenu
        open={showContextMenu}
        anchorEl={contextMenuAnchorRef.current}
        onClose={() => setShowContextMenu(false)}
        actions={{
          onAddChild: handleAddChild,
          onMoveUp: () => onMoveBlockUp(block.id),
          onMoveDown: () => onMoveBlockDown(block.id),
          onConvertToTask: handleConvertToTask,
          onCopyLink: handleCopyBlockLink,
          onDelete: () => onDeleteBlock(block.id),
          // F3 of quilt-fase2-ux-dead-buttons — the discoverable
          // way to open the BlockPropertiesPanel. The hover-revealed
          // Settings2 button on the row itself still works, but
          // the context menu is always reachable and matches the
          // design intent for "block properties".
          onShowProperties: () => setShowProperties(true),
        }}
      />

      {/* Date picker popover anchored to the block row.
          Rendered at the row level (not inside InlinePropertyBadges) so
          the slash command handler can also trigger it. */}
      {datePickerField && (
        <div
          ref={datePickerTriggerRef}
          style={{
            position: 'absolute',
            top: '100%',
            left: 0,
            zIndex: 9999,
          }}
        >
          <DatePicker
            // Read the current value from the block's scheduling fields.
            // ISO string "2026-06-15T00:00:00Z" → just the date portion "2026-06-15".
            value={
              (datePickerField === 'deadline' ? block.deadline : block.scheduled)
                ?.split('T')[0] ?? null
            }
            onChange={handleDatePickerSelect}
            onCancel={() => setDatePickerField(null)}
            placeholder={
              datePickerField === 'deadline'
                ? 'today, tomorrow…'
                : 'today, tomorrow…'
            }
          />
        </div>
      )}
    </div>
  )
}

// ──── CommentsThread ───────────────────────────────────────────────
// Recursive comment thread renderer. Comments are regular child blocks
// with `type: "comment"`; replies are nested comments of the same kind.

interface CommentsThreadProps {
  tree: ReturnType<typeof buildCommentTree>
  onResolve: (id: string) => void
  onReply: (id: string) => void
  onDelete?: (id: string) => void
  indent: number
}

function CommentsThread({
  tree,
  onResolve,
  onReply,
  onDelete,
  indent,
}: CommentsThreadProps) {
  const totalResolved = countResolved(tree)

  return (
    <div
      data-testid={`comments-thread`}
      style={{
        marginLeft: `${indent * 24 + 32}px`,
        marginTop: 'var(--space-1)',
        padding: 'var(--space-2) var(--space-3)',
        background: 'var(--color-surface-subtle)',
        borderLeft: '2px solid var(--color-accent)',
        borderRadius: 'var(--radius-sm)',
      }}
    >
      <div
        style={{
          fontSize: '11px',
          fontWeight: 600,
          color: 'var(--color-text-muted)',
          marginBottom: 'var(--space-1)',
          display: 'flex',
          alignItems: 'center',
          gap: 'var(--space-1)',
        }}
      >
        <span aria-hidden="true">💬</span>
        <span>
          {tree.length} comment{tree.length > 1 ? 's' : ''}
        </span>
        {totalResolved > 0 && (
          <span style={{ color: 'var(--color-success)' }}>
            ({totalResolved} resolved)
          </span>
        )}
      </div>
      {tree.map(node => (
        <CommentThreadNode
          key={node.comment.id}
          node={node}
          onResolve={onResolve}
          onReply={onReply}
          onDelete={onDelete}
        />
      ))}
    </div>
  )
}

function CommentThreadNode({
  node,
  onResolve,
  onReply,
  onDelete,
  depth = 0,
}: {
  node: ReturnType<typeof buildCommentTree>[number]
  onResolve: (id: string) => void
  onReply: (id: string) => void
  onDelete?: (id: string) => void
  depth?: number
}) {
  return (
    <div>
      <CommentRow
        comment={node.comment}
        onResolve={onResolve}
        onReply={onReply}
        onDelete={onDelete}
        depth={depth}
      />
      {node.replies.length > 0 && (
        <div
          style={{
            marginLeft: 'var(--space-3)',
            borderLeft: '1px solid var(--color-border)',
            paddingLeft: 'var(--space-2)',
          }}
        >
          {node.replies.map(reply => (
            <CommentThreadNode
              key={reply.comment.id}
              node={reply}
              onResolve={onResolve}
              onReply={onReply}
              onDelete={onDelete}
              depth={depth + 1}
            />
          ))}
        </div>
      )}
    </div>
  )
}

function countResolved(
  tree: ReturnType<typeof buildCommentTree>,
): number {
  let count = 0
  for (const node of tree) {
    const resolved =
      String(
        node.comment.properties?.find(p => p.key === 'resolved')?.value ??
          'false',
      ) === 'true'
    if (resolved) count += 1
    count += countResolved(node.replies)
  }
  return count
}

// ──── SavedView fallback (ADR-DRAFT-saved-view-block-role) ──────
//
// Lightweight placeholder rendered while the lazy-loaded
// SavedViewBlock bundle is being fetched. The full SavedViewBlock
// mounts the same `data-testid="saved-view-block"` so the fallback
// and the real component can be swapped without test churn.
function SavedViewFallback() {
  return (
    <div
      data-testid="saved-view-block"
      style={{
        padding: 'var(--space-2) 0',
        color: 'var(--color-text-muted)',
        fontSize: '13px',
      }}
    >
      Loading view…
    </div>
  )
}
