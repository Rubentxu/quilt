import { useState, useEffect, useRef } from 'react'
import {
  Type, Heading1, Heading2, Heading3, List, ListOrdered,
  CheckSquare, Quote, Code, Minus, Image,
  Circle, Loader, CheckCircle2, Zap, Clock, XCircle,
  AlertTriangle, AlertCircle, Flag,
  Calendar, CalendarDays, CalendarX, CalendarClock,
  FileText, FilePlus, Hash, MessageCircle,
} from 'lucide-react'

export interface SlashMenuItem {
  id: string
  label: string
  description: string
  icon: React.ReactNode
  blockType?: string  // For block type changes
  action?: string     // For property/status actions
  keywords: string[]
  category: string    // Grouping for the menu
}

export const SLASH_MENU_ITEMS: SlashMenuItem[] = [
  // ── Status ──
  { id: 'status-todo', label: 'TODO', description: 'Mark as to-do', icon: <Circle size={18} />, action: 'status:todo', keywords: ['todo', 'task'], category: 'Status' },
  { id: 'status-doing', label: 'DOING', description: 'Mark as in progress', icon: <Loader size={18} />, action: 'status:doing', keywords: ['doing', 'in progress', 'wip'], category: 'Status' },
  { id: 'status-done', label: 'DONE', description: 'Mark as completed', icon: <CheckCircle2 size={18} />, action: 'status:done', keywords: ['done', 'complete', 'finished'], category: 'Status' },
  { id: 'status-now', label: 'NOW', description: 'Mark as current focus', icon: <Zap size={18} />, action: 'status:now', keywords: ['now', 'current', 'active'], category: 'Status' },
  { id: 'status-later', label: 'LATER', description: 'Defer for later', icon: <Clock size={18} />, action: 'status:later', keywords: ['later', 'someday', 'defer'], category: 'Status' },
  { id: 'status-cancelled', label: 'CANCELLED', description: 'Mark as cancelled', icon: <XCircle size={18} />, action: 'status:cancelled', keywords: ['cancelled', 'cancel', 'abandoned'], category: 'Status' },

  // ── Priority ──
  { id: 'priority-a', label: 'Priority A', description: 'Highest priority', icon: <AlertTriangle size={18} />, action: 'priority:A', keywords: ['a', 'high', 'urgent'], category: 'Priority' },
  { id: 'priority-b', label: 'Priority B', description: 'Medium priority', icon: <AlertCircle size={18} />, action: 'priority:B', keywords: ['b', 'medium', 'normal'], category: 'Priority' },
  { id: 'priority-c', label: 'Priority C', description: 'Low priority', icon: <Flag size={18} />, action: 'priority:C', keywords: ['c', 'low', 'nice to have'], category: 'Priority' },

  // ── Dates ──
  { id: 'date-today', label: 'Today', description: "Insert today's date", icon: <Calendar size={18} />, action: 'date:today', keywords: ['today', 'date'], category: 'Dates' },
  { id: 'date-tomorrow', label: 'Tomorrow', description: "Insert tomorrow's date", icon: <CalendarDays size={18} />, action: 'date:tomorrow', keywords: ['tomorrow'], category: 'Dates' },
  { id: 'prop-deadline', label: 'Deadline', description: 'Set a deadline', icon: <CalendarX size={18} />, action: 'property:deadline', keywords: ['deadline', 'due', 'by'], category: 'Dates' },
  { id: 'prop-scheduled', label: 'Scheduled', description: 'Schedule for a date', icon: <CalendarClock size={18} />, action: 'property:scheduled', keywords: ['scheduled', 'plan', 'for'], category: 'Dates' },

  // ── References ──
  { id: 'ref-page', label: 'Page Reference', description: 'Link to a page', icon: <FileText size={18} />, action: 'ref:page', keywords: ['page', 'link', '[['], category: 'References' },
  { id: 'ref-block', label: 'Block Embed', description: 'Embed a block', icon: <Hash size={18} />, action: 'ref:block', keywords: ['block', 'embed', '(('], category: 'References' },

  // ── Templates (ADR-0003) ──
  // Creates a new page from a template. Templates are regular pages
  // whose name starts with `template/` (e.g. `template/daily-note`).
  // Label "New from Template" distinguishes this PAGE-creation action
  // from the future "Apply template" BLOCK-creation action (deferred
  // to PR 2 of quilt-fase2-ux-templates-discoverability).
  { id: 'insert-template', label: 'New from Template', description: 'Create a new page from a template', icon: <FilePlus size={18} />, action: 'template:insert', keywords: ['template', 'tpl', 'new from template', 'create from template'], category: 'Templates' },

  // ── Actions ──
  { id: 'add-comment', label: 'Add Comment', description: 'Add a comment to this block', icon: <MessageCircle size={18} />, action: 'comment:add', keywords: ['comment', 'discussion', 'note', 'feedback'], category: 'Actions' },

  // ── Block Types (existing) ──
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

interface SlashCommandMenuProps {
  position: { top: number; left: number } | null
  query: string
  onSelect: (item: SlashMenuItem) => void
  onClose: () => void
}

export function SlashCommandMenu({ position, query, onSelect, onClose }: SlashCommandMenuProps) {
  const [selectedIndex, setSelectedIndex] = useState(0)
  const menuRef = useRef<HTMLDivElement>(null)

  const filteredItems = SLASH_MENU_ITEMS.filter(item => {
    const q = query.toLowerCase()
    return (
      item.label.toLowerCase().includes(q) ||
      (item.blockType?.toLowerCase().includes(q) ?? false) ||
      item.keywords.some(k => k.includes(q))
    )
  })

  useEffect(() => {
    setSelectedIndex(0)
  }, [query])

  // Keyboard navigation
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (!position) return

      switch (e.key) {
        case 'ArrowDown':
          e.preventDefault()
          setSelectedIndex(i => (i + 1) % filteredItems.length)
          break
        case 'ArrowUp':
          e.preventDefault()
          setSelectedIndex(i => (i - 1 + filteredItems.length) % filteredItems.length)
          break
        case 'Enter':
          e.preventDefault()
          if (filteredItems[selectedIndex]) {
            onSelect(filteredItems[selectedIndex])
          }
          break
        case 'Escape':
          e.preventDefault()
          onClose()
          break
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [position, filteredItems, selectedIndex, onSelect, onClose])

  // Click outside closes menu
  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose()
      }
    }
    document.addEventListener('mousedown', handleClick)
    return () => document.removeEventListener('mousedown', handleClick)
  }, [onClose])

  if (!position || filteredItems.length === 0) return null

  // Group filtered items by category
  const grouped = new Map<string, SlashMenuItem[]>()
  for (const item of filteredItems) {
    const cat = item.category
    if (!grouped.has(cat)) grouped.set(cat, [])
    grouped.get(cat)!.push(item)
  }

  return (
    <div
      ref={menuRef}
      style={{
        position: 'fixed',
        top: position.top,
        left: position.left,
        zIndex: 200,
        background: 'var(--color-surface)',
        border: '1px solid var(--color-border)',
        borderRadius: 'var(--radius-md)',
        boxShadow: 'var(--shadow-lg)',
        minWidth: '280px',
        maxHeight: '420px',
        overflowY: 'auto',
        padding: 'var(--space-1) 0',
      }}
    >
      {[...grouped.entries()].map(([category, items]) => (
        <div key={category}>
          <div style={{
            padding: 'var(--space-1) var(--space-3)',
            fontSize: '11px',
            fontWeight: 600,
            color: 'var(--color-text-muted)',
            textTransform: 'uppercase',
            letterSpacing: '0.05em',
            background: 'var(--color-surface-subtle)',
          }}>
            {category}
          </div>
          {items.map(item => {
            const globalIdx = filteredItems.indexOf(item)
            return (
              <div
                key={item.id}
                onClick={() => onSelect(item)}
                onMouseEnter={() => setSelectedIndex(globalIdx)}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 'var(--space-3)',
                  padding: 'var(--space-2) var(--space-3)',
                  cursor: 'pointer',
                  background: globalIdx === selectedIndex ? 'var(--color-surface-subtle)' : 'transparent',
                  transition: 'background var(--motion-fast)',
                }}
              >
                <div style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  width: '32px',
                  height: '32px',
                  borderRadius: 'var(--radius-sm)',
                  border: '1px solid var(--color-border)',
                  background: 'var(--color-surface)',
                  color: 'var(--color-text-secondary)',
                  flexShrink: 0,
                }}>
                  {item.icon}
                </div>
                <div>
                  <div style={{ fontSize: '14px', fontWeight: 500, color: 'var(--color-text-primary)' }}>
                    {item.label}
                  </div>
                  <div style={{ fontSize: '12px', color: 'var(--color-text-muted)' }}>
                    {item.description}
                  </div>
                </div>
              </div>
            )
          })}
        </div>
      ))}
    </div>
  )
}
