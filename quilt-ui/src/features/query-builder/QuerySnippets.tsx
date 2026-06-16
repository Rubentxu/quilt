/**
 * QuerySnippets — predefined DSL query templates.
 *
 * Each snippet is a ready-to-execute DSL query the user can insert
 * with one click. Snippets are categorised by intent.
 *
 * Categories:
 *   - Journal: time-based queries over daily pages
 *   - Tasks: task marker filters
 *   - Pages: page-level filters
 *   - Recent: temporal recency filters
 *   - Media: content-type filters
 */

import { Copy, Check, Calendar, ListTodo, FileText, Clock, Image } from 'lucide-react'

export interface Snippet {
  id: string
  label: string
  description: string
  dsl: string
  category: SnippetCategory
  icon: React.ReactNode
}

export type SnippetCategory = 'journal' | 'tasks' | 'pages' | 'recent' | 'media'

export const SNIPPET_CATEGORIES: SnippetCategory[] = ['journal', 'tasks', 'pages', 'recent', 'media']

export const SNIPPET_CATEGORY_LABELS: Record<SnippetCategory, string> = {
  journal: 'Journal',
  tasks: 'Tasks',
  pages: 'Pages',
  recent: 'Recent',
  media: 'Media',
}

export const QUERY_SNIPPETS: Snippet[] = [
  // ── Journal ──────────────────────────────────────────────────────
  {
    id: 'journal-this-week',
    label: 'Journal this week',
    description: 'All blocks in journal pages created this week',
    dsl: '(temporal :this-week (page "{{page}}"))',
    category: 'journal',
    icon: <Calendar size={14} />,
  },
  {
    id: 'journal-today',
    label: 'Journal today',
    description: 'All blocks in today\'s journal page',
    dsl: '(temporal :today (page "{{page}}"))',
    category: 'journal',
    icon: <Calendar size={14} />,
  },

  // ── Tasks ─────────────────────────────────────────────────────────
  {
    id: 'tasks-scheduled-today',
    label: 'Tasks scheduled today',
    description: 'Tasks with scheduled date matching today',
    dsl: '(scheduled today)',
    category: 'tasks',
    icon: <ListTodo size={14} />,
  },
  {
    id: 'tasks-todo',
    label: 'All TODO tasks',
    description: 'Every block marked with the TODO task marker',
    dsl: '(task todo)',
    category: 'tasks',
    icon: <ListTodo size={14} />,
  },
  {
    id: 'tasks-overdue',
    label: 'Overdue tasks',
    description: 'Tasks with a scheduled date in the past',
    dsl: '(overdue)',
    category: 'tasks',
    icon: <ListTodo size={14} />,
  },
  {
    id: 'tasks-in-progress',
    label: 'In-progress tasks',
    description: 'Tasks currently marked as in-progress',
    dsl: '(in-progress)',
    category: 'tasks',
    icon: <ListTodo size={14} />,
  },
  {
    id: 'tasks-high-priority',
    label: 'High-priority tasks',
    description: 'Tasks with priority A',
    dsl: '(priority a)',
    category: 'tasks',
    icon: <ListTodo size={14} />,
  },

  // ── Pages ─────────────────────────────────────────────────────────
  {
    id: 'page-by-tag',
    label: 'Pages with tag',
    description: 'All pages that have a specific tag',
    dsl: '(tags "{{tag}}")',
    category: 'pages',
    icon: <FileText size={14} />,
  },
  {
    id: 'page-by-name',
    label: 'Page by name',
    description: 'Blocks on a specific page',
    dsl: '(page "{{page-name}}")',
    category: 'pages',
    icon: <FileText size={14} />,
  },

  // ── Recent ────────────────────────────────────────────────────────
  {
    id: 'recent-7-days',
    label: 'Recent pages (7 days)',
    description: 'Pages created in the last 7 days',
    dsl: '(temporal :relative "-7d" (page "{{page}}"))',
    category: 'recent',
    icon: <Clock size={14} />,
  },
  {
    id: 'recent-30-days',
    label: 'Recent pages (30 days)',
    description: 'Pages created in the last 30 days',
    dsl: '(temporal :relative "-30d" (page "{{page}}"))',
    category: 'recent',
    icon: <Clock size={14} />,
  },

  // ── Media ─────────────────────────────────────────────────────────
  {
    id: 'media-with-images',
    label: 'Blocks with images',
    description: 'Blocks containing image references',
    dsl: '(and (full-text-search "![[") (page "{{page}}"))',
    category: 'media',
    icon: <Image size={14} />,
  },
]

interface QuerySnippetsProps {
  /** Called when the user clicks a snippet — returns the DSL string to insert. */
  onInsert: (dsl: string) => void
  /** Show only this category (undefined = show all). */
  categoryFilter?: SnippetCategory
}

/**
 * Renders a panel of predefined query snippets grouped by category.
 * Each snippet is a clickable row that calls onInsert with the DSL.
 */
export function QuerySnippets({ onInsert, categoryFilter }: QuerySnippetsProps) {
  const categories = categoryFilter
    ? [categoryFilter]
    : SNIPPET_CATEGORIES

  return (
    <div
      data-testid="query-snippets"
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 'var(--space-3)',
      }}
    >
      {categories.map(cat => {
        const snippets = QUERY_SNIPPETS.filter(s => s.category === cat)
        if (snippets.length === 0) return null
        return (
          <div key={cat}>
            <div
              style={{
                fontSize: 'var(--font-size-micro)',
                fontWeight: 600,
                textTransform: 'uppercase',
                letterSpacing: 'var(--tracking-wider)',
                color: 'var(--color-text-muted)',
                marginBottom: 'var(--space-1)',
              }}
            >
              {SNIPPET_CATEGORY_LABELS[cat]}
            </div>
            <div
              style={{
                display: 'flex',
                flexDirection: 'column',
                gap: '2px',
              }}
            >
              {snippets.map(snippet => (
                <SnippetRow
                  key={snippet.id}
                  snippet={snippet}
                  onInsert={onInsert}
                />
              ))}
            </div>
          </div>
        )
      })}
    </div>
  )
}

function SnippetRow({
  snippet,
  onInsert,
}: {
  snippet: Snippet
  onInsert: (dsl: string) => void
}) {
  const [copied, setCopied] = useState(false)

  function handleInsert() {
    onInsert(snippet.dsl)
  }

  function handleCopy(e: React.MouseEvent) {
    e.stopPropagation()
    navigator.clipboard.writeText(snippet.dsl).then(() => {
      setCopied(true)
      setTimeout(() => setCopied(false), 1500)
    })
  }

  return (
    <div
      role="button"
      tabIndex={0}
      data-testid={`snippet-row-${snippet.id}`}
      onClick={handleInsert}
      onKeyDown={e => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault()
          handleInsert()
        }
      }}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 'var(--space-2)',
        padding: '6px var(--space-2)',
        borderRadius: 'var(--radius-sm)',
        cursor: 'pointer',
        transition: 'background var(--motion-fast) var(--ease-standard)',
        background: 'transparent',
      }}
      onMouseEnter={e => {
        e.currentTarget.style.background = 'var(--color-surface-subtle)'
      }}
      onMouseLeave={e => {
        e.currentTarget.style.background = 'transparent'
      }}
    >
      <span
        style={{
          color: 'var(--color-text-muted)',
          flexShrink: 0,
          display: 'flex',
          alignItems: 'center',
        }}
      >
        {snippet.icon}
      </span>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div
          style={{
            fontSize: 'var(--font-size-caption)',
            fontWeight: 600,
            color: 'var(--color-text-primary)',
          }}
        >
          {snippet.label}
        </div>
        <div
          style={{
            fontSize: 'var(--font-size-micro)',
            color: 'var(--color-text-muted)',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          {snippet.description}
        </div>
      </div>
      <button
        type="button"
        data-testid={`snippet-copy-${snippet.id}`}
        aria-label="Copy DSL"
        onClick={handleCopy}
        onMouseDown={e => e.stopPropagation()}
        onKeyDown={e => e.stopPropagation()}
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          justifyContent: 'center',
          padding: '4px',
          background: 'transparent',
          border: 'none',
          borderRadius: 'var(--radius-sm)',
          color: copied ? 'var(--color-accent)' : 'var(--color-text-muted)',
          cursor: 'pointer',
          flexShrink: 0,
        }}
      >
        {copied ? <Check size={12} /> : <Copy size={12} />}
      </button>
    </div>
  )
}

import { useState } from 'react'
