import { useState, useEffect, useRef } from 'react'
import { api } from '@core/api-client'
import type { Page } from '@shared/types/api'

interface PageAutocompleteProps {
  /** Position to render the dropdown at */
  position: { top: number; left: number } | null
  /** Current search query (text after [[) */
  query: string
  /** Called when user selects a page */
  onSelect: (pageName: string) => void
  /** Called when user cancels */
  onClose: () => void
}

export function PageAutocomplete({ position, query, onSelect, onClose }: PageAutocompleteProps) {
  const [pages, setPages] = useState<Page[]>([])
  const [selectedIndex, setSelectedIndex] = useState(0)
  const [allPages, setAllPages] = useState<Page[]>([])
  const ref = useRef<HTMLDivElement>(null)

  // Load pages once
  useEffect(() => {
    api.listPages().then(setAllPages).catch(() => {})
  }, [])

  // Filter by query
  useEffect(() => {
    const q = query.toLowerCase()
    const filtered = allPages.filter(p =>
      p.name.toLowerCase().includes(q) || (p.title && p.title.toLowerCase().includes(q))
    ).slice(0, 8)
    setPages(filtered)
    setSelectedIndex(0)
  }, [query, allPages])

  // Click outside handler
  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onClose()
      }
    }
    if (position) {
      // Use mousedown to catch clicks before blur events
      document.addEventListener('mousedown', handleClickOutside)
      return () => document.removeEventListener('mousedown', handleClickOutside)
    }
  }, [position, onClose])

  if (!position || pages.length === 0) return null

  return (
    <div
      ref={ref}
      style={{
        position: 'fixed',
        top: position.top,
        left: position.left,
        zIndex: 200,
        background: 'var(--color-surface)',
        border: '1px solid var(--color-border)',
        borderRadius: 'var(--radius-md)',
        boxShadow: 'var(--shadow-md)',
        minWidth: '250px',
        maxHeight: '300px',
        overflowY: 'auto',
      }}
    >
      {pages.map((page, i) => (
        <div
          key={page.id}
          onClick={() => onSelect(page.name)}
          onMouseEnter={() => setSelectedIndex(i)}
          style={{
            padding: 'var(--space-2) var(--space-3)',
            cursor: 'pointer',
            fontSize: '13px',
            background: i === selectedIndex ? 'var(--color-surface-subtle)' : 'transparent',
            color: i === selectedIndex ? 'var(--color-text-primary)' : 'var(--color-text-secondary)',
          }}
        >
          {page.title || page.name}
        </div>
      ))}
    </div>
  )
}
