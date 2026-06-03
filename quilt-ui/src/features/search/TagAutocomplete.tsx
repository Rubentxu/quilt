import { useState, useEffect, useRef, useMemo } from 'react'
import { Hash } from 'lucide-react'

interface TagAutocompleteProps {
  /** Position to render the dropdown at */
  position: { top: number; left: number } | null
  /** Current search query (text after #) */
  query: string
  /** Called when user selects a tag */
  onSelect: (tagName: string) => void
  /** Called when user cancels */
  onClose: () => void
}

/**
 * Default tag suggestions shown when the user types `#` in a block.
 *
 * Logseq ships a small built-in set (todo, bug, urgent, wip, idea,
 * question, important, done) and we mirror that for the MVP. A future
 * `api.listTags()` can replace this list with the user's actual tag
 * vocabulary (counts by usage) without changing this component's
 * contract.
 */
const DEFAULT_TAGS: readonly string[] = [
  'todo',
  'bug',
  'urgent',
  'wip',
  'idea',
  'question',
  'important',
  'done',
] as const

export function TagAutocomplete({ position, query, onSelect, onClose }: TagAutocompleteProps) {
  const [selectedIndex, setSelectedIndex] = useState(0)
  const menuRef = useRef<HTMLDivElement>(null)

  // Filter tags by prefix (case-insensitive). Empty query shows all.
  const tags = useMemo(() => {
    const q = query.toLowerCase()
    return DEFAULT_TAGS.filter(t => t.toLowerCase().startsWith(q)).slice(0, 8)
  }, [query])

  // Reset selection to the first row whenever the result set changes.
  useEffect(() => {
    setSelectedIndex(0)
  }, [tags])

  // Click outside handler — mirrors PageAutocomplete / BlockAutocomplete.
  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose()
      }
    }
    if (position) {
      document.addEventListener('mousedown', handleClickOutside)
      return () => document.removeEventListener('mousedown', handleClickOutside)
    }
  }, [position, onClose])

  // Keyboard navigation — ArrowUp/Down to move, Enter to select, Esc to close.
  // The BlockRow editor also intercepts these keys (so the editor doesn't
  // split the block or move the caret while the menu is open); the actual
  // selection update lives here.
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (!position) return
      switch (e.key) {
        case 'ArrowDown':
          e.preventDefault()
          setSelectedIndex(i => (i + 1) % Math.max(tags.length, 1))
          break
        case 'ArrowUp':
          e.preventDefault()
          setSelectedIndex(i => (i - 1 + tags.length) % Math.max(tags.length, 1))
          break
        case 'Enter':
          e.preventDefault()
          if (tags[selectedIndex]) {
            onSelect(tags[selectedIndex])
          }
          break
        case 'Escape':
          e.preventDefault()
          onClose()
          break
      }
    }
    if (position) {
      document.addEventListener('keydown', handleKeyDown)
      return () => document.removeEventListener('keydown', handleKeyDown)
    }
  }, [position, tags, selectedIndex, onSelect, onClose])

  if (!position || tags.length === 0) return null

  return (
    <div
      ref={menuRef}
      role="listbox"
      aria-label="Tag suggestions"
      data-testid="tag-autocomplete"
      style={{
        position: 'fixed',
        top: position.top,
        left: position.left,
        zIndex: 200,
        background: 'var(--color-surface)',
        border: '1px solid var(--color-border)',
        borderRadius: 'var(--radius-md)',
        boxShadow: 'var(--shadow-md)',
        minWidth: '200px',
        maxHeight: '280px',
        overflowY: 'auto',
      }}
    >
      <div
        style={{
          padding: 'var(--space-1) var(--space-3)',
          fontSize: '11px',
          fontWeight: 600,
          color: 'var(--color-text-muted)',
          textTransform: 'uppercase',
          letterSpacing: '0.05em',
        }}
      >
        Tags
      </div>
      {tags.map((tag, i) => (
        <div
          key={tag}
          role="option"
          aria-selected={i === selectedIndex}
          data-testid={`tag-option-${tag}`}
          onClick={() => onSelect(tag)}
          onMouseEnter={() => setSelectedIndex(i)}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 'var(--space-2)',
            padding: 'var(--space-2) var(--space-3)',
            cursor: 'pointer',
            background: i === selectedIndex ? 'var(--color-surface-subtle)' : 'transparent',
            color: 'var(--color-text-primary)',
            fontSize: '13px',
          }}
        >
          <Hash size={14} style={{ color: 'var(--color-text-muted)', flexShrink: 0 }} />
          <span>{tag}</span>
        </div>
      ))}
    </div>
  )
}
