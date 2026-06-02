import { useState, useEffect, useRef } from 'react'
import { Search, FileText, Calendar } from 'lucide-react'
import { useNavigate } from '@tanstack/react-router'
import { api } from '@core/api-client'
import type { Page } from '@shared/types/api'
import toast from 'react-hot-toast'

interface SearchModalProps {
  isOpen: boolean
  onClose: () => void
}

export function SearchModal({ isOpen, onClose }: SearchModalProps) {
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<Page[]>([])
  const [selectedIndex, setSelectedIndex] = useState(0)
  const [loading, setLoading] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)
  const navigate = useNavigate()

  // Focus input when opening
  useEffect(() => {
    if (isOpen) {
      // Reset state on open
      setQuery('')
      setResults([])
      setSelectedIndex(0)
      // Use RAF to ensure DOM is ready
      const raf = requestAnimationFrame(() => inputRef.current?.focus())
      return () => cancelAnimationFrame(raf)
    }
  }, [isOpen])

  // Search with debounce — shows recent pages when query is empty
  useEffect(() => {
    if (!query.trim()) {
      setLoading(true)
      api.listPages()
        .then(pages => {
          setResults(pages.slice(0, 10))
        })
        .catch(() => {
          // silently fail for initial load
        })
        .finally(() => setLoading(false))
      return
    }

    setLoading(true)
    const timer = setTimeout(async () => {
      try {
        const allPages = await api.listPages()
        const q = query.toLowerCase()
        const filtered = allPages.filter(p =>
          p.name.toLowerCase().includes(q) ||
          (p.title && p.title.toLowerCase().includes(q))
        )
        setResults(filtered.slice(0, 20))
        setSelectedIndex(0)
      } catch (e) {
        toast.error('Search failed')
      } finally {
        setLoading(false)
      }
    }, 200)

    return () => clearTimeout(timer)
  }, [query])

  // Keyboard navigation
  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setSelectedIndex(i => Math.min(i + 1, results.length - 1))
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setSelectedIndex(i => Math.max(i - 1, 0))
    } else if (e.key === 'Enter' && results[selectedIndex]) {
      selectResult(results[selectedIndex])
    } else if (e.key === 'Escape') {
      onClose()
    }
  }

  function selectResult(page: Page) {
    onClose()
    if (page.journal && page.journalDay) {
      // Convert journalDay (YYYYMMDD integer) to YYYY-MM-DD string
      const day = page.journalDay.toString()
      const date = `${day.slice(0, 4)}-${day.slice(4, 6)}-${day.slice(6, 8)}`
      navigate({ to: '/journal/$date', params: { date } })
    } else {
      navigate({ to: '/page/$name', params: { name: page.name } })
    }
  }

  if (!isOpen) return null

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 100,
        display: 'flex',
        alignItems: 'flex-start',
        justifyContent: 'center',
        paddingTop: '15vh',
        background: 'rgba(0, 0, 0, 0.4)',
      }}
      onClick={onClose}
    >
      <div
        style={{
          width: '100%',
          maxWidth: '640px',
          background: 'var(--color-surface)',
          borderRadius: 'var(--radius-lg)',
          boxShadow: 'var(--shadow-lg)',
          overflow: 'hidden',
        }}
        onClick={e => e.stopPropagation()}
      >
        {/* ─── Search input ─── */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            padding: 'var(--space-3) var(--space-4)',
            borderBottom: '1px solid var(--color-border)',
            gap: 'var(--space-3)',
          }}
        >
          <Search size={18} style={{ color: 'var(--color-text-muted)', flexShrink: 0 }} />
          <input
            ref={inputRef}
            value={query}
            onChange={e => { setQuery(e.target.value); setSelectedIndex(0) }}
            onKeyDown={handleKeyDown}
            placeholder="Search pages…"
            style={{
              flex: 1,
              border: 'none',
              outline: 'none',
              fontSize: '16px',
              background: 'transparent',
              color: 'var(--color-text-primary)',
            }}
          />
          <button
            onClick={onClose}
            style={{
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              color: 'var(--color-text-muted)',
              fontSize: '12px',
              fontFamily: 'var(--font-family)',
            }}
          >
            ESC
          </button>
        </div>

        {/* ─── Results ─── */}
        <div style={{ maxHeight: '400px', overflowY: 'auto' }}>
          {loading && results.length === 0 && (
            <div
              style={{
                padding: 'var(--space-8)',
                textAlign: 'center',
                color: 'var(--color-text-muted)',
                fontSize: '14px',
              }}
            >
              Searching…
            </div>
          )}

          {!loading && results.length === 0 && query && (
            <div
              style={{
                padding: 'var(--space-8)',
                textAlign: 'center',
                color: 'var(--color-text-muted)',
                fontSize: '14px',
              }}
            >
              No results for "{query}"
            </div>
          )}

          {results.map((page, index) => (
            <button
              key={page.id}
              onClick={() => selectResult(page)}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 'var(--space-3)',
                width: '100%',
                padding: 'var(--space-3) var(--space-4)',
                border: 'none',
                cursor: 'pointer',
                textAlign: 'left',
                background: index === selectedIndex ? 'var(--color-surface-subtle)' : 'transparent',
                color: index === selectedIndex ? 'var(--color-text-primary)' : 'var(--color-text-secondary)',
                fontSize: '14px',
                transition: 'background var(--motion-fast) var(--ease-standard)',
              }}
              onMouseEnter={() => setSelectedIndex(index)}
            >
              {page.journal ? (
                <Calendar size={16} style={{ flexShrink: 0, color: 'var(--color-accent)' }} />
              ) : (
                <FileText size={16} style={{ flexShrink: 0, color: 'var(--color-text-muted)' }} />
              )}
              <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                {page.title || page.name}
              </span>
              {page.journal && (
                <span
                  style={{
                    fontSize: '11px',
                    color: 'var(--color-text-muted)',
                    marginLeft: 'auto',
                    flexShrink: 0,
                  }}
                >
                  Journal
                </span>
              )}
            </button>
          ))}
        </div>
      </div>
    </div>
  )
}
