import { useState, useEffect, useRef } from 'react'
import { api } from '@core/api-client'
import type { Block } from '@shared/types/api'
import { Hash } from 'lucide-react'

interface BlockAutocompleteProps {
  /** Position to render the dropdown at */
  position: { top: number; left: number } | null
  /** Current search query (text after (( ) */
  query: string
  /** Called when user selects a block */
  onSelect: (blockId: string) => void
  /** Called when user cancels */
  onClose: () => void
}

export function BlockAutocomplete({ position, query, onSelect, onClose }: BlockAutocompleteProps) {
  const [blocks, setBlocks] = useState<Block[]>([])
  const [selectedIndex, setSelectedIndex] = useState(0)
  const menuRef = useRef<HTMLDivElement>(null)

  // Debounced search via API
  useEffect(() => {
    if (!query || query.length < 2) {
      setBlocks([])
      return
    }
    const timeout = setTimeout(async () => {
      try {
        const results = await api.searchBlocks(query, 8)
        setBlocks(results)
      } catch {
        setBlocks([])
      }
    }, 150)
    return () => clearTimeout(timeout)
  }, [query])

  // Reset selection when results change
  useEffect(() => {
    setSelectedIndex(0)
  }, [blocks])

  // Click outside handler
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

  // Keyboard navigation
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (!position) return
      switch (e.key) {
        case 'ArrowDown':
          e.preventDefault()
          setSelectedIndex(i => (i + 1) % Math.max(blocks.length, 1))
          break
        case 'ArrowUp':
          e.preventDefault()
          setSelectedIndex(i => (i - 1 + blocks.length) % Math.max(blocks.length, 1))
          break
        case 'Enter':
          e.preventDefault()
          if (blocks[selectedIndex]) {
            onSelect(blocks[selectedIndex].id)
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
  }, [position, blocks, selectedIndex, onSelect, onClose])

  if (!position || blocks.length === 0) return null

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
        boxShadow: 'var(--shadow-md)',
        minWidth: '300px',
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
        Block References
      </div>
      {blocks.map((block, i) => (
        <div
          key={block.id}
          onClick={() => onSelect(block.id)}
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
          <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
            {block.content?.substring(0, 80) || '(empty block)'}
          </span>
        </div>
      ))}
    </div>
  )
}
