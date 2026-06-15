// ─── SlashActionRegistry — quilt-architecture-review-c4-slash-registry ───
//
// Self-describing registry for slash-command actions. Replaces the legacy
// `SLASH_MENU_ITEMS` array (in SlashCommandMenu.tsx) + 100-line switch on
// `item.action.split(':')` (in BlockRow.tsx) with a single source of truth:
// each entry carries BOTH the menu metadata (label, icon, keywords) AND
// the handler function that runs when the user selects it.
//
// Why a registry and not a plain Map<id, handler>?
//   - We want the menu to enumerate entries by `registry.allItems()` so
//     the SlashCommandMenu component is a pure renderer.
//   - We want the handler to live with the metadata so adding a new
//     action is one `register()` call, not three file edits.
//   - The registry is a class (not a `Map` export) so callers can't
//     bypass `register()` and lose handler attachment.
//
// Connascence reduction (c4 candidate #4):
//   - Removes connascence-of-meaning between `item.action = "status:todo"`
//     in SlashCommandMenu and `prefix === "status"` in BlockRow.
//   - Removes connascence-of-execution between BlockRow.handleSlashSelect
//     and the menu (BlockRow no longer needs to know the action protocol).
//
// Usage from BlockRow:
//
//     import { defaultRegistry, type SlashContext } from './slashRegistry'
//
//     const handleSlashSelect = useCallback((item: SlashMenuItem) => {
//       setSlashCommand(null)
//       const ctx: SlashContext = {
//         block, allBlocks, api, toast, navigate,
//         setContent, setContentAtEnd, onUpdate, originalContent: localContent,
//         onAddComment,
//       }
//       const handler = defaultRegistry.getHandler(item.id)
//       handler?.(ctx, item)
//     }, [...])
//
// The `defaultRegistry` is built at module load with the 18 legacy items.
// Plugin authors can `defaultRegistry.register(myItem, myHandler)` to
// extend it from feature modules.

import {
  Type, Heading1, Heading2, Heading3, List, ListOrdered,
  CheckSquare, Quote, Code, Minus, Image,
  Circle, Loader, CheckCircle2, Zap, Clock, XCircle,
  AlertTriangle, AlertCircle, Flag,
  Calendar, CalendarDays, CalendarX, CalendarClock,
  FileText, FilePlus, Hash, MessageCircle,
  ListChecks, Terminal, LayoutTemplate,
  Hourglass,
} from 'lucide-react'
import type { Block, BlockType, TaskMarker, Priority, Page } from '@shared/types/api'
import type { api as ApiClient } from '@core/api-client'
import type toastFn from 'react-hot-toast'
import type { NavigateFn } from '@tanstack/react-router'

// ─── Public types ─────────────────────────────────────────────────────

/** Display metadata for a slash-menu entry. */
export interface SlashMenuItem {
  id: string
  label: string
  description: string
  icon: React.ReactNode
  /** For block-type changes. Items with `blockType` get the default
   *  `defaultBlockTypeHandler` when no explicit handler is registered. */
  blockType?: string
  /** For property/status actions. Kept for backwards compat with
   *  existing tests/imports (e.g. SlashTemplateFlow asserts on
   *  `item.action === 'template:insert'`). The registry does NOT
   *  parse this string — it routes by `id`. */
  action?: string
  keywords: string[]
  category: string
}

/** Everything a slash handler might need to do its job. The
 *  registry keeps the surface small but covers all 6 legacy switch
 *  branches + the dead `comment:add` branch. */
export interface SlashContext {
  /** The block the user is currently editing. */
  block: Block
  /** All blocks in the current page (for sibling lookups etc). */
  allBlocks: Block[]
  /** Typed view of the api-client (avoids each handler re-importing it). */
  api: typeof ApiClient
  /** Typed toast surface. */
  toast: typeof toastFn
  /** TanStack navigate function for cross-page jumps. */
  navigate: NavigateFn
  /** Replace the block content + persist after the debounce window. */
  setContent: (text: string) => void
  /** Same as `setContent` but moves the cursor to end after applying. */
  setContentAtEnd: (text: string) => void
  /** Push the API-returned block back up to the parent. */
  onUpdate: (block: Block) => void
  /** Snapshot of the block content at the moment the user selected
   *  a slash item. Used by `template:insert` to restore on cancel. */
  originalContent: string
  /** Optional comment callback from BlockRow props. */
  onAddComment?: (blockId: string) => void
  /** Optional template-insertion callback from BlockRow. Provided
   *  by the component because it owns the multi-step wizard
   *  (prompt → pick template → create → navigate). The registry
   *  handler delegates here so the switch is gone. */
  templateInsert?: (originalContent: string) => Promise<void> | void
  /** Opens the date picker popover anchored to the block.
   *  Field is 'deadline' or 'scheduled'.
   *  BlockRow provides this callback so the handler can signal
   *  the popover to open without owning the React state directly. */
  openDatePicker?: (field: 'deadline' | 'scheduled') => void
}

/** A slash action handler. Receives the active context and the
 *  selected item (for items that derive behaviour from the entry,
 *  e.g. `defaultBlockTypeHandler` reads `item.blockType`). */
export type SlashHandler = (ctx: SlashContext, item: SlashMenuItem) => void | Promise<void>

// ─── Registry class ───────────────────────────────────────────────────

/** Map from `SlashMenuItem.id` to a self-contained entry. The Map
 *  preserves insertion order, which we expose via `allItems()` so the
 *  SlashCommandMenu renders categories in registration order. */
export class SlashActionRegistry {
  private items = new Map<string, SlashMenuItem>()
  private handlers = new Map<string, SlashHandler>()

  /** Register an item and its handler. Re-registering with the same id
   *  replaces both the item and the handler (id is a key, not a slot). */
  register(item: SlashMenuItem, handler: SlashHandler): this {
    this.items.set(item.id, item)
    this.handlers.set(item.id, handler)
    return this
  }

  /** Look up a registered item by id. Returns undefined if not present. */
  getItem(id: string): SlashMenuItem | undefined {
    return this.items.get(id)
  }

  /** Look up a registered handler by id. Returns undefined if not present. */
  getHandler(id: string): SlashHandler | undefined {
    return this.handlers.get(id)
  }

  /** All registered items, in registration order. The SlashCommandMenu
   *  filters / groups from this list — no separate `SLASH_MENU_ITEMS`
   *  array needed. */
  allItems(): SlashMenuItem[] {
    return [...this.items.values()]
  }

  /** Number of registered entries. Useful for tests + dev introspection. */
  size(): number {
    return this.items.size
  }
}

// ─── Builtin handlers ─────────────────────────────────────────────────

/** Maps a slash item to a block-type update. The legacy switch handled
 *  block-type changes before the `action` switch, and the
 *  `api.updateBlock` + `toast.error` pattern is identical to status /
 *  priority updates — so we factor it out and reuse for any item that
 *  declares a `blockType` and has no explicit handler. */
export const defaultBlockTypeHandler: SlashHandler = async (ctx, item) => {
  if (!item.blockType) return
  try {
    const updated = await ctx.api.updateBlock(ctx.block.id, {
      blockType: item.blockType as BlockType,
    })
    ctx.onUpdate(updated)
  } catch {
    ctx.toast.error('Failed to change block type')
  }
}

/** The `comment:add` slash action. Delegates to the BlockRow-level
 *  `onAddComment` callback. If no callback is wired (e.g. tests,
 *  read-only contexts), we surface a toast instead of silently doing
 *  nothing. */
export const defaultCommentHandler: SlashHandler = (ctx, item) => {
  if (ctx.onAddComment) {
    ctx.onAddComment(ctx.block.id)
    return
  }
  ctx.toast.error('Comment callback not available')
}

// ─── Default registry construction ────────────────────────────────────
//
// We rebuild the 18 legacy entries here, but with a twist: each entry
// gets an explicit handler that captures the behaviour of the legacy
// switch. The result is a self-describing registry with zero string
// parsing and no centralized switch.

/** Status handler — updates the block's marker using the lowercased
 *  `id` suffix (e.g. `status-todo` → marker `Todo`). The legacy code
 *  parsed `item.action.split(':')[1]` and looked it up in a status
 *  map; we derive the marker directly from the id, which is the
 *  single source of truth. */
const statusMarkerByValue: Record<string, TaskMarker> = {
  todo: 'Todo',
  doing: 'Doing' as TaskMarker,
  done: 'Done',
  now: 'Now',
  later: 'Later',
  cancelled: 'Cancelled',
  waiting: 'Waiting' as TaskMarker,
}

const makeStatusHandler: (value: string) => SlashHandler = (value) => async (ctx) => {
  const marker = statusMarkerByValue[value]
  if (!marker) return
  try {
    const updated = await ctx.api.updateBlock(ctx.block.id, { marker })
    ctx.onUpdate(updated)
  } catch {
    ctx.toast.error('Failed to set status')
  }
}

const makePriorityHandler: (value: string) => SlashHandler = (value) => async (ctx) => {
  try {
    const updated = await ctx.api.updateBlock(ctx.block.id, {
      priority: value as Priority,
    })
    ctx.onUpdate(updated)
  } catch {
    ctx.toast.error('Failed to set priority')
  }
}

const makeDateHandler: (value: string) => SlashHandler = (value) => (ctx) => {
  const today = new Date().toISOString().split('T')[0]
  const tomorrow = new Date(Date.now() + 86400000).toISOString().split('T')[0]
  const dateStr = value === 'today' ? today : value === 'tomorrow' ? tomorrow : today
  ctx.setContent(dateStr)
}

const makePropertyHandler: (value: string) => SlashHandler = (value) => (ctx) => {
  // Insert property syntax (e.g. "deadline:: ") and place cursor at end.
  const propStr = `${value}:: `
  ctx.setContentAtEnd(propStr)
}

/**
 * Date picker handler — signals BlockRow to open the DatePicker popover
 * for the given date field ('deadline' or 'scheduled').
 *
 * The handler delegates to `ctx.openDatePicker` (provided by BlockRow)
 * which sets React state that mounts the DatePicker popover. On select,
 * BlockRow calls `ctx.setContentAtEnd` AND `ctx.api.updateBlock`.
 *
 * Falls back to the old text-insertion behaviour when `openDatePicker`
 * is not wired (e.g. in legacy/test contexts without BlockRow).
 */
const makeDatePickerHandler: (field: 'deadline' | 'scheduled') => SlashHandler =
  (field) => (ctx) => {
    if (ctx.openDatePicker) {
      ctx.openDatePicker(field)
    } else {
      // Legacy fallback — just insert the property text
      ctx.setContentAtEnd(`${field}:: `)
    }
  }

const makeRefHandler: (value: string) => SlashHandler = (value) => (ctx) => {
  if (value === 'page') {
    ctx.setContentAtEnd('[[')
  } else if (value === 'block') {
    ctx.setContentAtEnd('((')
  }
}

/** Apply a list of property pairs to the block via
 *  `api.setBlockProperty` and then refresh the block from the server
 *  so the parent sees the updated `properties` array. Surfaces errors
 *  via toast (consistent with the other builtin handlers).
 *
 *  Why `setBlockProperty` and not `api.updateBlock({ properties })`?
 *  The Rust `UpdateBlockRequest` does NOT carry a `properties` field —
 *  the only mutator for that column is `PUT /blocks/:id/properties`.
 *  This matches how the rest of the codebase mutates properties
 *  (`PageView.handleKanbanPropertyChange`, `KanbanPage`, the
 *  `BlockPropertiesPanel`).
 *
 *  Why `getPageBlocks` after the writes? `setBlockProperty` returns
 *  `void`, so the only way to hand the parent an up-to-date block
 *  (with the new `properties` shape) is to re-fetch. We scope by
 *  `block.pageName` to avoid a global reload. */
const makeRolePropertiesHandler: (
  pairs: ReadonlyArray<readonly [string, string]>,
  errorMessage: string,
) => SlashHandler = (pairs, errorMessage) => async (ctx) => {
  try {
    for (const [key, value] of pairs) {
      await ctx.api.setBlockProperty(ctx.block.id, key, value)
    }
    if (ctx.block.pageName) {
      const refreshed = await ctx.api.getPageBlocks(ctx.block.pageName)
      const updated = refreshed.find(b => b.id === ctx.block.id)
      if (updated) ctx.onUpdate(updated)
    }
  } catch {
    ctx.toast.error(errorMessage)
  }
}

/** Template insertion — delegates to the BlockRow-supplied wizard.
 *  The wizard (prompt → pick template → create → navigate) is too
 *  multi-step to live in a registry handler, so the component
 *  passes its own implementation via `ctx.templateInsert`. The
 *  original content snapshot is forwarded so cancel paths can
 *  restore. */
const templateInsertHandler: SlashHandler = async (ctx) => {
  if (ctx.templateInsert) {
    await ctx.templateInsert(ctx.originalContent)
  } else {
    ctx.toast.error('Template insertion not available in this context')
  }
}

// ─── The default registry ─────────────────────────────────────────────

/** The single source of truth for slash menu items. Re-exports the
 *  legacy `SLASH_MENU_ITEMS` constant from SlashCommandMenu (which
 *  derives it from `defaultRegistry.allItems()`) so old imports
 *  keep working. */
export const defaultRegistry: SlashActionRegistry = (() => {
  const reg = new SlashActionRegistry()

  // ── Status ──
  reg.register(
    { id: 'status-todo', label: 'TODO', description: 'Mark as to-do', icon: <Circle size={18} />, action: 'status:todo', keywords: ['todo', 'task'], category: 'Status' },
    makeStatusHandler('todo'),
  )
  reg.register(
    { id: 'status-doing', label: 'DOING', description: 'Mark as in progress', icon: <Loader size={18} />, action: 'status:doing', keywords: ['doing', 'in progress', 'wip'], category: 'Status' },
    makeStatusHandler('doing'),
  )
  reg.register(
    { id: 'status-done', label: 'DONE', description: 'Mark as completed', icon: <CheckCircle2 size={18} />, action: 'status:done', keywords: ['done', 'complete', 'finished'], category: 'Status' },
    makeStatusHandler('done'),
  )
  reg.register(
    { id: 'status-now', label: 'NOW', description: 'Mark as current focus', icon: <Zap size={18} />, action: 'status:now', keywords: ['now', 'current', 'active'], category: 'Status' },
    makeStatusHandler('now'),
  )
  reg.register(
    { id: 'status-later', label: 'LATER', description: 'Defer for later', icon: <Clock size={18} />, action: 'status:later', keywords: ['later', 'someday', 'defer'], category: 'Status' },
    makeStatusHandler('later'),
  )
  reg.register(
    { id: 'status-cancelled', label: 'CANCELLED', description: 'Mark as cancelled', icon: <XCircle size={18} />, action: 'status:cancelled', keywords: ['cancelled', 'cancel', 'abandoned'], category: 'Status' },
    makeStatusHandler('cancelled'),
  )
  reg.register(
    { id: 'status-waiting', label: 'WAITING', description: 'Mark as waiting on external', icon: <Hourglass size={18} />, action: 'status:waiting', keywords: ['waiting', 'blocked', 'paused'], category: 'Status' },
    makeStatusHandler('waiting'),
  )

  // ── Priority ──
  reg.register(
    { id: 'priority-a', label: 'Priority A', description: 'Highest priority', icon: <AlertTriangle size={18} />, action: 'priority:A', keywords: ['a', 'high', 'urgent'], category: 'Priority' },
    makePriorityHandler('A'),
  )
  reg.register(
    { id: 'priority-b', label: 'Priority B', description: 'Medium priority', icon: <AlertCircle size={18} />, action: 'priority:B', keywords: ['b', 'medium', 'normal'], category: 'Priority' },
    makePriorityHandler('B'),
  )
  reg.register(
    { id: 'priority-c', label: 'Priority C', description: 'Low priority', icon: <Flag size={18} />, action: 'priority:C', keywords: ['c', 'low', 'nice to have'], category: 'Priority' },
    makePriorityHandler('C'),
  )

  // ── Dates ──
  reg.register(
    { id: 'date-today', label: 'Today', description: "Insert today's date", icon: <Calendar size={18} />, action: 'date:today', keywords: ['today', 'date'], category: 'Dates' },
    makeDateHandler('today'),
  )
  reg.register(
    { id: 'date-tomorrow', label: 'Tomorrow', description: "Insert tomorrow's date", icon: <CalendarDays size={18} />, action: 'date:tomorrow', keywords: ['tomorrow'], category: 'Dates' },
    makeDateHandler('tomorrow'),
  )
  reg.register(
    { id: 'prop-deadline', label: 'Deadline', description: 'Set a deadline', icon: <CalendarX size={18} />, action: 'property:deadline', keywords: ['deadline', 'due', 'by'], category: 'Dates' },
    makeDatePickerHandler('deadline'),
  )
  reg.register(
    { id: 'prop-scheduled', label: 'Scheduled', description: 'Schedule for a date', icon: <CalendarClock size={18} />, action: 'property:scheduled', keywords: ['scheduled', 'plan', 'for'], category: 'Dates' },
    makeDatePickerHandler('scheduled'),
  )

  // ── References ──
  reg.register(
    { id: 'ref-page', label: 'Page Reference', description: 'Link to a page', icon: <FileText size={18} />, action: 'ref:page', keywords: ['page', 'link', '[['], category: 'References' },
    makeRefHandler('page'),
  )
  reg.register(
    { id: 'ref-block', label: 'Block Embed', description: 'Embed a block', icon: <Hash size={18} />, action: 'ref:block', keywords: ['block', 'embed', '(('], category: 'References' },
    makeRefHandler('block'),
  )

  // ── Templates (ADR-0003) ──
  reg.register(
    { id: 'insert-template', label: 'New from Template', description: 'Create a new page from a template', icon: <FilePlus size={18} />, action: 'template:insert', keywords: ['template', 'tpl', 'new from template', 'create from template'], category: 'Templates' },
    templateInsertHandler,
  )

  // ── Actions ──
  reg.register(
    { id: 'add-comment', label: 'Add Comment', description: 'Add a comment to this block', icon: <MessageCircle size={18} />, action: 'comment:add', keywords: ['comment', 'discussion', 'note', 'feedback'], category: 'Actions' },
    defaultCommentHandler,
  )

  // ── Roles (T11/T12/T13) ──
  // Property-only transforms. None of these declare a `blockType`, so
  // the visual block-type picker is unaffected — a `task` role still
  // renders as a paragraph (or whatever blockType the block already is).
  reg.register(
    { id: 'task-role', label: 'Task', description: 'Mark this block as a task (sets type:: task + status:: todo)', icon: <ListChecks size={18} />, action: 'role:task', keywords: ['task', 'todo', 'role', 'rol', 'tarea'], category: 'Roles' },
    makeRolePropertiesHandler(
      [['type', 'task'], ['status', 'todo']],
      'Failed to set task role',
    ),
  )
  reg.register(
    { id: 'query-role', label: 'Query', description: 'Embed a DSL query (prompts for the query string)', icon: <Terminal size={18} />, action: 'role:query', keywords: ['query', 'dsl', 'search', 'filter', 'rol'], category: 'Roles' },
    async (ctx) => {
      const dsl = window.prompt('Enter a DSL query:')
      if (!dsl || dsl.trim().length === 0) return
      await makeRolePropertiesHandler(
        [['type', 'query'], ['dsl', dsl.trim()]],
        'Failed to set query role',
      )(ctx, defaultRegistry.getItem('query-role')!)
    },
  )
  reg.register(
    { id: 'card-role', label: 'Card', description: 'Turn this block into a card (sets card-shape::)', icon: <LayoutTemplate size={18} />, action: 'role:card', keywords: ['card', 'shape', 'carta', 'tarjeta', 'rol'], category: 'Roles' },
    async (ctx) => {
      const CARD_SHAPES = ['content', 'reference', 'presentation', 'article', 'note'] as const
      const raw = window.prompt(
        `Card shape (one of: ${CARD_SHAPES.join(', ')}):`,
        'content',
      )
      const shape = (CARD_SHAPES as readonly string[]).includes(raw ?? '')
        ? (raw as string)
        : 'content'
      await makeRolePropertiesHandler(
        [['card-shape', shape]],
        'Failed to set card role',
      )(ctx, defaultRegistry.getItem('card-role')!)
    },
  )

  // ── Block Types (existing) ──
  // Items with `blockType` get `defaultBlockTypeHandler` automatically.
  const blockTypeEntries: Array<Omit<SlashMenuItem, 'icon' | 'keywords'> & { icon: React.ReactNode; keywords: string[] }> = [
    { id: 'paragraph', label: 'Text', description: 'Plain text block', icon: <Type size={18} />, blockType: 'paragraph', keywords: ['text', 'paragraph', 'plain'], category: 'Block Types' },
    { id: 'heading1', label: 'Heading 1', description: 'Large section heading', icon: <Heading1 size={18} />, blockType: 'heading1', keywords: ['heading', 'h1', 'title'], category: 'Block Types' },
    { id: 'heading2', label: 'Heading 2', description: 'Medium section heading', icon: <Heading2 size={18} />, blockType: 'heading2', keywords: ['heading', 'h2', 'subtitle'], category: 'Block Types' },
    { id: 'heading3', label: 'Heading 3', description: 'Small section heading', icon: <Heading3 size={18} />, blockType: 'heading3', keywords: ['heading', 'h3'], category: 'Block Types' },
    { id: 'bullet', label: 'Bullet List', description: 'Bulleted list item', icon: <List size={18} />, blockType: 'bullet', keywords: ['bullet', 'list', 'ul'], category: 'Block Types' },
    { id: 'numbered', label: 'Numbered List', description: 'Numbered list item', icon: <ListOrdered size={18} />, blockType: 'numbered', keywords: ['numbered', 'list', 'ol'], category: 'Block Types' },
    { id: 'todo', label: 'To-do', description: 'Checkbox task item', icon: <CheckSquare size={18} />, blockType: 'todo', keywords: ['todo', 'check', 'task', 'checkbox'], category: 'Block Types' },
    { id: 'quote', label: 'Quote', description: 'Block quotation', icon: <Quote size={18} />, blockType: 'quote', keywords: ['quote', 'blockquote'], category: 'Block Types' },
    { id: 'code', label: 'Code Block', description: 'Code snippet block', icon: <Code size={18} />, blockType: 'code', keywords: ['code', 'snippet', 'programming'], category: 'Block Types' },
    { id: 'divider', label: 'Divider', description: 'Horizontal divider line', icon: <Minus size={18} />, blockType: 'divider', keywords: ['divider', 'hr', 'separator', 'line'], category: 'Block Types' },
    { id: 'image', label: 'Image', description: 'Embed an image', icon: <Image size={18} />, blockType: 'image', keywords: ['image', 'photo', 'picture'], category: 'Block Types' },
  ]
  for (const entry of blockTypeEntries) {
    reg.register(entry, defaultBlockTypeHandler)
  }

  return reg
})()
