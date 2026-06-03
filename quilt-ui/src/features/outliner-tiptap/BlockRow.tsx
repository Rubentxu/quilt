import { lazy, Suspense, useState, useRef, useCallback, useEffect, type KeyboardEvent } from 'react'
import { GripVertical, Plus, MoreHorizontal, ChevronDown, ChevronRight, Settings2, MessageCircle } from 'lucide-react'
import { useNavigate } from '@tanstack/react-router'
import toast from 'react-hot-toast'
import { api } from '@core/api-client'
import { setCursorAt, getCursorPosition, isCursorAtStart, isCursorAtEnd } from '@shared/hooks/useCursor'
import type { Block, BlockType, TaskMarker, Priority, Page } from '@shared/types/api'
import { useTabs } from '@shared/contexts/TabsContext'
import { InlineContent } from './InlineContent'
import { BlockAutocomplete } from '@features/search/BlockAutocomplete'
import { TagAutocomplete } from '@features/search/TagAutocomplete'
import { CommentRow } from '@features/comments/CommentRow'
import { BlockContextMenu } from './BlockContextMenu'
import { buildCommentTree } from '@shared/utils/blockProperties'
// Type-only import — keeps the type for handleSlashSelect without
// pulling the (large) SlashCommandMenu module into the eager bundle.
import type { SlashMenuItem } from './SlashCommandMenu'

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
  Done: { bg: 'var(--color-success)', text: '#fff' },
  Now: { bg: 'var(--color-danger)', text: '#fff' },
  Later: { bg: 'var(--color-warning)', text: '#fff' },
  Cancelled: { bg: 'var(--color-text-disabled)', text: '#fff' },
}

const PRIORITY_STYLES: Record<Priority, { bg: string; text: string }> = {
  A: { bg: 'var(--color-danger)', text: '#fff' },
  B: { bg: 'var(--color-warning)', text: '#fff' },
  C: { bg: 'var(--color-text-muted)', text: '#fff' },
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
  const contentRef = useRef<HTMLDivElement>(null)
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const rowRef = useRef<HTMLDivElement>(null)

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
      pages.forEach(p => map.set(p.name, p))
      setPageMap(map)
    }).catch(() => {
      // Non-critical; refs gracefully fall back
    })
  }, [])

  // ── Content save ──────────────────────────────────────────────
  const saveToApi = useCallback(
    async (text: string) => {
      // Preserve the exact text the user typed. Logseq-style editors do not
      // silently trim leading/trailing whitespace on blur.
      if (text === block.content) return
      try {
        const updated = await api.updateBlock(block.id, { content: text })
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
  // delimiter — Logseq tags are atomic and do not need a closing char).
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
  const handleSlashSelect = useCallback(
    (item: SlashMenuItem) => {
      setSlashCommand(null)

      // Clear the "/" text from the block
      const newContent = ''
      setLocalContent(newContent)
      if (contentRef.current) contentRef.current.textContent = newContent

      if (item.blockType) {
        // Existing behavior: change block type
        api.updateBlock(block.id, { blockType: item.blockType as BlockType }).then(updated => {
          onUpdate(updated)
        }).catch(() => {
          toast.error('Failed to change block type')
        })
        return
      }

      if (item.action) {
        const [prefix, value] = item.action.split(':')

        switch (prefix) {
          case 'status': {
            // Map lowercase action values to TaskMarker casing; 'Doing' cast needed
            const statusMap: Record<string, TaskMarker> = {
              todo: 'Todo',
              doing: 'Doing' as TaskMarker,
              done: 'Done',
              now: 'Now',
              later: 'Later',
              cancelled: 'Cancelled',
            }
            const marker = statusMap[value]
            if (marker) {
              api.updateBlock(block.id, { marker }).then(onUpdate).catch(() => {
                toast.error('Failed to set status')
              })
            }
            break
          }

          case 'priority': {
            const priority = value as Priority
            api.updateBlock(block.id, { priority }).then(onUpdate).catch(() => {
              toast.error('Failed to set priority')
            })
            break
          }

          case 'date': {
            const today = new Date().toISOString().split('T')[0]
            const tomorrow = new Date(Date.now() + 86400000).toISOString().split('T')[0]
            const dateStr = value === 'today' ? today : value === 'tomorrow' ? tomorrow : today
            setLocalContent(dateStr)
            if (contentRef.current) contentRef.current.textContent = dateStr
            debouncedSave(dateStr)
            break
          }

          case 'property': {
            // Insert property syntax (e.g. "deadline:: ")
            const propStr = `${value}:: `
            setLocalContent(propStr)
            if (contentRef.current) contentRef.current.textContent = propStr
            // Place cursor after the inserted text
            setTimeout(() => {
              if (contentRef.current) setCursorAt(contentRef.current, 'end')
            }, 0)
            debouncedSave(propStr)
            break
          }

          case 'ref': {
            if (value === 'page') {
              // Trigger [[ autocomplete
              setLocalContent('[[')
              if (contentRef.current) contentRef.current.textContent = '[['
              setTimeout(() => {
                if (contentRef.current) setCursorAt(contentRef.current, 'end')
              }, 0)
            } else if (value === 'block') {
              // Trigger (( autocomplete
              setLocalContent('((')
              if (contentRef.current) contentRef.current.textContent = '(('
              setTimeout(() => {
                if (contentRef.current) setCursorAt(contentRef.current, 'end')
              }, 0)
            }
            break
          }

          case 'template': {
            // Insert Template — create a new page from a `template/...` page.
            // ADR-0003: templates define structure + types for human-agent collab.
            if (value === 'insert') {
              handleInsertTemplate()
            }
            break
          }
        }

        return
      }
    },
    [block.id, onUpdate, debouncedSave, navigate],
  )

  // ── Keyboard handling ─────────────────────────────────────────
  const handleKeyDown = useCallback(
    (e: KeyboardEvent<HTMLDivElement>) => {
      const el = contentRef.current
      if (!el) return

      // ── Inline formatting helper ──────────────────────────────
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

      // ── Slash menu is open: intercept keys so the menu handles them ──
      if (slashCommand) {
        if (e.key === 'Escape') {
          e.preventDefault()
          setSlashCommand(null)
          return
        }
        // Enter, ArrowUp, ArrowDown handled by the menu's document listener
        if (['Enter', 'ArrowUp', 'ArrowDown'].includes(e.key)) {
          e.preventDefault()
          return
        }
        return // Let other keys through (typing continues to filter)
      }

      // ── Block autocomplete is open: intercept keys so the menu handles them ──
      if (blockAutocomplete) {
        if (e.key === 'Escape') {
          e.preventDefault()
          setBlockAutocomplete(null)
          return
        }
        // Enter, ArrowUp, ArrowDown handled by the menu's document listener
        if (['Enter', 'ArrowUp', 'ArrowDown'].includes(e.key)) {
          e.preventDefault()
          return
        }
        return // Let other keys through (typing continues to filter)
      }

      // ── Tag autocomplete is open: intercept keys so the menu handles them ──
      if (tagAutocomplete) {
        if (e.key === 'Escape') {
          e.preventDefault()
          setTagAutocomplete(null)
          return
        }
        // Enter, ArrowUp, ArrowDown handled by the menu's document listener
        if (['Enter', 'ArrowUp', 'ArrowDown'].includes(e.key)) {
          e.preventDefault()
          return
        }
        return // Let other keys through (typing continues to filter)
      }

      // ── Page autocomplete is open: intercept keys so the menu handles them ──
      // The page autocomplete ([[..) handles keyboard nav via its own document
      // listener (PageAutocomplete.tsx), but we MUST prevent the editor from
      // also handling Enter (which would create a new block) and ArrowUp/Down
      // (which would move the cursor).
      if (autocomplete) {
        if (e.key === 'Escape') {
          e.preventDefault()
          setAutocomplete(null)
          return
        }
        if (['Enter', 'ArrowUp', 'ArrowDown'].includes(e.key)) {
          e.preventDefault()
          return
        }
        return // Let other keys through (typing continues to filter)
      }

      // ── Escape: exit editing mode (autocomplete menus are handled above) ──
      if (e.key === 'Escape') {
        e.preventDefault()
        if (contentRef.current) {
          contentRef.current.blur()
        }
        return
      }

      // ── Ctrl+Z: Undo ──
      if (e.key === 'z' && (e.ctrlKey || e.metaKey) && !e.shiftKey) {
        e.preventDefault()
        onUndo()
        return
      }

      // ── Ctrl+Shift+Z / Ctrl+Y: Redo ──
      if (
        (e.key === 'z' && (e.ctrlKey || e.metaKey) && e.shiftKey) ||
        (e.key === 'y' && (e.ctrlKey || e.metaKey))
      ) {
        e.preventDefault()
        onRedo()
        return
      }

      // ── Ctrl+B / Ctrl+I / Ctrl+`: Inline formatting ──
      if (e.key === 'b' && (e.ctrlKey || e.metaKey) && !e.shiftKey) {
        e.preventDefault()
        toggleInlineMark('**')
        return
      }
      if (e.key === 'i' && (e.ctrlKey || e.metaKey) && !e.shiftKey) {
        e.preventDefault()
        toggleInlineMark('*')
        return
      }
      if (e.key === '`' && (e.ctrlKey || e.metaKey)) {
        e.preventDefault()
        toggleInlineMark('`')
        return
      }

      // ── Mod+A / Mod+Shift+A: Select parent / Select all ──
      if (e.key === 'a' && (e.ctrlKey || e.metaKey)) {
        e.preventDefault()
        if (e.shiftKey) {
          // Mod+Shift+A — select all blocks
          onSelectAll?.()
        } else {
          // Mod+A — select parent (all siblings of current block)
          onSelectParent?.(block.id, block.parentId)
        }
        return
      }

      // ── Ctrl+C: Copy (block-level if no text selection) ──
      if (e.key === 'c' && (e.ctrlKey || e.metaKey) && !e.shiftKey) {
        const sel = window.getSelection()
        if (sel && sel.rangeCount > 0 && !sel.getRangeAt(0).collapsed) {
          // Text selection: let browser handle native copy
          return
        }
        e.preventDefault()
        const text = localContent || el.textContent || ''
        navigator.clipboard.writeText(text).catch(() => {})
        toast.success('Block copied')
        return
      }

      // ── Ctrl+X: Cut block ──
      if (e.key === 'x' && (e.ctrlKey || e.metaKey)) {
        const sel = window.getSelection()
        if (sel && sel.rangeCount > 0 && !sel.getRangeAt(0).collapsed) {
          // Text selection: let browser handle native cut
          return
        }
        e.preventDefault()
        const text = localContent || el.textContent || ''
        navigator.clipboard.writeText(text).catch(() => {})
        onDeleteBlock(block.id)
        toast.success('Block cut')
        return
      }

      // ── Ctrl+V: Paste creates new block ──
      if (e.key === 'v' && (e.ctrlKey || e.metaKey)) {
        e.preventDefault()
        navigator.clipboard.readText().then(clipText => {
          if (clipText) {
            onCreateBlock(block.id, clipText, block.parentId)
          }
        }).catch(() => {})
        return
      }

      // ── Shift+Enter: Soft newline within block ──
      if (e.key === 'Enter' && e.shiftKey) {
        e.preventDefault()
        const sel = window.getSelection()
        if (!sel || sel.rangeCount === 0) return

        const range = sel.getRangeAt(0)
        const newNode = document.createTextNode('\n')
        range.deleteContents()
        range.insertNode(newNode)

        // Move cursor after the inserted newline
        range.setStartAfter(newNode)
        range.setEndAfter(newNode)
        sel.removeAllRanges()
        sel.addRange(range)

        // Defer state update to let DOM settle
        setTimeout(() => {
          const text = el.textContent || ''
          setLocalContent(text)
          debouncedSave(text)
        }, 0)
        return
      }

      // ── Mod+Enter: Cycle marker (None → Todo → Done → None) ──
      if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
        e.preventDefault()

        // Match WASM CycleMarker: None → Todo → Done → None
        const CYCLE: (TaskMarker | null)[] = [null, 'Todo', 'Done']
        const currentIdx = CYCLE.indexOf(block.marker ?? null)
        const nextIdx = currentIdx >= 0
          ? (currentIdx + 1) % CYCLE.length
          : 0
        const nextMarker = CYCLE[nextIdx]

        // Optimistic update
        onUpdate({ ...block, marker: nextMarker })

        // Persist
        api.updateBlock(block.id, { marker: nextMarker }).catch(() => {
          toast.error('Failed to cycle marker')
        })
        return
      }

      // ── Enter: Split block or create sibling ──
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault()
        const cursorPos = getCursorPosition(el)
        const fullText = el.textContent ?? ''

        // Case 1: Empty block → create empty sibling below
        if (!fullText.trim()) {
          onCreateBlock(block.id, '', block.parentId)
          return
        }

        // Case 2: Cursor at end of non-empty block → create empty sibling below
        if (cursorPos >= fullText.length) {
          onCreateBlock(block.id, '', block.parentId)
          return
        }

        // Case 3: Cursor in middle → split block at cursor position
        const beforeCursor = fullText.slice(0, cursorPos)
        const afterCursor = fullText.slice(cursorPos)

        // Update current block with content before cursor
        el.textContent = beforeCursor
        setLocalContent(beforeCursor)
        if (saveTimerRef.current) clearTimeout(saveTimerRef.current)
        saveToApi(beforeCursor)

        // Position cursor at end of updated content
        setCursorAt(el, 'end')

        // Create new block with content after cursor
        onCreateBlock(block.id, afterCursor, block.parentId)
        return
      }

      // ── Backspace at start: Merge with prev ──
      if (e.key === 'Backspace' && isCursorAtStart(el)) {
        e.preventDefault()
        const prev = findPrevSibling(block, allBlocks)
        if (!prev) {
          // No previous sibling → outdent instead (if has parent)
          if (block.parentId) {
            handleOutdent()
          }
          return
        }

        const currentText = el.textContent ?? ''
        const mergedContent = prev.content + currentText

        // Update previous block with merged content
        api.updateBlock(prev.id, { content: mergedContent }).then(updated => onUpdate(updated))
        // Delete current block
        api.deleteBlock(block.id).then(() => onDeleteBlock(block.id))

        // Focus previous block at join point
        onFocusBlock(prev.id, 'end')

        // Update previous block's content optimistically in local state
        onUpdate({ ...prev, content: mergedContent })
        return
      }

      // ── Tab: Indent ──
      if (e.key === 'Tab' && !e.shiftKey) {
        e.preventDefault()
        handleIndent()
        return
      }

      // ── Shift+Tab: Outdent ──
      if (e.key === 'Tab' && e.shiftKey) {
        e.preventDefault()
        handleOutdent()
        return
      }

      // ── ArrowUp: Alt+Shift+Up (move), Alt+Up (multi-select), or default ──
      if (e.key === 'ArrowUp') {
        // KBD-014: Alt+Shift+Up — move block up
        if (e.altKey && e.shiftKey) {
          e.preventDefault()
          onMoveBlockUp(block.id)
          return
        }
        // KBD-020: Alt+Up — extend selection upward (Logseq-style multi-select)
        if (e.altKey) {
          e.preventDefault()
          onMultiSelect?.(block.id, 'up')
          return
        }
        // Default: ArrowUp at start — focus previous block
        if (isCursorAtStart(el)) {
          e.preventDefault()
          const prev = findPrevSibling(block, allBlocks)
          if (prev) {
            onFocusBlock(prev.id, 'end')
          }
        }
        return
      }

      // ── ArrowDown: Alt+Shift+Down (move), Alt+Down (multi-select), or default ──
      if (e.key === 'ArrowDown') {
        // KBD-015: Alt+Shift+Down — move block down
        if (e.altKey && e.shiftKey) {
          e.preventDefault()
          onMoveBlockDown(block.id)
          return
        }
        // KBD-020: Alt+Down — extend selection downward (Logseq-style multi-select)
        if (e.altKey) {
          e.preventDefault()
          onMultiSelect?.(block.id, 'down')
          return
        }
        // Default: ArrowDown at end — focus next block
        if (isCursorAtEnd(el)) {
          e.preventDefault()
          const next = findNextSibling(block, allBlocks)
          if (next) {
            onFocusBlock(next.id, 'start')
          }
        }
        return
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [block, allBlocks, pageName, autocomplete, blockAutocomplete, tagAutocomplete, slashCommand, saveToApi, onUpdate, onCreateBlock, onDeleteBlock, onFocusBlock, onMoveBlockUp, onMoveBlockDown, onMultiSelect, onSelectAll, onSelectParent, onUndo, onRedo],
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

    api.updateBlock(block.id, { parentId: newParentId, level: newLevel, order: newOrder })
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
  // The slash command's "Insert Template" action invokes this. The flow is:
  //   1. Ask the user for the new page's name.
  //   2. Fetch the list of pages and filter for `template/...` templates.
  //   3. If none, bail with a toast. If exactly one, use it. If several,
  //      prompt the user to pick one by name or title.
  //   4. POST /api/v1/pages/from-template — the server clones the
  //      template's block tree, substitutes `{{title}}` / `{{date}}` /
  //      `{{name}}` placeholders, and returns the new page id.
  //   5. Navigate to the freshly created page.
  //
  // This is deliberately a thin orchestration layer: a richer template
  // picker UI is left as a follow-up. The browser-native `prompt` keeps
  // the change minimal until the picker component lands.
  const handleInsertTemplate = useCallback(async () => {
    const newPageName = window.prompt('New page name:')
    if (!newPageName || !newPageName.trim()) return

    const trimmed = newPageName.trim()

    try {
      // 1. List pages and filter for templates (names starting with `template/`)
      const pages = await api.listPages()
      const templates = pages.filter(p => p.name.startsWith('template/'))

      if (templates.length === 0) {
        toast.error('No templates found. Create a page whose name starts with "template/".')
        return
      }

      // 2. Pick a template (auto-pick if only one)
      let template = templates[0]
      if (templates.length > 1) {
        const labels = templates.map(t => t.title || t.name).join(', ')
        const choice = window.prompt(
          `Choose template (${labels}):`,
          templates[0].title || templates[0].name,
        )
        if (!choice || !choice.trim()) return
        const picked = templates.find(
          t => t.name === choice.trim() || t.title === choice.trim(),
        )
        if (!picked) {
          toast.error(`Template not found: ${choice}`)
          return
        }
        template = picked
      }

      // 3. Call the server endpoint to clone the template's blocks
      const result = await api.createPageFromTemplate({
        templateName: template.name,
        pageName: trimmed,
        title: trimmed,
      })

      toast.success(
        `Created from template "${template.title || template.name}" (${result.blocksCreated} blocks)`,
      )

      // 4. Navigate to the new page
      navigate({ to: '/page/$name', params: { name: result.page.name } })
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      toast.error(`Failed to create from template: ${message}`)
    }
  }, [navigate])

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

  const isDimmed = block.marker === 'Done' || block.marker === 'Cancelled'

  return (
    <div
      ref={rowRef}
      className="block-row"
      data-testid={`block-row-${block.id}`}
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

      {/* Drag handle — visible on row hover */}
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
        }}
      >
        <GripVertical size={14} />
      </div>

      {/* Bullet / collapse toggle — chevron if has children, dot otherwise */}
      <button
        onClick={handleBulletClick}
        aria-label={hasChildren ? (isCollapsed ? 'Expand block' : 'Collapse block') : 'Bullet'}
        title={hasChildren ? (isCollapsed ? 'Expand block' : 'Collapse block') : undefined}
        style={{
          marginTop: hasChildren ? '5px' : '7px',
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

      {/* Marker badge — pill shape per DESIGN.md §9.6 */}
      {block.marker && (
        <span
          style={{
            flexShrink: 0,
            marginTop: '2px',
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
            marginTop: '2px',
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

      {/* Content: edit mode shows raw contentEditable, read mode shows rendered inline */}
      {isEditing ? (
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
          <InlineContent content={block.content} blocks={allBlocks} pageMap={pageMap} openTab={openTab} />
        </div>
      )}

      {/* ADR-0003: `created_by` badge — small pill that distinguishes
          human vs agent authorship. Sits inline with the block content. */}
      {createdByStr && (
        <span
          data-testid="created-by-badge"
          title={`Created by ${createdByStr}`}
          style={{
            flexShrink: 0,
            marginTop: '4px',
            alignSelf: 'flex-start',
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
        }}
      />
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
